use std::path::PathBuf;

use chrono::Utc;
use serde::Serialize;

use std::collections::HashSet;

use super::io::{compute_hash, generate_id, write_lk_file};
use super::schema::{Banner, BoardContent, Document, LkRoot, Presentation, Property, Resource};
use super::LkError;
use crate::prosemirror::from_markdown::from_markdown;

/// In-memory world being assembled for export.
pub struct WorldBuilder {
    name: String,
    root: LkRoot,
}

#[derive(Debug, Serialize)]
pub struct DraftResourceSummary {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub tags: Vec<String>,
    pub document_count: usize,
}

#[derive(Debug, Serialize)]
pub struct DraftSummary {
    pub name: String,
    pub resource_count: usize,
    pub resources: Vec<DraftResourceSummary>,
}

#[derive(Debug, Serialize)]
pub struct DraftDocumentSummary {
    pub id: String,
    pub name: String,
    pub doc_type: String,
}

impl WorldBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            root: LkRoot {
                version: 1,
                export_id: generate_id(),
                exported_at: String::new(),
                resources: Vec::new(),
                calendars: Vec::new(),
                resource_count: 0,
                hash: String::new(),
            },
        }
    }

    /// Create a builder from an existing LkRoot. Preserves all IDs and content.
    pub fn from_lk_root(name: String, root: LkRoot) -> Self {
        Self { name, root }
    }

    pub fn create_resource(
        &mut self,
        name: &str,
        parent_id: Option<&str>,
        tags: Option<Vec<String>>,
        content: Option<&str>,
        is_hidden: bool,
        aliases: Vec<String>,
        properties: Vec<Property>,
        icon_color: Option<String>,
        icon_glyph: Option<String>,
        icon_shape: Option<String>,
    ) -> Result<DraftResourceSummary, LkError> {
        // Validate parent exists if specified
        if let Some(pid) = parent_id {
            if !self.root.resources.iter().any(|r| r.id == pid) {
                return Err(LkError::DraftResourceNotFound(pid.to_string()));
            }
        }

        let resource_id = generate_id();
        let doc_id = generate_id();
        let now = Utc::now().to_rfc3339();

        let pm_content = Some(match content {
            Some(md) => from_markdown(md, &self.root.resources),
            None => serde_json::json!({"type": "doc", "content": []}),
        });

        let doc = Document {
            id: doc_id,
            name: "Main".to_string(),
            doc_type: "page".to_string(),
            locator_id: format!("lk://resources/{}/docs/main", resource_id),
            pos: "a0".to_string(),
            is_hidden: false,
            is_first: true,
            is_full_width: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            transforms: Vec::new(),
            sources: Vec::new(),
            presentation: Some(Presentation {
                document_type: "page".to_string(),
                calibration: None,
                default_mode: None,
                disallowed_modes: None,
            }),
            content: pm_content,
            map: None,
            calendar_id: None,
        };

        let resource = Resource {
            schema_version: 1,
            id: resource_id.clone(),
            name: name.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            pos: format!("a{}", self.root.resources.len()),
            created_by: "mcp-generator".to_string(),
            is_hidden,
            is_locked: false,
            show_property_bar: true,
            icon_color,
            icon_glyph,
            icon_shape,
            aliases,
            tags: tags.clone().unwrap_or_default(),
            documents: vec![doc],
            properties,
            banner: Banner {
                enabled: false,
                url: String::new(),
                y_position: 50,
            },
        };

        self.root.resources.push(resource);
        self.root.resource_count = self.root.resources.len();

        Ok(DraftResourceSummary {
            id: resource_id,
            name: name.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            tags: tags.unwrap_or_default(),
            document_count: 1,
        })
    }

    pub fn add_document(
        &mut self,
        resource_id: &str,
        name: &str,
        content: &str,
        doc_type: Option<&str>,
        is_hidden: bool,
    ) -> Result<DraftDocumentSummary, LkError> {
        let res_idx = self
            .root
            .resources
            .iter()
            .position(|r| r.id == resource_id)
            .ok_or_else(|| LkError::DraftResourceNotFound(resource_id.to_string()))?;

        let doc_id = generate_id();
        let now = Utc::now().to_rfc3339();
        let dtype = doc_type.unwrap_or("page");

        // Convert markdown before taking mutable borrow
        let pm_content = if dtype == "page" {
            Some(from_markdown(content, &self.root.resources))
        } else if dtype == "board" {
            let value: serde_json::Value = serde_json::from_str(content)
                .map_err(|e| LkError::InvalidInput(format!("invalid board JSON: {}", e)))?;
            let board: BoardContent = serde_json::from_value(value.clone())
                .map_err(|e| LkError::InvalidInput(format!("invalid board structure: {}", e)))?;
            validate_board_content(&board)?;
            Some(value)
        } else {
            serde_json::from_str(content).ok()
        };

        let resource = &mut self.root.resources[res_idx];
        let doc = Document {
            id: doc_id.clone(),
            name: name.to_string(),
            doc_type: dtype.to_string(),
            locator_id: format!("lk://resources/{}/docs/{}", resource_id, doc_id),
            pos: format!("a{}", resource.documents.len()),
            is_hidden,
            is_first: false,
            is_full_width: None,
            created_at: now.clone(),
            updated_at: now,
            transforms: Vec::new(),
            sources: Vec::new(),
            presentation: Some(Presentation {
                document_type: dtype.to_string(),
                calibration: None,
                default_mode: None,
                disallowed_modes: None,
            }),
            content: pm_content,
            map: None,
            calendar_id: None,
        };

        resource.documents.push(doc);

        Ok(DraftDocumentSummary {
            id: doc_id,
            name: name.to_string(),
            doc_type: dtype.to_string(),
        })
    }

    pub fn set_content(
        &mut self,
        resource_id: &str,
        document_id: Option<&str>,
        content: &str,
    ) -> Result<(), LkError> {
        // Convert markdown before taking mutable borrow
        let pm_content = from_markdown(content, &self.root.resources);

        let resource = self
            .root
            .resources
            .iter_mut()
            .find(|r| r.id == resource_id)
            .ok_or_else(|| LkError::DraftResourceNotFound(resource_id.to_string()))?;

        let doc = if let Some(did) = document_id {
            resource
                .documents
                .iter_mut()
                .find(|d| d.id == did)
                .ok_or_else(|| LkError::DraftDocumentNotFound(did.to_string()))?
        } else {
            resource
                .documents
                .iter_mut()
                .find(|d| d.doc_type == "page")
                .ok_or_else(|| {
                    LkError::DraftDocumentNotFound("no page document found".to_string())
                })?
        };

        doc.content = Some(pm_content);
        doc.updated_at = Utc::now().to_rfc3339();

        Ok(())
    }

    pub fn delete_resource(
        &mut self,
        resource_id: &str,
        recursive: bool,
    ) -> Result<Vec<String>, LkError> {
        // Check resource exists
        if !self.root.resources.iter().any(|r| r.id == resource_id) {
            return Err(LkError::DraftResourceNotFound(resource_id.to_string()));
        }

        // Collect IDs to delete
        let mut to_delete = HashSet::new();
        to_delete.insert(resource_id.to_string());

        if recursive {
            // Find all descendants via stack-based traversal
            let mut stack = vec![resource_id.to_string()];
            while let Some(parent) = stack.pop() {
                for r in &self.root.resources {
                    if r.parent_id.as_deref() == Some(&parent) && !to_delete.contains(&r.id) {
                        to_delete.insert(r.id.clone());
                        stack.push(r.id.clone());
                    }
                }
            }
        } else {
            // Check for children — refuse if any exist
            let has_children = self
                .root
                .resources
                .iter()
                .any(|r| r.parent_id.as_deref() == Some(resource_id));
            if has_children {
                return Err(LkError::InvalidInput(format!(
                    "Resource {} has children. Use recursive=true to delete them, or reparent them first.",
                    resource_id
                )));
            }
        }

        self.root
            .resources
            .retain(|r| !to_delete.contains(&r.id));
        self.root.resource_count = self.root.resources.len();

        let mut deleted: Vec<String> = to_delete.into_iter().collect();
        deleted.sort();
        Ok(deleted)
    }

    pub fn reparent_resource(
        &mut self,
        resource_id: &str,
        new_parent_id: Option<&str>,
    ) -> Result<(), LkError> {
        // Check resource exists
        if !self.root.resources.iter().any(|r| r.id == resource_id) {
            return Err(LkError::DraftResourceNotFound(resource_id.to_string()));
        }

        // Validate new parent exists (if not moving to top-level)
        if let Some(pid) = new_parent_id {
            if pid == resource_id {
                return Err(LkError::InvalidInput(
                    "Cannot parent a resource under itself".to_string(),
                ));
            }
            if !self.root.resources.iter().any(|r| r.id == pid) {
                return Err(LkError::DraftResourceNotFound(pid.to_string()));
            }
            // Check for circular parenting: walk new_parent's ancestor chain
            let max_depth = self.root.resources.len();
            let mut ancestor = Some(pid.to_string());
            let mut depth = 0;
            while let Some(ref aid) = ancestor {
                if aid == resource_id {
                    return Err(LkError::InvalidInput(
                        "Cannot reparent: would create a circular reference".to_string(),
                    ));
                }
                depth += 1;
                if depth > max_depth {
                    return Err(LkError::InvalidInput(
                        "Cycle detected in resource tree".to_string(),
                    ));
                }
                ancestor = self
                    .root
                    .resources
                    .iter()
                    .find(|r| r.id == *aid)
                    .and_then(|r| r.parent_id.clone());
            }
        }

        // Apply the reparent
        let resource = self
            .root
            .resources
            .iter_mut()
            .find(|r| r.id == resource_id)
            .expect("resource existence validated above");
        resource.parent_id = new_parent_id.map(|s| s.to_string());

        Ok(())
    }

    /// Look up a draft resource by ID (exact) or name (case-insensitive).
    /// Tries ID first, falls back to name — same lookup pattern as WorldStore.
    pub fn get_draft_resource(&self, id_or_name: &str) -> Result<&Resource, LkError> {
        // Try exact ID match
        if let Some(r) = self.root.resources.iter().find(|r| r.id == id_or_name) {
            return Ok(r);
        }
        // Fallback: case-insensitive name match
        let lower = id_or_name.to_lowercase();
        self.root
            .resources
            .iter()
            .find(|r| r.name.to_lowercase() == lower)
            .ok_or_else(|| LkError::DraftResourceNotFound(id_or_name.to_string()))
    }

    /// Get a specific document from a draft resource.
    /// If document_id is None, returns the first page-type document.
    pub fn get_draft_document(
        &self,
        resource_id: &str,
        document_id: Option<&str>,
    ) -> Result<&Document, LkError> {
        let resource = self
            .root
            .resources
            .iter()
            .find(|r| r.id == resource_id)
            .ok_or_else(|| LkError::DraftResourceNotFound(resource_id.to_string()))?;

        match document_id {
            Some(did) => resource
                .documents
                .iter()
                .find(|d| d.id == did)
                .ok_or_else(|| LkError::DraftDocumentNotFound(did.to_string())),
            None => resource
                .documents
                .iter()
                .find(|d| d.doc_type == "page")
                .ok_or_else(|| {
                    LkError::DraftDocumentNotFound("no page document found".to_string())
                }),
        }
    }

    /// Update non-None metadata fields on a draft resource.
    pub fn update_resource(
        &mut self,
        resource_id: &str,
        name: Option<&str>,
        tags: Option<Vec<String>>,
        is_hidden: Option<bool>,
        aliases: Option<Vec<String>>,
    ) -> Result<DraftResourceSummary, LkError> {
        let resource = self
            .root
            .resources
            .iter_mut()
            .find(|r| r.id == resource_id)
            .ok_or_else(|| LkError::DraftResourceNotFound(resource_id.to_string()))?;

        if let Some(n) = name {
            resource.name = n.to_string();
        }
        if let Some(t) = tags {
            resource.tags = t;
        }
        if let Some(h) = is_hidden {
            resource.is_hidden = h;
        }
        if let Some(a) = aliases {
            resource.aliases = a;
        }

        Ok(DraftResourceSummary {
            id: resource.id.clone(),
            name: resource.name.clone(),
            parent_id: resource.parent_id.clone(),
            tags: resource.tags.clone(),
            document_count: resource.documents.len(),
        })
    }

    /// Delete a document from a draft resource.
    /// Fails if it would leave the resource with zero documents.
    pub fn delete_document(
        &mut self,
        resource_id: &str,
        document_id: &str,
    ) -> Result<String, LkError> {
        let resource = self
            .root
            .resources
            .iter_mut()
            .find(|r| r.id == resource_id)
            .ok_or_else(|| LkError::DraftResourceNotFound(resource_id.to_string()))?;

        if resource.documents.len() <= 1 {
            return Err(LkError::InvalidInput(
                "Cannot delete the last document on a resource".to_string(),
            ));
        }

        let idx = resource
            .documents
            .iter()
            .position(|d| d.id == document_id)
            .ok_or_else(|| LkError::DraftDocumentNotFound(document_id.to_string()))?;

        let doc_name = resource.documents[idx].name.clone();
        resource.documents.remove(idx);
        Ok(doc_name)
    }

    pub fn list_draft(&self) -> DraftSummary {
        DraftSummary {
            name: self.name.clone(),
            resource_count: self.root.resources.len(),
            resources: self
                .root
                .resources
                .iter()
                .map(|r| DraftResourceSummary {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    parent_id: r.parent_id.clone(),
                    tags: r.tags.clone(),
                    document_count: r.documents.len(),
                })
                .collect(),
        }
    }

    pub fn export_world(&mut self, output_path: Option<&str>) -> Result<PathBuf, LkError> {
        // Finalize the root
        self.root.exported_at = Utc::now().to_rfc3339();
        self.root.resource_count = self.root.resources.len();
        self.root.hash = compute_hash(&self.root);

        // Determine output path
        let path = if let Some(p) = output_path {
            PathBuf::from(p)
        } else {
            let dir = default_export_dir()?;
            dir.join(format!("{}.lk", self.name))
        };

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        write_lk_file(&path, &self.root)?;
        Ok(path)
    }
}

/// Validate board content records have required fields for LegendKeeper import.
fn validate_board_content(board: &BoardContent) -> Result<(), LkError> {
    for (i, record) in board.shapes_v2.iter().enumerate() {
        let val = &record.val;
        let ctx = format!("shapesV2[{}] (key={})", i, record.key);

        // Every record must have id, typeName, and meta
        if val.get("id").and_then(|v| v.as_str()).is_none() {
            return Err(LkError::InvalidInput(format!(
                "{}: missing required field 'id'",
                ctx
            )));
        }
        if val.get("meta").is_none() {
            return Err(LkError::InvalidInput(format!(
                "{}: missing required field 'meta'",
                ctx
            )));
        }
        let type_name = val.get("typeName").and_then(|v| v.as_str()).ok_or_else(|| {
            LkError::InvalidInput(format!("{}: missing required field 'typeName'", ctx))
        })?;

        match type_name {
            "binding" => {
                for field in &["type", "fromId", "toId", "props"] {
                    if val.get(*field).is_none() {
                        return Err(LkError::InvalidInput(format!(
                            "{}: binding missing required field '{}'",
                            ctx, field
                        )));
                    }
                }
            }
            "shape" => {
                for field in &["type", "props", "parentId", "index"] {
                    if val.get(*field).is_none() {
                        return Err(LkError::InvalidInput(format!(
                            "{}: shape missing required field '{}'",
                            ctx, field
                        )));
                    }
                }
                // All tldraw shapes require scale in props
                if let Some(props) = val.get("props") {
                    if props.get("scale").is_none() {
                        return Err(LkError::InvalidInput(format!(
                            "{}: shape props missing required field 'scale'",
                            ctx
                        )));
                    }
                }
            }
            "document" | "page" => {} // minimal fields, id+typeName+meta suffice
            other => {
                return Err(LkError::InvalidInput(format!(
                    "{}: unknown typeName '{}'",
                    ctx, other
                )));
            }
        }
    }
    Ok(())
}

/// Public wrapper for tests — validates board content records.
#[cfg(test)]
pub fn validate_board_content_pub(board: &BoardContent) -> Result<(), LkError> {
    validate_board_content(board)
}

fn default_export_dir() -> Result<PathBuf, LkError> {
    let home = std::env::var("HOME")
        .map_err(|_| LkError::InvalidInput("HOME environment variable not set".to_string()))?;
    Ok(PathBuf::from(home).join(".lk-worlds").join("exports"))
}
