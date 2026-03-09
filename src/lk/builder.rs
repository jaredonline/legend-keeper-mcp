use std::path::PathBuf;

use chrono::Utc;
use serde::Serialize;

use super::io::{compute_hash, generate_id, write_lk_file};
use super::schema::{Banner, Document, LkRoot, Presentation, Property, Resource};
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

    pub fn create_resource(
        &mut self,
        name: &str,
        parent_id: Option<&str>,
        tags: Option<Vec<String>>,
        content: Option<&str>,
        is_hidden: bool,
        aliases: Vec<String>,
        properties: Vec<Property>,
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
            icon_color: None,
            icon_glyph: None,
            icon_shape: None,
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

fn default_export_dir() -> Result<PathBuf, LkError> {
    let home = std::env::var("HOME")
        .map_err(|_| LkError::InvalidInput("HOME environment variable not set".to_string()))?;
    Ok(PathBuf::from(home).join(".lk-worlds").join("exports"))
}
