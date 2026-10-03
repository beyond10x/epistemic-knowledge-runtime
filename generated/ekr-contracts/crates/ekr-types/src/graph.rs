// generated from ekr v1
// model digest 890aea90d130e26fabda8485a25a78599aa119258635fdea52179df20555f601
// contract digest 281bbd37905ec8f6f636fc68d1767f3895a88fb29b6dbd4e383ae18d0e714c43
// do not edit: regenerate with `ess synthesize`

//! Graph — `ekr.graph`.
//!
//! Graph roots, nodes, edges, bitemporal assertions with independent assessment and lifecycle, evidence and observations. Design § 13–17, § 21–23, § 36. This domain has no commands: every change to it is an operation inside a transaction the kernel commits (evidence included, through AddEvidence), or the seed, which is why assertion assessment/lifecycle and per-type lifecycle state are fields here rather than ESS lifecycles — one commit applies many operations, and an outcome causes one move.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// The states of `ekr.graph.Assertion`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Assertion<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssertionState {
    /// `Recorded`.
    Recorded,
}

/// AssertionId — `ekr.graph.AssertionId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionId(pub crate::primitives::Uuid);

/// AssertionLifecycleKind — `ekr.graph.AssertionLifecycleKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssertionLifecycleKind {
    /// `Active`.
    Active,
    /// `Retracted`.
    Retracted,
    /// `Superseded`.
    Superseded,
}

/// AssertionLifecycleProjection — `ekr.graph.AssertionLifecycleProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionLifecycleProjection {
    /// `kind` — `ekr.graph.AssertionLifecycleKind`.
    pub kind: AssertionLifecycleKind,
    /// `at_revision` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at_revision: Option<crate::kernel::RevisionNumber>,
    /// `reason` — `Optional<String>`.
    pub reason: Option<String>,
    /// `by` — `Optional<ekr.graph.AssertionId>`.
    pub by: Option<AssertionId>,
    /// `effective_from` — `Optional<Timestamp>`.
    pub effective_from: Option<crate::primitives::Timestamp>,
}

/// AssertionRecord — `ekr.graph.AssertionRecord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionRecord {
    /// `id` — `ekr.graph.AssertionId`.
    pub id: AssertionId,
    /// `root_id` — `ekr.graph.GraphRootId`.
    pub root_id: GraphRootId,
    /// `subject` — `ekr.graph.SubjectProjection`.
    pub subject: SubjectProjection,
    /// `predicate` — `ekr.graph.PredicateProjection`.
    pub predicate: PredicateProjection,
    /// `object` — `ekr.graph.ObjectProjection`.
    pub object: ObjectProjection,
    /// `evidence` — `List<ekr.graph.EvidenceId>`.
    pub evidence: Vec<EvidenceId>,
    /// `proposed_by` — `ekr.kernel.AgentId`.
    pub proposed_by: crate::kernel::AgentId,
    /// `assessment` — `ekr.graph.AssessmentProjection`.
    pub assessment: AssessmentProjection,
    /// `lifecycle` — `ekr.graph.AssertionLifecycleProjection`.
    pub lifecycle: AssertionLifecycleProjection,
    /// `valid_time` — `ekr.graph.TemporalRange`.
    pub valid_time: TemporalRange,
    /// `transaction_time` — `ekr.graph.TransactionTime`.
    pub transaction_time: TransactionTime,
}

/// AssessmentKind — `ekr.graph.AssessmentKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentKind {
    /// `Proposed`.
    Proposed,
    /// `Validating`.
    Validating,
    /// `Accepted`.
    Accepted,
    /// `Rejected`.
    Rejected,
    /// `Disputed`.
    Disputed,
}

/// AssessmentProjection — `ekr.graph.AssessmentProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssessmentProjection {
    /// `kind` — `ekr.graph.AssessmentKind`.
    pub kind: AssessmentKind,
    /// `completed` — `Optional<Integer>`.
    pub completed: Option<i64>,
    /// `required` — `Optional<Integer>`.
    pub required: Option<i64>,
    /// `validators` — `Optional<List<ekr.kernel.AgentId>>`.
    pub validators: Option<Vec<crate::kernel::AgentId>>,
    /// `issues` — `Optional<List<ekr.kernel.IssueId>>`.
    pub issues: Option<Vec<crate::kernel::IssueId>>,
    /// `competing_assertions` — `Optional<List<ekr.graph.AssertionId>>`.
    pub competing_assertions: Option<Vec<AssertionId>>,
}

/// AttachedEvidenceRecord — `ekr.graph.AttachedEvidenceRecord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedEvidenceRecord {
    /// `evidence` — `ekr.graph.EvidenceId`.
    pub evidence: EvidenceId,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
}

/// AttachmentId — `ekr.graph.AttachmentId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentId(pub crate::primitives::Uuid);

/// CanonicalValueKind — `ekr.graph.CanonicalValueKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalValueKind {
    /// `String`.
    String,
    /// `Boolean`.
    Boolean,
    /// `Integer`.
    Integer,
    /// `Decimal`.
    Decimal,
    /// `Timestamp`.
    Timestamp,
    /// `Duration`.
    Duration,
    /// `NodeRef`.
    NodeRef,
    /// `Enum`.
    Enum,
    /// `List`.
    List,
    /// `Record`.
    Record,
}

/// The states of `ekr.graph.Edge`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Edge<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeState {
    /// `Present`.
    Present,
}

/// EdgeId — `ekr.graph.EdgeId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeId(pub crate::primitives::Uuid);

/// EdgeRecord — `ekr.graph.EdgeRecord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeRecord {
    /// `id` — `ekr.graph.EdgeId`.
    pub id: EdgeId,
    /// `root_id` — `ekr.graph.GraphRootId`.
    pub root_id: GraphRootId,
    /// `type_id` — `ekr.ontology.TypeId`.
    pub type_id: crate::ontology::TypeId,
    /// `source` — `ekr.graph.NodeId`.
    pub source: NodeId,
    /// `target` — `ekr.graph.NodeId`.
    pub target: NodeId,
    /// `properties` — `Map<String, List<ekr.graph.TypedValue>>`.
    pub properties: std::collections::BTreeMap<String, Vec<TypedValue>>,
}

/// The states of `ekr.graph.Evidence`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Evidence<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceState {
    /// `Retained`.
    Retained,
}

/// The states of `ekr.graph.EvidenceAttachment`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `EvidenceAttachment<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceAttachmentState {
    /// `Attached`.
    Attached,
}

/// EvidenceId — `ekr.graph.EvidenceId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceId(pub crate::primitives::Uuid);

/// EvidenceKind — `ekr.graph.EvidenceKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    /// `Url`.
    Url,
    /// `Document`.
    Document,
    /// `DatabaseRecord`.
    DatabaseRecord,
    /// `GraphAssertion`.
    GraphAssertion,
    /// `Observation`.
    Observation,
    /// `HumanStatement`.
    HumanStatement,
}

/// EvidenceRecord — `ekr.graph.EvidenceRecord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRecord {
    /// `id` — `ekr.graph.EvidenceId`.
    pub id: EvidenceId,
    /// `source` — `ekr.graph.EvidenceSourceProjection`.
    pub source: EvidenceSourceProjection,
    /// `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: crate::kernel::ContentHash,
    /// `extracted_by` — `ekr.kernel.AgentId`.
    pub extracted_by: crate::kernel::AgentId,
    /// `observed_at` — `Timestamp`.
    pub observed_at: crate::primitives::Timestamp,
    /// `confidence_bp` — `Integer`.
    pub confidence_bp: i64,
}

/// EvidenceSourceProjection — `ekr.graph.EvidenceSourceProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSourceProjection {
    /// `kind` — `ekr.graph.EvidenceKind`.
    pub kind: EvidenceKind,
    /// `url` — `Optional<String>`.
    pub url: Option<String>,
    /// `document_id` — `Optional<String>`.
    pub document_id: Option<String>,
    /// `section` — `Optional<String>`.
    pub section: Option<String>,
    /// `database` — `Optional<String>`.
    pub database: Option<String>,
    /// `table` — `Optional<String>`.
    pub table: Option<String>,
    /// `key` — `Optional<String>`.
    pub key: Option<String>,
    /// `assertion` — `Optional<ekr.graph.AssertionId>`.
    pub assertion: Option<AssertionId>,
    /// `observation` — `Optional<ekr.graph.ObservationId>`.
    pub observation: Option<ObservationId>,
    /// `identity` — `Optional<String>`.
    pub identity: Option<String>,
}

/// GraphDocumentBodyProjection — `ekr.graph.GraphDocumentBodyProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphDocumentBodyProjection {
    /// `root` — `ekr.graph.GraphRootRecord`.
    pub root: GraphRootRecord,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `nodes` — `Map<String, ekr.graph.NodeRecord>`.
    pub nodes: std::collections::BTreeMap<String, NodeRecord>,
    /// `edges` — `Map<String, ekr.graph.EdgeRecord>`.
    pub edges: std::collections::BTreeMap<String, EdgeRecord>,
    /// `assertions` — `Map<String, ekr.graph.AssertionRecord>`.
    pub assertions: std::collections::BTreeMap<String, AssertionRecord>,
    /// `evidence` — `Map<String, ekr.graph.EvidenceRecord>`.
    pub evidence: std::collections::BTreeMap<String, EvidenceRecord>,
    /// `attachments` — `Optional<Map<String, List<ekr.graph.AttachedEvidenceRecord>>>`.
    pub attachments: Option<std::collections::BTreeMap<String, Vec<AttachedEvidenceRecord>>>,
}

/// GraphDocumentFormatV2 — `ekr.graph.GraphDocumentFormatV2`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphDocumentFormatV2(pub String);

/// GraphDocumentV2Projection — `ekr.graph.GraphDocumentV2Projection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphDocumentV2Projection {
    /// `format` — `ekr.graph.GraphDocumentFormatV2`.
    pub format: GraphDocumentFormatV2,
    /// `graph` — `ekr.graph.GraphDocumentBodyProjection`.
    pub graph: GraphDocumentBodyProjection,
}

/// The states of `ekr.graph.GraphRoot`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `GraphRoot<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphRootState {
    /// `Active`.
    Active,
}

/// GraphRootId — `ekr.graph.GraphRootId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRootId(pub crate::primitives::Uuid);

/// GraphRootRecord — `ekr.graph.GraphRootRecord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRootRecord {
    /// `id` — `ekr.graph.GraphRootId`.
    pub id: GraphRootId,
    /// `space` — `ekr.graph.Space`.
    pub space: Space,
    /// `schema_version_id` — `ekr.ontology.SchemaVersionId`.
    pub schema_version_id: crate::ontology::SchemaVersionId,
    /// `parent` — `Optional<ekr.graph.GraphRootId>`.
    pub parent: Option<GraphRootId>,
    /// `created_at` — `Timestamp`.
    pub created_at: crate::primitives::Timestamp,
}

/// The states of `ekr.graph.Node`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Node<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    /// `Present`.
    Present,
}

/// NodeId — `ekr.graph.NodeId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeId(pub crate::primitives::Uuid);

/// NodeRecord — `ekr.graph.NodeRecord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRecord {
    /// `id` — `ekr.graph.NodeId`.
    pub id: NodeId,
    /// `root_id` — `ekr.graph.GraphRootId`.
    pub root_id: GraphRootId,
    /// `type_id` — `ekr.ontology.TypeId`.
    pub type_id: crate::ontology::TypeId,
    /// `canonical_name` — `String`.
    pub canonical_name: String,
    /// `aliases` — `List<String>`.
    pub aliases: Vec<String>,
    /// `type_state` — `Optional<String>`.
    pub type_state: Option<String>,
    /// `properties` — `Map<String, List<ekr.graph.TypedValue>>`.
    pub properties: std::collections::BTreeMap<String, Vec<TypedValue>>,
}

/// ObjectKind — `ekr.graph.ObjectKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    /// `Value`.
    Value,
    /// `Node`.
    Node,
    /// `Type`.
    Type,
}

/// ObjectProjection — `ekr.graph.ObjectProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectProjection {
    /// `kind` — `ekr.graph.ObjectKind`.
    pub kind: ObjectKind,
    /// `value` — `Optional<ekr.graph.TypedValue>`.
    pub value: Option<TypedValue>,
    /// `reference` — `Optional<Uuid>`.
    pub reference: Option<crate::primitives::Uuid>,
}

/// The states of `ekr.graph.Observation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Observation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationState {
    /// `Recorded`.
    Recorded,
}

/// ObservationId — `ekr.graph.ObservationId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationId(pub crate::primitives::Uuid);

/// ObservationKind — `ekr.graph.ObservationKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationKind {
    /// `Document`.
    Document,
    /// `ApiResponse`.
    ApiResponse,
    /// `DatabaseRecord`.
    DatabaseRecord,
    /// `FeedItem`.
    FeedItem,
    /// `GraphFragment`.
    GraphFragment,
    /// `MessageBatch`.
    MessageBatch,
    /// `GitDiff`.
    GitDiff,
    /// `Blob`.
    Blob,
}

/// ObservationRecord — `ekr.graph.ObservationRecord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationRecord {
    /// `observation_id` — `ekr.graph.ObservationId`.
    pub observation_id: ObservationId,
    /// `source` — `String`.
    pub source: String,
    /// `source_native_id` — `Optional<String>`.
    pub source_native_id: Option<String>,
    /// `kind` — `ekr.graph.ObservationKind`.
    pub kind: ObservationKind,
    /// `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: crate::kernel::ContentHash,
    /// `captured_at` — `Timestamp`.
    pub captured_at: crate::primitives::Timestamp,
}

/// PredicateKind — `ekr.graph.PredicateKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateKind {
    /// `Property`.
    Property,
    /// `Relation`.
    Relation,
}

/// PredicateProjection — `ekr.graph.PredicateProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateProjection {
    /// `kind` — `ekr.graph.PredicateKind`.
    pub kind: PredicateKind,
    /// `id` — `Uuid`.
    pub id: crate::primitives::Uuid,
}

/// RevisionRoot — `ekr.graph.RevisionRoot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionRoot {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `parent` — `Optional<ekr.kernel.ContentHash>`.
    pub parent: Option<crate::kernel::ContentHash>,
    /// `ontology_root` — `ekr.kernel.ContentHash`.
    pub ontology_root: crate::kernel::ContentHash,
    /// `knowledge_root` — `ekr.kernel.ContentHash`.
    pub knowledge_root: crate::kernel::ContentHash,
    /// `evidence_root` — `ekr.kernel.ContentHash`.
    pub evidence_root: crate::kernel::ContentHash,
    /// `agent_root` — `ekr.kernel.ContentHash`.
    pub agent_root: crate::kernel::ContentHash,
    /// `transaction` — `ekr.kernel.ContentHash`.
    pub transaction: crate::kernel::ContentHash,
}

/// Space — `ekr.graph.Space`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    /// `Canonical`.
    Canonical,
    /// `Transient`.
    Transient,
}

/// SubjectKind — `ekr.graph.SubjectKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubjectKind {
    /// `Node`.
    Node,
    /// `Edge`.
    Edge,
    /// `Type`.
    Type,
}

/// SubjectProjection — `ekr.graph.SubjectProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectProjection {
    /// `kind` — `ekr.graph.SubjectKind`.
    pub kind: SubjectKind,
    /// `id` — `Uuid`.
    pub id: crate::primitives::Uuid,
}

/// The states of `ekr.graph.Support`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Support<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportState {
    /// `Linked`.
    Linked,
}

/// SupportId — `ekr.graph.SupportId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportId(pub crate::primitives::Uuid);

/// TemporalRange — `ekr.graph.TemporalRange`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalRange {
    /// `from` — `Optional<Timestamp>`.
    pub from: Option<crate::primitives::Timestamp>,
    /// `to` — `Optional<Timestamp>`.
    pub to: Option<crate::primitives::Timestamp>,
}

/// TransactionTime — `ekr.graph.TransactionTime`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionTime {
    /// `recorded_from` — `Timestamp`.
    pub recorded_from: crate::primitives::Timestamp,
    /// `recorded_to` — `Optional<Timestamp>`.
    pub recorded_to: Option<crate::primitives::Timestamp>,
}

/// TypedValue — `ekr.graph.TypedValue`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedValue {
    /// `kind` — `ekr.graph.CanonicalValueKind`.
    pub kind: CanonicalValueKind,
    /// `canonical_bytes` — `Bytes`.
    pub canonical_bytes: Vec<u8>,
}

/// What Assertion — `ekr.graph.Assertion` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Assertion<S>`], and at a boundary by [`AssertionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionData {
    /// The identity: `assertion_id` — `ekr.graph.AssertionId`.
    pub assertion_id: AssertionId,
    /// `root_id` — `ekr.graph.GraphRootId`.
    ///
    /// Carries `assertions`: `ekr.graph.GraphRoot` owns many `ekr.graph.Assertion`.
    pub root_id: GraphRootId,
    /// `subject_kind` — `ekr.graph.SubjectKind`.
    pub subject_kind: SubjectKind,
    /// `subject` — `Uuid`.
    pub subject: crate::primitives::Uuid,
    /// `predicate_kind` — `ekr.graph.PredicateKind`.
    pub predicate_kind: PredicateKind,
    /// `predicate` — `Uuid`.
    pub predicate: crate::primitives::Uuid,
    /// `object_kind` — `ekr.graph.ObjectKind`.
    pub object_kind: ObjectKind,
    /// `object_value` — `Optional<ekr.graph.TypedValue>`.
    pub object_value: Option<TypedValue>,
    /// `object_ref` — `Optional<Uuid>`.
    pub object_ref: Option<crate::primitives::Uuid>,
    /// `proposed_by` — `ekr.kernel.AgentId`.
    pub proposed_by: crate::kernel::AgentId,
    /// `assessment` — `ekr.graph.AssessmentProjection`.
    pub assessment: AssessmentProjection,
    /// `lifecycle` — `ekr.graph.AssertionLifecycleProjection`.
    pub lifecycle: AssertionLifecycleProjection,
    /// `valid_from` — `Optional<Timestamp>`.
    pub valid_from: Option<crate::primitives::Timestamp>,
    /// `valid_to` — `Optional<Timestamp>`.
    pub valid_to: Option<crate::primitives::Timestamp>,
    /// `recorded_from` — `Timestamp`.
    pub recorded_from: crate::primitives::Timestamp,
    /// `recorded_to` — `Optional<Timestamp>`.
    pub recorded_to: Option<crate::primitives::Timestamp>,
}

/// The states of `ekr.graph.Assertion`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](assertion_state::Marker), so [`Assertion<S>`](Assertion) can only ever rest in a real state.
pub mod assertion_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `Assertion`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::AssertionState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::AssertionState = super::AssertionState::Recorded;
    }
}

/// Assertion — `ekr.graph.Assertion` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`AssertionSnapshot`]
/// and [`AssertionSnapshot::refine`].
pub struct Assertion<S: assertion_state::Marker> {
    data: AssertionData,
    state: core::marker::PhantomData<S>,
}

impl<S: assertion_state::Marker> Assertion<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> AssertionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &AssertionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> AssertionData {
        self.data
    }
}

impl Assertion<assertion_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: AssertionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.graph.Assertion` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`AssertionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: AssertionState,
    /// What it holds.
    pub data: AssertionData,
}

/// An `Assertion` in whichever declared state it was found.
pub enum AnyAssertion {
    /// Resting in `Recorded`.
    Recorded(Assertion<assertion_state::Recorded>),
}

impl AssertionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `AssertionState` cannot spell one.
    pub fn refine(self) -> AnyAssertion {
        match self.state {
            AssertionState::Recorded => AnyAssertion::Recorded(Assertion {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyAssertion {
    /// The state, as the runtime value.
    pub fn state(&self) -> AssertionState {
        match self {
            Self::Recorded(_) => AssertionState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> AssertionSnapshot {
        match self {
            Self::Recorded(instance) => AssertionSnapshot {
                state: AssertionState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What Edge — `ekr.graph.Edge` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Edge<S>`], and at a boundary by [`EdgeSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeData {
    /// The identity: `edge_id` — `ekr.graph.EdgeId`.
    pub edge_id: EdgeId,
    /// `root_id` — `ekr.graph.GraphRootId`.
    ///
    /// Carries `edges`: `ekr.graph.GraphRoot` owns many `ekr.graph.Edge`.
    pub root_id: GraphRootId,
    /// `type_id` — `ekr.ontology.TypeId`.
    ///
    /// Carries `type`: `ekr.graph.Edge` references one `ekr.ontology.EdgeType`.
    pub type_id: crate::ontology::TypeId,
    /// `source` — `ekr.graph.NodeId`.
    ///
    /// Carries `from`: `ekr.graph.Edge` references one `ekr.graph.Node`.
    pub source: NodeId,
    /// `target` — `ekr.graph.NodeId`.
    ///
    /// Carries `to`: `ekr.graph.Edge` references one `ekr.graph.Node`.
    pub target: NodeId,
    /// `properties` — `Map<String, List<ekr.graph.TypedValue>>`.
    pub properties: std::collections::BTreeMap<String, Vec<TypedValue>>,
}

/// The states of `ekr.graph.Edge`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](edge_state::Marker), so [`Edge<S>`](Edge) can only ever rest in a real state.
pub mod edge_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Present {}
    }

    /// A declared state of `Edge`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::EdgeState;
    }

    /// `Present`. Where a new instance starts.
    pub struct Present;

    impl Marker for Present {
        const STATE: super::EdgeState = super::EdgeState::Present;
    }
}

/// Edge — `ekr.graph.Edge` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Present`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`EdgeSnapshot`]
/// and [`EdgeSnapshot::refine`].
pub struct Edge<S: edge_state::Marker> {
    data: EdgeData,
    state: core::marker::PhantomData<S>,
}

impl<S: edge_state::Marker> Edge<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> EdgeState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &EdgeData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> EdgeData {
        self.data
    }
}

impl Edge<edge_state::Present> {
    /// A new instance, resting in `Present` — the only state the lifecycle starts one in.
    pub fn new(data: EdgeData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.graph.Edge` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`EdgeSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: EdgeState,
    /// What it holds.
    pub data: EdgeData,
}

/// An `Edge` in whichever declared state it was found.
pub enum AnyEdge {
    /// Resting in `Present`.
    Present(Edge<edge_state::Present>),
}

impl EdgeSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `EdgeState` cannot spell one.
    pub fn refine(self) -> AnyEdge {
        match self.state {
            EdgeState::Present => AnyEdge::Present(Edge {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyEdge {
    /// The state, as the runtime value.
    pub fn state(&self) -> EdgeState {
        match self {
            Self::Present(_) => EdgeState::Present,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> EdgeSnapshot {
        match self {
            Self::Present(instance) => EdgeSnapshot {
                state: EdgeState::Present,
                data: instance.into_data(),
            },
        }
    }
}

/// What Evidence — `ekr.graph.Evidence` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Evidence<S>`], and at a boundary by [`EvidenceSnapshot::state`].
///
/// Every value satisfies `confidence_bp >= 0` — checked by [`EvidenceData::broken_invariant`].
/// Every value satisfies `confidence_bp <= 10000` — checked by [`EvidenceData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceData {
    /// The identity: `evidence_id` — `ekr.graph.EvidenceId`.
    pub evidence_id: EvidenceId,
    /// `kind` — `ekr.graph.EvidenceKind`.
    pub kind: EvidenceKind,
    /// `locator` — `String`.
    pub locator: String,
    /// `section` — `Optional<String>`.
    pub section: Option<String>,
    /// `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: crate::kernel::ContentHash,
    /// `extracted_by` — `ekr.kernel.AgentId`.
    pub extracted_by: crate::kernel::AgentId,
    /// `observed_at` — `Timestamp`.
    pub observed_at: crate::primitives::Timestamp,
    /// `confidence_bp` — `Integer`.
    pub confidence_bp: i64,
    /// `observation_id` — `Optional<ekr.graph.ObservationId>`.
    pub observation_id: Option<ObservationId>,
}

impl EvidenceData {
    /// The first declared invariant of `ekr.graph.Evidence` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.confidence_bp)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("confidence_bp >= 0");
        }
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.confidence_bp)), iv::Op::Le, iv::Fact::number("10000"), false, true)) {
            return Some("confidence_bp <= 10000");
        }
        None
    }
}

/// The states of `ekr.graph.Evidence`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](evidence_state::Marker), so [`Evidence<S>`](Evidence) can only ever rest in a real state.
pub mod evidence_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Retained {}
    }

    /// A declared state of `Evidence`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::EvidenceState;
    }

    /// `Retained`. Where a new instance starts.
    pub struct Retained;

    impl Marker for Retained {
        const STATE: super::EvidenceState = super::EvidenceState::Retained;
    }
}

/// Evidence — `ekr.graph.Evidence` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Retained`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`EvidenceSnapshot`]
/// and [`EvidenceSnapshot::refine`].
pub struct Evidence<S: evidence_state::Marker> {
    data: EvidenceData,
    state: core::marker::PhantomData<S>,
}

impl<S: evidence_state::Marker> Evidence<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> EvidenceState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &EvidenceData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> EvidenceData {
        self.data
    }
}

impl Evidence<evidence_state::Retained> {
    /// A new instance, resting in `Retained` — the only state the lifecycle starts one in.
    pub fn new(data: EvidenceData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.graph.Evidence` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`EvidenceSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: EvidenceState,
    /// What it holds.
    pub data: EvidenceData,
}

/// An `Evidence` in whichever declared state it was found.
pub enum AnyEvidence {
    /// Resting in `Retained`.
    Retained(Evidence<evidence_state::Retained>),
}

impl EvidenceSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `EvidenceState` cannot spell one.
    pub fn refine(self) -> AnyEvidence {
        match self.state {
            EvidenceState::Retained => AnyEvidence::Retained(Evidence {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyEvidence {
    /// The state, as the runtime value.
    pub fn state(&self) -> EvidenceState {
        match self {
            Self::Retained(_) => EvidenceState::Retained,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> EvidenceSnapshot {
        match self {
            Self::Retained(instance) => EvidenceSnapshot {
                state: EvidenceState::Retained,
                data: instance.into_data(),
            },
        }
    }
}

/// What EvidenceAttachment — `ekr.graph.EvidenceAttachment` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`EvidenceAttachment<S>`], and at a boundary by [`EvidenceAttachmentSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceAttachmentData {
    /// The identity: `attachment_id` — `ekr.graph.AttachmentId`.
    pub attachment_id: AttachmentId,
    /// `assertion_id` — `ekr.graph.AssertionId`.
    ///
    /// Carries `attachments`: `ekr.graph.Assertion` owns many `ekr.graph.EvidenceAttachment`.
    pub assertion_id: AssertionId,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    ///
    /// Carries `evidence`: `ekr.graph.EvidenceAttachment` references one `ekr.graph.Evidence`.
    pub evidence_id: EvidenceId,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
}

/// The states of `ekr.graph.EvidenceAttachment`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](evidence_attachment_state::Marker), so [`EvidenceAttachment<S>`](EvidenceAttachment) can only ever rest in a real state.
pub mod evidence_attachment_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Attached {}
    }

    /// A declared state of `EvidenceAttachment`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::EvidenceAttachmentState;
    }

    /// `Attached`. Where a new instance starts.
    pub struct Attached;

    impl Marker for Attached {
        const STATE: super::EvidenceAttachmentState = super::EvidenceAttachmentState::Attached;
    }
}

/// EvidenceAttachment — `ekr.graph.EvidenceAttachment` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Attached`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`EvidenceAttachmentSnapshot`]
/// and [`EvidenceAttachmentSnapshot::refine`].
pub struct EvidenceAttachment<S: evidence_attachment_state::Marker> {
    data: EvidenceAttachmentData,
    state: core::marker::PhantomData<S>,
}

impl<S: evidence_attachment_state::Marker> EvidenceAttachment<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> EvidenceAttachmentState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &EvidenceAttachmentData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> EvidenceAttachmentData {
        self.data
    }
}

impl EvidenceAttachment<evidence_attachment_state::Attached> {
    /// A new instance, resting in `Attached` — the only state the lifecycle starts one in.
    pub fn new(data: EvidenceAttachmentData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.graph.EvidenceAttachment` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`EvidenceAttachmentSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceAttachmentSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: EvidenceAttachmentState,
    /// What it holds.
    pub data: EvidenceAttachmentData,
}

/// An `EvidenceAttachment` in whichever declared state it was found.
pub enum AnyEvidenceAttachment {
    /// Resting in `Attached`.
    Attached(EvidenceAttachment<evidence_attachment_state::Attached>),
}

impl EvidenceAttachmentSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `EvidenceAttachmentState` cannot spell one.
    pub fn refine(self) -> AnyEvidenceAttachment {
        match self.state {
            EvidenceAttachmentState::Attached => AnyEvidenceAttachment::Attached(EvidenceAttachment {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyEvidenceAttachment {
    /// The state, as the runtime value.
    pub fn state(&self) -> EvidenceAttachmentState {
        match self {
            Self::Attached(_) => EvidenceAttachmentState::Attached,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> EvidenceAttachmentSnapshot {
        match self {
            Self::Attached(instance) => EvidenceAttachmentSnapshot {
                state: EvidenceAttachmentState::Attached,
                data: instance.into_data(),
            },
        }
    }
}

/// What GraphRoot — `ekr.graph.GraphRoot` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`GraphRoot<S>`], and at a boundary by [`GraphRootSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRootData {
    /// The identity: `root_id` — `ekr.graph.GraphRootId`.
    pub root_id: GraphRootId,
    /// `space` — `ekr.graph.Space`.
    pub space: Space,
    /// `schema_version_id` — `ekr.ontology.SchemaVersionId`.
    pub schema_version_id: crate::ontology::SchemaVersionId,
    /// `parent` — `Optional<ekr.graph.GraphRootId>`.
    pub parent: Option<GraphRootId>,
    /// `created_at` — `Timestamp`.
    pub created_at: crate::primitives::Timestamp,
}

/// The states of `ekr.graph.GraphRoot`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](graph_root_state::Marker), so [`GraphRoot<S>`](GraphRoot) can only ever rest in a real state.
pub mod graph_root_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Active {}
    }

    /// A declared state of `GraphRoot`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::GraphRootState;
    }

    /// `Active`. Where a new instance starts.
    pub struct Active;

    impl Marker for Active {
        const STATE: super::GraphRootState = super::GraphRootState::Active;
    }
}

/// GraphRoot — `ekr.graph.GraphRoot` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Active`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`GraphRootSnapshot`]
/// and [`GraphRootSnapshot::refine`].
pub struct GraphRoot<S: graph_root_state::Marker> {
    data: GraphRootData,
    state: core::marker::PhantomData<S>,
}

impl<S: graph_root_state::Marker> GraphRoot<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> GraphRootState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &GraphRootData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> GraphRootData {
        self.data
    }
}

impl GraphRoot<graph_root_state::Active> {
    /// A new instance, resting in `Active` — the only state the lifecycle starts one in.
    pub fn new(data: GraphRootData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.graph.GraphRoot` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`GraphRootSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRootSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: GraphRootState,
    /// What it holds.
    pub data: GraphRootData,
}

/// An `GraphRoot` in whichever declared state it was found.
pub enum AnyGraphRoot {
    /// Resting in `Active`.
    Active(GraphRoot<graph_root_state::Active>),
}

impl GraphRootSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `GraphRootState` cannot spell one.
    pub fn refine(self) -> AnyGraphRoot {
        match self.state {
            GraphRootState::Active => AnyGraphRoot::Active(GraphRoot {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyGraphRoot {
    /// The state, as the runtime value.
    pub fn state(&self) -> GraphRootState {
        match self {
            Self::Active(_) => GraphRootState::Active,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> GraphRootSnapshot {
        match self {
            Self::Active(instance) => GraphRootSnapshot {
                state: GraphRootState::Active,
                data: instance.into_data(),
            },
        }
    }
}

/// What Node — `ekr.graph.Node` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Node<S>`], and at a boundary by [`NodeSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeData {
    /// The identity: `node_id` — `ekr.graph.NodeId`.
    pub node_id: NodeId,
    /// `root_id` — `ekr.graph.GraphRootId`.
    ///
    /// Carries `nodes`: `ekr.graph.GraphRoot` owns many `ekr.graph.Node`.
    pub root_id: GraphRootId,
    /// `type_id` — `ekr.ontology.TypeId`.
    ///
    /// Carries `type`: `ekr.graph.Node` references one `ekr.ontology.NodeType`.
    pub type_id: crate::ontology::TypeId,
    /// `canonical_name` — `String`.
    pub canonical_name: String,
    /// `aliases` — `List<String>`.
    pub aliases: Vec<String>,
    /// `type_state` — `Optional<String>`.
    pub type_state: Option<String>,
    /// `properties` — `Map<String, List<ekr.graph.TypedValue>>`.
    pub properties: std::collections::BTreeMap<String, Vec<TypedValue>>,
}

/// The states of `ekr.graph.Node`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](node_state::Marker), so [`Node<S>`](Node) can only ever rest in a real state.
pub mod node_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Present {}
    }

    /// A declared state of `Node`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::NodeState;
    }

    /// `Present`. Where a new instance starts.
    pub struct Present;

    impl Marker for Present {
        const STATE: super::NodeState = super::NodeState::Present;
    }
}

/// Node — `ekr.graph.Node` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Present`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`NodeSnapshot`]
/// and [`NodeSnapshot::refine`].
pub struct Node<S: node_state::Marker> {
    data: NodeData,
    state: core::marker::PhantomData<S>,
}

impl<S: node_state::Marker> Node<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> NodeState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &NodeData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> NodeData {
        self.data
    }
}

impl Node<node_state::Present> {
    /// A new instance, resting in `Present` — the only state the lifecycle starts one in.
    pub fn new(data: NodeData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.graph.Node` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`NodeSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: NodeState,
    /// What it holds.
    pub data: NodeData,
}

/// An `Node` in whichever declared state it was found.
pub enum AnyNode {
    /// Resting in `Present`.
    Present(Node<node_state::Present>),
}

impl NodeSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `NodeState` cannot spell one.
    pub fn refine(self) -> AnyNode {
        match self.state {
            NodeState::Present => AnyNode::Present(Node {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyNode {
    /// The state, as the runtime value.
    pub fn state(&self) -> NodeState {
        match self {
            Self::Present(_) => NodeState::Present,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> NodeSnapshot {
        match self {
            Self::Present(instance) => NodeSnapshot {
                state: NodeState::Present,
                data: instance.into_data(),
            },
        }
    }
}

/// What Observation — `ekr.graph.Observation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Observation<S>`], and at a boundary by [`ObservationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationData {
    /// The identity: `observation_id` — `ekr.graph.ObservationId`.
    pub observation_id: ObservationId,
    /// `source` — `String`.
    pub source: String,
    /// `source_native_id` — `Optional<String>`.
    pub source_native_id: Option<String>,
    /// `kind` — `ekr.graph.ObservationKind`.
    pub kind: ObservationKind,
    /// `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: crate::kernel::ContentHash,
    /// `captured_at` — `Timestamp`.
    pub captured_at: crate::primitives::Timestamp,
}

/// The states of `ekr.graph.Observation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](observation_state::Marker), so [`Observation<S>`](Observation) can only ever rest in a real state.
pub mod observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `Observation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::ObservationState = super::ObservationState::Recorded;
    }
}

/// Observation — `ekr.graph.Observation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ObservationSnapshot`]
/// and [`ObservationSnapshot::refine`].
pub struct Observation<S: observation_state::Marker> {
    data: ObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: observation_state::Marker> Observation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ObservationData {
        self.data
    }
}

impl Observation<observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: ObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.graph.Observation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ObservationState,
    /// What it holds.
    pub data: ObservationData,
}

/// An `Observation` in whichever declared state it was found.
pub enum AnyObservation {
    /// Resting in `Recorded`.
    Recorded(Observation<observation_state::Recorded>),
}

impl ObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ObservationState` cannot spell one.
    pub fn refine(self) -> AnyObservation {
        match self.state {
            ObservationState::Recorded => AnyObservation::Recorded(Observation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> ObservationState {
        match self {
            Self::Recorded(_) => ObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ObservationSnapshot {
        match self {
            Self::Recorded(instance) => ObservationSnapshot {
                state: ObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What Support — `ekr.graph.Support` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Support<S>`], and at a boundary by [`SupportSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportData {
    /// The identity: `support_id` — `ekr.graph.SupportId`.
    pub support_id: SupportId,
    /// `assertion_id` — `ekr.graph.AssertionId`.
    ///
    /// Carries `support`: `ekr.graph.Assertion` owns many `ekr.graph.Support`.
    pub assertion_id: AssertionId,
    /// `evidence_id` — `ekr.graph.EvidenceId`.
    ///
    /// Carries `evidence`: `ekr.graph.Support` references one `ekr.graph.Evidence`.
    pub evidence_id: EvidenceId,
}

/// The states of `ekr.graph.Support`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](support_state::Marker), so [`Support<S>`](Support) can only ever rest in a real state.
pub mod support_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Linked {}
    }

    /// A declared state of `Support`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SupportState;
    }

    /// `Linked`. Where a new instance starts.
    pub struct Linked;

    impl Marker for Linked {
        const STATE: super::SupportState = super::SupportState::Linked;
    }
}

/// Support — `ekr.graph.Support` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Linked`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SupportSnapshot`]
/// and [`SupportSnapshot::refine`].
pub struct Support<S: support_state::Marker> {
    data: SupportData,
    state: core::marker::PhantomData<S>,
}

impl<S: support_state::Marker> Support<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SupportState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SupportData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SupportData {
        self.data
    }
}

impl Support<support_state::Linked> {
    /// A new instance, resting in `Linked` — the only state the lifecycle starts one in.
    pub fn new(data: SupportData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.graph.Support` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SupportSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SupportState,
    /// What it holds.
    pub data: SupportData,
}

/// An `Support` in whichever declared state it was found.
pub enum AnySupport {
    /// Resting in `Linked`.
    Linked(Support<support_state::Linked>),
}

impl SupportSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SupportState` cannot spell one.
    pub fn refine(self) -> AnySupport {
        match self.state {
            SupportState::Linked => AnySupport::Linked(Support {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySupport {
    /// The state, as the runtime value.
    pub fn state(&self) -> SupportState {
        match self {
            Self::Linked(_) => SupportState::Linked,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SupportSnapshot {
        match self {
            Self::Linked(instance) => SupportSnapshot {
                state: SupportState::Linked,
                data: instance.into_data(),
            },
        }
    }
}

/// Assertions — one row of the view `ekr.graph.Assertions`.
///
/// Projects `ekr.graph.Assertion` at `read_your_writes` consistency.
/// Serving it is an implementation obligation — see the plan — because how a projection is kept
/// current is a storage decision the specification does not take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assertions {
    /// `assertion_id` — `ekr.graph.AssertionId`.
    pub assertion_id: AssertionId,
    /// `root_id` — `ekr.graph.GraphRootId`.
    pub root_id: GraphRootId,
    /// `assessment` — `ekr.graph.AssessmentProjection`.
    pub assessment: AssessmentProjection,
    /// `lifecycle` — `ekr.graph.AssertionLifecycleProjection`.
    pub lifecycle: AssertionLifecycleProjection,
    /// `recorded_from` — `Timestamp`.
    pub recorded_from: crate::primitives::Timestamp,
}

/// Settled active assertions — one row of the view `ekr.graph.SettledAssertions`.
///
/// Projects `ekr.graph.Assertion` at `read_your_writes` consistency, containing instances where `(assessment.kind == Accepted and lifecycle.kind == Active)`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettledAssertions {
    /// `assertion_id` — `ekr.graph.AssertionId`.
    pub assertion_id: AssertionId,
    /// `root_id` — `ekr.graph.GraphRootId`.
    pub root_id: GraphRootId,
    /// `subject_kind` — `ekr.graph.SubjectKind`.
    pub subject_kind: SubjectKind,
    /// `subject` — `Uuid`.
    pub subject: crate::primitives::Uuid,
    /// `predicate_kind` — `ekr.graph.PredicateKind`.
    pub predicate_kind: PredicateKind,
    /// `predicate` — `Uuid`.
    pub predicate: crate::primitives::Uuid,
    /// `object_kind` — `ekr.graph.ObjectKind`.
    pub object_kind: ObjectKind,
    /// `object_value` — `Optional<ekr.graph.TypedValue>`.
    pub object_value: Option<TypedValue>,
    /// `object_ref` — `Optional<Uuid>`.
    pub object_ref: Option<crate::primitives::Uuid>,
    /// `proposed_by` — `ekr.kernel.AgentId`.
    pub proposed_by: crate::kernel::AgentId,
    /// `assessment` — `ekr.graph.AssessmentProjection`.
    pub assessment: AssessmentProjection,
    /// `lifecycle` — `ekr.graph.AssertionLifecycleProjection`.
    pub lifecycle: AssertionLifecycleProjection,
    /// `valid_from` — `Optional<Timestamp>`.
    pub valid_from: Option<crate::primitives::Timestamp>,
    /// `valid_to` — `Optional<Timestamp>`.
    pub valid_to: Option<crate::primitives::Timestamp>,
    /// `recorded_from` — `Timestamp`.
    pub recorded_from: crate::primitives::Timestamp,
    /// `recorded_to` — `Optional<Timestamp>`.
    pub recorded_to: Option<crate::primitives::Timestamp>,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The query `ekr.graph.Assertions` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by an order over the `Timestamp` field `recorded_from`, which the suite ranks by its text and not by its instant.
    ///
    /// Contract: a query answering `ekr.graph.Assertions` with rows projected from `ekr.graph.Assertion` at `read_your_writes` consistency.
    pub trait AssertionsQuery {
        /// Serves `ekr.graph.Assertions` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn assertions(&self) -> Result<Vec<super::Assertions>, crate::obligation::UnmetObligation>;
    }

    /// The query `ekr.graph.SettledAssertions` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SettledAssertionsQuery {
        /// Serves `ekr.graph.SettledAssertions` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn settled_assertions(&self) -> Result<Vec<super::SettledAssertions>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl AssertionsQuery for Unimplemented {
        fn assertions(&self) -> Result<Vec<super::Assertions>, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "view query", source: "ekr.graph.Assertions" })
        }
    }
}
