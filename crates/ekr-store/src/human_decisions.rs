//! Shared physical identity bindings for human decisions of every operation kind.
//!
//! New signed canonical envelopes and review events bind the generated record atomically.
//! Legacy signed history is immutable: current writers refuse old-format publication and pending
//! legacy recovery. Mixed old/new writer binaries are unsupported during this format transition.
//! No method here authenticates a proof, grants approval or creates a canonical publication.
use super::*;
use ekr_core::contract_data::{EkrIntegrateRetainedProposalReview, EkrKernelHumanDecisionRecord};

pub(super) const STREAM: &str = "ekr.kernel.human-decisions";
const BOUND: &str = "ekr.kernel.HumanDecisionBound";

/// Physical lookup only; kernel replay authenticates the returned decision and its audience.
/// The tenant and immutable canonical seed delimit the audience of this store. Protocol digests
/// are distinct from object hashes and are authenticated by the kernel, not by this index.
pub trait HumanDecisionRetention {
    /// Reads a shared binding or historical reservation without changing any retained bytes.
    /// # Errors
    /// Malformed identity, corrupt binding or conflicting historical reservations.
    fn human_decision(
        &self,
        decision_id: &str,
    ) -> Result<Option<EkrKernelHumanDecisionRecord>, StoreError>;
}
fn invalid(reason: impl std::fmt::Display) -> StoreError {
    StoreError::Document(format!("human-decision-binding: {reason}"))
}
fn identity(id: &str) -> Result<(), StoreError> {
    id.parse::<ekr_core::AgentId>().map(|_| ()).map_err(invalid)
}
fn checked(
    record: EkrKernelHumanDecisionRecord,
) -> Result<EkrKernelHumanDecisionRecord, StoreError> {
    identity(&record.decision_id)?;
    identity(&record.operator.actor.0)?;
    for hash in [
        &record.proof_digest,
        &record.policy_digest,
        &record.statement_digest,
        &record.proof_object_hash,
        &record.policy_object_hash,
        &record.statement_object_hash,
    ] {
        hash.0.parse::<ContentHash>().map_err(invalid)?;
    }
    Ok(record)
}
/// Only the generated nested physical record is interpreted here. Kernel authority validates the
/// complete surrounding transition/answer document before any canonical publication is admitted.
pub(super) fn decision_record(
    event: &RevisionEvent,
    bytes: &[u8],
) -> Result<Option<EkrKernelHumanDecisionRecord>, StoreError> {
    if !event.is_human_decision() {
        return Ok(None);
    }
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(json_error)?;
    let record = value
        .get("review")
        .ok_or_else(|| invalid("missing review record"))?;
    checked(serde_json::from_value(record.clone()).map_err(json_error)?).map(Some)
}
fn merge(
    found: &mut Option<EkrKernelHumanDecisionRecord>,
    next: EkrKernelHumanDecisionRecord,
) -> Result<(), StoreError> {
    if found.as_ref().is_some_and(|prior| prior != &next) {
        return Err(StoreError::PublicationInputConflict);
    }
    *found = Some(next);
    Ok(())
}
impl<S: EventStore> EventlogStore<S> {
    pub(super) fn human_binding(
        &self,
        decision_id: &str,
    ) -> Result<Option<EkrKernelHumanDecisionRecord>, StoreError> {
        identity(decision_id)?;
        let events = self.read_all(
            &StreamId::new(self.tenant.clone(), STREAM, decision_id)?,
            MAX_READ_LIMIT,
        )?;
        match events.as_slice() {
            [] => Ok(None),
            [event] if event.name == BOUND && event.schema_version == 1 => {
                let record =
                    checked(serde_json::from_value(event.data.clone()).map_err(json_error)?)?;
                if record.decision_id != decision_id {
                    return Err(invalid("identity mismatch"));
                }
                Ok(Some(record))
            }
            _ => Err(invalid("envelope")),
        }
    }
}
impl<S: AtomicBlobEventStore> EventlogStore<S> {
    pub(super) fn binding_append(
        &self,
        record: &EkrKernelHumanDecisionRecord,
    ) -> Result<StreamAppend, StoreError> {
        checked(record.clone())?;
        Ok(StreamAppend {
            stream: StreamId::new(self.tenant.clone(), STREAM, &record.decision_id)?,
            expected: Expected::NoStream,
            events: vec![NewEvent::new(
                BOUND,
                1,
                serde_json::to_value(record).map_err(json_error)?,
            )?],
        })
    }
    pub(super) fn new_human_binding(
        &self,
        record: &EkrKernelHumanDecisionRecord,
    ) -> Result<StreamAppend, StoreError> {
        if self.human_decision(&record.decision_id)?.is_some() {
            return Err(StoreError::PublicationInputConflict);
        }
        self.binding_append(record)
    }
    pub(super) fn publication_decision(
        &self,
        publication: &Publication,
    ) -> Result<Option<EkrKernelHumanDecisionRecord>, StoreError> {
        if !publication.event.is_human_decision() {
            return Ok(None);
        }
        let record = publication
            .objects
            .get(&publication.event.record_hash)
            .ok_or_else(|| invalid("missing publication record"))?;
        if ContentHash::of_bytes(&record.bytes) != publication.event.record_hash {
            return Err(invalid("publication record hash"));
        }
        decision_record(&publication.event, &record.bytes)
    }
}
impl<S: EventStore> EventlogStore<S> {
    pub(super) fn require_human_bindings(
        &self,
        history: &RetainedHistory,
    ) -> Result<(), StoreError> {
        for occurrence in &history.occurrences {
            if occurrence.event.requires_human_binding() {
                let record = decision_record(
                    &occurrence.event,
                    history.content(occurrence.event.record_hash, StorageClass::Canonical)?,
                )?
                .ok_or_else(|| invalid("missing signed record"))?;
                if self.human_binding(&record.decision_id)?.as_ref() != Some(&record) {
                    return Err(invalid("missing or changed index"));
                }
            }
        }
        Ok(())
    }
}
impl<S: AtomicBlobEventStore> HumanDecisionRetention for EventlogStore<S> {
    fn human_decision(
        &self,
        decision_id: &str,
    ) -> Result<Option<EkrKernelHumanDecisionRecord>, StoreError> {
        self.entered()?;
        let mut found = self.human_binding(decision_id)?;
        // Historical signed occurrences predate the shared index. Their ids remain reserved,
        // while their original replay rule never acquires a retroactive index requirement.
        for occurrence in self.occurrences(MAX_READ_LIMIT, None)? {
            if !occurrence.event.is_human_decision() {
                continue;
            }
            let object = self
                .object(occurrence.event.record_hash)?
                .ok_or_else(|| invalid("missing historical record"))?;
            if object.metadata.storage_class.retention_rank()
                < StorageClass::Canonical.retention_rank()
            {
                return Err(invalid("historical record retention"));
            }
            if let Some(record) = decision_record(&occurrence.event, &object.bytes)? {
                if occurrence.event.requires_human_binding()
                    && self.human_binding(&record.decision_id)?.as_ref() != Some(&record)
                {
                    return Err(invalid("missing or changed index"));
                }
                if record.decision_id == decision_id {
                    merge(&mut found, record)?;
                }
            }
        }
        // v1 proposal reviews already have a globally unique decision singleton. Read it without
        // calling the review reader (which checks this shared binding for v2 and would recurse).
        let stream = StreamId::new(
            self.tenant.clone(),
            "ekr.integrate.proposal-review-identities",
            format!("decision-{decision_id}"),
        )?;
        let legacy = self.read_all(&stream, MAX_READ_LIMIT)?;
        match legacy.as_slice() {
            [] => {}
            [event]
                if event.name == "ekr.integrate.ProposalReviewIdentityBound"
                    && event.schema_version == 1 =>
            {
                let record: EkrIntegrateRetainedProposalReview =
                    serde_json::from_value(event.data.clone()).map_err(json_error)?;
                if record.decision.decision_id != decision_id {
                    return Err(invalid("legacy decision identity mismatch"));
                }
                merge(&mut found, checked(*record.decision)?)?;
            }
            _ => return Err(invalid("legacy decision envelope")),
        }
        Ok(found)
    }
}
