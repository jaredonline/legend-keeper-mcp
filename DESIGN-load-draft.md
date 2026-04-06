# Design: Load Draft World

## Problem

Users export draft worlds to `.lk` files via `export_world`, then want to reload them later to continue editing. Currently, `WorldBuilder` only starts empty via `new()` — there's no way to populate it from existing data. The data flow is one-way: builder → export → file. We need the reverse: file → builder.

## Constraints

- Must work with the existing singleton `WorldBuilder` pattern (`Arc<Mutex<Option<WorldBuilder>>>`)
- Must preserve all resource IDs, parent references, document content, and metadata exactly as stored in the `.lk` file — no ID regeneration
- Must be consistent with `create_world`'s behavior (silently replaces any existing draft)
- Out of scope: validation of `.lk` file integrity (duplicate IDs, orphaned parents, etc.)

## Architecture

```mermaid
graph LR
    A[MCP Client] -->|"load_draft(name)"| B[LkServer handler]
    B -->|"1. resolve file"| C{exports dir?}
    C -->|found| D[read_lk_file]
    C -->|not found| E{WorldStore?}
    E -->|found| F[clone LkRoot]
    E -->|not found| G[error]
    D --> H[WorldBuilder::from_lk_root]
    F --> H
    H --> I[builder = Some(wb)]
```

Resolution order for the `name` parameter:
1. Look in exports directory (`~/.lk-worlds/exports/{name}.lk`) — this is the primary use case (reloading a previously exported draft)
2. Fall back to cloning from WorldStore (editing an existing loaded world)
3. Error if not found in either location

## Interfaces

### Internal: WorldBuilder

```rust
// builder.rs — new constructor
impl WorldBuilder {
    /// Create a builder from an existing LkRoot. Preserves all IDs and content.
    pub fn from_lk_root(name: String, root: LkRoot) -> Self {
        Self { name, root }
    }
}
```

No changes to existing builder methods — they all operate on `self.root.resources` by ID lookup and work unchanged on loaded data.

### MCP Tool: load_draft

```rust
// server.rs — new params struct
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LoadDraftParams {
    /// Name of the world to load. Checks the exports directory first
    /// (~/.lk-worlds/exports/{name}.lk), then falls back to cloning
    /// from a loaded world in the WorldStore.
    pub name: String,
}
```

```rust
// server.rs — tool handler
#[tool(description = "Load an existing world into the draft builder for editing. \
    Checks the exports directory first (~/.lk-worlds/exports/{name}.lk), then \
    falls back to cloning from loaded worlds. Replaces any existing draft. \
    After loading, use draft editing tools (set_content, delete_resource, etc.) \
    to modify, then export_world to save.")]
async fn load_draft(
    &self,
    Parameters(params): Parameters<LoadDraftParams>,
) -> Result<String, String> {
    // 1. Try exports directory
    let exports_dir = dirs::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?
        .join(".lk-worlds/exports");
    let export_path = exports_dir.join(format!("{}.lk", params.name));

    let (root, source) = if export_path.exists() {
        let root = read_lk_file(&export_path).map_err(|e| e.to_string())?;
        (root, "exports")
    } else {
        // 2. Fall back to WorldStore
        let root = self.store
            .get_world(&params.name)
            .map_err(|e| e.to_string())?;
        (root, "store")
    };

    let resource_count = root.resources.len();

    // 3. Replace builder (consistent with create_world behavior)
    let mut builder = self.builder.lock().map_err(|e| e.to_string())?;
    *builder = Some(WorldBuilder::from_lk_root(params.name.clone(), root));

    Ok(format!(
        "Loaded draft world '{}' from {} ({} resources). Use draft tools to edit, then export_world to save.",
        params.name, source, resource_count
    ))
}
```

### Internal: WorldStore (new method)

```rust
// store.rs — new method to clone a world's LkRoot
impl WorldStore {
    /// Clone a world's LkRoot for use in the builder.
    pub fn get_world(&self, name: &Option<String>) -> Result<LkRoot, LkError> {
        let worlds = self.worlds.read().unwrap();
        // Use resolve_world pattern but return cloned LkRoot instead of reference
        match name {
            Some(n) => worlds
                .get(n)
                .cloned()
                .ok_or_else(|| LkError::WorldNotFound(n.clone())),
            None => {
                if worlds.len() == 1 {
                    Ok(worlds.values().next().unwrap().clone())
                } else {
                    Err(LkError::InvalidInput(
                        "Multiple worlds loaded — specify a name".into(),
                    ))
                }
            }
        }
    }
}
```

Wait — the `name` param for `load_draft` is required (not `Option`), so `get_world` should take `&str`:

```rust
impl WorldStore {
    pub fn get_world(&self, name: &str) -> Result<LkRoot, LkError> {
        let worlds = self.worlds.read().unwrap();
        worlds
            .get(name)
            .cloned()
            .ok_or_else(|| LkError::WorldNotFound(name.to_string()))
    }
}
```

### Data Schemas

N/A — no schema changes. The `.lk` file format and `LkRoot` struct are unchanged.

## Data Flow

**Load from exports (primary path):**
1. MCP client calls `load_draft(name: "my-world")`
2. Handler checks `~/.lk-worlds/exports/my-world.lk` — exists
3. `read_lk_file()` gunzips and deserializes to `LkRoot`
4. `WorldBuilder::from_lk_root("my-world", root)` wraps it
5. Builder replaces any existing draft
6. Returns success with resource count and source

**Load from WorldStore (fallback):**
1. MCP client calls `load_draft(name: "rime")`
2. Handler checks `~/.lk-worlds/exports/rime.lk` — not found
3. `store.get_world("rime")` clones the `LkRoot` from the store
4. Same steps 4-6 as above

**Editing after load:**
1. Use `set_content`, `delete_resource`, `reparent_resource`, `add_document`, `update_draft_resource` etc.
2. Call `export_world` to save — generates new `exported_at`, `hash`, `resource_count`

## Key Decisions

| Decision | Chosen | Rejected | Why |
|----------|--------|----------|-----|
| Single tool vs two tools | One `load_draft` tool with name-based resolution | Separate `load_draft_from_store` and `load_draft_from_file` | Simpler for the MCP client; the exports-then-store resolution order is deterministic and covers both use cases with one parameter |
| Resolution order | Exports first, then store | Store first, then exports | The primary use case is reloading exported drafts; exports are the user's explicit save points |
| Preserve IDs | Keep all original IDs | Regenerate IDs | Regeneration requires rewriting parent_id chains, locator_ids, ProseMirror mentions, map URIs — complex and error-prone. Original IDs are fine since the builder is isolated from the store |
| Silent replace vs guard | Silent replace (matches `create_world`) | Error if draft exists | Consistency with existing `create_world` behavior. The MCP instructions already tell the agent to export before creating a new world |
| Builder constructor | `from_lk_root(name, root)` | `from_lk_file(path)` that reads + constructs | Separation of concerns: file I/O stays in `io.rs`, builder just takes data. The handler controls resolution logic |
| No arbitrary file path param | Name-only resolution | Accept explicit `path` parameter | Security (path traversal), simplicity. Users can copy files to the worlds or exports dir if needed |

## Invariants

- At most one `WorldBuilder` exists at a time (unchanged)
- All resource IDs, parent references, document IDs, and `locator_id` values are preserved exactly as loaded
- `from_lk_root` does not modify the `LkRoot` — no regeneration of `export_id`, `hash`, etc. (those are recomputed on `export_world`)
- The loaded draft is fully editable with all existing draft tools

## Open Questions

None — this is a straightforward feature with clear interfaces.
