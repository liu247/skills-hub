# Workspace MCP and Collection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore collection-first Skills navigation and add a collection-consistent MCP workspace with GitHub configuration import and manual setup.

**Architecture:** Preserve the existing SQLite-backed MCP server model, credential bridge and target adapters. Extend records with optional provenance, add a scanner that converts GitHub configuration files into validated import candidates, then replace the standalone MCP form with workspace pages that reuse the existing Git selection pattern. Restore the existing collection state/render path that was removed in the v0.8 UI merge.

**Tech Stack:** Tauri 2, Rust/rusqlite/serde_json/toml, React 19/TypeScript/Vitest, lucide-react, i18next.

## Global Constraints

- User-visible copy must have English and Chinese i18n entries.
- MCP credential values use the OS credential store only; literal source secrets must never be persisted or displayed.
- Existing host sync behavior for Codex, Claude Code, Kiro and Reasonix must remain compatible.
- Version stays 0.8.1 unless explicitly requested otherwise.
- Run `npm run check` and build a macOS DMG before delivery.

---

### Task 1: Restore collection-first Skills workspace

**Files:**
- Modify: `src/App.tsx`, `src/components/skills/Header.tsx`, `src/components/skills/CollectionsList.tsx`, `src/i18n/resources.ts`
- Test: `src/components/skills/CollectionsList.test.tsx`

**Interfaces:**
- Consumes: `CollectionDto`, `ManagedSkill.collection`, existing `list_collections` command.
- Produces: collection landing and collection detail navigation from the workspace sidebar.

- [ ] Write a rendering test that asserts a collection landing renders repository/collection cards and does not render the flattened list until a collection is selected.
- [ ] Run the test and confirm it fails against the v0.8 flattened page.
- [ ] Restore collection state/loading, collection filtering and breadcrumb handling from `cc099b3`, adapting it to the current Header/App interfaces.
- [ ] Run the targeted test and `npm run build`.
- [ ] Commit with `feat(skills): restore collection-first workspace`.

### Task 2: Persist MCP provenance and parse GitHub configuration candidates

**Files:**
- Modify: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/mcp.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`
- Create: `src-tauri/src/core/mcp_import.rs`, `src-tauri/src/core/tests/mcp_import.rs`

**Interfaces:**
- Produces: `McpImportCandidateDto { source_url, source_path, servers }` and `scan_mcp_git_source(source_url) -> Vec<McpImportCandidateDto>`.
- Consumes: existing GitHub download/client configuration and `McpServerInput` validation.

- [ ] Write Rust tests for JSON/TOML candidate extraction and rejection of literal environment/header credentials.
- [ ] Run the new tests and confirm missing scanner failures.
- [ ] Add schema migration for nullable MCP source URL/path and expose source fields through server DTOs.
- [ ] Implement GitHub file discovery, JSON/TOML parsing, validation and scanner Tauri command.
- [ ] Run targeted Rust tests and `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Commit with `feat(mcp): scan GitHub configuration sources`.

### Task 3: Add MCP import data contracts and selection UI

**Files:**
- Modify: `src/components/skills/types.ts`, `src/App.tsx`, `src/i18n/resources.ts`, `src/App.css`
- Create: `src/components/skills/McpImportPage.tsx`, `src/components/skills/McpImportPage.test.tsx`

**Interfaces:**
- Consumes: `scan_mcp_git_source`, `upsert_mcp_server`, `set_mcp_secret` and MCP server DTO source fields.
- Produces: GitHub candidate selection and manual-config entry points.

- [ ] Write a React rendering test showing discovered candidates, server selection and an empty GitHub URL validation state.
- [ ] Run the test and confirm it fails before the page exists.
- [ ] Build the import page with GitHub/manual tabs and candidate preview, reusing the existing Skills Git candidate controls and visual language.
- [ ] Wire App state to scan, select, import, request OS-keychain values only after selection, and refresh managed MCP servers.
- [ ] Run targeted tests, lint and TypeScript build.
- [ ] Commit with `feat(mcp): add GitHub and manual import workflow`.

### Task 4: Promote MCP to a workspace feature

**Files:**
- Modify: `src/components/skills/Header.tsx`, `src/components/skills/McpPage.tsx`, `src/App.tsx`, `src/App.css`, `src/i18n/resources.ts`
- Test: `src/components/skills/Header.test.tsx`, `src/components/skills/McpPage.test.tsx`

**Interfaces:**
- Consumes: managed MCP servers, source metadata and import page callbacks.
- Produces: `MCP` and `Add MCP` workspace navigation; management center no longer contains MCP.

- [ ] Write rendering tests proving MCP/Add MCP appear in Workspace and MCP is absent from Management Center.
- [ ] Run them and confirm the current navigation fails.
- [ ] Update Header and App view types/routes; render managed MCP cards with source, targets, credentials status and sync action.
- [ ] Rework manual editing as a focused page/modal rather than an inline raw form and retain all existing operations.
- [ ] Run targeted frontend tests and build.
- [ ] Commit with `feat(mcp): promote management to workspace`.

### Task 5: Integration verification and macOS package

**Files:**
- Modify only if verification identifies a defect.

- [ ] Run `npm run check`; if sandbox blocks existing local-listener tests, rerun `npm run rust:test` with approved local execution.
- [ ] Build `npm run tauri:build -- --bundles app` and verify `Skills Hub.app/Contents/Info.plist` has executable `app` and version `0.8.1`.
- [ ] Create/verify the DMG includes `Skills Hub.app` and an `Applications -> /Applications` link.
- [ ] Commit any final package-safe code fixes and report the artifact path.
