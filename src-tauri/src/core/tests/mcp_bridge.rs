use std::collections::BTreeMap;

use crate::core::credential_store::{CredentialStore, MemoryCredentialStore};
use crate::core::mcp_bridge::{resolve_credential_environment, resolve_credential_values};

#[test]
fn resolves_references_without_returning_the_reference_literal() {
    let store = MemoryCredentialStore::default();
    store.set("github", "GITHUB_TOKEN", "secret-value").unwrap();
    let env = resolve_credential_environment(
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
fn rejects_missing_credential() {
    let error = resolve_credential_environment(
        &MemoryCredentialStore::default(),
        "github",
        &BTreeMap::from([("GITHUB_TOKEN".into(), "${GITHUB_TOKEN}".into())]),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("missing credential reference GITHUB_TOKEN"));
}

#[test]
fn keeps_runtime_values_direct_and_resolves_only_secret_references() {
    let store = MemoryCredentialStore::default();
    store
        .set("tavily", "TAVILY_API_KEY", "secret-value")
        .unwrap();
    let env = resolve_credential_environment(
        &store,
        "tavily",
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
fn non_credential_key_with_reference_like_value_passes_through() {
    // A runtime value that happens to look like a reference must not be
    // treated as a credential: classification follows is_credential_name,
    // matching validate_mcp_server_input.
    let store = MemoryCredentialStore::default();
    let env = resolve_credential_environment(
        &store,
        "mcp-pdf",
        &BTreeMap::from([("NODE_PATH".into(), "${NODE_PATH}".into())]),
    )
    .unwrap();

    assert_eq!(env["NODE_PATH"], "${NODE_PATH}");
}

#[test]
fn resolves_header_reference_by_credential_name() {
    let store = MemoryCredentialStore::default();
    store.set("remote", "API_TOKEN", "secret-value").unwrap();
    let values = resolve_credential_values(
        &store,
        "remote",
        &BTreeMap::from([("Authorization".into(), "${API_TOKEN}".into())]),
    )
    .unwrap();

    assert_eq!(values["Authorization"], "secret-value");
}
