//! A run staged and published whole, or dropped whole (design § 107,
//! `story:a-run-is-staged-and-published-whole`).
//!
//! A stage is its own Eventlog tenant in the same store, begun by the store's preserving copy at
//! its head. Begin, seal, publish and abandon run here, over the store's handle and handles on the
//! stage's tenant, and the store's record of each stage is `ekr-store`'s ([`StageLog`]).
//!
//! A publication derives every occurrence the stage holds after its copy again for the store's
//! lineage, with the functions a preserving migration uses, and replays the store's history with
//! the suffix after it through this kernel's authority before it elects anything. The store then
//! admits the whole group only after its injected authority verifies the same history, so
//! invariant 1 holds as for any publication: no new constructor of a validated transaction, and
//! canonical state moves only for what the kernel authority stands behind.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ekr_core::{Canonical, ContentHash, Encoder, EventId, RevisionNumber, StageId};
use ekr_graph::RevisionPayload;
use ekr_store::{
    Initialize, Inventory, ObjectStore, ProviderKind, RecordedOccurrence, RetainedObject,
    RevisionLog, StageLog, StagePoint, StagePublication, StagePublicationObject, StageRecord,
    StageResult, StageState, StorageClass, StoreError, StoredObject,
};

use crate::migrate::CapturedStore;
use crate::replay::ReplayState;
use crate::{Commit, CommitError};

/// A stage as the store's record lists it (`ekr.store.Stages`): the stage and its state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageListing {
    /// The stage, with the store's revisions its publication added.
    pub stage: ekr_store::Stage,
    /// Its state.
    pub state: StageState,
}

fn conflict(stage: StageId, state: StageState) -> CommitError {
    StoreError::StageStateConflict {
        stage_id: stage,
        state,
    }
    .into()
}

/// The code a refusal of the stage's suffix is recorded under: a document refusal's leading code,
/// or the refusal's own text.
fn code_of(error: &StoreError) -> String {
    match error {
        StoreError::Document(text) => text.split(':').next().unwrap_or(text).to_owned(),
        other => other
            .to_string()
            .split(':')
            .next()
            .unwrap_or_default()
            .to_owned(),
    }
}

/// The store's head in `occurrences`: the number of its last committed revision.
fn head_of(occurrences: &[RecordedOccurrence]) -> RevisionNumber {
    occurrences
        .iter()
        .rev()
        .find_map(|held| match held.event.payload {
            RevisionPayload::Seeded { .. } => Some(RevisionNumber::SEED),
            RevisionPayload::RevisionCommitted { number, .. } => Some(number),
            _ => None,
        })
        .unwrap_or(RevisionNumber::SEED)
}

/// Whether `candidate`, the store's replay with the suffix after it, reaches what `stage`, the
/// stage's own replay, reached: every revision with its identities, time and roots, the head
/// graph, and every transaction of the stage in the same state. The store may hold transactions
/// the stage does not (another writer's proposals); never a revision.
fn suffix_agrees(stage: &ReplayState, candidate: &ReplayState) -> Result<(), StoreError> {
    let disagrees =
        |what: &str| StoreError::Document(format!("migrate-verification-disagrees: {what}"));
    if stage.revisions.len() != candidate.revisions.len() {
        return Err(disagrees("revision count"));
    }
    for ((number, a), (other, b)) in stage.revisions.iter().zip(&candidate.revisions) {
        if number != other
            || a.revision_id != b.revision_id
            || a.event_id != b.event_id
            || a.committed_at != b.committed_at
            || a.root.knowledge_root != b.root.knowledge_root
            || a.root.evidence_root != b.root.evidence_root
            || a.root.ontology_root != b.root.ontology_root
            || a.root.agent_root != b.root.agent_root
        {
            return Err(disagrees(&format!("revision {number}")));
        }
    }
    let (a, b) = (stage.head().graph()?, candidate.head().graph()?);
    if a.nodes != b.nodes || a.edges != b.edges || a.assertions != b.assertions {
        return Err(disagrees("head graph"));
    }
    if a.evidence != b.evidence {
        return Err(disagrees("evidence"));
    }
    for (id, record) in stage.transactions.iter() {
        if candidate
            .transactions
            .get(id)
            .is_none_or(|held| held.state() != record.state())
        {
            return Err(disagrees(&format!("transaction {id}")));
        }
    }
    Ok(())
}

/// The command input a stage's publication is elected for: the stage, the expected head, and the
/// host context and anchor.
fn stage_input(
    stage: StageId,
    expect_head: RevisionNumber,
    authority: &crate::KernelAuthority,
) -> ContentHash {
    struct Material(StageId, RevisionNumber);
    impl Canonical for Material {
        fn encode(&self, out: &mut Encoder) {
            "ekr.stage-publication-input/1".encode(out);
            self.0.encode(out);
            self.1.encode(out);
        }
    }
    let material = ContentHash::of(&Material(stage, expect_head));
    crate::commands::input_hash(
        "PublishStage",
        material.to_hex().as_bytes(),
        authority.context.operator,
        authority,
    )
}

/// A stage's object as the stage holds it: its first class, every raise, its `stored_at` and its
/// bytes.
fn ladder(object: &ekr_store::InventoriedObject) -> StagePublicationObject {
    StagePublicationObject {
        storage_class: object.stored_as,
        raised_to: object.raised_to.clone(),
        stored_at: object.object.metadata.stored_at,
        bytes: object.object.bytes.to_vec(),
    }
}

impl<C: RevisionLog + ObjectStore + Inventory> Commit<C> {
    /// Reads a stage's tenant once for a seal or a publication: its inventory, checked for a
    /// decision elected and never published, and replayed in full. A copy without its completion
    /// receipt, or no copy at all, is `stage-incomplete`.
    fn capture_stage(&self, stage: StageId) -> Result<CapturedStore<'_, C>, CommitError> {
        self.capture_with(|pending| StoreError::UnresolvedPreparation(pending).into())
            .map_err(|error| match error {
                CommitError::NotSeeded => StoreError::StageIncomplete(stage).into(),
                CommitError::Store(StoreError::Document(code))
                    if code.starts_with("migrate-incomplete") =>
                {
                    StoreError::StageIncomplete(stage).into()
                }
                other => other,
            })
    }
}

impl<S: RevisionLog + ObjectStore + Inventory + StageLog> Commit<S> {
    /// The record of `stage`, which must exist.
    fn stage(&self, stage: StageId) -> Result<StageRecord, CommitError> {
        self.store
            .stage_record(stage)?
            .ok_or_else(|| StoreError::StageNotFound(stage).into())
    }

    /// Refuses unless the store's head equals both `expect_head` and the stage's base.
    fn head_is_base(
        &self,
        record: &StageRecord,
        expect_head: RevisionNumber,
    ) -> Result<(), CommitError> {
        let current = self.store.head()?.ok_or(CommitError::NotSeeded)?.revision;
        if current == expect_head && current == record.base {
            Ok(())
        } else {
            Err(StoreError::StageHeadMoved {
                stage_id: record.stage_id,
                expected: expect_head,
                base: record.base,
                current,
            }
            .into())
        }
    }

    /// `ekr.cli.BeginStage`: mints the stage id, records `StageBegun` with the base `source`'s
    /// capture reads, and copies that capture into the stage's tenant, its completion receipt
    /// last. `source` is the store read as one image (SQLite) or the store itself (PostgreSQL,
    /// one capture); `open` opens the stage's tenant of the same store.
    pub(crate) fn begin_stage<I, D>(
        &self,
        source: &Commit<I>,
        open: impl FnOnce(&str) -> Result<Commit<D>, StoreError>,
    ) -> Result<StageResult, CommitError>
    where
        I: RevisionLog + ObjectStore + Inventory,
        D: RevisionLog + ObjectStore + Initialize + Inventory,
    {
        let provider = self.store.provider();
        if provider == ProviderKind::File {
            return Err(StoreError::StageUnsupportedProvider(provider).into());
        }
        let captured =
            source.capture_with(|pending| StoreError::UnresolvedPreparation(pending).into())?;
        let stage = StageId::mint();
        let tenant = ekr_store::stage_tenant(&self.store.store_tenant(), stage)?;
        let record = self.store.record_stage_begun(
            stage,
            &tenant,
            captured.head().revision,
            captured.head_revision(),
        )?;
        let destination = open(&tenant)?;
        captured.copy_into(&destination)?;
        Ok(record.result())
    }

    /// `ekr.cli.SealStage`: checks the stage Begun, complete and holding no decision elected and
    /// never published, and the store's head at `expect_head` and the base, then records
    /// `StageSealed`. A stage already Sealing answers its original result.
    pub(crate) fn seal_stage<C: RevisionLog + ObjectStore + Inventory>(
        &self,
        stage: StageId,
        expect_head: RevisionNumber,
        open: impl FnOnce(&str) -> Result<Commit<C>, StoreError>,
    ) -> Result<StageResult, CommitError> {
        let record = self.stage(stage)?;
        match record.state {
            StageState::Sealing => return Ok(record.result()),
            StageState::Published | StageState::Abandoned => {
                return Err(conflict(stage, record.state));
            }
            StageState::Begun => {}
        }
        self.head_is_base(&record, expect_head)?;
        open(&record.tenant)?.capture_stage(stage)?;
        ekr_store::stage::reached(StagePoint::SealChecked)?;
        match self.store.record_stage_sealed(&record) {
            Ok(sealed) => Ok(sealed.result()),
            Err(StoreError::Conflict) => {
                let now = self.stage(stage)?;
                if now.state == StageState::Sealing {
                    Ok(now.result())
                } else {
                    Err(conflict(stage, now.state))
                }
            }
            Err(error) => Err(error.into()),
        }
    }

    /// `ekr stage publish`: reads the stage's record and, on a Begun stage, seals it first;
    /// otherwise it publishes alone, so a retry after the seal publishes and one after the append
    /// returns the original result.
    pub(crate) fn seal_and_publish_stage<C: RevisionLog + ObjectStore + Inventory>(
        &self,
        stage: StageId,
        expect_head: RevisionNumber,
        open: impl Fn(&str) -> Result<Commit<C>, StoreError>,
    ) -> Result<StageResult, CommitError> {
        if self.stage(stage)?.state == StageState::Begun {
            self.seal_stage(stage, expect_head, &open)?;
            ekr_store::stage::reached(StagePoint::Sealed)?;
        }
        self.publish_stage(stage, expect_head, &open)
    }

    /// `ekr.cli.PublishStage`: publishes a Sealing stage's suffix into the store in one append
    /// group, or resumes its elected attempt; a Published stage retried with the same expected
    /// head returns its original result. Then the stage's tenant is forgotten.
    pub(crate) fn publish_stage<C: RevisionLog + ObjectStore + Inventory>(
        &self,
        stage: StageId,
        expect_head: RevisionNumber,
        open: impl FnOnce(&str) -> Result<Commit<C>, StoreError>,
    ) -> Result<StageResult, CommitError> {
        let record = self.stage(stage)?;
        match record.state {
            StageState::Begun | StageState::Abandoned => {
                return Err(conflict(stage, record.state));
            }
            StageState::Published => {
                if expect_head != record.base {
                    return Err(conflict(stage, record.state));
                }
                self.store.forget_stage_tenant(stage)?;
                return Ok(record.result());
            }
            StageState::Sealing => {}
        }
        self.head_is_base(&record, expect_head)?;
        let input = stage_input(stage, expect_head, &self.authority);
        match self.store.stage_preparation(stage)? {
            Some(elected) => {
                if elected.input_hash != input {
                    return Err(StoreError::PublicationInputConflict.into());
                }
                match self.store.resume_stage_publication(&elected) {
                    Ok(_) => {}
                    // Design §§ 94.2, 107.5: the successor keeps the suffix and appends at the
                    // stream's current version, its object appends rebuilt against what the store
                    // holds now; the store's authority checks it again before it is elected.
                    Err(
                        StoreError::StageStreamMoved { .. } | StoreError::StageObjectMoved { .. },
                    ) => {
                        let decision = StagePublication {
                            expected_version: self.store.revision_stream_version()?,
                            ..elected.decision.clone()
                        };
                        let next = self.store.prepare_stage_publication(
                            input,
                            &decision,
                            Some(&elected),
                        )?;
                        ekr_store::stage::reached(StagePoint::PublishElected)?;
                        let _ = self.store.resume_stage_publication(&next)?;
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            None => {
                let decision = self.stage_suffix(&record, &open(&record.tenant)?)?;
                let prepared = self
                    .store
                    .prepare_stage_publication(input, &decision, None)?;
                ekr_store::stage::reached(StagePoint::PublishElected)?;
                let _ = self.store.resume_stage_publication(&prepared)?;
            }
        }
        ekr_store::stage::reached(StagePoint::PublishAppended)?;
        self.store.forget_stage_tenant(stage)?;
        Ok(self.stage(stage)?.result())
    }

    /// The decision of `record`'s publication: the stage's occurrences after its copy, derived
    /// again for the store's lineage against the store as it reads now, each replayed through this
    /// kernel's authority after the store's history, and every object they and the run's other
    /// writes need. The store's replay with the suffix must reach what the stage's own replay
    /// reached. Any refusal of the stage's replay, of a derivation or of that agreement is
    /// `stage-suffix-refused`.
    fn stage_suffix<C: RevisionLog + ObjectStore + Inventory>(
        &self,
        record: &StageRecord,
        stage_store: &Commit<C>,
    ) -> Result<StagePublication, CommitError> {
        let stage = record.stage_id;
        let refused = |error: StoreError| -> CommitError {
            StoreError::StageSuffixRefused {
                stage_id: stage,
                code: code_of(&error),
                reason: error.to_string(),
            }
            .into()
        };
        let captured = stage_store
            .capture_stage(stage)
            .map_err(|error| match error {
                CommitError::Store(
                    error @ (StoreError::StageIncomplete(_) | StoreError::UnresolvedPreparation(_)),
                ) => error.into(),
                CommitError::Store(error) => refused(error),
                other => other,
            })?;
        let (stage_history, stage_state, inventory) =
            (captured.history(), captured.state(), captured.inventory());
        let store_history = self.store.history()?;
        let shared = stage_history
            .occurrences
            .iter()
            .zip(&store_history.occurrences)
            .take_while(|(a, b)| a.event.event_id == b.event.event_id)
            .count();
        let run = &stage_history.occurrences[shared..];
        let in_store: BTreeSet<EventId> = store_history
            .occurrences
            .iter()
            .map(|held| held.event.event_id)
            .collect();
        if run
            .iter()
            .any(|held| in_store.contains(&held.event.event_id))
        {
            return Err(refused(StoreError::Document(
                "stage-suffix-already-in-store".into(),
            )));
        }
        let current = head_of(&store_history.occurrences);
        if current != record.base {
            return Err(StoreError::StageHeadMoved {
                stage_id: stage,
                expected: record.base,
                base: record.base,
                current,
            }
            .into());
        }
        let mut state = self
            .authority
            .reconstruct(&store_history, None, None)?
            .ok_or(CommitError::NotSeeded)?;
        let expected_version = store_history.occurrences.len() as u64;
        let mut candidate = store_history;
        let mut occurrences = Vec::with_capacity(run.len());
        let mut objects: BTreeMap<ContentHash, StagePublicationObject> = BTreeMap::new();
        let mut replaced = BTreeSet::new();
        let held = |objects: &BTreeMap<ContentHash, StagePublicationObject>,
                    candidate: &mut ekr_store::RetainedHistory| {
            for (hash, object) in objects {
                candidate
                    .objects
                    .entry(*hash)
                    .or_insert_with(|| RetainedObject {
                        metadata: StoredObject {
                            content_hash: *hash,
                            storage_class: object
                                .raised_to
                                .last()
                                .copied()
                                .unwrap_or(object.storage_class),
                            byte_len: object.bytes.len() as u64,
                            stored_at: object.stored_at,
                        },
                        bytes: Arc::new(object.bytes.clone()),
                    });
            }
        };
        for occurrence in run {
            let event = &occurrence.event;
            let bytes = stage_history
                .content(event.record_hash, StorageClass::Canonical)
                .map_err(refused)?;
            let stored_at = stage_history
                .objects
                .get(&event.record_hash)
                .map(|held| held.metadata.stored_at)
                .ok_or_else(|| refused(StoreError::Document("migrate-record-missing".into())))?;
            let basis = match event.payload {
                RevisionPayload::TransactionValidated { against, .. } => self
                    .authority
                    .holding(&candidate, Arc::clone(&state), against)
                    .map_err(refused)?,
                _ => Arc::clone(&state),
            };
            let (derived, derived_bytes) =
                self.migrated_decision(&basis, event, bytes)
                    .map_err(|error| match error {
                        CommitError::Store(error) => refused(error),
                        other => other,
                    })?;
            if derived.record_hash != event.record_hash {
                replaced.insert(event.record_hash);
            }
            objects
                .entry(derived.record_hash)
                .or_insert(StagePublicationObject {
                    storage_class: StorageClass::Canonical,
                    raised_to: Vec::new(),
                    stored_at,
                    bytes: derived_bytes,
                });
            if let RevisionPayload::RevisionCommitted { transaction_id, .. } = derived.payload {
                let tx = basis
                    .transactions
                    .get(&transaction_id)
                    .ok_or(StoreError::ProposalMissing { transaction_id })
                    .map_err(refused)?;
                let document = basis.document(&tx.proposal).map_err(refused)?;
                for hash in crate::commands::added_payloads(document.transaction()).into_keys() {
                    let object = inventory.objects.get(&hash).ok_or_else(|| {
                        refused(StoreError::Document(format!(
                            "migrate-payload-missing: {hash}"
                        )))
                    })?;
                    objects.entry(hash).or_insert_with(|| ladder(object));
                }
            }
            held(&objects, &mut candidate);
            candidate.occurrences.push(RecordedOccurrence {
                version: candidate.occurrences.len() as u64 + 1,
                provider_event_id: format!("stage-suffix:{}", derived.event_id),
                event: derived.clone(),
            });
            state = self
                .authority
                .reconstruct(&candidate, None, None)
                .map_err(refused)?
                .ok_or(CommitError::NotSeeded)?;
            occurrences.push(derived);
        }
        // Every other object the stage stored or raised after its copy's completion receipt, the
        // copy's last write, with its class, raises and `stored_at` — never a record of the
        // stage's own lineage: the copy wrote those before its receipt, and the run's records the
        // publication derived again are left out in favour of what it derived.
        let claim = stage_state.migration_claim.ok_or_else(|| {
            refused(StoreError::Document(
                "stage-not-a-copy: no migration claim".into(),
            ))
        })?;
        let receipt = ContentHash::of_bytes(&crate::migrate::completion(
            claim,
            stage_state.seed.seed_hash,
        )?);
        let copied = inventory
            .objects
            .get(&receipt)
            .and_then(|held| held.positions.first().copied())
            .ok_or(StoreError::StageIncomplete(stage))?;
        for (hash, object) in &inventory.objects {
            if replaced.contains(hash)
                || objects.contains_key(hash)
                || !object.positions.iter().any(|position| *position > copied)
            {
                continue;
            }
            objects.insert(*hash, ladder(object));
        }
        suffix_agrees(stage_state, &state).map_err(refused)?;
        let added: Vec<_> = state
            .revisions
            .range(RevisionNumber::new(record.base.get() + 1)..)
            .map(|(_, revision)| revision.revision_id)
            .collect();
        Ok(StagePublication {
            stage_id: stage,
            base: record.base,
            expected_version,
            occurrences,
            objects,
            head: state.head().root.revision,
            published_first: added.first().copied(),
            published_last: added.last().copied(),
        })
    }

    /// `ekr.cli.AbandonStage`: records `StageAbandoned` for a Begun or Sealing stage, conditionally
    /// on the record as read, then forgets the stage's tenant. A stage already Abandoned answers
    /// its original result and finishes the forgetting.
    pub(crate) fn abandon_stage(&self, stage: StageId) -> Result<StageResult, CommitError> {
        let mut record = self.stage(stage)?;
        for _ in 0..8 {
            match record.state {
                StageState::Published => return Err(conflict(stage, record.state)),
                StageState::Abandoned => {
                    self.store.forget_stage_tenant(stage)?;
                    return Ok(record.result());
                }
                StageState::Begun | StageState::Sealing => {
                    ekr_store::stage::reached(StagePoint::AbandonRead)?;
                    match self.store.record_stage_abandoned(&record) {
                        Ok(abandoned) => {
                            ekr_store::stage::reached(StagePoint::AbandonRecorded)?;
                            self.store.forget_stage_tenant(stage)?;
                            return Ok(abandoned.result());
                        }
                        Err(StoreError::Conflict) => record = self.stage(stage)?,
                        Err(error) => return Err(error.into()),
                    }
                }
            }
        }
        Err(StoreError::Conflict.into())
    }

    /// Every stage the store has recorded (`ekr.store.Stages`), with the revisions a publication
    /// added read from the store's revision stream.
    pub(crate) fn stages(&self) -> Result<Vec<StageListing>, CommitError> {
        let records = self.store.stages()?;
        let committed: Vec<ekr_core::RevisionId> = if records
            .iter()
            .any(|record| record.state == StageState::Published)
        {
            self.store
                .history()?
                .occurrences
                .iter()
                .filter_map(|held| match held.event.payload {
                    RevisionPayload::RevisionCommitted { revision_id, .. } => Some(revision_id),
                    _ => None,
                })
                .collect()
        } else {
            Vec::new()
        };
        Ok(records
            .into_iter()
            .map(|record| {
                let published_revisions = match record.published.as_ref() {
                    Some(published) => {
                        match (published.published_first, published.published_last) {
                            (Some(first), Some(last)) => {
                                let from = committed.iter().position(|id| *id == first);
                                let to = committed.iter().position(|id| *id == last);
                                match (from, to) {
                                    (Some(from), Some(to)) if from <= to => {
                                        committed[from..=to].to_vec()
                                    }
                                    _ => Vec::new(),
                                }
                            }
                            _ => Vec::new(),
                        }
                    }
                    None => Vec::new(),
                };
                StageListing {
                    stage: ekr_store::Stage {
                        stage_id: record.stage_id,
                        store: ekr_store::StoreTenant(self.store.store_tenant()),
                        base: record.base,
                        base_revision: record.base_revision,
                        published_revisions,
                    },
                    state: record.state,
                }
            })
            .collect())
    }
}
