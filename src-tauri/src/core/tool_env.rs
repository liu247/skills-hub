//! Tool-level global environment injection.
//!
//! The industry-standard way to give skill scripts secrets is to put them in
//! the agent tool's *global* environment configuration, which the tool loads
//! into every session process (e.g. Claude Code's `settings.json` `env` map).
//! Skill scripts then read them via `os.environ`/`getenv` with no `.env`
//! layout assumptions.
//!
//! This module knows how to read/merge/remove a managed key set in each
//! supported tool's global env configuration. Unsupported tools are skipped;
//! the legacy per-skill `.env` write remains as a fallback.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// How a tool stores its global environment variables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvConfigFormat {
    /// `~/.claude/settings.json` → top-level `env` object.
    JsonEnvMap,
    /// `~/.codex/config.toml` → `[shell_environment_policy] set = { ... }`.
    CodexShellPolicy,
    /// `~/.reasonix/.env` → `KEY=VALUE` lines, other content preserved.
    DotEnvFile,
}

/// A tool whose global env configuration Skills Hub manages.
#[derive(Clone, Debug)]
pub struct ToolGlobalEnvConfig {
    pub tool_key: &'static str,
    pub relative_config_path: &'static str,
    pub format: EnvConfigFormat,
}

pub fn global_env_config_for(tool_key: &str) -> Option<ToolGlobalEnvConfig> {
    match tool_key {
        "claude_code" => Some(ToolGlobalEnvConfig {
            tool_key: "claude_code",
            relative_config_path: ".claude/settings.json",
            format: EnvConfigFormat::JsonEnvMap,
        }),
        "codex" => Some(ToolGlobalEnvConfig {
            tool_key: "codex",
            relative_config_path: ".codex/config.toml",
            format: EnvConfigFormat::CodexShellPolicy,
        }),
        "reasonix" => Some(ToolGlobalEnvConfig {
            tool_key: "reasonix",
            relative_config_path: ".reasonix/.env",
            format: EnvConfigFormat::DotEnvFile,
        }),
        _ => None,
    }
}

fn resolve_config_path(config: &ToolGlobalEnvConfig) -> Result<PathBuf> {
    let home = dirs::home_dir().context("failed to resolve home directory")?;
    Ok(home.join(config.relative_config_path))
}

/// Reads the tool's current global env map (including keys not managed by
/// Skills Hub). Returns an empty map when the config file does not exist.
pub fn read_global_env(config: &ToolGlobalEnvConfig) -> Result<BTreeMap<String, String>> {
    read_global_env_at(&resolve_config_path(config)?, &config.format)
}

pub fn read_global_env_at(
    path: &Path,
    format: &EnvConfigFormat,
) -> Result<BTreeMap<String, String>> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    match format {
        EnvConfigFormat::JsonEnvMap => {
            let text = std::fs::read_to_string(path).with_context(|| format!("read {:?}", path))?;
            let value: serde_json::Value =
                serde_json::from_str(&text).with_context(|| format!("parse JSON {:?}", path))?;
            Ok(value
                .get("env")
                .and_then(serde_json::Value::as_object)
                .map(|env| {
                    env.iter()
                        .filter_map(|(key, value)| {
                            value
                                .as_str()
                                .map(|string| (key.clone(), string.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default())
        }
        EnvConfigFormat::CodexShellPolicy => {
            let text = std::fs::read_to_string(path).with_context(|| format!("read {:?}", path))?;
            let doc = text
                .parse::<toml_edit::DocumentMut>()
                .with_context(|| format!("parse TOML {:?}", path))?;
            let mut map = BTreeMap::new();
            if let Some(set) = doc
                .get("shell_environment_policy")
                .and_then(|table| table.get("set"))
                .and_then(toml_edit::Item::as_table_like)
            {
                for (key, value) in set.iter() {
                    if let Some(string) = value.as_str() {
                        map.insert(key.to_string(), string.to_string());
                    }
                }
            }
            Ok(map)
        }
        EnvConfigFormat::DotEnvFile => {
            let text = std::fs::read_to_string(path).with_context(|| format!("read {:?}", path))?;
            Ok(parse_dotenv_lines(&text))
        }
    }
}

fn parse_dotenv_lines(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            if !key.is_empty() {
                map.insert(key.to_string(), value.trim().to_string());
            }
        }
    }
    map
}

/// Merges `managed` keys into the tool's global env config, preserving all
/// other keys and file content. Returns the keys that were written.
pub fn write_global_env(
    config: &ToolGlobalEnvConfig,
    managed: &BTreeMap<String, String>,
) -> Result<Vec<String>> {
    write_global_env_at(&resolve_config_path(config)?, &config.format, managed)
}

pub fn write_global_env_at(
    path: &Path,
    format: &EnvConfigFormat,
    managed: &BTreeMap<String, String>,
) -> Result<Vec<String>> {
    match format {
        EnvConfigFormat::JsonEnvMap => write_json_env_map(path, managed),
        EnvConfigFormat::CodexShellPolicy => write_codex_policy(path, managed),
        EnvConfigFormat::DotEnvFile => write_dotenv_file(path, managed),
    }
}

fn write_json_env_map(path: &Path, managed: &BTreeMap<String, String>) -> Result<Vec<String>> {
    let mut root: serde_json::Value = if path.exists() {
        let text = std::fs::read_to_string(path).with_context(|| format!("read {:?}", path))?;
        serde_json::from_str(&text)
            .unwrap_or_else(|_| serde_json::Value::Object(Default::default()))
    } else {
        serde_json::Value::Object(Default::default())
    };
    let env = root
        .as_object_mut()
        .context("settings file must be a JSON object")?
        .entry("env")
        .or_insert_with(|| serde_json::Value::Object(Default::default()));
    let env_obj = env
        .as_object_mut()
        .context("\"env\" must be a JSON object")?;
    let mut written = Vec::new();
    for (key, value) in managed {
        env_obj.insert(key.clone(), serde_json::Value::String(value.clone()));
        written.push(key.clone());
    }
    let next = serde_json::to_string_pretty(&root).context("serialize settings JSON")?;
    write_config_file(path, next.as_bytes())?;
    Ok(written)
}

fn write_codex_policy(path: &Path, managed: &BTreeMap<String, String>) -> Result<Vec<String>> {
    let text = if path.exists() {
        std::fs::read_to_string(path).with_context(|| format!("read {:?}", path))?
    } else {
        String::new()
    };
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("parse TOML {:?}", path))?;
    if !doc.contains_key("shell_environment_policy") {
        doc["shell_environment_policy"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let policy = doc
        .get_mut("shell_environment_policy")
        .and_then(toml_edit::Item::as_table_mut)
        .context("shell_environment_policy must be a table")?;
    if !policy.contains_key("set") {
        policy["set"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let set = policy
        .get_mut("set")
        .and_then(toml_edit::Item::as_table_mut)
        .context("set must be a table")?;
    let mut written = Vec::new();
    for (key, value) in managed {
        set.insert(key, toml_edit::value(value));
        written.push(key.clone());
    }
    let next = doc.to_string();
    write_config_file(path, next.as_bytes())?;
    Ok(written)
}

fn write_dotenv_file(path: &Path, managed: &BTreeMap<String, String>) -> Result<Vec<String>> {
    let original = if path.exists() {
        std::fs::read_to_string(path).with_context(|| format!("read {:?}", path))?
    } else {
        String::new()
    };
    let mut lines: Vec<String> = original.lines().map(str::to_string).collect();
    let mut updated = std::collections::HashSet::new();
    for line in &mut lines {
        let key = line
            .trim()
            .split_once('=')
            .map(|(key, _)| key.trim().to_string());
        if let Some(key) = key {
            if let Some(value) = managed.get(&key) {
                *line = format!("{key}={value}");
                updated.insert(key);
            }
        }
    }
    let mut written = Vec::new();
    for (key, value) in managed {
        if !updated.contains(key) {
            lines.push(format!("{key}={value}"));
        }
        written.push(key.clone());
    }
    let mut next = lines.join("\n");
    if !next.is_empty() {
        next.push('\n');
    }
    write_config_file(path, next.as_bytes())?;
    Ok(written)
}

/// Removes `keys` from the tool's global env config, preserving everything
/// else. Missing keys are ignored.
pub fn remove_global_env(config: &ToolGlobalEnvConfig, keys: &[String]) -> Result<()> {
    remove_global_env_at(&resolve_config_path(config)?, &config.format, keys)
}

pub fn remove_global_env_at(path: &Path, format: &EnvConfigFormat, keys: &[String]) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    match format {
        EnvConfigFormat::JsonEnvMap => {
            let text = std::fs::read_to_string(path).with_context(|| format!("read {:?}", path))?;
            let mut root: serde_json::Value =
                serde_json::from_str(&text).with_context(|| format!("parse JSON {:?}", path))?;
            if let Some(env) = root
                .get_mut("env")
                .and_then(serde_json::Value::as_object_mut)
            {
                for key in keys {
                    env.remove(key);
                }
            }
            let next = serde_json::to_string_pretty(&root).context("serialize settings JSON")?;
            write_config_file(path, next.as_bytes())?;
        }
        EnvConfigFormat::CodexShellPolicy => {
            let text = std::fs::read_to_string(path).with_context(|| format!("read {:?}", path))?;
            let mut doc: toml_edit::DocumentMut = text
                .parse()
                .with_context(|| format!("parse TOML {:?}", path))?;
            if let Some(set) = doc
                .get_mut("shell_environment_policy")
                .and_then(|table| table.get_mut("set"))
                .and_then(toml_edit::Item::as_table_mut)
            {
                for key in keys {
                    set.remove(key);
                }
            }
            let next = doc.to_string();
            write_config_file(path, next.as_bytes())?;
        }
        EnvConfigFormat::DotEnvFile => {
            let text = std::fs::read_to_string(path).with_context(|| format!("read {:?}", path))?;
            let kept: Vec<String> = text
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    !trimmed
                        .split_once('=')
                        .map(|(key, _)| keys.iter().any(|k| k == key.trim()))
                        .unwrap_or(false)
                })
                .map(str::to_string)
                .collect();
            let mut next = kept.join("\n");
            if !next.is_empty() {
                next.push('\n');
            }
            write_config_file(path, next.as_bytes())?;
        }
    }
    Ok(())
}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn write_config_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create dir {:?}", parent))?;
    }
    std::fs::write(path, bytes).with_context(|| format!("write {:?}", path))?;
    #[cfg(unix)]
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}
