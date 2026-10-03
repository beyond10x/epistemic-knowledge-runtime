// generated from ekr v1
// model digest 76cac94520a78179089e10b0871c7979eacd35197c8e7b584616fac60d49e705
// contract digest c0793b41517a2b54756a5ae9c6d0c85aa53885defe0f88328a27a2c46c880d14
// do not edit: regenerate with `ess synthesize`

//! Integrate — `ekr.integrate`.
//!
//! Entity resolution and the merge and split lineage around it. Design § 10, § 45, § 46 and amendment A10 (docs/predecessors.md § 2): a typed reference resolves against canonical nodes by identity, never by string matching, and a merge or a split is an explicit transaction with provenance. Its one command, ApplyExtraction, applies an extraction document; the resolver's verbs and the merge and split transactions are declared once the decision-blockers are answered.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// AdditiveSchemaOperation — `ekr.integrate.AdditiveSchemaOperation`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdditiveSchemaOperation {
    /// Tagged `AddOptionalProperty` — `ekr.integrate.OptionalPropertyAddition`.
    AddOptionalProperty(OptionalPropertyAddition),
    /// Tagged `DefineRelation` — `ekr.integrate.EdgeTypeSpec`.
    DefineRelation(EdgeTypeSpec),
    /// Tagged `DefineType` — `ekr.integrate.NodeTypeSpec`.
    DefineType(NodeTypeSpec),
}

/// AmbiguousExtraction — `ekr.integrate.AmbiguousExtraction`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbiguousExtraction {
    /// `reference` — `ekr.integrate.ExtractedReference`.
    pub reference: ExtractedReference,
    /// `candidates` — `List<ekr.graph.NodeId>`.
    pub candidates: Vec<crate::graph::NodeId>,
}

/// AmbiguousReference — `ekr.integrate.AmbiguousReference`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbiguousReference {
    /// `candidates` — `List<ekr.graph.NodeId>`.
    pub candidates: Vec<crate::graph::NodeId>,
}

/// ApplicationProgress — `ekr.integrate.ApplicationProgress`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationProgress {
    /// `SchemaCommitted`.
    SchemaCommitted,
    /// `FactsInProgress`.
    FactsInProgress,
    /// `Complete`.
    Complete,
    /// `Partial`.
    Partial,
}

/// The states of `ekr.integrate.ApplicationReceipt`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ApplicationReceipt<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationReceiptState {
    /// `Recorded`.
    Recorded,
}

/// ApplicationReceiptId — `ekr.integrate.ApplicationReceiptId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationReceiptId(pub crate::primitives::Uuid);

/// ApplicationReceiptSnapshot — `ekr.integrate.ApplicationReceiptSnapshot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationReceiptSnapshot {
    /// `receipt_id` — `ekr.integrate.ApplicationReceiptId`.
    pub receipt_id: ApplicationReceiptId,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `review_id` — `ekr.integrate.ProposalReviewId`.
    pub review_id: ProposalReviewId,
    /// `progress` — `ekr.integrate.ApplicationProgress`.
    pub progress: ApplicationProgress,
    /// `schema_transaction` — `ekr.kernel.TransactionId`.
    pub schema_transaction: crate::kernel::TransactionId,
    /// `schema_revision` — `ekr.kernel.RevisionNumber`.
    pub schema_revision: crate::kernel::RevisionNumber,
    /// `processing_receipts` — `List<ekr.integrate.ProcessingReceiptId>`.
    pub processing_receipts: Vec<ProcessingReceiptId>,
    /// `remaining_items` — `List<String>`.
    pub remaining_items: Vec<String>,
    /// `stop_reason` — `Optional<String>`.
    pub stop_reason: Option<String>,
}

/// ApplicationReport — `ekr.integrate.ApplicationReport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationReport {
    /// `receipt_id` — `ekr.integrate.ApplicationReceiptId`.
    pub receipt_id: ApplicationReceiptId,
    /// `progress` — `ekr.integrate.ApplicationProgress`.
    pub progress: ApplicationProgress,
    /// `schema_transaction` — `ekr.kernel.TransactionId`.
    pub schema_transaction: crate::kernel::TransactionId,
    /// `schema_revision` — `ekr.kernel.RevisionNumber`.
    pub schema_revision: crate::kernel::RevisionNumber,
    /// `items` — `List<ekr.integrate.IntegrationItemReceipt>`.
    pub items: Vec<IntegrationItemReceipt>,
    /// `remaining_items` — `List<String>`.
    pub remaining_items: Vec<String>,
    /// `stop_reason` — `Optional<String>`.
    pub stop_reason: Option<String>,
    /// `already_complete` — `Boolean`.
    pub already_complete: bool,
}

/// The states of `ekr.integrate.CanonicalDerivation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `CanonicalDerivation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalDerivationState {
    /// `Recorded`.
    Recorded,
}

/// CommittedExtraction — `ekr.integrate.CommittedExtraction`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedExtraction {
    /// `transaction_id` — `ekr.kernel.TransactionId`.
    pub transaction_id: crate::kernel::TransactionId,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
}

/// DeclaredSourceField — `ekr.integrate.DeclaredSourceField`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredSourceField {
    /// `declaration` — `String`.
    pub declaration: String,
    /// `field` — `String`.
    pub field: String,
}

/// DeclaredSourceRelation — `ekr.integrate.DeclaredSourceRelation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredSourceRelation {
    /// `declaration` — `String`.
    pub declaration: String,
    /// `relation` — `String`.
    pub relation: String,
}

/// EdgeTypeSpec — `ekr.integrate.EdgeTypeSpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeTypeSpec {
    /// `name` — `String`.
    pub name: String,
    /// `source_types` — `List<String>`.
    pub source_types: Vec<String>,
    /// `target_types` — `List<String>`.
    pub target_types: Vec<String>,
    /// `cardinality` — `ekr.ontology.Cardinality`.
    pub cardinality: crate::ontology::Cardinality,
    /// `properties` — `List<ekr.integrate.PropertySpec>`.
    pub properties: Vec<PropertySpec>,
}

/// ExtractedFact — `ekr.integrate.ExtractedFact`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractedFact {
    /// Tagged `Property` — `ekr.integrate.PropertyFact`.
    Property(PropertyFact),
    /// Tagged `Relation` — `ekr.integrate.RelationFact`.
    Relation(RelationFact),
}

/// ExtractedReference — `ekr.integrate.ExtractedReference`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedReference {
    /// `node_type` — `String`.
    pub node_type: String,
    /// `aliases` — `List<String>`.
    pub aliases: Vec<String>,
}

/// ExtractionDocument — `ekr.integrate.ExtractionDocument`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionDocument {
    /// `format` — `ekr.integrate.ExtractionFormat`.
    pub format: ExtractionFormat,
    /// `ontology` — `ekr.integrate.OntologySpec`.
    pub ontology: OntologySpec,
    /// `entities` — `List<ekr.integrate.ExtractedReference>`.
    pub entities: Vec<ExtractedReference>,
    /// `facts` — `List<ekr.integrate.ExtractedFact>`.
    pub facts: Vec<ExtractedFact>,
    /// `evidence` — `List<ekr.kernel.EvidenceAdditionProjection>`.
    pub evidence: Vec<crate::kernel::EvidenceAdditionProjection>,
}

/// ExtractionDocumentPath — `ekr.integrate.ExtractionDocumentPath`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionDocumentPath(pub String);

/// ExtractionFormat — `ekr.integrate.ExtractionFormat`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionFormat {
    /// `EkrExtractionDocument1`.
    EkrExtractionDocument1,
}

/// ExtractionIssue — `ekr.integrate.ExtractionIssue`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionIssue {
    /// `validator` — `ekr.kernel.ValidatorName`.
    pub validator: crate::kernel::ValidatorName,
    /// `code` — `String`.
    pub code: String,
    /// `message` — `String`.
    pub message: String,
}

/// ExtractionRefusal — `ekr.integrate.ExtractionRefusal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionRefusal {
    /// `code` — `ekr.integrate.ExtractionRefusalCode`.
    pub code: ExtractionRefusalCode,
    /// `name` — `String`.
    pub name: String,
}

/// ExtractionRefusalCode — `ekr.integrate.ExtractionRefusalCode`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionRefusalCode {
    /// `extraction-document-too-large`.
    ExtractionDocumentTooLarge,
    /// `extraction-document-too-deep`.
    ExtractionDocumentTooDeep,
    /// `extraction-yaml-alias`.
    ExtractionYamlAlias,
    /// `extraction-document-malformed`.
    ExtractionDocumentMalformed,
    /// `extraction-name-duplicate`.
    ExtractionNameDuplicate,
    /// `extraction-type-conflict`.
    ExtractionTypeConflict,
    /// `extraction-type-undeclared`.
    ExtractionTypeUndeclared,
    /// `extraction-property-conflict`.
    ExtractionPropertyConflict,
    /// `extraction-value-type-empty`.
    ExtractionValueTypeEmpty,
    /// `reference-without-identity`.
    ReferenceWithoutIdentity,
    /// `reference-type-has-subtypes`.
    ReferenceTypeHasSubtypes,
    /// `extraction-property-undeclared`.
    ExtractionPropertyUndeclared,
    /// `extraction-value-mismatch`.
    ExtractionValueMismatch,
    /// `extraction-relation-ends`.
    ExtractionRelationEnds,
    /// `fact-without-evidence`.
    FactWithoutEvidence,
    /// `fact-evidence-unlisted`.
    FactEvidenceUnlisted,
    /// `duplicate-identity`.
    DuplicateIdentity,
    /// `extraction-evidence-kind-unsupported`.
    ExtractionEvidenceKindUnsupported,
    /// `evidence-payload-mismatch`.
    EvidencePayloadMismatch,
}

/// ExtractionReport — `ekr.integrate.ExtractionReport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionReport {
    /// `committed` — `List<ekr.integrate.CommittedExtraction>`.
    pub committed: Vec<CommittedExtraction>,
    /// `rejected` — `List<ekr.integrate.RejectedExtraction>`.
    pub rejected: Vec<RejectedExtraction>,
    /// `ambiguous` — `List<ekr.integrate.AmbiguousExtraction>`.
    pub ambiguous: Vec<AmbiguousExtraction>,
    /// `held` — `List<ekr.integrate.HeldExtraction>`.
    pub held: Vec<HeldExtraction>,
    /// `stopped` — `Optional<String>`.
    pub stopped: Option<String>,
}

/// GapGroup — `ekr.integrate.GapGroup`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapGroup {
    /// `group_id` — `ekr.integrate.GapGroupId`.
    pub group_id: GapGroupId,
    /// `kind` — `ekr.integrate.IntegrationBlockerKind`.
    pub kind: IntegrationBlockerKind,
    /// `declaration` — `String`.
    pub declaration: String,
    /// `blockers` — `List<ekr.integrate.IntegrationBlockerId>`.
    pub blockers: Vec<IntegrationBlockerId>,
    /// `observations` — `List<ekr.graph.ObservationId>`.
    pub observations: Vec<crate::graph::ObservationId>,
    /// `sources` — `List<ekr.integrate.InterpretationVersion>`.
    pub sources: Vec<InterpretationVersion>,
}

/// GapGroupId — `ekr.integrate.GapGroupId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapGroupId(pub crate::primitives::Uuid);

/// HeldExtraction — `ekr.integrate.HeldExtraction`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldExtraction {
    /// `item` — `String`.
    pub item: String,
    /// `reason` — `ekr.integrate.HeldReason`.
    pub reason: HeldReason,
}

/// HeldReason — `ekr.integrate.HeldReason`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldReason {
    /// `asserted`.
    Asserted,
    /// `repeated`.
    Repeated,
    /// `retracted`.
    Retracted,
    /// `superseded`.
    Superseded,
}

/// IncubationImportReceipt — `ekr.integrate.IncubationImportReceipt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncubationImportReceipt {
    /// `outcome` — `ekr.integrate.IncubationOutcome`.
    pub outcome: IncubationOutcome,
    /// `version` — `ekr.integrate.InterpretationVersion`.
    pub version: InterpretationVersion,
    /// `already_retained` — `Boolean`.
    pub already_retained: bool,
    /// `blockers` — `List<ekr.integrate.IntegrationBlockerId>`.
    pub blockers: Vec<IntegrationBlockerId>,
}

/// IncubationOutcome — `ekr.integrate.IncubationOutcome`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncubationOutcome {
    /// `Retained`.
    Retained,
    /// `RetainedWithBlockers`.
    RetainedWithBlockers,
    /// `AlreadyRetained`.
    AlreadyRetained,
}

/// The states of `ekr.integrate.IntegrationBlocker`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `IntegrationBlocker<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationBlockerState {
    /// `Recorded`.
    Recorded,
}

/// IntegrationBlockerId — `ekr.integrate.IntegrationBlockerId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationBlockerId(pub crate::primitives::Uuid);

/// IntegrationBlockerKind — `ekr.integrate.IntegrationBlockerKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationBlockerKind {
    /// `UnknownType`.
    UnknownType,
    /// `UnknownProperty`.
    UnknownProperty,
    /// `UnknownRelation`.
    UnknownRelation,
    /// `UnresolvedReference`.
    UnresolvedReference,
    /// `ValueMismatch`.
    ValueMismatch,
    /// `Contradiction`.
    Contradiction,
    /// `RejectedInterpretation`.
    RejectedInterpretation,
}

/// IntegrationBlockerSnapshot — `ekr.integrate.IntegrationBlockerSnapshot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationBlockerSnapshot {
    /// `blocker_id` — `ekr.integrate.IntegrationBlockerId`.
    pub blocker_id: IntegrationBlockerId,
    /// `document_digest` — `ekr.kernel.ContentHash`.
    pub document_digest: crate::kernel::ContentHash,
    /// `item` — `String`.
    pub item: String,
    /// `kind` — `ekr.integrate.IntegrationBlockerKind`.
    pub kind: IntegrationBlockerKind,
    /// `declaration` — `String`.
    pub declaration: String,
    /// `reason` — `String`.
    pub reason: String,
    /// `basis_digest` — `ekr.kernel.ContentHash`.
    pub basis_digest: crate::kernel::ContentHash,
}

/// IntegrationItemReceipt — `ekr.integrate.IntegrationItemReceipt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationItemReceipt {
    /// `source` — `ekr.integrate.InterpretationVersion`.
    pub source: InterpretationVersion,
    /// `item` — `String`.
    pub item: String,
    /// `mapping_digest` — `ekr.kernel.ContentHash`.
    pub mapping_digest: crate::kernel::ContentHash,
    /// `disposition` — `ekr.integrate.ProcessingDisposition`.
    pub disposition: ProcessingDisposition,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<crate::kernel::TransactionId>,
    /// `assertions` — `List<ekr.graph.AssertionId>`.
    pub assertions: Vec<crate::graph::AssertionId>,
    /// `blockers` — `List<ekr.integrate.IntegrationBlockerId>`.
    pub blockers: Vec<IntegrationBlockerId>,
    /// `reason` — `Optional<String>`.
    pub reason: Option<String>,
}

/// The states of `ekr.integrate.Interpretation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Interpretation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpretationState {
    /// `Recorded`.
    Recorded,
}

/// InterpretationCoordinate — `ekr.integrate.InterpretationCoordinate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretationCoordinate {
    /// `interpretation_id` — `ekr.integrate.InterpretationId`.
    pub interpretation_id: InterpretationId,
    /// `version` — `Integer`.
    pub version: i64,
}

/// InterpretationDocument — `ekr.integrate.InterpretationDocument`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretationDocument {
    /// `version` — `ekr.integrate.InterpretationCoordinate`.
    pub version: InterpretationCoordinate,
    /// `root_id` — `ekr.graph.GraphRootId`.
    pub root_id: crate::graph::GraphRootId,
    /// `observations` — `List<ekr.graph.ObservationId>`.
    pub observations: Vec<crate::graph::ObservationId>,
    /// `local_schema` — `ekr.integrate.OntologySpec`.
    pub local_schema: OntologySpec,
    /// `entities` — `List<ekr.integrate.ExtractedReference>`.
    pub entities: Vec<ExtractedReference>,
    /// `facts` — `List<ekr.integrate.ExtractedFact>`.
    pub facts: Vec<ExtractedFact>,
    /// `evidence` — `List<ekr.kernel.EvidenceAdditionProjection>`.
    pub evidence: Vec<crate::kernel::EvidenceAdditionProjection>,
}

/// InterpretationId — `ekr.integrate.InterpretationId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretationId(pub crate::primitives::Uuid);

/// InterpretationImport — `ekr.integrate.InterpretationImport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretationImport {
    /// `document` — `ekr.integrate.InterpretationDocument`.
    pub document: InterpretationDocument,
    /// `payload` — `Bytes`.
    pub payload: Vec<u8>,
}

/// The states of `ekr.integrate.InterpretationObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `InterpretationObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpretationObservationState {
    /// `Recorded`.
    Recorded,
}

/// InterpretationRead — `ekr.integrate.InterpretationRead`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretationRead {
    /// `document` — `ekr.integrate.InterpretationDocument`.
    pub document: InterpretationDocument,
    /// `blockers` — `List<ekr.integrate.IntegrationBlockerSnapshot>`.
    pub blockers: Vec<IntegrationBlockerSnapshot>,
    /// `receipts` — `List<ekr.integrate.ProcessingReceiptSnapshot>`.
    pub receipts: Vec<ProcessingReceiptSnapshot>,
}

/// InterpretationVersion — `ekr.integrate.InterpretationVersion`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretationVersion {
    /// `interpretation_id` — `ekr.integrate.InterpretationId`.
    pub interpretation_id: InterpretationId,
    /// `version` — `Integer`.
    pub version: i64,
    /// `document_digest` — `ekr.kernel.ContentHash`.
    pub document_digest: crate::kernel::ContentHash,
}

/// KnowledgeMapping — `ekr.integrate.KnowledgeMapping`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeMapping {
    /// `source_item` — `String`.
    pub source_item: String,
    /// `source_type` — `String`.
    pub source_type: String,
    /// `target_type` — `String`.
    pub target_type: String,
    /// `target_member` — `String`.
    pub target_member: String,
    /// `value` — `ekr.integrate.MappingValue`.
    pub value: MappingValue,
}

/// MappingPreview — `ekr.integrate.MappingPreview`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingPreview {
    /// `source` — `ekr.integrate.InterpretationVersion`.
    pub source: InterpretationVersion,
    /// `item` — `String`.
    pub item: String,
    /// `mapping` — `ekr.integrate.KnowledgeMapping`.
    pub mapping: KnowledgeMapping,
    /// `blockers` — `List<String>`.
    pub blockers: Vec<String>,
    /// `corrections` — `List<ekr.kernel.ClaimCorrection>`.
    pub corrections: Vec<crate::kernel::ClaimCorrection>,
}

/// The states of `ekr.integrate.MappingRecord`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `MappingRecord<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingRecordState {
    /// `Recorded`.
    Recorded,
}

/// MappingRecordId — `ekr.integrate.MappingRecordId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingRecordId(pub crate::primitives::Uuid);

/// MappingValue — `ekr.integrate.MappingValue`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingValue {
    /// Tagged `Constant` — `ekr.graph.TypedValue`.
    Constant(crate::graph::TypedValue),
    /// Tagged `CopyField` — `ekr.integrate.DeclaredSourceField`.
    CopyField(DeclaredSourceField),
    /// Tagged `CopyRelation` — `ekr.integrate.DeclaredSourceRelation`.
    CopyRelation(DeclaredSourceRelation),
}

/// The states of `ekr.integrate.Merge`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Merge<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeState {
    /// `Recorded`.
    Recorded,
}

/// MergeId — `ekr.integrate.MergeId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeId(pub crate::primitives::Uuid);

/// NodeTypeSpec — `ekr.integrate.NodeTypeSpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeTypeSpec {
    /// `name` — `String`.
    pub name: String,
    /// `parents` — `List<String>`.
    pub parents: Vec<String>,
    /// `abstract_type` — `Boolean`.
    pub abstract_type: bool,
    /// `properties` — `List<ekr.integrate.PropertySpec>`.
    pub properties: Vec<PropertySpec>,
}

/// OntologySpec — `ekr.integrate.OntologySpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologySpec {
    /// `node_types` — `List<ekr.integrate.NodeTypeSpec>`.
    pub node_types: Vec<NodeTypeSpec>,
    /// `edge_types` — `List<ekr.integrate.EdgeTypeSpec>`.
    pub edge_types: Vec<EdgeTypeSpec>,
}

/// OptionalPropertyAddition — `ekr.integrate.OptionalPropertyAddition`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalPropertyAddition {
    /// `owner_type` — `String`.
    pub owner_type: String,
    /// `property` — `ekr.integrate.PropertySpec`.
    pub property: PropertySpec,
}

/// ProcessingDisposition — `ekr.integrate.ProcessingDisposition`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingDisposition {
    /// `Parked`.
    Parked,
    /// `Integrated`.
    Integrated,
    /// `Rejected`.
    Rejected,
    /// `AlreadyIntegrated`.
    AlreadyIntegrated,
}

/// The states of `ekr.integrate.ProcessingReceipt`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ProcessingReceipt<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingReceiptState {
    /// `Recorded`.
    Recorded,
}

/// ProcessingReceiptId — `ekr.integrate.ProcessingReceiptId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingReceiptId(pub crate::primitives::Uuid);

/// ProcessingReceiptSnapshot — `ekr.integrate.ProcessingReceiptSnapshot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingReceiptSnapshot {
    /// `receipt_id` — `ekr.integrate.ProcessingReceiptId`.
    pub receipt_id: ProcessingReceiptId,
    /// `mapping_digest` — `Optional<ekr.kernel.ContentHash>`.
    pub mapping_digest: Option<crate::kernel::ContentHash>,
    /// `document_digest` — `ekr.kernel.ContentHash`.
    pub document_digest: crate::kernel::ContentHash,
    /// `item` — `String`.
    pub item: String,
    /// `disposition` — `ekr.integrate.ProcessingDisposition`.
    pub disposition: ProcessingDisposition,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<crate::kernel::TransactionId>,
    /// `assertions` — `List<ekr.graph.AssertionId>`.
    pub assertions: Vec<crate::graph::AssertionId>,
    /// `basis_digest` — `ekr.kernel.ContentHash`.
    pub basis_digest: crate::kernel::ContentHash,
}

/// PropertyFact — `ekr.integrate.PropertyFact`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyFact {
    /// `subject` — `ekr.integrate.ExtractedReference`.
    pub subject: ExtractedReference,
    /// `property` — `String`.
    pub property: String,
    /// `value` — `ekr.graph.TypedValue`.
    pub value: crate::graph::TypedValue,
    /// `evidence` — `List<ekr.graph.EvidenceId>`.
    pub evidence: Vec<crate::graph::EvidenceId>,
}

/// PropertySpec — `ekr.integrate.PropertySpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertySpec {
    /// `name` — `String`.
    pub name: String,
    /// `value` — `ekr.integrate.ValueSpec`.
    pub value: std::boxed::Box<ValueSpec>,
    /// `cardinality` — `ekr.ontology.Cardinality`.
    pub cardinality: crate::ontology::Cardinality,
    /// `required` — `Boolean`.
    pub required: bool,
}

/// The states of `ekr.integrate.ProposalEvidence`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ProposalEvidence<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalEvidenceState {
    /// `Recorded`.
    Recorded,
}

/// The states of `ekr.integrate.ProposalObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ProposalObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalObservationState {
    /// `Recorded`.
    Recorded,
}

/// The states of `ekr.integrate.ProposalReview`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ProposalReview<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalReviewState {
    /// `Recorded`.
    Recorded,
}

/// ProposalReviewId — `ekr.integrate.ProposalReviewId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalReviewId(pub crate::primitives::Uuid);

/// ProposalReviewSnapshot — `ekr.integrate.ProposalReviewSnapshot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalReviewSnapshot {
    /// `human_proof_digest` — `ekr.kernel.ContentHash`.
    pub human_proof_digest: crate::kernel::ContentHash,
    /// `review_id` — `ekr.integrate.ProposalReviewId`.
    pub review_id: ProposalReviewId,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `basis` — `ekr.kernel.ReviewBasis`.
    pub basis: crate::kernel::ReviewBasis,
    /// `decision` — `ekr.integrate.ReviewDecision`.
    pub decision: ReviewDecision,
    /// `operator` — `ekr.kernel.TrustedOperatorIdentity`.
    pub operator: crate::kernel::TrustedOperatorIdentity,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    pub evidence_id: crate::graph::EvidenceId,
    /// `recorded_at` — `Timestamp`.
    pub recorded_at: crate::primitives::Timestamp,
}

/// ProposalSource — `ekr.integrate.ProposalSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalSource {
    /// `version` — `ekr.integrate.InterpretationVersion`.
    pub version: InterpretationVersion,
    /// `items` — `List<String>`.
    pub items: Vec<String>,
}

/// The states of `ekr.integrate.ProposalSourceBinding`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ProposalSourceBinding<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalSourceBindingState {
    /// `Recorded`.
    Recorded,
}

/// RejectedExtraction — `ekr.integrate.RejectedExtraction`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedExtraction {
    /// `item` — `String`.
    pub item: String,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<crate::kernel::TransactionId>,
    /// `issues` — `List<ekr.integrate.ExtractionIssue>`.
    pub issues: Vec<ExtractionIssue>,
    /// `refusal` — `Optional<String>`.
    pub refusal: Option<String>,
}

/// RelationFact — `ekr.integrate.RelationFact`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationFact {
    /// `subject` — `ekr.integrate.ExtractedReference`.
    pub subject: ExtractedReference,
    /// `relation` — `String`.
    pub relation: String,
    /// `object` — `ekr.integrate.ExtractedReference`.
    pub object: ExtractedReference,
    /// `evidence` — `List<ekr.graph.EvidenceId>`.
    pub evidence: Vec<crate::graph::EvidenceId>,
}

/// ResolutionOutcome — `ekr.integrate.ResolutionOutcome`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionOutcome {
    /// Tagged `Ambiguous` — `ekr.integrate.AmbiguousReference`.
    Ambiguous(AmbiguousReference),
    /// Tagged `ProposeNew` — `ekr.integrate.TypedReference`.
    ProposeNew(TypedReference),
    /// Tagged `Refused` — `ekr.integrate.ResolutionRefusal`.
    Refused(ResolutionRefusal),
    /// Tagged `Resolved` — `ekr.integrate.ResolvedReference`.
    Resolved(ResolvedReference),
}

/// ResolutionRefusal — `ekr.integrate.ResolutionRefusal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionRefusal {
    /// `code` — `ekr.integrate.ResolutionRefusalCode`.
    pub code: ResolutionRefusalCode,
    /// `reference` — `ekr.integrate.TypedReference`.
    pub reference: TypedReference,
}

/// ResolutionRefusalCode — `ekr.integrate.ResolutionRefusalCode`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionRefusalCode {
    /// `reference-without-identity`.
    ReferenceWithoutIdentity,
    /// `reference-type-has-subtypes`.
    ReferenceTypeHasSubtypes,
    /// `reference-type-undeclared`.
    ReferenceTypeUndeclared,
}

/// ResolvedReference — `ekr.integrate.ResolvedReference`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReference {
    /// `node_id` — `ekr.graph.NodeId`.
    pub node_id: crate::graph::NodeId,
}

/// RetainedInterpretation — `ekr.integrate.RetainedInterpretation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedInterpretation {
    /// `version` — `ekr.integrate.InterpretationVersion`.
    pub version: InterpretationVersion,
    /// `interpretation` — `ekr.integrate.InterpretationRead`.
    pub interpretation: InterpretationRead,
    /// `payload` — `Bytes`.
    pub payload: Vec<u8>,
    /// `root` — `ekr.graph.GraphRootRecord`.
    pub root: crate::graph::GraphRootRecord,
}

/// ReviewDecision — `ekr.integrate.ReviewDecision`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewDecision {
    /// `Approved`.
    Approved,
    /// `Rejected`.
    Rejected,
}

/// SchemaLearningRequest — `ekr.integrate.SchemaLearningRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaLearningRequest {
    /// `base_schema` — `ekr.ontology.SchemaVersionId`.
    pub base_schema: crate::ontology::SchemaVersionId,
    /// `groups` — `List<ekr.integrate.GapGroup>`.
    pub groups: Vec<GapGroup>,
    /// `evidence` — `List<ekr.graph.EvidenceId>`.
    pub evidence: Vec<crate::graph::EvidenceId>,
}

/// SchemaLearningResult — `ekr.integrate.SchemaLearningResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaLearningResult {
    /// `proposals` — `List<ekr.integrate.SchemaProposalDocument>`.
    pub proposals: Vec<SchemaProposalDocument>,
    /// `unresolved_groups` — `List<ekr.integrate.GapGroupId>`.
    pub unresolved_groups: Vec<GapGroupId>,
}

/// The states of `ekr.integrate.SchemaProposal`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `SchemaProposal<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaProposalState {
    /// `Recorded`.
    Recorded,
}

/// SchemaProposalDocument — `ekr.integrate.SchemaProposalDocument`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProposalDocument {
    /// `observations` — `List<ekr.graph.ObservationId>`.
    pub observations: Vec<crate::graph::ObservationId>,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `base_schema` — `ekr.ontology.SchemaVersionId`.
    pub base_schema: crate::ontology::SchemaVersionId,
    /// `sources` — `List<ekr.integrate.ProposalSource>`.
    pub sources: Vec<ProposalSource>,
    /// `evidence` — `List<ekr.graph.EvidenceId>`.
    pub evidence: Vec<crate::graph::EvidenceId>,
    /// `additions` — `List<ekr.integrate.AdditiveSchemaOperation>`.
    pub additions: Vec<AdditiveSchemaOperation>,
    /// `mappings` — `List<ekr.integrate.KnowledgeMapping>`.
    pub mappings: Vec<KnowledgeMapping>,
    /// `corrections` — `List<ekr.kernel.ClaimCorrection>`.
    pub corrections: Vec<crate::kernel::ClaimCorrection>,
    /// `explanation` — `String`.
    pub explanation: String,
}

/// SchemaProposalId — `ekr.integrate.SchemaProposalId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProposalId(pub crate::primitives::Uuid);

/// The states of `ekr.integrate.Split`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Split<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitState {
    /// `Recorded`.
    Recorded,
}

/// SplitId — `ekr.integrate.SplitId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitId(pub crate::primitives::Uuid);

/// TypedReference — `ekr.integrate.TypedReference`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedReference {
    /// `type_id` — `ekr.ontology.TypeId`.
    pub type_id: crate::ontology::TypeId,
    /// `aliases` — `List<String>`.
    pub aliases: Vec<String>,
}

/// ValueSpec — `ekr.integrate.ValueSpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueSpec {
    /// `value_kind` — `ekr.ontology.ValueKind`.
    pub value_kind: crate::ontology::ValueKind,
    /// `allowed_types` — `Optional<List<String>>`.
    pub allowed_types: Option<Vec<String>>,
    /// `variants` — `Optional<List<String>>`.
    pub variants: Option<Vec<String>>,
    /// `element` — `Optional<ekr.integrate.ValueSpec>`.
    pub element: Option<std::boxed::Box<ValueSpec>>,
    /// `fields` — `Optional<Map<String, ekr.integrate.ValueSpec>>`.
    pub fields: Option<std::collections::BTreeMap<String, std::boxed::Box<ValueSpec>>>,
}

/// What ApplicationReceipt — `ekr.integrate.ApplicationReceipt` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ApplicationReceipt<S>`], and at a boundary by [`ApplicationReceiptEntitySnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationReceiptData {
    /// The identity: `receipt_id` — `ekr.integrate.ApplicationReceiptId`.
    pub receipt_id: ApplicationReceiptId,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    ///
    /// Carries `proposal`: `ekr.integrate.ApplicationReceipt` references one `ekr.integrate.SchemaProposal`.
    pub proposal_id: SchemaProposalId,
    /// `review_id` — `ekr.integrate.ProposalReviewId`.
    ///
    /// Carries `review`: `ekr.integrate.ApplicationReceipt` references one `ekr.integrate.ProposalReview`.
    pub review_id: ProposalReviewId,
    /// `progress` — `ekr.integrate.ApplicationProgress`.
    pub progress: ApplicationProgress,
    /// `schema_transaction` — `ekr.kernel.TransactionId`.
    ///
    /// Carries `schema_transaction`: `ekr.integrate.ApplicationReceipt` references one `ekr.kernel.GraphTransaction`.
    pub schema_transaction: crate::kernel::TransactionId,
    /// `schema_revision` — `ekr.kernel.RevisionNumber`.
    pub schema_revision: crate::kernel::RevisionNumber,
    /// `processing_receipts` — `List<ekr.integrate.ProcessingReceiptId>`.
    pub processing_receipts: Vec<ProcessingReceiptId>,
    /// `remaining_items` — `List<String>`.
    pub remaining_items: Vec<String>,
    /// `stop_reason` — `Optional<String>`.
    pub stop_reason: Option<String>,
}

/// The states of `ekr.integrate.ApplicationReceipt`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](application_receipt_state::Marker), so [`ApplicationReceipt<S>`](ApplicationReceipt) can only ever rest in a real state.
pub mod application_receipt_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `ApplicationReceipt`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ApplicationReceiptState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::ApplicationReceiptState = super::ApplicationReceiptState::Recorded;
    }
}

/// ApplicationReceipt — `ekr.integrate.ApplicationReceipt` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ApplicationReceiptEntitySnapshot`]
/// and [`ApplicationReceiptEntitySnapshot::refine`].
pub struct ApplicationReceipt<S: application_receipt_state::Marker> {
    data: ApplicationReceiptData,
    state: core::marker::PhantomData<S>,
}

impl<S: application_receipt_state::Marker> ApplicationReceipt<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ApplicationReceiptState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ApplicationReceiptData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ApplicationReceiptData {
        self.data
    }
}

impl ApplicationReceipt<application_receipt_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: ApplicationReceiptData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.ApplicationReceipt` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ApplicationReceiptEntitySnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationReceiptEntitySnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ApplicationReceiptState,
    /// What it holds.
    pub data: ApplicationReceiptData,
}

/// An `ApplicationReceipt` in whichever declared state it was found.
pub enum AnyApplicationReceipt {
    /// Resting in `Recorded`.
    Recorded(ApplicationReceipt<application_receipt_state::Recorded>),
}

impl ApplicationReceiptEntitySnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ApplicationReceiptState` cannot spell one.
    pub fn refine(self) -> AnyApplicationReceipt {
        match self.state {
            ApplicationReceiptState::Recorded => AnyApplicationReceipt::Recorded(ApplicationReceipt {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyApplicationReceipt {
    /// The state, as the runtime value.
    pub fn state(&self) -> ApplicationReceiptState {
        match self {
            Self::Recorded(_) => ApplicationReceiptState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ApplicationReceiptEntitySnapshot {
        match self {
            Self::Recorded(instance) => ApplicationReceiptEntitySnapshot {
                state: ApplicationReceiptState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What CanonicalDerivation — `ekr.integrate.CanonicalDerivation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`CanonicalDerivation<S>`], and at a boundary by [`CanonicalDerivationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDerivationData {
    /// The identity: `derivation_id` — `Uuid`.
    pub derivation_id: crate::primitives::Uuid,
    /// `assertion_id` — `ekr.graph.AssertionId`.
    ///
    /// Carries `assertion`: `ekr.integrate.CanonicalDerivation` references one `ekr.graph.Assertion`.
    pub assertion_id: crate::graph::AssertionId,
    /// `mapping_id` — `ekr.integrate.MappingRecordId`.
    ///
    /// Carries `mapping`: `ekr.integrate.CanonicalDerivation` references one `ekr.integrate.MappingRecord`.
    pub mapping_id: MappingRecordId,
    /// `observation_id` — `ekr.graph.ObservationId`.
    ///
    /// Carries `observation`: `ekr.integrate.CanonicalDerivation` references one `ekr.observe.RetainedObservation`.
    pub observation_id: crate::graph::ObservationId,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    ///
    /// Carries `evidence`: `ekr.integrate.CanonicalDerivation` references one `ekr.graph.Evidence`.
    pub evidence_id: crate::graph::EvidenceId,
}

/// The states of `ekr.integrate.CanonicalDerivation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](canonical_derivation_state::Marker), so [`CanonicalDerivation<S>`](CanonicalDerivation) can only ever rest in a real state.
pub mod canonical_derivation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `CanonicalDerivation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::CanonicalDerivationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::CanonicalDerivationState = super::CanonicalDerivationState::Recorded;
    }
}

/// CanonicalDerivation — `ekr.integrate.CanonicalDerivation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`CanonicalDerivationSnapshot`]
/// and [`CanonicalDerivationSnapshot::refine`].
pub struct CanonicalDerivation<S: canonical_derivation_state::Marker> {
    data: CanonicalDerivationData,
    state: core::marker::PhantomData<S>,
}

impl<S: canonical_derivation_state::Marker> CanonicalDerivation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> CanonicalDerivationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &CanonicalDerivationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> CanonicalDerivationData {
        self.data
    }
}

impl CanonicalDerivation<canonical_derivation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: CanonicalDerivationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.CanonicalDerivation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`CanonicalDerivationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDerivationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: CanonicalDerivationState,
    /// What it holds.
    pub data: CanonicalDerivationData,
}

/// An `CanonicalDerivation` in whichever declared state it was found.
pub enum AnyCanonicalDerivation {
    /// Resting in `Recorded`.
    Recorded(CanonicalDerivation<canonical_derivation_state::Recorded>),
}

impl CanonicalDerivationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `CanonicalDerivationState` cannot spell one.
    pub fn refine(self) -> AnyCanonicalDerivation {
        match self.state {
            CanonicalDerivationState::Recorded => AnyCanonicalDerivation::Recorded(CanonicalDerivation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyCanonicalDerivation {
    /// The state, as the runtime value.
    pub fn state(&self) -> CanonicalDerivationState {
        match self {
            Self::Recorded(_) => CanonicalDerivationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> CanonicalDerivationSnapshot {
        match self {
            Self::Recorded(instance) => CanonicalDerivationSnapshot {
                state: CanonicalDerivationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What IntegrationBlocker — `ekr.integrate.IntegrationBlocker` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`IntegrationBlocker<S>`], and at a boundary by [`IntegrationBlockerEntitySnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationBlockerData {
    /// The identity: `blocker_id` — `ekr.integrate.IntegrationBlockerId`.
    pub blocker_id: IntegrationBlockerId,
    /// `document_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `interpretation`: `ekr.integrate.IntegrationBlocker` references one `ekr.integrate.Interpretation`.
    pub document_digest: crate::kernel::ContentHash,
    /// `item` — `String`.
    pub item: String,
    /// `kind` — `ekr.integrate.IntegrationBlockerKind`.
    pub kind: IntegrationBlockerKind,
    /// `declaration` — `String`.
    pub declaration: String,
    /// `reason` — `String`.
    pub reason: String,
    /// `basis_digest` — `ekr.kernel.ContentHash`.
    pub basis_digest: crate::kernel::ContentHash,
}

/// The states of `ekr.integrate.IntegrationBlocker`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](integration_blocker_state::Marker), so [`IntegrationBlocker<S>`](IntegrationBlocker) can only ever rest in a real state.
pub mod integration_blocker_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `IntegrationBlocker`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::IntegrationBlockerState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::IntegrationBlockerState = super::IntegrationBlockerState::Recorded;
    }
}

/// IntegrationBlocker — `ekr.integrate.IntegrationBlocker` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`IntegrationBlockerEntitySnapshot`]
/// and [`IntegrationBlockerEntitySnapshot::refine`].
pub struct IntegrationBlocker<S: integration_blocker_state::Marker> {
    data: IntegrationBlockerData,
    state: core::marker::PhantomData<S>,
}

impl<S: integration_blocker_state::Marker> IntegrationBlocker<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> IntegrationBlockerState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &IntegrationBlockerData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> IntegrationBlockerData {
        self.data
    }
}

impl IntegrationBlocker<integration_blocker_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: IntegrationBlockerData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.IntegrationBlocker` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`IntegrationBlockerEntitySnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationBlockerEntitySnapshot {
    /// Where the instance is in its lifecycle.
    pub state: IntegrationBlockerState,
    /// What it holds.
    pub data: IntegrationBlockerData,
}

/// An `IntegrationBlocker` in whichever declared state it was found.
pub enum AnyIntegrationBlocker {
    /// Resting in `Recorded`.
    Recorded(IntegrationBlocker<integration_blocker_state::Recorded>),
}

impl IntegrationBlockerEntitySnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `IntegrationBlockerState` cannot spell one.
    pub fn refine(self) -> AnyIntegrationBlocker {
        match self.state {
            IntegrationBlockerState::Recorded => AnyIntegrationBlocker::Recorded(IntegrationBlocker {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyIntegrationBlocker {
    /// The state, as the runtime value.
    pub fn state(&self) -> IntegrationBlockerState {
        match self {
            Self::Recorded(_) => IntegrationBlockerState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> IntegrationBlockerEntitySnapshot {
        match self {
            Self::Recorded(instance) => IntegrationBlockerEntitySnapshot {
                state: IntegrationBlockerState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What Interpretation — `ekr.integrate.Interpretation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Interpretation<S>`], and at a boundary by [`InterpretationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretationData {
    /// The identity: `document_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `bytes`: `ekr.integrate.Interpretation` references one `ekr.store.StoredObject`.
    pub document_digest: crate::kernel::ContentHash,
    /// `interpretation_id` — `ekr.integrate.InterpretationId`.
    pub interpretation_id: InterpretationId,
    /// `version` — `Integer`.
    pub version: i64,
    /// `root_id` — `ekr.graph.GraphRootId`.
    ///
    /// Carries `root`: `ekr.integrate.Interpretation` references one `ekr.graph.GraphRoot`.
    pub root_id: crate::graph::GraphRootId,
    /// `document` — `ekr.integrate.InterpretationDocument`.
    pub document: InterpretationDocument,
}

/// The states of `ekr.integrate.Interpretation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](interpretation_state::Marker), so [`Interpretation<S>`](Interpretation) can only ever rest in a real state.
pub mod interpretation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `Interpretation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::InterpretationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::InterpretationState = super::InterpretationState::Recorded;
    }
}

/// Interpretation — `ekr.integrate.Interpretation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`InterpretationSnapshot`]
/// and [`InterpretationSnapshot::refine`].
pub struct Interpretation<S: interpretation_state::Marker> {
    data: InterpretationData,
    state: core::marker::PhantomData<S>,
}

impl<S: interpretation_state::Marker> Interpretation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> InterpretationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &InterpretationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> InterpretationData {
        self.data
    }
}

impl Interpretation<interpretation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: InterpretationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.Interpretation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`InterpretationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: InterpretationState,
    /// What it holds.
    pub data: InterpretationData,
}

/// An `Interpretation` in whichever declared state it was found.
pub enum AnyInterpretation {
    /// Resting in `Recorded`.
    Recorded(Interpretation<interpretation_state::Recorded>),
}

impl InterpretationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `InterpretationState` cannot spell one.
    pub fn refine(self) -> AnyInterpretation {
        match self.state {
            InterpretationState::Recorded => AnyInterpretation::Recorded(Interpretation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyInterpretation {
    /// The state, as the runtime value.
    pub fn state(&self) -> InterpretationState {
        match self {
            Self::Recorded(_) => InterpretationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> InterpretationSnapshot {
        match self {
            Self::Recorded(instance) => InterpretationSnapshot {
                state: InterpretationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What InterpretationObservation — `ekr.integrate.InterpretationObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`InterpretationObservation<S>`], and at a boundary by [`InterpretationObservationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretationObservationData {
    /// The identity: `link_id` — `Uuid`.
    pub link_id: crate::primitives::Uuid,
    /// `document_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `interpretation`: `ekr.integrate.InterpretationObservation` references one `ekr.integrate.Interpretation`.
    pub document_digest: crate::kernel::ContentHash,
    /// `observation_id` — `ekr.graph.ObservationId`.
    ///
    /// Carries `observation`: `ekr.integrate.InterpretationObservation` references one `ekr.observe.RetainedObservation`.
    pub observation_id: crate::graph::ObservationId,
}

/// The states of `ekr.integrate.InterpretationObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](interpretation_observation_state::Marker), so [`InterpretationObservation<S>`](InterpretationObservation) can only ever rest in a real state.
pub mod interpretation_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `InterpretationObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::InterpretationObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::InterpretationObservationState = super::InterpretationObservationState::Recorded;
    }
}

/// InterpretationObservation — `ekr.integrate.InterpretationObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`InterpretationObservationSnapshot`]
/// and [`InterpretationObservationSnapshot::refine`].
pub struct InterpretationObservation<S: interpretation_observation_state::Marker> {
    data: InterpretationObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: interpretation_observation_state::Marker> InterpretationObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> InterpretationObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &InterpretationObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> InterpretationObservationData {
        self.data
    }
}

impl InterpretationObservation<interpretation_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: InterpretationObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.InterpretationObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`InterpretationObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretationObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: InterpretationObservationState,
    /// What it holds.
    pub data: InterpretationObservationData,
}

/// An `InterpretationObservation` in whichever declared state it was found.
pub enum AnyInterpretationObservation {
    /// Resting in `Recorded`.
    Recorded(InterpretationObservation<interpretation_observation_state::Recorded>),
}

impl InterpretationObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `InterpretationObservationState` cannot spell one.
    pub fn refine(self) -> AnyInterpretationObservation {
        match self.state {
            InterpretationObservationState::Recorded => AnyInterpretationObservation::Recorded(InterpretationObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyInterpretationObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> InterpretationObservationState {
        match self {
            Self::Recorded(_) => InterpretationObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> InterpretationObservationSnapshot {
        match self {
            Self::Recorded(instance) => InterpretationObservationSnapshot {
                state: InterpretationObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What MappingRecord — `ekr.integrate.MappingRecord` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`MappingRecord<S>`], and at a boundary by [`MappingRecordSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingRecordData {
    /// The identity: `mapping_id` — `ekr.integrate.MappingRecordId`.
    pub mapping_id: MappingRecordId,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `proposal_bytes`: `ekr.integrate.MappingRecord` references one `ekr.store.StoredObject`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `mapping_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `bytes`: `ekr.integrate.MappingRecord` references one `ekr.store.StoredObject`.
    pub mapping_digest: crate::kernel::ContentHash,
    /// `source_document_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `source_bytes`: `ekr.integrate.MappingRecord` references one `ekr.store.StoredObject`.
    pub source_document_digest: crate::kernel::ContentHash,
    /// `mapping` — `ekr.integrate.KnowledgeMapping`.
    pub mapping: KnowledgeMapping,
    /// `evidence` — `List<ekr.graph.EvidenceId>`.
    pub evidence: Vec<crate::graph::EvidenceId>,
}

/// The states of `ekr.integrate.MappingRecord`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](mapping_record_state::Marker), so [`MappingRecord<S>`](MappingRecord) can only ever rest in a real state.
pub mod mapping_record_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `MappingRecord`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::MappingRecordState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::MappingRecordState = super::MappingRecordState::Recorded;
    }
}

/// MappingRecord — `ekr.integrate.MappingRecord` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`MappingRecordSnapshot`]
/// and [`MappingRecordSnapshot::refine`].
pub struct MappingRecord<S: mapping_record_state::Marker> {
    data: MappingRecordData,
    state: core::marker::PhantomData<S>,
}

impl<S: mapping_record_state::Marker> MappingRecord<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> MappingRecordState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &MappingRecordData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> MappingRecordData {
        self.data
    }
}

impl MappingRecord<mapping_record_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: MappingRecordData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.MappingRecord` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`MappingRecordSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingRecordSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: MappingRecordState,
    /// What it holds.
    pub data: MappingRecordData,
}

/// An `MappingRecord` in whichever declared state it was found.
pub enum AnyMappingRecord {
    /// Resting in `Recorded`.
    Recorded(MappingRecord<mapping_record_state::Recorded>),
}

impl MappingRecordSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `MappingRecordState` cannot spell one.
    pub fn refine(self) -> AnyMappingRecord {
        match self.state {
            MappingRecordState::Recorded => AnyMappingRecord::Recorded(MappingRecord {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyMappingRecord {
    /// The state, as the runtime value.
    pub fn state(&self) -> MappingRecordState {
        match self {
            Self::Recorded(_) => MappingRecordState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> MappingRecordSnapshot {
        match self {
            Self::Recorded(instance) => MappingRecordSnapshot {
                state: MappingRecordState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What Merge — `ekr.integrate.Merge` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Merge<S>`], and at a boundary by [`MergeSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeData {
    /// The identity: `merge_id` — `ekr.integrate.MergeId`.
    pub merge_id: MergeId,
}

/// The states of `ekr.integrate.Merge`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](merge_state::Marker), so [`Merge<S>`](Merge) can only ever rest in a real state.
pub mod merge_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `Merge`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::MergeState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::MergeState = super::MergeState::Recorded;
    }
}

/// Merge — `ekr.integrate.Merge` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`MergeSnapshot`]
/// and [`MergeSnapshot::refine`].
pub struct Merge<S: merge_state::Marker> {
    data: MergeData,
    state: core::marker::PhantomData<S>,
}

impl<S: merge_state::Marker> Merge<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> MergeState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &MergeData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> MergeData {
        self.data
    }
}

impl Merge<merge_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: MergeData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.Merge` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`MergeSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: MergeState,
    /// What it holds.
    pub data: MergeData,
}

/// An `Merge` in whichever declared state it was found.
pub enum AnyMerge {
    /// Resting in `Recorded`.
    Recorded(Merge<merge_state::Recorded>),
}

impl MergeSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `MergeState` cannot spell one.
    pub fn refine(self) -> AnyMerge {
        match self.state {
            MergeState::Recorded => AnyMerge::Recorded(Merge {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyMerge {
    /// The state, as the runtime value.
    pub fn state(&self) -> MergeState {
        match self {
            Self::Recorded(_) => MergeState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> MergeSnapshot {
        match self {
            Self::Recorded(instance) => MergeSnapshot {
                state: MergeState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What ProcessingReceipt — `ekr.integrate.ProcessingReceipt` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ProcessingReceipt<S>`], and at a boundary by [`ProcessingReceiptEntitySnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingReceiptData {
    /// The identity: `receipt_id` — `ekr.integrate.ProcessingReceiptId`.
    pub receipt_id: ProcessingReceiptId,
    /// `mapping_digest` — `Optional<ekr.kernel.ContentHash>`.
    pub mapping_digest: Option<crate::kernel::ContentHash>,
    /// `document_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `interpretation`: `ekr.integrate.ProcessingReceipt` references one `ekr.integrate.Interpretation`.
    pub document_digest: crate::kernel::ContentHash,
    /// `item` — `String`.
    pub item: String,
    /// `disposition` — `ekr.integrate.ProcessingDisposition`.
    pub disposition: ProcessingDisposition,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<crate::kernel::TransactionId>,
    /// `assertions` — `List<ekr.graph.AssertionId>`.
    pub assertions: Vec<crate::graph::AssertionId>,
    /// `basis_digest` — `ekr.kernel.ContentHash`.
    pub basis_digest: crate::kernel::ContentHash,
}

/// The states of `ekr.integrate.ProcessingReceipt`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](processing_receipt_state::Marker), so [`ProcessingReceipt<S>`](ProcessingReceipt) can only ever rest in a real state.
pub mod processing_receipt_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `ProcessingReceipt`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ProcessingReceiptState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::ProcessingReceiptState = super::ProcessingReceiptState::Recorded;
    }
}

/// ProcessingReceipt — `ekr.integrate.ProcessingReceipt` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ProcessingReceiptEntitySnapshot`]
/// and [`ProcessingReceiptEntitySnapshot::refine`].
pub struct ProcessingReceipt<S: processing_receipt_state::Marker> {
    data: ProcessingReceiptData,
    state: core::marker::PhantomData<S>,
}

impl<S: processing_receipt_state::Marker> ProcessingReceipt<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ProcessingReceiptState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ProcessingReceiptData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ProcessingReceiptData {
        self.data
    }
}

impl ProcessingReceipt<processing_receipt_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: ProcessingReceiptData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.ProcessingReceipt` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ProcessingReceiptEntitySnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingReceiptEntitySnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ProcessingReceiptState,
    /// What it holds.
    pub data: ProcessingReceiptData,
}

/// An `ProcessingReceipt` in whichever declared state it was found.
pub enum AnyProcessingReceipt {
    /// Resting in `Recorded`.
    Recorded(ProcessingReceipt<processing_receipt_state::Recorded>),
}

impl ProcessingReceiptEntitySnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ProcessingReceiptState` cannot spell one.
    pub fn refine(self) -> AnyProcessingReceipt {
        match self.state {
            ProcessingReceiptState::Recorded => AnyProcessingReceipt::Recorded(ProcessingReceipt {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyProcessingReceipt {
    /// The state, as the runtime value.
    pub fn state(&self) -> ProcessingReceiptState {
        match self {
            Self::Recorded(_) => ProcessingReceiptState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ProcessingReceiptEntitySnapshot {
        match self {
            Self::Recorded(instance) => ProcessingReceiptEntitySnapshot {
                state: ProcessingReceiptState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What ProposalEvidence — `ekr.integrate.ProposalEvidence` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ProposalEvidence<S>`], and at a boundary by [`ProposalEvidenceSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalEvidenceData {
    /// The identity: `binding_id` — `Uuid`.
    pub binding_id: crate::primitives::Uuid,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    ///
    /// Carries `proposal`: `ekr.integrate.ProposalEvidence` references one `ekr.integrate.SchemaProposal`.
    pub proposal_id: SchemaProposalId,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    ///
    /// Carries `evidence`: `ekr.integrate.ProposalEvidence` references one `ekr.graph.Evidence`.
    pub evidence_id: crate::graph::EvidenceId,
}

/// The states of `ekr.integrate.ProposalEvidence`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](proposal_evidence_state::Marker), so [`ProposalEvidence<S>`](ProposalEvidence) can only ever rest in a real state.
pub mod proposal_evidence_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `ProposalEvidence`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ProposalEvidenceState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::ProposalEvidenceState = super::ProposalEvidenceState::Recorded;
    }
}

/// ProposalEvidence — `ekr.integrate.ProposalEvidence` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ProposalEvidenceSnapshot`]
/// and [`ProposalEvidenceSnapshot::refine`].
pub struct ProposalEvidence<S: proposal_evidence_state::Marker> {
    data: ProposalEvidenceData,
    state: core::marker::PhantomData<S>,
}

impl<S: proposal_evidence_state::Marker> ProposalEvidence<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ProposalEvidenceState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ProposalEvidenceData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ProposalEvidenceData {
        self.data
    }
}

impl ProposalEvidence<proposal_evidence_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: ProposalEvidenceData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.ProposalEvidence` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ProposalEvidenceSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalEvidenceSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ProposalEvidenceState,
    /// What it holds.
    pub data: ProposalEvidenceData,
}

/// An `ProposalEvidence` in whichever declared state it was found.
pub enum AnyProposalEvidence {
    /// Resting in `Recorded`.
    Recorded(ProposalEvidence<proposal_evidence_state::Recorded>),
}

impl ProposalEvidenceSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ProposalEvidenceState` cannot spell one.
    pub fn refine(self) -> AnyProposalEvidence {
        match self.state {
            ProposalEvidenceState::Recorded => AnyProposalEvidence::Recorded(ProposalEvidence {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyProposalEvidence {
    /// The state, as the runtime value.
    pub fn state(&self) -> ProposalEvidenceState {
        match self {
            Self::Recorded(_) => ProposalEvidenceState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ProposalEvidenceSnapshot {
        match self {
            Self::Recorded(instance) => ProposalEvidenceSnapshot {
                state: ProposalEvidenceState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What ProposalObservation — `ekr.integrate.ProposalObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ProposalObservation<S>`], and at a boundary by [`ProposalObservationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalObservationData {
    /// The identity: `binding_id` — `Uuid`.
    pub binding_id: crate::primitives::Uuid,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    ///
    /// Carries `proposal`: `ekr.integrate.ProposalObservation` references one `ekr.integrate.SchemaProposal`.
    pub proposal_id: SchemaProposalId,
    /// `observation_id` — `ekr.graph.ObservationId`.
    ///
    /// Carries `observation`: `ekr.integrate.ProposalObservation` references one `ekr.observe.RetainedObservation`.
    pub observation_id: crate::graph::ObservationId,
}

/// The states of `ekr.integrate.ProposalObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](proposal_observation_state::Marker), so [`ProposalObservation<S>`](ProposalObservation) can only ever rest in a real state.
pub mod proposal_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `ProposalObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ProposalObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::ProposalObservationState = super::ProposalObservationState::Recorded;
    }
}

/// ProposalObservation — `ekr.integrate.ProposalObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ProposalObservationSnapshot`]
/// and [`ProposalObservationSnapshot::refine`].
pub struct ProposalObservation<S: proposal_observation_state::Marker> {
    data: ProposalObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: proposal_observation_state::Marker> ProposalObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ProposalObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ProposalObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ProposalObservationData {
        self.data
    }
}

impl ProposalObservation<proposal_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: ProposalObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.ProposalObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ProposalObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ProposalObservationState,
    /// What it holds.
    pub data: ProposalObservationData,
}

/// An `ProposalObservation` in whichever declared state it was found.
pub enum AnyProposalObservation {
    /// Resting in `Recorded`.
    Recorded(ProposalObservation<proposal_observation_state::Recorded>),
}

impl ProposalObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ProposalObservationState` cannot spell one.
    pub fn refine(self) -> AnyProposalObservation {
        match self.state {
            ProposalObservationState::Recorded => AnyProposalObservation::Recorded(ProposalObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyProposalObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> ProposalObservationState {
        match self {
            Self::Recorded(_) => ProposalObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ProposalObservationSnapshot {
        match self {
            Self::Recorded(instance) => ProposalObservationSnapshot {
                state: ProposalObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What ProposalReview — `ekr.integrate.ProposalReview` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ProposalReview<S>`], and at a boundary by [`ProposalReviewEntitySnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalReviewData {
    /// The identity: `review_id` — `ekr.integrate.ProposalReviewId`.
    pub review_id: ProposalReviewId,
    /// `human_proof_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `human_proof`: `ekr.integrate.ProposalReview` references one `ekr.kernel.RetainedHumanDecision`.
    pub human_proof_digest: crate::kernel::ContentHash,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    ///
    /// Carries `proposal`: `ekr.integrate.ProposalReview` references one `ekr.integrate.SchemaProposal`.
    pub proposal_id: SchemaProposalId,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `basis` — `ekr.kernel.ReviewBasis`.
    pub basis: crate::kernel::ReviewBasis,
    /// `decision` — `ekr.integrate.ReviewDecision`.
    pub decision: ReviewDecision,
    /// `operator` — `ekr.kernel.TrustedOperatorIdentity`.
    pub operator: crate::kernel::TrustedOperatorIdentity,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    ///
    /// Carries `human_evidence`: `ekr.integrate.ProposalReview` references one `ekr.graph.Evidence`.
    pub evidence_id: crate::graph::EvidenceId,
    /// `recorded_at` — `Timestamp`.
    pub recorded_at: crate::primitives::Timestamp,
}

/// The states of `ekr.integrate.ProposalReview`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](proposal_review_state::Marker), so [`ProposalReview<S>`](ProposalReview) can only ever rest in a real state.
pub mod proposal_review_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `ProposalReview`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ProposalReviewState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::ProposalReviewState = super::ProposalReviewState::Recorded;
    }
}

/// ProposalReview — `ekr.integrate.ProposalReview` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ProposalReviewEntitySnapshot`]
/// and [`ProposalReviewEntitySnapshot::refine`].
pub struct ProposalReview<S: proposal_review_state::Marker> {
    data: ProposalReviewData,
    state: core::marker::PhantomData<S>,
}

impl<S: proposal_review_state::Marker> ProposalReview<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ProposalReviewState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ProposalReviewData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ProposalReviewData {
        self.data
    }
}

impl ProposalReview<proposal_review_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: ProposalReviewData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.ProposalReview` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ProposalReviewEntitySnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalReviewEntitySnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ProposalReviewState,
    /// What it holds.
    pub data: ProposalReviewData,
}

/// An `ProposalReview` in whichever declared state it was found.
pub enum AnyProposalReview {
    /// Resting in `Recorded`.
    Recorded(ProposalReview<proposal_review_state::Recorded>),
}

impl ProposalReviewEntitySnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ProposalReviewState` cannot spell one.
    pub fn refine(self) -> AnyProposalReview {
        match self.state {
            ProposalReviewState::Recorded => AnyProposalReview::Recorded(ProposalReview {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyProposalReview {
    /// The state, as the runtime value.
    pub fn state(&self) -> ProposalReviewState {
        match self {
            Self::Recorded(_) => ProposalReviewState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ProposalReviewEntitySnapshot {
        match self {
            Self::Recorded(instance) => ProposalReviewEntitySnapshot {
                state: ProposalReviewState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What ProposalSourceBinding — `ekr.integrate.ProposalSourceBinding` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ProposalSourceBinding<S>`], and at a boundary by [`ProposalSourceBindingSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalSourceBindingData {
    /// The identity: `binding_id` — `Uuid`.
    pub binding_id: crate::primitives::Uuid,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    ///
    /// Carries `proposal`: `ekr.integrate.ProposalSourceBinding` references one `ekr.integrate.SchemaProposal`.
    pub proposal_id: SchemaProposalId,
    /// `document_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `interpretation`: `ekr.integrate.ProposalSourceBinding` references one `ekr.integrate.Interpretation`.
    pub document_digest: crate::kernel::ContentHash,
}

/// The states of `ekr.integrate.ProposalSourceBinding`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](proposal_source_binding_state::Marker), so [`ProposalSourceBinding<S>`](ProposalSourceBinding) can only ever rest in a real state.
pub mod proposal_source_binding_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `ProposalSourceBinding`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ProposalSourceBindingState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::ProposalSourceBindingState = super::ProposalSourceBindingState::Recorded;
    }
}

/// ProposalSourceBinding — `ekr.integrate.ProposalSourceBinding` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ProposalSourceBindingSnapshot`]
/// and [`ProposalSourceBindingSnapshot::refine`].
pub struct ProposalSourceBinding<S: proposal_source_binding_state::Marker> {
    data: ProposalSourceBindingData,
    state: core::marker::PhantomData<S>,
}

impl<S: proposal_source_binding_state::Marker> ProposalSourceBinding<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ProposalSourceBindingState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ProposalSourceBindingData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ProposalSourceBindingData {
        self.data
    }
}

impl ProposalSourceBinding<proposal_source_binding_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: ProposalSourceBindingData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.ProposalSourceBinding` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ProposalSourceBindingSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalSourceBindingSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ProposalSourceBindingState,
    /// What it holds.
    pub data: ProposalSourceBindingData,
}

/// An `ProposalSourceBinding` in whichever declared state it was found.
pub enum AnyProposalSourceBinding {
    /// Resting in `Recorded`.
    Recorded(ProposalSourceBinding<proposal_source_binding_state::Recorded>),
}

impl ProposalSourceBindingSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ProposalSourceBindingState` cannot spell one.
    pub fn refine(self) -> AnyProposalSourceBinding {
        match self.state {
            ProposalSourceBindingState::Recorded => AnyProposalSourceBinding::Recorded(ProposalSourceBinding {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyProposalSourceBinding {
    /// The state, as the runtime value.
    pub fn state(&self) -> ProposalSourceBindingState {
        match self {
            Self::Recorded(_) => ProposalSourceBindingState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ProposalSourceBindingSnapshot {
        match self {
            Self::Recorded(instance) => ProposalSourceBindingSnapshot {
                state: ProposalSourceBindingState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What SchemaProposal — `ekr.integrate.SchemaProposal` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`SchemaProposal<S>`], and at a boundary by [`SchemaProposalSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProposalData {
    /// The identity: `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    ///
    /// Carries `bytes`: `ekr.integrate.SchemaProposal` references one `ekr.store.StoredObject`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `base_schema` — `ekr.ontology.SchemaVersionId`.
    ///
    /// Carries `base_schema`: `ekr.integrate.SchemaProposal` references one `ekr.ontology.SchemaVersion`.
    pub base_schema: crate::ontology::SchemaVersionId,
    /// `document` — `ekr.integrate.SchemaProposalDocument`.
    pub document: SchemaProposalDocument,
}

/// The states of `ekr.integrate.SchemaProposal`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](schema_proposal_state::Marker), so [`SchemaProposal<S>`](SchemaProposal) can only ever rest in a real state.
pub mod schema_proposal_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `SchemaProposal`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SchemaProposalState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::SchemaProposalState = super::SchemaProposalState::Recorded;
    }
}

/// SchemaProposal — `ekr.integrate.SchemaProposal` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SchemaProposalSnapshot`]
/// and [`SchemaProposalSnapshot::refine`].
pub struct SchemaProposal<S: schema_proposal_state::Marker> {
    data: SchemaProposalData,
    state: core::marker::PhantomData<S>,
}

impl<S: schema_proposal_state::Marker> SchemaProposal<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SchemaProposalState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SchemaProposalData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SchemaProposalData {
        self.data
    }
}

impl SchemaProposal<schema_proposal_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: SchemaProposalData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.SchemaProposal` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SchemaProposalSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProposalSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SchemaProposalState,
    /// What it holds.
    pub data: SchemaProposalData,
}

/// An `SchemaProposal` in whichever declared state it was found.
pub enum AnySchemaProposal {
    /// Resting in `Recorded`.
    Recorded(SchemaProposal<schema_proposal_state::Recorded>),
}

impl SchemaProposalSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SchemaProposalState` cannot spell one.
    pub fn refine(self) -> AnySchemaProposal {
        match self.state {
            SchemaProposalState::Recorded => AnySchemaProposal::Recorded(SchemaProposal {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySchemaProposal {
    /// The state, as the runtime value.
    pub fn state(&self) -> SchemaProposalState {
        match self {
            Self::Recorded(_) => SchemaProposalState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SchemaProposalSnapshot {
        match self {
            Self::Recorded(instance) => SchemaProposalSnapshot {
                state: SchemaProposalState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What Split — `ekr.integrate.Split` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Split<S>`], and at a boundary by [`SplitSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitData {
    /// The identity: `split_id` — `ekr.integrate.SplitId`.
    pub split_id: SplitId,
}

/// The states of `ekr.integrate.Split`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](split_state::Marker), so [`Split<S>`](Split) can only ever rest in a real state.
pub mod split_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `Split`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SplitState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::SplitState = super::SplitState::Recorded;
    }
}

/// Split — `ekr.integrate.Split` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SplitSnapshot`]
/// and [`SplitSnapshot::refine`].
pub struct Split<S: split_state::Marker> {
    data: SplitData,
    state: core::marker::PhantomData<S>,
}

impl<S: split_state::Marker> Split<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SplitState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SplitData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SplitData {
        self.data
    }
}

impl Split<split_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: SplitData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.integrate.Split` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SplitSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SplitState,
    /// What it holds.
    pub data: SplitData,
}

/// An `Split` in whichever declared state it was found.
pub enum AnySplit {
    /// Resting in `Recorded`.
    Recorded(Split<split_state::Recorded>),
}

impl SplitSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SplitState` cannot spell one.
    pub fn refine(self) -> AnySplit {
        match self.state {
            SplitState::Recorded => AnySplit::Recorded(Split {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySplit {
    /// The state, as the runtime value.
    pub fn state(&self) -> SplitState {
        match self {
            Self::Recorded(_) => SplitState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SplitSnapshot {
        match self {
            Self::Recorded(instance) => SplitSnapshot {
                state: SplitState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// Apply an extraction document — the input of `ekr.integrate.ApplyExtraction`.
///
/// Everything it can result in is [`ApplyExtractionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyExtraction {
    /// `extraction_document` — `ekr.integrate.ExtractionDocumentPath`.
    pub extraction_document: ExtractionDocumentPath,
}

/// Actual typed response of `ekr.integrate.ApplyExtraction`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyExtractionResponse {
    /// `report` — `ekr.integrate.ExtractionReport`.
    pub report: ExtractionReport,
}

/// Everything `ekr.integrate.ApplyExtraction` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyExtractionOutcome {
    /// `applied` — otherwise.
    ///
    /// The document was read and every part of it tried; the report lists what committed, what was rejected and which named things were ambiguous.
    Applied {
        /// The `ekr.integrate.ExtractionApplied` this outcome publishes.
        extraction_applied: ExtractionApplied,
    },
    /// `refused` — externally decided (the reader refuses the document against the ontology of the store's head).
    ///
    /// Nothing was proposed or written; the refusal names the first refused part in document order.
    Refused {
        /// Why it was refused: `ekr.integrate.ExtractionRefused`.
        error: ExtractionRefused,
    },
}

/// ApplySchemaProposal — the input of `ekr.integrate.ApplySchemaProposal`.
///
/// Everything it can result in is [`ApplySchemaProposalOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplySchemaProposal {
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `review_id` — `ekr.integrate.ProposalReviewId`.
    pub review_id: ProposalReviewId,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: crate::kernel::ContentHash,
}

/// Actual typed response of `ekr.integrate.ApplySchemaProposal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplySchemaProposalResponse {
    /// `receipt` — `ekr.integrate.ApplicationReport`.
    pub receipt: ApplicationReport,
}

/// Everything `ekr.integrate.ApplySchemaProposal` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplySchemaProposalOutcome {
    /// `answered` — otherwise.
    ///
    /// Revalidate the exact approved additions/mappings/corrections and latest effective review before each canonical write; a later rejection stops application/resume and reports confirmed partial progress. Commit schema with evidence first, then selected facts and corrections by ordinary validated transactions. Return durable partial progress on interruption; reconcile receipts and resume without duplicates.
    Answered {
        /// The `ekr.integrate.ApplySchemaProposalResult` this outcome publishes.
        apply_schema_proposal_result: ApplySchemaProposalResult,
    },
    /// `refused` — externally decided (Before mutation require the referenced review to match proposal_id and exact proposal digest, to be Approved, and to be the latest trusted human decision for that proposal. Refuse changed digest/evidence/options/effects or incompatible base schema. A later failure after any commit produces a partial receipt, not a refusal.).
    Refused {
        /// Why it was refused: `ekr.integrate.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// ApproveSchemaProposal — the input of `ekr.integrate.ApproveSchemaProposal`.
///
/// Everything it can result in is [`ApproveSchemaProposalOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApproveSchemaProposal {
    /// `human_proof` — `ekr.kernel.SignedHumanDecision`.
    pub human_proof: crate::kernel::SignedHumanDecision,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `basis` — `ekr.kernel.ReviewBasis`.
    pub basis: crate::kernel::ReviewBasis,
    /// `statement` — `Bytes`.
    pub statement: Vec<u8>,
}

/// Actual typed response of `ekr.integrate.ApproveSchemaProposal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApproveSchemaProposalResponse {
    /// `review_id` — `ekr.integrate.ProposalReviewId`.
    pub review_id: ProposalReviewId,
}

/// Everything `ekr.integrate.ApproveSchemaProposal` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApproveSchemaProposalOutcome {
    /// `answered` — otherwise.
    ///
    /// Record the host-authenticated human decision and retain its statement as evidence; approval includes additions, mappings and selected corrections.
    Answered {
        /// The `ekr.integrate.ApproveSchemaProposalResult` this outcome publishes.
        approve_schema_proposal_result: ApproveSchemaProposalResult,
    },
    /// `refused` — externally decided (Verify human_proof under the independently enrolled reviewer policy, with a target matching this approve/reject operation, proposal id, proposal digest, statement and expected predecessor decision. Derive the operator from the verified key registry; a host UUID, agent statement or caller-supplied key cannot authenticate a human. The exact proposal, reviewed evidence and intended effects must match. Agent-supplied content cannot grant approval.).
    Refused {
        /// Why it was refused: `ekr.integrate.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// DiscoverSchemaGaps — the input of `ekr.integrate.DiscoverSchemaGaps`.
///
/// Everything it can result in is [`DiscoverSchemaGapsOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverSchemaGaps {
}

/// Actual typed response of `ekr.integrate.DiscoverSchemaGaps`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverSchemaGapsResponse {
    /// `request` — `ekr.integrate.SchemaLearningRequest`.
    pub request: SchemaLearningRequest,
}

/// Everything `ekr.integrate.DiscoverSchemaGaps` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoverSchemaGapsOutcome {
    /// `answered` — otherwise.
    ///
    /// Deterministically group recurring integration blockers by kind and declared source vocabulary. Return a typed request for a consumer-supplied agent; do not start a model provider or scheduler.
    Answered {
        /// The `ekr.integrate.DiscoverSchemaGapsResult` this outcome publishes.
        discover_schema_gaps_result: DiscoverSchemaGapsResult,
    },
}

/// ImportInterpretation — the input of `ekr.integrate.ImportInterpretation`.
///
/// Everything it can result in is [`ImportInterpretationOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportInterpretation {
    /// `document` — `ekr.integrate.InterpretationDocument`.
    pub document: InterpretationDocument,
    /// `payload` — `Bytes`.
    pub payload: Vec<u8>,
}

/// Actual typed response of `ekr.integrate.ImportInterpretation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportInterpretationResponse {
    /// `receipt` — `ekr.integrate.IncubationImportReceipt`.
    pub receipt: IncubationImportReceipt,
}

/// Everything `ekr.integrate.ImportInterpretation` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportInterpretationOutcome {
    /// `answered` — otherwise.
    ///
    /// Retain immutable document bytes and local declarations, bind a Transient GraphRoot, and report blockers. Retain exact document bytes by content digest; materialize the TransientGraph for inspection without serializing or canonically hashing it. Supplied observation bytes must already be independently retained. Repeated exact import returns the same version and no duplicate blockers.
    Answered {
        /// The `ekr.integrate.ImportInterpretationResult` this outcome publishes.
        import_interpretation_result: ImportInterpretationResult,
    },
    /// `refused` — externally decided (The typed document must match its bytes and digest; its root must be Transient, every observation retained, every evidence source admissible, and an existing version immutable. Local declaration/reference integrity is checked independently of canonical ontology; a canonical vocabulary mismatch is a durable blocker, not an import refusal.).
    Refused {
        /// Why it was refused: `ekr.integrate.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// ListInterpretations — the input of `ekr.integrate.ListInterpretations`.
///
/// Everything it can result in is [`ListInterpretationsOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListInterpretations {
}

/// Actual typed response of `ekr.integrate.ListInterpretations`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListInterpretationsResponse {
    /// `interpretations` — `List<ekr.integrate.InterpretationVersion>`.
    pub interpretations: Vec<InterpretationVersion>,
}

/// Everything `ekr.integrate.ListInterpretations` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListInterpretationsOutcome {
    /// `answered` — otherwise.
    ///
    /// Read retained interpretation versions in stable identity/version order without canonical mutation.
    Answered {
        /// The `ekr.integrate.ListInterpretationsResult` this outcome publishes.
        list_interpretations_result: ListInterpretationsResult,
    },
}

/// RejectSchemaProposal — the input of `ekr.integrate.RejectSchemaProposal`.
///
/// Everything it can result in is [`RejectSchemaProposalOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectSchemaProposal {
    /// `human_proof` — `ekr.kernel.SignedHumanDecision`.
    pub human_proof: crate::kernel::SignedHumanDecision,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `basis` — `ekr.kernel.ReviewBasis`.
    pub basis: crate::kernel::ReviewBasis,
    /// `statement` — `Bytes`.
    pub statement: Vec<u8>,
}

/// Actual typed response of `ekr.integrate.RejectSchemaProposal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectSchemaProposalResponse {
    /// `review_id` — `ekr.integrate.ProposalReviewId`.
    pub review_id: ProposalReviewId,
}

/// Everything `ekr.integrate.RejectSchemaProposal` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectSchemaProposalOutcome {
    /// `answered` — otherwise.
    ///
    /// Record the host-authenticated human decision and retain its statement as evidence; approval includes additions, mappings and selected corrections.
    Answered {
        /// The `ekr.integrate.RejectSchemaProposalResult` this outcome publishes.
        reject_schema_proposal_result: RejectSchemaProposalResult,
    },
    /// `refused` — externally decided (Verify human_proof under the independently enrolled reviewer policy, with a target matching this approve/reject operation, proposal id, proposal digest, statement and expected predecessor decision. Derive the operator from the verified key registry; a host UUID, agent statement or caller-supplied key cannot authenticate a human. The exact proposal, reviewed evidence and intended effects must match. Agent-supplied content cannot grant approval.).
    Refused {
        /// Why it was refused: `ekr.integrate.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// ShowInterpretation — the input of `ekr.integrate.ShowInterpretation`.
///
/// Everything it can result in is [`ShowInterpretationOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowInterpretation {
    /// `version` — `ekr.integrate.InterpretationVersion`.
    pub version: InterpretationVersion,
}

/// Actual typed response of `ekr.integrate.ShowInterpretation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowInterpretationResponse {
    /// `document` — `ekr.integrate.InterpretationDocument`.
    pub document: InterpretationDocument,
    /// `blockers` — `List<ekr.integrate.IntegrationBlockerSnapshot>`.
    pub blockers: Vec<IntegrationBlockerSnapshot>,
    /// `receipts` — `List<ekr.integrate.ProcessingReceiptSnapshot>`.
    pub receipts: Vec<ProcessingReceiptSnapshot>,
}

/// Everything `ekr.integrate.ShowInterpretation` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowInterpretationOutcome {
    /// `answered` — otherwise.
    ///
    /// Read immutable local schema, facts, source references and processing history after reopen.
    Answered {
        /// The `ekr.integrate.ShowInterpretationResult` this outcome publishes.
        show_interpretation_result: ShowInterpretationResult,
    },
    /// `refused` — externally decided (An unknown version or changed document digest is refused.).
    Refused {
        /// Why it was refused: `ekr.integrate.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// ShowSchemaProposal — the input of `ekr.integrate.ShowSchemaProposal`.
///
/// Everything it can result in is [`ShowSchemaProposalOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowSchemaProposal {
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
}

/// Actual typed response of `ekr.integrate.ShowSchemaProposal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowSchemaProposalResponse {
    /// `proposal` — `ekr.integrate.SchemaProposalDocument`.
    pub proposal: SchemaProposalDocument,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `preview` — `List<ekr.integrate.MappingPreview>`.
    pub preview: Vec<MappingPreview>,
    /// `reviews` — `List<ekr.integrate.ProposalReviewSnapshot>`.
    pub reviews: Vec<ProposalReviewSnapshot>,
    /// `receipts` — `List<ekr.integrate.ApplicationReceiptSnapshot>`.
    pub receipts: Vec<ApplicationReceiptSnapshot>,
}

/// Everything `ekr.integrate.ShowSchemaProposal` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowSchemaProposalOutcome {
    /// `answered` — otherwise.
    ///
    /// Show proposed additions, supporting observations, exact mappings and review/application history through CLI, SDK and the read-only viewer.
    Answered {
        /// The `ekr.integrate.ShowSchemaProposalResult` this outcome publishes.
        show_schema_proposal_result: ShowSchemaProposalResult,
    },
    /// `refused` — externally decided (An unknown proposal is refused.).
    Refused {
        /// Why it was refused: `ekr.integrate.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// SubmitSchemaProposal — the input of `ekr.integrate.SubmitSchemaProposal`.
///
/// Everything it can result in is [`SubmitSchemaProposalOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitSchemaProposal {
    /// `proposal` — `ekr.integrate.SchemaProposalDocument`.
    pub proposal: SchemaProposalDocument,
    /// `payload` — `Bytes`.
    pub payload: Vec<u8>,
}

/// Actual typed response of `ekr.integrate.SubmitSchemaProposal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitSchemaProposalResponse {
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `preview` — `List<ekr.integrate.MappingPreview>`.
    pub preview: Vec<MappingPreview>,
}

/// Everything `ekr.integrate.SubmitSchemaProposal` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitSchemaProposalOutcome {
    /// `answered` — otherwise.
    ///
    /// Retain the exact proposal and a mapping preview without changing ontology or claims. Retained observations and immutable source documents are sufficient support; canonical evidence ids are optional additional citations before application.
    Answered {
        /// The `ekr.integrate.SubmitSchemaProposalResult` this outcome publishes.
        submit_schema_proposal_result: SubmitSchemaProposalResult,
    },
    /// `refused` — externally decided (Refuse mutable or missing sources/evidence, nonadditive changes, required property additions, undeclared selectors, mismatched typed constants and enum exhaustion inferred from observed values.).
    Refused {
        /// Why it was refused: `ekr.integrate.KnowledgeRefused`.
        error: KnowledgeRefused,
    },
}

/// ApplySchemaProposalResult — the event `ekr.integrate.ApplySchemaProposalResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplySchemaProposalResult {
    /// `receipt` — `ekr.integrate.ApplicationReport`.
    pub receipt: ApplicationReport,
}

/// ApproveSchemaProposalResult — the event `ekr.integrate.ApproveSchemaProposalResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApproveSchemaProposalResult {
    /// `review_id` — `ekr.integrate.ProposalReviewId`.
    pub review_id: ProposalReviewId,
}

/// DiscoverSchemaGapsResult — the event `ekr.integrate.DiscoverSchemaGapsResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverSchemaGapsResult {
    /// `request` — `ekr.integrate.SchemaLearningRequest`.
    pub request: SchemaLearningRequest,
}

/// ExtractionApplied — the event `ekr.integrate.ExtractionApplied`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionApplied {
    /// `committed` — `Integer`.
    pub committed: i64,
    /// `rejected` — `Integer`.
    pub rejected: i64,
    /// `ambiguous` — `Integer`.
    pub ambiguous: i64,
    /// `held` — `Integer`.
    pub held: i64,
}

/// ImportInterpretationResult — the event `ekr.integrate.ImportInterpretationResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportInterpretationResult {
    /// `receipt` — `ekr.integrate.IncubationImportReceipt`.
    pub receipt: IncubationImportReceipt,
}

/// ListInterpretationsResult — the event `ekr.integrate.ListInterpretationsResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListInterpretationsResult {
    /// `interpretations` — `List<ekr.integrate.InterpretationVersion>`.
    pub interpretations: Vec<InterpretationVersion>,
}

/// RejectSchemaProposalResult — the event `ekr.integrate.RejectSchemaProposalResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectSchemaProposalResult {
    /// `review_id` — `ekr.integrate.ProposalReviewId`.
    pub review_id: ProposalReviewId,
}

/// ShowInterpretationResult — the event `ekr.integrate.ShowInterpretationResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowInterpretationResult {
    /// `document` — `ekr.integrate.InterpretationDocument`.
    pub document: InterpretationDocument,
    /// `blockers` — `List<ekr.integrate.IntegrationBlockerSnapshot>`.
    pub blockers: Vec<IntegrationBlockerSnapshot>,
    /// `receipts` — `List<ekr.integrate.ProcessingReceiptSnapshot>`.
    pub receipts: Vec<ProcessingReceiptSnapshot>,
}

/// ShowSchemaProposalResult — the event `ekr.integrate.ShowSchemaProposalResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowSchemaProposalResult {
    /// `proposal` — `ekr.integrate.SchemaProposalDocument`.
    pub proposal: SchemaProposalDocument,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `preview` — `List<ekr.integrate.MappingPreview>`.
    pub preview: Vec<MappingPreview>,
    /// `reviews` — `List<ekr.integrate.ProposalReviewSnapshot>`.
    pub reviews: Vec<ProposalReviewSnapshot>,
    /// `receipts` — `List<ekr.integrate.ApplicationReceiptSnapshot>`.
    pub receipts: Vec<ApplicationReceiptSnapshot>,
}

/// SubmitSchemaProposalResult — the event `ekr.integrate.SubmitSchemaProposalResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitSchemaProposalResult {
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `preview` — `List<ekr.integrate.MappingPreview>`.
    pub preview: Vec<MappingPreview>,
}

/// The declared error `ekr.integrate.ExtractionRefused`.
///
/// The reader refused the extraction document; nothing was proposed and nothing was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionRefused {
    /// `code` — `ekr.integrate.ExtractionRefusalCode`.
    pub code: ExtractionRefusalCode,
    /// `name` — `String`.
    pub name: String,
}

/// The declared error `ekr.integrate.KnowledgeRefused`.
///
/// A named deterministic refusal; no unreported write occurred. Partial application is a typed report, never this refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeRefused {
    /// `code` — `String`.
    pub code: String,
    /// `reason` — `String`.
    pub reason: String,
}

/// ApplicationReceiptRecords — one row of the view `ekr.integrate.ApplicationReceiptRecords`.
///
/// Projects `ekr.integrate.ApplicationReceipt` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationReceiptRecords {
    /// `receipt_id` — `ekr.integrate.ApplicationReceiptId`.
    pub receipt_id: ApplicationReceiptId,
    /// `state` — `ekr.integrate.ApplicationReceipt.State`.
    pub state: ApplicationReceiptState,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `review_id` — `ekr.integrate.ProposalReviewId`.
    pub review_id: ProposalReviewId,
    /// `progress` — `ekr.integrate.ApplicationProgress`.
    pub progress: ApplicationProgress,
    /// `schema_transaction` — `ekr.kernel.TransactionId`.
    pub schema_transaction: crate::kernel::TransactionId,
    /// `schema_revision` — `ekr.kernel.RevisionNumber`.
    pub schema_revision: crate::kernel::RevisionNumber,
    /// `processing_receipts` — `List<ekr.integrate.ProcessingReceiptId>`.
    pub processing_receipts: Vec<ProcessingReceiptId>,
    /// `remaining_items` — `List<String>`.
    pub remaining_items: Vec<String>,
    /// `stop_reason` — `Optional<String>`.
    pub stop_reason: Option<String>,
}

/// CanonicalDerivationRecords — one row of the view `ekr.integrate.CanonicalDerivationRecords`.
///
/// Projects `ekr.integrate.CanonicalDerivation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDerivationRecords {
    /// `derivation_id` — `Uuid`.
    pub derivation_id: crate::primitives::Uuid,
    /// `state` — `ekr.integrate.CanonicalDerivation.State`.
    pub state: CanonicalDerivationState,
    /// `assertion_id` — `ekr.graph.AssertionId`.
    pub assertion_id: crate::graph::AssertionId,
    /// `mapping_id` — `ekr.integrate.MappingRecordId`.
    pub mapping_id: MappingRecordId,
    /// `observation_id` — `ekr.graph.ObservationId`.
    pub observation_id: crate::graph::ObservationId,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    pub evidence_id: crate::graph::EvidenceId,
}

/// IntegrationBlockerRecords — one row of the view `ekr.integrate.IntegrationBlockerRecords`.
///
/// Projects `ekr.integrate.IntegrationBlocker` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationBlockerRecords {
    /// `blocker_id` — `ekr.integrate.IntegrationBlockerId`.
    pub blocker_id: IntegrationBlockerId,
    /// `state` — `ekr.integrate.IntegrationBlocker.State`.
    pub state: IntegrationBlockerState,
    /// `document_digest` — `ekr.kernel.ContentHash`.
    pub document_digest: crate::kernel::ContentHash,
    /// `item` — `String`.
    pub item: String,
    /// `kind` — `ekr.integrate.IntegrationBlockerKind`.
    pub kind: IntegrationBlockerKind,
    /// `declaration` — `String`.
    pub declaration: String,
    /// `reason` — `String`.
    pub reason: String,
    /// `basis_digest` — `ekr.kernel.ContentHash`.
    pub basis_digest: crate::kernel::ContentHash,
}

/// InterpretationObservationRecords — one row of the view `ekr.integrate.InterpretationObservationRecords`.
///
/// Projects `ekr.integrate.InterpretationObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretationObservationRecords {
    /// `link_id` — `Uuid`.
    pub link_id: crate::primitives::Uuid,
    /// `state` — `ekr.integrate.InterpretationObservation.State`.
    pub state: InterpretationObservationState,
    /// `document_digest` — `ekr.kernel.ContentHash`.
    pub document_digest: crate::kernel::ContentHash,
    /// `observation_id` — `ekr.graph.ObservationId`.
    pub observation_id: crate::graph::ObservationId,
}

/// InterpretationRecords — one row of the view `ekr.integrate.InterpretationRecords`.
///
/// Projects `ekr.integrate.Interpretation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpretationRecords {
    /// `document_digest` — `ekr.kernel.ContentHash`.
    pub document_digest: crate::kernel::ContentHash,
    /// `state` — `ekr.integrate.Interpretation.State`.
    pub state: InterpretationState,
    /// `interpretation_id` — `ekr.integrate.InterpretationId`.
    pub interpretation_id: InterpretationId,
    /// `version` — `Integer`.
    pub version: i64,
    /// `root_id` — `ekr.graph.GraphRootId`.
    pub root_id: crate::graph::GraphRootId,
    /// `document` — `ekr.integrate.InterpretationDocument`.
    pub document: InterpretationDocument,
}

/// MappingRecordRecords — one row of the view `ekr.integrate.MappingRecordRecords`.
///
/// Projects `ekr.integrate.MappingRecord` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingRecordRecords {
    /// `mapping_id` — `ekr.integrate.MappingRecordId`.
    pub mapping_id: MappingRecordId,
    /// `state` — `ekr.integrate.MappingRecord.State`.
    pub state: MappingRecordState,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `mapping_digest` — `ekr.kernel.ContentHash`.
    pub mapping_digest: crate::kernel::ContentHash,
    /// `source_document_digest` — `ekr.kernel.ContentHash`.
    pub source_document_digest: crate::kernel::ContentHash,
    /// `mapping` — `ekr.integrate.KnowledgeMapping`.
    pub mapping: KnowledgeMapping,
    /// `evidence` — `List<ekr.graph.EvidenceId>`.
    pub evidence: Vec<crate::graph::EvidenceId>,
}

/// ProcessingReceiptRecords — one row of the view `ekr.integrate.ProcessingReceiptRecords`.
///
/// Projects `ekr.integrate.ProcessingReceipt` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingReceiptRecords {
    /// `mapping_digest` — `Optional<ekr.kernel.ContentHash>`.
    pub mapping_digest: Option<crate::kernel::ContentHash>,
    /// `receipt_id` — `ekr.integrate.ProcessingReceiptId`.
    pub receipt_id: ProcessingReceiptId,
    /// `state` — `ekr.integrate.ProcessingReceipt.State`.
    pub state: ProcessingReceiptState,
    /// `document_digest` — `ekr.kernel.ContentHash`.
    pub document_digest: crate::kernel::ContentHash,
    /// `item` — `String`.
    pub item: String,
    /// `disposition` — `ekr.integrate.ProcessingDisposition`.
    pub disposition: ProcessingDisposition,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<crate::kernel::TransactionId>,
    /// `assertions` — `List<ekr.graph.AssertionId>`.
    pub assertions: Vec<crate::graph::AssertionId>,
    /// `basis_digest` — `ekr.kernel.ContentHash`.
    pub basis_digest: crate::kernel::ContentHash,
}

/// ProposalEvidenceRecords — one row of the view `ekr.integrate.ProposalEvidenceRecords`.
///
/// Projects `ekr.integrate.ProposalEvidence` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalEvidenceRecords {
    /// `binding_id` — `Uuid`.
    pub binding_id: crate::primitives::Uuid,
    /// `state` — `ekr.integrate.ProposalEvidence.State`.
    pub state: ProposalEvidenceState,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    pub evidence_id: crate::graph::EvidenceId,
}

/// ProposalObservationRecords — one row of the view `ekr.integrate.ProposalObservationRecords`.
///
/// Projects `ekr.integrate.ProposalObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalObservationRecords {
    /// `binding_id` — `Uuid`.
    pub binding_id: crate::primitives::Uuid,
    /// `state` — `ekr.integrate.ProposalObservation.State`.
    pub state: ProposalObservationState,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `observation_id` — `ekr.graph.ObservationId`.
    pub observation_id: crate::graph::ObservationId,
}

/// ProposalReviewRecords — one row of the view `ekr.integrate.ProposalReviewRecords`.
///
/// Projects `ekr.integrate.ProposalReview` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalReviewRecords {
    /// `human_proof_digest` — `ekr.kernel.ContentHash`.
    pub human_proof_digest: crate::kernel::ContentHash,
    /// `review_id` — `ekr.integrate.ProposalReviewId`.
    pub review_id: ProposalReviewId,
    /// `state` — `ekr.integrate.ProposalReview.State`.
    pub state: ProposalReviewState,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `basis` — `ekr.kernel.ReviewBasis`.
    pub basis: crate::kernel::ReviewBasis,
    /// `decision` — `ekr.integrate.ReviewDecision`.
    pub decision: ReviewDecision,
    /// `operator` — `ekr.kernel.TrustedOperatorIdentity`.
    pub operator: crate::kernel::TrustedOperatorIdentity,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    pub evidence_id: crate::graph::EvidenceId,
    /// `recorded_at` — `Timestamp`.
    pub recorded_at: crate::primitives::Timestamp,
}

/// ProposalSourceBindingRecords — one row of the view `ekr.integrate.ProposalSourceBindingRecords`.
///
/// Projects `ekr.integrate.ProposalSourceBinding` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalSourceBindingRecords {
    /// `binding_id` — `Uuid`.
    pub binding_id: crate::primitives::Uuid,
    /// `state` — `ekr.integrate.ProposalSourceBinding.State`.
    pub state: ProposalSourceBindingState,
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `document_digest` — `ekr.kernel.ContentHash`.
    pub document_digest: crate::kernel::ContentHash,
}

/// SchemaProposalRecords — one row of the view `ekr.integrate.SchemaProposalRecords`.
///
/// Projects `ekr.integrate.SchemaProposal` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProposalRecords {
    /// `proposal_id` — `ekr.integrate.SchemaProposalId`.
    pub proposal_id: SchemaProposalId,
    /// `state` — `ekr.integrate.SchemaProposal.State`.
    pub state: SchemaProposalState,
    /// `proposal_digest` — `ekr.kernel.ContentHash`.
    pub proposal_digest: crate::kernel::ContentHash,
    /// `base_schema` — `ekr.ontology.SchemaVersionId`.
    pub base_schema: crate::ontology::SchemaVersionId,
    /// `document` — `ekr.integrate.SchemaProposalDocument`.
    pub document: SchemaProposalDocument,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `ekr.integrate.ApplyExtraction` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.integrate.ApplyExtraction` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `applied` otherwise, emits `ekr.integrate.ExtractionApplied`; `refused` externally decided (the reader refuses the document against the ontology of the store's head), error `ekr.integrate.ExtractionRefused`.
    pub trait ApplyExtractionBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.integrate.ApplyExtraction`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn apply_extraction(&mut self, input: super::ApplyExtraction) -> Result<super::ApplyExtractionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.integrate.ApplySchemaProposal` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.integrate.ApplySchemaProposal` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.ApplySchemaProposalResult`; `refused` externally decided (Before mutation require the referenced review to match proposal_id and exact proposal digest, to be Approved, and to be the latest trusted human decision for that proposal. Refuse changed digest/evidence/options/effects or incompatible base schema. A later failure after any commit produces a partial receipt, not a refusal.), error `ekr.integrate.KnowledgeRefused`.
    pub trait ApplySchemaProposalBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.integrate.ApplySchemaProposal`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn apply_schema_proposal(&mut self, input: super::ApplySchemaProposal) -> Result<super::ApplySchemaProposalOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.integrate.ApproveSchemaProposal` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.integrate.ApproveSchemaProposal` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.ApproveSchemaProposalResult`; `refused` externally decided (Verify human_proof under the independently enrolled reviewer policy, with a target matching this approve/reject operation, proposal id, proposal digest, statement and expected predecessor decision. Derive the operator from the verified key registry; a host UUID, agent statement or caller-supplied key cannot authenticate a human. The exact proposal, reviewed evidence and intended effects must match. Agent-supplied content cannot grant approval.), error `ekr.integrate.KnowledgeRefused`.
    pub trait ApproveSchemaProposalBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.integrate.ApproveSchemaProposal`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn approve_schema_proposal(&mut self, input: super::ApproveSchemaProposal) -> Result<super::ApproveSchemaProposalOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.integrate.DiscoverSchemaGaps` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.integrate.DiscoverSchemaGaps` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.DiscoverSchemaGapsResult`.
    pub trait DiscoverSchemaGapsBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.integrate.DiscoverSchemaGaps`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn discover_schema_gaps(&mut self, input: super::DiscoverSchemaGaps) -> Result<super::DiscoverSchemaGapsOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.integrate.ImportInterpretation` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.integrate.ImportInterpretation` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.ImportInterpretationResult`; `refused` externally decided (The typed document must match its bytes and digest; its root must be Transient, every observation retained, every evidence source admissible, and an existing version immutable. Local declaration/reference integrity is checked independently of canonical ontology; a canonical vocabulary mismatch is a durable blocker, not an import refusal.), error `ekr.integrate.KnowledgeRefused`.
    pub trait ImportInterpretationBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.integrate.ImportInterpretation`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn import_interpretation(&mut self, input: super::ImportInterpretation) -> Result<super::ImportInterpretationOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.integrate.ListInterpretations` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.integrate.ListInterpretations` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.ListInterpretationsResult`.
    pub trait ListInterpretationsBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.integrate.ListInterpretations`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn list_interpretations(&mut self, input: super::ListInterpretations) -> Result<super::ListInterpretationsOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.integrate.RejectSchemaProposal` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.integrate.RejectSchemaProposal` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.RejectSchemaProposalResult`; `refused` externally decided (Verify human_proof under the independently enrolled reviewer policy, with a target matching this approve/reject operation, proposal id, proposal digest, statement and expected predecessor decision. Derive the operator from the verified key registry; a host UUID, agent statement or caller-supplied key cannot authenticate a human. The exact proposal, reviewed evidence and intended effects must match. Agent-supplied content cannot grant approval.), error `ekr.integrate.KnowledgeRefused`.
    pub trait RejectSchemaProposalBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.integrate.RejectSchemaProposal`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn reject_schema_proposal(&mut self, input: super::RejectSchemaProposal) -> Result<super::RejectSchemaProposalOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.integrate.ShowInterpretation` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.integrate.ShowInterpretation` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.ShowInterpretationResult`; `refused` externally decided (An unknown version or changed document digest is refused.), error `ekr.integrate.KnowledgeRefused`.
    pub trait ShowInterpretationBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.integrate.ShowInterpretation`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn show_interpretation(&mut self, input: super::ShowInterpretation) -> Result<super::ShowInterpretationOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.integrate.ShowSchemaProposal` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.integrate.ShowSchemaProposal` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.ShowSchemaProposalResult`; `refused` externally decided (An unknown proposal is refused.), error `ekr.integrate.KnowledgeRefused`.
    pub trait ShowSchemaProposalBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.integrate.ShowSchemaProposal`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn show_schema_proposal(&mut self, input: super::ShowSchemaProposal) -> Result<super::ShowSchemaProposalOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.integrate.SubmitSchemaProposal` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.integrate.SubmitSchemaProposal` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.SubmitSchemaProposalResult`; `refused` externally decided (Refuse mutable or missing sources/evidence, nonadditive changes, required property additions, undeclared selectors, mismatched typed constants and enum exhaustion inferred from observed values.), error `ekr.integrate.KnowledgeRefused`.
    pub trait SubmitSchemaProposalBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.integrate.SubmitSchemaProposal`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn submit_schema_proposal(&mut self, input: super::SubmitSchemaProposal) -> Result<super::SubmitSchemaProposalOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.integrate.ApplicationReceiptRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ApplicationReceiptRecordsQuery {
        /// Serves `ekr.integrate.ApplicationReceiptRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn application_receipt_records(&self) -> Result<Vec<super::ApplicationReceiptRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.integrate.CanonicalDerivationRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait CanonicalDerivationRecordsQuery {
        /// Serves `ekr.integrate.CanonicalDerivationRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn canonical_derivation_records(&self) -> Result<Vec<super::CanonicalDerivationRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.integrate.IntegrationBlockerRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait IntegrationBlockerRecordsQuery {
        /// Serves `ekr.integrate.IntegrationBlockerRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn integration_blocker_records(&self) -> Result<Vec<super::IntegrationBlockerRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.integrate.InterpretationObservationRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait InterpretationObservationRecordsQuery {
        /// Serves `ekr.integrate.InterpretationObservationRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn interpretation_observation_records(&self) -> Result<Vec<super::InterpretationObservationRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.integrate.InterpretationRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait InterpretationRecordsQuery {
        /// Serves `ekr.integrate.InterpretationRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn interpretation_records(&self) -> Result<Vec<super::InterpretationRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.integrate.MappingRecordRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait MappingRecordRecordsQuery {
        /// Serves `ekr.integrate.MappingRecordRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn mapping_record_records(&self) -> Result<Vec<super::MappingRecordRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.integrate.ProcessingReceiptRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ProcessingReceiptRecordsQuery {
        /// Serves `ekr.integrate.ProcessingReceiptRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn processing_receipt_records(&self) -> Result<Vec<super::ProcessingReceiptRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.integrate.ProposalEvidenceRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ProposalEvidenceRecordsQuery {
        /// Serves `ekr.integrate.ProposalEvidenceRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn proposal_evidence_records(&self) -> Result<Vec<super::ProposalEvidenceRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.integrate.ProposalObservationRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ProposalObservationRecordsQuery {
        /// Serves `ekr.integrate.ProposalObservationRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn proposal_observation_records(&self) -> Result<Vec<super::ProposalObservationRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.integrate.ProposalReviewRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ProposalReviewRecordsQuery {
        /// Serves `ekr.integrate.ProposalReviewRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn proposal_review_records(&self) -> Result<Vec<super::ProposalReviewRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.integrate.ProposalSourceBindingRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ProposalSourceBindingRecordsQuery {
        /// Serves `ekr.integrate.ProposalSourceBindingRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn proposal_source_binding_records(&self) -> Result<Vec<super::ProposalSourceBindingRecords>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.integrate.SchemaProposalRecords` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SchemaProposalRecordsQuery {
        /// Serves `ekr.integrate.SchemaProposalRecords` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn schema_proposal_records(&self) -> Result<Vec<super::SchemaProposalRecords>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl ApplyExtractionBehavior for Unimplemented {
        fn apply_extraction(&mut self, _input: super::ApplyExtraction) -> Result<super::ApplyExtractionOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.integrate.ApplyExtraction" })
        }
    }

    impl ApplySchemaProposalBehavior for Unimplemented {
        fn apply_schema_proposal(&mut self, _input: super::ApplySchemaProposal) -> Result<super::ApplySchemaProposalOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.integrate.ApplySchemaProposal" })
        }
    }

    impl ApproveSchemaProposalBehavior for Unimplemented {
        fn approve_schema_proposal(&mut self, _input: super::ApproveSchemaProposal) -> Result<super::ApproveSchemaProposalOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.integrate.ApproveSchemaProposal" })
        }
    }

    impl DiscoverSchemaGapsBehavior for Unimplemented {
        fn discover_schema_gaps(&mut self, _input: super::DiscoverSchemaGaps) -> Result<super::DiscoverSchemaGapsOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.integrate.DiscoverSchemaGaps" })
        }
    }

    impl ImportInterpretationBehavior for Unimplemented {
        fn import_interpretation(&mut self, _input: super::ImportInterpretation) -> Result<super::ImportInterpretationOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.integrate.ImportInterpretation" })
        }
    }

    impl ListInterpretationsBehavior for Unimplemented {
        fn list_interpretations(&mut self, _input: super::ListInterpretations) -> Result<super::ListInterpretationsOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.integrate.ListInterpretations" })
        }
    }

    impl RejectSchemaProposalBehavior for Unimplemented {
        fn reject_schema_proposal(&mut self, _input: super::RejectSchemaProposal) -> Result<super::RejectSchemaProposalOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.integrate.RejectSchemaProposal" })
        }
    }

    impl ShowInterpretationBehavior for Unimplemented {
        fn show_interpretation(&mut self, _input: super::ShowInterpretation) -> Result<super::ShowInterpretationOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.integrate.ShowInterpretation" })
        }
    }

    impl ShowSchemaProposalBehavior for Unimplemented {
        fn show_schema_proposal(&mut self, _input: super::ShowSchemaProposal) -> Result<super::ShowSchemaProposalOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.integrate.ShowSchemaProposal" })
        }
    }

    impl SubmitSchemaProposalBehavior for Unimplemented {
        fn submit_schema_proposal(&mut self, _input: super::SubmitSchemaProposal) -> Result<super::SubmitSchemaProposalOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.integrate.SubmitSchemaProposal" })
        }
    }
}
