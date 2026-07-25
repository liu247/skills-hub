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
            || value != &format!("${{{key}}}")
        {
            anyhow::bail!("MCP environment values must be references in the form ${{NAME}}");
        }
    }
    Ok(())
}
