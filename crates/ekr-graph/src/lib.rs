//! The graph state of the Epistemic Knowledge Runtime.
//!
//! Implements the `ekr.graph` domain, `systems/ekr/domains/graph.yaml`: graph roots, nodes,
//! edges and bitemporal assertions with their validation state, evidence and observations. The
//! crate holds no writer of its own; every change arrives as a transaction the kernel committed.
//!
//! Nine modules, in dependency order:
//!
//! * [`root`] — [`GraphRoot`] and [`Space`] (design § 22–23), and [`Root`], the immutable
//!   revision root of design § 34.
//! * [`node`], [`edge`] — [`Node`] (design § 10, amendment 87) and [`Edge`] (design § 12).
//! * [`evidence`] — [`Observation`], [`Evidence`] and [`Support`] (design § 15–16, amendment 86).
//! * [`assertion`] — [`Assertion`] with its [`Subject`], [`Predicate`], [`Object`],
//!   [`TemporalRange`], [`TransactionTime`], [`ValidationState`] and [`AssertionStatus`]
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
//! use ekr_graph::{Assertion, CanonicalGraph, GraphRoot, GraphSnapshot, Node, Object, Predicate,
//!                 Space, Subject, TemporalRange, TransactionTime, ValidationState};
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
//!     subject: Subject::Node(alice),
//!     predicate: Predicate::Relation(ceo_of),
//!     object: Object::Node(acme),
//!     evidence: BTreeSet::new(),
//!     proposed_by: AgentId::mint(),
//!     validation: ValidationState::Accepted { validators: BTreeSet::new() },
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

pub use assertion::{
    Assertion, AssertionStatus, InvertedRange, Object, Predicate, RetractionReason, Subject,
    TemporalRange, TransactionTime, ValidationState,
};
pub use canonical::{CanonicalDependency, CanonicalGraph, CanonicalRef, CanonicalTarget};
pub use edge::Edge;
pub use events::RevisionEvent;
pub use evidence::{
    Confidence, ConfidenceOutOfRange, Evidence, EvidenceKind, EvidenceSource, Observation,
    ObservationContent, ObservationKind, Support,
};
pub use node::Node;
pub use root::{GraphRoot, Root, Space};
pub use snapshot::GraphSnapshot;
pub use transient::{LocalRef, TransientGraph, TransientRef};
