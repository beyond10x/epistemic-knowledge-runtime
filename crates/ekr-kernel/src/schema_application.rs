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

pub(crate) fn schema_template(
    read: &crate::VerifiedRead,
    proposal: &w::EkrIntegrateRetainedSchemaProposal,
    approval: &w::EkrIntegrateRetainedProposalReview,
    support: &crate::schema_proposals::SourceSupport,
    observations: &BTreeMap<ekr_core::ObservationId, w::EkrObserveRetainedObservationRead>,
) -> Result<w::EkrKernelCanonicalTransactionProjection, StoreError> {
    let actor = approval.review.operator.actor.0.parse().map_err(error)?;
    let candidate = crate::schema_proposal_schema::candidate(
        &read.graph.ontology,
        &proposal.proposal,
        &support.enums,
    )?;
    let supporting = crate::application_support::plan(
        read,
        &proposal.proposal,
        support,
        observations,
        actor,
        || Ok(ekr_core::EvidenceId::mint()),
    )?;
    let mut native = GraphTransaction {
        id: TransactionId::mint(),
        proposer: actor,
        operations: crate::application_plan::schema_operations(&read.graph.ontology, &candidate)?,
        evidence: supporting.manifest,
        schema_version: Some(SchemaVersionId::mint()),
    };
    native.operations.extend(
        supporting
            .additions
            .into_iter()
            .map(|addition| crate::GraphOperation::AddEvidence(Box::new(addition))),
    );
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
    // The same exact historical template admission runs before physical election and on replay.
    crate::application_auth::template(read, proposal, approval, &encoded, support, observations)?;
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
                if matches!(*receipt.progress, w::EkrIntegrateApplicationProgress::V0) {
                    return self.record_application_progress(election, None, true, at);
                }
            }
        }
        if reviews.last() != Some(approved) {
            return Err(error("approval is no longer the effective human decision"));
        }
        let read = self.read(None).map_err(error)?;
        let shown = self.project_schema_proposal_at(&proposal, false, &read)?;
        let (history, state) = self.replayed_state().map_err(error)?;
        let expected = if prior.is_some() {
            crate::application_auth::continuation_basis(
                &self.authority,
                &history,
                &state,
                &proposal,
                approved,
            )?
        } else {
            // The review was authenticated above using the retained proposal sources. Before
            // election those sources are not application-history dependencies yet, and there
            // are no application commits across which permission could advance.
            approved.review.basis.clone()
        };
        let a = &expected;
        let b = &shown.basis;
        if a.evidence_digest != b.evidence_digest
            || a.options_digest != b.options_digest
            || a.effects_digest != b.effects_digest
            || a.observed_revision.0.as_u64() > b.observed_revision.0.as_u64()
        {
            return Err(error("schema-review-required: current material changed"));
        }
        if (!proposal.proposal.sources.is_empty()
            || !proposal.proposal.observations.is_empty()
            || !proposal.proposal.corrections.is_empty())
            && !read
                .authority_at(read.root.revision)
                .validation_profile
                .supports_application_evidence()
        {
            return Err(error("knowledge-three-upgrade-required"));
        }
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
            let reviewed = self
                .read(Some(ekr_core::RevisionNumber::new(
                    approved
                        .review
                        .basis
                        .observed_revision
                        .0
                        .as_u64()
                        .ok_or_else(|| error("invalid approved revision"))?,
                )))
                .map_err(error)?;
            let supporting = crate::schema_proposals::source_support(
                &proposal.proposal,
                &read,
                |version| {
                    self.retained_interpretation(version)
                        .map(|held| *held.document)
                },
                |id| self.observation(id.0.parse().map_err(error)?).map(|_| ()),
            )?;
            // Validate present-day collisions before any election, while replay and allocation
            // share the exact reviewed basis. An identical later source addition is unrelated.
            crate::application_support::check_current(&read, &proposal.proposal, &supporting)?;
            let mut observations = BTreeMap::new();
            for id in &proposal.proposal.observations {
                let id = id.0.parse().map_err(error)?;
                observations.insert(id, self.observation(id)?);
            }
            let elected = w::EkrIntegrateRetainedApplicationElection {
                application_id: Box::new(SchemaApplicationId::mint()),
                proposal_id: proposal.proposal.proposal_id.clone(),
                proposal_digest: proposal.proposal_digest.clone(),
                base_schema: proposal.proposal.base_schema.clone(),
                initial_review_id: approved.review.review_id.clone(),
                initial_proof_digest: approved.review.human_proof_digest.clone(),
                elected_at: at_wire,
                schema_transaction: Box::new(schema_template(
                    &reviewed,
                    &proposal,
                    approved,
                    &supporting,
                    &observations,
                )?),
                selected_items: proposal
                    .proposal
                    .mappings
                    .iter()
                    .map(|mapping| crate::application_mapping::item(mapping).map(Box::new))
                    .collect::<Result<Vec<_>, _>>()?,
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
                            correction_review_id: w::EssPresence::Absent,
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
        // Later steps use the exact same ordinary command driver and immutable attempt protocol.
        let stop = self
            .execute_mapping_steps(&election, at)
            .and_then(|()| self.execute_correction_step(&election, at))
            .err()
            .map(|error| error.to_string());
        self.record_application_progress(&election, stop, false, at)
    }

    fn record_application_progress(
        &self,
        election: &w::EkrIntegrateRetainedApplicationElection,
        stop: Option<String>,
        already_complete: bool,
        at: Timestamp,
    ) -> Result<w::EkrIntegrateApplicationReport, StoreError> {
        let (history, state) = self.replayed_state().map_err(error)?;
        for receipt in crate::application_progress::processing(
            &history,
            &state,
            election,
            state.head().root.revision,
        )? {
            self.store.retain_processing_receipt(&receipt, at)?;
        }
        let (history, state) = self.replayed_state().map_err(error)?;
        let mut receipt = crate::application_auth::committed_receipt(&history, &state, election)?
            .ok_or_else(|| error("schema transaction has not committed"))?;
        if !matches!(*receipt.progress, w::EkrIntegrateApplicationProgress::V0) {
            if let Some(stop) = stop {
                *receipt.progress = w::EkrIntegrateApplicationProgress::V3;
                receipt.stop_reason = w::EssPresence::Present(stop);
            }
        }
        let (receipt, _) = self.store.retain_application_receipt(&receipt, at)?;
        let (history, state) = self.replayed_state().map_err(error)?;
        crate::application_progress::report(
            &history,
            &state,
            election,
            &receipt,
            already_complete && matches!(*receipt.progress, w::EkrIntegrateApplicationProgress::V0),
        )
    }

    fn execute_mapping_steps(
        &self,
        election: &w::EkrIntegrateRetainedApplicationElection,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let mut blocked = Vec::new();
        for selected in &election.selected_items {
            let (history, state) = self.replayed_state().map_err(error)?;
            let proposal =
                crate::application_auth::election(&self.authority, &history, &state, election)?;
            let read = self.authority.capture_read(&history, &state, false)?;
            if read
                .application_prefixes
                .get(&election.proposal_id.0)
                .is_some_and(|prefix| {
                    prefix.commits.iter().any(|(_, p)| {
                        p.guard.step.item == w::EssPresence::Present(selected.clone())
                    })
                })
            {
                continue;
            }
            let step = if let Some(step) = history.applications.steps().iter().find(|step| {
                step.application_id == election.application_id
                    && step.step.item == w::EssPresence::Present(selected.clone())
            }) {
                (**step).clone()
            } else {
                let mapping = proposal
                    .proposal
                    .mappings
                    .iter()
                    .find(|mapping| {
                        crate::application_mapping::item(mapping)
                            .is_ok_and(|item| item == **selected)
                    })
                    .ok_or_else(|| error("elected mapping is unavailable"))?;
                let (support, evidence_map) = crate::application_auth::source_support(
                    &self.authority,
                    &history,
                    &state,
                    election,
                )?;
                let inputs = crate::application_fact::BuildInputs {
                    read: &read,
                    proposal: &proposal,
                    mapping,
                    sources: &support.sources,
                    evidence_map: &evidence_map,
                    application_id: &election.application_id,
                    proposer: election
                        .schema_transaction
                        .proposer
                        .0
                        .parse()
                        .map_err(error)?,
                    elected_at: at,
                };
                let step = match crate::application_fact::build(&inputs)? {
                    crate::application_fact::FactStepOutcome::Ready(step) => step,
                    crate::application_fact::FactStepOutcome::Blocked(reasons) => {
                        blocked.extend(reasons);
                        continue;
                    }
                };
                self.store
                    .elect_application_step(
                        &w::EkrStoreApplicationStepRetention {
                            step,
                            objects: w::EkrStoreApplicationStepRetentionObjects {
                                ess_extra: BTreeMap::new(),
                            },
                        },
                        at,
                    )?
                    .0
            };
            self.execute_schema_step(&step, at)?;
        }
        if blocked.is_empty() {
            Ok(())
        } else {
            Err(error(blocked.join("; ")))
        }
    }

    fn execute_correction_step(
        &self,
        election: &w::EkrIntegrateRetainedApplicationElection,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let (history, state) = self.replayed_state().map_err(error)?;
        let proposal =
            crate::application_auth::election(&self.authority, &history, &state, election)?;
        let read = self.authority.capture_read(&history, &state, false)?;
        let prefix = read
            .application_prefixes
            .get(&election.proposal_id.0)
            .ok_or_else(|| error("corrections precede schema"))?;
        if prefix.corrections_done() || proposal.proposal.corrections.is_empty() {
            return Ok(());
        }
        if crate::application_correction::unresolved(&proposal.proposal) {
            return Err(error(
                "schema-correction-unresolved: selected uncertainty remains pending",
            ));
        }
        if !prefix.remaining(&proposal.proposal)?.mappings.is_empty() {
            return Err(error("corrections await selected mappings"));
        }
        let step = if let Some(step) = history.applications.steps().iter().find(|s| {
            s.application_id == election.application_id
                && matches!(*s.step.kind, w::EkrIntegrateApplicationStepKind::V0)
        }) {
            (**step).clone()
        } else {
            let cursor = history
                .applications
                .coordination()
                .iter()
                .find(|c| c.proposal_id == election.proposal_id)
                .and_then(|c| c.stream_version.as_u64())
                .ok_or_else(|| error("correction review cursor unavailable"))?;
            let chain = crate::application_auth::review_decisions(
                &self.authority,
                &history,
                &state,
                &proposal,
                cursor,
            )?;
            let (review, decision) = chain
                .last()
                .ok_or_else(|| error("correction approval unavailable"))?;
            let step = crate::application_correction::build(
                &crate::application_correction::Inputs {
                    read: &read,
                    proposal: &proposal,
                    review,
                    decision,
                    election,
                },
                at,
            )?;
            self.store
                .elect_application_step(
                    &w::EkrStoreApplicationStepRetention {
                        step: Box::new(step),
                        objects: w::EkrStoreApplicationStepRetentionObjects {
                            ess_extra: BTreeMap::new(),
                        },
                    },
                    at,
                )?
                .0
        };
        self.execute_schema_step(&step, at)
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
                    return Err(error("elected application transaction was rejected"))
                }
                _ => {
                    return Err(error(
                        "elected application transaction has no terminal outcome",
                    ))
                }
            }
        }
        Err(StoreError::Conflict)
    }
}
