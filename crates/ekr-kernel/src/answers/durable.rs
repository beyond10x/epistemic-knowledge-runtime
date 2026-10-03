//! Retained generated answer records and their independently replayed effects.
use super::{error, reviewed};
use crate::human_review as review;
use crate::replay::{self, ReplayState, Revision};
use crate::{KernelAuthority, ValidatedTransaction};
use ekr_core::contract_data::*;
use ekr_core::contracts::{graph as g, kernel as m, primitives::Uuid};
use ekr_core::{ContentHash, EventId, RevisionId, Timestamp};
use ekr_graph::{CanonicalGraph, RevisionPayload, Root};
use ekr_store::{RecordedOccurrence, RetainedHistory, StorageClass, StoreError};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::{BTreeMap, BTreeSet};
type AnswerState = (
    EkrKernelHumanAnswerRecord,
    CanonicalGraph,
    Root,
    BTreeMap<ekr_core::AssertionId, BTreeSet<ekr_core::AgentId>>,
);
use std::sync::Arc;

pub(super) fn bytes(value: &impl Serialize) -> Result<Vec<u8>, StoreError> {
    serde_json::to_vec(value).map_err(error)
}
pub(super) fn decode<T: DeserializeOwned>(value: &[u8]) -> Result<T, StoreError> {
    serde_json::from_slice(value).map_err(error)
}
fn hash(value: ContentHash) -> Box<EkrKernelContentHash> {
    Box::new(EkrKernelContentHash(value.to_string()))
}
fn parsed(value: &EkrKernelContentHash) -> Result<ContentHash, StoreError> {
    value.0.parse().map_err(error)
}
fn presence<T>(value: Option<T>) -> EssPresence<T> {
    value.map_or(EssPresence::Absent, EssPresence::Present)
}
fn optional<T>(value: &EssPresence<T>) -> Option<&T> {
    match value {
        EssPresence::Absent => None,
        EssPresence::Present(value) => Some(value),
    }
}
fn basis(value: &m::ReviewBasis) -> EkrKernelReviewBasis {
    EkrKernelReviewBasis {
        observed_revision: Box::new(EkrKernelRevisionNumber(value.observed_revision.0.into())),
        evidence_digest: Box::new(EkrKernelContentHash(value.evidence_digest.0.clone())),
        options_digest: Box::new(EkrKernelContentHash(value.options_digest.0.clone())),
        effects_digest: Box::new(EkrKernelContentHash(value.effects_digest.0.clone())),
    }
}
fn correction(value: &m::ClaimCorrection) -> EkrKernelClaimCorrection {
    EkrKernelClaimCorrection {
        kind: Box::new(match value.kind {
            m::ClaimCorrectionKind::Choose => EkrKernelClaimCorrectionKind::V0,
            m::ClaimCorrectionKind::CorrectTime => EkrKernelClaimCorrectionKind::V1,
            m::ClaimCorrectionKind::Retract => EkrKernelClaimCorrectionKind::V2,
            m::ClaimCorrectionKind::Unresolved => EkrKernelClaimCorrectionKind::V3,
        }),
        assertion_id: Box::new(EkrGraphAssertionId(value.assertion_id.0 .0.clone())),
        reason: value.reason.clone(),
        valid_from: presence(value.valid_from.as_ref().map(|v| v.0.clone())),
        valid_to: presence(value.valid_to.as_ref().map(|v| v.0.clone())),
    }
}
pub(super) fn replacements(record: &EkrKernelHumanAnswerRecord) -> Vec<m::ClaimReplacement> {
    record
        .replacements
        .iter()
        .map(|value| m::ClaimReplacement {
            previous: g::AssertionId(Uuid(value.previous.0.clone())),
            replacement: g::AssertionId(Uuid(value.replacement.0.clone())),
        })
        .collect()
}
pub(super) fn input(
    history: &RetainedHistory,
    record: &EkrKernelHumanAnswerRecord,
) -> Result<m::AttentionAnswerApplication, StoreError> {
    let proof = reviewed(review::read_proof(history.content(
        parsed(&record.review.proof_object_hash)?,
        StorageClass::Canonical,
    )?))?;
    let m::HumanDecisionTarget::AnswerAttention(target) = &proof.intent.target else {
        return Err(error("review-target"));
    };
    // The signed target is authoritative; record fields are compared with a fresh derived record.
    Ok(m::AttentionAnswerApplication {
        dispute_id: target.dispute_id.clone(),
        basis: target.basis.clone(),
        corrections: record
            .corrections
            .iter()
            .map(|v| review::correction_from_document(v))
            .collect(),
        statement: history
            .content(
                parsed(&record.review.statement_object_hash)?,
                StorageClass::Canonical,
            )?
            .to_vec(),
        human_proof: proof,
    })
}
pub(super) fn payload(
    record: &EkrKernelHumanAnswerRecord,
    root: &Root,
) -> Result<RevisionPayload, StoreError> {
    Ok(RevisionPayload::AttentionAnswered(
        ekr_graph::AnswerOccurrence::try_from(EkrKernelAttentionAnsweredPayload {
            answer_id: record.answer_id.clone(),
            transaction_id: record.transaction_id.clone(),
            revision_id: record.revision_id.clone(),
            number: Box::new(EkrKernelRevisionNumber(root.revision.get().into())),
            knowledge_root: hash(root.knowledge_root),
        })
        .map_err(error)?,
    ))
}
/// Construct the expected generated record from the privately sealed operation set.
#[allow(clippy::too_many_arguments)]
pub(super) fn record(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    input: &m::AttentionAnswerApplication,
    replacements: &[m::ClaimReplacement],
    validated: &ValidatedTransaction,
    proof: &review::VerifiedDecision,
    identities: (EventId, RevisionId, ekr_core::EvidenceId),
    at: Timestamp,
) -> Result<AnswerState, StoreError> {
    let validators = BTreeSet::from([authority.context.validator]);
    let (mut graph, mut root) = crate::apply::apply(state.head(), validated, &validators, at)?;
    let mut assessment_validators = state.assessment_validators.clone();
    crate::disputes::recompute(&mut graph, &mut assessment_validators)?;
    root.knowledge_root = ekr_store::knowledge_root(&graph);
    let old_items =
        crate::attention::disputes_at(state.head().graph()?, state.head().root.revision, |hash| {
            history.content(*hash, StorageClass::Provenance)
        })?;
    let old = old_items
        .iter()
        .find(|item| {
            optional(&item.subject.dispute_id).is_some_and(|id| id.0 == input.dispute_id.0 .0)
        })
        .ok_or_else(|| error("answer-question-missing"))?;
    let affected: BTreeSet<_> = old
        .claims
        .iter()
        .map(|id| id.0.as_str())
        .chain(
            replacements
                .iter()
                .map(|pair| pair.replacement.0 .0.as_str()),
        )
        .collect();
    let statement_hash = ContentHash::of_bytes(&input.statement);
    let remaining = crate::attention::disputes_at(&graph, root.revision, |hash| {
        if *hash == statement_hash {
            Ok(input.statement.as_slice())
        } else {
            history.content(*hash, StorageClass::Provenance)
        }
    })?
    .into_iter()
    .filter(|item| {
        item.claims
            .iter()
            .any(|id| affected.contains(id.0.as_str()))
    })
    .map(Box::new)
    .collect::<Vec<_>>();
    let outcome = if input
        .corrections
        .iter()
        .all(|v| v.kind == m::ClaimCorrectionKind::Unresolved)
    {
        EkrKernelAnswerOutcome::V3
    } else if remaining.is_empty() {
        EkrKernelAnswerOutcome::V2
    } else {
        EkrKernelAnswerOutcome::V1
    };
    let (event_id, revision_id, evidence_id) = identities;
    let answer_id = Box::new(EkrKernelHumanAnswerId(
        input.human_proof.intent.decision_id.0.clone(),
    ));
    let transaction_id = Box::new(EkrKernelTransactionId(
        validated.transaction().id.to_string(),
    ));
    let statement_evidence = Box::new(EkrGraphEvidenceId(evidence_id.to_string()));
    let record = EkrKernelHumanAnswerRecord {
        format: Box::new(EkrKernelAnswerRecordFormat::V0),
        answer_id: answer_id.clone(),
        event_id: Box::new(EkrKernelEventId(event_id.to_string())),
        revision_id: Box::new(EkrKernelRevisionId(revision_id.to_string())),
        transaction_id: transaction_id.clone(),
        dispute_id: Box::new(EkrKernelDisputeId(input.dispute_id.0 .0.clone())),
        basis: Box::new(basis(&input.basis)),
        corrections: input
            .corrections
            .iter()
            .map(|v| Box::new(correction(v)))
            .collect(),
        replacements: replacements
            .iter()
            .map(|v| {
                Box::new(EkrKernelClaimReplacement {
                    previous: Box::new(EkrGraphAssertionId(v.previous.0 .0.clone())),
                    replacement: Box::new(EkrGraphAssertionId(v.replacement.0 .0.clone())),
                })
            })
            .collect(),
        statement_evidence: statement_evidence.clone(),
        transaction_object_hash: hash(ContentHash::of_bytes(&bytes(validated.transaction())?)),
        validation_hash: hash(validated.validation_hash()),
        validators: validators
            .iter()
            .map(|id| Box::new(EkrKernelAgentId(id.to_string())))
            .collect(),
        result: Box::new(decode(&bytes(&root)?)?),
        review: Box::new(crate::upgrade::review_record(proof, at)?),
        receipt: Box::new(EkrKernelAnswerReceipt {
            answer_id,
            evidence_id: statement_evidence,
            outcome: Box::new(outcome),
            remaining,
            transaction_id: EssPresence::Present(transaction_id),
            revision: EssPresence::Present(Box::new(EkrKernelRevisionNumber(
                root.revision.get().into(),
            ))),
        }),
    };
    Ok((record, graph, root, assessment_validators))
}

pub(crate) fn required(history: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
    let mut wanted = BTreeSet::new();
    for occurrence in &history.occurrences {
        if matches!(
            occurrence.event.payload,
            RevisionPayload::AttentionAnswered(_)
        ) {
            let record: EkrKernelHumanAnswerRecord =
                decode(history.content(occurrence.event.record_hash, StorageClass::Canonical)?)?;
            for value in [
                &record.review.proof_object_hash,
                &record.review.policy_object_hash,
                &record.review.statement_object_hash,
                &record.transaction_object_hash,
            ] {
                wanted.insert(parsed(value)?);
            }
        }
    }
    Ok(wanted)
}
pub(crate) fn replay_answer(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &mut ReplayState,
    occurrence: &RecordedOccurrence,
) -> Result<(), StoreError> {
    let retained = history.content(occurrence.event.record_hash, StorageClass::Canonical)?;
    let found: EkrKernelHumanAnswerRecord = decode(retained)?;
    replay::require(bytes(&found)? == retained, "answer-record-not-canonical")?;
    let input = input(history, &found)?;
    let replacements = replacements(&found);
    let at = crate::incubation_document::timestamp(&found.review.recorded_at)?;
    let transaction_id = found.transaction_id.0.parse().map_err(error)?;
    let evidence_id = found.statement_evidence.0.parse().map_err(error)?;
    let revision_id = found.revision_id.0.parse().map_err(error)?;
    let (validated, proof) = authority.validate_answer(
        history,
        state,
        &input,
        &replacements,
        (transaction_id, evidence_id, at),
    )?;
    replay::require(
        history.content(
            parsed(&found.transaction_object_hash)?,
            StorageClass::Canonical,
        )? == bytes(validated.transaction())?,
        "answer-transaction-disagrees",
    )?;
    let (expected, graph, root, assessment_validators) = record(
        authority,
        history,
        state,
        &input,
        &replacements,
        &validated,
        &proof,
        (occurrence.event.event_id, revision_id, evidence_id),
        at,
    )?;
    replay::require(
        bytes(&expected)? == retained
            && occurrence.event.payload == payload(&expected, &root)?
            && !state.revision_ids.contains(&revision_id),
        "answer-record-disagrees",
    )?;
    for (hash, payload) in crate::commands::added_payloads(validated.transaction()) {
        replay::require(
            history.content(hash, StorageClass::Provenance)? == payload.as_slice(),
            "answer-evidence-disagrees",
        )?;
    }
    state.assessment_validators = assessment_validators;
    let prior = state.head();
    let asserted_edges = prior.asserted_edges_after(validated.transaction());
    let graph_root = prior.graph_root;
    let ontology = Arc::clone(&prior.ontology);
    // Empty alias cache is a safe fresh projection; answer operations cannot change aliases.
    state.held.hold(root.revision, validated.transaction());
    state.revision_ids.insert(revision_id);
    state.answers.insert(root.revision, expected);
    state.revisions.insert(
        root.revision,
        Revision {
            root,
            revision_id,
            event_id: occurrence.event.event_id,
            record_hash: occurrence.event.record_hash,
            committed_at: at,
            graph_root,
            ontology,
            graph: Some(Arc::new(graph)),
            asserted_edges,
            alias_holders: Default::default(),
        },
    );
    Ok(())
}
