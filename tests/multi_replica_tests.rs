//! Multi-replica synchronization tests.
//!
//! Simulates 2–4 terminals (replicas) sharing the same CRDT graph,
//! focusing on delivery timing, operation ordering, and convergence.

use crdt_graph::flatbuffers::simple as fb;
use crdt_graph::types::simple::{self, AddEdge, AddVertex, Graph};
use crdt_graph::types::{RemoveEdge, RemoveVertex};
use crdt_graph::{TwoPTwoPGraphError, Uuid};

fn new_id() -> Uuid {
    Uuid::now_v7()
}

/// Helper: assert two graphs have converged to the same logical state.
fn assert_converged(a: &Graph, b: &Graph) {
    let pa = a.generate_petgraph();
    let pb = b.generate_petgraph();
    assert_eq!(
        pa.node_count(),
        pb.node_count(),
        "vertex count mismatch: {} vs {}",
        pa.node_count(),
        pb.node_count()
    );
    assert_eq!(
        pa.edge_count(),
        pb.edge_count(),
        "edge count mismatch: {} vs {}",
        pa.edge_count(),
        pb.edge_count()
    );
}

// ========================================================================
// Two replicas: concurrent vertex additions
// ========================================================================

#[test]
fn two_replicas_concurrent_vertex_add() {
    // Both replicas independently add different vertices, then sync.
    let mut ra = Graph::new();
    let mut rb = Graph::new();

    let va = new_id();
    let vb = new_id();

    let op_a = ra.prepare(AddVertex { id: va }.into()).unwrap();
    let op_b = rb.prepare(AddVertex { id: vb }.into()).unwrap();

    // Cross-deliver
    rb.apply_downstream(op_a).unwrap();
    ra.apply_downstream(op_b).unwrap();

    assert!(ra.lookup_vertex(&va));
    assert!(ra.lookup_vertex(&vb));
    assert_converged(&ra, &rb);
}

// ========================================================================
// Two replicas: concurrent edge additions between the same vertices
// ========================================================================

#[test]
fn two_replicas_concurrent_edge_add_same_vertices() {
    let mut ra = Graph::new();
    let mut rb = Graph::new();

    let v1 = new_id();
    let v2 = new_id();

    // Setup: both have v1, v2
    let ops: Vec<simple::Operation> = vec![
        AddVertex { id: v1 }.into(),
        AddVertex { id: v2 }.into(),
    ];
    for op in &ops {
        ra.update_operation(op.clone()).unwrap();
        rb.apply_downstream(op.clone()).unwrap();
    }

    // Concurrently add different edges between v1 and v2
    let ea = new_id();
    let eb = new_id();
    let op_a = ra
        .prepare(AddEdge { id: ea, source: v1, target: v2 }.into())
        .unwrap();
    let op_b = rb
        .prepare(AddEdge { id: eb, source: v1, target: v2 }.into())
        .unwrap();

    rb.apply_downstream(op_a).unwrap();
    ra.apply_downstream(op_b).unwrap();

    // Both replicas should have 2 edges
    assert_eq!(ra.edge_count(), 2);
    assert_converged(&ra, &rb);
}

// ========================================================================
// Two replicas: concurrent add-edge vs remove-vertex conflict
// ========================================================================

#[test]
fn two_replicas_concurrent_add_edge_and_remove_vertex() {
    // Replica A adds an edge to v1.
    // Replica B concurrently removes v1.
    // After sync, v1 is removed but the edge is in E_A (dangling).
    // petgraph won't include the dangling edge.
    let mut ra = Graph::new();
    let mut rb = Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let v3 = new_id();

    // Setup: both have v1, v2, v3
    for op in [
        AddVertex { id: v1 }.into(),
        AddVertex { id: v2 }.into(),
        AddVertex { id: v3 }.into(),
    ] {
        let broadcast: simple::Operation = ra.prepare(op).unwrap();
        rb.apply_downstream(broadcast).unwrap();
    }

    // A adds edge v1→v2
    let e1 = new_id();
    let op_a = ra
        .prepare(AddEdge { id: e1, source: v1, target: v2 }.into())
        .unwrap();

    // B removes v1 (no edges on B's view, so atSource succeeds)
    let rv1 = new_id();
    let op_b = rb
        .prepare(RemoveVertex { id: rv1, add_vertex_id: v1 }.into())
        .unwrap();

    // Cross-deliver
    // B receives addEdge — downstream skips vertex existence check
    rb.apply_downstream(op_a).unwrap();
    // A receives removeVertex — downstream only checks addVertex delivered
    ra.apply_downstream(op_b).unwrap();

    // Both replicas: v1 is removed, edge exists in E_A but dangling
    assert!(!ra.lookup_vertex(&v1));
    assert!(!rb.lookup_vertex(&v1));

    // petgraph should exclude the dangling edge
    assert_eq!(ra.generate_petgraph().edge_count(), 0);
    assert_converged(&ra, &rb);
}

// ========================================================================
// Two replicas: concurrent remove of same vertex
// ========================================================================

#[test]
fn two_replicas_concurrent_remove_same_vertex() {
    let mut ra = Graph::new();
    let mut rb = Graph::new();

    let v1 = new_id();

    // Setup
    let op = ra.prepare(AddVertex { id: v1 }.into()).unwrap();
    rb.apply_downstream(op).unwrap();

    // Both independently remove v1 (different op IDs)
    let rv_a = new_id();
    let rv_b = new_id();

    let op_a = ra
        .prepare(RemoveVertex { id: rv_a, add_vertex_id: v1 }.into())
        .unwrap();
    let op_b = rb
        .prepare(RemoveVertex { id: rv_b, add_vertex_id: v1 }.into())
        .unwrap();

    // Cross-deliver: each receives the other's remove
    rb.apply_downstream(op_a).unwrap();
    ra.apply_downstream(op_b).unwrap();

    // Both have v1 removed (two entries in V_R, but still converged)
    assert!(!ra.lookup_vertex(&v1));
    assert_converged(&ra, &rb);
}

// ========================================================================
// Two replicas: concurrent remove of same edge
// ========================================================================

#[test]
fn two_replicas_concurrent_remove_same_edge() {
    let mut ra = Graph::new();
    let mut rb = Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    // Setup: both have v1, v2, e1
    for op in [
        AddVertex { id: v1 }.into(),
        AddVertex { id: v2 }.into(),
        AddEdge { id: e1, source: v1, target: v2 }.into(),
    ] {
        let broadcast: simple::Operation = ra.prepare(op).unwrap();
        rb.apply_downstream(broadcast).unwrap();
    }

    // Both independently remove e1
    let re_a = new_id();
    let re_b = new_id();

    let op_a = ra
        .prepare(RemoveEdge { id: re_a, add_edge_id: e1 }.into())
        .unwrap();
    let op_b = rb
        .prepare(RemoveEdge { id: re_b, add_edge_id: e1 }.into())
        .unwrap();

    rb.apply_downstream(op_a).unwrap();
    ra.apply_downstream(op_b).unwrap();

    assert_eq!(ra.edge_count(), 0);
    assert_converged(&ra, &rb);
}

// ========================================================================
// Delivery order: addEdge arrives before addVertex (downstream)
// ========================================================================

#[test]
fn downstream_add_edge_before_vertices_succeeds() {
    // Per the paper, downstream addEdge skips vertex existence checks.
    let mut remote = Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    // Deliver addEdge first — no vertices exist yet
    remote
        .apply_downstream(AddEdge { id: e1, source: v1, target: v2 }.into())
        .unwrap();

    // Edge is in E_A, but no vertices → petgraph is empty
    assert_eq!(remote.generate_petgraph().edge_count(), 0);

    // Now deliver vertices
    remote
        .apply_downstream(AddVertex { id: v1 }.into())
        .unwrap();
    remote
        .apply_downstream(AddVertex { id: v2 }.into())
        .unwrap();

    // Now petgraph shows the edge
    assert_eq!(remote.generate_petgraph().node_count(), 2);
    assert_eq!(remote.generate_petgraph().edge_count(), 1);
}

// ========================================================================
// Delivery order: removeVertex arrives before addVertex (must retry)
// ========================================================================

#[test]
fn downstream_remove_vertex_before_add_vertex_fails() {
    let mut remote = Graph::new();
    let v1 = new_id();
    let rv1 = new_id();

    // removeVertex before addVertex → fails
    let err = remote
        .apply_downstream(RemoveVertex { id: rv1, add_vertex_id: v1 }.into())
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::AddVertexNotDelivered(_)));

    // After addVertex is delivered, removeVertex succeeds
    remote
        .apply_downstream(AddVertex { id: v1 }.into())
        .unwrap();
    remote
        .apply_downstream(RemoveVertex { id: rv1, add_vertex_id: v1 }.into())
        .unwrap();

    assert!(!remote.lookup_vertex(&v1));
}

// ========================================================================
// Delivery order: removeEdge arrives before addEdge (must retry)
// ========================================================================

#[test]
fn downstream_remove_edge_before_add_edge_fails() {
    let mut remote = Graph::new();
    let e1 = new_id();
    let re1 = new_id();

    let err = remote
        .apply_downstream(RemoveEdge { id: re1, add_edge_id: e1 }.into())
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::AddEdgeNotDelivered(_)));

    // Deliver addEdge (downstream skips vertex check)
    let v1 = new_id();
    let v2 = new_id();
    remote
        .apply_downstream(AddEdge { id: e1, source: v1, target: v2 }.into())
        .unwrap();
    remote
        .apply_downstream(RemoveEdge { id: re1, add_edge_id: e1 }.into())
        .unwrap();
}

// ========================================================================
// Reverse delivery order: operations arrive in exact reverse
// ========================================================================

#[test]
fn reverse_delivery_order() {
    let mut origin = Graph::new();
    let mut remote = Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();
    let re1 = new_id();
    let rv1 = new_id();

    // Origin performs operations in sequence
    let ops: Vec<simple::Operation> = vec![
        origin.prepare(AddVertex { id: v1 }.into()).unwrap(),
        origin.prepare(AddVertex { id: v2 }.into()).unwrap(),
        origin.prepare(AddEdge { id: e1, source: v1, target: v2 }.into()).unwrap(),
        origin.prepare(RemoveEdge { id: re1, add_edge_id: e1 }.into()).unwrap(),
        origin.prepare(RemoveVertex { id: rv1, add_vertex_id: v1 }.into()).unwrap(),
    ];

    // Deliver in reverse order, buffering failed ops
    let mut pending: Vec<simple::Operation> = ops.into_iter().rev().collect();

    // Keep retrying until all are delivered (simulates causal delivery layer)
    let mut rounds = 0;
    while !pending.is_empty() {
        let mut still_pending = Vec::new();
        for op in pending {
            if remote.apply_downstream(op.clone()).is_err() {
                still_pending.push(op);
            }
        }
        pending = still_pending;
        rounds += 1;
        assert!(rounds <= 10, "delivery did not converge within 10 rounds");
    }

    assert_converged(&origin, &remote);
}

// ========================================================================
// Three replicas: independent edits, all converge
// ========================================================================

#[test]
fn three_replicas_independent_edits_converge() {
    let mut ra = Graph::new();
    let mut rb = Graph::new();
    let mut rc = Graph::new();

    // Common setup: 3 shared vertices
    let v1 = new_id();
    let v2 = new_id();
    let v3 = new_id();

    for op in [
        AddVertex { id: v1 }.into(),
        AddVertex { id: v2 }.into(),
        AddVertex { id: v3 }.into(),
    ] {
        let broadcast: simple::Operation = ra.prepare(op).unwrap();
        rb.apply_downstream(broadcast.clone()).unwrap();
        rc.apply_downstream(broadcast).unwrap();
    }

    // Each replica independently adds an edge
    let ea = new_id();
    let eb = new_id();
    let ec = new_id();

    let op_a = ra
        .prepare(AddEdge { id: ea, source: v1, target: v2 }.into())
        .unwrap();
    let op_b = rb
        .prepare(AddEdge { id: eb, source: v2, target: v3 }.into())
        .unwrap();
    let op_c = rc
        .prepare(AddEdge { id: ec, source: v3, target: v1 }.into())
        .unwrap();

    // Full mesh sync: everyone delivers to everyone else
    rb.apply_downstream(op_a.clone()).unwrap();
    rc.apply_downstream(op_a).unwrap();

    ra.apply_downstream(op_b.clone()).unwrap();
    rc.apply_downstream(op_b).unwrap();

    ra.apply_downstream(op_c.clone()).unwrap();
    rb.apply_downstream(op_c).unwrap();

    assert_eq!(ra.edge_count(), 3);
    assert_converged(&ra, &rb);
    assert_converged(&rb, &rc);
}

// ========================================================================
// Three replicas: different delivery orderings converge
// ========================================================================

#[test]
fn three_replicas_different_delivery_order_converge() {
    // A creates vertices & edges. B and C receive in different orders.
    let mut ra = Graph::new();
    let mut rb = Graph::new();
    let mut rc = Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let v3 = new_id();
    let e1 = new_id();
    let e2 = new_id();

    let ops: Vec<simple::Operation> = vec![
        ra.prepare(AddVertex { id: v1 }.into()).unwrap(),
        ra.prepare(AddVertex { id: v2 }.into()).unwrap(),
        ra.prepare(AddVertex { id: v3 }.into()).unwrap(),
        ra.prepare(AddEdge { id: e1, source: v1, target: v2 }.into()).unwrap(),
        ra.prepare(AddEdge { id: e2, source: v2, target: v3 }.into()).unwrap(),
    ];

    // B receives in original order
    for op in &ops {
        rb.apply_downstream(op.clone()).unwrap();
    }

    // C receives vertices first (out of original order: v3, v1, v2, then edges)
    rc.apply_downstream(ops[2].clone()).unwrap(); // v3
    rc.apply_downstream(ops[0].clone()).unwrap(); // v1
    rc.apply_downstream(ops[1].clone()).unwrap(); // v2
    rc.apply_downstream(ops[3].clone()).unwrap(); // e1
    rc.apply_downstream(ops[4].clone()).unwrap(); // e2

    assert_converged(&ra, &rb);
    assert_converged(&rb, &rc);
}

// ========================================================================
// Network partition: operations accumulate, then bulk sync
// ========================================================================

#[test]
fn network_partition_bulk_sync() {
    let mut ra = Graph::new();
    let mut rb = Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let v3 = new_id();

    // Setup: both have v1, v2
    for op in [
        AddVertex { id: v1 }.into(),
        AddVertex { id: v2 }.into(),
    ] {
        let broadcast: simple::Operation = ra.prepare(op).unwrap();
        rb.apply_downstream(broadcast).unwrap();
    }

    // ---- PARTITION ----
    // A: adds v3, edge v1→v3
    let mut a_ops = Vec::new();
    a_ops.push(ra.prepare(AddVertex { id: v3 }.into()).unwrap());
    let e1 = new_id();
    a_ops.push(
        ra.prepare(AddEdge { id: e1, source: v1, target: v3 }.into())
            .unwrap(),
    );

    // B: adds edge v1→v2, removes it, then removes v2
    let mut b_ops = Vec::new();
    let e2 = new_id();
    b_ops.push(
        rb.prepare(AddEdge { id: e2, source: v1, target: v2 }.into())
            .unwrap(),
    );
    let re2 = new_id();
    b_ops.push(
        rb.prepare(RemoveEdge { id: re2, add_edge_id: e2 }.into())
            .unwrap(),
    );
    let rv2 = new_id();
    b_ops.push(
        rb.prepare(RemoveVertex { id: rv2, add_vertex_id: v2 }.into())
            .unwrap(),
    );

    // ---- RECONNECT: bulk sync ----
    for op in &a_ops {
        rb.apply_downstream(op.clone()).unwrap();
    }
    for op in &b_ops {
        ra.apply_downstream(op.clone()).unwrap();
    }

    // Convergence: v1 alive, v2 removed, v3 alive, e1 alive, e2 removed
    assert!(ra.lookup_vertex(&v1));
    assert!(!ra.lookup_vertex(&v2));
    assert!(ra.lookup_vertex(&v3));
    assert_eq!(ra.vertex_count(), 2);
    assert_eq!(ra.edge_count(), 1);
    assert_converged(&ra, &rb);
}

// ========================================================================
// Network partition: conflicting concurrent edits
// ========================================================================

#[test]
fn partition_concurrent_edge_add_and_vertex_remove() {
    // A adds edge v2→v1 while B removes v1 — classic conflict.
    let mut ra = Graph::new();
    let mut rb = Graph::new();

    let v1 = new_id();
    let v2 = new_id();

    for op in [
        AddVertex { id: v1 }.into(),
        AddVertex { id: v2 }.into(),
    ] {
        let broadcast: simple::Operation = ra.prepare(op).unwrap();
        rb.apply_downstream(broadcast).unwrap();
    }

    // PARTITION
    let e1 = new_id();
    let op_a = ra
        .prepare(AddEdge { id: e1, source: v2, target: v1 }.into())
        .unwrap();

    let rv1 = new_id();
    let op_b = rb
        .prepare(RemoveVertex { id: rv1, add_vertex_id: v1 }.into())
        .unwrap();

    // RECONNECT
    rb.apply_downstream(op_a).unwrap();  // edge added, v1 already removed on B
    ra.apply_downstream(op_b).unwrap();  // v1 removed on A

    // Both: v1 removed, v2 alive, edge in E_A but dangling
    assert!(!ra.lookup_vertex(&v1));
    assert!(ra.lookup_vertex(&v2));
    assert_eq!(ra.generate_petgraph().edge_count(), 0); // dangling, not in petgraph
    assert_converged(&ra, &rb);
}

// ========================================================================
// Four replicas: diamond topology (A→B, A→C, B→D, C→D)
// ========================================================================

#[test]
fn four_replicas_diamond_topology() {
    let mut ra = Graph::new();
    let mut rb = Graph::new();
    let mut rc = Graph::new();
    let mut rd = Graph::new();

    let v1 = new_id();
    let v2 = new_id();

    // A creates vertices
    let op1 = ra.prepare(AddVertex { id: v1 }.into()).unwrap();
    let op2 = ra.prepare(AddVertex { id: v2 }.into()).unwrap();

    // A → B
    rb.apply_downstream(op1.clone()).unwrap();
    rb.apply_downstream(op2.clone()).unwrap();

    // A → C
    rc.apply_downstream(op1).unwrap();
    rc.apply_downstream(op2).unwrap();

    // B and C concurrently add edges
    let eb = new_id();
    let ec = new_id();

    let op_b = rb
        .prepare(AddEdge { id: eb, source: v1, target: v2 }.into())
        .unwrap();
    let op_c = rc
        .prepare(AddEdge { id: ec, source: v2, target: v1 }.into())
        .unwrap();

    // B → D, C → D (D gets both vertices + both edges)
    // D needs vertices first
    rd.apply_downstream(AddVertex { id: v1 }.into()).unwrap();
    rd.apply_downstream(AddVertex { id: v2 }.into()).unwrap();
    rd.apply_downstream(op_b.clone()).unwrap();
    rd.apply_downstream(op_c.clone()).unwrap();

    // Also sync between B↔C so all four converge
    rc.apply_downstream(op_b).unwrap();
    rb.apply_downstream(op_c).unwrap();

    // Sync B,C edges to A
    let all_edges_for_a: Vec<simple::Operation> = vec![
        AddEdge { id: eb, source: v1, target: v2 }.into(),
        AddEdge { id: ec, source: v2, target: v1 }.into(),
    ];
    for op in all_edges_for_a {
        ra.apply_downstream(op).unwrap();
    }

    // All four replicas converge: 2 vertices, 2 edges
    assert_eq!(rd.vertex_count(), 2);
    assert_eq!(rd.edge_count(), 2);
    assert_converged(&ra, &rb);
    assert_converged(&rb, &rc);
    assert_converged(&rc, &rd);
}

// ========================================================================
// Serialization: encode on one replica, decode + apply on others
// ========================================================================

#[test]
fn multi_replica_sync_via_flatbuffers() {
    let mut ra = Graph::new();
    let mut rb = Graph::new();
    let mut rc = Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let v3 = new_id();
    let e1 = new_id();

    // A builds the graph
    let ops_a: Vec<simple::Operation> = vec![
        ra.prepare(AddVertex { id: v1 }.into()).unwrap(),
        ra.prepare(AddVertex { id: v2 }.into()).unwrap(),
        ra.prepare(AddVertex { id: v3 }.into()).unwrap(),
        ra.prepare(AddEdge { id: e1, source: v1, target: v2 }.into()).unwrap(),
    ];

    // A → B via FlatBuffers
    let wire_ab = fb::encode_operation_log(&ops_a);
    for op in fb::decode_operation_log(&wire_ab).unwrap() {
        rb.apply_downstream(op).unwrap();
    }

    // B adds an edge and syncs to C via FlatBuffers
    let e2 = new_id();
    let op_b = rb
        .prepare(AddEdge { id: e2, source: v2, target: v3 }.into())
        .unwrap();

    // C gets A's ops + B's op in a single log
    let mut all_for_c = ops_a.clone();
    all_for_c.push(op_b.clone());
    let wire_c = fb::encode_operation_log(&all_for_c);
    for op in fb::decode_operation_log(&wire_c).unwrap() {
        rc.apply_downstream(op).unwrap();
    }

    // Sync B's edge back to A
    ra.apply_downstream(op_b).unwrap();

    assert_eq!(ra.edge_count(), 2);
    assert_converged(&ra, &rb);
    assert_converged(&rb, &rc);
}

// ========================================================================
// Stress: many concurrent operations from multiple replicas
// ========================================================================

#[test]
fn many_concurrent_operations_three_replicas() {
    let mut ra = Graph::new();
    let mut rb = Graph::new();
    let mut rc = Graph::new();

    // Each replica adds 10 vertices
    let mut a_ops = Vec::new();
    let mut b_ops = Vec::new();
    let mut c_ops = Vec::new();

    let mut a_verts = Vec::new();
    let mut b_verts = Vec::new();
    let mut c_verts = Vec::new();

    for _ in 0..10 {
        let va = new_id();
        a_verts.push(va);
        a_ops.push(ra.prepare(AddVertex { id: va }.into()).unwrap());

        let vb = new_id();
        b_verts.push(vb);
        b_ops.push(rb.prepare(AddVertex { id: vb }.into()).unwrap());

        let vc = new_id();
        c_verts.push(vc);
        c_ops.push(rc.prepare(AddVertex { id: vc }.into()).unwrap());
    }

    // Full mesh sync of all vertex adds
    for op in &a_ops {
        rb.apply_downstream(op.clone()).unwrap();
        rc.apply_downstream(op.clone()).unwrap();
    }
    for op in &b_ops {
        ra.apply_downstream(op.clone()).unwrap();
        rc.apply_downstream(op.clone()).unwrap();
    }
    for op in &c_ops {
        ra.apply_downstream(op.clone()).unwrap();
        rb.apply_downstream(op.clone()).unwrap();
    }

    assert_eq!(ra.vertex_count(), 30);
    assert_converged(&ra, &rb);
    assert_converged(&rb, &rc);

    // Each replica adds edges between its own verts and others
    let mut edge_ops_a = Vec::new();
    let mut edge_ops_b = Vec::new();

    for i in 0..5 {
        let e = new_id();
        edge_ops_a.push(
            ra.prepare(
                AddEdge { id: e, source: a_verts[i], target: b_verts[i] }.into(),
            )
            .unwrap(),
        );
        let e = new_id();
        edge_ops_b.push(
            rb.prepare(
                AddEdge { id: e, source: b_verts[i], target: c_verts[i] }.into(),
            )
            .unwrap(),
        );
    }

    // Sync edges
    for op in &edge_ops_a {
        rb.apply_downstream(op.clone()).unwrap();
        rc.apply_downstream(op.clone()).unwrap();
    }
    for op in &edge_ops_b {
        ra.apply_downstream(op.clone()).unwrap();
        rc.apply_downstream(op.clone()).unwrap();
    }

    assert_eq!(ra.vertex_count(), 30);
    assert_eq!(ra.edge_count(), 10);
    assert_converged(&ra, &rb);
    assert_converged(&rb, &rc);
}

// ========================================================================
// Edge case: add edge then remove both vertices concurrently
// ========================================================================

#[test]
fn concurrent_remove_both_endpoints_of_edge() {
    // A removes source vertex, B removes target vertex, concurrently.
    let mut ra = Graph::new();
    let mut rb = Graph::new();

    let v1 = new_id();
    let v2 = new_id();
    let e1 = new_id();

    // Setup: both have v1, v2, e1
    for op in [
        AddVertex { id: v1 }.into(),
        AddVertex { id: v2 }.into(),
        AddEdge { id: e1, source: v1, target: v2 }.into(),
    ] {
        let broadcast: simple::Operation = ra.prepare(op).unwrap();
        rb.apply_downstream(broadcast).unwrap();
    }

    // A removes edge, then source vertex
    let re1 = new_id();
    let rv1 = new_id();
    let ops_a: Vec<simple::Operation> = vec![
        ra.prepare(RemoveEdge { id: re1, add_edge_id: e1 }.into()).unwrap(),
        ra.prepare(RemoveVertex { id: rv1, add_vertex_id: v1 }.into()).unwrap(),
    ];

    // B also removes edge, then target vertex
    let re1b = new_id();
    let rv2 = new_id();
    let ops_b: Vec<simple::Operation> = vec![
        rb.prepare(RemoveEdge { id: re1b, add_edge_id: e1 }.into()).unwrap(),
        rb.prepare(RemoveVertex { id: rv2, add_vertex_id: v2 }.into()).unwrap(),
    ];

    // Cross-deliver
    for op in &ops_a {
        rb.apply_downstream(op.clone()).unwrap();
    }
    for op in &ops_b {
        ra.apply_downstream(op.clone()).unwrap();
    }

    // Both vertices removed, edge removed
    assert!(!ra.lookup_vertex(&v1));
    assert!(!ra.lookup_vertex(&v2));
    assert_eq!(ra.vertex_count(), 0);
    assert_eq!(ra.edge_count(), 0);
    assert_converged(&ra, &rb);
}

// ========================================================================
// Edge case: self-loop edge with concurrent vertex remove
// ========================================================================

#[test]
fn self_loop_concurrent_vertex_remove() {
    let mut ra = Graph::new();
    let mut rb = Graph::new();

    let v1 = new_id();

    let op = ra.prepare(AddVertex { id: v1 }.into()).unwrap();
    rb.apply_downstream(op).unwrap();

    // A adds self-loop
    let e1 = new_id();
    let op_a = ra
        .prepare(AddEdge { id: e1, source: v1, target: v1 }.into())
        .unwrap();

    // B removes v1 (no edges on B's copy)
    let rv1 = new_id();
    let op_b = rb
        .prepare(RemoveVertex { id: rv1, add_vertex_id: v1 }.into())
        .unwrap();

    // Cross-deliver
    rb.apply_downstream(op_a).unwrap();
    ra.apply_downstream(op_b).unwrap();

    // v1 removed, self-loop is dangling
    assert!(!ra.lookup_vertex(&v1));
    assert_eq!(ra.generate_petgraph().edge_count(), 0);
    assert_converged(&ra, &rb);
}

// ========================================================================
// Edge case: interleaved operations from 3 replicas
// ========================================================================

#[test]
fn three_replicas_interleaved_add_remove() {
    // A adds vertices, B adds edge, C removes the edge.
    // Delivery to a fresh replica D in random-ish order.
    let mut ra = Graph::new();
    let mut rb = Graph::new();
    let mut rc = Graph::new();

    let v1 = new_id();
    let v2 = new_id();

    // A adds vertices, syncs to B and C
    let op1 = ra.prepare(AddVertex { id: v1 }.into()).unwrap();
    let op2 = ra.prepare(AddVertex { id: v2 }.into()).unwrap();
    for op in [&op1, &op2] {
        rb.apply_downstream(op.clone()).unwrap();
        rc.apply_downstream(op.clone()).unwrap();
    }

    // B adds edge
    let e1 = new_id();
    let op3 = rb
        .prepare(AddEdge { id: e1, source: v1, target: v2 }.into())
        .unwrap();
    ra.apply_downstream(op3.clone()).unwrap();
    rc.apply_downstream(op3.clone()).unwrap();

    // C removes the edge
    let re1 = new_id();
    let op4 = rc
        .prepare(RemoveEdge { id: re1, add_edge_id: e1 }.into())
        .unwrap();
    ra.apply_downstream(op4.clone()).unwrap();
    rb.apply_downstream(op4.clone()).unwrap();

    // Fresh replica D receives all ops in shuffled order:
    // addEdge, removeEdge, addVertex v2, addVertex v1
    let mut rd = Graph::new();
    rd.apply_downstream(op3).unwrap();         // addEdge (succeeds: downstream)
    // removeEdge succeeds because addEdge already delivered
    rd.apply_downstream(op4).unwrap();
    rd.apply_downstream(op2).unwrap();         // addVertex v2
    rd.apply_downstream(op1).unwrap();         // addVertex v1

    // All four converge: 2 vertices, 0 edges
    assert_eq!(rd.vertex_count(), 2);
    assert_eq!(rd.edge_count(), 0);
    assert_converged(&ra, &rb);
    assert_converged(&rb, &rc);
    assert_converged(&rc, &rd);
}
