use serde_json::Value;

use super::types::{PmMark, PmNode};

/// Convert a ProseMirror JSON value to markdown.
pub fn to_markdown(value: &Value) -> String {
    let node: PmNode = match serde_json::from_value(value.clone()) {
        Ok(n) => n,
        Err(_) => return String::new(),
    };
    let mut out = String::new();
    render_node(&node, &mut out, &Context::default());
    out.trim_end().to_string()
}

#[derive(Default, Clone)]
struct Context {
    list_prefix: Option<String>,
    indent: String,
    in_table_cell: bool,
}

fn render_node(node: &PmNode, out: &mut String, ctx: &Context) {
    match node.node_type.as_str() {
        "doc" => render_children(node, out, ctx),

        "paragraph" => {
            render_inline_children(node, out, ctx);
            if ctx.in_table_cell {
                // no trailing newlines in table cells
            } else {
                out.push_str("\n\n");
            }
        }

        "heading" => {
            let level = node
                .attrs
                .as_ref()
                .and_then(|a| a.get("level"))
                .and_then(|l| l.as_u64())
                .unwrap_or(1) as usize;
            out.push_str(&ctx.indent);
            for _ in 0..level {
                out.push('#');
            }
            out.push(' ');
            render_inline_children(node, out, ctx);
            out.push_str("\n\n");
        }

        "text" => {
            let text = node.text.as_deref().unwrap_or("");
            let wrapped = apply_marks(text, node.marks.as_deref().unwrap_or(&[]));
            out.push_str(&wrapped);
        }

        "hardBreak" => {
            out.push('\n');
        }

        "bulletList" => {
            render_list_items(node, out, ctx, false);
        }

        "orderedList" => {
            render_list_items(node, out, ctx, true);
        }

        "listItem" => {
            let prefix = ctx.list_prefix.as_deref().unwrap_or("- ");
            out.push_str(&ctx.indent);
            out.push_str(prefix);
            let child_ctx = Context {
                indent: format!("{}  ", ctx.indent),
                list_prefix: None,
                ..(ctx.clone())
            };
            if let Some(children) = &node.content {
                for (i, child) in children.iter().enumerate() {
                    if i > 0 && matches!(child.node_type.as_str(), "bulletList" | "orderedList" | "taskList") {
                        // nested list — already handled by indent
                    }
                    if i == 0 {
                        // First child paragraph: render inline, no indent
                        if child.node_type == "paragraph" {
                            render_inline_children(child, out, ctx);
                            out.push('\n');
                            continue;
                        }
                    }
                    render_node(child, out, &child_ctx);
                }
            }
        }

        "taskList" => {
            render_task_items(node, out, ctx);
        }

        "taskItem" => {
            let state = node
                .attrs
                .as_ref()
                .and_then(|a| a.get("state"))
                .and_then(|s| s.as_str())
                .unwrap_or("TODO");
            let checkbox = if state == "DONE" { "- [x] " } else { "- [ ] " };
            out.push_str(&ctx.indent);
            out.push_str(checkbox);
            if let Some(children) = &node.content {
                for (i, child) in children.iter().enumerate() {
                    if i == 0 && child.node_type == "paragraph" {
                        render_inline_children(child, out, ctx);
                        out.push('\n');
                        continue;
                    }
                    let child_ctx = Context {
                        indent: format!("{}  ", ctx.indent),
                        ..(ctx.clone())
                    };
                    render_node(child, out, &child_ctx);
                }
            }
        }

        "blockquote" => {
            let mut inner = String::new();
            render_children(node, &mut inner, &Context::default());
            for line in inner.trim_end().lines() {
                out.push_str(&ctx.indent);
                out.push_str("> ");
                out.push_str(line);
                out.push('\n');
            }
            out.push('\n');
        }

        "codeBlock" => {
            let lang = node
                .attrs
                .as_ref()
                .and_then(|a| a.get("language"))
                .and_then(|l| l.as_str())
                .unwrap_or("");
            out.push_str(&ctx.indent);
            out.push_str("```");
            out.push_str(lang);
            out.push('\n');
            // Code block children are text nodes
            if let Some(children) = &node.content {
                for child in children {
                    if let Some(text) = &child.text {
                        out.push_str(text);
                    }
                }
            }
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&ctx.indent);
            out.push_str("```\n\n");
        }

        "rule" => {
            out.push_str(&ctx.indent);
            out.push_str("---\n\n");
        }

        "table" => {
            render_table(node, out, ctx);
        }

        "mention" => {
            let text = node
                .attrs
                .as_ref()
                .and_then(|a| a.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("?");
            out.push_str("[[");
            out.push_str(text);
            out.push_str("]]");
        }

        "mediaSingle" => {
            // Contains a media child
            if let Some(children) = &node.content {
                for child in children {
                    render_node(child, out, ctx);
                }
            }
            out.push_str("\n\n");
        }

        "media" => {
            let url = node
                .attrs
                .as_ref()
                .and_then(|a| a.get("url"))
                .and_then(|u| u.as_str())
                .unwrap_or("");
            out.push_str("![](");
            out.push_str(url);
            out.push(')');
        }

        "layoutSection" | "layoutColumn" => {
            // Flatten: render children sequentially
            render_children(node, out, ctx);
        }

        "panel" => {
            let panel_type = node
                .attrs
                .as_ref()
                .and_then(|a| a.get("panelType"))
                .and_then(|t| t.as_str())
                .unwrap_or("info");
            let mut inner = String::new();
            render_children(node, &mut inner, &Context::default());
            let first_line = true;
            for (i, line) in inner.trim_end().lines().enumerate() {
                out.push_str(&ctx.indent);
                out.push_str("> ");
                if i == 0 && first_line {
                    out.push_str("**");
                    // Capitalize first letter
                    let mut chars = panel_type.chars();
                    if let Some(c) = chars.next() {
                        out.extend(c.to_uppercase());
                        out.push_str(chars.as_str());
                    }
                    out.push_str(":** ");
                }
                out.push_str(line);
                out.push('\n');
            }
            out.push('\n');
        }

        "extension" => {
            // Extensions are LK-specific blocks. Render text attr if available.
            let text = node
                .attrs
                .as_ref()
                .and_then(|a| a.get("text"))
                .and_then(|t| t.as_str());
            if let Some(text) = text {
                out.push_str(&ctx.indent);
                out.push_str("*[");
                out.push_str(text);
                out.push_str("]*\n\n");
            }
        }

        "bodiedExtension" => {
            // Render children if present
            if node.content.is_some() {
                render_children(node, out, ctx);
            }
        }

        // Unknown nodes: recurse into children silently
        _ => {
            render_children(node, out, ctx);
        }
    }
}

fn render_children(node: &PmNode, out: &mut String, ctx: &Context) {
    if let Some(children) = &node.content {
        for child in children {
            render_node(child, out, ctx);
        }
    }
}

fn render_inline_children(node: &PmNode, out: &mut String, ctx: &Context) {
    if let Some(children) = &node.content {
        for child in children {
            render_node(child, out, ctx);
        }
    }
}

fn apply_marks(text: &str, marks: &[PmMark]) -> String {
    let mut result = text.to_string();
    for mark in marks {
        match mark.mark_type.as_str() {
            "strong" => {
                result = format!("**{}**", result);
            }
            "em" => {
                result = format!("*{}*", result);
            }
            "code" => {
                result = format!("`{}`", result);
            }
            "strike" => {
                result = format!("~~{}~~", result);
            }
            "underline" => {
                // No standard markdown for underline, use HTML
                result = format!("<u>{}</u>", result);
            }
            "link" => {
                let href = mark
                    .attrs
                    .as_ref()
                    .and_then(|a| a.get("href"))
                    .and_then(|h| h.as_str())
                    .unwrap_or("");
                result = format!("[{}]({})", result, href);
            }
            _ => {}
        }
    }
    result
}

fn render_list_items(node: &PmNode, out: &mut String, ctx: &Context, ordered: bool) {
    if let Some(children) = &node.content {
        for (i, child) in children.iter().enumerate() {
            let prefix = if ordered {
                format!("{}. ", i + 1)
            } else {
                "- ".to_string()
            };
            let item_ctx = Context {
                list_prefix: Some(prefix),
                ..(ctx.clone())
            };
            render_node(child, out, &item_ctx);
        }
    }
}

fn render_task_items(node: &PmNode, out: &mut String, ctx: &Context) {
    if let Some(children) = &node.content {
        for child in children {
            render_node(child, out, ctx);
        }
    }
}

fn render_table(node: &PmNode, out: &mut String, ctx: &Context) {
    let rows = match &node.content {
        Some(c) => c,
        None => return,
    };

    // Collect all rows as vectors of cell text
    let mut table_data: Vec<Vec<String>> = Vec::new();
    for row in rows {
        if row.node_type != "tableRow" {
            continue;
        }
        let mut row_cells = Vec::new();
        if let Some(cells) = &row.content {
            for cell in cells {
                let cell_ctx = Context {
                    in_table_cell: true,
                    ..(ctx.clone())
                };
                let mut cell_text = String::new();
                render_children(cell, &mut cell_text, &cell_ctx);
                // Clean up cell text: remove trailing newlines, replace internal newlines with spaces
                let cell_text = cell_text.trim().replace('\n', " ");
                row_cells.push(cell_text);
            }
        }
        table_data.push(row_cells);
    }

    if table_data.is_empty() {
        return;
    }

    // Calculate column widths
    let num_cols = table_data.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut col_widths = vec![3usize; num_cols];
    for row in &table_data {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                col_widths[i] = col_widths[i].max(cell.len());
            }
        }
    }

    // Render rows
    for (row_idx, row) in table_data.iter().enumerate() {
        out.push_str(&ctx.indent);
        out.push('|');
        for (i, width) in col_widths.iter().enumerate() {
            let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
            out.push(' ');
            out.push_str(cell);
            for _ in 0..(width - cell.len()) {
                out.push(' ');
            }
            out.push_str(" |");
        }
        out.push('\n');

        // After first row, insert separator
        if row_idx == 0 {
            out.push_str(&ctx.indent);
            out.push('|');
            for width in &col_widths {
                out.push(' ');
                for _ in 0..*width {
                    out.push('-');
                }
                out.push_str(" |");
            }
            out.push('\n');
        }
    }
    out.push('\n');
}
