use std::collections::BTreeMap;

use crate::core::credential_store::{CredentialStore, MemoryCredentialStore};
use crate::core::mcp_bridge::{resolve_bridge_environment, resolve_bridge_values};

#[test]
fn bridge_resolves_references_without_returning_the_reference_literal() {
    let store = MemoryCredentialStore::default();
    store.set("github", "GITHUB_TOKEN", "secret-value").unwrap();
    let env = resolve_bridge_environment(
        &store,
        "github",
        &BTreeMap::from([("GITHUB_TOKEN".into(), "${GITHUB_TOKEN}".into())]),
    )
    .unwrap();

    assert_eq!(
        env.get("GITHUB_TOKEN").map(String::as_str),
        Some("secret-value")
    );
    assert!(!env.values().any(|value| value.contains("${")));
}

#[test]
fn bridge_rejects_missing_credential() {
    let error = resolve_bridge_environment(
        &MemoryCredentialStore::default(),
        "github",
        &BTreeMap::from([("GITHUB_TOKEN".into(), "${GITHUB_TOKEN}".into())]),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("missing credential reference GITHUB_TOKEN"));
}

#[test]
fn bridge_keeps_runtime_values_direct_and_resolves_only_secret_references() {
    let store = MemoryCredentialStore::default();
    store
        .set("mcp-pdf", "TAVILY_API_KEY", "secret-value")
        .unwrap();
    let env = resolve_bridge_environment(
        &store,
        "mcp-pdf",
        &BTreeMap::from([
            ("NODE_PATH".into(), "/opt/node".into()),
            ("MCP_PDF_ALLOWED_PATHS".into(), "/tmp".into()),
            ("TAVILY_API_KEY".into(), "${TAVILY_API_KEY}".into()),
        ]),
    )
    .unwrap();

    assert_eq!(env["NODE_PATH"], "/opt/node");
    assert_eq!(env["MCP_PDF_ALLOWED_PATHS"], "/tmp");
    assert_eq!(env["TAVILY_API_KEY"], "secret-value");
}

#[test]
fn bridge_resolves_header_reference_by_credential_name() {
    let store = MemoryCredentialStore::default();
    store.set("remote", "API_TOKEN", "secret-value").unwrap();
    let values = resolve_bridge_values(
        &store,
        "remote",
        &BTreeMap::from([("Authorization".into(), "${API_TOKEN}".into())]),
    )
    .unwrap();

    assert_eq!(values["Authorization"], "secret-value");
}
