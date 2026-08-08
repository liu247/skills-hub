pub mod ai_parser;
pub mod auto_update;
pub mod cache_cleanup;
pub mod cancel_token;
pub mod central_repo;
pub mod content_hash;
pub mod credential_store;
pub mod featured_skills;
pub mod git_fetcher;
pub mod github_download;
pub mod github_search;
pub mod installer;
pub mod mcp;
pub mod mcp_adapters;
pub mod mcp_bridge;
pub mod mcp_discovery;
pub mod mcp_import;
pub mod network_proxy;
pub mod onboarding;
pub mod skill_files;
pub mod skill_store;
pub mod skills_search;
pub mod sync_engine;
pub mod system_scheduler;
pub mod temp_cleanup;
pub mod tool_adapters;
pub mod tool_env;

#[cfg(test)]
#[path = "tests/mcp.rs"]
mod mcp_tests;

#[cfg(test)]
#[path = "tests/mcp_import.rs"]
mod mcp_import_tests;

#[cfg(test)]
#[path = "tests/mcp_discovery.rs"]
mod mcp_discovery_tests;

#[cfg(test)]
#[path = "tests/credential_store.rs"]
mod credential_store_tests;

#[cfg(test)]
#[path = "tests/mcp_bridge.rs"]
mod mcp_bridge_tests;

#[cfg(test)]
#[path = "tests/mcp_adapters.rs"]
mod mcp_adapters_tests;

#[cfg(test)]
#[path = "tests/ai_parser.rs"]
mod ai_parser_tests;

#[cfg(test)]
#[path = "tests/tool_env.rs"]
mod tool_env_tests;
