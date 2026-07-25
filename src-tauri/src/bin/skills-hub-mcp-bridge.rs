use std::path::PathBuf;

use anyhow::{Context, Result};
use app_lib::core::credential_store::OsCredentialStore;
use app_lib::core::mcp_bridge::resolve_bridge_environment;
use app_lib::core::skill_store::SkillStore;

fn main() {
    if let Err(error) = run() {
        eprintln!("skills-hub-mcp-bridge: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().context("bridge mode is required")?;
    if mode != "stdio" {
        anyhow::bail!("only stdio bridge mode is currently supported");
    }
    let db_flag = args.next().context("--db is required")?;
    let db_path = args.next().context("database path is required")?;
    let id_flag = args.next().context("--server-id is required")?;
    let server_id = args.next().context("server id is required")?;
    if db_flag != "--db" || id_flag != "--server-id" {
        anyhow::bail!("expected stdio --db <path> --server-id <id>");
    }
    let separator = args.next().context("-- command separator is required")?;
    if separator != "--" {
        anyhow::bail!("expected -- before MCP command");
    }
    let command = args.next().context("MCP command is required")?;
    let command_args = args.collect::<Vec<_>>();

    let store = SkillStore::new(PathBuf::from(db_path));
    let server = store
        .list_mcp_servers()?
        .into_iter()
        .find(|server| server.id == server_id)
        .context("MCP server was not found")?;
    if server.transport != "stdio" {
        anyhow::bail!("MCP server is not a stdio server");
    }
    let references = server
        .env
        .iter()
        .map(|(key, value)| {
            value
                .as_str()
                .map(|value| (key.clone(), value.to_string()))
                .context("MCP environment reference must be a string")
        })
        .collect::<Result<_>>()?;
    let environment = resolve_bridge_environment(&OsCredentialStore, &server.id, &references)?;
    let status = std::process::Command::new(command)
        .args(command_args)
        .envs(environment)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("launch bridged MCP server")?;
    std::process::exit(status.code().unwrap_or(1));
}
