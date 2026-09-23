//! Properties of validation and of the `validation_hash`, over generated transactions.
//!
//! `tests/validation.rs` states what the story's acceptance asks, one defect at a time. This file
//! states the things that must hold for *every* transaction, and the reason it exists rather than
//! a table of hand-written rows is the shape of the risk: a `validation_hash` that omits one field
//! of one operation is a hash two different transactions share, and a mutation table catches
//! exactly the fields somebody remembered to write a row for.
//!
//! The generator draws ids from a small fixed pool, deliberately. Independent draws from a large
//! space differ in every field at once and would never put the injectivity property under
//! pressure; a pool of two or three makes two generated transactions that differ in exactly one
//! field an ordinary outcome, which is the case a dropped field survives.
//!
//! `tests/validation.rs` holds the structural sibling of the first property —
//! `every_field_of_every_encoded_type_reaches_its_encoding`, which reads the source. The two are
//! not redundant: the structural one cannot see a field that is named in the encoding but written
//! in a way that loses it, and this one cannot see a field no generated value ever varies.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionNumber, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue, GraphRoot, GraphSnapshot,
    Object, Predicate, Space, Subject, TemporalRange, TransactionTime, ValueSpace,
};
use ekr_kernel::{
    EdgeDraft, EntityMerge, GraphOperation, GraphTransaction, NodeDraft, Pipeline,
    PropertyMutation, ValidatorName,
};
use ekr_ontology::{
    Cardinality, EdgeType, NodeType, Ontology, OntologyDocument, PropertyDefinition, SchemaVersion,
    Value, ValueType,
};
use proptest::prelude::*;

/// The identities every generated transaction draws from: minted once, so that two transactions in
/// one run can differ in exactly one of them.
struct Pool {
    transactions: [TransactionId; 2],
    agents: [AgentId; 2],
    roots: [GraphRootId; 2],
    nodes: [NodeId; 3],
    edges: [EdgeId; 2],
    assertions: [AssertionId; 2],
    evidence: [EvidenceId; 2],
    types: [TypeId; 2],
    properties: [PropertyId; 2],
}

static POOL: LazyLock<Pool> = LazyLock::new(|| Pool {
    transactions: [TransactionId::mint(), TransactionId::mint()],
    agents: [AgentId::mint(), AgentId::mint()],
    roots: [GraphRootId::mint(), GraphRootId::mint()],
    nodes: [NodeId::mint(), NodeId::mint(), NodeId::mint()],
    edges: [EdgeId::mint(), EdgeId::mint()],
    assertions: [AssertionId::mint(), AssertionId::mint()],
    evidence: [EvidenceId::mint(), EvidenceId::mint()],
    types: [TypeId::mint(), TypeId::mint()],
    properties: [PropertyId::mint(), PropertyId::mint()],
});

/// A value canonical state admits, including the two compound shapes.
fn canonical_value() -> impl Strategy<Value = CanonicalValue> {
    let leaf = prop_oneof![
        "(one|two)".prop_map(CanonicalValue::String),
        any::<bool>().prop_map(CanonicalValue::Boolean),
        (0i64..3).prop_map(CanonicalValue::Integer),
        (0i64..3).prop_map(CanonicalValue::Duration),
        "(0|1)[.]0".prop_map(CanonicalValue::Decimal),
        (0usize..3).prop_map(|at| CanonicalValue::NodeRef(CanonicalRef::new(POOL.nodes[at]))),
        (0i64..3).prop_map(|at| CanonicalValue::Timestamp(Timestamp::from_millis(at))),
    ];
    leaf.prop_recursive(2, 6, 2, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..3).prop_map(CanonicalValue::List),
            proptest::collection::btree_map("(a|b)", inner, 0..3).prop_map(CanonicalValue::Record),
        ]
    })
}

/// A proposal's value, which may be the one canonical state does not admit.
fn proposed_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        8 => canonical_value().prop_map(Value::from),
        1 => (0i64..3).prop_map(|at| Value::Float(at as f64)),
    ]
}

/// A property definition, which is one of the declarations the kernel encodes by hand.
fn definition() -> impl Strategy<Value = PropertyDefinition> {
    (
        0usize..2,
        "(title|tags)",
        prop_oneof![
            Just(ValueType::String),
            Just(ValueType::Integer),
            Just(ValueType::List(Box::new(ValueType::String))),
        ],
        prop_oneof![Just(Cardinality::One), Just(Cardinality::Many)],
        any::<bool>(),
        proptest::collection::vec("(len|sum)", 0..2),
    )
        .prop_map(
            |(at, name, value_type, cardinality, required, constraints)| {
                let mut declared = PropertyDefinition::new(POOL.properties[at], name, value_type);
                declared.cardinality = cardinality;
                declared.required = required;
                declared.constraints = constraints;
                declared
            },
        )
}

/// A node type, the largest of the declarations the kernel encodes by hand.
fn node_type() -> impl Strategy<Value = NodeType> {
    (
        0usize..2,
        "(Decision|Observation)",
        any::<bool>(),
        proptest::collection::vec(definition(), 0..2),
        proptest::collection::vec(0usize..2, 0..2),
    )
        .prop_map(|(at, name, abstract_type, properties, parents)| {
            let mut declared = NodeType::new(POOL.types[at], name);
            declared.abstract_type = abstract_type;
            declared.properties = properties
                .into_iter()
                .map(|definition| (definition.id, definition))
                .collect();
            declared.parents = parents.into_iter().map(|at| POOL.types[at]).collect();
            declared
        })
}

/// An edge type, the other declaration the kernel encodes by hand and the one nothing constructed.
///
/// Every one of its nine fields varies, because the property below is what puts the variant tag
/// and the field *order* of that encoding under pressure: with no generated value carrying a
/// `DefineEdgeType`, swapping two of its fields left the whole suite green.
fn edge_type() -> impl Strategy<Value = EdgeType> {
    (
        0usize..2,
        "(depends_on|supersedes)",
        proptest::collection::vec(0usize..2, 0..2),
        proptest::collection::vec(0usize..2, 0..2),
        prop_oneof![Just(Cardinality::One), Just(Cardinality::Many)],
        proptest::collection::vec(definition(), 0..2),
        proptest::option::of(0usize..2),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(
                at,
                name,
                source_types,
                target_types,
                cardinality,
                properties,
                inverse,
                symmetric,
                transitive,
            )| {
                let mut declared = EdgeType::new(POOL.types[at], name);
                declared.source_types = source_types.into_iter().map(|at| POOL.types[at]).collect();
                declared.target_types = target_types.into_iter().map(|at| POOL.types[at]).collect();
                declared.cardinality = cardinality;
                declared.properties = properties
                    .into_iter()
                    .map(|definition| (definition.id, definition))
                    .collect();
                declared.inverse = inverse.map(|at| POOL.types[at]);
                declared.symmetric = symmetric;
                declared.transitive = transitive;
                declared
            },
        )
}

/// An assertion over `V`.
fn assertion<V: ValueSpace + std::fmt::Debug + Clone + 'static>(
    value: impl Strategy<Value = V>,
) -> impl Strategy<Value = Assertion<V>> {
    (
        0usize..2,
        0usize..2,
        0usize..3,
        0usize..2,
        0usize..2,
        0i64..3,
        value,
        any::<bool>(),
    )
        .prop_map(
            |(assertion_at, root_at, node_at, property_at, agent_at, at, value, has_evidence)| {
                Assertion {
                    id: POOL.assertions[assertion_at],
                    root_id: POOL.roots[root_at],
                    subject: Subject::Node(V::node_ref(POOL.nodes[node_at])),
                    predicate: Predicate::Property(POOL.properties[property_at]),
                    object: Object::Value(value),
                    evidence: if has_evidence {
                        [V::evidence_ref(POOL.evidence[0])].into_iter().collect()
                    } else {
                        BTreeSet::new()
                    },
                    proposed_by: POOL.agents[agent_at],
                    lifecycle: ekr_graph::AssertionLifecycle::Active,
                    assessment: Assessment::Proposed,
                    valid_time: TemporalRange::since(Timestamp::from_millis(at)),
                    transaction_time: TransactionTime::since(Timestamp::from_millis(at)),
                }
            },
        )
}

/// One operation over `V`, drawing its values from `value()` each time it needs one.
fn operation<V, S, F>(value: F) -> impl Strategy<Value = GraphOperation<V>>
where
    V: ValueSpace + std::fmt::Debug + Clone + 'static,
    S: Strategy<Value = V> + 'static,
    F: Fn() -> S + Clone + 'static,
{
    let values = {
        let value = value.clone();
        move || proptest::collection::vec(value(), 0..3)
    };
    let properties = {
        let values = values.clone();
        move || proptest::collection::btree_map(0usize..2, values(), 0..2)
    };
    prop_oneof![
        (0usize..3, 0usize..2, 0usize..2, "(one|two)", properties()).prop_map(
            |(node_at, root_at, type_at, canonical_name, properties)| GraphOperation::CreateNode(
                NodeDraft {
                    id: POOL.nodes[node_at],
                    root_id: POOL.roots[root_at],
                    type_id: POOL.types[type_at],
                    canonical_name,
                    properties: properties
                        .into_iter()
                        .map(|(at, values)| (POOL.properties[at], values))
                        .collect(),
                }
            )
        ),
        (0usize..3, 0usize..2, values()).prop_map(|(node_at, property_at, values)| {
            GraphOperation::UpdateProperty(PropertyMutation {
                node: POOL.nodes[node_at],
                property: POOL.properties[property_at],
                values,
            })
        }),
        (
            0usize..2,
            0usize..2,
            0usize..2,
            0usize..3,
            0usize..3,
            properties()
        )
            .prop_map(
                |(edge_at, root_at, type_at, source_at, target_at, properties)| {
                    GraphOperation::CreateEdge(EdgeDraft {
                        id: POOL.edges[edge_at],
                        root_id: POOL.roots[root_at],
                        type_id: POOL.types[type_at],
                        source: POOL.nodes[source_at],
                        target: POOL.nodes[target_at],
                        properties: properties
                            .into_iter()
                            .map(|(at, values)| (POOL.properties[at], values))
                            .collect(),
                    })
                }
            ),
        (0usize..2).prop_map(|at| GraphOperation::DeleteEdge(POOL.edges[at])),
        assertion(value()).prop_map(|held| GraphOperation::AddAssertion(Box::new(held))),
        (0usize..2).prop_map(
            |at| GraphOperation::RetractAssertion(ekr_kernel::Retraction {
                assertion: POOL.assertions[at],
                reason: ekr_graph::RetractionReason::new("fixture withdrawal")
            })
        ),
        (0usize..2, 0usize..2).prop_map(|(old, new)| GraphOperation::SupersedeAssertion(
            ekr_kernel::Supersession {
                assertion: POOL.assertions[old],
                by: POOL.assertions[new],
                effective_from: Timestamp::EPOCH
            }
        )),
        node_type().prop_map(|declared| GraphOperation::DefineNodeType(Box::new(declared))),
        edge_type().prop_map(|declared| GraphOperation::DefineEdgeType(Box::new(declared))),
        definition().prop_map(GraphOperation::ModifyProperty),
        // The second index steps past the first, so `absorbed != into` always. Drawing the two
        // independently from a pool of three produced a node merged into itself about one
        // proposal in three — a shape the structural validator now refuses, so a generator that
        // kept producing it would be asking a different question from the one these properties
        // say they ask.
        (0usize..3, 1usize..3).prop_map(|(absorbed, step)| GraphOperation::MergeEntity(
            EntityMerge {
                absorbed: POOL.nodes[absorbed],
                into: POOL.nodes[(absorbed + step) % 3],
            }
        )),
        (0usize..3, "(decide|reopen)", {
            let value = value.clone();
            proptest::collection::btree_map("(a|b)", value(), 0..2)
        })
            .prop_map(|(node_at, operation, arguments)| GraphOperation::Invoke {
                node: POOL.nodes[node_at],
                operation,
                arguments,
            }),
    ]
}

/// A transaction over `V`.
fn transaction<V, S, F>(value: F) -> impl Strategy<Value = GraphTransaction<V>>
where
    V: ValueSpace + std::fmt::Debug + Clone + 'static,
    S: Strategy<Value = V> + 'static,
    F: Fn() -> S + Clone + 'static,
{
    (
        0usize..2,
        0usize..2,
        proptest::collection::vec(operation(value), 0..3),
        proptest::collection::vec(0usize..2, 0..2),
    )
        .prop_map(
            |(transaction_at, agent_at, operations, evidence)| GraphTransaction {
                id: POOL.transactions[transaction_at],
                proposer: POOL.agents[agent_at],
                operations,
                evidence: evidence.into_iter().map(|at| POOL.evidence[at]).collect(),
            },
        )
}

/// Canonical state to validate against: one type, one property, no nodes.
fn empty_graph() -> CanonicalGraph {
    let schema = SchemaVersionId::mint();
    let mut declared = NodeType::new(POOL.types[0], "Decision");
    declared.properties.insert(
        POOL.properties[0],
        PropertyDefinition::new(POOL.properties[0], "title", ValueType::String),
    );
    CanonicalGraph {
        root: GraphRoot {
            id: POOL.roots[0],
            space: Space::Canonical,
            schema_version_id: schema,
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::new(3),
        ontology: Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(schema, Timestamp::EPOCH),
            node_types: vec![declared],
            edge_types: Vec::new(),
        })
        .expect("one type and one property cohere"),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    }
}

proptest! {
    /// Two transactions have one address exactly when they are one transaction.
    ///
    /// The direction that matters is the second: a field left out of an `encode` makes two
    /// different transactions share a `validation_hash`, and a transaction that shares an address
    /// with another can be substituted for it between validation and commit.
    #[test]
    fn the_encoding_separates_transactions_that_differ(
        left in transaction(canonical_value),
        right in transaction(canonical_value),
    ) {
        let (one, other) = (ContentHash::of(&left), ContentHash::of(&right));
        prop_assert_eq!(
            left == right,
            one == other,
            "{:?} and {:?} share an address only if they are equal",
            left,
            right
        );
    }

    /// Validation is a function of the proposal and the snapshot, and of nothing else.
    #[test]
    fn validating_twice_gives_the_same_answer(proposal in transaction(proposed_value)) {
        let graph = empty_graph();
        let snapshot = GraphSnapshot::of(&graph);
        let pipeline = Pipeline::deterministic(POOL.agents[1]);

        let first = pipeline.validate(&snapshot, &proposal);
        let second = pipeline.validate(&snapshot, &proposal);
        match (first, second) {
            (Ok(one), Ok(other)) => {
                prop_assert_eq!(one.validation_hash(), other.validation_hash());
                prop_assert_eq!(one.validated_against(), RevisionNumber::new(3));
            }
            (Err(one), Err(other)) => prop_assert_eq!(one, other),
            (one, other) => prop_assert!(
                false,
                "two runs of one validation disagreed: {:?} and {:?}",
                one.is_ok(),
                other.is_ok()
            ),
        }
    }

    /// Design § 6.10, for every transaction rather than for one: an agent validating its own
    /// proposal is refused, whatever the proposal is.
    #[test]
    fn no_transaction_is_ever_validated_by_its_own_proposer(
        proposal in transaction(proposed_value),
    ) {
        let graph = empty_graph();
        let snapshot = GraphSnapshot::of(&graph);
        let issues = Pipeline::deterministic(proposal.proposer)
            .validate(&snapshot, &proposal)
            .expect_err("a transaction cannot validate itself");
        prop_assert!(
            issues.iter().any(|issue| issue.validator == ValidatorName::Authorization
                && issue.code == "proposer-is-validator"),
            "the authorization validator is one of the refusals: {issues:?}"
        );
    }

    /// The two walks of one rule agree, on the verdict and on the path.
    ///
    /// `ekr_ontology::Value::inadmissible_in_canonical_state` is what the type validator asks, and
    /// `ekr_graph::CanonicalValue::try_from` is what the pipeline builds the canonical form with.
    /// They are two implementations of `architecture-decision-record:0005`'s rule, and the pipeline
    /// no longer carries a guard against their disagreeing — it runs the conversion unconditionally
    /// and lets the type validator supply the message — so this is what stands where that guard
    /// stood. It is a case rather than a branch because a branch nothing can reach says nothing
    /// about whether the two agree, and this does.
    #[test]
    fn the_two_walks_of_one_rule_agree(value in proposed_value()) {
        let predicate = value
            .inadmissible_in_canonical_state()
            .map(|path| path.to_string());
        let conversion = CanonicalValue::try_from(value.clone())
            .err()
            .map(|refusal| refusal.path().to_string());
        prop_assert_eq!(
            predicate,
            conversion,
            "the ontology's predicate and the graph's conversion disagreed about {:?}",
            value
        );
    }

    /// A value canonical state does not admit never reaches a validated transaction.
    ///
    /// `architecture-decision-record:0005-float-is-not-canonical`, as a property rather than as
    /// the one case in `tests/validation.rs`: whatever else is wrong or right with a proposal,
    /// one carrying a float anywhere is refused, and the refusal is the type validator's.
    #[test]
    fn a_float_anywhere_is_always_refused(proposal in transaction(proposed_value)) {
        let graph = empty_graph();
        let snapshot = GraphSnapshot::of(&graph);
        let carries_float = proposal
            .operations
            .iter()
            .any(operation_carries_a_float);

        let outcome = Pipeline::deterministic(POOL.agents[1]).validate(&snapshot, &proposal);
        if carries_float {
            let issues = outcome.expect_err("a float is not admissible in canonical state");
            prop_assert!(
                issues.iter().any(|issue| issue.code == "inadmissible-value"),
                "the refusal names the defect: {issues:?}"
            );
        }
    }
}

/// Whether any value in the operation is, or contains, a float.
fn operation_carries_a_float(operation: &GraphOperation) -> bool {
    let carries = |value: &Value| value.inadmissible_in_canonical_state().is_some();
    match operation {
        GraphOperation::CreateNode(draft) => draft.properties.values().flatten().any(carries),
        GraphOperation::UpdateProperty(mutation) => mutation.values.iter().any(carries),
        GraphOperation::CreateEdge(draft) => draft.properties.values().flatten().any(carries),
        GraphOperation::AddAssertion(assertion) => match &assertion.object {
            Object::Value(value) => carries(value),
            Object::Node(_) | Object::Type(_) => false,
        },
        GraphOperation::Invoke { arguments, .. } => arguments.values().any(carries),
        GraphOperation::DeleteEdge(_)
        | GraphOperation::RetractAssertion(_)
        | GraphOperation::SupersedeAssertion(_)
        | GraphOperation::DefineNodeType(_)
        | GraphOperation::DefineEdgeType(_)
        | GraphOperation::ModifyProperty(_)
        | GraphOperation::MergeEntity(_) => false,
    }
}
