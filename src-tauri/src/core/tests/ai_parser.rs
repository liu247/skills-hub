use crate::core::ai_parser::{
    get_provider_configs, normalize_ai_plan_json, save_provider_config, set_provider_api_key,
    validate_ai_plan_json, AiProvider, AiProviderConfig,
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
fn skill_plan_accepts_parameter_definitions_but_rejects_secret_defaults() {
    let accepted = validate_ai_plan_json(
        r#"{
          "protocol_version": "skills-hub-ai-plan/v2", "kind": "skill", "summary": "Skill",
          "source": { "url": "https://example.com/skill", "evidence": ["README"] }, "confidence": "high", "warnings": [],
          "skill_plan": { "name": "Example", "source_url": "https://example.com/skill", "parameters": [
            { "name": "EXAMPLE_API_KEY", "description": "Key", "is_sensitive": true, "required": true, "evidence": ["README"] },
            { "name": "EXAMPLE_MODEL", "description": "Model", "is_sensitive": false, "default_value": "small", "required": false, "evidence": ["README"] }
          ] }
        }"#,
    );
    assert!(accepted.is_ok());
    let rejected = validate_ai_plan_json(
        r#"{
          "protocol_version": "skills-hub-ai-plan/v2", "kind": "skill", "summary": "Skill",
          "source": { "url": "https://example.com/skill", "evidence": ["README"] }, "confidence": "high", "warnings": [],
          "skill_plan": { "name": "Example", "source_url": "https://example.com/skill", "parameters": [
            { "name": "EXAMPLE_API_KEY", "is_sensitive": true, "default_value": "secret", "evidence": ["README"] }
          ] }
        }"#,
    );
    assert!(rejected.is_err());
}

#[test]
fn skill_plan_accepts_a_model_supplied_type_hint() {
    let plan = validate_ai_plan_json(
        r#"{
          "protocol_version": "skills-hub-ai-plan/v2", "kind": "skill", "summary": "Skill",
          "source": { "url": "https://example.com/skill", "evidence": ["README"] }, "confidence": "high", "warnings": [],
          "skill_plan": { "type": "git", "name": "Example", "source_url": "https://example.com/skill", "parameters": [] }
        }"#,
    );
    assert!(plan.is_ok());
}

#[test]
fn skill_plan_ignores_unknown_model_fields_but_keeps_required_fields() {
    let plan = validate_ai_plan_json(
        r#"{
          "protocol_version": "skills-hub-ai-plan/v2", "kind": "skill", "summary": "Skill", "model_comment": "extra",
          "source": { "url": "https://example.com/skill", "evidence": ["README"], "retrieval_note": "extra" }, "confidence": "high", "warnings": [],
          "skill_plan": { "type": "git", "name": "Example", "source_url": "https://example.com/skill", "installation_notes": ["extra"], "parameters": [
            { "name": "EXAMPLE_API_KEY", "is_sensitive": true, "evidence": ["README"], "provider_hint": "extra" }
          ] }
        }"#,
    );
    assert!(plan.is_ok());
}

#[test]
fn normalizes_a_skill_response_that_omits_the_protocol_envelope() {
    let normalized = normalize_ai_plan_json(
        r#"{ "name": "SenseNova", "source_url": "https://github.com/OpenSenseNova/SenseNova-Skills.git", "parameters": [] }"#,
        "https://github.com/OpenSenseNova/SenseNova-Skills.git",
        None,
    )
    .expect("normalize response");
    let plan = validate_ai_plan_json(&normalized).expect("validate normalized plan");
    assert_eq!(plan.kind, crate::core::ai_parser::AiPlanKind::Skill);
    assert_eq!(plan.skill_plan.expect("skill plan").name, "SenseNova");
}

#[test]
fn normalizes_prose_wrapped_unknown_response_to_the_requested_kind() {
    let normalized = normalize_ai_plan_json(
        "Here is the plan:\n{ \"name\": \"SenseNova\", \"source_url\": \"https://github.com/OpenSenseNova/SenseNova-Skills.git\" }\nDone.",
        "https://github.com/OpenSenseNova/SenseNova-Skills.git",
        Some(crate::core::ai_parser::AiPlanKind::Skill),
    )
    .expect("normalize response");
    let plan = validate_ai_plan_json(&normalized).expect("validate normalized plan");
    assert_eq!(plan.kind, crate::core::ai_parser::AiPlanKind::Skill);
}

#[test]
fn falls_back_to_a_minimal_skill_plan_when_the_model_returns_no_json() {
    let normalized = normalize_ai_plan_json(
        "I cannot provide the requested JSON plan.",
        "https://example.com/skill.git",
        Some(crate::core::ai_parser::AiPlanKind::Skill),
    )
    .expect("normalize response");
    let plan = validate_ai_plan_json(&normalized).expect("validate normalized plan");
    assert_eq!(plan.kind, crate::core::ai_parser::AiPlanKind::Skill);
    let skill = plan.skill_plan.expect("skill plan");
    assert_eq!(skill.source_url, "https://example.com/skill.git");
    assert!(skill.parameters.is_empty());
}

#[test]
fn falls_back_to_a_minimal_skill_plan_when_json_is_not_an_object() {
    let normalized = normalize_ai_plan_json(
        "[]",
        "https://example.com/skill.git",
        Some(crate::core::ai_parser::AiPlanKind::Skill),
    )
    .expect("normalize response");
    let plan = validate_ai_plan_json(&normalized).expect("validate normalized plan");
    assert_eq!(plan.kind, crate::core::ai_parser::AiPlanKind::Skill);
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
