//! Proposals and the one type that may commit: design § 19, and amendment 87's `Invoke`.
//!
//! > No agent receives direct canonical mutation privileges. Agents propose graph transactions.
//!
//! A [`GraphTransaction`] is a proposal and nothing more. [`ValidatedTransaction`] is what the
//! pipeline of [`crate::validate`] produces from one, and design § 19 says what it is for: "Only a
//! `ValidatedTransaction` may be committed. This uses Rust's type system as part of the integrity
//! boundary." AGENTS.md invariant 1 says the same thing from the other end — only `ekr-kernel`
//! constructs one — so its fields are private, it derives no `Deserialize`, and
//! `tests/compile_fail/` holds both of those as build failures rather than as sentences.
//!
//! # Why a transaction is generic over its value
//!
//! `architecture-decision-record:0005-float-is-not-canonical`, as amended, and the same shape
//! `ekr-graph` took under it. A proposal carries [`ekr_ontology::Value`], which has a `Float`
//! variant: an agent may propose an approximate measurement, and the runtime has to be able to
//! *hold* that proposal in order to refuse it — a defect that cannot be represented cannot be
//! reported, and the story this file implements is asked to report it with a path into the value.
//!
//! A [`ValidatedTransaction`] holds `GraphTransaction<CanonicalValue>`, and
//! [`Canonical`] is implemented only where the value parameter is. So the `validation_hash` exists
//! exactly where a validated transaction does, by construction rather than by a rule: there is no
//! encoding of a proposal that has not been through the type validator, and no fallible hash whose
//! error some other layer is trusted to have made unreachable.
//!
//! The default parameter is `Value`, where `ekr_graph`'s three types default to `CanonicalValue`.
//! The defaults differ because the common case differs: `Node` on its own is canonical state, and
//! `GraphTransaction` on its own is a proposal.
//!
//! # The ontology types are encoded here, by hand
//!
//! Three operations carry a schema declaration — `DefineNodeType`, `DefineEdgeType`,
//! `ModifyProperty` — and `ekr_ontology` implements no [`Canonical`]. The kernel cannot implement
//! a foreign trait for a foreign type, so the encodings live in this file as local newtypes over a
//! borrowed declaration. `crates/ekr-kernel/tests/validation.rs` holds every `pub` field of every
//! one of those types to appearing in this file's encoding, so a field added upstream cannot
//! quietly fall out of the `validation_hash`.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionNumber, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, CanonicalRef, CanonicalValue, InadmissibleValue, Object, RetractionReason, Subject,
    ValueSpace,
};
use ekr_ontology::{EdgeType, NodeType, PropertyDefinition, Value};
use serde::{Deserialize, Serialize};

/// A node an operation proposes to create: the `NodeDraft` of design § 19.
///
/// Properties are a `Vec` per property, where [`ekr_graph::Node`] holds one value: a draft is
/// what was *proposed*, and a proposal that carries three values for a single-valued property is
/// exactly what the cardinality validator exists to refuse. A draft that cannot express the
/// violation cannot be refused for it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeDraft<V = Value> {
    /// The id the node will have. Minted by the proposer; identity is not derived from content.
    pub id: NodeId,
    /// The graph root it belongs to.
    pub root_id: GraphRootId,
    /// The node type it claims to be an instance of.
    pub type_id: TypeId,
    /// The name a reader will see. A property, not an identity (AGENTS.md invariant 3).
    pub canonical_name: String,
    /// The property values it arrives with, by the property's id.
    #[serde(
        deserialize_with = "ekr_core::decode::unique_map",
        bound(deserialize = "V: Deserialize<'de>")
    )]
    pub properties: BTreeMap<PropertyId, Vec<V>>,
}

/// A change to one property of one node: the `PropertyMutation` of design § 19.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyMutation<V = Value> {
    /// The node whose property moves.
    pub node: NodeId,
    /// The property.
    pub property: PropertyId,
    /// Its values afterwards. Empty clears the property, which a required property forbids.
    pub values: Vec<V>,
}

/// An edge an operation proposes to create: the `EdgeDraft` of design § 19.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeDraft<V = Value> {
    /// The id the edge will have.
    pub id: EdgeId,
    /// The graph root it belongs to.
    pub root_id: GraphRootId,
    /// The edge type it is an instance of.
    pub type_id: TypeId,
    /// The node the relation runs from.
    pub source: NodeId,
    /// The node it runs to.
    pub target: NodeId,
    /// Its property values, by the property's id.
    #[serde(
        deserialize_with = "ekr_core::decode::unique_map",
        bound(deserialize = "V: Deserialize<'de>")
    )]
    pub properties: BTreeMap<PropertyId, Vec<V>>,
}

/// Two nodes the proposer holds to be one: the `EntityMerge` of design § 19.
///
/// Which record survives is not a detail: design § 6.4 makes an id permanent, so a merge names
/// the id that remains and the id that becomes an alias of it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityMerge {
    /// The node that stops being its own entity.
    pub absorbed: NodeId,
    /// The node it is absorbed into, which keeps its id.
    pub into: NodeId,
}

/// Reasoned withdrawal, encoded at the original operation index five.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retraction {
    /// Assertion to withdraw.
    pub assertion: AssertionId,
    /// Retained statement of why.
    pub reason: RetractionReason,
}
/// Supported replacement at a half-open valid-time boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supersession {
    /// Earlier assertion.
    pub assertion: AssertionId,
    /// Accepted replacement, possibly added in the same unordered transaction.
    pub by: AssertionId,
    /// Replacement's required valid-time start.
    pub effective_from: ekr_core::Timestamp,
}
impl Canonical for Retraction {
    fn encode(&self, out: &mut Encoder) {
        self.assertion.encode(out);
        self.reason.encode(out);
    }
}
impl Canonical for Supersession {
    fn encode(&self, out: &mut Encoder) {
        self.assertion.encode(out);
        self.by.encode(out);
        self.effective_from.encode(out);
    }
}

/// One change a transaction proposes: design § 19, plus `Invoke` from amendment 87.
///
/// Eleven variants, which are the eleven `ekr.kernel.OperationKind` names of
/// `systems/ekr/domains/kernel.yaml`, in that order. The order is the encoding's contract: the
/// variant number is what separates two operations carrying the same payload shape, and moving a
/// number moves every `validation_hash` that contains the variant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum GraphOperation<V: ValueSpace = Value> {
    /// Create a node.
    CreateNode(NodeDraft<V>),
    /// Set the values of one property of one node.
    UpdateProperty(PropertyMutation<V>),
    /// Create an edge.
    CreateEdge(EdgeDraft<V>),
    /// Remove an edge. The claim that the relation held is an assertion and is retracted, not
    /// deleted; the edge itself is a structural record.
    DeleteEdge(EdgeId),
    /// Add an assertion. Boxed because an assertion is by far the largest payload here and an
    /// enum is as large as its largest variant.
    AddAssertion(Box<Assertion<V>>),
    /// Retract an assertion, which does not erase it (design § 36).
    RetractAssertion(Retraction),
    /// Declare a node type. Boxed for the reason [`GraphOperation::AddAssertion`] is.
    DefineNodeType(Box<NodeType>),
    /// Declare an edge type. Boxed for the same reason.
    DefineEdgeType(Box<EdgeType>),
    /// Redeclare a property of a type.
    ModifyProperty(PropertyDefinition),
    /// Hold two nodes to be one.
    MergeEntity(EntityMerge),
    /// Invoke a named operation of the node's type: amendment 87.
    Invoke {
        /// The node it acts on.
        node: NodeId,
        /// The operation's name, as the node's type declares it.
        operation: String,
        /// Its arguments, by name.
        #[serde(
            deserialize_with = "ekr_core::decode::unique_map",
            bound(deserialize = "V: Deserialize<'de>")
        )]
        arguments: BTreeMap<String, V>,
    },
    /// Replace an accepted assertion without erasing its assessment or former valid interval.
    SupersedeAssertion(Supersession),
}

/// What an agent proposes: design § 19, and `ekr.kernel.GraphTransaction`.
///
/// The domain entity carries `operations_hash` and `operation_count` where this carries the
/// operations themselves — "the operations travel as content-addressed bytes; the kernel holds
/// their hash and count". This is the in-process form the validators read; the hash the domain
/// names is what a store keeps of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphTransaction<V: ValueSpace = Value> {
    /// Its stable id.
    pub id: TransactionId,
    /// The agent that proposed it. Never the sole basis of its validation (design § 6.10).
    pub proposer: AgentId,
    /// The changes it proposes. A transaction is atomic, so this is a set of operations written
    /// in some order and not a sequence with instants between its steps: every validator that
    /// reads it reads it as a set, and one that did not would give two verdicts for one proposal
    /// depending on how the proposer happened to fill the vector.
    pub operations: Vec<GraphOperation<V>>,
    /// **The manifest: what this transaction declares it rests on.**
    ///
    /// It is not a way in for evidence. A transaction carries evidence *ids* and never an
    /// [`Evidence`](ekr_graph::Evidence), and no operation here creates one, so every id in it
    /// names something canonical state must already retain — which is what
    /// [`Reference`](crate::Reference) resolves it against, and only that. It said "the evidence
    /// it brings" once, and the reference validator believed it: a proposer naming a phantom id
    /// both on an assertion and here satisfied provenance with a non-empty set and reference with
    /// this list, and an assertion with no evidence at all reached canonical state.
    ///
    /// What it is for is the other direction. `ekr.kernel.GraphTransaction.evidence_hash` is a
    /// content address *of this set*, recorded beside `operations_hash` so that a reader of the
    /// stored transaction can say what it rested on without re-reading every operation. A declared
    /// value nothing compares to the operations is an address over a number the proposer chose, so
    /// [`Structural`](crate::Structural) holds it equal to the evidence the transaction's
    /// assertions cite. The domain's `operation_count` is the same shape: equally derivable from
    /// the operations, equally declared, and equally checked.
    pub evidence: BTreeSet<EvidenceId>,
}

/// A transaction that passed every deterministic validator: design § 19.
///
/// > Only a `ValidatedTransaction` may be committed. This uses Rust's type system as part of the
/// > integrity boundary.
///
/// Three private fields and no `Deserialize`, so the only one that exists anywhere is one
/// [`crate::validate::Pipeline::validate`] built — which is AGENTS.md invariant 1 stated to the
/// compiler. `tests/compile_fail/` holds both routes in as build failures.
///
/// It holds the transaction over [`CanonicalValue`]: every value in it has been through the type
/// validator, so the thing that commits cannot carry a value canonical state does not admit, and
/// its content address is total.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedTransaction {
    transaction: GraphTransaction<CanonicalValue>,
    validated_against: RevisionNumber,
    validation_hash: ContentHash,
}

impl ValidatedTransaction {
    /// Seals a transaction that every validator accepted against the snapshot at `against`.
    ///
    /// `pub(crate)`, which is the whole of AGENTS.md invariant 1 in this file: the kernel is the
    /// only crate that can reach it.
    pub(crate) fn seal(
        transaction: GraphTransaction<CanonicalValue>,
        validated_against: RevisionNumber,
    ) -> Self {
        let validation_hash = ContentHash::of(&SealedBasis {
            transaction: &transaction,
            validated_against,
        });
        Self {
            transaction,
            validated_against,
            validation_hash,
        }
    }

    /// What was validated, in the form that commits.
    #[must_use]
    pub const fn transaction(&self) -> &GraphTransaction<CanonicalValue> {
        &self.transaction
    }

    /// The revision it was validated against: design § 72's stale-commit check reads this and
    /// compares it to the revision canonical state is at when the commit is attempted.
    #[must_use]
    pub const fn validated_against(&self) -> RevisionNumber {
        self.validated_against
    }

    /// The content address of what was validated *and* of the revision it was validated against.
    ///
    /// Both, because either alone is a hash a transaction could be swapped under: the operations
    /// alone would let a proposal validated at revision 7 present itself as validated at 8, and
    /// the revision alone would not name the operations at all.
    #[must_use]
    pub const fn validation_hash(&self) -> ContentHash {
        self.validation_hash
    }
}

/// The complete basis a seal addresses, as one value: hashed with [`ContentHash::of`] so that it
/// lives in the value domain and no payload handed to a store can take its address.
struct SealedBasis<'a> {
    transaction: &'a GraphTransaction<CanonicalValue>,
    validated_against: RevisionNumber,
}

impl Canonical for SealedBasis<'_> {
    fn encode(&self, out: &mut Encoder) {
        self.transaction.encode(out);
        self.validated_against.encode(out);
    }
}

impl<V: ValueSpace + Canonical> Canonical for GraphTransaction<V> {
    /// The four fields in declaration order: structural, with no discriminant, which is rule 5 of
    /// `ekr_core::canonical` for a composite that is not a sum type.
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.proposer.encode(out);
        out.list(self.operations.iter());
        out.set(self.evidence.iter());
    }
}

impl<V: ValueSpace + Canonical> Canonical for GraphOperation<V> {
    /// The variant marker, then the variant's payload.
    ///
    /// A sum type, so it is tagged: `DeleteEdge(EdgeId)` and `RetractAssertion(AssertionId)` carry
    /// one id each and rule 5 makes a newtype structural, so without the tag two different
    /// operations over the same bits would share an address.
    ///
    /// The numbers are literals and they are the contract. They follow the declaration order,
    /// which follows `ekr.kernel.OperationKind`.
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::CreateNode(draft) => {
                out.variant(0);
                draft.encode(out);
            }
            Self::UpdateProperty(mutation) => {
                out.variant(1);
                mutation.encode(out);
            }
            Self::CreateEdge(draft) => {
                out.variant(2);
                draft.encode(out);
            }
            Self::DeleteEdge(edge) => {
                out.variant(3);
                edge.encode(out);
            }
            Self::AddAssertion(assertion) => {
                out.variant(4);
                assertion.encode(out);
            }
            Self::RetractAssertion(assertion) => {
                out.variant(5);
                assertion.encode(out);
            }
            Self::DefineNodeType(declared) => {
                out.variant(6);
                declared.encode(out);
            }
            Self::DefineEdgeType(declared) => {
                out.variant(7);
                declared.encode(out);
            }
            Self::ModifyProperty(declared) => {
                out.variant(8);
                declared.encode(out);
            }
            Self::MergeEntity(merge) => {
                out.variant(9);
                merge.encode(out);
            }
            Self::Invoke {
                node,
                operation,
                arguments,
            } => {
                out.variant(10);
                node.encode(out);
                operation.encode(out);
                out.map(arguments.iter());
            }
            Self::SupersedeAssertion(supersession) => {
                out.variant(11);
                supersession.encode(out);
            }
        }
    }
}

impl<V: Canonical> Canonical for NodeDraft<V> {
    /// The five fields in declaration order.
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.root_id.encode(out);
        self.type_id.encode(out);
        self.canonical_name.encode(out);
        out.map(self.properties.iter());
    }
}

impl<V: Canonical> Canonical for PropertyMutation<V> {
    /// The three fields in declaration order.
    fn encode(&self, out: &mut Encoder) {
        self.node.encode(out);
        self.property.encode(out);
        out.list(self.values.iter());
    }
}

impl<V: Canonical> Canonical for EdgeDraft<V> {
    /// The six fields in declaration order.
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.root_id.encode(out);
        self.type_id.encode(out);
        self.source.encode(out);
        self.target.encode(out);
        out.map(self.properties.iter());
    }
}

impl Canonical for EntityMerge {
    /// The two fields in declaration order. Not a set: which id survives is the decision the
    /// operation records, so a merge and its reverse are two different operations.
    fn encode(&self, out: &mut Encoder) {
        self.absorbed.encode(out);
        self.into.encode(out);
    }
}

impl TryFrom<GraphTransaction<Value>> for GraphTransaction<CanonicalValue> {
    type Error = InadmissibleValue;

    /// The proposal, in the form that has a content address.
    ///
    /// Total for every transaction the type validator accepted:
    /// `ekr_ontology::Value::inadmissible_in_canonical_state` — which is what that validator asks
    /// — answers `None` for exactly the values `CanonicalValue::try_from` accepts, and
    /// `crates/ekr-graph/tests/canonical_value_and_assertion.rs` is the case that holds the two
    /// walks equal. The error is kept rather than unwrapped because a `Result` a caller can act on
    /// is the honest shape when two walks of one rule are involved:
    /// [`crate::validate::Pipeline::validate`] turns it into an ordinary type issue.
    ///
    /// # Errors
    ///
    /// [`InadmissibleValue`], naming the path to the first value canonical state refuses.
    fn try_from(proposal: GraphTransaction<Value>) -> Result<Self, Self::Error> {
        Ok(Self {
            id: proposal.id,
            proposer: proposal.proposer,
            operations: proposal
                .operations
                .into_iter()
                .map(canonical_operation)
                .collect::<Result<Vec<_>, _>>()?,
            evidence: proposal.evidence,
        })
    }
}

/// One operation, with every value in it admissible in canonical state.
fn canonical_operation(
    operation: GraphOperation<Value>,
) -> Result<GraphOperation<CanonicalValue>, InadmissibleValue> {
    Ok(match operation {
        GraphOperation::CreateNode(draft) => GraphOperation::CreateNode(NodeDraft {
            id: draft.id,
            root_id: draft.root_id,
            type_id: draft.type_id,
            canonical_name: draft.canonical_name,
            properties: canonical_properties(draft.properties)?,
        }),
        GraphOperation::UpdateProperty(mutation) => {
            GraphOperation::UpdateProperty(PropertyMutation {
                node: mutation.node,
                property: mutation.property,
                values: canonical_values(mutation.values)?,
            })
        }
        GraphOperation::CreateEdge(draft) => GraphOperation::CreateEdge(EdgeDraft {
            id: draft.id,
            root_id: draft.root_id,
            type_id: draft.type_id,
            source: draft.source,
            target: draft.target,
            properties: canonical_properties(draft.properties)?,
        }),
        GraphOperation::DeleteEdge(edge) => GraphOperation::DeleteEdge(edge),
        GraphOperation::AddAssertion(assertion) => {
            GraphOperation::AddAssertion(Box::new(canonical_assertion(*assertion)?))
        }
        GraphOperation::RetractAssertion(assertion) => GraphOperation::RetractAssertion(assertion),
        GraphOperation::SupersedeAssertion(supersession) => {
            GraphOperation::SupersedeAssertion(supersession)
        }
        GraphOperation::DefineNodeType(declared) => GraphOperation::DefineNodeType(declared),
        GraphOperation::DefineEdgeType(declared) => GraphOperation::DefineEdgeType(declared),
        GraphOperation::ModifyProperty(declared) => GraphOperation::ModifyProperty(declared),
        GraphOperation::MergeEntity(merge) => GraphOperation::MergeEntity(merge),
        GraphOperation::Invoke {
            node,
            operation,
            arguments,
        } => GraphOperation::Invoke {
            node,
            operation,
            arguments: arguments
                .into_iter()
                .map(|(name, value)| CanonicalValue::try_from(value).map(|held| (name, held)))
                .collect::<Result<BTreeMap<_, _>, _>>()?,
        },
    })
}

/// One claim, with the value its object may carry made admissible and its references — node, edge
/// and evidence, since wave p1-14 — minted.
///
/// **This is the only place in the workspace where a [`CanonicalRef`] is minted from a proposal's
/// id**, and it is reached only from [`Pipeline::validate`](crate::Pipeline::validate) after every
/// deterministic validator has accepted — `Reference` among them, which is what resolved these
/// exact ids against the snapshot (`crates/ekr-kernel/src/validate/reference.rs`). The type holds
/// inside Rust and this validator holds the edge **on the transaction path**; this function is that
/// seam, and it sits *after* the validator rather than beside it. The seed path has no equivalent
/// and refuses nothing — `ekr_graph::canonical`'s header says where that is filed.
fn canonical_assertion(
    assertion: Assertion<Value>,
) -> Result<Assertion<CanonicalValue>, InadmissibleValue> {
    Ok(Assertion {
        id: assertion.id,
        root_id: assertion.root_id,
        subject: match assertion.subject {
            Subject::Node(node) => Subject::Node(CanonicalRef::new(node)),
            Subject::Edge(edge) => Subject::Edge(CanonicalRef::new(edge)),
            Subject::Type(type_id) => Subject::Type(type_id),
        },
        predicate: assertion.predicate,
        object: match assertion.object {
            Object::Value(value) => Object::Value(CanonicalValue::try_from(value)?),
            Object::Node(node) => Object::Node(CanonicalRef::new(node)),
            Object::Type(type_id) => Object::Type(type_id),
        },
        evidence: assertion
            .evidence
            .into_iter()
            .map(CanonicalRef::new)
            .collect(),
        proposed_by: assertion.proposed_by,
        assessment: assertion.assessment.map_assertions(CanonicalRef::new),
        lifecycle: assertion.lifecycle.map_assertions(CanonicalRef::new),
        valid_time: assertion.valid_time,
        transaction_time: assertion.transaction_time,
    })
}

/// A property bag, every value in it admissible.
fn canonical_properties(
    properties: BTreeMap<PropertyId, Vec<Value>>,
) -> Result<BTreeMap<PropertyId, Vec<CanonicalValue>>, InadmissibleValue> {
    properties
        .into_iter()
        .map(|(property, values)| canonical_values(values).map(|held| (property, held)))
        .collect()
}

/// A run of values, every one of them admissible.
fn canonical_values(values: Vec<Value>) -> Result<Vec<CanonicalValue>, InadmissibleValue> {
    values.into_iter().map(CanonicalValue::try_from).collect()
}
