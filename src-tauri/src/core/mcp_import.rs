use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde_json::Value;

use super::mcp::{validate_mcp_server_input, McpServerInput, McpTransport};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpImportCandidate {
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub url: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub source_path: String,
}

pub fn parse_mcp_config(source_path: &str, contents: &str) -> Result<Vec<McpImportCandidate>> {
    let document: Value = serde_json::from_str(contents)
        .with_context(|| format!("parse MCP JSON configuration {source_path}"))?;
    let root = document
        .as_object()
        .context("MCP configuration must be an object")?;
    let servers = root
        .get("mcpServers")
        .or_else(|| root.get("mcp_servers"))
        .and_then(Value::as_object)
        .context("MCP configuration is missing mcpServers")?;
    let mut candidates = servers
        .iter()
        .map(|(name, value)| parse_server(source_path, name, value))
        .collect::<Result<Vec<_>>>()?;
    candidates.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(candidates)
}

fn parse_server(source_path: &str, name: &str, value: &Value) -> Result<McpImportCandidate> {
    let server = value
        .as_object()
        .context("MCP server entry must be an object")?;
    let command = server
        .get("command")
        .and_then(Value::as_str)
        .map(str::to_string);
    let url = server
        .get("url")
        .and_then(Value::as_str)
        .map(str::to_string);
    let transport = if url.is_some() {
        McpTransport::Http
    } else {
        McpTransport::Stdio
    };
    let args = server
        .get("args")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(str::to_string)
                        .context("MCP args must be strings")
                })
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    let env = string_map(server.get("env"), "MCP environment values must be strings")?;
    let headers = string_map(server.get("headers"), "MCP header values must be strings")?;
    let input = McpServerInput {
        name: name.to_string(),
        transport: transport.clone(),
        command: command.clone(),
        args: args.clone(),
        env: env.clone(),
        url: url.clone(),
        headers: headers.clone(),
    };
    validate_mcp_server_input(&input)?;
    Ok(McpImportCandidate {
        name: name.to_string(),
        transport: match transport {
            McpTransport::Stdio => "stdio",
            McpTransport::Http => "http",
        }
        .to_string(),
        command,
        args,
        env,
        url,
        headers,
        source_path: source_path.to_string(),
    })
}

fn string_map(value: Option<&Value>, error: &str) -> Result<BTreeMap<String, String>> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    value
        .as_object()
        .context(error.to_string())?
        .iter()
        .map(|(key, value)| {
            Ok((
                key.clone(),
                value.as_str().context(error.to_string())?.to_string(),
            ))
        })
        .collect()
}
