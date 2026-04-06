use std::collections::HashSet;

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
            None,
            None,
            None,
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
            None,
            None,
            None,
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
            None,
            None,
            None,
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
        .create_resource("Tavern", None, None, Some("A cozy tavern."), false, Vec::new(), Vec::new(), None, None, None)
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
        .create_resource("Place", None, None, Some("Original content."), false, Vec::new(), Vec::new(), None, None, None)
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
    let result = builder.create_resource("Child", Some("nonexistent"), None, None, false, Vec::new(), Vec::new(), None, None, None);
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
        .create_resource("Gandalf", None, None, None, false, Vec::new(), Vec::new(), None, None, None)
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
            None,
            None,
            None,
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
fn resource_hidden_by_default_when_omitted() {
    let mut builder = WorldBuilder::new("test");

    // Create a resource without specifying is_hidden (the server passes unwrap_or(true))
    let defaulted = builder
        .create_resource("Default Resource", None, None, Some("No is_hidden specified."), true, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Create a resource with explicit is_hidden: false
    let explicit_visible = builder
        .create_resource("Visible Resource", None, None, Some("Explicitly visible."), false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("test.lk");
    builder.export_world(Some(output.to_str().unwrap())).unwrap();

    let root = read_lk_file(&output).unwrap();

    let defaulted_r = root.resources.iter().find(|r| r.id == defaulted.id).unwrap();
    assert!(defaulted_r.is_hidden, "Resource with is_hidden=true (server default) should be hidden");

    let visible_r = root.resources.iter().find(|r| r.id == explicit_visible.id).unwrap();
    assert!(!visible_r.is_hidden, "Resource with explicit is_hidden=false should be visible");
}

#[test]
fn hidden_resource_persists_through_export() {
    let mut builder = WorldBuilder::new("test");

    // Create a visible resource
    let visible = builder
        .create_resource("Public Location", None, None, Some("Everyone can see this."), false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Create a hidden resource
    let hidden = builder
        .create_resource("Secret Lair", None, None, Some("Only the DM knows."), true, Vec::new(), Vec::new(), None, None, None)
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
        .create_resource("Tavern", None, None, Some("A cozy tavern."), false, Vec::new(), Vec::new(), None, None, None)
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
fn board_document_validation_rejects_missing_id() {
    let mut builder = WorldBuilder::new("test");
    let res = builder
        .create_resource("Board Test", None, None, None, false, vec![], vec![], None, None, None)
        .unwrap();

    // Missing 'id' field on binding
    let bad_board = r#"{"shapes":[],"bindings":[],"shapesV2":[
        {"key":"document:document","val":{"id":"document:document","typeName":"document","gridSize":10,"name":"","meta":{}}},
        {"key":"page:page","val":{"id":"page:page","typeName":"page","name":"Page 1","index":"a1","meta":{}}},
        {"key":"binding:b1","val":{"typeName":"binding","type":"arrow","fromId":"shape:a1","toId":"shape:g1","meta":{},"props":{"terminal":"start","normalizedAnchor":{"x":0.5,"y":0.5},"isPrecise":false,"isExact":false}}}
    ]}"#;

    let err = builder.add_document(&res.id, "Board", bad_board, Some("board"), false);
    assert!(err.is_err(), "Should reject binding without id");
    let msg = err.unwrap_err().to_string();
    assert!(msg.contains("missing required field 'id'"), "Error should mention 'id', got: {}", msg);
}

#[test]
fn board_document_validation_rejects_missing_scale() {
    let mut builder = WorldBuilder::new("test");
    let res = builder
        .create_resource("Board Test", None, None, None, false, vec![], vec![], None, None, None)
        .unwrap();

    // Shape missing 'scale' in props
    let bad_board = r#"{"shapes":[],"bindings":[],"shapesV2":[
        {"key":"document:document","val":{"id":"document:document","typeName":"document","gridSize":10,"name":"","meta":{}}},
        {"key":"page:page","val":{"id":"page:page","typeName":"page","name":"Page 1","index":"a1","meta":{}}},
        {"key":"shape:g1","val":{"id":"shape:g1","typeName":"shape","type":"geo","x":0,"y":0,"rotation":0,"isLocked":false,"opacity":1,"meta":{},"parentId":"page:page","index":"a1","props":{"w":100,"h":100,"geo":"rectangle","color":"blue","fill":"none","text":"Test"}}}
    ]}"#;

    let err = builder.add_document(&res.id, "Board", bad_board, Some("board"), false);
    assert!(err.is_err(), "Should reject shape without scale in props");
    let msg = err.unwrap_err().to_string();
    assert!(msg.contains("scale"), "Error should mention 'scale', got: {}", msg);
}

#[test]
fn board_document_validation_accepts_valid_board() {
    let mut builder = WorldBuilder::new("test");
    let res = builder
        .create_resource("Board Test", None, None, None, false, vec![], vec![], None, None, None)
        .unwrap();

    let good_board = r#"{"shapes":[],"bindings":[],"shapesV2":[
        {"key":"document:document","val":{"id":"document:document","typeName":"document","gridSize":10,"name":"","meta":{}}},
        {"key":"page:page","val":{"id":"page:page","typeName":"page","name":"Page 1","index":"a1","meta":{}}},
        {"key":"shape:g1","val":{"id":"shape:g1","typeName":"shape","type":"geo","x":0,"y":0,"rotation":0,"isLocked":false,"opacity":1,"meta":{},"parentId":"page:page","index":"a1","props":{"w":100,"h":100,"geo":"rectangle","color":"blue","fill":"none","dash":"draw","size":"m","font":"draw","text":"Tavern","align":"middle","verticalAlign":"middle","growY":0,"url":"","labelColor":"black","scale":1}}},
        {"key":"binding:b1","val":{"id":"binding:b1","typeName":"binding","type":"arrow","fromId":"shape:a1","toId":"shape:g1","meta":{},"props":{"terminal":"start","normalizedAnchor":{"x":0.5,"y":0.5},"isPrecise":false,"isExact":false}}}
    ]}"#;

    let result = builder.add_document(&res.id, "Board", good_board, Some("board"), false);
    assert!(result.is_ok(), "Should accept valid board: {:?}", result.err());
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
    let (props, template_tags, icon) = store.get_template_properties(&world, "NPC").unwrap();
    assert!(!props.is_empty());
    assert!(template_tags.contains(&"npc".to_string()));

    // Create a resource with these properties and icon
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
            icon.icon_color,
            icon.icon_glyph,
            icon.icon_shape,
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
fn delete_resource_removes_single() {
    let mut builder = WorldBuilder::new("test");

    let root = builder
        .create_resource("Root", None, None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();
    let child = builder
        .create_resource("Child", Some(&root.id), None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Can't delete root without recursive because it has a child
    let err = builder.delete_resource(&root.id, false);
    assert!(err.is_err());

    // Can delete the leaf child
    let deleted = builder.delete_resource(&child.id, false).unwrap();
    assert_eq!(deleted.len(), 1);
    assert!(deleted.contains(&child.id));

    let summary = builder.list_draft();
    assert_eq!(summary.resource_count, 1);
    assert_eq!(summary.resources[0].name, "Root");
}

#[test]
fn delete_resource_recursive() {
    let mut builder = WorldBuilder::new("test");

    let root = builder
        .create_resource("Root", None, None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();
    let child = builder
        .create_resource("Child", Some(&root.id), None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();
    let grandchild = builder
        .create_resource("Grandchild", Some(&child.id), None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Recursive delete removes the whole subtree
    let deleted = builder.delete_resource(&root.id, true).unwrap();
    assert_eq!(deleted.len(), 3);
    let deleted_set: HashSet<&str> = deleted.iter().map(|s| s.as_str()).collect();
    assert!(deleted_set.contains(root.id.as_str()));
    assert!(deleted_set.contains(child.id.as_str()));
    assert!(deleted_set.contains(grandchild.id.as_str()));

    let summary = builder.list_draft();
    assert_eq!(summary.resource_count, 0);
}

#[test]
fn delete_nonexistent_resource_errors() {
    let mut builder = WorldBuilder::new("test");
    let err = builder.delete_resource("nonexistent", false);
    assert!(err.is_err());
}

#[test]
fn reparent_resource_basic() {
    let mut builder = WorldBuilder::new("test");

    let a = builder
        .create_resource("A", None, None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();
    let b = builder
        .create_resource("B", None, None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();
    let child = builder
        .create_resource("Child", Some(&a.id), None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Move child from A to B
    builder.reparent_resource(&child.id, Some(&b.id)).unwrap();

    let summary = builder.list_draft();
    let moved = summary.resources.iter().find(|r| r.id == child.id).unwrap();
    assert_eq!(moved.parent_id.as_deref(), Some(b.id.as_str()));
}

#[test]
fn reparent_to_top_level() {
    let mut builder = WorldBuilder::new("test");

    let parent = builder
        .create_resource("Parent", None, None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();
    let child = builder
        .create_resource("Child", Some(&parent.id), None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Move child to top level
    builder.reparent_resource(&child.id, None).unwrap();

    let summary = builder.list_draft();
    let moved = summary.resources.iter().find(|r| r.id == child.id).unwrap();
    assert!(moved.parent_id.is_none());
}

#[test]
fn reparent_prevents_circular_reference() {
    let mut builder = WorldBuilder::new("test");

    let parent = builder
        .create_resource("Parent", None, None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();
    let child = builder
        .create_resource("Child", Some(&parent.id), None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Can't parent Parent under its own Child
    let err = builder.reparent_resource(&parent.id, Some(&child.id));
    assert!(err.is_err());
}

#[test]
fn reparent_prevents_self_reference() {
    let mut builder = WorldBuilder::new("test");

    let a = builder
        .create_resource("A", None, None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    let err = builder.reparent_resource(&a.id, Some(&a.id));
    assert!(err.is_err());
}

#[test]
fn reparent_nonexistent_resource_errors() {
    let mut builder = WorldBuilder::new("test");

    let a = builder
        .create_resource("A", None, None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Nonexistent resource ID
    let err = builder.reparent_resource("nonexistent", Some(&a.id));
    assert!(err.is_err());

    // Nonexistent target parent
    let err = builder.reparent_resource(&a.id, Some("nonexistent"));
    assert!(err.is_err());
}

#[test]
fn delete_recursive_on_leaf_succeeds() {
    let mut builder = WorldBuilder::new("test");

    let leaf = builder
        .create_resource("Leaf", None, None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // recursive=true on a leaf should succeed, deleting just the leaf
    let deleted = builder.delete_resource(&leaf.id, true).unwrap();
    assert_eq!(deleted.len(), 1);
    assert!(deleted.contains(&leaf.id));

    let summary = builder.list_draft();
    assert_eq!(summary.resource_count, 0);
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

// --- New tests for draft editing tools ---

#[test]
fn get_draft_resource_by_id_and_name() {
    let mut builder = WorldBuilder::new("test");

    let res = builder
        .create_resource("Captain Reef", None, Some(vec!["npc".to_string()]), Some("A pirate captain."), false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Lookup by ID
    let found = builder.get_draft_resource(&res.id).unwrap();
    assert_eq!(found.name, "Captain Reef");

    // Lookup by name (case-insensitive)
    let found = builder.get_draft_resource("captain reef").unwrap();
    assert_eq!(found.id, res.id);

    // Not found
    let err = builder.get_draft_resource("nonexistent");
    assert!(err.is_err());
}

#[test]
fn get_draft_document_default_and_by_id() {
    let mut builder = WorldBuilder::new("test");

    let res = builder
        .create_resource("Tavern", None, None, Some("A cozy tavern."), false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    let doc2 = builder
        .add_document(&res.id, "DM Notes", "Secret info.", None, true)
        .unwrap();

    // Default (first page doc)
    let doc = builder.get_draft_document(&res.id, None).unwrap();
    assert_eq!(doc.name, "Main");

    // By ID
    let doc = builder.get_draft_document(&res.id, Some(&doc2.id)).unwrap();
    assert_eq!(doc.name, "DM Notes");
    assert!(doc.is_hidden);

    // Nonexistent document
    let err = builder.get_draft_document(&res.id, Some("nonexistent"));
    assert!(err.is_err());

    // Nonexistent resource
    let err = builder.get_draft_document("nonexistent", None);
    assert!(err.is_err());
}

#[test]
fn update_resource_metadata() {
    let mut builder = WorldBuilder::new("test");

    let res = builder
        .create_resource("Old Name", None, Some(vec!["tag1".to_string()]), None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Update name and tags
    let updated = builder.update_resource(&res.id, Some("New Name"), Some(vec!["tag2".to_string(), "tag3".to_string()]), None, None).unwrap();
    assert_eq!(updated.name, "New Name");
    assert_eq!(updated.tags, vec!["tag2", "tag3"]);

    // Update visibility
    let updated = builder.update_resource(&res.id, None, None, Some(true), None).unwrap();
    assert_eq!(updated.name, "New Name"); // unchanged

    // Update aliases
    let updated = builder.update_resource(&res.id, None, None, None, Some(vec!["Alias1".to_string()])).unwrap();
    assert_eq!(updated.name, "New Name"); // unchanged

    // All-None is a no-op
    let updated = builder.update_resource(&res.id, None, None, None, None).unwrap();
    assert_eq!(updated.name, "New Name");

    // Nonexistent resource
    let err = builder.update_resource("nonexistent", Some("X"), None, None, None);
    assert!(err.is_err());
}

#[test]
fn delete_document_from_resource() {
    let mut builder = WorldBuilder::new("test");

    let res = builder
        .create_resource("Place", None, None, Some("Main content."), false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    let doc2 = builder
        .add_document(&res.id, "History", "Long ago...", None, false)
        .unwrap();

    // Delete the second document
    let name = builder.delete_document(&res.id, &doc2.id).unwrap();
    assert_eq!(name, "History");

    // Verify only 1 document remains
    let summary = builder.list_draft();
    let place = summary.resources.iter().find(|r| r.id == res.id).unwrap();
    assert_eq!(place.document_count, 1);
}

#[test]
fn delete_last_document_fails() {
    let mut builder = WorldBuilder::new("test");

    let res = builder
        .create_resource("Place", None, None, Some("Content."), false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Get the main doc's ID
    let doc = builder.get_draft_document(&res.id, None).unwrap();
    let doc_id = doc.id.clone();

    // Can't delete the last document
    let err = builder.delete_document(&res.id, &doc_id);
    assert!(err.is_err());
}

#[test]
fn delete_document_nonexistent_errors() {
    let mut builder = WorldBuilder::new("test");

    let res = builder
        .create_resource("Place", None, None, None, false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Add a second doc so we can attempt deletion
    builder.add_document(&res.id, "Extra", "content", None, false).unwrap();

    // Nonexistent document ID
    let err = builder.delete_document(&res.id, "nonexistent");
    assert!(err.is_err());

    // Nonexistent resource ID
    let err = builder.delete_document("nonexistent", "whatever");
    assert!(err.is_err());
}

#[test]
fn template_icon_inheritance() {
    use legend_keeper_mcp::lk::store::WorldStore;
    use std::path::Path;

    let store = WorldStore::load(Path::new("tests/reference")).unwrap();
    let world = Some("siqram".to_string());

    let (props, _tags, icon) = store.get_template_properties(&world, "NPC").unwrap();

    let mut builder = WorldBuilder::new("test");
    let res = builder
        .create_resource(
            "Test NPC",
            None,
            Some(vec!["npc".to_string()]),
            Some("A test NPC."),
            false,
            Vec::new(),
            props,
            icon.icon_color.clone(),
            icon.icon_glyph.clone(),
            icon.icon_shape.clone(),
        )
        .unwrap();

    // Export and verify icon fields were set
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("test.lk");
    builder.export_world(Some(output.to_str().unwrap())).unwrap();

    let root = read_lk_file(&output).unwrap();
    let npc = root.resources.iter().find(|r| r.id == res.id).unwrap();

    // Icon fields should match what the template had
    assert_eq!(npc.icon_color, icon.icon_color);
    assert_eq!(npc.icon_glyph, icon.icon_glyph);
    assert_eq!(npc.icon_shape, icon.icon_shape);
}

// --- Tests for from_lk_root (load draft) ---

#[test]
fn from_lk_root_preserves_resources() {
    // Create a world, export it, read it back, then load into a new builder
    let mut builder = WorldBuilder::new("roundtrip");

    let root_res = builder
        .create_resource("Kingdom", None, Some(vec!["location".to_string()]), Some("A great kingdom."), false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();
    let child_res = builder
        .create_resource("Capital", Some(&root_res.id), Some(vec!["city".to_string()]), Some("The capital city."), false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();

    // Export
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("roundtrip.lk");
    builder.export_world(Some(output.to_str().unwrap())).unwrap();

    // Read back and load into a new builder
    let lk_root = read_lk_file(&output).unwrap();
    let loaded = WorldBuilder::from_lk_root("roundtrip".to_string(), lk_root);

    // Verify resources are present
    let summary = loaded.list_draft();
    assert_eq!(summary.name, "roundtrip");
    assert_eq!(summary.resource_count, 2);

    let names: Vec<&str> = summary.resources.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Kingdom"));
    assert!(names.contains(&"Capital"));

    // Verify IDs are preserved
    let kingdom = summary.resources.iter().find(|r| r.name == "Kingdom").unwrap();
    assert_eq!(kingdom.id, root_res.id);
    let capital = summary.resources.iter().find(|r| r.name == "Capital").unwrap();
    assert_eq!(capital.id, child_res.id);
    assert_eq!(capital.parent_id.as_deref(), Some(root_res.id.as_str()));
}

#[test]
fn from_lk_root_allows_editing() {
    // Load a reference world and verify we can edit it
    let lk_root = read_lk_file(std::path::Path::new("tests/reference/rime.lk")).unwrap();
    let original_count = lk_root.resources.len();

    let mut loaded = WorldBuilder::from_lk_root("rime".to_string(), lk_root);

    // Add a new resource
    let new_res = loaded
        .create_resource("New Location", None, Some(vec!["location".to_string()]), Some("A newly added place."), false, Vec::new(), Vec::new(), None, None, None)
        .unwrap();
    assert!(!new_res.id.is_empty());

    // Verify count increased
    let summary = loaded.list_draft();
    assert_eq!(summary.resource_count, original_count + 1);

    // Export and verify the new resource is included
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("rime-edited.lk");
    loaded.export_world(Some(output.to_str().unwrap())).unwrap();

    let exported = read_lk_file(&output).unwrap();
    assert_eq!(exported.resources.len(), original_count + 1);
    assert!(exported.resources.iter().any(|r| r.name == "New Location"));
}

#[test]
fn from_lk_root_set_content_on_loaded_resource() {
    let lk_root = read_lk_file(std::path::Path::new("tests/reference/rime.lk")).unwrap();
    let first_resource_id = lk_root.resources[0].id.clone();

    let mut loaded = WorldBuilder::from_lk_root("rime".to_string(), lk_root);

    // Update content on an existing resource
    loaded
        .set_content(&first_resource_id, None, "# Updated Content\n\nThis was edited after loading.")
        .unwrap();

    // Export and verify
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("rime-updated.lk");
    loaded.export_world(Some(output.to_str().unwrap())).unwrap();

    let exported = read_lk_file(&output).unwrap();
    let resource = exported.resources.iter().find(|r| r.id == first_resource_id).unwrap();
    let content = resource.documents[0].content.as_ref().unwrap();
    let content_str = serde_json::to_string(content).unwrap();
    assert!(content_str.contains("Updated Content"));
}
