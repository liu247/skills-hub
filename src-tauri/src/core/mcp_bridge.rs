use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::credential_store::{CredentialStore, OsCredentialStore};
use super::mcp::credential_name_from_reference;
use super::skill_store::SkillStore;

pub fn resolve_bridge_environment(
    credentials: &dyn CredentialStore,
    server_id: &str,
    references: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let mut environment = BTreeMap::new();
    for (name, reference) in references {
        let expected = format!("${{{name}}}");
        if reference != &expected {
            environment.insert(name.clone(), reference.clone());
            continue;
        }
        let value = credentials
            .get(server_id, name)?
            .ok_or_else(|| anyhow::anyhow!("missing credential reference {name}"))?;
        environment.insert(name.clone(), value);
    }
    Ok(environment)
}

pub fn resolve_bridge_values(
    credentials: &dyn CredentialStore,
    server_id: &str,
    references: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for (name, reference) in references {
        let credential_name = credential_name_from_reference(reference)
            .ok_or_else(|| anyhow::anyhow!("invalid credential reference for {name}"))?;
        let value = credentials
            .get(server_id, credential_name)?
            .ok_or_else(|| anyhow::anyhow!("missing credential reference {credential_name}"))?;
        values.insert(name.clone(), value);
    }
    Ok(values)
}

pub fn run_bridge_cli(arguments: impl IntoIterator<Item = String>) -> Result<()> {
    let mut args = arguments.into_iter();
    let mode = args.next().context("bridge mode is required")?;
    if mode == "http" {
        return run_http_bridge(args);
    }
    if mode != "stdio" {
        anyhow::bail!("unsupported bridge mode {mode}");
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

fn run_http_bridge(mut args: impl Iterator<Item = String>) -> Result<()> {
    let db_flag = args.next().context("--db is required")?;
    let db_path = args.next().context("database path is required")?;
    let id_flag = args.next().context("--server-id is required")?;
    let server_id = args.next().context("server id is required")?;
    let port_flag = args.next().context("--port is required")?;
    let port = args
        .next()
        .context("HTTP bridge port is required")?
        .parse::<u16>()
        .context("parse HTTP bridge port")?;
    if db_flag != "--db"
        || id_flag != "--server-id"
        || port_flag != "--port"
        || args.next().is_some()
    {
        anyhow::bail!("expected http --db <path> --server-id <id> --port <port>");
    }
    let store = SkillStore::new(PathBuf::from(db_path));
    let server = store
        .list_mcp_servers()?
        .into_iter()
        .find(|server| server.id == server_id)
        .context("MCP server was not found")?;
    if server.transport != "http" {
        anyhow::bail!("MCP server is not an HTTP server");
    }
    let upstream = server.url.context("HTTP server URL is required")?;
    let headers = resolve_bridge_values(
        &OsCredentialStore,
        &server.id,
        &server
            .headers
            .iter()
            .map(|(key, value)| {
                value
                    .as_str()
                    .map(|value| (key.clone(), value.to_string()))
                    .context("MCP HTTP header reference must be a string")
            })
            .collect::<Result<_>>()?,
    )?;
    let listener = tiny_http::Server::http(("127.0.0.1", port))
        .map_err(|error| anyhow::anyhow!("bind loopback MCP HTTP credential bridge: {error}"))?;
    let client = reqwest::blocking::Client::new();
    for mut request in listener.incoming_requests() {
        let mut body = Vec::new();
        if let Err(error) = std::io::Read::read_to_end(request.as_reader(), &mut body) {
            let _ = request
                .respond(tiny_http::Response::from_string(error.to_string()).with_status_code(400));
            continue;
        }
        let method = reqwest::Method::from_bytes(request.method().as_str().as_bytes())
            .unwrap_or(reqwest::Method::POST);
        let mut upstream_request = client.request(method, &upstream).body(body);
        for header in request.headers() {
            if !header.field.equiv("host") && !header.field.equiv("content-length") {
                upstream_request = upstream_request
                    .header(header.field.as_str().to_string(), header.value.as_str());
            }
        }
        for (name, value) in &headers {
            upstream_request = upstream_request.header(name, value);
        }
        match upstream_request.send() {
            Ok(response) => {
                let status = tiny_http::StatusCode(response.status().as_u16());
                let response_headers = response
                    .headers()
                    .iter()
                    .filter_map(|(name, value)| {
                        tiny_http::Header::from_bytes(name.as_str(), value.as_bytes()).ok()
                    })
                    .collect::<Vec<_>>();
                let length = response
                    .content_length()
                    .and_then(|value| usize::try_from(value).ok());
                let proxy_response =
                    tiny_http::Response::new(status, response_headers, response, length, None);
                let _ = request.respond(proxy_response);
            }
            Err(error) => {
                let _ = request.respond(
                    tiny_http::Response::from_string(error.to_string()).with_status_code(502),
                );
            }
        }
    }
    Ok(())
}
