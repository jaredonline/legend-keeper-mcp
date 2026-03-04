use std::future::Future;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::lk::schema::{Resource, TimelineContent};
use crate::lk::store::WorldStore;
use crate::prosemirror::to_markdown::to_markdown;

#[derive(Clone)]
pub struct LkServer {
    store: WorldStore,
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
pub struct GetCalendarParams {
    /// World name (filename stem). Optional if only one world is loaded.
    pub world: Option<String>,
    /// Calendar ID or name (case-insensitive).
    pub id_or_name: String,
}

// --- Tool implementations ---

#[tool_router]
impl LkServer {
    pub fn new(store: WorldStore) -> Self {
        Self {
            store,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List all loaded worlds with resource and calendar counts.")]
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

    #[tool(description = "Search page content and timeline event names. Returns matching snippets with resource and document context.")]
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
}

#[tool_handler]
impl ServerHandler for LkServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Legend Keeper MCP server. Provides read access to .lk world-building files. Use list_worlds to see available worlds, then browse resources, search content, and view calendars.".to_string()),
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
        if prop.is_hidden == Some(true) {
            continue;
        }
        if matches!(prop.prop_type.as_str(), "TAGS" | "ALIAS") {
            continue;
        }
        let value = match &prop.data {
            Some(v) => format_property_value(v),
            None => continue,
        };
        if !value.is_empty() && value != "{}" {
            out.push_str(&format!("**{}:** {}\n", prop.title, value));
        }
    }

    out.push('\n');

    // Documents
    for doc in &resource.documents {
        if doc.is_hidden {
            continue;
        }

        match doc.doc_type.as_str() {
            "page" => {
                if resource.documents.len() > 1 {
                    out.push_str(&format!("## 📄 {}\n\n", doc.name));
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
                out.push_str(&format!("## 🗺️ {}\n\n", doc.name));
                if let Some(map) = &doc.map {
                    out.push_str(&format!(
                        "Map image: {}\nBounds: ({}, {}) to ({}, {})\n\n",
                        map.map_id, map.min_x, map.min_y, map.max_x, map.max_y
                    ));
                }
            }
            "time" => {
                out.push_str(&format!("## 📅 {}\n\n", doc.name));
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
            _ => {}
        }
    }

    out.trim_end().to_string()
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
