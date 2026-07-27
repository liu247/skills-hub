use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::credential_store::CredentialStore;
use super::mcp::{
    credential_name_from_reference, is_credential_name, validate_mcp_server_input, McpServerInput,
    McpTransport,
};
use super::skill_store::SkillStore;

pub const AI_CREDENTIAL_OWNER: &str = "ai-provider";
pub const AI_PLAN_PROTOCOL_VERSION: &str = "skills-hub-ai-plan/v1";
const AI_PROVIDER_SETTING_PREFIX: &str = "ai_provider_config_v1_";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "deepseek")]
    DeepSeek,
    #[serde(rename = "kimi")]
    Kimi,
}

impl AiProvider {
    pub const ALL: [Self; 3] = [Self::OpenAi, Self::DeepSeek, Self::Kimi];

    pub fn key(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::DeepSeek => "deepseek",
            Self::Kimi => "kimi",
        }
    }

    pub fn default_config(self) -> AiProviderConfig {
        match self {
            Self::OpenAi => {
                AiProviderConfig::new(self, "gpt-4.1-mini", "https://api.openai.com/v1")
            }
            Self::DeepSeek => {
                AiProviderConfig::new(self, "deepseek-chat", "https://api.deepseek.com/v1")
            }
            Self::Kimi => {
                AiProviderConfig::new(self, "moonshot-v1-8k", "https://api.moonshot.cn/v1")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiProviderConfig {
    pub provider: AiProvider,
    pub enabled: bool,
    pub model: String,
    pub base_url: String,
}

impl AiProviderConfig {
    fn new(provider: AiProvider, model: &str, base_url: &str) -> Self {
        Self {
            provider,
            enabled: false,
            model: model.to_string(),
            base_url: base_url.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiProviderConfigStatus {
    pub provider: AiProvider,
    pub enabled: bool,
    pub model: String,
    pub base_url: String,
    pub has_api_key: bool,
}

fn setting_key(provider: AiProvider) -> String {
    format!("{AI_PROVIDER_SETTING_PREFIX}{}", provider.key())
}

pub fn save_provider_config(store: &SkillStore, config: AiProviderConfig) -> Result<()> {
    if config.model.trim().is_empty() {
        anyhow::bail!("AI provider model cannot be empty");
    }
    let parsed =
        reqwest::Url::parse(config.base_url.trim()).context("AI provider base URL is invalid")?;
    if !matches!(parsed.scheme(), "https" | "http") || parsed.host_str().is_none() {
        anyhow::bail!("AI provider base URL must be an HTTP URL");
    }
    store.set_setting(
        &setting_key(config.provider),
        &serde_json::to_string(&config)?,
    )
}

pub fn get_provider_configs(
    store: &SkillStore,
    credentials: &dyn CredentialStore,
) -> Result<Vec<AiProviderConfigStatus>> {
    AiProvider::ALL
        .into_iter()
        .map(|provider| {
            let config = store
                .get_setting(&setting_key(provider))?
                .map(|raw| serde_json::from_str(&raw).context("parse AI provider configuration"))
                .transpose()?
                .unwrap_or_else(|| provider.default_config());
            Ok(AiProviderConfigStatus {
                provider,
                enabled: config.enabled,
                model: config.model,
                base_url: config.base_url,
                has_api_key: credentials
                    .get(AI_CREDENTIAL_OWNER, provider.key())?
                    .is_some(),
            })
        })
        .collect()
}

pub fn set_provider_api_key(
    credentials: &dyn CredentialStore,
    provider: AiProvider,
    value: &str,
) -> Result<()> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("AI provider API key cannot be empty");
    }
    credentials.set(AI_CREDENTIAL_OWNER, provider.key(), value)
}

pub fn delete_provider_api_key(
    credentials: &dyn CredentialStore,
    provider: AiProvider,
) -> Result<()> {
    credentials.delete(AI_CREDENTIAL_OWNER, provider.key())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiPlanKind {
    Skill,
    Mcp,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiSourceEvidence {
    pub url: String,
    #[serde(default)]
    pub path: Option<String>,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiSkillPlan {
    pub name: String,
    pub source_url: String,
    #[serde(default)]
    pub subpath: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiMcpPlan {
    pub name: String,
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub recommended_targets: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiParsePlan {
    pub protocol_version: String,
    pub kind: AiPlanKind,
    pub summary: String,
    pub source: AiSourceEvidence,
    pub confidence: String,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub skill_plan: Option<AiSkillPlan>,
    #[serde(default)]
    pub mcp_plan: Option<AiMcpPlan>,
}

fn validate_http_url(value: &str, label: &str) -> Result<()> {
    let parsed =
        reqwest::Url::parse(value.trim()).with_context(|| format!("{label} is invalid"))?;
    if !matches!(parsed.scheme(), "https" | "http") || parsed.host_str().is_none() {
        anyhow::bail!("{label} must be an HTTP URL");
    }
    Ok(())
}

fn normalize_credential_reference(value: &str) -> Option<String> {
    if credential_name_from_reference(value).is_some() {
        return Some(value.to_string());
    }
    let name = value.strip_prefix("${credential:")?.strip_suffix('}')?;
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return None;
    }
    Some(format!("${{{name}}}"))
}

fn normalize_mcp_secret_references(plan: &mut AiMcpPlan) -> Result<()> {
    for (name, value) in &mut plan.env {
        if is_credential_name(name) {
            *value = normalize_credential_reference(value).ok_or_else(|| {
                anyhow::anyhow!("MCP credential values must use a credential reference")
            })?;
            if credential_name_from_reference(value).is_none() {
                anyhow::bail!("MCP credential values must use a credential reference");
            }
        }
    }
    for value in plan.headers.values_mut() {
        *value = normalize_credential_reference(value)
            .ok_or_else(|| anyhow::anyhow!("MCP HTTP headers must use a credential reference"))?;
    }
    Ok(())
}

pub fn validate_ai_plan_json(value: &str) -> Result<AiParsePlan> {
    let mut plan: AiParsePlan =
        serde_json::from_str(value).context("AI returned invalid plan JSON")?;
    if plan.protocol_version != AI_PLAN_PROTOCOL_VERSION {
        anyhow::bail!("AI plan protocol version is not supported");
    }
    if plan.summary.trim().is_empty() || plan.source.evidence.is_empty() {
        anyhow::bail!("AI plan must include a summary and source evidence");
    }
    validate_http_url(&plan.source.url, "AI plan source URL")?;
    match plan.kind {
        AiPlanKind::Unknown => {
            if plan.skill_plan.is_some() || plan.mcp_plan.is_some() {
                anyhow::bail!("unknown AI plan cannot include a configuration plan");
            }
        }
        AiPlanKind::Skill => {
            let skill = plan
                .skill_plan
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("skill AI plan is missing skill_plan"))?;
            if plan.mcp_plan.is_some() || skill.name.trim().is_empty() {
                anyhow::bail!("skill AI plan is invalid");
            }
            validate_http_url(&skill.source_url, "Skill source URL")?;
        }
        AiPlanKind::Mcp => {
            let mcp = plan
                .mcp_plan
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("MCP AI plan is missing mcp_plan"))?;
            if plan.skill_plan.is_some() {
                anyhow::bail!("MCP AI plan cannot include skill_plan");
            }
            normalize_mcp_secret_references(mcp)?;
            let transport = match mcp.transport.as_str() {
                "stdio" => McpTransport::Stdio,
                "http" => McpTransport::Http,
                _ => anyhow::bail!("AI plan has unsupported MCP transport"),
            };
            validate_mcp_server_input(&McpServerInput {
                name: mcp.name.clone(),
                transport,
                command: mcp.command.clone(),
                args: mcp.args.clone(),
                env: mcp.env.clone(),
                url: mcp.url.clone(),
                headers: mcp.headers.clone(),
            })?;
        }
    }
    Ok(plan)
}
