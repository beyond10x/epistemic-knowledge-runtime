//! Kernel-owned immutable read captures; projection consumers receive no storage authority.
use crate::replay::AliasCell;
use crate::{
    AuthorityStateV1, BootstrapContext, Commit, CommitError, SeedDocument, SeedResultV1,
    TransactionRecord,
};
use ekr_core::{ContentHash, EventId, RevisionId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{AliasIndex, CanonicalGraph, Root};
use ekr_ontology::Ontology;
use ekr_store::{ObjectStore, RevisionLog};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Decoding work the kernel did on the calling thread since [`read_cost`] last reported.
///
/// Test instrumentation, not part of the kernel's contract, as `ekr_store::ReadWork` is: it lets a
/// case count what one read decodes, which no clock under load can (`AGENTS.md`). Counted per
/// thread because every store and kernel call runs on its caller's thread.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadCost {
    /// Retained decision records decoded: seed results, proposals, validations, rejections,
    /// commit receipts and stale records.
    pub records_decoded: u64,
    /// Transaction documents parsed from a retained proposal's bytes.
    pub documents_parsed: u64,
    /// Replay checkpoints decoded, each with its head graph.
    pub checkpoints_decoded: u64,
    /// Reads answered from the persisted read model (design § 108) rather than a complete read.
    pub models_served: u64,
}

thread_local! {
    static COST: std::cell::Cell<ReadCost> = const { std::cell::Cell::new(ReadCost {
        records_decoded: 0,
        documents_parsed: 0,
        checkpoints_decoded: 0,
        models_served: 0,
    }) };
}

/// The decoding work counted on this thread since the last call, which starts the count again.
#[doc(hidden)]
#[must_use]
pub fn read_cost() -> ReadCost {
    COST.with(|cost| cost.replace(ReadCost::default()))
}

pub(crate) fn count(add: impl FnOnce(&mut ReadCost)) {
    COST.with(|cost| {
        let mut now = cost.get();
        add(&mut now);
        cost.set(now);
    });
}

/// Complete verified coordinates of one retained canonical revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedRevision {
    /// Domain revision identity.
    pub revision_id: RevisionId,
    /// Immutable occurrence identity.
    pub event_id: EventId,
    /// Payload address of the actual seed result or commit receipt.
    pub record_hash: ContentHash,
    /// Trusted original publication time.
    pub committed_at: Timestamp,
    /// Recomputed complete root.
    pub root: Root,
}
/// One verified read boundary, with enough retained input for explanation without another read.
/// Only the kernel constructs it; owning the capture grants no persistence authority.
///
/// The graph and the transaction snapshot are shared by reads of the same verified state:
/// a repeated read of an unchanged head costs no copy of either. Changing one through
/// [`Arc::make_mut`] changes this capture's copy, never the verified state.
pub struct VerifiedRead {
    /// Admitted graph at the chosen boundary.
    pub graph: Arc<CanonicalGraph>,
    /// Recomputed root for that graph.
    pub root: Root,
    /// Actual original seed result, even after later head advancement.
    pub seed: SeedResultV1,
    /// Original admitted seed input, including complete ontology and evidence declarations,
    /// shared with every other read of the same runtime rather than copied into each.
    pub seed_input: Arc<SeedDocument>,
    /// Actual retained bootstrap identities checked against the host at this same boundary.
    pub context: BootstrapContext,
    /// Complete original registry and validation profile, verified against the host anchor.
    pub authority: AuthorityStateV1,
    /// All canonical revision coordinates through this boundary.
    pub revisions: BTreeMap<RevisionNumber, VerifiedRevision>,
    /// Actual retained transaction decisions through this boundary.
    pub transactions: Arc<BTreeMap<TransactionId, TransactionRecord>>,
    objects: BTreeMap<ContentHash, Arc<Vec<u8>>>,
    /// The verified graph as the kernel admitted it, and the cell its [`AliasIndex`] is kept in.
    /// Holding the graph here keeps [`Arc::make_mut`] on [`Self::graph`] from changing it in place.
    indexed: (Arc<CanonicalGraph>, AliasCell),
}
impl VerifiedRead {
    /// Already verified retained bytes, with no provider access or new history observation.
    #[must_use]
    pub fn content(&self, hash: &ContentHash) -> Option<&[u8]> {
        self.objects.get(hash).map(|bytes| bytes.as_slice())
    }
    /// The [`AliasIndex`] of [`Self::graph`]. For the graph the kernel admitted it is built once
    /// per revision and shared by every read of that revision; for a graph this capture's owner
    /// replaced or changed it is built from that graph.
    #[must_use]
    pub fn aliases(&self) -> Cow<'_, AliasIndex> {
        match self.admitted_aliases() {
            Some(index) => Cow::Borrowed(index),
            None => Cow::Owned(AliasIndex::of(&self.graph)),
        }
    }
    /// [`Self::aliases`] as a shared handle, which a caller may keep beyond this capture.
    #[must_use]
    pub fn alias_index(&self) -> Arc<AliasIndex> {
        self.admitted_aliases()
            .map_or_else(|| Arc::new(AliasIndex::of(&self.graph)), Arc::clone)
    }
    /// The index of the graph the kernel admitted, built on first use, while [`Self::graph`] is
    /// still that graph.
    fn admitted_aliases(&self) -> Option<&Arc<AliasIndex>> {
        let (admitted, index) = &self.indexed;
        Arc::ptr_eq(&self.graph, admitted)
            .then(|| index.get_or_init(|| Arc::new(AliasIndex::of(admitted))))
    }
}
/// One revision's canonical state with the schema history of its lineage, from one verified
/// replay: what a view of that revision reads, without a replay per schema version.
/// Only the kernel constructs it; owning it grants no persistence authority.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SchemaHistory {
    /// Admitted graph at the chosen revision: the verified state's own when it holds that
    /// revision's graph, shared rather than copied.
    pub graph: Arc<CanonicalGraph>,
    /// Coordinates of every revision from the seed through the chosen one.
    pub revisions: BTreeMap<RevisionNumber, VerifiedRevision>,
    /// The transaction that committed each of those revisions; the seed has none.
    pub transactions: BTreeMap<RevisionNumber, TransactionId>,
    /// The ontology in force from each of those revisions whose ontology root differs from the
    /// revision before it: the seed's, and one more for each revision that changed the schema.
    pub schemas: BTreeMap<RevisionNumber, Ontology>,
    /// The content hashes, among the chosen revision's evidence, whose bytes the same verified
    /// history holds as objects: answered from that one read, not one history read per entry.
    pub retained_evidence: BTreeSet<ContentHash>,
}
impl<S: RevisionLog + ObjectStore> Commit<S> {
    /// The graph at `revision` and the schema history of its lineage.
    ///
    /// One verified replay of the current history, continued from whatever this authority or the
    /// store's replay checkpoint already reached, holds every revision's coordinates, committing
    /// transaction and ontology. The graph at `revision` comes from that same replay when it holds
    /// it; only when it does not (a state continued from a checkpoint holds only its head's graph)
    /// is the history replayed once more, up to `revision`.
    /// # Errors
    /// [`CommitError::NotSeeded`], [`CommitError::RevisionNotFound`] beyond the head, or any
    /// required history/object verification failure.
    pub fn schema_history(&self, revision: RevisionNumber) -> Result<SchemaHistory, CommitError> {
        // The replay that verifies the history keeps the chosen revision's graph where it passes
        // it, so that a replay from the seed is not followed by another one to that revision.
        // Dropping the hint releases that graph from the cached states again; `state`, this
        // read's own reference, still holds it.
        let keeping = self.authority.keeping(revision);
        let history = self.store.history()?;
        let state = self
            .authority
            .reconstruct(&history, None, None)?
            .ok_or(CommitError::NotSeeded)?;
        drop(keeping);
        let chosen = state
            .revisions
            .get(&revision)
            .ok_or(CommitError::RevisionNotFound { against: revision })?;
        let graph = self.authority.graph_at(&history, &state, revision)?;
        if graph.revision != revision
            || ContentHash::of(&graph.ontology) != chosen.root.ontology_root
        {
            return Err(ekr_store::StoreError::Document("schema-history-disagrees".into()).into());
        }
        let mut revisions = BTreeMap::new();
        let mut schemas = BTreeMap::new();
        let mut in_force = None;
        for (number, held) in state.revisions.range(..=revision) {
            if in_force != Some(held.root.ontology_root) {
                schemas.insert(*number, Ontology::clone(&held.ontology));
                in_force = Some(held.root.ontology_root);
            }
            revisions.insert(
                *number,
                VerifiedRevision {
                    revision_id: held.revision_id,
                    event_id: held.event_id,
                    record_hash: held.record_hash,
                    committed_at: held.committed_at,
                    root: held.root,
                },
            );
        }
        let transactions = state
            .transactions
            .iter()
            .filter_map(|(id, record)| {
                record
                    .committed
                    .as_ref()
                    .map(|receipt| (receipt.result.revision, *id))
            })
            .filter(|(number, _)| *number <= revision)
            .collect();
        // The kernel requires every evidence payload of the lineage (seeded or added), so the
        // history just verified holds each one it retains, already checked against its address.
        let retained_evidence = graph
            .evidence
            .values()
            .map(|evidence| evidence.content_hash)
            .filter(|hash| history.objects.contains_key(hash))
            .collect();
        Ok(SchemaHistory {
            graph,
            revisions,
            transactions,
            schemas,
            retained_evidence,
        })
    }
    /// How many replays this handle's authority has begun at the seed.
    #[must_use]
    pub fn seed_replays(&self) -> u64 {
        self.authority
            .cache
            .lock()
            .map_or(0, |cache| cache.seed_replays)
    }
    /// How many times this handle's authority has decoded the retained seed envelope in full.
    /// A diagnostic of read cost; it changes nothing.
    #[must_use]
    pub fn seed_envelope_decodes(&self) -> u64 {
        self.authority
            .cache
            .lock()
            .map_or(0, |cache| cache.envelope_decodes)
    }
    /// Captures one verified current or historical graph together with its actual retained input.
    /// # Errors
    /// Missing seed/revision or any required history/object verification failure.
    pub fn read(&self, revision: Option<RevisionNumber>) -> Result<VerifiedRead, CommitError> {
        let history = match revision {
            Some(revision) => self
                .store
                .history_at(revision)
                .map_err(|error| match error {
                    ekr_store::StoreError::NoMaterialisedState { requested } => {
                        CommitError::RevisionNotFound { against: requested }
                    }
                    other => other.into(),
                })?,
            None => self.store.history()?,
        };
        let state = self
            .authority
            .reconstruct(&history, None, revision)?
            .ok_or(CommitError::NotSeeded)?;
        let envelope = self
            .authority
            .seed_envelope(&history, state.seed.seed_hash)?;
        let seed_input = self
            .authority
            .seed_document(&history, state.seed.seed_hash, &envelope)?;
        let head = state.head();
        // Only the head read shares the one kept cell; a historical read indexes its own graph.
        let aliases = match (revision, self.authority.cache.lock()) {
            (None, Ok(mut cache)) => cache.alias_cell(head.revision_id, head.root),
            _ => AliasCell::default(),
        };
        let graph = Arc::clone(
            head.graph
                .as_ref()
                .ok_or_else(|| crate::replay::refuse(crate::replay::GRAPH_NOT_HELD))?,
        );
        Ok(VerifiedRead {
            graph: Arc::clone(&graph),
            root: head.root,
            seed: state.seed.clone(),
            seed_input,
            context: envelope.context,
            authority: envelope.authority.clone(),
            transactions: state.transaction_records(),
            revisions: state
                .revisions
                .iter()
                .map(|(number, state)| {
                    (
                        *number,
                        VerifiedRevision {
                            revision_id: state.revision_id,
                            event_id: state.event_id,
                            record_hash: state.record_hash,
                            committed_at: state.committed_at,
                            root: state.root,
                        },
                    )
                })
                .collect(),
            objects: history
                .objects
                .into_iter()
                .map(|(hash, object)| (hash, object.bytes))
                .collect(),
            indexed: (graph, aliases),
        })
    }
}

/// A verified read of one revision's graph, what `ekr snapshot` prints: served from the persisted
/// read model where the kernel verifies it (design § 108), and otherwise a complete
/// [`VerifiedRead`] of that revision. Either way the graph and root are the ones a complete read
/// answers. Only the kernel constructs it; owning it grants no persistence authority.
pub struct GraphRead {
    /// The admitted graph at the read revision.
    pub graph: Arc<CanonicalGraph>,
    /// That revision's complete root.
    pub root: Root,
    source: GraphSource,
}
enum GraphSource {
    /// Served from the read model: the revision's identity, from its retained record.
    Model { revision_id: RevisionId },
    /// A complete verified read.
    Read(Box<VerifiedRead>),
}
impl GraphRead {
    /// `ekr.kernel.Snapshot` of this read, byte for byte what [`VerifiedRead::snapshot`] of the
    /// same revision answers.
    /// # Errors
    /// [`crate::ProjectionError::Unverified`] where a complete read's capture is not bound to its
    /// retained root and seed; a read served from the model was held to them when it was served.
    pub fn snapshot(
        &self,
        valid_at: Option<Timestamp>,
    ) -> Result<crate::SnapshotResult, crate::ProjectionError> {
        match &self.source {
            GraphSource::Model { revision_id } => Ok(crate::SnapshotResult::of(
                *revision_id,
                self.root,
                &self.graph,
                valid_at,
            )),
            GraphSource::Read(read) => read.snapshot(valid_at),
        }
    }
    /// Whether the persisted read model answered this read.
    #[must_use]
    pub fn served_from_model(&self) -> bool {
        matches!(self.source, GraphSource::Model { .. })
    }
}

/// A verified read of the schema in force at one revision, what `ekr ontology` prints: served
/// from the persisted read model where the kernel verifies it (design § 108), and otherwise from a
/// complete [`VerifiedRead`] of that revision. Only the kernel constructs it.
pub struct OntologyRead {
    /// The read revision's complete root; its `ontology_root` is [`Self::ontology`]'s address.
    pub root: Root,
    /// The schema in force at that revision.
    pub ontology: Arc<Ontology>,
    from_model: bool,
}
impl OntologyRead {
    /// Whether the persisted read model answered this read.
    #[must_use]
    pub fn served_from_model(&self) -> bool {
        self.from_model
    }
}

impl<S: RevisionLog + ObjectStore> Commit<S> {
    /// The graph and root of `revision`, or of the head for `None`: from the persisted read model
    /// when the kernel verifies it for that revision (design § 108), which replays nothing and
    /// decodes only the revision's record, the seed envelope and the checkpoint, and loads and
    /// verifies every evidence payload a complete read requires; otherwise
    /// [`Self::read`]'s, with its answer and its refusals.
    /// # Errors
    /// Whatever [`Self::read`] refuses.
    pub fn read_graph(&self, revision: Option<RevisionNumber>) -> Result<GraphRead, CommitError> {
        if let Some(crate::checkpoint::Served {
            root,
            revision_id,
            graph: Some(graph),
            ..
        }) = self.served(revision, true)
        {
            return Ok(GraphRead {
                graph,
                root,
                source: GraphSource::Model { revision_id },
            });
        }
        let read = self.read(revision)?;
        Ok(GraphRead {
            graph: Arc::clone(&read.graph),
            root: read.root,
            source: GraphSource::Read(Box::new(read)),
        })
    }
    /// The schema in force at `revision`, or at the head for `None`, with that revision's root:
    /// from the persisted read model when the kernel verifies it for that revision (design § 108),
    /// which replays nothing, decodes only the revision's record, the seed envelope and the
    /// checkpoint (of its graph, the evidence entries), and loads and verifies every evidence
    /// payload a complete read requires; otherwise
    /// [`Self::read`]'s, with its answer and its refusals.
    /// # Errors
    /// Whatever [`Self::read`] refuses.
    pub fn read_ontology(
        &self,
        revision: Option<RevisionNumber>,
    ) -> Result<OntologyRead, CommitError> {
        if let Some(served) = self.served(revision, false) {
            return Ok(OntologyRead {
                root: served.root,
                ontology: served.ontology,
                from_model: true,
            });
        }
        let read = self.read(revision)?;
        Ok(OntologyRead {
            root: read.root,
            ontology: Arc::new(read.graph.ontology.clone()),
            from_model: false,
        })
    }
    /// `revision` served from the persisted read model, with its graph when `graph` asks for it,
    /// or `None` wherever the store has none or the kernel refuses it: never an error, because
    /// the complete read that answers instead says what is wrong, if anything is.
    ///
    /// Only the first read of a handle that has reached no replay state is served from the model:
    /// a one-shot command. A long-lived handle (a session) answers its later reads from the state a
    /// complete read reached and keeps, which costs it nothing again, where the model would be
    /// decoded anew for every read.
    fn served(
        &self,
        revision: Option<RevisionNumber>,
        graph: bool,
    ) -> Option<crate::checkpoint::Served> {
        {
            let cache = self.authority.cache.lock().ok()?;
            if cache.model_served || cache.newest().is_some() {
                return None;
            }
        }
        let (mut history, model) = self
            .store
            .read_model(&|occurrences| crate::KernelAuthority::model_objects(occurrences, revision))
            .ok()??;
        let served = self
            .authority
            .serve_model(&history, &model, revision, graph)
            .ok()?;
        // Every evidence payload a complete read of the revision requires, which invariant 2
        // rests canonical knowledge on, is loaded and checked as the complete read checks it
        // (design § 108.2 item 7). One that is absent or damaged is never answered over: the
        // complete read answers instead and refuses the store by its own name.
        self.store
            .load_required(&mut history, served.payloads.clone())
            .ok()?;
        for payload in &served.payloads {
            history
                .content(*payload, ekr_store::StorageClass::Provenance)
                .ok()?;
        }
        if let Ok(mut cache) = self.authority.cache.lock() {
            cache.model_served = true;
        }
        count(|cost| cost.models_served += 1);
        Some(served)
    }
}
