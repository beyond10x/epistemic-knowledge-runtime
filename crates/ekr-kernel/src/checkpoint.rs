//! Replay checkpoints: a verified head state, written down so that the next open of a store does
//! not replay the whole lineage again. `ekr.replay-checkpoint/1`, design § 96.
//!
//! # What a checkpoint is
//!
//! The state one complete kernel replay reached at the end of a revision-stream prefix, reduced
//! to what cannot be read back cheaply from the retained records themselves: the head revision's
//! graph and the schema version in force at each revision, and under validation profile v3 every
//! node and edge identity a revision of the lineage held, which profile v3 holds a new record
//! against (`task:deleted-edge-id-is-reusable`). Everything else — the seed result,
//! every transaction's records, every revision's coordinates — is decoded again from the verified
//! record bytes the history already loads.
//!
//! # What admitting one checks, and what it takes on trust
//!
//! A checkpoint is admitted only for the exact prefix it names — the chain digest over every
//! occurrence's position and event, which binds every record address — and only by an authority
//! with the same host context and anchor. Its head graph must reproduce the knowledge and
//! evidence roots the head's retained receipt records, and each schema version must reproduce
//! the ontology root of every revision it is in force at. What no revision root binds — the
//! head graph's root and the evidence payloads the seed requires — must equal what the seed
//! envelope the prefix binds says, so that a changed cache can neither become the root the
//! kernel's next decisions record nor drop bytes a verified read holds. Nor does a revision root
//! bind the identities a v3 lineage held, so they must equal what the seed envelope and the
//! retained proposal of every commit the prefix binds say were created, so that a changed cache
//! cannot free a deleted edge's id for a different record. What it takes on trust
//! is that the prefix was replayed and every retained decision re-derived when the checkpoint
//! was written: a checkpoint records that verification, it does not repeat it. A store opened
//! for full replay ignores checkpoints and repeats it.
//!
//! A checkpoint that fails any check is not an error of the store. It is ignored, and the
//! history is replayed in full as if it were absent.
//!
//! # When one is written
//!
//! A seed writes one. A commit writes one when it is due (design § 99): when the authority knows
//! of no retained checkpoint, or the head is [`REPLAY_CHECKPOINT_COMMITS`] revisions past the
//! retained one's, or the commits since it hold [`REPLAY_CHECKPOINT_OPERATIONS`] operations, or
//! a validation since it was made against a revision before its head. Any other commit appends only a pointer that names the retained checkpoint with the new
//! coverage. A proposal, a validation or a stale decision appends nothing: an open replays what
//! follows the checkpoint, and it answers the same wherever the checkpoint lies.
use crate::replay::{prefix_digests, refuse, require, ReplayState, Revision};
use crate::seed::{narrow_assertion, narrow_edge, narrow_node, SeedOutline};
use crate::validate::HeldIdentities;
use crate::{
    CommitReceiptV1, KernelAuthority, ProposalRecordV1, RejectionRecordV1, SeedResultV1,
    StaleRecordV1, TransactionRecord, ValidationReceiptV1,
};
use ekr_core::{Canonical, ContentHash, EdgeId, Encoder, NodeId, RevisionNumber};
use ekr_graph::{CanonicalGraph, RevisionPayload};
use ekr_ontology::{Ontology, OntologyDocument};
use ekr_store::{
    evidence_root, knowledge_root, GraphDocument, RetainedHistory, StorageClass, StoreError,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// The checkpoint format this kernel writes and reads.
pub(crate) const FORMAT: &str = "ekr.replay-checkpoint/1";

/// A commit writes a replay checkpoint when its revision is at least this many revisions past
/// the head of the retained checkpoint this authority knows of (design § 99). Between two, a
/// commit appends only a pointer, and an open replays at most this many commits less one after
/// the checkpoint it restores.
pub const REPLAY_CHECKPOINT_COMMITS: u64 = 4;

/// A commit also writes a replay checkpoint when the transactions committed after the retained
/// checkpoint this authority knows of hold at least this many operations together, so that a few
/// large commits are not replayed by every open (design § 99).
pub const REPLAY_CHECKPOINT_OPERATIONS: u64 = 512;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointV1 {
    format: String,
    /// The host context and anchor the replay ran under.
    authority: ContentHash,
    /// How many revision-stream occurrences the state covers.
    covered: u64,
    /// The prefix digest of those occurrences.
    prefix: ContentHash,
    /// The head revision at the end of that prefix.
    revision: RevisionNumber,
    /// The head revision's graph.
    graph: GraphDocument,
    /// Each schema version, from the revision it came into force at.
    ontologies: Vec<OntologyAt>,
    /// The evidence payloads the admitted seed envelope requires.
    seed_payloads: BTreeSet<ContentHash>,
    /// Under a profile that keeps identities, every node and edge id the lineage held, grouped
    /// by the revision that first held it, in revision order. Empty, and not written, under any
    /// other profile, so that their checkpoints are the bytes they were before the field existed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    held: Vec<HeldAt>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OntologyAt {
    from: RevisionNumber,
    ontology: OntologyDocument,
}
/// The identities first held at one revision.
#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HeldAt {
    from: RevisionNumber,
    nodes: BTreeSet<NodeId>,
    edges: BTreeSet<EdgeId>,
}

/// The checkpoint form of `held`.
fn held_at(held: &HeldIdentities) -> Vec<HeldAt> {
    held.by_revision()
        .into_iter()
        .map(|(from, (nodes, edges))| HeldAt { from, nodes, edges })
        .collect()
}

impl KernelAuthority {
    /// The value-domain address of the host context and anchor a replay ran under.
    fn checkpoint_authority(&self) -> ContentHash {
        struct Bound<'a>(&'a KernelAuthority);
        impl Canonical for Bound<'_> {
            fn encode(&self, out: &mut Encoder) {
                "ekr.replay-checkpoint-authority/1".encode(out);
                self.0.context.operator.encode(out);
                self.0.context.validator.encode(out);
                self.0.anchor.encode(out);
            }
        }
        ContentHash::of(&Bound(self))
    }

    /// This authority's record of having verified the first `covered` occurrences, whose prefix
    /// digest is `prefix`: what a checkpoint pointer carries, so that a later open under the same
    /// host can recognise its own verification of exactly that prefix.
    pub(crate) fn checkpoint_binding(&self, covered: u64, prefix: ContentHash) -> ContentHash {
        struct Binding(ContentHash, u64, ContentHash);
        impl Canonical for Binding {
            fn encode(&self, out: &mut Encoder) {
                "ekr.replay-checkpoint-binding/1".encode(out);
                self.0.encode(out);
                self.1.encode(out);
                self.2.encode(out);
            }
        }
        ContentHash::of(&Binding(self.checkpoint_authority(), covered, prefix))
    }

    /// The occurrences `state` covers and this authority's binding of them, or `None` for a state
    /// whose prefix digest replay did not compute.
    pub(crate) fn verification(&self, state: &ReplayState) -> Option<(u64, ContentHash)> {
        state.digest.map(|prefix| {
            (
                state.version,
                self.checkpoint_binding(state.version, prefix),
            )
        })
    }

    /// The head root of a history whose every occurrence `binding` says this authority verified,
    /// read from the head revision's retained record, the one object `history` must hold.
    /// # Errors
    /// A head record that does not decode or does not name its own occurrence.
    pub(crate) fn head_by_binding(
        &self,
        history: &RetainedHistory,
        binding: ContentHash,
    ) -> Result<Option<ekr_graph::Root>, StoreError> {
        let covered = history.occurrences.len();
        if covered == 0 {
            return Ok(None);
        }
        let prefix = prefix_digests(&history.occurrences)[covered];
        if self.checkpoint_binding(covered as u64, prefix) != binding {
            return Ok(None);
        }
        let Some(head) = history.occurrences.iter().rev().find(|held| {
            matches!(
                held.event.payload,
                RevisionPayload::Seeded { .. } | RevisionPayload::RevisionCommitted { .. }
            )
        }) else {
            return Ok(None);
        };
        let bytes = history.content(head.event.record_hash, StorageClass::Canonical)?;
        let root = match head.event.payload {
            RevisionPayload::Seeded { revision_id, .. } => {
                let record = SeedResultV1::from_bytes(bytes)?;
                require(
                    record.event_id == head.event.event_id
                        && record.revision_id == revision_id
                        && record.result_hash == ContentHash::of(&record.result),
                    "checkpoint-head-record",
                )?;
                record.result
            }
            RevisionPayload::RevisionCommitted {
                revision_id,
                number,
                knowledge_root,
                ..
            } => {
                let record = CommitReceiptV1::from_bytes(bytes)?;
                require(
                    record.event_id == head.event.event_id
                        && record.revision_id == revision_id
                        && record.result.revision == number
                        && record.result.knowledge_root == knowledge_root
                        && record.result_hash == ContentHash::of(&record.result),
                    "checkpoint-head-record",
                )?;
                record.result
            }
            _ => return Ok(None),
        };
        Ok(Some(root))
    }

    /// The checkpoint of `state` with the occurrences it covers and their binding, or `None` for
    /// a state whose prefix digest replay did not compute.
    /// # Errors
    /// A state without its head graph, or an encoding failure.
    pub(crate) fn checkpoint(
        &self,
        state: &ReplayState,
    ) -> Result<Option<(u64, ContentHash, Vec<u8>)>, StoreError> {
        let Some(prefix) = state.digest else {
            return Ok(None);
        };
        let head = state.head();
        let mut ontologies: Vec<OntologyAt> = Vec::new();
        let mut in_force = None;
        for (number, revision) in &state.revisions {
            if in_force != Some(revision.root.ontology_root) {
                ontologies.push(OntologyAt {
                    from: *number,
                    ontology: revision.ontology.to_document(),
                });
                in_force = Some(revision.root.ontology_root);
            }
        }
        let checkpoint = CheckpointV1 {
            format: FORMAT.into(),
            authority: self.checkpoint_authority(),
            covered: state.version,
            prefix,
            revision: head.root.revision,
            graph: GraphDocument::of(head.graph()?),
            ontologies,
            seed_payloads: state.seed_payloads.clone(),
            held: held_at(&state.held),
        };
        let bytes = serde_json::to_vec(&checkpoint)
            .map_err(|error| StoreError::Document(error.to_string()))?;
        Ok(Some((
            state.version,
            self.checkpoint_binding(state.version, prefix),
            bytes,
        )))
    }

    /// Admits `bytes` as the state at the end of the prefix it names, if it is one, and holds it
    /// for the next replay of `history` to continue from.
    /// # Errors
    /// Any check the checkpoint fails. Callers ignore the checkpoint on any error.
    pub(crate) fn restore_checkpoint(
        &self,
        history: &RetainedHistory,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        let mut checkpoint: CheckpointV1 = serde_json::from_slice(bytes)
            .map_err(|error| StoreError::Document(format!("checkpoint-decode: {error}")))?;
        require(checkpoint.format == FORMAT, "checkpoint-format")?;
        require(
            checkpoint.authority == self.checkpoint_authority(),
            "checkpoint-authority",
        )?;
        let covered =
            usize::try_from(checkpoint.covered).map_err(|_| refuse("checkpoint-coverage"))?;
        require(
            covered >= 1 && covered <= history.occurrences.len(),
            "checkpoint-coverage",
        )?;
        let digests = prefix_digests(&history.occurrences[..covered]);
        require(digests[covered] == checkpoint.prefix, "checkpoint-prefix")?;
        let held = self
            .cache
            .lock()
            .map_err(|_| refuse("replay-cache-poisoned"))?
            .longest(&digests)
            .is_some_and(|(reached, _)| reached == covered);
        if held {
            return Ok(());
        }
        let seed_hash = match history.occurrences[0].event.payload {
            RevisionPayload::Seeded { seed_hash, .. } => seed_hash,
            _ => return Err(StoreError::NotSeeded),
        };
        // The graph root and the seed's evidence payloads are the seed's, not the replay's: no
        // revision root binds them, so they are read from the seed envelope the prefix binds.
        // Neither needs the payloads' bytes, so unless this authority already holds the complete
        // envelope, its bytes are read without decoding them.
        let bytes = history.content(seed_hash, StorageClass::Canonical)?;
        let envelope = match self.held_envelope(seed_hash)? {
            Some(envelope) => SeedOutline::Full(envelope),
            None => crate::seed::outline(bytes)?,
        };
        require(
            checkpoint.graph.root == envelope.graph().root,
            "checkpoint-graph-root-identity",
        )?;
        require(
            envelope.payloads_are(&checkpoint.seed_payloads),
            "checkpoint-seed-payloads",
        )?;
        let payloads = checkpoint.seed_payloads.clone();
        let claimed = std::mem::take(&mut checkpoint.held);
        let retained = (checkpoint.covered, checkpoint.revision);
        let mut state = restored(history, covered, checkpoint)?;
        // The identities the lineage held are what the seed's graph holds and what each commit
        // the prefix binds created, read from their retained proposals: the checkpoint's list
        // must be exactly that, so that a changed cache cannot free an id for a new record.
        let derived = if self.anchor.validation_profile.keeps_identities() {
            held_by(envelope.graph(), &state)?
        } else {
            HeldIdentities::default()
        };
        require(held_at(&derived) == claimed, "checkpoint-held-identities")?;
        state.held = derived;
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| refuse("replay-cache-poisoned"))?;
        cache.insert(covered, digests[covered], Arc::new(state));
        cache.seed = Some((seed_hash, payloads, envelope.names_payloads()));
        cache.retained = Some(retained);
        Ok(())
    }

    /// Whether a commit that reached `state` writes a replay checkpoint, rather than only a
    /// pointer to the retained one (design § 99): when this authority knows of no retained
    /// checkpoint — it admitted none and wrote none — or when the head is at least
    /// [`REPLAY_CHECKPOINT_COMMITS`] revisions past the retained one's, or the transactions
    /// committed after it hold at least [`REPLAY_CHECKPOINT_OPERATIONS`] operations, or a
    /// validation or rejection after it was made against a revision before its head, which every
    /// open continuing from it would replay from the seed to re-derive.
    pub(crate) fn checkpoint_due(&self, state: &ReplayState) -> bool {
        let Some((covered, revision)) = self.cache.lock().ok().and_then(|cache| cache.retained)
        else {
            return true;
        };
        let head = state.head().root.revision.get();
        if covered > state.version || head < revision.get() {
            return true;
        }
        if head - revision.get() >= REPLAY_CHECKPOINT_COMMITS {
            return true;
        }
        if state
            .earlier_bases
            .iter()
            .any(|(version, basis)| *version > covered && *basis < revision)
        {
            return true;
        }
        let operations: u64 = state
            .transactions
            .values()
            .filter(|record| {
                record
                    .committed
                    .as_ref()
                    .is_some_and(|receipt| receipt.result.revision > revision)
            })
            .map(|record| record.proposal.operation_count)
            .sum();
        operations >= REPLAY_CHECKPOINT_OPERATIONS
    }

    /// Records that the checkpoint of the state covering `covered` occurrences at `revision` is
    /// the retained one, after this authority wrote it.
    pub(crate) fn checkpoint_retained(&self, covered: u64, revision: RevisionNumber) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.retained = Some((covered, revision));
        }
    }
}

fn held(
    transactions: &mut BTreeMap<ekr_core::TransactionId, TransactionRecord>,
    id: ekr_core::TransactionId,
) -> Result<&mut TransactionRecord, StoreError> {
    transactions
        .get_mut(&id)
        .ok_or_else(|| refuse("checkpoint-transaction-absent"))
}

/// [`ReplayState::note_basis`] for a state being restored, whose head so far is the newest of
/// `coordinates`.
fn note_basis(
    earlier: &mut Vec<(u64, RevisionNumber)>,
    coordinates: &BTreeMap<RevisionNumber, Coordinates>,
    version: u64,
    basis: RevisionNumber,
) {
    if coordinates
        .last_key_value()
        .is_some_and(|(head, _)| basis < *head)
    {
        earlier.push((version, basis));
    }
}

/// The coordinates of one revision, read from its retained record.
struct Coordinates {
    root: ekr_graph::Root,
    revision_id: ekr_core::RevisionId,
    event_id: ekr_core::EventId,
    record_hash: ContentHash,
    committed_at: ekr_core::Timestamp,
}

/// Decodes the records of the first `covered` occurrences into the state replay reached there,
/// with the checkpoint's head graph and schema versions.
fn restored(
    history: &RetainedHistory,
    covered: usize,
    checkpoint: CheckpointV1,
) -> Result<ReplayState, StoreError> {
    let occurrences = &history.occurrences[..covered];
    let first = &occurrences[0];
    let RevisionPayload::Seeded { revision_id, .. } = first.event.payload else {
        return Err(StoreError::NotSeeded);
    };
    let seed = SeedResultV1::from_bytes(
        history.content(first.event.record_hash, StorageClass::Canonical)?,
    )?;
    require(
        seed.event_id == first.event.event_id && seed.revision_id == revision_id,
        "checkpoint-seed",
    )?;
    let mut coordinates = BTreeMap::from([(
        RevisionNumber::SEED,
        Coordinates {
            root: seed.result,
            revision_id,
            event_id: first.event.event_id,
            record_hash: first.event.record_hash,
            committed_at: seed.committed_at,
        },
    )]);
    let mut transactions: BTreeMap<_, TransactionRecord> = BTreeMap::new();
    let mut revision_ids = BTreeSet::from([revision_id]);
    let mut event_ids = BTreeSet::from([first.event.event_id]);
    let mut issue_ids = BTreeSet::new();
    let mut version = first.version;
    let mut earlier_bases = Vec::new();
    for occurrence in &occurrences[1..] {
        let event = &occurrence.event;
        require(
            occurrence.version == version + 1 && event_ids.insert(event.event_id),
            "checkpoint-occurrence-order",
        )?;
        version = occurrence.version;
        let bytes = history.content(event.record_hash, StorageClass::Canonical)?;
        match event.payload {
            RevisionPayload::Seeded { .. } => return Err(StoreError::SeedIsNotFirst),
            RevisionPayload::TransactionProposed { transaction_id, .. } => {
                let proposal = ProposalRecordV1::from_bytes(bytes)?;
                require(
                    proposal.transaction_id == transaction_id,
                    "checkpoint-proposal",
                )?;
                transactions.insert(
                    transaction_id,
                    TransactionRecord {
                        proposal,
                        proposal_record_hash: event.record_hash,
                        validation: None,
                        validation_record_hash: None,
                        rejection: None,
                        committed: None,
                        stale: None,
                    },
                );
            }
            RevisionPayload::TransactionValidated {
                transaction_id,
                against,
                ..
            } => {
                let record = ValidationReceiptV1::from_bytes(bytes)?;
                note_basis(&mut earlier_bases, &coordinates, version, against);
                let held = held(&mut transactions, transaction_id)?;
                held.validation = Some(record);
                held.validation_record_hash = Some(event.record_hash);
            }
            RevisionPayload::TransactionRejected { transaction_id, .. } => {
                let record = RejectionRecordV1::from_bytes(bytes)?;
                let basis = record.requested_basis.previous_root.revision;
                note_basis(&mut earlier_bases, &coordinates, version, basis);
                issue_ids.extend(record.issues.iter().map(|issue| issue.id));
                held(&mut transactions, transaction_id)?.rejection = Some(record);
            }
            RevisionPayload::RevisionCommitted {
                transaction_id,
                revision_id,
                number,
                ..
            } => {
                let record = CommitReceiptV1::from_bytes(bytes)?;
                require(
                    record.revision_id == revision_id
                        && record.result.revision == number
                        && revision_ids.insert(revision_id),
                    "checkpoint-commit",
                )?;
                coordinates.insert(
                    number,
                    Coordinates {
                        root: record.result,
                        revision_id,
                        event_id: event.event_id,
                        record_hash: event.record_hash,
                        committed_at: record.committed_at,
                    },
                );
                held(&mut transactions, transaction_id)?.committed = Some(record);
            }
            RevisionPayload::TransactionStale { transaction_id, .. } => {
                let record = StaleRecordV1::from_bytes(bytes)?;
                held(&mut transactions, transaction_id)?.stale = Some(record);
            }
        }
    }
    let (&head, _) = coordinates
        .last_key_value()
        .expect("the seed revision is always present");
    require(head == checkpoint.revision, "checkpoint-head")?;

    let mut schemas: Vec<(RevisionNumber, Arc<Ontology>, ContentHash)> = Vec::new();
    for at in checkpoint.ontologies {
        let ontology = Ontology::load(at.ontology)
            .map_err(|error| refuse(&format!("checkpoint-ontology: {error}")))?;
        let root = ContentHash::of(&ontology);
        schemas.push((at.from, Arc::new(ontology), root));
    }
    let schema_at = |number: RevisionNumber| {
        schemas
            .iter()
            .rev()
            .find(|(from, _, _)| *from <= number)
            .ok_or_else(|| refuse("checkpoint-ontology-absent"))
    };
    for (number, held) in &coordinates {
        require(
            schema_at(*number)?.2 == held.root.ontology_root,
            "checkpoint-ontology-root",
        )?;
    }

    let document = checkpoint.graph;
    require(document.revision == head, "checkpoint-graph-revision")?;
    let narrowed = |error: ekr_store::MembraneError| refuse(&format!("checkpoint-graph: {error}"));
    let graph = CanonicalGraph {
        root: document.root,
        revision: document.revision,
        ontology: (*schema_at(head)?.1).clone(),
        nodes: document
            .nodes
            .into_iter()
            .map(|(id, node)| Ok((id, narrow_node(node)?)))
            .collect::<Result<_, _>>()
            .map_err(narrowed)?,
        edges: document
            .edges
            .into_iter()
            .map(|(id, edge)| Ok((id, narrow_edge(edge)?)))
            .collect::<Result<_, _>>()
            .map_err(narrowed)?,
        assertions: document
            .assertions
            .into_iter()
            .map(|(id, assertion)| Ok((id, narrow_assertion(assertion)?)))
            .collect::<Result<_, _>>()
            .map_err(narrowed)?,
        evidence: document.evidence,
    };
    let head_root = coordinates[&head].root;
    require(
        knowledge_root(&graph) == head_root.knowledge_root
            && evidence_root(&graph) == head_root.evidence_root,
        "checkpoint-graph-root",
    )?;
    let graph_root = graph.root.id;
    let mut graph = Some(Arc::new(graph));
    let mut revisions = BTreeMap::new();
    for (number, held) in coordinates {
        revisions.insert(
            number,
            Revision {
                root: held.root,
                revision_id: held.revision_id,
                event_id: held.event_id,
                record_hash: held.record_hash,
                committed_at: held.committed_at,
                graph_root,
                ontology: Arc::clone(&schema_at(number)?.1),
                graph: if number == head { graph.take() } else { None },
                aliases: Arc::default(),
            },
        );
    }
    Ok(ReplayState {
        seed,
        revisions,
        transactions: Arc::new(transactions),
        version,
        digest: Some(checkpoint.prefix),
        seed_payloads: checkpoint.seed_payloads,
        documents: BTreeMap::new(),
        validated: BTreeMap::new(),
        revision_ids,
        event_ids,
        issue_ids,
        held: HeldIdentities::default(),
        earlier_bases,
    })
}

/// The node and edge identities the lineage of `state` held: those of the seed graph at the seed
/// revision, and those each committed transaction created at the revision it produced, read from
/// its retained proposal. Under profile v3 that is every identity any of its revisions held,
/// because a v3 commit never creates an id an earlier revision held.
fn held_by(seed: &GraphDocument, state: &ReplayState) -> Result<HeldIdentities, StoreError> {
    let mut held = HeldIdentities::default();
    held.record(
        RevisionNumber::SEED,
        seed.nodes.keys().copied(),
        seed.edges.keys().copied(),
    );
    for record in state.transactions.values() {
        if let Some(committed) = &record.committed {
            let document = state.document(&record.proposal)?;
            held.hold(committed.result.revision, document.transaction());
        }
    }
    Ok(held)
}

#[cfg(test)]
mod tests {
    //! What a fresh open does with a checkpoint, seen from inside the kernel: the state it reaches
    //! holds the graph of the checkpoint's head and of each revision committed after it, and no
    //! earlier one, which a replay from the seed never produces, and so the lineage the
    //! checkpoint covers was not replayed again.
    use crate::{
        Agent, AuthorityStateV1, BootstrapContext, Commit, GraphOperation, GraphTransaction,
        NodeDraft, SeedDocument, ValidationProfileV1, REPLAY_CHECKPOINT_COMMITS,
    };
    use ekr_core::{NodeId, RevisionNumber, Timestamp, TransactionId};
    use ekr_ontology::NodeType;
    use ekr_store::FileStore;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    fn context() -> BootstrapContext {
        BootstrapContext {
            operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
            validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
        }
    }
    fn anchor() -> AuthorityStateV1 {
        let c = context();
        AuthorityStateV1 {
            format: "ekr.authority-state/1".into(),
            agents: [(c.operator, "operator"), (c.validator, "validator")]
                .into_iter()
                .map(|(id, name)| {
                    let capabilities = BTreeSet::new();
                    let name = name.into();
                    (
                        id,
                        Agent {
                            id,
                            name,
                            capabilities,
                        },
                    )
                })
                .collect(),
            validation_profile: ValidationProfileV1::deterministic(c.validator),
        }
    }
    fn open(path: &Path, full: bool) -> Commit<FileStore> {
        Commit::over_with_authority(context(), anchor(), |authority| {
            let mut store = FileStore::file(path, "unit", None)?.under(authority);
            store.set_full_replay(full);
            Ok(store)
        })
        .unwrap()
    }
    fn seed() -> SeedDocument {
        let mut seed =
            SeedDocument::from_yaml(include_str!("../tests/fixtures/seed-minimal-v2.yaml"))
                .unwrap();
        let type_id = "00000000-0000-4000-8000-000000000005".parse().unwrap();
        seed.ontology
            .node_types
            .push(NodeType::new(type_id, "Subject"));
        seed
    }
    fn document(seed: &SeedDocument, n: u64) -> (TransactionId, Vec<u8>) {
        #[derive(serde::Serialize)]
        struct Wire<'a> {
            format: &'static str,
            transaction: &'a GraphTransaction,
        }
        let tx = GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            operations: vec![GraphOperation::CreateNode(NodeDraft {
                id: NodeId::mint(),
                root_id: seed.graph.root.id,
                type_id: seed.ontology.node_types[0].id,
                canonical_name: format!("subject {n}"),
                properties: BTreeMap::new(),
                aliases: Vec::new(),
            })],
            evidence: BTreeSet::new(),
            schema_version: None,
        };
        let wire = Wire {
            format: "ekr.transaction-document/1",
            transaction: &tx,
        };
        (tx.id, serde_yaml_ng::to_string(&wire).unwrap().into_bytes())
    }

    #[test]
    fn a_fresh_open_restores_the_head_and_replays_nothing_the_checkpoint_covers() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = seed();
        open(path, false)
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        // The commit of revision `REPLAY_CHECKPOINT_COMMITS` writes a checkpoint (design § 99);
        // the one after it appends only a pointer.
        let checkpointed = REPLAY_CHECKPOINT_COMMITS;
        let head = checkpointed + 1;
        for n in 1..=head {
            let at = i64::try_from(n * 100).unwrap();
            let (tx, bytes) = document(&seed, n);
            let operator = context().operator;
            open(path, false)
                .propose(&bytes, operator, || Timestamp::from_millis(at))
                .unwrap();
            open(path, false)
                .validate(tx, RevisionNumber::new(n - 1), || {
                    Timestamp::from_millis(at + 1)
                })
                .unwrap();
            open(path, false)
                .commit(tx, operator, || Timestamp::from_millis(at + 2))
                .unwrap();
        }
        let held = |state: &crate::replay::ReplayState| {
            state
                .revisions
                .iter()
                .filter(|(_, revision)| revision.graph.is_some())
                .map(|(number, _)| number.get())
                .collect::<Vec<_>>()
        };
        let restored = open(path, false).read_state().unwrap();
        assert_eq!(
            held(&restored),
            [checkpointed, head],
            "the checkpoint's head graph, and the graph of the one commit replayed after it"
        );
        let replayed = open(path, true).read_state().unwrap();
        assert_eq!(
            held(&replayed),
            (0..=head).collect::<Vec<_>>(),
            "a full replay holds every graph"
        );
        assert_eq!(restored.head().root, replayed.head().root);
        assert_eq!(restored.transactions, replayed.transactions);
        assert_eq!(
            restored.head().graph().unwrap(),
            replayed.head().graph().unwrap()
        );
        for (number, revision) in &replayed.revisions {
            let other = &restored.revisions[number];
            assert_eq!(
                (
                    other.root,
                    other.revision_id,
                    other.event_id,
                    other.record_hash
                ),
                (
                    revision.root,
                    revision.revision_id,
                    revision.event_id,
                    revision.record_hash
                ),
            );
            assert_eq!(other.committed_at, revision.committed_at);
            assert_eq!(other.graph_root, revision.graph_root);
            assert_eq!(*other.ontology, *revision.ontology);
        }
        assert_eq!(restored.version, replayed.version);
        assert_eq!(restored.seed, replayed.seed);
        assert_eq!(restored.seed_payloads, replayed.seed_payloads);
        assert_eq!(restored.revision_ids, replayed.revision_ids);
        assert_eq!(restored.event_ids, replayed.event_ids);
        assert_eq!(restored.issue_ids, replayed.issue_ids);
        assert_eq!(restored.held, replayed.held);
    }
}
