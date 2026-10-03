// generated from ekr v1
// model digest 6e6b51b6bd6e58fd549ea5d5d5562997e603f9e0012f19b9285bfadec393cabc
// contract digest a0934f66cde7acd61a5a40778b7848f6896e36e413fcc19560b8f9a78863f051
// do not edit: regenerate with `ess synthesize`

//! Kernel — `ekr.kernel`.
//!
//! The trusted kernel: stable identity, the transaction boundary and the revision lineage. A proposal becomes a validated transaction only through the validators this domain names, and only a validated transaction commits a revision. Design § 9–10, § 19–20, § 34, § 70–72.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// The states of `ekr.kernel.Agent`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Agent<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    /// `Registered`.
    Registered,
}

/// AgentId — `ekr.kernel.AgentId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentId(pub crate::primitives::Uuid);

/// AliasAdditionProjection — `ekr.kernel.AliasAdditionProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasAdditionProjection {
    /// `node` — `ekr.graph.NodeId`.
    pub node: crate::graph::NodeId,
    /// `alias` — `String`.
    pub alias: String,
}

/// AnswerOutcome — `ekr.kernel.AnswerOutcome`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerOutcome {
    /// `Resolved`.
    Resolved,
    /// `PartiallyResolved`.
    PartiallyResolved,
    /// `Unresolved`.
    Unresolved,
    /// `AlreadyApplied`.
    AlreadyApplied,
}

/// AnswerReceipt — `ekr.kernel.AnswerReceipt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerReceipt {
    /// `answer_id` — `ekr.kernel.HumanAnswerId`.
    pub answer_id: HumanAnswerId,
    /// `outcome` — `ekr.kernel.AnswerOutcome`.
    pub outcome: AnswerOutcome,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<TransactionId>,
    /// `revision` — `Optional<ekr.kernel.RevisionNumber>`.
    pub revision: Option<RevisionNumber>,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    pub evidence_id: crate::graph::EvidenceId,
    /// `remaining` — `List<ekr.kernel.AttentionItem>`.
    pub remaining: Vec<AttentionItem>,
}

/// ApplicationProfileV1 — `ekr.kernel.ApplicationProfileV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationProfileV1(pub String);

/// AttentionAnswerTarget — `ekr.kernel.AttentionAnswerTarget`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttentionAnswerTarget {
    /// `dispute_id` — `ekr.kernel.DisputeId`.
    pub dispute_id: DisputeId,
    /// `basis` — `ekr.kernel.ReviewBasis`.
    pub basis: ReviewBasis,
    /// `corrections_digest` — `ekr.kernel.ContentHash`.
    pub corrections_digest: ContentHash,
}

/// AttentionItem — `ekr.kernel.AttentionItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttentionItem {
    /// `subject` — `ekr.kernel.AttentionSubject`.
    pub subject: AttentionSubject,
    /// `question` — `String`.
    pub question: String,
    /// `basis` — `ekr.kernel.ReviewBasis`.
    pub basis: ReviewBasis,
    /// `claims` — `List<ekr.graph.AssertionId>`.
    pub claims: Vec<crate::graph::AssertionId>,
    /// `evidence` — `List<ekr.graph.EvidenceId>`.
    pub evidence: Vec<crate::graph::EvidenceId>,
    /// `observations` — `List<ekr.graph.ObservationId>`.
    pub observations: Vec<crate::graph::ObservationId>,
}

/// AttentionKind — `ekr.kernel.AttentionKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionKind {
    /// `Dispute`.
    Dispute,
    /// `BlockedIntegration`.
    BlockedIntegration,
    /// `SchemaProposal`.
    SchemaProposal,
}

/// AttentionSubject — `ekr.kernel.AttentionSubject`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttentionSubject {
    /// `kind` — `ekr.kernel.AttentionKind`.
    pub kind: AttentionKind,
    /// `dispute_id` — `Optional<ekr.kernel.DisputeId>`.
    pub dispute_id: Option<DisputeId>,
    /// `blocker_id` — `Optional<ekr.integrate.IntegrationBlockerId>`.
    pub blocker_id: Option<crate::integrate::IntegrationBlockerId>,
    /// `proposal_id` — `Optional<ekr.integrate.SchemaProposalId>`.
    pub proposal_id: Option<crate::integrate::SchemaProposalId>,
}

/// AuthorityStateFormatV1 — `ekr.kernel.AuthorityStateFormatV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityStateFormatV1(pub String);

/// AuthorityStateV1 — `ekr.kernel.AuthorityStateV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityStateV1 {
    /// `format` — `ekr.kernel.AuthorityStateFormatV1`.
    pub format: AuthorityStateFormatV1,
    /// `agents` — `Map<String, ekr.kernel.RegisteredAgent>`.
    pub agents: std::collections::BTreeMap<String, RegisteredAgent>,
    /// `validation_profile` — `ekr.kernel.P1ValidationProfileV1`.
    pub validation_profile: P1ValidationProfileV1,
}

/// The states of `ekr.kernel.AuthorityTransition`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `AuthorityTransition<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityTransitionState {
    /// `Recorded`.
    Recorded,
}

/// AuthorityTransitionFormat — `ekr.kernel.AuthorityTransitionFormat`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityTransitionFormat {
    /// `KnowledgeAuthorityTransition1`.
    KnowledgeAuthorityTransition1,
}

/// AuthorityTransitionId — `ekr.kernel.AuthorityTransitionId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityTransitionId(pub crate::primitives::Uuid);

/// AuthorityUpgradeTarget — `ekr.kernel.AuthorityUpgradeTarget`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityUpgradeTarget {
    /// `preview_digest` — `ekr.kernel.ContentHash`.
    pub preview_digest: ContentHash,
    /// `reviewer_policy_digest` — `ekr.kernel.ContentHash`.
    pub reviewer_policy_digest: ContentHash,
}

/// AuthorityVersion — `ekr.kernel.AuthorityVersion`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityVersion {
    /// `ruleset` — `ekr.kernel.RulesetV1`.
    pub ruleset: RulesetV1,
    /// `application` — `ekr.kernel.ApplicationProfileV1`.
    pub application: ApplicationProfileV1,
    /// `profile_digest` — `ekr.kernel.ContentHash`.
    pub profile_digest: ContentHash,
}

/// BootstrapContext — `ekr.kernel.BootstrapContext`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapContext {
    /// `operator` — `ekr.kernel.AgentId`.
    pub operator: AgentId,
    /// `validator` — `ekr.kernel.AgentId`.
    pub validator: AgentId,
}

/// CanonicalOperationProjection — `ekr.kernel.CanonicalOperationProjection`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalOperationProjection {
    /// Tagged `AddAlias` — `ekr.kernel.AliasAdditionProjection`.
    AddAlias(AliasAdditionProjection),
    /// Tagged `AddAssertion` — `ekr.graph.AssertionRecord`.
    AddAssertion(crate::graph::AssertionRecord),
    /// Tagged `AddEvidence` — `ekr.kernel.EvidenceAdditionProjection`.
    AddEvidence(EvidenceAdditionProjection),
    /// Tagged `AttachEvidence` — `ekr.kernel.EvidenceAttachmentProjection`.
    AttachEvidence(EvidenceAttachmentProjection),
    /// Tagged `CreateEdge` — `ekr.kernel.EdgeDraftProjection`.
    CreateEdge(EdgeDraftProjection),
    /// Tagged `CreateNode` — `ekr.kernel.NodeDraftProjection`.
    CreateNode(NodeDraftProjection),
    /// Tagged `DefineEdgeType` — `ekr.ontology.EdgeTypeDeclaration`.
    DefineEdgeType(crate::ontology::EdgeTypeDeclaration),
    /// Tagged `DefineNodeType` — `ekr.ontology.NodeTypeDeclaration`.
    DefineNodeType(crate::ontology::NodeTypeDeclaration),
    /// Tagged `DeleteEdge` — `ekr.graph.EdgeId`.
    DeleteEdge(crate::graph::EdgeId),
    /// Tagged `Invoke` — `ekr.kernel.InvocationProjection`.
    Invoke(InvocationProjection),
    /// Tagged `MergeEntity` — `ekr.kernel.EntityMerge`.
    MergeEntity(EntityMerge),
    /// Tagged `ModifyProperty` — `ekr.kernel.PropertyModificationProjection`.
    ModifyProperty(PropertyModificationProjection),
    /// Tagged `RetractAssertion` — `ekr.kernel.Retraction`.
    RetractAssertion(Retraction),
    /// Tagged `SupersedeAssertion` — `ekr.kernel.Supersession`.
    SupersedeAssertion(Supersession),
    /// Tagged `UpdateProperty` — `ekr.kernel.PropertyMutationProjection`.
    UpdateProperty(PropertyMutationProjection),
    /// Tagged `WidenEdgeType` — `ekr.kernel.EdgeWideningProjection`.
    WidenEdgeType(EdgeWideningProjection),
}

/// CanonicalTransactionProjection — `ekr.kernel.CanonicalTransactionProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalTransactionProjection {
    /// `id` — `ekr.kernel.TransactionId`.
    pub id: TransactionId,
    /// `proposer` — `ekr.kernel.AgentId`.
    pub proposer: AgentId,
    /// `operations` — `List<ekr.kernel.CanonicalOperationProjection>`.
    pub operations: Vec<CanonicalOperationProjection>,
    /// `evidence` — `List<ekr.graph.EvidenceId>`.
    pub evidence: Vec<crate::graph::EvidenceId>,
    /// `schema_version` — `Optional<ekr.ontology.SchemaVersionId>`.
    pub schema_version: Option<crate::ontology::SchemaVersionId>,
}

/// ClaimCorrection — `ekr.kernel.ClaimCorrection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimCorrection {
    /// `kind` — `ekr.kernel.ClaimCorrectionKind`.
    pub kind: ClaimCorrectionKind,
    /// `assertion_id` — `ekr.graph.AssertionId`.
    pub assertion_id: crate::graph::AssertionId,
    /// `valid_from` — `Optional<Timestamp>`.
    pub valid_from: Option<crate::primitives::Timestamp>,
    /// `valid_to` — `Optional<Timestamp>`.
    pub valid_to: Option<crate::primitives::Timestamp>,
    /// `reason` — `String`.
    pub reason: String,
}

/// ClaimCorrectionKind — `ekr.kernel.ClaimCorrectionKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimCorrectionKind {
    /// `Choose`.
    Choose,
    /// `Retract`.
    Retract,
    /// `CorrectTime`.
    CorrectTime,
    /// `Unresolved`.
    Unresolved,
}

/// CliHostConfigurationV1 — `ekr.kernel.CliHostConfigurationV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliHostConfigurationV1 {
    /// `format` — `ekr.kernel.CliHostFormatV1`.
    pub format: CliHostFormatV1,
    /// `tenant` — `String`.
    pub tenant: String,
    /// `context` — `ekr.kernel.BootstrapContext`.
    pub context: BootstrapContext,
    /// `authority` — `ekr.kernel.AuthorityStateV1`.
    pub authority: AuthorityStateV1,
}

/// CliHostFormatV1 — `ekr.kernel.CliHostFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliHostFormatV1 {
    /// `EkrCliHost1`.
    EkrCliHost1,
}

/// CommitCommandResult — `ekr.kernel.CommitCommandResult`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitCommandResult {
    /// Tagged `Committed` — `ekr.kernel.CommitReceiptV1`.
    Committed(CommitReceiptV1),
    /// Tagged `Stale` — `ekr.kernel.StaleRecordV1`.
    Stale(StaleRecordV1),
}

/// CommitReceiptFormatV1 — `ekr.kernel.CommitReceiptFormatV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitReceiptFormatV1(pub String);

/// CommitReceiptV1 — `ekr.kernel.CommitReceiptV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitReceiptV1 {
    /// `format` — `ekr.kernel.CommitReceiptFormatV1`.
    pub format: CommitReceiptFormatV1,
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `revision_id` — `ekr.kernel.RevisionId`.
    pub revision_id: RevisionId,
    /// `proposal` — `ekr.kernel.ProposalRecordV1`.
    pub proposal: ProposalRecordV1,
    /// `validation` — `ekr.kernel.ValidationReceiptV1`.
    pub validation: ValidationReceiptV1,
    /// `validation_record_hash` — `ekr.kernel.ContentHash`.
    pub validation_record_hash: ContentHash,
    /// `committer` — `ekr.kernel.AgentId`.
    pub committer: AgentId,
    /// `committed_at` — `Timestamp`.
    pub committed_at: crate::primitives::Timestamp,
    /// `result` — `ekr.graph.RevisionRoot`.
    pub result: crate::graph::RevisionRoot,
    /// `result_hash` — `ekr.kernel.ContentHash`.
    pub result_hash: ContentHash,
    /// `created` — `Optional<ekr.kernel.CreatedIdentitiesV1>`.
    pub created: Option<CreatedIdentitiesV1>,
}

/// CommittedPayload — `ekr.kernel.CommittedPayload`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedPayload {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `revision_id` — `ekr.kernel.RevisionId`.
    pub revision_id: RevisionId,
    /// `number` — `ekr.kernel.RevisionNumber`.
    pub number: RevisionNumber,
    /// `knowledge_root` — `ekr.kernel.ContentHash`.
    pub knowledge_root: ContentHash,
}

/// ContentHash — `ekr.kernel.ContentHash`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentHash(pub String);

/// CreatedIdentitiesV1 — `ekr.kernel.CreatedIdentitiesV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedIdentitiesV1 {
    /// `nodes` — `List<ekr.graph.NodeId>`.
    pub nodes: Vec<crate::graph::NodeId>,
    /// `edges` — `List<ekr.graph.EdgeId>`.
    pub edges: Vec<crate::graph::EdgeId>,
}

/// The states of `ekr.kernel.Dispute`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Dispute<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeState {
    /// `Recorded`.
    Recorded,
}

/// The states of `ekr.kernel.DisputeClaim`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `DisputeClaim<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeClaimState {
    /// `Recorded`.
    Recorded,
}

/// DisputeId — `ekr.kernel.DisputeId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeId(pub crate::primitives::Uuid);

/// EdgeDraftProjection — `ekr.kernel.EdgeDraftProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeDraftProjection {
    /// `id` — `ekr.graph.EdgeId`.
    pub id: crate::graph::EdgeId,
    /// `root_id` — `ekr.graph.GraphRootId`.
    pub root_id: crate::graph::GraphRootId,
    /// `type_id` — `ekr.ontology.TypeId`.
    pub type_id: crate::ontology::TypeId,
    /// `source` — `ekr.graph.NodeId`.
    pub source: crate::graph::NodeId,
    /// `target` — `ekr.graph.NodeId`.
    pub target: crate::graph::NodeId,
    /// `properties` — `Map<String, List<ekr.graph.TypedValue>>`.
    pub properties: std::collections::BTreeMap<String, Vec<crate::graph::TypedValue>>,
}

/// EdgeWideningProjection — `ekr.kernel.EdgeWideningProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeWideningProjection {
    /// `edge_type` — `ekr.ontology.TypeId`.
    pub edge_type: crate::ontology::TypeId,
    /// `source_types` — `List<ekr.ontology.TypeId>`.
    pub source_types: Vec<crate::ontology::TypeId>,
    /// `target_types` — `List<ekr.ontology.TypeId>`.
    pub target_types: Vec<crate::ontology::TypeId>,
}

/// EntityMerge — `ekr.kernel.EntityMerge`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityMerge {
    /// `absorbed` — `ekr.graph.NodeId`.
    pub absorbed: crate::graph::NodeId,
    /// `into` — `ekr.graph.NodeId`.
    pub into: crate::graph::NodeId,
}

/// EventId — `ekr.kernel.EventId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventId(pub crate::primitives::Uuid);

/// EvidenceAdditionProjection — `ekr.kernel.EvidenceAdditionProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceAdditionProjection {
    /// `evidence` — `ekr.graph.EvidenceRecord`.
    pub evidence: crate::graph::EvidenceRecord,
    /// `payload` — `Bytes`.
    pub payload: Vec<u8>,
}

/// EvidenceAttachmentProjection — `ekr.kernel.EvidenceAttachmentProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceAttachmentProjection {
    /// `assertion` — `ekr.graph.AssertionId`.
    pub assertion: crate::graph::AssertionId,
    /// `evidence` — `ekr.graph.EvidenceId`.
    pub evidence: crate::graph::EvidenceId,
}

/// ExplainedAttachment — `ekr.kernel.ExplainedAttachment`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainedAttachment {
    /// `assertion_id` — `ekr.graph.AssertionId`.
    pub assertion_id: crate::graph::AssertionId,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    pub evidence_id: crate::graph::EvidenceId,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: RevisionNumber,
    /// `commit` — `ekr.kernel.ExplainedCommit`.
    pub commit: ExplainedCommit,
}

/// ExplainedCommit — `ekr.kernel.ExplainedCommit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainedCommit {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `revision_id` — `ekr.kernel.RevisionId`.
    pub revision_id: RevisionId,
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `committer` — `ekr.kernel.AgentId`.
    pub committer: AgentId,
    /// `committed_at` — `Timestamp`.
    pub committed_at: crate::primitives::Timestamp,
    /// `result` — `ekr.graph.RevisionRoot`.
    pub result: crate::graph::RevisionRoot,
    /// `result_hash` — `ekr.kernel.ContentHash`.
    pub result_hash: ContentHash,
    /// `record_hash` — `ekr.kernel.ContentHash`.
    pub record_hash: ContentHash,
    /// `proposal_record_hash` — `ekr.kernel.ContentHash`.
    pub proposal_record_hash: ContentHash,
    /// `validation_record_hash` — `ekr.kernel.ContentHash`.
    pub validation_record_hash: ContentHash,
}

/// ExplainedLifecycle — `ekr.kernel.ExplainedLifecycle`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainedLifecycle {
    /// `assertion_id` — `ekr.graph.AssertionId`.
    pub assertion_id: crate::graph::AssertionId,
    /// `lifecycle` — `ekr.graph.AssertionLifecycleProjection`.
    pub lifecycle: crate::graph::AssertionLifecycleProjection,
    /// `commit` — `ekr.kernel.ExplainedCommit`.
    pub commit: ExplainedCommit,
}

/// ExplainedProposal — `ekr.kernel.ExplainedProposal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainedProposal {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `record_hash` — `ekr.kernel.ContentHash`.
    pub record_hash: ContentHash,
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `submitter` — `ekr.kernel.AgentId`.
    pub submitter: AgentId,
    /// `submitted_at` — `Timestamp`.
    pub submitted_at: crate::primitives::Timestamp,
    /// `document_hash` — `ekr.kernel.ContentHash`.
    pub document_hash: ContentHash,
    /// `operation_count` — `Integer`.
    pub operation_count: i64,
    /// `operations` — `List<ekr.kernel.CanonicalOperationProjection>`.
    pub operations: Vec<CanonicalOperationProjection>,
}

/// ExplainedSeed — `ekr.kernel.ExplainedSeed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainedSeed {
    /// `result` — `ekr.kernel.SeedResultV1`.
    pub result: SeedResultV1,
    /// `context` — `ekr.kernel.BootstrapContext`.
    pub context: BootstrapContext,
    /// `validation_profile` — `ekr.kernel.P1ValidationProfileV1`.
    pub validation_profile: P1ValidationProfileV1,
    /// `record_hash` — `ekr.kernel.ContentHash`.
    pub record_hash: ContentHash,
}

/// ExplainedValidation — `ekr.kernel.ExplainedValidation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainedValidation {
    /// `receipt` — `ekr.kernel.ValidationReceiptV1`.
    pub receipt: ValidationReceiptV1,
    /// `validation_profile` — `ekr.kernel.P1ValidationProfileV1`.
    pub validation_profile: P1ValidationProfileV1,
    /// `record_hash` — `ekr.kernel.ContentHash`.
    pub record_hash: ContentHash,
}

/// ExplanationFormatV2 — `ekr.kernel.ExplanationFormatV2`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplanationFormatV2 {
    /// `EkrExplanation2`.
    EkrExplanation2,
}

/// ExplanationLink — `ekr.kernel.ExplanationLink`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplanationLink {
    /// Tagged `Assertion` — `ekr.graph.AssertionRecord`.
    Assertion(crate::graph::AssertionRecord),
    /// Tagged `Attachment` — `ekr.kernel.ExplainedAttachment`.
    Attachment(ExplainedAttachment),
    /// Tagged `Commit` — `ekr.kernel.ExplainedCommit`.
    Commit(ExplainedCommit),
    /// Tagged `Evidence` — `ekr.graph.EvidenceRecord`.
    Evidence(crate::graph::EvidenceRecord),
    /// Tagged `Lifecycle` — `ekr.kernel.ExplainedLifecycle`.
    Lifecycle(ExplainedLifecycle),
    /// Tagged `Proposal` — `ekr.kernel.ExplainedProposal`.
    Proposal(ExplainedProposal),
    /// Tagged `Seed` — `ekr.kernel.ExplainedSeed`.
    Seed(ExplainedSeed),
    /// Tagged `Validation` — `ekr.kernel.ExplainedValidation`.
    Validation(ExplainedValidation),
}

/// ExplanationResult — `ekr.kernel.ExplanationResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplanationResult {
    /// `format` — `ekr.kernel.ExplanationFormatV2`.
    pub format: ExplanationFormatV2,
    /// `assertion_id` — `ekr.graph.AssertionId`.
    pub assertion_id: crate::graph::AssertionId,
    /// `at` — `ekr.kernel.RevisionNumber`.
    pub at: RevisionNumber,
    /// `links` — `List<ekr.kernel.ExplanationLink>`.
    pub links: Vec<ExplanationLink>,
}

/// The states of `ekr.kernel.GraphTransaction`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `GraphTransaction<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphTransactionState {
    /// `Committed`.
    Committed,
    /// `Proposed`.
    Proposed,
    /// `Rejected`.
    Rejected,
    /// `Stale`.
    Stale,
    /// `Validated`.
    Validated,
}

/// The states of `ekr.kernel.HumanAnswer`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `HumanAnswer<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanAnswerState {
    /// `Recorded`.
    Recorded,
}

/// HumanAnswerId — `ekr.kernel.HumanAnswerId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanAnswerId(pub crate::primitives::Uuid);

/// HumanDecisionAudience — `ekr.kernel.HumanDecisionAudience`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanDecisionAudience {
    /// `tenant` — `String`.
    pub tenant: String,
    /// `seed_anchor` — `ekr.kernel.ContentHash`.
    pub seed_anchor: ContentHash,
}

/// HumanDecisionFormat — `ekr.kernel.HumanDecisionFormat`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanDecisionFormat {
    /// `HumanDecision1`.
    HumanDecision1,
}

/// HumanDecisionIntent — `ekr.kernel.HumanDecisionIntent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanDecisionIntent {
    /// `format` — `ekr.kernel.HumanDecisionFormat`.
    pub format: HumanDecisionFormat,
    /// `decision_id` — `Uuid`.
    pub decision_id: crate::primitives::Uuid,
    /// `audience` — `ekr.kernel.HumanDecisionAudience`.
    pub audience: HumanDecisionAudience,
    /// `reviewer_policy_digest` — `ekr.kernel.ContentHash`.
    pub reviewer_policy_digest: ContentHash,
    /// `signer_key_digest` — `ekr.kernel.ContentHash`.
    pub signer_key_digest: ContentHash,
    /// `target` — `ekr.kernel.HumanDecisionTarget`.
    pub target: HumanDecisionTarget,
    /// `statement_digest` — `ekr.kernel.ContentHash`.
    pub statement_digest: ContentHash,
    /// `expected_previous_decision` — `Optional<ekr.kernel.ContentHash>`.
    pub expected_previous_decision: Option<ContentHash>,
}

/// HumanDecisionRecord — `ekr.kernel.HumanDecisionRecord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanDecisionRecord {
    /// `decision_id` — `Uuid`.
    pub decision_id: crate::primitives::Uuid,
    /// `proof_digest` — `ekr.kernel.ContentHash`.
    pub proof_digest: ContentHash,
    /// `policy_digest` — `ekr.kernel.ContentHash`.
    pub policy_digest: ContentHash,
    /// `statement_digest` — `ekr.kernel.ContentHash`.
    pub statement_digest: ContentHash,
    /// `operator` — `ekr.kernel.TrustedOperatorIdentity`.
    pub operator: TrustedOperatorIdentity,
    /// `recorded_at` — `Timestamp`.
    pub recorded_at: crate::primitives::Timestamp,
}

/// HumanDecisionScope — `ekr.kernel.HumanDecisionScope`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanDecisionScope {
    /// `AnswerAttention`.
    AnswerAttention,
    /// `ApproveSchemaProposal`.
    ApproveSchemaProposal,
    /// `RejectSchemaProposal`.
    RejectSchemaProposal,
    /// `UpgradeAuthority`.
    UpgradeAuthority,
}

/// HumanDecisionTarget — `ekr.kernel.HumanDecisionTarget`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HumanDecisionTarget {
    /// Tagged `AnswerAttention` — `ekr.kernel.AttentionAnswerTarget`.
    AnswerAttention(AttentionAnswerTarget),
    /// Tagged `ApproveSchemaProposal` — `ekr.kernel.SchemaReviewTarget`.
    ApproveSchemaProposal(SchemaReviewTarget),
    /// Tagged `RejectSchemaProposal` — `ekr.kernel.SchemaReviewTarget`.
    RejectSchemaProposal(SchemaReviewTarget),
    /// Tagged `UpgradeAuthority` — `ekr.kernel.AuthorityUpgradeTarget`.
    UpgradeAuthority(AuthorityUpgradeTarget),
}

/// InvocationProjection — `ekr.kernel.InvocationProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationProjection {
    /// `node` — `ekr.graph.NodeId`.
    pub node: crate::graph::NodeId,
    /// `operation` — `String`.
    pub operation: String,
    /// `arguments` — `Map<String, ekr.graph.TypedValue>`.
    pub arguments: std::collections::BTreeMap<String, crate::graph::TypedValue>,
}

/// IssueId — `ekr.kernel.IssueId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueId(pub crate::primitives::Uuid);

/// MigratedOccurrence — `ekr.kernel.MigratedOccurrence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigratedOccurrence {
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `event` — `String`.
    pub event: String,
    /// `source_record_hash` — `ekr.kernel.ContentHash`.
    pub source_record_hash: ContentHash,
    /// `destination_record_hash` — `ekr.kernel.ContentHash`.
    pub destination_record_hash: ContentHash,
}

/// NodeDraftProjection — `ekr.kernel.NodeDraftProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDraftProjection {
    /// `id` — `ekr.graph.NodeId`.
    pub id: crate::graph::NodeId,
    /// `root_id` — `ekr.graph.GraphRootId`.
    pub root_id: crate::graph::GraphRootId,
    /// `type_id` — `ekr.ontology.TypeId`.
    pub type_id: crate::ontology::TypeId,
    /// `canonical_name` — `String`.
    pub canonical_name: String,
    /// `properties` — `Map<String, List<ekr.graph.TypedValue>>`.
    pub properties: std::collections::BTreeMap<String, Vec<crate::graph::TypedValue>>,
    /// `aliases` — `Optional<List<String>>`.
    pub aliases: Option<Vec<String>>,
}

/// OperationKind — `ekr.kernel.OperationKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    /// `CreateNode`.
    CreateNode,
    /// `UpdateProperty`.
    UpdateProperty,
    /// `CreateEdge`.
    CreateEdge,
    /// `DeleteEdge`.
    DeleteEdge,
    /// `AddAssertion`.
    AddAssertion,
    /// `RetractAssertion`.
    RetractAssertion,
    /// `DefineNodeType`.
    DefineNodeType,
    /// `DefineEdgeType`.
    DefineEdgeType,
    /// `ModifyProperty`.
    ModifyProperty,
    /// `MergeEntity`.
    MergeEntity,
    /// `Invoke`.
    Invoke,
    /// `SupersedeAssertion`.
    SupersedeAssertion,
    /// `AddEvidence`.
    AddEvidence,
    /// `WidenEdgeType`.
    WidenEdgeType,
    /// `AddAlias`.
    AddAlias,
    /// `AttachEvidence`.
    AttachEvidence,
}

/// P1ValidationProfileV1 — `ekr.kernel.P1ValidationProfileV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct P1ValidationProfileV1 {
    /// `format` — `ekr.kernel.ValidationProfileFormatV1`.
    pub format: ValidationProfileFormatV1,
    /// `ruleset` — `ekr.kernel.RulesetV1`.
    pub ruleset: RulesetV1,
    /// `checks` — `List<ekr.kernel.ValidatorName>`.
    pub checks: Vec<ValidatorName>,
    /// `validator` — `ekr.kernel.AgentId`.
    pub validator: AgentId,
    /// `proposer_separation` — `ekr.kernel.ProposerSeparationV1`.
    pub proposer_separation: ProposerSeparationV1,
    /// `provenance` — `ekr.kernel.ProvenanceProfileV1`.
    pub provenance: ProvenanceProfileV1,
    /// `application` — `ekr.kernel.ApplicationProfileV1`.
    pub application: ApplicationProfileV1,
}

/// PropertyModificationProjection — `ekr.kernel.PropertyModificationProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyModificationProjection {
    /// `owner` — `Optional<ekr.ontology.TypeId>`.
    pub owner: Option<crate::ontology::TypeId>,
    /// `property` — `ekr.ontology.PropertyDeclaration`.
    pub property: crate::ontology::PropertyDeclaration,
}

/// PropertyMutationProjection — `ekr.kernel.PropertyMutationProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyMutationProjection {
    /// `node` — `ekr.graph.NodeId`.
    pub node: crate::graph::NodeId,
    /// `property` — `ekr.ontology.PropertyId`.
    pub property: crate::ontology::PropertyId,
    /// `values` — `List<ekr.graph.TypedValue>`.
    pub values: Vec<crate::graph::TypedValue>,
}

/// ProposalRecordFormatV1 — `ekr.kernel.ProposalRecordFormatV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalRecordFormatV1(pub String);

/// ProposalRecordV1 — `ekr.kernel.ProposalRecordV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalRecordV1 {
    /// `format` — `ekr.kernel.ProposalRecordFormatV1`.
    pub format: ProposalRecordFormatV1,
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `submitted_at` — `Timestamp`.
    pub submitted_at: crate::primitives::Timestamp,
    /// `submitter` — `ekr.kernel.AgentId`.
    pub submitter: AgentId,
    /// `document_hash` — `ekr.kernel.ContentHash`.
    pub document_hash: ContentHash,
    /// `document_bytes` — `Bytes`.
    pub document_bytes: Vec<u8>,
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `operation_count` — `Integer`.
    pub operation_count: i64,
    /// `evidence_hash` — `ekr.kernel.ContentHash`.
    pub evidence_hash: ContentHash,
    /// `canonical_transaction_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub canonical_transaction_hash: Option<ContentHash>,
    /// `canonical_operations_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub canonical_operations_hash: Option<ContentHash>,
}

/// ProposedPayload — `ekr.kernel.ProposedPayload`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedPayload {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `proposer` — `ekr.kernel.AgentId`.
    pub proposer: AgentId,
    /// `operations_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub operations_hash: Option<ContentHash>,
}

/// ProposerSeparationV1 — `ekr.kernel.ProposerSeparationV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposerSeparationV1(pub String);

/// ProvenanceProfileV1 — `ekr.kernel.ProvenanceProfileV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceProfileV1(pub String);

/// RegisteredAgent — `ekr.kernel.RegisteredAgent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredAgent {
    /// `id` — `ekr.kernel.AgentId`.
    pub id: AgentId,
    /// `name` — `String`.
    pub name: String,
    /// `capabilities` — `List<String>`.
    pub capabilities: Vec<String>,
}

/// RejectedPayload — `ekr.kernel.RejectedPayload`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedPayload {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `issues` — `Integer`.
    pub issues: i64,
}

/// RejectedTransactionV1 — `ekr.kernel.RejectedTransactionV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedTransactionV1 {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `against` — `ekr.kernel.RevisionNumber`.
    pub against: RevisionNumber,
    /// `proposer` — `ekr.kernel.AgentId`.
    pub proposer: AgentId,
    /// `rejected_at` — `Timestamp`.
    pub rejected_at: crate::primitives::Timestamp,
    /// `issues` — `List<ekr.kernel.ValidationIssueRecord>`.
    pub issues: Vec<ValidationIssueRecord>,
}

/// RejectionRecordFormatV1 — `ekr.kernel.RejectionRecordFormatV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectionRecordFormatV1(pub String);

/// RejectionRecordV1 — `ekr.kernel.RejectionRecordV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectionRecordV1 {
    /// `format` — `ekr.kernel.RejectionRecordFormatV1`.
    pub format: RejectionRecordFormatV1,
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `proposed_event_id` — `ekr.kernel.EventId`.
    pub proposed_event_id: EventId,
    /// `proposal_record_hash` — `ekr.kernel.ContentHash`.
    pub proposal_record_hash: ContentHash,
    /// `requested_basis` — `ekr.kernel.ValidationBasisV1`.
    pub requested_basis: ValidationBasisV1,
    /// `validator` — `ekr.kernel.AgentId`.
    pub validator: AgentId,
    /// `rejected_at` — `Timestamp`.
    pub rejected_at: crate::primitives::Timestamp,
    /// `issues` — `List<ekr.kernel.ValidationIssueRecord>`.
    pub issues: Vec<ValidationIssueRecord>,
}

/// RejectionsFormatV1 — `ekr.kernel.RejectionsFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionsFormatV1 {
    /// `EkrRejections1`.
    EkrRejections1,
}

/// RejectionsV1 — `ekr.kernel.RejectionsV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectionsV1 {
    /// `format` — `ekr.kernel.RejectionsFormatV1`.
    pub format: RejectionsFormatV1,
    /// `from` — `Optional<ekr.kernel.RevisionNumber>`.
    pub from: Option<RevisionNumber>,
    /// `to` — `Optional<ekr.kernel.RevisionNumber>`.
    pub to: Option<RevisionNumber>,
    /// `rejections` — `List<ekr.kernel.RejectedTransactionV1>`.
    pub rejections: Vec<RejectedTransactionV1>,
}

/// The states of `ekr.kernel.RetainedHumanDecision`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `RetainedHumanDecision<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainedHumanDecisionState {
    /// `Verified`.
    Verified,
}

/// Retraction — `ekr.kernel.Retraction`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retraction {
    /// `assertion` — `ekr.graph.AssertionId`.
    pub assertion: crate::graph::AssertionId,
    /// `reason` — `String`.
    pub reason: String,
}

/// ReviewBasis — `ekr.kernel.ReviewBasis`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewBasis {
    /// `observed_revision` — `ekr.kernel.RevisionNumber`.
    pub observed_revision: RevisionNumber,
    /// `evidence_digest` — `ekr.kernel.ContentHash`.
    pub evidence_digest: ContentHash,
    /// `options_digest` — `ekr.kernel.ContentHash`.
    pub options_digest: ContentHash,
    /// `effects_digest` — `ekr.kernel.ContentHash`.
    pub effects_digest: ContentHash,
}

/// ReviewSignatureAlgorithm — `ekr.kernel.ReviewSignatureAlgorithm`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewSignatureAlgorithm {
    /// `Ed25519`.
    Ed25519,
}

/// ReviewerTrustEnrollment — `ekr.kernel.ReviewerTrustEnrollment`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerTrustEnrollment {
    /// `audience` — `ekr.kernel.HumanDecisionAudience`.
    pub audience: HumanDecisionAudience,
    /// `policy_digest` — `ekr.kernel.ContentHash`.
    pub policy_digest: ContentHash,
    /// `host_binding_digest` — `ekr.kernel.ContentHash`.
    pub host_binding_digest: ContentHash,
    /// `upgrade_proof_digest` — `ekr.kernel.ContentHash`.
    pub upgrade_proof_digest: ContentHash,
}

/// ReviewerTrustFormat — `ekr.kernel.ReviewerTrustFormat`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewerTrustFormat {
    /// `ReviewerTrust1`.
    ReviewerTrust1,
}

/// ReviewerTrustPolicy — `ekr.kernel.ReviewerTrustPolicy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerTrustPolicy {
    /// `format` — `ekr.kernel.ReviewerTrustFormat`.
    pub format: ReviewerTrustFormat,
    /// `audience` — `ekr.kernel.HumanDecisionAudience`.
    pub audience: HumanDecisionAudience,
    /// `keys` — `List<ekr.kernel.ReviewerVerificationKey>`.
    pub keys: Vec<ReviewerVerificationKey>,
}

/// ReviewerVerificationKey — `ekr.kernel.ReviewerVerificationKey`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerVerificationKey {
    /// `key_digest` — `ekr.kernel.ContentHash`.
    pub key_digest: ContentHash,
    /// `algorithm` — `ekr.kernel.ReviewSignatureAlgorithm`.
    pub algorithm: ReviewSignatureAlgorithm,
    /// `public_key` — `Bytes`.
    pub public_key: Vec<u8>,
    /// `operator` — `ekr.kernel.TrustedOperatorIdentity`.
    pub operator: TrustedOperatorIdentity,
    /// `scopes` — `List<ekr.kernel.HumanDecisionScope>`.
    pub scopes: Vec<HumanDecisionScope>,
}

/// The states of `ekr.kernel.Revision`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Revision<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionState {
    /// `Committed`.
    Committed,
}

/// RevisionEventFormatV2 — `ekr.kernel.RevisionEventFormatV2`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionEventFormatV2(pub String);

/// RevisionEventV2 — `ekr.kernel.RevisionEventV2`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionEventV2 {
    /// `format` — `ekr.kernel.RevisionEventFormatV2`.
    pub format: RevisionEventFormatV2,
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `record_hash` — `ekr.kernel.ContentHash`.
    pub record_hash: ContentHash,
    /// `payload` — `ekr.kernel.RevisionPayload`.
    pub payload: RevisionPayload,
}

/// RevisionId — `ekr.kernel.RevisionId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionId(pub crate::primitives::Uuid);

/// RevisionNumber — `ekr.kernel.RevisionNumber`: a distinct wrapper around `Integer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionNumber(pub i64);

/// RevisionPayload — `ekr.kernel.RevisionPayload`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevisionPayload {
    /// Tagged `RevisionCommitted` — `ekr.kernel.CommittedPayload`.
    RevisionCommitted(CommittedPayload),
    /// Tagged `Seeded` — `ekr.kernel.SeededPayload`.
    Seeded(SeededPayload),
    /// Tagged `TransactionProposed` — `ekr.kernel.ProposedPayload`.
    TransactionProposed(ProposedPayload),
    /// Tagged `TransactionRejected` — `ekr.kernel.RejectedPayload`.
    TransactionRejected(RejectedPayload),
    /// Tagged `TransactionStale` — `ekr.kernel.StalePayload`.
    TransactionStale(StalePayload),
    /// Tagged `TransactionValidated` — `ekr.kernel.ValidatedPayload`.
    TransactionValidated(ValidatedPayload),
}

/// RulesetV1 — `ekr.kernel.RulesetV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulesetV1(pub String);

/// SchemaReviewTarget — `ekr.kernel.SchemaReviewTarget`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaReviewTarget {
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: crate::integrate::SchemaProposalId,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: ContentHash,
    /// `basis` — `ekr.kernel.ReviewBasis`.
    pub basis: ReviewBasis,
}

/// The states of `ekr.kernel.SchemaTransactionEvidence`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `SchemaTransactionEvidence<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaTransactionEvidenceState {
    /// `Recorded`.
    Recorded,
}

/// SeedDocumentFormatV2 — `ekr.kernel.SeedDocumentFormatV2`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedDocumentFormatV2(pub String);

/// SeedDocumentPath — `ekr.kernel.SeedDocumentPath`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedDocumentPath(pub String);

/// SeedEnvelopeFormatV2 — `ekr.kernel.SeedEnvelopeFormatV2`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedEnvelopeFormatV2(pub String);

/// SeedEnvelopeFormatV3 — `ekr.kernel.SeedEnvelopeFormatV3`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedEnvelopeFormatV3(pub String);

/// SeedEnvelopeV2Projection — `ekr.kernel.SeedEnvelopeV2Projection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedEnvelopeV2Projection {
    /// `format` — `ekr.kernel.SeedEnvelopeFormatV2`.
    pub format: SeedEnvelopeFormatV2,
    /// `input` — `ekr.kernel.SeedInputV2Projection`.
    pub input: SeedInputV2Projection,
    /// `context` — `ekr.kernel.BootstrapContext`.
    pub context: BootstrapContext,
    /// `authority` — `ekr.kernel.AuthorityStateV1`.
    pub authority: AuthorityStateV1,
    /// `committed_at` — `Timestamp`.
    pub committed_at: crate::primitives::Timestamp,
}

/// SeedEnvelopeV3Projection — `ekr.kernel.SeedEnvelopeV3Projection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedEnvelopeV3Projection {
    /// `format` — `ekr.kernel.SeedEnvelopeFormatV3`.
    pub format: SeedEnvelopeFormatV3,
    /// `input` — `ekr.kernel.SeedInputV3Projection`.
    pub input: SeedInputV3Projection,
    /// `context` — `ekr.kernel.BootstrapContext`.
    pub context: BootstrapContext,
    /// `authority` — `ekr.kernel.AuthorityStateV1`.
    pub authority: AuthorityStateV1,
    /// `committed_at` — `Timestamp`.
    pub committed_at: crate::primitives::Timestamp,
}

/// SeedInputV2Projection — `ekr.kernel.SeedInputV2Projection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedInputV2Projection {
    /// `format` — `ekr.kernel.SeedDocumentFormatV2`.
    pub format: SeedDocumentFormatV2,
    /// `ontology` — `ekr.ontology.OntologyDocumentProjection`.
    pub ontology: crate::ontology::OntologyDocumentProjection,
    /// `graph` — `ekr.graph.GraphDocumentV2Projection`.
    pub graph: crate::graph::GraphDocumentV2Projection,
    /// `evidence_payloads` — `Map<String, Bytes>`.
    pub evidence_payloads: std::collections::BTreeMap<String, Vec<u8>>,
}

/// SeedInputV3Projection — `ekr.kernel.SeedInputV3Projection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedInputV3Projection {
    /// `format` — `ekr.kernel.SeedDocumentFormatV2`.
    pub format: SeedDocumentFormatV2,
    /// `ontology` — `ekr.ontology.OntologyDocumentProjection`.
    pub ontology: crate::ontology::OntologyDocumentProjection,
    /// `graph` — `ekr.graph.GraphDocumentV2Projection`.
    pub graph: crate::graph::GraphDocumentV2Projection,
    /// `evidence_payloads` — `List<ekr.kernel.ContentHash>`.
    pub evidence_payloads: Vec<ContentHash>,
}

/// SeedResultFormatV1 — `ekr.kernel.SeedResultFormatV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedResultFormatV1(pub String);

/// SeedResultV1 — `ekr.kernel.SeedResultV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedResultV1 {
    /// `format` — `ekr.kernel.SeedResultFormatV1`.
    pub format: SeedResultFormatV1,
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `revision_id` — `ekr.kernel.RevisionId`.
    pub revision_id: RevisionId,
    /// `seed_hash` — `ekr.kernel.ContentHash`.
    pub seed_hash: ContentHash,
    /// `authority_root` — `ekr.kernel.ContentHash`.
    pub authority_root: ContentHash,
    /// `committed_at` — `Timestamp`.
    pub committed_at: crate::primitives::Timestamp,
    /// `result` — `ekr.graph.RevisionRoot`.
    pub result: crate::graph::RevisionRoot,
    /// `result_hash` — `ekr.kernel.ContentHash`.
    pub result_hash: ContentHash,
}

/// SeededPayload — `ekr.kernel.SeededPayload`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeededPayload {
    /// `revision_id` — `ekr.kernel.RevisionId`.
    pub revision_id: RevisionId,
    /// `seed_hash` — `ekr.kernel.ContentHash`.
    pub seed_hash: ContentHash,
}

/// SignedHumanDecision — `ekr.kernel.SignedHumanDecision`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedHumanDecision {
    /// `intent` — `ekr.kernel.HumanDecisionIntent`.
    pub intent: HumanDecisionIntent,
    /// `algorithm` — `ekr.kernel.ReviewSignatureAlgorithm`.
    pub algorithm: ReviewSignatureAlgorithm,
    /// `signature` — `Bytes`.
    pub signature: Vec<u8>,
}

/// SnapshotResult — `ekr.kernel.SnapshotResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotResult {
    /// `revision_id` — `ekr.kernel.RevisionId`.
    pub revision_id: RevisionId,
    /// `root` — `ekr.graph.RevisionRoot`.
    pub root: crate::graph::RevisionRoot,
    /// `graph` — `ekr.graph.GraphDocumentV2Projection`.
    pub graph: crate::graph::GraphDocumentV2Projection,
    /// `valid_at` — `Optional<Timestamp>`.
    pub valid_at: Option<crate::primitives::Timestamp>,
    /// `matching_assertions` — `Optional<List<ekr.graph.AssertionId>>`.
    pub matching_assertions: Option<Vec<crate::graph::AssertionId>>,
}

/// StalePayload — `ekr.kernel.StalePayload`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StalePayload {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `validated_against` — `ekr.kernel.RevisionNumber`.
    pub validated_against: RevisionNumber,
    /// `current` — `ekr.kernel.RevisionNumber`.
    pub current: RevisionNumber,
}

/// StaleRecordFormatV1 — `ekr.kernel.StaleRecordFormatV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleRecordFormatV1(pub String);

/// StaleRecordV1 — `ekr.kernel.StaleRecordV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleRecordV1 {
    /// `format` — `ekr.kernel.StaleRecordFormatV1`.
    pub format: StaleRecordFormatV1,
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `validation_record_hash` — `ekr.kernel.ContentHash`.
    pub validation_record_hash: ContentHash,
    /// `expected_basis` — `ekr.kernel.ValidationBasisV1`.
    pub expected_basis: ValidationBasisV1,
    /// `observed_revision_id` — `ekr.kernel.RevisionId`.
    pub observed_revision_id: RevisionId,
    /// `observed_event_id` — `ekr.kernel.EventId`.
    pub observed_event_id: EventId,
    /// `observed_record_hash` — `ekr.kernel.ContentHash`.
    pub observed_record_hash: ContentHash,
    /// `observed_root` — `ekr.graph.RevisionRoot`.
    pub observed_root: crate::graph::RevisionRoot,
    /// `observed_root_hash` — `ekr.kernel.ContentHash`.
    pub observed_root_hash: ContentHash,
    /// `stale_at` — `Timestamp`.
    pub stale_at: crate::primitives::Timestamp,
}

/// StoreMigrationV1 — `ekr.kernel.StoreMigrationV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreMigrationV1 {
    /// `format` — `String`.
    pub format: String,
    /// `source_seed_hash` — `ekr.kernel.ContentHash`.
    pub source_seed_hash: ContentHash,
    /// `destination_seed_hash` — `ekr.kernel.ContentHash`.
    pub destination_seed_hash: ContentHash,
    /// `occurrences` — `List<ekr.kernel.MigratedOccurrence>`.
    pub occurrences: Vec<MigratedOccurrence>,
    /// `carried_objects` — `List<ekr.kernel.ContentHash>`.
    pub carried_objects: Vec<ContentHash>,
    /// `legacy_objects` — `List<ekr.kernel.ContentHash>`.
    pub legacy_objects: Vec<ContentHash>,
    /// `map_hash` — `ekr.kernel.ContentHash`.
    pub map_hash: ContentHash,
}

/// Supersession — `ekr.kernel.Supersession`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supersession {
    /// `assertion` — `ekr.graph.AssertionId`.
    pub assertion: crate::graph::AssertionId,
    /// `by` — `ekr.graph.AssertionId`.
    pub by: crate::graph::AssertionId,
    /// `effective_from` — `Timestamp`.
    pub effective_from: crate::primitives::Timestamp,
}

/// TransactionDocumentFormatV1 — `ekr.kernel.TransactionDocumentFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionDocumentFormatV1 {
    /// `EkrTransactionDocument1`.
    EkrTransactionDocument1,
    /// `EkrTransactionDocument2`.
    EkrTransactionDocument2,
}

/// TransactionDocumentPath — `ekr.kernel.TransactionDocumentPath`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionDocumentPath(pub String);

/// TransactionId — `ekr.kernel.TransactionId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionId(pub crate::primitives::Uuid);

/// TrustedOperatorIdentity — `ekr.kernel.TrustedOperatorIdentity`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedOperatorIdentity {
    /// `actor` — `ekr.kernel.AgentId`.
    pub actor: AgentId,
    /// `authentication_subject` — `String`.
    pub authentication_subject: String,
}

/// TrustedReviewHostBinding — `ekr.kernel.TrustedReviewHostBinding`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedReviewHostBinding {
    /// `audience` — `ekr.kernel.HumanDecisionAudience`.
    pub audience: HumanDecisionAudience,
    /// `reviewer_policy_digest` — `ekr.kernel.ContentHash`.
    pub reviewer_policy_digest: ContentHash,
}

/// UpgradeContradiction — `ekr.kernel.UpgradeContradiction`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeContradiction {
    /// `left` — `ekr.graph.AssertionId`.
    pub left: crate::graph::AssertionId,
    /// `right` — `ekr.graph.AssertionId`.
    pub right: crate::graph::AssertionId,
    /// `subject` — `ekr.graph.SubjectProjection`.
    pub subject: crate::graph::SubjectProjection,
    /// `predicate` — `ekr.graph.PredicateProjection`.
    pub predicate: crate::graph::PredicateProjection,
}

/// UpgradePreview — `ekr.kernel.UpgradePreview`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradePreview {
    /// `reviewer_policy_digest` — `ekr.kernel.ContentHash`.
    pub reviewer_policy_digest: ContentHash,
    /// `head_revision` — `ekr.kernel.RevisionNumber`.
    pub head_revision: RevisionNumber,
    /// `head_hash` — `ekr.kernel.ContentHash`.
    pub head_hash: ContentHash,
    /// `from` — `ekr.kernel.AuthorityVersion`.
    pub from: AuthorityVersion,
    /// `to` — `ekr.kernel.AuthorityVersion`.
    pub to: AuthorityVersion,
    /// `preview_digest` — `ekr.kernel.ContentHash`.
    pub preview_digest: ContentHash,
    /// `contradictions` — `List<ekr.kernel.UpgradeContradiction>`.
    pub contradictions: Vec<UpgradeContradiction>,
    /// `pending_revalidation` — `List<ekr.kernel.TransactionId>`.
    pub pending_revalidation: Vec<TransactionId>,
}

/// ValidatedPayload — `ekr.kernel.ValidatedPayload`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPayload {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `against` — `ekr.kernel.RevisionNumber`.
    pub against: RevisionNumber,
    /// `validation_hash` — `ekr.kernel.ContentHash`.
    pub validation_hash: ContentHash,
}

/// ValidationBasisFormatV1 — `ekr.kernel.ValidationBasisFormatV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationBasisFormatV1(pub String);

/// ValidationBasisV1 — `ekr.kernel.ValidationBasisV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationBasisV1 {
    /// `format` — `ekr.kernel.ValidationBasisFormatV1`.
    pub format: ValidationBasisFormatV1,
    /// `graph_root_id` — `ekr.graph.GraphRootId`.
    pub graph_root_id: crate::graph::GraphRootId,
    /// `previous_revision_id` — `ekr.kernel.RevisionId`.
    pub previous_revision_id: RevisionId,
    /// `previous_event_id` — `ekr.kernel.EventId`.
    pub previous_event_id: EventId,
    /// `previous_record_hash` — `ekr.kernel.ContentHash`.
    pub previous_record_hash: ContentHash,
    /// `previous_root` — `ekr.graph.RevisionRoot`.
    pub previous_root: crate::graph::RevisionRoot,
    /// `previous_root_hash` — `ekr.kernel.ContentHash`.
    pub previous_root_hash: ContentHash,
    /// `seed_hash` — `ekr.kernel.ContentHash`.
    pub seed_hash: ContentHash,
    /// `ontology_root` — `ekr.kernel.ContentHash`.
    pub ontology_root: ContentHash,
    /// `authority_root` — `ekr.kernel.ContentHash`.
    pub authority_root: ContentHash,
    /// `validation_profile_hash` — `ekr.kernel.ContentHash`.
    pub validation_profile_hash: ContentHash,
}

/// ValidationCommandResult — `ekr.kernel.ValidationCommandResult`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationCommandResult {
    /// Tagged `Rejected` — `ekr.kernel.RejectionRecordV1`.
    Rejected(RejectionRecordV1),
    /// Tagged `Validated` — `ekr.kernel.ValidationReceiptV1`.
    Validated(ValidationReceiptV1),
}

/// The states of `ekr.kernel.ValidationIssue`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ValidationIssue<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationIssueState {
    /// `Raised`.
    Raised,
}

/// ValidationIssueRecord — `ekr.kernel.ValidationIssueRecord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssueRecord {
    /// `id` — `ekr.kernel.IssueId`.
    pub id: IssueId,
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `validator` — `ekr.kernel.ValidatorName`.
    pub validator: ValidatorName,
    /// `code` — `String`.
    pub code: String,
    /// `message` — `String`.
    pub message: String,
}

/// ValidationMaterialFormatV1 — `ekr.kernel.ValidationMaterialFormatV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationMaterialFormatV1(pub String);

/// ValidationMaterialV1Projection — `ekr.kernel.ValidationMaterialV1Projection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationMaterialV1Projection {
    /// `format` — `ekr.kernel.ValidationMaterialFormatV1`.
    pub format: ValidationMaterialFormatV1,
    /// `transaction` — `ekr.kernel.CanonicalTransactionProjection`.
    pub transaction: CanonicalTransactionProjection,
    /// `basis` — `ekr.kernel.ValidationBasisV1`.
    pub basis: ValidationBasisV1,
    /// `validators` — `List<ekr.kernel.AgentId>`.
    pub validators: Vec<AgentId>,
}

/// ValidationProfileFormatV1 — `ekr.kernel.ValidationProfileFormatV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationProfileFormatV1(pub String);

/// ValidationReceiptFormatV1 — `ekr.kernel.ValidationReceiptFormatV1`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReceiptFormatV1(pub String);

/// ValidationReceiptV1 — `ekr.kernel.ValidationReceiptV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReceiptV1 {
    /// `format` — `ekr.kernel.ValidationReceiptFormatV1`.
    pub format: ValidationReceiptFormatV1,
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `proposed_event_id` — `ekr.kernel.EventId`.
    pub proposed_event_id: EventId,
    /// `proposal_record_hash` — `ekr.kernel.ContentHash`.
    pub proposal_record_hash: ContentHash,
    /// `transaction_hash` — `ekr.kernel.ContentHash`.
    pub transaction_hash: ContentHash,
    /// `operations_hash` — `ekr.kernel.ContentHash`.
    pub operations_hash: ContentHash,
    /// `evidence_hash` — `ekr.kernel.ContentHash`.
    pub evidence_hash: ContentHash,
    /// `operation_count` — `Integer`.
    pub operation_count: i64,
    /// `basis` — `ekr.kernel.ValidationBasisV1`.
    pub basis: ValidationBasisV1,
    /// `validators` — `List<ekr.kernel.AgentId>`.
    pub validators: Vec<AgentId>,
    /// `validated_at` — `Timestamp`.
    pub validated_at: crate::primitives::Timestamp,
    /// `validation_hash` — `ekr.kernel.ContentHash`.
    pub validation_hash: ContentHash,
}

/// ValidatorName — `ekr.kernel.ValidatorName`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidatorName {
    /// `Structural`.
    Structural,
    /// `Reference`.
    Reference,
    /// `Type`.
    Type,
    /// `Cardinality`.
    Cardinality,
    /// `OntologyConstraint`.
    OntologyConstraint,
    /// `Provenance`.
    Provenance,
    /// `Authorization`.
    Authorization,
}

/// What Agent — `ekr.kernel.Agent` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Agent<S>`], and at a boundary by [`AgentSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentData {
    /// The identity: `agent_id` — `ekr.kernel.AgentId`.
    pub agent_id: AgentId,
    /// `name` — `String`.
    pub name: String,
    /// `capabilities` — `List<String>`.
    pub capabilities: Vec<String>,
}

/// The states of `ekr.kernel.Agent`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](agent_state::Marker), so [`Agent<S>`](Agent) can only ever rest in a real state.
pub mod agent_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Registered {}
    }

    /// A declared state of `Agent`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::AgentState;
    }

    /// `Registered`. Where a new instance starts.
    pub struct Registered;

    impl Marker for Registered {
        const STATE: super::AgentState = super::AgentState::Registered;
    }
}

/// Agent — `ekr.kernel.Agent` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Registered`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`AgentSnapshot`]
/// and [`AgentSnapshot::refine`].
pub struct Agent<S: agent_state::Marker> {
    data: AgentData,
    state: core::marker::PhantomData<S>,
}

impl<S: agent_state::Marker> Agent<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> AgentState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &AgentData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> AgentData {
        self.data
    }
}

impl Agent<agent_state::Registered> {
    /// A new instance, resting in `Registered` — the only state the lifecycle starts one in.
    pub fn new(data: AgentData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.kernel.Agent` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`AgentSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: AgentState,
    /// What it holds.
    pub data: AgentData,
}

/// An `Agent` in whichever declared state it was found.
pub enum AnyAgent {
    /// Resting in `Registered`.
    Registered(Agent<agent_state::Registered>),
}

impl AgentSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `AgentState` cannot spell one.
    pub fn refine(self) -> AnyAgent {
        match self.state {
            AgentState::Registered => AnyAgent::Registered(Agent {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyAgent {
    /// The state, as the runtime value.
    pub fn state(&self) -> AgentState {
        match self {
            Self::Registered(_) => AgentState::Registered,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> AgentSnapshot {
        match self {
            Self::Registered(instance) => AgentSnapshot {
                state: AgentState::Registered,
                data: instance.into_data(),
            },
        }
    }
}

/// What AuthorityTransition — `ekr.kernel.AuthorityTransition` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`AuthorityTransition<S>`], and at a boundary by [`AuthorityTransitionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityTransitionData {
    /// The identity: `transition_id` — `ekr.kernel.AuthorityTransitionId`.
    pub transition_id: AuthorityTransitionId,
    /// `review_host_binding_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `reviewer_host_binding`: `ekr.kernel.AuthorityTransition` references one `ekr.store.StoredObject`.
    pub review_host_binding_digest: ContentHash,
    /// `reviewer_policy_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `reviewer_policy`: `ekr.kernel.AuthorityTransition` references one `ekr.store.StoredObject`.
    pub reviewer_policy_digest: ContentHash,
    /// `trust_enrollment` — `ekr.kernel.ReviewerTrustEnrollment`.
    pub trust_enrollment: ReviewerTrustEnrollment,
    /// `human_proof_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `human_proof`: `ekr.kernel.AuthorityTransition` references one `ekr.kernel.RetainedHumanDecision`.
    pub human_proof_digest: ContentHash,
    /// `target_profile_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `target_profile`: `ekr.kernel.AuthorityTransition` references one `ekr.store.StoredObject`.
    pub target_profile_digest: ContentHash,
    /// `format` — `ekr.kernel.AuthorityTransitionFormat`.
    pub format: AuthorityTransitionFormat,
    /// `seed_anchor` — `ekr.kernel.ContentHash`.
    pub seed_anchor: ContentHash,
    /// `predecessor_head` — `ekr.kernel.ContentHash`.
    pub predecessor_head: ContentHash,
    /// `activation_revision` — `ekr.kernel.RevisionNumber`.
    pub activation_revision: RevisionNumber,
    /// `from` — `ekr.kernel.AuthorityVersion`.
    pub from: AuthorityVersion,
    /// `to` — `ekr.kernel.AuthorityVersion`.
    pub to: AuthorityVersion,
    /// `preview_digest` — `ekr.kernel.ContentHash`.
    pub preview_digest: ContentHash,
    /// `operator` — `ekr.kernel.TrustedOperatorIdentity`.
    pub operator: TrustedOperatorIdentity,
    /// `recorded_at` — `Timestamp`.
    pub recorded_at: crate::primitives::Timestamp,
}

/// The states of `ekr.kernel.AuthorityTransition`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](authority_transition_state::Marker), so [`AuthorityTransition<S>`](AuthorityTransition) can only ever rest in a real state.
pub mod authority_transition_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `AuthorityTransition`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::AuthorityTransitionState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::AuthorityTransitionState = super::AuthorityTransitionState::Recorded;
    }
}

/// AuthorityTransition — `ekr.kernel.AuthorityTransition` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`AuthorityTransitionSnapshot`]
/// and [`AuthorityTransitionSnapshot::refine`].
pub struct AuthorityTransition<S: authority_transition_state::Marker> {
    data: AuthorityTransitionData,
    state: core::marker::PhantomData<S>,
}

impl<S: authority_transition_state::Marker> AuthorityTransition<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> AuthorityTransitionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &AuthorityTransitionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> AuthorityTransitionData {
        self.data
    }
}

impl AuthorityTransition<authority_transition_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: AuthorityTransitionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.kernel.AuthorityTransition` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`AuthorityTransitionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityTransitionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: AuthorityTransitionState,
    /// What it holds.
    pub data: AuthorityTransitionData,
}

/// An `AuthorityTransition` in whichever declared state it was found.
pub enum AnyAuthorityTransition {
    /// Resting in `Recorded`.
    Recorded(AuthorityTransition<authority_transition_state::Recorded>),
}

impl AuthorityTransitionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `AuthorityTransitionState` cannot spell one.
    pub fn refine(self) -> AnyAuthorityTransition {
        match self.state {
            AuthorityTransitionState::Recorded => AnyAuthorityTransition::Recorded(AuthorityTransition {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyAuthorityTransition {
    /// The state, as the runtime value.
    pub fn state(&self) -> AuthorityTransitionState {
        match self {
            Self::Recorded(_) => AuthorityTransitionState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> AuthorityTransitionSnapshot {
        match self {
            Self::Recorded(instance) => AuthorityTransitionSnapshot {
                state: AuthorityTransitionState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What Dispute — `ekr.kernel.Dispute` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Dispute<S>`], and at a boundary by [`DisputeSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeData {
    /// The identity: `dispute_id` — `ekr.kernel.DisputeId`.
    pub dispute_id: DisputeId,
    /// `basis_revision` — `ekr.kernel.RevisionNumber`.
    pub basis_revision: RevisionNumber,
    /// `subject` — `ekr.graph.SubjectProjection`.
    pub subject: crate::graph::SubjectProjection,
    /// `predicate` — `ekr.graph.PredicateProjection`.
    pub predicate: crate::graph::PredicateProjection,
    /// `basis_digest` — `ekr.kernel.ContentHash`.
    pub basis_digest: ContentHash,
}

/// The states of `ekr.kernel.Dispute`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](dispute_state::Marker), so [`Dispute<S>`](Dispute) can only ever rest in a real state.
pub mod dispute_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `Dispute`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::DisputeState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::DisputeState = super::DisputeState::Recorded;
    }
}

/// Dispute — `ekr.kernel.Dispute` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`DisputeSnapshot`]
/// and [`DisputeSnapshot::refine`].
pub struct Dispute<S: dispute_state::Marker> {
    data: DisputeData,
    state: core::marker::PhantomData<S>,
}

impl<S: dispute_state::Marker> Dispute<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> DisputeState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &DisputeData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> DisputeData {
        self.data
    }
}

impl Dispute<dispute_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: DisputeData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.kernel.Dispute` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`DisputeSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: DisputeState,
    /// What it holds.
    pub data: DisputeData,
}

/// An `Dispute` in whichever declared state it was found.
pub enum AnyDispute {
    /// Resting in `Recorded`.
    Recorded(Dispute<dispute_state::Recorded>),
}

impl DisputeSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `DisputeState` cannot spell one.
    pub fn refine(self) -> AnyDispute {
        match self.state {
            DisputeState::Recorded => AnyDispute::Recorded(Dispute {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyDispute {
    /// The state, as the runtime value.
    pub fn state(&self) -> DisputeState {
        match self {
            Self::Recorded(_) => DisputeState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> DisputeSnapshot {
        match self {
            Self::Recorded(instance) => DisputeSnapshot {
                state: DisputeState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What DisputeClaim — `ekr.kernel.DisputeClaim` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`DisputeClaim<S>`], and at a boundary by [`DisputeClaimSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeClaimData {
    /// The identity: `claim_link_id` — `Uuid`.
    pub claim_link_id: crate::primitives::Uuid,
    /// `dispute_id` — `ekr.kernel.DisputeId`.
    ///
    /// Carries `dispute`: `ekr.kernel.DisputeClaim` references one `ekr.kernel.Dispute`.
    pub dispute_id: DisputeId,
    /// `assertion_id` — `ekr.graph.AssertionId`.
    ///
    /// Carries `claim`: `ekr.kernel.DisputeClaim` references one `ekr.graph.Assertion`.
    pub assertion_id: crate::graph::AssertionId,
}

/// The states of `ekr.kernel.DisputeClaim`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](dispute_claim_state::Marker), so [`DisputeClaim<S>`](DisputeClaim) can only ever rest in a real state.
pub mod dispute_claim_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `DisputeClaim`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::DisputeClaimState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::DisputeClaimState = super::DisputeClaimState::Recorded;
    }
}

/// DisputeClaim — `ekr.kernel.DisputeClaim` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`DisputeClaimSnapshot`]
/// and [`DisputeClaimSnapshot::refine`].
pub struct DisputeClaim<S: dispute_claim_state::Marker> {
    data: DisputeClaimData,
    state: core::marker::PhantomData<S>,
}

impl<S: dispute_claim_state::Marker> DisputeClaim<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> DisputeClaimState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &DisputeClaimData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> DisputeClaimData {
        self.data
    }
}

impl DisputeClaim<dispute_claim_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: DisputeClaimData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.kernel.DisputeClaim` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`DisputeClaimSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeClaimSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: DisputeClaimState,
    /// What it holds.
    pub data: DisputeClaimData,
}

/// An `DisputeClaim` in whichever declared state it was found.
pub enum AnyDisputeClaim {
    /// Resting in `Recorded`.
    Recorded(DisputeClaim<dispute_claim_state::Recorded>),
}

impl DisputeClaimSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `DisputeClaimState` cannot spell one.
    pub fn refine(self) -> AnyDisputeClaim {
        match self.state {
            DisputeClaimState::Recorded => AnyDisputeClaim::Recorded(DisputeClaim {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyDisputeClaim {
    /// The state, as the runtime value.
    pub fn state(&self) -> DisputeClaimState {
        match self {
            Self::Recorded(_) => DisputeClaimState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> DisputeClaimSnapshot {
        match self {
            Self::Recorded(instance) => DisputeClaimSnapshot {
                state: DisputeClaimState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What GraphTransaction — `ekr.kernel.GraphTransaction` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`GraphTransaction<S>`], and at a boundary by [`GraphTransactionSnapshot::state`].
///
/// Every value satisfies `operation_count >= 1` — checked by [`GraphTransactionData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphTransactionData {
    /// The identity: `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `proposer` — `ekr.kernel.AgentId`.
    ///
    /// Carries `proposals`: `ekr.kernel.Agent` owns many `ekr.kernel.GraphTransaction`.
    pub proposer: AgentId,
    /// `document_hash` — `ekr.kernel.ContentHash`.
    pub document_hash: ContentHash,
    /// `proposal_record_hash` — `ekr.kernel.ContentHash`.
    pub proposal_record_hash: ContentHash,
    /// `canonical_transaction_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub canonical_transaction_hash: Option<ContentHash>,
    /// `canonical_operations_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub canonical_operations_hash: Option<ContentHash>,
    /// `operation_count` — `Optional<Integer>`.
    pub operation_count: Option<i64>,
    /// `evidence_hash` — `ekr.kernel.ContentHash`.
    pub evidence_hash: ContentHash,
    /// `validation_basis` — `Optional<ekr.kernel.ValidationBasisV1>`.
    pub validation_basis: Option<ValidationBasisV1>,
    /// `validated_against` — `Optional<ekr.kernel.RevisionNumber>`.
    pub validated_against: Option<RevisionNumber>,
    /// `validation_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub validation_hash: Option<ContentHash>,
    /// `validation_record_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub validation_record_hash: Option<ContentHash>,
    /// `terminal_record_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub terminal_record_hash: Option<ContentHash>,
}

impl GraphTransactionData {
    /// The first declared invariant of `ekr.kernel.GraphTransaction` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(self.operation_count.as_ref().map(|v| iv::Fact::integer(*v)), iv::Op::Ge, iv::Fact::number("1"), false, true)) {
            return Some("operation_count >= 1");
        }
        None
    }
}

/// The states of `ekr.kernel.GraphTransaction`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](graph_transaction_state::Marker), so [`GraphTransaction<S>`](GraphTransaction) can only ever rest in a real state.
pub mod graph_transaction_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Committed {}
        impl Sealed for super::Proposed {}
        impl Sealed for super::Rejected {}
        impl Sealed for super::Stale {}
        impl Sealed for super::Validated {}
    }

    /// A declared state of `GraphTransaction`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::GraphTransactionState;
    }

    /// `Committed`. Terminal: an instance may rest here forever.
    pub struct Committed;

    impl Marker for Committed {
        const STATE: super::GraphTransactionState = super::GraphTransactionState::Committed;
    }

    /// `Proposed`. Where a new instance starts.
    pub struct Proposed;

    impl Marker for Proposed {
        const STATE: super::GraphTransactionState = super::GraphTransactionState::Proposed;
    }

    /// `Rejected`. Terminal: an instance may rest here forever.
    pub struct Rejected;

    impl Marker for Rejected {
        const STATE: super::GraphTransactionState = super::GraphTransactionState::Rejected;
    }

    /// `Stale`. Terminal: an instance may rest here forever.
    pub struct Stale;

    impl Marker for Stale {
        const STATE: super::GraphTransactionState = super::GraphTransactionState::Stale;
    }

    /// `Validated`.
    pub struct Validated;

    impl Marker for Validated {
        const STATE: super::GraphTransactionState = super::GraphTransactionState::Validated;
    }
}

/// GraphTransaction — `ekr.kernel.GraphTransaction` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Proposed`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`GraphTransactionSnapshot`]
/// and [`GraphTransactionSnapshot::refine`].
pub struct GraphTransaction<S: graph_transaction_state::Marker> {
    data: GraphTransactionData,
    state: core::marker::PhantomData<S>,
}

impl<S: graph_transaction_state::Marker> GraphTransaction<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> GraphTransactionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &GraphTransactionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> GraphTransactionData {
        self.data
    }
}

impl GraphTransaction<graph_transaction_state::Proposed> {
    /// A new instance, resting in `Proposed` — the only state the lifecycle starts one in.
    pub fn new(data: GraphTransactionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl GraphTransaction<graph_transaction_state::Proposed> {
    /// `validate` — `Proposed` → `Validated`. Taken by the `validated` outcome of `ekr.kernel.Validate`.
    pub fn validate(self) -> GraphTransaction<graph_transaction_state::Validated> {
        GraphTransaction {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `reject` — `Proposed` → `Rejected`. Taken by the `rejected` outcome of `ekr.kernel.Validate`.
    pub fn reject(self) -> GraphTransaction<graph_transaction_state::Rejected> {
        GraphTransaction {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl GraphTransaction<graph_transaction_state::Validated> {
    /// `commit` — `Validated` → `Committed`. Taken by the `committed` outcome of `ekr.kernel.Commit`.
    pub fn commit(self) -> GraphTransaction<graph_transaction_state::Committed> {
        GraphTransaction {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `stale` — `Validated` → `Stale`. Taken by the `stale` outcome of `ekr.kernel.Commit`.
    pub fn stale(self) -> GraphTransaction<graph_transaction_state::Stale> {
        GraphTransaction {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.kernel.GraphTransaction` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`GraphTransactionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphTransactionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: GraphTransactionState,
    /// What it holds.
    pub data: GraphTransactionData,
}

/// An `GraphTransaction` in whichever declared state it was found.
pub enum AnyGraphTransaction {
    /// Resting in `Committed`.
    Committed(GraphTransaction<graph_transaction_state::Committed>),
    /// Resting in `Proposed`.
    Proposed(GraphTransaction<graph_transaction_state::Proposed>),
    /// Resting in `Rejected`.
    Rejected(GraphTransaction<graph_transaction_state::Rejected>),
    /// Resting in `Stale`.
    Stale(GraphTransaction<graph_transaction_state::Stale>),
    /// Resting in `Validated`.
    Validated(GraphTransaction<graph_transaction_state::Validated>),
}

impl GraphTransactionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `GraphTransactionState` cannot spell one.
    pub fn refine(self) -> AnyGraphTransaction {
        match self.state {
            GraphTransactionState::Committed => AnyGraphTransaction::Committed(GraphTransaction {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            GraphTransactionState::Proposed => AnyGraphTransaction::Proposed(GraphTransaction {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            GraphTransactionState::Rejected => AnyGraphTransaction::Rejected(GraphTransaction {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            GraphTransactionState::Stale => AnyGraphTransaction::Stale(GraphTransaction {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            GraphTransactionState::Validated => AnyGraphTransaction::Validated(GraphTransaction {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyGraphTransaction {
    /// The state, as the runtime value.
    pub fn state(&self) -> GraphTransactionState {
        match self {
            Self::Committed(_) => GraphTransactionState::Committed,
            Self::Proposed(_) => GraphTransactionState::Proposed,
            Self::Rejected(_) => GraphTransactionState::Rejected,
            Self::Stale(_) => GraphTransactionState::Stale,
            Self::Validated(_) => GraphTransactionState::Validated,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> GraphTransactionSnapshot {
        match self {
            Self::Committed(instance) => GraphTransactionSnapshot {
                state: GraphTransactionState::Committed,
                data: instance.into_data(),
            },
            Self::Proposed(instance) => GraphTransactionSnapshot {
                state: GraphTransactionState::Proposed,
                data: instance.into_data(),
            },
            Self::Rejected(instance) => GraphTransactionSnapshot {
                state: GraphTransactionState::Rejected,
                data: instance.into_data(),
            },
            Self::Stale(instance) => GraphTransactionSnapshot {
                state: GraphTransactionState::Stale,
                data: instance.into_data(),
            },
            Self::Validated(instance) => GraphTransactionSnapshot {
                state: GraphTransactionState::Validated,
                data: instance.into_data(),
            },
        }
    }
}

/// What HumanAnswer — `ekr.kernel.HumanAnswer` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`HumanAnswer<S>`], and at a boundary by [`HumanAnswerSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanAnswerData {
    /// The identity: `answer_id` — `ekr.kernel.HumanAnswerId`.
    pub answer_id: HumanAnswerId,
    /// `human_proof_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `human_proof`: `ekr.kernel.HumanAnswer` references one `ekr.kernel.RetainedHumanDecision`.
    pub human_proof_digest: ContentHash,
    /// `dispute_id` — `ekr.kernel.DisputeId`.
    ///
    /// Carries `dispute`: `ekr.kernel.HumanAnswer` references one `ekr.kernel.Dispute`.
    pub dispute_id: DisputeId,
    /// `basis` — `ekr.kernel.ReviewBasis`.
    pub basis: ReviewBasis,
    /// `operator` — `ekr.kernel.TrustedOperatorIdentity`.
    pub operator: TrustedOperatorIdentity,
    /// `corrections` — `List<ekr.kernel.ClaimCorrection>`.
    pub corrections: Vec<ClaimCorrection>,
    /// `statement_evidence` — `ekr.graph.EvidenceId`.
    ///
    /// Carries `statement`: `ekr.kernel.HumanAnswer` references one `ekr.graph.Evidence`.
    pub statement_evidence: crate::graph::EvidenceId,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<TransactionId>,
}

/// The states of `ekr.kernel.HumanAnswer`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](human_answer_state::Marker), so [`HumanAnswer<S>`](HumanAnswer) can only ever rest in a real state.
pub mod human_answer_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `HumanAnswer`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::HumanAnswerState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::HumanAnswerState = super::HumanAnswerState::Recorded;
    }
}

/// HumanAnswer — `ekr.kernel.HumanAnswer` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`HumanAnswerSnapshot`]
/// and [`HumanAnswerSnapshot::refine`].
pub struct HumanAnswer<S: human_answer_state::Marker> {
    data: HumanAnswerData,
    state: core::marker::PhantomData<S>,
}

impl<S: human_answer_state::Marker> HumanAnswer<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> HumanAnswerState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &HumanAnswerData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> HumanAnswerData {
        self.data
    }
}

impl HumanAnswer<human_answer_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: HumanAnswerData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.kernel.HumanAnswer` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`HumanAnswerSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanAnswerSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: HumanAnswerState,
    /// What it holds.
    pub data: HumanAnswerData,
}

/// An `HumanAnswer` in whichever declared state it was found.
pub enum AnyHumanAnswer {
    /// Resting in `Recorded`.
    Recorded(HumanAnswer<human_answer_state::Recorded>),
}

impl HumanAnswerSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `HumanAnswerState` cannot spell one.
    pub fn refine(self) -> AnyHumanAnswer {
        match self.state {
            HumanAnswerState::Recorded => AnyHumanAnswer::Recorded(HumanAnswer {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyHumanAnswer {
    /// The state, as the runtime value.
    pub fn state(&self) -> HumanAnswerState {
        match self {
            Self::Recorded(_) => HumanAnswerState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> HumanAnswerSnapshot {
        match self {
            Self::Recorded(instance) => HumanAnswerSnapshot {
                state: HumanAnswerState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What RetainedHumanDecision — `ekr.kernel.RetainedHumanDecision` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`RetainedHumanDecision<S>`], and at a boundary by [`RetainedHumanDecisionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedHumanDecisionData {
    /// The identity: `proof_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `proof_bytes`: `ekr.kernel.RetainedHumanDecision` references one `ekr.store.StoredObject`.
    pub proof_digest: ContentHash,
    /// `decision_id` — `Uuid`.
    pub decision_id: crate::primitives::Uuid,
    /// `policy_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `policy_bytes`: `ekr.kernel.RetainedHumanDecision` references one `ekr.store.StoredObject`.
    pub policy_digest: ContentHash,
    /// `statement_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `statement_bytes`: `ekr.kernel.RetainedHumanDecision` references one `ekr.store.StoredObject`.
    pub statement_digest: ContentHash,
    /// `operator` — `ekr.kernel.TrustedOperatorIdentity`.
    pub operator: TrustedOperatorIdentity,
    /// `recorded_at` — `Timestamp`.
    pub recorded_at: crate::primitives::Timestamp,
}

/// The states of `ekr.kernel.RetainedHumanDecision`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](retained_human_decision_state::Marker), so [`RetainedHumanDecision<S>`](RetainedHumanDecision) can only ever rest in a real state.
pub mod retained_human_decision_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Verified {}
    }

    /// A declared state of `RetainedHumanDecision`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::RetainedHumanDecisionState;
    }

    /// `Verified`. Where a new instance starts.
    pub struct Verified;

    impl Marker for Verified {
        const STATE: super::RetainedHumanDecisionState = super::RetainedHumanDecisionState::Verified;
    }
}

/// RetainedHumanDecision — `ekr.kernel.RetainedHumanDecision` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Verified`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`RetainedHumanDecisionSnapshot`]
/// and [`RetainedHumanDecisionSnapshot::refine`].
pub struct RetainedHumanDecision<S: retained_human_decision_state::Marker> {
    data: RetainedHumanDecisionData,
    state: core::marker::PhantomData<S>,
}

impl<S: retained_human_decision_state::Marker> RetainedHumanDecision<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> RetainedHumanDecisionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &RetainedHumanDecisionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> RetainedHumanDecisionData {
        self.data
    }
}

impl RetainedHumanDecision<retained_human_decision_state::Verified> {
    /// A new instance, resting in `Verified` — the only state the lifecycle starts one in.
    pub fn new(data: RetainedHumanDecisionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.kernel.RetainedHumanDecision` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`RetainedHumanDecisionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedHumanDecisionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: RetainedHumanDecisionState,
    /// What it holds.
    pub data: RetainedHumanDecisionData,
}

/// An `RetainedHumanDecision` in whichever declared state it was found.
pub enum AnyRetainedHumanDecision {
    /// Resting in `Verified`.
    Verified(RetainedHumanDecision<retained_human_decision_state::Verified>),
}

impl RetainedHumanDecisionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `RetainedHumanDecisionState` cannot spell one.
    pub fn refine(self) -> AnyRetainedHumanDecision {
        match self.state {
            RetainedHumanDecisionState::Verified => AnyRetainedHumanDecision::Verified(RetainedHumanDecision {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyRetainedHumanDecision {
    /// The state, as the runtime value.
    pub fn state(&self) -> RetainedHumanDecisionState {
        match self {
            Self::Verified(_) => RetainedHumanDecisionState::Verified,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> RetainedHumanDecisionSnapshot {
        match self {
            Self::Verified(instance) => RetainedHumanDecisionSnapshot {
                state: RetainedHumanDecisionState::Verified,
                data: instance.into_data(),
            },
        }
    }
}

/// What Revision — `ekr.kernel.Revision` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Revision<S>`], and at a boundary by [`RevisionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionData {
    /// The identity: `revision_id` — `ekr.kernel.RevisionId`.
    pub revision_id: RevisionId,
    /// `number` — `ekr.kernel.RevisionNumber`.
    pub number: RevisionNumber,
    /// `parent` — `Optional<ekr.kernel.RevisionId>`.
    pub parent: Option<RevisionId>,
    /// `ontology_root` — `ekr.kernel.ContentHash`.
    pub ontology_root: ContentHash,
    /// `knowledge_root` — `ekr.kernel.ContentHash`.
    pub knowledge_root: ContentHash,
    /// `evidence_root` — `ekr.kernel.ContentHash`.
    pub evidence_root: ContentHash,
    /// `agent_root` — `ekr.kernel.ContentHash`.
    pub agent_root: ContentHash,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<TransactionId>,
    /// `committed_at` — `Timestamp`.
    pub committed_at: crate::primitives::Timestamp,
}

/// The states of `ekr.kernel.Revision`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](revision_state::Marker), so [`Revision<S>`](Revision) can only ever rest in a real state.
pub mod revision_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Committed {}
    }

    /// A declared state of `Revision`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::RevisionState;
    }

    /// `Committed`. Where a new instance starts.
    pub struct Committed;

    impl Marker for Committed {
        const STATE: super::RevisionState = super::RevisionState::Committed;
    }
}

/// Revision — `ekr.kernel.Revision` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Committed`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`RevisionSnapshot`]
/// and [`RevisionSnapshot::refine`].
pub struct Revision<S: revision_state::Marker> {
    data: RevisionData,
    state: core::marker::PhantomData<S>,
}

impl<S: revision_state::Marker> Revision<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> RevisionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &RevisionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> RevisionData {
        self.data
    }
}

impl Revision<revision_state::Committed> {
    /// A new instance, resting in `Committed` — the only state the lifecycle starts one in.
    pub fn new(data: RevisionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.kernel.Revision` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`RevisionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: RevisionState,
    /// What it holds.
    pub data: RevisionData,
}

/// An `Revision` in whichever declared state it was found.
pub enum AnyRevision {
    /// Resting in `Committed`.
    Committed(Revision<revision_state::Committed>),
}

impl RevisionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `RevisionState` cannot spell one.
    pub fn refine(self) -> AnyRevision {
        match self.state {
            RevisionState::Committed => AnyRevision::Committed(Revision {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyRevision {
    /// The state, as the runtime value.
    pub fn state(&self) -> RevisionState {
        match self {
            Self::Committed(_) => RevisionState::Committed,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> RevisionSnapshot {
        match self {
            Self::Committed(instance) => RevisionSnapshot {
                state: RevisionState::Committed,
                data: instance.into_data(),
            },
        }
    }
}

/// What SchemaTransactionEvidence — `ekr.kernel.SchemaTransactionEvidence` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`SchemaTransactionEvidence<S>`], and at a boundary by [`SchemaTransactionEvidenceSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaTransactionEvidenceData {
    /// The identity: `link_id` — `Uuid`.
    pub link_id: crate::primitives::Uuid,
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    ///
    /// Carries `transaction`: `ekr.kernel.SchemaTransactionEvidence` references one `ekr.kernel.GraphTransaction`.
    pub transaction_id: TransactionId,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    ///
    /// Carries `evidence`: `ekr.kernel.SchemaTransactionEvidence` references one `ekr.graph.Evidence`.
    pub evidence_id: crate::graph::EvidenceId,
}

/// The states of `ekr.kernel.SchemaTransactionEvidence`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](schema_transaction_evidence_state::Marker), so [`SchemaTransactionEvidence<S>`](SchemaTransactionEvidence) can only ever rest in a real state.
pub mod schema_transaction_evidence_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `SchemaTransactionEvidence`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SchemaTransactionEvidenceState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::SchemaTransactionEvidenceState = super::SchemaTransactionEvidenceState::Recorded;
    }
}

/// SchemaTransactionEvidence — `ekr.kernel.SchemaTransactionEvidence` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SchemaTransactionEvidenceSnapshot`]
/// and [`SchemaTransactionEvidenceSnapshot::refine`].
pub struct SchemaTransactionEvidence<S: schema_transaction_evidence_state::Marker> {
    data: SchemaTransactionEvidenceData,
    state: core::marker::PhantomData<S>,
}

impl<S: schema_transaction_evidence_state::Marker> SchemaTransactionEvidence<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SchemaTransactionEvidenceState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SchemaTransactionEvidenceData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SchemaTransactionEvidenceData {
        self.data
    }
}

impl SchemaTransactionEvidence<schema_transaction_evidence_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: SchemaTransactionEvidenceData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.kernel.SchemaTransactionEvidence` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SchemaTransactionEvidenceSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaTransactionEvidenceSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SchemaTransactionEvidenceState,
    /// What it holds.
    pub data: SchemaTransactionEvidenceData,
}

/// An `SchemaTransactionEvidence` in whichever declared state it was found.
pub enum AnySchemaTransactionEvidence {
    /// Resting in `Recorded`.
    Recorded(SchemaTransactionEvidence<schema_transaction_evidence_state::Recorded>),
}

impl SchemaTransactionEvidenceSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SchemaTransactionEvidenceState` cannot spell one.
    pub fn refine(self) -> AnySchemaTransactionEvidence {
        match self.state {
            SchemaTransactionEvidenceState::Recorded => AnySchemaTransactionEvidence::Recorded(SchemaTransactionEvidence {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySchemaTransactionEvidence {
    /// The state, as the runtime value.
    pub fn state(&self) -> SchemaTransactionEvidenceState {
        match self {
            Self::Recorded(_) => SchemaTransactionEvidenceState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SchemaTransactionEvidenceSnapshot {
        match self {
            Self::Recorded(instance) => SchemaTransactionEvidenceSnapshot {
                state: SchemaTransactionEvidenceState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What ValidationIssue — `ekr.kernel.ValidationIssue` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ValidationIssue<S>`], and at a boundary by [`ValidationIssueSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssueData {
    /// The identity: `issue_id` — `ekr.kernel.IssueId`.
    pub issue_id: IssueId,
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    ///
    /// Carries `issues`: `ekr.kernel.GraphTransaction` owns many `ekr.kernel.ValidationIssue`.
    pub transaction_id: TransactionId,
    /// `against` — `ekr.kernel.RevisionNumber`.
    pub against: RevisionNumber,
    /// `validator` — `ekr.kernel.ValidatorName`.
    pub validator: ValidatorName,
    /// `code` — `String`.
    pub code: String,
    /// `message` — `String`.
    pub message: String,
}

/// The states of `ekr.kernel.ValidationIssue`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](validation_issue_state::Marker), so [`ValidationIssue<S>`](ValidationIssue) can only ever rest in a real state.
pub mod validation_issue_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Raised {}
    }

    /// A declared state of `ValidationIssue`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ValidationIssueState;
    }

    /// `Raised`. Where a new instance starts.
    pub struct Raised;

    impl Marker for Raised {
        const STATE: super::ValidationIssueState = super::ValidationIssueState::Raised;
    }
}

/// ValidationIssue — `ekr.kernel.ValidationIssue` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Raised`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ValidationIssueSnapshot`]
/// and [`ValidationIssueSnapshot::refine`].
pub struct ValidationIssue<S: validation_issue_state::Marker> {
    data: ValidationIssueData,
    state: core::marker::PhantomData<S>,
}

impl<S: validation_issue_state::Marker> ValidationIssue<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ValidationIssueState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ValidationIssueData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ValidationIssueData {
        self.data
    }
}

impl ValidationIssue<validation_issue_state::Raised> {
    /// A new instance, resting in `Raised` — the only state the lifecycle starts one in.
    pub fn new(data: ValidationIssueData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.kernel.ValidationIssue` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ValidationIssueSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssueSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ValidationIssueState,
    /// What it holds.
    pub data: ValidationIssueData,
}

/// An `ValidationIssue` in whichever declared state it was found.
pub enum AnyValidationIssue {
    /// Resting in `Raised`.
    Raised(ValidationIssue<validation_issue_state::Raised>),
}

impl ValidationIssueSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ValidationIssueState` cannot spell one.
    pub fn refine(self) -> AnyValidationIssue {
        match self.state {
            ValidationIssueState::Raised => AnyValidationIssue::Raised(ValidationIssue {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyValidationIssue {
    /// The state, as the runtime value.
    pub fn state(&self) -> ValidationIssueState {
        match self {
            Self::Raised(_) => ValidationIssueState::Raised,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ValidationIssueSnapshot {
        match self {
            Self::Raised(instance) => ValidationIssueSnapshot {
                state: ValidationIssueState::Raised,
                data: instance.into_data(),
            },
        }
    }
}

/// AnswerAttention — the input of `ekr.kernel.AnswerAttention`.
///
/// Everything it can result in is [`AnswerAttentionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerAttention {
    /// `human_proof` — `ekr.kernel.SignedHumanDecision`.
    pub human_proof: SignedHumanDecision,
    /// `dispute_id` — `ekr.kernel.DisputeId`.
    pub dispute_id: DisputeId,
    /// `basis` — `ekr.kernel.ReviewBasis`.
    pub basis: ReviewBasis,
    /// `corrections` — `List<ekr.kernel.ClaimCorrection>`.
    pub corrections: Vec<ClaimCorrection>,
    /// `statement` — `Bytes`.
    pub statement: Vec<u8>,
}

/// Actual typed response of `ekr.kernel.AnswerAttention`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerAttentionResponse {
    /// `receipt` — `ekr.kernel.AnswerReceipt`.
    pub receipt: AnswerReceipt,
}

/// Everything `ekr.kernel.AnswerAttention` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerAttentionOutcome {
    /// `answered` — otherwise.
    ///
    /// Verify the signed human proof and derive the operator from the pinned reviewer policy, retain the statement, validate supported corrections and commit them. Recompute affected disputes without dangling competitor ids. Resolved questions disappear; evidence and historical assertions remain.
    Answered {
        /// The `ekr.kernel.AnswerAttentionResult` this outcome publishes.
        answer_attention_result: AnswerAttentionResult,
    },
    /// `refused` — externally decided (Verify human_proof under the independently enrolled reviewer policy and the exact AnswerAttention target, statement, corrections and predecessor decision. A host operator UUID or supplied key establishes no human authority. Revalidate against current state. Unrelated revisions alone do not invalidate review; changed evidence, choices or intended effects require renewed human review. An agent-provided identity or unsupported claim cannot authorize resolution.).
    Refused {
        /// Why it was refused: `ekr.kernel.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// ApplyUpgrade — the input of `ekr.kernel.ApplyUpgrade`.
///
/// Everything it can result in is [`ApplyUpgradeOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyUpgrade {
    /// `statement` — `Bytes`.
    pub statement: Vec<u8>,
    /// `human_proof` — `ekr.kernel.SignedHumanDecision`.
    pub human_proof: SignedHumanDecision,
    /// `preview` — `ekr.kernel.UpgradePreview`.
    pub preview: UpgradePreview,
}

/// Actual typed response of `ekr.kernel.ApplyUpgrade`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyUpgradeResponse {
    /// `transition_id` — `ekr.kernel.AuthorityTransitionId`.
    pub transition_id: AuthorityTransitionId,
}

/// Everything `ekr.kernel.ApplyUpgrade` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyUpgradeOutcome {
    /// `answered` — otherwise.
    ///
    /// Verify the signed human proof and derive the operator from the pinned reviewer policy, then atomically append the authority boundary against the exact preview basis; preserve historical bytes and original replay rules.
    Answered {
        /// The `ekr.kernel.ApplyUpgradeResult` this outcome publishes.
        apply_upgrade_result: ApplyUpgradeResult,
    },
    /// `refused` — externally decided (Verify an UpgradeAuthority human_proof and the independently provisioned host/store reviewer binding before enrollment or mutation. Never trust a policy or verification key solely because request content supplies it. Before any mutation refuse a stale head, altered preview digest, unsupported target, unauthenticated operator or incomplete historical verification.).
    Refused {
        /// Why it was refused: `ekr.kernel.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// Commit a validated transaction — the input of `ekr.kernel.Commit`.
///
/// Everything it can result in is [`CommitOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
}

/// Actual typed response of `ekr.kernel.Commit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitResponse {
    /// `result` — `ekr.kernel.CommitCommandResult`.
    pub result: CommitCommandResult,
}

/// Everything `ekr.kernel.Commit` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitOutcome {
    /// `committed` — when the existing subject is in Validated.
    ///
    /// A new immutable revision exists whose parent is the one validated against.
    Committed {
        /// Actual response returned by this branch.
        response: CommitResponse,
        /// The `ekr.kernel.RevisionCommitted` this outcome publishes.
        revision_committed: RevisionCommitted,
        /// The `ekr.store.PublicationPrepared` this outcome publishes.
        publication_prepared: crate::store::PublicationPrepared,
        /// The `ekr.store.ObjectStored` this outcome publishes.
        object_stored: crate::store::ObjectStored,
        /// The `ekr.store.CheckpointWritten` this outcome publishes.
        checkpoint_written: crate::store::CheckpointWritten,
    },
    /// `stale` — externally decided (the canonical revision moved since the transaction was validated).
    ///
    /// Canonical state moved since validation; nothing committed and the transaction is Stale.
    Stale {
        /// The `ekr.kernel.TransactionStale` this outcome publishes.
        transaction_stale: TransactionStale,
        /// The `ekr.store.PublicationPrepared` this outcome publishes.
        publication_prepared: crate::store::PublicationPrepared,
        /// The `ekr.store.ObjectStored` this outcome publishes.
        object_stored: crate::store::ObjectStored,
    },
    /// `retained-commit` — when the existing subject is in Committed.
    ///
    /// Return this transaction's original CommitReceiptV1, including after head advancement; preserve the transaction and publish no event or object.
    RetainedCommit {
        /// Actual response returned by this branch.
        response: CommitResponse,
    },
    /// `transaction-not-found` — externally decided (no retained transaction carries input.transaction_id).
    ///
    /// Refuse before state or staleness evaluation; write no object, receipt or event.
    TransactionNotFound {
        /// Why it was refused: `ekr.kernel.TransactionNotFound`.
        error: TransactionNotFound,
    },
    /// `wrong-state` — otherwise.
    ///
    /// Proposed, Rejected and Stale transactions refuse without a commit, event or subject change.
    WrongState {
        /// Why it was refused: `ekr.kernel.TransactionStateConflict`.
        error: TransactionStateConflict,
    },
}

/// Explain an assertion — the input of `ekr.kernel.Explain`.
///
/// Everything it can result in is [`ExplainOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explain {
    /// `assertion_id` — `ekr.graph.AssertionId`.
    pub assertion_id: crate::graph::AssertionId,
}

/// Actual typed response of `ekr.kernel.Explain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainResponse {
    /// `result` — `ekr.kernel.ExplanationResult`.
    pub result: ExplanationResult,
}

/// Everything `ekr.kernel.Explain` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplainOutcome {
    /// `explained` — otherwise.
    ///
    /// The chain from the assertion to its external sources, one link per line.
    Explained {
        /// The `ekr.kernel.Explained` this outcome publishes.
        explained: Explained,
    },
    /// `not-found` — externally decided (the canonical core holds no such assertion).
    ///
    /// Nothing was explained.
    NotFound {
        /// Why it was refused: `ekr.kernel.AssertionNotFound`.
        error: AssertionNotFound,
    },
}

/// ListAttention — the input of `ekr.kernel.ListAttention`.
///
/// Everything it can result in is [`ListAttentionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAttention {
}

/// Actual typed response of `ekr.kernel.ListAttention`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAttentionResponse {
    /// `items` — `List<ekr.kernel.AttentionItem>`.
    pub items: Vec<AttentionItem>,
}

/// Everything `ekr.kernel.ListAttention` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListAttentionOutcome {
    /// `answered` — otherwise.
    ///
    /// Project disputes, pending integration blockers and schema proposals from retained state. Exclude settled questions and include all unresolved ones; this is not a queue database.
    Answered {
        /// The `ekr.kernel.ListAttentionResult` this outcome publishes.
        list_attention_result: ListAttentionResult,
    },
}

/// PreviewUpgrade — the input of `ekr.kernel.PreviewUpgrade`.
///
/// Everything it can result in is [`PreviewUpgradeOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewUpgrade {
    /// `target` — `ekr.kernel.AuthorityVersion`.
    pub target: AuthorityVersion,
}

/// Actual typed response of `ekr.kernel.PreviewUpgrade`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewUpgradeResponse {
    /// `preview` — `ekr.kernel.UpgradePreview`.
    pub preview: UpgradePreview,
}

/// Everything `ekr.kernel.PreviewUpgrade` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewUpgradeOutcome {
    /// `answered` — otherwise.
    ///
    /// Read the exact head, simulate newly disputed claims under target rules and list pending transactions requiring revalidation. Do not mutate authority or graph.
    Answered {
        /// The `ekr.kernel.PreviewUpgradeResult` this outcome publishes.
        preview_upgrade_result: PreviewUpgradeResult,
    },
    /// `refused` — externally decided (Unknown rule versions, unverified history and unsupported transitions are refused.).
    Refused {
        /// Why it was refused: `ekr.kernel.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// Propose a transaction — the input of `ekr.kernel.Propose`.
///
/// Everything it can result in is [`ProposeOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Propose {
    /// `transaction_document` — `ekr.kernel.TransactionDocumentPath`.
    pub transaction_document: TransactionDocumentPath,
}

/// Actual typed response of `ekr.kernel.Propose`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposeResponse {
    /// `result` — `ekr.kernel.ProposalRecordV1`.
    pub result: ProposalRecordV1,
}

/// Everything `ekr.kernel.Propose` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposeOutcome {
    /// `proposed` — otherwise.
    ///
    /// Exact document bytes and trusted submitter are retained; nothing canonical changed.
    Proposed {
        /// The `ekr.kernel.TransactionProposed` this outcome publishes.
        transaction_proposed: TransactionProposed,
        /// The `ekr.store.PublicationPrepared` this outcome publishes.
        publication_prepared: crate::store::PublicationPrepared,
        /// The `ekr.store.ObjectStored` this outcome publishes.
        object_stored: crate::store::ObjectStored,
    },
    /// `malformed` — externally decided (strict document parsing or structural admission refuses before recording).
    ///
    /// Unknown versions, documents over their version's frozen limits, malformed documents and empty operation lists record nothing.
    Malformed {
        /// Why it was refused: `ekr.kernel.StructurallyInvalid`.
        error: StructurallyInvalid,
    },
    /// `misattributed` — externally decided (the document names a proposer other than the trusted submitter, or the submitter is unregistered).
    ///
    /// The proposer is bound to the host submitter; a mismatch records nothing.
    Misattributed {
        /// Why it was refused: `ekr.kernel.ProposalAttribution`.
        error: ProposalAttribution,
    },
}

/// Load the seed — the input of `ekr.kernel.Seed`.
///
/// Everything it can result in is [`SeedOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seed {
    /// `seed_document` — `ekr.kernel.SeedDocumentPath`.
    pub seed_document: SeedDocumentPath,
}

/// Actual typed response of `ekr.kernel.Seed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedResponse {
    /// `result` — `ekr.kernel.SeedResultV1`.
    pub result: SeedResultV1,
}

/// Everything `ekr.kernel.Seed` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedOutcome {
    /// `seeded` — otherwise.
    ///
    /// Revision0 and its complete retained seed result exist atomically; real ontology and authority roots have no placeholders.
    Seeded {
        /// Actual response returned by this branch.
        response: SeedResponse,
        /// The `ekr.kernel.Seeded` this outcome publishes.
        seeded: Seeded,
        /// The `ekr.store.PublicationPrepared` this outcome publishes.
        publication_prepared: crate::store::PublicationPrepared,
        /// The `ekr.store.ObjectStored` this outcome publishes.
        object_stored: crate::store::ObjectStored,
        /// The `ekr.store.CheckpointWritten` this outcome publishes.
        checkpoint_written: crate::store::CheckpointWritten,
    },
    /// `already-seeded` — externally decided (a retained seed exists and its parsed full Seed2 input or actual BootstrapContext or trusted host AuthorityStateV1 anchor differs).
    ///
    /// Nothing was written; compare retained typed input and actual trusted context before allocating an occurrence or sampling time. YAML whitespace alone is not a different logical seed.
    AlreadySeeded {
        /// Why it was refused: `ekr.kernel.AlreadySeeded`.
        error: AlreadySeeded,
    },
    /// `retained-seed` — externally decided (a retained seed has the same full parsed Seed2 input, actual BootstrapContext and trusted host AuthorityStateV1 anchor).
    ///
    /// Return the original SeedResultV1 and preserve its original Revision, including after head advancement; allocate no new occurrence, sample no new time and write nothing.
    RetainedSeed {
        /// Actual response returned by this branch.
        response: SeedResponse,
    },
    /// `invalid-seed` — externally decided (deterministic bootstrap validation refuses the seed).
    ///
    /// Nothing was written; the refusal identifies the violated bootstrap rule.
    InvalidSeed {
        /// Why it was refused: `ekr.kernel.InvalidSeed`.
        error: InvalidSeed,
    },
}

/// ShowAttention — the input of `ekr.kernel.ShowAttention`.
///
/// Everything it can result in is [`ShowAttentionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowAttention {
    /// `subject` — `ekr.kernel.AttentionSubject`.
    pub subject: AttentionSubject,
}

/// Actual typed response of `ekr.kernel.ShowAttention`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowAttentionResponse {
    /// `item` — `ekr.kernel.AttentionItem`.
    pub item: AttentionItem,
}

/// Everything `ekr.kernel.ShowAttention` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowAttentionOutcome {
    /// `answered` — otherwise.
    ///
    /// Show both claims with retained evidence and a useful clarification question. The viewer renders the same read-only projection.
    Answered {
        /// The `ekr.kernel.ShowAttentionResult` this outcome publishes.
        show_attention_result: ShowAttentionResult,
    },
    /// `refused` — externally decided (Refuse an attention subject whose kind and supplied identity disagree or whose source does not exist.).
    Refused {
        /// Why it was refused: `ekr.kernel.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// Read a snapshot — the input of `ekr.kernel.Snapshot`.
///
/// Everything it can result in is [`SnapshotOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// `at` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at: Option<RevisionNumber>,
    /// `valid_at` — `Optional<Timestamp>`.
    pub valid_at: Option<crate::primitives::Timestamp>,
}

/// Actual typed response of `ekr.kernel.Snapshot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotResponse {
    /// `result` — `ekr.kernel.SnapshotResult`.
    pub result: SnapshotResult,
}

/// Everything `ekr.kernel.Snapshot` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotOutcome {
    /// `taken` — otherwise.
    ///
    /// The snapshot is consistent as of one revision and records which.
    Taken {
        /// The `ekr.kernel.SnapshotTaken` this outcome publishes.
        snapshot_taken: SnapshotTaken,
    },
    /// `not-found` — externally decided (no revision carries the requested number).
    ///
    /// Nothing was read.
    NotFound {
        /// Why it was refused: `ekr.kernel.RevisionNotFound`.
        error: RevisionNotFound,
    },
}

/// Validate a transaction — the input of `ekr.kernel.Validate`.
///
/// Everything it can result in is [`ValidateOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validate {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `against` — `ekr.kernel.RevisionNumber`.
    pub against: RevisionNumber,
}

/// Actual typed response of `ekr.kernel.Validate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateResponse {
    /// `result` — `ekr.kernel.ValidationCommandResult`.
    pub result: ValidationCommandResult,
}

/// Everything `ekr.kernel.Validate` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateOutcome {
    /// `validated` — otherwise.
    ///
    /// Every validator passed against the named revision; the transaction is Validated.
    Validated {
        /// The `ekr.kernel.TransactionValidated` this outcome publishes.
        transaction_validated: TransactionValidated,
        /// The `ekr.store.PublicationPrepared` this outcome publishes.
        publication_prepared: crate::store::PublicationPrepared,
        /// The `ekr.store.ObjectStored` this outcome publishes.
        object_stored: crate::store::ObjectStored,
    },
    /// `rejected` — externally decided (at least one deterministic validator raised an issue against the named revision).
    ///
    /// At least one validator raised an issue; the issues are recorded and the transaction is Rejected.
    Rejected {
        /// The `ekr.kernel.TransactionRejected` this outcome publishes.
        transaction_rejected: TransactionRejected,
        /// The `ekr.store.PublicationPrepared` this outcome publishes.
        publication_prepared: crate::store::PublicationPrepared,
        /// The `ekr.store.ObjectStored` this outcome publishes.
        object_stored: crate::store::ObjectStored,
    },
    /// `revision-not-found` — externally decided (the transaction is Proposed and no committed revision carries input.against).
    ///
    /// Leave the transaction Proposed with no validation or rejection receipt or event; an older existing revision is still a valid basis and may become Stale only at Commit.
    RevisionNotFound {
        /// Why it was refused: `ekr.kernel.RevisionNotFound`.
        error: RevisionNotFound,
    },
    /// `transaction-not-found` — externally decided (no retained transaction carries input.transaction_id).
    ///
    /// Refuse before state or revision-basis evaluation; write no object, receipt, issue or event.
    TransactionNotFound {
        /// Why it was refused: `ekr.kernel.TransactionNotFound`.
        error: TransactionNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The transaction is not Proposed, so nothing was validated.
    WrongState {
        /// Why it was refused: `ekr.kernel.TransactionStateConflict`.
        error: TransactionStateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// AnswerAttentionResult — the event `ekr.kernel.AnswerAttentionResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerAttentionResult {
    /// `receipt` — `ekr.kernel.AnswerReceipt`.
    pub receipt: AnswerReceipt,
}

/// ApplyUpgradeResult — the event `ekr.kernel.ApplyUpgradeResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyUpgradeResult {
    /// `transition_id` — `ekr.kernel.AuthorityTransitionId`.
    pub transition_id: AuthorityTransitionId,
}

/// Explained — the event `ekr.kernel.Explained`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explained {
    /// `assertion_id` — `ekr.graph.AssertionId`.
    pub assertion_id: crate::graph::AssertionId,
    /// `links` — `Integer`.
    pub links: i64,
}

/// ListAttentionResult — the event `ekr.kernel.ListAttentionResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAttentionResult {
    /// `items` — `List<ekr.kernel.AttentionItem>`.
    pub items: Vec<AttentionItem>,
}

/// PreviewUpgradeResult — the event `ekr.kernel.PreviewUpgradeResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewUpgradeResult {
    /// `preview` — `ekr.kernel.UpgradePreview`.
    pub preview: UpgradePreview,
}

/// RevisionCommitted — the event `ekr.kernel.RevisionCommitted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionCommitted {
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `record_hash` — `ekr.kernel.ContentHash`.
    pub record_hash: ContentHash,
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `revision_id` — `ekr.kernel.RevisionId`.
    pub revision_id: RevisionId,
    /// `number` — `ekr.kernel.RevisionNumber`.
    pub number: RevisionNumber,
    /// `knowledge_root` — `ekr.kernel.ContentHash`.
    pub knowledge_root: ContentHash,
}

/// Seeded — the event `ekr.kernel.Seeded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seeded {
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `record_hash` — `ekr.kernel.ContentHash`.
    pub record_hash: ContentHash,
    /// `revision_id` — `ekr.kernel.RevisionId`.
    pub revision_id: RevisionId,
    /// `seed_hash` — `ekr.kernel.ContentHash`.
    pub seed_hash: ContentHash,
}

/// ShowAttentionResult — the event `ekr.kernel.ShowAttentionResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowAttentionResult {
    /// `item` — `ekr.kernel.AttentionItem`.
    pub item: AttentionItem,
}

/// SnapshotTaken — the event `ekr.kernel.SnapshotTaken`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotTaken {
    /// `number` — `ekr.kernel.RevisionNumber`.
    pub number: RevisionNumber,
    /// `knowledge_root` — `ekr.kernel.ContentHash`.
    pub knowledge_root: ContentHash,
}

/// TransactionProposed — the event `ekr.kernel.TransactionProposed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionProposed {
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `record_hash` — `ekr.kernel.ContentHash`.
    pub record_hash: ContentHash,
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `proposer` — `ekr.kernel.AgentId`.
    pub proposer: AgentId,
    /// `operations_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub operations_hash: Option<ContentHash>,
}

/// TransactionRejected — the event `ekr.kernel.TransactionRejected`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionRejected {
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `record_hash` — `ekr.kernel.ContentHash`.
    pub record_hash: ContentHash,
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `issues` — `Integer`.
    pub issues: i64,
}

/// TransactionStale — the event `ekr.kernel.TransactionStale`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionStale {
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `record_hash` — `ekr.kernel.ContentHash`.
    pub record_hash: ContentHash,
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `validated_against` — `ekr.kernel.RevisionNumber`.
    pub validated_against: RevisionNumber,
    /// `current` — `ekr.kernel.RevisionNumber`.
    pub current: RevisionNumber,
}

/// TransactionValidated — the event `ekr.kernel.TransactionValidated`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionValidated {
    /// `event_id` — `ekr.kernel.EventId`.
    pub event_id: EventId,
    /// `record_hash` — `ekr.kernel.ContentHash`.
    pub record_hash: ContentHash,
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `against` — `ekr.kernel.RevisionNumber`.
    pub against: RevisionNumber,
    /// `validation_hash` — `ekr.kernel.ContentHash`.
    pub validation_hash: ContentHash,
}

/// The declared error `ekr.kernel.AlreadySeeded`.
///
/// A retained seed exists but its full parsed input, actual bootstrap context or trusted host authority anchor differs; an exact logical retry returns its retained result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlreadySeeded {
    /// `current` — `ekr.kernel.RevisionNumber`.
    pub current: RevisionNumber,
}

/// The declared error `ekr.kernel.AssertionNotFound`.
///
/// No assertion carries that identity in the canonical core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionNotFound {
    /// `requested` — `ekr.graph.AssertionId`.
    pub requested: crate::graph::AssertionId,
}

/// The declared error `ekr.kernel.InvalidSeed`.
///
/// Seed admission refused before canonical state or its lineage was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidSeed {
    /// `code` — `String`.
    pub code: String,
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `ekr.kernel.KnowledgeRefused`.
///
/// A named deterministic refusal; no unreported write occurred. Partial application is a typed report, never this refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeRefused {
    /// `code` — `String`.
    pub code: String,
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `ekr.kernel.ProposalAttribution`.
///
/// The trusted submitter is unregistered or differs from the proposer the document names; nothing was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalAttribution {
    /// `actor` — `ekr.kernel.AgentId`.
    pub actor: AgentId,
}

/// The declared error `ekr.kernel.RevisionNotFound`.
///
/// No committed revision carries that number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionNotFound {
    /// `requested` — `ekr.kernel.RevisionNumber`.
    pub requested: RevisionNumber,
}

/// The declared error `ekr.kernel.StructurallyInvalid`.
///
/// The proposal is not a well-formed transaction; nothing was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructurallyInvalid {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `ekr.kernel.TransactionNotFound`.
///
/// No retained transaction carries the requested identity; no state or decision is invented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionNotFound {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
}

/// The declared error `ekr.kernel.TransactionStateConflict`.
///
/// The transaction is not in a state this command acts from, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionStateConflict {
    /// `state` — `ekr.kernel.GraphTransaction.State`.
    pub state: GraphTransactionState,
}

/// AuthorityTransitionRecords — one row of the view `ekr.kernel.AuthorityTransitionRecords`.
///
/// Projects `ekr.kernel.AuthorityTransition` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityTransitionRecords {
    /// `review_host_binding_digest` — `ekr.kernel.ContentHash`.
    pub review_host_binding_digest: ContentHash,
    /// `reviewer_policy_digest` — `ekr.kernel.ContentHash`.
    pub reviewer_policy_digest: ContentHash,
    /// `trust_enrollment` — `ekr.kernel.ReviewerTrustEnrollment`.
    pub trust_enrollment: ReviewerTrustEnrollment,
    /// `human_proof_digest` — `ekr.kernel.ContentHash`.
    pub human_proof_digest: ContentHash,
    /// `target_profile_digest` — `ekr.kernel.ContentHash`.
    pub target_profile_digest: ContentHash,
    /// `transition_id` — `ekr.kernel.AuthorityTransitionId`.
    pub transition_id: AuthorityTransitionId,
    /// `state` — `ekr.kernel.AuthorityTransition.State`.
    pub state: AuthorityTransitionState,
    /// `format` — `ekr.kernel.AuthorityTransitionFormat`.
    pub format: AuthorityTransitionFormat,
    /// `seed_anchor` — `ekr.kernel.ContentHash`.
    pub seed_anchor: ContentHash,
    /// `predecessor_head` — `ekr.kernel.ContentHash`.
    pub predecessor_head: ContentHash,
    /// `activation_revision` — `ekr.kernel.RevisionNumber`.
    pub activation_revision: RevisionNumber,
    /// `from` — `ekr.kernel.AuthorityVersion`.
    pub from: AuthorityVersion,
    /// `to` — `ekr.kernel.AuthorityVersion`.
    pub to: AuthorityVersion,
    /// `preview_digest` — `ekr.kernel.ContentHash`.
    pub preview_digest: ContentHash,
    /// `operator` — `ekr.kernel.TrustedOperatorIdentity`.
    pub operator: TrustedOperatorIdentity,
    /// `recorded_at` — `Timestamp`.
    pub recorded_at: crate::primitives::Timestamp,
}

/// Current revision — one row of the view `ekr.kernel.CurrentRevision`.
///
/// Projects `ekr.kernel.Revision` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentRevision {
    /// `revision_id` — `ekr.kernel.RevisionId`.
    pub revision_id: RevisionId,
    /// `number` — `ekr.kernel.RevisionNumber`.
    pub number: RevisionNumber,
    /// `knowledge_root` — `ekr.kernel.ContentHash`.
    pub knowledge_root: ContentHash,
}

/// DisputeClaimRecords — one row of the view `ekr.kernel.DisputeClaimRecords`.
///
/// Projects `ekr.kernel.DisputeClaim` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeClaimRecords {
    /// `claim_link_id` — `Uuid`.
    pub claim_link_id: crate::primitives::Uuid,
    /// `state` — `ekr.kernel.DisputeClaim.State`.
    pub state: DisputeClaimState,
    /// `dispute_id` — `ekr.kernel.DisputeId`.
    pub dispute_id: DisputeId,
    /// `assertion_id` — `ekr.graph.AssertionId`.
    pub assertion_id: crate::graph::AssertionId,
}

/// DisputeRecords — one row of the view `ekr.kernel.DisputeRecords`.
///
/// Projects `ekr.kernel.Dispute` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeRecords {
    /// `dispute_id` — `ekr.kernel.DisputeId`.
    pub dispute_id: DisputeId,
    /// `state` — `ekr.kernel.Dispute.State`.
    pub state: DisputeState,
    /// `basis_revision` — `ekr.kernel.RevisionNumber`.
    pub basis_revision: RevisionNumber,
    /// `subject` — `ekr.graph.SubjectProjection`.
    pub subject: crate::graph::SubjectProjection,
    /// `predicate` — `ekr.graph.PredicateProjection`.
    pub predicate: crate::graph::PredicateProjection,
    /// `basis_digest` — `ekr.kernel.ContentHash`.
    pub basis_digest: ContentHash,
}

/// HumanAnswerRecords — one row of the view `ekr.kernel.HumanAnswerRecords`.
///
/// Projects `ekr.kernel.HumanAnswer` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanAnswerRecords {
    /// `human_proof_digest` — `ekr.kernel.ContentHash`.
    pub human_proof_digest: ContentHash,
    /// `answer_id` — `ekr.kernel.HumanAnswerId`.
    pub answer_id: HumanAnswerId,
    /// `state` — `ekr.kernel.HumanAnswer.State`.
    pub state: HumanAnswerState,
    /// `dispute_id` — `ekr.kernel.DisputeId`.
    pub dispute_id: DisputeId,
    /// `basis` — `ekr.kernel.ReviewBasis`.
    pub basis: ReviewBasis,
    /// `operator` — `ekr.kernel.TrustedOperatorIdentity`.
    pub operator: TrustedOperatorIdentity,
    /// `corrections` — `List<ekr.kernel.ClaimCorrection>`.
    pub corrections: Vec<ClaimCorrection>,
    /// `statement_evidence` — `ekr.graph.EvidenceId`.
    pub statement_evidence: crate::graph::EvidenceId,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<TransactionId>,
}

/// HumanDecisionRecords — one row of the view `ekr.kernel.HumanDecisionRecords`.
///
/// Projects `ekr.kernel.RetainedHumanDecision` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanDecisionRecords {
    /// `proof_digest` — `ekr.kernel.ContentHash`.
    pub proof_digest: ContentHash,
    /// `state` — `ekr.kernel.RetainedHumanDecision.State`.
    pub state: RetainedHumanDecisionState,
    /// `decision_id` — `Uuid`.
    pub decision_id: crate::primitives::Uuid,
    /// `policy_digest` — `ekr.kernel.ContentHash`.
    pub policy_digest: ContentHash,
    /// `statement_digest` — `ekr.kernel.ContentHash`.
    pub statement_digest: ContentHash,
    /// `operator` — `ekr.kernel.TrustedOperatorIdentity`.
    pub operator: TrustedOperatorIdentity,
    /// `recorded_at` — `Timestamp`.
    pub recorded_at: crate::primitives::Timestamp,
}

/// Pending transactions — one row of the view `ekr.kernel.PendingTransactions`.
///
/// Projects `ekr.kernel.GraphTransaction` at `read_your_writes` consistency, containing instances where `state == Proposed`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTransactions {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `proposer` — `ekr.kernel.AgentId`.
    pub proposer: AgentId,
    /// `operation_count` — `Optional<Integer>`.
    pub operation_count: Option<i64>,
}

/// Rejections — one row of the view `ekr.kernel.Rejections`.
///
/// Projects `ekr.kernel.ValidationIssue` at `read_your_writes` consistency, containing instances where `((not (defined(param.from)) or against >= param.from) and (not (defined(param.to)) or against <= param.to))`.
/// Serving it is an implementation obligation — see the plan — because how a projection is kept
/// current is a storage decision the specification does not take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejections {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `against` — `ekr.kernel.RevisionNumber`.
    pub against: RevisionNumber,
    /// `issue_id` — `ekr.kernel.IssueId`.
    pub issue_id: IssueId,
    /// `validator` — `ekr.kernel.ValidatorName`.
    pub validator: ValidatorName,
    /// `code` — `String`.
    pub code: String,
    /// `message` — `String`.
    pub message: String,
}

/// Retained evidence — one row of the view `ekr.kernel.RetainedEvidence`.
///
/// Projects `ekr.graph.Evidence` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedEvidence {
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    pub evidence_id: crate::graph::EvidenceId,
    /// `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: ContentHash,
    /// `extracted_by` — `ekr.kernel.AgentId`.
    pub extracted_by: AgentId,
}

/// Revisions — one row of the view `ekr.kernel.Revisions`.
///
/// Projects `ekr.kernel.Revision` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revisions {
    /// `revision_id` — `ekr.kernel.RevisionId`.
    pub revision_id: RevisionId,
    /// `number` — `ekr.kernel.RevisionNumber`.
    pub number: RevisionNumber,
    /// `parent` — `Optional<ekr.kernel.RevisionId>`.
    pub parent: Option<RevisionId>,
    /// `ontology_root` — `ekr.kernel.ContentHash`.
    pub ontology_root: ContentHash,
    /// `knowledge_root` — `ekr.kernel.ContentHash`.
    pub knowledge_root: ContentHash,
    /// `evidence_root` — `ekr.kernel.ContentHash`.
    pub evidence_root: ContentHash,
    /// `agent_root` — `ekr.kernel.ContentHash`.
    pub agent_root: ContentHash,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<TransactionId>,
    /// `committed_at` — `Timestamp`.
    pub committed_at: crate::primitives::Timestamp,
    /// `state` — `ekr.kernel.Revision.State`.
    pub state: RevisionState,
}

/// SchemaTransactionEvidenceRecords — one row of the view `ekr.kernel.SchemaTransactionEvidenceRecords`.
///
/// Projects `ekr.kernel.SchemaTransactionEvidence` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaTransactionEvidenceRecords {
    /// `link_id` — `Uuid`.
    pub link_id: crate::primitives::Uuid,
    /// `state` — `ekr.kernel.SchemaTransactionEvidence.State`.
    pub state: SchemaTransactionEvidenceState,
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    pub evidence_id: crate::graph::EvidenceId,
}

/// Transactions — one row of the view `ekr.kernel.Transactions`.
///
/// Projects `ekr.kernel.GraphTransaction` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transactions {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `proposer` — `ekr.kernel.AgentId`.
    pub proposer: AgentId,
    /// `document_hash` — `ekr.kernel.ContentHash`.
    pub document_hash: ContentHash,
    /// `proposal_record_hash` — `ekr.kernel.ContentHash`.
    pub proposal_record_hash: ContentHash,
    /// `canonical_transaction_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub canonical_transaction_hash: Option<ContentHash>,
    /// `canonical_operations_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub canonical_operations_hash: Option<ContentHash>,
    /// `operation_count` — `Optional<Integer>`.
    pub operation_count: Option<i64>,
    /// `evidence_hash` — `ekr.kernel.ContentHash`.
    pub evidence_hash: ContentHash,
    /// `validation_basis` — `Optional<ekr.kernel.ValidationBasisV1>`.
    pub validation_basis: Option<ValidationBasisV1>,
    /// `validated_against` — `Optional<ekr.kernel.RevisionNumber>`.
    pub validated_against: Option<RevisionNumber>,
    /// `validation_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub validation_hash: Option<ContentHash>,
    /// `validation_record_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub validation_record_hash: Option<ContentHash>,
    /// `terminal_record_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub terminal_record_hash: Option<ContentHash>,
    /// `state` — `ekr.kernel.GraphTransaction.State`.
    pub state: GraphTransactionState,
}

/// Validation issues — one row of the view `ekr.kernel.ValidationIssues`.
///
/// Projects `ekr.kernel.ValidationIssue` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssues {
    /// `issue_id` — `ekr.kernel.IssueId`.
    pub issue_id: IssueId,
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: TransactionId,
    /// `validator` — `ekr.kernel.ValidatorName`.
    pub validator: ValidatorName,
    /// `code` — `String`.
    pub code: String,
    /// `message` — `String`.
    pub message: String,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `ekr.kernel.AnswerAttention` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.kernel.AnswerAttention` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.kernel.AnswerAttentionResult`; `refused` externally decided (Verify human_proof under the independently enrolled reviewer policy and the exact AnswerAttention target, statement, corrections and predecessor decision. A host operator UUID or supplied key establishes no human authority. Revalidate against current state. Unrelated revisions alone do not invalidate review; changed evidence, choices or intended effects require renewed human review. An agent-provided identity or unsupported claim cannot authorize resolution.), error `ekr.kernel.KnowledgeRefused`.
    pub trait AnswerAttentionBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.kernel.AnswerAttention`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn answer_attention(&mut self, input: super::AnswerAttention) -> Result<super::AnswerAttentionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.kernel.ApplyUpgrade` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.kernel.ApplyUpgrade` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.kernel.ApplyUpgradeResult`; `refused` externally decided (Verify an UpgradeAuthority human_proof and the independently provisioned host/store reviewer binding before enrollment or mutation. Never trust a policy or verification key solely because request content supplies it. Before any mutation refuse a stale head, altered preview digest, unsupported target, unauthenticated operator or incomplete historical verification.), error `ekr.kernel.KnowledgeRefused`.
    pub trait ApplyUpgradeBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.kernel.ApplyUpgrade`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn apply_upgrade(&mut self, input: super::ApplyUpgrade) -> Result<super::ApplyUpgradeOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.kernel.Commit` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.kernel.Commit` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `committed` when the existing subject is in Validated, takes `commit` of `ekr.kernel.GraphTransaction`, emits `ekr.kernel.RevisionCommitted`, emits `ekr.store.PublicationPrepared`, emits `ekr.store.ObjectStored`, emits `ekr.store.CheckpointWritten`; `stale` externally decided (the canonical revision moved since the transaction was validated), takes `stale` of `ekr.kernel.GraphTransaction`, emits `ekr.kernel.TransactionStale`, emits `ekr.store.PublicationPrepared`, emits `ekr.store.ObjectStored`; `retained-commit` when the existing subject is in Committed, returns the exact retained result of `committed` without errors, events, or subject changes; `transaction-not-found` externally decided (no retained transaction carries input.transaction_id), error `ekr.kernel.TransactionNotFound`; `wrong-state` otherwise, error `ekr.kernel.TransactionStateConflict`.
    pub trait CommitBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.kernel.Commit`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn commit(&mut self, input: super::Commit) -> Result<super::CommitOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.kernel.Explain` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.kernel.Explain` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `explained` otherwise, emits `ekr.kernel.Explained`; `not-found` externally decided (the canonical core holds no such assertion), error `ekr.kernel.AssertionNotFound`.
    pub trait ExplainBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.kernel.Explain`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn explain(&mut self, input: super::Explain) -> Result<super::ExplainOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.kernel.ListAttention` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.kernel.ListAttention` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.kernel.ListAttentionResult`.
    pub trait ListAttentionBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.kernel.ListAttention`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn list_attention(&mut self, input: super::ListAttention) -> Result<super::ListAttentionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.kernel.PreviewUpgrade` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.kernel.PreviewUpgrade` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.kernel.PreviewUpgradeResult`; `refused` externally decided (Unknown rule versions, unverified history and unsupported transitions are refused.), error `ekr.kernel.KnowledgeRefused`.
    pub trait PreviewUpgradeBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.kernel.PreviewUpgrade`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn preview_upgrade(&mut self, input: super::PreviewUpgrade) -> Result<super::PreviewUpgradeOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.kernel.Propose` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.kernel.Propose` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `proposed` otherwise, creates `ekr.kernel.GraphTransaction`, emits `ekr.kernel.TransactionProposed`, emits `ekr.store.PublicationPrepared`, emits `ekr.store.ObjectStored`; `malformed` externally decided (strict document parsing or structural admission refuses before recording), error `ekr.kernel.StructurallyInvalid`; `misattributed` externally decided (the document names a proposer other than the trusted submitter, or the submitter is unregistered), error `ekr.kernel.ProposalAttribution`.
    pub trait ProposeBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.kernel.Propose`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn propose(&mut self, input: super::Propose) -> Result<super::ProposeOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.kernel.Seed` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.kernel.Seed` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `seeded` otherwise, creates `ekr.kernel.Revision`, emits `ekr.kernel.Seeded`, emits `ekr.store.PublicationPrepared`, emits `ekr.store.ObjectStored`, emits `ekr.store.CheckpointWritten`; `already-seeded` externally decided (a retained seed exists and its parsed full Seed2 input or actual BootstrapContext or trusted host AuthorityStateV1 anchor differs), error `ekr.kernel.AlreadySeeded`; `retained-seed` externally decided (a retained seed has the same full parsed Seed2 input, actual BootstrapContext and trusted host AuthorityStateV1 anchor), returns the exact retained result of `seeded` without errors, events, or subject changes; `invalid-seed` externally decided (deterministic bootstrap validation refuses the seed), error `ekr.kernel.InvalidSeed`.
    pub trait SeedBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.kernel.Seed`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn seed(&mut self, input: super::Seed) -> Result<super::SeedOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.kernel.ShowAttention` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.kernel.ShowAttention` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.kernel.ShowAttentionResult`; `refused` externally decided (Refuse an attention subject whose kind and supplied identity disagree or whose source does not exist.), error `ekr.kernel.KnowledgeRefused`.
    pub trait ShowAttentionBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.kernel.ShowAttention`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn show_attention(&mut self, input: super::ShowAttention) -> Result<super::ShowAttentionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.kernel.Snapshot` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.kernel.Snapshot` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `taken` otherwise, emits `ekr.kernel.SnapshotTaken`; `not-found` externally decided (no revision carries the requested number), error `ekr.kernel.RevisionNotFound`.
    pub trait SnapshotBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.kernel.Snapshot`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn snapshot(&mut self, input: super::Snapshot) -> Result<super::SnapshotOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.kernel.Validate` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.kernel.Validate` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `validated` otherwise, takes `validate` of `ekr.kernel.GraphTransaction`, emits `ekr.kernel.TransactionValidated`, emits `ekr.store.PublicationPrepared`, emits `ekr.store.ObjectStored`; `rejected` externally decided (at least one deterministic validator raised an issue against the named revision), takes `reject` of `ekr.kernel.GraphTransaction`, emits `ekr.kernel.TransactionRejected`, emits `ekr.store.PublicationPrepared`, emits `ekr.store.ObjectStored`; `revision-not-found` externally decided (the transaction is Proposed and no committed revision carries input.against), error `ekr.kernel.RevisionNotFound`; `transaction-not-found` externally decided (no retained transaction carries input.transaction_id), error `ekr.kernel.TransactionNotFound`; `wrong-state` from a state no declared move starts in, error `ekr.kernel.TransactionStateConflict`, and for an instance no record carries, without the error's fields.
    pub trait ValidateBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.kernel.Validate`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn validate(&mut self, input: super::Validate) -> Result<super::ValidateOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.AuthorityTransitionRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait AuthorityTransitionRecordsQuery {
        /// Serves `ekr.kernel.AuthorityTransitionRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn authority_transition_records(&self) -> Result<Vec<super::AuthorityTransitionRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.CurrentRevision` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait CurrentRevisionQuery {
        /// Serves `ekr.kernel.CurrentRevision` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn current_revision(&self) -> Result<Vec<super::CurrentRevision>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.DisputeClaimRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait DisputeClaimRecordsQuery {
        /// Serves `ekr.kernel.DisputeClaimRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn dispute_claim_records(&self) -> Result<Vec<super::DisputeClaimRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.DisputeRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait DisputeRecordsQuery {
        /// Serves `ekr.kernel.DisputeRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn dispute_records(&self) -> Result<Vec<super::DisputeRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.HumanAnswerRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait HumanAnswerRecordsQuery {
        /// Serves `ekr.kernel.HumanAnswerRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn human_answer_records(&self) -> Result<Vec<super::HumanAnswerRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.HumanDecisionRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait HumanDecisionRecordsQuery {
        /// Serves `ekr.kernel.HumanDecisionRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn human_decision_records(&self) -> Result<Vec<super::HumanDecisionRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.PendingTransactions` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait PendingTransactionsQuery {
        /// Serves `ekr.kernel.PendingTransactions` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn pending_transactions(&self) -> Result<Vec<super::PendingTransactions>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.Rejections` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a view parameter (`params:`), which a generated query does not apply.
    ///
    /// Contract: a query answering `ekr.kernel.Rejections` with rows projected from `ekr.kernel.ValidationIssue` at `read_your_writes` consistency, containing instances where `((not (defined(param.from)) or against >= param.from) and (not (defined(param.to)) or against <= param.to))`.
    pub trait RejectionsQuery {
        /// Serves `ekr.kernel.Rejections` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn rejections(&self, from: Option<crate::kernel::RevisionNumber>, to: Option<crate::kernel::RevisionNumber>) -> Result<Vec<super::Rejections>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.RetainedEvidence` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait RetainedEvidenceQuery {
        /// Serves `ekr.kernel.RetainedEvidence` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn retained_evidence(&self) -> Result<Vec<super::RetainedEvidence>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.Revisions` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait RevisionsQuery {
        /// Serves `ekr.kernel.Revisions` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn revisions(&self) -> Result<Vec<super::Revisions>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.SchemaTransactionEvidenceRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SchemaTransactionEvidenceRecordsQuery {
        /// Serves `ekr.kernel.SchemaTransactionEvidenceRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn schema_transaction_evidence_records(&self) -> Result<Vec<super::SchemaTransactionEvidenceRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.Transactions` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait TransactionsQuery {
        /// Serves `ekr.kernel.Transactions` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn transactions(&self) -> Result<Vec<super::Transactions>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.kernel.ValidationIssues` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ValidationIssuesQuery {
        /// Serves `ekr.kernel.ValidationIssues` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn validation_issues(&self) -> Result<Vec<super::ValidationIssues>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl AnswerAttentionBehavior for Unimplemented {
        fn answer_attention(&mut self, _input: super::AnswerAttention) -> Result<super::AnswerAttentionOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.kernel.AnswerAttention" })
        }
    }

    impl ApplyUpgradeBehavior for Unimplemented {
        fn apply_upgrade(&mut self, _input: super::ApplyUpgrade) -> Result<super::ApplyUpgradeOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.kernel.ApplyUpgrade" })
        }
    }

    impl CommitBehavior for Unimplemented {
        fn commit(&mut self, _input: super::Commit) -> Result<super::CommitOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.kernel.Commit" })
        }
    }

    impl ExplainBehavior for Unimplemented {
        fn explain(&mut self, _input: super::Explain) -> Result<super::ExplainOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.kernel.Explain" })
        }
    }

    impl ListAttentionBehavior for Unimplemented {
        fn list_attention(&mut self, _input: super::ListAttention) -> Result<super::ListAttentionOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.kernel.ListAttention" })
        }
    }

    impl PreviewUpgradeBehavior for Unimplemented {
        fn preview_upgrade(&mut self, _input: super::PreviewUpgrade) -> Result<super::PreviewUpgradeOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.kernel.PreviewUpgrade" })
        }
    }

    impl ProposeBehavior for Unimplemented {
        fn propose(&mut self, _input: super::Propose) -> Result<super::ProposeOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.kernel.Propose" })
        }
    }

    impl SeedBehavior for Unimplemented {
        fn seed(&mut self, _input: super::Seed) -> Result<super::SeedOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.kernel.Seed" })
        }
    }

    impl ShowAttentionBehavior for Unimplemented {
        fn show_attention(&mut self, _input: super::ShowAttention) -> Result<super::ShowAttentionOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.kernel.ShowAttention" })
        }
    }

    impl SnapshotBehavior for Unimplemented {
        fn snapshot(&mut self, _input: super::Snapshot) -> Result<super::SnapshotOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.kernel.Snapshot" })
        }
    }

    impl ValidateBehavior for Unimplemented {
        fn validate(&mut self, _input: super::Validate) -> Result<super::ValidateOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.kernel.Validate" })
        }
    }

    impl RejectionsQuery for Unimplemented {
        fn rejections(&self, _from: Option<crate::kernel::RevisionNumber>, _to: Option<crate::kernel::RevisionNumber>) -> Result<Vec<super::Rejections>, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "view query", source: "ekr.kernel.Rejections" })
        }
    }
}
