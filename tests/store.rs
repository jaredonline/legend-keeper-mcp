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
