// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments
        .first()
        .is_some_and(|argument| argument == "--mcp-bridge")
    {
        if let Err(error) = app_lib::core::mcp_bridge::run_bridge_cli(arguments.into_iter().skip(1))
        {
            eprintln!("skills-hub MCP credential bridge: {error:#}");
            std::process::exit(1);
        }
        return;
    }
    app_lib::run();
}
