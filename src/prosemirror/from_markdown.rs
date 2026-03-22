use comrak::nodes::{AstNode, ListType, NodeCode, NodeCodeBlock, NodeHeading, NodeLink, NodeValue};
use comrak::{parse_document, Arena, ExtensionOptions, Options};
use serde_json::{json, Value};

use crate::lk::schema::Resource;

/// Convert markdown text to ProseMirror JSON.
/// Resources are used to resolve `[[Name]]` mentions to resource IDs.
pub fn from_markdown(md: &str, resources: &[Resource]) -> Value {
    // Extract headerless tables (key-value style) and replace with placeholders.
    // These get converted to header-column tables (first column = tableHeader).
    let (md, extracted_tables) = extract_headerless_tables(md);

    let arena = Arena::new();
    let extension = ExtensionOptions::builder()
        .table(true)
        .strikethrough(true)
        .tasklist(true)
        .autolink(true)
        .build();
    let options = Options {
        extension,
        ..Options::default()
    };
    let root = parse_document(&arena, &md, &options);

    let children = convert_children(root, resources);

    // Replace placeholder paragraphs with the extracted header-column tables
    let children = replace_table_placeholders(children, &extracted_tables, resources);

    json!({
        "type": "doc",
        "content": children
    })
}

fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Must contain at least one pipe and look like a table row
    // Accept "| a | b |" or "a | b" styles
    trimmed.contains('|') && !is_separator_row(trimmed)
}

fn is_separator_row(line: &str) -> bool {
    let trimmed = line.trim().trim_matches('|').trim();
    if trimmed.is_empty() {
        return false;
    }
    // A separator row contains only dashes, pipes, spaces, and colons (for alignment)
    trimmed
        .chars()
        .all(|c| c == '-' || c == '|' || c == ' ' || c == ':')
        && trimmed.contains('-')
}

fn convert_children<'a>(node: &'a AstNode<'a>, resources: &[Resource]) -> Vec<Value> {
    let mut result = Vec::new();
    let mut child = node.first_child();
    while let Some(c) = child {
        if let Some(pm_node) = convert_node(c, resources) {
            result.push(pm_node);
        }
        child = c.next_sibling();
    }
    result
}

fn convert_node<'a>(node: &'a AstNode<'a>, resources: &[Resource]) -> Option<Value> {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Paragraph => {
            let inlines = convert_inline_children(node, resources);
            if inlines.is_empty() {
                Some(json!({ "type": "paragraph" }))
            } else {
                Some(json!({
                    "type": "paragraph",
                    "content": inlines
                }))
            }
        }
        NodeValue::Heading(NodeHeading { level, .. }) => {
            let inlines = convert_inline_children(node, resources);
            let mut h = json!({
                "type": "heading",
                "attrs": { "level": *level }
            });
            if !inlines.is_empty() {
                h["content"] = json!(inlines);
            }
            Some(h)
        }
        NodeValue::Text(text) => {
            // Handle [[mentions]] in text
            Some(json!(convert_text_with_mentions(text, resources)))
        }
        NodeValue::SoftBreak => Some(json!({ "type": "hardBreak" })),
        NodeValue::LineBreak => Some(json!({ "type": "hardBreak" })),
        NodeValue::Code(NodeCode { literal, .. }) => Some(json!({
            "type": "text",
            "text": literal,
            "marks": [{ "type": "code" }]
        })),
        NodeValue::Emph => {
            let inlines = convert_inline_children(node, resources);
            Some(json!(add_mark_to_nodes(&inlines, json!({ "type": "em" }))))
        }
        NodeValue::Strong => {
            let inlines = convert_inline_children(node, resources);
            Some(json!(add_mark_to_nodes(
                &inlines,
                json!({ "type": "strong" })
            )))
        }
        NodeValue::Strikethrough => {
            let inlines = convert_inline_children(node, resources);
            Some(json!(add_mark_to_nodes(
                &inlines,
                json!({ "type": "strike" })
            )))
        }
        NodeValue::Link(NodeLink { url, .. }) => {
            let inlines = convert_inline_children(node, resources);
            let mark = json!({
                "type": "link",
                "attrs": { "href": url }
            });
            Some(json!(add_mark_to_nodes(&inlines, mark)))
        }
        NodeValue::Image(link) => Some(json!({
            "type": "mediaSingle",
            "content": [{
                "type": "media",
                "attrs": {
                    "url": link.url,
                    "type": "external"
                }
            }]
        })),
        NodeValue::List(list) => {
            // Check if this list contains task items
            let has_task_items = {
                let mut child = node.first_child();
                let mut found = false;
                while let Some(c) = child {
                    if matches!(c.data.borrow().value, NodeValue::TaskItem(..)) {
                        found = true;
                        break;
                    }
                    child = c.next_sibling();
                }
                found
            };

            let list_type = if has_task_items {
                "taskList"
            } else if list.list_type == ListType::Ordered {
                "orderedList"
            } else {
                "bulletList"
            };
            let items = convert_children(node, resources);
            if items.is_empty() {
                Some(json!({ "type": list_type }))
            } else {
                Some(json!({
                    "type": list_type,
                    "content": items
                }))
            }
        }
        NodeValue::Item(_) => {
            let children = convert_children(node, resources);
            if children.is_empty() {
                Some(json!({ "type": "listItem" }))
            } else {
                Some(json!({
                    "type": "listItem",
                    "content": children
                }))
            }
        }
        NodeValue::TaskItem(checked) => {
            let state = if checked.is_some() { "DONE" } else { "TODO" };
            let children = convert_children(node, resources);
            // LK expects taskItem to contain inline content only (text, hardBreak, mention).
            // Comrak wraps content in paragraphs and may include nested lists.
            // Extract inlines from paragraphs; drop block-level children (bulletList, etc.)
            // since LK's schema doesn't support them inside taskItem.
            let mut inlines = Vec::new();
            for child in &children {
                let ctype = child.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if ctype == "paragraph" {
                    if let Some(content) = child.get("content").and_then(|c| c.as_array()) {
                        inlines.extend(content.iter().cloned());
                    }
                } else if is_inline_type(ctype) {
                    inlines.push(child.clone());
                }
                // Block-level children (bulletList, orderedList, etc.) are dropped —
                // LK cannot represent them inside taskItem.
            }
            let mut item = json!({
                "type": "taskItem",
                "attrs": { "state": state }
            });
            if !inlines.is_empty() {
                item["content"] = json!(inlines);
            }
            Some(item)
        }
        NodeValue::BlockQuote => {
            let children = convert_children(node, resources);
            if children.is_empty() {
                Some(json!({ "type": "blockquote" }))
            } else {
                Some(json!({
                    "type": "blockquote",
                    "content": children
                }))
            }
        }
        NodeValue::CodeBlock(NodeCodeBlock { info, literal, .. }) => {
            let mut block = json!({
                "type": "codeBlock",
            });
            if !info.is_empty() {
                block["attrs"] = json!({ "language": info });
            }
            if !literal.is_empty() {
                block["content"] = json!([{
                    "type": "text",
                    "text": literal.trim_end_matches('\n')
                }]);
            }
            Some(block)
        }
        NodeValue::ThematicBreak => Some(json!({ "type": "rule" })),
        NodeValue::Table(..) => {
            let rows = convert_children(node, resources);
            if rows.is_empty() {
                Some(json!({ "type": "table" }))
            } else {
                Some(json!({
                    "type": "table",
                    "content": rows
                }))
            }
        }
        NodeValue::TableRow(header) => {
            let cells: Vec<Value> = {
                let mut result = Vec::new();
                let mut child = node.first_child();
                while let Some(c) = child {
                    let cell_type = if *header {
                        "tableHeader"
                    } else {
                        "tableCell"
                    };
                    let inlines = convert_inline_children(c, resources);
                    let para = if inlines.is_empty() {
                        json!({ "type": "paragraph" })
                    } else {
                        json!({ "type": "paragraph", "content": inlines })
                    };
                    result.push(json!({
                        "type": cell_type,
                        "content": [para]
                    }));
                    child = c.next_sibling();
                }
                result
            };
            if cells.is_empty() {
                Some(json!({ "type": "tableRow" }))
            } else {
                Some(json!({
                    "type": "tableRow",
                    "content": cells
                }))
            }
        }
        NodeValue::TableCell => {
            // Handled by TableRow above
            None
        }
        NodeValue::HtmlInline(html) => {
            // Pass through as text
            if !html.is_empty() {
                Some(json!({ "type": "text", "text": html }))
            } else {
                None
            }
        }
        NodeValue::HtmlBlock(block) => {
            if !block.literal.is_empty() {
                Some(json!({
                    "type": "paragraph",
                    "content": [{ "type": "text", "text": block.literal.trim() }]
                }))
            } else {
                None
            }
        }
        // Document-level node — recurse
        NodeValue::Document => {
            let children = convert_children(node, resources);
            Some(json!({
                "type": "doc",
                "content": children
            }))
        }
        // Skip nodes we don't handle
        _ => None,
    }
}

/// Convert inline children of a node to ProseMirror inline nodes.
fn convert_inline_children<'a>(node: &'a AstNode<'a>, resources: &[Resource]) -> Vec<Value> {
    let mut result = Vec::new();
    let mut child = node.first_child();
    while let Some(c) = child {
        let nodes = convert_inline_node(c, resources);
        result.extend(nodes);
        child = c.next_sibling();
    }
    result
}

/// Convert a single inline node, potentially returning multiple PM nodes (e.g. text with mentions).
fn convert_inline_node<'a>(node: &'a AstNode<'a>, resources: &[Resource]) -> Vec<Value> {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Text(text) => convert_text_with_mentions(text, resources),
        NodeValue::SoftBreak => vec![json!({ "type": "hardBreak" })],
        NodeValue::LineBreak => vec![json!({ "type": "hardBreak" })],
        NodeValue::Code(NodeCode { literal, .. }) => {
            vec![json!({
                "type": "text",
                "text": literal,
                "marks": [{ "type": "code" }]
            })]
        }
        NodeValue::Emph => {
            let inlines = convert_inline_children(node, resources);
            add_mark_to_nodes(&inlines, json!({ "type": "em" }))
        }
        NodeValue::Strong => {
            let inlines = convert_inline_children(node, resources);
            add_mark_to_nodes(&inlines, json!({ "type": "strong" }))
        }
        NodeValue::Strikethrough => {
            let inlines = convert_inline_children(node, resources);
            add_mark_to_nodes(&inlines, json!({ "type": "strike" }))
        }
        NodeValue::Link(NodeLink { url, .. }) => {
            let inlines = convert_inline_children(node, resources);
            let mark = json!({
                "type": "link",
                "attrs": { "href": url }
            });
            add_mark_to_nodes(&inlines, mark)
        }
        NodeValue::Image(link) => {
            vec![json!({
                "type": "mediaSingle",
                "content": [{
                    "type": "media",
                    "attrs": {
                        "url": link.url,
                        "type": "external"
                    }
                }]
            })]
        }
        NodeValue::HtmlInline(html) => {
            if !html.is_empty() {
                vec![json!({ "type": "text", "text": html })]
            } else {
                vec![]
            }
        }
        _ => vec![],
    }
}

/// Parse text for `[[Resource Name]]` mentions and split into text + mention nodes.
fn convert_text_with_mentions(text: &str, resources: &[Resource]) -> Vec<Value> {
    let mut nodes = Vec::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("[[") {
        if let Some(end) = remaining[start..].find("]]") {
            let end = start + end;
            // Text before the mention
            if start > 0 {
                nodes.push(json!({
                    "type": "text",
                    "text": &remaining[..start]
                }));
            }
            // The mention text
            let mention_text = &remaining[start + 2..end];
            let resource_id = resources
                .iter()
                .find(|r| r.name.eq_ignore_ascii_case(mention_text))
                .map(|r| r.id.as_str())
                .unwrap_or("");
            nodes.push(json!({
                "type": "mention",
                "attrs": {
                    "id": resource_id,
                    "text": mention_text
                }
            }));
            remaining = &remaining[end + 2..];
        } else {
            // No closing ]], treat as plain text
            break;
        }
    }

    // Remaining text after last mention (or all text if no mentions)
    if !remaining.is_empty() {
        nodes.push(json!({
            "type": "text",
            "text": remaining
        }));
    }

    nodes
}

/// Represents a headerless table extracted from markdown before comrak parsing.
struct ExtractedTable {
    placeholder: String,
    rows: Vec<Vec<String>>, // Each row is a vec of cell values
}

/// Scan for runs of pipe-delimited lines without a separator row.
/// Replace them with unique placeholder text so comrak doesn't mangle them.
/// Returns the modified markdown and the extracted table data.
fn extract_headerless_tables(md: &str) -> (String, Vec<ExtractedTable>) {
    let lines: Vec<&str> = md.lines().collect();
    let mut out = String::with_capacity(md.len());
    let mut tables = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Skip non-table rows
        if !is_table_row(line) {
            out.push_str(lines[i]);
            out.push('\n');
            i += 1;
            continue;
        }

        // Found a table row — scan the full run
        let run_start = i;
        let mut run_end = i + 1;
        while run_end < lines.len() {
            let next = lines[run_end].trim();
            if is_table_row(next) || is_separator_row(next) {
                run_end += 1;
            } else {
                break;
            }
        }

        // Check if any line in the run is already a separator
        let has_sep = (run_start..run_end).any(|j| is_separator_row(lines[j].trim()));

        if has_sep {
            // Valid GFM table — let comrak handle it
            for j in run_start..run_end {
                out.push_str(lines[j]);
                out.push('\n');
            }
            i = run_end;
            continue;
        }

        if run_end - run_start < 2 {
            // Single pipe row — not a table, just emit
            out.push_str(lines[i]);
            out.push('\n');
            i += 1;
            continue;
        }

        // Headerless table — extract it
        let placeholder = format!("LKTABLEPLACEHOLDER{}", tables.len());
        let mut rows = Vec::new();
        for j in run_start..run_end {
            let cells = parse_pipe_row(lines[j]);
            rows.push(cells);
        }
        tables.push(ExtractedTable {
            placeholder: placeholder.clone(),
            rows,
        });

        // Emit placeholder as its own paragraph
        out.push('\n');
        out.push_str(&placeholder);
        out.push_str("\n\n");
        i = run_end;
    }

    // Match original trailing newline behavior
    if !md.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }

    (out, tables)
}

/// Parse a pipe-delimited row into cell values.
/// "| Sexuality | Heterosexual |" → ["Sexuality", "Heterosexual"]
fn parse_pipe_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let parts: Vec<&str> = trimmed.split('|').collect();
    let mut cells = Vec::new();
    for part in &parts {
        let cell = part.trim();
        if !cell.is_empty() {
            cells.push(cell.to_string());
        }
    }
    // Handle edge: if line starts/ends with |, split produces empty first/last
    // The loop above already skips empty strings, so we're fine
    cells
}

/// Replace placeholder paragraphs in the ProseMirror tree with header-column tables.
/// Header-column = first cell of every row is `tableHeader`, rest are `tableCell`.
fn replace_table_placeholders(
    children: Vec<Value>,
    tables: &[ExtractedTable],
    resources: &[Resource],
) -> Vec<Value> {
    if tables.is_empty() {
        return children;
    }

    let mut result = Vec::new();
    for child in children {
        if let Some(placeholder_idx) = find_placeholder_in_node(&child, tables) {
            let table = &tables[placeholder_idx];
            result.push(build_header_column_table(table, resources));
        } else {
            result.push(child);
        }
    }
    result
}

/// Check if a ProseMirror node is a paragraph containing only a placeholder string.
fn find_placeholder_in_node(node: &Value, tables: &[ExtractedTable]) -> Option<usize> {
    if node.get("type")?.as_str()? != "paragraph" {
        return None;
    }
    let content = node.get("content")?.as_array()?;
    if content.len() != 1 {
        return None;
    }
    let text = content[0].get("text")?.as_str()?;
    tables
        .iter()
        .position(|t| text.trim() == t.placeholder)
}

/// Build a ProseMirror table with header-column layout.
/// First cell of every row is `tableHeader`, rest are `tableCell`.
fn build_header_column_table(table: &ExtractedTable, resources: &[Resource]) -> Value {
    let rows: Vec<Value> = table
        .rows
        .iter()
        .map(|row| {
            let cells: Vec<Value> = row
                .iter()
                .enumerate()
                .map(|(col_idx, cell_text)| {
                    let cell_type = if col_idx == 0 {
                        "tableHeader"
                    } else {
                        "tableCell"
                    };
                    let inlines = convert_text_with_mentions(cell_text, resources);
                    let para = if inlines.is_empty() {
                        json!({ "type": "paragraph" })
                    } else {
                        json!({ "type": "paragraph", "content": inlines })
                    };
                    json!({
                        "type": cell_type,
                        "content": [para]
                    })
                })
                .collect();
            json!({
                "type": "tableRow",
                "content": cells
            })
        })
        .collect();
    json!({
        "type": "table",
        "content": rows
    })
}

/// Check if a ProseMirror node type is valid inline content for LK.
fn is_inline_type(node_type: &str) -> bool {
    matches!(
        node_type,
        "text" | "hardBreak" | "mention" | "inlineExtension"
    )
}

/// Add a mark to all text nodes in a list, returning the modified list.
fn add_mark_to_nodes(nodes: &[Value], mark: Value) -> Vec<Value> {
    nodes
        .iter()
        .map(|node| {
            let mut node = node.clone();
            if node.get("type").and_then(|t| t.as_str()) == Some("text") {
                let marks = node
                    .get("marks")
                    .and_then(|m| m.as_array())
                    .cloned()
                    .unwrap_or_default();
                let mut new_marks = marks;
                new_marks.push(mark.clone());
                node["marks"] = json!(new_marks);
            }
            node
        })
        .collect()
}
