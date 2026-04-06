use std::future::Future;
use std::sync::{Arc, Mutex};

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::lk::builder::WorldBuilder;
use crate::lk::schema::{Document, MapContent, Resource, TimelineContent};
use crate::lk::store::WorldStore;
use crate::prosemirror::to_markdown::to_markdown;

#[derive(Clone)]
pub struct LkServer {
    store: WorldStore,
    builder: Arc<Mutex<Option<WorldBuilder>>>,
    tool_router: ToolRouter<Self>,
}

// --- Parameter types ---

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListWorldsParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListResourcesParams {
    /// World name (filename stem). Optional if only one world is loaded.
    pub world: Option<String>,
    /// Filter by tag (exact match, case-insensitive).
    pub tag: Option<String>,
    /// Filter by name (substring match, case-insensitive).
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetResourceParams {
    /// World name (filename stem). Optional if only one world is loaded.
    pub world: Option<String>,
    /// Resource ID (8-char) or exact name (case-insensitive).
    pub id_or_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetResourceTreeParams {
    /// World name (filename stem). Optional if only one world is loaded.
    pub world: Option<String>,
    /// Root resource ID. If omitted, returns the full tree from top-level resources.
    pub root_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchContentParams {
    /// World name (filename stem). Optional if only one world is loaded.
    pub world: Option<String>,
    /// Search query (case-insensitive substring match).
    pub query: String,
    /// Maximum results to return (default 20).
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetMapParams {
    /// World name (filename stem). Optional if only one world is loaded.
    pub world: Option<String>,
    /// Resource ID (8-char) or exact name (case-insensitive).
    pub id_or_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetBoardParams {
    /// World name (filename stem). Optional if only one world is loaded.
    pub world: Option<String>,
    /// Resource ID (8-char) or exact name (case-insensitive).
    pub id_or_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetCalendarParams {
    /// World name (filename stem). Optional if only one world is loaded.
    pub world: Option<String>,
    /// Calendar ID or name (case-insensitive).
    pub id_or_name: String,
}

// --- Generation parameter types ---

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateWorldParams {
    /// Name for the new world.
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateResourceParams {
    /// Resource name.
    pub name: String,
    /// Parent resource ID. Omit for a top-level resource.
    pub parent_id: Option<String>,
    /// Tags for the resource.
    pub tags: Option<Vec<String>>,
    /// Markdown content for the resource's main page document.
    pub content: Option<String>,
    /// Mark this resource as hidden (DM-only). Defaults to true — resources are hidden on export so the DM can review before showing to players. Set to false to make visible immediately.
    pub is_hidden: Option<bool>,
    /// Template name to apply (e.g. "NPC", "Location"). Use list_templates to see available templates. Copies property blocks from the template.
    pub template: Option<String>,
    /// Alternative names for this resource.
    pub aliases: Option<Vec<String>>,
    /// World to source the template from. Only needed if multiple worlds are loaded.
    pub template_world: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTemplatesParams {
    /// World to list templates from. If only one world is loaded, this can be omitted.
    pub world: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddDocumentParams {
    /// Resource ID to add the document to.
    pub resource_id: String,
    /// Document name (e.g. "DM Notes", "History").
    pub name: String,
    /// Markdown content for the document.
    pub content: String,
    /// Document type: "page" (default), "map", or "time".
    pub doc_type: Option<String>,
    /// Mark this document as hidden (DM-only). Defaults to false.
    pub is_hidden: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetContentParams {
    /// Resource ID.
    pub resource_id: String,
    /// Document ID. If omitted, updates the first page document.
    pub document_id: Option<String>,
    /// Markdown content.
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteResourceParams {
    /// Resource ID to delete from the draft world.
    pub resource_id: String,
    /// If true, also deletes all child resources (entire subtree). If false (default), fails when the resource has children.
    pub recursive: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReparentResourceParams {
    /// Resource ID to move.
    pub resource_id: String,
    /// New parent resource ID. Omit or set to null to make it a top-level resource.
    pub new_parent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDraftResourceParams {
    /// Resource ID (8-char) or exact name (case-insensitive).
    pub id_or_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDraftDocumentParams {
    /// Resource ID.
    pub resource_id: String,
    /// Document ID. If omitted, returns the first page document.
    pub document_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateDraftResourceParams {
    /// Resource ID to update.
    pub resource_id: String,
    /// New name for the resource.
    pub name: Option<String>,
    /// Replace all tags (full replacement, not additive).
    pub tags: Option<Vec<String>>,
    /// Update visibility (true = hidden/DM-only).
    pub is_hidden: Option<bool>,
    /// Replace all aliases (full replacement, not additive).
    pub aliases: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteDocumentParams {
    /// Resource ID.
    pub resource_id: String,
    /// Document ID to delete.
    pub document_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListDraftParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LoadDraftParams {
    /// Name of the world to load. Checks the exports directory first
    /// (~/.lk-worlds/exports/{name}.lk), then falls back to cloning
    /// from a loaded world in the WorldStore.
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportWorldParams {
    /// Output file path. Defaults to ~/.lk-worlds/exports/{name}.lk
    pub output_path: Option<String>,
}

// --- Batch creation types ---

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchDocumentSpec {
    /// Document name (e.g. "DM Notes", "History").
    pub name: String,
    /// Markdown content for the document.
    pub content: String,
    /// Document type: "page" (default), "map", or "time".
    pub doc_type: Option<String>,
    /// Mark this document as hidden (DM-only). Defaults to false.
    pub is_hidden: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchResourceSpec {
    /// Resource name.
    pub name: String,
    /// Parent resource ID, or the name of another resource in this batch.
    pub parent: Option<String>,
    /// Tags for the resource.
    pub tags: Option<Vec<String>>,
    /// Markdown content for the resource's main page document.
    pub content: Option<String>,
    /// Mark this resource as hidden (DM-only). Defaults to true — resources are hidden on export so the DM can review before showing to players. Set to false to make visible immediately.
    pub is_hidden: Option<bool>,
    /// Template name to apply (e.g. "NPC", "Location").
    pub template: Option<String>,
    /// Alternative names for this resource.
    pub aliases: Option<Vec<String>>,
    /// Additional documents beyond the main page.
    pub documents: Option<Vec<BatchDocumentSpec>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchCreateParams {
    /// World name. If no draft world exists, one will be created with this name. Required if no draft world exists.
    pub world_name: Option<String>,
    /// World to source templates from. Only needed if multiple worlds are loaded.
    pub template_world: Option<String>,
    /// Resources to create, in order. Parent references can use IDs or names of resources earlier in this array.
    pub resources: Vec<BatchResourceSpec>,
}

// --- Tool implementations ---

#[tool_router]
impl LkServer {
    pub fn new(store: WorldStore) -> Self {
        Self {
            store,
            builder: Arc::new(Mutex::new(None)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List all loaded worlds with resource and calendar counts. If a world has a resource tagged 'llm-guide', its content is returned in the 'guide' field — read and follow these instructions.")]
    async fn list_worlds(&self, _params: Parameters<ListWorldsParams>) -> String {
        let worlds = self.store.list_worlds();
        serde_json::to_string_pretty(&worlds).unwrap_or_else(|e| format!("Error: {}", e))
    }

    #[tool(description = "List resources in a world, optionally filtered by tag or name. Returns summaries (id, name, tags, parentId) without document content.")]
    async fn list_resources(
        &self,
        Parameters(params): Parameters<ListResourcesParams>,
    ) -> Result<String, String> {
        let resources = self
            .store
            .list_resources(&params.world, &params.tag, &params.name)
            .map_err(|e| e.to_string())?;
        serde_json::to_string_pretty(&resources).map_err(|e| e.to_string())
    }

    #[tool(description = "Get a resource by ID or name. Returns metadata and all document content rendered as markdown. Timeline docs include lane and event summaries.")]
    async fn get_resource(
        &self,
        Parameters(params): Parameters<GetResourceParams>,
    ) -> Result<String, String> {
        let resource = self
            .store
            .get_resource(&params.world, &params.id_or_name)
            .map_err(|e| e.to_string())?;
        Ok(format_resource(&resource))
    }

    #[tool(description = "Get the resource tree structure. Returns nested JSON with id, name, and children. If root_id is provided, returns children of that resource.")]
    async fn get_resource_tree(
        &self,
        Parameters(params): Parameters<GetResourceTreeParams>,
    ) -> Result<String, String> {
        let tree = self
            .store
            .get_resource_tree(&params.world, &params.root_id)
            .map_err(|e| e.to_string())?;
        serde_json::to_string_pretty(&tree).map_err(|e| e.to_string())
    }

    #[tool(description = "Search page content, timeline event names, and board shape text. Returns matching snippets with resource and document context.")]
    async fn search_content(
        &self,
        Parameters(params): Parameters<SearchContentParams>,
    ) -> Result<String, String> {
        let results = self
            .store
            .search_content(&params.world, &params.query, params.limit)
            .map_err(|e| e.to_string())?;
        serde_json::to_string_pretty(&results).map_err(|e| e.to_string())
    }

    #[tool(description = "Get map data for a resource. Returns pins, regions, paths, labels, calibration, and the map image when available. Use this to visually inspect a map or reason about spatial relationships.")]
    async fn get_map(
        &self,
        Parameters(params): Parameters<GetMapParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let resource = self
            .store
            .get_resource(&params.world, &params.id_or_name)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let map_doc = resource
            .documents
            .iter()
            .find(|d| d.doc_type == "map")
            .ok_or_else(|| {
                ErrorData::internal_error(
                    format!("Resource '{}' has no map document", resource.name),
                    None,
                )
            })?;

        let hidden_tag = if map_doc.is_hidden { " *(hidden)*" } else { "" };
        let text = format_map_document(map_doc, &resource.name, hidden_tag);
        let mut contents = vec![Content::text(text)];

        if let Some(map) = &map_doc.map {
            let url = &map.map_id;
            if url.starts_with("http") {
                match WorldStore::fetch_image(url).await {
                    Ok((bytes, mime)) => {
                        use base64::Engine;
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        contents.push(Content::image(b64, mime));
                    }
                    Err(e) => {
                        contents.push(Content::text(format!(
                            "(Could not fetch map image: {})",
                            e
                        )));
                    }
                }
            }
        }

        Ok(CallToolResult::success(contents))
    }

    #[tool(description = "Get board data for a resource. Returns a structured summary of tldraw shapes (geo, arrow, text, line), bindings, and the connection graph between labeled nodes.")]
    async fn get_board(
        &self,
        Parameters(params): Parameters<GetBoardParams>,
    ) -> Result<String, String> {
        let resource = self
            .store
            .get_resource(&params.world, &params.id_or_name)
            .map_err(|e| e.to_string())?;
        let board_doc = resource
            .documents
            .iter()
            .find(|d| d.doc_type == "board")
            .ok_or_else(|| format!("Resource '{}' has no board document", resource.name))?;
        let hidden_tag = if board_doc.is_hidden {
            " *(hidden)*"
        } else {
            ""
        };
        Ok(format_board_document(board_doc, hidden_tag))
    }

    #[tool(description = "Get a calendar definition by ID or name. Returns month, weekday, and era structure.")]
    async fn get_calendar(
        &self,
        Parameters(params): Parameters<GetCalendarParams>,
    ) -> Result<String, String> {
        let calendar = self
            .store
            .get_calendar(&params.world, &params.id_or_name)
            .map_err(|e| e.to_string())?;
        serde_json::to_string_pretty(&calendar).map_err(|e| e.to_string())
    }

    // --- Generation tools ---

    #[tool(description = "Create a new world for generation. Only one world can be built at a time. Call export_world when done to produce a .lk file.")]
    async fn create_world(
        &self,
        Parameters(params): Parameters<CreateWorldParams>,
    ) -> Result<String, String> {
        let mut builder = self.builder.lock().map_err(|e| e.to_string())?;
        *builder = Some(WorldBuilder::new(&params.name));
        Ok(format!("Created draft world: {}", params.name))
    }

    #[tool(description = "List available resource templates from loaded worlds. Templates define property blocks (IMAGE, FRIENDS, ENEMIES, TAGS, etc.) that are copied onto new resources. Use list_templates before create_resource to pick the right template.")]
    async fn list_templates(
        &self,
        Parameters(params): Parameters<ListTemplatesParams>,
    ) -> Result<String, String> {
        let templates = self
            .store
            .list_templates(&params.world)
            .map_err(|e| e.to_string())?;
        serde_json::to_string_pretty(&templates).map_err(|e| e.to_string())
    }

    #[tool(description = "Create a resource in the draft world. Provide markdown content for the main page document. Use the 'template' param to apply property blocks from a template (call list_templates first). Returns the resource ID for use as parent_id in child resources.")]
    async fn create_resource(
        &self,
        Parameters(params): Parameters<CreateResourceParams>,
    ) -> Result<String, String> {
        // If a template is specified, look it up from the WorldStore
        let template_data = if let Some(ref template_name) = params.template {
            let (props, template_tags, icon) = self
                .store
                .get_template_properties(&params.template_world, template_name)
                .map_err(|e| e.to_string())?;
            Some((props, template_tags, icon))
        } else {
            None
        };

        let mut builder = self.builder.lock().map_err(|e| e.to_string())?;
        let b = builder.as_mut().ok_or_else(|| "No draft world — call create_world first".to_string())?;

        // Merge tags: explicit tags + template tags (deduped)
        let mut tags = params.tags.unwrap_or_default();
        if let Some((_, ref template_tags, _)) = template_data {
            for t in template_tags {
                if !tags.iter().any(|existing| existing.eq_ignore_ascii_case(t)) {
                    tags.push(t.clone());
                }
            }
        }

        let (props, icon_color, icon_glyph, icon_shape) = match template_data {
            Some((props, _, icon)) => (props, icon.icon_color, icon.icon_glyph, icon.icon_shape),
            None => (Vec::new(), None, None, None),
        };

        let summary = b
            .create_resource(
                &params.name,
                params.parent_id.as_deref(),
                Some(tags),
                params.content.as_deref(),
                params.is_hidden.unwrap_or(true),
                params.aliases.unwrap_or_default(),
                props,
                icon_color,
                icon_glyph,
                icon_shape,
            )
            .map_err(|e| e.to_string())?;
        serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())
    }

    #[tool(description = "Add an additional document to a resource in the draft world. Use this for secondary pages like 'DM Notes' or 'History'.")]
    async fn add_document(
        &self,
        Parameters(params): Parameters<AddDocumentParams>,
    ) -> Result<String, String> {
        let mut builder = self.builder.lock().map_err(|e| e.to_string())?;
        let b = builder.as_mut().ok_or_else(|| "No draft world — call create_world first".to_string())?;
        let summary = b
            .add_document(
                &params.resource_id,
                &params.name,
                &params.content,
                params.doc_type.as_deref(),
                params.is_hidden.unwrap_or(false),
            )
            .map_err(|e| e.to_string())?;
        serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())
    }

    #[tool(description = "Update the content of a document in the draft world. If document_id is omitted, updates the first page document of the resource.")]
    async fn set_content(
        &self,
        Parameters(params): Parameters<SetContentParams>,
    ) -> Result<String, String> {
        let mut builder = self.builder.lock().map_err(|e| e.to_string())?;
        let b = builder.as_mut().ok_or_else(|| "No draft world — call create_world first".to_string())?;
        b.set_content(
            &params.resource_id,
            params.document_id.as_deref(),
            &params.content,
        )
        .map_err(|e| e.to_string())?;
        Ok("Content updated".to_string())
    }

    #[tool(description = "Delete a resource from the draft world. By default, fails if the resource has children — set recursive=true to delete the entire subtree. Use this to clean up unwanted resources before exporting.")]
    async fn delete_resource(
        &self,
        Parameters(params): Parameters<DeleteResourceParams>,
    ) -> Result<String, String> {
        let mut builder = self.builder.lock().map_err(|e| e.to_string())?;
        let b = builder.as_mut().ok_or_else(|| "No draft world — call create_world first".to_string())?;
        let deleted = b
            .delete_resource(&params.resource_id, params.recursive.unwrap_or(false))
            .map_err(|e| e.to_string())?;
        let result = serde_json::json!({
            "deleted_count": deleted.len(),
            "deleted_ids": deleted,
        });
        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
    }

    #[tool(description = "Move a resource to a different parent in the draft world. Set new_parent_id to null or omit it to make the resource top-level. Use this to reorganize resources before exporting.")]
    async fn reparent_resource(
        &self,
        Parameters(params): Parameters<ReparentResourceParams>,
    ) -> Result<String, String> {
        let mut builder = self.builder.lock().map_err(|e| e.to_string())?;
        let b = builder.as_mut().ok_or_else(|| "No draft world — call create_world first".to_string())?;
        b.reparent_resource(&params.resource_id, params.new_parent_id.as_deref())
            .map_err(|e| e.to_string())?;
        Ok(format!(
            "Resource {} reparented to {}",
            params.resource_id,
            params.new_parent_id.as_deref().unwrap_or("top-level")
        ))
    }

    #[tool(description = "Get a draft resource by ID or name. Returns metadata and all document content rendered as markdown, matching the format of get_resource. Use this to review draft content before exporting.")]
    async fn get_draft_resource(
        &self,
        Parameters(params): Parameters<GetDraftResourceParams>,
    ) -> Result<String, String> {
        let builder = self.builder.lock().map_err(|e| e.to_string())?;
        let b = builder.as_ref().ok_or_else(|| "No draft world — call create_world first".to_string())?;
        let resource = b.get_draft_resource(&params.id_or_name).map_err(|e| e.to_string())?;
        Ok(format_resource(resource))
    }

    #[tool(description = "Get a single document from a draft resource. Returns the document's content as markdown with metadata. If document_id is omitted, returns the first page document.")]
    async fn get_draft_document(
        &self,
        Parameters(params): Parameters<GetDraftDocumentParams>,
    ) -> Result<String, String> {
        let builder = self.builder.lock().map_err(|e| e.to_string())?;
        let b = builder.as_ref().ok_or_else(|| "No draft world — call create_world first".to_string())?;
        let doc = b.get_draft_document(&params.resource_id, params.document_id.as_deref()).map_err(|e| e.to_string())?;

        let mut out = String::new();
        out.push_str(&format!("**Document:** {}\n", doc.name));
        out.push_str(&format!("**ID:** {}\n", doc.id));
        out.push_str(&format!("**Type:** {}\n", doc.doc_type));
        out.push_str(&format!("**Hidden:** {}\n\n", doc.is_hidden));

        if let Some(content) = &doc.content {
            if doc.doc_type == "page" {
                let md = to_markdown(content);
                if !md.is_empty() {
                    out.push_str(&md);
                }
            } else {
                out.push_str(&serde_json::to_string_pretty(content).unwrap_or_default());
            }
        }

        Ok(out.trim_end().to_string())
    }

    #[tool(description = "Update metadata (name, tags, visibility, aliases) on a draft resource. Does not change document content — use set_content for that. Tags and aliases are fully replaced, not merged.")]
    async fn update_draft_resource(
        &self,
        Parameters(params): Parameters<UpdateDraftResourceParams>,
    ) -> Result<String, String> {
        let mut builder = self.builder.lock().map_err(|e| e.to_string())?;
        let b = builder.as_mut().ok_or_else(|| "No draft world — call create_world first".to_string())?;
        let summary = b.update_resource(
            &params.resource_id,
            params.name.as_deref(),
            params.tags,
            params.is_hidden,
            params.aliases,
        ).map_err(|e| e.to_string())?;
        serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())
    }

    #[tool(description = "Delete a document from a draft resource. Cannot delete the last remaining document — every resource must have at least one.")]
    async fn delete_document(
        &self,
        Parameters(params): Parameters<DeleteDocumentParams>,
    ) -> Result<String, String> {
        let mut builder = self.builder.lock().map_err(|e| e.to_string())?;
        let b = builder.as_mut().ok_or_else(|| "No draft world — call create_world first".to_string())?;
        let doc_name = b.delete_document(&params.resource_id, &params.document_id).map_err(|e| e.to_string())?;
        Ok(format!("Deleted document: {}", doc_name))
    }

    #[tool(description = "List all resources in the draft world. Shows the current state of the world being built.")]
    async fn list_draft(&self, _params: Parameters<ListDraftParams>) -> Result<String, String> {
        let builder = self.builder.lock().map_err(|e| e.to_string())?;
        let b = builder.as_ref().ok_or_else(|| "No draft world — call create_world first".to_string())?;
        let summary = b.list_draft();
        serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())
    }

    #[tool(description = "Load an existing world into the draft builder for editing. Checks the exports directory first (~/.lk-worlds/exports/{name}.lk), then falls back to cloning from loaded worlds. Replaces any existing draft. After loading, use draft editing tools (set_content, delete_resource, etc.) to modify, then export_world to save.")]
    async fn load_draft(
        &self,
        Parameters(params): Parameters<LoadDraftParams>,
    ) -> Result<String, String> {
        use crate::lk::io::read_lk_file;
        use std::path::PathBuf;

        let home = std::env::var("HOME").map_err(|_| "HOME environment variable not set".to_string())?;
        let export_path = PathBuf::from(&home)
            .join(".lk-worlds/exports")
            .join(format!("{}.lk", params.name));

        let (root, source) = if export_path.exists() {
            let root = read_lk_file(&export_path).map_err(|e| e.to_string())?;
            (root, "exports")
        } else {
            let root = self.store.get_world(&params.name).map_err(|e| e.to_string())?;
            (root, "store")
        };

        let resource_count = root.resources.len();

        let mut builder = self.builder.lock().map_err(|e| e.to_string())?;
        *builder = Some(WorldBuilder::from_lk_root(params.name.clone(), root));

        Ok(format!(
            "Loaded draft world '{}' from {} ({} resources). Use draft tools to edit, then export_world to save.",
            params.name, source, resource_count
        ))
    }

    #[tool(description = "Export the draft world as a .lk file. Returns the file path. The file can be imported into Legend Keeper.")]
    async fn export_world(
        &self,
        Parameters(params): Parameters<ExportWorldParams>,
    ) -> Result<String, String> {
        let mut builder = self.builder.lock().map_err(|e| e.to_string())?;
        let b = builder.as_mut().ok_or_else(|| "No draft world — call create_world first".to_string())?;
        let path = b
            .export_world(params.output_path.as_deref())
            .map_err(|e| e.to_string())?;
        Ok(format!("Exported to: {}", path.display()))
    }

    #[tool(description = "Create multiple resources (with all their documents) in a single call. Optionally creates the draft world too. Each resource can specify a template, tags, content, aliases, visibility, and additional documents. Use this instead of calling create_resource + add_document repeatedly. Parent references can use the name of a resource earlier in the same batch.")]
    async fn batch_create(
        &self,
        Parameters(params): Parameters<BatchCreateParams>,
    ) -> Result<String, String> {
        // Pre-resolve all unique template names from the WorldStore before taking the builder lock
        use crate::lk::store::TemplateIcon;
        let mut template_cache: std::collections::HashMap<String, (Vec<crate::lk::schema::Property>, Vec<String>, TemplateIcon)> = std::collections::HashMap::new();
        for spec in &params.resources {
            if let Some(ref tname) = spec.template {
                let key = tname.to_lowercase();
                if !template_cache.contains_key(&key) {
                    let (props, tags, icon) = self
                        .store
                        .get_template_properties(&params.template_world, tname)
                        .map_err(|e| e.to_string())?;
                    template_cache.insert(key, (props, tags, icon));
                }
            }
        }

        let mut builder = self.builder.lock().map_err(|e| e.to_string())?;

        // Create world if needed
        if let Some(ref world_name) = params.world_name {
            if builder.is_none() {
                *builder = Some(WorldBuilder::new(world_name));
            }
        }

        let b = builder.as_mut().ok_or_else(|| {
            "No draft world — provide world_name or call create_world first".to_string()
        })?;

        // Track name→id mappings for parent references within this batch
        let mut name_to_id: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let mut created: Vec<serde_json::Value> = Vec::new();

        for spec in params.resources {
            // Resolve parent: try as ID first, then as a batch name reference
            let parent_id = spec.parent.as_ref().map(|p| {
                name_to_id
                    .get(&p.to_lowercase())
                    .cloned()
                    .unwrap_or_else(|| p.clone())
            });

            // Merge tags with template tags and extract icon
            let mut tags = spec.tags.unwrap_or_default();
            let (properties, icon_color, icon_glyph, icon_shape) = if let Some(ref tname) = spec.template {
                let key = tname.to_lowercase();
                if let Some((props, template_tags, icon)) = template_cache.get(&key) {
                    for t in template_tags {
                        if !tags.iter().any(|existing| existing.eq_ignore_ascii_case(t)) {
                            tags.push(t.clone());
                        }
                    }
                    (props.clone(), icon.icon_color.clone(), icon.icon_glyph.clone(), icon.icon_shape.clone())
                } else {
                    (Vec::new(), None, None, None)
                }
            } else {
                (Vec::new(), None, None, None)
            };

            let summary = b
                .create_resource(
                    &spec.name,
                    parent_id.as_deref(),
                    Some(tags),
                    spec.content.as_deref(),
                    spec.is_hidden.unwrap_or(true),
                    spec.aliases.unwrap_or_default(),
                    properties,
                    icon_color,
                    icon_glyph,
                    icon_shape,
                )
                .map_err(|e| e.to_string())?;

            let resource_id = summary.id.clone();
            name_to_id.insert(spec.name.to_lowercase(), resource_id.clone());

            let mut doc_summaries: Vec<serde_json::Value> = Vec::new();

            // Add additional documents
            if let Some(docs) = spec.documents {
                for doc_spec in docs {
                    let doc_summary = b
                        .add_document(
                            &resource_id,
                            &doc_spec.name,
                            &doc_spec.content,
                            doc_spec.doc_type.as_deref(),
                            doc_spec.is_hidden.unwrap_or(false),
                        )
                        .map_err(|e| e.to_string())?;
                    doc_summaries.push(serde_json::json!({
                        "id": doc_summary.id,
                        "name": doc_summary.name,
                    }));
                }
            }

            let mut entry = serde_json::json!({
                "id": summary.id,
                "name": summary.name,
            });
            if !doc_summaries.is_empty() {
                entry["additional_documents"] = serde_json::json!(doc_summaries);
            }
            created.push(entry);
        }

        let result = serde_json::json!({
            "created": created.len(),
            "resources": created,
        });
        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
    }
}

#[tool_handler]
impl ServerHandler for LkServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Legend Keeper MCP server. Provides read access to .lk world-building files. Use list_worlds to see available worlds, then browse resources, search content, and view calendars. You can also generate new worlds: call list_templates first to see available templates (NPC, Location, etc.), then use batch_create to create the world and all resources with their documents in a single call. Each resource can have a template, content, tags, aliases, visibility, and additional documents (like DM Notes). Use export_world when done to produce a .lk file for import into Legend Keeper. Use load_draft to reload a previously exported world for continued editing. Prefer batch_create over individual create_resource/add_document calls for efficiency. Resources are hidden by default so the DM can review before showing to players — set is_hidden to false to make a resource immediately visible. Use delete_resource and reparent_resource to clean up or reorganize the draft before exporting. Use get_draft_resource and get_draft_document to review draft content, update_draft_resource to fix metadata (name, tags, visibility, aliases), and delete_document to remove unwanted documents.".to_string()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..ServerInfo::default()
        }
    }
}

/// Format a resource with metadata and document content as markdown.
fn format_resource(resource: &Resource) -> String {
    let mut out = String::new();

    // Metadata header
    out.push_str(&format!("# {}\n\n", resource.name));
    out.push_str(&format!("**ID:** {}\n", resource.id));
    if let Some(parent_id) = &resource.parent_id {
        out.push_str(&format!("**Parent ID:** {}\n", parent_id));
    }
    if !resource.tags.is_empty() {
        out.push_str(&format!("**Tags:** {}\n", resource.tags.join(", ")));
    }
    if !resource.aliases.is_empty() {
        out.push_str(&format!("**Aliases:** {}\n", resource.aliases.join(", ")));
    }

    // Properties (skip TAGS/ALIAS since they're shown above)
    for prop in &resource.properties {
        if matches!(prop.prop_type.as_str(), "TAGS" | "ALIAS") {
            continue;
        }
        let value = match &prop.data {
            Some(v) => format_property_value(v),
            None => continue,
        };
        if !value.is_empty() && value != "{}" {
            let hidden_tag = if prop.is_hidden == Some(true) {
                " *(hidden)*"
            } else {
                ""
            };
            out.push_str(&format!("**{}{}:** {}\n", prop.title, hidden_tag, value));
        }
    }

    out.push('\n');

    // Documents
    for doc in &resource.documents {
        let hidden_tag = if doc.is_hidden { " *(hidden)*" } else { "" };

        match doc.doc_type.as_str() {
            "page" => {
                if resource.documents.len() > 1 {
                    out.push_str(&format!("## 📄 {}{}\n\n", doc.name, hidden_tag));
                }
                if let Some(content) = &doc.content {
                    let md = to_markdown(content);
                    if !md.is_empty() {
                        out.push_str(&md);
                        out.push_str("\n\n");
                    }
                }
            }
            "map" => {
                out.push_str(&format_map_document(doc, &resource.name, hidden_tag));
                out.push_str("\n\n");
            }
            "time" => {
                out.push_str(&format!("## 📅 {}{}\n\n", doc.name, hidden_tag));
                if let Some(calendar_id) = &doc.calendar_id {
                    out.push_str(&format!("Calendar: {}\n\n", calendar_id));
                }
                if let Some(content) = &doc.content {
                    if let Ok(timeline) =
                        serde_json::from_value::<TimelineContent>(content.clone())
                    {
                        if !timeline.lanes.is_empty() {
                            out.push_str("**Lanes:**\n");
                            for lane in &timeline.lanes {
                                out.push_str(&format!("- {}\n", lane.name));
                            }
                            out.push('\n');
                        }
                        if !timeline.events.is_empty() {
                            out.push_str("**Events:**\n");
                            for event in &timeline.events {
                                let time_str = if let Some(end) = event.end {
                                    format!("{} to {}", event.start, end)
                                } else {
                                    format!("{}", event.start)
                                };
                                out.push_str(&format!("- {} ({})\n", event.name, time_str));
                            }
                            out.push('\n');
                        }
                    }
                }
            }
            "board" => {
                out.push_str(&format_board_document(doc, hidden_tag));
                out.push_str("\n\n");
            }
            _ => {}
        }
    }

    out.trim_end().to_string()
}

/// Extract a resource ID from a `lk://resources/{id}/docs/{id}` URI.
fn extract_resource_id_from_uri(uri: &str) -> Option<&str> {
    let rest = uri.strip_prefix("lk://resources/")?;
    rest.split('/').next()
}

/// Format a map document with pins, regions, paths, labels, and calibration.
fn format_map_document(doc: &Document, _resource_name: &str, hidden_tag: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("## 🗺️ {}{}\n\n", doc.name, hidden_tag));

    if let Some(map) = &doc.map {
        out.push_str(&format!("Map image: {}\n", map.map_id));
        out.push_str(&format!(
            "Bounds: ({}, {}) to ({}, {})\n",
            map.min_x, map.min_y, map.max_x, map.max_y
        ));
    }

    // Calibration
    if let Some(pres) = &doc.presentation {
        if let Some(cal) = &pres.calibration {
            out.push_str(&format!(
                "Scale: 1 map unit = {} {}\n",
                cal.real_units_per_map_unit, cal.unit
            ));
        }
    }

    // Parse map content for features
    let features = doc
        .content
        .as_ref()
        .and_then(|c| serde_json::from_value::<MapContent>(c.clone()).ok());

    if let Some(map_content) = features {
        let pins: Vec<_> = map_content
            .pins
            .iter()
            .filter(|f| f.feature_type.is_none())
            .collect();
        let regions: Vec<_> = map_content
            .pins
            .iter()
            .filter(|f| f.feature_type.as_deref() == Some("region"))
            .collect();
        let paths: Vec<_> = map_content
            .pins
            .iter()
            .filter(|f| f.feature_type.as_deref() == Some("path"))
            .collect();
        let labels: Vec<_> = map_content
            .pins
            .iter()
            .filter(|f| f.feature_type.as_deref() == Some("label"))
            .collect();

        if !pins.is_empty() {
            out.push_str(&format!("\n**Pins ({}):**\n", pins.len()));
            out.push_str("| Name | Position | Icon | Link |\n");
            out.push_str("|------|----------|------|------|\n");
            for pin in &pins {
                let link = pin
                    .uri
                    .as_deref()
                    .and_then(extract_resource_id_from_uri)
                    .unwrap_or("—");
                let icon = pin.icon_glyph.as_deref().unwrap_or("—");
                out.push_str(&format!(
                    "| {} | ({:.1}, {:.1}) | {} | {} |\n",
                    pin.name, pin.pos[0], pin.pos[1], icon, link
                ));
            }
        }

        if !regions.is_empty() {
            out.push_str(&format!("\n**Regions ({}):**\n", regions.len()));
            for region in &regions {
                let fill = region.fill_style.as_deref().unwrap_or("solid");
                let border = region.border_style.as_deref().unwrap_or("solid");
                let vertices = region
                    .polygon
                    .as_ref()
                    .map(|pts| {
                        pts.iter()
                            .map(|p| format!("({:.1}, {:.1})", p[0], p[1]))
                            .collect::<Vec<_>>()
                            .join(" → ")
                    })
                    .unwrap_or_default();
                out.push_str(&format!(
                    "- **{}** ({} fill, {} border): {}\n",
                    region.name, fill, border, vertices
                ));
            }
        }

        if !paths.is_empty() {
            out.push_str(&format!("\n**Paths ({}):**\n", paths.len()));
            for path in &paths {
                let style = path.stroke_style.as_deref().unwrap_or("solid");
                let width = path.stroke_width.unwrap_or(1.0);
                let waypoints = path
                    .polyline
                    .as_ref()
                    .map(|pts| {
                        pts.iter()
                            .map(|p| format!("({:.1}, {:.1})", p[0], p[1]))
                            .collect::<Vec<_>>()
                            .join(" → ")
                    })
                    .unwrap_or_default();
                out.push_str(&format!(
                    "- **{}** ({}, width {}): {}\n",
                    path.name, style, width, waypoints
                ));
            }
        }

        if !labels.is_empty() {
            out.push_str(&format!("\n**Labels ({}):**\n", labels.len()));
            for label in &labels {
                let size = label.label_size.as_deref().unwrap_or("medium");
                out.push_str(&format!(
                    "- {} ({}, at {:.1}, {:.1})\n",
                    label.name, size, label.pos[0], label.pos[1]
                ));
            }
        }
    }

    out
}

/// Format a board document with shape summary and graph topology.
fn format_board_document(doc: &Document, hidden_tag: &str) -> String {
    use crate::lk::schema::BoardContent;

    let mut out = String::new();
    out.push_str(&format!("## Board: {}{}\n\n", doc.name, hidden_tag));

    let board = match doc
        .content
        .as_ref()
        .and_then(|c| serde_json::from_value::<BoardContent>(c.clone()).ok())
    {
        Some(b) => b,
        None => {
            out.push_str("(no board content)\n");
            return out;
        }
    };

    // Categorize records
    let mut geo_shapes: Vec<&serde_json::Value> = Vec::new();
    let mut arrows: Vec<&serde_json::Value> = Vec::new();
    let mut text_shapes: Vec<&serde_json::Value> = Vec::new();
    let mut line_shapes: Vec<&serde_json::Value> = Vec::new();
    let mut bindings: Vec<&serde_json::Value> = Vec::new();

    for record in &board.shapes_v2 {
        let val = &record.val;
        match val.get("typeName").and_then(|v| v.as_str()) {
            Some("shape") => match val.get("type").and_then(|v| v.as_str()) {
                Some("geo") => geo_shapes.push(val),
                Some("arrow") => arrows.push(val),
                Some("text") => text_shapes.push(val),
                Some("line") => line_shapes.push(val),
                _ => {}
            },
            Some("binding") => bindings.push(val),
            _ => {}
        }
    }

    out.push_str(&format!(
        "**Shapes:** {} ({} geo, {} arrows, {} text, {} lines)\n",
        geo_shapes.len() + arrows.len() + text_shapes.len() + line_shapes.len(),
        geo_shapes.len(),
        arrows.len(),
        text_shapes.len(),
        line_shapes.len()
    ));
    out.push_str(&format!("**Bindings:** {}\n\n", bindings.len()));

    // Geo shapes table
    if !geo_shapes.is_empty() {
        out.push_str(&format!("### Nodes ({} geo shapes)\n", geo_shapes.len()));
        out.push_str("| Label | Geo | Color | Position |\n");
        out.push_str("|-------|-----|-------|----------|\n");
        for shape in &geo_shapes {
            let props = shape.get("props");
            let label = props
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("");
            let geo = props
                .and_then(|p| p.get("geo"))
                .and_then(|g| g.as_str())
                .unwrap_or("rectangle");
            let color = props
                .and_then(|p| p.get("color"))
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let x = shape.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let y = shape.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if !label.is_empty() {
                out.push_str(&format!(
                    "| {} | {} | {} | ({:.0}, {:.0}) |\n",
                    label, geo, color, x, y
                ));
            }
        }
        out.push('\n');
    }

    // Text labels
    if !text_shapes.is_empty() {
        out.push_str(&format!("### Text Labels ({})\n", text_shapes.len()));
        for shape in &text_shapes {
            let props = shape.get("props");
            let text = props
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("(empty)");
            let scale = props
                .and_then(|p| p.get("scale"))
                .and_then(|s| s.as_f64())
                .unwrap_or(1.0);
            let x = shape.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let y = shape.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            out.push_str(&format!(
                "- {} (scale {:.1}, at {:.0}, {:.0})\n",
                text, scale, x, y
            ));
        }
        out.push('\n');
    }

    // Graph connections via bindings
    if !bindings.is_empty() && !arrows.is_empty() {
        // Build id→label map from geo shapes
        let mut id_to_label: std::collections::HashMap<&str, &str> =
            std::collections::HashMap::new();
        for shape in &geo_shapes {
            if let (Some(id), Some(text)) = (
                shape.get("id").and_then(|v| v.as_str()),
                shape
                    .get("props")
                    .and_then(|p| p.get("text"))
                    .and_then(|t| t.as_str()),
            ) {
                if !text.is_empty() {
                    id_to_label.insert(id, text);
                }
            }
        }

        // Build arrow→(start_target, end_target) map from bindings
        let mut arrow_endpoints: std::collections::HashMap<&str, [Option<&str>; 2]> =
            std::collections::HashMap::new();
        for binding in &bindings {
            let from_id = binding.get("fromId").and_then(|v| v.as_str()).unwrap_or("");
            let to_id = binding.get("toId").and_then(|v| v.as_str()).unwrap_or("");
            let terminal = binding
                .get("props")
                .and_then(|p| p.get("terminal"))
                .and_then(|t| t.as_str())
                .unwrap_or("");
            // fromId is the arrow shape, toId is the target shape
            // Strip "shape:" prefix for lookup
            let arrow_id = from_id.strip_prefix("shape:").unwrap_or(from_id);
            let target_id = to_id.strip_prefix("shape:").unwrap_or(to_id);
            let entry = arrow_endpoints.entry(arrow_id).or_insert([None, None]);
            match terminal {
                "start" => entry[0] = Some(target_id),
                "end" => entry[1] = Some(target_id),
                _ => {}
            }
        }

        // Render connections
        let mut connections = Vec::new();
        for (_, endpoints) in &arrow_endpoints {
            if let (Some(start_id), Some(end_id)) = (endpoints[0], endpoints[1]) {
                let start_label = id_to_label.get(start_id).unwrap_or(&start_id);
                let end_label = id_to_label.get(end_id).unwrap_or(&end_id);
                connections.push(format!("- {} -> {}", start_label, end_label));
            }
        }
        if !connections.is_empty() {
            connections.sort();
            out.push_str(&format!("### Graph ({} connections)\n", connections.len()));
            for conn in &connections {
                out.push_str(conn);
                out.push('\n');
            }
            out.push('\n');
        }
    }

    out
}

fn format_property_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<_>>()
            .join(", "),
        serde_json::Value::Object(obj) => {
            // Handle common property data shapes
            if let Some(val) = obj.get("value").and_then(|v| v.as_str()) {
                return val.to_string();
            }
            if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
                return url.to_string();
            }
            serde_json::to_string(value).unwrap_or_default()
        }
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}
