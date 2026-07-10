//! Companion-file lifecycle for "multi-host dist" repositories such as
//! [PaperSpine v4](https://github.com/WUBING2023/PaperSpine).
//!
//! Some upstream repositories ship, alongside the main skill folder, extra
//! entrypoint files that go **outside** the tool's `skills/` directory. The
//! canonical example is a Claude Code slash command at `~/.claude/commands/foo.md`
//! that pairs with a skill at `~/.claude/skills/foo/`.
//!
//! Skills Hub's install-once model requires us to:
//!
//! 1. **Stage** those companion files into a Skills Hub-owned area under the
//!    central repo path so we don't need to re-clone every time the user adds
//!    another sync target.
//! 2. **Install** the correct subset to each tool's home directory when the
//!    user chooses to sync that tool.
//! 3. **Track** every installed target path in the database so we can clean up
//!    on uninstall or un-sync.
//!
//! For tools whose companion files the upstream repo does *not* ship (Kiro,
//! Cursor, Windsurf, Gemini CLI, Cline, Roo Code, ...), nothing happens here —
//! only the skill folder itself is synced. Those tools rely on the skill's
//! `SKILL.md` frontmatter description to auto-trigger, which is by design.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use uuid::Uuid;

use super::installer::{CompanionFile, MultiHostDistManifest};
use super::skill_store::{SkillCompanionRecord, SkillStore};

/// The subdirectory under the central repo root that holds staged companion
/// files. Leading dot keeps it hidden from most skill scanners.
const META_DIR: &str = ".skillshub-meta";
const COMPANIONS_SUBDIR: &str = "companions";

/// A companion file that has been staged into the central metadata area,
/// ready to be installed to a tool's home directory when the user syncs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedCompanion {
    /// The Skills Hub tool key that this companion is scoped to
    /// (e.g. "claude_code", "codex").
    pub tool_key: String,
    /// Absolute path to the staged file inside the central metadata area.
    pub staged_path: PathBuf,
    /// Path relative to the tool's home marker where this file should land
    /// (e.g. ".claude/commands/paperspine.md").
    pub target_rel: String,
}

/// Location where staged companions live for a given skill inside central.
fn companions_root_for_skill(central_repo_root: &Path, skill_name: &str) -> PathBuf {
    central_repo_root
        .join(META_DIR)
        .join(COMPANIONS_SUBDIR)
        .join(skill_name)
}

/// Copy companion source files out of the repo into the central metadata area.
///
/// This is called once at install time. Later, when the user chooses to sync
/// this skill to a specific tool, `install_companions_for_tool` reads back
/// from the staged copy — no need to touch the git cache again.
///
/// `central_repo_root` is the directory that contains the per-skill folders
/// (i.e. the same root under which `<skill_name>/` lives).
/// `repo_dir` is the cached git checkout root.
pub fn stage_companions_for_skill(
    central_repo_root: &Path,
    skill_name: &str,
    repo_dir: &Path,
    manifest: &MultiHostDistManifest,
) -> Result<Vec<StagedCompanion>> {
    if manifest.companion_files.is_empty() {
        return Ok(Vec::new());
    }

    let root = companions_root_for_skill(central_repo_root, skill_name);
    // Wipe any prior staging for this skill so removals in the upstream repo
    // are reflected here.
    if root.exists() {
        std::fs::remove_dir_all(&root)
            .with_context(|| format!("clear staged companions {:?}", root))?;
    }

    let mut staged = Vec::with_capacity(manifest.companion_files.len());
    for c in &manifest.companion_files {
        let src = repo_dir.join(&c.source_rel);
        if !src.is_file() {
            // Manifest can outlive the on-disk state (e.g. cache TTL mid-flight).
            // Skip gracefully rather than fail the whole install.
            log::warn!(
                "[companions] source not found, skipping: {:?} (tool={})",
                src,
                c.tool_key
            );
            continue;
        }

        // Staged layout mirrors the target_rel path under the tool_key dir.
        let staged_path = root.join(&c.tool_key).join(&c.target_rel);
        if let Some(parent) = staged_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create staging dir {:?}", parent))?;
        }
        std::fs::copy(&src, &staged_path)
            .with_context(|| format!("stage companion {:?} -> {:?}", src, staged_path))?;

        staged.push(StagedCompanion {
            tool_key: c.tool_key.clone(),
            staged_path,
            target_rel: c.target_rel.clone(),
        });
    }

    Ok(staged)
}

/// List staged companions previously written by `stage_companions_for_skill`.
///
/// Reads from the central metadata area. If the skill has no staged
/// companions, returns an empty list.
pub fn list_staged_companions(
    central_repo_root: &Path,
    skill_name: &str,
) -> Result<Vec<StagedCompanion>> {
    let root = companions_root_for_skill(central_repo_root, skill_name);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    // The staged layout is `<root>/<tool_key>/<target_rel...>`. Walk it and
    // reconstruct entries.
    for tool_entry in std::fs::read_dir(&root)
        .with_context(|| format!("read staged companions {:?}", root))?
        .flatten()
    {
        let tool_dir = tool_entry.path();
        if !tool_dir.is_dir() {
            continue;
        }
        let tool_key = tool_entry.file_name().to_string_lossy().to_string();

        for file_path in walk_files(&tool_dir) {
            let target_rel = file_path
                .strip_prefix(&tool_dir)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if target_rel.is_empty() {
                continue;
            }
            out.push(StagedCompanion {
                tool_key: tool_key.clone(),
                staged_path: file_path,
                target_rel,
            });
        }
    }
    Ok(out)
}

/// Install (i.e. copy from central staging to the tool's home) all companions
/// staged for the given `tool_key`, and record each installed file in the DB.
///
/// Existing files at the target locations are overwritten (with a `.bak.<ts>`
/// backup if they were not previously installed by Skills Hub). Callers should
/// pass an absolute `tool_home` — typically the user's home dir.
pub fn install_companions_for_tool(
    store: &SkillStore,
    skill_id: &str,
    skill_name: &str,
    tool_key: &str,
    central_repo_root: &Path,
    tool_home: &Path,
) -> Result<Vec<PathBuf>> {
    let staged = list_staged_companions(central_repo_root, skill_name)?;
    let mut installed = Vec::new();

    // Snapshot of what Skills Hub thinks it previously installed for this
    // (skill, tool). Anything we're about to write to that Skills Hub already
    // wrote before can be overwritten silently.
    let previous = store.list_skill_companions_for_tool(skill_id, tool_key)?;
    let previous_paths: std::collections::HashSet<String> =
        previous.iter().map(|c| c.target_path.clone()).collect();

    for entry in staged.iter().filter(|e| e.tool_key == tool_key) {
        let target = tool_home.join(&entry.target_rel);

        if target.exists() && !previous_paths.contains(target.to_string_lossy().as_ref()) {
            // Belongs to the user or another tool — back it up before overwrite.
            // Use ms-since-epoch as a simple, monotonic-enough suffix.
            let ts = crate::core::skill_store::now_ms();
            let backup = target.with_extension(format!(
                "{}.bak.{}",
                target
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_default(),
                ts
            ));
            let _ = std::fs::rename(&target, &backup);
            log::info!(
                "[companions] backed up existing file: {:?} -> {:?}",
                target,
                backup
            );
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create target dir {:?}", parent))?;
        }
        std::fs::copy(&entry.staged_path, &target).with_context(|| {
            format!("install companion {:?} -> {:?}", entry.staged_path, target)
        })?;

        store.upsert_skill_companion(&SkillCompanionRecord {
            id: Uuid::new_v4().to_string(),
            skill_id: skill_id.to_string(),
            tool_key: tool_key.to_string(),
            target_path: target.to_string_lossy().to_string(),
            installed_at: crate::core::skill_store::now_ms(),
        })?;

        installed.push(target);
    }

    Ok(installed)
}

/// Uninstall (remove from disk + DB) all companion files that Skills Hub
/// previously installed for the given (skill, tool) pair.
pub fn uninstall_companions_for_tool(
    store: &SkillStore,
    skill_id: &str,
    tool_key: &str,
) -> Result<Vec<PathBuf>> {
    let records = store.list_skill_companions_for_tool(skill_id, tool_key)?;
    let mut removed = Vec::with_capacity(records.len());
    for r in &records {
        let p = PathBuf::from(&r.target_path);
        if p.exists() {
            if let Err(err) = std::fs::remove_file(&p) {
                log::warn!(
                    "[companions] failed to remove {:?}: {} (continuing)",
                    p,
                    err
                );
            } else {
                removed.push(p);
            }
        }
    }
    store.delete_skill_companions_for_tool(skill_id, tool_key)?;
    Ok(removed)
}

/// Cleanup everything Skills Hub owns for this skill: on-disk staging + all
/// installed companion files + DB rows. Used when the skill is deleted.
pub fn cleanup_all_companions(
    store: &SkillStore,
    skill_id: &str,
    skill_name: &str,
    central_repo_root: &Path,
) -> Result<()> {
    // 1. Remove all installed files, per record.
    let records = store.list_skill_companions(skill_id)?;
    for r in &records {
        let p = PathBuf::from(&r.target_path);
        if p.exists() {
            if let Err(err) = std::fs::remove_file(&p) {
                log::warn!(
                    "[companions] failed to remove {:?}: {} (continuing)",
                    p,
                    err
                );
            }
        }
    }
    // 2. Purge DB rows.
    store.delete_all_skill_companions(skill_id)?;
    // 3. Purge staging area.
    let root = companions_root_for_skill(central_repo_root, skill_name);
    if root.exists() {
        let _ = std::fs::remove_dir_all(&root);
    }
    Ok(())
}

/// Convenience: walk a directory returning file paths only (no dirs).
fn walk_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(p: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(rd) = std::fs::read_dir(p) {
            for entry in rd.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.is_file() {
                    out.push(path);
                }
            }
        }
    }
    walk(dir, &mut out);
    out
}

// ------------------ Convenience conversion ------------------

impl From<&CompanionFile> for StagedCompanion {
    fn from(c: &CompanionFile) -> Self {
        StagedCompanion {
            tool_key: c.tool_key.clone(),
            staged_path: PathBuf::new(),
            target_rel: c.target_rel.clone(),
        }
    }
}

#[cfg(test)]
#[path = "tests/companions.rs"]
mod tests;
