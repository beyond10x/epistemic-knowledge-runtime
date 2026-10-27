//! Adversary, wave 2026-10-20a unit U1, pass 1: `story:supersede-node-review-fixes` D9 changed
//! how retired edges are looked up and must change no answer — the coordinator's invariant is that
//! 0.0.40 changes no kernel verdict 0.0.39 could have recorded.
//!
//! [`RetiredEdges`] replaced three scans of release 0.0.39, each reproduced here verbatim from
//! the base commit as the oracle:
//!
//! * the kernel's `CanonicalGraph::retired_with` (`canonical.rs:484` at the base): the first
//!   record, in node-id order, whose `edges` holds the id;
//! * the views index's set (`ekr-views/src/index.rs:203` at the base): every record's edge ids;
//! * the sample's `subject_type` (`ekr-views/src/sample.rs:213` at the base): the first record's
//!   edge under that id.
//!
//! The record sets are generated from a fixed seed and include what no commit produces: one edge
//! held by several records, an edge held both by the head and by a record, a record filed under
//! another node's id, and a record whose `edges` key differs from the edge's own id.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use ekr_core::{EdgeId, GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp, TypeId};
use ekr_graph::{
    CanonicalGraph, CanonicalRef, Edge, GraphRoot, Node, RetiredEdges, Space, SupersededNode,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};

/// xorshift64*, fixed seed: no wall clock, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn node_id(n: u64) -> NodeId {
    format!("00000000-0000-4000-8000-{:012x}", 0x300 + n)
        .parse()
        .unwrap()
}

fn edge_id(n: u64) -> EdgeId {
    format!("00000000-0000-4000-8000-{:012x}", 0x500 + n)
        .parse()
        .unwrap()
}

fn graph(rng: &mut Rng) -> CanonicalGraph {
    let root = GraphRoot {
        id: "00000000-0000-4000-8000-000000000002".parse().unwrap(),
        space: Space::Canonical,
        schema_version_id: SchemaVersionId::from_uuid(
            "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        ),
        parent: None,
        created_at: Timestamp::EPOCH,
    };
    let type_id: TypeId = "00000000-0000-4000-8000-000000000201".parse().unwrap();
    let edge = |id: EdgeId, own: EdgeId, source: u64, target: u64, root: GraphRootId| {
        // `own` is the edge's id field; the map key is `id`. They differ only in records no
        // commit produces.
        let _ = id;
        Edge::new(
            own,
            root,
            type_id,
            CanonicalRef::new(node_id(source)),
            CanonicalRef::new(node_id(target)),
        )
    };
    let mut edges = BTreeMap::new();
    for e in 0..8 {
        if rng.below(4) == 0 {
            edges.insert(edge_id(e), edge(edge_id(e), edge_id(e), 0, 1, root.id));
        }
    }
    let mut superseded = BTreeMap::new();
    for _ in 0..rng.below(6) {
        let node = rng.below(8);
        let filed_under = if rng.below(8) == 0 {
            rng.below(8)
        } else {
            node
        };
        let mut held = BTreeMap::new();
        for e in 0..8 {
            if rng.below(3) == 0 {
                let own = if rng.below(10) == 0 {
                    edge_id(rng.below(9))
                } else {
                    edge_id(e)
                };
                held.insert(
                    edge_id(e),
                    edge(edge_id(e), own, node, rng.below(8), root.id),
                );
            }
        }
        let mut record_node = Node::new(node_id(node), root.id, type_id, format!("n{node}"));
        record_node.aliases = vec![format!("n{node}")];
        superseded.insert(
            node_id(filed_under),
            SupersededNode {
                node: record_node,
                edges: held,
                by: CanonicalRef::new(node_id(8 + rng.below(3))),
                revision: RevisionNumber::new(1 + rng.below(4)),
                reason: "recorded twice".into(),
                evidence: BTreeSet::new(),
                merge: None,
            },
        );
    }
    CanonicalGraph {
        root,
        revision: RevisionNumber::new(5),
        ontology: Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(
                SchemaVersionId::from_uuid("00000000-0000-4000-8000-000000000001".parse().unwrap()),
                Timestamp::EPOCH,
            ),
            node_types: Vec::new(),
            edge_types: Vec::new(),
        })
        .unwrap(),
        nodes: BTreeMap::new(),
        edges,
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
        attachments: BTreeMap::new(),
        superseded,
    }
}

/// D9's index answers every lookup as each 0.0.39 scan it replaced answered it, on 2 000 generated
/// record sets — committed shapes and shapes no commit produces alike.
#[test]
fn the_retired_edge_index_answers_as_every_scan_it_replaced() {
    let mut rng = Rng(0x0020_2610_20a1);
    let mut nonempty = 0;
    let mut shared = 0;
    for case in 0..2_000 {
        let graph = graph(&mut rng);
        let index: RetiredEdges<'_> = graph.retired_edges();
        let of = RetiredEdges::of(&graph.superseded);
        // 0.0.39's views set.
        let set: HashSet<EdgeId> = graph
            .superseded
            .values()
            .flat_map(|record| record.edges.keys().copied())
            .collect();
        nonempty += usize::from(!graph.superseded.is_empty());
        for e in 0..10 {
            let edge = edge_id(e);
            // 0.0.39's kernel scan.
            let scan = graph
                .superseded
                .values()
                .find(|record| record.edges.contains_key(&edge))
                .map(|record| (record.node.id, record.by.node()));
            // 0.0.39's sample scan.
            let sampled = graph
                .superseded
                .values()
                .find_map(|record| record.edges.get(&edge));
            shared += usize::from(
                graph
                    .superseded
                    .values()
                    .filter(|record| record.edges.contains_key(&edge))
                    .count()
                    > 1,
            );
            assert_eq!(index.retired_with(edge), scan, "case {case}, edge {e}");
            assert_eq!(of.retired_with(edge), scan, "case {case}, edge {e}");
            assert_eq!(graph.retired_with(edge), scan, "case {case}, edge {e}");
            assert_eq!(index.edge(edge), sampled, "case {case}, edge {e}");
            assert_eq!(
                index.record(edge).is_some(),
                set.contains(&edge),
                "case {case}, edge {e}"
            );
            assert_eq!(
                index
                    .record(edge)
                    .map(|record| (record.node.id, record.by.node())),
                scan,
                "case {case}, edge {e}"
            );
        }
    }
    assert!(
        nonempty > 1_000,
        "the generator made {nonempty} record sets"
    );
    assert!(shared > 100, "the generator shared an edge {shared} times");
}
