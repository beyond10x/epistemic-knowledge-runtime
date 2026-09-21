//! `NodeType.parents` is not decoration: a type's properties are its own and its ancestors', and a
//! `NodeRef` constrained to a type accepts a node of any type that conforms to it.
//!
//! Design § 11.1 gives `parents`, and § 12 requires that a source type be "compatible with"
//! `edge.source_types` — compatibility is conformance, so the checker resolves it through the
//! hierarchy rather than by identity.

use std::collections::BTreeMap;

use ekr_core::{NodeId, PropertyId, SchemaVersionId, TypeId};
use ekr_ontology::{
    CheckReason, NodeType, Ontology, OntologyDocument, PropertyDefinition, SchemaVersion, Value,
    ValueType,
};

/// `Thing` (abstract, has `name`) ← `Organisation` (has `founded`) ← `Charity`, and an unrelated
/// `Rock`.
struct Hierarchy {
    ontology: Ontology,
    thing: TypeId,
    organisation: TypeId,
    charity: TypeId,
    rock: TypeId,
    name: PropertyId,
    founded: PropertyId,
}

fn hierarchy() -> Hierarchy {
    let thing = TypeId::mint();
    let organisation = TypeId::mint();
    let charity = TypeId::mint();
    let rock = TypeId::mint();
    let name = PropertyId::mint();
    let founded = PropertyId::mint();

    let mut thing_type = NodeType::new(thing, "Thing");
    thing_type.abstract_type = true;
    let mut name_definition = PropertyDefinition::new(name, "name", ValueType::String);
    name_definition.required = true;
    thing_type.properties.insert(name, name_definition);

    let mut organisation_type = NodeType::new(organisation, "Organisation");
    organisation_type.parents.insert(thing);
    organisation_type.properties.insert(
        founded,
        PropertyDefinition::new(founded, "founded", ValueType::Timestamp),
    );

    let mut charity_type = NodeType::new(charity, "Charity");
    charity_type.parents.insert(organisation);

    Hierarchy {
        ontology: Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(SchemaVersionId::mint(), 0),
            node_types: vec![
                thing_type,
                organisation_type,
                charity_type,
                NodeType::new(rock, "Rock"),
            ],
            edge_types: Vec::new(),
        })
        .expect("the hierarchy loads"),
        thing,
        organisation,
        charity,
        rock,
        name,
        founded,
    }
}

#[test]
fn a_type_conforms_to_itself_and_to_every_ancestor_and_to_nothing_else() {
    let h = hierarchy();

    assert!(h.ontology.conforms_to(h.charity, h.charity));
    assert!(h.ontology.conforms_to(h.charity, h.organisation));
    assert!(h.ontology.conforms_to(h.charity, h.thing), "transitively");
    assert!(h.ontology.conforms_to(h.organisation, h.thing));

    assert!(!h.ontology.conforms_to(h.thing, h.charity), "not downwards");
    assert!(!h.ontology.conforms_to(h.organisation, h.charity));
    assert!(!h.ontology.conforms_to(h.charity, h.rock));
    assert!(!h.ontology.conforms_to(h.rock, h.thing));
    assert!(
        !h.ontology.conforms_to(TypeId::mint(), h.thing),
        "an undeclared type conforms to nothing"
    );
}

#[test]
fn a_types_properties_are_its_own_and_its_ancestors() {
    let h = hierarchy();

    let of_thing = h.ontology.properties_of(h.thing);
    assert_eq!(of_thing.keys().copied().collect::<Vec<_>>(), vec![h.name]);

    let of_charity = h.ontology.properties_of(h.charity);
    let mut expected = vec![h.name, h.founded];
    expected.sort();
    assert_eq!(of_charity.keys().copied().collect::<Vec<_>>(), expected);
    assert_eq!(
        of_charity.get(&h.name).map(|found| found.name.as_str()),
        Some("name")
    );
    assert!(
        h.ontology.properties_of(TypeId::mint()).is_empty(),
        "an undeclared type has no properties"
    );
}

#[test]
fn an_inherited_required_property_is_required_of_the_descendant() {
    let h = hierarchy();
    let nodes: BTreeMap<NodeId, TypeId> = BTreeMap::new();

    let refused = h
        .ontology
        .check_node(h.charity, &BTreeMap::new(), &nodes)
        .expect_err("`name` is required of every Thing, so of every Charity");
    assert_eq!(refused.property(), Some(h.name));
    assert_eq!(refused.reason(), &CheckReason::MissingRequiredProperty);

    assert_eq!(
        h.ontology
            .check_node(
                h.charity,
                &[(h.name, vec![Value::String("Oxfam".to_owned())])]
                    .into_iter()
                    .collect(),
                &nodes,
            )
            .err(),
        None
    );
}

#[test]
fn a_node_ref_accepts_a_node_whose_type_conforms_to_an_allowed_type() {
    let h = hierarchy();
    let charity_node = NodeId::mint();
    let rock_node = NodeId::mint();
    let nodes: BTreeMap<NodeId, TypeId> = [(charity_node, h.charity), (rock_node, h.rock)]
        .into_iter()
        .collect();

    let employer = PropertyId::mint();
    let subject = TypeId::mint();
    let mut subject_type = NodeType::new(subject, "Person");
    subject_type.properties.insert(
        employer,
        PropertyDefinition::new(
            employer,
            "employer",
            ValueType::NodeRef {
                allowed_types: [h.organisation].into_iter().collect(),
            },
        ),
    );

    let mut document = OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), 0),
        node_types: vec![subject_type],
        edge_types: Vec::new(),
    };
    for (id, name) in [
        (h.thing, "Thing"),
        (h.organisation, "Organisation"),
        (h.charity, "Charity"),
        (h.rock, "Rock"),
    ] {
        let mut declared = NodeType::new(id, name);
        if id == h.organisation {
            declared.parents.insert(h.thing);
        }
        if id == h.charity {
            declared.parents.insert(h.organisation);
        }
        document.node_types.push(declared);
    }
    let ontology = Ontology::load(document).expect("the ontology loads");

    let check = |node: NodeId| {
        ontology.check_node(
            subject,
            &[(employer, vec![Value::NodeRef(node)])]
                .into_iter()
                .collect(),
            &nodes,
        )
    };

    assert_eq!(
        check(charity_node).err(),
        None,
        "a Charity is an Organisation"
    );
    assert_eq!(
        check(rock_node)
            .expect_err("a Rock is not an Organisation")
            .reason(),
        &CheckReason::NodeRefNotAllowed {
            node: rock_node,
            node_type: h.rock,
        }
    );
}
