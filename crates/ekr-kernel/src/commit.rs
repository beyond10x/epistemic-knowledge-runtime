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

/// The immutable trusted anchor installed by the kernel; retained records never replace it.
#[derive(Clone)]
pub struct KernelAuthority {
    pub(crate) context: BootstrapContext,
    pub(crate) anchor: AuthorityStateV1,
    /// Verified replay states this authority reached, shared by every clone of it.
    pub(crate) cache: std::sync::Arc<std::sync::Mutex<crate::replay::ReplayCache>>,
}
impl CommitAuthority for KernelAuthority {
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
        // The seed envelope is content-addressed, so its admission and the payloads it names are
        // a function of `seed_hash`: admitted once, they are not admitted again by this authority.
        if let Some(required) = self.seed_requirements(seed_hash)? {
            return Ok(required);
        }
        let envelope = seed::envelope(history.content(seed_hash, StorageClass::Canonical)?)?;
        seed::admitted_graph(&envelope.input, envelope.context, envelope.committed_at)
            .map_err(|error| StoreError::InvalidSeed(error.to_string()))?;
        let required: BTreeSet<ContentHash> =
            envelope.input.evidence_payloads.keys().copied().collect();
        self.cache
            .lock()
            .map_err(|_| StoreError::Document("replay-cache-poisoned".into()))?
            .seed = Some((seed_hash, required.clone()));
        Ok(required)
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
    fn seed_requirements(
        &self,
        seed_hash: ContentHash,
    ) -> Result<Option<BTreeSet<ContentHash>>, StoreError> {
        Ok(self
            .cache
            .lock()
            .map_err(|_| StoreError::Document("replay-cache-poisoned".into()))?
            .seed
            .as_ref()
            .filter(|(admitted, _)| *admitted == seed_hash)
            .map(|(_, required)| required.clone()))
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
        let bytes = history.content(seed_hash, StorageClass::Canonical)?;
        let envelope = seed::envelope(bytes)?;
        let graph = seed::replay(bytes, ontology, self.context, &self.anchor)?;
        for (hash, original) in &envelope.input.evidence_payloads {
            if history.content(*hash, StorageClass::Provenance)? != original {
                return Err(StoreError::InvalidSeed(
                    "seed-evidence-payload-mismatch".into(),
                ));
            }
        }
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
        let payloads = envelope.input.evidence_payloads.keys().copied().collect();
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
        let envelope = seed::envelope(history.content(seed_hash, StorageClass::Canonical)?)?;
        if envelope.input != *document
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
    /// Validates and publishes a complete seed, or returns its exact original retained result.
    /// The host clock is invoked only after own-result lookup, never during a retry or replay.
    /// # Errors
    /// Invalid input/anchor, an existing different seed or failed atomic publication.
    pub fn seed(
        &self,
        document: SeedDocument,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<SeedResultV1, SeedError> {
        if let Some(result) = self.retained_seed(&document)? {
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
            if pending.input_hash != input {
                return Err(StoreError::PublicationInputConflict.into());
            }
            return self.finish_seed(pending, &document);
        }
        let committed_at = now();
        let graph = seed::admitted_graph(&document, self.authority.context, committed_at)?;
        let envelope = SeedEnvelope {
            format: "ekr-seed-envelope/2".into(),
            input: document.clone(),
            context: self.authority.context,
            authority: self.authority.anchor.clone(),
            committed_at,
        };
        let bytes = serde_json::to_vec(&envelope).map_err(|e| SeedError::Invalid(e.to_string()))?;
        let seed_hash = ContentHash::of_bytes(&bytes);
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
        for (hash, payload) in document.evidence_payloads {
            objects.entry(hash).or_insert(PublicationObject {
                storage_class: StorageClass::Provenance,
                stored_at: committed_at,
                bytes: payload,
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
        let selected = bootstrap_slot(self.store.prepare(&key, input, &publication, None))?;
        self.finish_seed(selected, &envelope.input)
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
