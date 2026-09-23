//! Revision-root sensitivity through the real kernel (§§ 34, 91.2, 91.5).
//!
//! Three layers, each holding the one below it to the producer:
//!
//! 1. **The producer computes each sub-root from its documented input.** A real seed and a real
//!    commit on both providers: `ontology_root` is the loaded ontology's address, `knowledge_root`
//!    and `evidence_root` are the store's two root functions over the admitted graph, `agent_root`
//!    is the host `AuthorityStateV1` anchor's address, `transaction` is the seed envelope or the
//!    validated canonical transaction, and `validation_hash` is `ValidationMaterialV1` over the
//!    retained basis. Without this the two layers below would test functions nobody calls.
//! 2. **Each input moves exactly the sub-roots it feeds.** One changed seed or transaction field
//!    per case, re-run through the kernel: the sub-roots it feeds change, the revision root
//!    changes, and every sub-root it does not feed stays byte-identical.
//! 3. **Every field reaches its address.** Field-by-field mutation of the host anchor, the
//!    ontology, the transaction, the revision root and the complete validation basis.
//!
//! The store owns `knowledge_root` and `evidence_root`, and their field-complete cases are
//! `crates/ekr-store/tests/current_root_sensitivity.rs`.
mod current_fixture;

use std::collections::{BTreeMap, BTreeSet};

use current_fixture::{anchor, anchor_for, context, id, ontology, seed, seeded, SEEDED_AT};
use ekr_core::{AgentId, ContentHash, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalRef, CanonicalValue, Confidence, Object,
    Predicate, Root, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, CommitReceiptV1,
    GraphOperation, GraphTransaction, NodeDraft, Runtime, SeedDocument, ValidationBasisV1,
    ValidationCommandResult, ValidationMaterialV1, ValidatorName,
};
use ekr_ontology::{
    Cardinality, EdgeType, Lifecycle, NodeType, Ontology, OntologyDocument, OperationDefinition,
    PropertyDefinition, Transition, Value, ValueType,
};
use ekr_store::{evidence_root, knowledge_root};
use serde::Serialize;

fn document(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}

/// A transaction creating one node and asserting one property of it, citing the seed evidence.
fn transaction(id_bits: u64, name: &str) -> GraphTransaction {
    let (node, label) = (id(0x60), id(0x06));
    GraphTransaction {
        id: id(id_bits),
        proposer: context().operator,
        operations: vec![
            GraphOperation::CreateNode(NodeDraft {
                id: node,
                root_id: id(0x02),
                type_id: id(0x05),
                canonical_name: name.into(),
                properties: BTreeMap::from([(label, vec![Value::String("beta".into())])]),
            }),
            GraphOperation::AddAssertion(Box::new(Assertion {
                id: id(0x61),
                root_id: id(0x02),
                subject: Subject::Node(node),
                predicate: Predicate::Property(label),
                object: Object::Value(Value::String("beta".into())),
                evidence: BTreeSet::from([id(0x13)]),
                proposed_by: context().operator,
                assessment: Assessment::Proposed,
                lifecycle: AssertionLifecycle::Active,
                valid_time: TemporalRange::UNBOUNDED,
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            })),
        ],
        evidence: BTreeSet::from([id(0x13)]),
    }
}

/// The same transaction in the value space canonical state admits, written out by hand.
fn canonical(tx: &GraphTransaction) -> GraphTransaction<CanonicalValue> {
    let string = |value: &Value| match value {
        Value::String(text) => CanonicalValue::String(text.clone()),
        other => panic!("fixture carries only strings: {other:?}"),
    };
    let operations = tx
        .operations
        .iter()
        .map(|operation| match operation {
            GraphOperation::CreateNode(draft) => GraphOperation::CreateNode(NodeDraft {
                id: draft.id,
                root_id: draft.root_id,
                type_id: draft.type_id,
                canonical_name: draft.canonical_name.clone(),
                properties: draft
                    .properties
                    .iter()
                    .map(|(key, values)| (*key, values.iter().map(string).collect()))
                    .collect(),
            }),
            GraphOperation::AddAssertion(assertion) => {
                let Subject::Node(node) = assertion.subject else {
                    panic!("fixture asserts about nodes")
                };
                let Object::Value(value) = &assertion.object else {
                    panic!("fixture asserts values")
                };
                GraphOperation::AddAssertion(Box::new(Assertion {
                    id: assertion.id,
                    root_id: assertion.root_id,
                    subject: Subject::Node(CanonicalRef::new(node)),
                    predicate: assertion.predicate,
                    object: Object::Value(string(value)),
                    evidence: assertion
                        .evidence
                        .iter()
                        .copied()
                        .map(CanonicalRef::new)
                        .collect(),
                    proposed_by: assertion.proposed_by,
                    assessment: assertion
                        .assessment
                        .clone()
                        .map_assertions(CanonicalRef::new),
                    lifecycle: assertion
                        .lifecycle
                        .clone()
                        .map_assertions(CanonicalRef::new),
                    valid_time: assertion.valid_time,
                    transaction_time: assertion.transaction_time,
                }))
            }
            other => panic!("fixture uses two operations: {other:?}"),
        })
        .collect();
    GraphTransaction {
        id: tx.id,
        proposer: tx.proposer,
        operations,
        evidence: tx.evidence.clone(),
    }
}

/// Propose, validate against the seed and commit, returning the actual commit receipt.
fn committed(runtime: &Runtime, tx: &GraphTransaction) -> CommitReceiptV1 {
    runtime
        .propose(&document(tx), context().operator, || {
            Timestamp::from_millis(20)
        })
        .unwrap();
    let validated = runtime
        .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(30))
        .unwrap();
    assert!(
        matches!(validated, ValidationCommandResult::Validated(_)),
        "{validated:?}"
    );
    match runtime
        .commit(tx.id, context().operator, || Timestamp::from_millis(40))
        .unwrap()
    {
        CommitCommandResult::Committed(receipt) => *receipt,
        CommitCommandResult::Stale(stale) => panic!("fresh commit became stale: {stale:?}"),
    }
}

/// Layer 1, and the coordinator's `agent_root` obligation: every sub-root of Root0 and Root1 is
/// its documented input's address, on both providers.
#[test]
fn the_kernel_computes_every_sub_root_from_its_documented_input_on_both_providers() {
    for file in [false, true] {
        let (_directory, runtime, seeded) = seeded(seed(), context(), anchor(), file, SEEDED_AT);
        let root0 = seeded.result;
        let graph0 = runtime.snapshot().unwrap();
        assert_eq!(root0.ontology_root, ContentHash::of(&graph0.ontology));
        assert_eq!(root0.knowledge_root, knowledge_root(&graph0));
        assert_eq!(root0.evidence_root, evidence_root(&graph0));
        assert_eq!(root0.agent_root, ContentHash::of(&anchor()));
        assert_eq!(root0.transaction, seeded.seed_hash);
        assert_eq!(runtime.head().unwrap(), Some(root0));

        let tx = transaction(0x51, "third");
        let receipt = committed(&runtime, &tx);
        let root1 = receipt.result;
        let graph1 = runtime.snapshot().unwrap();
        assert_eq!(root1.revision, RevisionNumber::new(1));
        assert_eq!(root1.parent, Some(ContentHash::of(&root0)));
        assert_eq!(root1.ontology_root, ContentHash::of(&graph1.ontology));
        assert_eq!(root1.knowledge_root, knowledge_root(&graph1));
        assert_eq!(root1.evidence_root, evidence_root(&graph1));
        assert_eq!(root1.agent_root, ContentHash::of(&anchor()));
        assert_eq!(root1.transaction, ContentHash::of(&canonical(&tx)));
        assert_eq!(root1.transaction, receipt.validation.transaction_hash);
        assert_eq!(receipt.result_hash, ContentHash::of(&root1));

        let validation = &receipt.validation;
        assert_eq!(validation.basis.previous_root, root0);
        assert_eq!(validation.basis.previous_root_hash, ContentHash::of(&root0));
        assert_eq!(validation.basis.seed_hash, seeded.seed_hash);
        assert_eq!(validation.basis.ontology_root, root0.ontology_root);
        assert_eq!(validation.basis.authority_root, root0.agent_root);
        assert_eq!(
            validation.basis.validation_profile_hash,
            ContentHash::of(&anchor().validation_profile)
        );
        assert_eq!(
            validation.validation_hash,
            ContentHash::of(&ValidationMaterialV1 {
                transaction: &canonical(&tx),
                basis: &validation.basis,
                validators: &validation.validators,
            })
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Sub {
    Ontology,
    Knowledge,
    Evidence,
    Agent,
    Transaction,
}
const SUBS: [Sub; 5] = [
    Sub::Ontology,
    Sub::Knowledge,
    Sub::Evidence,
    Sub::Agent,
    Sub::Transaction,
];
fn sub(root: &Root, which: Sub) -> ContentHash {
    match which {
        Sub::Ontology => root.ontology_root,
        Sub::Knowledge => root.knowledge_root,
        Sub::Evidence => root.evidence_root,
        Sub::Agent => root.agent_root,
        Sub::Transaction => root.transaction,
    }
}
/// Which sub-roots moved between two roots, plus whether the revision root did.
fn moved(base: &Root, changed: &Root) -> (BTreeSet<Sub>, bool) {
    (
        SUBS.into_iter()
            .filter(|which| sub(base, *which) != sub(changed, *which))
            .collect(),
        ContentHash::of(base) != ContentHash::of(changed),
    )
}

struct SeedCase {
    name: &'static str,
    feeds: &'static [Sub],
    seed: SeedDocument,
    context: BootstrapContext,
    anchor: AuthorityStateV1,
    at: Timestamp,
}
fn case(
    name: &'static str,
    feeds: &'static [Sub],
    change: impl FnOnce(&mut SeedDocument, &mut BootstrapContext, &mut AuthorityStateV1, &mut Timestamp),
) -> SeedCase {
    let (mut seed, mut context, mut anchor, mut at) = (seed(), context(), anchor(), SEEDED_AT);
    change(&mut seed, &mut context, &mut anchor, &mut at);
    SeedCase {
        name,
        feeds,
        seed,
        context,
        anchor,
        at,
    }
}

fn seed_cases() -> Vec<SeedCase> {
    use Sub::{Evidence, Knowledge, Ontology, Transaction};
    let first = id::<ekr_core::NodeId>(0x10);
    let edge = id::<ekr_core::EdgeId>(0x12);
    let assertion = id::<ekr_core::AssertionId>(0x14);
    let evidence = id::<ekr_core::EvidenceId>(0x13);
    let observer = id::<AgentId>(0x09);
    vec![
        case("unchanged", &[], |_, _, _, _| {}),
        case("node name", &[Knowledge, Transaction], |s, _, _, _| {
            s.graph.nodes.get_mut(&first).unwrap().canonical_name = "renamed".into();
        }),
        case("node property", &[Knowledge, Transaction], |s, _, _, _| {
            s.graph.nodes.get_mut(&first).unwrap().properties =
                BTreeMap::from([(id(0x06), vec![Value::String("gamma".into())])]);
        }),
        case("edge target", &[Knowledge, Transaction], |s, _, _, _| {
            s.graph.edges.get_mut(&edge).unwrap().target = first;
        }),
        case(
            "assertion valid time",
            &[Knowledge, Transaction],
            |s, _, _, _| {
                s.graph.assertions.get_mut(&assertion).unwrap().valid_time =
                    TemporalRange::since(Timestamp::from_millis(1));
            },
        ),
        case(
            "evidence observed_at",
            &[Evidence, Transaction],
            |s, _, _, _| {
                s.graph.evidence.get_mut(&evidence).unwrap().observed_at =
                    Timestamp::from_millis(5);
            },
        ),
        case(
            "evidence confidence",
            &[Evidence, Transaction],
            |s, _, _, _| {
                s.graph.evidence.get_mut(&evidence).unwrap().confidence =
                    Confidence::from_basis_points(9_000).unwrap();
            },
        ),
        case("node type name", &[Ontology, Transaction], |s, _, _, _| {
            s.ontology.node_types[0].name = "Topic".into();
        }),
        case("edge type name", &[Ontology, Transaction], |s, _, _, _| {
            s.ontology.edge_types[0].name = "links".into();
        }),
        case(
            "registered agent name",
            &[Sub::Agent, Transaction],
            |_, _, a, _| {
                a.agents.get_mut(&observer).unwrap().name = "reader".into();
            },
        ),
        case(
            "registered agent capability",
            &[Sub::Agent, Transaction],
            |_, _, a, _| {
                a.agents
                    .get_mut(&observer)
                    .unwrap()
                    .capabilities
                    .insert("audit".into());
            },
        ),
        case(
            "additional registered agent",
            &[Sub::Agent, Transaction],
            |_, _, a, _| {
                let extra = id::<AgentId>(0x0a);
                a.agents.insert(
                    extra,
                    Agent {
                        id: extra,
                        name: "auditor".into(),
                        capabilities: BTreeSet::new(),
                    },
                );
            },
        ),
        // The profile names the validator, and so does every accepted assertion's validator set:
        // a different profile is a different accepting agent, which is knowledge too.
        case(
            "validation profile validator",
            &[Sub::Agent, Knowledge, Transaction],
            |_, c, a, _| {
                c.validator = id(0x09);
                *a = anchor_for(*c);
                a.agents.insert(
                    id(0x04),
                    Agent {
                        id: id(0x04),
                        name: "validator".into(),
                        capabilities: BTreeSet::from(["validate".to_owned()]),
                    },
                );
            },
        ),
        case("seed clock", &[Knowledge, Transaction], |_, _, _, at| {
            *at = Timestamp::from_millis(11);
        }),
    ]
}

/// Layer 2 at the seed, and the coordinator's registry/profile obligation on both providers.
#[test]
fn each_seed_input_moves_exactly_the_sub_roots_it_feeds_on_both_providers() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let (_base_directory, _base_runtime, base) =
            seeded(seed(), context(), anchor(), file, SEEDED_AT);
        for case in seed_cases() {
            let (_directory, runtime, result) =
                seeded(case.seed, case.context, case.anchor.clone(), file, case.at);
            assert_eq!(result.result.agent_root, ContentHash::of(&case.anchor));
            assert_eq!(runtime.head().unwrap(), Some(result.result));
            let expected: BTreeSet<Sub> = case.feeds.iter().copied().collect();
            let observed = moved(&base.result, &result.result);
            if observed != (expected.clone(), !expected.is_empty()) {
                wrong.push(format!(
                    "file={file} {}: expected {expected:?}, observed {observed:?}",
                    case.name
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// Layer 2 at an ordinary commit: a transaction identity feeds the transaction root and nothing
/// the graph holds; a created node's name feeds both.
#[test]
fn each_transaction_input_moves_exactly_the_sub_roots_it_feeds() {
    use Sub::{Knowledge, Transaction};
    let run = |tx: GraphTransaction| {
        let (_directory, runtime, _) = seeded(seed(), context(), anchor(), true, SEEDED_AT);
        committed(&runtime, &tx).result
    };
    let base = run(transaction(0x51, "third"));
    let mut wrong = Vec::new();
    for (name, tx, feeds) in [
        ("unchanged", transaction(0x51, "third"), vec![]),
        (
            "transaction id",
            transaction(0x52, "third"),
            vec![Transaction],
        ),
        (
            "created node name",
            transaction(0x51, "fourth"),
            vec![Knowledge, Transaction],
        ),
    ] {
        let expected: BTreeSet<Sub> = feeds.into_iter().collect();
        let observed = moved(&base, &run(tx));
        if observed != (expected.clone(), !expected.is_empty()) {
            wrong.push(format!(
                "{name}: expected {expected:?}, observed {observed:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// Collects every named mutation whose address did not move.
fn unmoved<T: Clone>(
    base: &T,
    address: impl Fn(&T) -> ContentHash,
    cases: Vec<(&'static str, Change<T>)>,
) -> Vec<&'static str> {
    let original = address(base);
    cases
        .into_iter()
        .filter_map(|(name, change)| {
            let mut changed = base.clone();
            change(&mut changed);
            (address(&changed) == original).then_some(name)
        })
        .collect()
}

type Change<T> = Box<dyn Fn(&mut T)>;

#[test]
fn every_field_of_the_authority_anchor_reaches_agent_root() {
    let observer = id::<AgentId>(0x09);
    let cases: Vec<(&str, Change<AuthorityStateV1>)> = vec![
        ("format", Box::new(|a| a.format.push('x'))),
        (
            "agents: removed",
            Box::new(move |a| {
                a.agents.remove(&observer);
            }),
        ),
        (
            "agents: key",
            Box::new(move |a| {
                let held = a.agents.remove(&observer).unwrap();
                a.agents.insert(id(0x0b), held);
            }),
        ),
        (
            "agent.id",
            Box::new(move |a| a.agents.get_mut(&observer).unwrap().id = id(0x0b)),
        ),
        (
            "agent.name",
            Box::new(move |a| a.agents.get_mut(&observer).unwrap().name.push('x')),
        ),
        (
            "agent.capabilities",
            Box::new(move |a| {
                a.agents.get_mut(&observer).unwrap().capabilities.clear();
            }),
        ),
        (
            "profile.format",
            Box::new(|a| a.validation_profile.format.push('x')),
        ),
        (
            "profile.ruleset",
            Box::new(|a| a.validation_profile.ruleset.push('x')),
        ),
        (
            "profile.checks: order",
            Box::new(|a| a.validation_profile.checks.swap(0, 1)),
        ),
        (
            "profile.checks: member",
            Box::new(|a| a.validation_profile.checks[6] = ValidatorName::Structural),
        ),
        (
            "profile.validator",
            Box::new(|a| a.validation_profile.validator = id(0x0b)),
        ),
        (
            "profile.proposer_separation",
            Box::new(|a| a.validation_profile.proposer_separation.push('x')),
        ),
        (
            "profile.provenance",
            Box::new(|a| a.validation_profile.provenance.push('x')),
        ),
        (
            "profile.application",
            Box::new(|a| a.validation_profile.application.push('x')),
        ),
    ];
    assert_eq!(
        unmoved(&anchor(), ContentHash::of, cases),
        Vec::<&str>::new()
    );
}

/// An ontology using every declaration field: a parent type, a lifecycle with an operation, a
/// constrained required property and an inverse edge pair.
fn rich_ontology() -> OntologyDocument {
    let mut document = ontology();
    let (parent, child, prop, back) = (id(0x70), id(0x71), id(0x72), id(0x73));
    let mut base = NodeType::new(parent, "Base");
    base.abstract_type = true;
    let mut derived = NodeType::new(child, "Derived");
    derived.parents.insert(parent);
    let mut definition = PropertyDefinition::new(prop, "status", ValueType::String);
    definition.required = true;
    definition.constraints = vec!["non-empty".into()];
    derived.properties.insert(prop, definition);
    derived.lifecycle = Some(Lifecycle {
        initial: "open".into(),
        states: BTreeSet::from(["open".into(), "closed".into()]),
        transitions: BTreeSet::from([Transition {
            from: "open".into(),
            to: "closed".into(),
        }]),
    });
    derived.operations.insert(
        "close".into(),
        OperationDefinition {
            name: "close".into(),
            arguments: BTreeMap::from([("reason".into(), ValueType::String)]),
            preconditions: vec!["open".into()],
            transition: Some(Transition {
                from: "open".into(),
                to: "closed".into(),
            }),
            emits: vec!["Closed".into()],
        },
    );
    document.node_types.extend([base, derived]);
    let relates = document.edge_types[0].id;
    document.edge_types[0].inverse = Some(back);
    let mut inverse = EdgeType::new(back, "related_by");
    inverse.source_types.insert(id(0x05));
    inverse.target_types.insert(id(0x05));
    inverse.inverse = Some(relates);
    document.edge_types.push(inverse);
    document
}

fn ontology_root(document: &OntologyDocument) -> ContentHash {
    ContentHash::of(&Ontology::load(document.clone()).expect("each mutation still coheres"))
}

#[test]
fn every_field_of_the_ontology_reaches_ontology_root() {
    fn derived(d: &mut OntologyDocument) -> &mut NodeType {
        d.node_types
            .iter_mut()
            .find(|t| t.name == "Derived")
            .unwrap()
    }
    fn status(d: &mut OntologyDocument) -> &mut PropertyDefinition {
        derived(d).properties.values_mut().next().unwrap()
    }
    fn close(d: &mut OntologyDocument) -> &mut OperationDefinition {
        derived(d).operations.get_mut("close").unwrap()
    }
    let cases: Vec<(&str, Change<OntologyDocument>)> = vec![
        ("version.id", Box::new(|d| d.version.id = id(0x7f))),
        ("version.number", Box::new(|d| d.version.number = 1)),
        (
            "version.parent",
            Box::new(|d| d.version.parent = Some(id(0x7f))),
        ),
        (
            "version.created_at",
            Box::new(|d| d.version.created_at = Timestamp::from_millis(1)),
        ),
        (
            "unused node declaration",
            Box::new(|d| d.node_types.push(NodeType::new(id(0x7e), "Unused"))),
        ),
        (
            "unused edge declaration",
            Box::new(|d| {
                let mut unused = EdgeType::new(id(0x7d), "unused");
                unused.source_types.insert(id(0x05));
                unused.target_types.insert(id(0x05));
                d.edge_types.push(unused);
            }),
        ),
        ("node.id", Box::new(move |d| derived(d).id = id(0x7c))),
        ("node.name", Box::new(|d| d.node_types[0].name.push('x'))),
        (
            "node.parents",
            Box::new(move |d| derived(d).parents.clear()),
        ),
        (
            "node.abstract_type",
            Box::new(|d| d.node_types[0].abstract_type = true),
        ),
        (
            "node.lifecycle",
            Box::new(move |d| {
                derived(d).lifecycle = None;
                derived(d).operations.clear();
            }),
        ),
        (
            "lifecycle.initial",
            Box::new(move |d| derived(d).lifecycle.as_mut().unwrap().initial = "closed".into()),
        ),
        (
            "lifecycle.states",
            Box::new(move |d| {
                derived(d)
                    .lifecycle
                    .as_mut()
                    .unwrap()
                    .states
                    .insert("archived".into());
            }),
        ),
        (
            "lifecycle.transitions",
            Box::new(move |d| {
                derived(d)
                    .lifecycle
                    .as_mut()
                    .unwrap()
                    .transitions
                    .insert(Transition {
                        from: "closed".into(),
                        to: "open".into(),
                    });
            }),
        ),
        (
            "node.operations",
            Box::new(move |d| derived(d).operations.clear()),
        ),
        ("operation.name", Box::new(move |d| close(d).name.push('x'))),
        (
            "operation.arguments",
            Box::new(move |d| {
                close(d).arguments.clear();
            }),
        ),
        (
            "operation.preconditions",
            Box::new(move |d| close(d).preconditions.clear()),
        ),
        (
            "operation.transition",
            Box::new(move |d| close(d).transition = None),
        ),
        ("operation.emits", Box::new(move |d| close(d).emits.clear())),
        (
            "node.properties",
            Box::new(move |d| derived(d).properties.clear()),
        ),
        (
            "property.id",
            Box::new(move |d| {
                let held = derived(d).properties.values().next().unwrap().clone();
                derived(d).properties = BTreeMap::from([(
                    id(0x7b),
                    PropertyDefinition {
                        id: id(0x7b),
                        ..held
                    },
                )]);
            }),
        ),
        ("property.name", Box::new(move |d| status(d).name.push('x'))),
        (
            "property.value_type",
            Box::new(move |d| status(d).value_type = ValueType::Integer),
        ),
        (
            "property.cardinality",
            Box::new(move |d| status(d).cardinality = Cardinality::Many),
        ),
        (
            "property.required",
            Box::new(move |d| status(d).required = false),
        ),
        (
            "property.constraints",
            Box::new(move |d| status(d).constraints.clear()),
        ),
        (
            "edge.id",
            Box::new(|d| {
                d.edge_types[1].inverse = None;
                d.edge_types[0].inverse = None;
                d.edge_types[0].id = id(0x7a);
            }),
        ),
        ("edge.name", Box::new(|d| d.edge_types[0].name.push('x'))),
        (
            "edge.source_types",
            Box::new(|d| {
                d.edge_types[0].source_types.insert(id(0x71));
            }),
        ),
        (
            "edge.target_types",
            Box::new(|d| {
                d.edge_types[0].target_types.insert(id(0x71));
            }),
        ),
        (
            "edge.cardinality",
            Box::new(|d| d.edge_types[0].cardinality = Cardinality::Many),
        ),
        (
            "edge.properties",
            Box::new(|d| {
                d.edge_types[0].properties.insert(
                    id(0x79),
                    PropertyDefinition::new(id(0x79), "weight", ValueType::Integer),
                );
            }),
        ),
        ("edge.inverse", Box::new(|d| d.edge_types[0].inverse = None)),
        (
            "edge.symmetric",
            Box::new(|d| d.edge_types[0].symmetric = true),
        ),
        (
            "edge.transitive",
            Box::new(|d| d.edge_types[0].transitive = true),
        ),
    ];
    assert_eq!(
        unmoved(&rich_ontology(), ontology_root, cases),
        Vec::<&str>::new()
    );
}

#[test]
fn every_field_of_a_transaction_reaches_the_transaction_root() {
    let cases: Vec<(&str, Change<GraphTransaction<CanonicalValue>>)> = vec![
        ("id", Box::new(|t| t.id = id::<TransactionId>(0x5f))),
        ("proposer", Box::new(|t| t.proposer = id(0x09))),
        ("operations: order", Box::new(|t| t.operations.reverse())),
        (
            "operations: member",
            Box::new(|t| {
                t.operations.pop();
            }),
        ),
        ("evidence", Box::new(|t| t.evidence.clear())),
    ];
    assert_eq!(
        unmoved(
            &canonical(&transaction(0x51, "third")),
            ContentHash::of,
            cases
        ),
        Vec::<&str>::new()
    );
}

fn base_root() -> Root {
    Root {
        revision: RevisionNumber::new(1),
        parent: Some(ContentHash::from_bytes([0xf0; 32])),
        ontology_root: ContentHash::from_bytes([0xf1; 32]),
        knowledge_root: ContentHash::from_bytes([0xf2; 32]),
        evidence_root: ContentHash::from_bytes([0xf3; 32]),
        agent_root: ContentHash::from_bytes([0xf4; 32]),
        transaction: ContentHash::from_bytes([0xf5; 32]),
    }
}

fn root_changes() -> Vec<(&'static str, Change<Root>)> {
    let other = ContentHash::from_bytes([0x0f; 32]);
    vec![
        (
            "revision",
            Box::new(|r| r.revision = RevisionNumber::new(2)),
        ),
        ("parent", Box::new(|r| r.parent = None)),
        ("ontology_root", Box::new(move |r| r.ontology_root = other)),
        (
            "knowledge_root",
            Box::new(move |r| r.knowledge_root = other),
        ),
        ("evidence_root", Box::new(move |r| r.evidence_root = other)),
        ("agent_root", Box::new(move |r| r.agent_root = other)),
        ("transaction", Box::new(move |r| r.transaction = other)),
    ]
}

#[test]
fn every_field_of_the_revision_root_reaches_its_address() {
    assert_eq!(
        unmoved(&base_root(), ContentHash::of, root_changes()),
        Vec::<&str>::new()
    );
}

fn base_basis() -> ValidationBasisV1 {
    ValidationBasisV1 {
        format: ValidationBasisV1::FORMAT.into(),
        graph_root_id: id(0x02),
        previous_revision_id: id(0x52),
        previous_event_id: id(0x53),
        previous_record_hash: ContentHash::from_bytes([0xe3; 32]),
        previous_root: base_root(),
        previous_root_hash: ContentHash::of(&base_root()),
        seed_hash: ContentHash::from_bytes([0xe4; 32]),
        ontology_root: ContentHash::from_bytes([0xf1; 32]),
        authority_root: ContentHash::from_bytes([0xf4; 32]),
        validation_profile_hash: ContentHash::from_bytes([0xe5; 32]),
    }
}

/// The complete basis: every one of its fields, and every field of the root it retains, reaches
/// the validation hash, as do the transaction and the validator set beside it.
#[test]
fn every_field_of_the_validation_basis_reaches_the_validation_hash() {
    #[derive(Clone)]
    struct Material {
        transaction: GraphTransaction<CanonicalValue>,
        basis: ValidationBasisV1,
        validators: BTreeSet<AgentId>,
    }
    let address = |m: &Material| {
        ContentHash::of(&ValidationMaterialV1 {
            transaction: &m.transaction,
            basis: &m.basis,
            validators: &m.validators,
        })
    };
    let other = ContentHash::from_bytes([0x0f; 32]);
    let mut cases: Vec<(&str, Change<Material>)> = vec![
        ("basis.format", Box::new(|m| m.basis.format.push('x'))),
        (
            "basis.graph_root_id",
            Box::new(|m| m.basis.graph_root_id = id(0x5e)),
        ),
        (
            "basis.previous_revision_id",
            Box::new(|m| m.basis.previous_revision_id = id(0x5e)),
        ),
        (
            "basis.previous_event_id",
            Box::new(|m| m.basis.previous_event_id = id(0x5e)),
        ),
        (
            "basis.previous_record_hash",
            Box::new(move |m| m.basis.previous_record_hash = other),
        ),
        (
            "basis.previous_root_hash",
            Box::new(move |m| m.basis.previous_root_hash = other),
        ),
        (
            "basis.seed_hash",
            Box::new(move |m| m.basis.seed_hash = other),
        ),
        (
            "basis.ontology_root",
            Box::new(move |m| m.basis.ontology_root = other),
        ),
        (
            "basis.authority_root",
            Box::new(move |m| m.basis.authority_root = other),
        ),
        (
            "basis.validation_profile_hash",
            Box::new(move |m| m.basis.validation_profile_hash = other),
        ),
        (
            "transaction",
            Box::new(|m| m.transaction.id = id::<TransactionId>(0x5f)),
        ),
        (
            "validators",
            Box::new(|m| {
                m.validators.insert(id(0x09));
            }),
        ),
    ];
    for (name, change) in root_changes() {
        cases.push((
            match name {
                "revision" => "basis.previous_root.revision",
                "parent" => "basis.previous_root.parent",
                "ontology_root" => "basis.previous_root.ontology_root",
                "knowledge_root" => "basis.previous_root.knowledge_root",
                "evidence_root" => "basis.previous_root.evidence_root",
                "agent_root" => "basis.previous_root.agent_root",
                _ => "basis.previous_root.transaction",
            },
            Box::new(move |m| change(&mut m.basis.previous_root)),
        ));
    }
    let base = Material {
        transaction: canonical(&transaction(0x51, "third")),
        basis: base_basis(),
        validators: BTreeSet::from([id(0x04)]),
    };
    assert_eq!(cases.len(), 19);
    assert_eq!(unmoved(&base, address, cases), Vec::<&str>::new());
}
