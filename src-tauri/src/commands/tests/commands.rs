use super::*;
use crate::core::credential_store::{CredentialStore, LocalCredentialStore, MemoryCredentialStore};
use crate::core::skill_store::SkillRecord;

#[test]
fn mcp_dto_never_serializes_secret_values() {
    let dto = McpServerDto {
        id: "server-1".to_string(),
        name: "github".to_string(),
        transport: "stdio".to_string(),
        command: Some("npx".to_string()),
        args: vec!["-y".to_string()],
        env: std::collections::BTreeMap::from([(
            "GITHUB_TOKEN".to_string(),
            "${GITHUB_TOKEN}".to_string(),
        )]),
        cwd: None,
        url: None,
        headers: std::collections::BTreeMap::new(),
        enabled: true,
        proxy_enabled: true,
        source_url: None,
        source_path: None,
        secret_refs: vec![McpSecretStatusDto {
            env_var: "GITHUB_TOKEN".to_string(),
            has_value: true,
        }],
        targets: Vec::new(),
    };
    let encoded = serde_json::to_string(&dto).unwrap();
    assert!(encoded.contains("GITHUB_TOKEN"));
    assert!(!encoded.contains("secret-value"));
}

#[test]
fn repairs_legacy_local_mcp_records_from_backup_without_proxying_runtime_values() {
    let (dir, store) = make_store();
    let config = dir.path().join("config.toml");
    std::fs::write(
        &config,
        "[mcp_servers.mcp-pdf]\ncommand = \"bridge\"\nargs = []\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("config.toml.skills-hub.bak-1"),
        "[mcp_servers.mcp-pdf]\ncommand = \"uvx\"\nargs = [\"mcp-pdf\"]\n[mcp_servers.mcp-pdf.env]\nMCP_PDF_ALLOWED_PATHS = \"/tmp\"\nTAVILY_API_KEY = \"secret-value\"\n",
    )
    .unwrap();

    let mut server =
        crate::core::skill_store::McpServerRecord::stdio("mcp-pdf-id", "mcp-pdf", "bridge", vec![]);
    server.source_url = Some("local://codex".to_string());
    server.source_path = Some(config.to_string_lossy().to_string());
    server.env.insert(
        "MCP_PDF_ALLOWED_PATHS".to_string(),
        serde_json::Value::String("${MCP_PDF_ALLOWED_PATHS}".to_string()),
    );
    server.env.insert(
        "TAVILY_API_KEY".to_string(),
        serde_json::Value::String("${TAVILY_API_KEY}".to_string()),
    );
    store.upsert_mcp_server(&server).unwrap();
    store
        .replace_mcp_secret_refs(
            &server.id,
            &[
                McpSecretRefRecord::new(&server.id, "MCP_PDF_ALLOWED_PATHS"),
                McpSecretRefRecord::new(&server.id, "TAVILY_API_KEY"),
            ],
        )
        .unwrap();
    // The command layer resolves credential references against the same
    // SQLite-backed credential store it writes, so the test must use the
    // store-backed credentials instead of an in-memory stand-in.
    let credentials = LocalCredentialStore::from_store(&store).unwrap();

    assert_eq!(
        repair_legacy_local_mcp_records(&store, &credentials).unwrap(),
        1
    );

    let repaired = store
        .list_mcp_servers()
        .unwrap()
        .into_iter()
        .find(|record| record.id == server.id)
        .unwrap();
    assert_eq!(repaired.command.as_deref(), Some("uvx"));
    assert_eq!(repaired.env["MCP_PDF_ALLOWED_PATHS"], "/tmp");
    assert_eq!(repaired.env["TAVILY_API_KEY"], "${TAVILY_API_KEY}");
    assert_eq!(
        credentials
            .get(&server.id, "TAVILY_API_KEY")
            .unwrap()
            .as_deref(),
        Some("secret-value")
    );
    assert_eq!(
        store.list_mcp_secret_refs(&server.id).unwrap(),
        vec![McpSecretRefRecord::new(&server.id, "TAVILY_API_KEY")]
    );
}

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

fn make_target_record(
    skill_id: &str,
    tool: &str,
    target_path: &str,
) -> crate::core::skill_store::SkillTargetRecord {
    crate::core::skill_store::SkillTargetRecord {
        id: format!("{skill_id}-{tool}"),
        skill_id: skill_id.to_string(),
        tool: tool.to_string(),
        scope: "global".to_string(),
        project_path: None,
        target_path: target_path.to_string(),
        mode: "symlink".to_string(),
        status: "ok".to_string(),
        last_error: None,
        synced_at: None,
    }
}

#[test]
fn materialize_collection_env_writes_repo_root_env_for_repo_layout_scripts() {
    let (dir, store) = make_store();
    // 复现仓库布局安装：central = <root>/central/<skill>
    // 脚本按 repo_root = <skill_dir>.parent().parent() 找 .env
    let central = dir.path().join("central");
    let skill_dir = central.join("skill-a");
    std::fs::create_dir_all(&skill_dir).unwrap();
    let skill = SkillRecord {
        id: "skill-a".to_string(),
        name: "skill-a".to_string(),
        description: None,
        source_type: "git".to_string(),
        source_ref: None,
        source_subpath: None,
        source_revision: None,
        central_path: skill_dir.to_string_lossy().to_string(),
        content_hash: None,
        created_at: 1,
        updated_at: 1,
        last_sync_at: None,
        last_seen_at: 1,
        enabled: true,
        status: "ok".to_string(),
        collection: Some("TestCollection".to_string()),
    };
    store.upsert_skill(&skill).unwrap();
    store
        .upsert_collection_parameter(
            "TestCollection",
            "SN_API_KEY",
            "api key",
            false,
            Some("secret-value"),
        )
        .unwrap();

    super::materialize_collection_env(&store, &MemoryCredentialStore::default(), "TestCollection")
        .expect("materialize env");

    // skill 目录内的 .env（原有行为）
    let skill_env = std::fs::read_to_string(skill_dir.join(".env")).unwrap();
    assert!(skill_env.contains("SN_API_KEY=secret-value"));

    // parent 层（parents[2] 布局的 repo_root）
    let parent_env = std::fs::read_to_string(central.join(".env")).unwrap();
    assert!(parent_env.contains("SN_API_KEY=secret-value"));

    // parent.parent 层（parents[3] 布局的 repo_root，如 ~/.env）
    let repo_env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
    assert!(repo_env.contains("SN_API_KEY=secret-value"));
    assert!(repo_env.contains("Skills Hub managed parameters: TestCollection"));
}

#[test]
fn materialize_collection_env_keeps_multiple_collections_in_shared_env() {
    let (dir, store) = make_store();
    let skill_dir = dir.path().join("central").join("skill-a");
    std::fs::create_dir_all(&skill_dir).unwrap();
    for (id, collection) in [("skill-a", "CollA"), ("skill-b", "CollB")] {
        let skill_dir = dir.path().join("central").join(id);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let skill = SkillRecord {
            id: id.to_string(),
            name: id.to_string(),
            description: None,
            source_type: "git".to_string(),
            source_ref: None,
            source_subpath: None,
            source_revision: None,
            central_path: skill_dir.to_string_lossy().to_string(),
            content_hash: None,
            created_at: 1,
            updated_at: 1,
            last_sync_at: None,
            last_seen_at: 1,
            enabled: true,
            status: "ok".to_string(),
            collection: Some(collection.to_string()),
        };
        store.upsert_skill(&skill).unwrap();
        store
            .upsert_collection_parameter(collection, "KEY_A", "k", false, Some("value-a"))
            .unwrap();
        store
            .upsert_collection_parameter(collection, "KEY_B", "k", false, Some("value-b"))
            .unwrap();
        super::materialize_collection_env(&store, &MemoryCredentialStore::default(), collection)
            .expect("materialize env");
    }

    // 同一共享 repo_root .env 应同时含两个集合的 key（marker 按集合隔离）
    let shared = std::fs::read_to_string(dir.path().join(".env")).unwrap();
    assert!(shared.contains("KEY_A=value-a"));
    assert!(shared.contains("KEY_B=value-b"));
    assert_eq!(
        shared
            .matches("# >>> Skills Hub managed parameters:")
            .count(),
        2
    );
}

/// Live smoke test: syncs the real collection env into every supported
/// tool's global env config and verifies script-facing lookups.
/// Run with: cargo test -- --ignored smoke_sync_global_env_all_tools
#[test]
#[ignore = "live smoke test that writes real tool configs"]
fn smoke_sync_global_env_all_tools() {
    let db_path =
        "/Users/ywxklzd/Library/Application Support/com.qufei1993.skillshub/skills_hub.db";
    if !std::path::Path::new(db_path).exists() {
        eprintln!("SMOKE SKIP: app database not found");
        return;
    }
    let store = SkillStore::new(db_path.into());
    store.ensure_schema().expect("ensure schema");
    let home = dirs::home_dir().expect("home");
    for tool_key in ["claude_code", "codex", "reasonix"] {
        super::sync_tool_global_env(&store, tool_key).expect("sync global env");
        let config = crate::core::tool_env::global_env_config_for(tool_key).expect("config");
        let env = crate::core::tool_env::read_global_env(&config).expect("read env");
        println!(
            "SMOKE {}: SN_API_KEY={} SN_BASE_URL={}",
            tool_key,
            env.get("SN_API_KEY").map(|_| "set").unwrap_or("MISSING"),
            env.get("SN_BASE_URL").map(|_| "set").unwrap_or("MISSING"),
        );
    }
    let claude_settings =
        std::fs::read_to_string(home.join(".claude/settings.json")).expect("read claude settings");
    assert!(claude_settings.contains("SN_API_KEY"));
    let codex_config =
        std::fs::read_to_string(home.join(".codex/config.toml")).expect("read codex config");
    assert!(codex_config.contains("SN_API_KEY"));
    // reasonix currently has no activated skills, so the sync skips it; its
    // global .env may still carry user-provided keys (untouched by us).
    println!("SMOKE OK: activated tools carry collection env");
}

#[test]
fn unsync_removes_db_target_record_even_when_target_path_is_missing() {
    let (dir, store) = make_store();
    // 与用户场景一致：local 技能，源目录已不存在，target_path 也已不存在。
    let skill = SkillRecord {
        id: "skill-1".to_string(),
        name: "skill-1".to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: Some(
            dir.path()
                .join("missing-source")
                .to_string_lossy()
                .to_string(),
        ),
        source_subpath: None,
        source_revision: None,
        central_path: dir.path().join("central").to_string_lossy().to_string(),
        content_hash: None,
        created_at: 1,
        updated_at: 1,
        last_sync_at: None,
        last_seen_at: 1,
        enabled: true,
        status: "ok".to_string(),
        collection: None,
    };
    store.upsert_skill(&skill).unwrap();
    let missing_target = "/nonexistent/.codex/skills/skill-1";
    store
        .upsert_skill_target(&make_target_record("skill-1", "codex", missing_target))
        .unwrap();
    assert!(store
        .get_skill_target("skill-1", "codex", "global", None)
        .unwrap()
        .is_some());

    super::unsync_skill_from_tool_impl(&store, "skill-1", "codex", Some("global"), None)
        .expect("unsync must succeed");

    assert!(
        store
            .get_skill_target("skill-1", "codex", "global", None)
            .unwrap()
            .is_none(),
        "DB target record must be removed even when target_path is missing"
    );
}

#[test]
fn format_anyhow_error_passthrough_prefixes() {
    let err = anyhow::anyhow!("MULTI_SKILLS|abc");
    assert_eq!(format_anyhow_error(err), "MULTI_SKILLS|abc");
}

#[test]
fn format_anyhow_error_redacts_clone_temp_path() {
    let err = anyhow::anyhow!("clone https://example.com/a/b into /tmp/skills-hub-git-123");
    let msg = format_anyhow_error(err);
    assert!(msg.contains("已省略临时目录"));
    assert!(!msg.contains("/tmp/skills-hub-git-123"));
}

#[test]
fn format_anyhow_error_github_hint_auth() {
    let err = anyhow::anyhow!("git clone https://github.com/a/b failed: authentication failed");
    let msg = format_anyhow_error(err);
    assert!(msg.contains("无法访问该仓库"));
}

#[test]
fn expand_home_path_basic() {
    let home = dirs::home_dir().expect("home");
    assert_eq!(expand_home_path("~").unwrap(), home);
    assert_eq!(expand_home_path("~/abc").unwrap(), home.join("abc"));
}

#[test]
fn expand_home_path_empty_is_error() {
    let err = expand_home_path("  ").unwrap_err().to_string();
    assert!(err.contains("storage path is empty"));
}

#[test]
fn saving_custom_tool_config_creates_enabled_skills_dir() {
    let (dir, store) = make_store();
    let existing = dir.path().join("existing-skills");
    std::fs::create_dir_all(&existing).unwrap();
    let created = dir.path().join("created-skills");
    assert!(!created.exists());

    save_tool_config(
        &store,
        ToolConfig {
            disabled_builtin_tools: Vec::new(),
            custom_tools: vec![
                CustomToolConfig {
                    key: "custom_existing".to_string(),
                    label: "Existing".to_string(),
                    avatar: Some("data:image/png;base64,AA==".to_string()),
                    skills_dir: existing.to_string_lossy().to_string(),
                    project_skills_dir: None,
                    sync_mode: SyncMode::Auto,
                    enabled: true,
                },
                CustomToolConfig {
                    key: "custom_created".to_string(),
                    label: "Created".to_string(),
                    avatar: None,
                    skills_dir: created.to_string_lossy().to_string(),
                    project_skills_dir: None,
                    sync_mode: SyncMode::Copy,
                    enabled: true,
                },
            ],
        },
    )
    .unwrap();
    assert!(created.is_dir());

    let tools = runtime_tools(&store, true).unwrap();
    let existing_tool = tools
        .iter()
        .find(|tool| tool.key == "custom_existing")
        .unwrap();
    let created_tool = tools
        .iter()
        .find(|tool| tool.key == "custom_created")
        .unwrap();

    assert!(existing_tool.enabled);
    assert!(existing_tool.installed);
    assert_eq!(
        existing_tool.avatar.as_deref(),
        Some("data:image/png;base64,AA==")
    );
    assert_eq!(existing_tool.sync_mode, SyncMode::Auto);
    assert!(created_tool.enabled);
    assert!(created_tool.installed);
    assert_eq!(created_tool.sync_mode, SyncMode::Copy);
}

#[test]
fn normalize_scope_defaults_to_global_and_rejects_unknown() {
    assert_eq!(normalize_scope(None).unwrap(), "global");
    assert_eq!(normalize_scope(Some("global")).unwrap(), "global");
    assert_eq!(normalize_scope(Some("project")).unwrap(), "project");
    assert!(normalize_scope(Some("workspace")).is_err());
}

#[test]
fn recent_projects_are_deduped_ordered_and_limited() {
    let (_dir, store) = make_store();
    let project_root = tempfile::tempdir().unwrap();
    let mut paths = Vec::new();
    for i in 0..9 {
        let path = project_root.path().join(format!("project-{i}"));
        std::fs::create_dir_all(&path).unwrap();
        paths.push(path);
    }

    for path in &paths {
        save_recent_project_impl(&store, path.to_string_lossy().as_ref()).unwrap();
    }

    let recent = get_recent_projects_impl(&store).unwrap();
    assert_eq!(recent.len(), 8);
    assert_eq!(recent[0], paths[8].to_string_lossy());
    assert_eq!(recent[7], paths[1].to_string_lossy());
    assert!(!recent.contains(&paths[0].to_string_lossy().to_string()));

    save_recent_project_impl(&store, paths[3].to_string_lossy().as_ref()).unwrap();
    let recent = get_recent_projects_impl(&store).unwrap();
    assert_eq!(recent.len(), 8);
    assert_eq!(recent[0], paths[3].to_string_lossy());
    assert_eq!(
        recent
            .iter()
            .filter(|item| *item == &paths[3].to_string_lossy())
            .count(),
        1
    );
}

#[test]
fn save_recent_project_rejects_missing_directory() {
    let (_dir, store) = make_store();
    let missing = tempfile::tempdir().unwrap().path().join("missing-project");
    let err = save_recent_project_impl(&store, missing.to_string_lossy().as_ref())
        .unwrap_err()
        .to_string();
    assert!(err.contains("projectPath must be an existing directory"));
}

#[test]
fn remove_path_any_handles_file_dir_and_missing() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("f.txt");
    std::fs::write(&file, b"1").unwrap();
    remove_path_any(file.to_string_lossy().as_ref()).unwrap();
    assert!(!file.exists());

    let sub = dir.path().join("d");
    std::fs::create_dir_all(&sub).unwrap();
    remove_path_any(sub.to_string_lossy().as_ref()).unwrap();
    assert!(!sub.exists());

    remove_path_any(dir.path().join("missing").to_string_lossy().as_ref()).unwrap();
}

#[test]
#[cfg(unix)]
fn remove_path_any_removes_symlink_only() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real");
    std::fs::create_dir_all(&target).unwrap();
    let link = dir.path().join("link");
    symlink(&target, &link).unwrap();

    remove_path_any(link.to_string_lossy().as_ref()).unwrap();
    assert!(!link.exists());
    assert!(target.exists());
}

#[test]
fn get_managed_skills_impl_maps_targets() {
    let (_dir, store) = make_store();
    let skill = SkillRecord {
        id: "s1".to_string(),
        name: "S1".to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: Some("/tmp/src".to_string()),
        source_subpath: None,
        source_revision: None,
        central_path: "/tmp/central".to_string(),
        content_hash: None,
        created_at: 1,
        updated_at: 2,
        last_sync_at: None,
        last_seen_at: 1,
        enabled: true,
        status: "ok".to_string(),
        collection: None,
    };
    store.upsert_skill(&skill).unwrap();

    let target = SkillTargetRecord {
        id: "t1".to_string(),
        skill_id: "s1".to_string(),
        tool: "cursor".to_string(),
        scope: "global".to_string(),
        project_path: None,
        target_path: "/tmp/target".to_string(),
        mode: "copy".to_string(),
        status: "ok".to_string(),
        last_error: None,
        synced_at: None,
    };
    store.upsert_skill_target(&target).unwrap();
    let tag = store.create_tag("Frontend").unwrap();
    store.set_skill_tags("s1", &[tag.id]).unwrap();

    let out = get_managed_skills_impl(&store).unwrap();
    assert_eq!(out.len(), 1);
    assert!(out[0].enabled);
    assert_eq!(out[0].tags.len(), 1);
    assert_eq!(out[0].tags[0].name, "Frontend");
    assert_eq!(out[0].targets.len(), 1);
    assert_eq!(out[0].targets[0].tool, "cursor");
    assert_eq!(out[0].targets[0].scope, "global");
    assert!(out[0].targets[0].project_path.is_none());
}
