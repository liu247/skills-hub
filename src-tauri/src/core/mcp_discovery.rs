use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

use super::mcp_adapters::McpHost;

#[derive(Clone, Debug, Serialize)]
pub struct LocalMcpVariant {
    pub host: String,
    pub path: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env: BTreeMap<String, String>,
    pub headers: BTreeMap<String, String>,
    pub credential_names: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LocalMcpGroup {
    pub name: String,
    pub variants: Vec<LocalMcpVariant>,
    pub has_conflict: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct LocalMcpPlan {
    pub total_hosts_scanned: usize,
    pub total_servers_found: usize,
    pub groups: Vec<LocalMcpGroup>,
}

pub fn scan_local_mcp_configs_in(home: &Path) -> Result<LocalMcpPlan> {
    let hosts = [
        (McpHost::Codex, "codex", home.join(".codex/config.toml")),
        (McpHost::ClaudeCode, "claude_code", home.join(".claude.json")),
        (McpHost::Kiro, "kiro", home.join(".kiro/settings/mcp.json")),
        (McpHost::Reasonix, "reasonix", home.join(".reasonix/config.toml")),
    ];
    let mut scanned = 0;
    let mut variants = Vec::new();
    for (host, key, path) in hosts {
        if !path.exists() { continue; }
        scanned += 1;
        let content = std::fs::read_to_string(&path).with_context(|| format!("read MCP configuration {path:?}"))?;
        variants.extend(parse_host_config(host, key, &path, &content)?);
    }
    let found = variants.len();
    let mut by_name: HashMap<String, Vec<LocalMcpVariant>> = HashMap::new();
    for variant in variants { by_name.entry(variant.name.clone()).or_default().push(variant); }
    let mut groups: Vec<_> = by_name.into_iter().map(|(name, variants)| {
        let distinct = variants.iter().map(fingerprint).collect::<std::collections::HashSet<_>>().len();
        LocalMcpGroup { name, variants, has_conflict: distinct > 1 }
    }).collect();
    groups.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(LocalMcpPlan { total_hosts_scanned: scanned, total_servers_found: found, groups })
}

fn parse_host_config(host: McpHost, key: &str, path: &Path, content: &str) -> Result<Vec<LocalMcpVariant>> {
    if host == McpHost::Codex {
        let document = content.parse::<toml_edit::DocumentMut>()?;
        let Some(table) = document["mcp_servers"].as_table() else { return Ok(Vec::new()) };
        return table.iter().map(|(name, item)| {
            let fields = item.as_table().context("MCP server table must be a table")?;
            let command = fields.get("command").and_then(toml_edit::Item::as_str).map(str::to_string);
            let url = fields.get("url").and_then(toml_edit::Item::as_str).map(str::to_string);
            let args = fields.get("args").and_then(toml_edit::Item::as_array).map(|values| values.iter().filter_map(|value| value.as_str().map(str::to_string)).collect()).unwrap_or_default();
            let env = fields.get("env").map(toml_map).unwrap_or_default();
            let headers = fields.get("headers").map(toml_map).unwrap_or_default();
            let credential_names = env.iter().filter_map(|(k, v)| (!is_reference(v)).then_some(k.clone())).chain(headers.iter().filter_map(|(k, v)| (!is_reference(v)).then_some(k.clone()))).collect();
            Ok(LocalMcpVariant { host: key.to_string(), path: path.display().to_string(), name: name.to_string(), transport: if url.is_some() { "http" } else { "stdio" }.to_string(), command, args, url, env: sanitize(env), headers: sanitize(headers), credential_names })
        }).collect();
    }
    if host == McpHost::Reasonix {
        let document = content.parse::<toml_edit::DocumentMut>()?;
        let Some(plugins) = document["plugins"].as_array_of_tables() else { return Ok(Vec::new()) };
        return Ok(plugins.iter().filter_map(|plugin| {
            let name = plugin["name"].as_str()?;
            let command = plugin["command"].as_str().map(str::to_string);
            let url = plugin["url"].as_str().map(str::to_string);
            let args = plugin["args"].as_array().map(|values| values.iter().filter_map(|value| value.as_str().map(str::to_string)).collect()).unwrap_or_default();
            let env = plugin.get("env").map(toml_map).unwrap_or_default();
            let headers = plugin.get("headers").map(toml_map).unwrap_or_default();
            let credential_names = env.iter().filter_map(|(k, v)| (!is_reference(v)).then_some(k.clone())).chain(headers.iter().filter_map(|(k, v)| (!is_reference(v)).then_some(k.clone()))).collect();
            Some(LocalMcpVariant { host: key.to_string(), path: path.display().to_string(), name: name.to_string(), transport: if url.is_some() { "http" } else { "stdio" }.to_string(), command, args, url, env: sanitize(env), headers: sanitize(headers), credential_names })
        }).collect());
    }
    let root = match host {
        McpHost::ClaudeCode | McpHost::Kiro => serde_json::from_str::<Value>(content)?.get("mcpServers").cloned().unwrap_or(Value::Null),
        McpHost::Codex => unreachable!(),
        McpHost::Reasonix => unreachable!(),
    };
    let Some(servers) = root.as_object() else { return Ok(Vec::new()) };
    servers.iter().map(|(name, value)| parse_variant(key, path, name, value)).collect()
}

fn toml_map(item: &toml_edit::Item) -> BTreeMap<String, String> { item.as_table().map(|table| table.iter().filter_map(|(key, value)| value.as_str().map(|value| (key.to_string(), value.to_string()))).collect()).unwrap_or_default() }

fn parse_variant(host: &str, path: &Path, name: &str, value: &Value) -> Result<LocalMcpVariant> {
    let object = value.as_object().context("MCP server definition must be an object")?;
    let command = object.get("command").and_then(Value::as_str).map(str::to_string);
    let url = object.get("url").and_then(Value::as_str).map(str::to_string);
    let transport = if url.is_some() { "http" } else { "stdio" }.to_string();
    let args = object.get("args").and_then(Value::as_array).map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
    let env = string_map(object.get("env"));
    let headers = string_map(object.get("headers"));
    let credential_names = env.iter().filter_map(|(key, value)| (!is_reference(value)).then_some(key.clone())).chain(headers.iter().filter_map(|(key, value)| (!is_reference(value)).then_some(key.clone()))).collect();
    Ok(LocalMcpVariant { host: host.to_string(), path: path.display().to_string(), name: name.to_string(), transport, command, args, url, env: sanitize(env), headers: sanitize(headers), credential_names })
}

fn string_map(value: Option<&Value>) -> BTreeMap<String, String> { value.and_then(Value::as_object).map(|map| map.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect()).unwrap_or_default() }
fn is_reference(value: &str) -> bool { value.starts_with("${") && value.ends_with('}') }
fn sanitize(map: BTreeMap<String, String>) -> BTreeMap<String, String> { map.into_iter().map(|(k, v)| if is_reference(&v) { (k, v) } else { (k.clone(), format!("${{{k}}}")) }).collect() }
fn fingerprint(v: &LocalMcpVariant) -> String { format!("{:?}|{:?}|{:?}|{:?}|{:?}", v.transport, v.command, v.args, v.url, v.env) }
