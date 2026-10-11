use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use crate::TwoPTwoPGraphError;

/// Common trait for all operation types, providing a unique identifier.
pub trait TwoPTwoPId<Id> {
    /// Returns the unique identifier of this operation.
    fn id(&self) -> &Id;
}

/// Marker trait for a vertex-add operation.
pub trait TwoPTwoPAddVertex<Id>: TwoPTwoPId<Id> {}

/// Trait for a vertex-remove operation, linking back to the original add.
pub trait TwoPTwoPRemoveVertex<Id>: TwoPTwoPId<Id> {
    /// Returns the ID of the corresponding `addVertex` operation.
    fn add_vertex_id(&self) -> &Id;
}

/// Trait for an edge-add operation, specifying source and target vertices.
pub trait TwoPTwoPAddEdge<Id>: TwoPTwoPId<Id> {
    /// Returns the source vertex ID.
    fn source(&self) -> &Id;
    /// Returns the target vertex ID.
    fn target(&self) -> &Id;
}

/// Trait for an edge-remove operation, linking back to the original add.
pub trait TwoPTwoPRemoveEdge<Id>: TwoPTwoPId<Id> {
    /// Returns the ID of the corresponding `addEdge` operation.
    fn add_edge_id(&self) -> &Id;
}

/// Distinguishes the two phases of an op-based CRDT update.
pub enum UpdateType {
    /// Executed only on the originating replica; checks preconditions.
    AtSource,
    /// Executed on all replicas; applies the actual state change.
    Downstream,
}

/// An update operation that can be applied to a [`TwoPTwoPGraph`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateOperation<VA, VR, EA, ER> {
    AddVertex(VA),
    RemoveVertex(VR),
    AddEdge(EA),
    RemoveEdge(ER),
}

/// An op-based 2P2P-Graph CRDT.
///
/// Maintains four sets corresponding to the paper's payload:
/// - `V_A` — vertices added
/// - `V_R` — vertices removed
/// - `E_A` — edges added
/// - `E_R` — edges removed
///
/// Generic parameters:
/// - `VA` / `VR` — vertex add / remove operation types
/// - `EA` / `ER` — edge add / remove operation types
/// - `I` — the shared identifier type
///
/// The four sets are kept as append-only vectors: their order is the
/// history, which is what serialisation writes and the `all_*` slices
/// expose. Beside them sits an index derived from them and nothing else
/// (`Index`), so that membership and liveness are expected O(1) and an
/// edge's incidence costs O(log degree), rather than a scan of the
/// history. The identifier's `Clone` may panic without corrupting the
/// index -- every key is cloned before anything changes -- but its
/// `Hash` and `Eq` must not. The index is never
/// written anywhere; two graphs with the same four sets have the same
/// index.
#[derive(Clone)]
pub struct TwoPTwoPGraph<VA, VR, EA, ER, I>
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
    index: Index<I>,
}

/// What the four sets imply, kept so nothing has to be searched for.
#[derive(Clone)]
struct Index<I> {
    /// `V_A`: add id → position.
    vertex_at: HashMap<I, usize>,
    /// `E_A`: add id → position.
    edge_at: HashMap<I, usize>,
    /// Add ids `V_R` removes. A vertex is gone once any replica removed
    /// it; concurrent replicas may remove the same add under two remove
    /// ids, and both stay in the history.
    vertex_removed: HashSet<I>,
    /// Add ids `E_R` removes, likewise.
    edge_removed: HashSet<I>,
    /// The ids of the remove operations themselves, so that delivering
    /// the same one twice is refused.
    vertex_remove_ids: HashSet<I>,
    edge_remove_ids: HashSet<I>,
    /// Edges in `E_A \ E_R` touching each vertex id, by position in
    /// `E_A`, so the first is the earliest: the witness `VertexHasEdge`
    /// reports. Whether the other endpoint is present does not matter --
    /// the precondition is about edges, not visible edges. A self-loop
    /// is one entry. Keyed by id whether or not the vertex has arrived,
    /// because downstream an edge may precede its vertices.
    incident: HashMap<I, BTreeSet<usize>>,
}

impl<I> Default for Index<I> {
    fn default() -> Self {
        Self {
            vertex_at: HashMap::new(),
            edge_at: HashMap::new(),
            vertex_removed: HashSet::new(),
            edge_removed: HashSet::new(),
            vertex_remove_ids: HashSet::new(),
            edge_remove_ids: HashSet::new(),
            incident: HashMap::new(),
        }
    }
}

/// The history, which is the state; the index is derived and left out.
impl<VA, VR, EA, ER, I> Debug for TwoPTwoPGraph<VA, VR, EA, ER, I>
where
    VA: Clone + Debug + TwoPTwoPAddVertex<I>,
    VR: Clone + Debug + TwoPTwoPRemoveVertex<I>,
    EA: Clone + Debug + TwoPTwoPAddEdge<I>,
    ER: Clone + Debug + TwoPTwoPRemoveEdge<I>,
    I: Eq + Hash + Debug + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TwoPTwoPGraph")
            .field("vertices_added", &self.vertices_added)
            .field("vertices_removed", &self.vertices_removed)
            .field("edges_added", &self.edges_added)
            .field("edges_removed", &self.edges_removed)
            .finish()
    }
}

impl<VA, VR, EA, ER, I> Default for TwoPTwoPGraph<VA, VR, EA, ER, I>
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

impl<VA, VR, EA, ER, I> TwoPTwoPGraph<VA, VR, EA, ER, I>
where
    VA: Clone + TwoPTwoPAddVertex<I>,
    VR: Clone + TwoPTwoPRemoveVertex<I>,
    EA: Clone + TwoPTwoPAddEdge<I>,
    ER: Clone + TwoPTwoPRemoveEdge<I>,
    I: Eq + Hash + Debug + Clone,
{
    /// Creates an empty graph with all four sets initialized to ∅.
    pub fn new() -> Self {
        TwoPTwoPGraph {
            vertices_added: Vec::new(),
            vertices_removed: Vec::new(),
            edges_added: Vec::new(),
            edges_removed: Vec::new(),
            index: Index::default(),
        }
    }

    /// Returns `true` if the vertex is in `V_A \ V_R` (added and not removed).
    pub fn lookup_vertex(&self, vertex_id: &I) -> bool {
        self.index.vertex_at.contains_key(vertex_id)
            && !self.index.vertex_removed.contains(vertex_id)
    }

    /// The vertex-add operation with this id, removed or not.
    pub fn vertex_added(&self, vertex_id: &I) -> Option<&VA> {
        self.index
            .vertex_at
            .get(vertex_id)
            .map(|&at| &self.vertices_added[at])
    }

    /// The edge-add operation with this id, removed or not.
    pub fn edge_added(&self, edge_id: &I) -> Option<&EA> {
        self.index
            .edge_at
            .get(edge_id)
            .map(|&at| &self.edges_added[at])
    }

    /// Returns `true` if the edge was added and some replica removed it:
    /// it is in `E_A` and referenced from `E_R`.
    pub fn edge_is_removed(&self, edge_id: &I) -> bool {
        self.index.edge_removed.contains(edge_id)
    }

    /// Edges in `E_A \ E_R` touching the vertex, in `E_A` order, whether
    /// or not their other endpoint is present -- the set the `removeVertex`
    /// precondition is about. A self-loop appears once.
    pub fn incident_edges(&self, vertex_id: &I) -> impl Iterator<Item = &EA> {
        self.index
            .incident
            .get(vertex_id)
            .into_iter()
            .flatten()
            .map(|&at| &self.edges_added[at])
    }

    /// Returns the edge-add operation referenced by a given edge-remove operation, if present.
    pub fn get_edge_added_from_remove_edge(&self, remove_edge: &ER) -> Option<&EA> {
        self.edge_added(remove_edge.add_edge_id())
    }

    /// Returns `true` if the edge referenced by `remove_edge` exists in `E_A \ E_R`
    /// and both of its endpoint vertices are currently in `V_A \ V_R`.
    pub fn lookup_from_remove_edge(&self, remove_edge: &ER) -> bool {
        self.get_edge_added_from_remove_edge(remove_edge)
            .is_some_and(|edge_added| {
                self.lookup_vertex(edge_added.source())
                    && self.lookup_vertex(edge_added.target())
                    && !self.index.edge_removed.contains(remove_edge.add_edge_id())
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
        if self.index.vertex_at.contains_key(vertex.id()) {
            return Err(TwoPTwoPGraphError::VertexAlreadyExists(vertex.id().clone()));
        }
        let id = vertex.id().clone();
        self.index.vertex_at.insert(id, self.vertices_added.len());
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
        if self.index.edge_at.contains_key(edge.id()) {
            return Err(TwoPTwoPGraphError::EdgeAlreadyExists(edge.id().clone()));
        }
        // Every key cloned before anything changes, so a clone that panics
        // leaves the graph as it was rather than an index pointing past
        // the end of its history.
        let (id, source, target) = (
            edge.id().clone(),
            edge.source().clone(),
            edge.target().clone(),
        );
        let at = self.edges_added.len();
        self.index.edge_at.insert(id, at);
        for end in [source, target] {
            self.index.incident.entry(end).or_default().insert(at);
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
            // pre: E ⊆ V × V — vertex has no active edges. The earliest
            // such edge is the one reported.
            if let Some(edge) = self.incident_edges(vertex.add_vertex_id()).next() {
                return Err(TwoPTwoPGraphError::VertexHasEdge(
                    vertex.add_vertex_id().clone(),
                    edge.id().clone(),
                ));
            }
        }

        if matches!(update_type, UpdateType::Downstream) {
            // pre: addVertex(w) delivered
            if !self.index.vertex_at.contains_key(vertex.add_vertex_id()) {
                return Err(TwoPTwoPGraphError::AddVertexNotDelivered(
                    vertex.add_vertex_id().clone(),
                ));
            }
        }

        if self.index.vertex_remove_ids.contains(vertex.id()) {
            return Err(TwoPTwoPGraphError::VertexAlreadyExists(vertex.id().clone()));
        }
        let (id, removes) = (vertex.id().clone(), vertex.add_vertex_id().clone());
        self.index.vertex_remove_ids.insert(id);
        self.index.vertex_removed.insert(removes);
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
            if !self.index.edge_at.contains_key(remove_edge.add_edge_id()) {
                return Err(TwoPTwoPGraphError::AddEdgeNotDelivered(
                    remove_edge.add_edge_id().clone(),
                ));
            }
        }

        if self.index.edge_remove_ids.contains(remove_edge.id()) {
            return Err(TwoPTwoPGraphError::EdgeAlreadyExists(
                remove_edge.id().clone(),
            ));
        }
        let (id, removes) = (remove_edge.id().clone(), remove_edge.add_edge_id().clone());
        // Checked above, at source and downstream alike.
        let at = self.index.edge_at[&removes];
        self.index.edge_remove_ids.insert(id);
        // Only the first remove of an edge takes it out of the incidence
        // sets; a concurrent second one is history, not a change.
        if self.index.edge_removed.insert(removes) {
            let edge = &self.edges_added[at];
            for end in [edge.source(), edge.target()] {
                if let Some(edges) = self.index.incident.get_mut(end) {
                    edges.remove(&at);
                    if edges.is_empty() {
                        self.index.incident.remove(end);
                    }
                }
            }
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
        let mut vertex_map = HashMap::new();
        for va in self.vertices() {
            let vertex = graph.add_node(va.clone());
            vertex_map.insert(va.id().clone(), vertex);
        }
        for ea in self.edges() {
            if let (Some(&source), Some(&target)) =
                (vertex_map.get(ea.source()), vertex_map.get(ea.target()))
            {
                graph.add_edge(source, target, ea.clone());
            }
        }
        graph
    }

    /// Returns the number of active vertices (`V_A \ V_R`).
    pub fn vertex_count(&self) -> usize {
        // A remove is accepted only for an add that was delivered, so the
        // removed set is a subset of the added one.
        self.vertices_added.len() - self.index.vertex_removed.len()
    }

    /// Returns the number of active edges (`E_A \ E_R`).
    pub fn edge_count(&self) -> usize {
        self.edges_added.len() - self.index.edge_removed.len()
    }

    /// Returns `true` if the graph has no active vertices and no active edges.
    pub fn is_empty(&self) -> bool {
        self.vertex_count() == 0 && self.edge_count() == 0
    }

    /// Returns an iterator over all active (non-removed) vertex-add operations.
    pub fn vertices(&self) -> impl Iterator<Item = &VA> {
        self.vertices_added
            .iter()
            .filter(|va| !self.index.vertex_removed.contains(va.id()))
    }

    /// Returns an iterator over all active (non-removed) edge-add operations.
    pub fn edges(&self) -> impl Iterator<Item = &EA> {
        self.edges_added
            .iter()
            .filter(|ea| !self.index.edge_removed.contains(ea.id()))
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
