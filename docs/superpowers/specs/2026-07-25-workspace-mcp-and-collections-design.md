# Workspace MCP and Collection Design

## Goal

Restore the Skills Hub home page as a collection-first workspace and make MCP a first-class workspace feature with GitHub configuration import and manual setup.

## Information architecture

- The workspace sidebar contains `My Skills`, `Add Skills`, `MCP` and `Add MCP`.
- `My Skills` opens a collection landing page grouped by the source repository. Selecting a collection opens its skills; search intentionally flattens results across all collections.
- `MCP` opens the managed-server landing page. `Add MCP` opens an import page that mirrors `Add Skills` with GitHub and manual tabs.
- Management Center remains limited to tags, tools and updates.

## Collection behavior

- Existing explicit skill collections remain supported.
- Skills without an explicit collection are grouped by canonical GitHub repository when `source_ref` identifies one; local or unknown sources remain in the uncategorized collection.
- The collection landing shows repository identity, skill count and latest update. It does not show every skill at once.
- Filters, tags and direct search retain their existing semantics. A non-empty search displays the matching skills directly so a user can find a skill without entering a collection.

## MCP import behavior

- GitHub import accepts the same repository/tree/blob URL shapes as Skills import.
- The backend downloads/scans only configuration candidates. It recognizes common MCP JSON and TOML file names and extracts `mcpServers`/`mcp_servers` entries.
- A candidate contains its source file path and one or more validated server definitions. The user selects individual servers before import.
- Import persists only the server definition and its source URL/path metadata. Any literal secret in a source configuration is rejected; `${NAME}` references become credential references and are filled through the existing OS credential store.
- Manual setup remains available for stdio and HTTP transports and uses the same validation, target selection and credential handling as imported servers.

## Sync and update behavior

- A newly imported or manually created server can be synced to Codex, Claude Code, Kiro and Reasonix from its card.
- Existing per-host rendering, atomic backup, loopback proxy and OS credential-store behavior remain unchanged.
- The source link is displayed for imported servers. This first version records source provenance; automatic MCP update detection is explicitly out of scope.

## Verification

- Rust tests cover parsing, rejecting literal credentials, candidate discovery and persistence conversion.
- React tests cover workspace navigation, collection landing behavior and MCP candidate selection rendering.
- Full `npm run check` and a fresh macOS app/DMG build are required before delivery.
