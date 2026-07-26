use crate::core::mcp_adapters::{render_server, BridgeRuntime, McpHost};
use crate::core::skill_store::McpServerRecord;

#[test]
fn secret_bearing_stdio_renders_bridge_for_all_hosts() {
    let mut server = McpServerRecord::stdio("github-id", "github", "npx", vec!["-y".into()]);
    server
        .env
        .insert("GITHUB_TOKEN".into(), "${GITHUB_TOKEN}".into());

    for host in [
        McpHost::Codex,
        McpHost::ClaudeCode,
        McpHost::Claude3p,
        McpHost::Kiro,
        McpHost::Reasonix,
    ] {
        let rendered = render_server(
            host,
            &server,
            Some(&BridgeRuntime {
                executable: "/Applications/Skills Hub.app/Contents/MacOS/skills-hub".into(),
                database_path: "/Users/example/Library/Application Support/skills-hub/skills.db"
                    .into(),
                http_port: None,
            }),
        )
        .unwrap();
        assert!(rendered.contains("--mcp-bridge"));
        assert!(rendered.contains("--db"));
        assert!(rendered.contains("skills.db"));
        assert!(!rendered.contains("GITHUB_TOKEN"));
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
}

#[test]
fn sync_json_host_file_merges_and_creates_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mcp.json");
    std::fs::write(&path, r#"{"mcpServers":{"other":{"command":"other"}}}"#).unwrap();
    let server = McpServerRecord::stdio("github-id", "github", "npx", vec!["-y".into()]);

    let outcome =
        crate::core::mcp_adapters::sync_host_file(McpHost::ClaudeCode, &server, &path, false, None)
            .unwrap();
    assert!(outcome.backup_path.is_some());
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(value["mcpServers"]["other"]["command"], "other");
    assert_eq!(value["mcpServers"]["github"]["command"], "npx");
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
fn secret_bearing_http_renders_loopback_url_for_all_hosts() {
    let mut server = McpServerRecord::stdio("stripe-id", "stripe", "unused", vec![]);
    server.transport = "http".into();
    server.command = None;
    server.url = Some("https://mcp.stripe.com".into());
    server
        .headers
        .insert("Authorization".into(), "${STRIPE_KEY}".into());

    for host in [
        McpHost::Codex,
        McpHost::ClaudeCode,
        McpHost::Claude3p,
        McpHost::Kiro,
        McpHost::Reasonix,
    ] {
        let rendered = render_server(
            host,
            &server,
            Some(&BridgeRuntime {
                executable: "/Applications/Skills Hub.app/Contents/MacOS/skills-hub".into(),
                database_path: "/Users/example/Library/Application Support/skills-hub/skills.db"
                    .into(),
                http_port: Some(8765),
            }),
        )
        .unwrap();
        assert!(rendered.contains("127.0.0.1:8765"));
        assert!(!rendered.contains("STRIPE_KEY"));
    }
}
