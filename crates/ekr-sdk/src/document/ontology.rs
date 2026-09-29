//! The ontology by name: what a consumer declares ([`OntologySpec`]), what a store holds
//! ([`Ontology`], read from `ekr ontology`), and the schema operations between the two
//! ([`Ontology::ensure`]).
//!
//! A name is not an identity (`AGENTS.md` invariant 3): the store knows types and properties by
//! id. The spec names them because a consumer does, and the map this module keeps is the one
//! place a name becomes an id — minted here for what is new, read from `ekr ontology` for what the
//! store has. It adds no noun: a spec is expressed in what the ontology operations of today
//! declare (`DefineNodeType`, `DefineEdgeType`, `ModifyProperty`, `WidenEdgeType`), and a seed
//! built from one declares exactly what those operations would.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AgentId, PropertyId, SchemaVersionId, Timestamp, TypeId};
use serde::Deserialize;

use super::seed::{OntologySection, SchemaVersion, SeedBuilder};
use super::transaction::{
    EdgeWidening, Operation, PropertyModification, TransactionBuilder, TransactionDocument,
};
use super::value::{Cardinality, EdgeType, NodeType, PropertyDefinition, ValueType};
use super::DocumentError;

/// A value type, with node types named rather than identified.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueSpec {
    /// `String`.
    String,
    /// `Boolean`.
    Boolean,
    /// `Integer`.
    Integer,
    /// `Float`: declarable, never committed as a value.
    Float,
    /// `Decimal`.
    Decimal,
    /// `Timestamp`.
    Timestamp,
    /// `Duration`.
    Duration,
    /// `NodeRef` to the node types of these names.
    NodeRef(Vec<String>),
    /// `Enum` of these variants.
    Enum(Vec<String>),
    /// `List` of this element type.
    List(Box<ValueSpec>),
    /// `Record` of these fields.
    Record(BTreeMap<String, ValueSpec>),
}

/// A property by name, with its value kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertySpec {
    /// Its name, unique on the type that declares it.
    pub name: String,
    /// Its value type.
    pub value: ValueSpec,
    /// How many values a node may carry.
    pub cardinality: Cardinality,
    /// Whether a node must carry one.
    pub required: bool,
}

impl PropertySpec {
    /// A single-valued, optional property.
    #[must_use]
    pub fn new(name: impl Into<String>, value: ValueSpec) -> Self {
        Self {
            name: name.into(),
            value,
            cardinality: Cardinality::One,
            required: false,
        }
    }

    /// This property with `cardinality`.
    #[must_use]
    pub fn with_cardinality(mut self, cardinality: Cardinality) -> Self {
        self.cardinality = cardinality;
        self
    }

    /// This property, required.
    #[must_use]
    pub fn as_required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// A node type by name: its parents by name and the properties it declares itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeTypeSpec {
    /// Its name, unique among node types.
    pub name: String,
    /// The node types it specialises, by name.
    pub parents: Vec<String>,
    /// Whether it is abstract.
    pub abstract_type: bool,
    /// The properties it declares itself.
    pub properties: Vec<PropertySpec>,
}

impl NodeTypeSpec {
    /// A concrete type with no parent and no property.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parents: Vec::new(),
            abstract_type: false,
            properties: Vec::new(),
        }
    }

    /// This type specialising the node type named `parent`.
    #[must_use]
    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parents.push(parent.into());
        self
    }

    /// This type, abstract.
    #[must_use]
    pub fn as_abstract(mut self) -> Self {
        self.abstract_type = true;
        self
    }

    /// This type declaring `property`.
    #[must_use]
    pub fn with_property(mut self, property: PropertySpec) -> Self {
        self.properties.push(property);
        self
    }
}

/// An edge type by name: its ends by node type name, its cardinality and properties.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeTypeSpec {
    /// Its name, unique among edge types.
    pub name: String,
    /// The node types an edge may start at, by name.
    pub source_types: Vec<String>,
    /// The node types an edge may end at, by name.
    pub target_types: Vec<String>,
    /// How many edges may leave one node.
    pub cardinality: Cardinality,
    /// The properties an edge carries.
    pub properties: Vec<PropertySpec>,
}

impl EdgeTypeSpec {
    /// A single-cardinality edge type between the node types of these names.
    #[must_use]
    pub fn new<S: Into<String>, T: Into<String>>(
        name: impl Into<String>,
        source_types: impl IntoIterator<Item = S>,
        target_types: impl IntoIterator<Item = T>,
    ) -> Self {
        Self {
            name: name.into(),
            source_types: source_types.into_iter().map(Into::into).collect(),
            target_types: target_types.into_iter().map(Into::into).collect(),
            cardinality: Cardinality::One,
            properties: Vec::new(),
        }
    }

    /// This edge type with `cardinality`.
    #[must_use]
    pub fn with_cardinality(mut self, cardinality: Cardinality) -> Self {
        self.cardinality = cardinality;
        self
    }

    /// This edge type declaring `property`.
    #[must_use]
    pub fn with_property(mut self, property: PropertySpec) -> Self {
        self.properties.push(property);
        self
    }
}

/// The ontology a consumer needs, by name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OntologySpec {
    /// Its node types.
    pub node_types: Vec<NodeTypeSpec>,
    /// Its edge types.
    pub edge_types: Vec<EdgeTypeSpec>,
}

impl OntologySpec {
    /// A spec declaring nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// This spec with one more node type.
    #[must_use]
    pub fn with_node_type(mut self, node_type: NodeTypeSpec) -> Self {
        self.node_types.push(node_type);
        self
    }

    /// This spec with one more edge type.
    #[must_use]
    pub fn with_edge_type(mut self, edge_type: EdgeTypeSpec) -> Self {
        self.edge_types.push(edge_type);
        self
    }

    /// A seed declaring this spec, every id minted here, with an empty graph for the caller to
    /// fill, and the name→id map of what it declares.
    ///
    /// # Errors
    /// A name declared twice ([`OntologyError::DuplicateName`]) or named and never declared
    /// ([`OntologyError::UnknownName`]).
    pub fn seed(&self, created_at: Timestamp) -> Result<(SeedBuilder, Ontology), OntologyError> {
        let plan = Ontology::default().plan(self)?;
        let mut ontology = plan.next;
        let version = SchemaVersion::seed(created_at);
        ontology.schema_version = Some(version.id);
        let section = OntologySection {
            version,
            node_types: plan.define_nodes,
            edge_types: plan.define_edges,
        };
        Ok((SeedBuilder::new(section, created_at), ontology))
    }
}

/// The validation profile a store was seeded under, which decides whether its schema can change.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ValidationProfile {
    /// `ekr.p1-deterministic/1`: the schema is fixed at seeding.
    V1,
    /// `ekr.p2-deterministic/1`: schema changes are applied.
    V2,
    /// `ekr.p3-deterministic/1`: as v2, and a previously held id is not reused.
    V3,
}

impl ValidationProfile {
    /// Whether a store under this profile applies schema changes.
    #[must_use]
    pub const fn admits_schema_changes(self) -> bool {
        matches!(self, Self::V2 | Self::V3)
    }
}

/// Why a name→id map could not be read or brought to a spec.
#[derive(Debug, thiserror::Error)]
pub enum OntologyError {
    /// `ekr ontology`'s output did not read.
    #[error("reading `ekr ontology`: {0}")]
    Read(#[from] serde_json::Error),
    /// Two declarations share a name, so the name identifies neither.
    #[error("two {kind}s are named {name:?}; a name identifies one of them only if it is unique")]
    DuplicateName {
        /// `node type`, `edge type` or `property`.
        kind: &'static str,
        /// The name, `Type.property` for a property.
        name: String,
    },
    /// A spec names something it does not declare and the store does not hold.
    #[error("no {kind} is named {name:?}")]
    UnknownName {
        /// `node type` or `edge type`.
        kind: &'static str,
        /// The name.
        name: String,
    },
    /// The store holds a declaration by this name that no schema operation can make match.
    #[error("{kind} {name:?}: {reason}")]
    Conflict {
        /// `node type`, `edge type` or `property`.
        kind: &'static str,
        /// The name, `Type.property` for a property.
        name: String,
        /// What differs.
        reason: String,
    },
    /// The store's profile fixes its schema, and the spec needs schema operations.
    #[error(
        "validation profile v1 fixes the schema at seeding, and the spec needs {missing} schema \
         operations; seed a new store under profile v2 or v3"
    )]
    SchemaFixed {
        /// How many operations the spec needs.
        missing: usize,
    },
}

/// A node type the map knows.
#[derive(Clone, Debug, PartialEq, Eq)]
struct NodeEntry {
    id: TypeId,
    parents: BTreeSet<TypeId>,
    abstract_type: bool,
    properties: BTreeMap<String, PropertyDefinition>,
}

/// An edge type the map knows.
#[derive(Clone, Debug, PartialEq, Eq)]
struct EdgeEntry {
    id: TypeId,
    source_types: BTreeSet<TypeId>,
    target_types: BTreeSet<TypeId>,
    cardinality: Cardinality,
    properties: BTreeMap<String, PropertyDefinition>,
}

/// A store's ontology by name: every node type, edge type and property name, and the id it has.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ontology {
    schema_version: Option<SchemaVersionId>,
    node_types: BTreeMap<String, NodeEntry>,
    edge_types: BTreeMap<String, EdgeEntry>,
    node_names: BTreeMap<TypeId, String>,
}

/// The document `ekr ontology` prints, as far as the map reads it.
#[derive(Deserialize)]
struct Printed {
    schema_version: SchemaVersionId,
    node_types: Vec<PrintedNode>,
    edge_types: Vec<PrintedEdge>,
}

#[derive(Deserialize)]
struct Named {
    id: TypeId,
}

#[derive(Deserialize)]
struct PrintedNode {
    id: TypeId,
    name: String,
    parents: Vec<Named>,
    abstract_type: bool,
    properties: Vec<PropertyDefinition>,
}

#[derive(Deserialize)]
struct PrintedEdge {
    id: TypeId,
    name: String,
    source_types: Vec<Named>,
    target_types: Vec<Named>,
    cardinality: Cardinality,
    properties: Vec<PropertyDefinition>,
}

/// What a spec needs of an ontology, and the ontology once it has it.
struct Plan {
    define_nodes: Vec<NodeType>,
    define_edges: Vec<EdgeType>,
    modify: Vec<PropertyModification>,
    widen: Vec<EdgeWidening>,
    next: Ontology,
}

/// The properties of one declaration by name, refusing a name used twice.
fn by_name(
    owner: &str,
    properties: Vec<PropertyDefinition>,
) -> Result<BTreeMap<String, PropertyDefinition>, OntologyError> {
    let mut named = BTreeMap::new();
    for property in properties {
        let name = property.name.clone();
        if named.insert(name.clone(), property).is_some() {
            return Err(OntologyError::DuplicateName {
                kind: "property",
                name: format!("{owner}.{name}"),
            });
        }
    }
    Ok(named)
}

/// Whether two declarations of one property say the same, ids aside.
fn same_declaration(held: &PropertyDefinition, wanted: &PropertyDefinition) -> bool {
    held.value_type == wanted.value_type
        && held.cardinality == wanted.cardinality
        && held.required == wanted.required
        && held.constraints == wanted.constraints
}

impl Ontology {
    /// The map of what `ekr ontology` printed: its stdout, one JSON document.
    ///
    /// # Errors
    /// [`OntologyError::Read`] for a document that is not `ekr ontology`'s, and
    /// [`OntologyError::DuplicateName`] for a store holding two types, or two properties of one
    /// type, under one name.
    pub fn read(printed: &str) -> Result<Self, OntologyError> {
        let printed: Printed = serde_json::from_str(printed)?;
        let mut ontology = Self {
            schema_version: Some(printed.schema_version),
            ..Self::default()
        };
        for node in printed.node_types {
            let entry = NodeEntry {
                id: node.id,
                parents: node.parents.iter().map(|parent| parent.id).collect(),
                abstract_type: node.abstract_type,
                properties: by_name(&node.name, node.properties)?,
            };
            ontology.insert_node(node.name, entry)?;
        }
        for edge in printed.edge_types {
            let entry = EdgeEntry {
                id: edge.id,
                source_types: edge.source_types.iter().map(|end| end.id).collect(),
                target_types: edge.target_types.iter().map(|end| end.id).collect(),
                cardinality: edge.cardinality,
                properties: by_name(&edge.name, edge.properties)?,
            };
            ontology.insert_edge(edge.name, entry)?;
        }
        Ok(ontology)
    }

    fn insert_node(&mut self, name: String, entry: NodeEntry) -> Result<(), OntologyError> {
        if self.node_types.contains_key(&name) {
            return Err(OntologyError::DuplicateName {
                kind: "node type",
                name,
            });
        }
        self.node_names.insert(entry.id, name.clone());
        self.node_types.insert(name, entry);
        Ok(())
    }

    fn insert_edge(&mut self, name: String, entry: EdgeEntry) -> Result<(), OntologyError> {
        if self.edge_types.contains_key(&name) {
            return Err(OntologyError::DuplicateName {
                kind: "edge type",
                name,
            });
        }
        self.edge_types.insert(name, entry);
        Ok(())
    }

    /// The schema version the map was read at, or the one a seed built it for; `None` for the
    /// map a schema change produces, whose version exists once it commits.
    #[must_use]
    pub const fn schema_version(&self) -> Option<SchemaVersionId> {
        self.schema_version
    }

    /// The id of the node type named `name`.
    #[must_use]
    pub fn node_type(&self, name: &str) -> Option<TypeId> {
        self.node_types.get(name).map(|entry| entry.id)
    }

    /// The id of the edge type named `name`, which is also its relation id.
    #[must_use]
    pub fn edge_type(&self, name: &str) -> Option<TypeId> {
        self.edge_types.get(name).map(|entry| entry.id)
    }

    /// The id of the property `name` a node of the type named `node_type` carries: declared by
    /// that type, or else by the nearest ancestor that declares one of that name.
    #[must_use]
    pub fn property(&self, node_type: &str, name: &str) -> Option<PropertyId> {
        self.inherited(node_type, name).map(|property| property.id)
    }

    /// The id of the property `name` of the edge type named `edge_type`.
    #[must_use]
    pub fn edge_property(&self, edge_type: &str, name: &str) -> Option<PropertyId> {
        self.edge_types
            .get(edge_type)?
            .properties
            .get(name)
            .map(|property| property.id)
    }

    /// The declaration of `name` on `node_type` or its nearest declaring ancestor, breadth first.
    fn inherited(&self, node_type: &str, name: &str) -> Option<&PropertyDefinition> {
        let mut seen = BTreeSet::new();
        let mut level = vec![self.node_types.get(node_type)?];
        while !level.is_empty() {
            if let Some(found) = level.iter().find_map(|entry| entry.properties.get(name)) {
                return Some(found);
            }
            level = level
                .iter()
                .flat_map(|entry| entry.parents.iter())
                .filter(|parent| seen.insert(**parent))
                .filter_map(|parent| self.node_names.get(parent))
                .filter_map(|parent| self.node_types.get(parent))
                .collect();
        }
        None
    }

    /// The schema operations that make this ontology declare everything `spec` declares, and
    /// the map once they commit. Nothing when it already does.
    ///
    /// What is missing is added: a node type or edge type by `!DefineNodeType` or
    /// `!DefineEdgeType` (its properties declared with it), a property of a type the store holds
    /// by `!ModifyProperty`, an edge end by `!WidenEdgeType` writing each end whole. A property
    /// the store declares differently on the same type is redeclared under the id it has; the
    /// kernel decides whether the store's state admits that. What the store holds beyond the spec
    /// is left alone.
    ///
    /// # Errors
    /// [`OntologyError::DuplicateName`] and [`OntologyError::UnknownName`] for a spec that does
    /// not name its types uniquely and completely; [`OntologyError::Conflict`] for a declaration
    /// no operation changes (a type's parents or abstractness, an edge type's cardinality, a
    /// property an ancestor declares differently); [`OntologyError::SchemaFixed`] when operations
    /// are needed under `profile` v1.
    pub fn ensure(
        &self,
        spec: &OntologySpec,
        profile: ValidationProfile,
    ) -> Result<SchemaChange, OntologyError> {
        let plan = self.plan(spec)?;
        let mut operations: Vec<Operation> = Vec::new();
        operations.extend(plan.define_nodes.into_iter().map(Operation::from));
        operations.extend(plan.define_edges.into_iter().map(Operation::from));
        operations.extend(plan.modify.into_iter().map(Operation::from));
        operations.extend(plan.widen.into_iter().map(Operation::from));
        if !operations.is_empty() && !profile.admits_schema_changes() {
            return Err(OntologyError::SchemaFixed {
                missing: operations.len(),
            });
        }
        let mut ontology = plan.next;
        if !operations.is_empty() {
            ontology.schema_version = None;
        }
        Ok(SchemaChange {
            operations,
            ontology,
        })
    }

    fn resolve(&self, value: &ValueSpec) -> Result<ValueType, OntologyError> {
        Ok(match value {
            ValueSpec::String => ValueType::String,
            ValueSpec::Boolean => ValueType::Boolean,
            ValueSpec::Integer => ValueType::Integer,
            ValueSpec::Float => ValueType::Float,
            ValueSpec::Decimal => ValueType::Decimal,
            ValueSpec::Timestamp => ValueType::Timestamp,
            ValueSpec::Duration => ValueType::Duration,
            ValueSpec::NodeRef(names) => ValueType::NodeRef {
                allowed_types: names
                    .iter()
                    .map(|name| self.node_id(name))
                    .collect::<Result<_, _>>()?,
            },
            ValueSpec::Enum(variants) => ValueType::Enum {
                variants: variants.iter().cloned().collect(),
            },
            ValueSpec::List(element) => ValueType::List(Box::new(self.resolve(element)?)),
            ValueSpec::Record(fields) => ValueType::Record(
                fields
                    .iter()
                    .map(|(field, value)| Ok((field.clone(), self.resolve(value)?)))
                    .collect::<Result<_, OntologyError>>()?,
            ),
        })
    }

    fn node_id(&self, name: &str) -> Result<TypeId, OntologyError> {
        self.node_type(name)
            .ok_or_else(|| OntologyError::UnknownName {
                kind: "node type",
                name: name.to_owned(),
            })
    }

    fn definition(&self, spec: &PropertySpec) -> Result<PropertyDefinition, OntologyError> {
        Ok(PropertyDefinition {
            id: PropertyId::mint(),
            name: spec.name.clone(),
            value_type: self.resolve(&spec.value)?,
            cardinality: spec.cardinality,
            required: spec.required,
            constraints: Vec::new(),
        })
    }

    /// What `spec` needs of this ontology.
    fn plan(&self, spec: &OntologySpec) -> Result<Plan, OntologyError> {
        check_unique(spec)?;
        let mut next = self.clone();
        let mut new_nodes = BTreeSet::new();
        for node in &spec.node_types {
            if next.node_type(&node.name).is_none() {
                let entry = NodeEntry {
                    id: TypeId::mint(),
                    parents: BTreeSet::new(),
                    abstract_type: node.abstract_type,
                    properties: BTreeMap::new(),
                };
                next.insert_node(node.name.clone(), entry)?;
                new_nodes.insert(node.name.clone());
            }
        }
        let mut define_nodes = Vec::new();
        let mut modify = Vec::new();
        for node in &spec.node_types {
            let parents: BTreeSet<TypeId> = node
                .parents
                .iter()
                .map(|parent| next.node_id(parent))
                .collect::<Result<_, _>>()?;
            let wanted: Vec<PropertyDefinition> = node
                .properties
                .iter()
                .map(|property| next.definition(property))
                .collect::<Result<_, _>>()?;
            let owner = next.node_types[&node.name].id;
            if new_nodes.contains(&node.name) {
                let entry = next.node_types.get_mut(&node.name).expect("inserted above");
                entry.parents.clone_from(&parents);
                entry.properties = wanted
                    .iter()
                    .map(|property| (property.name.clone(), property.clone()))
                    .collect();
                define_nodes.push(NodeType {
                    id: owner,
                    name: node.name.clone(),
                    parents,
                    properties: wanted
                        .into_iter()
                        .map(|property| (property.id, property))
                        .collect(),
                    abstract_type: node.abstract_type,
                    lifecycle: None,
                    operations: BTreeMap::new(),
                });
                continue;
            }
            let held = &next.node_types[&node.name];
            if held.abstract_type != node.abstract_type || held.parents != parents {
                return Err(OntologyError::Conflict {
                    kind: "node type",
                    name: node.name.clone(),
                    reason: "the store declares it with other parents or another abstractness, \
                             which no schema operation changes"
                        .to_owned(),
                });
            }
            for mut property in wanted {
                let own = next.node_types[&node.name]
                    .properties
                    .get(&property.name)
                    .cloned();
                match own {
                    Some(held) if same_declaration(&held, &property) => {}
                    Some(held) => {
                        property.id = held.id;
                        modify.push(PropertyModification::new(owner, property.clone()));
                        next.node_types
                            .get_mut(&node.name)
                            .expect("held above")
                            .properties
                            .insert(property.name.clone(), property);
                    }
                    None => match next.inherited(&node.name, &property.name) {
                        Some(held) if same_declaration(held, &property) => {}
                        Some(_) => {
                            return Err(OntologyError::Conflict {
                                kind: "property",
                                name: format!("{}.{}", node.name, property.name),
                                reason: "an ancestor declares it differently; change it there"
                                    .to_owned(),
                            })
                        }
                        None => {
                            modify.push(PropertyModification::new(owner, property.clone()));
                            next.node_types
                                .get_mut(&node.name)
                                .expect("held above")
                                .properties
                                .insert(property.name.clone(), property);
                        }
                    },
                }
            }
        }
        let define_nodes = parents_first(define_nodes, &new_nodes, &next)?;

        let mut define_edges = Vec::new();
        let mut widen = Vec::new();
        for edge in &spec.edge_types {
            let ends = |names: &[String]| -> Result<BTreeSet<TypeId>, OntologyError> {
                names.iter().map(|name| next.node_id(name)).collect()
            };
            let (sources, targets) = (ends(&edge.source_types)?, ends(&edge.target_types)?);
            let wanted: Vec<PropertyDefinition> = edge
                .properties
                .iter()
                .map(|property| next.definition(property))
                .collect::<Result<_, _>>()?;
            let Some(held) = next.edge_types.get(&edge.name).cloned() else {
                let id = TypeId::mint();
                next.insert_edge(
                    edge.name.clone(),
                    EdgeEntry {
                        id,
                        source_types: sources.clone(),
                        target_types: targets.clone(),
                        cardinality: edge.cardinality,
                        properties: wanted
                            .iter()
                            .map(|property| (property.name.clone(), property.clone()))
                            .collect(),
                    },
                )?;
                define_edges.push(EdgeType {
                    id,
                    name: edge.name.clone(),
                    source_types: sources,
                    target_types: targets,
                    cardinality: edge.cardinality,
                    properties: wanted
                        .into_iter()
                        .map(|property| (property.id, property))
                        .collect(),
                    inverse: None,
                    symmetric: false,
                    transitive: false,
                });
                continue;
            };
            if held.cardinality != edge.cardinality {
                return Err(OntologyError::Conflict {
                    kind: "edge type",
                    name: edge.name.clone(),
                    reason: format!(
                        "the store declares cardinality {:?}, and no schema operation changes it",
                        held.cardinality
                    ),
                });
            }
            if !sources.is_subset(&held.source_types) || !targets.is_subset(&held.target_types) {
                let source_types: BTreeSet<TypeId> =
                    held.source_types.union(&sources).copied().collect();
                let target_types: BTreeSet<TypeId> =
                    held.target_types.union(&targets).copied().collect();
                widen.push(EdgeWidening::new(
                    held.id,
                    source_types.iter().copied(),
                    target_types.iter().copied(),
                ));
                let entry = next.edge_types.get_mut(&edge.name).expect("held above");
                entry.source_types = source_types;
                entry.target_types = target_types;
            }
            for mut property in wanted {
                match held.properties.get(&property.name) {
                    Some(own) if same_declaration(own, &property) => continue,
                    Some(own) => property.id = own.id,
                    None => {}
                }
                modify.push(PropertyModification::new(held.id, property.clone()));
                next.edge_types
                    .get_mut(&edge.name)
                    .expect("held above")
                    .properties
                    .insert(property.name.clone(), property);
            }
        }
        Ok(Plan {
            define_nodes,
            define_edges,
            modify,
            widen,
            next,
        })
    }
}

/// Refuses a spec that declares a node type, an edge type or one type's property twice.
fn check_unique(spec: &OntologySpec) -> Result<(), OntologyError> {
    let twice = |kind: &'static str, name: &str| OntologyError::DuplicateName {
        kind,
        name: name.to_owned(),
    };
    let mut nodes = BTreeSet::new();
    for node in &spec.node_types {
        if !nodes.insert(node.name.as_str()) {
            return Err(twice("node type", &node.name));
        }
        let mut properties = BTreeSet::new();
        for property in &node.properties {
            if !properties.insert(property.name.as_str()) {
                return Err(twice(
                    "property",
                    &format!("{}.{}", node.name, property.name),
                ));
            }
        }
    }
    let mut edges = BTreeSet::new();
    for edge in &spec.edge_types {
        if !edges.insert(edge.name.as_str()) {
            return Err(twice("edge type", &edge.name));
        }
        let mut properties = BTreeSet::new();
        for property in &edge.properties {
            if !properties.insert(property.name.as_str()) {
                return Err(twice(
                    "property",
                    &format!("{}.{}", edge.name, property.name),
                ));
            }
        }
    }
    Ok(())
}

/// The new node types ordered so that each follows every new parent it names.
fn parents_first(
    mut pending: Vec<NodeType>,
    new_nodes: &BTreeSet<String>,
    next: &Ontology,
) -> Result<Vec<NodeType>, OntologyError> {
    let new_ids: BTreeSet<TypeId> = new_nodes
        .iter()
        .filter_map(|name| next.node_type(name))
        .collect();
    let mut placed = BTreeSet::new();
    let mut ordered = Vec::with_capacity(pending.len());
    while !pending.is_empty() {
        let ready = pending.iter().position(|node| {
            node.parents
                .iter()
                .all(|parent| !new_ids.contains(parent) || placed.contains(parent))
        });
        let Some(at) = ready else {
            return Err(OntologyError::Conflict {
                kind: "node type",
                name: pending[0].name.clone(),
                reason: "its parents form a cycle".to_owned(),
            });
        };
        let node = pending.remove(at);
        placed.insert(node.id);
        ordered.push(node);
    }
    Ok(ordered)
}

/// The schema operations [`Ontology::ensure`] found missing, and the map once they commit.
#[derive(Clone, Debug, PartialEq)]
pub struct SchemaChange {
    operations: Vec<Operation>,
    ontology: Ontology,
}

impl SchemaChange {
    /// Whether the store already declares everything the spec does.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// The operations: node types, then edge types, then properties, then widenings.
    #[must_use]
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    /// The map once the operations commit, holding the ids minted for what they add.
    #[must_use]
    pub const fn ontology(&self) -> &Ontology {
        &self.ontology
    }

    /// The one schema-change transaction `proposer` proposes, naming a freshly minted schema
    /// version; `None` when nothing is missing.
    ///
    /// # Errors
    /// None in practice: the operations are all schema changes and there is at least one.
    pub fn transaction(
        &self,
        proposer: AgentId,
    ) -> Result<Option<TransactionDocument>, DocumentError> {
        if self.is_empty() {
            return Ok(None);
        }
        self.operations
            .iter()
            .cloned()
            .fold(TransactionBuilder::new(proposer), TransactionBuilder::push)
            .build()
            .map(Some)
    }
}
