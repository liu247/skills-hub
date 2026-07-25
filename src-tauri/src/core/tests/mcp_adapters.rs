use crate::core::mcp_adapters::{render_server, McpHost};
use crate::core::skill_store::McpServerRecord;

#[test]
fn secret_bearing_stdio_renders_bridge_for_all_hosts() {
    let mut server = McpServerRecord::stdio("github-id", "github", "npx", vec!["-y".into()]);
    server
        .env
        .insert("GITHUB_TOKEN".into(), "${GITHUB_TOKEN}".into());

    for host in [
        McpHost::Codex,
        McpHost::ClaudeCode,
        McpHost::Kiro,
        McpHost::Reasonix,
    ] {
        let rendered = render_server(host, &server, Some(8765)).unwrap();
        assert!(rendered.contains("skills-hub-mcp-bridge"));
        assert!(!rendered.contains("GITHUB_TOKEN"));
    }
}

#[test]
fn secret_bearing_http_renders_loopback_url_for_all_hosts() {
    let mut server = McpServerRecord::stdio("stripe-id", "stripe", "unused", vec![]);
    server.transport = "http".into();
    server.command = None;
    server.url = Some("https://mcp.stripe.com".into());
    server
        .headers
        .insert("Authorization".into(), "${STRIPE_KEY}".into());

    for host in [
        McpHost::Codex,
        McpHost::ClaudeCode,
        McpHost::Kiro,
        McpHost::Reasonix,
    ] {
        let rendered = render_server(host, &server, Some(8765)).unwrap();
        assert!(rendered.contains("127.0.0.1:8765"));
        assert!(!rendered.contains("STRIPE_KEY"));
    }
}
