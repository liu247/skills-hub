# MCP Source Workspace Design

## Goal

Make MCP management feel like the existing collection-first Skills workspace: users land on source collections, not a sparse raw configuration form.

## Scope

- The MCP workspace reuses the existing Skills page hierarchy: summary cards, compact filter toolbar, grid/list view controls and source collection cards.
- A source collection represents an imported GitHub repository/configuration URL, or the shared `Manual configuration` source for manually created services.
- Selecting a source collection opens only its services. The detail list exposes transport, target-app sync state, credential readiness and per-service actions.
- Add MCP is a discovery page: GitHub configuration URL is the primary workflow and manual configuration remains a clearly secondary action.
- Existing MCP persistence, OS credential storage, source metadata, four host adapters and sync behavior stay unchanged.

## Information architecture

### MCP landing

1. Summary cards show managed server count, connected app targets, credentials requiring attention and overall sync health.
2. The toolbar provides search, source/status filtering, sort order, batch sync, view mode and one primary `Add MCP` action.
3. The default grid shows source collection cards in the same three-column geometry and visual vocabulary as Skills collections. Each card has a source icon/name, service count and recent activity. No commands, URLs or secret names appear in the grid.
4. Search returns matching services directly to preserve fast lookup. Selecting a source opens its filtered service list.

### Source detail

- A breadcrumb returns to `MCP` collections.
- Service rows retain operational metadata and actions. Source URL/path is visible as secondary provenance, not the primary hierarchy.
- A source-level sync action synchronizes every service in that source to Codex, Claude Code, Kiro and Reasonix.

### Add MCP

- The page uses a titled discovery surface and a single compact GitHub URL input with a primary scan action.
- Discovered configurations are presented as selectable candidate cards.
- `Manual configuration` is a secondary outlined action which opens the existing service editor.
- Empty, loading and validation states remain within the content region; they never replace the workspace shell.

## Visual rules

- Reuse existing semantic tokens, `stats-grid`, toolbar controls and `collection-card` dimensions; do not introduce a second design language.
- Retain compact operational density and subtle surfaces/borders.
- The accent color marks the active source, focused controls and the primary action only.
- English and Chinese copy must be supplied together. Collapsed sidebar navigation remains icon-only with accessible labels.

## Verification

- React tests prove source grouping, source detail navigation and GitHub/manual import controls.
- Tests assert source cards use real server data and that manually created servers form a single source collection.
- `npm run check` and an arm64 macOS DMG build/verification run before delivery.
