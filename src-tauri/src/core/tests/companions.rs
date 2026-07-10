use std::fs;
use std::path::Path;

use crate::core::companions::{
    cleanup_all_companions, install_companions_for_tool, list_staged_companions,
    stage_companions_for_skill, uninstall_companions_for_tool,
};
use crate::core::installer::{CompanionFile, MultiHostDistManifest};
use crate::core::skill_store::SkillStore;

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().unwrap();
    (dir, store)
}

fn write(p: &Path, content: &str) {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, content).unwrap();
}

fn fake_paperspine_repo(root: &Path) {
    // Skill folder in canonical location.
    write(
        &root.join("dist/claude/skills/paper-spine/SKILL.md"),
        "---\nname: paper-spine\n---\n",
    );
    // Two companion files (one per host).
    write(
        &root.join("dist/claude/commands/paperspine.md"),
        "# /paperspine — claude",
    );
    write(
        &root.join("dist/codex/prompts/paperspine.md"),
        "# paperspine — codex",
    );
}

fn manifest() -> MultiHostDistManifest {
    MultiHostDistManifest {
        canonical_source_rel: "dist/claude/skills/paper-spine".to_string(),
        skill_name: "paper-spine".to_string(),
        companion_files: vec![
            CompanionFile {
                source_rel: "dist/claude/commands/paperspine.md".to_string(),
                tool_key: "claude_code".to_string(),
                target_rel: ".claude/commands/paperspine.md".to_string(),
            },
            CompanionFile {
                source_rel: "dist/codex/prompts/paperspine.md".to_string(),
                tool_key: "codex".to_string(),
                target_rel: ".codex/prompts/paperspine.md".to_string(),
            },
        ],
    }
}

#[test]
fn stage_companions_writes_metadata_and_survives_relist() {
    let repo = tempfile::tempdir().unwrap();
    fake_paperspine_repo(repo.path());
    let central = tempfile::tempdir().unwrap();

    let staged =
        stage_companions_for_skill(central.path(), "paper-spine", repo.path(), &manifest())
            .unwrap();

    assert_eq!(staged.len(), 2);
    assert!(staged
        .iter()
        .any(|s| s.tool_key == "claude_code" && s.target_rel == ".claude/commands/paperspine.md"));
    assert!(staged
        .iter()
        .any(|s| s.tool_key == "codex" && s.target_rel == ".codex/prompts/paperspine.md"));

    // Files actually exist under central metadata dir.
    let meta = central
        .path()
        .join(".skillshub-meta")
        .join("companions")
        .join("paper-spine");
    assert!(meta
        .join("claude_code/.claude/commands/paperspine.md")
        .is_file());
    assert!(meta.join("codex/.codex/prompts/paperspine.md").is_file());

    // list_staged reads them back.
    let relisted = list_staged_companions(central.path(), "paper-spine").unwrap();
    assert_eq!(relisted.len(), 2);
}

#[test]
fn install_companions_copies_only_matching_tool_and_records() {
    let (_dir, store) = make_store();

    // Simulate skill row present.
    use crate::core::skill_store::SkillRecord;
    store
        .upsert_skill(&SkillRecord {
            id: "s1".to_string(),
            name: "paper-spine".to_string(),
            description: None,
            source_type: "git".to_string(),
            source_ref: Some("https://x".to_string()),
            source_subpath: None,
            source_revision: None,
            central_path: "/tmp/central/paper-spine".to_string(),
            content_hash: None,
            created_at: 1,
            updated_at: 1,
            last_sync_at: None,
            last_seen_at: 1,
            enabled: true,
            status: "ok".to_string(),
            collection: Some("PaperSpine".to_string()),
        })
        .unwrap();

    let repo = tempfile::tempdir().unwrap();
    fake_paperspine_repo(repo.path());
    let central = tempfile::tempdir().unwrap();
    stage_companions_for_skill(central.path(), "paper-spine", repo.path(), &manifest()).unwrap();

    let tool_home = tempfile::tempdir().unwrap();

    // Install for claude_code only.
    let installed = install_companions_for_tool(
        &store,
        "s1",
        "paper-spine",
        "claude_code",
        central.path(),
        tool_home.path(),
    )
    .unwrap();
    assert_eq!(installed.len(), 1);
    assert!(tool_home
        .path()
        .join(".claude/commands/paperspine.md")
        .is_file());
    // Codex path was NOT touched.
    assert!(!tool_home
        .path()
        .join(".codex/prompts/paperspine.md")
        .exists());

    let db_rows = store.list_skill_companions("s1").unwrap();
    assert_eq!(db_rows.len(), 1);
    assert_eq!(db_rows[0].tool_key, "claude_code");
    assert!(db_rows[0]
        .target_path
        .ends_with(".claude/commands/paperspine.md"));
}

#[test]
fn uninstall_companions_removes_files_and_db_rows() {
    let (_dir, store) = make_store();

    use crate::core::skill_store::SkillRecord;
    store
        .upsert_skill(&SkillRecord {
            id: "s2".to_string(),
            name: "paper-spine".to_string(),
            description: None,
            source_type: "git".to_string(),
            source_ref: Some("https://x".to_string()),
            source_subpath: None,
            source_revision: None,
            central_path: "/tmp/central/paper-spine".to_string(),
            content_hash: None,
            created_at: 1,
            updated_at: 1,
            last_sync_at: None,
            last_seen_at: 1,
            enabled: true,
            status: "ok".to_string(),
            collection: None,
        })
        .unwrap();

    let repo = tempfile::tempdir().unwrap();
    fake_paperspine_repo(repo.path());
    let central = tempfile::tempdir().unwrap();
    stage_companions_for_skill(central.path(), "paper-spine", repo.path(), &manifest()).unwrap();

    let tool_home = tempfile::tempdir().unwrap();
    install_companions_for_tool(
        &store,
        "s2",
        "paper-spine",
        "claude_code",
        central.path(),
        tool_home.path(),
    )
    .unwrap();
    install_companions_for_tool(
        &store,
        "s2",
        "paper-spine",
        "codex",
        central.path(),
        tool_home.path(),
    )
    .unwrap();

    assert_eq!(store.list_skill_companions("s2").unwrap().len(), 2);

    // Uninstall codex only.
    let removed = uninstall_companions_for_tool(&store, "s2", "codex").unwrap();
    assert_eq!(removed.len(), 1);
    assert!(!tool_home
        .path()
        .join(".codex/prompts/paperspine.md")
        .exists());
    // Claude still there.
    assert!(tool_home
        .path()
        .join(".claude/commands/paperspine.md")
        .is_file());

    let remaining = store.list_skill_companions("s2").unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].tool_key, "claude_code");
}

#[test]
fn cleanup_all_wipes_files_db_and_central_staging() {
    let (_dir, store) = make_store();

    use crate::core::skill_store::SkillRecord;
    store
        .upsert_skill(&SkillRecord {
            id: "s3".to_string(),
            name: "paper-spine".to_string(),
            description: None,
            source_type: "git".to_string(),
            source_ref: Some("https://x".to_string()),
            source_subpath: None,
            source_revision: None,
            central_path: "/tmp/central/paper-spine".to_string(),
            content_hash: None,
            created_at: 1,
            updated_at: 1,
            last_sync_at: None,
            last_seen_at: 1,
            enabled: true,
            status: "ok".to_string(),
            collection: None,
        })
        .unwrap();

    let repo = tempfile::tempdir().unwrap();
    fake_paperspine_repo(repo.path());
    let central = tempfile::tempdir().unwrap();
    stage_companions_for_skill(central.path(), "paper-spine", repo.path(), &manifest()).unwrap();

    let tool_home = tempfile::tempdir().unwrap();
    install_companions_for_tool(
        &store,
        "s3",
        "paper-spine",
        "claude_code",
        central.path(),
        tool_home.path(),
    )
    .unwrap();

    cleanup_all_companions(&store, "s3", "paper-spine", central.path()).unwrap();

    assert!(store.list_skill_companions("s3").unwrap().is_empty());
    assert!(!tool_home
        .path()
        .join(".claude/commands/paperspine.md")
        .exists());
    assert!(!central
        .path()
        .join(".skillshub-meta/companions/paper-spine")
        .exists());
}

#[test]
fn existing_foreign_file_is_backed_up_not_clobbered() {
    let (_dir, store) = make_store();

    use crate::core::skill_store::SkillRecord;
    store
        .upsert_skill(&SkillRecord {
            id: "s4".to_string(),
            name: "paper-spine".to_string(),
            description: None,
            source_type: "git".to_string(),
            source_ref: Some("https://x".to_string()),
            source_subpath: None,
            source_revision: None,
            central_path: "/tmp/central/paper-spine".to_string(),
            content_hash: None,
            created_at: 1,
            updated_at: 1,
            last_sync_at: None,
            last_seen_at: 1,
            enabled: true,
            status: "ok".to_string(),
            collection: None,
        })
        .unwrap();

    let repo = tempfile::tempdir().unwrap();
    fake_paperspine_repo(repo.path());
    let central = tempfile::tempdir().unwrap();
    stage_companions_for_skill(central.path(), "paper-spine", repo.path(), &manifest()).unwrap();

    let tool_home = tempfile::tempdir().unwrap();
    // User already has a file at the target.
    write(
        &tool_home.path().join(".claude/commands/paperspine.md"),
        "USER CONTENT",
    );

    install_companions_for_tool(
        &store,
        "s4",
        "paper-spine",
        "claude_code",
        central.path(),
        tool_home.path(),
    )
    .unwrap();

    // Target now has our content.
    let installed_body =
        fs::read_to_string(tool_home.path().join(".claude/commands/paperspine.md")).unwrap();
    assert!(installed_body.contains("/paperspine — claude"));

    // A backup with `.bak.<ts>` suffix exists in the same dir.
    let dir = tool_home.path().join(".claude/commands");
    let backup_exists = fs::read_dir(&dir).unwrap().any(|e| {
        e.ok()
            .map(|e| e.file_name().to_string_lossy().contains(".bak."))
            .unwrap_or(false)
    });
    assert!(backup_exists, "expected a .bak.<ts> file in {:?}", dir);
}
