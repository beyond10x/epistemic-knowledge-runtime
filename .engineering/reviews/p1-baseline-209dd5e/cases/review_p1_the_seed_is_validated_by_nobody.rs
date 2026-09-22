//! Independent review of the P1 core: the seed path, read across the crate seam.
//!
//! Design § 21 says canonical state is "strongly typed, schema valid, referentially complete,
//! provenance compliant, transactionally committed, revisioned". `ekr-store`'s crossing
//! (`crates/ekr-store/src/snapshot.rs` § "What this crossing does not do") places the reference,
//! type and ontology-constraint checks in `ekr-kernel` "on purpose", and `ekr-kernel`'s validators
//! run over a *transaction* (`Pipeline::validate`). The seed is not a transaction. So the one path
//! by which P1 creates canonical state — `ObjectStore::put`, `append(Seeded)`, `fold()` — runs no
//! validator at all, and each crate's own guard passes while the composition is checked by nobody.
//!
//! Each case below builds a seed with exactly one defect against § 21, and every type it names is
//! declared, so the defect is the only thing a validator would have refused. Every function called
//! is public.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, RevisionNumber,
    SchemaVersionId, Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, CanonicalGraph, Confidence, Edge, Evidence, EvidenceSource, GraphRoot, Node, Object,
    Predicate, Space, Subject, TemporalRange, TransactionTime, ValidationState,
};
use ekr_ontology::{EdgeType, NodeType, Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{Appended, GraphDocument, ObjectStore, RevisionLog, SqliteStore, StorageClass};
use tempfile::TempDir;

/// One node type and one edge type between nodes of it, both declared.
struct Declared {
    ontology: Ontology,
    thing: TypeId,
    relates: TypeId,
}

fn declared() -> Declared {
    let (thing, relates) = (TypeId::mint(), TypeId::mint());
    let mut edge_type = EdgeType::new(relates, "relates");
    edge_type.source_types.insert(thing);
    edge_type.target_types.insert(thing);
    let ontology = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: vec![NodeType::new(thing, "Thing")],
        edge_types: vec![edge_type],
    })
    .expect("one node type and one edge type cohere");
    Declared {
        ontology,
        thing,
        relates,
    }
}

/// An empty canonical graph over `ontology`, for the cases to put one defect into.
fn empty(ontology: &Ontology) -> CanonicalGraph {
    CanonicalGraph {
        root: GraphRoot {
            id: GraphRootId::mint(),
            space: Space::Canonical,
            schema_version_id: ontology.version().id,
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::SEED,
        ontology: ontology.clone(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    }
}

/// Stores `graph` as the seed of a fresh lineage and folds it: the whole of the public seed path.
fn seed_and_fold(graph: &CanonicalGraph) -> Result<CanonicalGraph, ekr_store::StoreError> {
    let directory = TempDir::new().expect("a temporary directory");
    let store = SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        graph.ontology.clone(),
    )
    .expect("the SQLite provider opens");
    let document = GraphDocument::of(graph)
        .to_bytes()
        .expect("the seed serialises");
    let seed = store
        .put(StorageClass::Canonical, &document, Timestamp::EPOCH)
        .expect("the seed lands");
    assert_eq!(
        store
            .append(&ekr_graph::RevisionEvent::Seeded {
                revision_id: ekr_core::RevisionId::mint(),
                seed_hash: seed.content_hash,
            })
            .expect("seeded"),
        Appended::Written
    );
    store.fold()
}

/// Referentially complete: an edge whose target no node holds.
#[test]
fn a_seed_holding_an_edge_to_a_node_it_does_not_hold_becomes_canonical_state() {
    let Declared {
        ontology,
        thing,
        relates,
    } = declared();
    let mut graph = empty(&ontology);
    let root_id = graph.root.id;
    let held = Node::new(NodeId::mint(), root_id, thing, "held");
    let absent = NodeId::mint();
    let dangling = Edge::new(EdgeId::mint(), root_id, relates, held.id, absent);
    graph.nodes.insert(held.id, held);
    graph.edges.insert(dangling.id, dangling);

    let folded = seed_and_fold(&graph);
    assert!(
        folded.is_err(),
        "design § 21: canonical state is referentially complete, and AGENTS.md invariant 2 says a \
         reference out of canonical state to something canonical state does not hold is \
         unrepresentable; the seed path — put, Seeded, fold, all public — produced canonical state \
         holding an edge to node {absent}, which nothing holds, and no validator ran: \
         {:?}",
        folded.map(|graph| graph.edges.len())
    );
}

/// Strongly typed, schema valid: a node of a type the ontology does not declare.
#[test]
fn a_seed_holding_a_node_of_a_type_the_ontology_does_not_declare_becomes_canonical_state() {
    let Declared { ontology, .. } = declared();
    let mut graph = empty(&ontology);
    let undeclared = TypeId::mint();
    let node = Node::new(
        NodeId::mint(),
        graph.root.id,
        undeclared,
        "of-no-declared-type",
    );
    graph.nodes.insert(node.id, node);

    let folded = seed_and_fold(&graph);
    assert!(
        folded.is_err(),
        "design § 21: canonical state is strongly typed and schema valid; the seed path produced \
         canonical state holding a node of type {undeclared}, which the ontology the store was \
         opened with does not declare, and the type validator that would refuse it on the \
         transaction path (crates/ekr-kernel/src/validate/types.rs, `unknown-type`) never runs on \
         the seed path: {:?}",
        folded.map(|graph| graph.nodes.len())
    );
}

/// Provenance compliant, AGENTS.md invariant 4: an assertion marked `Accepted` by nobody.
///
/// On the transaction path the provenance validator refuses any `AddAssertion` that is not
/// `Proposed` (`crates/ekr-kernel/src/validate/provenance.rs`, `assertion-states-its-own-verdict`),
/// because "an assertion arriving already marked Accepted, naming validators that never ran, is an
/// agent writing down the answer to the question this pipeline exists to ask". On the seed path
/// the same record is canonical state with no question asked.
#[test]
fn a_seed_holding_an_assertion_accepted_by_no_validator_becomes_canonical_state() {
    let Declared {
        ontology,
        thing,
        relates,
    } = declared();
    let mut graph = empty(&ontology);
    let root_id = graph.root.id;
    let (subject, object) = (
        Node::new(NodeId::mint(), root_id, thing, "subject"),
        Node::new(NodeId::mint(), root_id, thing, "object"),
    );
    let evidence = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::Document {
            document_id: "docs/roadmap.md".to_owned(),
            section: Some("P1".to_owned()),
        },
        content_hash: ContentHash::of_bytes(b"the roadmap's P1 exit criterion"),
        extracted_by: AgentId::mint(),
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::CERTAIN,
    };
    let accepted_by_nobody = Assertion {
        id: AssertionId::mint(),
        root_id,
        subject: Subject::Node(subject.id),
        predicate: Predicate::Relation(relates),
        object: Object::Node(object.id),
        evidence: [evidence.id].into_iter().collect::<BTreeSet<_>>(),
        proposed_by: AgentId::mint(),
        validation: ValidationState::Accepted {
            validators: BTreeSet::new(),
        },
        valid_time: TemporalRange::since(Timestamp::EPOCH),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    graph.nodes.insert(subject.id, subject);
    graph.nodes.insert(object.id, object);
    graph.evidence.insert(evidence.id, evidence);
    graph
        .assertions
        .insert(accepted_by_nobody.id, accepted_by_nobody);

    let folded = seed_and_fold(&graph);
    assert!(
        folded.is_err(),
        "AGENTS.md invariant 4 and design § 6.5: an assertion is Accepted because validators \
         accepted it, and this one names none and passed through no pipeline; the seed path \
         produced canonical state holding it, where the provenance validator would have refused \
         the same record on the transaction path: {:?}",
        folded.map(|graph| graph.assertions.len())
    );
}
