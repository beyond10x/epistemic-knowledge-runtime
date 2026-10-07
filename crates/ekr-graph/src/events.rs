//! The revision event vocabulary: the `ekr.kernel` events of
//! `systems/ekr/domains/kernel.yaml`, as one data type.
//!
//! # Why the type is here
//!
//! The kernel publishes these and the store persists them (`systems/ekr/components.yaml`). Both
//! sit above this crate and neither depends on the other, so a definition in either one is a
//! definition the other can only copy — and two copies of an event vocabulary drift silently,
//! because each side only ever reads its own. This is a data type and nothing more: it has no
//! constructor that fills it and no path that emits it.

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{AgentId, ContentHash, EventId, RevisionId, RevisionNumber, TransactionId};
use serde::{Deserialize, Serialize};

/// Immutable, physically checked codec for the generated application guard.
/// This validates identities and representation, never approval or transaction semantics.
#[derive(Clone, Debug)]
pub struct ApplicationGuard {
    data: ekr_core::contract_data::EkrIntegrateApplicationPublicationGuard,
    bytes: Vec<u8>,
}
impl ApplicationGuard {
    /// Exact generated data; mutation requires a fresh checked codec.
    #[must_use]
    pub fn as_data(&self) -> &ekr_core::contract_data::EkrIntegrateApplicationPublicationGuard {
        &self.data
    }
}
impl TryFrom<ekr_core::contract_data::EkrIntegrateApplicationPublicationGuard>
    for ApplicationGuard
{
    type Error = String;
    fn try_from(
        data: ekr_core::contract_data::EkrIntegrateApplicationPublicationGuard,
    ) -> Result<Self, Self::Error> {
        use ekr_core::contract_data::{EkrIntegrateApplicationStepKind as Kind, EssPresence};
        for id in [
            &data.application_id.0,
            &data.step_election_id.0,
            &data.proposal_id.0,
            &data.review_id.0,
            &data.attempt_transaction.0,
        ] {
            let parsed: AgentId = id.parse().map_err(|_| "application guard identity")?;
            if parsed.to_string() != *id {
                return Err("application guard identity spelling".into());
            }
        }
        for hash in [&data.proposal_digest.0, &data.human_proof_digest.0] {
            hash.parse::<ContentHash>()
                .map_err(|_| "application guard hash")?;
        }
        if data
            .review_stream_version
            .as_u64()
            .filter(|n| *n > 0)
            .is_none()
        {
            return Err("application guard cursor must be positive".into());
        }
        match (&*data.step.kind, &data.step.item) {
            (Kind::V1, EssPresence::Present(item)) => {
                let id: AgentId = item
                    .source
                    .interpretation_id
                    .0
                    .parse()
                    .map_err(|_| "application source identity")?;
                if id.to_string() != item.source.interpretation_id.0
                    || item.source.version.as_u64().filter(|n| *n > 0).is_none()
                    || item
                        .item
                        .strip_prefix("facts[")
                        .and_then(|v| v.strip_suffix(']'))
                        .and_then(|v| v.parse::<u64>().ok())
                        .is_none()
                {
                    return Err("application item selector".into());
                }
                item.mapping_digest
                    .0
                    .parse::<ContentHash>()
                    .map_err(|_| "application mapping digest")?;
                item.source
                    .document_digest
                    .0
                    .parse::<ContentHash>()
                    .map_err(|_| "application source digest")?;
            }
            (Kind::V0 | Kind::V2, EssPresence::Absent) => {}
            _ => return Err("application step item disagrees with kind".into()),
        }
        let bytes = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
        Ok(Self { data, bytes })
    }
}
impl PartialEq for ApplicationGuard {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}
impl Eq for ApplicationGuard {}
impl Serialize for ApplicationGuard {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.data.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for ApplicationGuard {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        Self::try_from(
            ekr_core::contract_data::EkrIntegrateApplicationPublicationGuard::deserialize(decoder)?,
        )
        .map_err(serde::de::Error::custom)
    }
}
impl Canonical for ApplicationGuard {
    fn encode(&self, out: &mut Encoder) {
        self.bytes.encode(out);
    }
}

/// What happened to a transaction and the revision lineage: the historical six `ekr.kernel`
/// events plus separately versioned authority transitions and reviewed answers.
///
/// `ekr.kernel.SnapshotTaken` and `ekr.kernel.Explained` are declared in the same domain and are
/// not here: neither moves the lineage — one records that a reader took a snapshot, the other that
/// an explanation was produced — and `story:graph-model-and-assertions` scopes this type to the
/// original six. Authority upgrades and reviewed answers add separately versioned occurrences.
///
/// # The first sum type this runtime content-addresses
///
/// `crates/ekr-core/src/canonical.rs` rule 5 is that a newtype is structural and a sum type is
/// tagged, and this type is why the tagged half exists: `Encoder::variant` was added for it.
///
/// The index each variant carries is a **hand-maintained constant**, written out in
/// [`variant_index`](RevisionPayload::variant_index) as a match on variant *names*. It is not the
/// declared position and nothing derives it from one, so reordering the variants below moves no
/// address at all, and **changing a number moves every address that contains it**. The two can
/// therefore disagree, which is its own defect: a variant inserted mid-list whose arm lands at the
/// end leaves the list and the numbering describing different types.
/// `crates/ekr-graph/tests/revision_events.rs` holds both halves — it transcribes the frozen numbers,
/// which catches a renumbering, and it reads this file as text to check that declaration order and
/// numbering still agree, which catches the insert.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", deny_unknown_fields)]
pub enum RevisionPayload {
    /// The lineage began: `ekr.kernel.Seeded`.
    Seeded {
        /// The seed revision.
        revision_id: RevisionId,
        /// The content address of the seed state.
        seed_hash: ContentHash,
    },
    /// An agent proposed a transaction: `ekr.kernel.TransactionProposed`.
    TransactionProposed {
        /// The transaction.
        transaction_id: TransactionId,
        /// The agent that proposed it. Agents propose; they never commit.
        proposer: AgentId,
        /// The content address of its operations.
        operations_hash: Option<ContentHash>,
    },
    /// Validation accepted it: `ekr.kernel.TransactionValidated`.
    TransactionValidated {
        /// The transaction.
        transaction_id: TransactionId,
        /// The revision it was validated against — design § 71, and what makes § 72's stale
        /// commit detectable.
        against: RevisionNumber,
        /// The content address of the validation result.
        validation_hash: ContentHash,
    },
    /// Validation refused it: `ekr.kernel.TransactionRejected`.
    TransactionRejected {
        /// The transaction.
        transaction_id: TransactionId,
        /// How many issues were raised. The issues themselves are the kernel's records.
        issues: u32,
    },
    /// Canonical state moved under it before it committed: `ekr.kernel.TransactionStale`.
    TransactionStale {
        /// The transaction.
        transaction_id: TransactionId,
        /// The revision it was validated against.
        validated_against: RevisionNumber,
        /// The revision canonical state is at now.
        current: RevisionNumber,
    },
    /// It committed, and the lineage advanced: `ekr.kernel.RevisionCommitted`.
    RevisionCommitted {
        /// The transaction.
        transaction_id: TransactionId,
        /// The revision it produced.
        revision_id: RevisionId,
        /// That revision's position in the lineage.
        number: RevisionNumber,
        /// The content address of the graph state at it.
        knowledge_root: ContentHash,
    },
    /// Explicit authority transition; legacy `/3` or identity-bound `/5` envelope.
    AuthorityUpgraded {
        /// Stable identity of the transition, independent of its content.
        transition_id: EventId,
        /// New revision containing the recomputed assessments.
        revision_id: RevisionId,
        /// Activation revision.
        number: RevisionNumber,
        /// Resulting knowledge root.
        knowledge_root: ContentHash,
    },
    /// An atomic reviewed answer, in legacy `/4` or identity-bound `/5` envelopes.
    AttentionAnswered(crate::AnswerOccurrence),
}

/// A current occurrence and the address of its complete kernel-owned retained record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionEvent {
    /// `/2` for historical kinds, legacy `/3` upgrades and `/4` answers, or `/5` signed kinds.
    pub format: String,
    /// Immutable occurrence identity, allocated independently of content.
    pub event_id: EventId,
    /// Payload-domain address of the complete retained record.
    pub record_hash: ContentHash,
    /// Closed metadata vocabulary, matched to the envelope format by [`Self::supported`].
    pub payload: RevisionPayload,
    /// Kernel-derived application binding, present only in guarded ordinary `/6` events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application: Option<ApplicationGuard>,
}
impl RevisionEvent {
    /// The unchanged envelope format for the historical six event kinds.
    pub const FORMAT: &'static str = "ekr.revision-event/2";
    /// Envelope for the explicit authority transition. Historical occurrences remain version 2.
    pub const TRANSITION_FORMAT: &'static str = "ekr.revision-event/3";
    /// Envelope for reviewed answer publication.
    pub const ANSWER_FORMAT: &'static str = "ekr.revision-event/4";
    /// Signed publications with an atomic audience-wide decision identity binding.
    pub const SIGNED_FORMAT: &'static str = "ekr.revision-event/5";
    /// Ordinary application publications with an atomic proposal-stream marker.
    pub const APPLICATION_FORMAT: &'static str = "ekr.revision-event/6";
    /// Whether this envelope requires the shared human decision binding at replay.
    #[must_use]
    pub fn requires_human_binding(&self) -> bool {
        self.format == Self::SIGNED_FORMAT && self.is_human_decision()
    }
    /// Both canonical operation kinds carrying a generated HumanDecisionRecord.
    #[must_use]
    pub const fn is_human_decision(&self) -> bool {
        matches!(
            self.payload,
            RevisionPayload::AuthorityUpgraded { .. } | RevisionPayload::AttentionAnswered(_)
        )
    }
    /// Closed format dispatch; the new event cannot masquerade as a historical event.
    #[must_use]
    pub fn supported(&self) -> bool {
        if let Some(guard) = &self.application {
            return self.format == Self::APPLICATION_FORMAT
                && self
                    .transaction_id()
                    .is_some_and(|id| id.to_string() == guard.as_data().attempt_transaction.0);
        }
        self.format == self.payload.format() || self.requires_human_binding()
    }
    /// Ordinary transaction identity, absent for seed and separately signed publications.
    #[must_use]
    pub const fn transaction_id(&self) -> Option<TransactionId> {
        match self.payload {
            RevisionPayload::TransactionProposed { transaction_id, .. }
            | RevisionPayload::TransactionValidated { transaction_id, .. }
            | RevisionPayload::TransactionRejected { transaction_id, .. }
            | RevisionPayload::TransactionStale { transaction_id, .. }
            | RevisionPayload::RevisionCommitted { transaction_id, .. } => Some(transaction_id),
            _ => None,
        }
    }
    /// Native provider schema version matching the closed envelope vocabulary.
    #[must_use]
    pub fn schema_version(&self) -> u32 {
        if self.format == Self::APPLICATION_FORMAT {
            return 6;
        }
        if self.requires_human_binding() {
            return 5;
        }
        match self.payload {
            RevisionPayload::AuthorityUpgraded { .. } => 3,
            RevisionPayload::AttentionAnswered(_) => 4,
            _ => 2,
        }
    }
    /// Backend event name selected by the payload kind.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.payload.name()
    }
    /// Fixed payload kind index.
    #[must_use]
    pub const fn variant_index(&self) -> u32 {
        self.payload.variant_index()
    }
}
impl Canonical for RevisionEvent {
    fn encode(&self, out: &mut Encoder) {
        self.format.encode(out);
        self.event_id.encode(out);
        self.record_hash.encode(out);
        self.payload.encode(out);
        // Historical formats retain their exact bytes. Unsupported old envelopes carrying a
        // guard refuse admission; only the separately versioned encoding addresses that guard.
        if self.format == Self::APPLICATION_FORMAT {
            self.application.encode(out);
        }
    }
}

impl RevisionPayload {
    /// Original envelope version for this payload. New signed writers explicitly select `/5`.
    #[must_use]
    pub const fn format(&self) -> &'static str {
        match self {
            Self::AuthorityUpgraded { .. } => RevisionEvent::TRANSITION_FORMAT,
            Self::AttentionAnswered(_) => RevisionEvent::ANSWER_FORMAT,
            _ => RevisionEvent::FORMAT,
        }
    }
    /// The number the canonical encoding tags this variant with.
    ///
    /// Hand-maintained, and part of the contract rather than an implementation detail: every
    /// content address containing a revision event contains this number, so changing one
    /// invalidates every address ever recorded for that variant. The declaration order below is
    /// kept equal to it by a case rather than by the compiler — nothing here derives one from the
    /// other.
    #[must_use]
    pub const fn variant_index(&self) -> u32 {
        match self {
            Self::Seeded { .. } => 0,
            Self::TransactionProposed { .. } => 1,
            Self::TransactionValidated { .. } => 2,
            Self::TransactionRejected { .. } => 3,
            Self::TransactionStale { .. } => 4,
            Self::RevisionCommitted { .. } => 5,
            Self::AuthorityUpgraded { .. } => 6,
            Self::AttentionAnswered(_) => 7,
        }
    }

    /// The event's name in the ESS domain, fully qualified.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Seeded { .. } => "ekr.kernel.Seeded",
            Self::TransactionProposed { .. } => "ekr.kernel.TransactionProposed",
            Self::TransactionValidated { .. } => "ekr.kernel.TransactionValidated",
            Self::TransactionRejected { .. } => "ekr.kernel.TransactionRejected",
            Self::TransactionStale { .. } => "ekr.kernel.TransactionStale",
            Self::RevisionCommitted { .. } => "ekr.kernel.RevisionCommitted",
            Self::AuthorityUpgraded { .. } => "ekr.kernel.AuthorityUpgraded",
            Self::AttentionAnswered(_) => "ekr.kernel.AttentionAnswered",
        }
    }
}

impl Canonical for RevisionPayload {
    /// The variant marker, then the variant's fields in declaration order.
    ///
    /// The marker is not optional and is not a convenience: `RevisionId` and `TransactionId` are
    /// newtypes over the same UUID shape and therefore encode identically, so two variants whose
    /// field lists happened to line up would share a content address. That the six do not line up
    /// today is an accident of this version of `kernel.yaml`, and rule 5 refuses to rest on it.
    fn encode(&self, out: &mut Encoder) {
        out.variant(self.variant_index());
        match self {
            Self::AttentionAnswered(answer) => answer.encode(out),
            Self::Seeded {
                revision_id,
                seed_hash,
            } => {
                revision_id.encode(out);
                seed_hash.encode(out);
            }
            Self::TransactionProposed {
                transaction_id,
                proposer,
                operations_hash,
            } => {
                transaction_id.encode(out);
                proposer.encode(out);
                operations_hash.encode(out);
            }
            Self::TransactionValidated {
                transaction_id,
                against,
                validation_hash,
            } => {
                transaction_id.encode(out);
                against.encode(out);
                validation_hash.encode(out);
            }
            Self::TransactionRejected {
                transaction_id,
                issues,
            } => {
                transaction_id.encode(out);
                issues.encode(out);
            }
            Self::TransactionStale {
                transaction_id,
                validated_against,
                current,
            } => {
                transaction_id.encode(out);
                validated_against.encode(out);
                current.encode(out);
            }
            Self::RevisionCommitted {
                transaction_id,
                revision_id,
                number,
                knowledge_root,
            } => {
                transaction_id.encode(out);
                revision_id.encode(out);
                number.encode(out);
                knowledge_root.encode(out);
            }
            Self::AuthorityUpgraded {
                transition_id,
                revision_id,
                number,
                knowledge_root,
            } => {
                transition_id.encode(out);
                revision_id.encode(out);
                number.encode(out);
                knowledge_root.encode(out);
            }
        }
    }
}
