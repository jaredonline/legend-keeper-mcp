# Implementation TODO

## Phase 1: Project Skeleton
- [ ] Run `cargo init` to create Rust project
- [ ] Set up `Cargo.toml` with all dependencies (rmcp, serde, serde_json, schemars, flate2, comrak, sha2, chrono, rand, anyhow, thiserror, tokio)
- [ ] Create module directory structure: `src/lk/`, `src/prosemirror/`, `src/tools/`
- [ ] Create all `mod.rs` files with placeholder re-exports
- [ ] Write minimal `main.rs` that parses CLI args and prints usage
- [ ] Verify `cargo build` succeeds

## Phase 2: Schema + File I/O
- [ ] Define `LkRoot` struct in `src/lk/schema.rs` with all fields (version, exportId, exportedAt, resources, calendars, resourceCount, hash)
- [ ] Define `Resource` struct with all 17 fields (schemaVersion, id, name, parentId, pos, createdBy, isHidden, isLocked, showPropertyBar, iconColor, iconGlyph, iconShape, aliases, tags, documents, properties, banner)
- [ ] Define `Document` struct with all fields including optional content (ProseMirror) and map
- [ ] Define `Property` struct with type-agnostic `data: Value`
- [ ] Define `Banner` and `MapData` structs
- [ ] Implement `read_lk_file()` in `src/lk/io.rs` — open file, GzDecoder, serde_json::from_reader
- [ ] Implement `write_lk_file()` in `src/lk/io.rs` — temp file, GzEncoder, serde_json::to_writer, fs::rename
- [ ] Implement hash recomputation (SHA-256 of compact JSON resources array)
- [ ] **Roundtrip test**: read `rime.lk`, write to temp, read back, compare all fields (except hash)
- [ ] Fix any deserialization issues discovered by the roundtrip test

## Phase 3: In-Memory Store
- [ ] Implement `LkStore` struct in `src/lk/store.rs` with `Arc<RwLock<LkRoot>>` and `PathBuf`
- [ ] Implement `LkStore::load(path)` — calls `read_lk_file`, wraps in Arc<RwLock>
- [ ] Implement `LkStore::save()` — acquires read lock, calls `write_lk_file`
- [ ] Implement `resource_count()` helper
- [ ] Implement `list_resources(tag, name)` — filter by tag (exact) and/or name (case-insensitive substring)
- [ ] Implement `get_resource(id_or_name)` — lookup by ID first, fallback to case-insensitive name
- [ ] Implement `get_resource_tree(root_id)` — build nested tree from parentId relationships
- [ ] Implement `search_content(query, limit)` — extract plain text from ProseMirror, substring search
- [ ] Implement `generate_id()` — 8-char lowercase alphanumeric random string
- [ ] Implement `create_resource(req)` — generate ID, create default "Main" document, append, save
- [ ] Implement `update_resource(id, patch)` — find resource, apply non-None fields, save
- [ ] Implement `update_document_content(resource_id, doc_id, content, format)` — find doc, parse/replace content, update timestamp, save
- [ ] Implement `delete_resource(id, force)` — check children, remove (recursively if force), save
- [ ] Define `LkError` enum in `src/lk/mod.rs` with variants: ResourceNotFound, DocumentNotFound, HasChildren, InvalidInput, Io, Json

## Phase 4: ProseMirror-to-Markdown Converter
- [ ] Define `PmNode` and `PmMark` types in `src/prosemirror/types.rs`
- [ ] Implement `to_markdown(node: &Value) -> String` in `src/prosemirror/to_markdown.rs`
- [ ] Handle `doc` node (root container, recurse)
- [ ] Handle `paragraph` (render children + double newline)
- [ ] Handle `heading` (# × level + children + double newline)
- [ ] Handle `text` node with mark wrapping (strong→**, em→*, code→backtick, link→[](), strikethrough→~~, underline→<u>)
- [ ] Handle `bulletList` / `orderedList` / `listItem` with proper indentation for nesting
- [ ] Handle `taskList` / `taskItem` (- [ ] / - [x])
- [ ] Handle `blockquote` (> prefix on each line)
- [ ] Handle `rule` (---)
- [ ] Handle `hardBreak` (newline)
- [ ] Handle `table` / `tableRow` / `tableHeader` / `tableCell` (GFM table with | separators and --- header row)
- [ ] Handle `mention` → `[[attrs.text]]`
- [ ] Handle `mediaSingle` / `media` → `![](attrs.url)`
- [ ] Handle `layoutSection` / `layoutColumn` → flatten, render children sequentially
- [ ] Handle `panel` → blockquote with panelType prefix
- [ ] Handle `extension` / `bodiedExtension` → render children or extract text attr
- [ ] Handle unknown nodes → recurse into children silently
- [ ] Test against actual ProseMirror content from `rime.lk` resources

## Phase 5: Read Tools (MCP Wiring)
- [ ] Define request/response structs in `src/tools/mod.rs` with schemars::JsonSchema derives
  - [ ] `ListResourcesRequest` { tag?: String, name?: String }
  - [ ] `GetResourceRequest` { id_or_name: String }
  - [ ] `GetResourceTreeRequest` { root_id?: String }
  - [ ] `SearchContentRequest` { query: String, limit?: usize }
- [ ] Implement `LkServer` struct in `src/server.rs` holding LkStore + ToolRouter
- [ ] Implement `ServerHandler` for `LkServer` with `get_info()` returning server metadata
- [ ] Wire `list_resources` tool — calls store, returns JSON array of summaries
- [ ] Wire `get_resource` tool — calls store, converts document content to markdown, returns formatted output
- [ ] Wire `get_resource_tree` tool — calls store, returns nested JSON tree
- [ ] Wire `search_content` tool — calls store, returns matching snippets
- [ ] Update `main.rs` to create LkServer and start rmcp stdio transport
- [ ] **Test**: pipe a `tools/list` JSON-RPC request through stdin, verify all 4 read tools appear
- [ ] **Test**: pipe a `tools/call` for `list_resources`, verify response

## Phase 6: Markdown-to-ProseMirror Converter
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

## Phase 7: Write Tools (MCP Wiring)
- [ ] Define write request structs in `src/tools/mod.rs`
  - [ ] `CreateResourceRequest` { name, parent_id?, tags?, content? }
  - [ ] `UpdateResourceRequest` { id, name?, tags?, parent_id?, is_hidden? }
  - [ ] `UpdateDocumentContentRequest` { resource_id, document_id?, content, format? }
  - [ ] `DeleteResourceRequest` { id, force? }
- [ ] Wire `create_resource` tool — parse optional markdown content, call store, return created resource
- [ ] Wire `update_resource` tool — call store, return updated resource
- [ ] Wire `update_document_content` tool — parse content based on format, call store, return confirmation
- [ ] Wire `delete_resource` tool — call store, return confirmation or error if has children
- [ ] Implement `From<LkError> for McpError` conversion
- [ ] **Test**: create a resource via MCP, verify it appears in list_resources
- [ ] **Test**: update document content, verify get_resource returns new content
- [ ] **Test**: delete a resource, verify it's gone

## Phase 8: Polish & Integration
- [ ] Add startup logging to stderr (resource count, file path)
- [ ] Handle edge cases: empty .lk file (no resources), resource with no documents
- [ ] Verify `get_resource` name lookup handles multiple resources with same name gracefully
- [ ] Verify `delete_resource` with `force=true` cascades to all descendants
- [ ] Verify `update_document_content` without document_id defaults to first document
- [ ] Verify atomic write doesn't leave .lk.tmp on success
- [ ] Test full flow: configure as MCP server in Claude Code, use tools interactively
- [ ] Verify .lk file written by server can be re-read without errors (full cycle test)
