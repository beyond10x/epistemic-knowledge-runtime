//! The graph state of the Epistemic Knowledge Runtime.
//!
//! Implements the `ekr.graph` domain, `systems/ekr/domains/graph.yaml`: graph roots, nodes,
//! edges and bitemporal assertions with their validation state, evidence and observations. The
//! crate holds no writer of its own; every change arrives as a transaction the kernel committed.
//!
//! Ten modules, in dependency order:
//!
//! * [`value`] — [`CanonicalValue`], the values canonical state admits, and [`InadmissibleValue`],
//!   the refusal of the one kind it does not (`architecture-decision-record:0005`).
//! * [`root`] — [`GraphRoot`] and [`Space`] (design § 22–23), and [`Root`], the immutable
//!   revision root of design § 34.
//! * [`node`], [`edge`] — [`Node`] (design § 10, amendment 87) and [`Edge`] (design § 12).
//! * [`evidence`] — [`Observation`], [`Evidence`] and [`Support`] (design § 15–16, amendment 86).
//! * [`assertion`] — [`Assertion`] with its [`Subject`], [`Predicate`], [`Object`],
//!   [`TemporalRange`], [`TransactionTime`], [`Assessment`] and [`AssertionLifecycle`]
//!   (design § 13–14, § 17, § 36).
//! * [`canonical`], [`transient`] — the two knowledge spaces and the references each may hold
//!   (design § 21, § 23).
//! * [`snapshot`] — [`GraphSnapshot`], what a reader holds (design § 71). One read,
//!   [`GraphSnapshot::valid_at`]: the runtime has no clock, so the caller supplies the instant and
//!   gets the current world or a historical one from the same function.
//! * [`events`] — [`RevisionEvent`], the vocabulary the kernel publishes and the store persists.
//!
//! # The membrane is a type, not a rule
//!
//! AGENTS.md invariant 2: "Canonical knowledge depends only on canonical knowledge or retained
//! admissible evidence. A `Canonical → Transient` reference is unrepresentable at the type level,
//! not merely refused." [`CanonicalDependency`] is sealed and implemented for [`CanonicalRef`]
//! alone, so [`CanonicalGraph::resolve`] does not compile for a [`TransientRef`] and no crate
//! above this one can make it. A third build failure closes the way round it: [`CanonicalRef`]'s
//! marker is bounded by the sealed [`CanonicalTarget`], so `CanonicalRef<TransientRef<Node>>` is
//! not a type. `tests/compile_fail/` holds all three.
//!
//! Every reference canonical state holds goes through that machinery, since wave p1-14: a
//! [`CanonicalRef<T>`] holds [`CanonicalTarget::Id`] for its kind and resolves against that kind's
//! map, and each field below is a `CanonicalRef` in canonical state, so neither a bare id nor a
//! [`TransientRef`] inhabits it. The case that holds each, as a build failure:
//!
//! | Reference | Kind | Case |
//! |---|---|---|
//! | [`CanonicalRef<T>`] itself | its own | `tests/compile_fail/a_canonical_reference_holds_the_id_of_its_kind.rs`, `a_canonical_reference_targets_only_what_canonical_state_holds.rs` |
//! | [`Edge::source`], [`Edge::target`] | node | `tests/review_p1_compile_fail/a_canonical_edge_may_target_a_candidate_node.rs` |
//! | [`Subject::Node`], [`Object::Node`], [`CanonicalValue::NodeRef`] | node | `tests/compile_fail/a_canonical_claim_names_its_nodes_by_canonical_reference.rs` |
//! | [`Subject::Edge`] | edge | `tests/compile_fail/a_canonical_subject_names_its_edge_by_canonical_reference.rs` |
//! | [`Assertion::evidence`] | evidence | `tests/compile_fail/a_canonical_assertion_cites_evidence_by_canonical_reference.rs` |
//! | [`AssertionLifecycle::Superseded`] `by` | assertion | `tests/adversary_p1_14_exit_compile_fail/a_canonical_supersession_names_its_replacement_by_canonical_reference.rs` |
//! | [`Assessment::Disputed`] `competing_assertions` | assertion | `tests/adversary_p1_14_exit_compile_fail/a_canonical_dispute_names_its_competitors_by_canonical_reference.rs` |
//! | [`EvidenceSource::GraphAssertion`] | assertion | `tests/adversary_p1_14_exit_compile_fail/retained_evidence_names_its_source_assertion_by_canonical_reference.rs` |
//!
//! Not in the table, because canonical state keeps no map to resolve them against:
//! [`EvidenceSource::Observation`] (observations are not canonical state) and the graph root, which
//! the kernel's reference validator resolves as an equality.
//!
//! # The address is a type too
//!
//! `architecture-decision-record:0005-float-is-not-canonical`, as amended: [`Node`], [`Edge`] and
//! [`Assertion`] are generic over the value they carry and default to [`CanonicalValue`], which has
//! no float at any depth. A [`CanonicalGraph`] holds the default; a [`TransientGraph`] holds the
//! same three types over [`ekr_ontology::Value`], so a candidate may carry the approximate
//! measurement an import arrived with.
//!
//! [`Canonical`](ekr_core::canonical::Canonical) is implemented only where that parameter is
//! itself `Canonical`, which makes *only canonical state can be content-addressed* a property of
//! the type system rather than a convention: `ContentHash::of(&node)` compiles for a canonical node
//! and does not compile for a candidate.
//! `tests/compile_fail/transient_state_has_no_content_address.rs` is that guarantee as a build
//! failure, beside the three that hold the reference direction.
//!
//! # No writer
//!
//! Nothing here mutates canonical state. Every type is data with public fields and, where the
//! domain declares a field this crate answers rather than stores, an accessor. Construction of a
//! `ValidatedTransaction` — the one thing that may commit — belongs to `ekr-kernel`, which is
//! AGENTS.md invariant 1.
//!
//! ```
//! use std::collections::{BTreeMap, BTreeSet};
//!
//! use ekr_core::{AgentId, AssertionId, GraphRootId, NodeId, RevisionNumber, SchemaVersionId,
//!                Timestamp, TypeId};
//! use ekr_graph::{Assertion, CanonicalGraph, CanonicalRef, GraphRoot, GraphSnapshot, Node, Object,
//!                 Predicate, Space, Subject, TemporalRange, TransactionTime, Assessment,
//!                 AssertionLifecycle};
//! use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
//!
//! let (root_id, schema) = (GraphRootId::mint(), SchemaVersionId::mint());
//! let (alice, acme, ceo_of) = (NodeId::mint(), NodeId::mint(), TypeId::mint());
//! let handover = Timestamp::from_millis(1_773_273_600_000); // 2026-03-12
//! let recorded = Timestamp::from_millis(1_704_067_200_000);  // 2024-01-01
//!
//! let held = Assertion {
//!     id: AssertionId::mint(),
//!     root_id,
//!     subject: Subject::Node(CanonicalRef::new(alice)),
//!     predicate: Predicate::Relation(ceo_of),
//!     object: Object::Node(CanonicalRef::new(acme)),
//!     evidence: BTreeSet::new(),
//!     proposed_by: AgentId::mint(),
//!     assessment: Assessment::Accepted { validators: BTreeSet::new() },
//!     lifecycle: AssertionLifecycle::Active,
//!     // Closed: the world moved on, and the record says so rather than disappearing. The
//!     // constructor refuses an end before its start, so the range is `Option`.
//!     valid_time: TemporalRange::new(None, Some(handover)).expect("a bound is not inverted"),
//!     // Open, and with a start: a belief the runtime holds is one it began holding somewhere.
//!     transaction_time: TransactionTime::since(recorded),
//! };
//! let id = held.id;
//!
//! let graph = CanonicalGraph {
//!     root: GraphRoot {
//!         id: root_id,
//!         space: Space::Canonical,
//!         schema_version_id: schema,
//!         parent: None,
//!         created_at: Timestamp::EPOCH,
//!     },
//!     revision: RevisionNumber::new(1),
//!     ontology: Ontology::load(OntologyDocument {
//!         version: SchemaVersion::seed(schema, Timestamp::EPOCH),
//!         node_types: Vec::new(),
//!         edge_types: Vec::new(),
//!     })?,
//!     nodes: BTreeMap::new(),
//!     edges: BTreeMap::new(),
//!     assertions: [(id, held)].into_iter().collect(),
//!     evidence: BTreeMap::new(),
//! };
//!
//! // One read, asked twice. Before the handover the claim held; at it and after, it did not.
//! let snapshot = GraphSnapshot::of(&graph);
//! assert_eq!(snapshot.valid_at(Timestamp::EPOCH).len(), 1, "history remains readable");
//! assert!(snapshot.valid_at(handover).is_empty(), "valid time is half-open");
//! # Ok::<(), ekr_ontology::OntologyError>(())
//! ```

pub mod assertion;
pub mod canonical;
pub mod edge;
pub mod events;
pub mod evidence;
pub mod node;
pub mod root;
pub mod snapshot;
pub mod transient;
pub mod value;

pub use assertion::{
    Assertion, AssertionLifecycle, Assessment, InvertedRange, Object, Predicate, RetractionReason,
    Subject, TemporalRange, TransactionTime,
};
pub use canonical::{
    CanonicalDependency, CanonicalGraph, CanonicalRef, CanonicalTarget, ValueSpace,
};
pub use edge::Edge;
pub use events::{RevisionEvent, RevisionPayload};
pub use evidence::{
    Confidence, ConfidenceOutOfRange, Evidence, EvidenceKind, EvidenceSource, Observation,
    ObservationContent, ObservationKind, Support,
};
pub use node::Node;
pub use root::{GraphRoot, Root, Space};
pub use snapshot::GraphSnapshot;
pub use transient::{LocalRef, Resolved, TransientGraph, TransientRef};
pub use value::{CanonicalValue, InadmissibleValue};
/// Frozen original-format data; decoding grants no canonical authority.
pub mod legacy;
