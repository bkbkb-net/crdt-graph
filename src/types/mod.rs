use crate::graph::{TwoPTwoPId, TwoPTwoPRemoveEdge, TwoPTwoPRemoveVertex};
use uuid::Uuid;

pub mod bytes;
pub mod simple;
pub mod string;

// ===========================================================================
// Shared remove-operation types (identical across all graph variants)
// ===========================================================================

/// A vertex-remove operation. Only references the original add by ID.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RemoveVertex {
    pub id: Uuid,
    pub add_vertex_id: Uuid,
}

impl TwoPTwoPId<Uuid> for RemoveVertex {
    fn id(&self) -> &Uuid {
        &self.id
    }
}

impl TwoPTwoPRemoveVertex<Uuid> for RemoveVertex {
    fn add_vertex_id(&self) -> &Uuid {
        &self.add_vertex_id
    }
}

/// An edge-remove operation. Only references the original add by ID.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RemoveEdge {
    pub id: Uuid,
    pub add_edge_id: Uuid,
}

impl TwoPTwoPId<Uuid> for RemoveEdge {
    fn id(&self) -> &Uuid {
        &self.id
    }
}

impl TwoPTwoPRemoveEdge<Uuid> for RemoveEdge {
    fn add_edge_id(&self) -> &Uuid {
        &self.add_edge_id
    }
}
