use crate::graph::UpdateOperation;
use crate::types::string;
use flatbuffers::FlatBufferBuilder;

use super::crdt_graph_with_str_data_generated::crdt_graph::fb_str;
use super::{uuid_from_fb, DecodeError};

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

/// Encodes a single [`string::Operation`] into a FlatBuffer byte vector (file identifier `"CRD3"`).
pub fn encode_operation(op: &string::Operation) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let op_offset = write_update_operation(&mut builder, op);

    let ops_vec = builder.create_vector(&[op_offset]);
    let log = fb_str::OperationLog::create(
        &mut builder,
        &fb_str::OperationLogArgs {
            operations: Some(ops_vec),
        },
    );
    builder.finish(log, Some("CRD3"));
    builder.finished_data().to_vec()
}

/// Encodes multiple operations into a single FlatBuffer byte vector.
pub fn encode_operation_log(ops: &[string::Operation]) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();

    let offsets: Vec<_> = ops
        .iter()
        .map(|op| write_update_operation(&mut builder, op))
        .collect();

    let ops_vec = builder.create_vector(&offsets);
    let log = fb_str::OperationLog::create(
        &mut builder,
        &fb_str::OperationLogArgs {
            operations: Some(ops_vec),
        },
    );
    builder.finish(log, Some("CRD3"));
    builder.finished_data().to_vec()
}

fn write_update_operation<'bldr, A: flatbuffers::Allocator + 'bldr>(
    builder: &mut FlatBufferBuilder<'bldr, A>,
    op: &string::Operation,
) -> flatbuffers::WIPOffset<fb_str::UpdateOperation<'bldr>> {
    match op {
        UpdateOperation::AddVertex(v) => {
            let id = fb_str::Uuid(*v.id.as_bytes());
            let data = v.data.as_ref().map(|d| builder.create_string(d));
            let inner = fb_str::AddVertex::create(
                builder,
                &fb_str::AddVertexArgs {
                    id: Some(&id),
                    data,
                },
            );
            fb_str::UpdateOperation::create(
                builder,
                &fb_str::UpdateOperationArgs {
                    operation_type: fb_str::Operation::AddVertex,
                    operation: Some(inner.as_union_value()),
                },
            )
        }
        UpdateOperation::RemoveVertex(v) => {
            let id = fb_str::Uuid(*v.id.as_bytes());
            let add_vertex_id = fb_str::Uuid(*v.add_vertex_id.as_bytes());
            let inner = fb_str::RemoveVertex::create(
                builder,
                &fb_str::RemoveVertexArgs {
                    id: Some(&id),
                    add_vertex_id: Some(&add_vertex_id),
                },
            );
            fb_str::UpdateOperation::create(
                builder,
                &fb_str::UpdateOperationArgs {
                    operation_type: fb_str::Operation::RemoveVertex,
                    operation: Some(inner.as_union_value()),
                },
            )
        }
        UpdateOperation::AddEdge(e) => {
            let id = fb_str::Uuid(*e.id.as_bytes());
            let source = fb_str::Uuid(*e.source.as_bytes());
            let target = fb_str::Uuid(*e.target.as_bytes());
            let data = e.data.as_ref().map(|d| builder.create_string(d));
            let inner = fb_str::AddEdge::create(
                builder,
                &fb_str::AddEdgeArgs {
                    id: Some(&id),
                    source: Some(&source),
                    target: Some(&target),
                    data,
                },
            );
            fb_str::UpdateOperation::create(
                builder,
                &fb_str::UpdateOperationArgs {
                    operation_type: fb_str::Operation::AddEdge,
                    operation: Some(inner.as_union_value()),
                },
            )
        }
        UpdateOperation::RemoveEdge(e) => {
            let id = fb_str::Uuid(*e.id.as_bytes());
            let add_edge_id = fb_str::Uuid(*e.add_edge_id.as_bytes());
            let inner = fb_str::RemoveEdge::create(
                builder,
                &fb_str::RemoveEdgeArgs {
                    id: Some(&id),
                    add_edge_id: Some(&add_edge_id),
                },
            );
            fb_str::UpdateOperation::create(
                builder,
                &fb_str::UpdateOperationArgs {
                    operation_type: fb_str::Operation::RemoveEdge,
                    operation: Some(inner.as_union_value()),
                },
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Decoding
// ---------------------------------------------------------------------------

/// Decodes a single operation from a string-data FlatBuffer.
pub fn decode_operation(buf: &[u8]) -> Result<string::Operation, DecodeError> {
    let ops = decode_operation_log(buf)?;
    ops.into_iter()
        .next()
        .ok_or(DecodeError::UnknownOperationType)
}

/// Decodes all operations from a string-data FlatBuffer `OperationLog`.
pub fn decode_operation_log(buf: &[u8]) -> Result<Vec<string::Operation>, DecodeError> {
    let log = fb_str::root_as_operation_log(buf)?;
    let operations = log.operations();

    let mut result = Vec::with_capacity(operations.len());
    for fb_op in operations.iter() {
        let op = read_update_operation(&fb_op)?;
        result.push(op);
    }
    Ok(result)
}

fn read_update_operation(
    fb_op: &fb_str::UpdateOperation<'_>,
) -> Result<string::Operation, DecodeError> {
    match fb_op.operation_type() {
        fb_str::Operation::AddVertex => {
            let v = fb_op
                .operation_as_add_vertex()
                .ok_or(DecodeError::UnknownOperationType)?;
            Ok(UpdateOperation::AddVertex(string::AddVertex {
                id: uuid_from_fb(&v.id().0),
                data: v.data().map(|s| s.to_owned()),
            }))
        }
        fb_str::Operation::RemoveVertex => {
            let v = fb_op
                .operation_as_remove_vertex()
                .ok_or(DecodeError::UnknownOperationType)?;
            Ok(UpdateOperation::RemoveVertex(crate::types::RemoveVertex {
                id: uuid_from_fb(&v.id().0),
                add_vertex_id: uuid_from_fb(&v.add_vertex_id().0),
            }))
        }
        fb_str::Operation::AddEdge => {
            let e = fb_op
                .operation_as_add_edge()
                .ok_or(DecodeError::UnknownOperationType)?;
            Ok(UpdateOperation::AddEdge(string::AddEdge {
                id: uuid_from_fb(&e.id().0),
                source: uuid_from_fb(&e.source().0),
                target: uuid_from_fb(&e.target().0),
                data: e.data().map(|s| s.to_owned()),
            }))
        }
        fb_str::Operation::RemoveEdge => {
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
