//! Kernel-owned admission, immutable retained results and replay authority.
use crate::seed::{self, BootstrapContext, SeedDocument, SeedEnvelope, SeedError};
use crate::{AuthorityStateV1, SeedResultV1};
use ekr_core::{ContentHash, EventId, RevisionId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{CanonicalGraph, RevisionEvent, RevisionPayload, Root};
use ekr_store::{
    evidence_root, knowledge_root, AdmittedRevision, CommitAuthority, Initialize, ObjectStore,
    Publication, PublicationObject, RetainedHistory, RevisionLog, StorageClass, StoreError,
};
use std::collections::BTreeSet;
use std::sync::Arc;

/// The immutable trusted anchor installed by the kernel; retained records never replace it.
#[derive(Clone)]
pub struct KernelAuthority {
    pub(crate) context: BootstrapContext,
    pub(crate) anchor: AuthorityStateV1,
    /// Verified replay states this authority reached, shared by every clone of it.
    pub(crate) cache: std::sync::Arc<std::sync::Mutex<crate::replay::ReplayCache>>,
    /// The seed input this authority assembled from verified retained bytes, by the seed
    /// envelope's address, shared by every clone of it and every read that returns it.
    pub(crate) seed_input: std::sync::Arc<std::sync::Mutex<Option<HeldSeedInput>>>,
}
/// A seed input assembled from verified retained bytes, with the seed envelope address it is for.
type HeldSeedInput = (ContentHash, Arc<SeedDocument>);
impl CommitAuthority for KernelAuthority {
    /// The seed envelope's evidence payloads, and the payload of every evidence entry a committed
    /// `AddEvidence` brought: what a verified read, `explain` and replay read by content hash.
    fn required_objects(
        &self,
        history: &RetainedHistory,
    ) -> Result<BTreeSet<ContentHash>, StoreError> {
        let Some(first) = history.occurrences.first() else {
            return Ok(BTreeSet::new());
        };
        let RevisionPayload::Seeded { seed_hash, .. } = first.event.payload else {
            return Err(StoreError::NotSeeded);
        };
        // An `ekr-seed-envelope/2` requires the payloads it carries; an `/3` requires none and
        // reads the ones it names where held (`objects_if_held`), judging an absent one itself.
        let (payloads, named) = self.seed_payloads(history, seed_hash)?;
        let mut required = if named { BTreeSet::new() } else { payloads };
        required.extend(self.added_evidence_required(history, false)?);
        Ok(required)
    }
    /// [`Self::required_objects`] with only the added payloads a replay continuing from the state
    /// this authority reached reads: those the commits after it added. Every earlier one was
    /// checked by the replay that reached that state, and replay never reads it again.
    fn replay_objects(
        &self,
        history: &RetainedHistory,
    ) -> Result<BTreeSet<ContentHash>, StoreError> {
        let Some(first) = history.occurrences.first() else {
            return Ok(BTreeSet::new());
        };
        let RevisionPayload::Seeded { seed_hash, .. } = first.event.payload else {
            return Err(StoreError::NotSeeded);
        };
        let (payloads, named) = self.seed_payloads(history, seed_hash)?;
        let mut required = if named { BTreeSet::new() } else { payloads };
        required.extend(self.added_evidence_required(history, true)?);
        Ok(required)
    }
    fn objects_if_held(
        &self,
        history: &RetainedHistory,
    ) -> Result<BTreeSet<ContentHash>, StoreError> {
        let Some(first) = history.occurrences.first() else {
            return Ok(BTreeSet::new());
        };
        let RevisionPayload::Seeded { seed_hash, .. } = first.event.payload else {
            return Err(StoreError::NotSeeded);
        };
        let (payloads, named) = self.seed_payloads(history, seed_hash)?;
        let mut wanted = if named { payloads } else { BTreeSet::new() };
        // Until this authority has seen the store settled, the markers of a preserving migration
        // (design § 100.3) are read where held, so an unfinished one is refused by name.
        if !self.cache()?.migration_settled {
            wanted.extend(crate::migrate::required_markers(self, history)?);
        }
        Ok(wanted)
    }
    fn replay(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        revision: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        self.reconstruct(history, ontology, revision)?
            .map(|state| state.head().admitted())
            .transpose()
    }
    /// [`CommitAuthority::replay`]'s verdict without copying the head graph into an admitted
    /// revision: the same reconstruction, and the same refusal where the head graph is not held.
    fn verify(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        revision: Option<RevisionNumber>,
    ) -> Result<(), StoreError> {
        if let Some(state) = self.reconstruct(history, ontology, revision)? {
            state.head().graph()?;
        }
        Ok(())
    }
    /// [`CommitAuthority::replay`]'s root without copying the head graph into an admitted
    /// revision: the same reconstruction, and the same refusal where the head graph is not held.
    fn replay_root(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        revision: Option<RevisionNumber>,
    ) -> Result<Option<Root>, StoreError> {
        let Some(state) = self.reconstruct(history, ontology, revision)? else {
            return Ok(None);
        };
        let head = state.head();
        head.graph()?;
        Ok(Some(head.root))
    }
    fn restore(&self, history: &RetainedHistory, checkpoint: &[u8]) -> Result<(), StoreError> {
        self.restore_checkpoint(history, checkpoint)
    }
    fn checkpointed_head(
        &self,
        history: &RetainedHistory,
        binding: ContentHash,
    ) -> Result<Option<Root>, StoreError> {
        self.head_by_binding(history, binding)
    }
}
impl KernelAuthority {
    /// The content hash of every evidence entry a committed `AddEvidence` of `history` brought,
    /// whose payload the commit published as a Provenance object.
    ///
    /// Retained evidence is never removed, so a state this authority already reached over a
    /// prefix of `history` — replayed, or restored from a checkpoint whose head graph the
    /// head's recorded evidence root binds — names every one before it in its head graph's
    /// evidence. Only the commits after that prefix are read: each retained receipt's proposal
    /// document, parsed for its `AddEvidence` operations. A receipt or document that does not
    /// read contributes nothing here; replay refuses it by its own name.
    ///
    /// With `replaying`, the payloads that state's head graph names are left out: a replay that
    /// continues from it reads only the payloads of the commits after it. They are left out only
    /// where that replay can continue from it, which needs the graph of every revision an
    /// occurrence after it is validated or rejected against; otherwise every payload is named, as
    /// a replay from the seed reads every one.
    pub(crate) fn added_evidence_required(
        &self,
        history: &RetainedHistory,
        replaying: bool,
    ) -> Result<BTreeSet<ContentHash>, StoreError> {
        if history.occurrences.is_empty() {
            return Ok(BTreeSet::new());
        }
        let digests = self.cache()?.digests(&history.occurrences);
        let reached = self.cache()?.longest(&digests);
        let continues = |covered: usize, state: &crate::replay::ReplayState| {
            let head = state.head().root.revision;
            Self::bases(history, covered).keys().all(|basis| {
                *basis > head
                    || state
                        .revisions
                        .get(basis)
                        .is_some_and(|revision| revision.graph.is_some())
            })
        };
        let (start, mut required) = match reached
            .as_ref()
            .and_then(|(covered, state)| Some((*covered, state, state.head().graph.as_deref()?)))
        {
            Some((covered, state, _)) if replaying && continues(covered, state) => {
                (covered, BTreeSet::new())
            }
            Some((covered, _, graph)) => (
                covered,
                graph
                    .evidence
                    .values()
                    .map(|evidence| evidence.content_hash)
                    .collect(),
            ),
            None => (0, BTreeSet::new()),
        };
        for occurrence in history.occurrences.iter().skip(start) {
            if !matches!(
                occurrence.event.payload,
                RevisionPayload::RevisionCommitted { .. }
            ) {
                continue;
            }
            let Ok(bytes) = history.content(occurrence.event.record_hash, StorageClass::Canonical)
            else {
                continue;
            };
            let Ok(receipt) = crate::CommitReceiptV1::from_bytes(bytes) else {
                continue;
            };
            let Ok(document) = crate::TransactionDocument::parse(&receipt.proposal.document_bytes)
            else {
                continue;
            };
            required.extend(
                document
                    .transaction()
                    .operations
                    .iter()
                    .filter_map(|operation| match operation {
                        crate::GraphOperation::AddEvidence(addition) => {
                            Some(addition.evidence.content_hash)
                        }
                        _ => None,
                    }),
            );
        }
        Ok(required)
    }
    pub(crate) fn cache(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, crate::replay::ReplayCache>, StoreError> {
        self.cache
            .lock()
            .map_err(|_| StoreError::Document("replay-cache-poisoned".into()))
    }
    /// The complete envelope this authority holds for `seed_hash`, if it holds one.
    pub(crate) fn held_envelope(
        &self,
        seed_hash: ContentHash,
    ) -> Result<Option<Arc<SeedEnvelope>>, StoreError> {
        Ok(self
            .cache()?
            .envelope
            .as_ref()
            .filter(|(held, _)| *held == seed_hash)
            .map(|(_, envelope)| Arc::clone(envelope)))
    }
    /// The seed envelope `history` retains at `seed_hash`, decoded in full at most once by this
    /// authority. The retained bytes are read and verified on every call, exactly as before, so a
    /// history that does not hold them refuses as it did; only the decode of verified bytes at an
    /// address this authority already decoded is not repeated, because it is a function of them.
    pub(crate) fn seed_envelope(
        &self,
        history: &RetainedHistory,
        seed_hash: ContentHash,
    ) -> Result<Arc<SeedEnvelope>, StoreError> {
        let bytes = history.content(seed_hash, StorageClass::Canonical)?;
        if let Some(held) = self.held_envelope(seed_hash)? {
            return Ok(held);
        }
        let decoded = seed::envelope(bytes).map(|mut envelope| {
            // A carried payload is held as the retained object's allocation, not the decode's.
            envelope.input.share_retained(history);
            envelope
        });
        let mut cache = self.cache()?;
        cache.envelope_decodes += 1;
        let envelope = Arc::new(decoded?);
        // Only an envelope decoded from verified retained bytes is ever held.
        cache.envelope = Some((seed_hash, Arc::clone(&envelope)));
        Ok(envelope)
    }
    /// The complete seed input `envelope`, the seed envelope `history` retains at `seed_hash`,
    /// holds: assembled at most once by this authority, and shared, not copied, by every read
    /// after.
    ///
    /// It is a function of that envelope's verified bytes and of the payloads it names, each
    /// verified against its own address, so assembling it again would only copy the same bytes.
    /// Every call still requires and checks each payload `history` must hold for it, in the order
    /// and with the refusals assembling it would meet (`RetainedSeedInput::check_held`).
    pub(crate) fn seed_document(
        &self,
        history: &RetainedHistory,
        seed_hash: ContentHash,
        envelope: &SeedEnvelope,
    ) -> Result<Arc<SeedDocument>, StoreError> {
        let poisoned = || StoreError::Document("seed-input-cache-poisoned".into());
        let held = self
            .seed_input
            .lock()
            .map_err(|_| poisoned())?
            .as_ref()
            .filter(|(held, _)| *held == seed_hash)
            .map(|(_, document)| Arc::clone(document));
        if let Some(document) = held {
            envelope.input.check_held(history)?;
            return Ok(document);
        }
        let document = Arc::new(envelope.input.document(history)?);
        // Only an input assembled from verified retained bytes is ever held.
        *self.seed_input.lock().map_err(|_| poisoned())? = Some((seed_hash, Arc::clone(&document)));
        Ok(document)
    }
    /// The evidence payloads the seed envelope at `seed_hash` holds, and whether it names them
    /// rather than carrying them. The envelope is content-addressed, so its admission and its
    /// payloads are a function of `seed_hash`: admitted once, they are not admitted again by this
    /// authority.
    fn seed_payloads(
        &self,
        history: &RetainedHistory,
        seed_hash: ContentHash,
    ) -> Result<(BTreeSet<ContentHash>, bool), StoreError> {
        if let Some(known) = self
            .cache()?
            .seed
            .as_ref()
            .filter(|(admitted, _, _)| *admitted == seed_hash)
            .map(|(_, payloads, named)| (payloads.clone(), *named))
        {
            return Ok(known);
        }
        let envelope = self.seed_envelope(history, seed_hash)?;
        seed::admitted_retained(&envelope)
            .map_err(|error| StoreError::InvalidSeed(error.to_string()))?;
        let payloads = envelope.input.payload_keys();
        let named = matches!(envelope.input.payloads, seed::SeedPayloads::Named(_));
        self.cache()?.seed = Some((seed_hash, payloads.clone(), named));
        Ok((payloads, named))
    }
    pub(crate) fn seed_state(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
    ) -> Result<Option<(AdmittedRevision, BTreeSet<ContentHash>)>, StoreError> {
        self.anchor.check(self.context)?;
        let Some(first) = history.occurrences.first() else {
            return Ok(None);
        };
        let RevisionPayload::Seeded {
            revision_id,
            seed_hash,
        } = first.event.payload
        else {
            return Err(StoreError::NotSeeded);
        };
        if first.version != 1 || first.event.format != RevisionEvent::FORMAT {
            return Err(StoreError::Document("seed-occurrence-envelope".into()));
        }
        // Every replay from the seed admits the seed input in full and compares every retained
        // payload with the envelope's; only the decode of the envelope's bytes is shared.
        let envelope = self.seed_envelope(history, seed_hash)?;
        // A carried payload is admitted from the envelope's bytes, and its retained object must
        // then be exactly those bytes; a named payload is admitted from its retained object, and
        // one the store holds no object for is refused by name.
        let graph = match &envelope.input.payloads {
            seed::SeedPayloads::Carried(carried) => {
                let carried = carried
                    .iter()
                    .map(|(hash, bytes)| (*hash, bytes.as_slice()))
                    .collect();
                let graph =
                    seed::replay(&envelope, &carried, ontology, self.context, &self.anchor)?;
                envelope.payload_bytes(history)?;
                graph
            }
            seed::SeedPayloads::Named(_) => {
                let payloads = envelope.payload_bytes(history)?;
                seed::replay(&envelope, &payloads, ontology, self.context, &self.anchor)?
            }
        };
        let record = SeedResultV1::from_bytes(
            history.content(first.event.record_hash, StorageClass::Canonical)?,
        )?;
        let root = seed_root(&graph, seed_hash, &self.anchor);
        if record.event_id != first.event.event_id
            || record.revision_id != revision_id
            || record.seed_hash != seed_hash
            || record.authority_root != root.agent_root
            || record.committed_at != envelope.committed_at
            || record.result != root
            || record.result_hash != ContentHash::of(&root)
        {
            return Err(StoreError::Document("seed-result-disagrees".into()));
        }
        let result = AdmittedRevision {
            graph,
            root,
            revision_id,
            event_id: record.event_id,
            record_hash: first.event.record_hash,
            committed_at: record.committed_at,
        };
        let payloads = envelope.input.payload_keys();
        Ok(Some((result, payloads)))
    }
}
/// The Bootstrap slot's input hash binds the host context and anchor, so a preparation that the
/// store can only read as `AuthorityMismatch` was elected for different input to this slot
/// (§ 94.1, § 94.3), not a changed seed. Every read of that slot goes through here.
fn bootstrap_slot<T>(result: Result<T, StoreError>) -> Result<T, StoreError> {
    match result {
        Err(StoreError::AuthorityMismatch) => Err(StoreError::PublicationInputConflict),
        other => other,
    }
}
fn seed_root(graph: &CanonicalGraph, seed_hash: ContentHash, anchor: &AuthorityStateV1) -> Root {
    Root {
        revision: RevisionNumber::SEED,
        parent: None,
        ontology_root: ContentHash::of(&graph.ontology),
        knowledge_root: knowledge_root(graph),
        evidence_root: evidence_root(graph),
        agent_root: ContentHash::of(anchor),
        transaction: seed_hash,
    }
}

/// Durable command refusals; missing records are never fabricated transaction states.
#[derive(Debug, thiserror::Error)]
pub enum CommitError {
    /// Bounded proposal input failed before a proposal could be recorded.
    #[error(transparent)]
    Document(#[from] crate::DocumentError),
    /// The trusted submitter is unregistered or differs from the document's attribution.
    #[error("proposal attribution does not match registered submitter {actor}")]
    ProposalAttribution {
        /// Independently supplied host identity.
        actor: ekr_core::AgentId,
    },
    /// The requested revision has no retained committed basis.
    #[error("revision {against} does not exist")]
    RevisionNotFound {
        /// Requested canonical revision.
        against: RevisionNumber,
    },
    /// A retained transaction is in a state that cannot perform this command.
    #[error("transaction {transaction_id} is {state:?}")]
    TransactionStateConflict {
        /// Requested transaction identity.
        transaction_id: TransactionId,
        /// Its actual retained state.
        state: crate::replay::TransactionState,
    },
    /// Storage or retained-history verification refused.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// No seed exists.
    #[error("the lineage has no seed")]
    NotSeeded,
    /// The well-formed transaction identity has no retained proposal.
    #[error("transaction {transaction_id} does not exist")]
    TransactionNotFound {
        /// The requested identity.
        transaction_id: TransactionId,
    },
}

/// The shared synchronous command handler. It does not expose the raw store or a writer.
pub struct Commit<S: RevisionLog + ObjectStore> {
    pub(crate) store: S,
    pub(crate) authority: KernelAuthority,
}
impl<S: RevisionLog + ObjectStore> Commit<S> {
    /// Opens under an explicit host identity registry and exact validation profile.
    /// # Errors
    /// Invalid host anchor or provider opening failure.
    pub fn over_with_authority(
        context: BootstrapContext,
        anchor: AuthorityStateV1,
        open: impl FnOnce(KernelAuthority) -> Result<S, StoreError>,
    ) -> Result<Self, StoreError> {
        anchor.check(context)?;
        let authority = KernelAuthority {
            context,
            anchor,
            cache: std::sync::Arc::default(),
            seed_input: std::sync::Arc::default(),
        };
        let store = open(authority.clone())?;
        Ok(Self { store, authority })
    }
    /// The complete verified current root, if initialized.
    /// # Errors
    /// Any required retained record fails verification.
    pub fn head(&self) -> Result<Option<Root>, StoreError> {
        self.store.head()
    }
    /// Current canonical state reconstructed through this kernel's authority.
    /// # Errors
    /// Missing seed or invalid retained history.
    pub fn snapshot(&self) -> Result<CanonicalGraph, StoreError> {
        self.store.fold()
    }
    /// Reconstructs a selected committed revision.
    /// # Errors
    /// A missing revision or invalid required history.
    pub fn replay(&self, revision: RevisionNumber) -> Result<CanonicalGraph, StoreError> {
        self.store.replay(revision)
    }
    /// Verified retained payload bytes after validating the lineage that refers to them.
    /// # Errors
    /// Invalid retained history or corrupt native content.
    pub fn content(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        self.store.history()?;
        self.store.get(hash)
    }
    fn retained_seed(&self, document: &SeedDocument) -> Result<Option<SeedResultV1>, SeedError> {
        let history = match self.store.history() {
            Err(StoreError::AuthorityMismatch) => return Err(StoreError::AlreadySeeded.into()),
            result => result?,
        };
        let Some(first) = history.occurrences.first() else {
            return Ok(None);
        };
        let RevisionPayload::Seeded { seed_hash, .. } = first.event.payload else {
            return Err(StoreError::NotSeeded.into());
        };
        let envelope = self.authority.seed_envelope(&history, seed_hash)?;
        if !envelope.input.holds(document)
            || envelope.context != self.authority.context
            || envelope.authority != self.authority.anchor
        {
            return Err(StoreError::AlreadySeeded.into());
        }
        Ok(Some(SeedResultV1::from_bytes(history.content(
            first.event.record_hash,
            StorageClass::Canonical,
        )?)?))
    }
}
impl<S: RevisionLog + ObjectStore + Initialize> Commit<S> {
    /// Validates and publishes a complete seed only if the lineage has none: `Ok` means this call
    /// wrote the seed. A seed already published or elected — the identical document included —
    /// refuses as `AlreadySeeded` without sampling the clock, and so does losing the store's
    /// election for the Bootstrap slot to another caller. That election is the store's own
    /// conditional append, so the decision is made inside the write, not by an earlier read. An
    /// elected seed still unpublished is published before the refusal, as its own retry would.
    /// # Errors
    /// Invalid input/anchor, any existing or concurrently elected seed or failed atomic
    /// publication.
    pub fn seed_if_absent(
        &self,
        document: SeedDocument,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<SeedResultV1, SeedError> {
        // Every Bootstrap slot conflict is a seed another caller elected, so a seed exists.
        self.seed_with(document, now, true)
            .map_err(|error| match error {
                SeedError::Store(StoreError::PublicationInputConflict) => {
                    StoreError::AlreadySeeded.into()
                }
                other => other,
            })
    }
    /// Validates and publishes a complete seed, or returns its exact original retained result.
    /// The host clock is invoked only after own-result lookup, never during a retry or replay.
    /// # Errors
    /// Invalid input/anchor, an existing different seed or failed atomic publication.
    pub fn seed(
        &self,
        document: SeedDocument,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<SeedResultV1, SeedError> {
        self.seed_with(document, now, false)
    }
    fn seed_with(
        &self,
        document: SeedDocument,
        now: impl FnOnce() -> Timestamp,
        if_absent: bool,
    ) -> Result<SeedResultV1, SeedError> {
        if let Some(result) = self.retained_seed(&document)? {
            if if_absent {
                return Err(StoreError::AlreadySeeded.into());
            }
            return Ok(result);
        }
        let key = ekr_store::PublicationCommandKey {
            kind: ekr_store::PublicationCommandKind::Bootstrap,
            transaction_id: None,
            predecessor_event_id: None,
            predecessor_record_hash: None,
        };
        let material =
            serde_json::to_vec(&document).map_err(|error| SeedError::Invalid(error.to_string()))?;
        let input = crate::commands::input_hash(
            "Seed",
            &material,
            self.authority.context.operator,
            &self.authority,
        );
        if let Some(pending) = bootstrap_slot(self.store.preparation(&key))? {
            // An elected seed nobody has published yet is a seed: another caller's election, or
            // an earlier call of this one that did not learn its answer. Neither is this call's.
            if if_absent {
                return self.refuse_after_publishing(pending, &document);
            }
            if pending.input_hash != input {
                return Err(StoreError::PublicationInputConflict.into());
            }
            return self.finish_seed(pending, &document);
        }
        let committed_at = now();
        let graph = seed::admitted_graph(&document, self.authority.context, committed_at)?;
        // `ekr-seed-envelope/3` (design § 100.1): the input with its evidence payloads named. The
        // payloads are retained once, as the Provenance objects published beside it.
        let envelope = SeedEnvelope {
            format: seed::ENVELOPE_FORMAT.into(),
            input: seed::RetainedSeedInput::naming(&document),
            context: self.authority.context,
            authority: self.authority.anchor.clone(),
            committed_at,
            migration: None,
        };
        let bytes = envelope.to_bytes()?;
        let seed_hash = ContentHash::of_bytes(&bytes);
        // Nothing is held for `seed_hash` here: the replay that admits the publication decodes
        // the staged bytes themselves (invariant 1), and only that decode is then held.
        let root = seed_root(&graph, seed_hash, &self.authority.anchor);
        let record = SeedResultV1 {
            format: SeedResultV1::FORMAT.into(),
            event_id: EventId::mint(),
            revision_id: RevisionId::mint(),
            seed_hash,
            authority_root: root.agent_root,
            committed_at,
            result: root,
            result_hash: ContentHash::of(&root),
        };
        let record_bytes = record.to_bytes()?;
        let record_hash = ContentHash::of_bytes(&record_bytes);
        let mut objects = std::collections::BTreeMap::new();
        objects.insert(
            seed_hash,
            PublicationObject {
                storage_class: StorageClass::Canonical,
                stored_at: committed_at,
                bytes,
            },
        );
        objects.insert(
            record_hash,
            PublicationObject {
                storage_class: StorageClass::Canonical,
                stored_at: committed_at,
                bytes: record_bytes,
            },
        );
        for (hash, payload) in &document.evidence_payloads {
            objects.entry(*hash).or_insert_with(|| PublicationObject {
                storage_class: StorageClass::Provenance,
                stored_at: committed_at,
                bytes: payload.to_vec(),
            });
        }
        let publication = Publication {
            event: RevisionEvent {
                format: RevisionEvent::FORMAT.into(),
                event_id: record.event_id,
                record_hash,
                payload: RevisionPayload::Seeded {
                    revision_id: record.revision_id,
                    seed_hash,
                },
            },
            objects,
            expected_version: 0,
        };
        let own = record.event_id;
        // The store elects one preparation for the Bootstrap slot by a conditional append; a
        // loser is handed the winner's, or refused when the winner's input differs.
        let selected = match bootstrap_slot(self.store.prepare(&key, input, &publication, None)) {
            Err(StoreError::PublicationInputConflict) if if_absent => {
                return match bootstrap_slot(self.store.preparation(&key))? {
                    Some(winner) => self.refuse_after_publishing(winner, &document),
                    None => Err(StoreError::AlreadySeeded.into()),
                };
            }
            selected => selected?,
        };
        // A winner with another occurrence is another call's seed, and that holds for a
        // byte-identical document too, since every call mints its own event.
        if if_absent && selected.decision.event.event_id != own {
            return self.refuse_after_publishing(selected, &document);
        }
        let result = self.finish_seed(selected, &document)?;
        // Publication can still find a seed that never went through this slot (the revision
        // stream's own conditional append refuses it), which `finish_seed` answers as retained.
        if if_absent && result.event_id != own {
            return Err(StoreError::AlreadySeeded.into());
        }
        Ok(result)
    }
    /// The if-absent refusal by another call's election. The elected seed is published first,
    /// exactly as its own retry would publish it, so the refusal's claim holds when it is made:
    /// the lineage is seeded, and a caller that elected and stopped before publishing cannot leave
    /// a lineage every if-absent call refuses and none seeds.
    fn refuse_after_publishing(
        &self,
        elected: ekr_store::PublicationPreparationV1,
        document: &SeedDocument,
    ) -> Result<SeedResultV1, SeedError> {
        self.finish_seed(elected, document)?;
        Err(StoreError::AlreadySeeded.into())
    }
    fn finish_seed(
        &self,
        mut selected: ekr_store::PublicationPreparationV1,
        document: &SeedDocument,
    ) -> Result<SeedResultV1, SeedError> {
        for _ in 0..16 {
            match bootstrap_slot(self.store.resume(&selected)) {
                Ok(_) => {
                    let bytes = selected
                        .decision
                        .objects
                        .get(&selected.decision.event.record_hash)
                        .ok_or_else(|| {
                            StoreError::Document("elected-seed-record-missing".into())
                        })?;
                    let result = SeedResultV1::from_bytes(&bytes.bytes)?;
                    self.retain_checkpoint();
                    return Ok(result);
                }
                Err(StoreError::Conflict) => {
                    if let Some(result) = self.retained_seed(document)? {
                        return Ok(result);
                    }
                    selected = bootstrap_slot(self.store.prepare(
                        &selected.command_key,
                        selected.input_hash,
                        &selected.decision,
                        Some(&selected),
                    ))?;
                }
                Err(error) => return Err(error.into()),
            }
        }
        Err(StoreError::Conflict.into())
    }
}
