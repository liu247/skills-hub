use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(unix)]
use std::sync::Arc;

use anyhow::{Context, Result};

use super::credential_store::{CredentialStore, OsCredentialStore};
use super::mcp::credential_name_from_reference;
use super::skill_store::SkillStore;

/// Keeps credentials in the credential agent's memory for its lifetime so
/// repeated MCP process restarts do not repeatedly unlock the system keychain.
pub struct CachedCredentialStore<S> {
    inner: S,
    entries: Mutex<BTreeMap<(String, String), Option<String>>>,
}

impl<S> CachedCredentialStore<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            entries: Mutex::new(BTreeMap::new()),
        }
    }

    #[cfg(test)]
    pub fn inner(&self) -> &S {
        &self.inner
    }
}

impl<S: CredentialStore> CredentialStore for CachedCredentialStore<S> {
    fn set(&self, server_id: &str, env_var: &str, value: &str) -> Result<()> {
        self.inner.set(server_id, env_var, value)?;
        self.entries
            .lock()
            .map_err(|_| anyhow::anyhow!("credential cache lock poisoned"))?
            .insert(
                (server_id.to_string(), env_var.to_string()),
                Some(value.to_string()),
            );
        Ok(())
    }

    fn get(&self, server_id: &str, env_var: &str) -> Result<Option<String>> {
        let key = (server_id.to_string(), env_var.to_string());
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| anyhow::anyhow!("credential cache lock poisoned"))?;
        if let Some(value) = entries.get(&key).cloned() {
            return Ok(value);
        }
        let value = self.inner.get(server_id, env_var)?;
        entries.insert(key, value.clone());
        Ok(value)
    }

    fn delete(&self, server_id: &str, env_var: &str) -> Result<()> {
        self.inner.delete(server_id, env_var)?;
        self.entries
            .lock()
            .map_err(|_| anyhow::anyhow!("credential cache lock poisoned"))?
            .remove(&(server_id.to_string(), env_var.to_string()));
        Ok(())
    }
}

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
    let _command = args.next().context("MCP command is required")?;
    let _command_args = args.collect::<Vec<_>>();

    #[cfg(unix)]
    return run_stdio_agent_client(PathBuf::from(db_path), server_id);

    #[cfg(not(unix))]
    run_stdio_bridge_direct(PathBuf::from(db_path), server_id, _command, _command_args)
}

#[cfg(not(unix))]
fn run_stdio_bridge_direct(
    db_path: PathBuf,
    server_id: String,
    command: String,
    command_args: Vec<String>,
) -> Result<()> {
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

#[cfg(unix)]
pub fn run_credential_agent_cli(arguments: impl IntoIterator<Item = String>) -> Result<()> {
    let mut args = arguments.into_iter();
    let db_flag = args.next().context("--db is required")?;
    let db_path = PathBuf::from(args.next().context("database path is required")?);
    let socket_flag = args.next().context("--socket is required")?;
    let socket_path = PathBuf::from(args.next().context("socket path is required")?);
    if db_flag != "--db" || socket_flag != "--socket" || args.next().is_some() {
        anyhow::bail!("expected --db <path> --socket <path>");
    }

    if socket_path.exists() {
        if UnixStream::connect(&socket_path).is_ok() {
            return Ok(());
        }
        std::fs::remove_file(&socket_path).context("remove stale MCP credential agent socket")?;
    }
    let listener = UnixListener::bind(&socket_path).context("bind MCP credential agent socket")?;
    std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))
        .context("secure MCP credential agent socket")?;
    let credentials = Arc::new(CachedCredentialStore::new(OsCredentialStore));
    for connection in listener.incoming() {
        let stream = match connection {
            Ok(stream) => stream,
            Err(_) => continue,
        };
        let db_path = db_path.clone();
        let credentials = Arc::clone(&credentials);
        std::thread::spawn(move || {
            if let Err(error) = handle_agent_connection(stream, db_path, credentials) {
                eprintln!("skills-hub MCP credential agent: {error:#}");
            }
        });
    }
    Ok(())
}

#[cfg(unix)]
fn run_stdio_agent_client(db_path: PathBuf, server_id: String) -> Result<()> {
    let socket_path = credential_agent_socket_path(&db_path);
    let mut stream = match UnixStream::connect(&socket_path) {
        Ok(stream) => stream,
        Err(_) => {
            let executable = std::env::current_exe().context("resolve Skills Hub executable")?;
            let _ = std::process::Command::new(executable)
                .args([
                    "--mcp-credential-agent",
                    "--db",
                    &db_path.to_string_lossy(),
                    "--socket",
                    &socket_path.to_string_lossy(),
                ])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            connect_credential_agent(&socket_path)?
        }
    };
    stream
        .write_all(format!("{server_id}\n").as_bytes())
        .context("request managed MCP server from credential agent")?;
    let mut stdout = std::io::stdout().lock();
    let mut request_stream = stream
        .try_clone()
        .context("clone credential agent stream")?;
    let request_thread = std::thread::spawn(move || {
        let mut stdin = std::io::stdin();
        std::io::copy(&mut stdin, &mut request_stream)
    });
    std::io::copy(&mut stream, &mut stdout).context("read managed MCP output")?;
    let _ = request_thread.join();
    Ok(())
}

#[cfg(unix)]
fn connect_credential_agent(socket_path: &std::path::Path) -> Result<UnixStream> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match UnixStream::connect(socket_path) {
            Ok(stream) => return Ok(stream),
            Err(error) if std::time::Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(error) => return Err(error).context("connect MCP credential agent"),
        }
    }
}

#[cfg(unix)]
fn credential_agent_socket_path(db_path: &std::path::Path) -> PathBuf {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(db_path.to_string_lossy().as_bytes());
    let name = format!("skills-hub-mcp-{:x}.sock", digest);
    std::env::temp_dir().join(&name[..48])
}

#[cfg(unix)]
fn handle_agent_connection(
    mut stream: UnixStream,
    db_path: PathBuf,
    credentials: Arc<CachedCredentialStore<OsCredentialStore>>,
) -> Result<()> {
    let server_id = read_agent_server_id(&mut stream)?;
    let store = SkillStore::new(db_path);
    let server = store
        .list_mcp_servers()?
        .into_iter()
        .find(|server| server.id == server_id)
        .context("MCP server was not found")?;
    if server.transport != "stdio" {
        anyhow::bail!("MCP server is not a stdio server");
    }
    let command = server.command.context("MCP server command is required")?;
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
    let environment = resolve_bridge_environment(credentials.as_ref(), &server.id, &references)?;
    let mut child = std::process::Command::new(command)
        .args(server.args)
        .envs(environment)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .context("launch managed MCP server")?;
    let mut child_stdin = child.stdin.take().context("open managed MCP stdin")?;
    let mut child_stdout = child.stdout.take().context("open managed MCP stdout")?;
    let mut request_stream = stream.try_clone().context("clone agent request stream")?;
    let request_thread =
        std::thread::spawn(move || std::io::copy(&mut request_stream, &mut child_stdin));
    std::io::copy(&mut child_stdout, &mut stream).context("relay managed MCP output")?;
    let _ = request_thread.join();
    let _ = child.wait();
    Ok(())
}

#[cfg(unix)]
fn read_agent_server_id(stream: &mut UnixStream) -> Result<String> {
    let mut bytes = Vec::new();
    loop {
        let mut byte = [0_u8; 1];
        stream
            .read_exact(&mut byte)
            .context("read MCP agent request")?;
        if byte[0] == b'\n' {
            break;
        }
        if bytes.len() >= 256 {
            anyhow::bail!("MCP agent server id is too long");
        }
        bytes.push(byte[0]);
    }
    String::from_utf8(bytes).context("MCP agent server id is not UTF-8")
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
