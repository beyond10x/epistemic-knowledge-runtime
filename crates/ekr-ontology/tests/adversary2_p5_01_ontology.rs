//! Adversary pass 2 on unit A `p5-01-ontology`: `incompatibilities` driven by canonical state
//! built from real nodes and checked with the ontology's own checker, over changes `evolve` makes.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{NodeId, PropertyId, SchemaVersionId, Timestamp, TypeId};
use ekr_ontology::{
    incompatibilities, Cardinality, InstanceState, NodeType, Ontology, OntologyDocument,
    PropertyDefinition, SchemaChange, SchemaVersion, Value, ValueKind, ValueType,
};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestRunner};

const LATER: Timestamp = Timestamp::from_millis(1_000);

/// One node as canonical state holds it: its own type and the values of each property it carries.
struct Node {
    id: NodeId,
    type_id: TypeId,
    properties: BTreeMap<PropertyId, Vec<Value>>,
}

/// An `InstanceState` computed from nodes, exactly as `evolve.rs` documents each answer: `owner`
/// is a node's own type, values are top-level, and `min_values` is 0 when any instance lacks one.
struct Faithful<'a>(&'a [Node]);

impl Faithful<'_> {
    fn of(&self, owner: TypeId) -> impl Iterator<Item = &Node> {
        self.0.iter().filter(move |node| node.type_id == owner)
    }
    fn count(node: &Node, property: PropertyId) -> u64 {
        node.properties
            .get(&property)
            .map_or(0, |values| values.len() as u64)
    }
}

impl InstanceState for Faithful<'_> {
    fn node_count(&self, node_type: TypeId) -> u64 {
        self.of(node_type).count() as u64
    }
    fn edge_count(&self, _: TypeId) -> u64 {
        0
    }
    fn max_values(&self, owner: TypeId, property: PropertyId) -> u64 {
        self.of(owner)
            .map(|node| Self::count(node, property))
            .max()
            .unwrap_or(0)
    }
    fn min_values(&self, owner: TypeId, property: PropertyId) -> u64 {
        self.of(owner)
            .map(|node| Self::count(node, property))
            .min()
            .unwrap_or(0)
    }
    fn value_kinds(&self, owner: TypeId, property: PropertyId) -> BTreeSet<ValueKind> {
        self.of(owner)
            .filter_map(|node| node.properties.get(&property))
            .flatten()
            .map(Value::kind)
            .collect()
    }
}

/// `Base` (abstract) is specialised by `Leaf`; `Other` is unrelated. Only `Leaf` and `Other` can
/// have nodes.
struct Types {
    base: TypeId,
    leaf: TypeId,
    other: TypeId,
}

impl Types {
    fn mint() -> Self {
        Self {
            base: TypeId::mint(),
            leaf: TypeId::mint(),
            other: TypeId::mint(),
        }
    }
    fn by_index(&self, index: usize) -> TypeId {
        [self.base, self.leaf, self.other][index % 3]
    }
    fn declare(&self, properties: &[(TypeId, PropertyDefinition)]) -> Vec<NodeType> {
        let mut base = NodeType::new(self.base, "Base");
        base.abstract_type = true;
        let mut leaf = NodeType::new(self.leaf, "Leaf");
        leaf.parents.insert(self.base);
        let mut other = NodeType::new(self.other, "Other");
        for (owner, property) in properties {
            let declared = [&mut base, &mut leaf, &mut other]
                .into_iter()
                .find(|declared| declared.id == *owner)
                .expect("an owner of the three");
            declared.properties.insert(property.id, property.clone());
        }
        vec![base, leaf, other]
    }
}

fn enumeration(names: &[&str]) -> ValueType {
    ValueType::Enum {
        variants: names.iter().map(|name| (*name).to_owned()).collect(),
    }
}

fn record(field: ValueType) -> ValueType {
    ValueType::Record(BTreeMap::from([("x".to_owned(), field)]))
}

/// The value types a property is drawn from, parameterised over the three node types.
fn value_type(types: &Types, index: usize) -> ValueType {
    let refs = |at: TypeId| ValueType::NodeRef {
        allowed_types: BTreeSet::from([at]),
    };
    match index % 12 {
        0 => ValueType::String,
        1 => ValueType::Integer,
        2 => enumeration(&["a"]),
        3 => enumeration(&["a", "b"]),
        4 => refs(types.base),
        5 => refs(types.leaf),
        6 => refs(types.other),
        7 => ValueType::List(Box::new(ValueType::Integer)),
        8 => ValueType::List(Box::new(enumeration(&["a", "b"]))),
        9 => ValueType::List(Box::new(enumeration(&["a"]))),
        10 => record(ValueType::Integer),
        _ => record(enumeration(&["a", "b"])),
    }
}

/// The values a node may be given, whatever is declared; the prior checker filters them.
fn value(choice: usize, nodes: &[NodeId]) -> Value {
    match choice % 12 {
        0 => Value::String("s".to_owned()),
        1 => Value::Integer(1),
        2 => Value::Enum("a".to_owned()),
        3 => Value::Enum("b".to_owned()),
        4 => Value::NodeRef(nodes[0]),
        5 => Value::NodeRef(nodes[nodes.len() - 1]),
        6 => Value::List(vec![]),
        7 => Value::List(vec![Value::Integer(1)]),
        8 => Value::List(vec![Value::Enum("b".to_owned())]),
        9 => Value::List(vec![Value::Enum("a".to_owned())]),
        10 => Value::Record(BTreeMap::from([("x".to_owned(), Value::Integer(1))])),
        _ => Value::Record(BTreeMap::from([(
            "x".to_owned(),
            Value::Enum("b".to_owned()),
        )])),
    }
}

type Declaration = (usize, bool, bool);

fn definition(types: &Types, property: PropertyId, declared: Declaration) -> PropertyDefinition {
    let mut definition = PropertyDefinition::new(property, "p", value_type(types, declared.0));
    definition.cardinality = if declared.1 {
        Cardinality::Many
    } else {
        Cardinality::One
    };
    definition.required = declared.2;
    definition
}

fn declaration() -> impl Strategy<Value = Declaration> {
    (0..12usize, any::<bool>(), any::<bool>())
}

/// `Faithful`, except that it reports no value kinds: a state that cannot see what it holds, which
/// the soundness property must catch, or it is not testing anything.
struct KindBlind<'a>(Faithful<'a>);

impl InstanceState for KindBlind<'_> {
    fn node_count(&self, node_type: TypeId) -> u64 {
        self.0.node_count(node_type)
    }
    fn edge_count(&self, edge_type: TypeId) -> u64 {
        self.0.edge_count(edge_type)
    }
    fn max_values(&self, owner: TypeId, property: PropertyId) -> u64 {
        self.0.max_values(owner, property)
    }
    fn min_values(&self, owner: TypeId, property: PropertyId) -> u64 {
        self.0.min_values(owner, property)
    }
    fn value_kinds(&self, _: TypeId, _: PropertyId) -> BTreeSet<ValueKind> {
        BTreeSet::new()
    }
}

/// Soundness of the refusal set: over canonical state that `prior`'s own checker admits, a
/// `ModifyProperty` that `incompatibilities` finds nothing wrong with leaves every node admitted
/// by `next`'s checker. Returns the outcome and how many changes over held values were found
/// compatible and refused.
fn soundness(blind_to_kinds: bool) -> (Result<(), String>, u32, u32) {
    let mut runner = TestRunner::new(Config {
        cases: 20_000,
        max_global_rejects: 1_000_000,
        failure_persistence: None,
        rng_seed: RngSeed::Fixed(0x00AD_5E02),
        ..Config::default()
    });
    let strategy = (
        0..3usize,
        declaration(),
        proptest::option::of(declaration()),
        0..3usize,
        declaration(),
        proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(0..12usize, 0..3)),
            1..5,
        ),
    );
    // How many cases reached each verdict over nodes that carry values, so that a draw rejected
    // into vacuity fails here rather than passing.
    let (compatible, refused) = (std::cell::Cell::new(0u32), std::cell::Cell::new(0u32));
    let outcome =
        runner.run(
            &strategy,
            |(declarer, first, leaf_override, owner, change, drawn)| {
                let types = Types::mint();
                let property = PropertyId::mint();
                let mut declared = vec![(
                    types.by_index(declarer),
                    definition(&types, property, first),
                )];
                if let (0, Some(leaf_declared)) = (declarer, leaf_override) {
                    declared.push((types.leaf, definition(&types, property, leaf_declared)));
                }
                let seed = SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH);
                let Ok(prior) = Ontology::load(OntologyDocument {
                    version: seed,
                    node_types: types.declare(&declared),
                    edge_types: vec![],
                }) else {
                    return Err(TestCaseError::reject("prior does not cohere"));
                };

                let ids: Vec<NodeId> = drawn.iter().map(|_| NodeId::mint()).collect();
                let mut node_types: BTreeMap<NodeId, TypeId> = BTreeMap::new();
                for ((is_leaf, _), id) in drawn.iter().zip(&ids) {
                    node_types.insert(*id, if *is_leaf { types.leaf } else { types.other });
                }
                let mut nodes = Vec::new();
                for ((_, choices), id) in drawn.iter().zip(&ids) {
                    let type_id = node_types[id];
                    let values: Vec<Value> = choices.iter().map(|at| value(*at, &ids)).collect();
                    let carried = BTreeMap::from([(property, values)]);
                    let properties = if prior.check_node(type_id, &carried, &node_types).is_ok() {
                        carried
                    } else if prior
                        .check_node(type_id, &BTreeMap::new(), &node_types)
                        .is_ok()
                    {
                        BTreeMap::new()
                    } else {
                        return Err(TestCaseError::reject("no valid node of this draw"));
                    };
                    nodes.push(Node {
                        id: *id,
                        type_id,
                        properties,
                    });
                }

                let Ok(next) = prior.evolve(
                    SchemaVersionId::mint(),
                    LATER,
                    &[SchemaChange::ModifyProperty {
                        owner: types.by_index(owner),
                        property: definition(&types, property, change),
                    }],
                ) else {
                    return Err(TestCaseError::reject("evolve refused"));
                };

                let found = if blind_to_kinds {
                    incompatibilities(&prior, &next, &KindBlind(Faithful(&nodes)))
                } else {
                    incompatibilities(&prior, &next, &Faithful(&nodes))
                };
                let moved = [types.leaf, types.other].iter().any(|at| {
                    prior.properties_of(*at).get(&property)
                        != next.properties_of(*at).get(&property)
                });
                let held = nodes
                    .iter()
                    .any(|node| node.properties.values().any(|values| !values.is_empty()));
                if moved && held {
                    let tally = if found.is_empty() {
                        &compatible
                    } else {
                        &refused
                    };
                    tally.set(tally.get() + 1);
                }
                if found.is_empty() {
                    for node in &nodes {
                        if let Err(refused) =
                            next.check_node(node.type_id, &node.properties, &node_types)
                        {
                            return Err(TestCaseError::fail(format!(
                            "incompatibilities found nothing, and node {} ({}) is refused by the \
                             next version: {refused}",
                            node.id,
                            if node.type_id == types.leaf { "Leaf" } else { "Other" },
                        )));
                        }
                    }
                }
                Ok(())
            },
        );
    (
        outcome.map_err(|failure| failure.to_string()),
        compatible.get(),
        refused.get(),
    )
}

/// Design § 26's "proof that existing canonical state remains valid", and the kernel brief's
/// acceptance "Existing nodes ... stay valid across the change", as a property with a fixed seed.
#[test]
fn a_redeclaration_found_compatible_leaves_every_node_valid_under_the_next_version() {
    let (outcome, compatible, refused) = soundness(false);
    if let Err(failure) = outcome {
        panic!("{failure}");
    }
    assert!(
        compatible >= 100 && refused >= 100,
        "the draw is too thin to say anything: {compatible} compatible and {refused} refused \
         changes over held values"
    );
}

/// The property above has teeth: a state blind to the kinds it holds lets a kind change through,
/// and the property says so.
#[test]
fn the_soundness_property_catches_a_state_blind_to_its_value_kinds() {
    let (outcome, _, _) = soundness(true);
    assert!(
        outcome.is_err(),
        "a kind-blind state passed the soundness property"
    );
}

/// A false positive: every value a reference to the abstract `Base` can hold is a reference to a
/// node of a concrete type conforming to it, and `Leaf` is the only one. Narrowing the reference
/// to `Leaf` admits exactly what it admitted before, and the next version's checker says so for
/// the one node there is — `incompatibilities` still refuses it as `value-type-narrowed`.
#[test]
fn narrowing_a_reference_from_an_abstract_type_to_its_only_concrete_subtype_admits_every_value() {
    let types = Types::mint();
    let (about, target, holder) = (PropertyId::mint(), NodeId::mint(), NodeId::mint());
    let refs = |at: TypeId| ValueType::NodeRef {
        allowed_types: BTreeSet::from([at]),
    };
    let prior = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: types.declare(&[(
            types.other,
            PropertyDefinition::new(about, "about", refs(types.base)),
        )]),
        edge_types: vec![],
    })
    .expect("prior coheres");
    let next = prior
        .evolve(
            SchemaVersionId::mint(),
            LATER,
            &[SchemaChange::ModifyProperty {
                owner: types.other,
                property: PropertyDefinition::new(about, "about", refs(types.leaf)),
            }],
        )
        .expect("next coheres");

    let node_types = BTreeMap::from([(target, types.leaf), (holder, types.other)]);
    let nodes = [
        Node {
            id: target,
            type_id: types.leaf,
            properties: BTreeMap::new(),
        },
        Node {
            id: holder,
            type_id: types.other,
            properties: BTreeMap::from([(about, vec![Value::NodeRef(target)])]),
        },
    ];
    for node in &nodes {
        prior
            .check_node(node.type_id, &node.properties, &node_types)
            .expect("precondition: prior admits the node");
        next.check_node(node.type_id, &node.properties, &node_types)
            .expect("precondition: next admits the node");
    }

    let found = ekr_ontology::incompatibilities(&prior, &next, &Faithful(&nodes));
    assert!(
        found.is_empty(),
        "every node is admitted by the next version, and incompatibilities refused: {found:?}"
    );
}
