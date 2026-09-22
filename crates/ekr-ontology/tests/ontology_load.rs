//! An `Ontology` whose property has `value_kind: NodeRef` and an empty `allowed_types` is refused
//! at load — the fourth of the story's shipped tests. A reference to nothing is not a reference.
//!
//! The class this states is wider than the one instance: **a compound value type whose parameter
//! set is empty describes no value at all, and a type that describes no value is refused where it
//! is declared, at whatever depth it is declared.** So the cases here enumerate that class rather
//! than sampling it —
//!
//! * `NodeRef` with no `allowed_types`, at the top of a property and nested inside a `List` and a
//!   `Record`;
//! * `Enum` with no `variants`, likewise;
//! * an edge type with no `source_types` or no `target_types`, which is the same defect one level
//!   up: an edge from nothing to nothing.
//!
//! `List` and `Record` are not in the class and are asserted so: a `List` always carries its
//! element type by construction, and the empty `Record` is a value a caller can actually produce.
//!
//! The other load-time refusals — a reference to an undeclared type, a parent cycle, a duplicate
//! id, a lifecycle that names a state it does not have — are here for the same reason: an ontology
//! that fails these cannot be checked against, so it may not be loaded.

use std::collections::BTreeMap;

use ekr_core::{PropertyId, SchemaVersionId, Timestamp, TypeId};
use ekr_ontology::{
    Cardinality, DeclarationSite, EdgeType, Lifecycle, NodeType, Ontology, OntologyDocument,
    OntologyError, OperationDefinition, PropertyDefinition, SchemaVersion, Transition, ValueKind,
    ValueType,
};

/// A document carrying one `Subject` node type with one property of this declared type, plus a
/// declared `Other` type a `NodeRef` may legitimately point at.
fn document_with(value_type: ValueType) -> (OntologyDocument, PropertyId) {
    let subject = TypeId::mint();
    let other = TypeId::mint();
    let property = PropertyId::mint();

    let mut subject_type = NodeType::new(subject, "Subject");
    subject_type
        .properties
        .insert(property, PropertyDefinition::new(property, "p", value_type));

    (
        OntologyDocument {
            version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
            node_types: vec![subject_type, NodeType::new(other, "Other")],
            edge_types: Vec::new(),
        },
        property,
    )
}

#[test]
fn a_node_ref_with_no_allowed_types_is_refused_at_load() {
    let (document, property) = document_with(ValueType::NodeRef {
        allowed_types: Default::default(),
    });
    let refused = Ontology::load(document).expect_err("a reference to nothing is not a reference");
    assert_eq!(
        refused,
        OntologyError::EmptyValueType {
            site: DeclarationSite::Property(property),
            kind: ValueKind::NodeRef,
        }
    );
    assert!(
        refused.to_string().contains("NodeRef"),
        "the refusal reaches a reader: {refused}"
    );
}

#[test]
fn a_node_property_definition_filed_under_another_id_is_refused_at_load() {
    let (mut document, key) = document_with(ValueType::String);
    let declared = PropertyId::mint();
    document.node_types[0].properties.get_mut(&key).unwrap().id = declared;
    let expected = OntologyError::MisfiledProperty { key, declared };
    assert_eq!(Ontology::load(document.clone()).unwrap_err(), expected);
    let yaml = serde_yaml_ng::to_string(&document).unwrap();
    assert_eq!(Ontology::from_yaml(&yaml).unwrap_err(), expected);
    assert!(expected.to_string().contains(&key.to_string()));
    assert!(expected.to_string().contains(&declared.to_string()));

    document.node_types[0].properties.get_mut(&key).unwrap().id = key;
    assert!(
        Ontology::load(document).is_ok(),
        "matching identity remains valid"
    );
}

#[test]
fn an_edge_property_definition_filed_under_another_id_is_refused_at_load() {
    let (mut document, _) = document_with(ValueType::String);
    let (key, declared) = (PropertyId::mint(), PropertyId::mint());
    let mut edge = EdgeType::new(TypeId::mint(), "depends_on");
    edge.source_types.insert(document.node_types[0].id);
    edge.target_types.insert(document.node_types[1].id);
    edge.properties.insert(
        key,
        PropertyDefinition::new(declared, "support", ValueType::String),
    );
    document.edge_types.push(edge);
    let expected = OntologyError::MisfiledProperty { key, declared };
    assert_eq!(Ontology::load(document.clone()).unwrap_err(), expected);
    let yaml = serde_yaml_ng::to_string(&document).unwrap();
    assert_eq!(Ontology::from_yaml(&yaml).unwrap_err(), expected);

    document.edge_types[0].properties.get_mut(&key).unwrap().id = key;
    assert!(
        Ontology::load(document).is_ok(),
        "matching identity remains valid"
    );
}

#[test]
fn an_empty_compound_value_type_is_refused_at_every_depth_it_is_declared() {
    let empty_ref = || ValueType::NodeRef {
        allowed_types: Default::default(),
    };
    let empty_enum = || ValueType::Enum {
        variants: Default::default(),
    };

    let nested: Vec<(ValueType, ValueKind)> = vec![
        (empty_ref(), ValueKind::NodeRef),
        (ValueType::List(Box::new(empty_ref())), ValueKind::NodeRef),
        (
            ValueType::List(Box::new(ValueType::List(Box::new(empty_ref())))),
            ValueKind::NodeRef,
        ),
        (
            ValueType::Record([("f".to_owned(), empty_ref())].into_iter().collect()),
            ValueKind::NodeRef,
        ),
        (empty_enum(), ValueKind::Enum),
        (ValueType::List(Box::new(empty_enum())), ValueKind::Enum),
        (
            ValueType::Record([("f".to_owned(), empty_enum())].into_iter().collect()),
            ValueKind::Enum,
        ),
    ];

    for (declared, kind) in nested {
        let (document, property) = document_with(declared.clone());
        assert_eq!(
            Ontology::load(document).err(),
            Some(OntologyError::EmptyValueType {
                site: DeclarationSite::Property(property),
                kind
            }),
            "{declared:?} describes no value and must be refused"
        );
    }
}

#[test]
fn a_list_and_an_empty_record_are_not_in_that_class() {
    for declared in [
        ValueType::List(Box::new(ValueType::String)),
        ValueType::Record(BTreeMap::new()),
        ValueType::Record([("f".to_owned(), ValueType::String)].into_iter().collect()),
    ] {
        let (document, _) = document_with(declared.clone());
        assert!(
            Ontology::load(document).is_ok(),
            "{declared:?} describes values a caller can produce"
        );
    }
}

#[test]
fn a_node_ref_to_a_type_the_ontology_does_not_declare_is_refused_at_load() {
    let undeclared = TypeId::mint();
    // Two allowed types, so that the refusal is about the undeclared one and not about the
    // property having no allowed types at all.
    let subject = TypeId::mint();
    let other = TypeId::mint();
    let property = PropertyId::mint();
    let mut subject_type = NodeType::new(subject, "Subject");
    subject_type.properties.insert(
        property,
        PropertyDefinition::new(
            property,
            "p",
            ValueType::NodeRef {
                allowed_types: [undeclared, other].into_iter().collect(),
            },
        ),
    );

    assert_eq!(
        Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
            node_types: vec![subject_type, NodeType::new(other, "Other")],
            edge_types: Vec::new(),
        })
        .err(),
        Some(OntologyError::UnknownType {
            type_id: undeclared
        })
    );
}

#[test]
fn a_parent_the_ontology_does_not_declare_is_refused_at_load() {
    let undeclared = TypeId::mint();
    let subject = TypeId::mint();
    let mut subject_type = NodeType::new(subject, "Subject");
    subject_type.parents.insert(undeclared);

    assert_eq!(
        Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
            node_types: vec![subject_type],
            edge_types: Vec::new(),
        })
        .err(),
        Some(OntologyError::UnknownType {
            type_id: undeclared
        })
    );
}

#[test]
fn a_cycle_in_the_parent_graph_is_refused_at_load() {
    let a = TypeId::mint();
    let b = TypeId::mint();
    let mut first = NodeType::new(a, "A");
    first.parents.insert(b);
    let mut second = NodeType::new(b, "B");
    second.parents.insert(a);

    let refused = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: vec![first, second],
        edge_types: Vec::new(),
    })
    .expect_err("a cycle has no ancestors to enumerate");
    assert!(
        matches!(refused, OntologyError::CyclicParents { .. }),
        "{refused:?}"
    );
}

#[test]
fn a_type_declared_twice_is_refused_at_load() {
    let twice = TypeId::mint();
    assert_eq!(
        Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
            node_types: vec![NodeType::new(twice, "A"), NodeType::new(twice, "B")],
            edge_types: Vec::new(),
        })
        .err(),
        Some(OntologyError::DuplicateType { type_id: twice })
    );
}

#[test]
fn an_edge_type_with_no_source_or_no_target_types_is_refused_at_load() {
    let node = TypeId::mint();
    let edge = TypeId::mint();

    let build = |sources: Vec<TypeId>, targets: Vec<TypeId>| {
        let mut declared = EdgeType::new(edge, "works_at");
        declared.source_types = sources.into_iter().collect();
        declared.target_types = targets.into_iter().collect();
        declared.cardinality = Cardinality::Many;
        OntologyDocument {
            version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
            node_types: vec![NodeType::new(node, "Person")],
            edge_types: vec![declared],
        }
    };

    assert_eq!(
        Ontology::load(build(Vec::new(), vec![node])).err(),
        Some(OntologyError::EmptyEdgeEndpoint {
            type_id: edge,
            endpoint: "source_types".to_owned(),
        })
    );
    assert_eq!(
        Ontology::load(build(vec![node], Vec::new())).err(),
        Some(OntologyError::EmptyEdgeEndpoint {
            type_id: edge,
            endpoint: "target_types".to_owned(),
        })
    );

    let loaded = Ontology::load(build(vec![node], vec![node])).expect("both endpoints declared");
    assert_eq!(
        loaded.edge_type(edge).map(|found| found.name.as_str()),
        Some("works_at")
    );
    assert_eq!(loaded.edge_type(TypeId::mint()), None);
}

#[test]
fn a_lifecycle_naming_a_state_it_does_not_have_is_refused_at_load() {
    let subject = TypeId::mint();
    let build = |lifecycle: Lifecycle| {
        let mut declared = NodeType::new(subject, "Decision");
        declared.lifecycle = Some(lifecycle);
        OntologyDocument {
            version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
            node_types: vec![declared],
            edge_types: Vec::new(),
        }
    };

    let initial_outside = Lifecycle {
        initial: "archived".to_owned(),
        states: ["open"].into_iter().map(str::to_owned).collect(),
        transitions: Default::default(),
    };
    assert_eq!(
        Ontology::load(build(initial_outside)).err(),
        Some(OntologyError::UnknownState {
            type_id: subject,
            state: "archived".to_owned(),
        })
    );

    let endpoint_outside = Lifecycle {
        initial: "open".to_owned(),
        states: ["open"].into_iter().map(str::to_owned).collect(),
        transitions: [Transition::new("open", "decided")].into_iter().collect(),
    };
    assert_eq!(
        Ontology::load(build(endpoint_outside)).err(),
        Some(OntologyError::UnknownState {
            type_id: subject,
            state: "decided".to_owned(),
        })
    );
}

#[test]
fn an_operation_whose_move_the_lifecycle_does_not_declare_is_refused_at_load() {
    let subject = TypeId::mint();
    let lifecycle = Lifecycle {
        initial: "open".to_owned(),
        states: ["open", "decided"].into_iter().map(str::to_owned).collect(),
        transitions: [Transition::new("open", "decided")].into_iter().collect(),
    };

    let build = |transition: Option<Transition>| {
        let mut reopen = OperationDefinition::new("reopen");
        reopen.transition = transition;
        let mut declared = NodeType::new(subject, "Decision");
        declared.lifecycle = Some(lifecycle.clone());
        declared.operations.insert("reopen".to_owned(), reopen);
        OntologyDocument {
            version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
            node_types: vec![declared],
            edge_types: Vec::new(),
        }
    };

    assert_eq!(
        Ontology::load(build(Some(Transition::new("decided", "open")))).err(),
        Some(OntologyError::TransitionNotDeclared {
            type_id: subject,
            operation: "reopen".to_owned(),
            transition: Transition::new("decided", "open"),
        }),
        "an operation may not move a node a way the lifecycle does not declare"
    );
    assert!(
        Ontology::load(build(None)).is_ok(),
        "an operation that moves nothing needs no declared transition"
    );
}

#[test]
fn an_operation_with_a_transition_and_no_lifecycle_at_all_is_refused_at_load() {
    let subject = TypeId::mint();
    let mut decide = OperationDefinition::new("decide");
    decide.transition = Some(Transition::new("open", "decided"));
    let mut declared = NodeType::new(subject, "Decision");
    declared.operations.insert("decide".to_owned(), decide);

    assert_eq!(
        Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
            node_types: vec![declared],
            edge_types: Vec::new(),
        })
        .err(),
        Some(OntologyError::TransitionNotDeclared {
            type_id: subject,
            operation: "decide".to_owned(),
            transition: Transition::new("open", "decided"),
        })
    );
}

/// The ontology is data: it loads from a schema document, and the same refusal holds whether the
/// document was built in memory or read from one. `value_kind` is the discriminant
/// `systems/ekr/domains/ontology.yaml` names.
#[test]
fn an_ontology_loads_from_yaml_and_refuses_the_same_documents() {
    let good = r"
version:
  id: 018f2a00-0000-7000-8000-000000000001
  number: 0
  created_at: 0
node_types:
  - id: 018f2a00-0000-7000-8000-000000000002
    name: Organisation
  - id: 018f2a00-0000-7000-8000-000000000003
    name: Person
    properties:
      018f2a00-0000-7000-8000-000000000004:
        id: 018f2a00-0000-7000-8000-000000000004
        name: employer
        value_type:
          value_kind: NodeRef
          parameters:
            allowed_types:
              - 018f2a00-0000-7000-8000-000000000002
        cardinality: One
        required: true
";
    let ontology = Ontology::from_yaml(good).expect("a well-formed schema document loads");
    let person: TypeId = "018f2a00-0000-7000-8000-000000000003"
        .parse()
        .expect("a canonical uuid");
    assert_eq!(
        ontology.node_type(person).map(|found| found.name.as_str()),
        Some("Person")
    );
    assert_eq!(ontology.version().number, 0);
    assert_eq!(ontology.version().parent, None);

    let empty_allowed = good.replace(
        "            allowed_types:\n              - 018f2a00-0000-7000-8000-000000000002\n",
        "            allowed_types: []\n",
    );
    assert_ne!(empty_allowed, good, "the substitution must have happened");
    let refused = Ontology::from_yaml(&empty_allowed)
        .expect_err("value_kind NodeRef with an empty allowed_types is refused at load");
    assert!(
        matches!(
            refused,
            OntologyError::EmptyValueType {
                kind: ValueKind::NodeRef,
                ..
            }
        ),
        "{refused:?}"
    );

    let malformed = Ontology::from_yaml("node_types: [").expect_err("that is not a document");
    assert!(
        matches!(malformed, OntologyError::Syntax(_)),
        "{malformed:?}"
    );
}

/// Each nested input record can otherwise lose semantics before ontology validation sees them.
#[test]
fn unknown_semantic_members_of_ontology_records_are_refused() {
    let (mut document, property) = document_with(ValueType::String);
    let subject = document.node_types[0].id;
    let transition = Transition::new("open", "decided");
    document.node_types[0].lifecycle = Some(Lifecycle {
        initial: "open".into(),
        states: ["open".into(), "decided".into()].into_iter().collect(),
        transitions: [transition.clone()].into_iter().collect(),
    });
    let mut operation = OperationDefinition::new("decide");
    operation.transition = Some(transition);
    document.node_types[0]
        .operations
        .insert("decide".into(), operation);
    let mut edge = EdgeType::new(TypeId::mint(), "depends_on");
    edge.source_types.insert(subject);
    edge.target_types.insert(subject);
    document.edge_types.push(edge);
    let serialized = serde_json::to_value(&document).unwrap();
    let valid = serde_yaml_ng::to_string(&serialized).unwrap();
    assert_eq!(
        Ontology::from_yaml(&valid).unwrap(),
        Ontology::load(document.clone()).unwrap()
    );
    assert_eq!(
        serde_yaml_ng::from_str::<OntologyDocument>(&valid).unwrap(),
        document
    );

    let sites = [
        (String::new(), "rules"),
        ("/version".into(), "migration"),
        ("/node_types/0".into(), "constraints"),
        ("/edge_types/0".into(), "constraints"),
        (format!("/node_types/0/properties/{property}"), "unique"),
        ("/node_types/0/lifecycle".into(), "guards"),
        ("/node_types/0/lifecycle/transitions/0".into(), "condition"),
        ("/node_types/0/operations/decide".into(), "sets"),
        (
            "/node_types/0/operations/decide/transition".into(),
            "condition",
        ),
    ];
    let mut discarded = Vec::new();
    for (path, field) in sites {
        let mut input = serialized.clone();
        input
            .pointer_mut(&path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(field.into(), serde_json::json!({"requested": "effect"}));
        let yaml = serde_yaml_ng::to_string(&input).unwrap();
        match Ontology::from_yaml(&yaml) {
            Ok(_) => discarded.push(format!("{path}/{field}")),
            Err(OntologyError::Syntax(message)) => assert!(message.contains(field), "{message}"),
            Err(other) => {
                panic!("unknown field must be named by decoding, not masked by {other:?}")
            }
        }
    }
    assert!(
        discarded.is_empty(),
        "decoding silently discarded: {discarded:?}"
    );
}

#[test]
fn unknown_semantics_on_compound_value_types_are_refused() {
    let (mut document, property) = document_with(ValueType::String);
    let subject = document.node_types[0].id;
    document.node_types[0]
        .properties
        .get_mut(&property)
        .unwrap()
        .value_type = ValueType::Record(
        [
            (
                "reference".into(),
                ValueType::NodeRef {
                    allowed_types: [subject].into_iter().collect(),
                },
            ),
            (
                "choice".into(),
                ValueType::Enum {
                    variants: ["open".into()].into_iter().collect(),
                },
            ),
            ("list".into(), ValueType::List(Box::new(ValueType::Integer))),
        ]
        .into_iter()
        .collect(),
    );
    let serialized = serde_json::to_value(&document).unwrap();
    assert!(Ontology::from_yaml(&serde_yaml_ng::to_string(&serialized).unwrap()).is_ok());
    let base = format!("/node_types/0/properties/{property}/value_type");
    let sites = [
        base.clone(),
        format!("{base}/parameters/reference"),
        format!("{base}/parameters/reference/parameters"),
        format!("{base}/parameters/choice/parameters"),
        format!("{base}/parameters/list/parameters"),
    ];
    let mut discarded = Vec::new();
    for path in sites {
        let mut input = serialized.clone();
        input
            .pointer_mut(&path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(
                "unsupported_constraint".into(),
                serde_json::json!("must hold"),
            );
        let yaml = serde_yaml_ng::to_string(&input).unwrap();
        match Ontology::from_yaml(&yaml) {
            Ok(_) => discarded.push(path),
            Err(OntologyError::Syntax(message)) => {
                assert!(message.contains("unsupported_constraint"), "{message}")
            }
            Err(other) => panic!("unexpected refusal {other:?}"),
        }
    }
    assert!(
        discarded.is_empty(),
        "decoding silently discarded: {discarded:?}"
    );
}

#[test]
fn value_envelopes_do_not_discard_unknown_semantics() {
    let input = r#"{"value_kind":"Integer","value":1,"unit":"seconds"}"#;
    let refused = serde_yaml_ng::from_str::<ekr_ontology::Value>(input)
        .expect_err("an unknown unit must not silently become a unitless integer");
    assert!(refused.to_string().contains("unit"));
    let supported = r#"{"value_kind":"Integer","value":1}"#;
    assert_eq!(
        serde_yaml_ng::from_str::<ekr_ontology::Value>(supported).unwrap(),
        ekr_ontology::Value::Integer(1)
    );
}
