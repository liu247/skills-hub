use std::collections::BTreeMap;

use crate::core::mcp::{validate_mcp_server_input, McpServerInput, McpTransport};

#[test]
fn accepts_stdio_definition_with_environment_reference() {
    let input = McpServerInput::stdio(
        "github",
        "npx",
        vec!["-y".into(), "@modelcontextprotocol/server-github".into()],
        [("GITHUB_TOKEN".into(), "${GITHUB_TOKEN}".into())].into(),
    );

    assert!(validate_mcp_server_input(&input).is_ok());
}

#[test]
fn rejects_literal_environment_secret_and_unsafe_name() {
    let input = McpServerInput::stdio(
        "Bad_Name",
        "node",
        vec![],
        [("TOKEN".into(), "literal-secret".into())].into(),
    );

    assert!(validate_mcp_server_input(&input).is_err());
}

#[test]
fn rejects_literal_http_header_secret() {
    let input = McpServerInput {
        name: "remote".into(),
        transport: McpTransport::Http,
        command: None,
        args: vec![],
        env: BTreeMap::new(),
        url: Some("https://example.test/mcp".into()),
        headers: BTreeMap::from([("Authorization".into(), "Bearer literal-secret".into())]),
    };

    assert!(validate_mcp_server_input(&input).is_err());
}
