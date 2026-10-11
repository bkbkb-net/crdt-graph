//! The graph as it was before it was indexed (crdt-graph 0.3.1, `src/graph.rs`),
//! kept verbatim apart from its name and imports: every set a `Vec`, every
//! question a scan. It is the definition the indexed graph is checked
//! against in `tests/index_oracle.rs`, so it is deliberately not improved.

#![allow(dead_code, unused_imports)]

use crdt_graph::{
    TwoPTwoPAddEdge, TwoPTwoPAddVertex, TwoPTwoPGraphError, TwoPTwoPId, TwoPTwoPRemoveEdge,
    TwoPTwoPRemoveVertex, UpdateOperation, UpdateType,
};
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Clone, Debug)]
pub struct ScanGraph<VA, VR, EA, ER, I>
where
    VA: Clone + TwoPTwoPAddVertex<I>,
    VR: Clone + TwoPTwoPRemoveVertex<I>,
    EA: Clone + TwoPTwoPAddEdge<I>,
    ER: Clone + TwoPTwoPRemoveEdge<I>,
    I: Eq + Hash + Debug + Clone,
{
    vertices_added: Vec<VA>,
    vertices_removed: Vec<VR>,
    edges_added: Vec<EA>,
    edges_removed: Vec<ER>,
    _phantom: std::marker::PhantomData<I>,
}

impl<VA, VR, EA, ER, I> Default for ScanGraph<VA, VR, EA, ER, I>
where
    VA: Clone + TwoPTwoPAddVertex<I>,
    VR: Clone + TwoPTwoPRemoveVertex<I>,
    EA: Clone + TwoPTwoPAddEdge<I>,
    ER: Clone + TwoPTwoPRemoveEdge<I>,
    I: Eq + Hash + Debug + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<VA, VR, EA, ER, I> ScanGraph<VA, VR, EA, ER, I>
where
    VA: Clone + TwoPTwoPAddVertex<I>,
    VR: Clone + TwoPTwoPRemoveVertex<I>,
    EA: Clone + TwoPTwoPAddEdge<I>,
    ER: Clone + TwoPTwoPRemoveEdge<I>,
    I: Eq + Hash + Debug + Clone,
{
    /// Creates an empty graph with all four sets initialized to ∅.
    pub fn new() -> Self {
        ScanGraph {
            vertices_added: Vec::new(),
            vertices_removed: Vec::new(),
            edges_added: Vec::new(),
            edges_removed: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Returns `true` if the vertex is in `V_A \ V_R` (added and not removed).
    pub fn lookup_vertex(&self, vertex_id: &I) -> bool {
        self.vertices_added.iter().any(|va| va.id() == vertex_id)
            && !self
                .vertices_removed
                .iter()
                .any(|vr| vr.add_vertex_id() == vertex_id)
    }

    /// Returns the edge-add operation referenced by a given edge-remove operation, if present.
    pub fn get_edge_added_from_remove_edge(&self, remove_edge: &ER) -> Option<&EA> {
        self.edges_added
            .iter()
            .find(|ea| ea.id() == remove_edge.add_edge_id())
    }

    /// Returns `true` if the edge referenced by `remove_edge` exists in `E_A \ E_R`
    /// and both of its endpoint vertices are currently in `V_A \ V_R`.
    pub fn lookup_from_remove_edge(&self, remove_edge: &ER) -> bool {
        self.get_edge_added_from_remove_edge(remove_edge)
            .is_some_and(|edge_added| {
                self.lookup_vertex(edge_added.source())
                    && self.lookup_vertex(edge_added.target())
                    && !self
                        .edges_removed
                        .iter()
                        .any(|er| er.add_edge_id() == remove_edge.add_edge_id())
            })
    }

    /// Convenience method that calls [`prepare`](Self::prepare) and discards the returned operation.
    pub fn update_operation(
        &mut self,
        update_operation: UpdateOperation<VA, VR, EA, ER>,
    ) -> Result<(), TwoPTwoPGraphError<I>> {
        self.prepare(update_operation).map(|_| ())
    }

    /// Executes atSource precondition checks and applies the downstream effect locally.
    /// Returns the operation to broadcast to other replicas.
    pub fn prepare(
        &mut self,
        op: UpdateOperation<VA, VR, EA, ER>,
    ) -> Result<UpdateOperation<VA, VR, EA, ER>, TwoPTwoPGraphError<I>> {
        let broadcast = op.clone();
        match op {
            UpdateOperation::AddVertex(vertex) => self.add_vertex(vertex, UpdateType::AtSource)?,
            UpdateOperation::AddEdge(edge) => self.add_edge(edge, UpdateType::AtSource)?,
            UpdateOperation::RemoveVertex(vertex) => {
                self.remove_vertex(vertex, UpdateType::AtSource)?
            }
            UpdateOperation::RemoveEdge(edge) => self.remove_edge(edge, UpdateType::AtSource)?,
        }
        Ok(broadcast)
    }

    /// Applies an operation received from a remote replica (downstream).
    pub fn apply_downstream(
        &mut self,
        op: UpdateOperation<VA, VR, EA, ER>,
    ) -> Result<(), TwoPTwoPGraphError<I>> {
        match op {
            UpdateOperation::AddVertex(vertex) => self.add_vertex(vertex, UpdateType::Downstream),
            UpdateOperation::AddEdge(edge) => self.add_edge(edge, UpdateType::Downstream),
            UpdateOperation::RemoveVertex(vertex) => {
                self.remove_vertex(vertex, UpdateType::Downstream)
            }
            UpdateOperation::RemoveEdge(edge) => self.remove_edge(edge, UpdateType::Downstream),
        }
    }

    /// Adds a vertex to `V_A`. Fails if a vertex with the same ID already exists.
    ///
    /// Both `AtSource` and `Downstream` behave identically (no preconditions per the paper).
    pub fn add_vertex(
        &mut self,
        vertex: VA,
        _update_type: UpdateType,
    ) -> Result<(), TwoPTwoPGraphError<I>> {
        if self.vertices_added.iter().any(|va| va.id() == vertex.id()) {
            return Err(TwoPTwoPGraphError::VertexAlreadyExists(vertex.id().clone()));
        }
        self.vertices_added.push(vertex);
        Ok(())
    }

    /// Adds an edge to `E_A`.
    ///
    /// - **AtSource**: checks `lookup(source) ∧ lookup(target)`.
    /// - **Downstream**: skips vertex existence checks (per the paper).
    pub fn add_edge(
        &mut self,
        edge: EA,
        update_type: UpdateType,
    ) -> Result<(), TwoPTwoPGraphError<I>> {
        if matches!(update_type, UpdateType::AtSource) {
            if !self.lookup_vertex(edge.source()) {
                return Err(TwoPTwoPGraphError::VertexDoesNotExists(
                    edge.source().clone(),
                ));
            }
            if !self.lookup_vertex(edge.target()) {
                return Err(TwoPTwoPGraphError::VertexDoesNotExists(
                    edge.target().clone(),
                ));
            }
        }
        if self.edges_added.iter().any(|ea| ea.id() == edge.id()) {
            return Err(TwoPTwoPGraphError::EdgeAlreadyExists(edge.id().clone()));
        }
        self.edges_added.push(edge);
        Ok(())
    }

    /// Adds a vertex-remove to `V_R`.
    ///
    /// - **AtSource**: checks `lookup(w)` and that no active edge references `w`.
    /// - **Downstream**: checks that the corresponding `addVertex(w)` has been delivered.
    pub fn remove_vertex(
        &mut self,
        vertex: VR,
        update_type: UpdateType,
    ) -> Result<(), TwoPTwoPGraphError<I>> {
        if matches!(update_type, UpdateType::AtSource) {
            // pre
            if !self.lookup_vertex(vertex.add_vertex_id()) {
                return Err(TwoPTwoPGraphError::VertexDoesNotExists(
                    vertex.add_vertex_id().clone(),
                ));
            }
            // pre: E ⊆ V × V — vertex has no active edges
            for ea in self.edges_added.iter() {
                let is_removed = self
                    .edges_removed
                    .iter()
                    .any(|er| ea.id() == er.add_edge_id());
                if !is_removed
                    && (ea.source() == vertex.add_vertex_id()
                        || ea.target() == vertex.add_vertex_id())
                {
                    return Err(TwoPTwoPGraphError::VertexHasEdge(
                        vertex.add_vertex_id().clone(),
                        ea.id().clone(),
                    ));
                }
            }
        }

        if matches!(update_type, UpdateType::Downstream) {
            // pre: addVertex(w) delivered
            if !self
                .vertices_added
                .iter()
                .any(|va| va.id() == vertex.add_vertex_id())
            {
                return Err(TwoPTwoPGraphError::AddVertexNotDelivered(
                    vertex.add_vertex_id().clone(),
                ));
            }
        }

        if self
            .vertices_removed
            .iter()
            .any(|vr| vr.id() == vertex.id())
        {
            return Err(TwoPTwoPGraphError::VertexAlreadyExists(vertex.id().clone()));
        }
        self.vertices_removed.push(vertex);
        Ok(())
    }

    /// Adds an edge-remove to `E_R`.
    ///
    /// - **AtSource**: checks `lookup((u,v))`.
    /// - **Downstream**: checks that the corresponding `addEdge(u,v)` has been delivered.
    pub fn remove_edge(
        &mut self,
        remove_edge: ER,
        update_type: UpdateType,
    ) -> Result<(), TwoPTwoPGraphError<I>> {
        if matches!(update_type, UpdateType::AtSource) {
            // pre: lookup((u,v))
            if !self.lookup_from_remove_edge(&remove_edge) {
                return Err(TwoPTwoPGraphError::EdgeDoesNotExists(
                    remove_edge.add_edge_id().clone(),
                ));
            }
        }

        if matches!(update_type, UpdateType::Downstream) {
            // pre: addEdge(u,v) delivered
            if !self
                .edges_added
                .iter()
                .any(|ea| ea.id() == remove_edge.add_edge_id())
            {
                return Err(TwoPTwoPGraphError::AddEdgeNotDelivered(
                    remove_edge.add_edge_id().clone(),
                ));
            }
        }

        if self
            .edges_removed
            .iter()
            .any(|er| er.id() == remove_edge.id())
        {
            return Err(TwoPTwoPGraphError::EdgeAlreadyExists(
                remove_edge.id().clone(),
            ));
        }
        self.edges_removed.push(remove_edge);
        Ok(())
    }

    /// Converts the current CRDT state into a [`petgraph::graph::DiGraph`].
    ///
    /// Only vertices in `V_A \ V_R` and edges in `E_A \ E_R` (whose endpoints
    /// are present) are included in the resulting directed graph.
    pub fn generate_petgraph(&self) -> petgraph::graph::DiGraph<VA, EA> {
        let mut graph = petgraph::graph::DiGraph::new();
        let mut vertex_map = std::collections::HashMap::new();
        for va in self.vertices_added.iter() {
            let is_removed = self
                .vertices_removed
                .iter()
                .any(|vr| va.id() == vr.add_vertex_id());
            if !is_removed {
                let vertex = graph.add_node(va.clone());
                vertex_map.insert(va.id().clone(), vertex);
            }
        }
        for ea in self.edges_added.iter() {
            let is_removed = self
                .edges_removed
                .iter()
                .any(|er| ea.id() == er.add_edge_id());
            if !is_removed {
                if let (Some(&source), Some(&target)) =
                    (vertex_map.get(ea.source()), vertex_map.get(ea.target()))
                {
                    graph.add_edge(source, target, ea.clone());
                }
            }
        }
        graph
    }

    /// Returns the number of active vertices (`V_A \ V_R`).
    pub fn vertex_count(&self) -> usize {
        self.vertices_added
            .iter()
            .filter(|va| {
                !self
                    .vertices_removed
                    .iter()
                    .any(|vr| va.id() == vr.add_vertex_id())
            })
            .count()
    }

    /// Returns the number of active edges (`E_A \ E_R`).
    pub fn edge_count(&self) -> usize {
        self.edges_added
            .iter()
            .filter(|ea| {
                !self
                    .edges_removed
                    .iter()
                    .any(|er| ea.id() == er.add_edge_id())
            })
            .count()
    }

    /// Returns `true` if the graph has no active vertices and no active edges.
    pub fn is_empty(&self) -> bool {
        self.vertex_count() == 0 && self.edge_count() == 0
    }

    /// Returns an iterator over all active (non-removed) vertex-add operations.
    pub fn vertices(&self) -> impl Iterator<Item = &VA> {
        self.vertices_added.iter().filter(|va| {
            !self
                .vertices_removed
                .iter()
                .any(|vr| va.id() == vr.add_vertex_id())
        })
    }

    /// Returns an iterator over all active (non-removed) edge-add operations.
    pub fn edges(&self) -> impl Iterator<Item = &EA> {
        self.edges_added.iter().filter(|ea| {
            !self
                .edges_removed
                .iter()
                .any(|er| ea.id() == er.add_edge_id())
        })
    }

    /// Returns a slice of all vertex-add operations ever recorded (`V_A`).
    pub fn all_vertices_added(&self) -> &[VA] {
        &self.vertices_added
    }

    /// Returns a slice of all vertex-remove operations ever recorded (`V_R`).
    pub fn all_vertices_removed(&self) -> &[VR] {
        &self.vertices_removed
    }

    /// Returns a slice of all edge-add operations ever recorded (`E_A`).
    pub fn all_edges_added(&self) -> &[EA] {
        &self.edges_added
    }

    /// Returns a slice of all edge-remove operations ever recorded (`E_R`).
    pub fn all_edges_removed(&self) -> &[ER] {
        &self.edges_removed
    }
}
