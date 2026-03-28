use crdt_graph::{
    TwoPTwoPAddEdge, TwoPTwoPAddVertex, TwoPTwoPGraph, TwoPTwoPGraphError, TwoPTwoPId,
    TwoPTwoPRemoveEdge, TwoPTwoPRemoveVertex, UpdateOperation,
};

// --- Test types ---

type Id = u64;

#[derive(Clone, Debug)]
struct VA {
    id: Id,
}

impl TwoPTwoPId<Id> for VA {
    fn id(&self) -> &Id {
        &self.id
    }
}
impl TwoPTwoPAddVertex<Id> for VA {}

#[derive(Clone, Debug)]
struct VR {
    id: Id,
    add_vertex_id: Id,
}

impl TwoPTwoPId<Id> for VR {
    fn id(&self) -> &Id {
        &self.id
    }
}
impl TwoPTwoPRemoveVertex<Id> for VR {
    fn add_vertex_id(&self) -> &Id {
        &self.add_vertex_id
    }
}

#[derive(Clone, Debug)]
struct EA {
    id: Id,
    source: Id,
    target: Id,
}

impl TwoPTwoPId<Id> for EA {
    fn id(&self) -> &Id {
        &self.id
    }
}
impl TwoPTwoPAddEdge<Id> for EA {
    fn source(&self) -> &Id {
        &self.source
    }
    fn target(&self) -> &Id {
        &self.target
    }
}

#[derive(Clone, Debug)]
struct ER {
    id: Id,
    add_edge_id: Id,
}

impl TwoPTwoPId<Id> for ER {
    fn id(&self) -> &Id {
        &self.id
    }
}
impl TwoPTwoPRemoveEdge<Id> for ER {
    fn add_edge_id(&self) -> &Id {
        &self.add_edge_id
    }
}

type TestGraph = TwoPTwoPGraph<VA, VR, EA, ER, Id>;

fn new_graph() -> TestGraph {
    TwoPTwoPGraph::new()
}

// ========================================
// Basic operations (atSource via prepare)
// ========================================

#[test]
fn add_vertex_and_lookup() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    assert!(g.lookup_vertex(&1));
    assert!(!g.lookup_vertex(&99));
}

#[test]
fn add_edge_and_lookup() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 2,
    }))
    .unwrap();
    // Edge lookup relies on generate_petgraph; verify the edge is in the graph
    let pg = g.generate_petgraph();
    assert_eq!(pg.node_count(), 2);
    assert_eq!(pg.edge_count(), 1);
}

#[test]
fn remove_vertex_and_lookup() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    assert!(g.lookup_vertex(&1));
    g.prepare(UpdateOperation::RemoveVertex(VR {
        id: 100,
        add_vertex_id: 1,
    }))
    .unwrap();
    assert!(!g.lookup_vertex(&1));
}

#[test]
fn remove_edge() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 2,
    }))
    .unwrap();
    g.prepare(UpdateOperation::RemoveEdge(ER {
        id: 200,
        add_edge_id: 10,
    }))
    .unwrap();
    let pg = g.generate_petgraph();
    assert_eq!(pg.node_count(), 2);
    assert_eq!(pg.edge_count(), 0);
}

// ========================================
// Duplicate / already exists errors
// ========================================

#[test]
fn add_duplicate_vertex_fails() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    let err = g
        .prepare(UpdateOperation::AddVertex(VA { id: 1 }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::VertexAlreadyExists(1)));
}

#[test]
fn add_duplicate_edge_fails() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 2,
    }))
    .unwrap();
    let err = g
        .prepare(UpdateOperation::AddEdge(EA {
            id: 10,
            source: 1,
            target: 2,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::EdgeAlreadyExists(10)));
}

#[test]
fn remove_vertex_twice_fails() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::RemoveVertex(VR {
        id: 100,
        add_vertex_id: 1,
    }))
    .unwrap();
    // Second remove with same remove-id fails
    let err = g
        .prepare(UpdateOperation::RemoveVertex(VR {
            id: 100,
            add_vertex_id: 1,
        }))
        .unwrap_err();
    // The vertex is already removed so lookup fails at atSource
    assert!(matches!(err, TwoPTwoPGraphError::VertexDoesNotExists(1)));
}

#[test]
fn remove_edge_twice_fails() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 2,
    }))
    .unwrap();
    g.prepare(UpdateOperation::RemoveEdge(ER {
        id: 200,
        add_edge_id: 10,
    }))
    .unwrap();
    let err = g
        .prepare(UpdateOperation::RemoveEdge(ER {
            id: 200,
            add_edge_id: 10,
        }))
        .unwrap_err();
    // Edge no longer passes lookup
    assert!(matches!(err, TwoPTwoPGraphError::EdgeDoesNotExists(10)));
}

// ========================================
// AtSource precondition failures
// ========================================

#[test]
fn add_edge_source_missing() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();
    let err = g
        .prepare(UpdateOperation::AddEdge(EA {
            id: 10,
            source: 1,
            target: 2,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::VertexDoesNotExists(1)));
}

#[test]
fn add_edge_target_missing() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    let err = g
        .prepare(UpdateOperation::AddEdge(EA {
            id: 10,
            source: 1,
            target: 2,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::VertexDoesNotExists(2)));
}

#[test]
fn remove_nonexistent_vertex() {
    let mut g = new_graph();
    let err = g
        .prepare(UpdateOperation::RemoveVertex(VR {
            id: 100,
            add_vertex_id: 1,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::VertexDoesNotExists(1)));
}

#[test]
fn remove_vertex_with_active_edge_fails() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 2,
    }))
    .unwrap();
    let err = g
        .prepare(UpdateOperation::RemoveVertex(VR {
            id: 100,
            add_vertex_id: 1,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::VertexHasEdge(1, 10)));
}

#[test]
fn remove_vertex_after_edge_removed_succeeds() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 2,
    }))
    .unwrap();
    g.prepare(UpdateOperation::RemoveEdge(ER {
        id: 200,
        add_edge_id: 10,
    }))
    .unwrap();
    // Now vertex 1 has no active edges, remove should succeed
    g.prepare(UpdateOperation::RemoveVertex(VR {
        id: 100,
        add_vertex_id: 1,
    }))
    .unwrap();
    assert!(!g.lookup_vertex(&1));
}

#[test]
fn remove_nonexistent_edge() {
    let mut g = new_graph();
    let err = g
        .prepare(UpdateOperation::RemoveEdge(ER {
            id: 200,
            add_edge_id: 10,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::EdgeDoesNotExists(10)));
}

// ========================================
// Downstream
// ========================================

#[test]
fn downstream_add_vertex() {
    let mut g = new_graph();
    g.apply_downstream(UpdateOperation::AddVertex(VA { id: 1 }))
        .unwrap();
    assert!(g.lookup_vertex(&1));
}

#[test]
fn downstream_add_edge_skips_vertex_check() {
    // Per the paper, downstream addEdge has no precondition — vertex existence is NOT checked.
    let mut g = new_graph();
    // No vertices added, but downstream addEdge should still succeed.
    g.apply_downstream(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 2,
    }))
    .unwrap();
    // The edge is stored internally but generate_petgraph won't include it (no vertices).
    let pg = g.generate_petgraph();
    assert_eq!(pg.edge_count(), 0);
}

#[test]
fn downstream_remove_vertex_requires_add_delivered() {
    let mut g = new_graph();
    // addVertex(1) NOT delivered yet
    let err = g
        .apply_downstream(UpdateOperation::RemoveVertex(VR {
            id: 100,
            add_vertex_id: 1,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::AddVertexNotDelivered(1)));
}

#[test]
fn downstream_remove_vertex_after_add_delivered() {
    let mut g = new_graph();
    g.apply_downstream(UpdateOperation::AddVertex(VA { id: 1 }))
        .unwrap();
    g.apply_downstream(UpdateOperation::RemoveVertex(VR {
        id: 100,
        add_vertex_id: 1,
    }))
    .unwrap();
    assert!(!g.lookup_vertex(&1));
}

#[test]
fn downstream_remove_edge_requires_add_delivered() {
    let mut g = new_graph();
    let err = g
        .apply_downstream(UpdateOperation::RemoveEdge(ER {
            id: 200,
            add_edge_id: 10,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::AddEdgeNotDelivered(10)));
}

#[test]
fn downstream_remove_edge_after_add_delivered() {
    let mut g = new_graph();
    g.apply_downstream(UpdateOperation::AddVertex(VA { id: 1 }))
        .unwrap();
    g.apply_downstream(UpdateOperation::AddVertex(VA { id: 2 }))
        .unwrap();
    g.apply_downstream(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 2,
    }))
    .unwrap();
    g.apply_downstream(UpdateOperation::RemoveEdge(ER {
        id: 200,
        add_edge_id: 10,
    }))
    .unwrap();
    let pg = g.generate_petgraph();
    assert_eq!(pg.edge_count(), 0);
}

// ========================================
// Two-replica simulation (prepare + apply_downstream)
// ========================================

#[test]
fn two_replica_add_vertex_sync() {
    let mut replica_a = new_graph();
    let mut replica_b = new_graph();

    let op = replica_a
        .prepare(UpdateOperation::AddVertex(VA { id: 1 }))
        .unwrap();
    replica_b.apply_downstream(op).unwrap();

    assert!(replica_a.lookup_vertex(&1));
    assert!(replica_b.lookup_vertex(&1));
}

#[test]
fn two_replica_full_lifecycle() {
    let mut ra = new_graph();
    let mut rb = new_graph();

    // A adds vertices
    let op1 = ra
        .prepare(UpdateOperation::AddVertex(VA { id: 1 }))
        .unwrap();
    let op2 = ra
        .prepare(UpdateOperation::AddVertex(VA { id: 2 }))
        .unwrap();
    rb.apply_downstream(op1).unwrap();
    rb.apply_downstream(op2).unwrap();

    // B adds an edge
    let op3 = rb
        .prepare(UpdateOperation::AddEdge(EA {
            id: 10,
            source: 1,
            target: 2,
        }))
        .unwrap();
    ra.apply_downstream(op3).unwrap();

    // Both see the same graph
    assert_eq!(ra.generate_petgraph().edge_count(), 1);
    assert_eq!(rb.generate_petgraph().edge_count(), 1);

    // A removes the edge, then the vertex
    let op4 = ra
        .prepare(UpdateOperation::RemoveEdge(ER {
            id: 200,
            add_edge_id: 10,
        }))
        .unwrap();
    let op5 = ra
        .prepare(UpdateOperation::RemoveVertex(VR {
            id: 100,
            add_vertex_id: 1,
        }))
        .unwrap();
    rb.apply_downstream(op4).unwrap();
    rb.apply_downstream(op5).unwrap();

    assert!(!ra.lookup_vertex(&1));
    assert!(!rb.lookup_vertex(&1));
    assert_eq!(ra.generate_petgraph().edge_count(), 0);
    assert_eq!(rb.generate_petgraph().edge_count(), 0);
}

// ========================================
// Edge cases
// ========================================

#[test]
fn self_loop_edge() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 1,
    }))
    .unwrap();
    let pg = g.generate_petgraph();
    assert_eq!(pg.edge_count(), 1);
}

#[test]
fn self_loop_prevents_vertex_removal() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 1,
    }))
    .unwrap();
    let err = g
        .prepare(UpdateOperation::RemoveVertex(VR {
            id: 100,
            add_vertex_id: 1,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::VertexHasEdge(1, 10)));
}

#[test]
fn remove_vertex_target_side_of_edge_blocked() {
    // Removing the target vertex of an active edge should also fail
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 2,
    }))
    .unwrap();
    let err = g
        .prepare(UpdateOperation::RemoveVertex(VR {
            id: 100,
            add_vertex_id: 2,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::VertexHasEdge(2, 10)));
}

#[test]
fn generate_petgraph_empty() {
    let g = new_graph();
    let pg = g.generate_petgraph();
    assert_eq!(pg.node_count(), 0);
    assert_eq!(pg.edge_count(), 0);
}

#[test]
fn generate_petgraph_excludes_removed() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();
    g.prepare(UpdateOperation::AddVertex(VA { id: 3 })).unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 2,
    }))
    .unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 11,
        source: 2,
        target: 3,
    }))
    .unwrap();

    // Remove one edge and one vertex
    g.prepare(UpdateOperation::RemoveEdge(ER {
        id: 200,
        add_edge_id: 10,
    }))
    .unwrap();
    g.prepare(UpdateOperation::RemoveVertex(VR {
        id: 100,
        add_vertex_id: 1,
    }))
    .unwrap();

    let pg = g.generate_petgraph();
    assert_eq!(pg.node_count(), 2); // vertices 2 and 3
    assert_eq!(pg.edge_count(), 1); // edge 11 only
}

#[test]
fn update_operation_backward_compat() {
    // update_operation still works and returns ()
    let mut g = new_graph();
    g.update_operation(UpdateOperation::AddVertex(VA { id: 1 }))
        .unwrap();
    assert!(g.lookup_vertex(&1));
}

#[test]
fn downstream_out_of_causal_order_remove_before_add() {
    // Simulates a message arriving out of causal order:
    // removeVertex arrives before addVertex on the remote replica
    let mut remote = new_graph();
    let err = remote
        .apply_downstream(UpdateOperation::RemoveVertex(VR {
            id: 100,
            add_vertex_id: 1,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::AddVertexNotDelivered(1)));

    // After the addVertex arrives, removeVertex should succeed
    remote
        .apply_downstream(UpdateOperation::AddVertex(VA { id: 1 }))
        .unwrap();
    remote
        .apply_downstream(UpdateOperation::RemoveVertex(VR {
            id: 100,
            add_vertex_id: 1,
        }))
        .unwrap();
    assert!(!remote.lookup_vertex(&1));
}

#[test]
fn downstream_out_of_causal_order_remove_edge_before_add_edge() {
    let mut remote = new_graph();
    let err = remote
        .apply_downstream(UpdateOperation::RemoveEdge(ER {
            id: 200,
            add_edge_id: 10,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::AddEdgeNotDelivered(10)));

    // Deliver the addEdge (downstream skips vertex check)
    remote
        .apply_downstream(UpdateOperation::AddEdge(EA {
            id: 10,
            source: 1,
            target: 2,
        }))
        .unwrap();
    remote
        .apply_downstream(UpdateOperation::RemoveEdge(ER {
            id: 200,
            add_edge_id: 10,
        }))
        .unwrap();
}

#[test]
fn multiple_edges_between_same_vertices() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 2,
    }))
    .unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 11,
        source: 1,
        target: 2,
    }))
    .unwrap();
    let pg = g.generate_petgraph();
    assert_eq!(pg.edge_count(), 2);
}

#[test]
fn remove_one_of_multiple_edges() {
    let mut g = new_graph();
    g.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
    g.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 10,
        source: 1,
        target: 2,
    }))
    .unwrap();
    g.prepare(UpdateOperation::AddEdge(EA {
        id: 11,
        source: 1,
        target: 2,
    }))
    .unwrap();
    g.prepare(UpdateOperation::RemoveEdge(ER {
        id: 200,
        add_edge_id: 10,
    }))
    .unwrap();
    let pg = g.generate_petgraph();
    assert_eq!(pg.edge_count(), 1);
    // Vertex 1 still has edge 11, so removal should fail
    let err = g
        .prepare(UpdateOperation::RemoveVertex(VR {
            id: 100,
            add_vertex_id: 1,
        }))
        .unwrap_err();
    assert!(matches!(err, TwoPTwoPGraphError::VertexHasEdge(..)));
}

#[test]
fn prepare_returns_cloned_operation() {
    let mut g = new_graph();
    let op = UpdateOperation::AddVertex(VA { id: 42 });
    let returned = g.prepare(op).unwrap();
    // The returned operation should carry the same data
    match returned {
        UpdateOperation::AddVertex(va) => assert_eq!(*va.id(), 42),
        _ => panic!("expected AddVertex"),
    }
}
