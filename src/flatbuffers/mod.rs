#[allow(unused_imports, dead_code, clippy::all)]
#[path = "crdt_graph_generated.rs"]
mod crdt_graph_generated;

#[allow(unused_imports, dead_code, clippy::all)]
#[path = "crdt_graph_with_data_generated.rs"]
mod crdt_graph_with_data_generated;

#[allow(unused_imports, dead_code, clippy::all)]
#[path = "crdt_graph_with_str_data_generated.rs"]
mod crdt_graph_with_str_data_generated;

pub mod bytes;
pub mod simple;
pub mod string;

use uuid::Uuid;

/// Error type for FlatBuffer decoding.
#[derive(Debug)]
pub enum DecodeError {
    /// The buffer failed FlatBuffer verification.
    InvalidBuffer(flatbuffers::InvalidFlatbuffer),
    /// An operation had an unknown or NONE union type.
    UnknownOperationType,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::InvalidBuffer(e) => write!(f, "invalid flatbuffer: {e}"),
            DecodeError::UnknownOperationType => write!(f, "unknown operation type in union"),
        }
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodeError::InvalidBuffer(e) => Some(e),
            DecodeError::UnknownOperationType => None,
        }
    }
}

impl From<flatbuffers::InvalidFlatbuffer> for DecodeError {
    fn from(e: flatbuffers::InvalidFlatbuffer) -> Self {
        DecodeError::InvalidBuffer(e)
    }
}

/// Convert a FlatBuffers UUID struct (a `[u8; 16]` newtype) back to `uuid::Uuid`.
fn uuid_from_fb(fb: &[u8; 16]) -> Uuid {
    Uuid::from_bytes(*fb)
}
