# Design Principles

## 1. Lossless Deserialization

The .lk file must deserialize without data loss. Fields we don't fully understand (`transforms`, `leapDays`) are stored as `serde_json::Value` and passed through untouched. Never drop unknown fields — use `#[serde(flatten)]` or `Value` for extensibility.

**Phase 2 extension**: When writes are added, the file must also survive full read→write round-trips. Validation: deserialize, re-serialize, decompress both, and diff. The only acceptable difference is the `hash` field.

## 2. Markdown as the LLM Interface

LLMs work best with markdown. All document content exposed through read tools is converted from ProseMirror to markdown.

**Phase 2 extension**: Write tools will accept markdown by default (with a `format: "prosemirror"` escape hatch for raw JSON), requiring a Markdown→ProseMirror converter.

**Corollary**: The ProseMirror→Markdown converter is critical path code. It must handle all 25 observed node types gracefully. Unknown nodes are rendered by recursing into children, never by crashing.

## 3. Defensive Parsing, Strict Serialization

- **Parse**: Accept any valid JSON structure. Use `Option<T>` and `#[serde(default)]` liberally. A missing field should not crash the server.
- **Serialize**: Produce the exact camelCase field names Legend Keeper expects. Validate with `#[serde(rename_all = "camelCase")]`.

## 4. Atomic Persistence (Phase 2)

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

Avoid: ORMs, web frameworks, logging frameworks (use `eprintln!`), config file parsers.

## 7. Fail Loudly at Startup, Gracefully at Runtime

- If the worlds directory can't be created or accessed → panic with a clear error message at startup
- If a `.lk` file can't be parsed during hot-reload → log error to stderr, skip that file, keep serving other worlds
- If a tool receives bad input at runtime → return a structured MCP error, never panic
- Phase 2: If a write fails → return error, do not leave partial state

## 8. IDs Match Legend Keeper's Format (Phase 2)

Generated IDs must be 8-character lowercase alphanumeric strings (e.g., `a7pjf5dj`), matching the format observed in the reference data. This ensures compatibility when the file is re-imported into Legend Keeper.

## 9. Preserve Ordering (Phase 2)

Resources, documents, and properties each have a `pos` field for ordering. Maintain existing `pos` values. For new items, assign a `pos` that sorts after existing siblings (simple string like `"z"` or lexicographic midpoint).

## 10. Stderr for Logs, Stdout for MCP

The MCP stdio transport uses stdout for JSON-RPC. All diagnostic output (startup messages, warnings, debug info) goes to stderr via `eprintln!`. Never use `println!` — it would corrupt the MCP transport.

## 11. Convention Over Configuration

- No config files. The only input is the worlds directory path (default: `~/.lk-worlds/`).
- No feature flags. All tools are always available.
- No plugins. The tool set is fixed at compile time.

## 12. Test Against Real Data

Reference `.lk` files live in `tests/reference/` (gitignored). Currently: `rime.lk` (124 resources, no custom calendars) and `siqram.lk` (296 resources, 1 custom calendar with timelines). Integration tests deserialize every `.lk` file in that directory. When Legend Keeper updates their format, drop a fresh export in `tests/reference/`, run `cargo test`, and fix whatever breaks.
