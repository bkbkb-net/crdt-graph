use std::fmt::Debug;
use std::hash::Hash;

use crate::TwoPTwoPGraphError;

pub trait TwoPTwoPVertex<Id> {
    fn id(&self) -> &Id;
}

pub trait TwoPTwoPEdge<Id> {
    fn id(&self) -> &Id;
    fn source(&self) -> &Id;
    fn target(&self) -> &Id;
}

pub enum UpdateType {
    AtSource,
    Downstream,
}

pub struct TwoPTwoPGraph<V, E, I>
where
    V: Clone + TwoPTwoPVertex<I>,
    E: Clone + TwoPTwoPEdge<I>,
    I: Eq + Hash + Debug + Clone,
{
    vertices_added: Vec<V>,
    vertices_removed: Vec<V>,
    edges_added: Vec<E>,
    edges_removed: Vec<E>,
    _phantom: std::marker::PhantomData<I>,
}

impl<V, E, I> TwoPTwoPGraph<V, E, I>
where
    V: Clone + TwoPTwoPVertex<I>,
    E: Clone + TwoPTwoPEdge<I>,
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
                for ve in self.vertices_removed.iter() {
                    if ve.id() == vertex_id {
                        return false;
                    }
                }
                return true;
            }
        }
        return false;
    }

    pub fn lookup_edge(&self, edge: &E) -> bool {
        if self.lookup_vertex(&edge.source()) && self.lookup_vertex(&edge.target()) {
            for ea in self.edges_added.iter() {
                if ea.id() == edge.id() {
                    for er in self.edges_removed.iter() {
                        if er.id() == edge.id() {
                            return false;
                        }
                    }
                    return true;
                }
            }
        }

        return false;
    }

    pub fn add_vertex(
        &mut self,
        vertex: V,
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
        edge: E,
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
        vertex: V,
        update_type: UpdateType,
    ) -> Result<(), TwoPTwoPGraphError<I>> {
        if matches!(update_type, UpdateType::AtSource) {
            // pre
            if !self.lookup_vertex(vertex.id()) {
                return Err(TwoPTwoPGraphError::VertexDoesNotExists(vertex.id().clone()));
            }
            // pre
            for ea in self.edges_added.iter() {
                let mut found = false;
                for er in self.edges_removed.iter() {
                    if ea.id() == er.id() {
                        found = true;
                    }
                }
                if found == false {
                    if ea.source() == vertex.id() || ea.target() == vertex.id() {
                        return Err(TwoPTwoPGraphError::VertexHasEdge(
                            vertex.id().clone(),
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
        edge: E,
        update_type: UpdateType,
    ) -> Result<(), TwoPTwoPGraphError<I>> {
        if matches!(update_type, UpdateType::AtSource) {
            // pre
            if self.lookup_edge(&edge) == false {
                return Err(TwoPTwoPGraphError::EdgeDoesNotExists(edge.id().clone()));
            }
        }

        // TODO: addEdge(w) delivered

        for er in self.edges_removed.iter() {
            if er.id() == edge.id() {
                return Err(TwoPTwoPGraphError::EdgeAlreadyExists(edge.id().clone()));
            }
        }
        self.edges_removed.push(edge);

        Ok(())
    }

    // fn ea_divide_er<T: Fn(&E)>(&self, callback: T) {
    //     for ea in self.edges_added.iter() {
    //         let mut found = false;
    //         for er in self.edges_removed.iter() {
    //             if ea.id() == er.id() {
    //                 found = true;
    //             }
    //         }
    //         if found == false {
    //             callback(ea);
    //         }
    //     }
    // }

    pub fn into_petgraph(self) -> petgraph::graph::DiGraph<V, E> {
        let mut graph = petgraph::graph::DiGraph::new();
        let mut vertex_map = std::collections::HashMap::new();
        for va in self.vertices_added.iter() {
            let mut found = false;
            for vr in self.vertices_removed.iter() {
                if va.id() == vr.id() {
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
                if ea.id() == er.id() {
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
