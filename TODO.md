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
- [x] Render regions with vertex count and fill/border style
- [x] Render paths with waypoint count and stroke style
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

# Phase 2: Write Tools (Future)

## 2.1: File Output
- [ ] Add Phase 2 dependencies to Cargo.toml (comrak, chrono, rand)
- [ ] Implement `write_lk_file()` in `src/lk/io.rs` — temp file, GzEncoder, serde_json::to_writer, fs::rename
- [ ] Implement hash recomputation (SHA-256 of compact JSON resources array)
- [ ] **Roundtrip test**: read each `.lk` in `tests/reference/`, write to temp, read back, compare all fields (except hash)
- [ ] Decide output path strategy (separate from source .lk)

## 2.2: Markdown-to-ProseMirror Converter
- [ ] Implement `from_markdown(md: &str, resources: &[Resource]) -> Value` in `src/prosemirror/from_markdown.rs`
- [ ] Use `comrak` to parse markdown into AST
- [ ] Convert comrak paragraph → PM paragraph
- [ ] Convert comrak heading → PM heading with level attr
- [ ] Convert comrak list → PM bulletList/orderedList + listItem
- [ ] Convert comrak table → PM table + tableRow + tableHeader/tableCell
- [ ] Convert comrak blockquote → PM blockquote
- [ ] Convert comrak thematic break → PM rule
- [ ] Convert comrak image → PM mediaSingle + media with external type
- [ ] Convert comrak link → PM text with link mark
- [ ] Convert comrak emphasis/strong → PM text with em/strong marks
- [ ] Convert comrak code/code_block → PM text with code mark / codeBlock node
- [ ] Detect `[[Resource Name]]` in text → split into text + PM mention node (resolve name→ID from resources list) + text
- [ ] Convert comrak task list items → PM taskList + taskItem with state attr
- [ ] Handle comrak softbreak/linebreak → PM hardBreak

## 2.3: Write Tools (MCP Wiring)
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
