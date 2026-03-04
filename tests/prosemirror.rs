use std::path::Path;
use legend_keeper_mcp::lk::io::read_lk_file;
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
