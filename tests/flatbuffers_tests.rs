use crdt_graph::flatbuffers::simple as fb;
use crdt_graph::types::simple::{self, AddEdge, AddVertex, Graph};
use crdt_graph::types::{RemoveEdge, RemoveVertex};
use crdt_graph::{UpdateOperation, Uuid};

fn new_id() -> Uuid {
    Uuid::now_v7()
}

// ========================================
// Single operation round-trip
// ========================================

#[test]
fn roundtrip_add_vertex() {
    let vid = new_id();
    let op: simple::Operation = UpdateOperation::AddVertex(AddVertex { id: vid });
    let bytes = fb::encode_operation(&op);
    let decoded = fb::decode_operation(&bytes).unwrap();
    match decoded {
        UpdateOperation::AddVertex(v) => assert_eq!(v.id, vid),
        _ => panic!("expected AddVertex"),
    }
}

#[test]
fn roundtrip_remove_vertex() {
    let id = new_id();
    let add_vertex_id = new_id();
    let op: simple::Operation = UpdateOperation::RemoveVertex(RemoveVertex { id, add_vertex_id });
    let bytes = fb::encode_operation(&op);
    let decoded = fb::decode_operation(&bytes).unwrap();
    match decoded {
        UpdateOperation::RemoveVertex(v) => {
            assert_eq!(v.id, id);
            assert_eq!(v.add_vertex_id, add_vertex_id);
        }
        _ => panic!("expected RemoveVertex"),
    }
}

#[test]
fn roundtrip_add_edge() {
    let id = new_id();
    let source = new_id();
    let target = new_id();
    let op: simple::Operation = UpdateOperation::AddEdge(AddEdge { id, source, target });
    let bytes = fb::encode_operation(&op);
    let decoded = fb::decode_operation(&bytes).unwrap();
    match decoded {
        UpdateOperation::AddEdge(e) => {
            assert_eq!(e.id, id);
            assert_eq!(e.source, source);
            assert_eq!(e.target, target);
        }
        _ => panic!("expected AddEdge"),
    }
}

#[test]
fn roundtrip_remove_edge() {
    let id = new_id();
    let add_edge_id = new_id();
    let op: simple::Operation = UpdateOperation::RemoveEdge(RemoveEdge { id, add_edge_id });
    let bytes = fb::encode_operation(&op);
    let decoded = fb::decode_operation(&bytes).unwrap();
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
fn roundtrip_operation_log() {
    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();
    let re1 = new_id();
    let rv1 = new_id();

    let ops: Vec<simple::Operation> = vec![
        UpdateOperation::AddVertex(AddVertex { id: v1 }),
        UpdateOperation::AddVertex(AddVertex { id: v2 }),
        UpdateOperation::AddEdge(AddEdge {
            id: e1,
            source: v1,
            target: v2,
        }),
        UpdateOperation::RemoveEdge(RemoveEdge {
            id: re1,
            add_edge_id: e1,
        }),
        UpdateOperation::RemoveVertex(RemoveVertex {
            id: rv1,
            add_vertex_id: v1,
        }),
    ];

    let bytes = fb::encode_operation_log(&ops);
    let decoded = fb::decode_operation_log(&bytes).unwrap();
    assert_eq!(decoded.len(), 5);

    match &decoded[0] {
        UpdateOperation::AddVertex(v) => assert_eq!(v.id, v1),
        _ => panic!("expected AddVertex"),
    }
    match &decoded[2] {
        UpdateOperation::AddEdge(e) => {
            assert_eq!(e.source, v1);
            assert_eq!(e.target, v2);
        }
        _ => panic!("expected AddEdge"),
    }
}

// ========================================
// Integration: encode -> decode -> apply to graph
// ========================================

#[test]
fn encode_decode_apply_to_graph() {
    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    let ops: Vec<simple::Operation> = vec![
        UpdateOperation::AddVertex(AddVertex { id: v1 }),
        UpdateOperation::AddVertex(AddVertex { id: v2 }),
        UpdateOperation::AddEdge(AddEdge {
            id: e1,
            source: v1,
            target: v2,
        }),
    ];

    let bytes = fb::encode_operation_log(&ops);
    let decoded = fb::decode_operation_log(&bytes).unwrap();

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
// Two-replica sync via FlatBuffers
// ========================================

#[test]
fn two_replica_sync_via_flatbuffers() {
    let mut replica_a = Graph::new();
    let mut replica_b = Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    let op1 = replica_a
        .prepare(UpdateOperation::AddVertex(AddVertex { id: v1 }))
        .unwrap();
    let op2 = replica_a
        .prepare(UpdateOperation::AddVertex(AddVertex { id: v2 }))
        .unwrap();
    let op3 = replica_a
        .prepare(UpdateOperation::AddEdge(AddEdge {
            id: e1,
            source: v1,
            target: v2,
        }))
        .unwrap();

    let wire_bytes = fb::encode_operation_log(&[op1, op2, op3]);

    let remote_ops = fb::decode_operation_log(&wire_bytes).unwrap();
    for op in remote_ops {
        replica_b.apply_downstream(op).unwrap();
    }

    assert!(replica_b.lookup_vertex(&v1));
    assert!(replica_b.lookup_vertex(&v2));
    assert_eq!(replica_a.generate_petgraph().edge_count(), 1);
    assert_eq!(replica_b.generate_petgraph().edge_count(), 1);
}

// ========================================
// Invalid buffer handling
// ========================================

#[test]
fn decode_invalid_buffer_returns_error() {
    let garbage = vec![0u8; 16];
    let result = fb::decode_operation_log(&garbage);
    assert!(result.is_err());
}

#[test]
fn decode_empty_buffer_returns_error() {
    let result = fb::decode_operation_log(&[]);
    assert!(result.is_err());
}
