use std::collections::HashSet;
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
    V: TwoPTwoPVertex<I>,
    E: TwoPTwoPEdge<I>,
    I: Eq + Hash,
{
    vertices_added: Vec<V>,
    vertices_removed: Vec<V>,
    edges_added: Vec<E>,
    edges_removed: Vec<E>,
    _phantom: std::marker::PhantomData<I>,
}

impl<V, E, I> TwoPTwoPGraph<V, E, I>
where
    V: TwoPTwoPVertex<I>,
    E: TwoPTwoPEdge<I>,
    I: Eq + Hash,
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
    ) -> Result<(), TwoPTwoPGraphError> {
        // if matches!(update_type, UpdateType::AtSource) {}
        for va in self.vertices_added.iter() {
            if va.id() == vertex.id() {
                return Err(TwoPTwoPGraphError::VertexAlreadyExists);
            }
        }
        self.vertices_added.push(vertex);

        Ok(())
    }

    pub fn add_edge(&mut self, edge: E, update_type: UpdateType) -> Result<(), TwoPTwoPGraphError> {
        if matches!(update_type, UpdateType::AtSource) {
            if !self.lookup_vertex(&edge.source()) || !self.lookup_vertex(&edge.target()) {
                return Err(TwoPTwoPGraphError::VertexDoesNotExist);
            }
        }
        for ea in self.edges_added.iter() {
            if ea.id() == edge.id() {
                return Err(TwoPTwoPGraphError::EdgeAlreadyExists);
            }
        }
        self.edges_added.push(edge);

        Ok(())
    }

    // pub fn remove_vertex(&mut self, vertex: V) {
    //     self.vertices_removed.insert(vertex);
    //     self.vertices_added.remove(&vertex);
    // }

    // pub fn remove_edge(&mut self, edge: E) {
    //     self.edges_removed.insert(edge);
    //     self.edges_added.remove(&edge);
    // }
}
