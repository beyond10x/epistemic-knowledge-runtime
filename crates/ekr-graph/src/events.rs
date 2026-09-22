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

/// What happened to a transaction, and to the revision lineage: the six `ekr.kernel` events that
/// move a revision.
///
/// `ekr.kernel.SnapshotTaken` and `ekr.kernel.Explained` are declared in the same domain and are
/// not here: neither moves the lineage — one records that a reader took a snapshot, the other that
/// an explanation was produced — and `story:graph-model-and-assertions` scopes this type to the
/// six that do.
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
/// `crates/ekr-graph/tests/revision_events.rs` holds both halves — it transcribes the six numbers,
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
}

/// A current occurrence and the address of its complete kernel-owned retained record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionEvent {
    /// Exactly `ekr.revision-event/2`.
    pub format: String,
    /// Immutable occurrence identity, allocated independently of content.
    pub event_id: EventId,
    /// Payload-domain address of the complete retained record.
    pub record_hash: ContentHash,
    /// Existing six-kind metadata vocabulary.
    pub payload: RevisionPayload,
}
impl RevisionEvent {
    /// The only current event envelope format.
    pub const FORMAT: &'static str = "ekr.revision-event/2";
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
    }
}

impl RevisionPayload {
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
        }
    }
}
