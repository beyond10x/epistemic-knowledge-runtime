//! Resumable application through the ordinary retained transaction lifecycle.
use crate::{schema_proposals::error, Commit, GraphTransaction, TransactionState};
use ekr_core::{
    contract_data as w,
    generated_identity::{ApplicationStepId, Identity, SchemaApplicationId},
    SchemaVersionId, Timestamp, TransactionId,
};
use ekr_store::{
    ApplicationRetention, HumanDecisionRetention, IncubationRetention, ObjectStore,
    ObservationRetention, ProposalReviewRetention, RevisionLog, SchemaProposalRetention,
    StoreError,
};
use std::collections::BTreeMap;

pub(crate) fn wire_time(at: Timestamp) -> Result<w::EssTimestamp, StoreError> {
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(at.millis()) * 1_000_000)
        .map(w::EssTimestamp)
        .map_err(error)
}

fn supported_initial_slice(
    proposal: &w::EkrIntegrateSchemaProposalDocument,
) -> Result<(), StoreError> {
    // These branches remain refused until their effect builders and replay authorities are installed.
    if !proposal.sources.is_empty()
        || !proposal.observations.is_empty()
        || !proposal.mappings.is_empty()
        || !proposal.corrections.is_empty()
    {
        return Err(error("source mapping application is not implemented"));
    }
    Ok(())
}

pub(crate) fn schema_template(
    read: &crate::VerifiedRead,
    proposal: &w::EkrIntegrateRetainedSchemaProposal,
    approval: &w::EkrIntegrateRetainedProposalReview,
) -> Result<w::EkrKernelCanonicalTransactionProjection, StoreError> {
    supported_initial_slice(&proposal.proposal)?;
    let candidate =
        crate::schema_proposal_schema::candidate(&read.graph.ontology, &proposal.proposal, &[])?;
    let mut native = GraphTransaction {
        id: TransactionId::mint(),
        proposer: approval.review.operator.actor.0.parse().map_err(error)?,
        operations: crate::application_plan::schema_operations(&read.graph.ontology, &candidate)?,
        evidence: proposal
            .proposal
            .evidence
            .iter()
            .map(|id| id.0.parse().map_err(error))
            .collect::<Result<_, _>>()?,
        schema_version: Some(SchemaVersionId::mint()),
    };
    native
        .evidence
        .insert(approval.review.evidence_id.0.parse().map_err(error)?);
    let mut encoded = crate::application_transaction::encode(&native)?;
    encoded
        .operations
        .push(Box::new(w::EkrKernelCanonicalOperationProjection::V2(
            w::EkrKernelCanonicalOperationProjectionVariant2 {
                kind: w::EkrKernelCanonicalOperationProjectionVariant2Kind::V0,
                value: approval.statement.clone(),
            },
        )));
    // Decode the actual elected template now: unreadable transport never reaches retention.
    crate::application_transaction::decode(&encoded)?;
    Ok(encoded)
}

impl<
        S: RevisionLog
            + ObjectStore
            + ObservationRetention
            + IncubationRetention
            + SchemaProposalRetention
            + HumanDecisionRetention
            + ProposalReviewRetention
            + ApplicationRetention,
    > Commit<S>
{
    /// Apply an exact retained approval and resume only its elected ordinary transactions.
    /// # Errors
    /// Missing approval, changed material, unavailable application authority or persistence failure.
    pub fn apply_schema_proposal(
        &self,
        id: &w::EkrIntegrateSchemaProposalId,
        review_id: &w::EkrIntegrateProposalReviewId,
        digest: &w::EkrKernelContentHash,
        at: Timestamp,
    ) -> Result<w::EkrIntegrateApplicationReport, StoreError> {
        let proposal = self
            .store
            .retained_schema_proposals()?
            .into_iter()
            .find(|p| p.proposal.proposal_id.as_ref() == id)
            .ok_or_else(|| error("unknown proposal"))?;
        if proposal.proposal_digest.as_ref() != digest {
            return Err(error("application proposal digest differs"));
        }
        let reviews = self.verified_proposal_reviews(&proposal)?;
        let approved = reviews
            .iter()
            .find(|r| r.review.review_id.as_ref() == review_id)
            .filter(|r| matches!(*r.review.decision, w::EkrIntegrateReviewDecision::V0))
            .ok_or_else(|| error("exact approved review is missing"))?;
        let prior = self
            .store
            .retained_application_elections()?
            .into_iter()
            .find(|row| row.proposal_id.as_ref() == id && row.proposal_digest.as_ref() == digest);
        if let Some(election) = &prior {
            let (history, state) = self.replayed_state().map_err(error)?;
            if let Some(report) =
                crate::application_auth::complete_report(&history, &state, election)?
            {
                return Ok(report);
            }
            if let Some(receipt) =
                crate::application_auth::committed_receipt(&history, &state, election)?
            {
                self.store.retain_application_receipt(&receipt, at)?;
                let (history, state) = self.replayed_state().map_err(error)?;
                return crate::application_auth::complete_report(&history, &state, election)?
                    .ok_or_else(|| error("recovered schema receipt is unavailable"));
            }
        }
        if reviews.last() != Some(approved) {
            return Err(error("approval is no longer the effective human decision"));
        }
        let read = self.read(None).map_err(error)?;
        let shown = self.project_schema_proposal_at(&proposal, false, &read)?;
        let a = &approved.review.basis;
        let b = &shown.basis;
        if a.evidence_digest != b.evidence_digest
            || a.options_digest != b.options_digest
            || a.effects_digest != b.effects_digest
            || a.observed_revision.0.as_u64() > b.observed_revision.0.as_u64()
        {
            return Err(error("schema-review-required: current material changed"));
        }
        supported_initial_slice(&proposal.proposal)?;
        let at_wire = wire_time(at)?;
        if at_wire < approved.review.recorded_at
            || prior
                .as_ref()
                .is_some_and(|election| at_wire < election.elected_at)
        {
            return Err(error(
                "application time precedes retained approval or election",
            ));
        }
        let election = if let Some(prior) = prior {
            prior
        } else {
            let elected = w::EkrIntegrateRetainedApplicationElection {
                application_id: Box::new(SchemaApplicationId::mint()),
                proposal_id: proposal.proposal.proposal_id.clone(),
                proposal_digest: proposal.proposal_digest.clone(),
                base_schema: proposal.proposal.base_schema.clone(),
                initial_review_id: approved.review.review_id.clone(),
                initial_proof_digest: approved.review.human_proof_digest.clone(),
                elected_at: at_wire,
                schema_transaction: Box::new(schema_template(&read, &proposal, approved)?),
                selected_items: Vec::new(),
            };
            let objects = BTreeMap::from([(
                proposal.proposal_digest.0.clone(),
                Box::new(w::EkrStorePublicationObject {
                    bytes: proposal.payload.clone(),
                    storage_class: Box::new(w::EkrStoreStorageClass::V4),
                    stored_at: at_wire,
                }),
            )]);
            self.store
                .elect_application(
                    &w::EkrStoreApplicationElectionRetention {
                        election: Box::new(elected),
                        objects: w::EkrStoreApplicationElectionRetentionObjects {
                            ess_extra: objects,
                        },
                    },
                    at,
                )?
                .0
        };
        let steps = self
            .store
            .retained_application_steps(&election.application_id)?;
        let step = if let Some(held) = steps
            .iter()
            .find(|s| matches!(*s.step.kind, w::EkrIntegrateApplicationStepKind::V2))
        {
            held.clone()
        } else {
            self.store
                .elect_application_step(
                    &w::EkrStoreApplicationStepRetention {
                        step: Box::new(w::EkrIntegrateRetainedApplicationStep {
                            application_id: election.application_id.clone(),
                            step_election_id: Box::new(ApplicationStepId::mint()),
                            step: Box::new(w::EkrIntegrateApplicationStep {
                                kind: Box::new(w::EkrIntegrateApplicationStepKind::V2),
                                item: w::EssPresence::Absent,
                            }),
                            transaction: election.schema_transaction.clone(),
                            elected_at: at_wire,
                            derivations: vec![],
                            mappings: vec![],
                            replacements: vec![],
                        }),
                        objects: w::EkrStoreApplicationStepRetentionObjects {
                            ess_extra: BTreeMap::new(),
                        },
                    },
                    at,
                )?
                .0
        };
        self.execute_schema_step(&step, at)?;
        let (history, state) = self.replayed_state().map_err(error)?;
        let receipt = crate::application_auth::committed_receipt(&history, &state, &election)?
            .ok_or_else(|| error("schema transaction has not committed"))?;
        self.store.retain_application_receipt(&receipt, at)?;
        let (history, state) = self.replayed_state().map_err(error)?;
        let mut report = crate::application_auth::complete_report(&history, &state, &election)?
            .ok_or_else(|| error("committed schema application has no report"))?;
        report.already_complete = false;
        Ok(report)
    }

    fn execute_schema_step(
        &self,
        step: &w::EkrIntegrateRetainedApplicationStep,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        for _ in 0..16 {
            let at_wire = wire_time(at)?;
            let attempts = self
                .store
                .retained_application_attempts(&step.step_election_id)?;
            let attempt = if let Some(held) = attempts.last() {
                let transactions = self.transactions().map_err(error)?;
                let id = held.transaction_id.0.parse().map_err(error)?;
                if let Some(stale) = transactions.get(&id).and_then(|t| t.stale.as_ref()) {
                    if at < stale.stale_at {
                        return Err(error("successor time precedes terminal stale decision"));
                    }
                    let mut transaction = step.transaction.clone();
                    transaction.id =
                        Box::new(w::EkrKernelTransactionId(TransactionId::mint().to_string()));
                    self.store
                        .elect_application_attempt(
                            &w::EkrStoreApplicationAttemptRetention {
                                attempt: Box::new(w::EkrIntegrateRetainedApplicationAttempt {
                                    transaction_id: transaction.id.clone(),
                                    transaction,
                                    step_election_id: step.step_election_id.clone(),
                                    elected_at: at_wire,
                                    predecessor_transaction: w::EssPresence::Present(
                                        held.transaction_id.clone(),
                                    ),
                                    predecessor_record_hash: w::EssPresence::Present(Box::new(
                                        w::EkrKernelContentHash(
                                            ekr_core::ContentHash::of_bytes(&stale.to_bytes()?)
                                                .to_string(),
                                        ),
                                    )),
                                }),
                            },
                            at,
                        )?
                        .0
                } else {
                    held.clone()
                }
            } else {
                self.store
                    .elect_application_attempt(
                        &w::EkrStoreApplicationAttemptRetention {
                            attempt: Box::new(w::EkrIntegrateRetainedApplicationAttempt {
                                transaction_id: step.transaction.id.clone(),
                                transaction: step.transaction.clone(),
                                step_election_id: step.step_election_id.clone(),
                                elected_at: at_wire,
                                predecessor_transaction: w::EssPresence::Absent,
                                predecessor_record_hash: w::EssPresence::Absent,
                            }),
                        },
                        at,
                    )?
                    .0
            };
            let transaction = crate::application_transaction::decode(&attempt.transaction)?;
            let tx_id = transaction.id;
            let current = self.transactions().map_err(error)?;
            if !current.contains_key(&tx_id) {
                let body = serde_yaml_ng::to_string(&transaction).map_err(error)?;
                let bytes = format!(
                    "format: ekr.transaction-document/2\ntransaction:\n{}",
                    body.lines()
                        .map(|line| format!("  {line}\n"))
                        .collect::<String>()
                );
                self.propose(bytes.as_bytes(), transaction.proposer, || at)
                    .map_err(error)?;
            }
            let current = self.transactions().map_err(error)?;
            if current[&tx_id].state() == TransactionState::Proposed {
                self.validate_application_attempt(tx_id, || at)
                    .map_err(error)?;
            }
            let current = self.transactions().map_err(error)?;
            if current[&tx_id].state() == TransactionState::Validated {
                self.commit(tx_id, transaction.proposer, || at)
                    .map_err(error)?;
            }
            match self.transactions().map_err(error)?[&tx_id].state() {
                TransactionState::Committed => return Ok(()),
                TransactionState::Stale => continue,
                TransactionState::Rejected => {
                    return Err(error("elected schema transaction was rejected"))
                }
                _ => return Err(error("elected schema transaction has no terminal outcome")),
            }
        }
        Err(StoreError::Conflict)
    }
}
