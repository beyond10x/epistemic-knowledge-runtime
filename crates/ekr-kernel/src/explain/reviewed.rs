//! Read projections of already replay-verified human publications, using generated records.
use super::{require, unverified, ProjectionError};
use crate::{GraphTransaction, VerifiedRead};
use ekr_core::contract_data::{
    EkrKernelAuthorityTransitionRecord, EkrKernelContentHash, EkrKernelExplainedAnswer,
    EkrKernelHumanAnswerRecord,
};
use ekr_core::{ContentHash, RevisionId, RevisionNumber};
use ekr_graph::Root;

fn convert<T: serde::de::DeserializeOwned>(value: &impl serde::Serialize) -> Option<T> {
    serde_json::from_value(serde_json::to_value(value).ok()?).ok()
}

pub(super) fn root(bytes: &[u8]) -> Option<(Root, RevisionId)> {
    if let Ok(answer) = serde_json::from_slice::<EkrKernelHumanAnswerRecord>(bytes) {
        Some((convert(&answer.result)?, answer.revision_id.0.parse().ok()?))
    } else {
        let transition: EkrKernelAuthorityTransitionRecord = serde_json::from_slice(bytes).ok()?;
        Some((
            convert(&transition.result)?,
            transition.revision_id.0.parse().ok()?,
        ))
    }
}

impl VerifiedRead {
    pub(super) fn explained_answer(
        &self,
        revision: RevisionNumber,
    ) -> Result<(EkrKernelExplainedAnswer, GraphTransaction), ProjectionError> {
        let Some(answer) = self.answers.get(&revision) else {
            return unverified("answer-record-missing");
        };
        let Some(coordinate) = self.revisions.get(&revision) else {
            return unverified("answer-coordinate-missing");
        };
        require(revision <= self.root.revision, "answer-after-capture")?;
        let retained =
            self.content(&coordinate.record_hash)
                .ok_or_else(|| ProjectionError::Unverified {
                    code: "answer-record-missing".into(),
                })?;
        require(
            ContentHash::of_bytes(retained) == coordinate.record_hash,
            "answer-record-address",
        )?;
        let decoded: Option<EkrKernelHumanAnswerRecord> = serde_json::from_slice(retained).ok();
        require(decoded.as_ref() == Some(answer), "answer-record-disagrees")?;
        require(
            convert::<Root>(&answer.result) == Some(coordinate.root)
                && answer.revision_id.0 == coordinate.revision_id.to_string()
                && answer.event_id.0 == coordinate.event_id.to_string()
                && crate::incubation_document::timestamp_value(&answer.review.recorded_at).ok()
                    == Some(coordinate.committed_at),
            "answer-coordinate-disagrees",
        )?;
        let address: ContentHash =
            answer
                .transaction_object_hash
                .0
                .parse()
                .map_err(|_| ProjectionError::Unverified {
                    code: "answer-transaction-address".into(),
                })?;
        let bytes = self
            .content(&address)
            .ok_or_else(|| ProjectionError::Unverified {
                code: "answer-transaction-missing".into(),
            })?;
        require(
            ContentHash::of_bytes(bytes) == address,
            "answer-transaction-address",
        )?;
        let transaction: GraphTransaction =
            serde_json::from_slice(bytes).map_err(|_| ProjectionError::Unverified {
                code: "answer-transaction-invalid".into(),
            })?;
        require(
            transaction.id.to_string() == answer.transaction_id.0,
            "answer-transaction-disagrees",
        )?;
        Ok((
            EkrKernelExplainedAnswer {
                record: Box::new(answer.clone()),
                record_hash: Box::new(EkrKernelContentHash(coordinate.record_hash.to_string())),
            },
            transaction,
        ))
    }
}
