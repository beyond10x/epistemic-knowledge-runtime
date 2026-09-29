//! Builders for every document a consumer writes, and the ontology by name
//! (`story:sdk-typed-documents`).
//!
//! Four formats, each a serde model of what its reader decodes:
//!
//! * [`TransactionDocument`] — `ekr.transaction-document/2`, one [`Operation`] per kind
//!   `ekr operations` lists, built with [`TransactionBuilder`];
//! * [`SeedDocument`] — `ekr-seed/2`, built with [`SeedBuilder`], usually from an
//!   [`OntologySpec`];
//! * [`TypedReference`] — the document `ekr resolve` reads;
//! * the ontology by name: [`OntologySpec`] declares types, properties and edge ends by name,
//!   [`Ontology`] maps those names to the ids a store holds (read from `ekr ontology`), and
//!   [`Ontology::ensure`] writes the schema operations a store is missing.
//!
//! [`to_yaml`] writes every one of them, and writes each variant as a YAML tag (`!AddAssertion`,
//! `!Node <id>`, `!HumanStatement`): the readers refuse the one-key JSON form `{"!Kind": value}`,
//! so a document is never written as JSON.
//!
//! Ids and hashes are computed here, not asked for. A fresh id is `ekr-core`'s `Id::mint()`
//! (`NodeId::mint()`, …), the function `ekr mint` runs; a payload's content hash is
//! [`payload_hash`], the `ekr.payload.v1` hash `ekr hash` prints. Everything here is blocking
//! and does no I/O.
//!
//! `crates/ekr-sdk/tests/document_drift.rs` holds every builder to the kernel's readers, to
//! `ekr schema` and to a real store.

mod graph;
mod limits;
mod ontology;
mod reference;
mod seed;
mod transaction;
mod value;
mod yaml;

pub use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    SchemaVersionId, Timestamp, TransactionId, TypeId,
};

pub use graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, EdgeDraft, Evidence, EvidenceSource,
    NodeDraft, Object, Predicate, Subject, TemporalRange, TransactionTime,
};
pub use limits::{DocumentLimit, DocumentLimits, TRANSACTION_LIMITS};
pub use ontology::{
    EdgeTypeSpec, NodeTypeSpec, Ontology, OntologyError, OntologySpec, PropertySpec, SchemaChange,
    ValidationProfile, ValueSpec,
};
pub use reference::TypedReference;
pub use seed::{
    GraphRoot, GraphSection, OntologySection, SchemaVersion, SeedBuilder, SeedDocument, SeedGraph,
    SeedNode, Space, GRAPH_FORMAT, SEED_FORMAT,
};
pub use transaction::{
    EdgeWidening, EntityMerge, EvidenceAddition, Invocation, Operation, OperationKind,
    PropertyModification, PropertyMutation, Retraction, Supersession, Transaction,
    TransactionBuilder, TransactionDocument, TRANSACTION_FORMAT,
};
pub use value::{
    Cardinality, EdgeType, Lifecycle, NodeType, OperationDefinition, PropertyDefinition,
    Transition, Value, ValueType,
};
pub use yaml::to_yaml;

/// The content hash of a payload's exact bytes: sha256(`ekr.payload.v1` || bytes), what
/// `ekr hash` prints as `content_hash` and what an evidence entry's `content_hash` must be. A
/// trailing newline is part of the bytes.
#[must_use]
pub fn payload_hash(bytes: &[u8]) -> ContentHash {
    ContentHash::of_bytes(bytes)
}

/// Why a document was not built or not written.
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// A transaction holds at least one operation; the reader refuses an empty one.
    #[error("a transaction holds at least one operation")]
    EmptyTransaction,
    /// A transaction that holds a schema change holds nothing else; every profile refuses the
    /// mixture (`mixed-schema-transaction`, or `unsupported-operation` under profile v1).
    #[error("a schema change travels alone: this transaction mixes schema and data operations")]
    MixedSchemaTransaction,
    /// The document is past one of the format's frozen limits, which `ekr propose` would refuse
    /// by the same name: split the change into several transactions, or put a larger statement
    /// into the seed.
    #[error(
        "transaction document limit: {name} (at most {bound}): this document has {value}",
        name = limit.name()
    )]
    Limit {
        /// The limit.
        limit: DocumentLimit,
        /// Its inclusive bound.
        bound: usize,
        /// What the document has.
        value: usize,
    },
    /// The YAML writer refused the value.
    #[error("writing YAML: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),
}
