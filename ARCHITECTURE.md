# Legend Keeper MCP Server — Architecture

## Overview

A Rust MCP (Model Context Protocol) server that provides read access to Legend Keeper `.lk` export files. The server watches a directory for `.lk` files, loads them all into memory, and exposes MCP tools for browsing world-building data. When a `.lk` file is added, removed, or modified, the server hot-reloads it.

The server operates in two modes:
- **DM mode** (default): Local stdio transport. All content is visible, with hidden items annotated. For the DM's own use.
- **Player mode** (`--player`): HTTP transport with bearer-token auth. All hidden content is hard-filtered from memory before serving. For sharing with players over the web.

The user downloads `.lk` exports from Legend Keeper, drops them into the worlds directory (default: `~/.lk-worlds/`), and uses this server to browse/analyze the data with an LLM. The server automatically picks up new or updated files.

## Project Phases

- **Phase 1 (current):** Read-only local server. Load `.lk` into memory, expose 8 read tools via stdio. No file writes. DM sees everything including hidden content (annotated).
- **Phase 2 (next):** World generation. LLM builds a world from scratch via tool calls, then exports a `.lk` file for import into Legend Keeper. Adds `from_markdown.rs`, `write_lk_file()`, `WorldBuilder`, and 6 generation tools. One-way flow: LLM → `.lk` file → import.
- **Phase 3 (future):** Player-view web server. HTTP transport with shared-secret auth. Hard-filters all hidden content (resources, documents, properties) before storing in memory — hidden data never exists in the queryable store. Transitive visibility: if a parent resource is hidden, the entire subtree is removed. Containerized for deployment (EKS or similar).
- **Phase 4 (future):** Write tools that mutate existing worlds in-place. Read an existing `.lk`, modify resources, write it back. Superset of Phase 2's generation capabilities applied to existing data.

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
└── hash: String (SHA-256; computed on export in Phase 2)
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

Each resource has 1-3 documents. Four document types exist:

| Type | `content` shape | Extra fields |
|------|----------------|--------------|
| `"page"` | ProseMirror JSON (`{type: "doc", content: [...]}`) | — |
| `"map"` | `MapContent` JSON (`{pins: [...]}`) | `map: {locatorId, mapId, min_x, max_x, min_y, max_y, max_zoom}` |
| `"time"` | Timeline JSON (`{lanes: [...], events: [...]}`) | `calendarId: String` (references calendar by ID or built-in name like "harptos") |
| `"board"` | `BoardContent` JSON (`{shapesV2: [{key, val}...]}`) | `presentation.documentType: "board"` — tldraw whiteboard canvas with shapes (geo, arrow, text, line) and bindings |

```
Document
├── id: String
├── name: String (e.g. "Main", "DMs Notes", "Map", "Timeline")
├── type: String ("page" | "map" | "time" | "board")
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

**Property data shapes by type:**

| Type | Typical Titles | Data Shape |
|------|---------------|------------|
| `RESOURCE_LINK` | FRIENDS, ENEMIES, Partners, LOCATION, MEMBERS, etc. | `{"items": [{"id", "pos", "resourceId"}]}` |
| `TEXT_FIELD` | SUMMARY, DATE, Pronounciation | `{"fragment": {"type": "doc", "content": [...]}}` |
| `IMAGE` | IMAGE, FLAG, LOGO | `{"url": "", "origin": [0, 0], "scale": 1}` |
| `TAGS` | TAGS | `null` (actual tags live on `Resource.tags`) |
| `ALIAS` | ALIASES | `null` (actual aliases live on `Resource.aliases`) |
| `MENTION` | BACKLINKS | `null` (system-computed) |
| `SPOTIFY_SINGLE` | VIBE, AMBIENCE | `{"url": ""}` |

### Templates

Legend Keeper stores templates as resources under a special parent chain. The structure is:

```
Resource (parentId: null, id varies)
└── "Default Templates " (parentId: "templates")
    ├── NPC (tags: ["npc"], properties: [IMAGE, FRIENDS, ENEMIES, Partners, TAGS, ALIASES, BACKLINKS])
    ├── Character (tags: ["character"], properties: [IMAGE, VIBE, SUMMARY, FRIENDS, ENEMIES, TAGS])
    ├── Location (tags: ["location"], properties: [IMAGE, AMBIENCE, SUMMARY, LOCATED IN, TAGS])
    └── ... (Event, Country, Creature, Organization, etc.)
```

Templates are identified by walking the `parentId` chain — any resource whose ancestor has `id == "templates"` is part of the template hierarchy. Each template's `properties` array defines the property blocks that are cloned (with fresh IDs) onto new resources created with that template.

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
├── .beads/                  # bd issue tracker database
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
    # lk/io.rs              += write_lk_file() — gzip compression + output
    # lk/builder.rs         — WorldBuilder: in-memory world assembly + export
    # prosemirror/from_markdown.rs — Markdown -> ProseMirror converter (uses comrak)

    # Phase 3 additions:
    # lk/filter.rs          — visibility filtering (transitive hidden removal)
    # Dockerfile            — multi-stage build for containerized deployment
    # docker-compose.yml    — local testing config

    # Phase 4 additions:
    # tools/                 — request/response types for mutating existing worlds
```

---

## Module Details

### `src/main.rs`

- Resolves worlds directory: CLI arg or `LK_WORLDS` env var or default `~/.lk-worlds/`
- Creates directory if it doesn't exist
- Subcommand routing: `exports` subcommand lists generated `.lk` files in `exports/` subdirectory; no subcommand starts the MCP server
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
| `search_content` | `world?: String`, `query: String`, `limit?: usize` | Array of `{resourceId, resourceName, documentName, snippet, isHidden}`. Searches all pages (including hidden), timeline event names, and board shape text. |
| `get_calendar` | `world?: String`, `id_or_name: String` | Calendar definition: month/weekday/era structure |
| `get_map` | `world?: String`, `id_or_name: String` | Map metadata + pins with positions, regions with full vertex coordinates, paths with full waypoint coordinates, labels, and calibration for a resource's map document. Coordinates enable precise distance/area calculations. Errors if resource has no map. |
| `get_board` | `world?: String`, `id_or_name: String` | Board summary: shape counts by type (geo, arrow, text, line), labeled geo nodes with position/color, text labels with scale/position, and graph connections derived from arrow bindings. Errors if resource has no board. |

### Phase 2: Generation Tools (14 tools)

The generation tools let the LLM build a new world from scratch during a conversation and export it as a `.lk` file. The world is assembled in memory via a `WorldBuilder` — separate from the read-only `WorldStore`. The LLM sends content as markdown, which is converted to ProseMirror for the `.lk` file. Only one world can be built at a time per session. Resources default to hidden (`isHidden: true`) so exported worlds are safe to import — the DM unhides resources in Legend Keeper after review. Set `is_hidden: false` explicitly to make a resource visible. Draft resources can be deleted, reparented, read back, and edited before export.

Templates are extracted from loaded worlds — Legend Keeper stores templates as resources under a special "templates" parent. When creating a resource, the LLM can specify a template name to copy its property blocks (IMAGE, TAGS, ALIASES, FRIENDS, ENEMIES, etc.) and icon fields (color, glyph, shape) onto the new resource. Relationship properties are created as empty blocks — they cannot be populated during generation.

| Method | Input | Output |
|--------|-------|--------|
| `create_world` | `name: String` | Confirmation with world name |
| `list_templates` | `world?: String` | Available template names with property block summaries |
| `create_resource` | `name: String`, `parent_id?: String`, `tags?: Vec<String>`, `content?: String`, `is_hidden?: bool`, `template?: String`, `aliases?: Vec<String>` | Created resource summary (id, name) |
| `add_document` | `resource_id: String`, `name: String`, `content: String`, `type?: String`, `is_hidden?: bool` | Created document summary |
| `set_content` | `resource_id: String`, `document_id?: String`, `content: String` | Confirmation |
| `list_draft` | *(none)* | Summary of in-progress world (resource names, hierarchy) |
| `load_draft` | `name: String` | Loads an existing world into the draft builder for editing. Checks exports directory first (`~/.lk-worlds/exports/{name}.lk`), then falls back to cloning from the WorldStore. Replaces any existing draft. |
| `export_world` | `output_path?: String` | File path to the generated `.lk` file |
| `batch_create` | `world_name?: String`, `template_world?: String`, `resources: Vec<BatchResourceSpec>` | Summary of all created resources. Each resource spec includes name, parent, tags, content, template, aliases, is_hidden, and additional documents. Creates the draft world if `world_name` is provided and no draft exists. Parent references can use names of resources earlier in the same batch. |
| `delete_resource` | `resource_id: String`, `recursive?: bool` | Deletes a resource from the draft. If `recursive` is true, deletes the entire subtree. If false (default), fails when the resource has children. Returns list of deleted IDs. |
| `reparent_resource` | `resource_id: String`, `new_parent_id?: String` | Moves a resource to a different parent in the draft. Omit `new_parent_id` to make it top-level. Prevents circular references. |
| `get_draft_resource` | `id_or_name: String` | Full resource with all documents rendered as markdown. Mirrors `get_resource` format but reads from the draft `WorldBuilder`. Lookup: ID-first, name-fallback (case-insensitive). |
| `get_draft_document` | `resource_id: String`, `document_id?: String` | Single document content as markdown with metadata. If `document_id` omitted, returns the first page document. |
| `update_draft_resource` | `resource_id: String`, `name?: String`, `tags?: Vec<String>`, `is_hidden?: bool`, `aliases?: Vec<String>` | Updates metadata (not content) on a draft resource. Tags and aliases are full replacements. Returns updated summary. |
| `delete_document` | `resource_id: String`, `document_id: String` | Removes a document from a draft resource. Fails if it would leave zero documents. |

Generation tools coexist with read tools — the LLM can read from existing worlds for reference while building a new one.

### Phase 3: Player-View Mode

In player mode (`--player`), the server operates identically to Phase 1 but with two key differences:

1. **Visibility filtering**: All hidden content is removed from memory at load time. The same 8 read tools are exposed, but they can only return player-visible data.
2. **HTTP transport + auth**: Instead of stdio, the server listens on HTTP with `Authorization: Bearer <token>` authentication. Friends connect their own LLM clients (Claude Desktop, ChatGPT, etc.) to the remote URL.

**Filtering rules** (applied after `read_lk_file()`, before storing in WorldStore):
- Resources with `isHidden: true` → removed entirely
- Transitive: if a resource is hidden, all descendants (via parentId chain) are removed regardless of their own `isHidden`
- Documents with `isHidden: true` → removed from their parent resource
- Properties with `isHidden: true` → removed from their parent resource
- `resourceCount` updated to reflect filtered count

**Behavioral changes in player mode:**
- `list_resources` response omits the `isHidden` field (always false, so it's noise)
- `search_content` response omits the `isHidden` field
- `get_resource` never emits `*(hidden)*` annotations (there's nothing hidden to annotate)
- `get_resource_tree` has no gaps — hidden subtrees are fully pruned

### Phase 4: Write Tools — Mutate Existing Worlds (5 tools, future)

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
- `Arc<RwLock>`: In Phase 1/2/3, writes only happen during reload. Phase 4 adds mutation via tools.
- In player mode, `filter_hidden()` is applied after reading each `.lk` file and before inserting into the HashMap. Hidden content never exists in the queryable store.
- The dataset is small (hundreds of resources per world) — linear scans are fine.

**Query methods (Phase 1):**
- `list_worlds()` — return list of loaded world names with resource counts. Includes `guide` field from resources tagged `llm-guide`.
- `list_resources(world, tag, name)` — filter/iterate resources, return summaries (no document content)
- `get_resource(world, id_or_name)` — lookup by ID first, fallback to case-insensitive name match. All documents included (hidden ones annotated with `*(hidden)*`). Map docs rendered with full coordinates for pins, regions, and paths.
- `get_resource_tree(world, root_id)` — build tree from parentId relationships; roots have parentId=None
- `search_content(world, query, limit)` — iterate all docs (including hidden), convert ProseMirror to plaintext, case-insensitive substring match, return snippets with context and `is_hidden` flag. Also searches timeline event names.
- `get_calendar(world, id_or_name)` — lookup by ID first, fallback to case-insensitive name match

**Mutation methods (Phase 4, future):**
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
  GzEncoder → serde_json::to_writer → write to output path
```
Hash computation on write: SHA-256 of compact JSON serialization of the resources array.

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
- Standard GFM tables (with `|---|` separator) → table + tableRow + tableHeader (first row) / tableCell (data rows)
- Headerless pipe tables (LLM-generated, no separator row) → extracted before comrak parsing, converted to header-COLUMN tables where the first cell of every row is `tableHeader` and the rest are `tableCell`. This matches Legend Keeper's key-value table style.
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
| `axum` | 0.8.x | Phase 3: HTTP server for player-mode transport |
| `tower-http` | 0.6.x | Phase 3: HTTP middleware (CORS, auth layer) |

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
| `NoDraftWorld` | -32001 | Generation tool called without `create_world` first |
| `DraftResourceNotFound(id)` | -32001 | `add_document`/`set_content` with unknown resource ID in draft |
| `DraftDocumentNotFound(id)` | -32001 | `set_content` with unknown document ID in draft |

**Phase 3 additions:**

| Error | HTTP Code | When |
|-------|-----------|------|
| `Unauthorized` | 401 | Missing or invalid bearer token in player mode |

**Phase 4 additions:**

| Error | MCP Code | When |
|-------|----------|------|
| `DocumentNotFound(id)` | -32001 | update_document_content with unknown doc ID |
| `TimelineNotFound(id)` | -32001 | add_timeline_event when resource has no timeline doc |
| `HasChildren` | -32002 | delete_resource without force when resource has children |

All `LkError` variants convert to `McpError` via `From` impl at the tool boundary.

---

## Runtime Configuration

**CLI usage (DM mode — Phase 1, default):**
```
legend-keeper-mcp [worlds-directory]
```

**CLI usage (Player mode — Phase 3):**
```
legend-keeper-mcp --player --secret <token> [--port <port>] [worlds-directory]
```

The worlds directory is resolved in order:
1. CLI argument (positional, if provided)
2. `LK_WORLDS` env var
3. Default: `~/.lk-worlds/`

Player mode also accepts environment variables (useful for containerized deployment):
- `LK_SECRET` — shared secret token (alternative to `--secret`)
- `LK_PORT` — HTTP port (alternative to `--port`, default: 8080)

Drop `.lk` files into this directory. The server loads all of them on startup and watches for changes.

**Claude Code MCP config — local DM mode** (in `.claude/settings.json` or project config):
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

**Claude Desktop MCP config — remote player mode** (friends connecting to your server):
```json
{
  "mcpServers": {
    "legend-keeper": {
      "url": "https://your-server.example.com/mcp",
      "headers": {
        "Authorization": "Bearer <shared-secret>"
      }
    }
  }
}
```

---

## Data Flow

### Phase 1: DM Mode (stdio)

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
         │  (all content incl. │
         │   hidden, annotated)│
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
    DM's LLM client
```

### Phase 2: World Generation (stdio)

```
    LLM client (Claude Code, etc.)
         │
         │ create_world, create_resource,
         │ add_document, set_content, ...
         ▼
    ┌─────────────────────┐
    │   MCP stdio         │
    │   (JSON-RPC)        │
    └─────────┬───────────┘
              │
              ▼
    ┌─────────────────────┐
    │    WorldBuilder      │
    │  (in-memory LkRoot  │
    │   being assembled)  │
    └─────────┬───────────┘
              │ export_world
              ▼
    ┌─────────────────────┐
    │  write_lk_file()    │
    │  ~/.lk-worlds/      │
    │    exports/world.lk │
    └─────────────────────┘
              │
              ▼
    Import into Legend Keeper
```

Note: Read tools (WorldStore) and generation tools (WorldBuilder) coexist in the same server session. The LLM can read existing worlds for reference while building a new one.

### Phase 3: Player Mode (HTTP)

```
         ~/.lk-worlds/ (or /data/worlds/ in container)
         ├── rime.lk
         └── ...
              │
              │ file watcher + read_lk_file()
              ▼
         ┌─────────────────────┐
         │   filter_hidden()   │
         │  remove hidden      │
         │  resources, docs,   │
         │  properties +       │
         │  transitive subtrees│
         └─────────┬───────────┘
                   ▼
         ┌─────────────────────┐
         │     WorldStore      │
         │  (player_mode=true) │
         │  only visible data  │
         │  exists in memory   │
         └─────────┬───────────┘
                   │
         ┌─────────┤
         │         │
    Read tools  ProseMirror
    (7 tools)   → Markdown
         │
         ▼
    ┌─────────────────────┐
    │   HTTP transport     │
    │   Bearer token auth  │
    │   (JSON-RPC over HTTP)│
    └─────────────────────┘
         ↕ (HTTPS via ingress)
    Friends' LLM clients
```
