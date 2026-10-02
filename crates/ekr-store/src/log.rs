//! Verified storage inputs and the fallible kernel authority boundary.
use crate::{StorageClass, StoreError, StoredObject};
use ekr_core::{Canonical, ContentHash, Encoder, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{CanonicalGraph, RevisionEvent, Root};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

thread_local! {
    static KNOWLEDGE_ROOTS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
/// How many knowledge roots, each a hash of a whole graph, the calling thread has computed.
///
/// Test instrumentation, as [`crate::ReadWork`] is: it lets a test show that a commit hashes the
/// graph once. Counted per thread because every store and kernel call runs on its caller's
/// thread, and tests in one binary run on several.
#[doc(hidden)]
#[must_use]
pub fn knowledge_roots_hashed() -> u64 {
    KNOWLEDGE_ROOTS.with(std::cell::Cell::get)
}
/// Value-domain address of complete node, edge and assertion collections, in that order, and of
/// the attachment collection after them when it holds one.
///
/// The attachments (`story:evidence-attaches-to-a-held-assertion`, design § 103.3) are written as a
/// tagged `Some` and only when there are any, the shape `GraphTransaction::schema_version` takes:
/// a graph with none encodes to exactly the bytes it encoded to before the collection existed, so
/// no root a store records moves, and the `Some` tag cannot be read as anything else.
#[must_use]
pub fn knowledge_root(graph: &CanonicalGraph) -> ContentHash {
    struct Knowledge<'a>(&'a CanonicalGraph);
    impl Canonical for Knowledge<'_> {
        fn encode(&self, out: &mut Encoder) {
            self.0.nodes.encode(out);
            self.0.edges.encode(out);
            self.0.assertions.encode(out);
            if !self.0.attachments.is_empty() {
                out.option(Some(&self.0.attachments));
            }
        }
    }
    KNOWLEDGE_ROOTS.with(|count| count.set(count.get() + 1));
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
    ///
    /// Shared, not owned: every history a store handle returns holds the one allocation that
    /// handle verified, so a later read of the same object copies nothing. Changing them
    /// (`Arc::make_mut`) or replacing them gives them another allocation, which
    /// [`RetainedHistory::content`] checks in full.
    pub bytes: Arc<Vec<u8>>,
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
    ///
    /// The bytes are checked against `hash` on every call. Bytes that are the very allocation
    /// this process already hashed to `hash` are that copy and are neither compared nor hashed;
    /// bytes equal to it in another allocation are compared rather than hashed again, which gives
    /// the same answer (`crate::verified`); any other bytes are hashed.
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
            || !crate::verified::addresses(hash, &held.bytes)
        {
            return Err(StoreError::Document("required-object-integrity".into()));
        }
        Ok(held.bytes.as_slice())
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
    /// [`Self::required_objects`] for a history the store only has the authority replay, never
    /// hands to a reader: the payloads a replay continuing from what this authority has already
    /// reached reads. A store may load only these for such a history, and loads
    /// [`Self::required_objects`] instead whenever the replay refuses it, so the answer is the one
    /// the complete history gives. [`Self::required_objects`] by default.
    /// # Errors
    /// Malformed retained records refuse discovery.
    fn replay_objects(
        &self,
        history: &RetainedHistory,
    ) -> Result<BTreeSet<ContentHash>, StoreError> {
        self.required_objects(history)
    }
    /// Payloads replay reads where the store holds them, and whose absence the authority judges
    /// itself, by name: the evidence payloads an `ekr-seed-envelope/3` names (design § 100.1).
    /// The store loads each of them it holds an object for, verified as a required object is, and
    /// leaves out each it holds none for. None by default.
    /// # Errors
    /// Malformed retained records refuse discovery.
    fn objects_if_held(
        &self,
        history: &RetainedHistory,
    ) -> Result<BTreeSet<ContentHash>, StoreError> {
        let _ = history;
        Ok(BTreeSet::new())
    }
    /// Reconstructs and verifies history, optionally stopping at a committed revision.
    /// # Errors
    /// Any unsupported, incomplete or inconsistent history refuses explicitly.
    fn replay(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        revision: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError>;
    /// [`Self::replay`] for a caller that needs only the verdict, not the admitted state: `Ok`
    /// exactly where `replay` admits the history or finds it empty, and `replay`'s refusal
    /// otherwise. An authority may answer it without building the admitted head graph.
    /// # Errors
    /// Whatever [`Self::replay`] refuses.
    fn verify(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        revision: Option<RevisionNumber>,
    ) -> Result<(), StoreError> {
        self.replay(history, ontology, revision).map(|_| ())
    }
    /// [`Self::replay`] for a caller that needs only the admitted revision's root, such as a head
    /// read no checkpoint pointer answers: the root exactly where `replay` admits a revision,
    /// `None` where it finds the history empty, and `replay`'s refusal otherwise. An authority
    /// may answer it without building the admitted head graph.
    /// # Errors
    /// Whatever [`Self::replay`] refuses.
    fn replay_root(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        revision: Option<RevisionNumber>,
    ) -> Result<Option<Root>, StoreError> {
        Ok(self
            .replay(history, ontology, revision)?
            .map(|admitted| admitted.root))
    }
    /// Offers the retained replay checkpoint for `history`, which the authority may admit as the
    /// state its next replay of that history continues from. An authority that keeps none
    /// ignores it.
    /// # Errors
    /// A checkpoint the authority does not admit; the store then ignores it.
    fn restore(&self, history: &RetainedHistory, checkpoint: &[u8]) -> Result<(), StoreError> {
        let _ = (history, checkpoint);
        Ok(())
    }
    /// The head root of `history`, if `binding` is this authority's own record of having
    /// verified all of its occurrences. `history` holds every occurrence and, of the objects,
    /// only the head revision's record. `None` sends the caller to a full replay.
    /// # Errors
    /// A head record that does not decode.
    fn checkpointed_head(
        &self,
        history: &RetainedHistory,
        binding: ContentHash,
    ) -> Result<Option<Root>, StoreError> {
        let _ = (history, binding);
        Ok(None)
    }
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
    /// [`Self::history`] for a caller that only replays it: the same occurrences, verified, with
    /// the objects [`CommitAuthority::replay_objects`] names rather than every required one. Where
    /// replaying that history is refused, the complete [`Self::history`] is loaded and its answer
    /// returned. [`Self::history`] by default.
    /// # Errors
    /// Whatever [`Self::history`] refuses.
    fn replay_history(&self) -> Result<RetainedHistory, StoreError> {
        self.history()
    }
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
    /// How many revision-stream occurrences the newest checkpoint pointer the log holds now says
    /// were verified, read from the log rather than from anything this handle remembers; `None`
    /// when it holds none, or keeps none (design § 99.5).
    /// # Errors
    /// Provider failure.
    fn checkpoint_covered(&self) -> Result<Option<u64>, StoreError> {
        Ok(None)
    }
    /// Records that the authority verified the first `covered` occurrences of the revision stream,
    /// under its own `binding` of that prefix, and retains `checkpoint` as their replay checkpoint,
    /// replacing an older one. Without `checkpoint` the retained checkpoint is kept and only the
    /// record of verification advances. Private cache data: it confers no authority and may be
    /// lost at any time. A log that keeps none ignores it.
    ///
    /// `true` when the newest pointer is this one afterwards — appended now, or already the newest
    /// — so that `checkpoint`, when given, is the retained one; `false` when nothing was written:
    /// the append lost to another writer, there is no retained checkpoint for a pointer without
    /// one to name, or the log keeps none (design § 99).
    /// # Errors
    /// Provider failure.
    fn write_checkpoint(
        &self,
        covered: u64,
        binding: ContentHash,
        checkpoint: Option<&[u8]>,
    ) -> Result<bool, StoreError> {
        let _ = (covered, binding, checkpoint);
        Ok(false)
    }
}
/// Initialization requires the same complete atomic publication path with no existing stream.
pub trait Initialize {
    /// Atomically creates the lineage from a kernel-owned Seeded occurrence.
    /// # Errors
    /// An existing different lineage, invalid seed, contention or unresolved commit outcome.
    fn initialize(&self, publication: &Publication) -> Result<Appended, StoreError>;
}
