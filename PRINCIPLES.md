# Design Principles

## 1. Lossless Round-Trip

The .lk file must survive read→write cycles without data loss. Fields we don't fully understand (`transforms`, `sources`, `presentation`, `calendars`) are stored as `serde_json::Value` and passed through untouched. Never drop unknown fields — use `#[serde(flatten)]` or `Value` for extensibility.

**Validation**: Deserialize `rime.lk`, re-serialize, decompress both, and diff. The only acceptable difference is the `hash` field.

## 2. Markdown as the LLM Interface

LLMs work best with markdown. All document content exposed through read tools is converted from ProseMirror to markdown. All content accepted through write tools is markdown by default (with a `format: "prosemirror"` escape hatch for raw JSON).

**Corollary**: The ProseMirror↔Markdown converters are critical path code. They must handle all 21 observed node types gracefully. Unknown nodes are rendered by recursing into children, never by crashing.

## 3. Defensive Parsing, Strict Serialization

- **Parse**: Accept any valid JSON structure. Use `Option<T>` and `#[serde(default)]` liberally. A missing field should not crash the server.
- **Serialize**: Produce the exact camelCase field names Legend Keeper expects. Validate with `#[serde(rename_all = "camelCase")]`.

## 4. Atomic Persistence

Every write operation must leave the .lk file in a consistent state. Write to a temp file first, then `fs::rename`. Never write directly to the target file — a crash mid-write would corrupt it.

## 5. Simple Concurrency Model

- `Arc<RwLock<LkRoot>>` — reads are concurrent, writes are exclusive
- No background threads, no caches, no indexes
- The dataset is small (hundreds of resources) — linear scans are fine
- One MCP server instance per .lk file

## 6. Minimal Dependencies

Only add crates that earn their weight:
- `rmcp` — we need the MCP protocol implementation
- `serde`/`serde_json` — we need JSON serialization
- `flate2` — we need gzip
- `comrak` — we need markdown parsing (writing our own parser is not worth it)
- `tokio` — rmcp requires async
- `sha2`, `chrono`, `rand` — small, focused utilities

Avoid: ORMs, web frameworks, logging frameworks (use `eprintln!`), config file parsers.

## 7. Fail Loudly at Startup, Gracefully at Runtime

- If the .lk file doesn't exist or can't be parsed → panic with a clear error message at startup
- If a tool receives bad input at runtime → return a structured MCP error, never panic
- If a write fails → return error, do not leave partial state

## 8. IDs Match Legend Keeper's Format

Generated IDs must be 8-character lowercase alphanumeric strings (e.g., `a7pjf5dj`), matching the format observed in the reference data. This ensures compatibility when the file is re-imported into Legend Keeper.

## 9. Preserve Ordering

Resources, documents, and properties each have a `pos` field for ordering. Maintain existing `pos` values. For new items, assign a `pos` that sorts after existing siblings (simple string like `"z"` or lexicographic midpoint).

## 10. Stderr for Logs, Stdout for MCP

The MCP stdio transport uses stdout for JSON-RPC. All diagnostic output (startup messages, warnings, debug info) goes to stderr via `eprintln!`. Never use `println!` — it would corrupt the MCP transport.

## 11. Convention Over Configuration

- No config files. The only input is the .lk file path.
- No feature flags. All tools are always available.
- No plugins. The tool set is fixed at compile time.

## 12. Test Against Real Data

The reference file `rime.lk` is the ground truth. All schema types, ProseMirror converters, and query logic should be validated against it. If our types can't deserialize `rime.lk`, they're wrong.
