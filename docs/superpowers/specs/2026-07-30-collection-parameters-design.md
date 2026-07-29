# Managed Skill Collection Parameters

## Goal

Let users configure the environment parameters required by a managed Skill collection from Skills Hub, keep sensitive values encrypted in the existing local credential store, and materialize the resolved values into each managed Skill's `.env` file before the normal target-sync flow.

## Scope

- Replace the collection-card pencil action with a collection parameter editor.
- Preserve collection rename as a secondary action in that editor.
- Let users manually add, edit, and remove collection parameter definitions for every collection.
- Extend AI Skill plans so an AI installation can propose source-backed parameter definitions.
- Store secret values through `LocalCredentialStore`; never return or display existing secret values.
- Store ordinary values in SQLite and make them editable in the UI.
- Write a managed `.env` file in every Skill directory belonging to the collection and re-sync existing targets after a parameter change.
- Show parameter readiness and a clear warning when a collection has unresolved required values.

## Non-goals

- No automatic discovery of parameters for Git/local/manual installs. Those collections start with no parameters and are configured manually.
- No attempt to infer or rewrite arbitrary agent-global configuration files.
- No support for arbitrary configuration formats such as TOML, YAML, JSON, or command-line flags in this release.
- No promise that a Skill will consume `.env`: Skills Hub writes the standard file, but each Skill must load environment variables or `.env` itself.
- No plaintext secret storage in SQLite DTOs, logs, AI plans, or UI state after submission.

## User experience

### Collection card and editor

The existing pencil button opens a focused **Collection parameters** dialog. It does not open a browser prompt. The dialog shows each parameter's name, optional description, type, value/readiness, and actions.

- A regular parameter has a visible value input.
- A sensitive parameter shows only `Not configured` or `Configured`; entering a replacement value overwrites the local encrypted credential, and a Clear action deletes it.
- Add parameter opens an inline row with name, description, and a Sensitive switch. Names must be unique in the collection and match a portable environment-variable identifier: uppercase letters, digits, and underscores, beginning with a letter or underscore.
- Users can edit descriptions and change a parameter between regular and sensitive. Changing to sensitive moves the supplied value into the encrypted store and clears the SQLite value; changing to regular requires a new visible value and removes the encrypted value only after the save succeeds.
- Remove parameter requires confirmation because it removes the generated entry and deletes any locally stored value.
- A readiness summary identifies required entries without a value. The editor still permits Save, but warns that generated Skill configuration is incomplete.
- Rename collection is available as a low-emphasis action inside the dialog. Renaming preserves all parameter definitions and values.

### Installation paths

For a normal Git, local, or manual install, the resulting collection has an empty parameter list. The user may open its editor and add parameters.

For an AI-assisted Skill install, the editable AI preview shows the proposed parameters. After the user confirms the plan and installation completes, Skills Hub creates the definitions but leaves every value unset. The collection card therefore immediately indicates that configuration is needed; no secret value appears in the plan or is requested from the AI.

### Runtime materialization and sync

On every saved parameter change, Skills Hub resolves values for that collection and writes a generated `.env` file into each managed Skill's central directory. It overwrites only the block owned by Skills Hub, delimited by stable comments; any user-authored content outside the block is retained. The generated block contains only currently configured values. Missing values are omitted and reported in the editor.

After materialization, Skills Hub re-runs the existing sync operation for every enabled target of every affected Skill. Link-based targets see the central change immediately; copy-based targets receive the updated directory through the existing overwrite-safe sync path. A partial sync failure identifies the affected Skill/target and leaves the parameter configuration saved for retry.

Because `.env` must be available to an external Skill process, a configured secret is necessarily materialized as plaintext in that local runtime file. Skills Hub marks this in the editor and limits the file to the managed local Skill directories; it never serializes secrets over IPC or displays them after saving.

## Data model

Collections are currently derived from `skills.collection` names. Add a stable `collection_id` UUID to the collection metadata so renamed collections retain their parameter owner identity. Migrate existing distinct non-empty collection names into collection rows, then backfill the same id to every member skill. New collections created during install or assignment receive a UUID. Existing string names remain the display/grouping field until the collection migration is complete.

Add two SQLite tables:

- `skill_collections`: `id`, unique `name`, `created_at`, and `updated_at`.
- `collection_parameters`: `collection_id`, `name`, `description`, `is_sensitive`, nullable `plain_value`, `created_at`, and `updated_at`, with unique `(collection_id, name)`.

Sensitive values use `LocalCredentialStore` with owner `collection:<collection_id>` and the parameter name as the key. The normal parameter record contains no secret value. Collection and parameter DTOs contain only `has_value` for sensitive entries.

Deleting a collection parameter deletes its encrypted value. Deleting or ungrouping every Skill does not automatically delete the collection configuration in this release; it remains available if the collection is rebuilt, and a future explicit collection-delete action may clean it up.

## AI plan protocol

Bump the AI plan protocol to `skills-hub-ai-plan/v2`. A `skill_plan` may include a `parameters` array:

```json
{
  "name": "SENSENOVA_API_KEY",
  "description": "SenseNova Platform API key used by image-generation commands.",
  "is_sensitive": true,
  "default_value": null,
  "required": true,
  "evidence": ["The source setup section lists SENSENOVA_API_KEY as required."]
}
```

The parser validates unique portable variable names, maximum lengths, and source evidence. A sensitive parameter must have no default value. An ordinary parameter may have a source-provided default value, but the user reviews and can edit it in the AI preview. AI output can only define parameters; it cannot supply a secret or cause a configuration file to be written before the user confirms the install.

## Backend and command boundaries

Core storage owns collection metadata, parameter CRUD, validation, value-state lookup, rename migration, and generated `.env` block rendering. It exposes a small sync coordinator that writes all affected managed files then invokes the existing per-target sync service.

The Tauri command layer maps the safe DTOs and calls core operations in `spawn_blocking`. Commands cover listing a collection with parameters, saving parameter definitions and values, clearing a sensitive value, deleting a parameter, and re-syncing the collection after materialization. Existing collection list results include a parameter readiness count so cards can show attention without revealing names or values.

## Frontend architecture

`CollectionsList` remains a presentational card grid and receives an `onConfigureCollection` callback for the pencil action. A new `CollectionParametersModal` owns only dialog-local editable rows and calls App-level callbacks for persistence. `App.tsx` remains the central state owner and reloads collections/managed Skills after successful saves.

The modal uses existing dialog, form-field, warning, and secondary/destructive button patterns. It supports keyboard close, visible focus, both themes, and English/Chinese copy. The card uses a compact warning badge/count only when required parameters are missing; it does not expose secret metadata on the landing grid.

## Error handling

- Invalid names, duplicate names, missing collection ownership, and unsafe state transitions return field-level errors.
- A credential write failure does not replace an existing secret or modify the generated file.
- `.env` write and sync failures identify the collection, Skill, and target while retaining the saved configuration for a retry.
- The application never puts secret values into toasts, error strings, debug output, or returned DTOs.

## Testing

- Rust store tests for migration, collection rename retention, parameter validation, secret value-state handling, deletion, and AI-plan validation.
- Rust materialization tests that preserve non-managed `.env` content, omit missing values, and never leak secret contents through safe status DTOs.
- Command tests that prove sensitive values cannot serialize to the frontend and failed credential updates retain prior values.
- Frontend tests for the collection pencil action, regular/sensitive field states, manual parameter creation, AI-plan parameter preview, and missing-configuration messaging.
- Run `npm run check` as the release gate.
