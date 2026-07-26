use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

use super::mcp::is_credential_name;
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

#[derive(Clone, Debug)]
pub struct LocalMcpSelection {
    pub variant: LocalMcpVariant,
    pub literal_credentials: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct LocalMcpDiscovery {
    plan: LocalMcpPlan,
    variants: Vec<PrivateMcpVariant>,
}

impl LocalMcpDiscovery {
    pub fn plan(&self) -> LocalMcpPlan {
        self.plan.clone()
    }

    pub fn select(&self, name: &str, host: &str) -> Option<LocalMcpSelection> {
        self.variants
            .iter()
            .find(|variant| variant.public.name == name && variant.public.host == host)
            .map(|variant| LocalMcpSelection {
                variant: variant.public.clone(),
                literal_credentials: variant.literal_credentials.clone(),
            })
    }
}

#[derive(Clone, Debug)]
struct PrivateMcpVariant {
    public: LocalMcpVariant,
    literal_credentials: BTreeMap<String, String>,
    fingerprint: String,
}

struct RawMcpDefinition {
    command: Option<String>,
    args: Vec<String>,
    url: Option<String>,
    env: BTreeMap<String, String>,
    headers: BTreeMap<String, String>,
}

pub fn scan_local_mcp_configs_in(home: &Path) -> Result<LocalMcpPlan> {
    Ok(scan_local_mcp_configs_with_secrets_in(home)?.plan())
}

pub fn scan_local_mcp_configs_with_secrets_in(home: &Path) -> Result<LocalMcpDiscovery> {
    let hosts = [
        (McpHost::Codex, "codex", home.join(".codex/config.toml")),
        (
            McpHost::ClaudeCode,
            "claude_code",
            home.join(".claude.json"),
        ),
        (
            McpHost::Claude3p,
            "claude_3p",
            home.join("Library/Application Support/Claude-3p/claude_desktop_config.json"),
        ),
        (McpHost::Kiro, "kiro", home.join(".kiro/settings/mcp.json")),
        (
            McpHost::Reasonix,
            "reasonix",
            home.join(".reasonix/config.toml"),
        ),
    ];
    let mut scanned = 0;
    let mut variants = Vec::new();
    for (host, key, path) in hosts {
        if !path.exists() {
            continue;
        }
        scanned += 1;
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("read MCP configuration {path:?}"))?;
        variants.extend(parse_host_config(host, key, &path, &content)?);
    }
    build_discovery(scanned, variants)
}

pub fn select_local_mcp_from_config(
    path: &Path,
    host_key: &str,
    name: &str,
) -> Result<LocalMcpSelection> {
    let host = match host_key {
        "codex" => McpHost::Codex,
        "claude_code" => McpHost::ClaudeCode,
        "claude_3p" => McpHost::Claude3p,
        "kiro" => McpHost::Kiro,
        "reasonix" => McpHost::Reasonix,
        _ => anyhow::bail!("unsupported local MCP host {host_key}"),
    };
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("read MCP configuration {path:?}"))?;
    let variants = parse_host_config(host, host_key, path, &content)?;
    variants
        .into_iter()
        .find(|variant| variant.public.name == name)
        .map(|variant| LocalMcpSelection {
            variant: variant.public,
            literal_credentials: variant.literal_credentials,
        })
        .context("MCP service was not found in local configuration backup")
}

pub fn without_managed_targets(
    discovery: LocalMcpDiscovery,
    managed: &HashSet<(String, String)>,
) -> LocalMcpDiscovery {
    let variants = discovery
        .variants
        .into_iter()
        .filter(|variant| {
            !managed.contains(&(variant.public.host.clone(), variant.public.name.clone()))
        })
        .collect();
    build_discovery(discovery.plan.total_hosts_scanned, variants)
        .expect("local MCP discovery must be valid")
}

fn build_discovery(scanned: usize, variants: Vec<PrivateMcpVariant>) -> Result<LocalMcpDiscovery> {
    let found = variants.len();
    let mut by_name: HashMap<String, Vec<&PrivateMcpVariant>> = HashMap::new();
    for variant in &variants {
        by_name
            .entry(variant.public.name.clone())
            .or_default()
            .push(variant);
    }
    let mut groups: Vec<_> = by_name
        .into_iter()
        .map(|(name, items)| {
            let distinct = items
                .iter()
                .map(|variant| &variant.fingerprint)
                .collect::<HashSet<_>>()
                .len();
            LocalMcpGroup {
                name,
                variants: items
                    .into_iter()
                    .map(|variant| variant.public.clone())
                    .collect(),
                has_conflict: distinct > 1,
            }
        })
        .collect();
    groups.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(LocalMcpDiscovery {
        plan: LocalMcpPlan {
            total_hosts_scanned: scanned,
            total_servers_found: found,
            groups,
        },
        variants,
    })
}

fn parse_host_config(
    host: McpHost,
    key: &str,
    path: &Path,
    content: &str,
) -> Result<Vec<PrivateMcpVariant>> {
    if host == McpHost::Codex {
        let document = content.parse::<toml_edit::DocumentMut>()?;
        let Some(table) = document["mcp_servers"].as_table() else {
            return Ok(Vec::new());
        };
        return table
            .iter()
            .map(|(name, item)| {
                let fields = item
                    .as_table()
                    .context("MCP server table must be a table")?;
                let command = fields
                    .get("command")
                    .and_then(toml_edit::Item::as_str)
                    .map(str::to_string);
                let url = fields
                    .get("url")
                    .and_then(toml_edit::Item::as_str)
                    .map(str::to_string);
                let args = fields
                    .get("args")
                    .and_then(toml_edit::Item::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(|value| value.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                private_variant(
                    key,
                    path,
                    name,
                    RawMcpDefinition {
                        command,
                        args,
                        url,
                        env: fields.get("env").map(toml_map).unwrap_or_default(),
                        headers: fields.get("headers").map(toml_map).unwrap_or_default(),
                    },
                )
            })
            .collect();
    }
    if host == McpHost::Reasonix {
        let document = content.parse::<toml_edit::DocumentMut>()?;
        let Some(plugins) = document["plugins"].as_array_of_tables() else {
            return Ok(Vec::new());
        };
        return plugins
            .iter()
            .filter_map(|plugin| {
                let name = plugin["name"].as_str()?;
                Some(private_variant(
                    key,
                    path,
                    name,
                    RawMcpDefinition {
                        command: plugin["command"].as_str().map(str::to_string),
                        args: plugin["args"]
                            .as_array()
                            .map(|values| {
                                values
                                    .iter()
                                    .filter_map(|value| value.as_str().map(str::to_string))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        url: plugin["url"].as_str().map(str::to_string),
                        env: plugin.get("env").map(toml_map).unwrap_or_default(),
                        headers: plugin.get("headers").map(toml_map).unwrap_or_default(),
                    },
                ))
            })
            .collect();
    }
    let root = serde_json::from_str::<Value>(content)?
        .get("mcpServers")
        .cloned()
        .unwrap_or(Value::Null);
    let Some(servers) = root.as_object() else {
        return Ok(Vec::new());
    };
    servers
        .iter()
        .map(|(name, value)| {
            let object = value
                .as_object()
                .context("MCP server definition must be an object")?;
            private_variant(
                key,
                path,
                name,
                RawMcpDefinition {
                    command: object
                        .get("command")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    args: object
                        .get("args")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect()
                        })
                        .unwrap_or_default(),
                    url: object
                        .get("url")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    env: string_map(object.get("env")),
                    headers: string_map(object.get("headers")),
                },
            )
        })
        .collect()
}

fn private_variant(
    host: &str,
    path: &Path,
    name: &str,
    raw: RawMcpDefinition,
) -> Result<PrivateMcpVariant> {
    let RawMcpDefinition {
        command,
        args,
        url,
        env,
        headers,
    } = raw;
    let literal_credentials: BTreeMap<String, String> = env
        .iter()
        .filter_map(|(key, value)| {
            (!is_reference(value) && is_credential_name(key))
                .then_some((key.clone(), value.clone()))
        })
        .chain(headers.iter().filter_map(|(key, value)| {
            (!is_reference(value)).then_some((key.clone(), value.clone()))
        }))
        .collect();
    let public = LocalMcpVariant {
        host: host.to_string(),
        path: path.display().to_string(),
        name: name.to_string(),
        transport: if url.is_some() { "http" } else { "stdio" }.to_string(),
        command,
        args,
        url,
        env: sanitize(&env, is_credential_name),
        headers: sanitize(&headers, |_| true),
        credential_names: literal_credentials.keys().cloned().collect(),
    };
    let fingerprint = format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
        public.transport, public.command, public.args, public.url, env, headers
    );
    Ok(PrivateMcpVariant {
        public,
        literal_credentials,
        fingerprint,
    })
}

fn toml_map(item: &toml_edit::Item) -> BTreeMap<String, String> {
    item.as_table()
        .map(|table| {
            table
                .iter()
                .filter_map(|(key, value)| {
                    value
                        .as_str()
                        .map(|value| (key.to_string(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}
fn string_map(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}
fn is_reference(value: &str) -> bool {
    value.starts_with("${") && value.ends_with('}')
}
fn sanitize(
    map: &BTreeMap<String, String>,
    is_credential: impl Fn(&str) -> bool,
) -> BTreeMap<String, String> {
    map.iter()
        .map(|(key, value)| {
            if is_reference(value) || !is_credential(key) {
                (key.clone(), value.clone())
            } else {
                (key.clone(), format!("${{{key}}}"))
            }
        })
        .collect()
}
