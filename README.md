# Legend Keeper MCP Server

A Rust MCP server that provides read access to [Legend Keeper](https://www.legendkeeper.com/) `.lk` export files. Drop your world exports into a directory and browse them with any MCP client (Claude Code, Claude Desktop, ChatGPT, etc.).

Runs in two modes:
- **DM mode** (default): Local stdio server. All content visible, hidden items annotated. Can also generate new `.lk` files from scratch.
- **Player mode** (planned): Web server with bearer-token auth. Hidden content hard-filtered from memory. Share your worlds with friends.

## Features

- Load multiple `.lk` world files simultaneously
- Hot-reload: add, update, or remove `.lk` files while the server is running
- ProseMirror content rendered as clean markdown
- Map awareness: pins, regions, paths, labels, and calibration data with full coordinates for spatial reasoning and distance calculations
- Visibility-aware: hidden documents and properties are exposed with annotations, letting the LLM distinguish player-visible from DM-only content
- World instructions: tag a resource `llm-guide` to give Claude world-specific instructions
- Template-aware generation: new resources inherit property blocks (IMAGE, FRIENDS, ENEMIES, etc.) from your world's templates
- 7 read tools + 8 generation tools

## Build

Requires Rust 1.85+ (edition 2021).

```sh
cargo build --release
```

The binary will be at `target/release/legend-keeper-mcp`.

## Setup

### 1. Create a worlds directory

```sh
mkdir -p ~/.lk-worlds
```

### 2. Export from Legend Keeper

In Legend Keeper, go to **Project Settings → Export → Download .lk file**. Drop the downloaded `.lk` file into `~/.lk-worlds/`.

### 3. Configure as an MCP server

**Claude Code** — run:

```sh
claude mcp add legend-keeper -- /path/to/legend-keeper-mcp
```

Or with a custom worlds directory:

```sh
claude mcp add legend-keeper -- /path/to/legend-keeper-mcp /path/to/my-worlds
```

**Claude Desktop** — add to your config (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "legend-keeper": {
      "command": "/path/to/legend-keeper-mcp"
    }
  }
}
```

### 4. Use it

Ask Claude about your worlds:

- "What worlds do I have loaded?"
- "List all NPCs in my world"
- "Tell me about the capital city"
- "Search for mentions of dragons"
- "Show me the calendar"
- "Show me the resource tree"
- "What locations are on the main map?"
- "How far is Bryn Shander from Caer-Dineval?"

## Tools

| Tool | Description |
|------|-------------|
| `list_worlds` | List loaded worlds with resource/calendar counts. Includes world guide if present. |
| `list_resources` | List resources, optionally filtered by `tag` or `name` |
| `get_resource` | Get a resource by ID or name with full markdown content. Hidden documents/properties are included with visibility annotations. |
| `get_resource_tree` | Get the nested resource hierarchy |
| `search_content` | Search page text and timeline event names. Results include visibility flag. |
| `get_calendar` | Get a custom calendar definition |
| `get_map` | Get map data: pins with positions, regions with vertex coordinates, paths with full waypoint coordinates, labels, and calibration. Coordinates enable precise distance/area calculations. |

All tools that operate on a specific world take an optional `world` parameter (the filename without `.lk`). If only one world is loaded, it's used automatically.

## Checking exports

List generated `.lk` files waiting to be imported into Legend Keeper:

```sh
legend-keeper-mcp exports
```

Shows the world name, resource count, export date, and file size for each file in `~/.lk-worlds/exports/`.

## World guide (`.lk-guide`)

You can give Claude world-specific instructions by creating a resource in Legend Keeper and tagging it `llm-guide`. Its page content will be included in the `list_worlds` response, so Claude sees it at the start of every conversation.

Example guide content:

- "This is a D&D 5e campaign set in the Forgotten Realms"
- "Never reveal content from resources tagged 'dm-secret' unless I ask"
- "Use the Harptos calendar for all dates"

## World generation

Use the generation tools to have an LLM build a new Legend Keeper world from scratch. The LLM creates resources, writes content in markdown, and exports a `.lk` file you can import into Legend Keeper. Generated resources are **hidden by default** so you can review them before showing to players — unhide resources in Legend Keeper as you approve them.

Templates are extracted from your loaded worlds — the same templates you've defined in Legend Keeper's Templates folder. When creating a resource, the LLM picks a template and the server copies its property blocks (IMAGE, FRIENDS, ENEMIES, TAGS, etc.) onto the new resource.

Example prompts:
- "List the available templates, then create a new world called 'Sunken Isles' with 5 island locations using the Location template"
- "Add an NPC called Captain Reef using the NPC template, with a backstory"
- "Export the world so I can import it into Legend Keeper"

Generation tools work alongside read tools — the LLM can reference your existing worlds while building a new one.

### Generation tools

| Tool | Description |
|------|-------------|
| `create_world` | Start a new world |
| `list_templates` | List available templates from loaded worlds (NPC, Location, Character, etc.) with their property blocks |
| `create_resource` | Add a resource with optional template, markdown content, tags, aliases, and visibility |
| `add_document` | Add an additional document to a resource, optionally hidden |
| `set_content` | Update a document's content |
| `list_draft` | See what's been built so far |
| `export_world` | Write the `.lk` file to disk |
| `batch_create` | Create the world + multiple resources with all their documents in one call. Supports templates, tags, content, aliases, visibility, and parent references by name within the batch. |

**Prefer `batch_create`** over calling `create_resource` + `add_document` individually — it's much faster since it avoids per-resource round trips.

Exported files go to `~/.lk-worlds/exports/` by default.

## Player mode (sharing with friends)

Start the server in player mode to share your world with friends over HTTP. All hidden content (resources, documents, properties) is hard-filtered from memory — players can never access DM-only content.

```sh
legend-keeper-mcp --player --secret <shared-token> --port 8080
```

Friends configure their MCP client (Claude Desktop, ChatGPT, etc.) to connect to your server:

```json
{
  "mcpServers": {
    "legend-keeper": {
      "url": "https://your-server.example.com/mcp",
      "headers": {
        "Authorization": "Bearer <shared-token>"
      }
    }
  }
}
```

### Visibility rules

- Resources marked hidden in Legend Keeper are removed entirely, along with their entire subtree of children
- Hidden documents on a visible resource are removed
- Hidden properties on a visible resource are removed

### Docker

```sh
docker build -t legend-keeper-mcp .
docker run -p 8080:8080 \
  -v /path/to/worlds:/data/worlds \
  -e LK_SECRET=your-secret-here \
  legend-keeper-mcp --player
```

## Worlds directory resolution

The server looks for the worlds directory in this order:

1. CLI argument: `legend-keeper-mcp /path/to/worlds`
2. `LK_WORLDS` environment variable
3. Default: `~/.lk-worlds/` (local) or `/data/worlds/` (Docker)

## Running tests

Place `.lk` files in `tests/reference/`, then:

```sh
cargo test
```
