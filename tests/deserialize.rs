use std::path::Path;

#[test]
fn deserialize_all_reference_files() {
    let ref_dir = Path::new("tests/reference");
    if !ref_dir.exists() {
        panic!("tests/reference/ directory not found");
    }

    let mut count = 0;
    for entry in std::fs::read_dir(ref_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("lk") {
            eprintln!("Deserializing: {}", path.display());
            let root = legend_keeper_mcp::lk::io::read_lk_file(&path)
                .unwrap_or_else(|e| panic!("Failed to deserialize {}: {}", path.display(), e));
            eprintln!(
                "  {} — {} resources, {} calendars",
                path.file_stem().unwrap().to_str().unwrap(),
                root.resources.len(),
                root.calendars.len()
            );
            assert_eq!(root.resources.len(), root.resource_count);
            count += 1;
        }
    }
    assert!(count > 0, "No .lk files found in tests/reference/");
}
