//! The indexed graph against the scanning one it replaced.
//!
//! `support/scan_graph.rs` is the graph as it was before it had an index:
//! every set a `Vec`, every question a scan, which makes it slow and
//! obviously right. Random operations go to both, through both the
//! at-source and the downstream path, and after every one the two must
//! agree on everything a caller can observe: the result (errors compared
//! by value, which includes the witness `VertexHasEdge` reports), the four
//! histories, the live sets, the counts, every lookup, and the petgraph.
//!
//! Identifiers are drawn from small ranges so the interesting cases come
//! up constantly: duplicate adds, the same add removed under two remove
//! ids (as concurrent replicas do), duplicate removes, self-loops, edges
//! that arrive downstream before their vertices, and removals of vertices
//! that still have edges. Seeds are fixed, so a failure reproduces.

mod support {
    pub mod scan_graph;
}

use crdt_graph::{
    TwoPTwoPAddEdge, TwoPTwoPAddVertex, TwoPTwoPGraph, TwoPTwoPId, TwoPTwoPRemoveEdge,
    TwoPTwoPRemoveVertex, UpdateOperation,
};
use support::scan_graph::ScanGraph;

type Id = u64;

#[derive(Clone, Debug, PartialEq)]
struct VA {
    id: Id,
}
impl TwoPTwoPId<Id> for VA {
    fn id(&self) -> &Id {
        &self.id
    }
}
impl TwoPTwoPAddVertex<Id> for VA {}

#[derive(Clone, Debug, PartialEq)]
struct VR {
    id: Id,
    add_vertex_id: Id,
}
impl TwoPTwoPId<Id> for VR {
    fn id(&self) -> &Id {
        &self.id
    }
}
impl TwoPTwoPRemoveVertex<Id> for VR {
    fn add_vertex_id(&self) -> &Id {
        &self.add_vertex_id
    }
}

#[derive(Clone, Debug, PartialEq)]
struct EA {
    id: Id,
    source: Id,
    target: Id,
}
impl TwoPTwoPId<Id> for EA {
    fn id(&self) -> &Id {
        &self.id
    }
}
impl TwoPTwoPAddEdge<Id> for EA {
    fn source(&self) -> &Id {
        &self.source
    }
    fn target(&self) -> &Id {
        &self.target
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ER {
    id: Id,
    add_edge_id: Id,
}
impl TwoPTwoPId<Id> for ER {
    fn id(&self) -> &Id {
        &self.id
    }
}
impl TwoPTwoPRemoveEdge<Id> for ER {
    fn add_edge_id(&self) -> &Id {
        &self.add_edge_id
    }
}

type Indexed = TwoPTwoPGraph<VA, VR, EA, ER, Id>;
type Scanned = ScanGraph<VA, VR, EA, ER, Id>;
type Op = UpdateOperation<VA, VR, EA, ER>;

/// Where vertex ids, edge-add ids and remove ids are drawn from, each a
/// small range. Disjoint for half the seeds; for the other half the three
/// share one space, so an edge id may equal a vertex id or a remove id,
/// which keyed maps must not confuse.
#[derive(Clone)]
struct Ranges {
    vertices: std::ops::Range<Id>,
    edges: std::ops::Range<Id>,
    removes: std::ops::Range<Id>,
}

impl Ranges {
    fn for_seed(seed: u64) -> Self {
        if seed.is_multiple_of(2) {
            Self {
                vertices: 0..10,
                edges: 100..130,
                removes: 200..240,
            }
        } else {
            Self {
                vertices: 0..10,
                edges: 0..24,
                removes: 0..32,
            }
        }
    }
}

/// xorshift64*: deterministic, dependency-free.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn pick(&mut self, range: std::ops::Range<Id>) -> Id {
        range.start + self.next() % (range.end - range.start)
    }
}

fn random_op(rng: &mut Rng, ranges: &Ranges) -> Op {
    match rng.next() % 10 {
        0..=2 => UpdateOperation::AddVertex(VA {
            id: rng.pick(ranges.vertices.clone()),
        }),
        3..=6 => {
            let source = rng.pick(ranges.vertices.clone());
            // A self-loop about one time in five.
            let target = if rng.next().is_multiple_of(5) {
                source
            } else {
                rng.pick(ranges.vertices.clone())
            };
            UpdateOperation::AddEdge(EA {
                id: rng.pick(ranges.edges.clone()),
                source,
                target,
            })
        }
        7 => UpdateOperation::RemoveVertex(VR {
            id: rng.pick(ranges.removes.clone()),
            add_vertex_id: rng.pick(ranges.vertices.clone()),
        }),
        _ => UpdateOperation::RemoveEdge(ER {
            id: rng.pick(ranges.removes.clone()),
            add_edge_id: rng.pick(ranges.edges.clone()),
        }),
    }
}

/// Everything a caller can observe, compared.
fn assert_agree(indexed: &Indexed, scanned: &Scanned, ranges: &Ranges, context: &str) {
    assert_eq!(
        indexed.all_vertices_added(),
        scanned.all_vertices_added(),
        "{context}: V_A"
    );
    assert_eq!(
        indexed.all_vertices_removed(),
        scanned.all_vertices_removed(),
        "{context}: V_R"
    );
    assert_eq!(
        indexed.all_edges_added(),
        scanned.all_edges_added(),
        "{context}: E_A"
    );
    assert_eq!(
        indexed.all_edges_removed(),
        scanned.all_edges_removed(),
        "{context}: E_R"
    );
    assert_eq!(
        indexed.vertices().collect::<Vec<_>>(),
        scanned.vertices().collect::<Vec<_>>(),
        "{context}: live vertices"
    );
    assert_eq!(
        indexed.edges().collect::<Vec<_>>(),
        scanned.edges().collect::<Vec<_>>(),
        "{context}: live edges"
    );
    assert_eq!(indexed.vertex_count(), scanned.vertex_count(), "{context}");
    assert_eq!(indexed.edge_count(), scanned.edge_count(), "{context}");
    assert_eq!(indexed.is_empty(), scanned.is_empty(), "{context}");
    for id in ranges.vertices.clone() {
        assert_eq!(
            indexed.lookup_vertex(&id),
            scanned.lookup_vertex(&id),
            "{context}: lookup_vertex({id})"
        );
        assert_eq!(
            indexed.vertex_added(&id),
            scanned.all_vertices_added().iter().find(|va| va.id == id),
            "{context}: vertex_added({id})"
        );
        // The precondition's set, from the definition: edges not removed,
        // touching the vertex, in E_A order.
        let incident: Vec<&EA> = scanned
            .all_edges_added()
            .iter()
            .filter(|ea| {
                !scanned
                    .all_edges_removed()
                    .iter()
                    .any(|er| er.add_edge_id == ea.id)
                    && (ea.source == id || ea.target == id)
            })
            .collect();
        assert_eq!(
            indexed.incident_edges(&id).collect::<Vec<_>>(),
            incident,
            "{context}: incident_edges({id})"
        );
    }
    for id in ranges.edges.clone() {
        assert_eq!(
            indexed.edge_added(&id),
            scanned.all_edges_added().iter().find(|ea| ea.id == id),
            "{context}: edge_added({id})"
        );
        assert_eq!(
            indexed.edge_position(&id),
            scanned.all_edges_added().iter().position(|ea| ea.id == id),
            "{context}: edge_position({id})"
        );
        assert_eq!(
            indexed.edge_is_removed(&id),
            scanned
                .all_edges_removed()
                .iter()
                .any(|er| er.add_edge_id == id),
            "{context}: edge_is_removed({id})"
        );
        let probe = ER {
            id: 0,
            add_edge_id: id,
        };
        assert_eq!(
            indexed.get_edge_added_from_remove_edge(&probe),
            scanned.get_edge_added_from_remove_edge(&probe),
            "{context}: get_edge_added_from_remove_edge({id})"
        );
        assert_eq!(
            indexed.lookup_from_remove_edge(&probe),
            scanned.lookup_from_remove_edge(&probe),
            "{context}: lookup_from_remove_edge({id})"
        );
    }
    let (a, b) = (indexed.generate_petgraph(), scanned.generate_petgraph());
    assert_eq!(
        a.node_weights().collect::<Vec<_>>(),
        b.node_weights().collect::<Vec<_>>(),
        "{context}: petgraph nodes"
    );
    assert_eq!(
        a.raw_edges()
            .iter()
            .map(|e| (e.source(), e.target(), &e.weight))
            .collect::<Vec<_>>(),
        b.raw_edges()
            .iter()
            .map(|e| (e.source(), e.target(), &e.weight))
            .collect::<Vec<_>>(),
        "{context}: petgraph edges"
    );
}

#[test]
fn the_indexed_graph_behaves_exactly_as_the_scanning_one() {
    for seed in 1..=150_u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let ranges = Ranges::for_seed(seed);
        let mut indexed = Indexed::new();
        let mut scanned = Scanned::new();
        let mut frozen = None;
        let mut accepted = 0;
        for step in 0..300 {
            let op = random_op(&mut rng, &ranges);
            // Half the operations as a source would issue them, half as
            // a replica receives them: the preconditions differ.
            let at_source = rng.next().is_multiple_of(2);
            let (a, b) = if at_source {
                let (a, b) = (indexed.prepare(op.clone()), scanned.prepare(op.clone()));
                // What a source broadcasts is the operation it applied.
                if let Ok(broadcast) = &a {
                    assert_eq!(broadcast, &op, "seed {seed} step {step}: broadcast");
                }
                (a.map(|_| ()), b.map(|_| ()))
            } else {
                (
                    indexed.apply_downstream(op.clone()),
                    scanned.apply_downstream(op.clone()),
                )
            };
            let context = format!("seed {seed} step {step} {op:?} at_source={at_source}");
            assert_eq!(format!("{a:?}"), format!("{b:?}"), "{context}: result");
            accepted += usize::from(a.is_ok());
            assert_agree(&indexed, &scanned, &ranges, &context);
            // A clone must carry its own index: it answers the same, the
            // copy goes on agreeing once changed, and the original it was
            // taken from is untouched by those changes.
            if step == 150 {
                let copy = indexed.clone();
                assert_agree(&copy, &scanned, &ranges, &format!("{context} (clone)"));
                frozen = Some((std::mem::replace(&mut indexed, copy), scanned.clone()));
            }
        }
        let (original, at_clone) = frozen.expect("a clone was taken");
        assert_agree(
            &original,
            &at_clone,
            &ranges,
            &format!("seed {seed}: original"),
        );
        assert!(
            accepted >= 40,
            "seed {seed}: only {accepted} operations were accepted, so this \
             exercised the error paths more than the index"
        );
    }
}

/// An identifier whose `Clone` panics on demand: ids of 100 and above
/// refuse to be cloned while the flag is set. The ones below always clone,
/// so an operation can get partway -- some keys cloned, then a panic --
/// which is the state a careless mutator leaves behind.
#[derive(Debug, PartialEq, Eq, Hash)]
struct Fragile(u64);

thread_local! {
    static CLONES_PANIC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

impl Clone for Fragile {
    fn clone(&self) -> Self {
        if self.0 >= 100 && CLONES_PANIC.with(|flag| flag.get()) {
            panic!("clone refused");
        }
        Fragile(self.0)
    }
}

#[derive(Clone, Debug)]
struct FV(Fragile);
impl TwoPTwoPId<Fragile> for FV {
    fn id(&self) -> &Fragile {
        &self.0
    }
}
impl TwoPTwoPAddVertex<Fragile> for FV {}

#[derive(Clone, Debug)]
struct FE {
    id: Fragile,
    source: Fragile,
    target: Fragile,
}
impl TwoPTwoPId<Fragile> for FE {
    fn id(&self) -> &Fragile {
        &self.id
    }
}
impl TwoPTwoPAddEdge<Fragile> for FE {
    fn source(&self) -> &Fragile {
        &self.source
    }
    fn target(&self) -> &Fragile {
        &self.target
    }
}

#[derive(Clone, Debug)]
struct FR {
    id: Fragile,
    of: Fragile,
}
impl TwoPTwoPId<Fragile> for FR {
    fn id(&self) -> &Fragile {
        &self.id
    }
}
impl TwoPTwoPRemoveVertex<Fragile> for FR {
    fn add_vertex_id(&self) -> &Fragile {
        &self.of
    }
}
impl TwoPTwoPRemoveEdge<Fragile> for FR {
    fn add_edge_id(&self) -> &Fragile {
        &self.of
    }
}

type FragileGraph = TwoPTwoPGraph<FV, FR, FE, FR, Fragile>;
type FragileOp = UpdateOperation<FV, FR, FE, FR>;

/// Applies `op` downstream with clones refused, and insists it panicked.
fn refused(graph: &mut FragileGraph, op: FragileOp) {
    CLONES_PANIC.with(|flag| flag.set(true));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = graph.apply_downstream(op);
    }));
    CLONES_PANIC.with(|flag| flag.set(false));
    assert!(result.is_err(), "the clone was supposed to panic");
}

/// **A panicking `Clone` leaves the graph as it was.** The index clones
/// keys the scanning graph never had to, so an operation whose second or
/// third clone fails must not leave the first one's entry behind: an index
/// pointing past the end of its history, or a remove id recorded for a
/// remove that never happened. Each mutator is made to panic partway; the
/// graph must answer as if nothing had been attempted, and the same
/// operations must then be accepted when retried.
#[test]
fn a_clone_that_panics_changes_nothing() {
    use crdt_graph::UpdateType::Downstream;
    let mut graph = FragileGraph::new();
    graph.add_vertex(FV(Fragile(1)), Downstream).unwrap();
    graph.add_vertex(FV(Fragile(100)), Downstream).unwrap();
    graph
        .add_edge(
            FE {
                id: Fragile(101),
                source: Fragile(1),
                target: Fragile(1),
            },
            Downstream,
        )
        .unwrap();

    // Each refuses at its second key: the edge's source, the removed
    // vertex, the removed edge.
    let add_edge = || {
        UpdateOperation::AddEdge(FE {
            id: Fragile(11),
            source: Fragile(100),
            target: Fragile(1),
        })
    };
    let remove_vertex = || {
        UpdateOperation::RemoveVertex(FR {
            id: Fragile(21),
            of: Fragile(100),
        })
    };
    let remove_edge = || {
        UpdateOperation::RemoveEdge(FR {
            id: Fragile(20),
            of: Fragile(101),
        })
    };
    refused(&mut graph, add_edge());
    refused(&mut graph, remove_vertex());
    refused(&mut graph, remove_edge());

    assert_eq!(graph.all_vertices_added().len(), 2);
    assert_eq!(graph.all_edges_added().len(), 1);
    assert!(graph.all_vertices_removed().is_empty());
    assert!(graph.all_edges_removed().is_empty());
    assert!(graph.edge_added(&Fragile(11)).is_none());
    assert!(graph.lookup_vertex(&Fragile(100)));
    assert!(!graph.edge_is_removed(&Fragile(101)));
    assert_eq!(graph.incident_edges(&Fragile(100)).count(), 0);
    assert_eq!((graph.vertex_count(), graph.edge_count()), (2, 1));

    // Nothing half-recorded stands in the way of doing it properly.
    graph.apply_downstream(add_edge()).unwrap();
    graph.apply_downstream(remove_edge()).unwrap();
    graph.apply_downstream(remove_vertex()).unwrap();
    assert_eq!((graph.vertex_count(), graph.edge_count()), (1, 1));
}
