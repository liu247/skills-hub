use anyhow::{Context, Result};
use serde_json::{json, Map, Value};

use super::skill_store::McpServerRecord;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpHost {
    Codex,
    ClaudeCode,
    Kiro,
    Reasonix,
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
