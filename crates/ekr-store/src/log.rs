//! Verified storage inputs and the fallible kernel authority boundary.
use crate::{StorageClass, StoreError, StoredObject};
use ekr_core::{Canonical, ContentHash, Encoder, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{CanonicalGraph, RevisionEvent, Root};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Value-domain address of complete node, edge and assertion collections, in that order.
#[must_use]
pub fn knowledge_root(graph: &CanonicalGraph) -> ContentHash {
    struct Knowledge<'a>(&'a CanonicalGraph);
    impl Canonical for Knowledge<'_> {
        fn encode(&self, out: &mut Encoder) {
            self.0.nodes.encode(out);
            self.0.edges.encode(out);
            self.0.assertions.encode(out);
        }
    }
    ContentHash::of(&Knowledge(graph))
}
/// Value-domain address of the complete evidence collection.
#[must_use]
pub fn evidence_root(graph: &CanonicalGraph) -> ContentHash {
    ContentHash::of(&graph.evidence)
}

/// A retained occurrence with the provider coordinates preserved alongside domain identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedOccurrence {
    /// One-based revision-stream position, not a canonical revision number.
    pub version: u64,
    /// Opaque native provider event identity, distinct from domain EventId.
    pub provider_event_id: String,
    /// The checked current event envelope.
    pub event: RevisionEvent,
}
/// Verified payload bytes and their strongest retention metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedObject {
    /// Verified object address, size, retention and original storage time.
    pub metadata: StoredObject,
    /// Actual native blob bytes, already checked against metadata and content address.
    pub bytes: Vec<u8>,
}
/// Immutable inputs to pure kernel replay, assembled outside a provider transaction.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RetainedHistory {
    /// Ordered complete revision stream.
    pub occurrences: Vec<RecordedOccurrence>,
    /// All objects required by the injected authority.
    pub objects: BTreeMap<ContentHash, RetainedObject>,
}
impl RetainedHistory {
    /// Reads verified bytes at a minimum retention strength.
    /// # Errors
    /// Missing, corrupt or insufficiently retained objects refuse replay.
    pub fn content(&self, hash: ContentHash, class: StorageClass) -> Result<&[u8], StoreError> {
        let held = self
            .objects
            .get(&hash)
            .ok_or_else(|| StoreError::Document("required-object-missing".into()))?;
        if held.metadata.content_hash != hash
            || held.metadata.byte_len != held.bytes.len() as u64
            || held.metadata.storage_class.retention_rank() < class.retention_rank()
            || ContentHash::of_bytes(&held.bytes) != hash
        {
            return Err(StoreError::Document("required-object-integrity".into()));
        }
        Ok(&held.bytes)
    }
}
/// A state independently reconstructed and verified by the kernel.
#[derive(Clone, Debug)]
pub struct AdmittedRevision {
    /// Full canonical graph after applying its transactions.
    pub graph: CanonicalGraph,
    /// Recomputed complete root.
    pub root: Root,
    /// Domain identity of this committed revision.
    pub revision_id: RevisionId,
    /// Occurrence that created it.
    pub event_id: EventId,
    /// Complete seed result or commit receipt address.
    pub record_hash: ContentHash,
    /// Original trusted commit time.
    pub committed_at: Timestamp,
}
/// Fallible kernel interpretation. Deserialized records never authorize themselves.
pub trait CommitAuthority {
    /// Additional payloads needed beyond event records and the seed envelope.
    /// # Errors
    /// Malformed retained records refuse discovery.
    fn required_objects(
        &self,
        history: &RetainedHistory,
    ) -> Result<BTreeSet<ContentHash>, StoreError>;
    /// Reconstructs and verifies history, optionally stopping at a committed revision.
    /// # Errors
    /// Any unsupported, incomplete or inconsistent history refuses explicitly.
    fn replay(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        revision: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError>;
}
/// One object staged for atomic publication. No provider bytes exist merely because it is staged.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationObject {
    /// Requested minimum retention.
    pub storage_class: StorageClass,
    /// Trusted decision time used for a new metadata record.
    pub stored_at: Timestamp,
    /// Exact staged native payload.
    pub bytes: Vec<u8>,
}
/// An immutable domain occurrence and all objects it publishes atomically.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Publication {
    /// Exact current occurrence, allocated once by the kernel.
    pub event: RevisionEvent,
    /// Required objects keyed by their actual payload-domain addresses.
    #[serde(deserialize_with = "ekr_core::decode::unique_map")]
    pub objects: BTreeMap<ContentHash, PublicationObject>,
    /// Read revision-stream position. Zero means Expected::NoStream on every retry.
    pub expected_version: u64,
}
/// Whether this exact occurrence was newly written or already retained.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[must_use]
pub enum Appended {
    /// Native atomic publication completed.
    Written,
    /// Exact domain occurrence and content were already retained.
    AlreadyRecorded,
}
/// Synchronous storage port. Kernel replay owns all domain admission and root construction.
pub trait RevisionLog {
    /// Reads the elected private attempt without publishing or recovering anything.
    /// # Errors
    /// Corrupt or inconsistent preparation history refuses.
    fn preparation(
        &self,
        key: &crate::PublicationCommandKey,
    ) -> Result<Option<crate::PublicationPreparationV1>, StoreError>;
    /// Elects an initial decision, or a successor after definitively resolving its predecessor.
    /// # Errors
    /// Input conflict, conditional contention, invalid authority or unresolved selection.
    fn prepare(
        &self,
        key: &crate::PublicationCommandKey,
        input_hash: ContentHash,
        decision: &Publication,
        previous: Option<&crate::PublicationPreparationV1>,
    ) -> Result<crate::PublicationPreparationV1, StoreError>;
    /// Retries the exact elected native request before reinterpreting head movement.
    /// # Errors
    /// Definite conflict, unresolved result, corruption or failed kernel admission.
    fn resume(&self, preparation: &crate::PublicationPreparationV1)
        -> Result<Appended, StoreError>;
    /// Complete verified immutable inputs, including retained decision records.
    /// # Errors
    /// Any physical history or object integrity failure.
    fn history(&self) -> Result<RetainedHistory, StoreError>;
    /// Verified history ending exactly at the requested committed revision.
    /// # Errors
    /// Missing revision or invalid required prefix; later payloads are never loaded.
    fn history_at(&self, revision: RevisionNumber) -> Result<RetainedHistory, StoreError>;
    /// Publishes one occurrence and its objects after fallible kernel admission.
    /// # Errors
    /// Conditional contention, invalid candidate history or an unresolved commit outcome.
    fn publish(&self, publication: &Publication) -> Result<Appended, StoreError>;
    /// Retained complete seed envelope, if the lineage exists.
    /// # Errors
    /// Invalid history or a missing seed payload.
    fn seed_bytes(&self) -> Result<Option<Vec<u8>>, StoreError>;
    /// Current verified canonical graph.
    /// # Errors
    /// Missing seed or invalid history.
    fn fold(&self) -> Result<CanonicalGraph, StoreError>;
    /// Current complete verified root.
    /// # Errors
    /// Invalid history or absent authority.
    fn head(&self) -> Result<Option<Root>, StoreError>;
    /// Reconstructs the selected committed revision, not a stream offset.
    /// # Errors
    /// Missing revision or invalid history through that revision.
    fn replay(&self, revision: RevisionNumber) -> Result<CanonicalGraph, StoreError>;
}
/// Initialization requires the same complete atomic publication path with no existing stream.
pub trait Initialize {
    /// Atomically creates the lineage from a kernel-owned Seeded occurrence.
    /// # Errors
    /// An existing different lineage, invalid seed, contention or unresolved commit outcome.
    fn initialize(&self, publication: &Publication) -> Result<Appended, StoreError>;
}
