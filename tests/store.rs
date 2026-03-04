use std::path::Path;
use legend_keeper_mcp::lk::store::WorldStore;

#[test]
fn load_and_query_worlds() {
    let store = WorldStore::load(Path::new("tests/reference")).unwrap();

    // list_worlds
    let worlds = store.list_worlds();
    assert!(worlds.len() >= 2, "Expected at least 2 worlds");
    eprintln!("Worlds: {:?}", worlds.iter().map(|w| &w.name).collect::<Vec<_>>());

    // list_resources with no filter
    let resources = store.list_resources(&Some("rime".to_string()), &None, &None).unwrap();
    assert!(!resources.is_empty());
    eprintln!("Rime resources: {}", resources.len());

    // list_resources with tag filter
    let npcs = store.list_resources(&Some("siqram".to_string()), &Some("npc".to_string()), &None).unwrap();
    eprintln!("Siqram NPCs: {}", npcs.len());

    // list_resources with name filter
    let named = store.list_resources(&Some("rime".to_string()), &None, &Some("caer".to_string())).unwrap();
    assert!(!named.is_empty(), "Should find resources matching 'caer'");
    eprintln!("Rime 'caer' matches: {}", named.len());

    // get_resource by ID
    let first_id = &resources[0].id;
    let resource = store.get_resource(&Some("rime".to_string()), first_id).unwrap();
    assert_eq!(resource.id, *first_id);

    // get_resource by name (case-insensitive)
    let resource = store.get_resource(&Some("rime".to_string()), &resources[0].name.to_uppercase()).unwrap();
    assert_eq!(resource.id, *first_id);

    // get_resource_tree (full tree)
    let tree = store.get_resource_tree(&Some("rime".to_string()), &None).unwrap();
    assert!(!tree.is_empty(), "Tree should have root nodes");
    eprintln!("Rime root nodes: {}", tree.len());

    // search_content
    let results = store.search_content(&Some("rime".to_string()), "ice", Some(5)).unwrap();
    eprintln!("Search 'ice' in rime: {} results", results.len());

    // get_calendar
    let cal = store.get_calendar(&Some("siqram".to_string()), "Siqram");
    assert!(cal.is_ok(), "Should find Siqram calendar");
    let cal = cal.unwrap();
    eprintln!("Calendar: {} ({} months, {} weekdays)", cal.name, cal.months.len(), cal.weekdays.len());
}

#[test]
fn hot_reload_add_and_remove() {
    let tmp = tempfile::tempdir().unwrap();
    let store = WorldStore::load(tmp.path()).unwrap();
    let _watcher = store.start_watcher().unwrap();

    assert_eq!(store.list_worlds().len(), 0);

    // Copy a reference .lk file into the watched directory
    let src = Path::new("tests/reference/rime.lk");
    let dst = tmp.path().join("rime.lk");
    std::fs::copy(src, &dst).unwrap();

    // Give the watcher time to pick it up
    std::thread::sleep(std::time::Duration::from_millis(500));

    let worlds = store.list_worlds();
    assert_eq!(worlds.len(), 1, "Should have loaded rime after copy");
    assert_eq!(worlds[0].name, "rime");

    // Remove the file
    std::fs::remove_file(&dst).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));

    assert_eq!(store.list_worlds().len(), 0, "Should have unloaded rime after delete");
}

#[test]
fn map_documents_have_pins() {
    let store = WorldStore::load(Path::new("tests/reference")).unwrap();
    let world = Some("rime".to_string());
    let resources = store.list_resources(&world, &None, &None).unwrap();

    let mut map_count = 0;
    let mut total_pins = 0;

    for summary in &resources {
        let resource = store.get_resource(&world, &summary.id).unwrap();
        for doc in &resource.documents {
            if doc.doc_type == "map" {
                map_count += 1;
                if let Some(content) = &doc.content {
                    if let Ok(map_content) = serde_json::from_value::<legend_keeper_mcp::lk::schema::MapContent>(content.clone()) {
                        let pins: Vec<_> = map_content.pins.iter().filter(|f| f.feature_type.is_none()).collect();
                        eprintln!("Map '{}' on '{}': {} pins", doc.name, resource.name, pins.len());
                        total_pins += pins.len();
                    }
                }
            }
        }
    }

    assert!(map_count > 0, "Rime should have at least one map document");
    assert!(total_pins > 0, "Rime maps should have pins");
    eprintln!("Total: {} maps, {} pins", map_count, total_pins);
}

#[test]
fn world_guide_detection() {
    // Reference worlds likely don't have llm-guide tags, so verify it returns None gracefully
    let store = WorldStore::load(Path::new("tests/reference")).unwrap();
    let worlds = store.list_worlds();

    for w in &worlds {
        eprintln!("World '{}': guide = {:?}", w.name, w.guide.as_deref().map(|g| &g[..g.len().min(50)]));
    }
    // Just verify it doesn't crash — guide will be None unless reference data has the tag
}

#[test]
fn inspect_map_data() {
    let store = WorldStore::load(Path::new("tests/reference")).unwrap();
    for world_name in &["rime", "siqram"] {
        let world = Some(world_name.to_string());
        let resources = store.list_resources(&world, &None, &None).unwrap();
        for summary in &resources {
            let resource = store.get_resource(&world, &summary.id).unwrap();
            for doc in &resource.documents {
                if doc.doc_type != "map" { continue; }
                let map_id = doc.map.as_ref().map(|m| m.map_id.as_str()).unwrap_or("(none)");
                let mut pins = 0; let mut regions = 0; let mut labels = 0; let mut paths = 0; let mut other = 0;
                if let Some(content) = &doc.content {
                    if let Ok(mc) = serde_json::from_value::<legend_keeper_mcp::lk::schema::MapContent>(content.clone()) {
                        for f in &mc.pins {
                            match f.feature_type.as_deref() {
                                None => pins += 1,
                                Some("region") => regions += 1,
                                Some("label") => labels += 1,
                                Some("path") => paths += 1,
                                Some(_) => other += 1,
                            }
                        }
                    }
                }
                eprintln!("[{}] {} / {} — map_id={} | pins={} regions={} labels={} paths={} other={}", world_name, resource.name, doc.name, map_id, pins, regions, labels, paths, other);
            }
        }
    }
}
