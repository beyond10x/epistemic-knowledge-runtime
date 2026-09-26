//! Retained command responses and the shared durable command handlers.
use crate::replay::{self, ReplayState};
use crate::{
    Commit, CommitError, CommitReceiptV1, ProposalRecordV1, RecordedValidationIssue,
    RejectionRecordV1, StaleRecordV1, TransactionRecord, TransactionState, ValidationReceiptV1,
};
use ekr_core::{
    AgentId, Canonical, ContentHash, Encoder, EventId, IssueId, RevisionId, RevisionNumber,
    Timestamp, TransactionId,
};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_store::{
    ObjectStore, Publication, PublicationCommandKey, PublicationCommandKind, PublicationObject,
    PublicationPreparationV1, RevisionLog, StorageClass, StoreError,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The actual retained validation decision, without replacing issues by a count.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ValidationCommandResult {
    /// Accepted by the exact registered deterministic validator.
    Validated(ValidationReceiptV1),
    /// Refused by the recorded deterministic checks.
    Rejected(RejectionRecordV1),
}
/// The actual retained publication outcome, distinct from effect-free command refusal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum CommitCommandResult {
    /// A newly published or exactly retained original commit.
    Committed(Box<CommitReceiptV1>),
    /// Another canonical revision won; the original validation is retained as stale.
    Stale(Box<StaleRecordV1>),
}
fn target(state: &ReplayState, id: TransactionId) -> Result<&TransactionRecord, CommitError> {
    state
        .transactions
        .get(&id)
        .ok_or(CommitError::TransactionNotFound { transaction_id: id })
}
fn require_state(tx: &TransactionRecord, expected: TransactionState) -> Result<(), CommitError> {
    if tx.state() == expected {
        Ok(())
    } else {
        Err(CommitError::TransactionStateConflict {
            transaction_id: tx.proposal.transaction_id,
            state: tx.state(),
        })
    }
}
pub(crate) fn publication(
    event_id: EventId,
    payload: RevisionPayload,
    bytes: Vec<u8>,
    at: Timestamp,
    version: u64,
) -> Publication {
    let hash = ContentHash::of_bytes(&bytes);
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id,
            record_hash: hash,
            payload,
        },
        objects: BTreeMap::from([(
            hash,
            PublicationObject {
                storage_class: StorageClass::Canonical,
                stored_at: at,
                bytes,
            },
        )]),
        expected_version: version,
    }
}
pub(crate) fn input_hash(
    command: &str,
    material: &[u8],
    actor: AgentId,
    authority: &crate::KernelAuthority,
) -> ContentHash {
    struct Input<'a> {
        command: &'a str,
        material: &'a [u8],
        actor: AgentId,
        authority: &'a crate::KernelAuthority,
    }
    impl Canonical for Input<'_> {
        fn encode(&self, out: &mut Encoder) {
            "ekr.publication-input/1".encode(out);
            self.command.encode(out);
            self.material.encode(out);
            self.actor.encode(out);
            self.authority.context.operator.encode(out);
            self.authority.context.validator.encode(out);
            self.authority.anchor.encode(out);
        }
    }
    ContentHash::of(&Input {
        command,
        material,
        actor,
        authority,
    })
}
pub(crate) fn elected_bytes(prepared: &PublicationPreparationV1) -> Result<&[u8], CommitError> {
    prepared
        .decision
        .objects
        .get(&prepared.decision.event.record_hash)
        .map(|object| object.bytes.as_slice())
        .ok_or_else(|| replay::refuse("elected-record-missing").into())
}
fn validation_result(
    prepared: &PublicationPreparationV1,
) -> Result<ValidationCommandResult, CommitError> {
    match prepared.decision.event.payload {
        RevisionPayload::TransactionValidated { .. } => Ok(ValidationCommandResult::Validated(
            ValidationReceiptV1::from_bytes(elected_bytes(prepared)?)?,
        )),
        RevisionPayload::TransactionRejected { .. } => Ok(ValidationCommandResult::Rejected(
            RejectionRecordV1::from_bytes(elected_bytes(prepared)?)?,
        )),
        _ => Err(replay::refuse("elected-validation-kind").into()),
    }
}
fn commit_result(prepared: &PublicationPreparationV1) -> Result<CommitCommandResult, CommitError> {
    match prepared.decision.event.payload {
        RevisionPayload::RevisionCommitted { .. } => Ok(CommitCommandResult::Committed(Box::new(
            CommitReceiptV1::from_bytes(elected_bytes(prepared)?)?,
        ))),
        RevisionPayload::TransactionStale { .. } => Ok(CommitCommandResult::Stale(Box::new(
            StaleRecordV1::from_bytes(elected_bytes(prepared)?)?,
        ))),
        _ => Err(replay::refuse("elected-commit-kind").into()),
    }
}
impl<S: RevisionLog + ObjectStore> Commit<S> {
    pub(crate) fn pending(
        &self,
        key: &PublicationCommandKey,
        input: ContentHash,
    ) -> Result<Option<PublicationPreparationV1>, CommitError> {
        let held = self.store.preparation(key)?;
        if held
            .as_ref()
            .is_some_and(|prepared| prepared.input_hash != input)
        {
            return Err(StoreError::PublicationInputConflict.into());
        }
        Ok(held)
    }
    fn drive(
        &self,
        mut prepared: PublicationPreparationV1,
        mut after_conflict: impl FnMut(
            &ReplayState,
            &PublicationPreparationV1,
        ) -> Result<Publication, CommitError>,
    ) -> Result<PublicationPreparationV1, CommitError> {
        for _ in 0..16 {
            match self.store.resume(&prepared) {
                Ok(_) => return Ok(prepared),
                Err(StoreError::Conflict) => {
                    if let Some(latest) =
                        self.pending(&prepared.command_key, prepared.input_hash)?
                    {
                        if latest != prepared {
                            prepared = latest;
                            continue;
                        }
                    }
                    let state = self.read_state()?;
                    let next = after_conflict(&state, &prepared)?;
                    prepared = self.store.prepare(
                        &prepared.command_key,
                        prepared.input_hash,
                        &next,
                        Some(&prepared),
                    )?;
                }
                Err(error) => return Err(error.into()),
            }
        }
        Err(StoreError::Conflict.into())
    }
    /// Reads under the frozen document byte limit before entering the shared proposal handler.
    /// # Errors
    /// Bounded ingress/parse refusal, state/authority conflict or publication failure.
    pub fn propose_reader(
        &self,
        reader: impl std::io::Read,
        actor: AgentId,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<ProposalRecordV1, CommitError> {
        let document = crate::TransactionDocument::read(reader)?;
        self.propose(document.bytes(), actor, now)
    }
    pub(crate) fn read_state(&self) -> Result<ReplayState, CommitError> {
        let history = self.store.history()?;
        self.authority
            .reconstruct(&history, None, None)?
            .ok_or(CommitError::NotSeeded)
    }
    /// [`Self::read_state`], replayed from the seed so that every revision's graph is held.
    pub(crate) fn read_state_in_full(&self) -> Result<ReplayState, CommitError> {
        let history = self.store.history()?;
        self.authority
            .reconstruct_in_full(&history)?
            .ok_or(CommitError::NotSeeded)
    }
    /// Captures all actual retained transaction records, including terminal decisions.
    /// # Errors
    /// Missing initialization or invalid required history.
    pub fn transactions(&self) -> Result<BTreeMap<TransactionId, TransactionRecord>, CommitError> {
        Ok(self.read_state()?.transactions)
    }
    /// Retains exact input bytes under a trusted registered submitter.
    /// # Errors
    /// Invalid input, identity/state conflict or persistence failure.
    pub fn propose(
        &self,
        bytes: &[u8],
        actor: AgentId,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<ProposalRecordV1, CommitError> {
        let state = self.read_state()?;
        let parsed = crate::TransactionDocument::parse(bytes)?;
        let id = parsed.transaction().id;
        if !self.authority.anchor.agents.contains_key(&actor)
            || parsed.transaction().proposer != actor
            || parsed.transaction().operations.iter().any(
                |op| matches!(op,crate::GraphOperation::AddAssertion(a) if a.proposed_by!=actor),
            )
        {
            return Err(CommitError::ProposalAttribution { actor });
        }
        if let Some(held) = state.transactions.get(&id) {
            return Err(CommitError::TransactionStateConflict {
                transaction_id: id,
                state: held.state(),
            });
        }
        let key = PublicationCommandKey {
            kind: PublicationCommandKind::Propose,
            transaction_id: Some(id),
            predecessor_event_id: None,
            predecessor_record_hash: None,
        };
        let input = input_hash("Propose", bytes, actor, &self.authority);
        let prepared = if let Some(pending) = self.pending(&key, input)? {
            pending
        } else {
            let at = now();
            let record =
                replay::proposal(bytes, actor, EventId::mint(), at, &self.authority.anchor)?;
            let decision = publication(
                record.event_id,
                RevisionPayload::TransactionProposed {
                    transaction_id: id,
                    proposer: actor,
                    operations_hash: record.canonical_operations_hash,
                },
                record.to_bytes()?,
                at,
                state.version,
            );
            self.store.prepare(&key, input, &decision, None)?
        };
        let prepared = self.drive(prepared, |state, prior| {
            if let Some(held) = state.transactions.get(&id) {
                return Err(CommitError::TransactionStateConflict {
                    transaction_id: id,
                    state: held.state(),
                });
            }
            let mut next = prior.decision.clone();
            next.expected_version = state.version;
            Ok(next)
        })?;
        let record = ProposalRecordV1::from_bytes(elected_bytes(&prepared)?)?;
        self.retain_verification();
        Ok(record)
    }
    /// Validates the retained proposal against a complete existing revision basis.
    /// # Errors
    /// Missing target/basis, state conflict or persistence failure.
    pub fn validate(
        &self,
        id: TransactionId,
        against: RevisionNumber,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<ValidationCommandResult, CommitError> {
        let mut state = self.read_state()?;
        // Validating against an earlier revision needs that revision's graph, which a state
        // restored from a checkpoint does not hold; the history is then replayed in full.
        if state
            .revisions
            .get(&against)
            .is_some_and(|revision| revision.graph.is_none())
        {
            state = self.read_state_in_full()?;
        }
        let tx = target(&state, id)?;
        require_state(tx, TransactionState::Proposed)?;
        let key = PublicationCommandKey {
            kind: PublicationCommandKind::Validate,
            transaction_id: Some(id),
            predecessor_event_id: Some(tx.proposal.event_id),
            predecessor_record_hash: Some(tx.proposal_record_hash),
        };
        let material = serde_json::to_vec(&(id, &key, against))
            .map_err(|error| replay::refuse(&error.to_string()))?;
        let input = input_hash(
            "Validate",
            &material,
            self.authority.context.validator,
            &self.authority,
        );
        let prepared = if let Some(pending) = self.pending(&key, input)? {
            pending
        } else {
            let prior = state
                .revisions
                .get(&against)
                .ok_or(CommitError::RevisionNotFound { against })?;
            let basis = replay::basis(prior, state.seed.seed_hash, &self.authority.anchor);
            let verdict = replay::validate(
                &*state.document(&tx.proposal)?,
                &state.revisions,
                prior,
                &self.authority.anchor,
                self.authority.context.validator,
            )?;
            let at = now();
            replay::require(
                at >= tx.proposal.submitted_at && at >= prior.committed_at,
                "validation-time-order",
            )?;
            let event_id = EventId::mint();
            let (payload, bytes) = match verdict {
                Ok(validated) => {
                    let record = replay::validation_record(
                        &tx.proposal,
                        tx.proposal_record_hash,
                        &validated,
                        basis,
                        self.authority.context.validator,
                        event_id,
                        at,
                    );
                    (
                        RevisionPayload::TransactionValidated {
                            transaction_id: id,
                            against,
                            validation_hash: record.validation_hash,
                        },
                        record.to_bytes()?,
                    )
                }
                Err(issues) => {
                    let record = RejectionRecordV1 {
                        format: RejectionRecordV1::FORMAT.into(),
                        event_id,
                        proposed_event_id: tx.proposal.event_id,
                        proposal_record_hash: tx.proposal_record_hash,
                        requested_basis: basis,
                        validator: self.authority.context.validator,
                        rejected_at: at,
                        issues: issues
                            .into_iter()
                            .map(|issue| RecordedValidationIssue {
                                id: IssueId::mint(),
                                transaction_id: issue.transaction_id,
                                validator: issue.validator,
                                code: issue.code,
                                message: issue.message,
                            })
                            .collect(),
                    };
                    (
                        RevisionPayload::TransactionRejected {
                            transaction_id: id,
                            issues: u32::try_from(record.issues.len())
                                .map_err(|_| replay::refuse("issue-count-overflow"))?,
                        },
                        record.to_bytes()?,
                    )
                }
            };
            self.store.prepare(
                &key,
                input,
                &publication(event_id, payload, bytes, at, state.version),
                None,
            )?
        };
        let prepared = self.drive(prepared, |state, prior| {
            require_state(target(state, id)?, TransactionState::Proposed)?;
            let mut next = prior.decision.clone();
            next.expected_version = state.version;
            Ok(next)
        })?;
        let result = validation_result(&prepared)?;
        self.retain_verification();
        Ok(result)
    }
    /// Applies an accepted transaction, or returns its retained success before sampling time.
    /// # Errors
    /// Missing target, state/authority/time conflict or persistence failure.
    pub fn commit(
        &self,
        id: TransactionId,
        actor: AgentId,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<CommitCommandResult, CommitError> {
        let state = self.read_state()?;
        let tx = target(&state, id)?;
        replay::registered(&self.authority.anchor, actor)?;
        if let Some(held) = &tx.committed {
            return Ok(CommitCommandResult::Committed(Box::new(held.clone())));
        }
        require_state(tx, TransactionState::Validated)?;
        let validation = tx.validation.as_ref().expect("validated state");
        let key = PublicationCommandKey {
            kind: PublicationCommandKind::Commit,
            transaction_id: Some(id),
            predecessor_event_id: Some(validation.event_id),
            predecessor_record_hash: tx.validation_record_hash,
        };
        let material =
            serde_json::to_vec(&(id, &key)).map_err(|error| replay::refuse(&error.to_string()))?;
        let input = input_hash("Commit", &material, actor, &self.authority);
        let prepared = if let Some(pending) = self.pending(&key, input)? {
            pending
        } else {
            let at = now();
            replay::require(at >= validation.validated_at, "commit-time-order")?;
            let decision =
                self.commit_decision(&state, id, actor, at, EventId::mint(), RevisionId::mint())?;
            self.store.prepare(&key, input, &decision, None)?
        };
        let prepared = self.drive(prepared, |state, prior| {
            let tx = target(state, id)?;
            require_state(tx, TransactionState::Validated)?;
            let result = commit_result(prior)?;
            match result {
                CommitCommandResult::Committed(receipt)
                    if receipt.validation.basis
                        != replay::basis(
                            state.head(),
                            state.seed.seed_hash,
                            &self.authority.anchor,
                        ) =>
                {
                    self.commit_decision(
                        state,
                        id,
                        actor,
                        receipt.committed_at,
                        EventId::mint(),
                        receipt.revision_id,
                    )
                }
                _ => {
                    let mut next = prior.decision.clone();
                    next.expected_version = state.version;
                    Ok(next)
                }
            }
        })?;
        let result = commit_result(&prepared)?;
        if matches!(result, CommitCommandResult::Committed(_)) {
            self.retain_checkpoint();
        } else {
            self.retain_verification();
        }
        Ok(result)
    }
    /// Writes the replay checkpoint of the head this handle just published, so that the next
    /// open continues from it rather than replaying the lineage from the seed.
    ///
    /// Best effort by design: the publication has already succeeded, and a checkpoint that is
    /// not written costs the next open time, never correctness.
    pub(crate) fn retain_checkpoint(&self) {
        let Some(state) = self.published_state() else {
            return;
        };
        if let Ok(Some((covered, binding, bytes))) = self.authority.checkpoint(&state) {
            let _ = self.store.write_checkpoint(covered, binding, Some(&bytes));
        }
    }
    /// Records that the occurrence this handle just published, which moved no revision, was
    /// verified with everything before it, keeping the retained checkpoint. Best effort, as
    /// [`Self::retain_checkpoint`] is.
    pub(crate) fn retain_verification(&self) {
        let Some(state) = self.published_state() else {
            return;
        };
        if let Some((covered, binding)) = self.authority.verification(&state) {
            let _ = self.store.write_checkpoint(covered, binding, None);
        }
    }
    /// The state the publication just made reached: the newest this authority verified, which
    /// is the published candidate it admitted before writing it. Whatever it is, what is written
    /// from it is bound to the prefix it covers and is admitted only for exactly that prefix.
    fn published_state(&self) -> Option<std::sync::Arc<ReplayState>> {
        self.authority.cache.lock().ok()?.newest()
    }
    fn commit_decision(
        &self,
        state: &ReplayState,
        id: TransactionId,
        actor: AgentId,
        at: Timestamp,
        event_id: EventId,
        revision_id: RevisionId,
    ) -> Result<Publication, CommitError> {
        let tx = target(state, id)?;
        let validation = tx.validation.as_ref().expect("validated state");
        let head = state.head();
        let (payload, bytes) = if validation.basis
            != replay::basis(head, state.seed.seed_hash, &self.authority.anchor)
        {
            let record = StaleRecordV1 {
                format: StaleRecordV1::FORMAT.into(),
                event_id,
                validation_record_hash: tx.validation_record_hash.expect("validated address"),
                expected_basis: validation.basis.clone(),
                observed_revision_id: head.revision_id,
                observed_event_id: head.event_id,
                observed_record_hash: head.record_hash,
                observed_root: head.root,
                observed_root_hash: ContentHash::of(&head.root),
                stale_at: at,
            };
            (
                RevisionPayload::TransactionStale {
                    transaction_id: id,
                    validated_against: validation.basis.previous_root.revision,
                    current: head.root.revision,
                },
                record.to_bytes()?,
            )
        } else {
            // The basis is the head, so a validation this state sealed against it is this one.
            let validated = match state.validated.get(&id) {
                Some(validated) => std::sync::Arc::clone(validated),
                None => std::sync::Arc::new(
                    replay::validate(
                        &*state.document(&tx.proposal)?,
                        &state.revisions,
                        head,
                        &self.authority.anchor,
                        self.authority.context.validator,
                    )?
                    .map_err(|_| replay::refuse("retained-validation-refused"))?,
                ),
            };
            let (_, root) = crate::apply::apply(head, &validated, &validation.validators, at)?;
            let record = CommitReceiptV1 {
                format: CommitReceiptV1::FORMAT.into(),
                event_id,
                revision_id,
                proposal: tx.proposal.clone(),
                validation: validation.clone(),
                validation_record_hash: tx.validation_record_hash.expect("validated address"),
                committer: actor,
                committed_at: at,
                result: root,
                result_hash: ContentHash::of(&root),
            };
            (
                RevisionPayload::RevisionCommitted {
                    transaction_id: id,
                    revision_id,
                    number: root.revision,
                    knowledge_root: root.knowledge_root,
                },
                record.to_bytes()?,
            )
        };
        Ok(publication(event_id, payload, bytes, at, state.version))
    }
}
