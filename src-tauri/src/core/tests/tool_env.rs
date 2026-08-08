use std::collections::BTreeMap;

use crate::core::tool_env::{
    read_global_env_at, remove_global_env_at, write_global_env_at, EnvConfigFormat,
};

fn managed(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn json_env_map_preserves_other_fields_and_removes_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(
        &path,
        r#"{
  "env": { "KEEP_ME": "keep" },
  "includeCoAuthoredBy": false
}"#,
    )
    .unwrap();

    write_global_env_at(
        &path,
        &EnvConfigFormat::JsonEnvMap,
        &managed(&[("SN_API_KEY", "v1")]),
    )
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"KEEP_ME\": \"keep\""));
    assert!(text.contains("\"SN_API_KEY\": \"v1\""));
    assert!(text.contains("\"includeCoAuthoredBy\": false"));

    let read = read_global_env_at(&path, &EnvConfigFormat::JsonEnvMap).unwrap();
    assert_eq!(read.get("KEEP_ME").map(String::as_str), Some("keep"));
    assert_eq!(read.get("SN_API_KEY").map(String::as_str), Some("v1"));

    remove_global_env_at(
        &path,
        &EnvConfigFormat::JsonEnvMap,
        &["SN_API_KEY".to_string()],
    )
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("SN_API_KEY"));
    assert!(text.contains("\"KEEP_ME\": \"keep\""));
}

#[test]
fn codex_policy_preserves_other_toml_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        "model = \"gpt-5\"\n\n[mcp_servers.tavily]\ncommand = \"node\"\n",
    )
    .unwrap();

    write_global_env_at(
        &path,
        &EnvConfigFormat::CodexShellPolicy,
        &managed(&[("SN_API_KEY", "v1")]),
    )
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("[shell_environment_policy]"));
    assert!(text.contains("SN_API_KEY = \"v1\""));
    assert!(text.contains("model = \"gpt-5\""));
    assert!(text.contains("[mcp_servers.tavily]"));

    let read = read_global_env_at(&path, &EnvConfigFormat::CodexShellPolicy).unwrap();
    assert_eq!(read.get("SN_API_KEY").map(String::as_str), Some("v1"));

    remove_global_env_at(
        &path,
        &EnvConfigFormat::CodexShellPolicy,
        &["SN_API_KEY".to_string()],
    )
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("SN_API_KEY"));
    assert!(text.contains("[mcp_servers.tavily]"));
}

#[test]
fn dotenv_file_keeps_comments_and_unrelated_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".env");
    std::fs::write(&path, "# existing\nEXISTING=1\n").unwrap();

    write_global_env_at(
        &path,
        &EnvConfigFormat::DotEnvFile,
        &managed(&[("SN_API_KEY", "v1")]),
    )
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("# existing"));
    assert!(text.contains("EXISTING=1"));
    assert!(text.contains("SN_API_KEY=v1"));

    // 更新已存在的 key，不重复追加
    write_global_env_at(
        &path,
        &EnvConfigFormat::DotEnvFile,
        &managed(&[("SN_API_KEY", "v2")]),
    )
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text.matches("SN_API_KEY=").count(), 1);
    assert!(text.contains("SN_API_KEY=v2"));

    remove_global_env_at(
        &path,
        &EnvConfigFormat::DotEnvFile,
        &["SN_API_KEY".to_string()],
    )
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("SN_API_KEY"));
    assert!(text.contains("# existing"));
    assert!(text.contains("EXISTING=1"));
}

#[test]
fn unsupported_tools_have_no_global_env_config() {
    for key in ["cursor", "kiro", "openclaw", "claude_3p"] {
        assert!(
            crate::core::tool_env::global_env_config_for(key).is_none(),
            "{key} should not claim global env support"
        );
    }
    assert!(crate::core::tool_env::global_env_config_for("claude_code").is_some());
    assert!(crate::core::tool_env::global_env_config_for("codex").is_some());
    assert!(crate::core::tool_env::global_env_config_for("reasonix").is_some());
    assert!(crate::core::tool_env::global_env_config_for("custom_reasonix").is_some());
}
