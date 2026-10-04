//! Explanation of actual guarded schema corrections from immutable captured provenance.
use super::{indexed, require, unverified, ProjectionError};
use crate::{GraphOperation, GraphTransaction, VerifiedRead};
use ekr_core::{contract_data as w, ContentHash, EvidenceId, TransactionId};

fn invalid() -> ProjectionError {
    ProjectionError::Unverified {
        code: "schema-correction-provenance".into(),
    }
}

impl VerifiedRead {
    pub(super) fn explained_schema_correction(
        &self,
        correction: &w::EkrKernelExplainedSchemaCorrection,
    ) -> Result<(GraphTransaction, EvidenceId), ProjectionError> {
        let publication = &correction.publication;
        let guard = &publication.guard;
        let step = &correction.step;
        let review = &correction.review;
        let origin = &correction.original_review;
        let proposal = &correction.proposal;
        let id: TransactionId = publication
            .transaction_id
            .0
            .parse()
            .map_err(|_| invalid())?;
        let c = self
            .transactions
            .get(&id)
            .and_then(indexed)
            .ok_or_else(invalid)?;
        let (_, record_hash) = self.verify(&c)?;
        require(
            c.revision <= self.root.revision
                && record_hash.to_string() == publication.record_hash.0
                && c.receipt.event_id.to_string() == publication.event_id.0
                && guard.attempt_transaction == publication.transaction_id
                && guard.step_election_id == step.step_election_id
                && guard.step == step.step
                && guard.application_id == step.application_id
                && guard.review_id == review.review.review_id
                && guard.human_proof_digest == review.review.human_proof_digest
                && guard.proposal_id == proposal.proposal.proposal_id
                && guard.proposal_digest == proposal.proposal_digest,
            "schema-correction-provenance",
        )?;
        require(
            matches!(*step.step.kind, w::EkrIntegrateApplicationStepKind::V0)
                && matches!(*review.review.decision, w::EkrIntegrateReviewDecision::V0)
                && matches!(*origin.review.decision, w::EkrIntegrateReviewDecision::V0)
                && step.correction_review_id
                    == w::EssPresence::Present(origin.review.review_id.clone()),
            "schema-correction-provenance",
        )?;
        for (address, encoded) in [
            (&proposal.proposal_digest.0, &proposal.payload),
            (&origin.decision.proof_object_hash.0, &origin.proof),
            (&origin.decision.policy_object_hash.0, &origin.policy),
            (
                &origin.decision.statement_object_hash.0,
                &origin.statement.payload,
            ),
            (&review.decision.proof_object_hash.0, &review.proof),
            (&review.decision.policy_object_hash.0, &review.policy),
            (
                &review.decision.statement_object_hash.0,
                &review.statement.payload,
            ),
        ] {
            let hash: ContentHash = address.parse().map_err(|_| invalid())?;
            let bytes = ekr_core::bytes::decode(encoded).map_err(|_| invalid())?;
            require(
                ContentHash::of_bytes(&bytes) == hash
                    && self.content(&hash) == Some(bytes.as_slice()),
                "schema-correction-provenance",
            )?;
        }
        let payload = ekr_core::bytes::decode(&proposal.payload).map_err(|_| invalid())?;
        let decoded: w::EkrIntegrateSchemaProposalDocument =
            serde_json::from_slice(&payload).map_err(|_| invalid())?;
        require(
            decoded == *proposal.proposal,
            "schema-correction-provenance",
        )?;
        let transaction = c.transaction()?;
        let mut frozen =
            crate::application_transaction::decode(&step.transaction).map_err(|_| invalid())?;
        frozen.id = id;
        let actual: GraphTransaction<ekr_graph::CanonicalValue> =
            transaction.clone().try_into().map_err(|_| invalid())?;
        require(frozen == actual, "schema-correction-provenance")?;

        // Use the checked generated/native codec to compare the complete real statement record.
        let mut statement = *step.transaction.clone();
        statement.operations = vec![Box::new(w::EkrKernelCanonicalOperationProjection::V2(
            w::EkrKernelCanonicalOperationProjectionVariant2 {
                kind: w::EkrKernelCanonicalOperationProjectionVariant2Kind::V0,
                value: origin.statement.clone(),
            },
        ))];
        let decoded = crate::application_transaction::decode(&statement).map_err(|_| invalid())?;
        let [GraphOperation::AddEvidence(addition)] = decoded.operations.as_slice() else {
            return unverified("schema-correction-provenance");
        };
        require(
            self.graph.evidence.get(&addition.evidence.id) == Some(&addition.evidence)
                && self.content(&addition.evidence.content_hash)
                    == Some(addition.payload.as_slice()),
            "schema-correction-provenance",
        )?;
        Ok((transaction, addition.evidence.id))
    }
}
