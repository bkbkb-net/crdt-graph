use std::fmt::Debug;

use thiserror::Error;

/// Errors that can occur when performing operations on a [`TwoPTwoPGraph`](crate::TwoPTwoPGraph).
#[derive(Error, Debug)]
pub enum TwoPTwoPGraphError<Id>
where
    Id: Debug,
{
    /// A vertex with the given ID has already been added.
    #[error("Vertex {0} already exists")]
    VertexAlreadyExists(Id),
    /// The referenced vertex does not exist in `V_A \ V_R`.
    #[error("Vertex {0} does not exists")]
    VertexDoesNotExists(Id),
    /// An edge with the given ID has already been added.
    #[error("Edge already exists")]
    EdgeAlreadyExists(Id),
    /// The referenced edge does not exist in `E_A \ E_R`.
    #[error("Edge does not exists")]
    EdgeDoesNotExists(Id),
    /// Cannot remove the vertex because it still has an active (non-removed) edge.
    #[error("Vertex {0} has edge {1}")]
    VertexHasEdge(Id, Id),
    /// Downstream precondition failure: the corresponding `addVertex` has not been delivered yet.
    #[error("addVertex({0}) not yet delivered")]
    AddVertexNotDelivered(Id),
    /// Downstream precondition failure: the corresponding `addEdge` has not been delivered yet.
    #[error("addEdge({0}) not yet delivered")]
    AddEdgeNotDelivered(Id),
}
