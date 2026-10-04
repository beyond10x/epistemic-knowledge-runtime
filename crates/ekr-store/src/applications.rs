//! Physical application elections and immutable replay inputs; no semantic approval authority.
use super::*;
use ekr_core::contract_data as w;
use serde::{de::DeserializeOwned, Serialize};

const STREAM: &str = "ekr.integrate.application-retention";
pub(super) const MARKER: &str = "ekr.integrate.ApplicationPublicationRecorded";

/// Operational read boundary, not a persisted domain model or a grant of authority.
#[derive(Clone, Copy)]
pub(super) enum ApplicationCapture<'a> {
    Complete,
    Prefix,
    Candidate(&'a RevisionEvent),
    NewCandidate(&'a RevisionEvent),
}

/// Generated application records plus independently retained observation metadata.
/// Private checked codec bytes provide exact Eq without asserting Eq on generated semantic data.
#[derive(Clone, Debug)]
pub struct ApplicationHistory {
    data: w::EkrStoreApplicationRetentionHistory,
    observations: Vec<w::EkrObserveObservationImport>,
    canonical_through: Option<u64>,
    bytes: Vec<u8>,
}
impl Default for ApplicationHistory {
    fn default() -> Self {
        let data = w::EkrStoreApplicationRetentionHistory {
            elections: vec![],
            steps: vec![],
            attempts: vec![],
            coordination: vec![],
            receipts: vec![],
            processing_receipts: vec![],
        };
        Self::new(data, vec![]).expect("empty generated application history serializes")
    }
}
impl PartialEq for ApplicationHistory {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}
impl Eq for ApplicationHistory {}
impl ApplicationHistory {
    fn new(
        data: w::EkrStoreApplicationRetentionHistory,
        observations: Vec<w::EkrObserveObservationImport>,
    ) -> Result<Self, StoreError> {
        let bytes = serde_json::to_vec(&(&data, &observations, None::<u64>)).map_err(json_error)?;
        Ok(Self {
            data,
            observations,
            canonical_through: None,
            bytes,
        })
    }
    /// Complete generated immutable transport. This grants no semantic admission.
    #[must_use]
    pub fn as_data(&self) -> &w::EkrStoreApplicationRetentionHistory {
        &self.data
    }
    /// Explicit canonical occurrence prefix; `None` means a complete capture.
    #[must_use]
    pub fn canonical_through(&self) -> Option<u64> {
        self.canonical_through
    }
    fn scoped(mut self, through: Option<u64>) -> Result<Self, StoreError> {
        self.canonical_through = through;
        self.bytes =
            serde_json::to_vec(&(&self.data, &self.observations, through)).map_err(json_error)?;
        Ok(self)
    }
    /// Retained source records, independently of interpretation/root metadata.
    #[must_use]
    pub fn observations(&self) -> &[w::EkrObserveObservationImport] {
        &self.observations
    }
    /// Whether there is any application material requiring explicit semantic authority.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.elections.is_empty()
            && self.data.steps.is_empty()
            && self.data.attempts.is_empty()
            && self.data.receipts.is_empty()
            && self.data.processing_receipts.is_empty()
            && !self.data.coordination.iter().any(|p| {
                p.entries
                    .iter()
                    .any(|e| matches!(&*e.record, w::EkrIntegrateProposalCoordinationRecord::V0(_)))
            })
    }
    /// Immutable elections.
    #[must_use]
    pub fn elections(&self) -> &[Box<w::EkrIntegrateRetainedApplicationElection>] {
        &self.data.elections
    }
    /// Immutable step templates.
    #[must_use]
    pub fn steps(&self) -> &[Box<w::EkrIntegrateRetainedApplicationStep>] {
        &self.data.steps
    }
    /// Immutable transaction attempts.
    #[must_use]
    pub fn attempts(&self) -> &[Box<w::EkrIntegrateRetainedApplicationAttempt>] {
        &self.data.attempts
    }
    /// Actual physical review and publication prefixes.
    #[must_use]
    pub fn coordination(&self) -> &[Box<w::EkrIntegrateProposalCoordinationRead>] {
        &self.data.coordination
    }
    /// Post-schema progress snapshots.
    #[must_use]
    pub fn receipts(&self) -> &[Box<w::EkrIntegrateApplicationReceiptSnapshot>] {
        &self.data.receipts
    }
    /// Qualified processing receipts.
    #[must_use]
    pub fn processing_receipts(&self) -> &[Box<w::EkrIntegrateProcessingReceiptSnapshot>] {
        &self.data.processing_receipts
    }
}

/// Immutable physical retention. Kernel hooks verify application semantics; this port never
/// creates a canonical transaction or treats a decoded proposal as authority.
pub trait ApplicationRetention {
    /// All elected applications. # Errors: invalid envelope, bytes, identity, or provider failure.
    fn retained_application_elections(
        &self,
    ) -> Result<Vec<w::EkrIntegrateRetainedApplicationElection>, StoreError>;
    /// Elect one immutable proposal/digest winner and atomically pin the supplied objects.
    /// # Errors
    /// Invalid physical input, changed identity, or provider failure.
    fn elect_application(
        &self,
        input: &w::EkrStoreApplicationElectionRetention,
        at: Timestamp,
    ) -> Result<(w::EkrIntegrateRetainedApplicationElection, bool), StoreError>;
    /// All elected steps for an application. # Errors: invalid records or provider failure.
    fn retained_application_steps(
        &self,
        id: &w::EkrIntegrateSchemaApplicationId,
    ) -> Result<Vec<w::EkrIntegrateRetainedApplicationStep>, StoreError>;
    /// Elect the immutable step/allocation winner. # Errors: invalid input or conflicting identity.
    fn elect_application_step(
        &self,
        input: &w::EkrStoreApplicationStepRetention,
        at: Timestamp,
    ) -> Result<(w::EkrIntegrateRetainedApplicationStep, bool), StoreError>;
    /// All attempts of a step in election order. # Errors: invalid records or provider failure.
    fn retained_application_attempts(
        &self,
        id: &w::EkrIntegrateApplicationStepId,
    ) -> Result<Vec<w::EkrIntegrateRetainedApplicationAttempt>, StoreError>;
    /// Elect an attempt against its exact predecessor; semantic authority must admit it.
    /// # Errors
    /// Changed identities/template, missing terminal stale predecessor, or provider failure.
    fn elect_application_attempt(
        &self,
        input: &w::EkrStoreApplicationAttemptRetention,
        at: Timestamp,
    ) -> Result<(w::EkrIntegrateRetainedApplicationAttempt, bool), StoreError>;
    /// Retained progress snapshots. # Errors: invalid records or provider failure.
    fn retained_application_receipts(
        &self,
        id: &w::EkrIntegrateSchemaApplicationId,
    ) -> Result<Vec<w::EkrIntegrateApplicationReceiptSnapshot>, StoreError>;
    /// Record a post-schema snapshot. # Errors: changed identity, invalid links or provider failure.
    fn retain_application_receipt(
        &self,
        record: &w::EkrIntegrateApplicationReceiptSnapshot,
        at: Timestamp,
    ) -> Result<(w::EkrIntegrateApplicationReceiptSnapshot, bool), StoreError>;
    /// All retained qualified item results. # Errors: invalid records or provider failure.
    fn retained_processing_receipts(
        &self,
    ) -> Result<Vec<w::EkrIntegrateProcessingReceiptSnapshot>, StoreError>;
    /// Record a qualified item result. # Errors: changed identity, invalid links or provider failure.
    fn retain_processing_receipt(
        &self,
        record: &w::EkrIntegrateProcessingReceiptSnapshot,
        at: Timestamp,
    ) -> Result<(w::EkrIntegrateProcessingReceiptSnapshot, bool), StoreError>;
}
fn invalid(message: impl std::fmt::Display) -> StoreError {
    StoreError::Document(format!("application: {message}"))
}
fn uuid(s: &str) -> Result<(), StoreError> {
    let id: synonym::Id = s.parse().map_err(invalid)?;
    if id.to_string() != s {
        return Err(invalid("identity spelling"));
    }
    Ok(())
}
mod synonym {
    pub type Id = ekr_core::AgentId;
}
fn hash(s: &str) -> Result<ContentHash, StoreError> {
    s.parse().map_err(invalid)
}
fn value<T: Serialize>(v: &T) -> Result<serde_json::Value, StoreError> {
    serde_json::to_value(v).map_err(json_error)
}
fn encoded<T: Serialize>(v: &T) -> Result<Vec<u8>, StoreError> {
    serde_json::to_vec(v).map_err(json_error)
}
fn identity<T: Serialize>(v: &T, field: &str) -> Result<String, StoreError> {
    value(v)?[field]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid("missing record identity"))
}
fn record_without<T: Serialize>(v: &T, field: &str) -> Result<serde_json::Value, StoreError> {
    let mut v = value(v)?;
    v.as_object_mut()
        .ok_or_else(|| invalid("record"))?
        .remove(field);
    Ok(v)
}
fn objects<'a>(
    items: impl IntoIterator<Item = (&'a String, &'a Box<w::EkrStorePublicationObject>)>,
) -> Result<BTreeMap<ContentHash, crate::PublicationObject>, StoreError> {
    let mut result = BTreeMap::new();
    for (address, object) in items {
        let address = hash(address)?;
        let bytes = ekr_core::bytes::decode(&object.bytes).map_err(invalid)?;
        let class: StorageClass =
            serde_json::from_value(value(&object.storage_class)?).map_err(json_error)?;
        if ContentHash::of_bytes(&bytes) != address
            || class.retention_rank() < StorageClass::Provenance.retention_rank()
        {
            return Err(invalid("unpinned or mismatched staged bytes"));
        }
        let millis = i64::try_from(object.stored_at.0.unix_timestamp_nanos() / 1_000_000)
            .map_err(invalid)?;
        result.insert(
            address,
            crate::PublicationObject {
                bytes,
                storage_class: class,
                stored_at: Timestamp::from_millis(millis),
            },
        );
    }
    Ok(result)
}
fn event_name(table: &str) -> &str {
    match table {
        "elections" => "ekr.integrate.ApplicationElected",
        "steps" => "ekr.integrate.ApplicationStepElected",
        "attempts" => "ekr.integrate.ApplicationAttemptElected",
        "receipts" => "ekr.integrate.ApplicationReceiptRetained",
        "processing" => "ekr.integrate.ProcessingReceiptRetained",
        _ => unreachable!("closed physical table"),
    }
}
impl<S: EventStore> EventlogStore<S> {
    fn application_rows<T: Serialize + DeserializeOwned>(
        &self,
        table: &str,
        id_field: &str,
    ) -> Result<Vec<T>, StoreError> {
        self.entered()?;
        let stream = StreamId::new(self.tenant.clone(), STREAM, table)?;
        let mut ids = BTreeSet::new();
        let mut rows = vec![];
        for event in self.read_all(&stream, MAX_READ_LIMIT)? {
            if event.name != event_name(table)
                || event.schema_version != 1
                || event.version != rows.len() as u64 + 1
            {
                return Err(invalid("record envelope or position"));
            }
            let row: T = serde_json::from_value(event.data).map_err(json_error)?;
            let id = identity(&row, id_field)?;
            uuid(&id)?;
            if !ids.insert(id) {
                return Err(invalid("duplicate record identity"));
            }
            let bytes = encoded(&row)?;
            let address = ContentHash::of_bytes(&bytes);
            let held = self
                .object(address)?
                .ok_or_else(|| invalid("record bytes absent"))?;
            if *held.bytes != bytes
                || held.metadata.storage_class.retention_rank()
                    < StorageClass::Provenance.retention_rank()
            {
                return Err(invalid("record bytes changed or unpinned"));
            }
            rows.push(row);
        }
        Ok(rows)
    }
    fn application_rows_capture<T: Serialize + DeserializeOwned>(
        &self,
        table: &str,
        id_field: &str,
        audit: &[RecordedEvent],
        cutoff: Option<u64>,
        candidate: Option<&str>,
    ) -> Result<Vec<T>, StoreError> {
        if cutoff.is_none() {
            return self.application_rows(table, id_field);
        }
        let mut rows = vec![];
        let mut identities = BTreeSet::new();
        for (index, event) in audit
            .iter()
            .filter(|event| event.stream_type == STREAM && event.stream_id == table)
            .enumerate()
        {
            if event.version != index as u64 + 1 {
                return Err(invalid("capture physical position"));
            }
            let selected = event.global_seq <= cutoff.unwrap_or_default()
                || candidate.is_some_and(|id| event.data[id_field].as_str() == Some(id));
            if !selected {
                continue;
            }
            if event.name != event_name(table) || event.schema_version != 1 || event.is_redacted() {
                return Err(invalid("capture record envelope"));
            }
            let row: T = serde_json::from_value(event.data.clone()).map_err(json_error)?;
            let id = identity(&row, id_field)?;
            uuid(&id)?;
            if !identities.insert(id) {
                return Err(invalid("duplicate capture identity"));
            }
            let bytes = encoded(&row)?;
            let held = self
                .object(ContentHash::of_bytes(&bytes))?
                .ok_or_else(|| invalid("capture record bytes absent"))?;
            if *held.bytes != bytes
                || held.metadata.storage_class.retention_rank()
                    < StorageClass::Provenance.retention_rank()
            {
                return Err(invalid("capture record bytes changed or unpinned"));
            }
            rows.push(row);
        }
        Ok(rows)
    }
    fn append_application<T: Serialize>(
        &self,
        table: &str,
        length: usize,
        record: &T,
        mut held: BTreeMap<ContentHash, crate::PublicationObject>,
        at: Timestamp,
    ) -> Result<bool, StoreError>
    where
        S: AtomicBlobEventStore,
    {
        let bytes = encoded(record)?;
        held.insert(
            ContentHash::of_bytes(&bytes),
            crate::PublicationObject {
                bytes,
                storage_class: StorageClass::Provenance,
                stored_at: at,
            },
        );
        let mut appends = vec![StreamAppend {
            stream: StreamId::new(self.tenant.clone(), STREAM, table)?,
            expected: if length == 0 {
                Expected::NoStream
            } else {
                Expected::Exact(length as u64)
            },
            events: vec![NewEvent::new(event_name(table), 1, value(record)?)?],
        }];
        let mut blobs = vec![];
        for (address, object) in held {
            if let Some(append) = self.object_append(address, &object)? {
                appends.push(append);
            }
            blobs.push(BlobWrite {
                digest: address.to_hex(),
                bytes: object.bytes,
            });
        }
        let mut request = BlobAppendGroup {
            group: AppendGroup {
                tenant: self.tenant.clone(),
                appends,
                meta: envelope(
                    "ekr.application-retention",
                    ContentHash::of_bytes(&encoded(record)?).to_hex(),
                ),
            },
            blobs,
        };
        let key = format!("ekr.application-retention.{}", request.fingerprint()?);
        request.group.meta.idempotency_key = key.clone();
        request.group.meta.request_id = key.clone();
        request.group.meta.trace_id = key;
        self.atomic(&request).map(|r| !r.deduplicated)
    }
    fn linked_proposal(
        &self,
        e: &w::EkrIntegrateRetainedApplicationElection,
    ) -> Result<w::EkrIntegrateRetainedSchemaProposal, StoreError> {
        uuid(&e.application_id.0)?;
        uuid(&e.base_schema.0)?;
        uuid(&e.schema_transaction.id.0)?;
        hash(&e.proposal_digest.0)?;
        hash(&e.initial_proof_digest.0)?;
        let p = self
            .read_schema_proposals()?
            .into_iter()
            .find(|p| {
                p.proposal.proposal_id == e.proposal_id && p.proposal_digest == e.proposal_digest
            })
            .ok_or_else(|| invalid("proposal/digest absent"))?;
        if !self.read_proposal_reviews(&e.proposal_id)?.iter().any(|r| {
            r.review.review_id == e.initial_review_id
                && r.decision.proof_digest == e.initial_proof_digest
        }) {
            return Err(invalid("initial review absent"));
        }
        Ok(p)
    }
    fn application_for(
        &self,
        id: &w::EkrIntegrateSchemaApplicationId,
    ) -> Result<w::EkrIntegrateRetainedApplicationElection, StoreError> {
        self.application_rows::<w::EkrIntegrateRetainedApplicationElection>(
            "elections",
            "application_id",
        )?
        .into_iter()
        .find(|e| *e.application_id == *id)
        .ok_or_else(|| invalid("application absent"))
    }
    fn pin_application_input(
        &self,
        address: ContentHash,
        staged: &mut BTreeMap<ContentHash, crate::PublicationObject>,
        at: Timestamp,
    ) -> Result<Vec<u8>, StoreError> {
        if let Some(object) = staged.get(&address) {
            return Ok(object.bytes.clone());
        }
        let object = self
            .object(address)?
            .ok_or_else(|| invalid("independent source bytes missing"))?;
        let bytes = (*object.bytes).clone();
        staged.insert(
            address,
            crate::PublicationObject {
                bytes: bytes.clone(),
                storage_class: StorageClass::strongest(
                    object.metadata.storage_class,
                    StorageClass::Provenance,
                ),
                stored_at: at,
            },
        );
        Ok(bytes)
    }
    fn election_material(
        &self,
        e: &w::EkrIntegrateRetainedApplicationElection,
        staged: &mut BTreeMap<ContentHash, crate::PublicationObject>,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let p = self.linked_proposal(e)?;
        self.pin_application_input(hash(&e.proposal_digest.0)?, staged, at)?;
        let mut versions = p
            .proposal
            .sources
            .iter()
            .map(|s| &*s.version)
            .collect::<Vec<_>>();
        versions.extend(e.selected_items.iter().map(|i| &*i.source));
        let mut observations = p
            .proposal
            .observations
            .iter()
            .map(|o| o.0.clone())
            .collect::<BTreeSet<_>>();
        for version in versions {
            uuid(&version.interpretation_id.0)?;
            let bytes =
                self.pin_application_input(hash(&version.document_digest.0)?, staged, at)?;
            let document: w::EkrIntegrateInterpretationDocument =
                serde_json::from_slice(&bytes).map_err(json_error)?;
            if document.version.interpretation_id != version.interpretation_id
                || document.version.version != version.version
            {
                return Err(invalid("independent source version mismatch"));
            }
            observations.extend(document.observations.iter().map(|o| o.0.clone()));
            for evidence in &document.evidence {
                let bytes = ekr_core::bytes::decode(&evidence.payload).map_err(invalid)?;
                let address = hash(&evidence.evidence.content_hash.0)?;
                if ContentHash::of_bytes(&bytes) != address {
                    return Err(invalid("source evidence payload mismatch"));
                }
                staged.insert(
                    address,
                    crate::PublicationObject {
                        bytes,
                        storage_class: StorageClass::Provenance,
                        stored_at: at,
                    },
                );
            }
        }
        let retained = self.read_observations()?;
        for id in observations {
            let observation = retained
                .iter()
                .find(|o| o.observation.observation_id.0 == id)
                .ok_or_else(|| invalid("independent observation missing"))?;
            self.pin_application_input(hash(&observation.observation.content_hash.0)?, staged, at)?;
        }
        Ok(())
    }
}
impl<S: AtomicBlobEventStore> EventlogStore<S> {
    fn require_unprepared_application_transaction(
        &self,
        id: &w::EkrKernelTransactionId,
    ) -> Result<(), StoreError> {
        let key = crate::PublicationCommandKey {
            kind: crate::PublicationCommandKind::Propose,
            transaction_id: Some(id.0.parse().map_err(invalid)?),
            predecessor_event_id: None,
            predecessor_record_hash: None,
            answer_id: None,
        };
        if self.read_preparation(&key)?.is_some() {
            return Err(invalid("application transaction already prepared"));
        }
        Ok(())
    }
}
impl<S: AtomicBlobEventStore> ApplicationRetention for EventlogStore<S> {
    fn retained_application_elections(
        &self,
    ) -> Result<Vec<w::EkrIntegrateRetainedApplicationElection>, StoreError> {
        self.application_rows("elections", "application_id")
    }
    fn elect_application(
        &self,
        input: &w::EkrStoreApplicationElectionRetention,
        at: Timestamp,
    ) -> Result<(w::EkrIntegrateRetainedApplicationElection, bool), StoreError> {
        let row = &*input.election;
        let mut staged = objects(input.objects.ess_extra.iter())?;
        self.election_material(row, &mut staged, at)?;
        for _ in 0..16 {
            let rows = self.application_rows::<w::EkrIntegrateRetainedApplicationElection>(
                "elections",
                "application_id",
            )?;
            if let Some(held) = rows.iter().find(|h| h.application_id == row.application_id) {
                return if held == row {
                    Ok((held.clone(), false))
                } else {
                    Err(StoreError::PublicationInputConflict)
                };
            }
            if let Some(held) = rows.iter().find(|h| {
                h.proposal_id == row.proposal_id && h.proposal_digest == row.proposal_digest
            }) {
                return Ok((held.clone(), false));
            }
            if rows
                .iter()
                .any(|h| h.schema_transaction.id == row.schema_transaction.id)
            {
                return Err(StoreError::PublicationInputConflict);
            }
            if self.occurrences(MAX_READ_LIMIT, None)?.iter().any(|o| {
                o.event
                    .transaction_id()
                    .is_some_and(|tx| tx.to_string() == row.schema_transaction.id.0)
            }) {
                return Err(invalid("election transaction already published"));
            }
            self.require_unprepared_application_transaction(&row.schema_transaction.id)?;
            match self.append_application("elections", rows.len(), row, staged.clone(), at) {
                Ok(inserted) => return Ok((row.clone(), inserted)),
                Err(StoreError::Conflict) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(StoreError::Conflict)
    }
    fn retained_application_steps(
        &self,
        id: &w::EkrIntegrateSchemaApplicationId,
    ) -> Result<Vec<w::EkrIntegrateRetainedApplicationStep>, StoreError> {
        uuid(&id.0)?;
        Ok(self
            .application_rows::<w::EkrIntegrateRetainedApplicationStep>(
                "steps",
                "step_election_id",
            )?
            .into_iter()
            .filter(|r| *r.application_id == *id)
            .collect())
    }
    fn elect_application_step(
        &self,
        input: &w::EkrStoreApplicationStepRetention,
        at: Timestamp,
    ) -> Result<(w::EkrIntegrateRetainedApplicationStep, bool), StoreError> {
        let row = &*input.step;
        uuid(&row.step_election_id.0)?;
        uuid(&row.transaction.id.0)?;
        let e = self.application_for(&row.application_id)?;
        let mut staged = objects(input.objects.ess_extra.iter())?;
        self.election_material(&e, &mut staged, at)?;
        if matches!(*row.step.kind, w::EkrIntegrateApplicationStepKind::V2)
            && row.transaction != e.schema_transaction
        {
            return Err(invalid("schema step differs from election"));
        }
        for mapping in &row.mappings {
            uuid(&mapping.mapping_id.0)?;
            let bytes = ekr_core::bytes::decode(&mapping.payload).map_err(invalid)?;
            if ContentHash::of_bytes(&bytes) != hash(&mapping.mapping_digest.0)?
                || bytes != encoded(&mapping.mapping)?
                || mapping.proposal_digest != e.proposal_digest
            {
                return Err(invalid("mapping bytes"));
            }
            self.pin_application_input(hash(&mapping.source_document_digest.0)?, &mut staged, at)?;
            staged.insert(
                hash(&mapping.mapping_digest.0)?,
                crate::PublicationObject {
                    bytes,
                    storage_class: StorageClass::Provenance,
                    stored_at: at,
                },
            );
        }
        for _ in 0..16 {
            let rows = self.application_rows::<w::EkrIntegrateRetainedApplicationStep>(
                "steps",
                "step_election_id",
            )?;
            if let Some(held) = rows
                .iter()
                .find(|h| h.step_election_id == row.step_election_id)
            {
                return if held == row {
                    Ok((held.clone(), false))
                } else {
                    Err(StoreError::PublicationInputConflict)
                };
            }
            if let Some(held) = rows
                .iter()
                .find(|h| h.application_id == row.application_id && h.step == row.step)
            {
                return Ok((held.clone(), false));
            }
            if rows.iter().any(|h| h.transaction.id == row.transaction.id) {
                return Err(StoreError::PublicationInputConflict);
            }
            self.require_unprepared_application_transaction(&row.transaction.id)?;
            if self.occurrences(MAX_READ_LIMIT, None)?.iter().any(|o| {
                o.event
                    .transaction_id()
                    .is_some_and(|tx| tx.to_string() == row.transaction.id.0)
            }) {
                return Err(invalid("step transaction already published"));
            }
            match self.append_application("steps", rows.len(), row, staged.clone(), at) {
                Ok(inserted) => return Ok((row.clone(), inserted)),
                Err(StoreError::Conflict) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(StoreError::Conflict)
    }
    fn retained_application_attempts(
        &self,
        id: &w::EkrIntegrateApplicationStepId,
    ) -> Result<Vec<w::EkrIntegrateRetainedApplicationAttempt>, StoreError> {
        uuid(&id.0)?;
        Ok(self
            .application_rows::<w::EkrIntegrateRetainedApplicationAttempt>(
                "attempts",
                "transaction_id",
            )?
            .into_iter()
            .filter(|r| *r.step_election_id == *id)
            .collect())
    }
    fn elect_application_attempt(
        &self,
        input: &w::EkrStoreApplicationAttemptRetention,
        at: Timestamp,
    ) -> Result<(w::EkrIntegrateRetainedApplicationAttempt, bool), StoreError> {
        let row = &*input.attempt;
        uuid(&row.transaction_id.0)?;
        if row.transaction_id != row.transaction.id {
            return Err(invalid("attempt transaction identity"));
        }
        let step = self
            .application_rows::<w::EkrIntegrateRetainedApplicationStep>(
                "steps",
                "step_election_id",
            )?
            .into_iter()
            .find(|s| s.step_election_id == row.step_election_id)
            .ok_or_else(|| invalid("attempt step absent"))?;
        if record_without(&row.transaction, "id")? != record_without(&step.transaction, "id")? {
            return Err(invalid("attempt changed template"));
        }
        for _ in 0..16 {
            let rows = self.application_rows::<w::EkrIntegrateRetainedApplicationAttempt>(
                "attempts",
                "transaction_id",
            )?;
            if let Some(held) = rows.iter().find(|h| h.transaction_id == row.transaction_id) {
                return if held == row {
                    Ok((held.clone(), false))
                } else {
                    Err(StoreError::PublicationInputConflict)
                };
            }
            if let Some(held) = rows.iter().find(|h| {
                h.step_election_id == row.step_election_id
                    && h.predecessor_transaction == row.predecessor_transaction
            }) {
                return Ok((held.clone(), false));
            }
            let previous = rows
                .iter()
                .rev()
                .find(|h| h.step_election_id == row.step_election_id);
            match (
                previous,
                &row.predecessor_transaction,
                &row.predecessor_record_hash,
            ) {
                (None, w::EssPresence::Absent, w::EssPresence::Absent)
                    if row.transaction == step.transaction => {}
                (Some(prior), w::EssPresence::Present(id), w::EssPresence::Present(record))
                    if *id == prior.transaction_id =>
                {
                    hash(&record.0)?;
                }
                _ => return Err(StoreError::Conflict),
            }
            let history = self.load_history(MAX_READ_LIMIT, None)?;
            self.authority()?
                .verify_application_attempt(&history, row)?;
            match self.append_application("attempts", rows.len(), row, BTreeMap::new(), at) {
                Ok(inserted) => return Ok((row.clone(), inserted)),
                Err(StoreError::Conflict) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(StoreError::Conflict)
    }
    fn retained_application_receipts(
        &self,
        id: &w::EkrIntegrateSchemaApplicationId,
    ) -> Result<Vec<w::EkrIntegrateApplicationReceiptSnapshot>, StoreError> {
        uuid(&id.0)?;
        Ok(self
            .application_rows::<w::EkrIntegrateApplicationReceiptSnapshot>(
                "receipts",
                "receipt_id",
            )?
            .into_iter()
            .filter(|r| *r.application_id == *id)
            .collect())
    }
    fn retain_application_receipt(
        &self,
        row: &w::EkrIntegrateApplicationReceiptSnapshot,
        at: Timestamp,
    ) -> Result<(w::EkrIntegrateApplicationReceiptSnapshot, bool), StoreError> {
        uuid(&row.receipt_id.0)?;
        let e = self.application_for(&row.application_id)?;
        if row.proposal_id != e.proposal_id
            || matches!(*row.progress, w::EkrIntegrateApplicationProgress::V1)
        {
            return Err(invalid("receipt requires schema commit"));
        }
        let history = self.load_history(MAX_READ_LIMIT, None)?;
        let tx = row
            .schema_transaction
            .0
            .parse::<ekr_core::TransactionId>()
            .map_err(invalid)?;
        if !history.occurrences.iter().any(|o| {
            matches!(o.event.payload, RevisionPayload::RevisionCommitted { transaction_id, number, .. }
                if transaction_id == tx && Some(number.get()) == row.schema_revision.0.as_u64())
                && o.event.application.as_ref().is_some_and(|guard| {
                    guard.as_data().application_id == row.application_id
                        && matches!(*guard.as_data().step.kind, w::EkrIntegrateApplicationStepKind::V2)
                })
        }) { return Err(invalid("receipt schema commit absent")); }
        if !history.applications.coordination().iter().flat_map(|p| &p.entries).any(|entry| {
            matches!(&*entry.record, w::EkrIntegrateProposalCoordinationRecord::V1(r)
                if r.value.review.proposal_id == row.proposal_id && r.value.review.review_id == row.review_id)
        }) { return Err(invalid("receipt review absent")); }
        let mut receipt_ids = BTreeSet::new();
        for id in &row.processing_receipts {
            uuid(&id.0)?;
            if !receipt_ids.insert(&id.0)
                || !history
                    .applications
                    .processing_receipts()
                    .iter()
                    .any(|r| r.receipt_id == *id)
            {
                return Err(invalid("receipt processing link absent or duplicate"));
            }
        }
        for _ in 0..16 {
            let rows = self.application_rows::<w::EkrIntegrateApplicationReceiptSnapshot>(
                "receipts",
                "receipt_id",
            )?;
            for held in &rows {
                if held.receipt_id == row.receipt_id {
                    return if held == row {
                        Ok((held.clone(), false))
                    } else {
                        Err(StoreError::PublicationInputConflict)
                    };
                }
                if record_without(held, "receipt_id")? == record_without(row, "receipt_id")? {
                    return Ok((held.clone(), false));
                }
            }
            match self.append_application("receipts", rows.len(), row, BTreeMap::new(), at) {
                Ok(inserted) => return Ok((row.clone(), inserted)),
                Err(StoreError::Conflict) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(StoreError::Conflict)
    }
    fn retained_processing_receipts(
        &self,
    ) -> Result<Vec<w::EkrIntegrateProcessingReceiptSnapshot>, StoreError> {
        self.application_rows("processing", "receipt_id")
    }
    fn retain_processing_receipt(
        &self,
        row: &w::EkrIntegrateProcessingReceiptSnapshot,
        at: Timestamp,
    ) -> Result<(w::EkrIntegrateProcessingReceiptSnapshot, bool), StoreError> {
        uuid(&row.receipt_id.0)?;
        hash(&row.document_digest.0)?;
        hash(&row.basis_digest.0)?;
        let mut staged = BTreeMap::new();
        self.pin_application_input(hash(&row.document_digest.0)?, &mut staged, at)?;
        if let w::EssPresence::Present(mapping) = &row.mapping_digest {
            self.pin_application_input(hash(&mapping.0)?, &mut staged, at)?;
        }
        if let w::EssPresence::Present(tx) = &row.transaction_id {
            uuid(&tx.0)?;
        }
        for assertion in &row.assertions {
            uuid(&assertion.0)?;
        }
        for _ in 0..16 {
            let rows = self.application_rows::<w::EkrIntegrateProcessingReceiptSnapshot>(
                "processing",
                "receipt_id",
            )?;
            for held in &rows {
                if held.receipt_id == row.receipt_id {
                    return if held == row {
                        Ok((held.clone(), false))
                    } else {
                        Err(StoreError::PublicationInputConflict)
                    };
                }
                if record_without(held, "receipt_id")? == record_without(row, "receipt_id")? {
                    return Ok((held.clone(), false));
                }
            }
            match self.append_application("processing", rows.len(), row, staged.clone(), at) {
                Ok(inserted) => return Ok((row.clone(), inserted)),
                Err(StoreError::Conflict) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(StoreError::Conflict)
    }
}

impl<S: EventStore> EventlogStore<S> {
    fn application_audit(&self) -> Result<Vec<RecordedEvent>, StoreError> {
        let mut after = 0;
        let mut events = vec![];
        loop {
            crate::verified::count_stream_read(|reads| reads.feed += 1);
            let page = self
                .runtime()
                .block_on(self.store.read_feed(&self.tenant, after, MAX_READ_LIMIT))
                .map_err(|error| {
                    // Preserve the provider's recovery signal: a held reader must reopen a
                    // replaced history. Other unavailable feeds still fail this mandatory audit.
                    if crate::eventlog::diverged(&error) {
                        StoreError::from(error)
                    } else {
                        StoreError::Document("application-audit-unavailable".into())
                    }
                })?;
            for event in &page.events {
                if event.tenant != self.tenant || event.global_seq <= after {
                    return Err(invalid("application feed order"));
                }
                after = event.global_seq;
                events.push(event.clone());
            }
            if !page.has_more {
                return Ok(events);
            }
            if page.events.is_empty() {
                return Err(invalid("application feed stalled"));
            }
        }
    }
    pub(super) fn has_application_records(&self) -> Result<bool, StoreError> {
        Ok(self
            .application_audit()?
            .iter()
            .any(|event| event.stream_type == STREAM || event.name == MARKER))
    }
    pub(super) fn application_marker(
        &self,
        event: &RevisionEvent,
    ) -> Result<Option<w::EkrIntegrateApplicationPublicationRecord>, StoreError> {
        let Some(guard) = &event.application else {
            return Ok(None);
        };
        if !event.supported() {
            return Err(invalid("unsupported guarded event"));
        }
        let command = match event.payload {
            RevisionPayload::TransactionProposed { .. } => "Propose",
            RevisionPayload::TransactionValidated { .. }
            | RevisionPayload::TransactionRejected { .. } => "Validate",
            RevisionPayload::TransactionStale { .. }
            | RevisionPayload::RevisionCommitted { .. } => "Commit",
            _ => return Err(invalid("guarded event is not ordinary")),
        };
        serde_json::from_value(serde_json::json!({"guard":guard.as_data(),"event_id":event.event_id,"record_hash":event.record_hash,"transaction_id":event.transaction_id(),"command":command})).map(Some).map_err(json_error)
    }
    pub(super) fn application_append(
        &self,
        event: &RevisionEvent,
    ) -> Result<Option<StreamAppend>, StoreError> {
        self.application_marker(event)?
            .map(|record| {
                Ok(StreamAppend {
                    stream: StreamId::new(
                        self.tenant.clone(),
                        proposal_reviews::STREAM,
                        &record.guard.proposal_id.0,
                    )?,
                    expected: Expected::Exact(
                        record
                            .guard
                            .review_stream_version
                            .as_u64()
                            .ok_or_else(|| invalid("marker cursor"))?,
                    ),
                    events: vec![NewEvent::new(MARKER, 1, value(&record)?)?],
                })
            })
            .transpose()
    }
    fn physical_application_links(
        &self,
        data: &w::EkrStoreApplicationRetentionHistory,
        occurrences: &[RecordedOccurrence],
    ) -> Result<(), StoreError> {
        let mut markers = BTreeMap::new();
        for coordination in &data.coordination {
            if coordination.stream_version.as_u64() != Some(coordination.entries.len() as u64) {
                return Err(invalid("coordination repeated cursor"));
            }
            let mut canonical_version = 0;
            for (index, entry) in coordination.entries.iter().enumerate() {
                if entry.stream_version.as_u64() != Some(index as u64 + 1) {
                    return Err(invalid("coordination position"));
                }
                if let w::EkrIntegrateProposalCoordinationRecord::V0(marker) = &*entry.record {
                    let marker = &*marker.value;
                    let held = occurrences
                        .iter()
                        .find(|o| o.event.event_id.to_string() == marker.event_id.0)
                        .ok_or_else(|| invalid("marker lacks canonical occurrence"))?;
                    if self.application_marker(&held.event)?.as_ref() != Some(marker)
                        || held.version <= canonical_version
                        || markers.insert(marker.event_id.0.clone(), marker).is_some()
                    {
                        return Err(invalid("duplicate/mismatched/out-of-order marker link"));
                    }
                    canonical_version = held.version;
                }
            }
        }
        for held in occurrences {
            let elected = held.event.transaction_id().is_some_and(|tx| {
                data.attempts
                    .iter()
                    .any(|a| a.transaction_id.0 == tx.to_string())
                    || data
                        .steps
                        .iter()
                        .any(|s| s.transaction.id.0 == tx.to_string())
                    || data
                        .elections
                        .iter()
                        .any(|e| e.schema_transaction.id.0 == tx.to_string())
            });
            if elected && held.event.application.is_none() {
                return Err(invalid("elected transaction omitted application guard"));
            }
            let Some(guard) = &held.event.application else {
                continue;
            };
            let guard = guard.as_data();
            if !markers.contains_key(&held.event.event_id.to_string()) {
                return Err(invalid("guarded occurrence lacks marker"));
            }
            let election = data
                .elections
                .iter()
                .find(|e| e.application_id == guard.application_id)
                .ok_or_else(|| invalid("guard election absent"))?;
            let step = data
                .steps
                .iter()
                .find(|s| s.step_election_id == guard.step_election_id)
                .ok_or_else(|| invalid("guard step absent"))?;
            let attempt = data
                .attempts
                .iter()
                .find(|a| a.transaction_id == guard.attempt_transaction)
                .ok_or_else(|| invalid("guard attempt absent"))?;
            if election.proposal_id != guard.proposal_id
                || election.proposal_digest != guard.proposal_digest
                || step.application_id != guard.application_id
                || step.step != guard.step
                || attempt.step_election_id != guard.step_election_id
                || attempt.transaction_id != attempt.transaction.id
            {
                return Err(invalid("guard election/step/attempt links"));
            }
            let coordination = data
                .coordination
                .iter()
                .find(|p| p.proposal_id == guard.proposal_id)
                .ok_or_else(|| invalid("guard coordination absent"))?;
            let prefix = usize::try_from(
                guard
                    .review_stream_version
                    .as_u64()
                    .ok_or_else(|| invalid("guard cursor"))?,
            )
            .map_err(invalid)?;
            let latest = coordination
                .entries
                .get(..prefix)
                .ok_or_else(|| invalid("guard prefix absent"))?
                .iter()
                .rev()
                .find_map(|entry| match &*entry.record {
                    w::EkrIntegrateProposalCoordinationRecord::V1(r) => Some(&*r.value),
                    _ => None,
                })
                .ok_or_else(|| invalid("guard review absent"))?;
            if latest.review.review_id != guard.review_id
                || latest.decision.proof_digest != guard.human_proof_digest
                || !matches!(*latest.review.decision, w::EkrIntegrateReviewDecision::V0)
            {
                return Err(invalid("guard effective approval mismatch"));
            }
        }
        Ok(())
    }
    pub(super) fn load_application_material(
        &self,
        history: &mut RetainedHistory,
        capture: ApplicationCapture<'_>,
    ) -> Result<(), StoreError> {
        let audit = self.application_audit()?;
        // New requests must respect identities already reserved after their canonical basis.
        // Re-reading an immutable preparation instead uses Candidate and its original prefix;
        // unrelated later elections cannot become that preparation's authority input.
        if let ApplicationCapture::NewCandidate(event) = capture {
            if event.application.is_none() {
                if let Some(transaction) = event.transaction_id() {
                    let id = transaction.to_string();
                    if audit.iter().any(|row| {
                        row.stream_type == STREAM
                            && match row.stream_id.as_str() {
                                "elections" => {
                                    row.data["schema_transaction"]["id"].as_str() == Some(&id)
                                }
                                "steps" => row.data["transaction"]["id"].as_str() == Some(&id),
                                "attempts" => row.data["transaction_id"].as_str() == Some(&id),
                                _ => false,
                            }
                    }) {
                        return Err(invalid("elected transaction omitted application guard"));
                    }
                }
            }
        }
        let cutoff = match capture {
            ApplicationCapture::Complete => None,
            _ => Some(match history.occurrences.last() {
                None => 0,
                Some(last) => {
                    audit
                        .iter()
                        .find(|event| {
                            event.stream_type == REVISION_STREAM_TYPE
                                && event.stream_id == REVISION_STREAM_ID
                                && event.event_id == last.provider_event_id
                                && event.version == last.version
                                && serde_json::to_value(&last.event)
                                    .is_ok_and(|data| data == event.data)
                        })
                        .ok_or_else(|| invalid("capture canonical boundary absent"))?
                        .global_seq
                }
            }),
        };
        let through = cutoff.map(|_| history.occurrences.len() as u64);
        let candidate = match capture {
            ApplicationCapture::Candidate(event) | ApplicationCapture::NewCandidate(event) => {
                event.application.as_ref().map(|g| g.as_data())
            }
            _ => None,
        };
        let selected_events = history
            .occurrences
            .iter()
            .map(|o| o.event.event_id.to_string())
            .collect::<BTreeSet<_>>();
        let markers = audit
            .iter()
            .filter(|event| {
                event.name == MARKER
                    && (cutoff.is_none_or(|position| event.global_seq <= position)
                        || event.data["event_id"]
                            .as_str()
                            .is_some_and(|id| selected_events.contains(id)))
            })
            .collect::<Vec<_>>();
        let mut retention_present = false;
        for event in audit.iter().filter(|event| {
            event.stream_type == STREAM
                && cutoff.is_none_or(|position| event.global_seq <= position)
        }) {
            retention_present = true;
            if !matches!(
                event.stream_id.as_str(),
                "elections" | "steps" | "attempts" | "receipts" | "processing"
            ) || event.schema_version != 1
                || event.name != event_name(&event.stream_id)
                || event.is_redacted()
            {
                return Err(invalid("unknown or redacted retention envelope"));
            }
        }
        let elections = self
            .application_rows_capture::<w::EkrIntegrateRetainedApplicationElection>(
                "elections",
                "application_id",
                &audit,
                cutoff,
                candidate.map(|g| g.application_id.0.as_str()),
            )?;
        // Absence is established from the actual tenant feed, including markers on unknown
        // proposal identities. An empty election table alone cannot rule out an orphan marker.
        if elections.is_empty()
            && markers.is_empty()
            && !retention_present
            && !history
                .occurrences
                .iter()
                .any(|o| o.event.application.is_some())
        {
            history.applications = ApplicationHistory::default().scoped(through)?;
            return Ok(());
        }
        let steps = self.application_rows_capture::<w::EkrIntegrateRetainedApplicationStep>(
            "steps",
            "step_election_id",
            &audit,
            cutoff,
            candidate.map(|g| g.step_election_id.0.as_str()),
        )?;
        let attempts = self.application_rows_capture::<w::EkrIntegrateRetainedApplicationAttempt>(
            "attempts",
            "transaction_id",
            &audit,
            cutoff,
            candidate.map(|g| g.attempt_transaction.0.as_str()),
        )?;
        let receipts = self.application_rows_capture::<w::EkrIntegrateApplicationReceiptSnapshot>(
            "receipts",
            "receipt_id",
            &audit,
            cutoff,
            None,
        )?;
        let processing_receipts = self
            .application_rows_capture::<w::EkrIntegrateProcessingReceiptSnapshot>(
                "processing",
                "receipt_id",
                &audit,
                cutoff,
                None,
            )?;
        let mut coordination = vec![];
        let mut observed = BTreeSet::new();
        let mut required = BTreeSet::new();
        let mut proposals = BTreeSet::new();
        for receipt in &processing_receipts {
            required.insert(hash(&receipt.document_digest.0)?);
            if let w::EssPresence::Present(mapping) = &receipt.mapping_digest {
                required.insert(hash(&mapping.0)?);
            }
        }
        for receipt in &receipts {
            if !elections.iter().any(|e| {
                e.application_id == receipt.application_id && e.proposal_id == receipt.proposal_id
            }) {
                return Err(invalid("orphan application receipt"));
            }
            for id in &receipt.processing_receipts {
                if !processing_receipts.iter().any(|r| r.receipt_id == *id) {
                    return Err(invalid("missing processing receipt"));
                }
            }
        }
        for election in &elections {
            uuid(&election.application_id.0)?;
            if !proposals.insert(election.proposal_id.0.clone()) {
                return Err(invalid("duplicate election slot"));
            }
            let proposal = self
                .read_schema_proposals_selected(Some(&election.proposal_id.0))?
                .into_iter()
                .find(|p| p.proposal_digest == election.proposal_digest)
                .ok_or_else(|| invalid("captured proposal/digest absent"))?;
            let candidate_prefix = candidate
                .filter(|g| g.proposal_id == election.proposal_id)
                .and_then(|g| g.review_stream_version.as_u64());
            let maximum = audit
                .iter()
                .filter(|event| {
                    event.stream_type == proposal_reviews::STREAM
                        && event.stream_id == election.proposal_id.0
                })
                .filter(|event| {
                    cutoff.is_none_or(|position| event.global_seq <= position)
                        || candidate_prefix.is_some_and(|version| event.version <= version)
                        || markers
                            .iter()
                            .any(|marker| marker.event_id == event.event_id)
                })
                .map(|event| event.version)
                .max()
                .unwrap_or_default();
            let events = audit
                .iter()
                .filter(|event| {
                    event.stream_type == proposal_reviews::STREAM
                        && event.stream_id == election.proposal_id.0
                        && event.version <= maximum
                })
                .cloned()
                .collect();
            let prefix =
                self.read_proposal_coordination_events(&election.proposal_id, &proposal, events)?;
            if !prefix.entries.iter().any(|entry| matches!(&*entry.record, w::EkrIntegrateProposalCoordinationRecord::V1(r)
                if r.value.review.review_id == election.initial_review_id && r.value.decision.proof_digest == election.initial_proof_digest)) {
                return Err(invalid("captured initial review absent"));
            }
            required.insert(hash(&proposal.proposal_digest.0)?);
            observed.extend(proposal.proposal.observations.iter().map(|o| o.0.clone()));
            let versions = proposal
                .proposal
                .sources
                .iter()
                .map(|s| &*s.version)
                .chain(election.selected_items.iter().map(|item| &*item.source));
            for version in versions {
                let digest = hash(&version.document_digest.0)?;
                required.insert(digest);
                let object = self
                    .object(digest)?
                    .ok_or_else(|| invalid("immutable source document missing"))?;
                if object.metadata.storage_class.retention_rank()
                    < StorageClass::Provenance.retention_rank()
                {
                    return Err(invalid("source document is not pinned"));
                }
                let document: w::EkrIntegrateInterpretationDocument =
                    serde_json::from_slice(&object.bytes).map_err(json_error)?;
                if document.version.interpretation_id != version.interpretation_id
                    || document.version.version != version.version
                {
                    return Err(invalid("source version mismatch"));
                }
                observed.extend(document.observations.iter().map(|o| o.0.clone()));
                for evidence in &document.evidence {
                    required.insert(hash(&evidence.evidence.content_hash.0)?);
                }
            }
            for entry in &prefix.entries {
                if let w::EkrIntegrateProposalCoordinationRecord::V1(r) = &*entry.record {
                    for address in [
                        &r.value.decision.proof_object_hash.0,
                        &r.value.decision.policy_object_hash.0,
                        &r.value.decision.statement_object_hash.0,
                    ] {
                        required.insert(hash(address)?);
                    }
                }
            }
            coordination.push(Box::new(prefix));
        }
        for step in &steps {
            if !elections
                .iter()
                .any(|e| e.application_id == step.application_id)
            {
                return Err(invalid("orphan step"));
            }
            for mapping in &step.mappings {
                for address in [
                    &mapping.mapping_digest.0,
                    &mapping.proposal_digest.0,
                    &mapping.source_document_digest.0,
                ] {
                    required.insert(hash(address)?);
                }
            }
        }
        for attempt in &attempts {
            if !steps
                .iter()
                .any(|s| s.step_election_id == attempt.step_election_id)
            {
                return Err(invalid("orphan attempt"));
            }
        }
        let observations = self
            .read_observations_selected(Some(&observed))?
            .into_iter()
            .filter(|o| observed.contains(&o.observation.observation_id.0))
            .collect::<Vec<_>>();
        if observations.len() != observed.len() {
            return Err(invalid("referenced observation missing"));
        }
        for observation in &observations {
            required.insert(hash(&observation.observation.content_hash.0)?);
        }
        let data = w::EkrStoreApplicationRetentionHistory {
            elections: elections.into_iter().map(Box::new).collect(),
            steps: steps.into_iter().map(Box::new).collect(),
            attempts: attempts.into_iter().map(Box::new).collect(),
            receipts: receipts.into_iter().map(Box::new).collect(),
            processing_receipts: processing_receipts.into_iter().map(Box::new).collect(),
            coordination,
        };
        let physical = data
            .coordination
            .iter()
            .flat_map(|p| {
                p.entries
                    .iter()
                    .filter_map(move |entry| match &*entry.record {
                        w::EkrIntegrateProposalCoordinationRecord::V0(marker) => Some((
                            &p.proposal_id.0,
                            entry.stream_version.as_u64(),
                            &*marker.value,
                        )),
                        _ => None,
                    })
            })
            .collect::<Vec<_>>();
        if physical.len() != markers.len() {
            return Err(invalid("orphan application marker in tenant feed"));
        }
        for marker in markers {
            if marker.is_redacted()
                || marker.stream_type != proposal_reviews::STREAM
                || marker.schema_version != 1
                || !physical.iter().any(|(proposal, version, record)| {
                    marker.stream_id == **proposal
                        && Some(marker.version) == *version
                        && value(*record).is_ok_and(|data| data == marker.data)
                })
            {
                return Err(invalid("unbound application marker in tenant feed"));
            }
        }
        // A selected capture is explicitly marker-closed. The read port has no native group
        // identity; no grouping is inferred from caller-controlled request metadata.
        self.physical_application_links(&data, &history.occurrences)?;
        self.load_objects(history, required.iter().copied())?;
        for address in required {
            history.content(address, StorageClass::Provenance)?;
        }
        history.applications = ApplicationHistory::new(data, observations)?.scoped(through)?;
        Ok(())
    }
    pub(super) fn stage_application_marker(
        &self,
        history: &mut RetainedHistory,
        event: &RevisionEvent,
    ) -> Result<(), StoreError> {
        let Some(marker) = self.application_marker(event)? else {
            if let Some(tx) = event.transaction_id() {
                if history
                    .applications
                    .attempts()
                    .iter()
                    .any(|a| a.transaction_id.0 == tx.to_string())
                    || history
                        .applications
                        .steps()
                        .iter()
                        .any(|s| s.transaction.id.0 == tx.to_string())
                    || history
                        .applications
                        .elections()
                        .iter()
                        .any(|e| e.schema_transaction.id.0 == tx.to_string())
                {
                    return Err(invalid("elected transaction omitted application guard"));
                }
            }
            return Ok(());
        };
        let mut data = history.applications.data.clone();
        let target = data
            .coordination
            .iter_mut()
            .find(|p| p.proposal_id == marker.guard.proposal_id)
            .ok_or_else(|| invalid("candidate proposal prefix absent"))?;
        let prefix = usize::try_from(
            marker
                .guard
                .review_stream_version
                .as_u64()
                .ok_or_else(|| invalid("candidate cursor"))?,
        )
        .map_err(invalid)?;
        if prefix > target.entries.len() {
            return Err(invalid("candidate prefix absent"));
        }
        target.entries.truncate(prefix);
        target
            .entries
            .push(Box::new(w::EkrIntegrateProposalCoordinationEntry {
                stream_version: (prefix as u64 + 1).into(),
                record: Box::new(w::EkrIntegrateProposalCoordinationRecord::V0(
                    w::EkrIntegrateProposalCoordinationRecordVariant0 {
                        kind: w::EkrIntegrateProposalCoordinationRecordVariant0Kind::V0,
                        value: Box::new(marker),
                    },
                )),
            }));
        target.stream_version = (prefix as u64 + 1).into();
        let through = history
            .applications
            .canonical_through
            .map(|_| history.occurrences.len() as u64);
        history.applications =
            ApplicationHistory::new(data, history.applications.observations.clone())?
                .scoped(through)?;
        // Candidate links are exact; unrelated later markers may exist in retained preparation reads,
        // so the immutable basis is checked by kernel authority rather than treating later events as
        // an authorization for this pending request.
        let mut selected = history.applications.data.clone();
        for p in &mut selected.coordination {
            if let Some(index)=p.entries.iter().position(|entry|matches!(&*entry.record,w::EkrIntegrateProposalCoordinationRecord::V0(r) if !history.occurrences.iter().any(|o|o.event.event_id.to_string()==r.value.event_id.0))){p.entries.truncate(index);p.stream_version=(index as u64).into();}
        }
        self.physical_application_links(&selected, &history.occurrences)?;
        // Validate the candidate's identities and effective review without renumbering historical
        // prefixes: the full generated snapshot is what semantic replay receives.
        self.authority()?.verify_application_history(history)
    }
}
