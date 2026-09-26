//! Schema evolution: `Ontology::evolve` and `incompatibilities` — part A of
//! `story:schema-evolution-transactions`.
//!
//! Two questions, answered by two functions, because they need different inputs:
//!
//! * **Does the next version cohere?** [`Ontology::evolve`] answers from the ontology alone: the
//!   next version is the prior one with `number + 1`, `parent = prior`, and the changes applied in
//!   order, and it is refused exactly where `Ontology::load` would refuse it — plus the refusals
//!   that only make sense for a change (redefining a type, modifying a property of nothing, an
//!   empty change list, a version that names itself as its parent).
//! * **May it replace the prior one while canonical state is what it is?** [`incompatibilities`]
//!   answers with the kernel's help, through [`InstanceState`], without the ontology naming a
//!   graph type (`AGENTS.md` invariant 8).
//!
//! Every refusal carries a stable kebab-case code, and the set of codes is held equal to the
//! enumerations `systems/ekr/domains/ontology.yaml` declares for them.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{PropertyId, SchemaVersionId, Timestamp, TypeId};
use ekr_ontology::{
    incompatibilities, Cardinality, EdgeType, EvolveError, Incompatibility, InstanceState,
    NodeType, Ontology, OntologyDocument, OntologyError, PropertyDefinition, SchemaChange,
    SchemaVersion, ValueKind, ValueType,
};

/// A seed with a `Note` node type carrying an optional `title`, a `Topic` node type, and a
/// `cites` edge type from `Note` to `Note`.
struct Seed {
    ontology: Ontology,
    note: TypeId,
    topic: TypeId,
    cites: TypeId,
    title: PropertyId,
}

fn seed() -> Seed {
    let (note, topic, cites, title) = (
        TypeId::mint(),
        TypeId::mint(),
        TypeId::mint(),
        PropertyId::mint(),
    );
    let mut note_type = NodeType::new(note, "Note");
    note_type.properties.insert(
        title,
        PropertyDefinition::new(title, "title", ValueType::String),
    );
    let mut cites_type = EdgeType::new(cites, "cites");
    cites_type.source_types.insert(note);
    cites_type.target_types.insert(note);
    let ontology = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: vec![note_type, NodeType::new(topic, "Topic")],
        edge_types: vec![cites_type],
    })
    .expect("the seed coheres");
    Seed {
        ontology,
        note,
        topic,
        cites,
        title,
    }
}

const LATER: Timestamp = Timestamp::from_millis(1_000);

fn evolve(prior: &Ontology, changes: &[SchemaChange]) -> Result<Ontology, EvolveError> {
    prior.evolve(SchemaVersionId::mint(), LATER, changes)
}

/// `prior`'s declarations under a version that is its successor, for a `next` that `evolve`
/// cannot build (a removal, a type-level change).
fn successor_document(prior: &Ontology) -> OntologyDocument {
    let mut document = prior.to_document();
    document.version = SchemaVersion {
        id: SchemaVersionId::mint(),
        number: prior.version().number + 1,
        parent: Some(prior.version().id),
        created_at: LATER,
    };
    document
}

// ---------------------------------------------------------------------------------------------
// evolve: the lineage

#[test]
fn evolve_derives_the_next_version_from_the_prior_one() {
    let seed = seed();
    let next_id = SchemaVersionId::mint();
    let section = TypeId::mint();
    let next = seed
        .ontology
        .evolve(
            next_id,
            LATER,
            &[SchemaChange::DefineNodeType(NodeType::new(
                section, "Section",
            ))],
        )
        .expect("a new type coheres");

    let version = next.version();
    assert_eq!(version.id, next_id);
    assert_eq!(version.number, seed.ontology.version().number + 1);
    assert_eq!(version.parent, Some(seed.ontology.version().id));
    assert_eq!(version.created_at, LATER);
    assert!(next.node_type(section).is_some());
    assert!(
        seed.ontology.node_type(section).is_none(),
        "the prior is untouched"
    );
    // Everything the prior declared is still declared.
    assert_eq!(
        next.node_type(seed.note),
        seed.ontology.node_type(seed.note)
    );
    assert_eq!(
        next.edge_type(seed.cites),
        seed.ontology.edge_type(seed.cites)
    );
}

#[test]
fn an_evolved_version_is_exactly_what_load_would_hold() {
    let seed = seed();
    let section = TypeId::mint();
    let mut section_type = NodeType::new(section, "Section");
    section_type.parents.insert(seed.note);
    let next = evolve(
        &seed.ontology,
        &[SchemaChange::DefineNodeType(section_type)],
    )
    .expect("a specialisation coheres");
    let reloaded = Ontology::load(next.to_document()).expect("an evolved version loads");
    assert_eq!(reloaded, next);
    // Inherited through the new parent link, as load resolves it.
    assert!(next.properties_of(section).contains_key(&seed.title));
}

#[test]
fn evolution_chains_and_each_version_names_the_one_before() {
    let seed = seed();
    let first = evolve(
        &seed.ontology,
        &[SchemaChange::DefineNodeType(NodeType::new(
            TypeId::mint(),
            "A",
        ))],
    )
    .expect("first");
    let second = evolve(
        &first,
        &[SchemaChange::DefineNodeType(NodeType::new(
            TypeId::mint(),
            "B",
        ))],
    )
    .expect("second");
    assert_eq!(second.version().number, 2);
    assert_eq!(second.version().parent, Some(first.version().id));
}

#[test]
fn modify_property_adds_a_property_and_redeclares_one() {
    let seed = seed();
    let summary = PropertyId::mint();
    let mut title = PropertyDefinition::new(seed.title, "title", ValueType::String);
    title.cardinality = Cardinality::Many;
    let next = evolve(
        &seed.ontology,
        &[
            SchemaChange::ModifyProperty {
                owner: seed.note,
                property: PropertyDefinition::new(summary, "summary", ValueType::String),
            },
            SchemaChange::ModifyProperty {
                owner: seed.note,
                property: title.clone(),
            },
        ],
    )
    .expect("adding and redeclaring cohere");
    let properties = next.properties_of(seed.note);
    assert_eq!(properties.len(), 2);
    assert_eq!(properties[&seed.title], &title);
    assert!(properties.contains_key(&summary));
}

#[test]
fn modify_property_reaches_an_edge_type() {
    let seed = seed();
    let weight = PropertyId::mint();
    let next = evolve(
        &seed.ontology,
        &[SchemaChange::ModifyProperty {
            owner: seed.cites,
            property: PropertyDefinition::new(weight, "weight", ValueType::Integer),
        }],
    )
    .expect("an edge type owns properties too");
    assert!(next
        .edge_type(seed.cites)
        .expect("still declared")
        .properties
        .contains_key(&weight));
}

#[test]
fn changes_apply_in_order_so_a_type_defined_earlier_owns_a_later_property() {
    let seed = seed();
    let section = TypeId::mint();
    let heading = PropertyId::mint();
    let next = evolve(
        &seed.ontology,
        &[
            SchemaChange::DefineNodeType(NodeType::new(section, "Section")),
            SchemaChange::ModifyProperty {
                owner: section,
                property: PropertyDefinition::new(heading, "heading", ValueType::String),
            },
        ],
    )
    .expect("the owner exists by the time the property is modified");
    assert!(next.properties_of(section).contains_key(&heading));

    // The other order is a property of nothing.
    let refused = evolve(
        &seed.ontology,
        &[
            SchemaChange::ModifyProperty {
                owner: section,
                property: PropertyDefinition::new(heading, "heading", ValueType::String),
            },
            SchemaChange::DefineNodeType(NodeType::new(section, "Section")),
        ],
    )
    .expect_err("order matters");
    assert_eq!(refused, EvolveError::UnknownOwner { owner: section });
}

#[test]
fn define_edge_type_adds_an_edge_type_between_existing_node_types() {
    let seed = seed();
    let tagged = TypeId::mint();
    let mut tagged_type = EdgeType::new(tagged, "tagged");
    tagged_type.source_types.insert(seed.note);
    tagged_type.target_types.insert(seed.topic);
    let next = evolve(&seed.ontology, &[SchemaChange::DefineEdgeType(tagged_type)])
        .expect("an edge between declared types coheres");
    assert!(next.edge_type(tagged).is_some());
}

// ---------------------------------------------------------------------------------------------
// evolve: the refusals

#[test]
fn an_empty_change_list_is_refused() {
    let seed = seed();
    let refused = evolve(&seed.ontology, &[]).expect_err("a version that changes nothing");
    assert_eq!(refused, EvolveError::EmptyChanges);
    assert_eq!(refused.code(), "empty-schema-change");
}

#[test]
fn redefining_a_type_the_prior_version_declares_is_refused_for_every_kind_of_declaration() {
    let seed = seed();
    // A node type over a node type, a node type over an edge type, an edge type over a node type
    // and an edge type over an edge type: one id space, four collisions.
    let mut edge_over_note = EdgeType::new(seed.note, "e");
    edge_over_note.source_types.insert(seed.topic);
    edge_over_note.target_types.insert(seed.topic);
    let mut edge_over_cites = EdgeType::new(seed.cites, "e");
    edge_over_cites.source_types.insert(seed.topic);
    edge_over_cites.target_types.insert(seed.topic);
    for (change, type_id) in [
        (
            SchemaChange::DefineNodeType(NodeType::new(seed.note, "Note again")),
            seed.note,
        ),
        (
            SchemaChange::DefineNodeType(NodeType::new(seed.cites, "cites as a node")),
            seed.cites,
        ),
        (SchemaChange::DefineEdgeType(edge_over_note), seed.note),
        (SchemaChange::DefineEdgeType(edge_over_cites), seed.cites),
    ] {
        let refused = evolve(&seed.ontology, &[change]).expect_err("already declared");
        assert_eq!(refused, EvolveError::TypeAlreadyDeclared { type_id });
        assert_eq!(refused.code(), "type-already-declared");
    }
}

#[test]
fn a_type_defined_twice_in_one_change_list_is_refused() {
    let seed = seed();
    let section = TypeId::mint();
    let refused = evolve(
        &seed.ontology,
        &[
            SchemaChange::DefineNodeType(NodeType::new(section, "Section")),
            SchemaChange::DefineNodeType(NodeType::new(section, "Section")),
        ],
    )
    .expect_err("one id, two declarations");
    assert_eq!(
        refused,
        EvolveError::TypeAlreadyDeclared { type_id: section }
    );
}

#[test]
fn modifying_a_property_of_an_undeclared_owner_is_refused() {
    let seed = seed();
    let nowhere = TypeId::mint();
    let property = PropertyId::mint();
    let refused = evolve(
        &seed.ontology,
        &[SchemaChange::ModifyProperty {
            owner: nowhere,
            property: PropertyDefinition::new(property, "p", ValueType::String),
        }],
    )
    .expect_err("a property of nothing");
    assert_eq!(refused, EvolveError::UnknownOwner { owner: nowhere });
    assert_eq!(refused.code(), "unknown-property-owner");
}

#[test]
fn a_result_that_fails_a_load_refusal_is_refused_with_that_refusal() {
    let seed = seed();
    let nowhere = TypeId::mint();
    let property = PropertyId::mint();

    // A reference to a type no version declares.
    let refused = evolve(
        &seed.ontology,
        &[SchemaChange::ModifyProperty {
            owner: seed.note,
            property: PropertyDefinition::new(
                property,
                "about",
                ValueType::NodeRef {
                    allowed_types: [nowhere].into_iter().collect(),
                },
            ),
        }],
    )
    .expect_err("a reference to an undeclared type");
    assert_eq!(
        refused,
        EvolveError::Incoherent(OntologyError::UnknownType { type_id: nowhere })
    );
    assert_eq!(refused.code(), "incoherent-schema");

    // An edge from nothing.
    let hollow = TypeId::mint();
    let refused = evolve(
        &seed.ontology,
        &[SchemaChange::DefineEdgeType(EdgeType::new(
            hollow, "hollow",
        ))],
    )
    .expect_err("an edge with no endpoints");
    assert!(matches!(
        refused,
        EvolveError::Incoherent(OntologyError::EmptyEdgeEndpoint { type_id, .. }) if type_id == hollow
    ));

    // A new type whose parent is not declared.
    let mut orphan = NodeType::new(TypeId::mint(), "Orphan");
    orphan.parents.insert(nowhere);
    let refused = evolve(&seed.ontology, &[SchemaChange::DefineNodeType(orphan)])
        .expect_err("a parent that is not there");
    assert_eq!(
        refused,
        EvolveError::Incoherent(OntologyError::UnknownType { type_id: nowhere })
    );
}

#[test]
fn a_redeclaration_that_makes_an_inherited_property_ambiguous_is_refused() {
    // `Section` specialises both `Note` and `Topic`. Giving `Topic` a different `title` from the
    // one `Note` declares leaves `Section` with two unrelated declarations of one property.
    let seed = seed();
    let section = TypeId::mint();
    let mut section_type = NodeType::new(section, "Section");
    section_type.parents.extend([seed.note, seed.topic]);
    let refused = evolve(
        &seed.ontology,
        &[
            SchemaChange::DefineNodeType(section_type),
            SchemaChange::ModifyProperty {
                owner: seed.topic,
                property: PropertyDefinition::new(seed.title, "title", ValueType::Integer),
            },
        ],
    )
    .expect_err("ambiguous");
    assert_eq!(
        refused,
        EvolveError::Incoherent(OntologyError::AmbiguousProperty {
            type_id: section,
            property: seed.title,
        })
    );
}

#[test]
fn a_version_that_names_itself_as_its_parent_is_refused() {
    let seed = seed();
    let same = seed.ontology.version().id;
    let refused = seed
        .ontology
        .evolve(
            same,
            LATER,
            &[SchemaChange::DefineNodeType(NodeType::new(
                TypeId::mint(),
                "A",
            ))],
        )
        .expect_err("a lineage of one id is a cycle");
    assert_eq!(refused, EvolveError::VersionIdReused { id: same });
    assert_eq!(refused.code(), "schema-version-reused");
}

#[test]
fn a_version_that_reuses_its_grandparent_id_is_refused() {
    let seed = seed();
    let first = evolve(
        &seed.ontology,
        &[SchemaChange::DefineNodeType(NodeType::new(
            TypeId::mint(),
            "A",
        ))],
    )
    .expect("first");
    let grandparent = seed.ontology.version().id;
    assert_eq!(first.version().parent, Some(grandparent), "precondition");
    let refused = first
        .evolve(
            grandparent,
            LATER,
            &[SchemaChange::DefineNodeType(NodeType::new(
                TypeId::mint(),
                "B",
            ))],
        )
        .expect_err("the parent's id is already on the lineage");
    assert_eq!(refused, EvolveError::VersionIdReused { id: grandparent });
}

#[test]
fn a_change_list_whose_result_declares_what_the_prior_did_is_refused() {
    let seed = seed();
    let same = PropertyDefinition::new(seed.title, "title", ValueType::String);
    for changes in [
        vec![SchemaChange::ModifyProperty {
            owner: seed.note,
            property: same.clone(),
        }],
        // Two changes that cancel: the result still declares exactly the prior.
        vec![
            SchemaChange::ModifyProperty {
                owner: seed.note,
                property: PropertyDefinition::new(seed.title, "title", ValueType::Integer),
            },
            SchemaChange::ModifyProperty {
                owner: seed.note,
                property: same.clone(),
            },
        ],
    ] {
        let refused = evolve(&seed.ontology, &changes).expect_err("no effect");
        assert_eq!(refused, EvolveError::WithoutEffect);
        assert_eq!(refused.code(), "schema-change-without-effect");
    }
    // A rename is an effect.
    assert!(evolve(
        &seed.ontology,
        &[SchemaChange::ModifyProperty {
            owner: seed.note,
            property: PropertyDefinition::new(seed.title, "heading", ValueType::String),
        }],
    )
    .is_ok());
}

// ---------------------------------------------------------------------------------------------
// incompatibilities

/// Canonical state as a case states it.
#[derive(Default)]
struct State {
    nodes: BTreeMap<TypeId, u64>,
    edges: BTreeMap<TypeId, u64>,
    max: BTreeMap<(TypeId, PropertyId), u64>,
    /// Absent means "the same as `max`": every instance holds as many as the fullest one.
    min: BTreeMap<(TypeId, PropertyId), u64>,
    kinds: BTreeMap<(TypeId, PropertyId), BTreeSet<ValueKind>>,
}

impl State {
    fn nodes(mut self, of: TypeId, count: u64) -> Self {
        self.nodes.insert(of, count);
        self
    }
    fn edges(mut self, of: TypeId, count: u64) -> Self {
        self.edges.insert(of, count);
        self
    }
    fn values(mut self, owner: TypeId, property: PropertyId, max: u64, kind: ValueKind) -> Self {
        self.max.insert((owner, property), max);
        self.kinds
            .entry((owner, property))
            .or_default()
            .insert(kind);
        self
    }
    /// Some instance of `owner` holds no value of `property`, whatever the fullest one holds.
    fn lacking(mut self, owner: TypeId, property: PropertyId) -> Self {
        self.min.insert((owner, property), 0);
        self
    }
}

impl InstanceState for State {
    fn node_count(&self, node_type: TypeId) -> u64 {
        self.nodes.get(&node_type).copied().unwrap_or(0)
    }
    fn edge_count(&self, edge_type: TypeId) -> u64 {
        self.edges.get(&edge_type).copied().unwrap_or(0)
    }
    fn max_values(&self, owner: TypeId, property: PropertyId) -> u64 {
        self.max.get(&(owner, property)).copied().unwrap_or(0)
    }
    fn min_values(&self, owner: TypeId, property: PropertyId) -> u64 {
        self.min
            .get(&(owner, property))
            .copied()
            .unwrap_or_else(|| self.max_values(owner, property))
    }
    fn value_kinds(&self, owner: TypeId, property: PropertyId) -> BTreeSet<ValueKind> {
        self.kinds
            .get(&(owner, property))
            .cloned()
            .unwrap_or_default()
    }
}

fn redeclare(seed: &Seed, owner: TypeId, property: PropertyDefinition) -> Ontology {
    evolve(
        &seed.ontology,
        &[SchemaChange::ModifyProperty { owner, property }],
    )
    .expect("the redeclaration coheres")
}

#[test]
fn a_property_made_required_on_a_type_whose_instances_lack_it_is_incompatible() {
    let seed = seed();
    let mut title = PropertyDefinition::new(seed.title, "title", ValueType::String);
    title.required = true;
    let next = redeclare(&seed, seed.note, title);

    let found = incompatibilities(&seed.ontology, &next, &State::default().nodes(seed.note, 3));
    assert_eq!(
        found,
        vec![Incompatibility::RequiredWithoutValues {
            owner: seed.note,
            property: seed.title,
            instances: 3,
        }]
    );
    assert_eq!(found[0].code(), "required-property-missing");

    // No instances, or instances that carry it: nothing to refuse.
    assert!(incompatibilities(&seed.ontology, &next, &State::default()).is_empty());
    let carried =
        State::default()
            .nodes(seed.note, 3)
            .values(seed.note, seed.title, 1, ValueKind::String);
    assert!(incompatibilities(&seed.ontology, &next, &carried).is_empty());
}

#[test]
fn a_new_required_property_on_a_type_with_instances_is_incompatible() {
    let seed = seed();
    let summary = PropertyId::mint();
    let mut declared = PropertyDefinition::new(summary, "summary", ValueType::String);
    declared.required = true;
    let next = redeclare(&seed, seed.note, declared);
    assert_eq!(
        incompatibilities(&seed.ontology, &next, &State::default().nodes(seed.note, 1)),
        vec![Incompatibility::RequiredWithoutValues {
            owner: seed.note,
            property: summary,
            instances: 1,
        }]
    );
}

#[test]
fn a_property_made_required_on_a_parent_reaches_the_instances_of_its_children() {
    let seed = seed();
    let section = TypeId::mint();
    let mut section_type = NodeType::new(section, "Section");
    section_type.parents.insert(seed.note);
    let prior = evolve(
        &seed.ontology,
        &[SchemaChange::DefineNodeType(section_type)],
    )
    .expect("specialise");
    let mut title = PropertyDefinition::new(seed.title, "title", ValueType::String);
    title.required = true;
    let next = evolve(
        &prior,
        &[SchemaChange::ModifyProperty {
            owner: seed.note,
            property: title,
        }],
    )
    .expect("coheres");

    // Only `Section` has instances; the property is declared on `Note`.
    assert_eq!(
        incompatibilities(&prior, &next, &State::default().nodes(section, 2)),
        vec![Incompatibility::RequiredWithoutValues {
            owner: section,
            property: seed.title,
            instances: 2,
        }]
    );
}

#[test]
fn a_property_made_required_while_one_instance_of_several_lacks_it_is_incompatible() {
    // One of three notes carries a title and two do not: the fullest instance holds one value,
    // and the emptiest holds none. `max_values` alone would call this compatible.
    let seed = seed();
    let mut title = PropertyDefinition::new(seed.title, "title", ValueType::String);
    title.required = true;
    let next = redeclare(&seed, seed.note, title);
    let partly = State::default()
        .nodes(seed.note, 3)
        .values(seed.note, seed.title, 1, ValueKind::String)
        .lacking(seed.note, seed.title);
    assert_eq!(
        incompatibilities(&seed.ontology, &next, &partly),
        vec![Incompatibility::RequiredWithoutValues {
            owner: seed.note,
            property: seed.title,
            instances: 3,
        }]
    );
}

#[test]
fn a_property_made_required_on_an_edge_type_some_edges_lack_is_incompatible() {
    let seed = seed();
    let weight = PropertyId::mint();
    let mut optional = PropertyDefinition::new(weight, "weight", ValueType::Integer);
    let prior = redeclare(&seed, seed.cites, optional.clone());
    optional.required = true;
    let next = evolve(
        &prior,
        &[SchemaChange::ModifyProperty {
            owner: seed.cites,
            property: optional,
        }],
    )
    .expect("coheres");
    let partly = State::default()
        .edges(seed.cites, 2)
        .values(seed.cites, weight, 1, ValueKind::Integer)
        .lacking(seed.cites, weight);
    assert_eq!(
        incompatibilities(&prior, &next, &partly),
        vec![Incompatibility::RequiredWithoutValues {
            owner: seed.cites,
            property: weight,
            instances: 2,
        }]
    );
    // Every edge carries one: compatible.
    let all =
        State::default()
            .edges(seed.cites, 2)
            .values(seed.cites, weight, 1, ValueKind::Integer);
    assert!(incompatibilities(&prior, &next, &all).is_empty());
}

#[test]
fn a_property_made_required_on_an_edge_type_counts_edges() {
    let seed = seed();
    let weight = PropertyId::mint();
    let mut declared = PropertyDefinition::new(weight, "weight", ValueType::Integer);
    declared.required = true;
    let next = redeclare(&seed, seed.cites, declared);
    assert_eq!(
        incompatibilities(
            &seed.ontology,
            &next,
            &State::default().edges(seed.cites, 4)
        ),
        vec![Incompatibility::RequiredWithoutValues {
            owner: seed.cites,
            property: weight,
            instances: 4,
        }]
    );
    // Nodes of an unrelated count do not stand in for edges.
    assert!(incompatibilities(
        &seed.ontology,
        &next,
        &State::default().nodes(seed.cites, 4)
    )
    .is_empty());
}

#[test]
fn a_cardinality_narrowed_below_what_instances_hold_is_incompatible() {
    let seed = seed();
    let mut many = PropertyDefinition::new(seed.title, "title", ValueType::String);
    many.cardinality = Cardinality::Many;
    let prior = redeclare(&seed, seed.note, many);
    let next = evolve(
        &prior,
        &[SchemaChange::ModifyProperty {
            owner: seed.note,
            property: PropertyDefinition::new(seed.title, "title", ValueType::String),
        }],
    )
    .expect("back to One");

    let held =
        State::default()
            .nodes(seed.note, 2)
            .values(seed.note, seed.title, 3, ValueKind::String);
    let found = incompatibilities(&prior, &next, &held);
    assert_eq!(
        found,
        vec![Incompatibility::CardinalityNarrowed {
            owner: seed.note,
            property: seed.title,
            cardinality: Cardinality::One,
            max_values: 3,
        }]
    );
    assert_eq!(found[0].code(), "cardinality-narrowed");

    let one_each =
        State::default()
            .nodes(seed.note, 2)
            .values(seed.note, seed.title, 1, ValueKind::String);
    assert!(incompatibilities(&prior, &next, &one_each).is_empty());
}

#[test]
fn a_value_type_changed_under_values_of_the_old_kind_is_incompatible() {
    let seed = seed();
    let next = redeclare(
        &seed,
        seed.note,
        PropertyDefinition::new(seed.title, "title", ValueType::Integer),
    );
    let held =
        State::default()
            .nodes(seed.note, 1)
            .values(seed.note, seed.title, 1, ValueKind::String);
    let found = incompatibilities(&seed.ontology, &next, &held);
    assert_eq!(
        found,
        vec![Incompatibility::ValueKindNotAdmitted {
            owner: seed.note,
            property: seed.title,
            held: ValueKind::String,
            declared: ValueKind::Integer,
        }]
    );
    assert_eq!(found[0].code(), "value-kind-not-admitted");

    // No values held: the kind may change freely.
    assert!(
        incompatibilities(&seed.ontology, &next, &State::default().nodes(seed.note, 1)).is_empty()
    );
}

#[test]
fn a_same_kind_value_type_narrowed_under_held_values_is_incompatible_and_a_widening_is_not() {
    let seed = seed();
    let status = PropertyId::mint();
    let variants = |names: &[&str]| ValueType::Enum {
        variants: names.iter().map(|name| (*name).to_owned()).collect(),
    };
    let prior = redeclare(
        &seed,
        seed.note,
        PropertyDefinition::new(status, "status", variants(&["draft", "final"])),
    );
    let held = State::default()
        .nodes(seed.note, 1)
        .values(seed.note, status, 1, ValueKind::Enum);

    let narrowed = evolve(
        &prior,
        &[SchemaChange::ModifyProperty {
            owner: seed.note,
            property: PropertyDefinition::new(status, "status", variants(&["draft"])),
        }],
    )
    .expect("coheres");
    let found = incompatibilities(&prior, &narrowed, &held);
    assert_eq!(
        found,
        vec![Incompatibility::ValueTypeNarrowed {
            owner: seed.note,
            property: status,
        }]
    );
    assert_eq!(found[0].code(), "value-type-narrowed");

    let widened = evolve(
        &prior,
        &[SchemaChange::ModifyProperty {
            owner: seed.note,
            property: PropertyDefinition::new(
                status,
                "status",
                variants(&["draft", "final", "archived"]),
            ),
        }],
    )
    .expect("coheres");
    assert!(incompatibilities(&prior, &widened, &held).is_empty());

    // A NodeRef that stops admitting a type it used to admit is the same narrowing.
    let about = PropertyId::mint();
    let refs = |types: &[TypeId]| ValueType::NodeRef {
        allowed_types: types.iter().copied().collect(),
    };
    let prior = redeclare(
        &seed,
        seed.note,
        PropertyDefinition::new(about, "about", refs(&[seed.note, seed.topic])),
    );
    let narrowed = evolve(
        &prior,
        &[SchemaChange::ModifyProperty {
            owner: seed.note,
            property: PropertyDefinition::new(about, "about", refs(&[seed.topic])),
        }],
    )
    .expect("coheres");
    let held = State::default()
        .nodes(seed.note, 1)
        .values(seed.note, about, 1, ValueKind::NodeRef);
    assert_eq!(
        incompatibilities(&prior, &narrowed, &held),
        vec![Incompatibility::ValueTypeNarrowed {
            owner: seed.note,
            property: about,
        }]
    );
}

#[test]
fn a_version_that_changes_nothing_anyone_holds_is_compatible() {
    let seed = seed();
    let next = evolve(
        &seed.ontology,
        &[
            SchemaChange::DefineNodeType(NodeType::new(TypeId::mint(), "Section")),
            SchemaChange::ModifyProperty {
                owner: seed.note,
                property: PropertyDefinition::new(PropertyId::mint(), "summary", ValueType::String),
            },
        ],
    )
    .expect("coheres");
    let busy = State::default()
        .nodes(seed.note, 10)
        .nodes(seed.topic, 10)
        .edges(seed.cites, 10)
        .values(seed.note, seed.title, 1, ValueKind::String);
    assert!(incompatibilities(&seed.ontology, &next, &busy).is_empty());
}

#[test]
fn removing_a_type_or_property_that_instances_hold_is_incompatible() {
    // `evolve` cannot remove anything (wave decision 6), but `incompatibilities` compares any two
    // ontologies, and a second route to `next` must not pass a removal as compatible.
    let seed = seed();
    let mut document = successor_document(&seed.ontology);
    document
        .node_types
        .retain(|declared| declared.id != seed.topic);
    for declared in &mut document.node_types {
        declared.properties.clear();
    }
    let next = Ontology::load(document).expect("coheres");
    let held = State::default()
        .nodes(seed.topic, 1)
        .nodes(seed.note, 1)
        .values(seed.note, seed.title, 1, ValueKind::String);
    let found = incompatibilities(&seed.ontology, &next, &held);
    assert!(found.contains(&Incompatibility::TypeRemoved {
        type_id: seed.topic,
        instances: 1,
    }));
    assert!(found.contains(&Incompatibility::PropertyRemoved {
        owner: seed.note,
        property: seed.title,
        max_values: 1,
    }));
}

#[test]
fn a_type_changed_in_more_than_its_properties_is_incompatible_while_it_has_instances() {
    let seed = seed();
    let mut document = successor_document(&seed.ontology);
    for declared in &mut document.node_types {
        if declared.id == seed.topic {
            declared.abstract_type = true;
        }
    }
    for declared in &mut document.edge_types {
        declared.target_types.insert(seed.topic);
    }
    let next = Ontology::load(document).expect("coheres");
    let found = incompatibilities(
        &seed.ontology,
        &next,
        &State::default().nodes(seed.topic, 2).edges(seed.cites, 1),
    );
    assert_eq!(
        found,
        vec![
            Incompatibility::DeclarationChanged {
                type_id: seed.topic,
                instances: 2,
            },
            Incompatibility::DeclarationChanged {
                type_id: seed.cites,
                instances: 1,
            },
        ]
    );
    assert_eq!(found[0].code(), "type-declaration-changed");
    assert!(incompatibilities(&seed.ontology, &next, &State::default()).is_empty());
}

#[test]
fn a_prior_at_the_largest_version_number_is_refused_rather_than_wrapped() {
    let seed = seed();
    let mut document = seed.ontology.to_document();
    document.version.number = u64::MAX;
    let last = Ontology::load(document).expect("coheres");
    let refused = evolve(
        &last,
        &[SchemaChange::DefineNodeType(NodeType::new(
            TypeId::mint(),
            "A",
        ))],
    )
    .expect_err("no next number");
    assert_eq!(
        refused,
        EvolveError::VersionNumberExhausted {
            id: last.version().id
        }
    );
    assert_eq!(refused.code(), "schema-version-exhausted");
}

#[test]
fn a_constraint_changed_on_a_property_is_incompatible_wherever_instances_resolve_it() {
    // The constraint is declared on `Note`; only `Section`, which specialises it, has instances.
    let seed = seed();
    let section = TypeId::mint();
    let mut section_type = NodeType::new(section, "Section");
    section_type.parents.insert(seed.note);
    let prior = evolve(
        &seed.ontology,
        &[SchemaChange::DefineNodeType(section_type)],
    )
    .expect("specialise");
    let mut constrained = PropertyDefinition::new(seed.title, "title", ValueType::String);
    constrained.constraints.push("non_empty".to_owned());
    let next = evolve(
        &prior,
        &[SchemaChange::ModifyProperty {
            owner: seed.note,
            property: constrained.clone(),
        }],
    )
    .expect("coheres");

    let found = incompatibilities(&prior, &next, &State::default().nodes(section, 2));
    assert_eq!(
        found,
        vec![Incompatibility::ConstraintChanged {
            owner: section,
            property: seed.title,
            instances: 2,
        }]
    );
    assert_eq!(found[0].code(), "constraint-changed");
    assert!(incompatibilities(&prior, &next, &State::default()).is_empty());

    // Relaxing it back is a change too: the kernel cannot evaluate either side.
    let relaxed = evolve(
        &next,
        &[SchemaChange::ModifyProperty {
            owner: seed.note,
            property: PropertyDefinition::new(seed.title, "title", ValueType::String),
        }],
    )
    .expect("coheres");
    assert_eq!(
        incompatibilities(&next, &relaxed, &State::default().nodes(section, 2)),
        vec![Incompatibility::ConstraintChanged {
            owner: section,
            property: seed.title,
            instances: 2,
        }]
    );

    // A new constrained property on an edge type with edges.
    let weight = PropertyId::mint();
    let mut declared = PropertyDefinition::new(weight, "weight", ValueType::Integer);
    declared.constraints.push("positive".to_owned());
    let next = redeclare(&seed, seed.cites, declared);
    assert_eq!(
        incompatibilities(
            &seed.ontology,
            &next,
            &State::default().edges(seed.cites, 1)
        ),
        vec![Incompatibility::ConstraintChanged {
            owner: seed.cites,
            property: weight,
            instances: 1,
        }]
    );
}

#[test]
fn a_type_level_change_counts_the_instances_of_every_type_conforming_to_it() {
    // `Topic` becomes abstract; it has no instances, and `Subtopic`, which specialises it, has.
    let seed = seed();
    let subtopic = TypeId::mint();
    let mut subtopic_type = NodeType::new(subtopic, "Subtopic");
    subtopic_type.parents.insert(seed.topic);
    let prior = evolve(
        &seed.ontology,
        &[SchemaChange::DefineNodeType(subtopic_type)],
    )
    .expect("specialise");
    let mut document = successor_document(&prior);
    for declared in &mut document.node_types {
        if declared.id == seed.topic {
            declared.abstract_type = true;
        }
    }
    let next = Ontology::load(document).expect("coheres");
    assert_eq!(
        incompatibilities(
            &prior,
            &next,
            &State::default().nodes(subtopic, 3).nodes(seed.note, 5)
        ),
        vec![Incompatibility::DeclarationChanged {
            type_id: seed.topic,
            instances: 3,
        }]
    );
}

#[test]
fn a_next_that_is_not_the_priors_successor_is_incompatible() {
    let seed = seed();
    let successor = evolve(
        &seed.ontology,
        &[SchemaChange::DefineNodeType(NodeType::new(
            TypeId::mint(),
            "A",
        ))],
    )
    .expect("coheres");
    assert!(incompatibilities(&seed.ontology, &successor, &State::default()).is_empty());

    let mut orphan = successor.to_document();
    orphan.version.parent = Some(SchemaVersionId::mint());
    let mut skipped = successor.to_document();
    skipped.version.number += 1;
    let mut seedlike = successor.to_document();
    seedlike.version.parent = None;
    for document in [orphan, skipped, seedlike] {
        let next = Ontology::load(document).expect("coheres");
        let found = incompatibilities(&seed.ontology, &next, &State::default());
        assert_eq!(
            found,
            vec![Incompatibility::NotASuccessor {
                prior: seed.ontology.version().id,
                next: next.version().id,
            }]
        );
        assert_eq!(found[0].code(), "not-a-successor");
    }
}

#[test]
fn a_value_type_change_is_refused_over_the_kind_of_an_assertion_object_alone() {
    // No node holds a `title` value; one active property assertion on a note has a String object.
    // `value_kinds` reports that kind, `max_values` and `min_values` count no assertion.
    let seed = seed();
    let assertion_only = State {
        nodes: BTreeMap::from([(seed.note, 1)]),
        kinds: BTreeMap::from([((seed.note, seed.title), BTreeSet::from([ValueKind::String]))]),
        ..State::default()
    };
    let retyped = redeclare(
        &seed,
        seed.note,
        PropertyDefinition::new(seed.title, "title", ValueType::Integer),
    );
    assert_eq!(
        incompatibilities(&seed.ontology, &retyped, &assertion_only),
        vec![Incompatibility::ValueKindNotAdmitted {
            owner: seed.note,
            property: seed.title,
            held: ValueKind::String,
            declared: ValueKind::Integer,
        }]
    );

    // Same kind, narrower parameters, again with only an assertion's object held.
    let status = PropertyId::mint();
    let variants = |names: &[&str]| ValueType::Enum {
        variants: names.iter().map(|name| (*name).to_owned()).collect(),
    };
    let prior = redeclare(
        &seed,
        seed.note,
        PropertyDefinition::new(status, "status", variants(&["draft", "final"])),
    );
    let narrowed = evolve(
        &prior,
        &[SchemaChange::ModifyProperty {
            owner: seed.note,
            property: PropertyDefinition::new(status, "status", variants(&["draft"])),
        }],
    )
    .expect("coheres");
    let assertion_only = State {
        nodes: BTreeMap::from([(seed.note, 1)]),
        kinds: BTreeMap::from([((seed.note, status), BTreeSet::from([ValueKind::Enum]))]),
        ..State::default()
    };
    assert_eq!(
        incompatibilities(&prior, &narrowed, &assertion_only),
        vec![Incompatibility::ValueTypeNarrowed {
            owner: seed.note,
            property: status,
        }]
    );
}

#[test]
fn a_reference_narrowing_is_judged_by_the_concrete_types_it_admits() {
    // `Topic` becomes an abstract `Kind` with concrete subtypes; a note's `about` points at it.
    let seed = seed();
    let (kind, first, second, about) = (
        TypeId::mint(),
        TypeId::mint(),
        TypeId::mint(),
        PropertyId::mint(),
    );
    let mut kind_type = NodeType::new(kind, "Kind");
    kind_type.abstract_type = true;
    let mut first_type = NodeType::new(first, "First");
    first_type.parents.insert(kind);
    let mut second_type = NodeType::new(second, "Second");
    second_type.parents.insert(kind);
    let refs = |types: &[TypeId]| ValueType::NodeRef {
        allowed_types: types.iter().copied().collect(),
    };
    let narrow_to_first = |prior: &Ontology, wrap: fn(ValueType) -> ValueType| {
        evolve(
            prior,
            &[SchemaChange::ModifyProperty {
                owner: seed.note,
                property: PropertyDefinition::new(about, "about", wrap(refs(&[first]))),
            }],
        )
        .expect("coheres")
    };
    let top = |value_type: ValueType| value_type;
    let listed = |value_type: ValueType| ValueType::List(Box::new(value_type));

    for (wrap, held_kind) in [
        (top as fn(ValueType) -> ValueType, ValueKind::NodeRef),
        (listed, ValueKind::List),
    ] {
        let held = State::default()
            .nodes(seed.note, 1)
            .values(seed.note, about, 1, held_kind);
        // One concrete subtype: `Kind` and `First` admit the same references.
        let prior = evolve(
            &seed.ontology,
            &[
                SchemaChange::DefineNodeType(kind_type.clone()),
                SchemaChange::DefineNodeType(first_type.clone()),
                SchemaChange::ModifyProperty {
                    owner: seed.note,
                    property: PropertyDefinition::new(about, "about", wrap(refs(&[kind]))),
                },
            ],
        )
        .expect("coheres");
        let next = narrow_to_first(&prior, wrap);
        assert!(incompatibilities(&prior, &next, &held).is_empty());

        // Two concrete subtypes: narrowing to one loses `Second`.
        let prior = evolve(
            &seed.ontology,
            &[
                SchemaChange::DefineNodeType(kind_type.clone()),
                SchemaChange::DefineNodeType(first_type.clone()),
                SchemaChange::DefineNodeType(second_type.clone()),
                SchemaChange::ModifyProperty {
                    owner: seed.note,
                    property: PropertyDefinition::new(about, "about", wrap(refs(&[kind]))),
                },
            ],
        )
        .expect("coheres");
        let next = narrow_to_first(&prior, wrap);
        assert_eq!(
            incompatibilities(&prior, &next, &held),
            vec![Incompatibility::ValueTypeNarrowed {
                owner: seed.note,
                property: about,
            }]
        );
    }
}

// ---------------------------------------------------------------------------------------------
// codes

/// One of every refusal either enumeration can produce. A variant added to either enum without a
/// line here fails `every_refusal_code_is_kebab_case_and_declared_by_the_domain` through the
/// `CODES` comparison.
fn every_evolve_error() -> Vec<EvolveError> {
    let type_id = TypeId::mint();
    vec![
        EvolveError::EmptyChanges,
        EvolveError::TypeAlreadyDeclared { type_id },
        EvolveError::UnknownOwner { owner: type_id },
        EvolveError::VersionIdReused {
            id: SchemaVersionId::mint(),
        },
        EvolveError::VersionNumberExhausted {
            id: SchemaVersionId::mint(),
        },
        EvolveError::Incoherent(OntologyError::UnknownType { type_id }),
        EvolveError::WithoutEffect,
    ]
}

fn every_incompatibility() -> Vec<Incompatibility> {
    let (owner, property) = (TypeId::mint(), PropertyId::mint());
    vec![
        Incompatibility::NotASuccessor {
            prior: SchemaVersionId::mint(),
            next: SchemaVersionId::mint(),
        },
        Incompatibility::ConstraintChanged {
            owner,
            property,
            instances: 1,
        },
        Incompatibility::RequiredWithoutValues {
            owner,
            property,
            instances: 1,
        },
        Incompatibility::CardinalityNarrowed {
            owner,
            property,
            cardinality: Cardinality::One,
            max_values: 2,
        },
        Incompatibility::ValueKindNotAdmitted {
            owner,
            property,
            held: ValueKind::String,
            declared: ValueKind::Integer,
        },
        Incompatibility::ValueTypeNarrowed { owner, property },
        Incompatibility::TypeRemoved {
            type_id: owner,
            instances: 1,
        },
        Incompatibility::DeclarationChanged {
            type_id: owner,
            instances: 1,
        },
        Incompatibility::PropertyRemoved {
            owner,
            property,
            max_values: 1,
        },
    ]
}

/// The variants of one `kind: enum` type of `systems/ekr/domains/ontology.yaml`.
fn domain_enum(name: &str) -> BTreeSet<String> {
    let path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("../../systems/ekr/domains/ontology.yaml");
    let text = std::fs::read_to_string(path).expect("the ESS domain is beside the crates");
    let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("parses");
    document["types"]
        .as_sequence()
        .expect("types")
        .iter()
        .find(|declared| declared["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("the domain declares {name}"))["variants"]
        .as_sequence()
        .expect("variants")
        .iter()
        .map(|variant| variant.as_str().expect("a name").to_owned())
        .collect()
}

fn is_kebab(code: &str) -> bool {
    !code.is_empty()
        && !code.starts_with('-')
        && !code.ends_with('-')
        && !code.contains("--")
        && code.chars().all(|c| c.is_ascii_lowercase() || c == '-')
}

#[test]
fn every_refusal_code_is_kebab_case_and_declared_by_the_domain() {
    let evolve_codes: BTreeSet<String> = every_evolve_error()
        .iter()
        .map(|refused| {
            let code = refused.code();
            assert!(is_kebab(code), "{code} is kebab-case");
            assert!(
                refused.to_string().starts_with(&format!("{code}: ")),
                "the message leads with its code: {refused}"
            );
            code.to_owned()
        })
        .collect();
    let incompatibility_codes: BTreeSet<String> = every_incompatibility()
        .iter()
        .map(|found| {
            let code = found.code();
            assert!(is_kebab(code), "{code} is kebab-case");
            assert!(
                found.to_string().starts_with(&format!("{code}: ")),
                "the message leads with its code: {found}"
            );
            code.to_owned()
        })
        .collect();

    let declared: BTreeSet<String> = EvolveError::CODES.iter().map(|c| (*c).to_owned()).collect();
    assert_eq!(
        evolve_codes, declared,
        "every EvolveError variant is constructed above"
    );
    let declared: BTreeSet<String> = Incompatibility::CODES
        .iter()
        .map(|c| (*c).to_owned())
        .collect();
    assert_eq!(
        incompatibility_codes, declared,
        "every Incompatibility variant is constructed above"
    );

    assert_eq!(evolve_codes, domain_enum("ekr.ontology.EvolveRefusalCode"));
    assert_eq!(
        incompatibility_codes,
        domain_enum("ekr.ontology.IncompatibilityCode")
    );
}

#[test]
fn the_domain_states_the_lineage_rule_and_declares_the_schema_change() {
    let path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("../../systems/ekr/domains/ontology.yaml");
    let text = std::fs::read_to_string(path).expect("the ESS domain is beside the crates");
    for stale in [
        "the seed schema is the only version",
        "P1 has exactly the seed",
    ] {
        assert!(!text.contains(stale), "the domain still says {stale:?}");
    }
    let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("parses");
    let change = document["types"]
        .as_sequence()
        .expect("types")
        .iter()
        .find(|declared| declared["name"].as_str() == Some("ekr.ontology.SchemaChange"))
        .expect("the domain declares ekr.ontology.SchemaChange");
    let variants: BTreeSet<&str> = change["variants"]
        .as_mapping()
        .expect("a union")
        .keys()
        .filter_map(serde_yaml_ng::Value::as_str)
        .collect();
    assert_eq!(
        variants,
        BTreeSet::from(["DefineNodeType", "DefineEdgeType", "ModifyProperty"])
    );
}
