use std::path::Path;
use legend_keeper_mcp::lk::io::read_lk_file;
use legend_keeper_mcp::prosemirror::from_markdown::from_markdown;
use legend_keeper_mcp::prosemirror::to_markdown::to_markdown;

#[test]
fn convert_all_page_documents() {
    let ref_dir = Path::new("tests/reference");
    let mut total_docs = 0;
    let mut total_nonempty = 0;

    for entry in std::fs::read_dir(ref_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("lk") {
            continue;
        }
        let root = read_lk_file(&path).unwrap();
        let world = path.file_stem().unwrap().to_str().unwrap();

        for resource in &root.resources {
            for doc in &resource.documents {
                if doc.doc_type == "page" {
                    if let Some(content) = &doc.content {
                        total_docs += 1;
                        let md = to_markdown(content);
                        if !md.is_empty() {
                            total_nonempty += 1;
                        }
                    }
                }
            }
        }

        // Show a few samples
        let mut shown = 0;
        for resource in &root.resources {
            if shown >= 3 { break; }
            for doc in &resource.documents {
                if doc.doc_type == "page" {
                    if let Some(content) = &doc.content {
                        let md = to_markdown(content);
                        if md.len() > 100 {
                            eprintln!("--- {}/{}/{} ---", world, resource.name, doc.name);
                            // Show first 500 chars
                            eprintln!("{}", &md[..md.len().min(500)]);
                            eprintln!();
                            shown += 1;
                            break;
                        }
                    }
                }
            }
        }
    }

    eprintln!("Total page docs: {}, non-empty: {}", total_docs, total_nonempty);
    assert!(total_docs > 0);
}

// --- from_markdown tests ---

#[test]
fn from_markdown_table() {
    let md = "| Trait | Value |\n|---|---|\n| Sexuality | n/a |\n| Languages | Common, Draconic |\n";
    let pm = from_markdown(md, &[]);

    // Should produce a table node
    let content = pm["content"].as_array().unwrap();
    let table = &content[0];
    assert_eq!(table["type"], "table", "Expected table node, got: {}", serde_json::to_string_pretty(&pm).unwrap());

    let rows = table["content"].as_array().unwrap();
    assert_eq!(rows.len(), 3, "Expected 3 rows (1 header + 2 data)");

    // First row should be header
    let header_row = &rows[0];
    assert_eq!(header_row["type"], "tableRow");
    let header_cells = header_row["content"].as_array().unwrap();
    assert_eq!(header_cells[0]["type"], "tableHeader");
    // Header cell should contain a paragraph with text "Trait"
    let header_para = &header_cells[0]["content"][0];
    assert_eq!(header_para["type"], "paragraph");
    let header_text = header_para["content"][0]["text"].as_str().unwrap();
    assert_eq!(header_text, "Trait");

    // Second row should be data
    let data_row = &rows[1];
    let data_cells = data_row["content"].as_array().unwrap();
    assert_eq!(data_cells[0]["type"], "tableCell");
    let cell_text = data_cells[0]["content"][0]["content"][0]["text"].as_str().unwrap();
    assert_eq!(cell_text, "Sexuality");
    let cell_value = data_cells[1]["content"][0]["content"][0]["text"].as_str().unwrap();
    assert_eq!(cell_value, "n/a");

    // Third row
    let row3 = &rows[2];
    let cells3 = row3["content"].as_array().unwrap();
    let lang_value = cells3[1]["content"][0]["content"][0]["text"].as_str().unwrap();
    assert_eq!(lang_value, "Common, Draconic");
}

#[test]
fn from_markdown_table_with_surrounding_content() {
    let md = "## Other Traits\n\n| Key | Value |\n|---|---|\n| Intelligence | Sharp |\n\n## Personality\n\nBold and brave.\n";
    let pm = from_markdown(md, &[]);

    let content = pm["content"].as_array().unwrap();
    // Should be: heading, table, heading, paragraph
    assert!(content.len() >= 4, "Expected at least 4 nodes, got {}: {}", content.len(), serde_json::to_string_pretty(&pm).unwrap());
    assert_eq!(content[0]["type"], "heading");
    assert_eq!(content[1]["type"], "table");
    assert_eq!(content[2]["type"], "heading");
    assert_eq!(content[3]["type"], "paragraph");
}

#[test]
fn from_markdown_multi_column_table() {
    let md = "| Name | Race | Class | Level |\n|---|---|---|---|\n| Gandalf | Maia | Wizard | 20 |\n| Aragorn | Human | Ranger | 16 |\n";
    let pm = from_markdown(md, &[]);

    let table = &pm["content"][0];
    assert_eq!(table["type"], "table");

    let rows = table["content"].as_array().unwrap();
    assert_eq!(rows.len(), 3);

    // Header should have 4 cells
    let header_cells = rows[0]["content"].as_array().unwrap();
    assert_eq!(header_cells.len(), 4);

    // Data rows should have 4 cells each
    let data_cells = rows[1]["content"].as_array().unwrap();
    assert_eq!(data_cells.len(), 4);
}

#[test]
fn from_markdown_strikethrough() {
    let md = "This is ~~deleted~~ text.\n";
    let pm = from_markdown(md, &[]);

    let para = &pm["content"][0];
    assert_eq!(para["type"], "paragraph");
    let inlines = para["content"].as_array().unwrap();

    // Should have text nodes, one with strike mark
    let struck = inlines.iter().find(|n| {
        n.get("marks")
            .and_then(|m| m.as_array())
            .map(|marks| marks.iter().any(|m| m["type"] == "strike"))
            .unwrap_or(false)
    });
    assert!(struck.is_some(), "Expected a struck-through text node, got: {:?}", inlines);
    assert_eq!(struck.unwrap()["text"], "deleted");
}

#[test]
fn from_markdown_tasklist() {
    let md = "- [x] Done task\n- [ ] Todo task\n";
    let pm = from_markdown(md, &[]);

    let list = &pm["content"][0];
    assert_eq!(list["type"], "taskList", "Expected taskList, got: {}", serde_json::to_string_pretty(&pm).unwrap());

    let items = list["content"].as_array().unwrap();
    assert_eq!(items.len(), 2);

    assert_eq!(items[0]["type"], "taskItem");
    assert_eq!(items[0]["attrs"]["state"], "DONE");
    // taskItem should contain inline content directly (not wrapped in paragraph)
    let done_content = items[0]["content"].as_array().unwrap();
    assert_eq!(done_content[0]["type"], "text", "taskItem should contain text directly, got: {}", serde_json::to_string_pretty(&items[0]).unwrap());
    assert_eq!(done_content[0]["text"], "Done task");

    assert_eq!(items[1]["type"], "taskItem");
    assert_eq!(items[1]["attrs"]["state"], "TODO");
    let todo_content = items[1]["content"].as_array().unwrap();
    assert_eq!(todo_content[0]["type"], "text");
    assert_eq!(todo_content[0]["text"], "Todo task");
}

#[test]
fn from_markdown_table_without_header_separator() {
    // LLMs often generate tables without the header separator row.
    // These should become header-COLUMN tables: first cell of every row is tableHeader.
    let md = "| Sexuality | Heterosexual |\n| Languages | Common, Dwarvish, Draconic |\n| Intelligence | Sharp and tactical |\n";
    let pm = from_markdown(md, &[]);

    let content = pm["content"].as_array().unwrap();
    let table = content.iter().find(|n| n["type"] == "table");
    assert!(table.is_some(), "Expected a table node from headerless pipe syntax, got: {}", serde_json::to_string_pretty(&pm).unwrap());

    let rows = table.unwrap()["content"].as_array().unwrap();
    assert_eq!(rows.len(), 3, "Expected 3 rows");

    // Every row: first cell = tableHeader, second cell = tableCell
    for (i, row) in rows.iter().enumerate() {
        let cells = row["content"].as_array().unwrap();
        assert_eq!(cells[0]["type"], "tableHeader", "Row {} first cell should be tableHeader", i);
        assert_eq!(cells[1]["type"], "tableCell", "Row {} second cell should be tableCell", i);
    }

    // Verify cell content
    let cell_text = rows[0]["content"][0]["content"][0]["content"][0]["text"].as_str().unwrap();
    assert_eq!(cell_text, "Sexuality");
    let cell_value = rows[0]["content"][1]["content"][0]["content"][0]["text"].as_str().unwrap();
    assert_eq!(cell_value, "Heterosexual");
}

#[test]
fn from_markdown_table_without_separator_surrounded_by_content() {
    // Table without separator between other content
    let md = "## Other Traits\n\n| Sexuality | Heterosexual |\n| Languages | Common |\n\n## Personality\n\nBold.\n";
    let pm = from_markdown(md, &[]);

    let content = pm["content"].as_array().unwrap();
    let types: Vec<&str> = content.iter().map(|n| n["type"].as_str().unwrap()).collect();
    assert!(types.contains(&"table"), "Expected table in output, got types: {:?}\nFull: {}", types, serde_json::to_string_pretty(&pm).unwrap());
}

#[test]
fn from_markdown_table_roundtrip() {
    // Create a table via from_markdown, convert back via to_markdown, verify structure
    let original_md = "| Trait | Value |\n|---|---|\n| Sexuality | n/a |\n| Languages | Common |\n";
    let pm = from_markdown(original_md, &[]);

    // Convert back to markdown
    let roundtrip_md = to_markdown(&pm);

    // The roundtrip markdown should contain a table with the same data
    assert!(roundtrip_md.contains("Trait"), "Roundtrip should contain 'Trait': {}", roundtrip_md);
    assert!(roundtrip_md.contains("Sexuality"), "Roundtrip should contain 'Sexuality': {}", roundtrip_md);
    assert!(roundtrip_md.contains("n/a"), "Roundtrip should contain 'n/a': {}", roundtrip_md);
    assert!(roundtrip_md.contains("Languages"), "Roundtrip should contain 'Languages': {}", roundtrip_md);
    assert!(roundtrip_md.contains("|"), "Roundtrip should contain table pipes: {}", roundtrip_md);
}
