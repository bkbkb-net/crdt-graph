use std::fmt::Debug;
use std::hash::Hash;

use crate::TwoPTwoPGraphError;

pub trait TwoPTwoPId<Id> {
    fn id(&self) -> &Id;
}

pub trait TwoPTwoPAddVertex<Id>: TwoPTwoPId<Id> {}

pub trait TwoPTwoPRemoveVertex<Id>: TwoPTwoPId<Id> {
    fn add_vertex_id(&self) -> &Id;
}

pub trait TwoPTwoPAddEdge<Id>: TwoPTwoPId<Id> {
    fn source(&self) -> &Id;
    fn target(&self) -> &Id;
}

pub trait TwoPTwoPRemoveEdge<Id>: TwoPTwoPId<Id> {
    fn add_edge_id(&self) -> &Id;
}

pub enum UpdateType {
    AtSource,
    Downstream,
}

#[derive(Clone, Debug)]
pub enum UpdateOperation<VA, VR, EA, ER> {
    AddVertex(VA),
    RemoveVertex(VR),
    AddEdge(EA),
    RemoveEdge(ER),
}

#[derive(Clone, Debug)]
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
    _phantom: std::marker::PhantomData<I>,
}

impl<VA, VR, EA, ER, I> TwoPTwoPGraph<VA, VR, EA, ER, I>
where
    VA: Clone + TwoPTwoPAddVertex<I>,
    VR: Clone + TwoPTwoPRemoveVertex<I>,
    EA: Clone + TwoPTwoPAddEdge<I>,
    ER: Clone + TwoPTwoPRemoveEdge<I>,
    I: Eq + Hash + Debug + Clone,
{
    pub fn new() -> Self {
        TwoPTwoPGraph {
            vertices_added: Vec::new(),
            vertices_removed: Vec::new(),
            edges_added: Vec::new(),
            edges_removed: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn lookup_vertex(&self, vertex_id: &I) -> bool {
        for va in self.vertices_added.iter() {
            if va.id() == vertex_id {
                for vr in self.vertices_removed.iter() {
                    if vr.add_vertex_id() == vertex_id {
                        return false;
                    }
                }
                return true;
            }
        }
        return false;
    }

    pub fn get_edge_added_from_remove_edge(&self, remove_edge: &ER) -> Option<&EA> {
        for ea in self.edges_added.iter() {
            if ea.id() == remove_edge.add_edge_id() {
                return Some(ea);
            }
        }
        return None;
    }

    pub fn lookup_from_remove_edge(&self, remove_edge: &ER) -> bool {
        match self.get_edge_added_from_remove_edge(remove_edge) {
            Some(edge_added) => {
                if self.lookup_vertex(&edge_added.source())
                    && self.lookup_vertex(&edge_added.target())
                {
                    for er in self.edges_removed.iter() {
                        if er.add_edge_id() == remove_edge.add_edge_id() {
                            return false;
                        }
                    }
                    return true;
                }
                return false;
            }
            None => false,
        }
    }

    pub fn update_operation(
        &mut self,
        update_operation: UpdateOperation<VA, VR, EA, ER>,
    ) -> Result<(), TwoPTwoPGraphError<I>> {
        match update_operation {
            UpdateOperation::AddVertex(vertex) => self.add_vertex(vertex, UpdateType::AtSource),
            UpdateOperation::AddEdge(edge) => self.add_edge(edge, UpdateType::AtSource),
            UpdateOperation::RemoveVertex(vertex) => {
                self.remove_vertex(vertex, UpdateType::AtSource)
            }
            UpdateOperation::RemoveEdge(edge) => self.remove_edge(edge, UpdateType::AtSource),
        }
    }

    pub fn add_vertex(
        &mut self,
        vertex: VA,
        _update_type: UpdateType,
    ) -> Result<(), TwoPTwoPGraphError<I>> {
        // if matches!(update_type, UpdateType::AtSource) {}
        for va in self.vertices_added.iter() {
            if va.id() == vertex.id() {
                return Err(TwoPTwoPGraphError::VertexAlreadyExists(vertex.id().clone()));
            }
        }
        self.vertices_added.push(vertex);

        Ok(())
    }

    pub fn add_edge(
        &mut self,
        edge: EA,
        update_type: UpdateType,
    ) -> Result<(), TwoPTwoPGraphError<I>> {
        if matches!(update_type, UpdateType::AtSource) {
            if !self.lookup_vertex(&edge.source()) {
                return Err(TwoPTwoPGraphError::VertexDoesNotExists(
                    edge.source().clone(),
                ));
            }
            if !self.lookup_vertex(&edge.target()) {
                return Err(TwoPTwoPGraphError::VertexDoesNotExists(
                    edge.target().clone(),
                ));
            }
        }
        for ea in self.edges_added.iter() {
            if ea.id() == edge.id() {
                return Err(TwoPTwoPGraphError::EdgeAlreadyExists(edge.id().clone()));
            }
        }
        self.edges_added.push(edge);

        Ok(())
    }

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
            // pre
            for ea in self.edges_added.iter() {
                let mut found = false;
                for er in self.edges_removed.iter() {
                    if ea.id() == er.add_edge_id() {
                        found = true;
                    }
                }
                if found == false {
                    if ea.source() == vertex.add_vertex_id()
                        || ea.target() == vertex.add_vertex_id()
                    {
                        return Err(TwoPTwoPGraphError::VertexHasEdge(
                            vertex.add_vertex_id().clone(),
                            ea.id().clone(),
                        ));
                    }
                }
            }
        }

        // TODO: pre addVertex(w) delivered

        for vr in self.vertices_removed.iter() {
            if vr.id() == vertex.id() {
                return Err(TwoPTwoPGraphError::VertexAlreadyExists(vertex.id().clone()));
            }
        }
        self.vertices_removed.push(vertex);

        Ok(())
    }

    pub fn remove_edge(
        &mut self,
        remove_edge: ER,
        update_type: UpdateType,
    ) -> Result<(), TwoPTwoPGraphError<I>> {
        if matches!(update_type, UpdateType::AtSource) {
            // pre
            if self.lookup_from_remove_edge(&remove_edge) == false {
                return Err(TwoPTwoPGraphError::EdgeDoesNotExists(
                    remove_edge.add_edge_id().clone(),
                ));
            }
        }

        // TODO: addEdge(w) delivered

        for er in self.edges_removed.iter() {
            if er.id() == remove_edge.id() {
                return Err(TwoPTwoPGraphError::EdgeAlreadyExists(
                    remove_edge.id().clone(),
                ));
            }
        }
        self.edges_removed.push(remove_edge);

        Ok(())
    }

    pub fn generate_petgraph(&self) -> petgraph::graph::DiGraph<VA, EA> {
        let mut graph = petgraph::graph::DiGraph::new();
        let mut vertex_map = std::collections::HashMap::new();
        for va in self.vertices_added.iter() {
            let mut found = false;
            for vr in self.vertices_removed.iter() {
                if va.id() == vr.add_vertex_id() {
                    found = true;
                }
            }
            if found == false {
                let vertex = graph.add_node(va.clone());
                vertex_map.insert(va.id().clone(), vertex);
            }
        }
        for ea in self.edges_added.iter() {
            let mut found = false;
            for er in self.edges_removed.iter() {
                if ea.id() == er.add_edge_id() {
                    found = true;
                }
            }
            if found == false {
                // edge will be added only if source and target vertices are present
                if let Some(source) = vertex_map.get(ea.source()) {
                    if let Some(target) = vertex_map.get(ea.target()) {
                        graph.add_edge(*source, *target, ea.clone());
                    }
                }
            }
        }
        graph
    }
}
