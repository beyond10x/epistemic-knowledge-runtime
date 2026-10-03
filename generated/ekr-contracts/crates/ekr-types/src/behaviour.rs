// generated from ekr v1
// model digest 6e6b51b6bd6e58fd549ea5d5d5562997e603f9e0012f19b9285bfadec393cabc
// contract digest a0934f66cde7acd61a5a40778b7848f6896e36e413fcc19560b8f9a78863f051
// do not edit: regenerate with `ess synthesize`

//! What the specification fully determines, generated: the behaviour of every command the plan
//! lists as generated, written against ports the implementor supplies.
//!
//! Storage is a port: one trait per entity, get, put and delete of a snapshot by identity. ess
//! generates the trait and never a store. `Context` is the other port: the caller's attributes,
//! every identity and value the model says the implementation assigns, and the answer to each
//! `external:` branch. [`Generated`] implements every generated `…Behavior` trait over those ports
//! and forwards every behaviour and query the plan still owes to them, so it is a complete bundle
//! for every component port. To replace one generated behaviour, write a bundle of your own that
//! implements that trait and delegates the rest to a `Generated`.
//!
//! An `Err` from a generated behaviour is the typed refusal naming the command: the model declares
//! no outcome for the request (a guard is undecidable over it, or no declared branch answers it),
//! or — as `entity invariant` — the declared outcome would leave an entity breaking an invariant.

use crate::obligation::UnmetObligation;

/// Where `ekr.graph.Assertion` is stored — a port the implementor provides.
///
/// Keyed by the identity `assertion_id`. ess generates this trait and never an implementation of it.
pub trait AssertionStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::graph::AssertionId) -> Option<crate::graph::AssertionSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::graph::AssertionSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::graph::AssertionId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::graph::AssertionSnapshot>;
}

/// Where `ekr.graph.Evidence` is stored — a port the implementor provides.
///
/// Keyed by the identity `evidence_id`. ess generates this trait and never an implementation of it.
pub trait EvidenceStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::graph::EvidenceId) -> Option<crate::graph::EvidenceSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::graph::EvidenceSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::graph::EvidenceId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::graph::EvidenceSnapshot>;
}

/// Where `ekr.integrate.ApplicationReceipt` is stored — a port the implementor provides.
///
/// Keyed by the identity `receipt_id`. ess generates this trait and never an implementation of it.
pub trait ApplicationReceiptStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::integrate::ApplicationReceiptId) -> Option<crate::integrate::ApplicationReceiptEntitySnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::integrate::ApplicationReceiptEntitySnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::integrate::ApplicationReceiptId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::integrate::ApplicationReceiptEntitySnapshot>;
}

/// Where `ekr.integrate.CanonicalDerivation` is stored — a port the implementor provides.
///
/// Keyed by the identity `derivation_id`. ess generates this trait and never an implementation of it.
pub trait CanonicalDerivationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::primitives::Uuid) -> Option<crate::integrate::CanonicalDerivationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::integrate::CanonicalDerivationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::primitives::Uuid);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::integrate::CanonicalDerivationSnapshot>;
}

/// Where `ekr.integrate.IntegrationBlocker` is stored — a port the implementor provides.
///
/// Keyed by the identity `blocker_id`. ess generates this trait and never an implementation of it.
pub trait IntegrationBlockerStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::integrate::IntegrationBlockerId) -> Option<crate::integrate::IntegrationBlockerEntitySnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::integrate::IntegrationBlockerEntitySnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::integrate::IntegrationBlockerId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::integrate::IntegrationBlockerEntitySnapshot>;
}

/// Where `ekr.integrate.Interpretation` is stored — a port the implementor provides.
///
/// Keyed by the identity `document_digest`. ess generates this trait and never an implementation of it.
pub trait InterpretationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::kernel::ContentHash) -> Option<crate::integrate::InterpretationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::integrate::InterpretationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::kernel::ContentHash);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::integrate::InterpretationSnapshot>;
}

/// Where `ekr.integrate.InterpretationObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `link_id`. ess generates this trait and never an implementation of it.
pub trait InterpretationObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::primitives::Uuid) -> Option<crate::integrate::InterpretationObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::integrate::InterpretationObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::primitives::Uuid);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::integrate::InterpretationObservationSnapshot>;
}

/// Where `ekr.integrate.MappingRecord` is stored — a port the implementor provides.
///
/// Keyed by the identity `mapping_id`. ess generates this trait and never an implementation of it.
pub trait MappingRecordStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::integrate::MappingRecordId) -> Option<crate::integrate::MappingRecordSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::integrate::MappingRecordSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::integrate::MappingRecordId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::integrate::MappingRecordSnapshot>;
}

/// Where `ekr.integrate.ProcessingReceipt` is stored — a port the implementor provides.
///
/// Keyed by the identity `receipt_id`. ess generates this trait and never an implementation of it.
pub trait ProcessingReceiptStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::integrate::ProcessingReceiptId) -> Option<crate::integrate::ProcessingReceiptEntitySnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::integrate::ProcessingReceiptEntitySnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::integrate::ProcessingReceiptId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::integrate::ProcessingReceiptEntitySnapshot>;
}

/// Where `ekr.integrate.ProposalEvidence` is stored — a port the implementor provides.
///
/// Keyed by the identity `binding_id`. ess generates this trait and never an implementation of it.
pub trait ProposalEvidenceStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::primitives::Uuid) -> Option<crate::integrate::ProposalEvidenceSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::integrate::ProposalEvidenceSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::primitives::Uuid);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::integrate::ProposalEvidenceSnapshot>;
}

/// Where `ekr.integrate.ProposalObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `binding_id`. ess generates this trait and never an implementation of it.
pub trait ProposalObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::primitives::Uuid) -> Option<crate::integrate::ProposalObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::integrate::ProposalObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::primitives::Uuid);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::integrate::ProposalObservationSnapshot>;
}

/// Where `ekr.integrate.ProposalReview` is stored — a port the implementor provides.
///
/// Keyed by the identity `review_id`. ess generates this trait and never an implementation of it.
pub trait ProposalReviewStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::integrate::ProposalReviewId) -> Option<crate::integrate::ProposalReviewEntitySnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::integrate::ProposalReviewEntitySnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::integrate::ProposalReviewId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::integrate::ProposalReviewEntitySnapshot>;
}

/// Where `ekr.integrate.ProposalSourceBinding` is stored — a port the implementor provides.
///
/// Keyed by the identity `binding_id`. ess generates this trait and never an implementation of it.
pub trait ProposalSourceBindingStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::primitives::Uuid) -> Option<crate::integrate::ProposalSourceBindingSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::integrate::ProposalSourceBindingSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::primitives::Uuid);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::integrate::ProposalSourceBindingSnapshot>;
}

/// Where `ekr.integrate.SchemaProposal` is stored — a port the implementor provides.
///
/// Keyed by the identity `proposal_id`. ess generates this trait and never an implementation of it.
pub trait SchemaProposalStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::integrate::SchemaProposalId) -> Option<crate::integrate::SchemaProposalSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::integrate::SchemaProposalSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::integrate::SchemaProposalId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::integrate::SchemaProposalSnapshot>;
}

/// Where `ekr.kernel.AuthorityTransition` is stored — a port the implementor provides.
///
/// Keyed by the identity `transition_id`. ess generates this trait and never an implementation of it.
pub trait AuthorityTransitionStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::kernel::AuthorityTransitionId) -> Option<crate::kernel::AuthorityTransitionSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::kernel::AuthorityTransitionSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::kernel::AuthorityTransitionId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::kernel::AuthorityTransitionSnapshot>;
}

/// Where `ekr.kernel.Dispute` is stored — a port the implementor provides.
///
/// Keyed by the identity `dispute_id`. ess generates this trait and never an implementation of it.
pub trait DisputeStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::kernel::DisputeId) -> Option<crate::kernel::DisputeSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::kernel::DisputeSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::kernel::DisputeId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::kernel::DisputeSnapshot>;
}

/// Where `ekr.kernel.DisputeClaim` is stored — a port the implementor provides.
///
/// Keyed by the identity `claim_link_id`. ess generates this trait and never an implementation of it.
pub trait DisputeClaimStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::primitives::Uuid) -> Option<crate::kernel::DisputeClaimSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::kernel::DisputeClaimSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::primitives::Uuid);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::kernel::DisputeClaimSnapshot>;
}

/// Where `ekr.kernel.GraphTransaction` is stored — a port the implementor provides.
///
/// Keyed by the identity `transaction_id`. ess generates this trait and never an implementation of it.
pub trait GraphTransactionStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::kernel::TransactionId) -> Option<crate::kernel::GraphTransactionSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::kernel::GraphTransactionSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::kernel::TransactionId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::kernel::GraphTransactionSnapshot>;
}

/// Where `ekr.kernel.HumanAnswer` is stored — a port the implementor provides.
///
/// Keyed by the identity `answer_id`. ess generates this trait and never an implementation of it.
pub trait HumanAnswerStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::kernel::HumanAnswerId) -> Option<crate::kernel::HumanAnswerSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::kernel::HumanAnswerSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::kernel::HumanAnswerId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::kernel::HumanAnswerSnapshot>;
}

/// Where `ekr.kernel.RetainedHumanDecision` is stored — a port the implementor provides.
///
/// Keyed by the identity `proof_digest`. ess generates this trait and never an implementation of it.
pub trait RetainedHumanDecisionStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::kernel::ContentHash) -> Option<crate::kernel::RetainedHumanDecisionSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::kernel::RetainedHumanDecisionSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::kernel::ContentHash);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::kernel::RetainedHumanDecisionSnapshot>;
}

/// Where `ekr.kernel.Revision` is stored — a port the implementor provides.
///
/// Keyed by the identity `revision_id`. ess generates this trait and never an implementation of it.
pub trait RevisionStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::kernel::RevisionId) -> Option<crate::kernel::RevisionSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::kernel::RevisionSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::kernel::RevisionId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::kernel::RevisionSnapshot>;
}

/// Where `ekr.kernel.SchemaTransactionEvidence` is stored — a port the implementor provides.
///
/// Keyed by the identity `link_id`. ess generates this trait and never an implementation of it.
pub trait SchemaTransactionEvidenceStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::primitives::Uuid) -> Option<crate::kernel::SchemaTransactionEvidenceSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::kernel::SchemaTransactionEvidenceSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::primitives::Uuid);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::kernel::SchemaTransactionEvidenceSnapshot>;
}

/// Where `ekr.kernel.ValidationIssue` is stored — a port the implementor provides.
///
/// Keyed by the identity `issue_id`. ess generates this trait and never an implementation of it.
pub trait ValidationIssueStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::kernel::IssueId) -> Option<crate::kernel::ValidationIssueSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::kernel::ValidationIssueSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::kernel::IssueId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::kernel::ValidationIssueSnapshot>;
}

/// Where `ekr.observe.RetainedObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. ess generates this trait and never an implementation of it.
pub trait RetainedObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::graph::ObservationId) -> Option<crate::observe::RetainedObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observe::RetainedObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::graph::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observe::RetainedObservationSnapshot>;
}

/// Where `ekr.ontology.NodeType` is stored — a port the implementor provides.
///
/// Keyed by the identity `type_id`. ess generates this trait and never an implementation of it.
pub trait NodeTypeStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::ontology::TypeId) -> Option<crate::ontology::NodeTypeSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::ontology::NodeTypeSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::ontology::TypeId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::ontology::NodeTypeSnapshot>;
}

/// Every generated behaviour of this workspace, over the ports `P` supplies.
///
/// `P` implements the storage trait of each entity a generated behaviour reads or writes,
/// `Context` where one asks it anything, and every `…Behavior` and `…Query` trait the plan still
/// owes; `Generated<P>` forwards those to it.
pub struct Generated<P> {
    /// The storage and context ports, and every behaviour or query still owed.
    pub ports: P,
}

impl<P> Generated<P> {
    /// The generated behaviours, over `ports`.
    pub fn new(ports: P) -> Self {
        Self { ports }
    }
}

impl<P: crate::integrate::obligations::ApplyExtractionBehavior> crate::integrate::obligations::ApplyExtractionBehavior for Generated<P> {
    fn apply_extraction(&mut self, input: crate::integrate::ApplyExtraction) -> Result<crate::integrate::ApplyExtractionOutcome, UnmetObligation> {
        crate::integrate::obligations::ApplyExtractionBehavior::apply_extraction(&mut self.ports, input)
    }
}

impl<P: crate::integrate::obligations::ApplySchemaProposalBehavior> crate::integrate::obligations::ApplySchemaProposalBehavior for Generated<P> {
    fn apply_schema_proposal(&mut self, input: crate::integrate::ApplySchemaProposal) -> Result<crate::integrate::ApplySchemaProposalOutcome, UnmetObligation> {
        crate::integrate::obligations::ApplySchemaProposalBehavior::apply_schema_proposal(&mut self.ports, input)
    }
}

impl<P: crate::integrate::obligations::ApproveSchemaProposalBehavior> crate::integrate::obligations::ApproveSchemaProposalBehavior for Generated<P> {
    fn approve_schema_proposal(&mut self, input: crate::integrate::ApproveSchemaProposal) -> Result<crate::integrate::ApproveSchemaProposalOutcome, UnmetObligation> {
        crate::integrate::obligations::ApproveSchemaProposalBehavior::approve_schema_proposal(&mut self.ports, input)
    }
}

impl<P: crate::integrate::obligations::DiscoverSchemaGapsBehavior> crate::integrate::obligations::DiscoverSchemaGapsBehavior for Generated<P> {
    fn discover_schema_gaps(&mut self, input: crate::integrate::DiscoverSchemaGaps) -> Result<crate::integrate::DiscoverSchemaGapsOutcome, UnmetObligation> {
        crate::integrate::obligations::DiscoverSchemaGapsBehavior::discover_schema_gaps(&mut self.ports, input)
    }
}

impl<P: crate::integrate::obligations::ImportInterpretationBehavior> crate::integrate::obligations::ImportInterpretationBehavior for Generated<P> {
    fn import_interpretation(&mut self, input: crate::integrate::ImportInterpretation) -> Result<crate::integrate::ImportInterpretationOutcome, UnmetObligation> {
        crate::integrate::obligations::ImportInterpretationBehavior::import_interpretation(&mut self.ports, input)
    }
}

impl<P: crate::integrate::obligations::ListInterpretationsBehavior> crate::integrate::obligations::ListInterpretationsBehavior for Generated<P> {
    fn list_interpretations(&mut self, input: crate::integrate::ListInterpretations) -> Result<crate::integrate::ListInterpretationsOutcome, UnmetObligation> {
        crate::integrate::obligations::ListInterpretationsBehavior::list_interpretations(&mut self.ports, input)
    }
}

impl<P: crate::integrate::obligations::RejectSchemaProposalBehavior> crate::integrate::obligations::RejectSchemaProposalBehavior for Generated<P> {
    fn reject_schema_proposal(&mut self, input: crate::integrate::RejectSchemaProposal) -> Result<crate::integrate::RejectSchemaProposalOutcome, UnmetObligation> {
        crate::integrate::obligations::RejectSchemaProposalBehavior::reject_schema_proposal(&mut self.ports, input)
    }
}

impl<P: crate::integrate::obligations::ShowInterpretationBehavior> crate::integrate::obligations::ShowInterpretationBehavior for Generated<P> {
    fn show_interpretation(&mut self, input: crate::integrate::ShowInterpretation) -> Result<crate::integrate::ShowInterpretationOutcome, UnmetObligation> {
        crate::integrate::obligations::ShowInterpretationBehavior::show_interpretation(&mut self.ports, input)
    }
}

impl<P: crate::integrate::obligations::ShowSchemaProposalBehavior> crate::integrate::obligations::ShowSchemaProposalBehavior for Generated<P> {
    fn show_schema_proposal(&mut self, input: crate::integrate::ShowSchemaProposal) -> Result<crate::integrate::ShowSchemaProposalOutcome, UnmetObligation> {
        crate::integrate::obligations::ShowSchemaProposalBehavior::show_schema_proposal(&mut self.ports, input)
    }
}

impl<P: crate::integrate::obligations::SubmitSchemaProposalBehavior> crate::integrate::obligations::SubmitSchemaProposalBehavior for Generated<P> {
    fn submit_schema_proposal(&mut self, input: crate::integrate::SubmitSchemaProposal) -> Result<crate::integrate::SubmitSchemaProposalOutcome, UnmetObligation> {
        crate::integrate::obligations::SubmitSchemaProposalBehavior::submit_schema_proposal(&mut self.ports, input)
    }
}

impl<P: crate::kernel::obligations::AnswerAttentionBehavior> crate::kernel::obligations::AnswerAttentionBehavior for Generated<P> {
    fn answer_attention(&mut self, input: crate::kernel::AnswerAttention) -> Result<crate::kernel::AnswerAttentionOutcome, UnmetObligation> {
        crate::kernel::obligations::AnswerAttentionBehavior::answer_attention(&mut self.ports, input)
    }
}

impl<P: crate::kernel::obligations::ApplyUpgradeBehavior> crate::kernel::obligations::ApplyUpgradeBehavior for Generated<P> {
    fn apply_upgrade(&mut self, input: crate::kernel::ApplyUpgrade) -> Result<crate::kernel::ApplyUpgradeOutcome, UnmetObligation> {
        crate::kernel::obligations::ApplyUpgradeBehavior::apply_upgrade(&mut self.ports, input)
    }
}

impl<P: crate::kernel::obligations::CommitBehavior> crate::kernel::obligations::CommitBehavior for Generated<P> {
    fn commit(&mut self, input: crate::kernel::Commit) -> Result<crate::kernel::CommitOutcome, UnmetObligation> {
        crate::kernel::obligations::CommitBehavior::commit(&mut self.ports, input)
    }
}

impl<P: crate::kernel::obligations::ExplainBehavior> crate::kernel::obligations::ExplainBehavior for Generated<P> {
    fn explain(&mut self, input: crate::kernel::Explain) -> Result<crate::kernel::ExplainOutcome, UnmetObligation> {
        crate::kernel::obligations::ExplainBehavior::explain(&mut self.ports, input)
    }
}

impl<P: crate::kernel::obligations::ListAttentionBehavior> crate::kernel::obligations::ListAttentionBehavior for Generated<P> {
    fn list_attention(&mut self, input: crate::kernel::ListAttention) -> Result<crate::kernel::ListAttentionOutcome, UnmetObligation> {
        crate::kernel::obligations::ListAttentionBehavior::list_attention(&mut self.ports, input)
    }
}

impl<P: crate::kernel::obligations::PreviewUpgradeBehavior> crate::kernel::obligations::PreviewUpgradeBehavior for Generated<P> {
    fn preview_upgrade(&mut self, input: crate::kernel::PreviewUpgrade) -> Result<crate::kernel::PreviewUpgradeOutcome, UnmetObligation> {
        crate::kernel::obligations::PreviewUpgradeBehavior::preview_upgrade(&mut self.ports, input)
    }
}

impl<P: crate::kernel::obligations::ProposeBehavior> crate::kernel::obligations::ProposeBehavior for Generated<P> {
    fn propose(&mut self, input: crate::kernel::Propose) -> Result<crate::kernel::ProposeOutcome, UnmetObligation> {
        crate::kernel::obligations::ProposeBehavior::propose(&mut self.ports, input)
    }
}

impl<P: crate::kernel::obligations::SeedBehavior> crate::kernel::obligations::SeedBehavior for Generated<P> {
    fn seed(&mut self, input: crate::kernel::Seed) -> Result<crate::kernel::SeedOutcome, UnmetObligation> {
        crate::kernel::obligations::SeedBehavior::seed(&mut self.ports, input)
    }
}

impl<P: crate::kernel::obligations::ShowAttentionBehavior> crate::kernel::obligations::ShowAttentionBehavior for Generated<P> {
    fn show_attention(&mut self, input: crate::kernel::ShowAttention) -> Result<crate::kernel::ShowAttentionOutcome, UnmetObligation> {
        crate::kernel::obligations::ShowAttentionBehavior::show_attention(&mut self.ports, input)
    }
}

impl<P: crate::kernel::obligations::SnapshotBehavior> crate::kernel::obligations::SnapshotBehavior for Generated<P> {
    fn snapshot(&mut self, input: crate::kernel::Snapshot) -> Result<crate::kernel::SnapshotOutcome, UnmetObligation> {
        crate::kernel::obligations::SnapshotBehavior::snapshot(&mut self.ports, input)
    }
}

impl<P: crate::kernel::obligations::ValidateBehavior> crate::kernel::obligations::ValidateBehavior for Generated<P> {
    fn validate(&mut self, input: crate::kernel::Validate) -> Result<crate::kernel::ValidateOutcome, UnmetObligation> {
        crate::kernel::obligations::ValidateBehavior::validate(&mut self.ports, input)
    }
}

impl<P: crate::observe::obligations::ImportObservationBehavior> crate::observe::obligations::ImportObservationBehavior for Generated<P> {
    fn import_observation(&mut self, input: crate::observe::ImportObservation) -> Result<crate::observe::ImportObservationOutcome, UnmetObligation> {
        crate::observe::obligations::ImportObservationBehavior::import_observation(&mut self.ports, input)
    }
}

impl<P: crate::observe::obligations::ListObservationsBehavior> crate::observe::obligations::ListObservationsBehavior for Generated<P> {
    fn list_observations(&mut self, input: crate::observe::ListObservations) -> Result<crate::observe::ListObservationsOutcome, UnmetObligation> {
        crate::observe::obligations::ListObservationsBehavior::list_observations(&mut self.ports, input)
    }
}

impl<P: crate::observe::obligations::ShowObservationBehavior> crate::observe::obligations::ShowObservationBehavior for Generated<P> {
    fn show_observation(&mut self, input: crate::observe::ShowObservation) -> Result<crate::observe::ShowObservationOutcome, UnmetObligation> {
        crate::observe::obligations::ShowObservationBehavior::show_observation(&mut self.ports, input)
    }
}

impl<P: crate::views::obligations::ChangesSinceBehavior> crate::views::obligations::ChangesSinceBehavior for Generated<P> {
    fn changes_since(&mut self, input: crate::views::ChangesSince) -> Result<crate::views::ChangesSinceOutcome, UnmetObligation> {
        crate::views::obligations::ChangesSinceBehavior::changes_since(&mut self.ports, input)
    }
}

impl<P: crate::views::obligations::DescribeNodeBehavior> crate::views::obligations::DescribeNodeBehavior for Generated<P> {
    fn describe_node(&mut self, input: crate::views::DescribeNode) -> Result<crate::views::DescribeNodeOutcome, UnmetObligation> {
        crate::views::obligations::DescribeNodeBehavior::describe_node(&mut self.ports, input)
    }
}

impl<P: crate::views::obligations::DrawFactSampleBehavior> crate::views::obligations::DrawFactSampleBehavior for Generated<P> {
    fn draw_fact_sample(&mut self, input: crate::views::DrawFactSample) -> Result<crate::views::DrawFactSampleOutcome, UnmetObligation> {
        crate::views::obligations::DrawFactSampleBehavior::draw_fact_sample(&mut self.ports, input)
    }
}

impl<P: crate::views::obligations::ExpandNeighbourhoodBehavior> crate::views::obligations::ExpandNeighbourhoodBehavior for Generated<P> {
    fn expand_neighbourhood(&mut self, input: crate::views::ExpandNeighbourhood) -> Result<crate::views::ExpandNeighbourhoodOutcome, UnmetObligation> {
        crate::views::obligations::ExpandNeighbourhoodBehavior::expand_neighbourhood(&mut self.ports, input)
    }
}

impl<P: crate::views::obligations::ExportOcelBehavior> crate::views::obligations::ExportOcelBehavior for Generated<P> {
    fn export_ocel(&mut self, input: crate::views::ExportOcel) -> Result<crate::views::ExportOcelOutcome, UnmetObligation> {
        crate::views::obligations::ExportOcelBehavior::export_ocel(&mut self.ports, input)
    }
}

impl<P: crate::views::obligations::FindCodeNamesBehavior> crate::views::obligations::FindCodeNamesBehavior for Generated<P> {
    fn find_code_names(&mut self, input: crate::views::FindCodeNames) -> Result<crate::views::FindCodeNamesOutcome, UnmetObligation> {
        crate::views::obligations::FindCodeNamesBehavior::find_code_names(&mut self.ports, input)
    }
}

impl<P: crate::views::obligations::ProjectGraphBehavior> crate::views::obligations::ProjectGraphBehavior for Generated<P> {
    fn project_graph(&mut self, input: crate::views::ProjectGraph) -> Result<crate::views::ProjectGraphOutcome, UnmetObligation> {
        crate::views::obligations::ProjectGraphBehavior::project_graph(&mut self.ports, input)
    }
}

impl<P: crate::views::obligations::ProjectOverviewBehavior> crate::views::obligations::ProjectOverviewBehavior for Generated<P> {
    fn project_overview(&mut self, input: crate::views::ProjectOverview) -> Result<crate::views::ProjectOverviewOutcome, UnmetObligation> {
        crate::views::obligations::ProjectOverviewBehavior::project_overview(&mut self.ports, input)
    }
}

impl<P: crate::views::obligations::ProjectTimelineBehavior> crate::views::obligations::ProjectTimelineBehavior for Generated<P> {
    fn project_timeline(&mut self, input: crate::views::ProjectTimeline) -> Result<crate::views::ProjectTimelineOutcome, UnmetObligation> {
        crate::views::obligations::ProjectTimelineBehavior::project_timeline(&mut self.ports, input)
    }
}

impl<P: crate::views::obligations::ReportFactQualityBehavior> crate::views::obligations::ReportFactQualityBehavior for Generated<P> {
    fn report_fact_quality(&mut self, input: crate::views::ReportFactQuality) -> Result<crate::views::ReportFactQualityOutcome, UnmetObligation> {
        crate::views::obligations::ReportFactQualityBehavior::report_fact_quality(&mut self.ports, input)
    }
}

impl<P: crate::views::obligations::ReportStoreQualityBehavior> crate::views::obligations::ReportStoreQualityBehavior for Generated<P> {
    fn report_store_quality(&mut self, input: crate::views::ReportStoreQuality) -> Result<crate::views::ReportStoreQualityOutcome, UnmetObligation> {
        crate::views::obligations::ReportStoreQualityBehavior::report_store_quality(&mut self.ports, input)
    }
}

impl<P: crate::views::obligations::SearchNodesBehavior> crate::views::obligations::SearchNodesBehavior for Generated<P> {
    fn search_nodes(&mut self, input: crate::views::SearchNodes) -> Result<crate::views::SearchNodesOutcome, UnmetObligation> {
        crate::views::obligations::SearchNodesBehavior::search_nodes(&mut self.ports, input)
    }
}

impl<P: crate::graph::obligations::AssertionsQuery> crate::graph::obligations::AssertionsQuery for Generated<P> {
    fn assertions(&self) -> Result<Vec<crate::graph::Assertions>, UnmetObligation> {
        crate::graph::obligations::AssertionsQuery::assertions(&self.ports)
    }
}

/// `ekr.graph.SettledAssertions`, generated: every row is one the specification fully determines from the stored `ekr.graph.Assertion`s.
impl<P> crate::graph::obligations::SettledAssertionsQuery for Generated<P>
where
    P: AssertionStorage,
{
    fn settled_assertions(&self) -> Result<Vec<crate::graph::SettledAssertions>, UnmetObligation> {
        let mut admitted = AssertionStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| all(&[equal(Some(&held.data.assessment).map(|value| &value.kind).map(|value| match value { crate::graph::AssessmentKind::Proposed => "Proposed", crate::graph::AssessmentKind::Validating => "Validating", crate::graph::AssessmentKind::Accepted => "Accepted", crate::graph::AssessmentKind::Rejected => "Rejected", crate::graph::AssessmentKind::Disputed => "Disputed" }.to_owned()), Some("Accepted".to_owned())), equal(Some(&held.data.lifecycle).map(|value| &value.kind).map(|value| match value { crate::graph::AssertionLifecycleKind::Active => "Active", crate::graph::AssertionLifecycleKind::Retracted => "Retracted", crate::graph::AssertionLifecycleKind::Superseded => "Superseded" }.to_owned()), Some("Active".to_owned()))]) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::graph::SettledAssertions {
                assertion_id: held.data.assertion_id,
                root_id: held.data.root_id,
                subject_kind: held.data.subject_kind,
                subject: held.data.subject,
                predicate_kind: held.data.predicate_kind,
                predicate: held.data.predicate,
                object_kind: held.data.object_kind,
                object_value: held.data.object_value,
                object_ref: held.data.object_ref,
                proposed_by: held.data.proposed_by,
                assessment: held.data.assessment,
                lifecycle: held.data.lifecycle,
                valid_from: held.data.valid_from,
                valid_to: held.data.valid_to,
                recorded_from: held.data.recorded_from,
                recorded_to: held.data.recorded_to,
            })
            .collect())
    }
}

/// `ekr.integrate.ApplicationReceiptRecords`, generated: every row is one the specification fully determines from the stored `ekr.integrate.ApplicationReceipt`s.
impl<P> crate::integrate::obligations::ApplicationReceiptRecordsQuery for Generated<P>
where
    P: ApplicationReceiptStorage,
{
    fn application_receipt_records(&self) -> Result<Vec<crate::integrate::ApplicationReceiptRecords>, UnmetObligation> {
        let admitted = ApplicationReceiptStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::integrate::ApplicationReceiptRecords {
                receipt_id: held.data.receipt_id,
                state: held.state,
                proposal_id: held.data.proposal_id,
                review_id: held.data.review_id,
                progress: held.data.progress,
                schema_transaction: held.data.schema_transaction,
                schema_revision: held.data.schema_revision,
                processing_receipts: held.data.processing_receipts,
                remaining_items: held.data.remaining_items,
                stop_reason: held.data.stop_reason,
            })
            .collect())
    }
}

/// `ekr.integrate.CanonicalDerivationRecords`, generated: every row is one the specification fully determines from the stored `ekr.integrate.CanonicalDerivation`s.
impl<P> crate::integrate::obligations::CanonicalDerivationRecordsQuery for Generated<P>
where
    P: CanonicalDerivationStorage,
{
    fn canonical_derivation_records(&self) -> Result<Vec<crate::integrate::CanonicalDerivationRecords>, UnmetObligation> {
        let admitted = CanonicalDerivationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::integrate::CanonicalDerivationRecords {
                derivation_id: held.data.derivation_id,
                state: held.state,
                assertion_id: held.data.assertion_id,
                mapping_id: held.data.mapping_id,
                observation_id: held.data.observation_id,
                evidence_id: held.data.evidence_id,
            })
            .collect())
    }
}

/// `ekr.integrate.IntegrationBlockerRecords`, generated: every row is one the specification fully determines from the stored `ekr.integrate.IntegrationBlocker`s.
impl<P> crate::integrate::obligations::IntegrationBlockerRecordsQuery for Generated<P>
where
    P: IntegrationBlockerStorage,
{
    fn integration_blocker_records(&self) -> Result<Vec<crate::integrate::IntegrationBlockerRecords>, UnmetObligation> {
        let admitted = IntegrationBlockerStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::integrate::IntegrationBlockerRecords {
                blocker_id: held.data.blocker_id,
                state: held.state,
                document_digest: held.data.document_digest,
                item: held.data.item,
                kind: held.data.kind,
                declaration: held.data.declaration,
                reason: held.data.reason,
                basis_digest: held.data.basis_digest,
            })
            .collect())
    }
}

/// `ekr.integrate.InterpretationObservationRecords`, generated: every row is one the specification fully determines from the stored `ekr.integrate.InterpretationObservation`s.
impl<P> crate::integrate::obligations::InterpretationObservationRecordsQuery for Generated<P>
where
    P: InterpretationObservationStorage,
{
    fn interpretation_observation_records(&self) -> Result<Vec<crate::integrate::InterpretationObservationRecords>, UnmetObligation> {
        let admitted = InterpretationObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::integrate::InterpretationObservationRecords {
                link_id: held.data.link_id,
                state: held.state,
                document_digest: held.data.document_digest,
                observation_id: held.data.observation_id,
            })
            .collect())
    }
}

/// `ekr.integrate.InterpretationRecords`, generated: every row is one the specification fully determines from the stored `ekr.integrate.Interpretation`s.
impl<P> crate::integrate::obligations::InterpretationRecordsQuery for Generated<P>
where
    P: InterpretationStorage,
{
    fn interpretation_records(&self) -> Result<Vec<crate::integrate::InterpretationRecords>, UnmetObligation> {
        let admitted = InterpretationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::integrate::InterpretationRecords {
                document_digest: held.data.document_digest,
                state: held.state,
                interpretation_id: held.data.interpretation_id,
                version: held.data.version,
                root_id: held.data.root_id,
                document: held.data.document,
            })
            .collect())
    }
}

/// `ekr.integrate.MappingRecordRecords`, generated: every row is one the specification fully determines from the stored `ekr.integrate.MappingRecord`s.
impl<P> crate::integrate::obligations::MappingRecordRecordsQuery for Generated<P>
where
    P: MappingRecordStorage,
{
    fn mapping_record_records(&self) -> Result<Vec<crate::integrate::MappingRecordRecords>, UnmetObligation> {
        let admitted = MappingRecordStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::integrate::MappingRecordRecords {
                mapping_id: held.data.mapping_id,
                state: held.state,
                proposal_digest: held.data.proposal_digest,
                mapping_digest: held.data.mapping_digest,
                source_document_digest: held.data.source_document_digest,
                mapping: held.data.mapping,
                evidence: held.data.evidence,
            })
            .collect())
    }
}

/// `ekr.integrate.ProcessingReceiptRecords`, generated: every row is one the specification fully determines from the stored `ekr.integrate.ProcessingReceipt`s.
impl<P> crate::integrate::obligations::ProcessingReceiptRecordsQuery for Generated<P>
where
    P: ProcessingReceiptStorage,
{
    fn processing_receipt_records(&self) -> Result<Vec<crate::integrate::ProcessingReceiptRecords>, UnmetObligation> {
        let admitted = ProcessingReceiptStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::integrate::ProcessingReceiptRecords {
                mapping_digest: held.data.mapping_digest,
                receipt_id: held.data.receipt_id,
                state: held.state,
                document_digest: held.data.document_digest,
                item: held.data.item,
                disposition: held.data.disposition,
                transaction_id: held.data.transaction_id,
                assertions: held.data.assertions,
                basis_digest: held.data.basis_digest,
            })
            .collect())
    }
}

/// `ekr.integrate.ProposalEvidenceRecords`, generated: every row is one the specification fully determines from the stored `ekr.integrate.ProposalEvidence`s.
impl<P> crate::integrate::obligations::ProposalEvidenceRecordsQuery for Generated<P>
where
    P: ProposalEvidenceStorage,
{
    fn proposal_evidence_records(&self) -> Result<Vec<crate::integrate::ProposalEvidenceRecords>, UnmetObligation> {
        let admitted = ProposalEvidenceStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::integrate::ProposalEvidenceRecords {
                binding_id: held.data.binding_id,
                state: held.state,
                proposal_id: held.data.proposal_id,
                evidence_id: held.data.evidence_id,
            })
            .collect())
    }
}

/// `ekr.integrate.ProposalObservationRecords`, generated: every row is one the specification fully determines from the stored `ekr.integrate.ProposalObservation`s.
impl<P> crate::integrate::obligations::ProposalObservationRecordsQuery for Generated<P>
where
    P: ProposalObservationStorage,
{
    fn proposal_observation_records(&self) -> Result<Vec<crate::integrate::ProposalObservationRecords>, UnmetObligation> {
        let admitted = ProposalObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::integrate::ProposalObservationRecords {
                binding_id: held.data.binding_id,
                state: held.state,
                proposal_id: held.data.proposal_id,
                observation_id: held.data.observation_id,
            })
            .collect())
    }
}

/// `ekr.integrate.ProposalReviewRecords`, generated: every row is one the specification fully determines from the stored `ekr.integrate.ProposalReview`s.
impl<P> crate::integrate::obligations::ProposalReviewRecordsQuery for Generated<P>
where
    P: ProposalReviewStorage,
{
    fn proposal_review_records(&self) -> Result<Vec<crate::integrate::ProposalReviewRecords>, UnmetObligation> {
        let admitted = ProposalReviewStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::integrate::ProposalReviewRecords {
                human_proof_digest: held.data.human_proof_digest,
                review_id: held.data.review_id,
                state: held.state,
                proposal_id: held.data.proposal_id,
                proposal_digest: held.data.proposal_digest,
                basis: held.data.basis,
                decision: held.data.decision,
                operator: held.data.operator,
                evidence_id: held.data.evidence_id,
                recorded_at: held.data.recorded_at,
            })
            .collect())
    }
}

/// `ekr.integrate.ProposalSourceBindingRecords`, generated: every row is one the specification fully determines from the stored `ekr.integrate.ProposalSourceBinding`s.
impl<P> crate::integrate::obligations::ProposalSourceBindingRecordsQuery for Generated<P>
where
    P: ProposalSourceBindingStorage,
{
    fn proposal_source_binding_records(&self) -> Result<Vec<crate::integrate::ProposalSourceBindingRecords>, UnmetObligation> {
        let admitted = ProposalSourceBindingStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::integrate::ProposalSourceBindingRecords {
                binding_id: held.data.binding_id,
                state: held.state,
                proposal_id: held.data.proposal_id,
                document_digest: held.data.document_digest,
            })
            .collect())
    }
}

/// `ekr.integrate.SchemaProposalRecords`, generated: every row is one the specification fully determines from the stored `ekr.integrate.SchemaProposal`s.
impl<P> crate::integrate::obligations::SchemaProposalRecordsQuery for Generated<P>
where
    P: SchemaProposalStorage,
{
    fn schema_proposal_records(&self) -> Result<Vec<crate::integrate::SchemaProposalRecords>, UnmetObligation> {
        let admitted = SchemaProposalStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::integrate::SchemaProposalRecords {
                proposal_id: held.data.proposal_id,
                state: held.state,
                proposal_digest: held.data.proposal_digest,
                base_schema: held.data.base_schema,
                document: held.data.document,
            })
            .collect())
    }
}

/// `ekr.kernel.AuthorityTransitionRecords`, generated: every row is one the specification fully determines from the stored `ekr.kernel.AuthorityTransition`s.
impl<P> crate::kernel::obligations::AuthorityTransitionRecordsQuery for Generated<P>
where
    P: AuthorityTransitionStorage,
{
    fn authority_transition_records(&self) -> Result<Vec<crate::kernel::AuthorityTransitionRecords>, UnmetObligation> {
        let admitted = AuthorityTransitionStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::kernel::AuthorityTransitionRecords {
                review_host_binding_digest: held.data.review_host_binding_digest,
                reviewer_policy_digest: held.data.reviewer_policy_digest,
                trust_enrollment: held.data.trust_enrollment,
                human_proof_digest: held.data.human_proof_digest,
                target_profile_digest: held.data.target_profile_digest,
                transition_id: held.data.transition_id,
                state: held.state,
                format: held.data.format,
                seed_anchor: held.data.seed_anchor,
                predecessor_head: held.data.predecessor_head,
                activation_revision: held.data.activation_revision,
                from: held.data.from,
                to: held.data.to,
                preview_digest: held.data.preview_digest,
                operator: held.data.operator,
                recorded_at: held.data.recorded_at,
            })
            .collect())
    }
}

/// `ekr.kernel.CurrentRevision`, generated: every row is one the specification fully determines from the stored `ekr.kernel.Revision`s.
impl<P> crate::kernel::obligations::CurrentRevisionQuery for Generated<P>
where
    P: RevisionStorage,
{
    fn current_revision(&self) -> Result<Vec<crate::kernel::CurrentRevision>, UnmetObligation> {
        let admitted = RevisionStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::kernel::CurrentRevision {
                revision_id: held.data.revision_id,
                number: held.data.number,
                knowledge_root: held.data.knowledge_root,
            })
            .collect())
    }
}

/// `ekr.kernel.DisputeClaimRecords`, generated: every row is one the specification fully determines from the stored `ekr.kernel.DisputeClaim`s.
impl<P> crate::kernel::obligations::DisputeClaimRecordsQuery for Generated<P>
where
    P: DisputeClaimStorage,
{
    fn dispute_claim_records(&self) -> Result<Vec<crate::kernel::DisputeClaimRecords>, UnmetObligation> {
        let admitted = DisputeClaimStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::kernel::DisputeClaimRecords {
                claim_link_id: held.data.claim_link_id,
                state: held.state,
                dispute_id: held.data.dispute_id,
                assertion_id: held.data.assertion_id,
            })
            .collect())
    }
}

/// `ekr.kernel.DisputeRecords`, generated: every row is one the specification fully determines from the stored `ekr.kernel.Dispute`s.
impl<P> crate::kernel::obligations::DisputeRecordsQuery for Generated<P>
where
    P: DisputeStorage,
{
    fn dispute_records(&self) -> Result<Vec<crate::kernel::DisputeRecords>, UnmetObligation> {
        let admitted = DisputeStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::kernel::DisputeRecords {
                dispute_id: held.data.dispute_id,
                state: held.state,
                basis_revision: held.data.basis_revision,
                subject: held.data.subject,
                predicate: held.data.predicate,
                basis_digest: held.data.basis_digest,
            })
            .collect())
    }
}

/// `ekr.kernel.HumanAnswerRecords`, generated: every row is one the specification fully determines from the stored `ekr.kernel.HumanAnswer`s.
impl<P> crate::kernel::obligations::HumanAnswerRecordsQuery for Generated<P>
where
    P: HumanAnswerStorage,
{
    fn human_answer_records(&self) -> Result<Vec<crate::kernel::HumanAnswerRecords>, UnmetObligation> {
        let admitted = HumanAnswerStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::kernel::HumanAnswerRecords {
                human_proof_digest: held.data.human_proof_digest,
                answer_id: held.data.answer_id,
                state: held.state,
                dispute_id: held.data.dispute_id,
                basis: held.data.basis,
                operator: held.data.operator,
                corrections: held.data.corrections,
                statement_evidence: held.data.statement_evidence,
                transaction_id: held.data.transaction_id,
            })
            .collect())
    }
}

/// `ekr.kernel.HumanDecisionRecords`, generated: every row is one the specification fully determines from the stored `ekr.kernel.RetainedHumanDecision`s.
impl<P> crate::kernel::obligations::HumanDecisionRecordsQuery for Generated<P>
where
    P: RetainedHumanDecisionStorage,
{
    fn human_decision_records(&self) -> Result<Vec<crate::kernel::HumanDecisionRecords>, UnmetObligation> {
        let admitted = RetainedHumanDecisionStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::kernel::HumanDecisionRecords {
                proof_digest: held.data.proof_digest,
                state: held.state,
                decision_id: held.data.decision_id,
                policy_digest: held.data.policy_digest,
                statement_digest: held.data.statement_digest,
                operator: held.data.operator,
                recorded_at: held.data.recorded_at,
            })
            .collect())
    }
}

/// `ekr.kernel.PendingTransactions`, generated: every row is one the specification fully determines from the stored `ekr.kernel.GraphTransaction`s.
impl<P> crate::kernel::obligations::PendingTransactionsQuery for Generated<P>
where
    P: GraphTransactionStorage,
{
    fn pending_transactions(&self) -> Result<Vec<crate::kernel::PendingTransactions>, UnmetObligation> {
        let mut admitted = GraphTransactionStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| equal(Some(&held.state).map(|value| match value { crate::kernel::GraphTransactionState::Committed => "Committed", crate::kernel::GraphTransactionState::Proposed => "Proposed", crate::kernel::GraphTransactionState::Rejected => "Rejected", crate::kernel::GraphTransactionState::Stale => "Stale", crate::kernel::GraphTransactionState::Validated => "Validated" }.to_owned()), Some("Proposed".to_owned())) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::kernel::PendingTransactions {
                transaction_id: held.data.transaction_id,
                proposer: held.data.proposer,
                operation_count: held.data.operation_count,
            })
            .collect())
    }
}

impl<P: crate::kernel::obligations::RejectionsQuery> crate::kernel::obligations::RejectionsQuery for Generated<P> {
    fn rejections(&self, from: Option<crate::kernel::RevisionNumber>, to: Option<crate::kernel::RevisionNumber>) -> Result<Vec<crate::kernel::Rejections>, UnmetObligation> {
        crate::kernel::obligations::RejectionsQuery::rejections(&self.ports, from, to)
    }
}

/// `ekr.kernel.RetainedEvidence`, generated: every row is one the specification fully determines from the stored `ekr.graph.Evidence`s.
impl<P> crate::kernel::obligations::RetainedEvidenceQuery for Generated<P>
where
    P: EvidenceStorage,
{
    fn retained_evidence(&self) -> Result<Vec<crate::kernel::RetainedEvidence>, UnmetObligation> {
        let admitted = EvidenceStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::kernel::RetainedEvidence {
                evidence_id: held.data.evidence_id,
                content_hash: held.data.content_hash,
                extracted_by: held.data.extracted_by,
            })
            .collect())
    }
}

/// `ekr.kernel.Revisions`, generated: every row is one the specification fully determines from the stored `ekr.kernel.Revision`s.
impl<P> crate::kernel::obligations::RevisionsQuery for Generated<P>
where
    P: RevisionStorage,
{
    fn revisions(&self) -> Result<Vec<crate::kernel::Revisions>, UnmetObligation> {
        let admitted = RevisionStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::kernel::Revisions {
                revision_id: held.data.revision_id,
                number: held.data.number,
                parent: held.data.parent,
                ontology_root: held.data.ontology_root,
                knowledge_root: held.data.knowledge_root,
                evidence_root: held.data.evidence_root,
                agent_root: held.data.agent_root,
                transaction_id: held.data.transaction_id,
                committed_at: held.data.committed_at,
                state: held.state,
            })
            .collect())
    }
}

/// `ekr.kernel.SchemaTransactionEvidenceRecords`, generated: every row is one the specification fully determines from the stored `ekr.kernel.SchemaTransactionEvidence`s.
impl<P> crate::kernel::obligations::SchemaTransactionEvidenceRecordsQuery for Generated<P>
where
    P: SchemaTransactionEvidenceStorage,
{
    fn schema_transaction_evidence_records(&self) -> Result<Vec<crate::kernel::SchemaTransactionEvidenceRecords>, UnmetObligation> {
        let admitted = SchemaTransactionEvidenceStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::kernel::SchemaTransactionEvidenceRecords {
                link_id: held.data.link_id,
                state: held.state,
                transaction_id: held.data.transaction_id,
                evidence_id: held.data.evidence_id,
            })
            .collect())
    }
}

/// `ekr.kernel.Transactions`, generated: every row is one the specification fully determines from the stored `ekr.kernel.GraphTransaction`s.
impl<P> crate::kernel::obligations::TransactionsQuery for Generated<P>
where
    P: GraphTransactionStorage,
{
    fn transactions(&self) -> Result<Vec<crate::kernel::Transactions>, UnmetObligation> {
        let admitted = GraphTransactionStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::kernel::Transactions {
                transaction_id: held.data.transaction_id,
                proposer: held.data.proposer,
                document_hash: held.data.document_hash,
                proposal_record_hash: held.data.proposal_record_hash,
                canonical_transaction_hash: held.data.canonical_transaction_hash,
                canonical_operations_hash: held.data.canonical_operations_hash,
                operation_count: held.data.operation_count,
                evidence_hash: held.data.evidence_hash,
                validation_basis: held.data.validation_basis,
                validated_against: held.data.validated_against,
                validation_hash: held.data.validation_hash,
                validation_record_hash: held.data.validation_record_hash,
                terminal_record_hash: held.data.terminal_record_hash,
                state: held.state,
            })
            .collect())
    }
}

/// `ekr.kernel.ValidationIssues`, generated: every row is one the specification fully determines from the stored `ekr.kernel.ValidationIssue`s.
impl<P> crate::kernel::obligations::ValidationIssuesQuery for Generated<P>
where
    P: ValidationIssueStorage,
{
    fn validation_issues(&self) -> Result<Vec<crate::kernel::ValidationIssues>, UnmetObligation> {
        let admitted = ValidationIssueStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::kernel::ValidationIssues {
                issue_id: held.data.issue_id,
                transaction_id: held.data.transaction_id,
                validator: held.data.validator,
                code: held.data.code,
                message: held.data.message,
            })
            .collect())
    }
}

/// `ekr.observe.RetainedObservationRecords`, generated: every row is one the specification fully determines from the stored `ekr.observe.RetainedObservation`s.
impl<P> crate::observe::obligations::RetainedObservationRecordsQuery for Generated<P>
where
    P: RetainedObservationStorage,
{
    fn retained_observation_records(&self) -> Result<Vec<crate::observe::RetainedObservationRecords>, UnmetObligation> {
        let admitted = RetainedObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observe::RetainedObservationRecords {
                observation_id: held.data.observation_id,
                state: held.state,
                key: held.data.key,
                content_hash: held.data.content_hash,
                retained_at: held.data.retained_at,
            })
            .collect())
    }
}

/// `ekr.ontology.NodeTypesByVersion`, generated: every row is one the specification fully determines from the stored `ekr.ontology.NodeType`s.
impl<P> crate::ontology::obligations::NodeTypesByVersionQuery for Generated<P>
where
    P: NodeTypeStorage,
{
    fn node_types_by_version(&self) -> Result<Vec<crate::ontology::NodeTypesByVersion>, UnmetObligation> {
        let mut admitted = NodeTypeStorage::list(&self.ports);
        // `order_by:`, stably: rows equal on every key keep the port's order.
        admitted.sort_by(|left, right| Some(&left.data.name).map(|value| value.clone()).cmp(&Some(&right.data.name).map(|value| value.clone())));
        Ok(admitted
            .into_iter()
            .map(|held| crate::ontology::NodeTypesByVersion {
                type_id: held.data.type_id,
                schema_version_id: held.data.schema_version_id,
                name: held.data.name,
            })
            .collect())
    }
}

/// Three-valued conjunction: false wins, then Unknown.
fn all(truths: &[Option<bool>]) -> Option<bool> {
    if truths.contains(&Some(false)) {
        Some(false)
    } else if truths.contains(&None) {
        None
    } else {
        Some(true)
    }
}

/// Equality of two read values; an unread one is Unknown.
fn equal<T: PartialEq>(left: Option<T>, right: Option<T>) -> Option<bool> {
    Some(left? == right?)
}
