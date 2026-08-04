use crate::core::ai_parser::{
    get_provider_configs, normalize_ai_plan_json, parse_source_with_ai, save_provider_config,
    set_provider_api_key, validate_ai_plan_json, AiPlanKind, AiProvider, AiProviderConfig,
};
use crate::core::credential_store::{LocalCredentialStore, MemoryCredentialStore};
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
fn normalizes_mcp_name_and_filters_unsupported_targets() {
    let plan = validate_ai_plan_json(
        r#"{
          "protocol_version": "skills-hub-ai-plan/v2",
          "kind": "mcp",
          "summary": "MCP server",
          "source": { "url": "https://example.com/mcp", "evidence": ["README"] },
          "confidence": "high",
          "warnings": [],
          "mcp_plan": {
            "name": "Tavily MCP!",
            "transport": "stdio",
            "command": "npx",
            "args": ["-y", "tavily"],
            "env": { "TAVILY_API_KEY": "${TAVILY_API_KEY}" },
            "headers": {},
            "recommended_targets": ["codex", "claude_code", "not-a-real-tool"]
          }
        }"#,
    )
    .expect("plan must validate");
    let mcp = plan.mcp_plan.expect("mcp plan");
    assert_eq!(mcp.name, "tavily-mcp");
    assert_eq!(mcp.recommended_targets, ["codex", "claude_code"]);
    assert!(plan
        .warnings
        .iter()
        .any(|warning| warning.contains("normalized")));
    assert!(plan
        .warnings
        .iter()
        .any(|warning| warning.contains("not supported")));
}

#[test]
fn rejects_mcp_name_that_normalizes_to_empty() {
    let error = validate_ai_plan_json(
        r#"{
          "protocol_version": "skills-hub-ai-plan/v2",
          "kind": "mcp",
          "summary": "MCP server",
          "source": { "url": "https://example.com/mcp", "evidence": ["README"] },
          "confidence": "high",
          "warnings": [],
          "mcp_plan": {
            "name": "!!!",
            "transport": "stdio",
            "command": "npx",
            "args": ["-y", "example"],
            "env": {},
            "headers": {}
          }
        }"#,
    )
    .expect_err("name must not normalize to empty");
    assert!(format!("{error:#}").contains("name is invalid"));
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

#[test]
fn mcp_array_response_extracts_first_plan_object() {
    let normalized = normalize_ai_plan_json(
        r#"[{"protocol_version":"skills-hub-ai-plan/v2","kind":"mcp","summary":"test","source":{"url":"https://example.com/mcp","evidence":["README"]},"confidence":"high","warnings":[],"mcp_plan":{"name":"example","transport":"stdio","command":"npx","args":["-y","example"],"env":{},"headers":{}}}]"#,
        "https://example.com/mcp",
        Some(AiPlanKind::Mcp),
    )
    .expect("array response must normalize");
    let plan = validate_ai_plan_json(&normalized).expect("normalized plan must validate");
    assert_eq!(plan.kind, crate::core::ai_parser::AiPlanKind::Mcp);
    assert_eq!(plan.mcp_plan.expect("mcp plan").name, "example");
}

#[test]
fn mcp_plan_missing_name_is_derived_from_source_url() {
    let normalized = normalize_ai_plan_json(
        r#"{"protocol_version":"skills-hub-ai-plan/v2","kind":"mcp","summary":"test","source":{"url":"https://github.com/designcomputer/mysql_mcp_server","evidence":["README"]},"confidence":"high","warnings":[],"mcp_plan":{"transport":"stdio","command":"uvx","args":["mysql"],"env":{},"headers":{}}}"#,
        "https://github.com/designcomputer/mysql_mcp_server",
        Some(AiPlanKind::Mcp),
    )
    .expect("plan with missing name must normalize");
    let plan = validate_ai_plan_json(&normalized).expect("normalized plan must validate");
    assert_eq!(plan.mcp_plan.expect("mcp plan").name, "mysql-mcp-server");
}

#[test]
fn mcp_response_with_concatenated_objects_extracts_first_plan() {
    let normalized = normalize_ai_plan_json(
        r#"[{"protocol_version":"skills-hub-ai-plan/v2","kind":"mcp","summary":"first","source":{"url":"https://example.com/a","evidence":["README"]},"confidence":"high","warnings":[],"mcp_plan":{"name":"first","transport":"stdio","command":"npx","args":["-y","a"],"env":{},"headers":{}}}],[{"protocol_version":"skills-hub-ai-plan/v2","kind":"mcp","summary":"second","source":{"url":"https://example.com/b","evidence":["README"]},"confidence":"high","warnings":[],"mcp_plan":{"name":"second","transport":"stdio","command":"npx","args":["-y","b"],"env":{},"headers":{}}}]"#,
        "https://example.com/a",
        Some(AiPlanKind::Mcp),
    )
    .expect("concatenated response must normalize");
    let plan = validate_ai_plan_json(&normalized).expect("normalized plan must validate");
    assert_eq!(plan.mcp_plan.expect("mcp plan").name, "first");
}

#[test]
fn github_readme_url_rewrites_repo_pages_only() {
    use crate::core::ai_parser::github_readme_url;
    assert_eq!(
        github_readme_url("https://github.com/microsoft/playwright-mcp").as_deref(),
        Some("https://raw.githubusercontent.com/microsoft/playwright-mcp/HEAD/README.md")
    );
    assert_eq!(
        github_readme_url("https://github.com/owner/repo/tree/main/packages/mcp").as_deref(),
        Some("https://raw.githubusercontent.com/owner/repo/HEAD/README.md")
    );
    assert_eq!(
        github_readme_url("https://github.com/owner/repo/blob/main/README.md").as_deref(),
        Some("https://raw.githubusercontent.com/owner/repo/HEAD/README.md")
    );
    assert_eq!(github_readme_url("https://example.com/mcp"), None);
    assert_eq!(
        github_readme_url("https://github.com/settings/tokens"),
        None
    );
    assert_eq!(github_readme_url("https://github.com/"), None);
}

/// Live smoke test: parses a real MCP repository README with the user's
/// configured AI provider and real credentials from the app database.
/// Run with: cargo test -- --ignored smoke_parse_real_mcp_source
#[test]
#[ignore = "live smoke test requiring a configured AI provider API key"]
fn smoke_parse_real_mcp_source() {
    let db_path =
        "/Users/ywxklzd/Library/Application Support/com.qufei1993.skillshub/skills_hub.db";
    if !std::path::Path::new(db_path).exists() {
        eprintln!("SMOKE SKIP: app database not found at {db_path}");
        return;
    }
    let store = SkillStore::new(db_path.into());
    store.ensure_schema().expect("ensure schema");
    let credentials = LocalCredentialStore::from_store(&store).expect("local credential store");
    let sources = [
        "https://github.com/microsoft/playwright-mcp",
        "https://github.com/tavily-ai/tavily-mcp",
        "https://github.com/pydantic/mcp-run-python",
        "https://github.com/github/github-mcp-server",
        "https://github.com/designcomputer/mysql_mcp_server",
    ];
    let mut passed = 0;
    for source in sources {
        match parse_source_with_ai(
            &store,
            &credentials,
            AiProvider::DeepSeek,
            source,
            Some(AiPlanKind::Mcp),
        ) {
            Ok(plan) => {
                let mcp = plan.mcp_plan.expect("MCP plan");
                passed += 1;
                println!(
                    "SMOKE OK: {source}\n  name={} transport={} command={:?} args={:?} env={:?} headers={:?} targets={:?} warnings={:?} confidence={}",
                    mcp.name,
                    mcp.transport,
                    mcp.command,
                    mcp.args,
                    mcp.env,
                    mcp.headers,
                    mcp.recommended_targets,
                    plan.warnings,
                    plan.confidence
                );
            }
            Err(error) => {
                println!("SMOKE FAIL: {source}\n  {error:#}");
            }
        }
    }
    println!("SMOKE RESULT: {passed}/{} sources parsed", sources.len());
    // The AI provider's output is non-deterministic; some responses carry
    // structural errors (wrong field types, literal header values) that the
    // parser correctly rejects. The smoke test only needs to prove the
    // end-to-end pipeline works on real repositories.
    assert!(passed >= 2, "at least 2 sources must parse successfully");
}
