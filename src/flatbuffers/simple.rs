use crate::graph::UpdateOperation;
use crate::types::simple;
use flatbuffers::FlatBufferBuilder;

use super::crdt_graph_generated::crdt_graph::fb;
use super::{uuid_from_fb, DecodeError};

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

/// Encodes a single [`simple::Operation`] into a FlatBuffer byte vector.
///
/// The returned bytes are a complete, verified FlatBuffer with file identifier `"CRDT"`.
pub fn encode_operation(op: &simple::Operation) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let op_offset = write_update_operation(&mut builder, op);

    let ops_vec = builder.create_vector(&[op_offset]);
    let log = fb::OperationLog::create(
        &mut builder,
        &fb::OperationLogArgs {
            operations: Some(ops_vec),
        },
    );
    builder.finish(log, Some("CRDT"));
    builder.finished_data().to_vec()
}

/// Encodes multiple operations into a single FlatBuffer byte vector.
pub fn encode_operation_log(ops: &[simple::Operation]) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();

    let offsets: Vec<_> = ops
        .iter()
        .map(|op| write_update_operation(&mut builder, op))
        .collect();

    let ops_vec = builder.create_vector(&offsets);
    let log = fb::OperationLog::create(
        &mut builder,
        &fb::OperationLogArgs {
            operations: Some(ops_vec),
        },
    );
    builder.finish(log, Some("CRDT"));
    builder.finished_data().to_vec()
}

fn write_update_operation<'bldr, A: flatbuffers::Allocator + 'bldr>(
    builder: &mut FlatBufferBuilder<'bldr, A>,
    op: &simple::Operation,
) -> flatbuffers::WIPOffset<fb::UpdateOperation<'bldr>> {
    match op {
        UpdateOperation::AddVertex(v) => {
            let id = fb::Uuid(*v.id.as_bytes());
            let inner = fb::AddVertex::create(builder, &fb::AddVertexArgs { id: Some(&id) });
            fb::UpdateOperation::create(
                builder,
                &fb::UpdateOperationArgs {
                    operation_type: fb::Operation::AddVertex,
                    operation: Some(inner.as_union_value()),
                },
            )
        }
        UpdateOperation::RemoveVertex(v) => {
            let id = fb::Uuid(*v.id.as_bytes());
            let add_vertex_id = fb::Uuid(*v.add_vertex_id.as_bytes());
            let inner = fb::RemoveVertex::create(
                builder,
                &fb::RemoveVertexArgs {
                    id: Some(&id),
                    add_vertex_id: Some(&add_vertex_id),
                },
            );
            fb::UpdateOperation::create(
                builder,
                &fb::UpdateOperationArgs {
                    operation_type: fb::Operation::RemoveVertex,
                    operation: Some(inner.as_union_value()),
                },
            )
        }
        UpdateOperation::AddEdge(e) => {
            let id = fb::Uuid(*e.id.as_bytes());
            let source = fb::Uuid(*e.source.as_bytes());
            let target = fb::Uuid(*e.target.as_bytes());
            let inner = fb::AddEdge::create(
                builder,
                &fb::AddEdgeArgs {
                    id: Some(&id),
                    source: Some(&source),
                    target: Some(&target),
                },
            );
            fb::UpdateOperation::create(
                builder,
                &fb::UpdateOperationArgs {
                    operation_type: fb::Operation::AddEdge,
                    operation: Some(inner.as_union_value()),
                },
            )
        }
        UpdateOperation::RemoveEdge(e) => {
            let id = fb::Uuid(*e.id.as_bytes());
            let add_edge_id = fb::Uuid(*e.add_edge_id.as_bytes());
            let inner = fb::RemoveEdge::create(
                builder,
                &fb::RemoveEdgeArgs {
                    id: Some(&id),
                    add_edge_id: Some(&add_edge_id),
                },
            );
            fb::UpdateOperation::create(
                builder,
                &fb::UpdateOperationArgs {
                    operation_type: fb::Operation::RemoveEdge,
                    operation: Some(inner.as_union_value()),
                },
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Decoding
// ---------------------------------------------------------------------------

/// Decodes a single operation from a FlatBuffer.
pub fn decode_operation(buf: &[u8]) -> Result<simple::Operation, DecodeError> {
    let ops = decode_operation_log(buf)?;
    ops.into_iter()
        .next()
        .ok_or(DecodeError::UnknownOperationType)
}

/// Decodes all operations from a FlatBuffer `OperationLog`.
pub fn decode_operation_log(buf: &[u8]) -> Result<Vec<simple::Operation>, DecodeError> {
    let log = fb::root_as_operation_log(buf)?;
    let operations = log.operations();

    let mut result = Vec::with_capacity(operations.len());
    for fb_op in operations.iter() {
        let op = read_update_operation(&fb_op)?;
        result.push(op);
    }
    Ok(result)
}

fn read_update_operation(
    fb_op: &fb::UpdateOperation<'_>,
) -> Result<simple::Operation, DecodeError> {
    match fb_op.operation_type() {
        fb::Operation::AddVertex => {
            let v = fb_op
                .operation_as_add_vertex()
                .ok_or(DecodeError::UnknownOperationType)?;
            Ok(UpdateOperation::AddVertex(simple::AddVertex {
                id: uuid_from_fb(&v.id().0),
            }))
        }
        fb::Operation::RemoveVertex => {
            let v = fb_op
                .operation_as_remove_vertex()
                .ok_or(DecodeError::UnknownOperationType)?;
            Ok(UpdateOperation::RemoveVertex(crate::types::RemoveVertex {
                id: uuid_from_fb(&v.id().0),
                add_vertex_id: uuid_from_fb(&v.add_vertex_id().0),
            }))
        }
        fb::Operation::AddEdge => {
            let e = fb_op
                .operation_as_add_edge()
                .ok_or(DecodeError::UnknownOperationType)?;
            Ok(UpdateOperation::AddEdge(simple::AddEdge {
                id: uuid_from_fb(&e.id().0),
                source: uuid_from_fb(&e.source().0),
                target: uuid_from_fb(&e.target().0),
            }))
        }
        fb::Operation::RemoveEdge => {
            let e = fb_op
                .operation_as_remove_edge()
                .ok_or(DecodeError::UnknownOperationType)?;
            Ok(UpdateOperation::RemoveEdge(crate::types::RemoveEdge {
                id: uuid_from_fb(&e.id().0),
                add_edge_id: uuid_from_fb(&e.add_edge_id().0),
            }))
        }
        _ => Err(DecodeError::UnknownOperationType),
    }
}
