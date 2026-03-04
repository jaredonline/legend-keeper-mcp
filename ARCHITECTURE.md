# Legend Keeper MCP Server — Architecture

## Overview

A Rust MCP (Model Context Protocol) server that provides read/write access to local Legend Keeper `.lk` export files. The server loads a `.lk` file into memory, exposes 8 MCP tools for browsing and editing world-building data, and persists changes back to the file atomically.

The user manages import/export between this local file and the Legend Keeper web app manually.

## The .lk File Format

`.lk` files are **gzip-compressed JSON**. The decompressed JSON has this schema:

```
LkRoot
├── version: u32 (always 1)
├── exportId: String (8-char alphanumeric, e.g. "sopqot1j")
├── exportedAt: String (ISO 8601 datetime)
├── resources: Vec<Resource> (flat list, tree structure via parentId)
├── calendars: Vec<Value> (opaque, preserve as-is)
├── resourceCount: usize
└── hash: String (SHA-256, recomputed on write)
```

### Resource

```
Resource
├── schemaVersion: u32 (always 1)
├── id: String (8-char lowercase alphanumeric, e.g. "a7pjf5dj")
├── name: String
├── parentId: Option<String> (references another resource's id; None = root)
├── pos: String (fractional index for ordering among siblings)
├── createdBy: String (user ID)
├── isHidden: bool
├── isLocked: bool
├── showPropertyBar: bool
├── iconColor: Option<String> (hex color)
├── iconGlyph: Option<String> (emoji)
├── iconShape: Option<String>
├── aliases: Vec<String>
├── tags: Vec<String> (e.g. ["npc", "bryn shander"])
├── documents: Vec<Document>
├── properties: Vec<Property>
└── banner: Banner { enabled, url, yPosition }
```

### Document

Each resource has 1-3 documents (pages or maps).

```
Document
├── id: String
├── name: String (e.g. "Main", "DMs Notes", "Map")
├── type: String ("page" | "map")
├── locatorId: String (full path reference)
├── pos: String
├── isHidden: bool
├── isFirst: bool
├── createdAt: String (ISO 8601)
├── updatedAt: String (ISO 8601)
├── transforms: Vec<Value> (preserve as-is)
├── sources: Vec<Value> (preserve as-is)
├── presentation: Option<Value> (preserve as-is)
├── content: Option<Value> (ProseMirror JSON, for type="page")
└── map: Option<MapData> (for type="map")
```

### Property

```
Property
├── id: String
├── pos: String
├── type: String (TAGS | TEXT_FIELD | ALIAS | IMAGE | RESOURCE_LINK | SPOTIFY_SINGLE)
├── title: String
└── data: Value (type-specific, preserve structure)
```

### ProseMirror Content

Page documents store content as ProseMirror JSON. The following node types are observed in the reference data:

**Block nodes:** `doc`, `paragraph`, `heading` (attrs.level 1-6), `bulletList`, `orderedList`, `listItem`, `taskList`, `taskItem` (attrs.state: TODO|DONE), `blockquote`, `rule`, `table`, `tableRow`, `tableHeader`, `tableCell`, `layoutSection`, `layoutColumn` (attrs.width), `panel` (attrs.panelType), `mediaSingle`, `bodiedExtension`, `extension`

**Inline nodes:** `text`, `hardBreak`, `media` (attrs.url), `mention` (attrs.id, attrs.text referencing other resources)

**Text marks:** bold, italic, link, code, underline, strikethrough, etc.

---

## Project Structure

```
legend-keeper-mcp/
├── Cargo.toml
├── ARCHITECTURE.md          # This file
├── TODO.md                  # Implementation task list
├── PRINCIPLES.md            # Design principles and conventions
├── rime.lk                  # Reference .lk file (gitignored)
├── .gitignore
└── src/
    ├── main.rs              # Entry point: arg parsing, LkStore::load, stdio transport
    ├── server.rs            # LkServer struct, ServerHandler impl, tool routing
    ├── lk/
    │   ├── mod.rs           # Re-exports, LkError enum
    │   ├── schema.rs        # All serde types for .lk JSON (LkRoot, Resource, Document, etc.)
    │   ├── store.rs         # LkStore: Arc<RwLock<LkRoot>>, query/mutate methods, save
    │   └── io.rs            # read_lk_file(), write_lk_file() — gzip + atomic rename
    ├── prosemirror/
    │   ├── mod.rs           # Re-exports
    │   ├── types.rs         # ProseMirror node serde types (PmNode, PmMark, etc.)
    │   ├── to_markdown.rs   # ProseMirror -> Markdown converter
    │   └── from_markdown.rs # Markdown -> ProseMirror converter (uses comrak)
    └── tools/
        ├── mod.rs           # Request/response structs for all 8 tools
        ├── read.rs          # list_resources, get_resource, get_resource_tree, search_content
        └── write.rs         # create_resource, update_resource, update_document_content, delete_resource
```

---

## Module Details

### `src/main.rs`

- Parses `.lk` file path from CLI arg 1 or `LK_FILE` env var
- Calls `LkStore::load(path)` to read and decompress the file
- Logs resource count to stderr (stdout is the MCP transport)
- Creates `LkServer`, starts rmcp stdio transport, awaits shutdown

### `src/server.rs` — MCP Server

`LkServer` holds an `LkStore` and a `ToolRouter<Self>`.

Implements `ServerHandler` via `#[tool_handler]` macro:
- `get_info()` returns server name "legend-keeper-mcp", protocol version, tool capabilities
- `instructions` field describes available tools to the LLM

8 tool methods annotated with `#[tool(description = "...")]`:

#### Read Tools

| Method | Input | Output |
|--------|-------|--------|
| `list_resources` | `tag?: String`, `name?: String` | JSON array of `{id, name, tags, parentId}` summaries |
| `get_resource` | `id_or_name: String` | Resource metadata + each document's content as markdown |
| `get_resource_tree` | `root_id?: String` | Nested JSON tree: `{id, name, children: [...]}` |
| `search_content` | `query: String`, `limit?: usize` | Array of `{resourceId, resourceName, documentName, snippet}` |

#### Write Tools

| Method | Input | Output |
|--------|-------|--------|
| `create_resource` | `name: String`, `parent_id?: String`, `tags?: Vec<String>`, `content?: String` | Created resource summary |
| `update_resource` | `id: String`, `name?: String`, `tags?: Vec<String>`, `parent_id?: String`, `is_hidden?: bool` | Updated resource summary |
| `update_document_content` | `resource_id: String`, `document_id?: String`, `content: String`, `format?: String` | Updated document summary |
| `delete_resource` | `id: String`, `force?: bool` | Confirmation message |

### `src/lk/schema.rs` — Data Types

All types derive `Debug, Clone, Serialize, Deserialize` with `#[serde(rename_all = "camelCase")]` since the .lk JSON uses camelCase keys.

Key types: `LkRoot`, `Resource`, `Document`, `Property`, `Banner`, `MapData`.

Fields that aren't fully understood (transforms, sources, presentation, calendars) use `serde_json::Value` to preserve them losslessly through read/write cycles.

### `src/lk/store.rs` — In-Memory Store

```rust
pub struct LkStore {
    data: Arc<RwLock<LkRoot>>,
    file_path: PathBuf,
}
```

- `Arc<RwLock>`: Multiple read tools execute concurrently; writes take exclusive lock
- Every mutation method calls `self.save()` after modifying data
- `save()` serializes to JSON, gzip compresses, writes atomically via temp file + rename

**Query methods:**
- `list_resources(tag, name)` — filter/iterate resources, return summaries (no document content)
- `get_resource(id_or_name)` — lookup by ID first, fallback to case-insensitive name match
- `get_resource_tree(root_id)` — build tree from parentId relationships; roots have parentId=None
- `search_content(query, limit)` — iterate all docs, convert ProseMirror to plaintext, case-insensitive substring match, return snippets with context

**Mutation methods:**
- `create_resource(req)` — generate 8-char ID, create default "Main" document, append to resources, increment resourceCount
- `update_resource(id, patch)` — find by ID, apply non-None fields
- `update_document_content(resource_id, doc_id, content, format)` — find doc, parse markdown to ProseMirror (or accept raw), update content and updatedAt
- `delete_resource(id, force)` — check for children; if force, recursively delete subtree; remove from list, decrement count

### `src/lk/io.rs` — File I/O

```
read_lk_file(path) -> Result<LkRoot>
  Open file → GzDecoder → serde_json::from_reader

write_lk_file(path, root) -> Result<()>
  Create temp file (path.lk.tmp) → GzEncoder → serde_json::to_writer → fs::rename (atomic)
```

Hash recomputation on write: SHA-256 of compact JSON serialization of the resources array.

### `src/prosemirror/types.rs` — ProseMirror Serde Types

```rust
pub struct PmNode {
    pub node_type: String,         // "paragraph", "heading", etc.
    pub attrs: Option<Value>,      // node-specific attributes
    pub content: Option<Vec<PmNode>>,  // child nodes
    pub marks: Option<Vec<PmMark>>,    // inline marks (bold, italic, etc.)
    pub text: Option<String>,      // text content (for "text" nodes)
}

pub struct PmMark {
    pub mark_type: String,         // "strong", "em", "link", etc.
    pub attrs: Option<Value>,      // mark-specific attrs (e.g. href for link)
}
```

### `src/prosemirror/to_markdown.rs` — PM → Markdown

Recursive converter. Node type mapping:

| PM Node | Markdown Output |
|---------|----------------|
| `doc` | Recurse into children |
| `paragraph` | Text + `\n\n` |
| `heading` | `#`×level + text + `\n\n` |
| `text` | Literal text, wrapped in mark syntax |
| `bulletList` | `- ` prefixed items |
| `orderedList` | `1. ` prefixed items |
| `listItem` | Recurse, indent nested lists |
| `taskList` | Container for taskItems |
| `taskItem` | `- [ ]` or `- [x]` based on attrs.state |
| `blockquote` | `> ` prefixed lines |
| `rule` | `---\n\n` |
| `table/Row/Header/Cell` | GFM table syntax with `|` separators |
| `hardBreak` | `\n` |
| `mention` | `[[attrs.text]]` |
| `mediaSingle/media` | `![](attrs.url)` |
| `layoutSection/Column` | Flatten: render children sequentially |
| `panel` | `> **panelType:** ` + children |
| `extension/bodiedExtension` | Render children if present, else skip |
| Unknown | Recurse into children silently |

**Mark rendering:** Wrap text in `**bold**`, `*italic*`, `` `code` ``, `[text](url)`, `~~strikethrough~~`, etc.

### `src/prosemirror/from_markdown.rs` — Markdown → PM

Uses `comrak` crate to parse markdown into an AST, then converts each AST node to ProseMirror nodes.

Special handling:
- `[[Resource Name]]` syntax in text → split into text + mention node + text. Resolve resource name to ID via the store's resource list.
- Images → mediaSingle + media nodes with external type
- Tables → table + tableRow + tableHeader/tableCell
- Task lists → taskList + taskItem with state attrs

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1.x | Async runtime (features: full) |
| `rmcp` | 0.3.x | Official Rust MCP SDK (features: server, transport-io) |
| `serde` | 1.x | Serialization (features: derive) |
| `serde_json` | 1.x | JSON parsing/generation |
| `schemars` | 1.0.x | JSON Schema generation for tool parameters |
| `anyhow` | 1.x | Application error handling |
| `thiserror` | 2.x | Library error type derivation |
| `flate2` | 1.x | Gzip compression/decompression |
| `comrak` | 0.35.x | CommonMark markdown parsing (for MD → PM) |
| `sha2` | 0.10.x | SHA-256 hash computation |
| `chrono` | 0.4.x | ISO 8601 timestamp generation (features: serde) |
| `rand` | 0.9.x | Random ID generation |

---

## Error Handling

Domain errors defined in `src/lk/mod.rs`:

| Error | MCP Code | When |
|-------|----------|------|
| `ResourceNotFound(id)` | -32001 | get/update/delete with unknown ID |
| `DocumentNotFound(id)` | -32001 | update_document_content with unknown doc ID |
| `HasChildren` | -32002 | delete_resource without force when resource has children |
| `InvalidInput(msg)` | -32602 | Bad parameters (empty name, invalid format, etc.) |
| `Io(err)` | -32603 | File read/write failures |
| `Json(err)` | -32603 | Serialization/deserialization failures |

All `LkError` variants convert to `McpError` via `From` impl at the tool boundary.

---

## Runtime Configuration

**CLI usage:**
```
legend-keeper-mcp <path-to-file.lk>
```

Or via environment variable:
```
LK_FILE=/path/to/world.lk legend-keeper-mcp
```

**Claude Code MCP config** (in `.claude/settings.json` or project config):
```json
{
  "mcpServers": {
    "legend-keeper": {
      "command": "/path/to/legend-keeper-mcp",
      "args": ["/path/to/rime.lk"]
    }
  }
}
```

---

## Data Flow

```
                    ┌─────────────────┐
                    │   .lk file      │
                    │ (gzip JSON)     │
                    └────────┬────────┘
                             │ read_lk_file()
                             ▼
                    ┌─────────────────┐
                    │    LkStore      │
                    │ Arc<RwLock<     │
                    │   LkRoot>>     │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
         Read tools    Write tools    ProseMirror
         (concurrent)  (exclusive)    converters
              │              │         ↕
              │              │     Markdown
              │              │
              ▼              ▼
         ┌─────────────────────┐
         │   MCP stdio         │
         │   (JSON-RPC)        │
         └─────────────────────┘
              ↕
         LLM / Claude Code
```
