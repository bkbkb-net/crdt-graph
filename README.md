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

## Usage

First, define concrete types that implement the required traits:

```rust
use crdt_graph::{
    TwoPTwoPAddEdge, TwoPTwoPAddVertex, TwoPTwoPGraph, TwoPTwoPId,
    TwoPTwoPRemoveEdge, TwoPTwoPRemoveVertex, UpdateOperation,
};

type Id = u64;

// -- Vertex add --
#[derive(Clone, Debug)]
struct VA { id: Id }

impl TwoPTwoPId<Id> for VA {
    fn id(&self) -> &Id { &self.id }
}
impl TwoPTwoPAddVertex<Id> for VA {}

// -- Vertex remove --
#[derive(Clone, Debug)]
struct VR { id: Id, add_vertex_id: Id }

impl TwoPTwoPId<Id> for VR {
    fn id(&self) -> &Id { &self.id }
}
impl TwoPTwoPRemoveVertex<Id> for VR {
    fn add_vertex_id(&self) -> &Id { &self.add_vertex_id }
}

// -- Edge add --
#[derive(Clone, Debug)]
struct EA { id: Id, source: Id, target: Id }

impl TwoPTwoPId<Id> for EA {
    fn id(&self) -> &Id { &self.id }
}
impl TwoPTwoPAddEdge<Id> for EA {
    fn source(&self) -> &Id { &self.source }
    fn target(&self) -> &Id { &self.target }
}

// -- Edge remove --
#[derive(Clone, Debug)]
struct ER { id: Id, add_edge_id: Id }

impl TwoPTwoPId<Id> for ER {
    fn id(&self) -> &Id { &self.id }
}
impl TwoPTwoPRemoveEdge<Id> for ER {
    fn add_edge_id(&self) -> &Id { &self.add_edge_id }
}

# fn main() {
// Create two replicas
let mut replica_a: TwoPTwoPGraph<VA, VR, EA, ER, Id> = TwoPTwoPGraph::new();
let mut replica_b: TwoPTwoPGraph<VA, VR, EA, ER, Id> = TwoPTwoPGraph::new();

// Replica A: add vertices (atSource + local downstream)
let op1 = replica_a.prepare(UpdateOperation::AddVertex(VA { id: 1 })).unwrap();
let op2 = replica_a.prepare(UpdateOperation::AddVertex(VA { id: 2 })).unwrap();

// Broadcast to Replica B (downstream)
replica_b.apply_downstream(op1).unwrap();
replica_b.apply_downstream(op2).unwrap();

// Replica B: add an edge
let op3 = replica_b.prepare(UpdateOperation::AddEdge(EA { id: 10, source: 1, target: 2 })).unwrap();
replica_a.apply_downstream(op3).unwrap();

// Both replicas now have the same state
assert!(replica_a.lookup_vertex(&1));
assert!(replica_b.lookup_vertex(&1));
assert_eq!(replica_a.generate_petgraph().edge_count(), 1);
assert_eq!(replica_b.generate_petgraph().edge_count(), 1);
# }
```

### API

| Method | Description |
|--------|-------------|
| `prepare(op)` | Executes atSource checks, applies locally, and returns the operation to broadcast. |
| `apply_downstream(op)` | Applies an operation received from a remote replica. |
| `update_operation(op)` | Convenience wrapper around `prepare` that discards the return value. |
| `lookup_vertex(id)` | Returns `true` if the vertex is in `V_A \ V_R`. |
| `generate_petgraph()` | Converts the current state into a `petgraph::DiGraph`. |

### Preconditions

| Operation | atSource | downstream |
|-----------|----------|------------|
| `addVertex(w)` | — | — |
| `addEdge(u, v)` | `lookup(u) ∧ lookup(v)` | — |
| `removeVertex(w)` | `lookup(w)`, no active edges | `addVertex(w)` delivered |
| `removeEdge(u, v)` | `lookup((u, v))` | `addEdge(u, v)` delivered |
