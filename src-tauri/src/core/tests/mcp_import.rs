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
