//! What a validator says when it refuses: `ekr.kernel.ValidationIssue` of
//! `systems/ekr/domains/kernel.yaml`.
//!
//! An issue is four things and no more: which transaction, which validator, a stable code and a
//! message a reader can act on. "Invalid" that does not say which operation and why is not a
//! finding anyone can use, which is the same reason `ekr_ontology::CheckError` carries a reason.

use ekr_core::TransactionId;
use serde::{Deserialize, Serialize};

/// One deterministic validator of design § 20: `ekr.kernel.ValidatorName`.
///
/// Seven of the ten the design lists. The contradiction, temporal-consistency and policy
/// validators (items 8 to 10) arrive with the subsystems that need them, in P3 and P5; the domain
/// says so where it declares this enumeration, and a name here with nothing behind it would be a
/// claim the runtime checks something it does not.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ValidatorName {
    /// The proposal is a well-formed transaction, whatever the graph holds.
    Structural,
    /// Every identity it names resolves.
    Reference,
    /// Every value satisfies its declared type, and is one canonical state admits.
    Type,
    /// Every count the ontology constrains is within it.
    Cardinality,
    /// Every named operation is one the node's type and lifecycle declare.
    OntologyConstraint,
    /// Every canonical assertion has sufficient provenance (design § 6.5).
    Provenance,
    /// The actor validating is not the actor that proposed (design § 6.10).
    Authorization,
}

/// A refusal raised against a transaction: `ekr.kernel.ValidationIssue`.
///
/// Fields rather than accessors, as everything a validator hands back in this workspace is: an
/// issue is a record, and nothing here answers a question the fields do not.
///
/// The domain gives the entity an `issue_id` and this does not carry one. An id is minted when an
/// issue is *recorded* — `ekr_graph::ValidationState::Rejected` names issues by id, and recording
/// belongs to the store — and a deterministic validator that minted one would not be
/// deterministic: the same transaction against the same snapshot would produce different bytes on
/// two runs, which is the property design § 73 puts these validators in the deterministic half
/// for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// The transaction it was raised against.
    pub transaction_id: TransactionId,
    /// The validator that raised it.
    pub validator: ValidatorName,
    /// A stable, machine-readable label for the kind of refusal. Every one the kernel can raise
    /// is named by a case: `crates/ekr-kernel/tests/validation.rs` reads them out of the source
    /// rather than keeping a list beside it.
    pub code: String,
    /// What was wrong, naming the operation and the value.
    pub message: String,
}
