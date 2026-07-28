use crate::core::ai_parser::{migrate_credentials_to_local_store, AiProvider, AI_CREDENTIAL_OWNER};
use crate::core::credential_store::{CredentialStore, LocalCredentialStore, MemoryCredentialStore};
use crate::core::skill_store::{McpSecretRefRecord, McpServerRecord, SkillStore};

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let directory = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(directory.path().join("test.db"));
    store.ensure_schema().expect("ensure schema");
    (directory, store)
}

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

#[test]
fn local_credential_store_persists_encrypted_values_across_reopen() {
    let (_directory, store) = make_store();
    let credentials = LocalCredentialStore::from_store(&store).expect("open local store");
    credentials
        .set("server-1", "API_KEY", "secret-value")
        .unwrap();

    let reopened = LocalCredentialStore::from_store(&store).expect("reopen local store");
    assert_eq!(
        reopened.get("server-1", "API_KEY").unwrap().as_deref(),
        Some("secret-value")
    );
    assert!(!store
        .get_setting(&LocalCredentialStore::setting_key("server-1", "API_KEY"))
        .unwrap()
        .unwrap_or_default()
        .contains("secret-value"));
}

#[test]
fn migration_moves_keychain_credentials_into_the_local_store() {
    let (_directory, store) = make_store();
    let server = McpServerRecord::stdio("server-1", "Example", "node", vec![]);
    store.upsert_mcp_server(&server).unwrap();
    store
        .replace_mcp_secret_refs(
            &server.id,
            &[McpSecretRefRecord::new(&server.id, "API_KEY")],
        )
        .unwrap();
    let legacy = MemoryCredentialStore::default();
    legacy
        .set(AI_CREDENTIAL_OWNER, AiProvider::OpenAi.key(), "ai-secret")
        .unwrap();
    legacy.set(&server.id, "API_KEY", "mcp-secret").unwrap();

    migrate_credentials_to_local_store(&store, &legacy).expect("migrate credentials");
    let local = LocalCredentialStore::from_store(&store).expect("open local store");
    assert_eq!(
        local
            .get(AI_CREDENTIAL_OWNER, AiProvider::OpenAi.key())
            .unwrap()
            .as_deref(),
        Some("ai-secret")
    );
    assert_eq!(
        local.get(&server.id, "API_KEY").unwrap().as_deref(),
        Some("mcp-secret")
    );
    assert_eq!(legacy.get(&server.id, "API_KEY").unwrap(), None);
}
