use legend_keeper_mcp::lk::builder::WorldBuilder;
use legend_keeper_mcp::lk::io::read_lk_file;
use tempfile::TempDir;

#[test]
fn create_world_and_export() {
    let mut builder = WorldBuilder::new("test-world");

    // Create a root resource with content
    let root = builder
        .create_resource(
            "The Kingdom",
            None,
            Some(vec!["location".to_string()]),
            Some("# The Kingdom\n\nA vast and ancient realm."),
            false,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
    assert!(!root.id.is_empty());
    assert_eq!(root.name, "The Kingdom");
    assert_eq!(root.document_count, 1);

    // Create a child resource
    let child = builder
        .create_resource(
            "The Capital",
            Some(&root.id),
            Some(vec!["location".to_string(), "city".to_string()]),
            Some("The capital city sits on a hill."),
            false,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
    assert_eq!(child.parent_id.as_deref(), Some(root.id.as_str()));

    // Create another child
    let _npc = builder
        .create_resource(
            "King Aldric",
            Some(&root.id),
            Some(vec!["npc".to_string()]),
            Some("The aging king who rules the realm with a firm hand."),
            false,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();

    // Verify draft summary
    let summary = builder.list_draft();
    assert_eq!(summary.name, "test-world");
    assert_eq!(summary.resource_count, 3);

    // Export to temp directory
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("test-world.lk");
    let path = builder
        .export_world(Some(output.to_str().unwrap()))
        .unwrap();
    assert!(path.exists());

    // Read back and verify
    let root = read_lk_file(&path).unwrap();
    assert_eq!(root.version, 1);
    assert_eq!(root.resource_count, 3);
    assert_eq!(root.resources.len(), 3);
    assert!(!root.hash.is_empty());
    assert!(!root.exported_at.is_empty());

    // Verify resource names
    let names: Vec<&str> = root.resources.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"The Kingdom"));
    assert!(names.contains(&"The Capital"));
    assert!(names.contains(&"King Aldric"));

    // Verify parent-child relationships
    let capital = root.resources.iter().find(|r| r.name == "The Capital").unwrap();
    let kingdom = root.resources.iter().find(|r| r.name == "The Kingdom").unwrap();
    assert_eq!(capital.parent_id.as_deref(), Some(kingdom.id.as_str()));

    // Verify ProseMirror content was generated
    let kingdom_doc = &kingdom.documents[0];
    assert_eq!(kingdom_doc.doc_type, "page");
    assert!(kingdom_doc.content.is_some());
    let content = kingdom_doc.content.as_ref().unwrap();
    assert_eq!(content["type"], "doc");
    // Should contain the heading and paragraph
    let children = content["content"].as_array().unwrap();
    assert!(children.len() >= 2);
}

#[test]
fn add_document_to_resource() {
    let mut builder = WorldBuilder::new("test");

    let res = builder
        .create_resource("Tavern", None, None, Some("A cozy tavern."), false, Vec::new(), Vec::new())
        .unwrap();

    // Add a second document
    let doc = builder
        .add_document(&res.id, "DM Notes", "Secret passage behind the bar.", None, false)
        .unwrap();
    assert!(!doc.id.is_empty());
    assert_eq!(doc.name, "DM Notes");
    assert_eq!(doc.doc_type, "page");

    // Verify resource now has 2 documents
    let summary = builder.list_draft();
    let tavern = summary.resources.iter().find(|r| r.name == "Tavern").unwrap();
    assert_eq!(tavern.document_count, 2);
}

#[test]
fn set_content_updates_document() {
    let mut builder = WorldBuilder::new("test");

    let res = builder
        .create_resource("Place", None, None, Some("Original content."), false, Vec::new(), Vec::new())
        .unwrap();

    // Update the content
    builder
        .set_content(&res.id, None, "# Updated\n\nNew content here.")
        .unwrap();

    // Export and verify
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("test.lk");
    builder
        .export_world(Some(output.to_str().unwrap()))
        .unwrap();

    let root = read_lk_file(&output).unwrap();
    let place = root.resources.iter().find(|r| r.name == "Place").unwrap();
    let content = place.documents[0].content.as_ref().unwrap();
    // Should have a heading "Updated"
    let children = content["content"].as_array().unwrap();
    let heading = children.iter().find(|n| n["type"] == "heading").unwrap();
    assert!(heading["content"][0]["text"].as_str().unwrap().contains("Updated"));
}

#[test]
fn invalid_parent_id_errors() {
    let mut builder = WorldBuilder::new("test");
    let result = builder.create_resource("Child", Some("nonexistent"), None, None, false, Vec::new(), Vec::new());
    assert!(result.is_err());
}

#[test]
fn operations_without_draft_resource_error() {
    let mut builder = WorldBuilder::new("test");
    let result = builder.add_document("nonexistent", "Doc", "content", None, false);
    assert!(result.is_err());

    let result = builder.set_content("nonexistent", None, "content");
    assert!(result.is_err());
}

#[test]
fn mention_resolution_in_content() {
    let mut builder = WorldBuilder::new("test");

    let npc = builder
        .create_resource("Gandalf", None, None, None, false, Vec::new(), Vec::new())
        .unwrap();

    // Create a resource that mentions Gandalf
    let _place = builder
        .create_resource(
            "Shire",
            None,
            None,
            Some("[[Gandalf]] visited the Shire."),
            false,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();

    // Export and verify mention was resolved
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("test.lk");
    builder
        .export_world(Some(output.to_str().unwrap()))
        .unwrap();

    let root = read_lk_file(&output).unwrap();
    let shire = root.resources.iter().find(|r| r.name == "Shire").unwrap();
    let content = shire.documents[0].content.as_ref().unwrap();

    // Find the mention node
    let content_str = serde_json::to_string(content).unwrap();
    assert!(content_str.contains("\"type\":\"mention\""));
    assert!(content_str.contains(&npc.id));
}

#[test]
fn hidden_resource_persists_through_export() {
    let mut builder = WorldBuilder::new("test");

    // Create a visible resource
    let visible = builder
        .create_resource("Public Location", None, None, Some("Everyone can see this."), false, Vec::new(), Vec::new())
        .unwrap();

    // Create a hidden resource
    let hidden = builder
        .create_resource("Secret Lair", None, None, Some("Only the DM knows."), true, Vec::new(), Vec::new())
        .unwrap();

    // Export and verify
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("test.lk");
    builder
        .export_world(Some(output.to_str().unwrap()))
        .unwrap();

    let root = read_lk_file(&output).unwrap();

    let public = root.resources.iter().find(|r| r.id == visible.id).unwrap();
    assert!(!public.is_hidden);

    let secret = root.resources.iter().find(|r| r.id == hidden.id).unwrap();
    assert!(secret.is_hidden);
}

#[test]
fn hidden_document_persists_through_export() {
    let mut builder = WorldBuilder::new("test");

    let res = builder
        .create_resource("Tavern", None, None, Some("A cozy tavern."), false, Vec::new(), Vec::new())
        .unwrap();

    // Add a hidden document
    builder
        .add_document(&res.id, "DM Notes", "Secret passage behind the bar.", None, true)
        .unwrap();

    // Add a visible document
    builder
        .add_document(&res.id, "Menu", "Ale: 5cp, Stew: 1sp", None, false)
        .unwrap();

    // Export and verify
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("test.lk");
    builder
        .export_world(Some(output.to_str().unwrap()))
        .unwrap();

    let root = read_lk_file(&output).unwrap();
    let tavern = root.resources.iter().find(|r| r.id == res.id).unwrap();

    assert_eq!(tavern.documents.len(), 3); // Main + DM Notes + Menu

    let dm_notes = tavern.documents.iter().find(|d| d.name == "DM Notes").unwrap();
    assert!(dm_notes.is_hidden);

    let menu = tavern.documents.iter().find(|d| d.name == "Menu").unwrap();
    assert!(!menu.is_hidden);

    let main = tavern.documents.iter().find(|d| d.name == "Main").unwrap();
    assert!(!main.is_hidden);
}

#[test]
fn template_properties_applied_to_resource() {
    // Load a reference world to get templates
    use legend_keeper_mcp::lk::store::WorldStore;
    use std::path::Path;

    let store = WorldStore::load(Path::new("tests/reference")).unwrap();
    let world = Some("siqram".to_string());
    let templates = store.list_templates(&world).unwrap();

    // Find the NPC template
    let npc_template = templates.iter().find(|t| t.name == "NPC").expect("NPC template should exist");
    assert!(!npc_template.properties.is_empty(), "NPC template should have properties");

    // Get the full properties for cloning
    let (props, template_tags) = store.get_template_properties(&world, "NPC").unwrap();
    assert!(!props.is_empty());
    assert!(template_tags.contains(&"npc".to_string()));

    // Create a resource with these properties
    let mut builder = WorldBuilder::new("test");
    let explicit_tags = vec!["custom-tag".to_string()];

    // Merge tags like the server does
    let mut tags = explicit_tags.clone();
    for t in &template_tags {
        if !tags.iter().any(|existing| existing.eq_ignore_ascii_case(t)) {
            tags.push(t.clone());
        }
    }

    let res = builder
        .create_resource(
            "Test NPC",
            None,
            Some(tags),
            Some("A brave warrior."),
            false,
            vec!["The Brave".to_string()],
            props,
        )
        .unwrap();

    // Export and verify
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("test.lk");
    builder
        .export_world(Some(output.to_str().unwrap()))
        .unwrap();

    let root = read_lk_file(&output).unwrap();
    let npc = root.resources.iter().find(|r| r.id == res.id).unwrap();

    // Verify aliases
    assert_eq!(npc.aliases, vec!["The Brave"]);

    // Verify tags merged
    assert!(npc.tags.contains(&"custom-tag".to_string()));
    assert!(npc.tags.contains(&"npc".to_string()));

    // Verify properties were copied
    assert!(!npc.properties.is_empty());
    let prop_titles: Vec<&str> = npc.properties.iter().map(|p| p.title.as_str()).collect();
    // NPC template should have IMAGE and TAGS at minimum
    assert!(prop_titles.contains(&"IMAGE") || prop_titles.contains(&"TAGS"),
        "Expected template properties, got: {:?}", prop_titles);
}

#[test]
fn template_extraction_from_reference_worlds() {
    use legend_keeper_mcp::lk::store::WorldStore;
    use std::path::Path;

    let store = WorldStore::load(Path::new("tests/reference")).unwrap();

    // Test against both worlds
    let worlds = store.list_worlds();
    for world in &worlds {
        let templates = store.list_templates(&Some(world.name.clone())).unwrap();
        assert!(!templates.is_empty(), "World '{}' should have templates", world.name);

        // Common templates should exist
        let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"NPC"), "World '{}' should have NPC template", world.name);
        assert!(names.contains(&"Location"), "World '{}' should have Location template", world.name);
        assert!(names.contains(&"Character"), "World '{}' should have Character template", world.name);

        // Each template should have at least one property
        for t in &templates {
            assert!(!t.properties.is_empty(), "Template '{}' should have properties", t.name);
        }
    }
}
