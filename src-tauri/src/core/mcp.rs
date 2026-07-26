use std::collections::BTreeMap;

use anyhow::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpServerInput {
    pub name: String,
    pub transport: McpTransport,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub url: Option<String>,
    pub headers: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpTransport {
    Stdio,
    Http,
}

impl McpServerInput {
    pub fn stdio(
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        env: BTreeMap<String, String>,
    ) -> Self {
        Self {
            name: name.into(),
            transport: McpTransport::Stdio,
            command: Some(command.into()),
            args,
            env,
            url: None,
            headers: BTreeMap::new(),
        }
    }
}

pub fn validate_mcp_server_input(input: &McpServerInput) -> Result<()> {
    let valid_name = input.name.chars().enumerate().all(|(index, ch)| {
        (index == 0 && ch.is_ascii_lowercase())
            || (index > 0 && (ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'))
    });
    if !valid_name || input.name == "workspace" {
        anyhow::bail!("MCP server name must be lowercase alphanumeric with hyphens");
    }

    match input.transport {
        McpTransport::Stdio
            if input
                .command
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty() =>
        {
            anyhow::bail!("stdio MCP server requires a command");
        }
        McpTransport::Http if input.url.as_deref().unwrap_or_default().trim().is_empty() => {
            anyhow::bail!("HTTP MCP server requires a URL");
        }
        _ => {}
    }

    for (key, value) in &input.env {
        if key.is_empty()
            || !key
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
            || (is_credential_name(key) && value != &format!("${{{key}}}"))
        {
            anyhow::bail!("MCP environment values must be references in the form ${{NAME}}");
        }
    }
    for (header, value) in &input.headers {
        if header.trim().is_empty() || credential_name_from_reference(value).is_none() {
            anyhow::bail!("MCP HTTP headers must use credential references in the form ${{NAME}}");
        }
    }
    Ok(())
}

pub fn credential_name_from_reference(value: &str) -> Option<&str> {
    let name = value.strip_prefix("${")?.strip_suffix('}')?;
    (!name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_'))
    .then_some(name)
}

pub fn is_credential_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper.ends_with("_KEY")
        || upper.ends_with("_TOKEN")
        || upper.ends_with("_SECRET")
        || upper.ends_with("_PASSWORD")
        || upper == "PASSWORD"
        || upper == "AUTHORIZATION"
}
