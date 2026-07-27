use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::core::credential_store::{CredentialStore, MemoryCredentialStore};
use crate::core::mcp_bridge::{
    resolve_bridge_environment, resolve_bridge_values, CachedCredentialStore,
};
use anyhow::Result;

struct CountingCredentialStore {
    reads: AtomicUsize,
}

struct SlowCredentialStore {
    reads: AtomicUsize,
}

impl CredentialStore for SlowCredentialStore {
    fn set(&self, _server_id: &str, _env_var: &str, _value: &str) -> Result<()> {
        Ok(())
    }

    fn get(&self, _server_id: &str, _env_var: &str) -> Result<Option<String>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(50));
        Ok(Some("secret-value".into()))
    }

    fn delete(&self, _server_id: &str, _env_var: &str) -> Result<()> {
        Ok(())
    }
}

impl CredentialStore for CountingCredentialStore {
    fn set(&self, _server_id: &str, _env_var: &str, _value: &str) -> Result<()> {
        Ok(())
    }

    fn get(&self, _server_id: &str, _env_var: &str) -> Result<Option<String>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(Some("secret-value".into()))
    }

    fn delete(&self, _server_id: &str, _env_var: &str) -> Result<()> {
        Ok(())
    }
}

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

#[test]
fn credential_agent_cache_reads_each_keychain_entry_once_per_session() {
    let store = CachedCredentialStore::new(CountingCredentialStore {
        reads: AtomicUsize::new(0),
    });

    assert_eq!(
        store.get("tavily", "TAVILY_API_KEY").unwrap().as_deref(),
        Some("secret-value")
    );
    assert_eq!(
        store.get("tavily", "TAVILY_API_KEY").unwrap().as_deref(),
        Some("secret-value")
    );

    assert_eq!(store.inner().reads.load(Ordering::SeqCst), 1);
}

#[test]
fn credential_agent_cache_deduplicates_concurrent_keychain_reads() {
    let store = Arc::new(CachedCredentialStore::new(SlowCredentialStore {
        reads: AtomicUsize::new(0),
    }));
    let first = {
        let store = Arc::clone(&store);
        std::thread::spawn(move || store.get("tavily", "TAVILY_API_KEY").unwrap())
    };
    let second = {
        let store = Arc::clone(&store);
        std::thread::spawn(move || store.get("tavily", "TAVILY_API_KEY").unwrap())
    };

    assert_eq!(first.join().unwrap().as_deref(), Some("secret-value"));
    assert_eq!(second.join().unwrap().as_deref(), Some("secret-value"));
    assert_eq!(store.inner().reads.load(Ordering::SeqCst), 1);
}
