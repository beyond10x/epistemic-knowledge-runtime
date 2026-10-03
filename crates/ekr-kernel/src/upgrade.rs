//! Explicit, human-reviewed authority transitions in the canonical revision stream.
use crate::human_review::{self as review, Reviewer};
use crate::replay::{self, ReplayState, Revision};
use crate::{
    AuthorityStateV1, BootstrapContext, Commit, CommitError, KernelAuthority, TransactionState,
    ValidationProfileV1,
};
use ekr_core::contract_data::*;
use ekr_core::contracts::{kernel as m, primitives};
use ekr_core::{ContentHash, EventId, RevisionId, Timestamp};
use ekr_graph::{CanonicalGraph, RevisionPayload, Root};
use ekr_store::{
    ObjectStore, PublicationCommandKey, PublicationCommandKind, PublicationObject,
    RecordedOccurrence, RetainedHistory, RevisionLog, StorageClass, StoreError,
};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

fn error(detail: impl std::fmt::Display) -> StoreError {
    replay::refuse(&format!("authority-upgrade: {detail}"))
}
fn reviewed<T>(value: Result<T, m::KnowledgeRefused>) -> Result<T, StoreError> {
    value.map_err(|e| error(e.code))
}
fn bytes(value: &impl Serialize) -> Result<Vec<u8>, StoreError> {
    serde_json::to_vec(value).map_err(error)
}
fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, StoreError> {
    serde_json::from_slice(bytes).map_err(error)
}
fn convert<T: DeserializeOwned>(value: &impl Serialize) -> Result<T, StoreError> {
    decode(&bytes(value)?)
}
fn hash(value: ContentHash) -> Box<EkrKernelContentHash> {
    Box::new(EkrKernelContentHash(value.to_string()))
}
fn parsed(value: &EkrKernelContentHash) -> Result<ContentHash, StoreError> {
    value.0.parse().map_err(error)
}
pub(crate) fn record_hash(
    record: &EkrKernelAuthorityTransitionRecord,
) -> Result<ContentHash, StoreError> {
    Ok(ContentHash::of_bytes(&bytes(record)?))
}
fn version(profile: &ValidationProfileV1) -> EkrKernelAuthorityVersion {
    EkrKernelAuthorityVersion {
        ruleset: Box::new(EkrKernelRulesetV1(profile.ruleset.clone())),
        application: Box::new(EkrKernelApplicationProfileV1(profile.application.clone())),
        profile_digest: hash(ContentHash::of(profile)),
    }
}
fn target(preview: &EkrKernelUpgradePreview) -> m::HumanDecisionTarget {
    m::HumanDecisionTarget::UpgradeAuthority(m::AuthorityUpgradeTarget {
        preview_digest: m::ContentHash(preview.preview_digest.0.clone()),
        reviewer_policy_digest: m::ContentHash(preview.reviewer_policy_digest.0.clone()),
    })
}
fn time_text(at: Timestamp) -> Result<String, StoreError> {
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(at.millis()) * 1_000_000)
        .map_err(error)?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(error)
}
pub(crate) fn review_record(
    proof: &review::VerifiedDecision,
    at: Timestamp,
) -> Result<EkrKernelHumanDecisionRecord, StoreError> {
    let record = reviewed(proof.record(primitives::Timestamp(time_text(at)?)))?;
    Ok(EkrKernelHumanDecisionRecord {
        decision_id: record.decision_id.0,
        proof_object_hash: Box::new(EkrKernelContentHash(record.proof_object_hash.0)),
        policy_object_hash: Box::new(EkrKernelContentHash(record.policy_object_hash.0)),
        statement_object_hash: Box::new(EkrKernelContentHash(record.statement_object_hash.0)),
        proof_digest: Box::new(EkrKernelContentHash(record.proof_digest.0)),
        policy_digest: Box::new(EkrKernelContentHash(record.policy_digest.0)),
        statement_digest: Box::new(EkrKernelContentHash(record.statement_digest.0)),
        operator: Box::new(EkrKernelTrustedOperatorIdentity {
            actor: Box::new(EkrKernelAgentId(record.operator.actor.0 .0)),
            authentication_subject: record.operator.authentication_subject,
        }),
        recorded_at: crate::incubation_document::wire_timestamp(&record.recorded_at.0)?,
    })
}
impl KernelAuthority {
    pub(crate) fn reviewer(
        &self,
        state: &ReplayState,
        policy: &m::ReviewerTrustPolicy,
    ) -> Result<Reviewer, StoreError> {
        let binding = self
            .review_host
            .as_ref()
            .ok_or_else(|| error("review-host-not-provisioned"))?;
        replay::require(
            binding.audience.seed_anchor.0 == state.seed.seed_hash.to_string(),
            "review-seed-anchor",
        )?;
        reviewed(Reviewer::from_host(binding, policy.clone()))
    }
    fn preview(
        &self,
        history: &RetainedHistory,
        state: &ReplayState,
        policy: &m::ReviewerTrustPolicy,
    ) -> Result<EkrKernelUpgradePreview, StoreError> {
        self.reviewer(state, policy)?;
        replay::require(state.transition.is_none(), "authority-already-upgraded")?;
        let mut preview = EkrKernelUpgradePreview {
            stream_version: state.version.into(),
            stream_digest: hash(
                replay::prefix_digests(&history.occurrences)[state.version as usize],
            ),
            reviewer_policy_digest: hash(review::digest(&reviewed(review::policy_bytes(policy))?)),
            head_revision: Box::new(EkrKernelRevisionNumber(
                state.head().root.revision.get().into(),
            )),
            head_hash: hash(ContentHash::of(&state.head().root)),
            from: Box::new(version(&self.anchor.validation_profile)),
            to: Box::new(version(&ValidationProfileV1::knowledge(
                self.context.validator,
            ))),
            preview_digest: hash(review::digest(b"")),
            contradictions: crate::disputes::preview_contradictions(state.head().graph()?)
                .into_iter()
                .map(Box::new)
                .collect(),
            pending_revalidation: state
                .transactions
                .iter()
                .filter(|(_, tx)| tx.state() == TransactionState::Validated)
                .map(|(id, _)| Box::new(EkrKernelTransactionId(id.to_string())))
                .collect(),
        };
        // Digest the generated canonical JSON with the digest slot fixed to SHA256(empty).
        preview.preview_digest = hash(review::digest(&bytes(&preview)?));
        Ok(preview)
    }
}
impl<S: RevisionLog + ObjectStore> Commit<S> {
    /// Open with a independently provisioned, read-only reviewer binding. Request content must
    /// never choose this value. The original seed authority remains unchanged.
    pub fn over_with_review_authority(
        context: BootstrapContext,
        anchor: AuthorityStateV1,
        binding: m::TrustedReviewHostBinding,
        open: impl FnOnce(KernelAuthority) -> Result<S, StoreError>,
    ) -> Result<Self, StoreError> {
        anchor.check(context)?;
        reviewed(review::host_binding_bytes(&binding))?;
        let authority = KernelAuthority {
            context,
            anchor,
            review_host: Some(binding),
            cache: Arc::default(),
            seed_input: Arc::default(),
        };
        let store = open(authority.clone())?;
        Ok(Self { store, authority })
    }
    /// Preview the exact head, contradictions and pending validations without writing anything.
    pub fn preview_upgrade(
        &self,
        policy: &m::ReviewerTrustPolicy,
    ) -> Result<EkrKernelUpgradePreview, CommitError> {
        let (history, state) = self.replayed_state()?;
        Ok(self.authority.preview(&history, &state, policy)?)
    }
    /// Atomically publish the exact reviewed transition. A stale preview refuses before preparing
    /// a write. Retries resume the elected bytes; no new signature or identities are fabricated.
    pub fn apply_upgrade(
        &self,
        preview: &EkrKernelUpgradePreview,
        policy: &m::ReviewerTrustPolicy,
        proof: &m::SignedHumanDecision,
        statement: &[u8],
        now: impl FnOnce() -> Timestamp,
    ) -> Result<EkrKernelAuthorityTransitionRecord, CommitError> {
        let (history, state) = self.replayed_state()?;
        let reviewed_version = preview
            .stream_version
            .as_u64()
            .and_then(|n| n.checked_sub(1))
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| error("upgrade-preview-position"))?;
        let predecessor = history
            .occurrences
            .get(reviewed_version)
            .ok_or_else(|| error("upgrade-preview-position"))?;
        let key = PublicationCommandKey {
            answer_id: None,
            kind: PublicationCommandKind::UpgradeAuthority,
            transaction_id: None,
            predecessor_event_id: Some(predecessor.event.event_id),
            predecessor_record_hash: Some(predecessor.event.record_hash),
        };
        let material = bytes(&(
            preview,
            reviewed(review::policy_bytes(policy))?,
            reviewed(review::proof_bytes(proof))?,
            statement,
        ))?;
        let input = crate::commands::input_hash(
            "UpgradeAuthority",
            &material,
            self.authority.context.operator,
            &self.authority,
        );
        // Even a retry must pass the independently provisioned binding and signature check.
        let verified = reviewed(self.authority.reviewer(&state, policy)?.verify(
            proof,
            &target(preview),
            statement,
            None,
        ))?;
        let prepared = if let Some(pending) = self.pending(&key, input)? {
            pending
        } else {
            let current = self.authority.preview(&history, &state, policy)?;
            replay::require(bytes(&current)? == bytes(preview)?, "upgrade-preview-stale")?;
            let at = now();
            replay::require(at >= state.head().committed_at, "upgrade-time-order")?;
            let profile = ValidationProfileV1::knowledge(self.authority.context.validator);
            let binding = reviewed(review::host_binding_bytes(
                self.authority
                    .review_host
                    .as_ref()
                    .expect("reviewer checked"),
            ))?;
            let profile_bytes = bytes(&profile)?;
            let event_id = EventId::mint();
            let revision_id = RevisionId::mint();
            let transition_id = EventId::mint();
            let (_, root) = upgraded_graph(&state, preview, &profile, &self.authority.anchor)?;
            let record = EkrKernelAuthorityTransitionRecord {
                format: Box::new(EkrKernelAuthorityTransitionFormat::V0),
                transition_id: Box::new(EkrKernelAuthorityTransitionId(transition_id.to_string())),
                event_id: Box::new(EkrKernelEventId(event_id.to_string())),
                revision_id: Box::new(EkrKernelRevisionId(revision_id.to_string())),
                preview: Box::new(preview.clone()),
                result: Box::new(convert(&root)?),
                review: Box::new(review_record(&verified, at)?),
                host_binding_object_hash: hash(ContentHash::of_bytes(&binding)),
                target_profile_object_hash: hash(ContentHash::of_bytes(&profile_bytes)),
            };
            let mut decision = crate::commands::publication(
                event_id,
                RevisionPayload::AuthorityUpgraded {
                    transition_id,
                    revision_id,
                    number: root.revision,
                    knowledge_root: root.knowledge_root,
                },
                bytes(&record)?,
                at,
                state.version,
            );
            for retained in [
                binding,
                profile_bytes,
                verified.canonical_policy().to_vec(),
                verified.canonical_proof().to_vec(),
                statement.to_vec(),
            ] {
                decision.objects.insert(
                    ContentHash::of_bytes(&retained),
                    PublicationObject {
                        bytes: retained,
                        storage_class: StorageClass::Canonical,
                        stored_at: at,
                    },
                );
            }
            self.store.prepare(&key, input, &decision, None)?
        };
        let prepared = self.drive(prepared, |_, _| Err(error("upgrade-preview-stale").into()))?;
        Ok(decode(crate::commands::elected_bytes(&prepared)?)?)
    }
}
fn upgraded_graph(
    state: &ReplayState,
    preview: &EkrKernelUpgradePreview,
    profile: &ValidationProfileV1,
    anchor: &AuthorityStateV1,
) -> Result<(CanonicalGraph, Root), StoreError> {
    let mut graph = state.head().graph()?.clone();
    crate::disputes::recompute(&mut graph, &mut state.assessment_validators.clone())?;
    graph.revision = graph
        .revision
        .next()
        .ok_or_else(|| error("revision-overflow"))?;
    let mut active = anchor.clone();
    active.validation_profile = profile.clone();
    let root = Root {
        revision: graph.revision,
        parent: Some(ContentHash::of(&state.head().root)),
        ontology_root: ContentHash::of(&graph.ontology),
        knowledge_root: ekr_store::knowledge_root(&graph),
        evidence_root: ekr_store::evidence_root(&graph),
        agent_root: ContentHash::of(&active),
        transaction: parsed(&preview.preview_digest)?,
    };
    Ok((graph, root))
}
/// Required transition objects are discovered from the generated retained record, then verified
/// in full by replay. Discovery itself grants no authority.
pub(crate) fn required(
    history: &RetainedHistory,
) -> Result<std::collections::BTreeSet<ContentHash>, StoreError> {
    let mut wanted = std::collections::BTreeSet::new();
    for occurrence in &history.occurrences {
        if matches!(
            occurrence.event.payload,
            RevisionPayload::AuthorityUpgraded { .. }
        ) {
            let record: EkrKernelAuthorityTransitionRecord =
                decode(history.content(occurrence.event.record_hash, StorageClass::Canonical)?)?;
            for value in [
                &record.review.proof_object_hash,
                &record.review.policy_object_hash,
                &record.review.statement_object_hash,
                &record.host_binding_object_hash,
                &record.target_profile_object_hash,
            ] {
                wanted.insert(parsed(value)?);
            }
        }
    }
    Ok(wanted)
}
pub(crate) fn replay_transition(
    authority: &KernelAuthority,
    history: &RetainedHistory,
    state: &mut ReplayState,
    occurrence: &RecordedOccurrence,
) -> Result<(), StoreError> {
    let record: EkrKernelAuthorityTransitionRecord =
        decode(history.content(occurrence.event.record_hash, StorageClass::Canonical)?)?;
    replay::require(
        record_hash(&record)? == occurrence.event.record_hash,
        "upgrade-record-not-canonical",
    )?;
    let content =
        |hash: &EkrKernelContentHash| history.content(parsed(hash)?, StorageClass::Canonical);
    let policy = reviewed(review::read_policy(content(
        &record.review.policy_object_hash,
    )?))?;
    let binding = reviewed(review::read_host_binding(content(
        &record.host_binding_object_hash,
    )?))?;
    let trusted = authority
        .review_host
        .as_ref()
        .ok_or_else(|| error("review-host-not-provisioned"))?;
    replay::require(
        reviewed(review::host_binding_bytes(&binding))?
            == reviewed(review::host_binding_bytes(trusted))?,
        "review-host-binding-mismatch",
    )?;
    let proof = reviewed(review::read_proof(content(
        &record.review.proof_object_hash,
    )?))?;
    let verified = reviewed(authority.reviewer(state, &policy)?.verify(
        &proof,
        &target(&record.preview),
        content(&record.review.statement_object_hash)?,
        None,
    ))?;
    let preview = authority.preview(history, state, &policy)?;
    replay::require(
        bytes(&preview)? == bytes(&record.preview)?,
        "upgrade-preview-stale",
    )?;
    let at = crate::incubation_document::timestamp_value(&record.review.recorded_at)?;
    replay::require(
        at >= state.head().committed_at
            && bytes(&review_record(&verified, at)?)? == bytes(&record.review)?,
        "upgrade-review-record",
    )?;
    let profile: ValidationProfileV1 = decode(content(&record.target_profile_object_hash)?)?;
    replay::require(
        profile == ValidationProfileV1::knowledge(authority.context.validator),
        "upgrade-target-profile",
    )?;
    let (mut graph, root) = upgraded_graph(state, &preview, &profile, &authority.anchor)?;
    let revision_id = record.revision_id.0.parse().map_err(error)?;
    let transition_id = record.transition_id.0.parse().map_err(error)?;
    let expected = RevisionPayload::AuthorityUpgraded {
        transition_id,
        revision_id,
        number: root.revision,
        knowledge_root: root.knowledge_root,
    };
    replay::require(
        record.event_id.0 == occurrence.event.event_id.to_string()
            && occurrence.event.payload == expected
            && bytes(&record.result)? == bytes(&convert::<EkrGraphRevisionRoot>(&root)?)?
            && state.revision_ids.insert(revision_id),
        "upgrade-record-disagrees",
    )?;
    // Rebuild identity history from admitted historical revisions, including stores seeded under
    // profiles which did not previously retain deleted identities. No historical root is changed.
    for number in state.revisions.keys().copied().collect::<Vec<_>>() {
        let held = authority.graph_at(history, state, number)?;
        state.held.record(
            number,
            held.nodes.keys().copied(),
            held.edges.keys().copied(),
        );
    }
    // Capture original accepted attribution before marking either competitor disputed.
    let mut original = state.head().graph()?.clone();
    crate::disputes::recompute(&mut original, &mut state.assessment_validators)?;
    crate::disputes::recompute(&mut graph, &mut state.assessment_validators)?;
    let graph_root = graph.root.id;
    state.revisions.insert(
        root.revision,
        Revision {
            root,
            revision_id,
            event_id: occurrence.event.event_id,
            record_hash: occurrence.event.record_hash,
            committed_at: at,
            graph_root,
            ontology: Arc::new(graph.ontology.clone()),
            graph: Some(Arc::new(graph)),
            asserted_edges: Default::default(),
            alias_holders: Default::default(),
        },
    );
    let pending = state
        .transactions
        .iter()
        .filter(|(_, tx)| tx.state() == TransactionState::Validated)
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    for id in pending {
        let tx = state.transaction_mut(id);
        tx.validation = None;
        tx.validation_record_hash = None;
    }
    state.validated.clear();
    let mut active = authority.anchor.clone();
    active.validation_profile = profile;
    state.upgraded_authority = Some(active);
    state.transition = Some(record);
    Ok(())
}
