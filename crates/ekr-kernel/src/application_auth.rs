//! Pure application authorization over authenticated inputs and the pre-occurrence replay state.
use crate::{
    application_inputs as inputs, replay::ReplayState, schema_proposals::error, GraphOperation,
    GraphTransaction, KernelAuthority, TransactionState, VerifiedRead,
};
use ekr_core::{contract_data as w, ContentHash, RevisionNumber, TransactionId};
use ekr_graph::CanonicalValue;
use ekr_store::{RecordedOccurrence, RetainedHistory, StorageClass, StoreError};

/// A private capability derived from authenticated immutable application inputs. No public
/// constructor, caller flag, source enum or evidence identity can create one.
#[derive(Clone)]
pub(crate) struct EvidenceAdmission {
    additions: Vec<crate::EvidenceAddition>,
}
impl EvidenceAdmission {
    pub(crate) fn permits(&self, addition: &crate::EvidenceAddition) -> bool {
        self.additions.contains(addition)
    }
}

/// Compute only the evidence capability. Ordinary publication still verifies the effective
/// historical/current review and the exact atomic guard; this cannot authorize publication.
pub(crate) fn evidence_admission(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    document: &crate::TransactionDocument,
) -> Result<Option<EvidenceAdmission>, StoreError> {
    if !state
        .active_authority(&authority.anchor)
        .validation_profile
        .supports_application_evidence()
    {
        return Ok(None);
    }
    let Some(attempt) = history
        .applications
        .attempts()
        .iter()
        .find(|attempt| attempt.transaction_id.0 == document.transaction().id.to_string())
    else {
        return Ok(None);
    };
    let step = history
        .applications
        .steps()
        .iter()
        .find(|step| step.step_election_id == attempt.step_election_id)
        .ok_or_else(|| error("application evidence step is unavailable"))?;
    let elected = history
        .applications
        .elections()
        .iter()
        .find(|elected| elected.application_id == step.application_id)
        .ok_or_else(|| error("application evidence election is unavailable"))?;
    election(authority, history, state, elected)?;
    verify_attempt(authority, history, state, attempt)?;
    if !matches!(*step.step.kind, w::EkrIntegrateApplicationStepKind::V2) {
        // Mapping steps reuse canonical support; their exact operations are checked by the
        // ordinary guarded publication authority, never a source-kind allowance.
        return Ok(None);
    }
    let native: GraphTransaction<CanonicalValue> =
        document.transaction().clone().try_into().map_err(error)?;
    if *attempt.transaction != crate::application_transaction::encode(&native)?
        || step.transaction != elected.schema_transaction
        || !matches!(*step.step.kind, w::EkrIntegrateApplicationStepKind::V2)
    {
        return Err(error(
            "application evidence differs from elected operations",
        ));
    }
    Ok(Some(EvidenceAdmission {
        additions: native
            .operations
            .into_iter()
            .filter_map(|op| match op {
                GraphOperation::AddEvidence(addition)
                    if matches!(
                        addition.evidence.source,
                        ekr_graph::EvidenceSource::Observation(_)
                    ) =>
                {
                    Some(*addition)
                }
                _ => None,
            })
            .collect(),
    }))
}

pub(crate) fn committed_receipt(
    history: &RetainedHistory,
    state: &ReplayState,
    election: &w::EkrIntegrateRetainedApplicationElection,
) -> Result<Option<w::EkrIntegrateApplicationReceiptSnapshot>, StoreError> {
    crate::application_progress::snapshot(history, state, election, state.head().root.revision)
}

pub(crate) fn read(
    history: &RetainedHistory,
    state: &ReplayState,
    proposal: &w::EkrIntegrateSchemaProposalId,
) -> Result<w::EssPresence<Box<w::EkrIntegrateApplicationRead>>, StoreError> {
    let Some(election) = history
        .applications
        .elections()
        .iter()
        .find(|e| e.proposal_id.as_ref() == proposal)
    else {
        return Ok(w::EssPresence::Absent);
    };
    let steps: Vec<_> = history
        .applications
        .steps()
        .iter()
        .filter(|s| s.application_id == election.application_id)
        .cloned()
        .collect();
    let attempts = history
        .applications
        .attempts()
        .iter()
        .filter(|a| {
            steps
                .iter()
                .any(|s| s.step_election_id == a.step_election_id)
        })
        .cloned()
        .collect();
    let publications = history
        .applications
        .coordination()
        .iter()
        .filter(|c| c.proposal_id.as_ref() == proposal)
        .flat_map(|c| c.entries.iter())
        .filter_map(|e| match &*e.record {
            w::EkrIntegrateProposalCoordinationRecord::V0(p)
                if p.value.guard.application_id == election.application_id =>
            {
                Some(p.value.clone())
            }
            _ => None,
        })
        .collect();
    let receipts = history
        .applications
        .receipts()
        .iter()
        .filter(|r| r.application_id == election.application_id)
        .cloned()
        .collect();
    let progress = committed_receipt(history, state, election)?;
    let original = inputs::proposal(history, election)?;
    let remaining_items = progress.as_ref().map_or_else(
        || election.selected_items.clone(),
        |p| p.remaining_items.clone(),
    );
    let corrections_pending = progress
        .as_ref()
        .map_or(!original.proposal.corrections.is_empty(), |p| {
            p.corrections_pending
        });
    Ok(w::EssPresence::Present(Box::new(
        w::EkrIntegrateApplicationRead {
            election: election.clone(),
            steps,
            attempts,
            publications,
            receipts,
            remaining_items,
            corrections_pending,
        },
    )))
}

pub(crate) fn attach(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    mut publication: ekr_store::Publication,
) -> Result<ekr_store::Publication, StoreError> {
    let Some(id) = publication.event.transaction_id() else {
        return Ok(publication);
    };
    let Some(attempt) = history
        .applications
        .attempts()
        .iter()
        .find(|a| a.transaction_id.0 == id.to_string())
    else {
        if history
            .applications
            .elections()
            .iter()
            .any(|e| e.schema_transaction.id.0 == id.to_string())
            || history
                .applications
                .steps()
                .iter()
                .any(|s| s.transaction.id.0 == id.to_string())
        {
            return Err(error(
                "reserved application transaction has no elected attempt",
            ));
        }
        if publication.event.application.is_some() {
            return Err(error("guard has no elected transaction"));
        }
        return Ok(publication);
    };
    let step = history
        .applications
        .steps()
        .iter()
        .find(|s| s.step_election_id == attempt.step_election_id)
        .ok_or_else(|| error("application step is unavailable"))?;
    let elected = history
        .applications
        .elections()
        .iter()
        .find(|e| e.application_id == step.application_id)
        .ok_or_else(|| error("application election is unavailable"))?;
    let proposal = election(authority, history, state, elected)?;
    let coordination = history
        .applications
        .coordination()
        .iter()
        .find(|p| p.proposal_id == elected.proposal_id)
        .ok_or_else(|| error("application coordination is unavailable"))?;
    let cursor = coordination
        .stream_version
        .as_u64()
        .ok_or_else(|| error("review cursor"))?;
    let chain = reviews(authority, history, state, &proposal, cursor)?;
    let latest = chain
        .last()
        .ok_or_else(|| error("missing effective approval"))?;
    if !matches!(*latest.review.decision, w::EkrIntegrateReviewDecision::V0) {
        return Err(error("application has been rejected"));
    }
    publication.event.application = Some(
        ekr_graph::events::ApplicationGuard::try_from(w::EkrIntegrateApplicationPublicationGuard {
            application_id: elected.application_id.clone(),
            proposal_id: elected.proposal_id.clone(),
            proposal_digest: elected.proposal_digest.clone(),
            review_id: latest.review.review_id.clone(),
            human_proof_digest: latest.review.human_proof_digest.clone(),
            review_stream_version: cursor.into(),
            step: step.step.clone(),
            step_election_id: step.step_election_id.clone(),
            attempt_transaction: attempt.transaction_id.clone(),
        })
        .map_err(error)?,
    );
    publication.event.format = ekr_graph::RevisionEvent::APPLICATION_FORMAT.into();
    Ok(publication)
}

pub(crate) fn at(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    number: RevisionNumber,
) -> Result<VerifiedRead, StoreError> {
    let mut prefix = state.clone();
    prefix.revisions.retain(|revision, _| *revision <= number);
    prefix.answers.retain(|revision, _| *revision <= number);
    prefix
        .authority_changes
        .retain(|revision, _| *revision <= number);
    if prefix
        .revisions
        .last_key_value()
        .map(|(revision, _)| *revision)
        != Some(number)
    {
        return Err(error("application review revision is unavailable"));
    }
    authority.capture_read(history, &prefix, false)
}

fn projected(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    proposal: &w::EkrIntegrateRetainedSchemaProposal,
    number: RevisionNumber,
) -> Result<(VerifiedRead, w::EkrIntegrateSchemaProposalRead), StoreError> {
    let read = at(authority, history, state, number)?;
    let base = state
        .revisions
        .values()
        .find(|r| r.ontology.version().id.to_string() == proposal.proposal.base_schema.0)
        .ok_or_else(|| error("application base schema is unavailable"))?;
    let observations = inputs::observed(history.applications.observations())?;
    let projected = inputs::project(history, &observations, proposal, &read, &base.ontology)?;
    Ok((read, projected))
}

/// Advance permission only across this application's already verified commits. Each pre-commit
/// material must still match the preceding baseline; intervening external drift is not excused.
pub(crate) fn continuation_basis(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    proposal: &w::EkrIntegrateRetainedSchemaProposal,
    review: &w::EkrIntegrateRetainedProposalReview,
) -> Result<Box<w::EkrKernelReviewBasis>, StoreError> {
    let observed = RevisionNumber::new(
        review
            .review
            .basis
            .observed_revision
            .0
            .as_u64()
            .ok_or_else(|| error("invalid approved revision"))?,
    );
    let (_, shown) = projected(authority, history, state, proposal, observed)?;
    if shown.basis != review.review.basis {
        return Err(error("historical approval material differs"));
    }
    let mut basis = shown.basis;
    let prefixes =
        crate::application_material::prefixes(history, state, state.head().root.revision)?;
    if let Some(prefix) = prefixes.get(&proposal.proposal.proposal_id.0) {
        for (revision, _) in prefix
            .commits
            .iter()
            .filter(|(revision, _)| *revision > observed)
        {
            let before = RevisionNumber::new(
                revision
                    .get()
                    .checked_sub(1)
                    .ok_or_else(|| error("application commit has no predecessor"))?,
            );
            let (_, preceding) = projected(authority, history, state, proposal, before)?;
            if !same_material(&basis, &preceding.basis) {
                return Err(error(
                    "schema-review-required: intervening material changed",
                ));
            }
            basis = projected(authority, history, state, proposal, *revision)?
                .1
                .basis;
        }
    }
    Ok(basis)
}

pub(crate) fn same_material(
    left: &w::EkrKernelReviewBasis,
    right: &w::EkrKernelReviewBasis,
) -> bool {
    left.evidence_digest == right.evidence_digest
        && left.options_digest == right.options_digest
        && left.effects_digest == right.effects_digest
}

fn reviews(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    proposal: &w::EkrIntegrateRetainedSchemaProposal,
    cursor: u64,
) -> Result<Vec<w::EkrIntegrateRetainedProposalReview>, StoreError> {
    let coordination = history
        .applications
        .coordination()
        .iter()
        .find(|p| p.proposal_id == proposal.proposal.proposal_id)
        .ok_or_else(|| error("application review coordination is unavailable"))?;
    if cursor == 0
        || coordination
            .stream_version
            .as_u64()
            .is_none_or(|v| cursor > v)
    {
        return Err(error("invalid application review cursor"));
    }
    let transition = state
        .transition
        .as_ref()
        .ok_or_else(|| error("authority-upgrade-required"))?;
    let policy_bytes = history.content(
        transition
            .review
            .policy_object_hash
            .0
            .parse()
            .map_err(error)?,
        StorageClass::Canonical,
    )?;
    let policy = crate::human_review::read_policy(policy_bytes).map_err(|e| error(e.reason))?;
    let reviewer = authority.reviewer(state, &policy)?;
    let mut result = Vec::new();
    let mut previous = None;
    for (index, entry) in coordination.entries.iter().enumerate() {
        let version = entry
            .stream_version
            .as_u64()
            .ok_or_else(|| error("invalid coordination position"))?;
        if version != index as u64 + 1 {
            return Err(error("noncontiguous coordination history"));
        }
        if version > cursor {
            break;
        }
        if let w::EkrIntegrateProposalCoordinationRecord::V1(row) = &*entry.record {
            let record = &row.value;
            let number = record
                .review
                .basis
                .observed_revision
                .0
                .as_u64()
                .ok_or_else(|| error("invalid review revision"))?;
            let (read, shown) = projected(
                authority,
                history,
                state,
                proposal,
                RevisionNumber::new(number),
            )?;
            let verified = crate::schema_proposal_review::verify_captured_review(
                &reviewer,
                policy_bytes,
                record,
                &shown,
                &read,
                previous,
            )?;
            previous = Some(verified.proof_digest());
            result.push((**record).clone());
        }
    }
    Ok(result)
}

fn initial_review(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    election: &w::EkrIntegrateRetainedApplicationElection,
    proposal: &w::EkrIntegrateRetainedSchemaProposal,
) -> Result<w::EkrIntegrateRetainedProposalReview, StoreError> {
    let coordination = history
        .applications
        .coordination()
        .iter()
        .find(|p| p.proposal_id == election.proposal_id)
        .ok_or_else(|| error("application coordination is unavailable"))?;
    let cursor = coordination
        .entries
        .iter()
        .find_map(|entry| match &*entry.record {
            w::EkrIntegrateProposalCoordinationRecord::V1(r)
                if r.value.review.review_id == election.initial_review_id =>
            {
                entry.stream_version.as_u64()
            }
            _ => None,
        })
        .ok_or_else(|| error("initial approval is unavailable"))?;
    let records = reviews(authority, history, state, proposal, cursor)?;
    let record = records
        .last()
        .ok_or_else(|| error("initial approval is unavailable"))?;
    if record.review.review_id != election.initial_review_id
        || record.review.human_proof_digest != election.initial_proof_digest
        || !matches!(*record.review.decision, w::EkrIntegrateReviewDecision::V0)
        || record.review.recorded_at > election.elected_at
    {
        return Err(error("election differs from initial approval"));
    }
    Ok(record.clone())
}

pub(crate) fn template(
    read: &VerifiedRead,
    proposal: &w::EkrIntegrateRetainedSchemaProposal,
    approval: &w::EkrIntegrateRetainedProposalReview,
    encoded: &w::EkrKernelCanonicalTransactionProjection,
    support: &crate::schema_proposals::SourceSupport,
    observations: &std::collections::BTreeMap<
        ekr_core::ObservationId,
        w::EkrObserveRetainedObservationRead,
    >,
) -> Result<(), StoreError> {
    let p = &proposal.proposal;
    if !p.corrections.is_empty() {
        return Err(error(
            "source mapping application authority is not implemented",
        ));
    }
    let tx = crate::application_transaction::decode(encoded)?;
    if tx.proposer.to_string() != approval.review.operator.actor.0 || tx.schema_version.is_none() {
        return Err(error("schema template attribution or version disagrees"));
    }
    let base = &read.graph.ontology;
    let mut document = base.to_document();
    let mut schema = Vec::new();
    let mut evidence = Vec::new();
    for op in &tx.operations {
        match op {
            GraphOperation::DefineNodeType(node) => document.node_types.push((**node).clone()),
            GraphOperation::DefineEdgeType(edge) => document.edge_types.push((**edge).clone()),
            GraphOperation::ModifyProperty(value) => {
                let owner = value
                    .owner
                    .ok_or_else(|| error("schema property has no owner"))?;
                if let Some(node) = document.node_types.iter_mut().find(|n| n.id == owner) {
                    if node
                        .properties
                        .insert(value.property.id, value.property.clone())
                        .is_some()
                    {
                        return Err(error("schema template replaces an existing property"));
                    }
                } else if let Some(edge) = document.edge_types.iter_mut().find(|n| n.id == owner) {
                    if edge
                        .properties
                        .insert(value.property.id, value.property.clone())
                        .is_some()
                    {
                        return Err(error("schema template replaces an existing property"));
                    }
                } else {
                    return Err(error("schema property owner is unavailable"));
                }
            }
            GraphOperation::AddEvidence(_) => {
                evidence.push(op.clone());
                continue;
            }
            _ => return Err(error("schema template contains an unapproved operation")),
        }
        if !evidence.is_empty() {
            return Err(error("schema operation follows supporting evidence"));
        }
        schema.push(op.clone());
    }
    let candidate = ekr_ontology::Ontology::load(document).map_err(error)?;
    let expected = crate::schema_proposal_schema::candidate(base, p, &support.enums)?;
    if schema != crate::application_plan::schema_operations(base, &candidate)?
        || schema.len() != p.additions.len()
        || crate::schema_proposal_material::relevant(base, &candidate, p)?
            != crate::schema_proposal_material::relevant(base, &expected, p)?
    {
        return Err(error("schema template differs from reviewed additions"));
    }
    let mut wrapper_ids = evidence
        .iter()
        .take(evidence.len().saturating_sub(1))
        .map(|op| {
            if let GraphOperation::AddEvidence(addition) = op {
                Ok(addition.evidence.id)
            } else {
                Err(error("unexpected evidence operation"))
            }
        });
    let supporting =
        crate::application_support::plan(read, p, support, observations, tx.proposer, || {
            wrapper_ids
                .next()
                .ok_or_else(|| error("schema support wrapper is missing"))?
        })?;
    if wrapper_ids.next().is_some() {
        return Err(error("unapproved schema support wrapper"));
    }
    let mut expected_manifest = supporting.manifest;
    expected_manifest.insert(approval.review.evidence_id.0.parse().map_err(error)?);
    if tx.evidence != expected_manifest || evidence.len() != supporting.additions.len() + 1 {
        return Err(error("schema template support differs from approval"));
    }
    for (actual, expected) in evidence.iter().zip(supporting.additions) {
        if !matches!(actual, GraphOperation::AddEvidence(addition) if **addition == expected) {
            return Err(error("schema support wrapper differs from selected source"));
        }
    }
    let only_evidence = GraphTransaction {
        operations: evidence.into_iter().rev().take(1).collect(),
        ..tx
    };
    let encoded = crate::application_transaction::encode(&only_evidence)?;
    let [one] = encoded.operations.as_slice() else {
        return Err(error("schema template statement count"));
    };
    if !matches!(one.as_ref(), w::EkrKernelCanonicalOperationProjection::V2(e) if e.value == approval.statement)
    {
        return Err(error(
            "schema template statement differs from retained human evidence",
        ));
    }
    Ok(())
}

pub(crate) fn election(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    elected: &w::EkrIntegrateRetainedApplicationElection,
) -> Result<w::EkrIntegrateRetainedSchemaProposal, StoreError> {
    let proposal = inputs::proposal(history, elected)?;
    let selected = proposal
        .proposal
        .mappings
        .iter()
        .map(|mapping| crate::application_mapping::item(mapping).map(Box::new))
        .collect::<Result<Vec<_>, _>>()?;
    if elected.selected_items != selected {
        return Err(error(
            "application selected items differ from approved mappings",
        ));
    }
    let review = initial_review(authority, history, state, elected, &proposal)?;
    let revision = review
        .review
        .basis
        .observed_revision
        .0
        .as_u64()
        .ok_or_else(|| error("review revision"))?;
    let read = at(authority, history, state, RevisionNumber::new(revision))?;
    let observations = inputs::observed(history.applications.observations())?;
    let support = inputs::support(history, &observations, &proposal.proposal, &read)?;
    template(
        &read,
        &proposal,
        &review,
        &elected.schema_transaction,
        &support,
        &observations,
    )?;
    Ok(proposal)
}

/// Recover the original-source to admitted-wrapper correspondence from the frozen schema step.
pub(crate) fn source_support(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    elected: &w::EkrIntegrateRetainedApplicationElection,
) -> Result<
    (
        crate::schema_proposals::SourceSupport,
        std::collections::BTreeMap<ekr_core::EvidenceId, ekr_core::EvidenceId>,
    ),
    StoreError,
> {
    let proposal = inputs::proposal(history, elected)?;
    let approval = initial_review(authority, history, state, elected, &proposal)?;
    let reviewed = at(
        authority,
        history,
        state,
        RevisionNumber::new(
            approval
                .review
                .basis
                .observed_revision
                .0
                .as_u64()
                .ok_or_else(|| error("approved revision"))?,
        ),
    )?;
    let observations = inputs::observed(history.applications.observations())?;
    let support = inputs::support(history, &observations, &proposal.proposal, &reviewed)?;
    let tx = crate::application_transaction::decode(&elected.schema_transaction)?;
    let additions: Vec<_> = tx
        .operations
        .iter()
        .filter_map(|op| match op {
            GraphOperation::AddEvidence(addition) => Some(addition.evidence.id),
            _ => None,
        })
        .collect();
    let mut wrappers = additions.iter().take(additions.len().saturating_sub(1));
    let plan = crate::application_support::plan(
        &reviewed,
        &proposal.proposal,
        &support,
        &observations,
        tx.proposer,
        || {
            wrappers
                .next()
                .copied()
                .ok_or_else(|| error("elected schema wrapper is missing"))
        },
    )?;
    if wrappers.next().is_some() {
        return Err(error("elected schema has an extra wrapper"));
    }
    Ok((support, plan.source_evidence))
}

pub(crate) fn verify_attempt(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    attempt: &w::EkrIntegrateRetainedApplicationAttempt,
) -> Result<(), StoreError> {
    let step = history
        .applications
        .steps()
        .iter()
        .find(|s| s.step_election_id == attempt.step_election_id)
        .ok_or_else(|| error("application step is unavailable"))?;
    let native = crate::application_transaction::decode(&attempt.transaction)?;
    if native.id.to_string() != attempt.transaction_id.0 || attempt.elected_at < step.elected_at {
        return Err(error("attempt transaction identity disagrees"));
    }
    let mut expected = crate::application_transaction::decode(&step.transaction)?;
    match (
        &attempt.predecessor_transaction,
        &attempt.predecessor_record_hash,
    ) {
        (w::EssPresence::Absent, w::EssPresence::Absent) => {
            if expected.id != native.id {
                return Err(error("initial attempt changes transaction identity"));
            }
        }
        (w::EssPresence::Present(id), w::EssPresence::Present(hash)) => {
            let prior_id: TransactionId = id.0.parse().map_err(error)?;
            if prior_id == native.id {
                return Err(error("stale successor reuses transaction identity"));
            }
            let predecessor = history
                .applications
                .attempts()
                .iter()
                .find(|a| a.step_election_id == attempt.step_election_id && a.transaction_id == *id)
                .ok_or_else(|| error("stale predecessor attempt is unavailable"))?;
            let held = state
                .transactions
                .get(&prior_id)
                .ok_or_else(|| error("stale predecessor transaction is unavailable"))?;
            let stale = held
                .stale
                .as_ref()
                .ok_or_else(|| error("only terminal Stale authorizes a successor"))?;
            if held.state() != TransactionState::Stale
                || ContentHash::of_bytes(&stale.to_bytes()?).to_string() != hash.0
                || predecessor.elected_at > attempt.elected_at
                || crate::schema_application::wire_time(stale.stale_at)? > attempt.elected_at
            {
                return Err(error("stale predecessor binding disagrees"));
            }
            expected.id = native.id;
        }
        _ => return Err(error("incomplete stale predecessor")),
    }
    if expected != native {
        return Err(error("attempt changes the frozen transaction template"));
    }
    let _ = authority;
    Ok(())
}

fn verify_step(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    election: &w::EkrIntegrateRetainedApplicationElection,
    step: &w::EkrIntegrateRetainedApplicationStep,
    semantic: bool,
) -> Result<(), StoreError> {
    if step.application_id != election.application_id || step.elected_at < election.elected_at {
        return Err(error("application step coordinates disagree"));
    }
    match *step.step.kind {
        w::EkrIntegrateApplicationStepKind::V2 => {
            if step.transaction != election.schema_transaction
                || !matches!(step.step.item, w::EssPresence::Absent)
                || !step.derivations.is_empty()
                || !step.mappings.is_empty()
                || !step.replacements.is_empty()
            {
                return Err(error("unapproved schema application step"));
            }
        }
        w::EkrIntegrateApplicationStepKind::V1 => {
            let w::EssPresence::Present(key) = &step.step.item else {
                return Err(error("mapping step has no qualified item"));
            };
            if !election.selected_items.contains(key) {
                return Err(error("mapping step is outside the elected selection"));
            }
            let proposal = inputs::proposal(history, election)?;
            let mapping = proposal
                .proposal
                .mappings
                .iter()
                .find(|mapping| {
                    crate::application_mapping::item(mapping).is_ok_and(|item| item == **key)
                })
                .ok_or_else(|| error("selected mapping bytes are unavailable"))?;
            let read = at(authority, history, state, state.head().root.revision)?;
            let prefix = read
                .application_prefixes
                .get(&election.proposal_id.0)
                .ok_or_else(|| error("mapping precedes schema commit"))?;
            if !prefix.schema_done() {
                return Err(error("mapping precedes schema commit"));
            }
            let (support, evidence_map) = source_support(authority, history, state, election)?;
            let input = crate::application_fact::BuildInputs {
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
                elected_at: crate::incubation_document::timestamp_value(&step.elected_at)?,
            };
            crate::application_fact::verify_structure(&input, step)?;
            for mapping in &step.mappings {
                let bytes = ekr_core::bytes::decode(&mapping.payload).map_err(error)?;
                if history
                    .content(
                        mapping.mapping_digest.0.parse().map_err(error)?,
                        StorageClass::Provenance,
                    )
                    .map_err(|fault| error(format!("retained mapping object: {fault}")))?
                    != bytes
                {
                    return Err(error("retained mapping object differs from elected bytes"));
                }
            }
            if semantic {
                crate::application_fact::verify_semantic(&input, step)?;
            }
        }
        w::EkrIntegrateApplicationStepKind::V0 => {
            return Err(error("correction step authority is not implemented"))
        }
    }
    Ok(())
}

pub(crate) fn verify_occurrence(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
    occurrence: &RecordedOccurrence,
) -> Result<(), StoreError> {
    let Some(guard) = &occurrence.event.application else {
        if occurrence.event.transaction_id().is_some_and(|id| {
            history
                .applications
                .attempts()
                .iter()
                .any(|attempt| attempt.transaction_id.0 == id.to_string())
                || history
                    .applications
                    .elections()
                    .iter()
                    .any(|e| e.schema_transaction.id.0 == id.to_string())
                || history
                    .applications
                    .steps()
                    .iter()
                    .any(|s| s.transaction.id.0 == id.to_string())
        }) {
            return Err(error(
                "elected application transaction is missing its guard",
            ));
        }
        return Ok(());
    };
    let guard = guard.as_data();
    let cursor = guard
        .review_stream_version
        .as_u64()
        .ok_or_else(|| error("invalid review cursor"))?;
    let marker = history
        .applications
        .coordination()
        .iter()
        .find(|p| p.proposal_id == guard.proposal_id)
        .and_then(|p| {
            p.entries
                .iter()
                .find(|e| e.stream_version.as_u64() == cursor.checked_add(1))
        })
        .and_then(|e| match &*e.record {
            w::EkrIntegrateProposalCoordinationRecord::V0(p) => Some(&p.value),
            _ => None,
        })
        .ok_or_else(|| error("guarded occurrence has no exact coordination marker"))?;
    let command = match occurrence.event.payload {
        ekr_graph::RevisionPayload::TransactionProposed { .. } => {
            w::EkrStorePublicationCommandKind::V3
        }
        ekr_graph::RevisionPayload::TransactionValidated { .. }
        | ekr_graph::RevisionPayload::TransactionRejected { .. } => {
            w::EkrStorePublicationCommandKind::V5
        }
        ekr_graph::RevisionPayload::RevisionCommitted { .. }
        | ekr_graph::RevisionPayload::TransactionStale { .. } => {
            w::EkrStorePublicationCommandKind::V2
        }
        _ => return Err(error("application guard on nonordinary event")),
    };
    if marker.guard.as_ref() != guard
        || marker.event_id.0 != occurrence.event.event_id.to_string()
        || marker.record_hash.0 != occurrence.event.record_hash.to_string()
        || marker.transaction_id != guard.attempt_transaction
        || *marker.command != command
        || occurrence
            .event
            .transaction_id()
            .map(|id| id.to_string())
            .as_ref()
            != Some(&guard.attempt_transaction.0)
    {
        return Err(error("application marker differs from ordinary occurrence"));
    }
    let elected = history
        .applications
        .elections()
        .iter()
        .find(|e| e.application_id == guard.application_id)
        .ok_or_else(|| error("application election is unavailable"))?;
    let proposal = election(authority, history, state, elected)?;
    let step = history
        .applications
        .steps()
        .iter()
        .find(|s| s.step_election_id == guard.step_election_id)
        .ok_or_else(|| error("application step is unavailable"))?;
    let attempt = history
        .applications
        .attempts()
        .iter()
        .find(|a| a.transaction_id == guard.attempt_transaction)
        .ok_or_else(|| error("application attempt is unavailable"))?;
    if step.application_id != elected.application_id
        || step.step != guard.step
        || attempt.step_election_id != step.step_election_id
        || guard.proposal_id != elected.proposal_id
        || guard.proposal_digest != elected.proposal_digest
    {
        return Err(error("schema application guard and elected step disagree"));
    }
    verify_step(
        authority,
        history,
        state,
        elected,
        step,
        matches!(
            occurrence.event.payload,
            ekr_graph::RevisionPayload::TransactionProposed { .. }
        ),
    )?;
    let prefixes =
        crate::application_material::prefixes(history, state, state.head().root.revision)?;
    if prefixes.get(&elected.proposal_id.0).is_some_and(|prefix| {
        prefix.corrections_done()
            || prefix
                .commits
                .iter()
                .any(|(_, p)| p.guard.step_election_id == step.step_election_id)
    }) {
        return Err(error(
            "application step is already committed or corrections are final",
        ));
    }
    verify_attempt(authority, history, state, attempt)?;
    let cursor = guard
        .review_stream_version
        .as_u64()
        .ok_or_else(|| error("invalid review cursor"))?;
    let chain = reviews(authority, history, state, &proposal, cursor)?;
    let effective = chain.last().ok_or_else(|| error("no effective approval"))?;
    if effective.review.review_id != guard.review_id
        || effective.review.human_proof_digest != guard.human_proof_digest
        || !matches!(
            *effective.review.decision,
            w::EkrIntegrateReviewDecision::V0
        )
    {
        return Err(error("effective approval differs from publication guard"));
    }
    let (current_read, current) = projected(
        authority,
        history,
        state,
        &proposal,
        state.head().root.revision,
    )?;
    let observations = inputs::observed(history.applications.observations())?;
    let support = inputs::support(history, &observations, &proposal.proposal, &current_read)?;
    crate::application_support::check_current(&current_read, &proposal.proposal, &support)?;
    let approved = continuation_basis(authority, history, state, &proposal, effective)?;
    if !same_material(&approved, &current.basis) {
        return Err(error(
            "schema-review-required: application material changed",
        ));
    }
    let id: TransactionId = guard.attempt_transaction.0.parse().map_err(error)?;
    let doc = if let ekr_graph::RevisionPayload::TransactionProposed { transaction_id, .. } =
        occurrence.event.payload
    {
        if transaction_id != id {
            return Err(error("guarded proposal transaction disagrees"));
        }
        let record = crate::ProposalRecordV1::from_bytes(
            history.content(occurrence.event.record_hash, StorageClass::Canonical)?,
        )?;
        crate::TransactionDocument::parse(&record.document_bytes).map_err(error)?
    } else {
        let held = state
            .transactions
            .get(&id)
            .ok_or_else(|| error("guarded transaction is unavailable"))?;
        crate::TransactionDocument::parse(&held.proposal.document_bytes).map_err(error)?
    };
    let native: GraphTransaction<CanonicalValue> =
        doc.transaction().clone().try_into().map_err(error)?;
    if crate::application_transaction::encode(&native)? != *attempt.transaction {
        return Err(error("ordinary transaction differs from elected attempt"));
    }
    Ok(())
}

pub(crate) fn verify_history(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &ReplayState,
) -> Result<(), StoreError> {
    for elected in history.applications.elections() {
        election(authority, history, state, elected)?;
    }
    for step in history.applications.steps() {
        let e = history
            .applications
            .elections()
            .iter()
            .find(|e| e.application_id == step.application_id)
            .ok_or_else(|| error("step has no application"))?;
        verify_step(authority, history, state, e, step, false)?;
    }
    for attempt in history.applications.attempts() {
        verify_attempt(authority, history, state, attempt)?;
    }
    for receipt in history.applications.receipts() {
        let elected = history
            .applications
            .elections()
            .iter()
            .find(|e| e.application_id == receipt.application_id)
            .ok_or_else(|| error("receipt has no application"))?;
        verify_receipt(history, state, elected, receipt)?;
    }
    crate::application_progress::verify_processing(history, state)?;
    Ok(())
}

fn verify_receipt(
    history: &RetainedHistory,
    state: &ReplayState,
    election: &w::EkrIntegrateRetainedApplicationElection,
    receipt: &w::EkrIntegrateApplicationReceiptSnapshot,
) -> Result<(), StoreError> {
    crate::application_progress::verify_receipt(history, state, election, receipt)
}

pub(crate) fn complete_report(
    history: &RetainedHistory,
    state: &ReplayState,
    election: &w::EkrIntegrateRetainedApplicationElection,
) -> Result<Option<w::EkrIntegrateApplicationReport>, StoreError> {
    let Some(current) = committed_receipt(history, state, election)? else {
        return Ok(None);
    };
    if !matches!(*current.progress, w::EkrIntegrateApplicationProgress::V0) {
        return Ok(None);
    }
    let Some(receipt) = history
        .applications
        .receipts()
        .iter()
        .rev()
        .find(|r| r.application_id == election.application_id)
    else {
        return Ok(None);
    };
    verify_receipt(history, state, election, receipt)?;
    if !matches!(*receipt.progress, w::EkrIntegrateApplicationProgress::V0) {
        return Ok(None);
    }
    Ok(Some(crate::application_progress::report(
        history, state, election, receipt, true,
    )?))
}
