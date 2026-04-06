use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::schema::{BoardContent, BoardRecord};
use super::LkError;

// --- Public types ---

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphNode {
    /// Unique node identifier (referenced by edges).
    pub id: String,
    /// Display label for the node.
    pub label: String,
    /// Node category — determines shape and default color.
    /// "location" = pentagon/blue, "person" = rectangle/green,
    /// "organization" = rectangle/violet, "event" = diamond/orange,
    /// "activity" = rectangle/grey.
    pub node_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphEdge {
    /// Source node ID.
    pub from: String,
    /// Target node ID.
    pub to: String,
    /// Optional edge label (e.g. clue description).
    pub label: Option<String>,
}

// --- Layout constants ---

const NODE_WIDTH: f64 = 200.0;
const NODE_HEIGHT: f64 = 100.0;
const COLUMN_SPACING: f64 = 400.0;
const ROW_SPACING: f64 = 200.0;
const TITLE_SCALE: f64 = 3.0;

const VALID_NODE_TYPES: &[&str] = &["location", "person", "organization", "event", "activity"];

/// Column ordering for deterministic layout.
const COLUMN_ORDER: &[&str] = &["location", "person", "organization", "event", "activity"];

// --- Node type → tldraw shape mapping ---

fn geo_for_type(node_type: &str) -> &'static str {
    match node_type {
        "location" => "pentagon",
        "event" => "diamond",
        _ => "rectangle",
    }
}

fn color_for_type(node_type: &str) -> &'static str {
    match node_type {
        "location" => "blue",
        "person" => "green",
        "organization" => "violet",
        "event" => "orange",
        "activity" => "grey",
        _ => "black",
    }
}

// --- Validation ---

pub fn validate_graph(nodes: &[GraphNode], edges: &[GraphEdge]) -> Result<(), LkError> {
    if nodes.is_empty() {
        return Err(LkError::InvalidInput(
            "At least one node is required".to_string(),
        ));
    }

    let mut seen_ids = HashMap::new();
    for node in nodes {
        if let Some(_) = seen_ids.insert(&node.id, ()) {
            return Err(LkError::InvalidInput(format!(
                "Duplicate node ID: {}",
                node.id
            )));
        }
        if !VALID_NODE_TYPES.contains(&node.node_type.as_str()) {
            return Err(LkError::InvalidInput(format!(
                "Unknown node_type: {}. Must be one of: location, person, organization, event, activity",
                node.node_type
            )));
        }
    }

    for edge in edges {
        if !seen_ids.contains_key(&edge.from) {
            return Err(LkError::InvalidInput(format!(
                "Edge references unknown node: {}",
                edge.from
            )));
        }
        if !seen_ids.contains_key(&edge.to) {
            return Err(LkError::InvalidInput(format!(
                "Edge references unknown node: {}",
                edge.to
            )));
        }
    }

    Ok(())
}

// --- Generation ---

/// Convert a high-level graph spec into tldraw BoardContent.
/// Handles layout, record generation, and serialization in a single pass.
/// Nodes are arranged in columns by node_type, spaced with fixed constants.
pub fn graph_to_board_content(nodes: &[GraphNode], edges: &[GraphEdge]) -> BoardContent {
    let mut records: Vec<BoardRecord> = Vec::new();

    // Boilerplate: document + page records
    records.push(BoardRecord {
        key: "document:document".to_string(),
        val: json!({
            "gridSize": 10,
            "name": "",
            "meta": {},
            "id": "document:document",
            "typeName": "document"
        }),
    });
    records.push(BoardRecord {
        key: "page:page".to_string(),
        val: json!({
            "meta": {},
            "id": "page:page",
            "name": "Page 1",
            "index": "a1",
            "typeName": "page"
        }),
    });

    // Group nodes by type, maintaining column order
    let mut columns: Vec<(&str, Vec<&GraphNode>)> = Vec::new();
    for &col_type in COLUMN_ORDER {
        let col_nodes: Vec<&GraphNode> = nodes.iter().filter(|n| n.node_type == col_type).collect();
        if !col_nodes.is_empty() {
            columns.push((col_type, col_nodes));
        }
    }

    // Track node positions for arrow generation
    let mut node_positions: HashMap<&str, (f64, f64)> = HashMap::new();

    // Column header offset: title sits above the first node row
    let title_y_offset = -80.0;

    for (col_idx, (col_type, col_nodes)) in columns.iter().enumerate() {
        let col_x = col_idx as f64 * COLUMN_SPACING;

        // Column header (text shape)
        let title_id = format!("shape:title_{}", col_type);
        records.push(BoardRecord {
            key: title_id.clone(),
            val: json!({
                "id": title_id,
                "type": "text",
                "typeName": "shape",
                "x": col_x,
                "y": title_y_offset,
                "rotation": 0,
                "isLocked": false,
                "opacity": 1,
                "meta": {},
                "parentId": "page:page",
                "index": format!("a{}", col_idx),
                "props": {
                    "color": color_for_type(col_type),
                    "size": "m",
                    "w": NODE_WIDTH,
                    "text": *col_type,
                    "font": "draw",
                    "textAlign": "start",
                    "autoSize": true,
                    "scale": TITLE_SCALE,
                }
            }),
        });

        // Geo shapes for each node in the column
        for (row_idx, node) in col_nodes.iter().enumerate() {
            let x = col_x;
            let y = row_idx as f64 * ROW_SPACING;
            node_positions.insert(&node.id, (x, y));

            let shape_id = format!("shape:{}", node.id);
            records.push(BoardRecord {
                key: shape_id.clone(),
                val: json!({
                    "id": shape_id,
                    "type": "geo",
                    "typeName": "shape",
                    "x": x,
                    "y": y,
                    "rotation": 0,
                    "isLocked": false,
                    "opacity": 1,
                    "meta": {},
                    "parentId": "page:page",
                    "index": format!("b{}_{}", col_idx, row_idx),
                    "props": {
                        "geo": geo_for_type(&node.node_type),
                        "w": NODE_WIDTH,
                        "h": NODE_HEIGHT,
                        "dash": "draw",
                        "size": "m",
                        "color": color_for_type(&node.node_type),
                        "fill": "semi",
                        "text": node.label,
                        "font": "draw",
                        "textAlign": "middle",
                        "verticalAlign": "middle",
                        "growY": 0,
                        "url": "",
                        "scale": 1,
                    }
                }),
            });
        }
    }

    // Arrow shapes and bindings for edges
    for (edge_idx, edge) in edges.iter().enumerate() {
        let arrow_id = format!("shape:arrow_{}", edge_idx);
        let from_shape_id = format!("shape:{}", edge.from);
        let to_shape_id = format!("shape:{}", edge.to);

        // Get positions for arrow start/end hints
        let (from_x, from_y) = node_positions
            .get(edge.from.as_str())
            .copied()
            .unwrap_or((0.0, 0.0));
        let (to_x, to_y) = node_positions
            .get(edge.to.as_str())
            .copied()
            .unwrap_or((0.0, 0.0));

        let label_text = edge
            .label
            .as_deref()
            .map(|l| if l.len() > 40 { &l[..40] } else { l })
            .unwrap_or("");

        records.push(BoardRecord {
            key: arrow_id.clone(),
            val: json!({
                "id": arrow_id,
                "type": "arrow",
                "typeName": "shape",
                "x": from_x + NODE_WIDTH / 2.0,
                "y": from_y + NODE_HEIGHT / 2.0,
                "rotation": 0,
                "isLocked": false,
                "opacity": 1,
                "meta": {},
                "parentId": "page:page",
                "index": format!("c{}", edge_idx),
                "props": {
                    "dash": "draw",
                    "size": "m",
                    "fill": "none",
                    "color": "black",
                    "labelColor": "black",
                    "bend": 0,
                    "start": {
                        "x": 0,
                        "y": 0,
                    },
                    "end": {
                        "x": to_x - from_x,
                        "y": to_y - from_y,
                    },
                    "arrowheadStart": "none",
                    "arrowheadEnd": "arrow",
                    "text": label_text,
                    "labelPosition": 0.5,
                    "font": "draw",
                    "scale": 1,
                }
            }),
        });

        // Binding: arrow start → source node
        let start_binding_id = format!("binding:arrow_{}_start", edge_idx);
        records.push(BoardRecord {
            key: start_binding_id.clone(),
            val: json!({
                "id": start_binding_id,
                "type": "arrow",
                "typeName": "binding",
                "fromId": arrow_id,
                "toId": from_shape_id,
                "meta": {},
                "props": {
                    "isPrecise": false,
                    "isExact": false,
                    "normalizedAnchor": { "x": 0.5, "y": 0.5 },
                    "terminal": "start",
                }
            }),
        });

        // Binding: arrow end → target node
        let end_binding_id = format!("binding:arrow_{}_end", edge_idx);
        records.push(BoardRecord {
            key: end_binding_id.clone(),
            val: json!({
                "id": end_binding_id,
                "type": "arrow",
                "typeName": "binding",
                "fromId": arrow_id,
                "toId": to_shape_id,
                "meta": {},
                "props": {
                    "isPrecise": false,
                    "isExact": false,
                    "normalizedAnchor": { "x": 0.5, "y": 0.5 },
                    "terminal": "end",
                }
            }),
        });
    }

    BoardContent {
        shapes: Vec::new(),
        bindings: Vec::new(),
        shapes_v2: records,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_nodes() {
        let result = validate_graph(&[], &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("At least one node"));
    }

    #[test]
    fn test_validate_duplicate_ids() {
        let nodes = vec![
            GraphNode { id: "a".into(), label: "A".into(), node_type: "person".into() },
            GraphNode { id: "a".into(), label: "B".into(), node_type: "person".into() },
        ];
        let result = validate_graph(&nodes, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate node ID: a"));
    }

    #[test]
    fn test_validate_unknown_node_type() {
        let nodes = vec![
            GraphNode { id: "a".into(), label: "A".into(), node_type: "dragon".into() },
        ];
        let result = validate_graph(&nodes, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown node_type: dragon"));
    }

    #[test]
    fn test_validate_bad_edge_from() {
        let nodes = vec![
            GraphNode { id: "a".into(), label: "A".into(), node_type: "person".into() },
        ];
        let edges = vec![
            GraphEdge { from: "missing".into(), to: "a".into(), label: None },
        ];
        let result = validate_graph(&nodes, &edges);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Edge references unknown node: missing"));
    }

    #[test]
    fn test_validate_bad_edge_to() {
        let nodes = vec![
            GraphNode { id: "a".into(), label: "A".into(), node_type: "person".into() },
        ];
        let edges = vec![
            GraphEdge { from: "a".into(), to: "missing".into(), label: None },
        ];
        let result = validate_graph(&nodes, &edges);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Edge references unknown node: missing"));
    }

    #[test]
    fn test_valid_graph() {
        let nodes = vec![
            GraphNode { id: "a".into(), label: "A".into(), node_type: "person".into() },
            GraphNode { id: "b".into(), label: "B".into(), node_type: "location".into() },
        ];
        let edges = vec![
            GraphEdge { from: "a".into(), to: "b".into(), label: Some("goes to".into()) },
        ];
        assert!(validate_graph(&nodes, &edges).is_ok());
    }

    #[test]
    fn test_graph_to_board_content_structure() {
        let nodes = vec![
            GraphNode { id: "npc1".into(), label: "Guard".into(), node_type: "person".into() },
            GraphNode { id: "loc1".into(), label: "Tavern".into(), node_type: "location".into() },
        ];
        let edges = vec![
            GraphEdge { from: "npc1".into(), to: "loc1".into(), label: Some("works at".into()) },
        ];

        let board = graph_to_board_content(&nodes, &edges);

        // document + page + 2 column headers + 2 geo shapes + 1 arrow + 2 bindings = 9 records
        assert_eq!(board.shapes_v2.len(), 9);
        assert!(board.shapes.is_empty());
        assert!(board.bindings.is_empty());

        // Check that all shapes have required fields for validation
        for record in &board.shapes_v2 {
            let val = &record.val;
            assert!(val.get("id").is_some(), "record missing id: {}", record.key);
            assert!(val.get("typeName").is_some(), "record missing typeName: {}", record.key);
            assert!(val.get("meta").is_some(), "record missing meta: {}", record.key);

            let type_name = val["typeName"].as_str().unwrap();
            match type_name {
                "shape" => {
                    assert!(val.get("type").is_some(), "shape missing type: {}", record.key);
                    assert!(val.get("props").is_some(), "shape missing props: {}", record.key);
                    assert!(val.get("parentId").is_some(), "shape missing parentId: {}", record.key);
                    assert!(val.get("index").is_some(), "shape missing index: {}", record.key);
                    assert!(val["props"].get("scale").is_some(), "shape missing props.scale: {}", record.key);
                }
                "binding" => {
                    assert!(val.get("type").is_some(), "binding missing type: {}", record.key);
                    assert!(val.get("fromId").is_some(), "binding missing fromId: {}", record.key);
                    assert!(val.get("toId").is_some(), "binding missing toId: {}", record.key);
                    assert!(val.get("props").is_some(), "binding missing props: {}", record.key);
                }
                "document" | "page" => {}
                other => panic!("unexpected typeName: {}", other),
            }
        }
    }

    #[test]
    fn test_deterministic_layout() {
        let nodes = vec![
            GraphNode { id: "a".into(), label: "A".into(), node_type: "person".into() },
            GraphNode { id: "b".into(), label: "B".into(), node_type: "person".into() },
        ];
        let board1 = graph_to_board_content(&nodes, &[]);
        let board2 = graph_to_board_content(&nodes, &[]);

        let json1 = serde_json::to_string(&board1).unwrap();
        let json2 = serde_json::to_string(&board2).unwrap();
        assert_eq!(json1, json2, "Layout must be deterministic");
    }

    #[test]
    fn test_edge_label_truncation() {
        let nodes = vec![
            GraphNode { id: "a".into(), label: "A".into(), node_type: "person".into() },
            GraphNode { id: "b".into(), label: "B".into(), node_type: "location".into() },
        ];
        let long_label = "x".repeat(100);
        let edges = vec![
            GraphEdge { from: "a".into(), to: "b".into(), label: Some(long_label) },
        ];

        let board = graph_to_board_content(&nodes, &edges);
        let arrow = board.shapes_v2.iter().find(|r| r.key.starts_with("shape:arrow_")).unwrap();
        let text = arrow.val["props"]["text"].as_str().unwrap();
        assert_eq!(text.len(), 40, "Edge labels should be truncated to 40 chars");
    }

    #[test]
    fn test_board_passes_validation() {
        use super::super::builder::validate_board_content_pub;

        let nodes = vec![
            GraphNode { id: "a".into(), label: "A".into(), node_type: "person".into() },
            GraphNode { id: "b".into(), label: "B".into(), node_type: "location".into() },
            GraphNode { id: "c".into(), label: "C".into(), node_type: "event".into() },
        ];
        let edges = vec![
            GraphEdge { from: "a".into(), to: "b".into(), label: Some("clue".into()) },
            GraphEdge { from: "b".into(), to: "c".into(), label: None },
        ];

        let board = graph_to_board_content(&nodes, &edges);
        validate_board_content_pub(&board).expect("Generated board must pass validation");
    }
}
