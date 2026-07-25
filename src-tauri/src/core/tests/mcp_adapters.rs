use crate::core::mcp_adapters::{render_server, McpHost};
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
        McpHost::Kiro,
        McpHost::Reasonix,
    ] {
        let rendered = render_server(host, &server, Some(8765)).unwrap();
        assert!(rendered.contains("skills-hub-mcp-bridge"));
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
        &replacement,
        "github",
        false
    )
    .is_err());
    let merged =
        crate::core::mcp_adapters::merge_json_host_config(existing, &replacement, "github", true)
            .unwrap();
    let value: serde_json::Value = serde_json::from_str(&merged).unwrap();
    assert_eq!(value["theme"], "dark");
    assert_eq!(value["mcpServers"]["other"]["command"], "other");
    assert_eq!(value["mcpServers"]["github"]["command"], "new");
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
        McpHost::Kiro,
        McpHost::Reasonix,
    ] {
        let rendered = render_server(host, &server, Some(8765)).unwrap();
        assert!(rendered.contains("127.0.0.1:8765"));
        assert!(!rendered.contains("STRIPE_KEY"));
    }
}
