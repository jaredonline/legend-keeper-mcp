# Legend Keeper MCP Server — Architecture

## Overview

A Rust MCP (Model Context Protocol) server that provides read access to local Legend Keeper `.lk` export files. The server watches a directory for `.lk` files, loads them all into memory, and exposes MCP tools for browsing world-building data. When a `.lk` file is added, removed, or modified, the server hot-reloads it.

The user downloads `.lk` exports from Legend Keeper, drops them into the worlds directory (default: `~/.lk-worlds/`), and uses this server to browse/analyze the data with an LLM. The server automatically picks up new or updated files.

## Project Phases

- **Phase 1 (current):** Read-only. Load `.lk` into memory, expose 7 read tools. No file writes.
- **Phase 2 (future):** Write tools that produce a new `.lk` file for re-import. Adds `from_markdown.rs`, atomic file writes, ID generation, and 5 write tools.

## The .lk File Format

`.lk` files are **gzip-compressed JSON**. The decompressed JSON has this schema:

```
LkRoot
├── version: u32 (always 1)
├── exportId: String (8-char alphanumeric, e.g. "sopqot1j")
├── exportedAt: String (ISO 8601 datetime)
├── resources: Vec<Resource> (flat list, tree structure via parentId)
├── calendars: Vec<Calendar> (custom calendar definitions; built-in calendars NOT included)
├── resourceCount: usize
└── hash: String (SHA-256; recomputed on write in Phase 2)
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

Each resource has 1-3 documents. Three document types exist:

| Type | `content` shape | Extra fields |
|------|----------------|--------------|
| `"page"` | ProseMirror JSON (`{type: "doc", content: [...]}`) | — |
| `"map"` | `MapContent` JSON (`{pins: [...]}`) | `map: {locatorId, mapId, min_x, max_x, min_y, max_y, max_zoom}` |
| `"time"` | Timeline JSON (`{lanes: [...], events: [...]}`) | `calendarId: String` (references calendar by ID or built-in name like "harptos") |

```
Document
├── id: String
├── name: String (e.g. "Main", "DMs Notes", "Map", "Timeline")
├── type: String ("page" | "map" | "time")
├── locatorId: String (full path reference)
├── pos: String
├── isHidden: bool
├── isFirst: bool
├── isFullWidth: Option<bool>
├── createdAt: String (ISO 8601)
├── updatedAt: String (ISO 8601)
├── transforms: Vec<Value> (preserve as-is)
├── sources: Vec<Source> (timeline docs can link to other timelines)
├── presentation: Option<Presentation> (see below)
├── content: Option<Value> (ProseMirror JSON for "page", TimelineContent for "time")
├── map: Option<MapData> (for type="map")
└── calendarId: Option<String> (for type="time", references Calendar.id or built-in name)
```

### Calendar

Custom calendar definitions live at the root level. Built-in calendars (e.g. "harptos") are NOT included — only user-created ones.

```
Calendar
├── id: String (e.g. "c1nq5rzp")
├── name: String (e.g. "Siqram")
├── hasZeroYear: bool
├── maxMinutes: i64
├── months: Vec<Month>
│   └── Month { id, name, isIntercalary, length, interval, offset }
├── leapDays: Vec<Value> (observed empty, preserve as Value)
├── weekdays: Vec<Weekday>
│   └── Weekday { id, name }
├── epochWeekday: u32
├── weekResetsEachMonth: bool
├── hoursInDay: u32
├── minutesInHour: u32
├── negativeEra: Era { id, name, abbr, hideAbbr, startsAt, resetMode }
├── positiveEras: Vec<Era>
├── format: CalendarFormat { id, year, month, day, time }
└── halfClock: bool
```

### Timeline Content

Timeline documents (`type: "time"`) store content as:

```
TimelineContent
├── lanes: Vec<Lane>
│   └── Lane { id, name, pos, size }
└── events: Vec<TimelineEvent>
    └── TimelineEvent { id, laneId, type, pos, detail, start (i64 minutes),
        end (Option<i64>), name, iconGlyph, color, imageUrl, imageFit,
        opacity, isSynced, data }
```

### Source

Timeline documents can link to other timelines via `sources`:

```
Source { id, uri: "lk://resources/{id}/docs/{id}", type: "legacy-document",
         createdAt, updatedAt, resourceId, documentId }
```

### Map Content

Map documents store content as a `MapContent` object with a `pins` array. Despite the name, this array contains all map features differentiated by `type`:

```
MapContent
└── pins: Vec<MapFeature>

MapFeature
├── id: String
├── name: String
├── pos: [f64; 2] (normalized coordinates on the map)
├── type: Option<String> (absent=pin, "region", "label", "path")
├── rank: Option<String>
├── isSynced: bool
│
├── // Pin fields (type absent)
├── uri: Option<String> ("lk://resources/{id}/docs/{id}" — link to another resource)
├── iconGlyph: Option<String> (semantic icon name, e.g. "beer-mug")
├── iconColor: Option<String> (hex color)
├── iconShape: Option<String> (e.g. "marker-medium")
│
├── // Region fields (type="region")
├── polygon: Option<Vec<[f64; 2]>> (polygon vertices)
├── fillOpacity: Option<f64>
├── fillVisibility: Option<String>
├── labelVisibility: Option<String>
├── borderStyle: Option<String>
├── fillStyle: Option<String>
│
├── // Label fields (type="label")
├── labelSize: Option<String> (e.g. "large")
├── fontFamily: Option<String>
├── colorA: Option<String>
├── colorB: Option<String>
├── labelStyle: Option<Value>
│
├── // Path fields (type="path")
├── polyline: Option<Vec<[f64; 2]>> (path waypoints)
├── color: Option<String>
├── strokeWidth: Option<f64>
├── strokeStyle: Option<String> (e.g. "dashed")
├── strokeOpacity: Option<f64>
└── curviness: Option<String>
```

When rendered, features are grouped by type: pins as a table with positions, regions and paths as lists with full coordinate arrays (enabling precise distance/area calculations), and labels as a simple list. Pin URIs are parsed to extract linked resource IDs. Calibration data from the presentation is included when present to provide the scale factor for real-world distance calculations.

### Presentation

Every document has a `presentation` object. The `documentType` field is always present; other fields are optional and type-specific.

```
Presentation
├── documentType: String ("page" | "blank" | "map" | "time")
├── calibration: Option<Calibration> (map docs only)
│   └── Calibration { realUnitsPerMapUnit, unit, calibrationDistance, calibrationMapDistance }
├── defaultMode: Option<String> (timeline docs, e.g. "calendar")
└── disallowedModes: Option<Vec<String>> (timeline docs)
```

### Property

```
Property
├── id: String
├── pos: String
├── type: String (TAGS | TEXT_FIELD | ALIAS | IMAGE | RESOURCE_LINK | MENTION | SPOTIFY_SINGLE)
├── title: String
├── isHidden: bool
├── isTitleHidden: bool
└── data: Value (type-specific, preserve structure)
```

### ProseMirror Content

Page documents store content as ProseMirror JSON. The following node types are observed in the reference data:

**Block nodes:** `doc`, `paragraph`, `heading` (attrs.level 1-6), `bulletList`, `orderedList`, `listItem`, `taskList`, `taskItem` (attrs.state: TODO|DONE), `blockquote`, `rule`, `codeBlock`, `table`, `tableRow`, `tableHeader`, `tableCell`, `layoutSection`, `layoutColumn` (attrs.width), `panel` (attrs.panelType), `mediaSingle`, `bodiedExtension`, `extension`

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
├── .gitignore
├── tests/
│   └── reference/           # Reference .lk files for integration tests (gitignored)
│       ├── rime.lk          # 124 resources, no custom calendars
│       └── siqram.lk        # 296 resources, 1 custom calendar
└── src/
    ├── main.rs              # Entry point: resolve worlds dir, start watcher, stdio transport
    ├── server.rs            # LkServer struct, ServerHandler impl, tool routing
    ├── lk/
    │   ├── mod.rs           # Re-exports, LkError enum
    │   ├── schema.rs        # All serde types for .lk JSON (LkRoot, Resource, Document, etc.)
    │   ├── store.rs         # WorldStore: manages multiple worlds, file watching, hot-reload
    │   └── io.rs            # read_lk_file() — gzip decompression
    └── prosemirror/
        ├── mod.rs           # Re-exports
        ├── types.rs         # ProseMirror node serde types (PmNode, PmMark, etc.)
        └── to_markdown.rs   # ProseMirror -> Markdown converter

    # Phase 2 additions:
    # lk/io.rs              += write_lk_file() — gzip + atomic rename
    # prosemirror/from_markdown.rs — Markdown -> ProseMirror converter (uses comrak)
    # tools/                 — request/response types, read.rs, write.rs
```

---

## Module Details

### `src/main.rs`

- Resolves worlds directory: CLI arg 1, `LK_WORLDS` env var, or default `~/.lk-worlds/`
- Creates directory if it doesn't exist
- Calls `WorldStore::load(dir)` to read all `.lk` files and start file watcher
- Logs world count and resource counts to stderr (stdout is the MCP transport)
- Creates `LkServer`, starts rmcp stdio transport, awaits shutdown

### `src/server.rs` — MCP Server

`LkServer` holds an `LkStore` and a `ToolRouter<Self>`.

Implements `ServerHandler` via `#[tool_handler]` macro:
- `get_info()` returns server name "legend-keeper-mcp", protocol version, tool capabilities
- `instructions` field describes available tools to the LLM

### Phase 1: Read Tools (7 tools)

All tools that operate on a specific world take a `world` parameter (the filename stem, e.g. `"rime"`). If only one world is loaded, the parameter can be omitted.

| Method | Input | Output |
|--------|-------|--------|
| `list_worlds` | *(none)* | Array of `{name, resourceCount, calendarCount, guide?}` for each loaded world. `guide` contains markdown from the first resource tagged `llm-guide`. |
| `list_resources` | `world?: String`, `tag?: String`, `name?: String` | JSON array of `{id, name, tags, parentId, isHidden}` summaries |
| `get_resource` | `world?: String`, `id_or_name: String` | Resource metadata + each document's content as markdown. Hidden documents and properties are included with `*(hidden)*` annotations so the LLM can reason about visibility. Map docs include pins, regions with vertex coordinates, paths with full waypoint coordinates, labels, and calibration. Timeline docs rendered with lane/event summaries. |
| `get_resource_tree` | `world?: String`, `root_id?: String` | Nested JSON tree: `{id, name, children: [...]}` |
| `search_content` | `world?: String`, `query: String`, `limit?: usize` | Array of `{resourceId, resourceName, documentName, snippet, isHidden}`. Searches all pages (including hidden) + timeline event names. |
| `get_calendar` | `world?: String`, `id_or_name: String` | Calendar definition: month/weekday/era structure |
| `get_map` | `world?: String`, `id_or_name: String` | Map metadata + pins with positions, regions with full vertex coordinates, paths with full waypoint coordinates, labels, and calibration for a resource's map document. Coordinates enable precise distance/area calculations. Errors if resource has no map. |

### Phase 2: Write Tools (5 tools, future)

| Method | Input | Output |
|--------|-------|--------|
| `create_resource` | `name: String`, `parent_id?: String`, `tags?: Vec<String>`, `content?: String` | Created resource summary |
| `update_resource` | `id: String`, `name?: String`, `tags?: Vec<String>`, `parent_id?: String`, `is_hidden?: bool` | Updated resource summary |
| `update_document_content` | `resource_id: String`, `document_id?: String`, `content: String`, `format?: String` | Updated document summary |
| `delete_resource` | `id: String`, `force?: bool` | Confirmation message |
| `add_timeline_event` | `resource_id: String`, `lane_name: String`, `event_name: String`, `start: i64`, `end?: i64`, `color?: String` | Created event summary. Creates lane if name doesn't exist. |

### `src/lk/schema.rs` — Data Types

All types derive `Debug, Clone, Serialize, Deserialize` with `#[serde(rename_all = "camelCase")]` since the .lk JSON uses camelCase keys.

Key types: `LkRoot`, `Resource`, `Document`, `Property`, `Banner`, `MapData`, `Calendar`, `Month`, `Weekday`, `Era`, `CalendarFormat`, `TimelineContent`, `Lane`, `TimelineEvent`, `Source`, `Presentation`, `Calibration`, `MapContent`, `MapFeature`.

Fields that aren't fully understood (`transforms`, `leapDays`) use `serde_json::Value` to preserve them losslessly through read/write cycles.

### `src/lk/store.rs` — World Store

```rust
pub struct WorldStore {
    worlds: Arc<RwLock<HashMap<String, LkRoot>>>,  // keyed by world name (filename stem)
    dir: PathBuf,
}
```

- Loads all `.lk` files from the worlds directory on startup
- Watches directory for file changes (add/remove/modify) and hot-reloads affected worlds
- World name derived from filename: `rime.lk` → `"rime"`, `siqram.lk` → `"siqram"`
- `Arc<RwLock>`: In Phase 1, writes only happen during reload. Phase 2 adds mutation via tools.
- The dataset is small (hundreds of resources per world) — linear scans are fine.

**Query methods (Phase 1):**
- `list_worlds()` — return list of loaded world names with resource counts. Includes `guide` field from resources tagged `llm-guide`.
- `list_resources(world, tag, name)` — filter/iterate resources, return summaries (no document content)
- `get_resource(world, id_or_name)` — lookup by ID first, fallback to case-insensitive name match. All documents included (hidden ones annotated with `*(hidden)*`). Map docs rendered with full coordinates for pins, regions, and paths.
- `get_resource_tree(world, root_id)` — build tree from parentId relationships; roots have parentId=None
- `search_content(world, query, limit)` — iterate all docs (including hidden), convert ProseMirror to plaintext, case-insensitive substring match, return snippets with context and `is_hidden` flag. Also searches timeline event names.
- `get_calendar(world, id_or_name)` — lookup by ID first, fallback to case-insensitive name match

**Mutation methods (Phase 2, future):**
- `create_resource(req)` — generate 8-char ID, create default "Main" document, append to resources, increment resourceCount
- `update_resource(id, patch)` — find by ID, apply non-None fields
- `update_document_content(resource_id, doc_id, content, format)` — find doc, parse markdown to ProseMirror (or accept raw), update content and updatedAt
- `delete_resource(id, force)` — check for children; if force, recursively delete subtree; remove from list, decrement count
- `add_timeline_event(resource_id, lane_name, event_name, start, end, color)` — find timeline doc, create/find lane by name, append event, save

### `src/lk/io.rs` — File I/O

**Phase 1:**
```
read_lk_file(path) -> Result<LkRoot>
  Open file → GzDecoder → serde_json::from_reader
```

**Phase 2 additions:**
```
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

### `src/prosemirror/from_markdown.rs` — Markdown → PM (Phase 2)

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
| `schemars` | 1.0.x | JSON Schema generation for MCP tool parameters |
| `anyhow` | 1.x | Application error handling |
| `thiserror` | 2.x | Library error type derivation |
| `flate2` | 1.x | Gzip decompression (Phase 1); compression added in Phase 2 |
| `notify` | 7.x | File system watching for hot-reload of .lk files |
| `sha2` | 0.10.x | SHA-256 hash verification |
| `comrak` | 0.35.x | Phase 2: CommonMark markdown parsing (for MD → PM) |
| `chrono` | 0.4.x | Phase 2: ISO 8601 timestamp generation (features: serde) |
| `rand` | 0.9.x | Phase 2: Random ID generation |

---

## Error Handling

Domain errors defined in `src/lk/mod.rs`:

| Error | MCP Code | When |
|-------|----------|------|
| `WorldNotFound(name)` | -32001 | world parameter doesn't match any loaded .lk file |
| `ResourceNotFound(id)` | -32001 | get_resource/get_resource_tree with unknown ID |
| `CalendarNotFound(id)` | -32001 | get_calendar with unknown ID/name |
| `InvalidInput(msg)` | -32602 | Bad parameters (empty query, etc.) |
| `Io(err)` | -32603 | File read failures |
| `Json(err)` | -32603 | Deserialization failures |

**Phase 2 additions:**

| Error | MCP Code | When |
|-------|----------|------|
| `DocumentNotFound(id)` | -32001 | update_document_content with unknown doc ID |
| `TimelineNotFound(id)` | -32001 | add_timeline_event when resource has no timeline doc |
| `HasChildren` | -32002 | delete_resource without force when resource has children |

All `LkError` variants convert to `McpError` via `From` impl at the tool boundary.

---

## Runtime Configuration

**CLI usage:**
```
legend-keeper-mcp [worlds-directory]
```

The worlds directory is resolved in order:
1. CLI arg 1 (if provided)
2. `LK_WORLDS` env var
3. Default: `~/.lk-worlds/`

Drop `.lk` files into this directory. The server loads all of them on startup and watches for changes.

**Claude Code MCP config** (in `.claude/settings.json` or project config):
```json
{
  "mcpServers": {
    "legend-keeper": {
      "command": "/path/to/legend-keeper-mcp"
    }
  }
}
```

Or with a custom directory:
```json
{
  "mcpServers": {
    "legend-keeper": {
      "command": "/path/to/legend-keeper-mcp",
      "args": ["/path/to/my-worlds/"]
    }
  }
}
```

---

## Data Flow (Phase 1)

```
         ~/.lk-worlds/
         ├── rime.lk
         ├── siqram.lk
         └── ...
              │
              │ file watcher + read_lk_file()
              ▼
         ┌─────────────────────┐
         │     WorldStore      │
         │  HashMap<String,    │
         │    LkRoot>          │
         │  (hot-reloads on    │
         │   file changes)     │
         └─────────┬───────────┘
                   │
         ┌─────────┤
         │         │
    Read tools  ProseMirror
    (7 tools)   → Markdown
         │
         ▼
    ┌─────────────────────┐
    │   MCP stdio         │
    │   (JSON-RPC)        │
    └─────────────────────┘
         ↕
    LLM / Claude Code
```
