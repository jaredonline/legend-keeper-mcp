use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::Value;

use super::io::read_lk_file;
use super::schema::{BoardContent, Calendar, LkRoot, Property, Resource, TimelineContent};
use super::LkError;
use crate::prosemirror::to_markdown::to_markdown;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TemplateSummary {
    pub name: String,
    pub tags: Vec<String>,
    pub properties: Vec<TemplatePropertySummary>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TemplatePropertySummary {
    pub prop_type: String,
    pub title: String,
}

/// Icon fields extracted from a template resource.
#[derive(Clone)]
pub struct TemplateIcon {
    pub icon_color: Option<String>,
    pub icon_glyph: Option<String>,
    pub icon_shape: Option<String>,
}

#[derive(Clone)]
pub struct WorldStore {
    worlds: Arc<RwLock<HashMap<String, LkRoot>>>,
    dir: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WorldSummary {
    pub name: String,
    pub resource_count: usize,
    pub calendar_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guide: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceSummary {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    pub is_hidden: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceTree {
    pub id: String,
    pub name: String,
    pub children: Vec<ResourceTree>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub resource_id: String,
    pub resource_name: String,
    pub document_name: String,
    pub snippet: String,
    pub is_hidden: bool,
}

impl WorldStore {
    pub fn load(dir: &Path) -> Result<Self, LkError> {
        let worlds = Arc::new(RwLock::new(HashMap::new()));
        let store = WorldStore {
            worlds,
            dir: dir.to_path_buf(),
        };
        store.scan_directory()?;
        Ok(store)
    }

    fn scan_directory(&self) -> Result<(), LkError> {
        let mut worlds = self.worlds.write().unwrap();
        worlds.clear();

        if !self.dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("lk") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                match read_lk_file(&path) {
                    Ok(root) => {
                        eprintln!("  Loaded {} ({} resources, {} calendars)", name, root.resources.len(), root.calendars.len());
                        worlds.insert(name, root);
                    }
                    Err(e) => {
                        eprintln!("  Failed to load {}: {}", name, e);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn start_watcher(&self) -> Result<RecommendedWatcher, LkError> {
        let worlds = self.worlds.clone();
        let dir = self.dir.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            let event = match res {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("File watcher error: {}", e);
                    return;
                }
            };

            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
                _ => return,
            }

            for path in &event.paths {
                if path.extension().and_then(|e| e.to_str()) != Some("lk") {
                    continue;
                }

                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let mut worlds = worlds.write().unwrap();

                if matches!(event.kind, EventKind::Remove(_)) || !path.exists() {
                    if worlds.remove(&name).is_some() {
                        eprintln!("Hot-reload: removed world '{}'", name);
                    }
                } else {
                    match read_lk_file(path) {
                        Ok(root) => {
                            eprintln!("Hot-reload: loaded world '{}' ({} resources)", name, root.resources.len());
                            worlds.insert(name, root);
                        }
                        Err(e) => {
                            eprintln!("Hot-reload: failed to load '{}': {}", name, e);
                        }
                    }
                }
            }
        })
        .map_err(|e| LkError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        watcher
            .watch(&dir, RecursiveMode::NonRecursive)
            .map_err(|e| LkError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(watcher)
    }

    fn resolve_world<'a>(
        worlds: &'a HashMap<String, LkRoot>,
        world: &'a Option<String>,
    ) -> Result<(&'a str, &'a LkRoot), LkError> {
        match world {
            Some(name) => worlds
                .get(name.as_str())
                .map(|r| (name.as_str(), r))
                .ok_or_else(|| LkError::WorldNotFound(name.clone())),
            None => {
                if worlds.len() == 1 {
                    let (name, root) = worlds.iter().next().unwrap();
                    Ok((name.as_str(), root))
                } else if worlds.is_empty() {
                    Err(LkError::WorldNotFound("no worlds loaded".to_string()))
                } else {
                    Err(LkError::InvalidInput(
                        "multiple worlds loaded; specify 'world' parameter".to_string(),
                    ))
                }
            }
        }
    }

    /// Clone a world's LkRoot for use in the builder.
    pub fn get_world(&self, name: &str) -> Result<LkRoot, LkError> {
        let worlds = self.worlds.read().unwrap();
        worlds
            .get(name)
            .cloned()
            .ok_or_else(|| LkError::WorldNotFound(name.to_string()))
    }

    pub fn list_worlds(&self) -> Vec<WorldSummary> {
        let worlds = self.worlds.read().unwrap();
        let mut result: Vec<_> = worlds
            .iter()
            .map(|(name, root)| WorldSummary {
                name: name.clone(),
                resource_count: root.resources.len(),
                calendar_count: root.calendars.len(),
                guide: Self::extract_world_guide(root),
            })
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    /// Find a resource tagged `llm-guide` and return its first page document as markdown.
    fn extract_world_guide(root: &LkRoot) -> Option<String> {
        let resource = root.resources.iter().find(|r| {
            r.tags.iter().any(|t| t.eq_ignore_ascii_case("llm-guide"))
        })?;
        let page_doc = resource.documents.iter().find(|d| d.doc_type == "page")?;
        let content = page_doc.content.as_ref()?;
        let md = to_markdown(content);
        if md.trim().is_empty() {
            None
        } else {
            Some(md)
        }
    }

    pub fn list_resources(
        &self,
        world: &Option<String>,
        tag: &Option<String>,
        name: &Option<String>,
    ) -> Result<Vec<ResourceSummary>, LkError> {
        let worlds = self.worlds.read().unwrap();
        let (_, root) = Self::resolve_world(&worlds, world)?;

        let mut results: Vec<ResourceSummary> = root
            .resources
            .iter()
            .filter(|r| {
                if let Some(tag) = tag {
                    if !r.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                        return false;
                    }
                }
                if let Some(name) = name {
                    let name_lower = name.to_lowercase();
                    if !r.name.to_lowercase().contains(&name_lower) {
                        return false;
                    }
                }
                true
            })
            .map(|r| ResourceSummary {
                id: r.id.clone(),
                name: r.name.clone(),
                tags: r.tags.clone(),
                parent_id: r.parent_id.clone(),
                is_hidden: r.is_hidden,
            })
            .collect();

        results.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(results)
    }

    pub fn get_resource(
        &self,
        world: &Option<String>,
        id_or_name: &str,
    ) -> Result<Resource, LkError> {
        let worlds = self.worlds.read().unwrap();
        let (_, root) = Self::resolve_world(&worlds, world)?;

        // Try ID first
        if let Some(r) = root.resources.iter().find(|r| r.id == id_or_name) {
            return Ok(r.clone());
        }

        // Fallback: case-insensitive name match
        let lower = id_or_name.to_lowercase();
        root.resources
            .iter()
            .find(|r| r.name.to_lowercase() == lower)
            .cloned()
            .ok_or_else(|| LkError::ResourceNotFound(id_or_name.to_string()))
    }

    pub fn get_resource_tree(
        &self,
        world: &Option<String>,
        root_id: &Option<String>,
    ) -> Result<Vec<ResourceTree>, LkError> {
        let worlds = self.worlds.read().unwrap();
        let (_, root) = Self::resolve_world(&worlds, world)?;

        // Build parent -> children map
        let mut children_map: HashMap<Option<String>, Vec<&Resource>> = HashMap::new();
        for r in &root.resources {
            children_map
                .entry(r.parent_id.clone())
                .or_default()
                .push(r);
        }

        // Sort children by pos
        for children in children_map.values_mut() {
            children.sort_by(|a, b| a.pos.cmp(&b.pos));
        }

        fn build_tree(
            id: &Option<String>,
            children_map: &HashMap<Option<String>, Vec<&Resource>>,
        ) -> Vec<ResourceTree> {
            let Some(children) = children_map.get(id) else {
                return vec![];
            };
            children
                .iter()
                .map(|r| ResourceTree {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    children: build_tree(&Some(r.id.clone()), children_map),
                })
                .collect()
        }

        match root_id {
            Some(id) => {
                // Verify the root resource exists
                if !root.resources.iter().any(|r| r.id == *id) {
                    return Err(LkError::ResourceNotFound(id.clone()));
                }
                Ok(build_tree(&Some(id.clone()), &children_map))
            }
            None => Ok(build_tree(&None, &children_map)),
        }
    }

    pub fn search_content(
        &self,
        world: &Option<String>,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SearchResult>, LkError> {
        if query.is_empty() {
            return Err(LkError::InvalidInput("query cannot be empty".to_string()));
        }

        let worlds = self.worlds.read().unwrap();
        let (_, root) = Self::resolve_world(&worlds, world)?;
        let query_lower = query.to_lowercase();
        let limit = limit.unwrap_or(20);
        let mut results = Vec::new();

        for resource in &root.resources {
            if results.len() >= limit {
                break;
            }

            for doc in &resource.documents {
                if results.len() >= limit {
                    break;
                }

                match doc.doc_type.as_str() {
                    "page" => {
                        if let Some(content) = &doc.content {
                            let text = extract_text_from_prosemirror(content);
                            if let Some(snippet) = find_snippet(&text, &query_lower) {
                                results.push(SearchResult {
                                    resource_id: resource.id.clone(),
                                    resource_name: resource.name.clone(),
                                    document_name: doc.name.clone(),
                                    snippet,
                                    is_hidden: doc.is_hidden,
                                });
                            }
                        }
                    }
                    "time" => {
                        if let Some(content) = &doc.content {
                            if let Ok(timeline) =
                                serde_json::from_value::<TimelineContent>(content.clone())
                            {
                                for event in &timeline.events {
                                    if results.len() >= limit {
                                        break;
                                    }
                                    if event.name.to_lowercase().contains(&query_lower) {
                                        results.push(SearchResult {
                                            resource_id: resource.id.clone(),
                                            resource_name: resource.name.clone(),
                                            document_name: doc.name.clone(),
                                            snippet: format!("Timeline event: {}", event.name),
                                            is_hidden: doc.is_hidden,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    "board" => {
                        if let Some(content) = &doc.content {
                            if let Ok(board) =
                                serde_json::from_value::<BoardContent>(content.clone())
                            {
                                for record in &board.shapes_v2 {
                                    if results.len() >= limit {
                                        break;
                                    }
                                    let text = extract_text_from_board_record(record);
                                    if !text.is_empty()
                                        && text.to_lowercase().contains(&query_lower)
                                    {
                                        results.push(SearchResult {
                                            resource_id: resource.id.clone(),
                                            resource_name: resource.name.clone(),
                                            document_name: doc.name.clone(),
                                            snippet: format!("Board shape: {}", text),
                                            is_hidden: doc.is_hidden,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(results)
    }

    pub fn get_calendar(
        &self,
        world: &Option<String>,
        id_or_name: &str,
    ) -> Result<Calendar, LkError> {
        let worlds = self.worlds.read().unwrap();
        let (_, root) = Self::resolve_world(&worlds, world)?;

        // Try ID first
        if let Some(c) = root.calendars.iter().find(|c| c.id == id_or_name) {
            return Ok(c.clone());
        }

        // Fallback: case-insensitive name match
        let lower = id_or_name.to_lowercase();
        root.calendars
            .iter()
            .find(|c| c.name.to_lowercase() == lower)
            .cloned()
            .ok_or_else(|| LkError::CalendarNotFound(id_or_name.to_string()))
    }

    /// Extract templates from a world. Templates are resources under the "templates" parent chain.
    pub fn list_templates(
        &self,
        world: &Option<String>,
    ) -> Result<Vec<TemplateSummary>, LkError> {
        let worlds = self.worlds.read().unwrap();
        let (_, root) = Self::resolve_world(&worlds, world)?;
        let templates = Self::extract_templates(root);
        Ok(templates
            .iter()
            .map(|r| TemplateSummary {
                name: r.name.clone(),
                tags: r.tags.clone(),
                properties: r
                    .properties
                    .iter()
                    .map(|p| TemplatePropertySummary {
                        prop_type: p.prop_type.clone(),
                        title: p.title.clone(),
                    })
                    .collect(),
            })
            .collect())
    }

    /// Get the full property list for a template by name, with fresh IDs generated for each property.
    /// Also returns the template's tags and icon fields.
    pub fn get_template_properties(
        &self,
        world: &Option<String>,
        template_name: &str,
    ) -> Result<(Vec<Property>, Vec<String>, TemplateIcon), LkError> {
        let worlds = self.worlds.read().unwrap();
        let (_, root) = Self::resolve_world(&worlds, world)?;
        let templates = Self::extract_templates(root);
        let lower = template_name.to_lowercase();
        let template = templates
            .iter()
            .find(|r| r.name.to_lowercase() == lower)
            .ok_or_else(|| {
                LkError::InvalidInput(format!("Template '{}' not found", template_name))
            })?;

        use crate::lk::io::generate_id;

        let properties: Vec<Property> = template
            .properties
            .iter()
            .map(|p| Property {
                id: generate_id(),
                pos: p.pos.clone(),
                prop_type: p.prop_type.clone(),
                title: p.title.clone(),
                is_hidden: p.is_hidden,
                is_title_hidden: p.is_title_hidden,
                data: p.data.clone(),
            })
            .collect();

        let icon = TemplateIcon {
            icon_color: template.icon_color.clone(),
            icon_glyph: template.icon_glyph.clone(),
            icon_shape: template.icon_shape.clone(),
        };

        Ok((properties, template.tags.clone(), icon))
    }

    /// Find all template resources by walking the parentId chain looking for id == "templates".
    fn extract_templates(root: &LkRoot) -> Vec<&Resource> {
        // Build a set of resource IDs that are in the templates hierarchy.
        // The "templates" parent ID is a special sentinel — resources whose parentId chain
        // includes a resource with id "templates" are part of the template system.
        // In practice, the "Default Templates" resource has parentId: "templates",
        // and individual templates are children of that resource.

        // Find the "Default Templates" resource (direct child of "templates" parent)
        let template_folder_ids: Vec<&str> = root
            .resources
            .iter()
            .filter(|r| r.parent_id.as_deref() == Some("templates"))
            .map(|r| r.id.as_str())
            .collect();

        // Find all children of those template folders — these are the actual templates
        root.resources
            .iter()
            .filter(|r| {
                if let Some(pid) = &r.parent_id {
                    template_folder_ids.contains(&pid.as_str())
                } else {
                    false
                }
            })
            .collect()
    }

    pub async fn fetch_image(url: &str) -> Result<(Vec<u8>, String), LkError> {
        let resp = reqwest::get(url)
            .await
            .map_err(|e| LkError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(LkError::Http(format!("HTTP {}", resp.status())));
        }

        let mime = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| guess_mime_from_url(url));

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| LkError::Http(e.to_string()))?;

        Ok((bytes.to_vec(), mime))
    }
}

fn guess_mime_from_url(url: &str) -> String {
    let lower = url.to_lowercase();
    if lower.ends_with(".png") {
        "image/png".to_string()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if lower.ends_with(".webp") {
        "image/webp".to_string()
    } else {
        "image/png".to_string()
    }
}

/// Recursively extract plain text from a ProseMirror JSON node.
fn extract_text_from_prosemirror(node: &Value) -> String {
    let mut text = String::new();
    extract_text_recursive(node, &mut text);
    text
}

fn extract_text_recursive(node: &Value, out: &mut String) {
    if let Some(t) = node.get("text").and_then(|v| v.as_str()) {
        out.push_str(t);
    }
    if let Some(content) = node.get("content").and_then(|v| v.as_array()) {
        for child in content {
            extract_text_recursive(child, out);
            // Add space between block nodes
            if let Some(node_type) = child.get("type").and_then(|v| v.as_str()) {
                if matches!(
                    node_type,
                    "paragraph" | "heading" | "listItem" | "blockquote" | "codeBlock"
                ) {
                    out.push(' ');
                }
            }
        }
    }
}

/// Extract searchable text from a board record (shape text, arrow labels, etc.).
fn extract_text_from_board_record(record: &super::schema::BoardRecord) -> String {
    let val = &record.val;
    // Only shapes have user-visible text
    if val.get("typeName").and_then(|v| v.as_str()) != Some("shape") {
        return String::new();
    }
    // Text lives in props.text for geo, text, and arrow shapes
    val.get("props")
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string()
}

/// Find a snippet around the first match of query in text.
fn find_snippet(text: &str, query_lower: &str) -> Option<String> {
    let text_lower = text.to_lowercase();
    let pos = text_lower.find(query_lower)?;

    // Map byte offset from lowercased text back to original text by walking
    // both strings character-by-character.
    let mut orig_pos = 0;
    let mut lower_offset = 0;
    for ch in text.chars() {
        if lower_offset >= pos {
            break;
        }
        let lower_ch_len: usize = ch.to_lowercase().map(|c| c.len_utf8()).sum();
        lower_offset += lower_ch_len;
        orig_pos += ch.len_utf8();
    }

    // Walk backwards from orig_pos to find start (up to 50 chars back)
    let mut start = orig_pos;
    let mut chars_back = 0;
    for (i, _) in text[..orig_pos].char_indices().rev() {
        start = i;
        chars_back += 1;
        if chars_back >= 50 {
            break;
        }
    }

    // Walk forwards from orig_pos to find end (query length + 50 chars forward)
    let mut end = orig_pos;
    let mut chars_fwd = 0;
    let target_fwd = query_lower.chars().count() + 50;
    for (i, ch) in text[orig_pos..].char_indices() {
        end = orig_pos + i + ch.len_utf8();
        chars_fwd += 1;
        if chars_fwd >= target_fwd {
            break;
        }
    }

    let mut snippet = String::new();
    if start > 0 {
        snippet.push_str("...");
    }
    snippet.push_str(&text[start..end]);
    if end < text.len() {
        snippet.push_str("...");
    }
    Some(snippet)
}
