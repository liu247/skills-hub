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
        McpHost::DeepSeekHarness,
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
        McpHost::DeepSeekHarness,
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
        McpHost::DeepSeekHarness,
    ] {
        let rendered = render_server(host, &server).unwrap();
        assert!(rendered.contains("https://mcp.stripe.com"));
        assert!(!rendered.contains("127.0.0.1"));
        assert!(rendered.contains("Authorization"));
        assert!(rendered.contains("sk-live-123"));
    }
}

#[test]
fn dsh_stdio_render_matches_cordis_insert_shape() {
    let mut server = McpServerRecord::stdio(
        "memory-id",
        "memory",
        "mcp-server-memory",
        vec!["--db".into(), "x".into()],
    );
    server.cwd = Some("/tmp".into());
    server
        .env
        .insert("MEMORY_FILE_PATH".into(), "~/.dsh/memory.jsonl".into());

    let rendered = render_server(McpHost::DeepSeekHarness, &server).unwrap();
    assert!(rendered.starts_with("- insert:\n"), "{rendered}");
    assert!(
        rendered.contains("    - id: skills-hub-memory"),
        "{rendered}"
    );
    assert!(
        rendered.contains("name: '@deepseek-ai/dsh-mcp-client'"),
        "{rendered}"
    );
    assert!(rendered.contains("serverName: memory"), "{rendered}");
    assert!(rendered.contains("transport: stdio"), "{rendered}");
    assert!(
        rendered.contains("command: mcp-server-memory"),
        "{rendered}"
    );
    assert!(rendered.contains("          - --db"), "{rendered}");
    assert!(rendered.contains("          - x"), "{rendered}");
    assert!(rendered.contains("        env:"), "{rendered}");
    assert!(
        rendered.contains("          MEMORY_FILE_PATH: ~/.dsh/memory.jsonl"),
        "{rendered}"
    );
    assert!(rendered.contains("        cwd: /tmp"), "{rendered}");
}

#[test]
fn dsh_http_render_uses_streamable_http_transport() {
    let mut server = McpServerRecord::stdio("http-id", "remote", "unused", vec![]);
    server.transport = "http".into();
    server.command = None;
    server.url = Some("https://mcp.example.com/sse".into());
    server
        .headers
        .insert("Authorization".into(), "Bearer tok:en".into());

    let rendered = render_server(McpHost::DeepSeekHarness, &server).unwrap();
    assert!(
        rendered.contains("transport: streamable-http"),
        "{rendered}"
    );
    assert!(
        rendered.contains("url: https://mcp.example.com/sse"),
        "{rendered}"
    );
    assert!(rendered.contains("        headers:"), "{rendered}");
    // 冒号/特殊字符值必须被正确引用，保持 YAML 有效
    assert!(
        rendered.contains("Authorization: \"Bearer tok:en\""),
        "{rendered}"
    );
}

#[test]
fn dsh_merge_appends_entry_and_preserves_user_patches_with_js_tags() {
    let existing = "- insert:\n    - id: user-memory\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: user_memory\n        transport: stdio\n        command: mcp-server-memory\n        cwd: !!js process.cwd()\n";
    let server = McpServerRecord::stdio("memory-id", "memory", "mcp-server-memory", vec![]);
    let rendered = render_server(McpHost::DeepSeekHarness, &server).unwrap();

    let merged =
        crate::core::mcp_adapters::merge_dsh_yaml_config(existing, &rendered, "memory", false)
            .unwrap();
    assert!(merged.contains("cwd: !!js process.cwd()"), "{merged}");
    assert!(merged.contains("- id: skills-hub-memory"), "{merged}");
    // 用户条目在 skills-hub 条目之前
    assert!(
        merged.find("user-memory").unwrap() < merged.find("skills-hub-memory").unwrap(),
        "{merged}"
    );
}

#[test]
fn dsh_merge_replaces_owned_entry_in_place() {
    let existing =
        "- insert:\n    - id: skills-hub-memory\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: memory\n        transport: stdio\n        command: old-cmd\n";
    let mut server = McpServerRecord::stdio("memory-id", "memory", "new-cmd", vec![]);
    server.env.insert("MODE".into(), "new".into());
    let rendered = render_server(McpHost::DeepSeekHarness, &server).unwrap();

    let merged =
        crate::core::mcp_adapters::merge_dsh_yaml_config(existing, &rendered, "memory", true)
            .unwrap();
    assert!(!merged.contains("old-cmd"), "{merged}");
    assert!(merged.contains("command: new-cmd"), "{merged}");
    assert!(merged.contains("MODE: new"), "{merged}");
    assert_eq!(merged.matches("- insert:").count(), 1, "{merged}");
}

#[test]
fn dsh_merge_rejects_unowned_collision() {
    let existing = "- insert:\n    - id: skills-hub-memory\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: memory\n";
    let server = McpServerRecord::stdio("memory-id", "memory", "cmd", vec![]);
    let rendered = render_server(McpHost::DeepSeekHarness, &server).unwrap();

    assert!(
        crate::core::mcp_adapters::merge_dsh_yaml_config(existing, &rendered, "memory", false)
            .is_err()
    );
}

#[test]
fn dsh_remove_keeps_unrelated_entries() {
    let existing = "- insert:\n    - id: user-memory\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        cwd: !!js process.cwd()\n- insert:\n    - id: skills-hub-tavily\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: tavily\n";
    let next = crate::core::mcp_adapters::remove_dsh_yaml_config(existing, "tavily").unwrap();
    assert!(next.contains("user-memory"), "{next}");
    assert!(next.contains("cwd: !!js process.cwd()"), "{next}");
    assert!(!next.contains("skills-hub-tavily"), "{next}");
    assert_eq!(next.matches("- insert:").count(), 1, "{next}");
}

#[test]
fn dsh_entry_id_with_special_characters_is_quoted() {
    let mut server = McpServerRecord::stdio("weird-id", "my server:v1", "cmd", vec![]);
    server.cwd = Some("/path with space".into());
    let rendered = render_server(McpHost::DeepSeekHarness, &server).unwrap();
    assert!(
        rendered.contains(r#"- id: "skills-hub-my server:v1""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#"cwd: "/path with space""#),
        "{rendered}"
    );
}
