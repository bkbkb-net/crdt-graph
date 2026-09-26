#![doc = include_str!("../README.md")]

mod error;
pub mod flatbuffers;
mod graph;
mod transaction;
pub mod types;

pub use error::TwoPTwoPGraphError;
pub use graph::{
    TwoPTwoPAddEdge, TwoPTwoPAddVertex, TwoPTwoPGraph, TwoPTwoPId, TwoPTwoPRemoveEdge,
    TwoPTwoPRemoveVertex, UpdateOperation, UpdateType,
};
pub use transaction::{GraphTransaction, TransactionStack, DEFAULT_TRANSACTION_HISTORY_LIMIT};
pub use uuid::Uuid;
