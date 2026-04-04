# crdt-graph

An op-based **2P2P-Graph CRDT** implementation in Rust.

Based on the 2P2P-Graph specification from Shapiro et al., *"A comprehensive study of Convergent and Commutative Replicated Data Types"* (2011).

## Overview

A 2P2P-Graph (Two-Phase Two-Phase Graph) is a conflict-free replicated data type that models a directed graph with add and remove operations for both vertices and edges. Each replica maintains four sets:

| Set | Description |
|-----|-------------|
| `V_A` | Vertices added |
| `V_R` | Vertices removed |
| `E_A` | Edges added |
| `E_R` | Edges removed |

Updates follow the **op-based CRDT** model with two phases:

- **atSource** — Precondition checks executed only on the originating replica.
- **downstream** — State mutations applied on all replicas (including the source).

## Features

- **Three built-in graph variants** — ID-only (`simple`), binary payload (`bytes`), and string payload (`string`).
- **FlatBuffers serialization** — Compact binary encoding with 16-byte inline UUID structs (zero-copy).
- **UUID v7 identifiers** — Time-ordered, globally unique IDs via `uuid::Uuid`.
- **`petgraph` integration** — Convert CRDT state to a standard `DiGraph` for graph algorithms.
- **Convenience traits** — `Default`, `PartialEq`, `Eq`, `Hash`, `From` conversions on all operation types.
- **State inspection** — `vertex_count()`, `edge_count()`, `is_empty()`, `vertices()`, `edges()` iterators.

## Quick Start

Using the built-in `simple` types (UUID-based, no payload):

```rust
use crdt_graph::types::simple::{self, AddVertex, AddEdge, Graph};
use crdt_graph::types::{RemoveEdge, RemoveVertex};
use crdt_graph::Uuid;

# fn main() {
let mut replica_a = Graph::new();
let mut replica_b = Graph::new();

let v1 = Uuid::now_v7();
let v2 = Uuid::now_v7();

// Replica A: add vertices
let op1 = replica_a.prepare(AddVertex { id: v1 }.into()).unwrap();
let op2 = replica_a.prepare(AddVertex { id: v2 }.into()).unwrap();

// Broadcast to Replica B
replica_b.apply_downstream(op1).unwrap();
replica_b.apply_downstream(op2).unwrap();

// Replica B: add an edge
let e1 = Uuid::now_v7();
let op3 = replica_b.prepare(AddEdge { id: e1, source: v1, target: v2 }.into()).unwrap();
replica_a.apply_downstream(op3).unwrap();

// Both replicas have converged
assert_eq!(replica_a.vertex_count(), 2);
assert_eq!(replica_a.edge_count(), 1);
assert_eq!(replica_b.vertex_count(), 2);
assert_eq!(replica_b.edge_count(), 1);
# }
```

### Custom Types

You can also define your own types by implementing the required traits:

```rust
use crdt_graph::{
    TwoPTwoPAddEdge, TwoPTwoPAddVertex, TwoPTwoPGraph, TwoPTwoPId,
    TwoPTwoPRemoveEdge, TwoPTwoPRemoveVertex, UpdateOperation,
};

type Id = u64;

#[derive(Clone, Debug)]
struct VA { id: Id }
impl TwoPTwoPId<Id> for VA { fn id(&self) -> &Id { &self.id } }
impl TwoPTwoPAddVertex<Id> for VA {}

#[derive(Clone, Debug)]
struct VR { id: Id, add_vertex_id: Id }
impl TwoPTwoPId<Id> for VR { fn id(&self) -> &Id { &self.id } }
impl TwoPTwoPRemoveVertex<Id> for VR { fn add_vertex_id(&self) -> &Id { &self.add_vertex_id } }

#[derive(Clone, Debug)]
struct EA { id: Id, source: Id, target: Id }
impl TwoPTwoPId<Id> for EA { fn id(&self) -> &Id { &self.id } }
impl TwoPTwoPAddEdge<Id> for EA {
    fn source(&self) -> &Id { &self.source }
    fn target(&self) -> &Id { &self.target }
}

#[derive(Clone, Debug)]
struct ER { id: Id, add_edge_id: Id }
impl TwoPTwoPId<Id> for ER { fn id(&self) -> &Id { &self.id } }
impl TwoPTwoPRemoveEdge<Id> for ER { fn add_edge_id(&self) -> &Id { &self.add_edge_id } }

# fn main() {
let mut graph: TwoPTwoPGraph<VA, VR, EA, ER, Id> = TwoPTwoPGraph::new();
graph.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
graph.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();
graph.prepare(UpdateOperation::AddEdge(EA { id: 10, source: 1, target: 2 })).unwrap();
assert_eq!(graph.vertex_count(), 2);
assert_eq!(graph.edge_count(), 1);
# }
```

## Built-in Graph Variants

| Module | Payload | FlatBuffers File ID |
|--------|---------|---------------------|
| `types::simple` | None | `"CRDT"` |
| `types::bytes` | `Option<Vec<u8>>` | `"CRD2"` |
| `types::string` | `Option<String>` | `"CRD3"` |

Each variant provides: `AddVertex`, `AddEdge`, `Graph` (type alias), `Operation` (type alias).  
`RemoveVertex` and `RemoveEdge` are shared across all variants (`crdt_graph::types::{RemoveVertex, RemoveEdge}`).

## FlatBuffers Serialization

```rust
use crdt_graph::types::simple::{AddVertex, Graph};
use crdt_graph::flatbuffers::simple as fb;
use crdt_graph::Uuid;

# fn main() {
let mut graph = Graph::new();
let v1 = Uuid::now_v7();
let op = graph.prepare(AddVertex { id: v1 }.into()).unwrap();

// Encode
let bytes = fb::encode_operation(&op);

// Decode
let decoded = fb::decode_operation(&bytes).unwrap();
# }
```

Batch encoding/decoding is also supported via `encode_operation_log()` / `decode_operation_log()`.

## API

| Method | Description |
|--------|-------------|
| `prepare(op)` | Executes atSource checks, applies locally, and returns the operation to broadcast. |
| `apply_downstream(op)` | Applies an operation received from a remote replica. |
| `update_operation(op)` | Convenience wrapper around `prepare` that discards the return value. |
| `lookup_vertex(id)` | Returns `true` if the vertex is in `V_A \ V_R`. |
| `vertex_count()` | Number of active (non-removed) vertices. |
| `edge_count()` | Number of active (non-removed) edges. |
| `is_empty()` | `true` if no active vertices or edges. |
| `vertices()` | Iterator over active vertices. |
| `edges()` | Iterator over active edges. |
| `generate_petgraph()` | Converts the current state into a `petgraph::DiGraph`. |

## Preconditions

| Operation | atSource | downstream |
|-----------|----------|------------|
| `addVertex(w)` | — | — |
| `addEdge(u, v)` | `lookup(u) ∧ lookup(v)` | — |
| `removeVertex(w)` | `lookup(w)`, no active edges | `addVertex(w)` delivered |
| `removeEdge(u, v)` | `lookup((u, v))` | `addEdge(u, v)` delivered |
