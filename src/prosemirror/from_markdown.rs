use comrak::nodes::{AstNode, ListType, NodeCode, NodeCodeBlock, NodeHeading, NodeLink, NodeValue};
use comrak::{parse_document, Arena, Options};
use serde_json::{json, Value};

use crate::lk::schema::Resource;

/// Convert markdown text to ProseMirror JSON.
/// Resources are used to resolve `[[Name]]` mentions to resource IDs.
pub fn from_markdown(md: &str, resources: &[Resource]) -> Value {
    let arena = Arena::new();
    let options = Options::default();
    let root = parse_document(&arena, md, &options);

    let children = convert_children(root, resources);
    json!({
        "type": "doc",
        "content": children
    })
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
            let mut item = json!({
                "type": "taskItem",
                "attrs": { "state": state }
            });
            if !children.is_empty() {
                item["content"] = json!(children);
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
