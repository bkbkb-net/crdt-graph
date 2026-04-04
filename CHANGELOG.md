# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2025-04-05

### Added

- **Built-in graph types** — Three ready-to-use graph variants under `types::simple`, `types::bytes`, and `types::string`, each with `AddVertex`, `AddEdge`, `Graph`, and `Operation` type aliases.
- **Shared remove types** — `RemoveVertex` and `RemoveEdge` structs shared across all graph variants (`crdt_graph::types::{RemoveVertex, RemoveEdge}`).
- **FlatBuffers serialization** — `flatbuffers::simple`, `flatbuffers::bytes`, and `flatbuffers::string` modules with `encode_operation()`, `encode_operation_log()`, `decode_operation()`, and `decode_operation_log()` functions.
- **UUID v7 identifiers** — All built-in types use `uuid::Uuid` (v7) for globally unique, time-ordered IDs. `Uuid` is re-exported from the crate root.
- **Query methods** — `vertex_count()`, `edge_count()`, `is_empty()`, `vertices()`, `edges()` on `TwoPTwoPGraph`.
- **`Default` implementation** for `TwoPTwoPGraph`.
- **`PartialEq` / `Eq`** on `UpdateOperation`.
- **`Hash`** derive on all built-in operation types.
- **`From` conversions** — `From<AddVertex>`, `From<RemoveVertex>`, `From<AddEdge>`, `From<RemoveEdge>` into each variant's `Operation` type.
- **Comprehensive test suite** — 97 tests covering graph operations, FlatBuffers round-trips, type traits, and multi-replica synchronization edge cases.

### Changed

- **Module structure** — Types and FlatBuffers code reorganized into submodule hierarchies (`types/` and `flatbuffers/` with `simple`, `bytes`, `string` submodules).
- **FlatBuffers UUID encoding** — UUIDs are now stored as `struct Uuid { bytes:[ubyte:16]; }` (16-byte inline struct) instead of strings, reducing wire size and eliminating string allocation on decode.
- **FlatBuffers file identifiers** — `"CRDT"` (simple), `"CRD2"` (bytes), `"CRD3"` (string).

### Removed

- `InvalidUuid` error variant (no longer needed with binary UUID encoding).

## [0.2.0] - 2025-03-28

### Added

- Initial release on crates.io.
- Op-based 2P2P-Graph CRDT with `prepare()` / `apply_downstream()` two-phase protocol.
- `TwoPTwoPGraph` generic struct with configurable vertex/edge operation types.
- Traits: `TwoPTwoPId`, `TwoPTwoPAddVertex`, `TwoPTwoPRemoveVertex`, `TwoPTwoPAddEdge`, `TwoPTwoPRemoveEdge`.
- `generate_petgraph()` for converting CRDT state to `petgraph::DiGraph`.
- `TwoPTwoPGraphError` with 7 error variants for precondition violations.

[0.3.0]: https://github.com/bkbkb-net/crdt-graph/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/bkbkb-net/crdt-graph/releases/tag/v0.2.0
