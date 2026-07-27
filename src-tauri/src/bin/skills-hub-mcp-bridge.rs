fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    #[cfg(unix)]
    if arguments
        .first()
        .is_some_and(|argument| argument == "--mcp-credential-agent")
    {
        if let Err(error) =
            app_lib::core::mcp_bridge::run_credential_agent_cli(arguments.into_iter().skip(1))
        {
            eprintln!("skills-hub MCP credential agent: {error:#}");
            std::process::exit(1);
        }
        return;
    }
    if let Err(error) = app_lib::core::mcp_bridge::run_bridge_cli(arguments) {
        eprintln!("skills-hub-mcp-bridge: {error:#}");
        std::process::exit(1);
    }
}
