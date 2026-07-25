use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use super::skill_store::McpServerRecord;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpHost {
    Codex,
    ClaudeCode,
    Kiro,
    Reasonix,
}

#[derive(Clone, Debug)]
pub struct McpSyncOutcome {
    pub backup_path: Option<PathBuf>,
}

pub fn render_server(
    host: McpHost,
    server: &McpServerRecord,
    proxy_port: Option<u16>,
) -> Result<String> {
    let requires_bridge = !server.env.is_empty() || !server.headers.is_empty();
    let value = match server.transport.as_str() {
        "stdio" => render_stdio(server, requires_bridge)?,
        "http" => render_http(server, requires_bridge, proxy_port)?,
        _ => anyhow::bail!("unsupported MCP transport {}", server.transport),
    };
    match host {
        McpHost::Codex => render_codex(server, &value),
        McpHost::Reasonix => render_reasonix(server, &value),
        McpHost::ClaudeCode | McpHost::Kiro => Ok(serde_json::to_string_pretty(&json!({
            "mcpServers": { server.name.clone(): value }
        }))?),
    }
}

pub fn merge_json_host_config(
    existing: &str,
    rendered: &str,
    server_name: &str,
    owns_existing_entry: bool,
) -> Result<String> {
    let mut document = if existing.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str::<Value>(existing).context("parse existing MCP JSON configuration")?
    };
    let replacement =
        serde_json::from_str::<Value>(rendered).context("parse rendered MCP JSON configuration")?;
    let root = document
        .as_object_mut()
        .context("MCP JSON configuration must be an object")?;
    let replacement_servers = replacement
        .get("mcpServers")
        .and_then(Value::as_object)
        .context("rendered MCP JSON must contain mcpServers")?;
    let replacement_entry = replacement_servers
        .get(server_name)
        .context("rendered MCP JSON is missing server entry")?
        .clone();
    let servers = root
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .context("mcpServers must be an object")?;
    if servers.contains_key(server_name) && !owns_existing_entry {
        anyhow::bail!("MCP server name is already used by an unmanaged host entry");
    }
    servers.insert(server_name.to_string(), replacement_entry);
    Ok(serde_json::to_string_pretty(&document)?)
}

pub fn sync_host_file(
    host: McpHost,
    server: &McpServerRecord,
    path: &Path,
    owns_existing_entry: bool,
    proxy_port: Option<u16>,
) -> Result<McpSyncOutcome> {
    let existing = if path.exists() {
        std::fs::read_to_string(path).context("read existing MCP configuration")?
    } else {
        String::new()
    };
    let rendered = render_server(host, server, proxy_port)?;
    let next = match host {
        McpHost::ClaudeCode | McpHost::Kiro => {
            merge_json_host_config(&existing, &rendered, &server.name, owns_existing_entry)?
        }
        McpHost::Codex => {
            merge_codex_toml_config(&existing, &rendered, &server.name, owns_existing_entry)?
        }
        McpHost::Reasonix => {
            merge_reasonix_toml_config(&existing, &rendered, &server.name, owns_existing_entry)?
        }
    };
    Ok(McpSyncOutcome {
        backup_path: write_config_atomically(path, &next)?,
    })
}

pub fn merge_codex_toml_config(
    existing: &str,
    rendered: &str,
    server_name: &str,
    owns_existing_entry: bool,
) -> Result<String> {
    let mut document = existing
        .parse::<toml_edit::DocumentMut>()
        .context("parse existing Codex TOML configuration")?;
    let replacement = rendered
        .parse::<toml_edit::DocumentMut>()
        .context("parse rendered Codex TOML configuration")?;
    let replacement_entry = replacement["mcp_servers"][server_name]
        .as_table()
        .context("rendered Codex configuration is missing server table")?
        .clone();

    let root = document["mcp_servers"].or_insert(toml_edit::table());
    let servers = root
        .as_table_mut()
        .context("mcp_servers must be a TOML table")?;
    if servers.contains_key(server_name) && !owns_existing_entry {
        anyhow::bail!("MCP server name is already used by an unmanaged host entry");
    }
    servers.insert(server_name, toml_edit::Item::Table(replacement_entry));
    Ok(document.to_string())
}

pub fn merge_reasonix_toml_config(
    existing: &str,
    rendered: &str,
    server_name: &str,
    owns_existing_entry: bool,
) -> Result<String> {
    let mut document = existing
        .parse::<toml_edit::DocumentMut>()
        .context("parse existing Reasonix TOML configuration")?;
    let replacement = rendered
        .parse::<toml_edit::DocumentMut>()
        .context("parse rendered Reasonix TOML configuration")?;
    let replacement_table = replacement["plugins"]
        .as_array_of_tables()
        .and_then(|tables| tables.get(0))
        .context("rendered Reasonix configuration is missing plugin table")?
        .clone();
    let plugins = document["plugins"].or_insert(toml_edit::array());
    let tables = plugins
        .as_array_of_tables_mut()
        .context("plugins must be an array of tables")?;
    let matching_index = tables
        .iter()
        .enumerate()
        .find_map(|(index, table)| (table["name"].as_str() == Some(server_name)).then_some(index));
    if let Some(index) = matching_index {
        if !owns_existing_entry {
            anyhow::bail!("MCP server name is already used by an unmanaged host entry");
        }
        *tables
            .get_mut(index)
            .context("missing existing Reasonix plugin table")? = replacement_table;
    } else {
        tables.push(replacement_table);
    }
    Ok(document.to_string())
}

pub fn write_config_atomically(path: &Path, contents: &str) -> Result<Option<PathBuf>> {
    let parent = path
        .parent()
        .context("MCP configuration path has no parent directory")?;
    std::fs::create_dir_all(parent).context("create MCP configuration directory")?;
    let backup = if path.exists() {
        let name = path
            .file_name()
            .context("MCP configuration path has no file name")?
            .to_string_lossy();
        let backup = parent.join(format!("{name}.skills-hub.bak-{}", now_ms()));
        std::fs::copy(path, &backup).context("backup existing MCP configuration")?;
        Some(backup)
    } else {
        None
    };
    let temp = parent.join(format!(
        ".{}.skills-hub-{}.tmp",
        path.file_name()
            .context("MCP configuration path has no file name")?
            .to_string_lossy(),
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&temp, contents).context("write temporary MCP configuration")?;
    std::fs::rename(&temp, path).context("atomically replace MCP configuration")?;
    Ok(backup)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn render_stdio(server: &McpServerRecord, requires_bridge: bool) -> Result<Value> {
    let command = server
        .command
        .as_deref()
        .context("stdio server command is required")?;
    if requires_bridge {
        let mut args = vec![
            "stdio".to_string(),
            "--server-id".to_string(),
            server.id.clone(),
            "--".to_string(),
            command.to_string(),
        ];
        args.extend(server.args.clone());
        return Ok(json!({ "command": "skills-hub-mcp-bridge", "args": args }));
    }
    Ok(json!({ "command": command, "args": server.args }))
}

fn render_http(
    server: &McpServerRecord,
    requires_bridge: bool,
    proxy_port: Option<u16>,
) -> Result<Value> {
    let url = server
        .url
        .as_deref()
        .context("HTTP server URL is required")?;
    if requires_bridge {
        let port = proxy_port.context("credential bridge port is required")?;
        return Ok(json!({ "url": format!("http://127.0.0.1:{port}/mcp") }));
    }
    Ok(json!({ "url": url }))
}

fn render_codex(server: &McpServerRecord, value: &Value) -> Result<String> {
    let mut doc = toml_edit::DocumentMut::new();
    let root = doc["mcp_servers"].or_insert(toml_edit::table());
    let table = root[&server.name].or_insert(toml_edit::table());
    let object = value
        .as_object()
        .context("rendered MCP entry must be an object")?;
    for (key, value) in object {
        table[key] = match value {
            Value::String(value) => toml_edit::value(value),
            Value::Array(values) => toml_edit::value(toml_array(&values)),
            _ => anyhow::bail!("unsupported Codex field {key}"),
        };
    }
    Ok(doc.to_string())
}

fn render_reasonix(server: &McpServerRecord, value: &Value) -> Result<String> {
    let object: &Map<String, Value> = value
        .as_object()
        .context("rendered MCP entry must be an object")?;
    let mut doc = toml_edit::DocumentMut::new();
    let plugins = doc["plugins"].or_insert(toml_edit::array());
    let array = plugins
        .as_array_of_tables_mut()
        .context("plugins must be an array of tables")?;
    let mut table = toml_edit::Table::new();
    table["name"] = toml_edit::value(&server.name);
    for (key, value) in object {
        table[key] = match value {
            Value::String(value) => toml_edit::value(value),
            Value::Array(values) => toml_edit::value(toml_array(&values)),
            _ => anyhow::bail!("unsupported Reasonix field {key}"),
        };
    }
    array.push(table);
    Ok(doc.to_string())
}

fn toml_array(values: &[Value]) -> toml_edit::Array {
    let mut array = toml_edit::Array::new();
    for value in values.iter().filter_map(Value::as_str) {
        array.push(value);
    }
    array
}
