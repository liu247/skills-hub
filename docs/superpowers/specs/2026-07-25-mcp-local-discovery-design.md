# MCP Local Discovery Design

## Goal

Let users discover pre-existing local MCP configurations and deliberately bring selected services under Skills Hub management, using the same scan-review-import model as Skills onboarding.

## Scan scope

- Scan global MCP files for Codex (`~/.codex/config.toml`), Claude Code (`~/.claude.json`), Kiro (`~/.kiro/settings/mcp.json`) and Reasonix (`~/.reasonix/config.toml`).
- Parse each host's existing configuration format and extract only MCP server definitions.
- Exclude entries already owned by Skills Hub, identified by the persisted MCP target for the host and server id/name. The scan is read-only.

## Review model

- Group candidates by server name.
- Equivalent definitions in multiple apps form one candidate and list their source apps.
- Differing definitions under the same name are marked as a conflict. The user selects the authoritative variant before import.
- The review dialog supports search, select-all, individual selection and per-conflict variant choice, matching Skills import behavior.
- A toolbar action on the MCP landing explicitly starts a new scan; scanning never runs automatically.

## Import and credentials

- Only selected candidates are persisted as MCP servers and synchronized to the apps in which they were discovered.
- Literal credential values are never displayed in the review UI or persisted in SQLite.
- When a selected server contains a literal environment value or HTTP header credential, the confirmation path migrates that value into the OS credential store, creates the corresponding `${NAME}` reference, then synchronizes the normalized configuration. The user has explicitly authorized this migration by selecting the candidate and confirming import.
- `${NAME}` references remain references and are resolved from the credential store through the existing bridge. If a value is absent, the UI prompts the user after import.
- Unselected entries remain untouched. A failed import does not modify host config files.

## Verification

- Rust tests cover JSON and TOML local scan parsing, grouping consistent/conflicting variants, ignoring managed targets and sanitizing literal credentials for migration.
- React tests cover the scan entry point and review selection/conflict rendering.
- Full check and macOS DMG build run before delivery.
