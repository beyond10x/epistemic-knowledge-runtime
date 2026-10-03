// generated from ekr v1
// model digest 7e8583c5dd6b1d7e52589c443b3c8b6c5233df495af62c0e960428a3a47ece11
// contract digest 9550c8d18f57a577443582d1bea77eb4d77722699be32435ef6a2c207c435278
// do not edit: regenerate with `ess synthesize`

//! Ontology — `ekr.ontology`.
//!
//! The type system of a graph, held as data: schema versions, node and edge types, property definitions with typed values, and the per-type lifecycles and named operations of amendment 87. Design § 11–12, § 26, § 50–52. Versions form a lineage: the seed is number 0 with no parent, and each later version has number + 1 and the version it was derived from as its parent. A schema transaction derives the next version by applying SchemaChanges in order.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// Cardinality — `ekr.ontology.Cardinality`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    /// `One`.
    One,
    /// `Many`.
    Many,
}

/// The states of `ekr.ontology.EdgeType`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `EdgeType<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeTypeState {
    /// `Declared`.
    Declared,
}

/// EdgeTypeDeclaration — `ekr.ontology.EdgeTypeDeclaration`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeTypeDeclaration {
    /// `id` — `ekr.ontology.TypeId`.
    pub id: TypeId,
    /// `name` — `String`.
    pub name: String,
    /// `source_types` — `List<ekr.ontology.TypeId>`.
    pub source_types: Vec<TypeId>,
    /// `target_types` — `List<ekr.ontology.TypeId>`.
    pub target_types: Vec<TypeId>,
    /// `cardinality` — `ekr.ontology.Cardinality`.
    pub cardinality: Cardinality,
    /// `properties` — `Map<String, ekr.ontology.PropertyDeclaration>`.
    pub properties: std::collections::BTreeMap<String, PropertyDeclaration>,
    /// `inverse` — `Optional<ekr.ontology.TypeId>`.
    pub inverse: Option<TypeId>,
    /// `symmetric` — `Boolean`.
    pub symmetric: bool,
    /// `transitive` — `Boolean`.
    pub transitive: bool,
}

/// EdgeWidening — `ekr.ontology.EdgeWidening`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeWidening {
    /// `edge_type` — `ekr.ontology.TypeId`.
    pub edge_type: TypeId,
    /// `source_types` — `List<ekr.ontology.TypeId>`.
    pub source_types: Vec<TypeId>,
    /// `target_types` — `List<ekr.ontology.TypeId>`.
    pub target_types: Vec<TypeId>,
}

/// EvolveRefusalCode — `ekr.ontology.EvolveRefusalCode`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvolveRefusalCode {
    /// `empty-schema-change`.
    EmptySchemaChange,
    /// `schema-version-reused`.
    SchemaVersionReused,
    /// `schema-version-exhausted`.
    SchemaVersionExhausted,
    /// `type-already-declared`.
    TypeAlreadyDeclared,
    /// `unknown-property-owner`.
    UnknownPropertyOwner,
    /// `incoherent-schema`.
    IncoherentSchema,
    /// `schema-change-without-effect`.
    SchemaChangeWithoutEffect,
    /// `unknown-edge-type`.
    UnknownEdgeType,
    /// `unknown-endpoint-type`.
    UnknownEndpointType,
    /// `edge-endpoint-removed`.
    EdgeEndpointRemoved,
}

/// IncompatibilityCode — `ekr.ontology.IncompatibilityCode`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncompatibilityCode {
    /// `required-property-missing`.
    RequiredPropertyMissing,
    /// `cardinality-narrowed`.
    CardinalityNarrowed,
    /// `value-kind-not-admitted`.
    ValueKindNotAdmitted,
    /// `value-type-narrowed`.
    ValueTypeNarrowed,
    /// `type-removed`.
    TypeRemoved,
    /// `type-declaration-changed`.
    TypeDeclarationChanged,
    /// `property-removed`.
    PropertyRemoved,
    /// `not-a-successor`.
    NotASuccessor,
    /// `constraint-changed`.
    ConstraintChanged,
}

/// LifecycleDeclaration — `ekr.ontology.LifecycleDeclaration`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleDeclaration {
    /// `initial` — `String`.
    pub initial: String,
    /// `states` — `List<String>`.
    pub states: Vec<String>,
    /// `transitions` — `List<ekr.ontology.Transition>`.
    pub transitions: Vec<Transition>,
}

/// The states of `ekr.ontology.NodeType`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `NodeType<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTypeState {
    /// `Declared`.
    Declared,
}

/// NodeTypeDeclaration — `ekr.ontology.NodeTypeDeclaration`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeTypeDeclaration {
    /// `id` — `ekr.ontology.TypeId`.
    pub id: TypeId,
    /// `name` — `String`.
    pub name: String,
    /// `parents` — `List<ekr.ontology.TypeId>`.
    pub parents: Vec<TypeId>,
    /// `properties` — `Map<String, ekr.ontology.PropertyDeclaration>`.
    pub properties: std::collections::BTreeMap<String, PropertyDeclaration>,
    /// `abstract_type` — `Boolean`.
    pub abstract_type: bool,
    /// `lifecycle` — `Optional<ekr.ontology.LifecycleDeclaration>`.
    pub lifecycle: Option<LifecycleDeclaration>,
    /// `operations` — `Map<String, ekr.ontology.OperationDefinition>`.
    pub operations: std::collections::BTreeMap<String, OperationDefinition>,
}

/// OntologyDocumentProjection — `ekr.ontology.OntologyDocumentProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyDocumentProjection {
    /// `version` — `ekr.ontology.SchemaVersionRecord`.
    pub version: SchemaVersionRecord,
    /// `node_types` — `List<ekr.ontology.NodeTypeDeclaration>`.
    pub node_types: Vec<NodeTypeDeclaration>,
    /// `edge_types` — `List<ekr.ontology.EdgeTypeDeclaration>`.
    pub edge_types: Vec<EdgeTypeDeclaration>,
}

/// OperationDefinition — `ekr.ontology.OperationDefinition`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationDefinition {
    /// `name` — `String`.
    pub name: String,
    /// `arguments` — `Map<String, ekr.ontology.ValueTypeProjection>`.
    pub arguments: std::collections::BTreeMap<String, std::boxed::Box<ValueTypeProjection>>,
    /// `preconditions` — `List<String>`.
    pub preconditions: Vec<String>,
    /// `transition` — `Optional<ekr.ontology.Transition>`.
    pub transition: Option<Transition>,
    /// `emits` — `List<String>`.
    pub emits: Vec<String>,
}

/// PropertyDeclaration — `ekr.ontology.PropertyDeclaration`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDeclaration {
    /// `id` — `ekr.ontology.PropertyId`.
    pub id: PropertyId,
    /// `name` — `String`.
    pub name: String,
    /// `value_type` — `ekr.ontology.ValueTypeProjection`.
    pub value_type: std::boxed::Box<ValueTypeProjection>,
    /// `cardinality` — `ekr.ontology.Cardinality`.
    pub cardinality: Cardinality,
    /// `required` — `Boolean`.
    pub required: bool,
    /// `constraints` — `List<String>`.
    pub constraints: Vec<String>,
}

/// The states of `ekr.ontology.PropertyDefinition`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `PropertyDefinition<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyDefinitionState {
    /// `Declared`.
    Declared,
}

/// PropertyId — `ekr.ontology.PropertyId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyId(pub crate::primitives::Uuid);

/// PropertyModification — `ekr.ontology.PropertyModification`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyModification {
    /// `owner` — `ekr.ontology.TypeId`.
    pub owner: TypeId,
    /// `property` — `ekr.ontology.PropertyDeclaration`.
    pub property: PropertyDeclaration,
}

/// SchemaChange — `ekr.ontology.SchemaChange`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaChange {
    /// Tagged `DefineEdgeType` — `ekr.ontology.EdgeTypeDeclaration`.
    DefineEdgeType(EdgeTypeDeclaration),
    /// Tagged `DefineNodeType` — `ekr.ontology.NodeTypeDeclaration`.
    DefineNodeType(NodeTypeDeclaration),
    /// Tagged `ModifyProperty` — `ekr.ontology.PropertyModification`.
    ModifyProperty(PropertyModification),
    /// Tagged `WidenEdgeType` — `ekr.ontology.EdgeWidening`.
    WidenEdgeType(EdgeWidening),
}

/// The states of `ekr.ontology.SchemaVersion`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `SchemaVersion<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaVersionState {
    /// `Canonical`.
    Canonical,
}

/// SchemaVersionId — `ekr.ontology.SchemaVersionId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersionId(pub crate::primitives::Uuid);

/// SchemaVersionRecord — `ekr.ontology.SchemaVersionRecord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersionRecord {
    /// `id` — `ekr.ontology.SchemaVersionId`.
    pub id: SchemaVersionId,
    /// `number` — `Integer`.
    pub number: i64,
    /// `parent` — `Optional<ekr.ontology.SchemaVersionId>`.
    pub parent: Option<SchemaVersionId>,
    /// `created_at` — `Timestamp`.
    pub created_at: crate::primitives::Timestamp,
}

/// Transition — `ekr.ontology.Transition`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    /// `from` — `String`.
    pub from: String,
    /// `to` — `String`.
    pub to: String,
}

/// TypeId — `ekr.ontology.TypeId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeId(pub crate::primitives::Uuid);

/// ValueKind — `ekr.ontology.ValueKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// `String`.
    String,
    /// `Boolean`.
    Boolean,
    /// `Integer`.
    Integer,
    /// `Float`.
    Float,
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

/// ValueTypeProjection — `ekr.ontology.ValueTypeProjection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueTypeProjection {
    /// `kind` — `ekr.ontology.ValueKind`.
    pub kind: ValueKind,
    /// `allowed_types` — `Optional<List<ekr.ontology.TypeId>>`.
    pub allowed_types: Option<Vec<TypeId>>,
    /// `variants` — `Optional<List<String>>`.
    pub variants: Option<Vec<String>>,
    /// `element` — `Optional<ekr.ontology.ValueTypeProjection>`.
    pub element: Option<std::boxed::Box<ValueTypeProjection>>,
    /// `fields` — `Optional<Map<String, ekr.ontology.ValueTypeProjection>>`.
    pub fields: Option<std::collections::BTreeMap<String, std::boxed::Box<ValueTypeProjection>>>,
}

/// What EdgeType — `ekr.ontology.EdgeType` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`EdgeType<S>`], and at a boundary by [`EdgeTypeSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeTypeData {
    /// The identity: `type_id` — `ekr.ontology.TypeId`.
    pub type_id: TypeId,
    /// `schema_version_id` — `ekr.ontology.SchemaVersionId`.
    ///
    /// Carries `edge_types`: `ekr.ontology.SchemaVersion` owns many `ekr.ontology.EdgeType`.
    pub schema_version_id: SchemaVersionId,
    /// `name` — `String`.
    pub name: String,
    /// `source_types` — `List<ekr.ontology.TypeId>`.
    pub source_types: Vec<TypeId>,
    /// `target_types` — `List<ekr.ontology.TypeId>`.
    pub target_types: Vec<TypeId>,
    /// `cardinality` — `ekr.ontology.Cardinality`.
    pub cardinality: Cardinality,
    /// `inverse` — `Optional<ekr.ontology.TypeId>`.
    pub inverse: Option<TypeId>,
    /// `symmetric` — `Boolean`.
    pub symmetric: bool,
    /// `transitive` — `Boolean`.
    pub transitive: bool,
}

/// The states of `ekr.ontology.EdgeType`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](edge_type_state::Marker), so [`EdgeType<S>`](EdgeType) can only ever rest in a real state.
pub mod edge_type_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Declared {}
    }

    /// A declared state of `EdgeType`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::EdgeTypeState;
    }

    /// `Declared`. Where a new instance starts.
    pub struct Declared;

    impl Marker for Declared {
        const STATE: super::EdgeTypeState = super::EdgeTypeState::Declared;
    }
}

/// EdgeType — `ekr.ontology.EdgeType` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Declared`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`EdgeTypeSnapshot`]
/// and [`EdgeTypeSnapshot::refine`].
pub struct EdgeType<S: edge_type_state::Marker> {
    data: EdgeTypeData,
    state: core::marker::PhantomData<S>,
}

impl<S: edge_type_state::Marker> EdgeType<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> EdgeTypeState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &EdgeTypeData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> EdgeTypeData {
        self.data
    }
}

impl EdgeType<edge_type_state::Declared> {
    /// A new instance, resting in `Declared` — the only state the lifecycle starts one in.
    pub fn new(data: EdgeTypeData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.ontology.EdgeType` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`EdgeTypeSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeTypeSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: EdgeTypeState,
    /// What it holds.
    pub data: EdgeTypeData,
}

/// An `EdgeType` in whichever declared state it was found.
pub enum AnyEdgeType {
    /// Resting in `Declared`.
    Declared(EdgeType<edge_type_state::Declared>),
}

impl EdgeTypeSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `EdgeTypeState` cannot spell one.
    pub fn refine(self) -> AnyEdgeType {
        match self.state {
            EdgeTypeState::Declared => AnyEdgeType::Declared(EdgeType {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyEdgeType {
    /// The state, as the runtime value.
    pub fn state(&self) -> EdgeTypeState {
        match self {
            Self::Declared(_) => EdgeTypeState::Declared,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> EdgeTypeSnapshot {
        match self {
            Self::Declared(instance) => EdgeTypeSnapshot {
                state: EdgeTypeState::Declared,
                data: instance.into_data(),
            },
        }
    }
}

/// What NodeType — `ekr.ontology.NodeType` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`NodeType<S>`], and at a boundary by [`NodeTypeSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeTypeData {
    /// The identity: `type_id` — `ekr.ontology.TypeId`.
    pub type_id: TypeId,
    /// `schema_version_id` — `ekr.ontology.SchemaVersionId`.
    ///
    /// Carries `node_types`: `ekr.ontology.SchemaVersion` owns many `ekr.ontology.NodeType`.
    pub schema_version_id: SchemaVersionId,
    /// `name` — `String`.
    pub name: String,
    /// `parents` — `List<ekr.ontology.TypeId>`.
    pub parents: Vec<TypeId>,
    /// `abstract_type` — `Boolean`.
    pub abstract_type: bool,
    /// `lifecycle_initial` — `Optional<String>`.
    pub lifecycle_initial: Option<String>,
    /// `lifecycle_states` — `List<String>`.
    pub lifecycle_states: Vec<String>,
    /// `lifecycle_transitions` — `List<ekr.ontology.Transition>`.
    pub lifecycle_transitions: Vec<Transition>,
    /// `operations` — `List<ekr.ontology.OperationDefinition>`.
    pub operations: Vec<OperationDefinition>,
}

/// The states of `ekr.ontology.NodeType`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](node_type_state::Marker), so [`NodeType<S>`](NodeType) can only ever rest in a real state.
pub mod node_type_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Declared {}
    }

    /// A declared state of `NodeType`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::NodeTypeState;
    }

    /// `Declared`. Where a new instance starts.
    pub struct Declared;

    impl Marker for Declared {
        const STATE: super::NodeTypeState = super::NodeTypeState::Declared;
    }
}

/// NodeType — `ekr.ontology.NodeType` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Declared`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`NodeTypeSnapshot`]
/// and [`NodeTypeSnapshot::refine`].
pub struct NodeType<S: node_type_state::Marker> {
    data: NodeTypeData,
    state: core::marker::PhantomData<S>,
}

impl<S: node_type_state::Marker> NodeType<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> NodeTypeState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &NodeTypeData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> NodeTypeData {
        self.data
    }
}

impl NodeType<node_type_state::Declared> {
    /// A new instance, resting in `Declared` — the only state the lifecycle starts one in.
    pub fn new(data: NodeTypeData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.ontology.NodeType` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`NodeTypeSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeTypeSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: NodeTypeState,
    /// What it holds.
    pub data: NodeTypeData,
}

/// An `NodeType` in whichever declared state it was found.
pub enum AnyNodeType {
    /// Resting in `Declared`.
    Declared(NodeType<node_type_state::Declared>),
}

impl NodeTypeSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `NodeTypeState` cannot spell one.
    pub fn refine(self) -> AnyNodeType {
        match self.state {
            NodeTypeState::Declared => AnyNodeType::Declared(NodeType {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyNodeType {
    /// The state, as the runtime value.
    pub fn state(&self) -> NodeTypeState {
        match self {
            Self::Declared(_) => NodeTypeState::Declared,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> NodeTypeSnapshot {
        match self {
            Self::Declared(instance) => NodeTypeSnapshot {
                state: NodeTypeState::Declared,
                data: instance.into_data(),
            },
        }
    }
}

/// What PropertyDefinition — `ekr.ontology.PropertyDefinition` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`PropertyDefinition<S>`], and at a boundary by [`PropertyDefinitionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDefinitionData {
    /// The identity: `property_id` — `ekr.ontology.PropertyId`.
    pub property_id: PropertyId,
    /// `schema_version_id` — `ekr.ontology.SchemaVersionId`.
    ///
    /// Carries `properties`: `ekr.ontology.SchemaVersion` owns many `ekr.ontology.PropertyDefinition`.
    pub schema_version_id: SchemaVersionId,
    /// `owner_type_id` — `ekr.ontology.TypeId`.
    pub owner_type_id: TypeId,
    /// `name` — `String`.
    pub name: String,
    /// `value_type` — `ekr.ontology.ValueTypeProjection`.
    pub value_type: std::boxed::Box<ValueTypeProjection>,
    /// `cardinality` — `ekr.ontology.Cardinality`.
    pub cardinality: Cardinality,
    /// `required` — `Boolean`.
    pub required: bool,
    /// `constraints` — `List<String>`.
    pub constraints: Vec<String>,
}

/// The states of `ekr.ontology.PropertyDefinition`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](property_definition_state::Marker), so [`PropertyDefinition<S>`](PropertyDefinition) can only ever rest in a real state.
pub mod property_definition_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Declared {}
    }

    /// A declared state of `PropertyDefinition`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::PropertyDefinitionState;
    }

    /// `Declared`. Where a new instance starts.
    pub struct Declared;

    impl Marker for Declared {
        const STATE: super::PropertyDefinitionState = super::PropertyDefinitionState::Declared;
    }
}

/// PropertyDefinition — `ekr.ontology.PropertyDefinition` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Declared`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`PropertyDefinitionSnapshot`]
/// and [`PropertyDefinitionSnapshot::refine`].
pub struct PropertyDefinition<S: property_definition_state::Marker> {
    data: PropertyDefinitionData,
    state: core::marker::PhantomData<S>,
}

impl<S: property_definition_state::Marker> PropertyDefinition<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> PropertyDefinitionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &PropertyDefinitionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> PropertyDefinitionData {
        self.data
    }
}

impl PropertyDefinition<property_definition_state::Declared> {
    /// A new instance, resting in `Declared` — the only state the lifecycle starts one in.
    pub fn new(data: PropertyDefinitionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.ontology.PropertyDefinition` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`PropertyDefinitionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDefinitionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: PropertyDefinitionState,
    /// What it holds.
    pub data: PropertyDefinitionData,
}

/// An `PropertyDefinition` in whichever declared state it was found.
pub enum AnyPropertyDefinition {
    /// Resting in `Declared`.
    Declared(PropertyDefinition<property_definition_state::Declared>),
}

impl PropertyDefinitionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `PropertyDefinitionState` cannot spell one.
    pub fn refine(self) -> AnyPropertyDefinition {
        match self.state {
            PropertyDefinitionState::Declared => AnyPropertyDefinition::Declared(PropertyDefinition {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyPropertyDefinition {
    /// The state, as the runtime value.
    pub fn state(&self) -> PropertyDefinitionState {
        match self {
            Self::Declared(_) => PropertyDefinitionState::Declared,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> PropertyDefinitionSnapshot {
        match self {
            Self::Declared(instance) => PropertyDefinitionSnapshot {
                state: PropertyDefinitionState::Declared,
                data: instance.into_data(),
            },
        }
    }
}

/// What SchemaVersion — `ekr.ontology.SchemaVersion` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`SchemaVersion<S>`], and at a boundary by [`SchemaVersionSnapshot::state`].
///
/// Every value satisfies `number >= 0` — checked by [`SchemaVersionData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersionData {
    /// The identity: `schema_version_id` — `ekr.ontology.SchemaVersionId`.
    pub schema_version_id: SchemaVersionId,
    /// `number` — `Integer`.
    pub number: i64,
    /// `parent` — `Optional<ekr.ontology.SchemaVersionId>`.
    pub parent: Option<SchemaVersionId>,
    /// `created_at` — `Timestamp`.
    pub created_at: crate::primitives::Timestamp,
}

impl SchemaVersionData {
    /// The first declared invariant of `ekr.ontology.SchemaVersion` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.number)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("number >= 0");
        }
        None
    }
}

/// The states of `ekr.ontology.SchemaVersion`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](schema_version_state::Marker), so [`SchemaVersion<S>`](SchemaVersion) can only ever rest in a real state.
pub mod schema_version_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Canonical {}
    }

    /// A declared state of `SchemaVersion`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SchemaVersionState;
    }

    /// `Canonical`. Where a new instance starts.
    pub struct Canonical;

    impl Marker for Canonical {
        const STATE: super::SchemaVersionState = super::SchemaVersionState::Canonical;
    }
}

/// SchemaVersion — `ekr.ontology.SchemaVersion` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Canonical`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SchemaVersionSnapshot`]
/// and [`SchemaVersionSnapshot::refine`].
pub struct SchemaVersion<S: schema_version_state::Marker> {
    data: SchemaVersionData,
    state: core::marker::PhantomData<S>,
}

impl<S: schema_version_state::Marker> SchemaVersion<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SchemaVersionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SchemaVersionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SchemaVersionData {
        self.data
    }
}

impl SchemaVersion<schema_version_state::Canonical> {
    /// A new instance, resting in `Canonical` — the only state the lifecycle starts one in.
    pub fn new(data: SchemaVersionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `ekr.ontology.SchemaVersion` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SchemaVersionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SchemaVersionState,
    /// What it holds.
    pub data: SchemaVersionData,
}

/// An `SchemaVersion` in whichever declared state it was found.
pub enum AnySchemaVersion {
    /// Resting in `Canonical`.
    Canonical(SchemaVersion<schema_version_state::Canonical>),
}

impl SchemaVersionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SchemaVersionState` cannot spell one.
    pub fn refine(self) -> AnySchemaVersion {
        match self.state {
            SchemaVersionState::Canonical => AnySchemaVersion::Canonical(SchemaVersion {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySchemaVersion {
    /// The state, as the runtime value.
    pub fn state(&self) -> SchemaVersionState {
        match self {
            Self::Canonical(_) => SchemaVersionState::Canonical,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SchemaVersionSnapshot {
        match self {
            Self::Canonical(instance) => SchemaVersionSnapshot {
                state: SchemaVersionState::Canonical,
                data: instance.into_data(),
            },
        }
    }
}

/// Node types — one row of the view `ekr.ontology.NodeTypesByVersion`.
///
/// Projects `ekr.ontology.NodeType` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeTypesByVersion {
    /// `type_id` — `ekr.ontology.TypeId`.
    pub type_id: TypeId,
    /// `schema_version_id` — `ekr.ontology.SchemaVersionId`.
    pub schema_version_id: SchemaVersionId,
    /// `name` — `String`.
    pub name: String,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The query `ekr.ontology.NodeTypesByVersion` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait NodeTypesByVersionQuery {
        /// Serves `ekr.ontology.NodeTypesByVersion` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn node_types_by_version(&self) -> Result<Vec<super::NodeTypesByVersion>, crate::obligation::UnmetObligation>;
    }

}
