use crdt_graph::flatbuffers::bytes as fb;
use crdt_graph::types::bytes::{self, AddEdge, AddVertex, Graph};
use crdt_graph::types::{RemoveEdge, RemoveVertex};
use crdt_graph::{UpdateOperation, Uuid};

fn new_id() -> Uuid {
    Uuid::now_v7()
}

// ========================================
// Single operation round-trips (with data)
// ========================================

#[test]
fn roundtrip_add_vertex_with_data() {
    let vid = new_id();
    let op: bytes::Operation = UpdateOperation::AddVertex(AddVertex {
        id: vid,
        data: Some(b"hello vertex".to_vec()),
    });
    let buf = fb::encode_operation(&op);
    let decoded = fb::decode_operation(&buf).unwrap();
    match decoded {
        UpdateOperation::AddVertex(v) => {
            assert_eq!(v.id, vid);
            assert_eq!(v.data.as_deref(), Some(b"hello vertex".as_slice()));
        }
        _ => panic!("expected AddVertex"),
    }
}

#[test]
fn roundtrip_add_vertex_no_data() {
    let vid = new_id();
    let op: bytes::Operation = UpdateOperation::AddVertex(AddVertex {
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
fn roundtrip_remove_vertex_with_data_format() {
    let id = new_id();
    let add_vertex_id = new_id();
    let op: bytes::Operation =
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
fn roundtrip_add_edge_with_data() {
    let id = new_id();
    let source = new_id();
    let target = new_id();
    let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let op: bytes::Operation = UpdateOperation::AddEdge(AddEdge {
        id,
        source,
        target,
        data: Some(payload.clone()),
    });
    let buf = fb::encode_operation(&op);
    let decoded = fb::decode_operation(&buf).unwrap();
    match decoded {
        UpdateOperation::AddEdge(e) => {
            assert_eq!(e.id, id);
            assert_eq!(e.source, source);
            assert_eq!(e.target, target);
            assert_eq!(e.data.as_deref(), Some(payload.as_slice()));
        }
        _ => panic!("expected AddEdge"),
    }
}

#[test]
fn roundtrip_add_edge_no_data() {
    let id = new_id();
    let source = new_id();
    let target = new_id();
    let op: bytes::Operation = UpdateOperation::AddEdge(AddEdge {
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
fn roundtrip_remove_edge_with_data_format() {
    let id = new_id();
    let add_edge_id = new_id();
    let op: bytes::Operation =
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
// Operation log round-trip with mixed data
// ========================================

#[test]
fn roundtrip_operation_log_with_data() {
    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();
    let re1 = new_id();
    let rv2 = new_id();

    let ops: Vec<bytes::Operation> = vec![
        UpdateOperation::AddVertex(AddVertex {
            id: v1,
            data: Some(br#"{"label":"Alice"}"#.to_vec()),
        }),
        UpdateOperation::AddVertex(AddVertex {
            id: v2,
            data: None,
        }),
        UpdateOperation::AddEdge(AddEdge {
            id: e1,
            source: v1,
            target: v2,
            data: Some(b"weight=42".to_vec()),
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
            assert_eq!(v.data.as_deref(), Some(br#"{"label":"Alice"}"#.as_slice()));
        }
        _ => panic!("expected AddVertex"),
    }
    match &decoded[1] {
        UpdateOperation::AddVertex(v) => {
            assert_eq!(v.id, v2);
            assert!(v.data.is_none());
        }
        _ => panic!("expected AddVertex"),
    }
    match &decoded[2] {
        UpdateOperation::AddEdge(e) => {
            assert_eq!(e.data.as_deref(), Some(b"weight=42".as_slice()));
        }
        _ => panic!("expected AddEdge"),
    }
}

// ========================================
// Integration: with-data format applied to graph
// ========================================

#[test]
fn encode_decode_apply_to_graph_with_data() {
    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    let ops: Vec<bytes::Operation> = vec![
        UpdateOperation::AddVertex(AddVertex {
            id: v1,
            data: Some(b"vertex-1-payload".to_vec()),
        }),
        UpdateOperation::AddVertex(AddVertex {
            id: v2,
            data: Some(b"vertex-2-payload".to_vec()),
        }),
        UpdateOperation::AddEdge(AddEdge {
            id: e1,
            source: v1,
            target: v2,
            data: Some(b"edge-payload".to_vec()),
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
// Two-replica sync via with-data format
// ========================================

#[test]
fn two_replica_sync_with_data() {
    let mut replica_a = Graph::new();
    let mut replica_b = Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    let op1 = replica_a
        .prepare(UpdateOperation::AddVertex(AddVertex {
            id: v1,
            data: Some(b"node-a".to_vec()),
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
            data: Some(b"link".to_vec()),
        }))
        .unwrap();

    let wire_bytes = fb::encode_operation_log(&[op1, op2, op3]);
    let remote_ops = fb::decode_operation_log(&wire_bytes).unwrap();
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
fn decode_with_data_invalid_buffer_returns_error() {
    let garbage = vec![0u8; 16];
    assert!(fb::decode_operation_log(&garbage).is_err());
}

#[test]
fn decode_with_data_empty_buffer_returns_error() {
    assert!(fb::decode_operation_log(&[]).is_err());
}
