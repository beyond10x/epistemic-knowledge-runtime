//! The trusted kernel of the Epistemic Knowledge Runtime.
//!
//! Implements the command half of the `ekr.kernel` domain,
//! `systems/ekr/domains/kernel.yaml`: the transaction boundary from proposal through validation
//! to an immutable committed revision, snapshot reads and the explain chain. This is the only
//! crate that constructs a validated transaction; the domain's types live in `ekr-core`.
//!
//! Four modules, in dependency order:
//!
//! * [`transaction`] — [`GraphTransaction`] and its [`GraphOperation`]s (design § 19, plus
//!   amendment 87's `Invoke`), and [`ValidatedTransaction`], which only this crate builds.
//! * [`issue`] — [`ValidationIssue`] and [`ValidatorName`], what a refusal says.
//! * [`validate`] — the [`Validator`] trait of design § 20 and the
//!   [`Pipeline`] of its first seven, deterministic, validators.
//! * [`commit`] — [`Commit`], the one path that spends a [`ValidatedTransaction`], and
//!   [`Validations`], the authority `ekr-store`'s fold commits on
//!   (`architecture-decision-record:0007-the-commit-path-is-the-kernels`).
//!
//! # The membrane is a type, not a rule
//!
//! AGENTS.md invariant 1: "Only `ekr-kernel` constructs a `ValidatedTransaction`, and only a
//! `ValidatedTransaction` commits." [`ValidatedTransaction`]'s fields are private and it derives no
//! `Deserialize`, so the only one that exists anywhere is one [`Pipeline::validate`] built.
//! `tests/compile_fail/` holds both of those as build failures, because a comment saying a type
//! cannot be built is not a thing anybody can check.
//!
//! The second half is [`commit`]'s, and it is **not** a type — ADR 0007 says which mechanism
//! carries it and how far it reaches, and `AGENTS.md`'s own sentence was narrowed to match.
//!
//! # And no model runs in it
//!
//! AGENTS.md invariant 7, and design § 73: identity, references, types, cardinality, transaction
//! integrity and provenance are code. Every validator reads a
//! [`GraphSnapshot`](ekr_graph::GraphSnapshot) and the ontology in it and nothing else — no clock,
//! no network, no model — so one proposal against one snapshot yields the same issues and the same
//! [`ValidatedTransaction::validation_hash`] on every run.
//!
//! ```
//! use std::collections::{BTreeMap, BTreeSet};
//!
//! use ekr_core::{AgentId, GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp,
//!                TransactionId, TypeId};
//! use ekr_graph::{CanonicalGraph, GraphRoot, GraphSnapshot, Space};
//! use ekr_kernel::{GraphOperation, GraphTransaction, NodeDraft, Pipeline};
//! use ekr_ontology::{NodeType, Ontology, OntologyDocument, SchemaVersion};
//!
//! let (root_id, schema, decision) = (GraphRootId::mint(), SchemaVersionId::mint(), TypeId::mint());
//! let graph = CanonicalGraph {
//!     root: GraphRoot {
//!         id: root_id,
//!         space: Space::Canonical,
//!         schema_version_id: schema,
//!         parent: None,
//!         created_at: Timestamp::EPOCH,
//!     },
//!     revision: RevisionNumber::new(7),
//!     ontology: Ontology::load(OntologyDocument {
//!         version: SchemaVersion::seed(schema, Timestamp::EPOCH),
//!         node_types: vec![NodeType::new(decision, "Decision")],
//!         edge_types: Vec::new(),
//!     })?,
//!     nodes: BTreeMap::new(),
//!     edges: BTreeMap::new(),
//!     assertions: BTreeMap::new(),
//!     evidence: BTreeMap::new(),
//! };
//!
//! let (proposer, reviewer) = (AgentId::mint(), AgentId::mint());
//! let proposal = GraphTransaction {
//!     id: TransactionId::mint(),
//!     proposer,
//!     operations: vec![GraphOperation::CreateNode(NodeDraft {
//!         id: NodeId::mint(),
//!         root_id,
//!         type_id: decision,
//!         canonical_name: "Hash canonical state only".to_owned(),
//!         properties: BTreeMap::new(),
//!     })],
//!     evidence: BTreeSet::new(),
//! };
//!
//! let snapshot = GraphSnapshot::of(&graph);
//! let validated = Pipeline::deterministic(reviewer)
//!     .validate(&snapshot, &proposal)
//!     .expect("nothing is wrong with this proposal");
//! assert_eq!(validated.validated_against(), RevisionNumber::new(7));
//!
//! // Design § 6.10: the proposer is never the sole basis of validation.
//! assert!(Pipeline::deterministic(proposer).validate(&snapshot, &proposal).is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod apply;
pub mod authority;
pub mod commands;
pub mod commit;
pub mod document;
pub mod issue;
mod read;
pub mod records;
mod replay;
pub mod runtime;
pub mod seed;
pub mod transaction;
pub mod validate;

pub use authority::{Agent, AuthorityStateV1, ValidationProfileV1};
pub use commands::{CommitCommandResult, ValidationCommandResult};
pub use commit::{Commit, CommitError, KernelAuthority};
pub use document::{
    DocumentError, DocumentLimit, DocumentLimits, TransactionDocument, DOCUMENT_V1_LIMITS,
};
/// Typed persistence failures exposed without granting the caller storage or writer access.
pub use ekr_store::StoreError as PersistenceError;
pub use issue::{ValidationIssue, ValidatorName};
pub use read::{VerifiedRead, VerifiedRevision};
pub use records::{
    CommitReceiptV1, ProposalRecordV1, RecordedValidationIssue, RejectionRecordV1, SeedResultV1,
    StaleRecordV1, ValidationBasisV1, ValidationMaterialV1, ValidationReceiptV1,
};
pub use replay::{TransactionRecord, TransactionState};
pub use runtime::Runtime;
pub use seed::{BootstrapContext, SeedDocument, SeedError};
pub use transaction::{
    EdgeDraft, EntityMerge, GraphOperation, GraphTransaction, NodeDraft, PropertyMutation,
    Retraction, Supersession, ValidatedTransaction,
};
pub use validate::{
    Authorization, Cardinality, OntologyConstraint, Pipeline, Provenance, Reference, Structural,
    Types, Validator,
};
/// Frozen original-format seed and transaction verification.
pub mod legacy;
