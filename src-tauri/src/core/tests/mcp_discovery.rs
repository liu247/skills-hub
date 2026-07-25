use std::fs;
use tempfile::tempdir;

use super::mcp_discovery::{scan_local_mcp_configs_in, scan_local_mcp_configs_with_secrets_in};

#[test]
fn groups_matching_servers_and_marks_different_variants_as_conflicts() {
    let home = tempdir().unwrap();
    fs::create_dir_all(home.path().join(".codex")).unwrap();
    fs::write(home.path().join(".codex/config.toml"), "[mcp_servers.files]\ncommand = \"npx\"\nargs = [\"-y\", \"files\"]\n[mcp_servers.other]\ncommand = \"node\"\n").unwrap();
    fs::write(home.path().join(".claude.json"), r#"{"mcpServers":{"files":{"command":"npx","args":["-y","files"]},"other":{"command":"node","args":["x.js"]}}}"#).unwrap();
    let plan = scan_local_mcp_configs_in(home.path()).unwrap();
    assert_eq!(plan.total_servers_found, 4);
    assert!(
        !plan
            .groups
            .iter()
            .find(|group| group.name == "files")
            .unwrap()
            .has_conflict
    );
    assert!(
        plan.groups
            .iter()
            .find(|group| group.name == "other")
            .unwrap()
            .has_conflict
    );
}

#[test]
fn keeps_literal_credentials_private_while_preserving_them_for_selected_import() {
    let home = tempdir().unwrap();
    fs::create_dir_all(home.path().join(".codex")).unwrap();
    fs::write(
        home.path().join(".codex/config.toml"),
        "[mcp_servers.private]\ncommand = \"npx\"\n[mcp_servers.private.env]\nAPI_TOKEN = \"local-secret\"\n",
    )
    .unwrap();

    let plan = scan_local_mcp_configs_in(home.path()).unwrap();
    let variant = &plan.groups[0].variants[0];
    assert_eq!(variant.env.get("API_TOKEN").unwrap(), "${API_TOKEN}");
    assert!(!format!("{plan:?}").contains("local-secret"));

    let discovery = scan_local_mcp_configs_with_secrets_in(home.path()).unwrap();
    let selected = discovery.select("private", "codex").unwrap();
    assert_eq!(
        selected.literal_credentials.get("API_TOKEN"),
        Some(&"local-secret".to_string())
    );
}
