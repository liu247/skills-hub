use crate::core::mcp_import::parse_mcp_config;

#[test]
fn parses_mcp_servers_from_json_configuration() {
    let candidates = parse_mcp_config(
        "mcp.json",
        r#"{
          "mcpServers": {
            "github": {"command":"npx", "args":["-y", "@modelcontextprotocol/server-github"], "env":{"GITHUB_TOKEN":"${GITHUB_TOKEN}"}},
            "docs": {"url":"https://example.test/mcp", "headers":{"Authorization":"${DOCS_TOKEN}"}}
          }
        }"#,
    ).unwrap();

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].name, "docs");
    assert_eq!(candidates[0].transport, "http");
    assert_eq!(candidates[1].name, "github");
    assert_eq!(
        candidates[1].env.get("GITHUB_TOKEN").unwrap(),
        "${GITHUB_TOKEN}"
    );
}

#[test]
fn rejects_literal_secret_values_from_imported_configuration() {
    let result = parse_mcp_config(
        "mcp.json",
        r#"{"mcpServers":{"unsafe":{"command":"npx","env":{"API_KEY":"literal-secret"}}}}"#,
    );

    assert!(format!("{:#}", result.unwrap_err()).contains("references"));
}

#[test]
fn parses_codex_toml_configuration() {
    let candidates = parse_mcp_config(
        "config.toml",
        r#"
            [mcp_servers.docs]
            command = "npx"
            args = ["-y", "@example/docs"]

            [mcp_servers.docs.env]
            DOCS_TOKEN = "${DOCS_TOKEN}"
        "#,
    )
    .unwrap();

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].name, "docs");
    assert_eq!(candidates[0].command.as_deref(), Some("npx"));
}

#[test]
fn absolutize_resolves_repo_relative_paths_but_keeps_external_commands() {
    use crate::core::mcp_import::{absolutize_candidate_paths, McpImportCandidate};
    use std::collections::BTreeMap;

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("dist")).unwrap();
    std::fs::write(dir.path().join("server.js"), "// a").unwrap();
    std::fs::write(dir.path().join("dist").join("cli.js"), "// b").unwrap();

    let candidate = |command: Option<&str>, args: Vec<String>| McpImportCandidate {
        name: "server".into(),
        transport: "stdio".into(),
        command: command.map(str::to_string),
        args,
        env: BTreeMap::new(),
        url: None,
        headers: BTreeMap::new(),
        source_path: "mcp.json".into(),
        source_url: "https://github.com/a/b".into(),
    };

    // node + 仓库内相对脚本 → 脚本绝对化
    let mut c1 = candidate(Some("node"), vec!["dist/cli.js".into()]);
    absolutize_candidate_paths(&mut c1, dir.path());
    assert_eq!(c1.command.as_deref(), Some("node"));
    assert_eq!(
        c1.args[0],
        dir.path()
            .join("dist")
            .join("cli.js")
            .to_string_lossy()
            .to_string()
    );

    // command 本身是仓库内文件 → 绝对化
    let mut c2 = candidate(Some("server.js"), vec![]);
    absolutize_candidate_paths(&mut c2, dir.path());
    assert_eq!(
        c2.command.as_deref(),
        Some(dir.path().join("server.js").to_string_lossy().as_ref())
    );

    // 外部命令与绝对路径保持
    let mut c3 = candidate(Some("npx"), vec!["-y".into(), "tavily-mcp".into()]);
    absolutize_candidate_paths(&mut c3, dir.path());
    assert_eq!(c3.command.as_deref(), Some("npx"));
    assert_eq!(c3.args, vec!["-y".to_string(), "tavily-mcp".to_string()]);

    let mut c4 = candidate(Some("/usr/local/bin/server"), vec!["/etc/x.conf".into()]);
    absolutize_candidate_paths(&mut c4, dir.path());
    assert_eq!(c4.command.as_deref(), Some("/usr/local/bin/server"));
    assert_eq!(c4.args, vec!["/etc/x.conf".to_string()]);
}
