//! The acceptance of `story:ontology-types-and-values`: a value generated to violate exactly one
//! property of its declared type is refused.
//!
//! The generator is the oracle. A property's `ValueType` is generated first, a well-typed value is
//! generated *from* that type, and the pair is asserted `Ok` before anything is broken — so a
//! checker that refuses everything dies here rather than passing the acceptance by accident. Then
//! exactly one property of the bag is broken, in exactly one of the five ways a declared type can
//! be violated, and the refusal must name that property and give that reason.
//!
//! The independence of the oracle is the whole point: nothing in this file asks `check_node`
//! whether a value satisfies its type. It asks the generator, which built the value from the type
//! and then broke a named part of it.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{NodeId, PropertyId, SchemaVersionId, TypeId};
use ekr_ontology::{
    Cardinality, CheckReason, NodeType, Ontology, OntologyDocument, PropertyDefinition,
    SchemaVersion, Value, ValueKind, ValueType,
};
use proptest::prelude::*;

/// How many distinct node types a generated `NodeRef` may point at.
const TARGETS: usize = 3;

/// A value type and a well-typed value for it, described together so that one generated case
/// cannot drift between the two.
#[derive(Debug, Clone)]
enum Spec {
    /// A scalar: the value itself, whose [`Value::kind`] gives the declared type.
    Scalar(Value),
    /// A reference constrained to `allowed` (indices into the fixture's target types), pointing at
    /// the `chosen`th of them.
    NodeRef {
        allowed: BTreeSet<usize>,
        chosen: usize,
    },
    /// An enumeration over `variants`, valued at the `chosen`th of them.
    Enum {
        variants: BTreeSet<String>,
        chosen: usize,
    },
    /// A list of `count` elements, each of `element`.
    List { element: Box<Spec>, count: usize },
    /// A record with these fields, in this order (duplicates collapse, in both halves alike).
    Record(Vec<(String, Spec)>),
}

impl Spec {
    /// The declared type this spec describes.
    fn value_type(&self, targets: &[TypeId]) -> ValueType {
        match self {
            Self::Scalar(value) => scalar_type(value.kind()),
            Self::NodeRef { allowed, .. } => ValueType::NodeRef {
                allowed_types: allowed.iter().map(|at| targets[*at]).collect(),
            },
            Self::Enum { variants, .. } => ValueType::Enum {
                variants: variants.clone(),
            },
            Self::List { element, .. } => ValueType::List(Box::new(element.value_type(targets))),
            Self::Record(fields) => ValueType::Record(
                fields
                    .iter()
                    .map(|(name, spec)| (name.clone(), spec.value_type(targets)))
                    .collect(),
            ),
        }
    }

    /// A value that satisfies [`Spec::value_type`], by construction rather than by checking.
    fn value(&self, nodes: &[NodeId]) -> Value {
        match self {
            Self::Scalar(value) => value.clone(),
            Self::NodeRef { allowed, chosen } => {
                let allowed: Vec<usize> = allowed.iter().copied().collect();
                Value::NodeRef(nodes[allowed[chosen % allowed.len()]])
            }
            Self::Enum { variants, chosen } => {
                let variants: Vec<&String> = variants.iter().collect();
                Value::Enum(variants[chosen % variants.len()].clone())
            }
            Self::List { element, count } => {
                Value::List((0..*count).map(|_| element.value(nodes)).collect())
            }
            Self::Record(fields) => Value::Record(
                fields
                    .iter()
                    .map(|(name, spec)| (name.clone(), spec.value(nodes)))
                    .collect(),
            ),
        }
    }
}

/// The declared type of a scalar kind. Panics on a compound kind, which [`Spec::Scalar`] never
/// carries.
fn scalar_type(kind: ValueKind) -> ValueType {
    match kind {
        ValueKind::String => ValueType::String,
        ValueKind::Boolean => ValueType::Boolean,
        ValueKind::Integer => ValueType::Integer,
        ValueKind::Float => ValueType::Float,
        ValueKind::Decimal => ValueType::Decimal,
        ValueKind::Timestamp => ValueType::Timestamp,
        ValueKind::Duration => ValueType::Duration,
        other => panic!("{other} is not a scalar kind"),
    }
}

/// One declared property: its type, how many values it may carry, and whether it must be present.
#[derive(Debug, Clone)]
struct PropSpec {
    spec: Spec,
    cardinality: Cardinality,
    required: bool,
}

/// The five ways a value can violate exactly one property of its declared type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Break {
    /// A value of a different [`ValueKind`] than the property declares.
    WrongKind,
    /// A `NodeRef` to a node whose type is outside `allowed_types`.
    ForeignNodeRef,
    /// An `Enum` value that is not one of the declared variants.
    UndeclaredVariant,
    /// Two values where the declared cardinality permits one.
    TooMany,
    /// A required property that is absent.
    Missing,
}

/// A scalar value of some kind other than `declared`, so that breaking the kind is always
/// available whatever the property declares.
fn value_of_another_kind(declared: ValueKind) -> Value {
    if declared == ValueKind::Boolean {
        Value::Integer(7)
    } else {
        Value::Boolean(true)
    }
}

/// A string that is not one of `variants`.
fn undeclared_variant(variants: &BTreeSet<String>) -> String {
    let mut candidate = String::from("z");
    while variants.contains(&candidate) {
        candidate.push('z');
    }
    candidate
}

/// The loaded ontology and everything a case needs to address it.
struct Fixture {
    ontology: Ontology,
    subject: TypeId,
    property_ids: Vec<PropertyId>,
    nodes: BTreeMap<NodeId, TypeId>,
    node_of_type: Vec<NodeId>,
}

/// A `Subject` node type carrying exactly these properties, over three unrelated target types.
fn fixture(specs: &[PropSpec]) -> Fixture {
    let targets: Vec<TypeId> = (0..TARGETS).map(|_| TypeId::mint()).collect();
    let node_of_type: Vec<NodeId> = (0..TARGETS).map(|_| NodeId::mint()).collect();
    let nodes: BTreeMap<NodeId, TypeId> = node_of_type
        .iter()
        .copied()
        .zip(targets.iter().copied())
        .collect();

    let subject = TypeId::mint();
    let mut subject_type = NodeType::new(subject, "Subject");
    let mut property_ids = Vec::new();
    for (at, declared) in specs.iter().enumerate() {
        let id = PropertyId::mint();
        property_ids.push(id);
        let mut definition =
            PropertyDefinition::new(id, format!("p{at}"), declared.spec.value_type(&targets));
        definition.cardinality = declared.cardinality;
        definition.required = declared.required;
        subject_type.properties.insert(id, definition);
    }

    let mut node_types = vec![subject_type];
    for (at, id) in targets.iter().enumerate() {
        node_types.push(NodeType::new(*id, format!("T{at}")));
    }

    let document = OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), 0),
        node_types,
        edge_types: Vec::new(),
    };

    Fixture {
        ontology: Ontology::load(document).expect("the generated ontology loads"),
        subject,
        property_ids,
        nodes,
        node_of_type,
    }
}

fn scalar_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        "[a-z]{0,6}".prop_map(Value::String),
        any::<bool>().prop_map(Value::Boolean),
        any::<i64>().prop_map(Value::Integer),
        (-1.0e6f64..1.0e6).prop_map(Value::Float),
        "[0-9]{1,6}".prop_map(Value::Decimal),
        any::<i64>().prop_map(Value::Timestamp),
        any::<i64>().prop_map(Value::Duration),
    ]
}

fn spec() -> impl Strategy<Value = Spec> {
    let leaf = prop_oneof![
        scalar_value().prop_map(Spec::Scalar),
        (
            prop::collection::btree_set(0usize..TARGETS, 1..=TARGETS),
            0usize..8
        )
            .prop_map(|(allowed, chosen)| Spec::NodeRef { allowed, chosen }),
        (prop::collection::btree_set("[a-c]{1,3}", 1..4), 0usize..8)
            .prop_map(|(variants, chosen)| Spec::Enum { variants, chosen }),
    ];
    leaf.prop_recursive(2, 8, 3, |inner| {
        prop_oneof![
            (inner.clone(), 0usize..3).prop_map(|(element, count)| Spec::List {
                element: Box::new(element),
                count
            }),
            prop::collection::vec(("[a-c]{1,3}", inner), 1..3).prop_map(Spec::Record),
        ]
    })
}

fn prop_spec() -> impl Strategy<Value = PropSpec> {
    (spec(), any::<bool>(), any::<bool>()).prop_map(|(spec, one, required)| PropSpec {
        spec,
        cardinality: if one {
            Cardinality::One
        } else {
            Cardinality::Many
        },
        required,
    })
}

proptest! {
    #[test]
    fn a_value_breaking_exactly_one_property_of_its_declared_type_is_refused(
        specs in prop::collection::vec(prop_spec(), 1..4),
        target in 0usize..64,
        choice in 0usize..64,
    ) {
        let fixture = fixture(&specs);

        // The generator's own claim: this bag satisfies the declared types.
        let mut bag: BTreeMap<PropertyId, Vec<Value>> = BTreeMap::new();
        for (at, declared) in specs.iter().enumerate() {
            bag.insert(
                fixture.property_ids[at],
                vec![declared.spec.value(&fixture.node_of_type)],
            );
        }
        prop_assert_eq!(
            fixture.ontology.check_node(fixture.subject, &bag, &fixture.nodes).err(),
            None,
            "a value generated from its own type must check Ok"
        );

        // Exactly one property, broken exactly one way.
        let at = target % specs.len();
        let broken = fixture.property_ids[at];
        let declared = &specs[at];

        let mut available = vec![Break::WrongKind];
        match &declared.spec {
            Spec::NodeRef { allowed, .. } if allowed.len() < TARGETS => {
                available.push(Break::ForeignNodeRef);
            }
            Spec::Enum { .. } => available.push(Break::UndeclaredVariant),
            _ => {}
        }
        if declared.cardinality == Cardinality::One {
            available.push(Break::TooMany);
        }
        if declared.required {
            available.push(Break::Missing);
        }
        let how = available[choice % available.len()];

        let good = declared.spec.value(&fixture.node_of_type);
        match how {
            Break::WrongKind => {
                let wrong = value_of_another_kind(good.kind());
                bag.insert(broken, vec![wrong]);
            }
            Break::ForeignNodeRef => {
                let Spec::NodeRef { allowed, .. } = &declared.spec else {
                    unreachable!("ForeignNodeRef is only offered for a NodeRef property")
                };
                let outside = (0..TARGETS)
                    .find(|at| !allowed.contains(at))
                    .expect("fewer than every target type is allowed");
                bag.insert(broken, vec![Value::NodeRef(fixture.node_of_type[outside])]);
            }
            Break::UndeclaredVariant => {
                let Spec::Enum { variants, .. } = &declared.spec else {
                    unreachable!("UndeclaredVariant is only offered for an Enum property")
                };
                bag.insert(broken, vec![Value::Enum(undeclared_variant(variants))]);
            }
            Break::TooMany => {
                bag.insert(broken, vec![good.clone(), good.clone()]);
            }
            Break::Missing => {
                bag.remove(&broken);
            }
        }

        let refused = fixture
            .ontology
            .check_node(fixture.subject, &bag, &fixture.nodes)
            .expect_err("a value that violates its declared type must be refused");

        prop_assert_eq!(
            refused.property(),
            Some(broken),
            "the refusal must name the property that was broken: {:?}",
            refused
        );

        let reason_fits = match how {
            Break::WrongKind => matches!(refused.reason(), CheckReason::WrongKind { .. }),
            Break::ForeignNodeRef => {
                matches!(refused.reason(), CheckReason::NodeRefNotAllowed { .. })
            }
            Break::UndeclaredVariant => {
                matches!(refused.reason(), CheckReason::UndeclaredVariant { .. })
            }
            Break::TooMany => matches!(refused.reason(), CheckReason::CardinalityExceeded { .. }),
            Break::Missing => matches!(refused.reason(), CheckReason::MissingRequiredProperty),
        };
        prop_assert!(
            reason_fits,
            "{:?} was broken by {:?} and refused for {:?}",
            broken,
            how,
            refused.reason()
        );
    }
}

/// The other half of the acceptance, stated on its own so that it cannot be lost in the noise of
/// the generated case: a well-typed value checks `Ok`. A checker that refuses everything passes
/// the acceptance and fails this.
#[test]
fn a_well_typed_value_checks_ok() {
    let target = TypeId::mint();
    let node = NodeId::mint();
    let nodes: BTreeMap<NodeId, TypeId> = [(node, target)].into_iter().collect();

    let subject = TypeId::mint();
    let mut subject_type = NodeType::new(subject, "Subject");

    let name = PropertyId::mint();
    let mut name_definition = PropertyDefinition::new(name, "name", ValueType::String);
    name_definition.required = true;
    subject_type.properties.insert(name, name_definition);

    let employer = PropertyId::mint();
    subject_type.properties.insert(
        employer,
        PropertyDefinition::new(
            employer,
            "employer",
            ValueType::NodeRef {
                allowed_types: [target].into_iter().collect(),
            },
        ),
    );

    let state = PropertyId::mint();
    subject_type.properties.insert(
        state,
        PropertyDefinition::new(
            state,
            "state",
            ValueType::Enum {
                variants: ["open".to_owned(), "decided".to_owned()]
                    .into_iter()
                    .collect(),
            },
        ),
    );

    let tags = PropertyId::mint();
    let mut tags_definition =
        PropertyDefinition::new(tags, "tags", ValueType::List(Box::new(ValueType::String)));
    tags_definition.cardinality = Cardinality::Many;
    subject_type.properties.insert(tags, tags_definition);

    let period = PropertyId::mint();
    subject_type.properties.insert(
        period,
        PropertyDefinition::new(
            period,
            "period",
            ValueType::Record(
                [
                    ("from".to_owned(), ValueType::Timestamp),
                    ("days".to_owned(), ValueType::Duration),
                ]
                .into_iter()
                .collect(),
            ),
        ),
    );

    let ontology = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), 0),
        node_types: vec![subject_type, NodeType::new(target, "Organisation")],
        edge_types: Vec::new(),
    })
    .expect("the ontology loads");

    let bag: BTreeMap<PropertyId, Vec<Value>> = [
        (name, vec![Value::String("Ada".to_owned())]),
        (employer, vec![Value::NodeRef(node)]),
        (state, vec![Value::Enum("open".to_owned())]),
        (
            tags,
            vec![
                Value::List(vec![Value::String("a".to_owned())]),
                Value::List(Vec::new()),
            ],
        ),
        (
            period,
            vec![Value::Record(
                [
                    ("from".to_owned(), Value::Timestamp(1)),
                    ("days".to_owned(), Value::Duration(2)),
                ]
                .into_iter()
                .collect(),
            )],
        ),
    ]
    .into_iter()
    .collect();

    assert_eq!(
        ontology.check_node(subject, &bag, &nodes).err(),
        None,
        "every value here was built from its declared type"
    );
}

/// A refusal names the property and the reason — the story's first shipped test, stated over each
/// reason the checker can give for one value rather than over a generated bag.
#[test]
fn a_refusal_names_the_property_and_the_reason() {
    let allowed_type = TypeId::mint();
    let other_type = TypeId::mint();
    let allowed_node = NodeId::mint();
    let other_node = NodeId::mint();
    let unknown_node = NodeId::mint();
    let nodes: BTreeMap<NodeId, TypeId> = [(allowed_node, allowed_type), (other_node, other_type)]
        .into_iter()
        .collect();

    let subject = TypeId::mint();
    let mut subject_type = NodeType::new(subject, "Subject");

    let reference = PropertyId::mint();
    let mut reference_definition = PropertyDefinition::new(
        reference,
        "employer",
        ValueType::NodeRef {
            allowed_types: [allowed_type].into_iter().collect(),
        },
    );
    reference_definition.required = true;
    subject_type
        .properties
        .insert(reference, reference_definition);

    let ontology = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), 0),
        node_types: vec![
            subject_type,
            NodeType::new(allowed_type, "Organisation"),
            NodeType::new(other_type, "Person"),
        ],
        edge_types: Vec::new(),
    })
    .expect("the ontology loads");

    let check = |values: Vec<Value>| {
        let bag: BTreeMap<PropertyId, Vec<Value>> = [(reference, values)].into_iter().collect();
        ontology.check_node(subject, &bag, &nodes)
    };

    let wrong_kind =
        check(vec![Value::String("OpenAI".to_owned())]).expect_err("a string is not a reference");
    assert_eq!(wrong_kind.property(), Some(reference));
    assert_eq!(
        wrong_kind.reason(),
        &CheckReason::WrongKind {
            expected: ValueKind::NodeRef,
            found: ValueKind::String,
        }
    );
    assert!(
        wrong_kind.to_string().contains("NodeRef"),
        "the reason reaches a reader: {wrong_kind}"
    );

    let foreign = check(vec![Value::NodeRef(other_node)])
        .expect_err("a reference outside allowed_types is refused");
    assert_eq!(foreign.property(), Some(reference));
    assert_eq!(
        foreign.reason(),
        &CheckReason::NodeRefNotAllowed {
            node: other_node,
            node_type: other_type,
        }
    );

    let unresolved = check(vec![Value::NodeRef(unknown_node)])
        .expect_err("a reference to a node of unknown type cannot be shown to be allowed");
    assert_eq!(unresolved.property(), Some(reference));
    assert_eq!(
        unresolved.reason(),
        &CheckReason::UnresolvedNodeRef { node: unknown_node }
    );

    let too_many = check(vec![
        Value::NodeRef(allowed_node),
        Value::NodeRef(allowed_node),
    ])
    .expect_err("Cardinality::One permits one value");
    assert_eq!(too_many.property(), Some(reference));
    assert_eq!(
        too_many.reason(),
        &CheckReason::CardinalityExceeded {
            cardinality: Cardinality::One,
            count: 2,
        }
    );

    let missing = check(Vec::new()).expect_err("a required property is not optional");
    assert_eq!(missing.property(), Some(reference));
    assert_eq!(missing.reason(), &CheckReason::MissingRequiredProperty);

    let undeclared_property = PropertyId::mint();
    let bag: BTreeMap<PropertyId, Vec<Value>> = [
        (reference, vec![Value::NodeRef(allowed_node)]),
        (undeclared_property, vec![Value::Boolean(true)]),
    ]
    .into_iter()
    .collect();
    let undeclared = ontology
        .check_node(subject, &bag, &nodes)
        .expect_err("a property the type does not declare is refused");
    assert_eq!(undeclared.property(), Some(undeclared_property));
    assert_eq!(undeclared.reason(), &CheckReason::UndeclaredProperty);
}

/// `Enum.variants` and the fields of a `Record` are enforced inside a compound value, not only at
/// the top of one — the second of the story's shipped tests, at depth.
#[test]
fn enum_variants_and_record_fields_are_enforced_inside_a_compound_value() {
    let subject = TypeId::mint();
    let mut subject_type = NodeType::new(subject, "Subject");

    let states = PropertyId::mint();
    subject_type.properties.insert(
        states,
        PropertyDefinition::new(
            states,
            "states",
            ValueType::List(Box::new(ValueType::Enum {
                variants: ["open".to_owned(), "decided".to_owned()]
                    .into_iter()
                    .collect(),
            })),
        ),
    );

    let period = PropertyId::mint();
    subject_type.properties.insert(
        period,
        PropertyDefinition::new(
            period,
            "period",
            ValueType::Record(
                [("from".to_owned(), ValueType::Timestamp)]
                    .into_iter()
                    .collect(),
            ),
        ),
    );

    let ontology = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), 0),
        node_types: vec![subject_type],
        edge_types: Vec::new(),
    })
    .expect("the ontology loads");
    let nodes: BTreeMap<NodeId, TypeId> = BTreeMap::new();

    let refused = ontology
        .check_node(
            subject,
            &[(
                states,
                vec![Value::List(vec![
                    Value::Enum("open".to_owned()),
                    Value::Enum("moot".to_owned()),
                ])],
            )]
            .into_iter()
            .collect(),
            &nodes,
        )
        .expect_err("`moot` is not a declared variant");
    assert_eq!(refused.property(), Some(states));
    assert_eq!(
        refused.reason(),
        &CheckReason::UndeclaredVariant {
            variant: "moot".to_owned()
        }
    );

    let missing_field = ontology
        .check_node(
            subject,
            &[(period, vec![Value::Record(BTreeMap::new())])]
                .into_iter()
                .collect(),
            &nodes,
        )
        .expect_err("a declared field is not optional");
    assert_eq!(
        missing_field.reason(),
        &CheckReason::MissingField {
            field: "from".to_owned()
        }
    );

    let extra_field = ontology
        .check_node(
            subject,
            &[(
                period,
                vec![Value::Record(
                    [
                        ("from".to_owned(), Value::Timestamp(1)),
                        ("until".to_owned(), Value::Timestamp(2)),
                    ]
                    .into_iter()
                    .collect(),
                )],
            )]
            .into_iter()
            .collect(),
            &nodes,
        )
        .expect_err("a field the record type does not declare is refused");
    assert_eq!(
        extra_field.reason(),
        &CheckReason::UndeclaredField {
            field: "until".to_owned()
        }
    );
}

/// A value is checked against a type directly, without a property around it — what validators 3–5
/// of design § 20 call when the value is not a node's property.
#[test]
fn a_value_is_checkable_against_a_type_on_its_own() {
    let ontology = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), 0),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("an empty ontology loads");
    let nodes: BTreeMap<NodeId, TypeId> = BTreeMap::new();

    assert_eq!(
        ontology.check_value(
            &Value::Decimal("1.5".to_owned()),
            &ValueType::Decimal,
            &nodes
        ),
        Ok(())
    );
    assert_eq!(
        ontology.check_value(&Value::Float(1.5), &ValueType::Decimal, &nodes),
        Err(CheckReason::WrongKind {
            expected: ValueKind::Decimal,
            found: ValueKind::Float,
        }),
        "Float and Decimal are different types on purpose — canonical.rs rule 4"
    );
}

/// An abstract type is a type no node has; instantiating one is refused before any property is
/// looked at, and an unknown type is refused the same way. Neither refusal names a property.
#[test]
fn an_abstract_or_unknown_type_is_not_instantiable() {
    let abstract_type = TypeId::mint();
    let mut declared = NodeType::new(abstract_type, "Thing");
    declared.abstract_type = true;

    let ontology = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), 0),
        node_types: vec![declared],
        edge_types: Vec::new(),
    })
    .expect("the ontology loads");
    let nodes: BTreeMap<NodeId, TypeId> = BTreeMap::new();
    let empty: BTreeMap<PropertyId, Vec<Value>> = BTreeMap::new();

    let refused = ontology
        .check_node(abstract_type, &empty, &nodes)
        .expect_err("an abstract type is not instantiable");
    assert_eq!(refused.property(), None);
    assert_eq!(
        refused.reason(),
        &CheckReason::AbstractType {
            type_id: abstract_type
        }
    );

    let unknown = TypeId::mint();
    let refused = ontology
        .check_node(unknown, &empty, &nodes)
        .expect_err("a type the ontology does not declare is not instantiable");
    assert_eq!(refused.property(), None);
    assert_eq!(
        refused.reason(),
        &CheckReason::UnknownType { type_id: unknown }
    );
}

/// `Cardinality` decides how many values a property may carry, and nothing else decides it.
#[test]
fn cardinality_permits_the_counts_it_names() {
    assert!(Cardinality::One.permits(0));
    assert!(Cardinality::One.permits(1));
    assert!(!Cardinality::One.permits(2));
    assert!(Cardinality::Many.permits(0));
    assert!(Cardinality::Many.permits(7));
}

/// Every `ValueType` answers with the kind its mirroring `Value` answers with — design § 11.3's
/// "runtime values mirror the declared type", stated as a case rather than left to the reader.
#[test]
fn every_value_kind_mirrors_its_value_type() {
    let node = NodeId::mint();
    let pairs: Vec<(ValueType, Value)> = vec![
        (ValueType::String, Value::String(String::new())),
        (ValueType::Boolean, Value::Boolean(false)),
        (ValueType::Integer, Value::Integer(0)),
        (ValueType::Float, Value::Float(0.0)),
        (ValueType::Decimal, Value::Decimal("0".to_owned())),
        (ValueType::Timestamp, Value::Timestamp(0)),
        (ValueType::Duration, Value::Duration(0)),
        (
            ValueType::NodeRef {
                allowed_types: BTreeSet::new(),
            },
            Value::NodeRef(node),
        ),
        (
            ValueType::Enum {
                variants: BTreeSet::new(),
            },
            Value::Enum(String::new()),
        ),
        (
            ValueType::List(Box::new(ValueType::String)),
            Value::List(Vec::new()),
        ),
        (
            ValueType::Record(BTreeMap::new()),
            Value::Record(BTreeMap::new()),
        ),
    ];

    assert_eq!(pairs.len(), 11, "design § 11.3 gives eleven kinds");
    for (declared, value) in pairs {
        assert_eq!(
            declared.kind(),
            value.kind(),
            "{declared:?} and {value:?} are the same kind"
        );
        assert!(
            !declared.kind().to_string().is_empty(),
            "a kind names itself for an error message"
        );
    }
}
