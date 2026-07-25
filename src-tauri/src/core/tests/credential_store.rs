use crate::core::credential_store::{CredentialStore, MemoryCredentialStore};

#[test]
fn memory_credential_store_roundtrips_and_deletes_a_secret() {
    let store = MemoryCredentialStore::default();
    store.set("server-1", "GITHUB_TOKEN", "secret").unwrap();
    assert_eq!(
        store.get("server-1", "GITHUB_TOKEN").unwrap().as_deref(),
        Some("secret")
    );
    store.delete("server-1", "GITHUB_TOKEN").unwrap();
    assert_eq!(store.get("server-1", "GITHUB_TOKEN").unwrap(), None);
}
