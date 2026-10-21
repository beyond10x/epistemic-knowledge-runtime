//! `ekr.transaction-document/2`: one transaction and its operations (`docs/cli.md`,
//! "Transaction documents" and "Operation kinds").

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, EdgeId, EvidenceId, NodeId, PropertyId, SchemaVersionId, Timestamp,
    TransactionId, TypeId,
};
use serde::{Deserialize, Serialize};

use super::graph::{Assertion, Confidence, EdgeDraft, Evidence, EvidenceSource, NodeDraft};
use super::value::{EdgeType, NodeType, PropertyDefinition, Value};
use super::{DocumentError, ValidationProfile};

/// The format every transaction document the SDK writes declares.
pub const TRANSACTION_FORMAT: &str = "ekr.transaction-document/2";

/// An `ekr.transaction-document/2` document: what `ekr propose` reads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionDocument {
    /// [`TRANSACTION_FORMAT`].
    pub format: String,
    /// The proposal.
    pub transaction: Transaction,
}

impl TransactionDocument {
    /// Reads `text` as `ekr propose` reads it in a store under `profile`: under
    /// [`ValidationProfile::V4`] a `!SupersedeNode` listing one evidence id twice is refused, and
    /// under v1 to v3 it reads as naming the id once, as a plain [`serde`] read of this type does
    /// (ADR 0102 decision 5).
    ///
    /// # Errors
    /// The reader's refusal: invalid YAML, an unknown or missing field, a value of the wrong
    /// shape, and the duplicate evidence id above.
    pub fn from_yaml(text: &str, profile: ValidationProfile) -> Result<Self, serde_yaml_ng::Error> {
        ekr_core::decode::with_strict_sets(profile.supersedes_exactly(), || {
            serde_yaml_ng::from_str(text)
        })
    }

    /// The document as YAML, every variant written as a tag.
    ///
    /// # Errors
    /// [`DocumentError::Yaml`] when the writer refuses a value.
    pub fn to_yaml(&self) -> Result<String, DocumentError> {
        super::to_yaml(self)
    }

    /// Whether the document, as [`Self::to_yaml`] writes it, is within the frozen limits of
    /// `ekr.transaction-document/2` ([`TRANSACTION_LIMITS`](super::TRANSACTION_LIMITS)).
    /// [`TransactionBuilder::build`] runs it; a document assembled by hand should too.
    ///
    /// # Errors
    /// [`DocumentError::Limit`] naming the first limit it is past, its bound and the document's
    /// value, and [`DocumentError::Yaml`] when the writer refuses a value.
    pub fn check_limits(&self) -> Result<(), DocumentError> {
        super::limits::check(&self.to_yaml()?)
    }
}

/// One proposed transaction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transaction {
    /// Its id, fresh for every proposal.
    pub id: TransactionId,
    /// The host operator.
    pub proposer: AgentId,
    /// One or more operations, applied all or nothing.
    pub operations: Vec<Operation>,
    /// Exactly the evidence ids its `!AddAssertion`s cite, its `!AttachEvidence`s attach, its
    /// `!CiteEvidence`s cite and its `!SupersedeNode`s and `!MergeEntity`s rest on.
    pub evidence: BTreeSet<EvidenceId>,
    /// The schema version a schema change produces; absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<SchemaVersionId>,
}

/// Builds a [`TransactionDocument`] whose bookkeeping cannot disagree with its operations: the
/// evidence manifest is the set its operations rest on ([`Operation::rests_on`]), and a schema
/// change names a freshly minted schema version.
#[derive(Clone, Debug)]
pub struct TransactionBuilder {
    id: TransactionId,
    proposer: AgentId,
    operations: Vec<Operation>,
    schema_version: Option<SchemaVersionId>,
}

impl TransactionBuilder {
    /// A transaction `proposer` proposes, under a freshly minted id.
    #[must_use]
    pub fn new(proposer: AgentId) -> Self {
        Self {
            id: TransactionId::mint(),
            proposer,
            operations: Vec::new(),
            schema_version: None,
        }
    }

    /// This transaction under `id` rather than a minted one.
    #[must_use]
    pub fn with_id(mut self, id: TransactionId) -> Self {
        self.id = id;
        self
    }

    /// This transaction producing schema version `id` rather than a minted one.
    #[must_use]
    pub fn with_schema_version(mut self, id: SchemaVersionId) -> Self {
        self.schema_version = Some(id);
        self
    }

    /// This transaction with one more operation, after the others.
    #[must_use]
    pub fn push(mut self, operation: Operation) -> Self {
        self.operations.push(operation);
        self
    }

    /// The document.
    ///
    /// # Errors
    /// [`DocumentError::EmptyTransaction`] with no operation,
    /// [`DocumentError::CitationWithoutSchemaChange`] for a `!CiteEvidence` with no schema change
    /// beside it, [`DocumentError::MixedSchemaTransaction`] when schema and data operations are
    /// mixed, and
    /// [`DocumentError::Limit`] for a document past one of the format's frozen limits
    /// ([`TransactionDocument::check_limits`]).
    pub fn build(self) -> Result<TransactionDocument, DocumentError> {
        if self.operations.is_empty() {
            return Err(DocumentError::EmptyTransaction);
        }
        let schema = self
            .operations
            .iter()
            .filter(|operation| operation.is_schema_change())
            .count();
        // A `!CiteEvidence`, and the `!AddEvidence` of an entry one cites, travel with schema
        // changes (design § 109); anything else beside them is mixed.
        let cited: BTreeSet<EvidenceId> = self
            .operations
            .iter()
            .filter_map(|operation| match operation {
                Operation::CiteEvidence(evidence) => Some(*evidence),
                _ => None,
            })
            .collect();
        let travelling = self
            .operations
            .iter()
            .filter(|operation| match operation {
                Operation::CiteEvidence(_) => true,
                Operation::AddEvidence(addition) => cited.contains(&addition.evidence.id),
                _ => false,
            })
            .count();
        if schema == 0 && !cited.is_empty() {
            return Err(DocumentError::CitationWithoutSchemaChange);
        }
        if schema != 0 && schema + travelling != self.operations.len() {
            return Err(DocumentError::MixedSchemaTransaction);
        }
        let evidence = self
            .operations
            .iter()
            .flat_map(Operation::rests_on)
            .collect();
        let schema_version =
            (schema != 0).then(|| self.schema_version.unwrap_or_else(SchemaVersionId::mint));
        let document = TransactionDocument {
            format: TRANSACTION_FORMAT.to_owned(),
            transaction: Transaction {
                id: self.id,
                proposer: self.proposer,
                operations: self.operations,
                evidence,
                schema_version,
            },
        };
        document.check_limits()?;
        Ok(document)
    }
}

/// One operation, written as its kind's tag. Every kind `ekr operations` lists is a variant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Operation {
    /// `!CreateNode`.
    CreateNode(NodeDraft),
    /// `!UpdateProperty`.
    UpdateProperty(PropertyMutation),
    /// `!CreateEdge`.
    CreateEdge(EdgeDraft),
    /// `!DeleteEdge <edge id>`.
    DeleteEdge(EdgeId),
    /// `!AddAssertion`.
    AddAssertion(Box<Assertion>),
    /// `!RetractAssertion`.
    RetractAssertion(Retraction),
    /// `!DefineNodeType`, a schema change.
    DefineNodeType(Box<NodeType>),
    /// `!DefineEdgeType`, a schema change.
    DefineEdgeType(Box<EdgeType>),
    /// `!ModifyProperty`, a schema change.
    ModifyProperty(PropertyModification),
    /// `!MergeEntity`: a duplicate node absorbed into the node it duplicates, every reference to
    /// it rewritten to that node, its evidence in the transaction's `evidence` (design § 111).
    MergeEntity(EntityMerge),
    /// `!Invoke`.
    Invoke(Invocation),
    /// `!SupersedeAssertion`.
    SupersedeAssertion(Supersession),
    /// `!AddEvidence`.
    AddEvidence(Box<EvidenceAddition>),
    /// `!WidenEdgeType`, a schema change.
    WidenEdgeType(EdgeWidening),
    /// `!AddAlias`: one more alias for a node that exists.
    AddAlias(AliasAddition),
    /// `!AttachEvidence`: evidence attached to an assertion the store holds.
    AttachEvidence(EvidenceAttachment),
    /// `!CiteEvidence <evidence id>`: evidence the schema version a schema change makes rests on,
    /// beside schema changes only (design § 109).
    CiteEvidence(EvidenceId),
    /// `!SupersedeNode`: a duplicate node retired in favour of the node that replaces it, its
    /// evidence in the transaction's `evidence` (design § 110).
    SupersedeNode(NodeSupersession),
}

impl Operation {
    /// Its kind.
    #[must_use]
    pub const fn kind(&self) -> OperationKind {
        match self {
            Self::CreateNode(_) => OperationKind::CreateNode,
            Self::UpdateProperty(_) => OperationKind::UpdateProperty,
            Self::CreateEdge(_) => OperationKind::CreateEdge,
            Self::DeleteEdge(_) => OperationKind::DeleteEdge,
            Self::AddAssertion(_) => OperationKind::AddAssertion,
            Self::RetractAssertion(_) => OperationKind::RetractAssertion,
            Self::DefineNodeType(_) => OperationKind::DefineNodeType,
            Self::DefineEdgeType(_) => OperationKind::DefineEdgeType,
            Self::ModifyProperty(_) => OperationKind::ModifyProperty,
            Self::MergeEntity(_) => OperationKind::MergeEntity,
            Self::Invoke(_) => OperationKind::Invoke,
            Self::SupersedeAssertion(_) => OperationKind::SupersedeAssertion,
            Self::AddEvidence(_) => OperationKind::AddEvidence,
            Self::WidenEdgeType(_) => OperationKind::WidenEdgeType,
            Self::AddAlias(_) => OperationKind::AddAlias,
            Self::AttachEvidence(_) => OperationKind::AttachEvidence,
            Self::CiteEvidence(_) => OperationKind::CiteEvidence,
            Self::SupersedeNode(_) => OperationKind::SupersedeNode,
        }
    }

    /// The evidence ids it rests on: those an `!AddAssertion` cites, the one an
    /// `!AttachEvidence` attaches, the one a `!CiteEvidence` cites, those a `!SupersedeNode` or a
    /// `!MergeEntity` rests on, and none for any other kind.
    /// A transaction's `evidence` is exactly the union of these over its operations.
    #[must_use]
    pub fn rests_on(&self) -> Vec<EvidenceId> {
        match self {
            Self::AddAssertion(assertion) => assertion.evidence.iter().copied().collect(),
            Self::AttachEvidence(attachment) => vec![attachment.evidence],
            Self::CiteEvidence(evidence) => vec![*evidence],
            Self::SupersedeNode(supersession) => supersession.evidence.iter().copied().collect(),
            Self::MergeEntity(merge) => merge.evidence.iter().flatten().copied().collect(),
            _ => Vec::new(),
        }
    }

    /// Whether it is a schema change, which travels in a transaction of its own.
    #[must_use]
    pub const fn is_schema_change(&self) -> bool {
        self.kind().is_schema_change()
    }
}

/// The kind of an [`Operation`]: its YAML tag without `!`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperationKind {
    /// `!CreateNode`.
    CreateNode,
    /// `!UpdateProperty`.
    UpdateProperty,
    /// `!CreateEdge`.
    CreateEdge,
    /// `!DeleteEdge`.
    DeleteEdge,
    /// `!AddAssertion`.
    AddAssertion,
    /// `!RetractAssertion`.
    RetractAssertion,
    /// `!DefineNodeType`.
    DefineNodeType,
    /// `!DefineEdgeType`.
    DefineEdgeType,
    /// `!ModifyProperty`.
    ModifyProperty,
    /// `!MergeEntity`.
    MergeEntity,
    /// `!Invoke`.
    Invoke,
    /// `!SupersedeAssertion`.
    SupersedeAssertion,
    /// `!AddEvidence`.
    AddEvidence,
    /// `!WidenEdgeType`.
    WidenEdgeType,
    /// `!AddAlias`.
    AddAlias,
    /// `!AttachEvidence`.
    AttachEvidence,
    /// `!CiteEvidence`.
    CiteEvidence,
    /// `!SupersedeNode`.
    SupersedeNode,
}

impl OperationKind {
    /// Every kind, in the order `ekr operations` lists them.
    pub const ALL: [Self; 18] = [
        Self::CreateNode,
        Self::UpdateProperty,
        Self::CreateEdge,
        Self::DeleteEdge,
        Self::AddAssertion,
        Self::RetractAssertion,
        Self::DefineNodeType,
        Self::DefineEdgeType,
        Self::ModifyProperty,
        Self::MergeEntity,
        Self::Invoke,
        Self::SupersedeAssertion,
        Self::AddEvidence,
        Self::WidenEdgeType,
        Self::AddAlias,
        Self::AttachEvidence,
        Self::CiteEvidence,
        Self::SupersedeNode,
    ];

    /// Its tag without `!`, as `ekr operations` prints it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::CreateNode => "CreateNode",
            Self::UpdateProperty => "UpdateProperty",
            Self::CreateEdge => "CreateEdge",
            Self::DeleteEdge => "DeleteEdge",
            Self::AddAssertion => "AddAssertion",
            Self::RetractAssertion => "RetractAssertion",
            Self::DefineNodeType => "DefineNodeType",
            Self::DefineEdgeType => "DefineEdgeType",
            Self::ModifyProperty => "ModifyProperty",
            Self::MergeEntity => "MergeEntity",
            Self::Invoke => "Invoke",
            Self::SupersedeAssertion => "SupersedeAssertion",
            Self::AddEvidence => "AddEvidence",
            Self::WidenEdgeType => "WidenEdgeType",
            Self::AddAlias => "AddAlias",
            Self::AttachEvidence => "AttachEvidence",
            Self::CiteEvidence => "CiteEvidence",
            Self::SupersedeNode => "SupersedeNode",
        }
    }

    /// Whether the kind is a schema change: applied under profile v2, v3 or v4 only, in a
    /// transaction of schema changes that names its `schema_version`.
    #[must_use]
    pub const fn is_schema_change(self) -> bool {
        matches!(
            self,
            Self::DefineNodeType
                | Self::DefineEdgeType
                | Self::ModifyProperty
                | Self::WidenEdgeType
        )
    }
}

/// `!UpdateProperty`: every value of one property of one node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyMutation {
    /// The node.
    pub node: NodeId,
    /// The property.
    pub property: PropertyId,
    /// Its values afterwards; empty clears it.
    pub values: Vec<Value>,
}

impl PropertyMutation {
    /// Set `property` of `node` to `values`.
    #[must_use]
    pub const fn new(node: NodeId, property: PropertyId, values: Vec<Value>) -> Self {
        Self {
            node,
            property,
            values,
        }
    }
}

/// `!RetractAssertion`: withdraw an accepted, active assertion, with a reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retraction {
    /// The assertion.
    pub assertion: AssertionId,
    /// Why.
    pub reason: String,
}

impl Retraction {
    /// Retract `assertion` because of `reason`.
    #[must_use]
    pub fn new(assertion: AssertionId, reason: impl Into<String>) -> Self {
        Self {
            assertion,
            reason: reason.into(),
        }
    }
}

/// `!SupersedeAssertion`: replace an accepted, active assertion from an instant on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supersession {
    /// The assertion replaced.
    pub assertion: AssertionId,
    /// Its replacement, whose valid time starts exactly at `effective_from`.
    pub by: AssertionId,
    /// The instant the replacement takes over.
    pub effective_from: Timestamp,
}

impl Supersession {
    /// `by` replaces `assertion` from `effective_from` on.
    #[must_use]
    pub const fn new(assertion: AssertionId, by: AssertionId, effective_from: Timestamp) -> Self {
        Self {
            assertion,
            by,
            effective_from,
        }
    }
}

/// `!AddEvidence`: one evidence entry and the exact bytes it rests on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAddition {
    /// The entry; its `content_hash` is the payload's.
    pub evidence: Evidence,
    /// The bytes, written as one list element each, so at most `sequence_elements` (16,384) of
    /// them; [`TransactionBuilder::build`] refuses more by that name. A larger statement goes
    /// into the seed, or is split into several entries.
    pub payload: Vec<u8>,
}

impl EvidenceAddition {
    /// The entry for `payload`, hashed here, under a freshly minted evidence id.
    #[must_use]
    pub fn new(
        source: EvidenceSource,
        extracted_by: AgentId,
        observed_at: Timestamp,
        confidence: Confidence,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            evidence: Evidence::for_payload(
                &payload,
                source,
                extracted_by,
                observed_at,
                confidence,
            ),
            payload,
        }
    }
}

/// `!MergeEntity`: a duplicate node absorbed into the node it duplicates, in a revision of its own
/// (design § 111). The absorbed node leaves the head into a supersession record naming `into`, as a
/// `!SupersedeNode` retires a node, and every edge, assertion and `NodeRef` naming it names `into`
/// instead, keeping its id; `ekr snapshot` lists the record, with the lineage of what moved, under
/// `superseded`. The evidence is in the transaction's `evidence`.
///
/// Written without `reason` and `evidence`, it is the bare P1 shape, which the kernel parses and
/// refuses under every profile as `unsupported-operation`: [`EntityMerge::bare`] builds it, so a
/// document carrying one round-trips; [`EntityMerge::new`] builds the shape that applies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityMerge {
    /// The node that stops being its own entity: one the store holds and has not superseded.
    pub absorbed: NodeId,
    /// The node that keeps its id and its type: one the store holds, or one a `!CreateNode` of
    /// the same transaction creates, of `absorbed`'s type or one related to it through `parents`.
    pub into: NodeId,
    /// Why: not empty, and not only whitespace. Absent only in the bare P1 shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// At least one evidence id: retained, or added by an `!AddEvidence` of the same transaction.
    /// Absent only in the bare P1 shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<BTreeSet<EvidenceId>>,
}

impl EntityMerge {
    /// Merge `absorbed` into `into`, for `reason`, on `evidence`.
    #[must_use]
    pub fn new(
        absorbed: NodeId,
        into: NodeId,
        reason: impl Into<String>,
        evidence: impl IntoIterator<Item = EvidenceId>,
    ) -> Self {
        Self {
            absorbed,
            into,
            reason: Some(reason.into()),
            evidence: Some(evidence.into_iter().collect()),
        }
    }

    /// The bare P1 shape `{absorbed, into}`, which every profile refuses.
    #[must_use]
    pub const fn bare(absorbed: NodeId, into: NodeId) -> Self {
        Self {
            absorbed,
            into,
            reason: None,
            evidence: None,
        }
    }
}

/// `!AddAlias`: one more alias for a node that exists, which `ekr resolve` finds it by from the
/// next revision on. Nothing removes an alias.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AliasAddition {
    /// The node: one the store holds, or one a `!CreateNode` of the same transaction creates.
    pub node: NodeId,
    /// The alias: not empty, and held by no node of the node's type.
    pub alias: String,
}

impl AliasAddition {
    /// Give `node` the alias `alias`.
    #[must_use]
    pub fn new(node: NodeId, alias: impl Into<String>) -> Self {
        Self {
            node,
            alias: alias.into(),
        }
    }
}

/// `!AttachEvidence`: evidence attached to an assertion the store holds, accepted and active. The
/// assertion is not changed; the attachment is a record of its own, which `ekr explain` lists from
/// the revision that made it on. The evidence is one the store retains or an `!AddEvidence` of the
/// same transaction adds, and is in the transaction's `evidence`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAttachment {
    /// The assertion: one the store holds (`ekr snapshot`), accepted and active.
    pub assertion: AssertionId,
    /// The evidence: retained, or added by an `!AddEvidence` of the same transaction; not one the
    /// assertion cites or has attached.
    pub evidence: EvidenceId,
}

impl EvidenceAttachment {
    /// Attach `evidence` to `assertion`.
    #[must_use]
    pub const fn new(assertion: AssertionId, evidence: EvidenceId) -> Self {
        Self {
            assertion,
            evidence,
        }
    }
}

/// `!SupersedeNode`: a duplicate node retired in favour of the node that replaces it (design
/// § 110). The node and every edge at it leave the head and are kept in a supersession record
/// naming `by`, which `ekr snapshot` lists under `superseded`; its aliases resolve to `by`. The
/// evidence is in the transaction's `evidence`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeSupersession {
    /// The node retired: one the store holds and has not superseded.
    pub node: NodeId,
    /// The node kept: one the store holds, or one a `!CreateNode` of the same transaction
    /// creates; not `node`, and not superseded.
    pub by: NodeId,
    /// Why: not empty, and not only whitespace.
    pub reason: String,
    /// At least one evidence id: retained, or added by an `!AddEvidence` of the same transaction.
    /// A document listing one id twice is read as listing it once, as profiles v1 to v3 read it,
    /// and refused by [`TransactionDocument::from_yaml`] under profile v4, as `ekr propose` refuses
    /// it in a v4 store.
    #[serde(deserialize_with = "ekr_core::decode::profiled_set")]
    pub evidence: BTreeSet<EvidenceId>,
}

impl NodeSupersession {
    /// Supersede `node` by `by`, for `reason`, on `evidence`.
    #[must_use]
    pub fn new(
        node: NodeId,
        by: NodeId,
        reason: impl Into<String>,
        evidence: impl IntoIterator<Item = EvidenceId>,
    ) -> Self {
        Self {
            node,
            by,
            reason: reason.into(),
            evidence: evidence.into_iter().collect(),
        }
    }
}

/// `!WidenEdgeType`: an edge type's ends written whole, as they are to be.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeWidening {
    /// The edge type.
    pub edge_type: TypeId,
    /// Every source type it has now, and the ones added.
    pub source_types: BTreeSet<TypeId>,
    /// Every target type it has now, and the ones added.
    pub target_types: BTreeSet<TypeId>,
}

impl EdgeWidening {
    /// Widen `edge_type` to these ends.
    #[must_use]
    pub fn new(
        edge_type: TypeId,
        source_types: impl IntoIterator<Item = TypeId>,
        target_types: impl IntoIterator<Item = TypeId>,
    ) -> Self {
        Self {
            edge_type,
            source_types: source_types.into_iter().collect(),
            target_types: target_types.into_iter().collect(),
        }
    }
}

/// `!ModifyProperty`: add a property to a type, or redeclare one it declares, in the
/// `{owner, property}` shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyModification {
    /// The node type or edge type that declares it.
    pub owner: TypeId,
    /// The whole declaration, under its own id.
    pub property: PropertyDefinition,
}

impl PropertyModification {
    /// Declare `property` on `owner`.
    #[must_use]
    pub const fn new(owner: TypeId, property: PropertyDefinition) -> Self {
        Self { owner, property }
    }
}

/// `!Invoke`: call an operation of the node's type, by its key.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    /// The node.
    pub node: NodeId,
    /// The operation's key under its type's `operations`.
    pub operation: String,
    /// Every argument, by name.
    pub arguments: BTreeMap<String, Value>,
}

impl Invocation {
    /// Call `operation` on `node` with no arguments yet.
    #[must_use]
    pub fn new(node: NodeId, operation: impl Into<String>) -> Self {
        Self {
            node,
            operation: operation.into(),
            arguments: BTreeMap::new(),
        }
    }

    /// This call passing one more argument.
    #[must_use]
    pub fn with_argument(mut self, name: impl Into<String>, value: Value) -> Self {
        self.arguments.insert(name.into(), value);
        self
    }
}

/// `Operation::from(payload)` for every payload type, so a builder reads `.push(draft.into())`.
macro_rules! operation_from {
    ($($payload:ty => $variant:ident($wrap:expr)),+ $(,)?) => {
        $(impl From<$payload> for Operation {
            fn from(payload: $payload) -> Self {
                Self::$variant($wrap(payload))
            }
        })+
    };
}

operation_from! {
    NodeDraft => CreateNode(std::convert::identity),
    PropertyMutation => UpdateProperty(std::convert::identity),
    EdgeDraft => CreateEdge(std::convert::identity),
    Assertion => AddAssertion(Box::new),
    Retraction => RetractAssertion(std::convert::identity),
    NodeType => DefineNodeType(Box::new),
    EdgeType => DefineEdgeType(Box::new),
    PropertyModification => ModifyProperty(std::convert::identity),
    EntityMerge => MergeEntity(std::convert::identity),
    Invocation => Invoke(std::convert::identity),
    Supersession => SupersedeAssertion(std::convert::identity),
    EvidenceAddition => AddEvidence(Box::new),
    EdgeWidening => WidenEdgeType(std::convert::identity),
    AliasAddition => AddAlias(std::convert::identity),
    EvidenceAttachment => AttachEvidence(std::convert::identity),
    NodeSupersession => SupersedeNode(std::convert::identity),
}
