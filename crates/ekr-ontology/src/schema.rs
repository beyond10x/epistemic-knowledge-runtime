//! A schema version and the registry of the types it declares: design § 69 `SchemaRegistry`.
//!
//! One version is canonical at a time, and in P1 the seed is the only one — schema versions are
//! data here, and the transactions that move a version arrive in P5 (design § 26). What this
//! module owns is the step from a *document* someone wrote to an [`Ontology`] the checker may be
//! run against, and that step refuses: an ontology whose declarations do not cohere cannot decide
//! anything about a value, so it is not loaded at all rather than loaded and consulted.

use std::fmt;

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{PropertyId, SchemaVersionId, Timestamp, TypeId};
use serde::{Deserialize, Serialize};

use crate::lifecycle::Transition;
use crate::types::{EdgeType, NodeType, PropertyDefinition};
use crate::value::{ValueKind, ValueType};

/// One version of the schema: `ekr.ontology.SchemaVersion` of `systems/ekr/domains/ontology.yaml`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

impl SchemaVersion {
    /// The seed version: number zero, no parent.
    #[must_use]
    pub const fn seed(id: SchemaVersionId, created_at: Timestamp) -> Self {
        Self {
            id,
            number: 0,
            parent: None,
            created_at,
        }
    }
}

/// The written form of an ontology: a schema version and the types it declares.
///
/// This is what a document carries and what serde reads. It is not an [`Ontology`]: nothing has
/// checked that its declarations cohere, so nothing may be checked against it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// The types of one schema version, loaded and coherent: design § 69 `SchemaRegistry`.
///
/// Held only through [`Ontology::load`], so every instance has passed the refusals below.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ontology {
    version: SchemaVersion,
    node_types: BTreeMap<TypeId, NodeType>,
    edge_types: BTreeMap<TypeId, EdgeType>,
}

impl Ontology {
    /// Loads a document, refusing one whose declarations do not cohere.
    ///
    /// # Errors
    ///
    /// See [`OntologyError`]. Each refusal is a declaration that would leave the checker unable to
    /// answer: a reference to nothing, a reference to a type that is not there, a parent cycle
    /// with no ancestors to enumerate, a lifecycle naming a state it does not have.
    pub fn load(document: OntologyDocument) -> Result<Self, OntologyError> {
        // Declaration order is kept, and every refusal below is made in it. The registry itself is
        // keyed by `TypeId`, and walking *that* would make the reason a two-fault document is
        // refused a function of which id sorted first — two readers of one document sent after two
        // different defects. The order a document is written in is the order it is read in.
        let mut order: Vec<TypeId> = Vec::with_capacity(document.node_types.len());
        let mut node_types: BTreeMap<TypeId, NodeType> = BTreeMap::new();
        for declared in document.node_types {
            let type_id = declared.id;
            if node_types.insert(type_id, declared).is_some() {
                return Err(OntologyError::DuplicateType { type_id });
            }
            order.push(type_id);
        }
        let mut edge_order: Vec<TypeId> = Vec::with_capacity(document.edge_types.len());
        let mut edge_types: BTreeMap<TypeId, EdgeType> = BTreeMap::new();
        for declared in document.edge_types {
            let type_id = declared.id;
            if node_types.contains_key(&type_id) || edge_types.insert(type_id, declared).is_some() {
                return Err(OntologyError::DuplicateType { type_id });
            }
            edge_order.push(type_id);
        }

        let loaded = Self {
            version: document.version,
            node_types,
            edge_types,
        };
        loaded.check_declarations(&order, &edge_order)?;
        Ok(loaded)
    }

    /// Every refusal [`Ontology::load`] makes, over an ontology that is built but not yet handed
    /// out, in the order the document declared its types.
    fn check_declarations(
        &self,
        order: &[TypeId],
        edge_order: &[TypeId],
    ) -> Result<(), OntologyError> {
        // The hierarchy closes, everywhere, before anything is asked of it. `ancestors` says *how*
        // it fails to: an ancestor that is not declared and a cycle are two different defects and
        // send a reader two different ways.
        //
        // This is a pass of its own rather than a step in the loop below, because a cycle among
        // two types is reachable from a third whose own walk terminates — and resolving that
        // third type's properties against a cyclic ancestor would report an ambiguity in place of
        // the cycle that caused it.
        for type_id in order {
            self.ancestors(*type_id)?;
        }

        for type_id in order {
            let declared = &self.node_types[type_id];
            // Resolving the property set is itself a refusal: two ancestors neither of which
            // specialises the other, declaring one property differently, leave nothing to choose.
            self.resolve_properties(*type_id)?;
            self.check_properties(&declared.properties)?;
            self.check_lifecycle(declared)?;
        }

        for declared in edge_order.iter().map(|type_id| &self.edge_types[type_id]) {
            // An edge from nothing to nothing is the empty `NodeRef` one level up.
            for (endpoint, types) in [
                ("source_types", &declared.source_types),
                ("target_types", &declared.target_types),
            ] {
                if types.is_empty() {
                    return Err(OntologyError::EmptyEdgeEndpoint {
                        type_id: declared.id,
                        endpoint: endpoint.to_owned(),
                    });
                }
                for type_id in types {
                    if !self.node_types.contains_key(type_id) {
                        return Err(OntologyError::UnknownType { type_id: *type_id });
                    }
                }
            }
            if let Some(inverse) = declared.inverse {
                if !self.edge_types.contains_key(&inverse) {
                    return Err(OntologyError::UnknownType { type_id: inverse });
                }
            }
            self.check_properties(&declared.properties)?;
        }

        Ok(())
    }

    /// Every property is filed by its identity and its value type names inhabitable declarations.
    fn check_properties(
        &self,
        properties: &BTreeMap<PropertyId, PropertyDefinition>,
    ) -> Result<(), OntologyError> {
        for (key, definition) in properties {
            if *key != definition.id {
                return Err(OntologyError::MisfiledProperty {
                    key: *key,
                    declared: definition.id,
                });
            }
            self.check_value_type(
                &DeclarationSite::Property(definition.id),
                &definition.value_type,
            )?;
        }
        Ok(())
    }

    /// One declared value type, at every depth: it describes values a caller can produce, and it
    /// names only types this ontology declares.
    ///
    /// Every position a `ValueType` can be declared in goes through here — a node type's
    /// property, an edge type's property and an operation's argument — because a position that
    /// does not is a declaration that loads uninhabitable, which is the one thing `load` exists to
    /// prevent. `OperationDefinition::arguments` was such a position until round 1 of review.
    fn check_value_type(
        &self,
        site: &DeclarationSite,
        value_type: &ValueType,
    ) -> Result<(), OntologyError> {
        let mut reached = Vec::new();
        reachable(value_type, &mut reached);
        for reached_type in reached {
            match reached_type {
                ValueType::NodeRef { ref allowed_types } => {
                    if allowed_types.is_empty() {
                        return Err(OntologyError::EmptyValueType {
                            site: site.clone(),
                            kind: ValueKind::NodeRef,
                        });
                    }
                    for type_id in allowed_types {
                        if !self.node_types.contains_key(type_id) {
                            return Err(OntologyError::UnknownType { type_id: *type_id });
                        }
                    }
                }
                ValueType::Enum { ref variants } if variants.is_empty() => {
                    return Err(OntologyError::EmptyValueType {
                        site: site.clone(),
                        kind: ValueKind::Enum,
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// A lifecycle names only states it has, and an operation moves a node only a way the
    /// lifecycle declares — including the case of an operation on a type with no lifecycle at all,
    /// which declares no move and therefore permits none.
    fn check_lifecycle(&self, declared: &NodeType) -> Result<(), OntologyError> {
        if let Some(lifecycle) = &declared.lifecycle {
            let mut named = vec![lifecycle.initial.clone()];
            for transition in &lifecycle.transitions {
                named.push(transition.from.clone());
                named.push(transition.to.clone());
            }
            for state in named {
                if !lifecycle.states.contains(&state) {
                    return Err(OntologyError::UnknownState {
                        type_id: declared.id,
                        state,
                    });
                }
            }
        }

        for (name, operation) in &declared.operations {
            // Amendment 87's operations are part of the ontology, not an annex to it: an argument
            // type is a declared type and is held to what every other declared type is held to.
            for (argument, value_type) in &operation.arguments {
                self.check_value_type(
                    &DeclarationSite::OperationArgument {
                        type_id: declared.id,
                        operation: name.clone(),
                        argument: argument.clone(),
                    },
                    value_type,
                )?;
            }

            let Some(transition) = &operation.transition else {
                continue;
            };
            let permitted = declared
                .lifecycle
                .as_ref()
                .is_some_and(|lifecycle| lifecycle.declares(transition));
            if !permitted {
                return Err(OntologyError::TransitionNotDeclared {
                    type_id: declared.id,
                    operation: name.clone(),
                    transition: transition.clone(),
                });
            }
        }

        Ok(())
    }

    /// Loads a schema document written as YAML.
    ///
    /// # Errors
    ///
    /// [`OntologyError::Syntax`] if the text is not a schema document, and everything
    /// [`Ontology::load`] refuses otherwise.
    pub fn from_yaml(text: &str) -> Result<Self, OntologyError> {
        let document: OntologyDocument =
            serde_yaml_ng::from_str(text).map_err(|e| OntologyError::Syntax(e.to_string()))?;
        Self::load(document)
    }

    /// The version these types belong to.
    #[must_use]
    pub const fn version(&self) -> &SchemaVersion {
        &self.version
    }

    /// The node type with this id, if the ontology declares one.
    #[must_use]
    pub fn node_type(&self, type_id: TypeId) -> Option<&NodeType> {
        self.node_types.get(&type_id)
    }

    /// The edge type with this id, if the ontology declares one.
    #[must_use]
    pub fn edge_type(&self, type_id: TypeId) -> Option<&EdgeType> {
        self.edge_types.get(&type_id)
    }

    /// Whether a node of `type_id` may stand where `ancestor` is asked for: design § 12's
    /// "compatible with".
    ///
    /// A type conforms to itself and to every ancestor, transitively, and to nothing else. An
    /// undeclared type conforms to nothing, including itself: the ontology cannot say what it is.
    #[must_use]
    pub fn conforms_to(&self, type_id: TypeId, ancestor: TypeId) -> bool {
        self.ancestors(type_id)
            .is_ok_and(|found| found.contains(&ancestor))
    }

    /// Every property a node of `type_id` may carry: its own and its ancestors', with the most
    /// *specific* declaration of a property winning.
    ///
    /// Empty for an undeclared type, which carries nothing because it declares nothing.
    ///
    /// "Most specific" is the specialisation order the document itself declares — a declaration in
    /// `V` beats one in `W` exactly when [`Ontology::conforms_to(V, W)`](Ontology::conforms_to) —
    /// and it is not distance. The two agree on a chain and part company as soon as a type reaches
    /// one ancestor by two routes of different length, which an ordinary document does whenever it
    /// names both a base and a refinement of that base. Two earlier rules were wrong here: a merge
    /// over the ancestor *set* let the highest `TypeId` win, and a breadth-first merge let the
    /// nearer of two declarations win even when the farther one was a redeclaration of it.
    #[must_use]
    pub fn properties_of(&self, type_id: TypeId) -> BTreeMap<PropertyId, &PropertyDefinition> {
        // A loaded ontology has already been resolved once by `check_declarations`, so the error
        // arm is unreachable for any `Ontology` that exists; an undeclared type resolves to the
        // empty map rather than to an error.
        self.resolve_properties(type_id).unwrap_or_default()
    }

    /// [`Ontology::properties_of`], with the refusal it is resolved under.
    ///
    /// For each property, the declaring types among `type_id`'s ancestors are ordered by
    /// specialisation and the maximal ones taken — those no other declarer specialises. One
    /// maximal declarer is the answer. Several maximal declarers saying the same thing is also an
    /// answer, because there is nothing to choose between. Several saying different things is a
    /// document that does not choose: no declaring type refines another, so every remaining
    /// tiebreak would be an id order, and the document is refused instead.
    ///
    /// # Errors
    ///
    /// [`OntologyError::AmbiguousProperty`] if two ancestors of `type_id`, neither of which
    /// specialises the other, declare one property differently.
    fn resolve_properties(
        &self,
        type_id: TypeId,
    ) -> Result<BTreeMap<PropertyId, &PropertyDefinition>, OntologyError> {
        // An undeclared type declares nothing; a hierarchy that does not close is reported by
        // `check_declarations` through `ancestors` directly, and not a second time from here.
        let Ok(closure) = self.ancestors(type_id) else {
            return Ok(BTreeMap::new());
        };

        // Who declares what, and the ancestry of each declarer — computed once per type rather
        // than once per comparison, since `conforms_to` walks the hierarchy on every call.
        let mut ancestry: BTreeMap<TypeId, BTreeSet<TypeId>> = BTreeMap::new();
        let mut declarers: BTreeMap<PropertyId, Vec<TypeId>> = BTreeMap::new();
        for at in &closure {
            let Some(declared) = self.node_types.get(at) else {
                continue;
            };
            if declared.properties.is_empty() {
                continue;
            }
            ancestry.insert(*at, self.ancestors(*at).unwrap_or_default());
            for property in declared.properties.keys() {
                declarers.entry(*property).or_default().push(*at);
            }
        }

        let definition_of = |at: &TypeId, property: &PropertyId| {
            self.node_types[at]
                .properties
                .get(property)
                .expect("a declarer of a property declares it")
        };

        let mut resolved: BTreeMap<PropertyId, &PropertyDefinition> = BTreeMap::new();
        for (property, candidates) in declarers {
            // `at` is maximal when no *other* declarer specialises it. `ancestry[other]` holds
            // `other` itself, so the `other != at` guard is what keeps a type from excluding
            // itself.
            let maximal: Vec<&TypeId> = candidates
                .iter()
                .filter(|at| {
                    !candidates
                        .iter()
                        .any(|other| other != *at && ancestry[other].contains(at))
                })
                .collect();

            let Some(most_specific) = maximal.first() else {
                // Unreachable for a document that has passed the hierarchy pass of
                // `check_declarations`: specialisation is then a strict partial order, and a
                // finite non-empty set under one has a maximal element. Refusing rather than
                // indexing keeps a future cycle from becoming a panic.
                return Err(OntologyError::AmbiguousProperty { type_id, property });
            };
            let first = definition_of(most_specific, &property);
            if maximal.len() > 1
                && maximal
                    .iter()
                    .any(|at| definition_of(at, &property) != first)
            {
                return Err(OntologyError::AmbiguousProperty { type_id, property });
            }
            resolved.insert(property, first);
        }

        Ok(resolved)
    }

    /// Every ancestor of `type_id`, itself included, or why the hierarchy does not close.
    ///
    /// # Errors
    ///
    /// [`OntologyError::UnknownType`] if `type_id` or any type reachable through its parents is
    /// not declared, and [`OntologyError::CyclicParents`] if `type_id` is its own strict ancestor.
    /// The two were one `None` until round 1 of review, and the caller guessed `CyclicParents` for
    /// both — so an absent grandparent was reported as a cycle the document did not contain, and
    /// a reader was sent after a defect that was not there.
    ///
    /// The cycle is detected by reaching `type_id` again and not by exhausting a visited set: a
    /// visited set terminates on a cycle just as it terminates on a diamond, so a walk that only
    /// dedupes reports no cycle at all.
    fn ancestors(&self, type_id: TypeId) -> Result<BTreeSet<TypeId>, OntologyError> {
        let declared = |at: TypeId| {
            self.node_types
                .get(&at)
                .ok_or(OntologyError::UnknownType { type_id: at })
        };

        let mut found = BTreeSet::new();
        found.insert(type_id);
        let mut pending: Vec<TypeId> = declared(type_id)?.parents.iter().copied().collect();
        while let Some(next) = pending.pop() {
            if next == type_id {
                return Err(OntologyError::CyclicParents { type_id });
            }
            if !found.insert(next) {
                continue;
            }
            pending.extend(declared(next)?.parents.iter().copied());
        }
        Ok(found)
    }
}

/// Where a `ValueType` was declared, so that a refusal about one names the declaration and not
/// merely the kind.
///
/// Design § 11.2 gives a property an id to be named by; amendment 87 gives an operation argument
/// a name within an operation within a type, and no id at all. One refusal covers both, so it
/// carries whichever of the two the declaration actually has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclarationSite {
    /// A property definition's `value_type`, on a node type or an edge type.
    Property(PropertyId),
    /// An `OperationDefinition::arguments` entry (amendment 87).
    OperationArgument {
        /// The node type the operation belongs to.
        type_id: TypeId,
        /// The operation's name.
        operation: String,
        /// The argument's name.
        argument: String,
    },
}

impl fmt::Display for DeclarationSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Property(property) => write!(f, "property {property}"),
            Self::OperationArgument {
                type_id,
                operation,
                argument,
            } => write!(
                f,
                "argument {argument:?} of operation {operation:?} on {type_id}"
            ),
        }
    }
}

/// Why a document was not loaded as an ontology.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OntologyError {
    /// The text is not a schema document at all.
    #[error("the schema document does not parse: {0}")]
    Syntax(String),
    /// A property map indexes a definition by a different stable identity.
    #[error("property map key {key} disagrees with definition id {declared}")]
    MisfiledProperty {
        /// The identity used to look up the property.
        key: PropertyId,
        /// The identity carried by the definition itself.
        declared: PropertyId,
    },
    /// Two declarations carry the same type id.
    #[error("{type_id} is declared twice")]
    DuplicateType {
        /// The id declared more than once.
        type_id: TypeId,
    },
    /// A declaration names a type the document does not declare.
    #[error("{type_id} is referenced and not declared")]
    UnknownType {
        /// The id that is named and not declared.
        type_id: TypeId,
    },
    /// A compound value type carries no parameters, so no value inhabits it.
    #[error("the {kind} declared at {site} declares nothing it accepts")]
    EmptyValueType {
        /// Where the uninhabitable type was declared.
        site: DeclarationSite,
        /// Which compound kind it was.
        kind: ValueKind,
    },
    /// Two ancestors, neither of which specialises the other, declare one property differently.
    #[error(
        "{type_id} inherits two different declarations of property {property} from types neither \
         of which specialises the other, so the hierarchy does not choose between them"
    )]
    AmbiguousProperty {
        /// The type whose property set cannot be resolved.
        type_id: TypeId,
        /// The property two unrelated ancestors declare differently.
        property: PropertyId,
    },
    /// An edge type declares no source types or no target types.
    #[error("edge type {type_id} declares no {endpoint}")]
    EmptyEdgeEndpoint {
        /// The edge type.
        type_id: TypeId,
        /// `source_types` or `target_types`.
        endpoint: String,
    },
    /// A type's parents form a cycle, so its ancestors cannot be enumerated.
    #[error("the parents of {type_id} form a cycle")]
    CyclicParents {
        /// A type on the cycle.
        type_id: TypeId,
    },
    /// A lifecycle names a state it does not have.
    #[error("the lifecycle of {type_id} names {state:?}, which is not one of its states")]
    UnknownState {
        /// The type whose lifecycle it is.
        type_id: TypeId,
        /// The state that is named and not declared.
        state: String,
    },
    /// An operation moves a node a way its type's lifecycle does not declare.
    #[error("operation {operation:?} of {type_id} moves {:?} -> {:?}, which its lifecycle does not declare", transition.from, transition.to)]
    TransitionNotDeclared {
        /// The type the operation belongs to.
        type_id: TypeId,
        /// The operation's name.
        operation: String,
        /// The move it declares.
        transition: Transition,
    },
}

/// Every value type reachable from this one, itself included.
fn reachable(value_type: &ValueType, out: &mut Vec<ValueType>) {
    out.push(value_type.clone());
    match value_type {
        ValueType::List(element) => reachable(element, out),
        ValueType::Record(fields) => {
            for field in fields.values() {
                reachable(field, out);
            }
        }
        _ => {}
    }
}
