//! Schema evolution: how one version of the ontology becomes the next (design § 26).
//!
//! Two questions, deliberately answered apart:
//!
//! * [`Ontology::evolve`] — **does the next version cohere?** It is answered from the ontology
//!   alone. The next version is the prior one with `number + 1`, `parent = Some(prior)`, and the
//!   [`SchemaChange`]s applied in order; it is then held to every refusal [`Ontology::load`]
//!   makes, by being loaded.
//! * [`incompatibilities`] — **may it replace the prior one while canonical state is what it
//!   is?** The ontology holds no graph state (`AGENTS.md` invariant 8), so the kernel answers the
//!   questions this needs through [`InstanceState`], which names types and properties and nothing
//!   a graph declares.
//!
//! Nothing here removes a type or a property: no schema change does (wave p5-01, decision 6).
//! [`incompatibilities`] still refuses a removal, because it compares any two ontologies and a
//! second route to `next` must not pass one as compatible.
//!
//! The encoding of a [`SchemaChange`] is not this module's. The kernel encodes operations
//! (`crates/ekr-kernel/src/transaction.rs`), and a change reaches it as one.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{PropertyId, SchemaVersionId, Timestamp, TypeId};

use crate::schema::{Ontology, OntologyDocument, OntologyError, SchemaVersion};
use crate::types::{EdgeType, NodeType, PropertyDefinition};
use crate::value::{Cardinality, ValueKind, ValueType};

/// One schema change, as the three kernel operations carry it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemaChange {
    /// Declare a node type no version before has declared.
    DefineNodeType(NodeType),
    /// Declare an edge type no version before has declared.
    DefineEdgeType(EdgeType),
    /// Add a property to `owner`, or redeclare one it already declares.
    ModifyProperty {
        /// The node type or edge type that declares the property.
        owner: TypeId,
        /// The whole new declaration, filed under its own id.
        property: PropertyDefinition,
    },
}

impl Ontology {
    /// The next version: `number + 1`, `parent = Some(self.version().id)`, `id = next`, then the
    /// changes applied in order. The result passes every refusal `Ontology::load` applies.
    ///
    /// "In order" is observable: a type defined by an earlier change may own a property a later
    /// change modifies, and the reverse order is [`EvolveError::UnknownOwner`].
    ///
    /// **Lineage uniqueness is checked only as far as this version can see.** `next` is refused
    /// when it is this version's id or its parent's; an `Ontology` holds no older ancestor, so
    /// that `next` appears nowhere else on the lineage is the kernel's to check against the
    /// versions it has committed.
    ///
    /// # Errors
    ///
    /// See [`EvolveError`]. The refusals that belong to a change are made first, as each change
    /// is applied; a result that does not cohere is [`EvolveError::Incoherent`], carrying the
    /// [`OntologyError`] `load` gave; a result that declares exactly what this version declares
    /// is [`EvolveError::WithoutEffect`].
    pub fn evolve(
        &self,
        next: SchemaVersionId,
        created_at: Timestamp,
        changes: &[SchemaChange],
    ) -> Result<Self, EvolveError> {
        if changes.is_empty() {
            return Err(EvolveError::EmptyChanges);
        }
        let prior = self.version();
        if next == prior.id || prior.parent == Some(next) {
            return Err(EvolveError::VersionIdReused { id: next });
        }
        let Some(number) = prior.number.checked_add(1) else {
            return Err(EvolveError::VersionNumberExhausted { id: prior.id });
        };

        let mut document = self.to_document();
        document.version = SchemaVersion {
            id: next,
            number,
            parent: Some(prior.id),
            created_at,
        };

        for change in changes {
            match change {
                SchemaChange::DefineNodeType(declared) => {
                    refuse_declared(&document, declared.id)?;
                    document.node_types.push(declared.clone());
                }
                SchemaChange::DefineEdgeType(declared) => {
                    refuse_declared(&document, declared.id)?;
                    document.edge_types.push(declared.clone());
                }
                SchemaChange::ModifyProperty { owner, property } => {
                    let properties = if let Some(declared) = document
                        .node_types
                        .iter_mut()
                        .find(|declared| declared.id == *owner)
                    {
                        &mut declared.properties
                    } else if let Some(declared) = document
                        .edge_types
                        .iter_mut()
                        .find(|declared| declared.id == *owner)
                    {
                        &mut declared.properties
                    } else {
                        return Err(EvolveError::UnknownOwner { owner: *owner });
                    };
                    properties.insert(property.id, property.clone());
                }
            }
        }

        let evolved = Self::load(document).map_err(EvolveError::Incoherent)?;
        // Changes that cancel, or a redeclaration identical to what is declared, derive a version
        // that differs from this one only in its version record.
        let (before, after) = (self.to_document(), evolved.to_document());
        if before.node_types == after.node_types && before.edge_types == after.edge_types {
            return Err(EvolveError::WithoutEffect);
        }
        Ok(evolved)
    }
}

/// A type id is one namespace across node and edge types, and a version under construction
/// declares what the prior one did plus every change applied so far.
fn refuse_declared(document: &OntologyDocument, type_id: TypeId) -> Result<(), EvolveError> {
    let declared = document.node_types.iter().any(|at| at.id == type_id)
        || document.edge_types.iter().any(|at| at.id == type_id);
    if declared {
        Err(EvolveError::TypeAlreadyDeclared { type_id })
    } else {
        Ok(())
    }
}

/// Why [`Ontology::evolve`] produced no next version.
///
/// `Display` leads with the stable kebab-case [`code`](EvolveError::code), which
/// `systems/ekr/domains/ontology.yaml` declares as `ekr.ontology.EvolveRefusalCode`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EvolveError {
    /// No change at all: a version that changes nothing is not a next version.
    #[error("empty-schema-change: a schema version needs at least one change")]
    EmptyChanges,
    /// The next version's id is already on the lineage this version can see: its own id, so the
    /// next version's parent would be itself, or its parent's.
    #[error("schema-version-reused: {id} is already on the prior version's lineage")]
    VersionIdReused {
        /// The id offered for the next version.
        id: SchemaVersionId,
    },
    /// The prior version's number is the largest there is.
    #[error("schema-version-exhausted: {id} has the largest version number there is")]
    VersionNumberExhausted {
        /// The prior version.
        id: SchemaVersionId,
    },
    /// A `DefineNodeType` or `DefineEdgeType` whose id the version already declares — the prior
    /// version, or an earlier change of the same list.
    #[error("type-already-declared: {type_id} is already declared by this version")]
    TypeAlreadyDeclared {
        /// The id declared again.
        type_id: TypeId,
    },
    /// A `ModifyProperty` whose owner the version does not declare.
    #[error(
        "unknown-property-owner: {owner} is neither a node type nor an edge type of this version"
    )]
    UnknownOwner {
        /// The owner named.
        owner: TypeId,
    },
    /// The changed version fails a coherence rule [`Ontology::load`] applies.
    #[error("incoherent-schema: {0}")]
    Incoherent(OntologyError),
    /// The changes, applied in order, leave every declaration as the prior version has it: an
    /// identical redeclaration, or changes that cancel.
    #[error("schema-change-without-effect: the changes leave every declaration as it was")]
    WithoutEffect,
}

impl EvolveError {
    /// Every code [`EvolveError::code`] can return.
    pub const CODES: [&'static str; 7] = [
        "empty-schema-change",
        "schema-version-reused",
        "schema-version-exhausted",
        "type-already-declared",
        "unknown-property-owner",
        "incoherent-schema",
        "schema-change-without-effect",
    ];

    /// The stable kebab-case code of this refusal.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyChanges => Self::CODES[0],
            Self::VersionIdReused { .. } => Self::CODES[1],
            Self::VersionNumberExhausted { .. } => Self::CODES[2],
            Self::TypeAlreadyDeclared { .. } => Self::CODES[3],
            Self::UnknownOwner { .. } => Self::CODES[4],
            Self::Incoherent(_) => Self::CODES[5],
            Self::WithoutEffect => Self::CODES[6],
        }
    }
}

/// What the kernel can tell the ontology about canonical state, without the ontology naming graph
/// types (AGENTS.md invariant 8).
///
/// `owner` is the type an instance *is* — a node's own type, an edge's own type — and not any
/// ancestor of it: [`incompatibilities`] asks about every type whose resolved property set moved,
/// inherited properties included, so a count that also included subtypes would be counted twice.
///
/// **Values are top-level.** A value is one entry of the list a property holds on one instance —
/// what [`PropertyDefinition::cardinality`] bounds — and a [`Value::List`](crate::Value::List) is
/// one value however many elements it has. Elements and record fields are never counted and their
/// kinds are never reported; the parameters below that level are compared by declaration.
pub trait InstanceState {
    /// How many nodes of exactly this type canonical state holds.
    fn node_count(&self, node_type: TypeId) -> u64;
    /// How many edges of exactly this type canonical state holds.
    fn edge_count(&self, edge_type: TypeId) -> u64;
    /// The largest number of values any instance of `owner` holds for `property`; 0 if none.
    ///
    /// Counts top-level values: a property holding one list of five elements holds one value.
    fn max_values(&self, owner: TypeId, property: PropertyId) -> u64;
    /// The smallest number of values any instance of `owner` holds for `property`; 0 if any
    /// instance lacks it or none exists.
    ///
    /// Counts top-level values, as [`InstanceState::max_values`] does.
    fn min_values(&self, owner: TypeId, property: PropertyId) -> u64;
    /// The value kinds held for `property` on instances of `owner`.
    ///
    /// The kinds of top-level values only: a list of integers is reported as
    /// [`ValueKind::List`], never as [`ValueKind::Integer`].
    fn value_kinds(&self, owner: TypeId, property: PropertyId) -> BTreeSet<ValueKind>;
}

/// Why `next` cannot replace `prior` while canonical state is as `state` says.
///
/// Empty when it can. A lineage refusal comes first; the rest are ordered by owner, node types
/// before edge types, then by property, so two readers of one answer read it the same way.
///
/// First, `next` must be `prior`'s successor: its parent is `prior` and its number is one more.
/// Then, for each type `prior` declares: a type `next` no longer declares is refused if it has
/// instances; a type whose declaration moved in anything other than its properties is refused if
/// it, or any type conforming to it in either version, has instances, because `state` cannot say
/// whether they still conform; and every property whose resolved declaration moved is checked
/// against what instances hold. A changed constraint is refused under any instance: nothing can
/// evaluate one yet, so whether instances satisfy it cannot be shown.
#[must_use]
pub fn incompatibilities(
    prior: &Ontology,
    next: &Ontology,
    state: &dyn InstanceState,
) -> Vec<Incompatibility> {
    let mut found = Vec::new();

    let (was, is) = (prior.version(), next.version());
    if is.id == was.id || is.parent != Some(was.id) || was.number.checked_add(1) != Some(is.number)
    {
        found.push(Incompatibility::NotASuccessor {
            prior: was.id,
            next: is.id,
        });
    }

    // Every node of `type_id` or of a type that specialises it, in either version: a type-level
    // change to an abstract parent moves the conformance of its concrete children.
    let node_types: BTreeSet<TypeId> = prior
        .node_types()
        .chain(next.node_types())
        .map(|(id, _)| *id)
        .collect();
    let population = |type_id: TypeId| {
        node_types
            .iter()
            .filter(|at| prior.conforms_to(**at, type_id) || next.conforms_to(**at, type_id))
            .map(|at| state.node_count(*at))
            .fold(0_u64, u64::saturating_add)
    };

    for (type_id, before) in prior.node_types() {
        let type_id = *type_id;
        let instances = state.node_count(type_id);
        let Some(after) = next.node_type(type_id) else {
            let instances = population(type_id);
            if instances > 0 {
                found.push(Incompatibility::TypeRemoved { type_id, instances });
            }
            continue;
        };
        if !same_apart_from_properties_node(before, after) {
            let instances = population(type_id);
            if instances > 0 {
                found.push(Incompatibility::DeclarationChanged { type_id, instances });
            }
        }
        compare_properties(
            &Owner {
                type_id,
                instances,
                next,
                state,
            },
            &prior.properties_of(type_id),
            &next.properties_of(type_id),
            &mut found,
        );
    }

    for (type_id, before) in prior.edge_types() {
        let type_id = *type_id;
        let instances = state.edge_count(type_id);
        let Some(after) = next.edge_type(type_id) else {
            if instances > 0 {
                found.push(Incompatibility::TypeRemoved { type_id, instances });
            }
            continue;
        };
        if instances > 0 && !same_apart_from_properties_edge(before, after) {
            found.push(Incompatibility::DeclarationChanged { type_id, instances });
        }
        compare_properties(
            &Owner {
                type_id,
                instances,
                next,
                state,
            },
            &before.properties.iter().map(|(id, at)| (*id, at)).collect(),
            &after.properties.iter().map(|(id, at)| (*id, at)).collect(),
            &mut found,
        );
    }

    found
}

fn same_apart_from_properties_node(before: &NodeType, after: &NodeType) -> bool {
    let strip = |declared: &NodeType| NodeType {
        properties: BTreeMap::new(),
        ..declared.clone()
    };
    strip(before) == strip(after)
}

fn same_apart_from_properties_edge(before: &EdgeType, after: &EdgeType) -> bool {
    let strip = |declared: &EdgeType| EdgeType {
        properties: BTreeMap::new(),
        ..declared.clone()
    };
    strip(before) == strip(after)
}

/// The type whose instances a property comparison is about.
struct Owner<'a> {
    type_id: TypeId,
    instances: u64,
    next: &'a Ontology,
    state: &'a dyn InstanceState,
}

fn compare_properties(
    owner: &Owner<'_>,
    before: &BTreeMap<PropertyId, &PropertyDefinition>,
    after: &BTreeMap<PropertyId, &PropertyDefinition>,
    found: &mut Vec<Incompatibility>,
) {
    for property in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
        let property = *property;
        let (was, is) = (before.get(&property), after.get(&property));
        if was == is {
            continue;
        }
        let max_values = owner.state.max_values(owner.type_id, property);
        let Some(is) = is else {
            if max_values > 0 {
                found.push(Incompatibility::PropertyRemoved {
                    owner: owner.type_id,
                    property,
                    max_values,
                });
            }
            continue;
        };

        // The kernel refuses every write to a type whose properties carry a constraint it cannot
        // evaluate, and it can evaluate none, so a constraint added, tightened, relaxed or dropped
        // under instances is one whose effect on them cannot be shown.
        let was_constrained = was.map_or(&[][..], |was| was.constraints.as_slice());
        if owner.instances > 0 && was_constrained != is.constraints.as_slice() {
            found.push(Incompatibility::ConstraintChanged {
                owner: owner.type_id,
                property,
                instances: owner.instances,
            });
        }

        let newly_required = is.required && !was.is_some_and(|was| was.required);
        if newly_required
            && owner.instances > 0
            && owner.state.min_values(owner.type_id, property) == 0
        {
            found.push(Incompatibility::RequiredWithoutValues {
                owner: owner.type_id,
                property,
                instances: owner.instances,
            });
        }

        if !permits(is.cardinality, max_values) {
            found.push(Incompatibility::CardinalityNarrowed {
                owner: owner.type_id,
                property,
                cardinality: is.cardinality,
                max_values,
            });
        }

        let declared = is.value_type.kind();
        let held = owner.state.value_kinds(owner.type_id, property);
        let mut admitted = true;
        for kind in &held {
            if *kind != declared {
                admitted = false;
                found.push(Incompatibility::ValueKindNotAdmitted {
                    owner: owner.type_id,
                    property,
                    held: *kind,
                    declared,
                });
            }
        }
        // Same kind, different parameters: the kinds held say nothing about which variant or
        // which referenced type a value carries, so only a declaration that admits everything the
        // old one did is shown compatible.
        if admitted && held.contains(&declared) {
            if let Some(was) = was {
                if !admits_all_of(owner.next, &was.value_type, &is.value_type) {
                    found.push(Incompatibility::ValueTypeNarrowed {
                        owner: owner.type_id,
                        property,
                    });
                }
            }
        }
    }
}

const fn permits(cardinality: Cardinality, count: u64) -> bool {
    match cardinality {
        Cardinality::One => count <= 1,
        Cardinality::Many => true,
    }
}

/// Whether every value `old` admits is admitted by `new`, read in `next`'s hierarchy.
fn admits_all_of(next: &Ontology, old: &ValueType, new: &ValueType) -> bool {
    match (old, new) {
        (ValueType::Enum { variants: old }, ValueType::Enum { variants: new }) => {
            old.is_subset(new)
        }
        (ValueType::NodeRef { allowed_types: old }, ValueType::NodeRef { allowed_types: new }) => {
            old.iter()
                .all(|was| new.iter().any(|is| next.conforms_to(*was, *is)))
        }
        (ValueType::List(old), ValueType::List(new)) => admits_all_of(next, old, new),
        (ValueType::Record(old), ValueType::Record(new)) => {
            old.len() == new.len()
                && old.iter().all(|(field, was)| {
                    new.get(field)
                        .is_some_and(|is| admits_all_of(next, was, is))
                })
        }
        (old, new) => old.kind() == new.kind() && is_scalar(old),
    }
}

const fn is_scalar(value_type: &ValueType) -> bool {
    !matches!(
        value_type,
        ValueType::NodeRef { .. }
            | ValueType::Enum { .. }
            | ValueType::List(_)
            | ValueType::Record(_)
    )
}

/// Why a next version cannot replace the prior one over the canonical state there is.
///
/// `Display` leads with the stable kebab-case [`code`](Incompatibility::code), which
/// `systems/ekr/domains/ontology.yaml` declares as `ekr.ontology.IncompatibilityCode`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Incompatibility {
    /// `next` is not `prior`'s successor: its parent is not `prior`, or its number is not
    /// `prior`'s plus one, or it is `prior` itself.
    #[error("not-a-successor: {next} is not the next version of {prior}")]
    NotASuccessor {
        /// The version being replaced.
        prior: SchemaVersionId,
        /// The version offered to replace it.
        next: SchemaVersionId,
    },
    /// A property's constraints changed on a type with instances. No constraint can be evaluated
    /// yet, so whether the instances satisfy the new set cannot be shown.
    #[error(
        "constraint-changed: the constraints of property {property} on {owner} changed, and it \
         has {instances} instances"
    )]
    ConstraintChanged {
        /// The type whose instances resolve the property.
        owner: TypeId,
        /// The property.
        property: PropertyId,
        /// How many instances there are.
        instances: u64,
    },
    /// A property made required on a type at least one of whose instances carries no value of it.
    #[error(
        "required-property-missing: property {property} is required on {owner}, and at least \
         one of its {instances} instances carries no value of it"
    )]
    RequiredWithoutValues {
        /// The type whose instances lack it.
        owner: TypeId,
        /// The property.
        property: PropertyId,
        /// How many instances there are.
        instances: u64,
    },
    /// A cardinality that no longer permits what some instance holds.
    #[error(
        "cardinality-narrowed: property {property} of {owner} is {cardinality}, and an instance \
         holds {max_values} values"
    )]
    CardinalityNarrowed {
        /// The type whose instance holds too many.
        owner: TypeId,
        /// The property.
        property: PropertyId,
        /// The new cardinality.
        cardinality: Cardinality,
        /// The most values one instance holds.
        max_values: u64,
    },
    /// A value type that does not admit a kind instances hold.
    #[error(
        "value-kind-not-admitted: property {property} of {owner} is declared {declared}, and \
         instances hold a {held}"
    )]
    ValueKindNotAdmitted {
        /// The type whose instances hold it.
        owner: TypeId,
        /// The property.
        property: PropertyId,
        /// The kind held.
        held: ValueKind,
        /// The kind now declared.
        declared: ValueKind,
    },
    /// A value type of the same kind that no longer admits everything the old one did — a variant
    /// or a referenced type dropped, at any depth — while instances hold values of it.
    #[error(
        "value-type-narrowed: property {property} of {owner} admits less than it did, and \
         instances hold values of it"
    )]
    ValueTypeNarrowed {
        /// The type whose instances hold values.
        owner: TypeId,
        /// The property.
        property: PropertyId,
    },
    /// A type that has instances is not declared by the next version.
    #[error(
        "type-removed: {type_id} is not declared by the next version and has {instances} instances"
    )]
    TypeRemoved {
        /// The type.
        type_id: TypeId,
        /// How many instances it has.
        instances: u64,
    },
    /// A type that has instances changed in something other than its properties — parents,
    /// abstractness, lifecycle, operations, endpoints — which instance state cannot check.
    #[error(
        "type-declaration-changed: {type_id} changed in more than its properties and has \
         {instances} instances"
    )]
    DeclarationChanged {
        /// The type.
        type_id: TypeId,
        /// How many instances it has.
        instances: u64,
    },
    /// A property instances hold values of is not declared for their type by the next version.
    #[error(
        "property-removed: property {property} is no longer declared for {owner}, and an instance \
         holds {max_values} values of it"
    )]
    PropertyRemoved {
        /// The type whose instances hold it.
        owner: TypeId,
        /// The property.
        property: PropertyId,
        /// The most values one instance holds.
        max_values: u64,
    },
}

impl Incompatibility {
    /// Every code [`Incompatibility::code`] can return.
    pub const CODES: [&'static str; 9] = [
        "required-property-missing",
        "cardinality-narrowed",
        "value-kind-not-admitted",
        "value-type-narrowed",
        "type-removed",
        "type-declaration-changed",
        "property-removed",
        "not-a-successor",
        "constraint-changed",
    ];

    /// The stable kebab-case code of this incompatibility.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RequiredWithoutValues { .. } => Self::CODES[0],
            Self::CardinalityNarrowed { .. } => Self::CODES[1],
            Self::ValueKindNotAdmitted { .. } => Self::CODES[2],
            Self::ValueTypeNarrowed { .. } => Self::CODES[3],
            Self::TypeRemoved { .. } => Self::CODES[4],
            Self::DeclarationChanged { .. } => Self::CODES[5],
            Self::PropertyRemoved { .. } => Self::CODES[6],
            Self::NotASuccessor { .. } => Self::CODES[7],
            Self::ConstraintChanged { .. } => Self::CODES[8],
        }
    }
}
