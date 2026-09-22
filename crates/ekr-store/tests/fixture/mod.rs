//! The state the store's cases write, read back and fold.
//!
//! Shared by all three test binaries, because each needs a seed with real content in it: a fold
//! whose graph is empty reports the same address whatever the log did, and a case built on one
//! cannot tell a store that persisted from a store that did not.
//!
//! The *events* that move a lineage are in `lineage/mod.rs` rather than here, because
//! `membrane_boundary.rs` needs the state and not the lineage — and an integration test binary
//! compiles every shared module it declares, so a helper with no caller in one binary is dead code
//! in that binary and `-D warnings` says so. Two modules, each wholly used by whoever declares it.
//!
//! Every name here is the runtime's own vocabulary. AGENTS.md: customer or personal data is not
//! copied into fixtures.

#![allow(dead_code)] // Different provider binaries use different parts of the shared fixture.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionNumber, SchemaVersionId, Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, CanonicalGraph, CanonicalRef, CanonicalValue, Confidence, Edge, Evidence,
    EvidenceSource, GraphRoot, Node, Object, Predicate, Space, Subject, TemporalRange,
    TransactionTime, ValidationState,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};

/// An ontology with no types.
///
/// The store type-checks nothing — `AGENTS.md` invariant 7 puts type validity in the kernel — so
/// the declarations would be decoration here. What the fixture needs from the ontology is that it
/// exists, because a [`CanonicalGraph`] carries one.
#[must_use]
pub fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("a document with no declarations coheres")
}

/// A seed with two nodes, an edge, an assertion and the evidence it rests on.
///
/// Content in all four maps, so that `knowledge_root` and `evidence_root` are functions of
/// something rather than constants: an empty seed would make every "the address is the same"
/// assertion below pass for a store that lost the lot.
///
/// # Both ends of the edge are here, and one of them was not
///
/// The edge's `target` named a node this map did not hold — a dangling reference inside canonical
/// state, found by reading in the independent review of the P1 core and *now found by building*:
/// under `architecture-decision-record:0008-canonical-state-references-are-typed` the two ends are
/// [`CanonicalRef<Node>`](ekr_graph::CanonicalRef), so wrapping the same absent id would have been
/// a deliberate act rather than an oversight. It is a seed of canonical state, and design § 21 says
/// canonical state is referentially complete, so the missing node is added rather than the
/// reference being pointed somewhere convenient.
///
/// **The type does not catch this and is not claimed to.** `CanonicalRef::new` takes any id;
/// what the type refuses is a reference into a *transient* root. This provider-only fixture uses
/// substitute authority; real bootstrap semantic checks run in the kernel's seed suite.
#[must_use]
pub fn seed_graph(ontology: &Ontology) -> CanonicalGraph {
    let root_id = GraphRootId::mint();
    let type_id = TypeId::mint();
    let property = PropertyId::mint();
    let (subject, object) = (NodeId::mint(), NodeId::mint());
    let evidence_id = EvidenceId::mint();

    let mut observed = Node::new(subject, root_id, type_id, "revision-lineage");
    observed
        .properties
        .insert(property, CanonicalValue::Decimal("1.0".to_owned()));
    let reached = Node::new(object, root_id, type_id, "revision-lineage-target");

    let mut holds = Edge::new(
        EdgeId::mint(),
        root_id,
        type_id,
        CanonicalRef::new(subject),
        CanonicalRef::new(object),
    );
    holds
        .properties
        .insert(property, CanonicalValue::Enum("canonical".to_owned()));

    let evidence = Evidence {
        id: evidence_id,
        source: EvidenceSource::Document {
            document_id: "docs/roadmap.md".to_owned(),
            section: Some("P1".to_owned()),
        },
        content_hash: ContentHash::of_bytes(b"the roadmap's P1 exit criterion"),
        extracted_by: AgentId::mint(),
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::CERTAIN,
    };

    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id,
        subject: Subject::Node(CanonicalRef::new(subject)),
        predicate: Predicate::Relation(type_id),
        object: Object::Node(CanonicalRef::new(object)),
        evidence: [evidence_id].into_iter().collect::<BTreeSet<_>>(),
        proposed_by: AgentId::mint(),
        validation: ValidationState::Accepted {
            validators: BTreeSet::new(),
        },
        valid_time: TemporalRange::since(Timestamp::EPOCH),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };

    CanonicalGraph {
        root: GraphRoot {
            id: root_id,
            space: Space::Canonical,
            schema_version_id: ontology.version().id,
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::SEED,
        ontology: ontology.clone(),
        nodes: [(subject, observed), (object, reached)]
            .into_iter()
            .collect::<BTreeMap<_, _>>(),
        edges: [(holds.id, holds)].into_iter().collect::<BTreeMap<_, _>>(),
        assertions: [(assertion.id, assertion)]
            .into_iter()
            .collect::<BTreeMap<_, _>>(),
        evidence: [(evidence_id, evidence)]
            .into_iter()
            .collect::<BTreeMap<_, _>>(),
    }
}

/// A deliberately permissive test-only seed authority for provider mechanics.
/// This is NOT kernel acceptance evidence; kernel/tests/seed.rs owns those claims.
#[derive(Default)]
pub struct SeedOnly;

impl ekr_store::CommitAuthority for SeedOnly {
    fn attests(&self, _: &ekr_store::RecordedValidation) -> bool {
        false
    }
    fn admit_seed(
        &self,
        bytes: &[u8],
        ontology: &Ontology,
    ) -> Result<CanonicalGraph, ekr_store::StoreError> {
        admit_seed(bytes, ontology)
    }
}

/// Deserializes exactly what a provider fixture wrote, without claiming semantic validation.
pub fn admit_seed(
    bytes: &[u8],
    ontology: &Ontology,
) -> Result<CanonicalGraph, ekr_store::StoreError> {
    #[derive(serde::Deserialize)]
    struct ProviderGraph {
        root: GraphRoot,
        revision: RevisionNumber,
        nodes: BTreeMap<NodeId, Node>,
        edges: BTreeMap<EdgeId, Edge>,
        assertions: BTreeMap<AssertionId, Assertion>,
        evidence: BTreeMap<EvidenceId, Evidence>,
    }
    let graph: ProviderGraph = serde_json::from_slice(bytes)
        .map_err(|error| ekr_store::StoreError::Document(error.to_string()))?;
    Ok(CanonicalGraph {
        root: graph.root,
        revision: graph.revision,
        ontology: ontology.clone(),
        nodes: graph.nodes,
        edges: graph.edges,
        assertions: graph.assertions,
        evidence: graph.evidence,
    })
}
