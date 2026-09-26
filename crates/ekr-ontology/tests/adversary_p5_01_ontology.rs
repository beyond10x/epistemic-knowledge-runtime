//! Adversary pass 1 on unit A `p5-01-ontology`: `Ontology::evolve` and `incompatibilities`,
//! driven from the documents the unit wrote about them.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{PropertyId, SchemaVersionId, Timestamp, TypeId};
use ekr_ontology::{
    incompatibilities, EdgeType, InstanceState, NodeType, Ontology, OntologyDocument,
    PropertyDefinition, SchemaChange, SchemaVersion, ValueKind, ValueType,
};

const LATER: Timestamp = Timestamp::from_millis(1_000);

/// Canonical state as a case states it. Every instance holds as many values as the fullest one.
#[derive(Default)]
struct State {
    nodes: BTreeMap<TypeId, u64>,
    edges: BTreeMap<TypeId, u64>,
    values: BTreeMap<(TypeId, PropertyId), (u64, ValueKind)>,
}

impl InstanceState for State {
    fn node_count(&self, node_type: TypeId) -> u64 {
        self.nodes.get(&node_type).copied().unwrap_or(0)
    }
    fn edge_count(&self, edge_type: TypeId) -> u64 {
        self.edges.get(&edge_type).copied().unwrap_or(0)
    }
    fn max_values(&self, owner: TypeId, property: PropertyId) -> u64 {
        self.values.get(&(owner, property)).map_or(0, |held| held.0)
    }
    fn min_values(&self, owner: TypeId, property: PropertyId) -> u64 {
        self.max_values(owner, property)
    }
    fn value_kinds(&self, owner: TypeId, property: PropertyId) -> BTreeSet<ValueKind> {
        self.values
            .get(&(owner, property))
            .map(|held| BTreeSet::from([held.1]))
            .unwrap_or_default()
    }
}

/// A `Note` node type with an optional String `title`.
fn note_seed() -> (Ontology, TypeId, PropertyId) {
    let (note, title) = (TypeId::mint(), PropertyId::mint());
    let mut note_type = NodeType::new(note, "Note");
    note_type.properties.insert(
        title,
        PropertyDefinition::new(title, "title", ValueType::String),
    );
    let ontology = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: vec![note_type],
        edge_types: vec![],
    })
    .expect("the seed coheres");
    (ontology, note, title)
}

/// Design § 26 lists "relax or tighten a constraint" among schema proposals and requires "proof
/// that existing canonical state remains valid". `incompatibilities` refuses a type-level change
/// under instances "because `state` cannot say whether they still conform" (evolve.rs:222-224),
/// and `InstanceState` can say no more about an opaque constraint. The kernel refuses every node
/// write to a type whose properties carry one (`crates/ekr-kernel/src/validate/ontology.rs:83`),
/// so this change also freezes the three existing notes.
#[test]
fn a_constraint_added_to_a_property_instances_hold_is_not_passed_as_compatible() {
    let (prior, note, title) = note_seed();
    let mut tightened = prior.properties_of(note)[&title].clone();
    tightened.constraints.push("max_length(8)".to_owned());
    let next = prior
        .evolve(
            SchemaVersionId::mint(),
            LATER,
            &[SchemaChange::ModifyProperty {
                owner: note,
                property: tightened,
            }],
        )
        .expect("a constraint coheres");
    let mut state = State::default();
    state.nodes.insert(note, 3);
    state.values.insert((note, title), (1, ValueKind::String));

    let found = incompatibilities(&prior, &next, &state);
    assert!(
        !found.is_empty(),
        "a constraint no evaluator can check was added to a property 3 instances hold, and \
         incompatibilities found nothing"
    );
}

/// `EvolveError::EmptyChanges` (evolve.rs:135): "a version that changes nothing is not a next
/// version". A change list whose only change redeclares a property exactly as it is declared is
/// a version that changes nothing.
#[test]
fn a_change_list_that_changes_nothing_is_not_a_next_version() {
    let (prior, note, title) = note_seed();
    let same = prior.properties_of(note)[&title].clone();
    let derived = prior.evolve(
        SchemaVersionId::mint(),
        LATER,
        &[SchemaChange::ModifyProperty {
            owner: note,
            property: same,
        }],
    );
    if let Ok(next) = &derived {
        assert_eq!(
            next.to_document().node_types,
            prior.to_document().node_types,
            "precondition: the derived version declares exactly what the prior one did"
        );
    }
    assert!(
        derived.is_err(),
        "evolve derived version {} from an identical redeclaration: a version that changes \
         nothing",
        derived
            .as_ref()
            .map(|next| next.version().number)
            .unwrap_or(0)
    );
}

/// evolve.rs:14-16: "`incompatibilities` still refuses a removal, because it compares any two
/// ontologies and a second route to `next` must not pass one as compatible." A hierarchy change
/// on a type with no instances of its own moves the conformance of a type that has them.
#[test]
fn a_parent_dropped_by_an_abstract_type_moves_the_conformance_of_its_concrete_children() {
    let (general, middle, concrete, link) = (
        TypeId::mint(),
        TypeId::mint(),
        TypeId::mint(),
        TypeId::mint(),
    );
    let document = |middle_parents: BTreeSet<TypeId>, version: SchemaVersion| {
        let mut general_type = NodeType::new(general, "General");
        general_type.abstract_type = true;
        let mut middle_type = NodeType::new(middle, "Middle");
        middle_type.abstract_type = true;
        middle_type.parents = middle_parents;
        let mut concrete_type = NodeType::new(concrete, "Concrete");
        concrete_type.parents = BTreeSet::from([middle]);
        let mut link_type = EdgeType::new(link, "link");
        link_type.source_types.insert(general);
        link_type.target_types.insert(general);
        OntologyDocument {
            version,
            node_types: vec![general_type, middle_type, concrete_type],
            edge_types: vec![link_type],
        }
    };
    let seed = SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH);
    let prior = Ontology::load(document(BTreeSet::from([general]), seed.clone())).expect("prior");
    let next = Ontology::load(document(
        BTreeSet::new(),
        SchemaVersion {
            id: SchemaVersionId::mint(),
            number: 1,
            parent: Some(seed.id),
            created_at: LATER,
        },
    ))
    .expect("next");
    assert!(prior.conforms_to(concrete, general), "precondition");
    assert!(!next.conforms_to(concrete, general), "precondition");

    let mut state = State::default();
    state.nodes.insert(concrete, 2);
    state.edges.insert(link, 1);

    let found = incompatibilities(&prior, &next, &state);
    assert!(
        !found.is_empty(),
        "2 Concrete nodes stop conforming to General, the only endpoint type of 1 link edge, and \
         incompatibilities found nothing"
    );
}
