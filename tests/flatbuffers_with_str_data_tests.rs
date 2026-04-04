use crdt_graph::flatbuffers::string as fb;
use crdt_graph::types::string::{self, AddEdge, AddVertex, Graph};
use crdt_graph::types::{RemoveEdge, RemoveVertex};
use crdt_graph::{UpdateOperation, Uuid};

fn new_id() -> Uuid {
    Uuid::now_v7()
}

// ========================================
// Single operation round-trips
// ========================================

#[test]
fn roundtrip_add_vertex_with_string_data() {
    let vid = new_id();
    let op: string::Operation = UpdateOperation::AddVertex(AddVertex {
        id: vid,
        data: Some("Alice".into()),
    });
    let buf = fb::encode_operation(&op);
    let decoded = fb::decode_operation(&buf).unwrap();
    match decoded {
        UpdateOperation::AddVertex(v) => {
            assert_eq!(v.id, vid);
            assert_eq!(v.data.as_deref(), Some("Alice"));
        }
        _ => panic!("expected AddVertex"),
    }
}

#[test]
fn roundtrip_add_vertex_no_string_data() {
    let vid = new_id();
    let op: string::Operation = UpdateOperation::AddVertex(AddVertex {
        id: vid,
        data: None,
    });
    let buf = fb::encode_operation(&op);
    let decoded = fb::decode_operation(&buf).unwrap();
    match decoded {
        UpdateOperation::AddVertex(v) => {
            assert_eq!(v.id, vid);
            assert!(v.data.is_none());
        }
        _ => panic!("expected AddVertex"),
    }
}

#[test]
fn roundtrip_remove_vertex_string_format() {
    let id = new_id();
    let add_vertex_id = new_id();
    let op: string::Operation =
        UpdateOperation::RemoveVertex(RemoveVertex { id, add_vertex_id });
    let buf = fb::encode_operation(&op);
    let decoded = fb::decode_operation(&buf).unwrap();
    match decoded {
        UpdateOperation::RemoveVertex(v) => {
            assert_eq!(v.id, id);
            assert_eq!(v.add_vertex_id, add_vertex_id);
        }
        _ => panic!("expected RemoveVertex"),
    }
}

#[test]
fn roundtrip_add_edge_with_string_data() {
    let id = new_id();
    let source = new_id();
    let target = new_id();
    let op: string::Operation = UpdateOperation::AddEdge(AddEdge {
        id,
        source,
        target,
        data: Some(r#"{"weight": 42}"#.into()),
    });
    let buf = fb::encode_operation(&op);
    let decoded = fb::decode_operation(&buf).unwrap();
    match decoded {
        UpdateOperation::AddEdge(e) => {
            assert_eq!(e.id, id);
            assert_eq!(e.source, source);
            assert_eq!(e.target, target);
            assert_eq!(e.data.as_deref(), Some(r#"{"weight": 42}"#));
        }
        _ => panic!("expected AddEdge"),
    }
}

#[test]
fn roundtrip_add_edge_no_string_data() {
    let id = new_id();
    let source = new_id();
    let target = new_id();
    let op: string::Operation = UpdateOperation::AddEdge(AddEdge {
        id,
        source,
        target,
        data: None,
    });
    let buf = fb::encode_operation(&op);
    let decoded = fb::decode_operation(&buf).unwrap();
    match decoded {
        UpdateOperation::AddEdge(e) => {
            assert_eq!(e.id, id);
            assert!(e.data.is_none());
        }
        _ => panic!("expected AddEdge"),
    }
}

#[test]
fn roundtrip_remove_edge_string_format() {
    let id = new_id();
    let add_edge_id = new_id();
    let op: string::Operation =
        UpdateOperation::RemoveEdge(RemoveEdge { id, add_edge_id });
    let buf = fb::encode_operation(&op);
    let decoded = fb::decode_operation(&buf).unwrap();
    match decoded {
        UpdateOperation::RemoveEdge(e) => {
            assert_eq!(e.id, id);
            assert_eq!(e.add_edge_id, add_edge_id);
        }
        _ => panic!("expected RemoveEdge"),
    }
}

// ========================================
// Operation log round-trip
// ========================================

#[test]
fn roundtrip_operation_log_string() {
    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();
    let re1 = new_id();
    let rv2 = new_id();

    let ops: Vec<string::Operation> = vec![
        UpdateOperation::AddVertex(AddVertex {
            id: v1,
            data: Some("Alice".into()),
        }),
        UpdateOperation::AddVertex(AddVertex {
            id: v2,
            data: None,
        }),
        UpdateOperation::AddEdge(AddEdge {
            id: e1,
            source: v1,
            target: v2,
            data: Some("knows".into()),
        }),
        UpdateOperation::RemoveEdge(RemoveEdge {
            id: re1,
            add_edge_id: e1,
        }),
        UpdateOperation::RemoveVertex(RemoveVertex {
            id: rv2,
            add_vertex_id: v2,
        }),
    ];

    let buf = fb::encode_operation_log(&ops);
    let decoded = fb::decode_operation_log(&buf).unwrap();
    assert_eq!(decoded.len(), 5);

    match &decoded[0] {
        UpdateOperation::AddVertex(v) => {
            assert_eq!(v.id, v1);
            assert_eq!(v.data.as_deref(), Some("Alice"));
        }
        _ => panic!("expected AddVertex"),
    }
    match &decoded[2] {
        UpdateOperation::AddEdge(e) => {
            assert_eq!(e.data.as_deref(), Some("knows"));
        }
        _ => panic!("expected AddEdge"),
    }
}

// ========================================
// Integration: encode -> decode -> use StringGraph
// ========================================

#[test]
fn encode_decode_apply_to_string_graph() {
    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    let ops: Vec<string::Operation> = vec![
        UpdateOperation::AddVertex(AddVertex {
            id: v1,
            data: Some("Node A".into()),
        }),
        UpdateOperation::AddVertex(AddVertex {
            id: v2,
            data: Some("Node B".into()),
        }),
        UpdateOperation::AddEdge(AddEdge {
            id: e1,
            source: v1,
            target: v2,
            data: Some("link".into()),
        }),
    ];

    let buf = fb::encode_operation_log(&ops);
    let decoded = fb::decode_operation_log(&buf).unwrap();

    let mut graph = Graph::new();
    for op in decoded {
        graph.update_operation(op).unwrap();
    }

    assert!(graph.lookup_vertex(&v1));
    assert!(graph.lookup_vertex(&v2));
    let pg = graph.generate_petgraph();
    assert_eq!(pg.node_count(), 2);
    assert_eq!(pg.edge_count(), 1);
}

// ========================================
// Two-replica sync
// ========================================

#[test]
fn two_replica_sync_string() {
    let mut replica_a = Graph::new();
    let mut replica_b = Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    let op1 = replica_a
        .prepare(UpdateOperation::AddVertex(AddVertex {
            id: v1,
            data: Some("Person A".into()),
        }))
        .unwrap();
    let op2 = replica_a
        .prepare(UpdateOperation::AddVertex(AddVertex {
            id: v2,
            data: None,
        }))
        .unwrap();
    let op3 = replica_a
        .prepare(UpdateOperation::AddEdge(AddEdge {
            id: e1,
            source: v1,
            target: v2,
            data: Some("follows".into()),
        }))
        .unwrap();

    let wire = fb::encode_operation_log(&[op1, op2, op3]);
    let remote_ops = fb::decode_operation_log(&wire).unwrap();
    for op in remote_ops {
        replica_b.apply_downstream(op).unwrap();
    }

    assert!(replica_b.lookup_vertex(&v1));
    assert!(replica_b.lookup_vertex(&v2));
    assert_eq!(replica_b.generate_petgraph().edge_count(), 1);
}

// ========================================
// Error handling
// ========================================

#[test]
fn decode_string_invalid_buffer_returns_error() {
    let garbage = vec![0u8; 16];
    assert!(fb::decode_operation_log(&garbage).is_err());
}

#[test]
fn decode_string_empty_buffer_returns_error() {
    assert!(fb::decode_operation_log(&[]).is_err());
}
