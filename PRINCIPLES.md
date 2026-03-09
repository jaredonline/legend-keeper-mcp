# Design Principles

## 1. Lossless Deserialization

The .lk file must deserialize without data loss. Fields we don't fully understand (`transforms`, `leapDays`) are stored as `serde_json::Value` and passed through untouched. Never drop unknown fields — use `#[serde(flatten)]` or `Value` for extensibility.

**Phase 2 extension**: Generated `.lk` files must be valid Legend Keeper imports. The schema types are shared between read and generation paths, ensuring structural consistency.

**Phase 4 extension**: When mutation writes are added, the file must also survive full read→write round-trips. Validation: deserialize, re-serialize, decompress both, and diff. The only acceptable difference is the `hash` field.

## 2. Markdown as the LLM Interface

LLMs work best with markdown. All document content exposed through read tools is converted from ProseMirror to markdown.

**Phase 2 extension**: Generation tools accept markdown from the LLM and convert it to ProseMirror for the `.lk` file, requiring a Markdown→ProseMirror converter.

**Corollary**: The ProseMirror→Markdown converter is critical path code. It must handle all 25 observed node types gracefully. Unknown nodes are rendered by recursing into children, never by crashing.

## 3. Defensive Parsing, Strict Serialization

- **Parse**: Accept any valid JSON structure. Use `Option<T>` and `#[serde(default)]` liberally. A missing field should not crash the server.
- **Serialize**: Produce the exact camelCase field names Legend Keeper expects. Validate with `#[serde(rename_all = "camelCase")]`.

## 4. Atomic Persistence (Phase 4)

Every write operation must leave the .lk file in a consistent state. Write to a temp file first, then `fs::rename`. Never write directly to the target file — a crash mid-write would corrupt it. Write output goes to a separate file — the source `.lk` is never modified.

## 5. Simple Concurrency Model

- `Arc<RwLock<HashMap<String, LkRoot>>>` — read-only in Phase 1 (writes only during hot-reload)
- One background thread for file watching; otherwise no caches, no indexes
- The dataset is small (hundreds of resources per world) — linear scans are fine
- One MCP server instance watches one directory of `.lk` files

## 6. Minimal Dependencies

Only add crates that earn their weight:
- `rmcp` — we need the MCP protocol implementation
- `serde`/`serde_json` — we need JSON serialization
- `flate2` — we need gzip
- `tokio` — rmcp requires async
- `sha2` — hash verification
- `notify` — file watching for hot-reload
- Phase 2: `comrak` (markdown parsing), `chrono` (timestamps), `rand` (ID generation)
- Phase 3: `axum` (HTTP server), `tower-http` (middleware/auth)

Avoid: ORMs, web frameworks, logging frameworks (use `eprintln!`), config file parsers.

## 7. Fail Loudly at Startup, Gracefully at Runtime

- If the worlds directory can't be created or accessed → panic with a clear error message at startup
- If a `.lk` file can't be parsed during hot-reload → log error to stderr, skip that file, keep serving other worlds
- If a tool receives bad input at runtime → return a structured MCP error, never panic
- Phase 2: If generation tool called without a draft world → return clear MCP error
- Phase 3: If auth fails → return 401, never leak hidden content in error messages
- Phase 4: If a write fails → return error, do not leave partial state

## 8. IDs Match Legend Keeper's Format (Phase 2+)

Generated IDs must be 8-character lowercase alphanumeric strings (e.g., `a7pjf5dj`), matching the format observed in the reference data. This ensures compatibility when the file is re-imported into Legend Keeper.

## 9. Preserve Ordering (Phase 2+)

Resources, documents, and properties each have a `pos` field for ordering. Maintain existing `pos` values. For new items, assign a `pos` that sorts after existing siblings (simple string like `"z"` or lexicographic midpoint).

## 10. Stderr for Logs, Stdout for MCP

The MCP stdio transport uses stdout for JSON-RPC. All diagnostic output (startup messages, warnings, debug info) goes to stderr via `eprintln!`. Never use `println!` — it would corrupt the MCP transport.

## 11. Convention Over Configuration

- No config files. Inputs are the worlds directory path (default: `~/.lk-worlds/`) and CLI flags / env vars.
- No feature flags. All tools are always available (the tool set is the same in DM and player mode — only the data changes).
- No plugins. The tool set is fixed at compile time.

## 12. World-Level Conventions via Tags

Special behavior is driven by resource tags rather than names or config files. Currently:

- `llm-guide`: A resource tagged `llm-guide` is treated as a world instruction guide. Its first page document is rendered as markdown and included in the `list_worlds` response, so the LLM sees it automatically. The resource name doesn't matter — only the tag.

This pattern keeps configuration inside the world data (no external config files) and is discoverable via `list_resources` with a tag filter.

## 13. Expose Document Traits, Don't Filter (DM Mode)

In DM mode, document and property metadata (visibility, type, name, etc.) should be exposed to the LLM as inline annotations rather than used to filter content. The LLM needs these traits to make intelligent decisions — e.g., distinguishing player-visible content from hidden DM notes — but the server should never silently hide data. Mark it (e.g., `*(hidden)*`), don't suppress it.

## 14. Hard Filter in Player Mode — No Exceptions (Phase 3)

In player mode, hidden content must be removed from memory entirely. This is a security boundary, not a presentation choice. The filtering happens at load time (after deserialization, before storing in WorldStore), so hidden data never exists in the queryable store and cannot be leaked through any tool, error message, or side channel. The rules are:

- `Resource.isHidden: true` → resource and its entire descendant subtree removed (transitive via parentId)
- `Document.isHidden: true` → document removed from its parent resource
- `Property.isHidden: true` → property removed from its parent resource

No tag-based filtering. No permission-based filtering. Only the `isHidden` fields at the resource, document, and property level.

## 15. Templates From the Source World

Generated resources should match the structure of the source world. Rather than hardcoding template definitions, templates are extracted from loaded worlds at runtime. The LLM calls `list_templates` to see available templates, picks one, and `create_resource` clones its property blocks. This means generated `.lk` files have the same property structure as the user's existing world — if they customized their NPC template to add a "Partners" block, generated NPCs will have it too.

Relationship properties (RESOURCE_LINK) are created as empty blocks — populating them during generation would be error-prone and is better handled in Legend Keeper's UI.

## 16. Test Against Real Data

Reference `.lk` files live in `tests/reference/` (gitignored). Currently: `rime.lk` (124 resources, no custom calendars) and `siqram.lk` (296 resources, 1 custom calendar with timelines). Integration tests deserialize every `.lk` file in that directory. When Legend Keeper updates their format, drop a fresh export in `tests/reference/`, run `cargo test`, and fix whatever breaks.
