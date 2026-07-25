use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use walkdir::WalkDir;

use super::mcp::{validate_mcp_server_input, McpServerInput, McpTransport};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

pub fn scan_mcp_config_files(repo_dir: &std::path::Path) -> Result<Vec<McpImportCandidate>> {
    let mut candidates = Vec::new();
    for entry in WalkDir::new(repo_dir)
        .max_depth(5)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file()
            || !is_mcp_config_name(entry.file_name().to_string_lossy().as_ref())
        {
            continue;
        }
        let contents = std::fs::read_to_string(entry.path())
            .with_context(|| format!("read MCP configuration {:?}", entry.path()))?;
        let source_path = entry
            .path()
            .strip_prefix(repo_dir)
            .unwrap_or(entry.path())
            .to_string_lossy();
        match parse_mcp_config(&source_path, &contents) {
            Ok(mut found) => candidates.append(&mut found),
            Err(error) => log::debug!("skip MCP config {}: {error:#}", source_path),
        }
    }
    if candidates.is_empty() {
        anyhow::bail!("no importable MCP configuration found in repository")
    }
    candidates.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.source_path.cmp(&right.source_path))
    });
    Ok(candidates)
}

fn is_mcp_config_name(name: &str) -> bool {
    matches!(
        name,
        "mcp.json" | ".mcp.json" | "claude_desktop_config.json" | "mcp.config.json"
    )
}

pub fn parse_mcp_config(source_path: &str, contents: &str) -> Result<Vec<McpImportCandidate>> {
    match serde_json::from_str(contents) {
        Ok(document) => parse_json_config(source_path, document),
        Err(_) => parse_toml_config(source_path, contents),
    }
}

fn parse_json_config(source_path: &str, document: Value) -> Result<Vec<McpImportCandidate>> {
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

fn parse_toml_config(source_path: &str, contents: &str) -> Result<Vec<McpImportCandidate>> {
    let document = contents
        .parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("parse MCP TOML configuration {source_path}"))?;
    let servers = document["mcp_servers"]
        .as_table()
        .context("MCP TOML configuration is missing mcp_servers")?;
    let mut candidates = servers
        .iter()
        .map(|(name, item)| {
            parse_toml_server(
                source_path,
                name,
                item.as_table().context("MCP TOML server must be a table")?,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    candidates.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(candidates)
}

fn parse_toml_server(
    source_path: &str,
    name: &str,
    server: &toml_edit::Table,
) -> Result<McpImportCandidate> {
    let command = server
        .get("command")
        .and_then(toml_edit::Item::as_str)
        .map(str::to_string);
    let url = server
        .get("url")
        .and_then(toml_edit::Item::as_str)
        .map(str::to_string);
    let transport = if url.is_some() {
        McpTransport::Http
    } else {
        McpTransport::Stdio
    };
    let args = server
        .get("args")
        .and_then(toml_edit::Item::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(str::to_string)
                        .context("MCP TOML args must be strings")
                })
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    let env = toml_string_map(server.get("env"))?;
    let headers = toml_string_map(server.get("headers"))?;
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

fn toml_string_map(item: Option<&toml_edit::Item>) -> Result<BTreeMap<String, String>> {
    let Some(item) = item else {
        return Ok(BTreeMap::new());
    };
    item.as_table_like()
        .context("MCP TOML env and headers must be tables")?
        .iter()
        .map(|(key, value)| {
            Ok((
                key.to_string(),
                value
                    .as_str()
                    .context("MCP TOML values must be strings")?
                    .to_string(),
            ))
        })
        .collect()
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
