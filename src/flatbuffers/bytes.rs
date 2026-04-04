use crate::graph::UpdateOperation;
use crate::types::bytes;
use flatbuffers::FlatBufferBuilder;

use super::crdt_graph_with_data_generated::crdt_graph::fb_data;
use super::{uuid_from_fb, DecodeError};

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

/// Encodes a single [`bytes::Operation`] into a FlatBuffer byte vector (file identifier `"CRD2"`).
pub fn encode_operation(op: &bytes::Operation) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let op_offset = write_update_operation(&mut builder, op);

    let ops_vec = builder.create_vector(&[op_offset]);
    let log = fb_data::OperationLog::create(
        &mut builder,
        &fb_data::OperationLogArgs {
            operations: Some(ops_vec),
        },
    );
    builder.finish(log, Some("CRD2"));
    builder.finished_data().to_vec()
}

/// Encodes multiple operations into a single FlatBuffer byte vector.
pub fn encode_operation_log(ops: &[bytes::Operation]) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();

    let offsets: Vec<_> = ops
        .iter()
        .map(|op| write_update_operation(&mut builder, op))
        .collect();

    let ops_vec = builder.create_vector(&offsets);
    let log = fb_data::OperationLog::create(
        &mut builder,
        &fb_data::OperationLogArgs {
            operations: Some(ops_vec),
        },
    );
    builder.finish(log, Some("CRD2"));
    builder.finished_data().to_vec()
}

fn write_update_operation<'bldr, A: flatbuffers::Allocator + 'bldr>(
    builder: &mut FlatBufferBuilder<'bldr, A>,
    op: &bytes::Operation,
) -> flatbuffers::WIPOffset<fb_data::UpdateOperation<'bldr>> {
    match op {
        UpdateOperation::AddVertex(v) => {
            let id = fb_data::Uuid(*v.id.as_bytes());
            let data = v.data.as_ref().map(|d| builder.create_vector(d));
            let inner = fb_data::AddVertex::create(
                builder,
                &fb_data::AddVertexArgs {
                    id: Some(&id),
                    data,
                },
            );
            fb_data::UpdateOperation::create(
                builder,
                &fb_data::UpdateOperationArgs {
                    operation_type: fb_data::Operation::AddVertex,
                    operation: Some(inner.as_union_value()),
                },
            )
        }
        UpdateOperation::RemoveVertex(v) => {
            let id = fb_data::Uuid(*v.id.as_bytes());
            let add_vertex_id = fb_data::Uuid(*v.add_vertex_id.as_bytes());
            let inner = fb_data::RemoveVertex::create(
                builder,
                &fb_data::RemoveVertexArgs {
                    id: Some(&id),
                    add_vertex_id: Some(&add_vertex_id),
                },
            );
            fb_data::UpdateOperation::create(
                builder,
                &fb_data::UpdateOperationArgs {
                    operation_type: fb_data::Operation::RemoveVertex,
                    operation: Some(inner.as_union_value()),
                },
            )
        }
        UpdateOperation::AddEdge(e) => {
            let id = fb_data::Uuid(*e.id.as_bytes());
            let source = fb_data::Uuid(*e.source.as_bytes());
            let target = fb_data::Uuid(*e.target.as_bytes());
            let data = e.data.as_ref().map(|d| builder.create_vector(d));
            let inner = fb_data::AddEdge::create(
                builder,
                &fb_data::AddEdgeArgs {
                    id: Some(&id),
                    source: Some(&source),
                    target: Some(&target),
                    data,
                },
            );
            fb_data::UpdateOperation::create(
                builder,
                &fb_data::UpdateOperationArgs {
                    operation_type: fb_data::Operation::AddEdge,
                    operation: Some(inner.as_union_value()),
                },
            )
        }
        UpdateOperation::RemoveEdge(e) => {
            let id = fb_data::Uuid(*e.id.as_bytes());
            let add_edge_id = fb_data::Uuid(*e.add_edge_id.as_bytes());
            let inner = fb_data::RemoveEdge::create(
                builder,
                &fb_data::RemoveEdgeArgs {
                    id: Some(&id),
                    add_edge_id: Some(&add_edge_id),
                },
            );
            fb_data::UpdateOperation::create(
                builder,
                &fb_data::UpdateOperationArgs {
                    operation_type: fb_data::Operation::RemoveEdge,
                    operation: Some(inner.as_union_value()),
                },
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Decoding
// ---------------------------------------------------------------------------

/// Decodes a single operation from a with-data FlatBuffer.
pub fn decode_operation(buf: &[u8]) -> Result<bytes::Operation, DecodeError> {
    let ops = decode_operation_log(buf)?;
    ops.into_iter()
        .next()
        .ok_or(DecodeError::UnknownOperationType)
}

/// Decodes all operations from a with-data FlatBuffer `OperationLog`.
pub fn decode_operation_log(buf: &[u8]) -> Result<Vec<bytes::Operation>, DecodeError> {
    let log = fb_data::root_as_operation_log(buf)?;
    let operations = log.operations();

    let mut result = Vec::with_capacity(operations.len());
    for fb_op in operations.iter() {
        let op = read_update_operation(&fb_op)?;
        result.push(op);
    }
    Ok(result)
}

fn read_update_operation(
    fb_op: &fb_data::UpdateOperation<'_>,
) -> Result<bytes::Operation, DecodeError> {
    match fb_op.operation_type() {
        fb_data::Operation::AddVertex => {
            let v = fb_op
                .operation_as_add_vertex()
                .ok_or(DecodeError::UnknownOperationType)?;
            Ok(UpdateOperation::AddVertex(bytes::AddVertex {
                id: uuid_from_fb(&v.id().0),
                data: v.data().map(|d| d.bytes().to_vec()),
            }))
        }
        fb_data::Operation::RemoveVertex => {
            let v = fb_op
                .operation_as_remove_vertex()
                .ok_or(DecodeError::UnknownOperationType)?;
            Ok(UpdateOperation::RemoveVertex(crate::types::RemoveVertex {
                id: uuid_from_fb(&v.id().0),
                add_vertex_id: uuid_from_fb(&v.add_vertex_id().0),
            }))
        }
        fb_data::Operation::AddEdge => {
            let e = fb_op
                .operation_as_add_edge()
                .ok_or(DecodeError::UnknownOperationType)?;
            Ok(UpdateOperation::AddEdge(bytes::AddEdge {
                id: uuid_from_fb(&e.id().0),
                source: uuid_from_fb(&e.source().0),
                target: uuid_from_fb(&e.target().0),
                data: e.data().map(|d| d.bytes().to_vec()),
            }))
        }
        fb_data::Operation::RemoveEdge => {
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
