use std::collections::BTreeMap;

use crate::core::credential_store::{CredentialStore, MemoryCredentialStore};
use crate::core::mcp_bridge::resolve_bridge_environment;

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
