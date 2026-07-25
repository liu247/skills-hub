# MCP Target Management Design

## Goal

Make each managed MCP service show and control its actual synchronization targets, while presenting local discovery as a readable source rather than an internal URI.

## User experience

- A local discovery source is labeled `Local import · Codex` (and the equivalent Chinese text), never `local://codex`.
- The service card lists the four supported Apps. Each App has one clear state: synced, not selected, or error.
- `Manage sync targets` opens a focused dialog with checkboxes for Codex, Claude Code, Kiro, and Reasonix.
- Confirming selection synchronizes newly selected targets. Removing a selection removes only this service from the corresponding App configuration and deletes its target record.
- A failed add or removal preserves the target record and reports the error; the remaining targets are unaffected.

## Architecture

- Keep the existing `McpServerTargetRecord` as the source of truth for each service/App relationship.
- Extend the existing MCP sync command with a target-reconciliation command that receives the complete selected target set, syncs additions, and unsyncs removals through host-specific config merging.
- The React MCP card renders target badges from `server.targets` and opens a modal that submits the selected target set.
- Source grouping converts `local://<host>` into a localized source identity before rendering.

## Safety

- Only the named MCP service is removed from a host configuration; all unrelated configuration remains intact.
- A target is removed from SQLite only after its host file was atomically updated successfully.
- Credential values remain in the OS credential store and are never returned to the UI.

## Verification

- Unit tests cover readable local source identities and target selection rendering.
- Rust tests cover host configuration removal for an owned server entry.
- Run frontend tests/build, Rust format/clippy/tests, build the 0.8.1 DMG, and validate it with `hdiutil verify`.
