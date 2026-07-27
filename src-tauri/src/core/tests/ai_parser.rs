use crate::core::ai_parser::{
    get_provider_configs, save_provider_config, set_provider_api_key, validate_ai_plan_json,
    AiProvider, AiProviderConfig,
};
use crate::core::credential_store::MemoryCredentialStore;
use crate::core::skill_store::SkillStore;

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let directory = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(directory.path().join("test.db"));
    store.ensure_schema().expect("ensure schema");
    (directory, store)
}

#[test]
fn rejects_literal_secret_from_ai_mcp_plan() {
    let error = validate_ai_plan_json(
        r#"{
          "protocol_version": "skills-hub-ai-plan/v1",
          "kind": "mcp",
          "summary": "MCP server",
          "source": { "url": "https://example.com/mcp", "evidence": ["README"] },
          "confidence": "high",
          "warnings": [],
          "mcp_plan": {
            "name": "example-mcp",
            "transport": "stdio",
            "command": "npx",
            "args": ["-y", "example"],
            "env": { "EXAMPLE_API_KEY": "secret-value" },
            "headers": {}
          }
        }"#,
    )
    .expect_err("literal secret must be rejected");
    assert!(format!("{error:#}").contains("credential reference"));
}

#[test]
fn provider_config_roundtrip_never_exposes_api_key() {
    let (_directory, store) = make_store();
    let credentials = MemoryCredentialStore::default();
    save_provider_config(
        &store,
        AiProviderConfig {
            provider: AiProvider::OpenAi,
            enabled: true,
            model: "gpt-4.1-mini".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
        },
    )
    .expect("save provider config");
    set_provider_api_key(&credentials, AiProvider::OpenAi, "secret-value")
        .expect("store provider key");

    let configs = get_provider_configs(&store, &credentials).expect("get provider configs");
    let openai = configs
        .iter()
        .find(|config| config.provider == AiProvider::OpenAi)
        .expect("OpenAI config");
    assert!(openai.has_api_key);
    assert_eq!(openai.model, "gpt-4.1-mini");
    assert!(!format!("{configs:?}").contains("secret-value"));
}
