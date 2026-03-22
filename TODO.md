# Implementation TODO

# Phase 1: Read-Only Server

## 1.1: Project Skeleton
- [x] Run `cargo init` to create Rust project
- [x] Set up `Cargo.toml` with Phase 1 dependencies (rmcp, serde, serde_json, schemars, flate2, sha2, notify, anyhow, thiserror, tokio)
- [x] Create module directory structure: `src/lk/`, `src/prosemirror/`
- [x] Create `tests/reference/` directory, move `rime.lk` and `siqram.lk` there
- [x] Update `.gitignore` for `tests/reference/*.lk`
- [x] Create all `mod.rs` files with placeholder re-exports
- [x] Write minimal `main.rs` that resolves worlds directory and prints usage
- [x] Verify `cargo build` succeeds

## 1.2: Schema + File I/O
- [x] Define `LkRoot` struct in `src/lk/schema.rs` with all fields (version, exportId, exportedAt, resources, calendars, resourceCount, hash)
- [x] Define `Resource` struct with all 17 fields (schemaVersion, id, name, parentId, pos, createdBy, isHidden, isLocked, showPropertyBar, iconColor, iconGlyph, iconShape, aliases, tags, documents, properties, banner)
- [x] Define `Document` struct with all fields including optional content, map, calendarId, isFullWidth, and typed presentation
- [x] Define `Property` struct with type-agnostic `data: Value`, plus `isHidden` and `isTitleHidden` fields
- [x] Define `Banner` and `MapData` structs
- [x] Define `Presentation` struct with `documentType`, optional `calibration`, `defaultMode`, `disallowedModes`
- [x] Define `Calibration` struct
- [x] Define `Calendar` struct with all fields (id, name, hasZeroYear, maxMinutes, months, leapDays, weekdays, epochWeekday, weekResetsEachMonth, hoursInDay, minutesInHour, negativeEra, positiveEras, format, halfClock)
- [x] Define `Month`, `Weekday`, `Era`, `CalendarFormat` structs
- [x] Define `TimelineContent` struct with lanes and events
- [x] Define `Lane` and `TimelineEvent` structs
- [x] Define `Source` struct (id, uri, type, createdAt, updatedAt, resourceId, documentId)
- [x] Implement `read_lk_file()` in `src/lk/io.rs` — open file, GzDecoder, serde_json::from_reader
- [x] **Integration test**: deserialize every `.lk` file in `tests/reference/`, verify no errors, log resource/calendar counts
- [x] Fix any deserialization issues discovered by the tests

## 1.3: World Store (Multi-World + Hot-Reload)
- [x] Implement `WorldStore` struct in `src/lk/store.rs` with `Arc<RwLock<HashMap<String, LkRoot>>>` and `PathBuf`
- [x] Implement `WorldStore::load(dir)` — scan directory for `.lk` files, read each, key by filename stem
- [x] Implement file watcher using `notify` crate — watch worlds directory, reload on add/modify/remove
- [x] World name derived from filename stem: `rime.lk` → `"rime"`
- [x] Implement `list_worlds()` — return list of loaded world names with resource/calendar counts
- [x] Implement `resolve_world(world?)` — if only one world loaded and param omitted, use it; otherwise require param
- [x] Implement `list_resources(world, tag, name)` — filter by tag (exact) and/or name (case-insensitive substring)
- [x] Implement `get_resource(world, id_or_name)` — lookup by ID first, fallback to case-insensitive name
- [x] Implement `get_resource_tree(world, root_id)` — build nested tree from parentId relationships
- [x] Implement `search_content(world, query, limit)` — extract plain text from ProseMirror + timeline event names, substring search
- [x] Implement `get_calendar(world, id_or_name)` — lookup by ID first, fallback to case-insensitive name
- [x] Define `LkError` enum in `src/lk/mod.rs` with variants: WorldNotFound, ResourceNotFound, CalendarNotFound, InvalidInput, Io, Json

## 1.4: ProseMirror-to-Markdown Converter
- [x] Define `PmNode` and `PmMark` types in `src/prosemirror/types.rs`
- [x] Implement `to_markdown(node: &Value) -> String` in `src/prosemirror/to_markdown.rs`
- [x] Handle `doc` node (root container, recurse)
- [x] Handle `paragraph` (render children + double newline)
- [x] Handle `heading` (# × level + children + double newline)
- [x] Handle `text` node with mark wrapping (strong→**, em→*, code→backtick, link→[](), strikethrough→~~, underline→<u>)
- [x] Handle `bulletList` / `orderedList` / `listItem` with proper indentation for nesting
- [x] Handle `taskList` / `taskItem` (- [ ] / - [x])
- [x] Handle `blockquote` (> prefix on each line)
- [x] Handle `codeBlock` (``` fenced code block with optional language attr)
- [x] Handle `rule` (---)
- [x] Handle `hardBreak` (newline)
- [x] Handle `table` / `tableRow` / `tableHeader` / `tableCell` (GFM table with | separators and --- header row)
- [x] Handle `mention` → `[[attrs.text]]`
- [x] Handle `mediaSingle` / `media` → `![](attrs.url)`
- [x] Handle `layoutSection` / `layoutColumn` → flatten, render children sequentially
- [x] Handle `panel` → blockquote with panelType prefix
- [x] Handle `extension` / `bodiedExtension` → render children or extract text attr
- [x] Handle unknown nodes → recurse into children silently
- [x] Test against actual ProseMirror content from reference `.lk` files

## 1.5: MCP Server + Read Tools
- [x] Implement `LkServer` struct in `src/server.rs` holding WorldStore
- [x] Implement `ServerHandler` for `LkServer` with `get_info()` returning server metadata
- [x] Wire `list_worlds` tool — calls store, returns world summaries
- [x] Wire `list_resources` tool — calls store, returns JSON array of summaries
- [x] Wire `get_resource` tool — calls store, converts page content to markdown, renders timeline docs with lane/event summaries
- [x] Wire `get_resource_tree` tool — calls store, returns nested JSON tree
- [x] Wire `search_content` tool — calls store, returns matching snippets
- [x] Wire `get_calendar` tool — calls store, returns calendar definition
- [x] Update `main.rs` to create LkServer and start rmcp stdio transport
- [x] **Test**: pipe a `tools/list` JSON-RPC request through stdin, verify all 6 tools appear
- [x] **Test**: pipe a `tools/call` for `list_worlds`, verify response

## 1.6: Polish & Integration
- [x] Add startup logging to stderr (worlds dir, world count, per-world resource/calendar counts)
- [x] Log hot-reload events to stderr (file added/modified/removed)
- [x] Handle edge cases: empty worlds directory (no .lk files), resource with no documents, empty content
- [x] Verify `get_resource` name lookup handles multiple resources with same name gracefully
- [x] Verify `get_calendar` returns useful calendar structure
- [x] Verify world omission works when only one world loaded
- [x] Test hot-reload: start server, drop new .lk file in directory, verify `list_worlds` reflects it
- [ ] Test full flow: configure as MCP server in Claude Code, use tools interactively

## 1.7: Map & Image Awareness
- [x] Add `MapFeature` struct to `schema.rs` with optional fields for pin/region/label/path types
- [x] Add `MapContent` struct to `schema.rs`
- [x] Implement `format_map_document()` in `server.rs` — shared renderer for map docs
- [x] Render pins as markdown table with name, position, icon, linked resource ID
- [x] Render regions with full vertex coordinates and fill/border style
- [x] Render paths with full waypoint coordinates and stroke style
- [x] Render labels with size and position
- [x] Include calibration scale when present on map presentation
- [x] Parse `lk://resources/{id}/docs/{id}` URIs to extract resource IDs for pin links
- [x] Update `format_resource()` to use `format_map_document()` for map documents
- [x] Add `get_map` tool — lookup resource, find map document, render with `format_map_document()`
- [x] Verify deserialization still passes with new MapFeature/MapContent types
- [x] Test map pin deserialization against reference data (18 pins across 15 maps in Rime)
- [ ] Test with Claude Desktop: "What locations are on the main map?" → gets pin names and positions

## 1.8: World Instructions (llm-guide)
- [x] Add `guide: Option<String>` to `WorldSummary` in `store.rs`
- [x] Implement `extract_world_guide()` — scan resources for `llm-guide` tag, return first page doc as markdown
- [x] Include guide in `list_worlds` response
- [x] Update `list_worlds` tool description to mention guide field
- [x] Test guide detection runs without error on reference worlds (no llm-guide tags present)
- [ ] Test with a world that has a resource tagged `llm-guide`

---

# Phase 2: World Generation (.lk File Export)

## 2.1: File Output
- [x] Add Phase 2 dependencies to Cargo.toml (comrak, chrono, rand)
- [x] Implement `write_lk_file()` in `src/lk/io.rs` — GzEncoder, serde_json::to_writer, write to output path
- [x] Implement hash computation (SHA-256 of compact JSON resources array)
- [x] Implement `generate_id()` — 8-char lowercase alphanumeric random string
- [x] **Test**: write a minimal LkRoot to `.lk`, read it back, verify all fields survive

## 2.2: Markdown-to-ProseMirror Converter
- [x] Implement `from_markdown(md: &str, resources: &[Resource]) -> Value` in `src/prosemirror/from_markdown.rs`
- [x] Use `comrak` to parse markdown into AST
- [x] Convert comrak paragraph → PM paragraph
- [x] Convert comrak heading → PM heading with level attr
- [x] Convert comrak list → PM bulletList/orderedList + listItem
- [x] Convert comrak table → PM table + tableRow + tableHeader/tableCell
- [x] Convert headerless pipe tables (LLM-generated) → header-column tables (first cell = tableHeader on every row)
- [x] Convert comrak blockquote → PM blockquote
- [x] Convert comrak thematic break → PM rule
- [x] Convert comrak image → PM mediaSingle + media with external type
- [x] Convert comrak link → PM text with link mark
- [x] Convert comrak emphasis/strong → PM text with em/strong marks
- [x] Convert comrak code/code_block → PM text with code mark / codeBlock node
- [x] Detect `[[Resource Name]]` in text → split into text + PM mention node (resolve name→ID from resources list) + text
- [x] Convert comrak task list items → PM taskList + taskItem with state attr (inline content, not wrapped in paragraph)
- [x] Handle comrak softbreak/linebreak → PM hardBreak
- [ ] **Test**: round-trip — take ProseMirror from reference `.lk`, convert to markdown, convert back, verify structural equivalence

## 2.3: WorldBuilder
- [x] Implement `WorldBuilder` struct in `src/lk/builder.rs` — holds an in-progress `LkRoot` in memory
- [x] `WorldBuilder::new(name)` — create empty world with generated exportId, version=1, empty resources/calendars
- [x] `create_resource(name, parent_id?, tags?, content?)` — generate ID, create default "Main" page document, convert markdown content to ProseMirror if provided, assign pos, append to resources
- [x] `add_document(resource_id, name, content, type?)` — add a page/map/timeline document to an existing resource, generate ID, convert markdown content
- [x] `set_content(resource_id, document_id?, content)` — update content of existing document (default: first page doc), convert markdown to ProseMirror
- [x] `list_draft_resources()` — return summary of resources in the in-progress world (so the LLM can see what it's built so far)
- [x] `export_world(output_path?)` — finalize: set exportedAt, compute resourceCount, compute hash, call `write_lk_file()`, return file path
- [x] Default output directory: `~/.lk-worlds/exports/` (created if needed)
- [x] Output filename: `{world_name}.lk`
- [x] **Test**: create a world with 3 resources in a hierarchy, export, read back, verify structure
- [x] **Test**: create resource with markdown content, export, verify ProseMirror content is valid

## 2.4: Generation Tools (MCP Wiring)
- [x] Wire `create_world` tool — creates a new WorldBuilder session, returns world name
- [x] Wire `create_resource` tool — calls builder, returns created resource summary (id, name)
- [x] Wire `add_document` tool — calls builder, returns document summary
- [x] Wire `set_content` tool — calls builder, returns confirmation
- [x] Wire `list_draft` tool — calls builder, returns summary of in-progress world
- [x] Wire `export_world` tool — calls builder, writes `.lk` file, returns file path for download
- [x] Only one world can be built at a time per server session (simplicity — no need for multi-session)
- [x] Add LkError variants: NoDraftWorld, DraftResourceNotFound, DraftDocumentNotFound
- [x] **Test**: full flow via MCP — create_world, create_resource ×3, set_content, export_world, verify file exists and is valid
- [ ] **Test**: export_world without create_world returns clear error
- [ ] **Test**: create_resource without create_world returns clear error

## 2.5: Visibility Support
- [x] Add optional `is_hidden: Option<bool>` parameter to `CreateResourceParams` — defaults to false if omitted
- [x] Add optional `is_hidden: Option<bool>` parameter to `AddDocumentParams` — defaults to false if omitted
- [x] Pass `is_hidden` through `WorldBuilder::create_resource()` → sets `Resource.is_hidden`
- [x] Pass `is_hidden` through `WorldBuilder::add_document()` → sets `Document.is_hidden`
- [x] **Test**: create a hidden resource, export, verify `is_hidden: true` in the `.lk` file
- [x] **Test**: add a hidden document to a visible resource, export, verify document has `is_hidden: true`

## 2.6: Template Support
- [x] Implement `extract_templates()` in `store.rs` — find resources under the `parentId: "templates"` chain, return template name → property list mapping
- [x] Add `list_templates` tool — returns available template names with their property block summaries (type + title) from loaded worlds
- [x] Add optional `template: Option<String>` param to `CreateResourceParams`
- [x] When `template` is specified in `create_resource`, look up the template from the WorldStore, clone its properties (with fresh IDs), and apply them to the new resource
- [x] Also apply the template's tags to the resource (merged with any explicitly provided tags)
- [x] Add optional `aliases: Option<Vec<String>>` param to `CreateResourceParams` — sets `Resource.aliases`
- [x] **Test**: extract templates from reference `.lk` files, verify NPC/Location/Character templates are found with correct property blocks
- [x] **Test**: create a resource with `template: "NPC"`, export, verify properties match the NPC template (IMAGE, FRIENDS, ENEMIES, etc.)
- [x] **Test**: create a resource with `template: "NPC"` and explicit tags, verify tags are merged (template tags + explicit tags)
- [x] **Test**: `list_templates` via MCP returns template names

## 2.7: Batch Creation Tool
- [x] Add `BatchResourceSpec` param type — name, parent_id, tags, content, is_hidden, template, aliases, plus `documents` array
- [x] Add `BatchDocumentSpec` — name, content, doc_type, is_hidden
- [x] Add `BatchCreateParams` — world_name (optional, creates world if provided), template_world, resources array
- [x] Implement `batch_create` tool in server.rs — optionally creates world, then creates all resources with their documents in one call
- [x] Support parent references by name within the batch (for nesting resources created in the same batch)
- [x] Update ARCHITECTURE.md with new tool
- [x] Update README.md with new tool

## 2.8: CLI `exports` Subcommand
- [x] Add `exports` subcommand to `main.rs` — lists `.lk` files in `~/.lk-worlds/exports/`
- [x] For each file: read metadata (world name, resource count, export date, file size)
- [x] Print a formatted summary table to stdout
- [x] Handle empty directory gracefully

## 2.8: Polish & Integration
- [x] Update README with world generation instructions
- [x] Add server instructions mentioning generation tools to the LLM
- [x] Verify generation tools coexist with read tools (can read existing worlds AND build new ones simultaneously)
- [ ] Verify exported `.lk` file imports successfully into Legend Keeper
- [ ] **Test**: end-to-end in Claude Code — "Create a world with 5 locations and export it", verify `.lk` file is produced

---

# Phase 3: Player-View Web Server

## 3.1: Visibility Filtering
- [ ] Implement `filter_hidden()` in `src/lk/filter.rs` — takes `LkRoot`, returns filtered `LkRoot`
- [ ] Build set of hidden resource IDs: any resource with `isHidden: true`
- [ ] Compute transitive hidden set: walk parentId chains — if a parent is hidden, all descendants are hidden regardless of their own `isHidden`
- [ ] Remove all hidden resources from the filtered `LkRoot.resources`
- [ ] Remove hidden documents (`isHidden: true`) from remaining visible resources
- [ ] Remove hidden properties (`isHidden: true`) from remaining visible resources
- [ ] Update `resourceCount` to reflect filtered count
- [ ] **Test**: load reference `.lk`, mark a root resource hidden, verify its children are also removed
- [ ] **Test**: verify a visible resource's hidden document is stripped but the resource remains
- [ ] **Test**: verify a visible resource's hidden property is stripped but the resource remains

## 3.2: Player-Mode WorldStore
- [ ] Add `player_mode: bool` field to `WorldStore`
- [ ] When `player_mode` is true, apply `filter_hidden()` after `read_lk_file()` before storing in the HashMap
- [ ] Apply the same filter on hot-reload
- [ ] Remove `*(hidden)*` annotations from server.rs output in player mode (they should never appear since hidden content is gone)
- [ ] Remove `isHidden` field from `list_resources` and `search_content` responses in player mode (always false)
- [ ] **Test**: load a world in player mode, verify `list_resources` returns no hidden resources
- [ ] **Test**: verify `search_content` returns no results from hidden documents
- [ ] **Test**: verify `get_resource_tree` has no gaps — hidden subtrees are fully removed

## 3.3: HTTP Transport + Auth
- [ ] Add Phase 2 dependencies to Cargo.toml (axum, tower-http, or rmcp streamable-http transport)
- [ ] Research rmcp's streamable HTTP server support — determine if it handles HTTP natively or if we need axum
- [ ] Implement shared-secret auth: check `Authorization: Bearer <token>` header on every request
- [ ] Reject requests with missing/invalid token with 401 Unauthorized
- [ ] Add CLI flags: `--player` (enable player mode + HTTP), `--secret <token>`, `--port <port>` (default 8080)
- [ ] When `--player` is set: skip stdio transport, start HTTP transport instead
- [ ] When `--player` is set without `--secret`: refuse to start (require auth for web-exposed server)
- [ ] Log auth failures to stderr (but don't log the token itself)
- [ ] **Test**: start server with `--player --secret test123 --port 0`, send authenticated request, verify 200
- [ ] **Test**: send request without token, verify 401
- [ ] **Test**: send request with wrong token, verify 401

## 3.4: Containerization & Deployment
- [ ] Create `Dockerfile` — multi-stage build (rust builder → minimal runtime image)
- [ ] Binary runs as non-root user in container
- [ ] Worlds directory mounted as a volume (default: `/data/worlds/`)
- [ ] `LK_SECRET` env var as alternative to `--secret` flag (for k8s secrets)
- [ ] `LK_PORT` env var as alternative to `--port` flag
- [ ] Create basic `docker-compose.yml` for local testing
- [ ] Document EKS deployment: container image, PersistentVolume for worlds, k8s Secret for token, Service/Ingress for HTTPS
- [ ] **Test**: `docker build` succeeds
- [ ] **Test**: `docker run` with a mounted `.lk` file, verify tools respond over HTTP

## 3.5: Polish & Integration
- [ ] Update README with player-mode setup instructions
- [ ] Document how friends configure Claude Desktop / ChatGPT to connect to the remote MCP server
- [ ] Add health check endpoint (GET `/health` → 200 OK, no auth required)
- [ ] Add startup banner to stderr: mode (DM/player), transport (stdio/HTTP), port, world count
- [ ] Verify hot-reload works in player mode (new .lk file picked up, filtered, served)
- [ ] **Test**: end-to-end — friend configures Claude Desktop with remote URL + token, queries world data, sees only player-visible content

---

# Phase 4: Write Tools — Mutate Existing Worlds (Future)

## 4.1: File Output
- [ ] Add Phase 4 dependencies to Cargo.toml (if not already present from Phase 2)
- [ ] Implement `write_lk_file()` in `src/lk/io.rs` — temp file, GzEncoder, serde_json::to_writer, fs::rename
- [ ] Implement hash recomputation (SHA-256 of compact JSON resources array)
- [ ] **Roundtrip test**: read each `.lk` in `tests/reference/`, write to temp, read back, compare all fields (except hash)
- [ ] Decide output path strategy (separate from source .lk)

## 4.2: Write Tools (MCP Wiring)
- [ ] Create `src/tools/` module with request/response types
- [ ] Implement `generate_id()` — 8-char lowercase alphanumeric random string
- [ ] Wire `create_resource` tool — parse optional markdown content, call store, return created resource
- [ ] Wire `update_resource` tool — call store, return updated resource
- [ ] Wire `update_document_content` tool — parse content based on format, call store, return confirmation
- [ ] Wire `delete_resource` tool — call store, return confirmation or error if has children
- [ ] Wire `add_timeline_event` tool — find/create lane, append event, call store, return confirmation
- [ ] Add LkError variants: DocumentNotFound, TimelineNotFound, HasChildren
- [ ] Implement `From<LkError> for McpError` conversion
- [ ] **Test**: create a resource via MCP, verify it appears in list_resources
- [ ] **Test**: update document content, verify get_resource returns new content
- [ ] **Test**: delete a resource, verify it's gone
- [ ] Verify output .lk file can be re-read without errors
