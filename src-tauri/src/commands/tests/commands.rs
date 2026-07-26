use super::*;
use crate::core::credential_store::{CredentialStore, MemoryCredentialStore};
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
    let credentials = MemoryCredentialStore::default();

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
