use crate::graph::{
    TwoPTwoPAddEdge, TwoPTwoPAddVertex, TwoPTwoPGraph, TwoPTwoPId, UpdateOperation,
};
use super::{RemoveEdge, RemoveVertex};
use uuid::Uuid;

/// A vertex-add operation carrying only an identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AddVertex {
    pub id: Uuid,
}

impl TwoPTwoPId<Uuid> for AddVertex {
    fn id(&self) -> &Uuid {
        &self.id
    }
}

impl TwoPTwoPAddVertex<Uuid> for AddVertex {}

/// An edge-add operation carrying only identifiers.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AddEdge {
    pub id: Uuid,
    pub source: Uuid,
    pub target: Uuid,
}

impl TwoPTwoPId<Uuid> for AddEdge {
    fn id(&self) -> &Uuid {
        &self.id
    }
}

impl TwoPTwoPAddEdge<Uuid> for AddEdge {
    fn source(&self) -> &Uuid {
        &self.source
    }
    fn target(&self) -> &Uuid {
        &self.target
    }
}

/// A graph whose vertices and edges carry no payload beyond their IDs.
pub type Graph = TwoPTwoPGraph<AddVertex, RemoveVertex, AddEdge, RemoveEdge, Uuid>;

/// An update operation for [`Graph`].
pub type Operation = UpdateOperation<AddVertex, RemoveVertex, AddEdge, RemoveEdge>;

impl From<AddVertex> for Operation {
    fn from(v: AddVertex) -> Self {
        UpdateOperation::AddVertex(v)
    }
}

impl From<RemoveVertex> for Operation {
    fn from(v: RemoveVertex) -> Self {
        UpdateOperation::RemoveVertex(v)
    }
}

impl From<AddEdge> for Operation {
    fn from(e: AddEdge) -> Self {
        UpdateOperation::AddEdge(e)
    }
}

impl From<RemoveEdge> for Operation {
    fn from(e: RemoveEdge) -> Self {
        UpdateOperation::RemoveEdge(e)
    }
}
