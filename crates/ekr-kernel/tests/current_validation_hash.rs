//! `task:validation-hash-uses-payload-domain`: `ValidatedTransaction::validation_hash` is the
//! value-domain address of the complete validation basis — the canonical transaction and the
//! revision it was validated against — and never the payload-domain hash of those same bytes.
//!
//! `ekr_core::ContentHash::of` says "`of(&v)` is never `of_bytes(&v.canonical_bytes())`"; the
//! archived witness `.engineering/reviews/p1-baseline-209dd5e/cases/review_p1_validation_hash_domain.rs`
//! measured `seal` taking exactly that forbidden composition.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{
    AgentId, ContentHash, GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp,
    TransactionId, TypeId,
};
use ekr_graph::{CanonicalGraph, CanonicalValue, GraphRoot, GraphSnapshot, Space};
use ekr_kernel::{GraphOperation, GraphTransaction, NodeDraft, Pipeline, ValidatedTransaction};
use ekr_ontology::{NodeType, Ontology, OntologyDocument, SchemaVersion};

/// The complete basis `seal` addresses, encoded by this test from public parts only.
struct Basis<'a> {
    transaction: &'a GraphTransaction<CanonicalValue>,
    validated_against: RevisionNumber,
}
impl Canonical for Basis<'_> {
    fn encode(&self, out: &mut Encoder) {
        self.transaction.encode(out);
        self.validated_against.encode(out);
    }
}
fn basis(validated: &ValidatedTransaction) -> Basis<'_> {
    Basis {
        transaction: validated.transaction(),
        validated_against: validated.validated_against(),
    }
}

struct World {
    graph: CanonicalGraph,
    root_id: GraphRootId,
    decision: TypeId,
    validator: AgentId,
}
impl World {
    fn new() -> Self {
        let (root_id, schema, decision) =
            (GraphRootId::mint(), SchemaVersionId::mint(), TypeId::mint());
        Self {
            graph: CanonicalGraph {
                root: GraphRoot {
                    id: root_id,
                    space: Space::Canonical,
                    schema_version_id: schema,
                    parent: None,
                    created_at: Timestamp::EPOCH,
                },
                revision: RevisionNumber::new(7),
                ontology: Ontology::load(OntologyDocument {
                    version: SchemaVersion::seed(schema, Timestamp::EPOCH),
                    node_types: vec![NodeType::new(decision, "Decision")],
                    edge_types: Vec::new(),
                })
                .expect("one node type coheres"),
                nodes: BTreeMap::new(),
                edges: BTreeMap::new(),
                assertions: BTreeMap::new(),
                evidence: BTreeMap::new(),
            },
            root_id,
            decision,
            validator: AgentId::mint(),
        }
    }
    fn proposal(&self, name: &str) -> GraphTransaction {
        GraphTransaction {
            id: TransactionId::mint(),
            proposer: AgentId::mint(),
            operations: vec![GraphOperation::CreateNode(NodeDraft {
                id: NodeId::mint(),
                root_id: self.root_id,
                type_id: self.decision,
                canonical_name: name.to_owned(),
                properties: BTreeMap::new(),
            })],
            evidence: BTreeSet::new(),
        }
    }
    fn validate(&self, graph: &CanonicalGraph, tx: &GraphTransaction) -> ValidatedTransaction {
        Pipeline::deterministic(self.validator)
            .validate(&GraphSnapshot::of(graph), tx)
            .expect("nothing is wrong with this proposal")
    }
}

#[test]
fn the_validation_hash_is_the_value_address_of_the_complete_basis() {
    let world = World::new();
    let validated = world.validate(&world.graph, &world.proposal("hash canonical state only"));
    let basis = basis(&validated);

    assert_eq!(validated.validation_hash(), ContentHash::of(&basis));
    assert_ne!(
        validated.validation_hash(),
        ContentHash::of_bytes(&basis.canonical_bytes()),
        "a value's canonical bytes hashed as a payload must never share the value's address"
    );
}

#[test]
fn the_validation_hash_is_deterministic_for_one_transaction_and_revision() {
    let world = World::new();
    let proposal = world.proposal("hash canonical state only");
    let first = world.validate(&world.graph, &proposal);
    let second = world.validate(&world.graph, &proposal);

    assert_eq!(first.validation_hash(), second.validation_hash());
    assert_eq!(second.validation_hash(), ContentHash::of(&basis(&second)));
}

#[test]
fn the_validation_hash_moves_with_the_transaction() {
    let world = World::new();
    let one = world.proposal("hash canonical state only");
    let mut other = one.clone();
    let GraphOperation::CreateNode(draft) = &mut other.operations[0] else {
        unreachable!()
    };
    draft.canonical_name = "hash canonical state only, twice".to_owned();
    let first = world.validate(&world.graph, &one);
    let second = world.validate(&world.graph, &other);

    assert_ne!(first.validation_hash(), second.validation_hash());
    for validated in [&first, &second] {
        assert_eq!(
            validated.validation_hash(),
            ContentHash::of(&basis(validated))
        );
    }
}

#[test]
fn the_validation_hash_moves_with_the_revision_validated_against() {
    let world = World::new();
    let proposal = world.proposal("hash canonical state only");
    let at_seven = world.validate(&world.graph, &proposal);
    let mut moved = world.graph.clone();
    moved.revision = RevisionNumber::new(8);
    let at_eight = world.validate(&moved, &proposal);

    assert_eq!(at_eight.validated_against(), RevisionNumber::new(8));
    assert_ne!(at_seven.validation_hash(), at_eight.validation_hash());
    for validated in [&at_seven, &at_eight] {
        assert_eq!(
            validated.validation_hash(),
            ContentHash::of(&basis(validated))
        );
    }
}
