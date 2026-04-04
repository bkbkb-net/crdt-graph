use crdt_graph::types::bytes;
use crdt_graph::types::simple;
use crdt_graph::types::string;
use crdt_graph::types::{RemoveEdge, RemoveVertex};
use crdt_graph::{UpdateOperation, Uuid};

fn new_id() -> Uuid {
    Uuid::now_v7()
}

// ========================================
// simple::Graph (ID-only)
// ========================================

#[test]
fn simple_graph_basic_operations() {
    let mut g = simple::Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    g.update_operation(UpdateOperation::AddVertex(simple::AddVertex { id: v1 }))
        .unwrap();
    g.update_operation(UpdateOperation::AddVertex(simple::AddVertex { id: v2 }))
        .unwrap();
    g.update_operation(UpdateOperation::AddEdge(simple::AddEdge {
        id: e1,
        source: v1,
        target: v2,
    }))
    .unwrap();

    assert!(g.lookup_vertex(&v1));
    assert!(g.lookup_vertex(&v2));
    assert_eq!(g.generate_petgraph().edge_count(), 1);

    let re1 = new_id();
    let rv1 = new_id();

    g.update_operation(UpdateOperation::RemoveEdge(RemoveEdge {
        id: re1,
        add_edge_id: e1,
    }))
    .unwrap();
    g.update_operation(UpdateOperation::RemoveVertex(RemoveVertex {
        id: rv1,
        add_vertex_id: v1,
    }))
    .unwrap();

    assert!(!g.lookup_vertex(&v1));
    assert_eq!(g.generate_petgraph().node_count(), 1);
}

// ========================================
// bytes::Graph (binary payload)
// ========================================

#[test]
fn bytes_graph_with_payload() {
    let mut g = bytes::Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    g.update_operation(UpdateOperation::AddVertex(bytes::AddVertex {
        id: v1,
        data: Some(vec![0xCA, 0xFE]),
    }))
    .unwrap();
    g.update_operation(UpdateOperation::AddVertex(bytes::AddVertex {
        id: v2,
        data: None,
    }))
    .unwrap();
    g.update_operation(UpdateOperation::AddEdge(bytes::AddEdge {
        id: e1,
        source: v1,
        target: v2,
        data: Some(b"edge-data".to_vec()),
    }))
    .unwrap();

    assert!(g.lookup_vertex(&v1));
    let pg = g.generate_petgraph();
    assert_eq!(pg.node_count(), 2);
    assert_eq!(pg.edge_count(), 1);
}

// ========================================
// string::Graph (String payload)
// ========================================

#[test]
fn string_graph_with_payload() {
    let mut g = string::Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    g.update_operation(UpdateOperation::AddVertex(string::AddVertex {
        id: v1,
        data: Some("Alice".into()),
    }))
    .unwrap();
    g.update_operation(UpdateOperation::AddVertex(string::AddVertex {
        id: v2,
        data: Some("Bob".into()),
    }))
    .unwrap();
    g.update_operation(UpdateOperation::AddEdge(string::AddEdge {
        id: e1,
        source: v1,
        target: v2,
        data: Some("knows".into()),
    }))
    .unwrap();

    assert!(g.lookup_vertex(&v1));
    assert!(g.lookup_vertex(&v2));
    let pg = g.generate_petgraph();
    assert_eq!(pg.node_count(), 2);
    assert_eq!(pg.edge_count(), 1);
}

// ========================================
// Shared RemoveVertex / RemoveEdge across variants
// ========================================

#[test]
fn shared_remove_types_across_graph_variants() {
    let mut g = bytes::Graph::new();

    let v1 = new_id();

    g.update_operation(UpdateOperation::AddVertex(bytes::AddVertex {
        id: v1,
        data: Some(b"data".to_vec()),
    }))
    .unwrap();

    let rv1 = new_id();
    g.update_operation(UpdateOperation::RemoveVertex(RemoveVertex {
        id: rv1,
        add_vertex_id: v1,
    }))
    .unwrap();

    assert!(!g.lookup_vertex(&v1));

    let mut g2 = string::Graph::new();

    let v1b = new_id();

    g2.update_operation(UpdateOperation::AddVertex(string::AddVertex {
        id: v1b,
        data: Some("hello".into()),
    }))
    .unwrap();

    let rv1b = new_id();
    g2.update_operation(UpdateOperation::RemoveVertex(RemoveVertex {
        id: rv1b,
        add_vertex_id: v1b,
    }))
    .unwrap();

    assert!(!g2.lookup_vertex(&v1b));
}

// ========================================
// Default trait
// ========================================

#[test]
fn graph_default_is_empty() {
    let g = simple::Graph::default();
    assert!(g.is_empty());
    assert_eq!(g.vertex_count(), 0);
    assert_eq!(g.edge_count(), 0);
}

// ========================================
// From conversions (.into())
// ========================================

#[test]
fn from_add_vertex_into_operation() {
    let id = new_id();
    let op: simple::Operation = simple::AddVertex { id }.into();
    assert_eq!(op, UpdateOperation::AddVertex(simple::AddVertex { id }));
}

#[test]
fn from_remove_vertex_into_operation() {
    let id = new_id();
    let add_vertex_id = new_id();
    let op: simple::Operation = RemoveVertex { id, add_vertex_id }.into();
    assert_eq!(
        op,
        UpdateOperation::RemoveVertex(RemoveVertex { id, add_vertex_id })
    );
}

#[test]
fn from_add_edge_into_operation() {
    let id = new_id();
    let source = new_id();
    let target = new_id();
    let op: bytes::Operation = bytes::AddEdge {
        id,
        source,
        target,
        data: Some(vec![1, 2, 3]),
    }
    .into();
    assert_eq!(
        op,
        UpdateOperation::AddEdge(bytes::AddEdge {
            id,
            source,
            target,
            data: Some(vec![1, 2, 3]),
        })
    );
}

#[test]
fn from_remove_edge_into_operation() {
    let id = new_id();
    let add_edge_id = new_id();
    let op: string::Operation = RemoveEdge { id, add_edge_id }.into();
    assert_eq!(
        op,
        UpdateOperation::RemoveEdge(RemoveEdge { id, add_edge_id })
    );
}

// ========================================
// PartialEq for UpdateOperation
// ========================================

#[test]
fn update_operation_equality() {
    let id = new_id();
    let a: simple::Operation = UpdateOperation::AddVertex(simple::AddVertex { id });
    let b: simple::Operation = UpdateOperation::AddVertex(simple::AddVertex { id });
    assert_eq!(a, b);

    let c: simple::Operation = UpdateOperation::AddVertex(simple::AddVertex { id: new_id() });
    assert_ne!(a, c);
}

// ========================================
// Query methods: vertex_count, edge_count, is_empty, vertices, edges
// ========================================

#[test]
fn query_methods() {
    let mut g = simple::Graph::new();
    assert!(g.is_empty());

    let v1 = new_id();
    let v2 = new_id();
    let v3 = new_id();
    let e1 = new_id();
    let e2 = new_id();

    g.update_operation(simple::AddVertex { id: v1 }.into())
        .unwrap();
    g.update_operation(simple::AddVertex { id: v2 }.into())
        .unwrap();
    g.update_operation(simple::AddVertex { id: v3 }.into())
        .unwrap();

    assert_eq!(g.vertex_count(), 3);
    assert_eq!(g.edge_count(), 0);
    assert!(!g.is_empty());

    g.update_operation(
        simple::AddEdge {
            id: e1,
            source: v1,
            target: v2,
        }
        .into(),
    )
    .unwrap();
    g.update_operation(
        simple::AddEdge {
            id: e2,
            source: v2,
            target: v3,
        }
        .into(),
    )
    .unwrap();

    assert_eq!(g.edge_count(), 2);

    // Remove one edge, then one vertex
    let re1 = new_id();
    g.update_operation(
        RemoveEdge {
            id: re1,
            add_edge_id: e1,
        }
        .into(),
    )
    .unwrap();
    assert_eq!(g.edge_count(), 1);

    let rv1 = new_id();
    g.update_operation(
        RemoveVertex {
            id: rv1,
            add_vertex_id: v1,
        }
        .into(),
    )
    .unwrap();
    assert_eq!(g.vertex_count(), 2);
}

#[test]
fn vertices_and_edges_iterators() {
    let mut g = simple::Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    g.update_operation(simple::AddVertex { id: v1 }.into())
        .unwrap();
    g.update_operation(simple::AddVertex { id: v2 }.into())
        .unwrap();
    g.update_operation(
        simple::AddEdge {
            id: e1,
            source: v1,
            target: v2,
        }
        .into(),
    )
    .unwrap();

    let vertex_ids: Vec<_> = g.vertices().map(|v| v.id).collect();
    assert_eq!(vertex_ids.len(), 2);
    assert!(vertex_ids.contains(&v1));
    assert!(vertex_ids.contains(&v2));

    let edge_ids: Vec<_> = g.edges().map(|e| e.id).collect();
    assert_eq!(edge_ids, vec![e1]);

    // Remove v1 (after removing its edge)
    let re1 = new_id();
    g.update_operation(
        RemoveEdge {
            id: re1,
            add_edge_id: e1,
        }
        .into(),
    )
    .unwrap();
    let rv1 = new_id();
    g.update_operation(
        RemoveVertex {
            id: rv1,
            add_vertex_id: v1,
        }
        .into(),
    )
    .unwrap();

    let vertex_ids: Vec<_> = g.vertices().map(|v| v.id).collect();
    assert_eq!(vertex_ids, vec![v2]);
    assert_eq!(g.edges().count(), 0);
}

// ========================================
// Hash trait
// ========================================

#[test]
fn types_are_hashable() {
    use std::collections::HashSet;

    let id1 = new_id();
    let id2 = new_id();

    let mut set = HashSet::new();
    set.insert(simple::AddVertex { id: id1 });
    set.insert(simple::AddVertex { id: id2 });
    set.insert(simple::AddVertex { id: id1 }); // duplicate
    assert_eq!(set.len(), 2);

    let mut edge_set = HashSet::new();
    edge_set.insert(RemoveEdge {
        id: id1,
        add_edge_id: id2,
    });
    edge_set.insert(RemoveEdge {
        id: id1,
        add_edge_id: id2,
    }); // duplicate
    assert_eq!(edge_set.len(), 1);
}
