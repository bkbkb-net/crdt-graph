#![doc = include_str!("../README.md")]

mod error;
pub mod flatbuffers;
mod graph;
pub mod types;

pub use error::TwoPTwoPGraphError;
pub use graph::{
    TwoPTwoPAddEdge, TwoPTwoPAddVertex, TwoPTwoPGraph, TwoPTwoPId, TwoPTwoPRemoveEdge,
    TwoPTwoPRemoveVertex, UpdateOperation, UpdateType,
};
pub use uuid::Uuid;
