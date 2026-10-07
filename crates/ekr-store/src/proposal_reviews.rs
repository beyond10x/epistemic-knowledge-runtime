//! Physical human-review retention; authentication and semantic admission belong to the kernel.
use super::*;
use ekr_core::contract_data::{EkrIntegrateRetainedProposalReview, EkrIntegrateSchemaProposalId};
use ekr_core::generated_identity::{Identity, ProposalReviewId, SchemaProposalId};

pub(super) const STREAM: &str = "ekr.integrate.proposal-reviews";
const IDENTITIES: &str = "ekr.integrate.proposal-review-identities";
const RECORDED: &str = "ekr.integrate.ProposalReviewRetained";
const BOUND: &str = "ekr.integrate.ProposalReviewIdentityBound";

/// Immutable physical review records and pinned input bytes, not decision authority.
/// Protocol digests, signatures, targets and operator authentication are verified by the kernel;
/// this port checks object addresses and duplicated fields. The kernel verifies the proof's
/// predecessor against the explicit expected predecessor passed to this port.
/// Human protocol digests are not the domain-addressed hashes of stored objects.
pub trait ProposalReviewRetention {
    /// Physical proposal stream including markers; markers never become human predecessors.
    /// # Errors
    /// Invalid closed event union, cursor, links, identities or pinned bytes.
    fn proposal_coordination(
        &self,
        proposal_id: &EkrIntegrateSchemaProposalId,
    ) -> Result<ekr_core::contract_data::EkrIntegrateProposalCoordinationRead, StoreError>;
    /// Reads the verified physical records of one retained proposal, in publication order.
    /// # Errors
    /// Missing proposal, corrupt records or pinned bytes, or provider failure.
    fn retained_proposal_reviews(
        &self,
        proposal_id: &EkrIntegrateSchemaProposalId,
    ) -> Result<Vec<EkrIntegrateRetainedProposalReview>, StoreError>;
    /// Atomically binds both identities and pins the exact proof, policy and statement bytes.
    /// Exact retries return the original before checking the current human predecessor.
    /// # Errors
    /// Inconsistent input, changed identity input, stale predecessor or provider failure.
    fn retain_proposal_review(
        &self,
        record: &EkrIntegrateRetainedProposalReview,
        expected_previous: Option<ContentHash>,
        at: Timestamp,
    ) -> Result<(EkrIntegrateRetainedProposalReview, bool), StoreError>;
}

fn invalid(detail: impl std::fmt::Display) -> StoreError {
    StoreError::Document(format!("proposal-review: {detail}"))
}
fn hash(text: &str) -> Result<ContentHash, StoreError> {
    text.parse().map_err(invalid)
}
fn uuid(text: &str) -> Result<(), StoreError> {
    text.parse::<ekr_core::AgentId>()
        .map(|_| ())
        .map_err(invalid)
}
fn payloads(
    record: &EkrIntegrateRetainedProposalReview,
) -> Result<BTreeMap<ContentHash, Vec<u8>>, StoreError> {
    let review = &record.review;
    let decision = &record.decision;
    let evidence = &record.statement.evidence;
    SchemaProposalId::parse_identity(&review.proposal_id.0).map_err(invalid)?;
    ProposalReviewId::parse_identity(&review.review_id.0).map_err(invalid)?;
    uuid(&decision.decision_id)?;
    uuid(&decision.operator.actor.0)?;
    uuid(&review.operator.actor.0)?;
    uuid(&evidence.id.0)?;
    uuid(&evidence.extracted_by.0)?;
    for text in [
        &review.proposal_digest.0,
        &review.human_proof_digest.0,
        &review.basis.evidence_digest.0,
        &review.basis.options_digest.0,
        &review.basis.effects_digest.0,
        &decision.proof_digest.0,
        &decision.policy_digest.0,
        &decision.statement_digest.0,
    ] {
        hash(text)?;
    }
    if review.operator != decision.operator
        || review.recorded_at != decision.recorded_at
        || review.human_proof_digest != decision.proof_digest
        || review.evidence_id != evidence.id
        || evidence.content_hash != decision.statement_object_hash
    {
        return Err(invalid("inconsistent review projection"));
    }
    let mut result = BTreeMap::new();
    for (encoded, address) in [
        (&record.proof, &decision.proof_object_hash.0),
        (&record.policy, &decision.policy_object_hash.0),
        (&record.statement.payload, &decision.statement_object_hash.0),
    ] {
        let bytes = ekr_core::bytes::decode(encoded).map_err(invalid)?;
        let address = hash(address)?;
        if ContentHash::of_bytes(&bytes) != address {
            return Err(invalid("object hash mismatch"));
        }
        result.insert(address, bytes);
    }
    Ok(result)
}
fn identity_keys(record: &EkrIntegrateRetainedProposalReview) -> [String; 2] {
    [
        format!("review-{}", record.review.review_id.0),
        format!("decision-{}", record.decision.decision_id),
    ]
}

impl<S: EventStore> EventlogStore<S> {
    fn review_identity(
        &self,
        key: &str,
    ) -> Result<Option<EkrIntegrateRetainedProposalReview>, StoreError> {
        let stream = StreamId::new(self.tenant.clone(), IDENTITIES, key)?;
        let events = self.read_all(&stream, MAX_READ_LIMIT)?;
        match events.as_slice() {
            [] => Ok(None),
            [event] if event.name == BOUND && event.schema_version == 1 => {
                serde_json::from_value(event.data.clone())
                    .map(Some)
                    .map_err(json_error)
            }
            _ => Err(invalid("identity envelope")),
        }
    }
}

impl<S: EventStore> EventlogStore<S> {
    pub(super) fn read_proposal_reviews(
        &self,
        proposal_id: &EkrIntegrateSchemaProposalId,
    ) -> Result<Vec<EkrIntegrateRetainedProposalReview>, StoreError> {
        use ekr_core::contract_data::EkrIntegrateProposalCoordinationRecord as Record;
        Ok(self
            .read_proposal_coordination(proposal_id)?
            .entries
            .into_iter()
            .filter_map(|entry| match *entry.record {
                Record::V1(record) => Some(*record.value),
                Record::V0(_) => None,
            })
            .collect())
    }
    pub(super) fn read_proposal_coordination(
        &self,
        proposal_id: &EkrIntegrateSchemaProposalId,
    ) -> Result<ekr_core::contract_data::EkrIntegrateProposalCoordinationRead, StoreError> {
        self.entered()?;
        SchemaProposalId::parse_identity(&proposal_id.0).map_err(invalid)?;
        let proposal = self
            .read_schema_proposals_selected(Some(&proposal_id.0))?
            .into_iter()
            .find(|record| *record.proposal.proposal_id == *proposal_id)
            .ok_or_else(|| invalid("unknown proposal"))?;
        let stream = StreamId::new(self.tenant.clone(), STREAM, &proposal_id.0)?;
        self.read_proposal_coordination_events(
            proposal_id,
            &proposal,
            self.read_all(&stream, MAX_READ_LIMIT)?,
        )
    }
    pub(super) fn read_proposal_coordination_events(
        &self,
        proposal_id: &EkrIntegrateSchemaProposalId,
        proposal: &ekr_core::contract_data::EkrIntegrateRetainedSchemaProposal,
        events: Vec<RecordedEvent>,
    ) -> Result<ekr_core::contract_data::EkrIntegrateProposalCoordinationRead, StoreError> {
        use ekr_core::contract_data as w;
        let mut identities = BTreeSet::new();
        let mut proofs = BTreeSet::new();
        let mut records = Vec::new();
        let mut native_ids = BTreeSet::new();
        for event in events {
            if event.tenant != self.tenant
                || event.stream_type != STREAM
                || event.stream_id != proposal_id.0
                || event.is_redacted()
                || !native_ids.insert(event.event_id.clone())
                || event.version != records.len() as u64 + 1
            {
                return Err(invalid("physical cursor discontinuity"));
            }
            if event.name == applications::MARKER && event.schema_version == 1 {
                let record: w::EkrIntegrateApplicationPublicationRecord =
                    serde_json::from_value(event.data).map_err(json_error)?;
                let guard = ekr_graph::events::ApplicationGuard::try_from((*record.guard).clone())
                    .map_err(invalid)?;
                uuid(&record.event_id.0)?;
                hash(&record.record_hash.0)?;
                if record.guard.proposal_id.as_ref() != proposal_id
                    || record.guard.proposal_digest != proposal.proposal_digest
                    || record.transaction_id != record.guard.attempt_transaction
                    || guard.as_data().review_stream_version.as_u64() != Some(event.version - 1)
                {
                    return Err(invalid("application marker identity or cursor"));
                }
                records.push(Box::new(w::EkrIntegrateProposalCoordinationEntry {
                    stream_version: event.version.into(),
                    record: Box::new(w::EkrIntegrateProposalCoordinationRecord::V0(
                        w::EkrIntegrateProposalCoordinationRecordVariant0 {
                            kind: w::EkrIntegrateProposalCoordinationRecordVariant0Kind::V0,
                            value: Box::new(record),
                        },
                    )),
                }));
                continue;
            }
            if event.name != RECORDED || ![1, 2].contains(&event.schema_version) {
                return Err(invalid("review envelope"));
            }
            let record: EkrIntegrateRetainedProposalReview =
                serde_json::from_value(event.data).map_err(json_error)?;
            if *record.review.proposal_id != *proposal_id
                || record.review.proposal_digest != proposal.proposal_digest
            {
                return Err(invalid("proposal mismatch"));
            }
            let keys = identity_keys(&record);
            for key in keys
                .iter()
                .take(if event.schema_version == 1 { 2 } else { 1 })
            {
                if !identities.insert(key.clone()) {
                    return Err(invalid("duplicate identity"));
                }
                if self.review_identity(key)?.as_ref() != Some(&record) {
                    return Err(invalid("identity binding mismatch"));
                }
            }
            if event.schema_version == 2
                && self.human_binding(&record.decision.decision_id)?.as_ref()
                    != Some(&*record.decision)
            {
                return Err(invalid("shared human-decision-binding mismatch"));
            }
            if !proofs.insert(record.decision.proof_digest.0.clone()) {
                return Err(invalid("duplicate proof digest"));
            }
            for (address, bytes) in payloads(&record)? {
                let held = self
                    .object(address)?
                    .ok_or_else(|| invalid("missing pinned bytes"))?;
                if *held.bytes != bytes
                    || held.metadata.storage_class.retention_rank()
                        < StorageClass::Provenance.retention_rank()
                {
                    return Err(invalid("changed or unpinned bytes"));
                }
            }
            records.push(Box::new(w::EkrIntegrateProposalCoordinationEntry {
                stream_version: event.version.into(),
                record: Box::new(w::EkrIntegrateProposalCoordinationRecord::V1(
                    w::EkrIntegrateProposalCoordinationRecordVariant1 {
                        kind: w::EkrIntegrateProposalCoordinationRecordVariant1Kind::V0,
                        value: Box::new(record),
                    },
                )),
            }));
        }
        Ok(w::EkrIntegrateProposalCoordinationRead {
            proposal_id: Box::new(proposal_id.clone()),
            stream_version: (records.len() as u64).into(),
            entries: records,
        })
    }
}
impl<S: AtomicBlobEventStore> ProposalReviewRetention for EventlogStore<S> {
    fn retained_proposal_reviews(
        &self,
        id: &EkrIntegrateSchemaProposalId,
    ) -> Result<Vec<EkrIntegrateRetainedProposalReview>, StoreError> {
        self.read_proposal_reviews(id)
    }
    fn proposal_coordination(
        &self,
        id: &EkrIntegrateSchemaProposalId,
    ) -> Result<ekr_core::contract_data::EkrIntegrateProposalCoordinationRead, StoreError> {
        self.read_proposal_coordination(id)
    }
    fn retain_proposal_review(
        &self,
        record: &EkrIntegrateRetainedProposalReview,
        expected_previous: Option<ContentHash>,
        at: Timestamp,
    ) -> Result<(EkrIntegrateRetainedProposalReview, bool), StoreError> {
        self.entered()?;
        let payloads = payloads(record)?;
        let proposal_id = &record.review.proposal_id;
        // Retained proposals are immutable; observing this exact binding needs no mutable-head CAS.
        if !self.read_schema_proposals()?.iter().any(|proposal| {
            proposal.proposal.proposal_id == *proposal_id
                && proposal.proposal_digest == record.review.proposal_digest
        }) {
            return Err(invalid("unknown proposal or changed digest"));
        }
        for _ in 0..16 {
            let coordination = self.read_proposal_coordination(proposal_id)?;
            let held = self.retained_proposal_reviews(proposal_id)?;
            if let Some(previous) = held.iter().find(|previous| {
                previous.review.review_id == record.review.review_id
                    || previous.decision.decision_id == record.decision.decision_id
                    || previous.decision.proof_digest == record.decision.proof_digest
            }) {
                return if previous == record {
                    Ok((previous.clone(), false))
                } else {
                    Err(StoreError::PublicationInputConflict)
                };
            }
            let keys = identity_keys(record);
            for key in &keys {
                if self.review_identity(key)?.is_some() {
                    return Err(StoreError::PublicationInputConflict);
                }
            }
            let binding = self.new_human_binding(&record.decision)?;
            let latest = held
                .last()
                .map(|previous| hash(&previous.decision.proof_digest.0))
                .transpose()?;
            if latest != expected_previous {
                return Err(StoreError::Conflict);
            }
            let data = serde_json::to_value(record).map_err(json_error)?;
            let mut appends = vec![StreamAppend {
                stream: StreamId::new(self.tenant.clone(), STREAM, &proposal_id.0)?,
                expected: if coordination.entries.is_empty() {
                    Expected::NoStream
                } else {
                    Expected::Exact(coordination.entries.len() as u64)
                },
                events: vec![NewEvent::new(RECORDED, 2, data.clone())?],
            }];
            appends.push(binding);
            for key in keys.iter().take(1) {
                appends.push(StreamAppend {
                    stream: StreamId::new(self.tenant.clone(), IDENTITIES, key)?,
                    expected: Expected::NoStream,
                    events: vec![NewEvent::new(BOUND, 1, data.clone())?],
                });
            }
            let mut blobs = Vec::new();
            for (address, bytes) in &payloads {
                let object = crate::PublicationObject {
                    bytes: bytes.clone(),
                    storage_class: StorageClass::Provenance,
                    stored_at: at,
                };
                if let Some(append) = self.object_append(*address, &object)? {
                    appends.push(append);
                }
                blobs.push(BlobWrite {
                    digest: address.to_hex(),
                    bytes: bytes.clone(),
                });
            }
            let mut meta = envelope(
                "ekr.proposal-review.request",
                record.decision.proof_digest.0.clone(),
            );
            meta.occurred_at =
                OffsetDateTime::from_unix_timestamp_nanos(i128::from(at.millis()) * 1_000_000)
                    .map_err(invalid)?;
            let mut request = BlobAppendGroup {
                group: AppendGroup {
                    tenant: self.tenant.clone(),
                    appends,
                    meta,
                },
                blobs,
            };
            let key = format!("ekr.proposal-review.{}", request.fingerprint()?);
            request.group.meta.idempotency_key = key.clone();
            request.group.meta.request_id = key.clone();
            request.group.meta.trace_id = key;
            match self.atomic(&request) {
                Ok(result) => return Ok((record.clone(), !result.deduplicated)),
                Err(StoreError::Conflict) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(StoreError::Conflict)
    }
}
