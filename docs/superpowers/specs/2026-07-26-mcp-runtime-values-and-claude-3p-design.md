# MCP Runtime Values and Claude-3p Design

## Goal

Prevent ordinary MCP runtime variables from being treated as missing credentials, repair existing local imports from their atomic configuration backups, and expose Claude-3p as a target distinct from Claude Code.

## Credential classification

- A literal environment value whose name signals a secret (`*_KEY`, `*_TOKEN`, `*_SECRET`, `*_PASSWORD`, or `*_API_KEY`) is moved to the OS credential store and stored as a `${NAME}` reference.
- Literal values for other environment names are runtime values. They are persisted in the MCP definition and supplied directly to stdio processes; they never use the credential bridge.
- Literal HTTP headers remain credentials unless they are explicitly safe metadata headers.
- Existing `${NAME}` references continue to use the credential bridge.

## Existing local imports

- A repair command finds the newest atomic backup adjacent to a locally sourced configuration file.
- It reads only the corresponding MCP service from that backup, restores runtime values and credentials with the new classification, then re-syncs the service to its selected targets.
- If no backup exists, the command reports that the original local configuration cannot be recovered; it never invents values.

## Claude-3p

- Add a distinct MCP host and target key `claude_3p`.
- Its global configuration is `~/Library/Application Support/Claude-3p/claude_desktop_config.json`.
- It shares the JSON MCP server format with Claude Code, but has independent scan, target, and source labels.

## Verification

- Unit tests prove `NODE_PATH` and `MCP_PDF_ALLOWED_PATHS` stay runtime values while `TAVILY_API_KEY` is a credential reference.
- Adapter tests prove Claude-3p uses its own configuration path and JSON rendering.
- Run full frontend/Rust checks and validate a rebuilt 0.8.1 DMG.
