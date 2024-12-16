use std::fmt::Debug;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TwoPTwoPGraphError<Id>
where
    Id: Debug,
{
    // #[error("data store disconnected")]
    // Disconnect(#[from] io::Error),
    // #[error("the data for key `{0}` is not available")]
    // Redaction(String),
    // #[error("invalid header (expected {expected:?}, found {found:?})")]
    // InvalidHeader { expected: String, found: String },
    // #[error("unknown data store error")]
    // Unknown,
    #[error("Vertex {0} already exists")]
    VertexAlreadyExists(Id),
    #[error("Vertex {0} does not exists")]
    VertexDoesNotExists(Id),
    #[error("Edge already exists")]
    EdgeAlreadyExists(Id),
    #[error("Edge does not exists")]
    EdgeDoesNotExists(Id),
    #[error("Vertex {0} has edge {1}")]
    VertexHasEdge(Id, Id),
}
