fn main() {
    if let Err(error) = app_lib::core::mcp_bridge::run_bridge_cli(std::env::args().skip(1)) {
        eprintln!("skills-hub-mcp-bridge: {error:#}");
        std::process::exit(1);
    }
}
