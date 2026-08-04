use crate::core::mcp_adapters::{render_server, McpHost};
use crate::core::skill_store::McpServerRecord;

#[test]
fn secret_bearing_stdio_renders_plaintext_env_for_all_hosts() {
    // The command layer resolves ${NAME} references before rendering; here we
    // render the already-resolved record to prove no bridge is emitted.
    let mut server = McpServerRecord::stdio("github-id", "github", "npx", vec!["-y".into()]);
    server
        .env
        .insert("GITHUB_TOKEN".into(), "secret-value".into());

    for host in [
        McpHost::Codex,
        McpHost::ClaudeCode,
        McpHost::Claude3p,
        McpHost::Kiro,
        McpHost::Reasonix,
    ] {
        let rendered = render_server(host, &server).unwrap();
        assert!(!rendered.contains("--mcp-bridge"));
        assert!(!rendered.contains("skills.db"));
        assert!(rendered.contains("GITHUB_TOKEN"));
        assert!(rendered.contains("secret-value"));
    }
}

#[test]
fn json_merge_preserves_unmanaged_entries_and_rejects_unowned_collision() {
    let existing =
        r#"{"theme":"dark","mcpServers":{"other":{"command":"other"},"github":{"command":"old"}}}"#;
    let replacement = r#"{"mcpServers":{"github":{"command":"new"}}}"#;

    assert!(crate::core::mcp_adapters::merge_json_host_config(
        existing,
        replacement,
        "github",
        false
    )
    .is_err());
    let merged =
        crate::core::mcp_adapters::merge_json_host_config(existing, replacement, "github", true)
            .unwrap();
    let value: serde_json::Value = serde_json::from_str(&merged).unwrap();
    assert_eq!(value["theme"], "dark");
    assert_eq!(value["mcpServers"]["other"]["command"], "other");
    assert_eq!(value["mcpServers"]["github"]["command"], "new");
}

#[test]
fn removes_owned_json_server_without_touching_other_configuration() {
    let existing =
        r#"{"theme":"dark","mcpServers":{"other":{"command":"other"},"github":{"command":"old"}}}"#;

    let next = crate::core::mcp_adapters::remove_json_host_config(existing, "github").unwrap();
    let value: serde_json::Value = serde_json::from_str(&next).unwrap();
    assert_eq!(value["theme"], "dark");
    assert_eq!(value["mcpServers"]["other"]["command"], "other");
    assert!(value["mcpServers"].get("github").is_none());
}

#[test]
fn codex_toml_merge_preserves_comments_and_rejects_unowned_collision() {
    let existing = "# keep this comment\nmodel = \"gpt\"\n[mcp_servers.other]\ncommand = \"other\"\n[mcp_servers.github]\ncommand = \"old\"\n";
    let replacement = "[mcp_servers.github]\ncommand = \"new\"\n";

    assert!(crate::core::mcp_adapters::merge_codex_toml_config(
        existing,
        replacement,
        "github",
        false
    )
    .is_err());
    let merged =
        crate::core::mcp_adapters::merge_codex_toml_config(existing, replacement, "github", true)
            .unwrap();
    assert!(merged.contains("# keep this comment"));
    assert!(merged.contains("[mcp_servers.other]"));
    assert!(merged.contains("command = \"other\""));
    assert!(merged.contains("[mcp_servers.github]"));
    assert!(merged.contains("command = \"new\""));
}

#[test]
fn atomic_write_keeps_backup_and_replaces_target() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mcp.json");
    std::fs::write(&path, "old").unwrap();

    let backup = crate::core::mcp_adapters::write_config_atomically(&path, "new")
        .unwrap()
        .unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    assert_eq!(std::fs::read_to_string(backup).unwrap(), "old");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "rendered config holds plaintext credentials");
    }
}

#[test]
fn sync_json_host_file_merges_and_creates_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mcp.json");
    std::fs::write(&path, r#"{"mcpServers":{"other":{"command":"other"}}}"#).unwrap();
    let server = McpServerRecord::stdio("github-id", "github", "npx", vec!["-y".into()]);

    let outcome =
        crate::core::mcp_adapters::sync_host_file(McpHost::ClaudeCode, &server, &path, false)
            .unwrap();
    assert!(outcome.backup_path.is_some());
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(value["mcpServers"]["other"]["command"], "other");
    assert_eq!(value["mcpServers"]["github"]["command"], "npx");
}

#[test]
fn activating_then_deactivating_host_config_leaves_no_trace() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mcp.json");
    std::fs::write(&path, r#"{"theme":"dark","mcpServers":{}}"#).unwrap();
    let server =
        crate::core::skill_store::McpServerRecord::stdio("github-id", "github", "npx", vec![]);

    // 激活：把 server 写入目标 app 配置
    crate::core::mcp_adapters::sync_host_file(McpHost::ClaudeCode, &server, &path, true).unwrap();
    let after_sync: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(after_sync["mcpServers"]["github"]["command"], "npx");

    // 取消：把 server 从目标 app 配置删除，且不破坏其他配置
    crate::core::mcp_adapters::remove_host_file(McpHost::ClaudeCode, "github", &path).unwrap();
    let after_remove: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(after_remove["mcpServers"].get("github").is_none());
    assert_eq!(after_remove["theme"], "dark");
}

#[test]
fn detects_an_existing_unmanaged_server_before_target_takeover() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mcp.json");
    std::fs::write(&path, r#"{"mcpServers":{"playwright":{"command":"npx"}}}"#).unwrap();

    assert!(crate::core::mcp_adapters::host_config_contains_server(
        McpHost::Kiro,
        &path,
        "playwright"
    )
    .unwrap());
    assert!(!crate::core::mcp_adapters::host_config_contains_server(
        McpHost::Kiro,
        &path,
        "mcp-pdf"
    )
    .unwrap());
}

#[test]
fn reasonix_merge_preserves_other_plugins_and_protects_unowned_collision() {
    let existing = "default_model = \"x\"\n[[plugins]]\nname = \"other\"\ncommand = \"other\"\n[[plugins]]\nname = \"github\"\ncommand = \"old\"\n";
    let replacement = "[[plugins]]\nname = \"github\"\ncommand = \"new\"\n";

    assert!(crate::core::mcp_adapters::merge_reasonix_toml_config(
        existing,
        replacement,
        "github",
        false
    )
    .is_err());
    let merged = crate::core::mcp_adapters::merge_reasonix_toml_config(
        existing,
        replacement,
        "github",
        true,
    )
    .unwrap();
    assert!(merged.contains("default_model = \"x\""));
    assert!(merged.contains("name = \"other\""));
    assert!(merged.contains("command = \"other\""));
    assert!(merged.contains("name = \"github\""));
    assert!(merged.contains("command = \"new\""));
}

#[test]
fn reasonix_renders_env_as_inline_table_and_stays_valid_toml() {
    let mut server = crate::core::skill_store::McpServerRecord::stdio(
        "tavily-id",
        "tavily",
        "npx",
        vec!["tavily-mcp@0.2.15".into()],
    );
    server
        .env
        .insert("TAVILY_API_KEY".into(), "secret-value".into());
    let rendered = render_server(McpHost::Reasonix, &server).unwrap();

    assert!(
        !rendered.contains("[plugins.env]"),
        "env must not be a table header"
    );
    assert!(rendered.contains("env = {"), "env must be an inline table");
    rendered.parse::<toml_edit::DocumentMut>().unwrap();

    let mut second = crate::core::skill_store::McpServerRecord::stdio(
        "pdf-id",
        "mcp-pdf",
        "uvx",
        vec!["mcp-pdf".into()],
    );
    second
        .env
        .insert("MCP_PDF_ALLOWED_PATHS".into(), "/tmp".into());
    let second_rendered = render_server(McpHost::Reasonix, &second).unwrap();
    let merged = crate::core::mcp_adapters::merge_reasonix_toml_config(
        &rendered,
        &second_rendered,
        "mcp-pdf",
        false,
    )
    .unwrap();
    let document = merged.parse::<toml_edit::DocumentMut>().unwrap();
    let plugins = document["plugins"].as_array_of_tables().unwrap();
    assert_eq!(plugins.len(), 2);
    assert_eq!(plugins.get(0).unwrap()["name"].as_str(), Some("tavily"));
    assert_eq!(plugins.get(1).unwrap()["name"].as_str(), Some("mcp-pdf"));
}

#[test]
fn supported_hosts_have_global_config_paths() {
    for host in [
        McpHost::Codex,
        McpHost::ClaudeCode,
        McpHost::Claude3p,
        McpHost::Kiro,
        McpHost::Reasonix,
    ] {
        assert!(crate::core::mcp_adapters::global_config_path(host).is_ok());
    }
}

#[test]
fn claude_3p_uses_its_own_desktop_configuration_path() {
    let home = std::path::Path::new("/Users/example");
    assert_eq!(
        crate::core::mcp_adapters::global_config_path_in(home, McpHost::Claude3p).unwrap(),
        home.join("Library/Application Support/Claude-3p/claude_desktop_config.json"),
    );
}

#[test]
fn secret_bearing_http_renders_plaintext_headers_for_all_hosts() {
    let mut server = McpServerRecord::stdio("stripe-id", "stripe", "unused", vec![]);
    server.transport = "http".into();
    server.command = None;
    server.url = Some("https://mcp.stripe.com".into());
    server
        .headers
        .insert("Authorization".into(), "sk-live-123".into());

    for host in [
        McpHost::Codex,
        McpHost::ClaudeCode,
        McpHost::Claude3p,
        McpHost::Kiro,
        McpHost::Reasonix,
    ] {
        let rendered = render_server(host, &server).unwrap();
        assert!(rendered.contains("https://mcp.stripe.com"));
        assert!(!rendered.contains("127.0.0.1"));
        assert!(rendered.contains("Authorization"));
        assert!(rendered.contains("sk-live-123"));
    }
}
