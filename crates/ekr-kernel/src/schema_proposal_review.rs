//! Human review admission and independent replay over immutable proposal material.
use crate::{human_review as h, schema_proposals::error, Commit};
use ekr_core::generated_identity::{Identity, ProposalReviewId};
use ekr_core::{
    contract_data as w, contracts::kernel as m, ContentHash, EvidenceId, RevisionNumber, Timestamp,
};
use ekr_store::{
    HumanDecisionRetention, IncubationRetention, ObjectStore, ObservationRetention,
    ProposalReviewRetention, RevisionLog, SchemaProposalRetention, StorageClass, StoreError,
};

fn checked<T>(value: Result<T, m::KnowledgeRefused>) -> Result<T, StoreError> {
    value.map_err(|e| error(format!("{}: {}", e.code, e.reason)))
}
fn target(
    id: &w::EkrIntegrateSchemaProposalId,
    digest: &w::EkrKernelContentHash,
    basis: &w::EkrKernelReviewBasis,
) -> Result<m::SchemaReviewTarget, StoreError> {
    Ok(m::SchemaReviewTarget {
        proposal_id: ekr_core::contracts::integrate::SchemaProposalId(
            ekr_core::contracts::primitives::Uuid(id.0.clone()),
        ),
        proposal_digest: m::ContentHash(digest.0.clone()),
        basis: checked(h::basis_from_document(basis))?,
    })
}
fn statement(
    proof: &h::VerifiedDecision,
    decision: &w::EkrKernelHumanDecisionRecord,
    id: Box<w::EkrGraphEvidenceId>,
) -> w::EkrKernelEvidenceAdditionProjection {
    use w::EssPresence::{Absent, Present};
    w::EkrKernelEvidenceAdditionProjection {
        evidence: Box::new(w::EkrGraphEvidenceRecord {
            id,
            content_hash: decision.statement_object_hash.clone(),
            extracted_by: decision.operator.actor.clone(),
            observed_at: decision.recorded_at,
            confidence_bp: 10000.into(),
            source: Box::new(w::EkrGraphEvidenceSourceProjection {
                kind: Box::new(w::EkrGraphEvidenceKind::V3),
                identity: Present(proof.operator().authentication_subject.clone()),
                assertion: Absent,
                database: Absent,
                document_id: Absent,
                key: Absent,
                observation: Absent,
                section: Absent,
                table: Absent,
                url: Absent,
            }),
        }),
        payload: ekr_core::bytes::encode(proof.statement()),
    }
}

/// Verify a retained signature against an already authenticated historical material capture.
/// The caller separately checks audience-wide decision identity and supplies the enrolled policy.
pub(crate) fn verify_captured_review(
    reviewer: &h::Reviewer,
    policy_bytes: &[u8],
    record: &w::EkrIntegrateRetainedProposalReview,
    projected: &w::EkrIntegrateSchemaProposalRead,
    read: &crate::VerifiedRead,
    previous: Option<ContentHash>,
) -> Result<h::VerifiedDecision, StoreError> {
    if ekr_core::bytes::decode(&record.policy).map_err(error)? != policy_bytes {
        return Err(error("review policy differs from enrolled policy"));
    }
    let snapshot = &record.review;
    if snapshot.proposal_id != projected.proposal.proposal_id
        || snapshot.proposal_digest != projected.proposal_digest
        || snapshot.basis != projected.basis
        || snapshot.operator != record.decision.operator
        || snapshot.human_proof_digest != record.decision.proof_digest
        || snapshot.recorded_at != record.decision.recorded_at
        || snapshot.basis.observed_revision.0.as_u64() != Some(read.root.revision.get())
    {
        return Err(error(
            "retained review differs from historical material or decision",
        ));
    }
    let current = target(
        &projected.proposal.proposal_id,
        &projected.proposal_digest,
        &projected.basis,
    )?;
    let approve = matches!(*snapshot.decision, w::EkrIntegrateReviewDecision::V0);
    if approve {
        super::schema_proposal_corrections::validate(read, &projected.proposal)?;
    }
    let proof = checked(h::read_proof(
        &ekr_core::bytes::decode(&record.proof).map_err(error)?,
    ))?;
    let bytes = ekr_core::bytes::decode(&record.statement.payload).map_err(error)?;
    let verified =
        checked(reviewer.verify_schema_proposal(&proof, &current, approve, &bytes, previous))?;
    let at = crate::incubation_document::timestamp_value(&record.decision.recorded_at)?;
    let expected = crate::upgrade::review_record(&verified, at)?;
    if *record.decision != expected
        || *record.statement != statement(&verified, &expected, snapshot.evidence_id.clone())
    {
        return Err(error(
            "retained review decision or statement differs from verified proof",
        ));
    }
    if !read
        .authority_at(read.root.revision)
        .agents
        .contains_key(&verified.operator().actor.0 .0.parse().map_err(error)?)
    {
        return Err(error("review operator is not registered"));
    }
    Ok(verified)
}

impl<
        S: RevisionLog
            + ObjectStore
            + ObservationRetention
            + IncubationRetention
            + SchemaProposalRetention
            + HumanDecisionRetention
            + ProposalReviewRetention,
    > Commit<S>
{
    /// Read authenticated proposal decisions with their retained human statement evidence.
    /// # Errors
    /// Unknown proposal, invalid retained decision or unavailable evidence.
    pub fn schema_proposal_reviews(
        &self,
        id: &w::EkrIntegrateSchemaProposalId,
    ) -> Result<Vec<w::EkrIntegrateRetainedProposalReview>, StoreError> {
        let proposal = self
            .store
            .retained_schema_proposals()?
            .into_iter()
            .find(|proposal| *proposal.proposal.proposal_id == *id)
            .ok_or_else(|| error("unknown proposal"))?;
        self.verified_proposal_reviews(&proposal)
    }

    /// Retain the exact externally signed approval without changing canonical knowledge.
    /// # Errors
    /// Missing trusted enrollment, invalid signature, changed material or predecessor, or storage failure.
    pub fn approve_schema_proposal(
        &self,
        input: &w::EkrIntegrateSchemaProposalReviewApplication,
        at: Timestamp,
    ) -> Result<w::EkrIntegrateProposalReviewSnapshot, StoreError> {
        crate::schema_review_behavior::approve(self, input, at)
    }
    /// Retain the exact externally signed rejection without changing canonical knowledge.
    /// # Errors
    /// Missing trusted enrollment, invalid signature, changed material or predecessor, or storage failure.
    pub fn reject_schema_proposal(
        &self,
        input: &w::EkrIntegrateSchemaProposalReviewApplication,
        at: Timestamp,
    ) -> Result<w::EkrIntegrateProposalReviewSnapshot, StoreError> {
        crate::schema_review_behavior::reject(self, input, at)
    }

    pub(super) fn verified_proposal_reviews(
        &self,
        proposal: &w::EkrIntegrateRetainedSchemaProposal,
    ) -> Result<Vec<w::EkrIntegrateRetainedProposalReview>, StoreError> {
        let records = self
            .store
            .retained_proposal_reviews(&proposal.proposal.proposal_id)?;
        if records.is_empty() {
            return Ok(records);
        }
        let (history, state) = self.replayed_state().map_err(error)?;
        let transition = state
            .transition
            .as_ref()
            .ok_or_else(|| error("authority-upgrade-required"))?;
        let enrolled: ContentHash = transition
            .review
            .policy_object_hash
            .0
            .parse()
            .map_err(error)?;
        let policy_bytes = history.content(enrolled, StorageClass::Canonical)?;
        let policy = checked(h::read_policy(policy_bytes))?;
        let reviewer = self.authority.reviewer(&state, &policy)?;
        let mut previous = None;
        for record in &records {
            if self
                .store
                .human_decision(&record.decision.decision_id)?
                .as_ref()
                != Some(&*record.decision)
            {
                return Err(error(
                    "review decision identity differs from its audience-wide binding",
                ));
            }
            let revision = record
                .review
                .basis
                .observed_revision
                .0
                .as_u64()
                .ok_or_else(|| error("review revision must be a nonnegative integer"))?;
            let read = self
                .read(Some(RevisionNumber::new(revision)))
                .map_err(error)?;
            let projected = self.project_schema_proposal_at(proposal, false, &read)?;
            previous = Some(
                verify_captured_review(
                    &reviewer,
                    policy_bytes,
                    record,
                    &projected,
                    &read,
                    previous,
                )?
                .proof_digest(),
            );
        }
        Ok(records)
    }

    pub(super) fn retain_schema_proposal_review(
        &self,
        input: &w::EkrIntegrateSchemaProposalReviewApplication,
        approve: bool,
        at: Timestamp,
    ) -> Result<w::EkrIntegrateProposalReviewSnapshot, StoreError> {
        let proof = checked(h::proof_from_document(&input.human_proof))?;
        let claimed = target(&input.proposal_id, &input.proposal_digest, &input.basis)?;
        let signed = if approve {
            m::HumanDecisionTarget::ApproveSchemaProposal(claimed)
        } else {
            m::HumanDecisionTarget::RejectSchemaProposal(claimed)
        };
        if proof.intent.target != signed {
            return Err(error("review input differs from signed target"));
        }
        let statement_bytes = ekr_core::bytes::decode(&input.statement).map_err(error)?;
        let proposal = self
            .store
            .retained_schema_proposals()?
            .into_iter()
            .find(|p| p.proposal.proposal_id == input.proposal_id)
            .ok_or_else(|| error("unknown proposal"))?;
        let records = self.verified_proposal_reviews(&proposal)?;
        // A retry returns its original decision, even after rejection or a later canonical revision.
        if let Some(record) = records
            .iter()
            .find(|r| r.decision.decision_id == proof.intent.decision_id.0)
        {
            if record.proof != ekr_core::bytes::encode(&checked(h::proof_bytes(&proof))?)
                || record.statement.payload != input.statement
            {
                return Err(StoreError::PublicationInputConflict);
            }
            return Ok(*record.review.clone());
        }
        let (history, state) = self.replayed_state().map_err(error)?;
        let transition = state
            .transition
            .as_ref()
            .ok_or_else(|| error("authority-upgrade-required"))?;
        if at < state.head().committed_at {
            return Err(error("review time precedes canonical head"));
        }
        let policy_hash = transition
            .review
            .policy_object_hash
            .0
            .parse()
            .map_err(error)?;
        let policy = checked(h::read_policy(
            history.content(policy_hash, StorageClass::Canonical)?,
        ))?;
        let reviewer = self.authority.reviewer(&state, &policy)?;
        let read = self.read(None).map_err(error)?;
        let shown = self.project_schema_proposal_at(&proposal, false, &read)?;
        if approve {
            super::schema_proposal_corrections::validate(&read, &proposal.proposal)?;
        }
        let current = target(
            &shown.proposal.proposal_id,
            &shown.proposal_digest,
            &shown.basis,
        )?;
        let previous = records
            .last()
            .map(|r| r.review.human_proof_digest.0.parse())
            .transpose()
            .map_err(error)?;
        let verified = checked(reviewer.verify_schema_proposal(
            &proof,
            &current,
            approve,
            &statement_bytes,
            previous,
        ))?;
        let signed_basis = checked(h::basis_from_document(&input.basis))?;
        let reviewed_read = self
            .read(Some(RevisionNumber::new(
                u64::try_from(signed_basis.observed_revision.0).map_err(error)?,
            )))
            .map_err(error)?;
        let historical = self.project_schema_proposal_at(&proposal, false, &reviewed_read)?;
        if approve {
            super::schema_proposal_corrections::validate(&reviewed_read, &proposal.proposal)?;
        }
        if *historical.basis != *input.basis {
            return Err(error(
                "review basis differs from claimed historical material",
            ));
        }
        if !read
            .authority_at(read.root.revision)
            .agents
            .contains_key(&verified.operator().actor.0 .0.parse().map_err(error)?)
        {
            return Err(error("review operator is not registered"));
        }
        let decision = crate::upgrade::review_record(&verified, at)?;
        let evidence_id = Box::new(w::EkrGraphEvidenceId(EvidenceId::mint().to_string()));
        let record = w::EkrIntegrateRetainedProposalReview {
            review: Box::new(w::EkrIntegrateProposalReviewSnapshot {
                review_id: Box::new(ProposalReviewId::mint()),
                proposal_id: input.proposal_id.clone(),
                proposal_digest: input.proposal_digest.clone(),
                basis: input.basis.clone(),
                decision: Box::new(if approve {
                    w::EkrIntegrateReviewDecision::V0
                } else {
                    w::EkrIntegrateReviewDecision::V1
                }),
                operator: decision.operator.clone(),
                evidence_id: evidence_id.clone(),
                human_proof_digest: decision.proof_digest.clone(),
                recorded_at: decision.recorded_at,
            }),
            statement: Box::new(statement(&verified, &decision, evidence_id)),
            decision: Box::new(decision),
            proof: ekr_core::bytes::encode(verified.canonical_proof()),
            policy: ekr_core::bytes::encode(verified.canonical_policy()),
        };
        match self.store.retain_proposal_review(&record, previous, at) {
            Ok((retained, _)) => Ok(*retained.review),
            Err(fault @ (StoreError::Conflict | StoreError::PublicationInputConflict)) => {
                // Another invocation can elect distinct recording IDs/time for the same proof.
                // Recover only the independently verified exact decision; never overwrite it.
                for winner in self.verified_proposal_reviews(&proposal)? {
                    if winner.decision.decision_id == proof.intent.decision_id.0
                        && winner.proof == record.proof
                        && winner.statement.payload == input.statement
                    {
                        return Ok(*winner.review);
                    }
                }
                Err(fault)
            }
            Err(fault) => Err(fault),
        }
    }
}
