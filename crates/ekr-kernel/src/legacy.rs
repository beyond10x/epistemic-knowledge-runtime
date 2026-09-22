//! Frozen encodings from source commit 73ab8b0a5aa5c670bacbc4c1abdca02f87b62bf0.
//! These historical data transfer types never construct canonical state capabilities.
//! Field order, discriminants and scalar property shape intentionally remain unchanged.
//! Only ekr-core scalar identities, timestamps, hashes and canonical primitives are shared.
//! Immutable vectors pin those primitives too; no current graph or ontology codec is called.

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::*;
use ekr_graph::legacy::{unique_map, unique_set, Assertion, Value};
use ekr_store::legacy::{decode_json, verify_payload, GraphDocument, Refusal};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
/// Original property multiplicity.
pub enum Cardinality {
    /// At most one value.
    #[default]
    One,
    /// Any number of values.
    Many,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "value_kind", content = "parameters", deny_unknown_fields)]
/// Original recursive declared value type.
pub enum ValueType {
    /// Text.
    String,
    /// A truth value.
    Boolean,
    /// A whole number.
    Integer,
    /// An approximate number.
    Float,
    /// An exact number, held as text so that it has one representation.
    Decimal,
    /// A point in time.
    Timestamp,
    /// A length of time.
    Duration,
    /// A reference to a node of one of the allowed types, or of a type that conforms to one.
    NodeRef {
        /// The types a value of this type may point at. Empty is refused at load: a reference to
        /// nothing is not a reference.
        #[serde(deserialize_with = "unique_set")]
        allowed_types: BTreeSet<TypeId>,
    },
    /// One of a declared set of variants.
    Enum {
        /// The variants a value of this type may be. Empty is refused at load: a type no value
        /// inhabits is not a type.
        #[serde(deserialize_with = "unique_set")]
        variants: BTreeSet<String>,
    },
    /// A sequence whose every element has this type.
    List(Box<ValueType>),
    /// A set of named fields, each with its own type. Exactly these fields, no more and no fewer.
    Record(#[serde(deserialize_with = "unique_map")] BTreeMap<String, ValueType>),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Original lifecycle transition data.
pub struct Transition {
    /// The state a node must be in for this move.
    pub from: String,
    /// The state it is in afterwards.
    pub to: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Original lifecycle declaration data.
pub struct Lifecycle {
    /// The state a newly created node of this type is in. Must be one of `states`.
    pub initial: String,
    /// Every state a node of this type may be in.
    #[serde(deserialize_with = "unique_set")]
    pub states: BTreeSet<String>,
    /// Every move a node of this type may make. A pair not here is not a move.
    #[serde(default)]
    #[serde(deserialize_with = "unique_set")]
    pub transitions: BTreeSet<Transition>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Original named operation declaration, retaining opaque effects without interpreting them.
pub struct OperationDefinition {
    /// The operation's name, as `GraphOperation::Invoke` names it.
    pub name: String,
    /// The arguments it takes, by name, each typed.
    #[serde(default)]
    #[serde(deserialize_with = "unique_map")]
    pub arguments: BTreeMap<String, ValueType>,
    /// Constraint expressions over the node's properties and the arguments. The language is
    /// `UNMAPPED`; these are carried and nothing here refuses one.
    #[serde(default)]
    pub preconditions: Vec<String>,
    /// The move it makes, if it makes one.
    #[serde(default)]
    pub transition: Option<Transition>,
    /// The event types it emits.
    #[serde(default)]
    pub emits: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Original property declaration.
pub struct PropertyDefinition {
    /// The property's stable id. A name is a property of a property, not its identity.
    pub id: PropertyId,
    /// What a reader calls it.
    pub name: String,
    /// The type every value of it must have.
    pub value_type: ValueType,
    /// How many values of it a node may carry.
    #[serde(default)]
    pub cardinality: Cardinality,
    /// Whether a node must carry at least one.
    #[serde(default)]
    pub required: bool,
    /// Design § 11.2 `constraints: Vec<Constraint>`. The constraint language is not defined by the
    /// design, and `systems/ekr/domains/ontology.yaml` marks it `UNMAPPED`, so these are carried
    /// as opaque text. This layer does not interpret them; the kernel refuses affected writes
    /// until an evaluator exists. The kernel case
    /// `applicable_opaque_property_constraints_refuse_all_node_write_paths` holds that boundary.
    #[serde(default)]
    pub constraints: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Original node type declaration.
pub struct NodeType {
    /// The type's stable id.
    pub id: TypeId,
    /// What a reader calls it.
    pub name: String,
    /// The types this one specialises. A node of this type carries their properties too.
    #[serde(default)]
    #[serde(deserialize_with = "unique_set")]
    pub parents: BTreeSet<TypeId>,
    /// The properties this type declares itself, by id.
    #[serde(default)]
    #[serde(deserialize_with = "unique_map")]
    pub properties: BTreeMap<PropertyId, PropertyDefinition>,
    /// Whether the type is only ever specialised. An abstract type has no nodes.
    #[serde(default)]
    pub abstract_type: bool,
    /// The lifecycle of a node of this type, if it has one (amendment 87).
    #[serde(default)]
    pub lifecycle: Option<Lifecycle>,
    /// The named operations of this type, by name (amendment 87).
    #[serde(default)]
    #[serde(deserialize_with = "unique_map")]
    pub operations: BTreeMap<String, OperationDefinition>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Original edge type declaration.
pub struct EdgeType {
    /// The type's stable id.
    pub id: TypeId,
    /// What a reader calls it.
    pub name: String,
    /// The node types an edge of this type may start at. Empty is refused at load.
    #[serde(default)]
    #[serde(deserialize_with = "unique_set")]
    pub source_types: BTreeSet<TypeId>,
    /// The node types an edge of this type may end at. Empty is refused at load.
    #[serde(default)]
    #[serde(deserialize_with = "unique_set")]
    pub target_types: BTreeSet<TypeId>,
    /// How many edges of this type one source may have.
    #[serde(default)]
    pub cardinality: Cardinality,
    /// The properties an edge of this type declares.
    #[serde(default)]
    #[serde(deserialize_with = "unique_map")]
    pub properties: BTreeMap<PropertyId, PropertyDefinition>,
    /// The edge type that is this one read backwards, if there is one.
    #[serde(default)]
    pub inverse: Option<TypeId>,
    /// Whether the relation holds in both directions.
    #[serde(default)]
    pub symmetric: bool,
    /// Whether the relation composes with itself.
    #[serde(default)]
    pub transitive: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Original ontology version metadata.
pub struct SchemaVersion {
    /// The version's stable id.
    pub id: SchemaVersionId,
    /// Its position in the lineage, counting from zero at the seed.
    pub number: u64,
    /// The version this one was derived from, or `None` for the seed.
    #[serde(default)]
    pub parent: Option<SchemaVersionId>,
    /// When it was created: `ekr.ontology.SchemaVersion.created_at`, declared `Timestamp` by
    /// `systems/ekr/domains/ontology.yaml` and carried as the `ekr-core` newtype ADR 0004 settled.
    pub created_at: Timestamp,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Original ontology document; this does not establish an admitted ontology.
pub struct OntologyDocument {
    /// The version these types belong to.
    pub version: SchemaVersion,
    /// The node types it declares.
    #[serde(default)]
    pub node_types: Vec<NodeType>,
    /// The edge types it declares.
    #[serde(default)]
    pub edge_types: Vec<EdgeType>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(deserialize = "V: Deserialize<'de>"))]
/// Original create-node operation payload; draft property vectors were already sequences.
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
    #[serde(deserialize_with = "unique_map")]
    pub properties: BTreeMap<PropertyId, Vec<V>>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(deserialize = "V: Deserialize<'de>"))]
/// Original create-edge operation payload.
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
    #[serde(deserialize_with = "unique_map")]
    pub properties: BTreeMap<PropertyId, Vec<V>>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Original property update payload.
pub struct PropertyMutation<V = Value> {
    /// The node whose property moves.
    pub node: NodeId,
    /// The property.
    pub property: PropertyId,
    /// Its values afterwards. Empty clears the property, which a required property forbids.
    pub values: Vec<V>,
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Original proposed merge payload, without permission to apply it.
pub struct EntityMerge {
    /// The node that stops being its own entity.
    pub absorbed: NodeId,
    /// The node it is absorbed into, which keeps its id.
    pub into: NodeId,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// All eleven original proposal variants; decoding never grants permission to apply them.
pub enum GraphOperation<V = Value> {
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
    RetractAssertion(AssertionId),
    /// Declare a node type. Boxed for the reason `GraphOperation::AddAssertion` is.
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
        #[serde(deserialize_with = "unique_map")]
        arguments: BTreeMap<String, V>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Original transaction proposal, encoded with its ordered operation vector.
pub struct GraphTransaction<V = Value> {
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
    /// `Evidence`(ekr_graph::Evidence), and no operation here creates one, so every id in it
    /// names something canonical state must already retain — which is what
    /// `Reference`(crate::Reference) resolves it against, and only that. It said "the evidence
    /// it brings" once, and the reference validator believed it: a proposer naming a phantom id
    /// both on an assertion and here satisfied provenance with a non-empty set and reference with
    /// this list, and an assertion with no evidence at all reached canonical state.
    ///
    /// What it is for is the other direction. `ekr.kernel.GraphTransaction.evidence_hash` is a
    /// content address *of this set*, recorded beside `operations_hash` so that a reader of the
    /// stored transaction can say what it rested on without re-reading every operation. A declared
    /// value nothing compares to the operations is an address over a number the proposer chose, so
    /// `Structural`(crate::Structural) holds it equal to the evidence the transaction's
    /// assertions cite. The domain's `operation_count` is the same shape: equally derivable from
    /// the operations, equally declared, and equally checked.
    #[serde(deserialize_with = "unique_set")]
    pub evidence: BTreeSet<EvidenceId>,
}
impl<V: Canonical> Canonical for GraphTransaction<V> {
    /// The four fields in declaration order: structural, with no discriminant, which is rule 5 of
    /// `ekr_core::canonical` for a composite that is not a sum type.
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.proposer.encode(out);
        out.list(self.operations.iter());
        out.set(self.evidence.iter());
    }
}

impl<V: Canonical> Canonical for GraphOperation<V> {
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
                DeclaredNodeType(declared).encode(out);
            }
            Self::DefineEdgeType(declared) => {
                out.variant(7);
                DeclaredEdgeType(declared).encode(out);
            }
            Self::ModifyProperty(declared) => {
                out.variant(8);
                Definition(declared).encode(out);
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

struct Declared<'a>(&'a ValueType);

struct Definition<'a>(&'a PropertyDefinition);

struct DeclaredNodeType<'a>(&'a NodeType);

struct DeclaredEdgeType<'a>(&'a EdgeType);

struct DeclaredLifecycle<'a>(&'a Lifecycle);

/// Frozen original-format representation or operation.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Move<'a>(&'a Transition);

struct Operation<'a>(&'a OperationDefinition);

struct Count(Cardinality);

impl Canonical for Count {
    /// The variant marker. Two variants carrying nothing, so the marker is the whole encoding.
    fn encode(&self, out: &mut Encoder) {
        out.variant(match self.0 {
            Cardinality::One => 0,
            Cardinality::Many => 1,
        });
    }
}

impl Canonical for Declared<'_> {
    /// The variant marker, then the variant's parameters, in the declaration order of
    /// `ekr_ontology::ValueType`.
    fn encode(&self, out: &mut Encoder) {
        match self.0 {
            ValueType::String => out.variant(0),
            ValueType::Boolean => out.variant(1),
            ValueType::Integer => out.variant(2),
            ValueType::Float => out.variant(3),
            ValueType::Decimal => out.variant(4),
            ValueType::Timestamp => out.variant(5),
            ValueType::Duration => out.variant(6),
            ValueType::NodeRef { allowed_types } => {
                out.variant(7);
                out.set(allowed_types.iter());
            }
            ValueType::Enum { variants } => {
                out.variant(8);
                out.set(variants.iter());
            }
            ValueType::List(element) => {
                out.variant(9);
                Self(element).encode(out);
            }
            ValueType::Record(fields) => {
                out.variant(10);
                let declared: Vec<Self> = fields.values().map(Self).collect();
                out.map(fields.keys().zip(declared.iter()));
            }
        }
    }
}

impl Canonical for Definition<'_> {
    /// The six fields of `ekr_ontology::PropertyDefinition`, in declaration order.
    fn encode(&self, out: &mut Encoder) {
        self.0.id.encode(out);
        self.0.name.encode(out);
        Declared(&self.0.value_type).encode(out);
        Count(self.0.cardinality).encode(out);
        self.0.required.encode(out);
        out.list(self.0.constraints.iter());
    }
}

impl Canonical for Move<'_> {
    /// The two fields of `ekr_ontology::Transition`, in declaration order.
    fn encode(&self, out: &mut Encoder) {
        self.0.from.encode(out);
        self.0.to.encode(out);
    }
}

impl Canonical for DeclaredLifecycle<'_> {
    /// The three fields of `ekr_ontology::Lifecycle`, in declaration order.
    fn encode(&self, out: &mut Encoder) {
        self.0.initial.encode(out);
        out.set(self.0.states.iter());
        let moves: Vec<Move<'_>> = self.0.transitions.iter().map(Move).collect();
        out.set(moves.iter());
    }
}

impl Canonical for Operation<'_> {
    /// The five fields of `ekr_ontology::OperationDefinition`, in declaration order.
    fn encode(&self, out: &mut Encoder) {
        self.0.name.encode(out);
        let arguments: Vec<Declared<'_>> = self.0.arguments.values().map(Declared).collect();
        out.map(self.0.arguments.keys().zip(arguments.iter()));
        out.list(self.0.preconditions.iter());
        out.option(self.0.transition.as_ref().map(Move).as_ref());
        out.list(self.0.emits.iter());
    }
}

impl Canonical for DeclaredNodeType<'_> {
    /// The seven fields of `ekr_ontology::NodeType`, in declaration order.
    fn encode(&self, out: &mut Encoder) {
        self.0.id.encode(out);
        self.0.name.encode(out);
        out.set(self.0.parents.iter());
        let properties: Vec<Definition<'_>> = self.0.properties.values().map(Definition).collect();
        out.map(self.0.properties.keys().zip(properties.iter()));
        self.0.abstract_type.encode(out);
        out.option(self.0.lifecycle.as_ref().map(DeclaredLifecycle).as_ref());
        let operations: Vec<Operation<'_>> = self.0.operations.values().map(Operation).collect();
        out.map(self.0.operations.keys().zip(operations.iter()));
    }
}

impl Canonical for DeclaredEdgeType<'_> {
    /// The nine fields of `ekr_ontology::EdgeType`, in declaration order.
    fn encode(&self, out: &mut Encoder) {
        self.0.id.encode(out);
        self.0.name.encode(out);
        out.set(self.0.source_types.iter());
        out.set(self.0.target_types.iter());
        Count(self.0.cardinality).encode(out);
        let properties: Vec<Definition<'_>> = self.0.properties.values().map(Definition).collect();
        out.map(self.0.properties.keys().zip(properties.iter()));
        out.option(self.0.inverse.as_ref());
        self.0.symmetric.encode(out);
        self.0.transitive.encode(out);
    }
}
/// Execution identities retained by the original seed envelope, not authenticated by this reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapContext {
    /// Recorded operator identity.
    pub operator: AgentId,
    /// Recorded independent validator identity.
    pub validator: AgentId,
}

/// Frozen `ekr-seed/1` input, before bootstrap attributes acceptance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedDocument {
    /// Original seed format discriminator.
    pub format: String,
    /// Complete original ontology data, not an admitted ontology capability.
    pub ontology: OntologyDocument,
    /// Proposed original graph records.
    pub graph: GraphDocument,
    /// Original evidence bytes, with every address checked independently.
    #[serde(deserialize_with = "unique_map")]
    pub evidence_payloads: BTreeMap<ContentHash, Vec<u8>>,
}

/// Frozen persisted `ekr-seed-envelope/1`, including the original bootstrap attribution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedEnvelope {
    /// Original persisted envelope discriminator.
    pub format: String,
    /// Original seed request; assertions remain Proposed in these retained bytes.
    pub input: SeedDocument,
    /// Original execution metadata; reading it does not authenticate its identities.
    pub context: BootstrapContext,
}

impl SeedDocument {
    /// Checks supplied original JSON bytes and their payload address.
    /// This is format/address verification, not a rerun of kernel admission.
    ///
    /// # Errors
    /// Unknown format/fields, duplicates, corrupt addresses, or missing evidence bytes.
    pub fn verify_bytes(bytes: &[u8], address: ContentHash) -> Result<Self, Refusal> {
        verify_payload(bytes, address)?;
        let input: Self = decode_json(bytes)?;
        input.check_supplied_data()?;
        Ok(input)
    }

    fn check_supplied_data(&self) -> Result<(), Refusal> {
        if self.format != "ekr-seed/1" {
            return Err(Refusal::UnsupportedFormat(self.format.clone()));
        }
        self.graph.check_identities()?;
        let mut types = BTreeSet::new();
        for id in self
            .ontology
            .node_types
            .iter()
            .map(|t| t.id)
            .chain(self.ontology.edge_types.iter().map(|t| t.id))
        {
            if !types.insert(id) {
                return Err(Refusal::DuplicateMember(format!("ontology-type: {id}")));
            }
        }
        for declaration in &self.ontology.node_types {
            if declaration.properties.iter().any(|(id, p)| *id != p.id)
                || declaration
                    .operations
                    .iter()
                    .any(|(name, op)| *name != op.name)
            {
                return Err(Refusal::InvalidData("misfiled-ontology-member".into()));
            }
        }
        for declaration in &self.ontology.edge_types {
            if declaration.properties.iter().any(|(id, p)| *id != p.id) {
                return Err(Refusal::InvalidData("misfiled-ontology-member".into()));
            }
        }
        for (address, bytes) in &self.evidence_payloads {
            verify_payload(bytes, *address)?;
        }
        for evidence in self.graph.evidence.values() {
            if !self.evidence_payloads.contains_key(&evidence.content_hash) {
                return Err(Refusal::MissingEvidence {
                    evidence: evidence.id,
                    address: evidence.content_hash,
                });
            }
        }
        Ok(())
    }
}

impl SeedEnvelope {
    /// Checks the exact supplied envelope bytes, recorded format and retained evidence addresses.
    /// It does not check a live provider, event ordering, complete history or execution authority.
    ///
    /// # Errors
    /// Unknown formats/fields, duplicates, corrupt addresses, or missing evidence.
    pub fn verify_bytes(bytes: &[u8], address: ContentHash) -> Result<Self, Refusal> {
        verify_payload(bytes, address)?;
        let envelope: Self = decode_json(bytes)?;
        if envelope.format != "ekr-seed-envelope/1" {
            return Err(Refusal::UnsupportedFormat(envelope.format));
        }
        envelope.input.check_supplied_data()?;
        Ok(envelope)
    }

    /// Reproduces the original Proposed→Accepted attribution on a plain historical document.
    /// The result is only data for reproducing addresses, never canonical graph authority.
    /// No missing acceptance history is invented for withdrawn or caller-accepted assertions.
    ///
    /// # Errors
    /// Unsupported format, missing/corrupt evidence, mismatched attribution or original seed shape.
    pub fn attributed_document(&self) -> Result<GraphDocument, Refusal> {
        if self.format != "ekr-seed-envelope/1" {
            return Err(Refusal::UnsupportedFormat(self.format.clone()));
        }
        self.input.check_supplied_data()?;
        let mut graph = self.input.graph.clone();
        let ontology = &self.input.ontology;
        if self.context.operator == self.context.validator
            || graph.revision != RevisionNumber::SEED
            || graph.root.parent.is_some()
            || graph.root.space != ekr_graph::legacy::Space::Canonical
            || ontology.version.number != 0
            || ontology.version.parent.is_some()
            || ontology.version.id != graph.root.schema_version_id
        {
            return Err(Refusal::InvalidData(
                "unsupported-original-bootstrap-context".into(),
            ));
        }
        for evidence in graph.evidence.values() {
            if evidence.extracted_by != self.context.operator
                || !matches!(
                    evidence.source,
                    ekr_graph::legacy::EvidenceSource::HumanStatement { .. }
                )
            {
                return Err(Refusal::InvalidData(
                    "unsupported-original-bootstrap-evidence".into(),
                ));
            }
        }
        for assertion in graph.assertions.values_mut() {
            if assertion.validation != ekr_graph::legacy::ValidationState::Proposed
                || assertion.proposed_by != self.context.operator
            {
                return Err(Refusal::InvalidData(
                    "unsupported-original-bootstrap-attribution".into(),
                ));
            }
            assertion.validation = ekr_graph::legacy::ValidationState::Accepted {
                validators: BTreeSet::from([self.context.validator]),
            };
        }
        Ok(graph)
    }
}

impl GraphTransaction {
    /// Frozen original bytes for validation: ordered transaction encoding followed by revision.
    /// Application set semantics did not make the historical operation vector encoding unordered.
    #[must_use]
    pub fn validation_bytes(&self, against: RevisionNumber) -> Vec<u8> {
        let mut encoder = Encoder::new();
        self.encode(&mut encoder);
        against.encode(&mut encoder);
        encoder.finish()
    }

    /// Original validation address used the PAYLOAD domain, despite encoding a structured value.
    #[must_use]
    pub fn validation_address(&self, against: RevisionNumber) -> ContentHash {
        ContentHash::of_bytes(&self.validation_bytes(against))
    }

    /// Checks supplied transaction bytes and the original validation claim against a revision.
    /// Success reproduces addresses only: it is not a ValidatedTransaction or validator verdict.
    ///
    /// # Errors
    /// Malformed/unknown fields, duplicate identities, or an original address mismatch.
    pub fn verify_bytes(
        bytes: &[u8],
        payload_address: ContentHash,
        against: RevisionNumber,
        validation_address: ContentHash,
    ) -> Result<Self, Refusal> {
        verify_payload(bytes, payload_address)?;
        let transaction: Self = decode_json(bytes)?;
        let observed = transaction.validation_address(against);
        if observed != validation_address {
            return Err(Refusal::PayloadAddressMismatch {
                expected: validation_address,
                observed,
            });
        }
        Ok(transaction)
    }
}
