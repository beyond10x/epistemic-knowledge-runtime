//! The P1 exit properties, over generated transactions and generated lineages, on both providers.
//!
//! `story:p1-exit-properties`. `tests/validation.rs` holds one hand-written row per dangling
//! reference, through the pipeline alone; `tests/seed.rs` and `crates/ekr/tests/retraction_example.rs`
//! hold one fixed lineage each. This file states the same two things for **every** transaction and
//! lineage a generator reaches, and through the whole durable command path — [`Runtime`] over the
//! File and the SQLite provider, with the kernel's own authority, rather than a pipeline called
//! from a test:
//!
//! * `no_transaction_naming_an_absent_identity_commits_on_either_provider` — a proposal whose
//!   operations name a node, an edge, an assertion, a piece of evidence or a graph root nothing
//!   holds, in every one of the 21 places an operation can name one, is
//!   rejected with the reference validator's named code, cannot be committed, and publishes no
//!   revision: the head, the revision count and the physical event count are read back;
//! * `replay_from_the_seed_reproduces_every_root_of_a_generated_lineage_on_both_providers` — a
//!   generated lineage of committed transactions, replayed from the seed in the same process and
//!   in a freshly opened runtime, reproduces the root recorded at every revision, and the two
//!   providers agree on every one of them.
//!
//! The absent identity takes every form, enumerated rather than drawn: freshly minted, and **with
//! the bits of an entity of each other kind the graph does hold**. The second is the shape
//! `task:canonical-reference-holds-a-node-id-for-every-target` was about — an identity that
//! resolves if anything reads it as the wrong kind.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId, RevisionNumber,
    Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Edge, Evidence, EvidenceSource, Node,
    Object, Predicate, RetractionReason, Root, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, CommitError, EdgeDraft,
    EntityMerge, GraphOperation, GraphTransaction, NodeDraft, PropertyMutation, Retraction,
    Runtime, SeedDocument, Supersession, TransactionState, ValidationCommandResult,
    ValidationProfileV1, ValidatorName,
};
use ekr_ontology::{Cardinality, EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use proptest::prelude::*;
use serde::Serialize;

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}

fn anchor() -> AuthorityStateV1 {
    let c = context();
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [(c.operator, "operator"), (c.validator, "validator")]
            .into_iter()
            .map(|(id, name)| {
                (
                    id,
                    Agent {
                        id,
                        name: name.into(),
                        capabilities: BTreeSet::new(),
                    },
                )
            })
            .collect(),
        validation_profile: ValidationProfileV1::deterministic(c.validator),
    }
}

fn open(path: &std::path::Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, "test", context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), anchor())
    }
    .unwrap()
}

/// The seeded world every case starts from: one node type with a many-valued label, one edge type
/// between nodes of it, two nodes joined by one edge, one assertion and the evidence it rests on.
struct World {
    seed: SeedDocument,
    node_type: TypeId,
    edge_type: TypeId,
    label: PropertyId,
    link: PropertyId,
    nodes: [NodeId; 2],
    edge: EdgeId,
    assertion: AssertionId,
    evidence: EvidenceId,
}

fn world() -> World {
    let mut seed = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let root = seed.graph.root.id;
    let (node_type, edge_type, label) = (TypeId::mint(), TypeId::mint(), PropertyId::mint());
    let mut declared = NodeType::new(node_type, "Subject");
    let mut labels = PropertyDefinition::new(label, "labels", ValueType::String);
    labels.cardinality = Cardinality::Many;
    declared.properties.insert(label, labels);
    // A property whose values are node references, so a `NodeRef` inside a value is well typed and
    // the only thing wrong with it is what it points at.
    let link = PropertyId::mint();
    let mut links = PropertyDefinition::new(
        link,
        "links",
        ValueType::NodeRef {
            allowed_types: BTreeSet::from([node_type]),
        },
    );
    links.cardinality = Cardinality::Many;
    declared.properties.insert(link, links);
    seed.ontology.node_types.push(declared);
    let mut relation = EdgeType::new(edge_type, "relates_to");
    relation.source_types.insert(node_type);
    relation.target_types.insert(node_type);
    relation.cardinality = Cardinality::Many;
    seed.ontology.edge_types.push(relation);

    let nodes = [NodeId::mint(), NodeId::mint()];
    for (at, id) in nodes.iter().enumerate() {
        seed.graph.nodes.insert(
            *id,
            Node::<Value>::new(*id, root, node_type, format!("seed-{at}")),
        );
    }
    let edge = EdgeId::mint();
    seed.graph.edges.insert(
        edge,
        Edge::<Value>::new(edge, root, edge_type, nodes[0], nodes[1]),
    );
    let bytes = b"synthetic human evidence".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let evidence = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::CERTAIN,
    };
    let assertion = claim(root, label, nodes[0], "seeded", evidence.id);
    let assertion_id = assertion.id;
    seed.graph.assertions.insert(assertion.id, assertion);
    seed.graph.evidence.insert(evidence.id, evidence.clone());
    seed.evidence_payloads.insert(hash, bytes);
    World {
        seed,
        node_type,
        edge_type,
        label,
        link,
        nodes,
        edge,
        assertion: assertion_id,
        evidence: evidence.id,
    }
}

/// A proposed label claim about `node`, resting on `evidence`.
fn claim(
    root: ekr_core::GraphRootId,
    label: PropertyId,
    node: NodeId,
    text: &str,
    evidence: EvidenceId,
) -> Assertion<Value> {
    Assertion {
        id: AssertionId::mint(),
        root_id: root,
        subject: Subject::Node(node),
        predicate: Predicate::Property(label),
        object: Object::Value(Value::String(text.into())),
        evidence: BTreeSet::from([evidence]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}

fn encode(tx: &GraphTransaction) -> Vec<u8> {
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

/// Every event the provider physically holds for the tenant, counted from its own feed.
fn physical_events(path: &std::path::Path, file: bool) -> usize {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    executor.block_on(async {
        let provider: Box<dyn eventlog_core::EventStore> = if file {
            Box::new(eventlog_file::FileEventStore::open(path).await.unwrap())
        } else {
            Box::new(
                eventlog_sqlite::SqliteEventStore::open(
                    &path.join("state.db").to_string_lossy(),
                    "ekr",
                )
                .await
                .unwrap(),
            )
        };
        let tenant = eventlog_core::TenantId::new("test").unwrap();
        let (mut count, mut after) = (0, 0);
        loop {
            let page = provider.read_feed(&tenant, after, 100).await.unwrap();
            count += page.events.len();
            if !page.has_more {
                break;
            }
            after = page.next_position;
        }
        count
    })
}

/// The kind of identity a dangling proposal names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Node,
    Edge,
    Assertion,
    Evidence,
    GraphRoot,
}

impl Kind {
    /// The reference validator's name for this kind of refusal.
    fn code(self) -> &'static str {
        match self {
            Self::Node => "unresolved-node",
            Self::Edge => "unresolved-edge",
            Self::Assertion => "unresolved-assertion",
            Self::Evidence => "unresolved-evidence",
            Self::GraphRoot => "unresolved-graph-root",
        }
    }
}

/// Every place an operation can name a graph identity, which is every shape the property is asked
/// of. **Enumerated rather than drawn**: `ALL` is iterated, so every shape runs in every run and
/// only what surrounds it is random.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    UpdatedNode,
    EdgeSource,
    EdgeTarget,
    SubjectNode,
    ObjectNode,
    MergeAbsorbed,
    MergeInto,
    InvokedNode,
    NodeRefInCreatedProperty,
    NodeRefInUpdatedProperty,
    NodeRefInInvokeArgument,
    NodeRefInAssertedValue,
    DeletedEdge,
    SubjectEdge,
    RetractedAssertion,
    SupersededAssertion,
    SupersedingAssertion,
    CitedEvidence,
    CreatedNodeRoot,
    CreatedEdgeRoot,
    AssertionRoot,
}

impl Shape {
    const ALL: [Self; 21] = [
        Self::UpdatedNode,
        Self::EdgeSource,
        Self::EdgeTarget,
        Self::SubjectNode,
        Self::ObjectNode,
        Self::MergeAbsorbed,
        Self::MergeInto,
        Self::InvokedNode,
        Self::NodeRefInCreatedProperty,
        Self::NodeRefInUpdatedProperty,
        Self::NodeRefInInvokeArgument,
        Self::NodeRefInAssertedValue,
        Self::DeletedEdge,
        Self::SubjectEdge,
        Self::RetractedAssertion,
        Self::SupersededAssertion,
        Self::SupersedingAssertion,
        Self::CitedEvidence,
        Self::CreatedNodeRoot,
        Self::CreatedEdgeRoot,
        Self::AssertionRoot,
    ];

    /// The kind of identity this shape names.
    fn kind(self) -> Kind {
        match self {
            Self::UpdatedNode
            | Self::EdgeSource
            | Self::EdgeTarget
            | Self::SubjectNode
            | Self::ObjectNode
            | Self::MergeAbsorbed
            | Self::MergeInto
            | Self::InvokedNode
            | Self::NodeRefInCreatedProperty
            | Self::NodeRefInUpdatedProperty
            | Self::NodeRefInInvokeArgument
            | Self::NodeRefInAssertedValue => Kind::Node,
            Self::DeletedEdge | Self::SubjectEdge => Kind::Edge,
            Self::RetractedAssertion | Self::SupersededAssertion | Self::SupersedingAssertion => {
                Kind::Assertion
            }
            Self::CitedEvidence => Kind::Evidence,
            Self::CreatedNodeRoot | Self::CreatedEdgeRoot | Self::AssertionRoot => Kind::GraphRoot,
        }
    }
}

/// One operation that names an identity nothing holds, in `shape`, and whether it cites the
/// world's evidence. `bits` carries the absent identity's 128 bits.
fn dangling(world: &World, shape: Shape, bits: NodeId) -> (GraphOperation, bool) {
    let root = world.seed.graph.root.id;
    let uuid = bits.to_uuid();
    let node = NodeId::from_uuid(uuid);
    let reference = Value::NodeRef(node);
    let edge = |source, target, root_id| {
        GraphOperation::CreateEdge(EdgeDraft {
            id: EdgeId::mint(),
            root_id,
            type_id: world.edge_type,
            source,
            target,
            properties: BTreeMap::new(),
        })
    };
    let asserted = |held: Assertion<Value>| (GraphOperation::AddAssertion(Box::new(held)), true);
    let about = |subject| claim(root, world.label, subject, "x", world.evidence);
    let supersession = |assertion, by| {
        GraphOperation::SupersedeAssertion(Supersession {
            assertion,
            by,
            effective_from: Timestamp::EPOCH,
        })
    };
    let created = |root_id, properties| {
        GraphOperation::CreateNode(NodeDraft {
            id: NodeId::mint(),
            root_id,
            type_id: world.node_type,
            canonical_name: "created".into(),
            properties,
        })
    };
    let operation = match shape {
        Shape::UpdatedNode => GraphOperation::UpdateProperty(PropertyMutation {
            node,
            property: world.label,
            values: vec![Value::String("changed".into())],
        }),
        Shape::EdgeSource => edge(node, world.nodes[1], root),
        Shape::EdgeTarget => edge(world.nodes[0], node, root),
        Shape::SubjectNode => return asserted(about(node)),
        Shape::ObjectNode => {
            let mut held = about(world.nodes[0]);
            held.predicate = Predicate::Relation(world.edge_type);
            held.object = Object::Node(node);
            return asserted(held);
        }
        Shape::MergeAbsorbed => GraphOperation::MergeEntity(EntityMerge {
            absorbed: node,
            into: world.nodes[0],
        }),
        Shape::MergeInto => GraphOperation::MergeEntity(EntityMerge {
            absorbed: world.nodes[0],
            into: node,
        }),
        Shape::InvokedNode => GraphOperation::Invoke {
            node,
            operation: "decide".into(),
            arguments: BTreeMap::new(),
        },
        Shape::NodeRefInCreatedProperty => {
            created(root, BTreeMap::from([(world.link, vec![reference])]))
        }
        Shape::NodeRefInUpdatedProperty => GraphOperation::UpdateProperty(PropertyMutation {
            node: world.nodes[1],
            property: world.link,
            values: vec![reference],
        }),
        Shape::NodeRefInInvokeArgument => GraphOperation::Invoke {
            node: world.nodes[0],
            operation: "decide".into(),
            arguments: BTreeMap::from([("target".into(), reference)]),
        },
        Shape::NodeRefInAssertedValue => {
            let mut held = about(world.nodes[0]);
            held.predicate = Predicate::Property(world.link);
            held.object = Object::Value(reference);
            return asserted(held);
        }
        Shape::DeletedEdge => GraphOperation::DeleteEdge(EdgeId::from_uuid(uuid)),
        Shape::SubjectEdge => {
            let mut held = about(world.nodes[0]);
            held.subject = Subject::Edge(EdgeId::from_uuid(uuid));
            return asserted(held);
        }
        Shape::RetractedAssertion => GraphOperation::RetractAssertion(Retraction {
            assertion: AssertionId::from_uuid(uuid),
            reason: RetractionReason::new("withdrawn"),
        }),
        Shape::SupersededAssertion => supersession(AssertionId::from_uuid(uuid), world.assertion),
        Shape::SupersedingAssertion => supersession(world.assertion, AssertionId::from_uuid(uuid)),
        Shape::CitedEvidence => {
            let absent = EvidenceId::from_uuid(uuid);
            return (
                GraphOperation::AddAssertion(Box::new(claim(
                    root,
                    world.label,
                    world.nodes[0],
                    "x",
                    absent,
                ))),
                true,
            );
        }
        Shape::CreatedNodeRoot => created(GraphRootId::from_uuid(uuid), BTreeMap::new()),
        Shape::CreatedEdgeRoot => {
            edge(world.nodes[0], world.nodes[1], GraphRootId::from_uuid(uuid))
        }
        Shape::AssertionRoot => {
            let mut held = about(world.nodes[0]);
            held.root_id = GraphRootId::from_uuid(uuid);
            return asserted(held);
        }
    };
    (operation, false)
}

impl Kind {
    const ALL: [Self; 5] = [
        Self::Node,
        Self::Edge,
        Self::Assertion,
        Self::Evidence,
        Self::GraphRoot,
    ];
}

/// Where the absent identity's bits come from. **Enumerated, not drawn**: every shape runs with a
/// fresh id and with the bits of an entity of each *other* kind the world holds, which is the form
/// `task:canonical-reference-holds-a-node-id-for-every-target` was about — an id that resolves if
/// anything reads it as the wrong kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Absence {
    /// A freshly minted id nothing holds.
    Fresh,
    /// The bits of an entity of this other kind, which the world holds.
    BorrowedFrom(Kind),
}

impl Absence {
    /// Every form for an identity of `kind`: one fresh, and one borrowed from each other kind.
    fn every_form_for(kind: Kind) -> Vec<Self> {
        std::iter::once(Self::Fresh)
            .chain(
                Kind::ALL
                    .into_iter()
                    .filter(|other| *other != kind)
                    .map(Self::BorrowedFrom),
            )
            .collect()
    }

    /// The absent identity's bits. `at` picks which of the world's two nodes a node borrow uses.
    fn bits(self, world: &World, at: usize) -> NodeId {
        match self {
            Self::Fresh => NodeId::mint(),
            Self::BorrowedFrom(Kind::Node) => world.nodes[at % 2],
            Self::BorrowedFrom(Kind::Edge) => NodeId::from_uuid(world.edge.to_uuid()),
            Self::BorrowedFrom(Kind::Assertion) => NodeId::from_uuid(world.assertion.to_uuid()),
            Self::BorrowedFrom(Kind::Evidence) => NodeId::from_uuid(world.evidence.to_uuid()),
            Self::BorrowedFrom(Kind::GraphRoot) => {
                NodeId::from_uuid(world.seed.graph.root.id.to_uuid())
            }
        }
    }
}

/// A well-formed operation that names only what the world holds, to surround the dangling one
/// with, so the refusal is not a property of a proposal that is otherwise empty.
fn benign(world: &World, at: usize) -> GraphOperation {
    let root = world.seed.graph.root.id;
    match at % 3 {
        0 => GraphOperation::CreateNode(NodeDraft {
            id: NodeId::mint(),
            root_id: root,
            type_id: world.node_type,
            canonical_name: "benign".into(),
            properties: BTreeMap::new(),
        }),
        1 => GraphOperation::UpdateProperty(PropertyMutation {
            node: world.nodes[1],
            property: world.label,
            values: vec![Value::String("benign".into())],
        }),
        _ => GraphOperation::CreateEdge(EdgeDraft {
            id: EdgeId::mint(),
            root_id: root,
            type_id: world.edge_type,
            source: world.nodes[1],
            target: world.nodes[0],
            properties: BTreeMap::new(),
        }),
    }
}

/// What surrounds the dangling operation: the only part of a case that is drawn.
#[derive(Clone, Debug)]
struct Surroundings {
    at: usize,
    before: Vec<usize>,
    after: Vec<usize>,
}

fn surroundings() -> impl Strategy<Value = Surroundings> {
    (
        0usize..2,
        proptest::collection::vec(0usize..3, 0..3),
        proptest::collection::vec(0usize..3, 0..2),
    )
        .prop_map(|(at, before, after)| Surroundings { at, before, after })
}

/// Generated cases per stratum. A stratum is one shape with one absent-id form: 21 shapes, five
/// forms each, two providers per case.
const CASES_PER_STRATUM: u32 = 2;

/// The seed of stratum `index`'s generator. Fixed, so a red run is a red run again: its message
/// names the stratum, and the stratum names the seed.
const SEED: u64 = 0x0001_0014_e417;

/// Every stratum the property runs, in a fixed order.
fn strata() -> Vec<(Shape, Absence)> {
    Shape::ALL
        .into_iter()
        .flat_map(|shape| {
            Absence::every_form_for(shape.kind())
                .into_iter()
                .map(move |absence| (shape, absence))
        })
        .collect()
}

/// One generated case of the property, for `shape` with `absence`, on both providers.
fn refused_and_unpublished(
    shape: Shape,
    absence: Absence,
    around: &Surroundings,
) -> Result<(), TestCaseError> {
    let world = world();
    let kind = shape.kind();
    let bits = absence.bits(&world, around.at);
    let (operation, cites) = dangling(&world, shape, bits);
    let mut operations: Vec<GraphOperation> =
        around.before.iter().map(|at| benign(&world, *at)).collect();
    operations.push(operation);
    operations.extend(around.after.iter().map(|at| benign(&world, *at)));
    let evidence = match (shape, cites) {
        (Shape::CitedEvidence, _) => BTreeSet::from([EvidenceId::from_uuid(bits.to_uuid())]),
        (_, true) => BTreeSet::from([world.evidence]),
        (_, false) => BTreeSet::new(),
    };
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence,
    };

    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let kernel = open(directory.path(), file);
        kernel
            .seed(world.seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let head = kernel.head().unwrap();
        let seeded = kernel.snapshot().unwrap();
        let revisions = kernel.read(None).unwrap().revisions.len();

        kernel
            .propose(&encode(&tx), context().operator, || {
                Timestamp::from_millis(20)
            })
            .unwrap();
        let verdict = kernel
            .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(30))
            .unwrap();
        let ValidationCommandResult::Rejected(rejection) = verdict else {
            return Err(TestCaseError::fail(format!(
                "file={file}: a proposal naming an absent {kind:?} as {shape:?} validated: {tx:?}"
            )));
        };
        prop_assert!(
            rejection
                .issues
                .iter()
                .any(|issue| issue.validator == ValidatorName::Reference
                    && issue.code == kind.code()),
            "file={file}: the refusal names the absent {:?} of {:?} as {}: {:?}",
            kind,
            shape,
            kind.code(),
            rejection.issues
        );

        let events = physical_events(directory.path(), file);
        let refused = kernel.commit(tx.id, context().operator, || Timestamp::from_millis(40));
        prop_assert!(
            matches!(
                refused,
                Err(CommitError::TransactionStateConflict {
                    state: TransactionState::Rejected,
                    ..
                })
            ),
            "file={file}: a rejected transaction is refused at commit: {refused:?}"
        );
        prop_assert_eq!(
            physical_events(directory.path(), file),
            events,
            "file={} a refused commit appended an event",
            file
        );
        prop_assert_eq!(kernel.head().unwrap(), head, "file={} the head moved", file);
        prop_assert_eq!(
            kernel.read(None).unwrap().revisions.len(),
            revisions,
            "file={} a revision was published",
            file
        );
        prop_assert_eq!(
            kernel.snapshot().unwrap(),
            seeded,
            "file={} canonical state changed",
            file
        );
    }
    Ok(())
}

/// P1 exit: no transaction whose operations reference an absent node, edge, assertion, piece of
/// evidence or graph root commits. The refusal is named, and nothing is published.
///
/// **Stratified**: every entry of [`Shape::ALL`] — each place an operation can name a graph
/// identity, including a `NodeRef` inside a value, a merge's target and a record's graph root —
/// runs with every [`Absence`]: a fresh id, and the bits of an entity of each other kind the world
/// holds. Each of those 105 strata runs [`CASES_PER_STRATUM`] generated cases from a fixed seed
/// ([`SEED`] plus the stratum's index), and only what surrounds the dangling operation is drawn.
/// Drawing the shape, and then the form, left most combinations out of most runs.
///
/// What this asserts, per case and provider: validation answers `Rejected` with an issue from
/// `ValidatorName::Reference` carrying the kind's own code; `commit` refuses with
/// `TransactionStateConflict { state: Rejected }` and appends **zero** events; the head root, the
/// revision count and the canonical snapshot equal the seed's. And, over the run, that every
/// stratum executed exactly [`CASES_PER_STRATUM`] cases.
///
/// Entity merge has no P1 application path, so a merge is refused whatever it names; the case
/// still asserts that the reference validator is among the refusals.
#[test]
fn no_transaction_naming_an_absent_identity_commits_on_either_provider() {
    let strata = strata();
    assert_eq!(
        strata.len(),
        Shape::ALL.len() * Kind::ALL.len(),
        "21 shapes, 5 forms each"
    );
    // One thread per stratum: each case opens its own directories on both providers, so strata
    // share nothing, and running them one after another made this the slowest case in the crate.
    let outcomes: Vec<(String, u32, Result<(), String>)> = std::thread::scope(|scope| {
        let running: Vec<_> = strata
            .iter()
            .copied()
            .zip(0u64..)
            .map(|((shape, absence), index)| {
                scope.spawn(move || {
                    let seed = SEED + index;
                    let mut runner = proptest::test_runner::TestRunner::new(ProptestConfig {
                        cases: CASES_PER_STRATUM,
                        failure_persistence: None,
                        rng_seed: proptest::test_runner::RngSeed::Fixed(seed),
                        ..ProptestConfig::default()
                    });
                    let count = std::cell::Cell::new(0u32);
                    let outcome = runner.run(&surroundings(), |around| {
                        count.set(count.get() + 1);
                        refused_and_unpublished(shape, absence, &around)
                    });
                    (
                        format!("{shape:?}/{absence:?} (seed {seed:#x})"),
                        count.get(),
                        outcome.map_err(|failure| failure.to_string()),
                    )
                })
            })
            .collect();
        running
            .into_iter()
            .map(|thread| thread.join().expect("a stratum's thread completes"))
            .collect()
    });
    let failures: Vec<String> = outcomes
        .iter()
        .filter_map(|(stratum, _, outcome)| {
            outcome
                .as_ref()
                .err()
                .map(|why| format!("{stratum}: {why}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let executed: BTreeMap<&str, u32> = outcomes
        .iter()
        .map(|(stratum, count, _)| (stratum.as_str(), *count))
        .collect();
    assert_eq!(executed.len(), strata.len(), "every stratum ran");
    assert!(
        executed.values().all(|count| *count == CASES_PER_STRATUM),
        "every stratum ran {CASES_PER_STRATUM} cases: {executed:?}"
    );
}

/// One step of a generated lineage, with indices resolved against what exists when it runs.
#[derive(Clone, Debug)]
enum Step {
    NewNode { labels: Vec<usize> },
    NewEdge { from: usize, to: usize },
    Claim { about: usize, label: usize },
    Relabel { node: usize, labels: Vec<usize> },
    Unlink { edge: usize },
    Retract { assertion: usize },
}

const LABELS: [&str; 3] = ["red", "green", "blue"];

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        proptest::collection::vec(0usize..3, 0..3).prop_map(|labels| Step::NewNode { labels }),
        (0usize..8, 0usize..8).prop_map(|(from, to)| Step::NewEdge { from, to }),
        (0usize..8, 0usize..3).prop_map(|(about, label)| Step::Claim { about, label }),
        (0usize..8, proptest::collection::vec(0usize..3, 1..3))
            .prop_map(|(node, labels)| Step::Relabel { node, labels }),
        (0usize..8).prop_map(|edge| Step::Unlink { edge }),
        (0usize..8).prop_map(|assertion| Step::Retract { assertion }),
    ]
}

/// What a lineage has made so far, which the next step's indices are resolved against.
struct Lineage {
    nodes: Vec<NodeId>,
    edges: Vec<EdgeId>,
    active: Vec<AssertionId>,
}

impl Lineage {
    /// One transaction from `steps`, naming only what exists, and updating what exists.
    fn transaction(&mut self, world: &World, steps: &[Step]) -> GraphTransaction {
        let root = world.seed.graph.root.id;
        let labels = |picked: &[usize]| {
            picked
                .iter()
                .map(|at| Value::String(LABELS[*at].into()))
                .collect::<Vec<_>>()
        };
        let (mut operations, mut cites) = (Vec::new(), false);
        let (mut unlinked, mut retracted) = (BTreeSet::new(), BTreeSet::new());
        // One write per node and property per transaction: the structural validator refuses two
        // as `conflicting-write`, because a transaction is a set and has no order to pick by.
        let mut written = BTreeSet::new();
        for step in steps {
            match step {
                Step::NewNode { labels: picked } => {
                    let id = NodeId::mint();
                    let mut properties = BTreeMap::new();
                    if !picked.is_empty() {
                        properties.insert(world.label, labels(picked));
                    }
                    operations.push(GraphOperation::CreateNode(NodeDraft {
                        id,
                        root_id: root,
                        type_id: world.node_type,
                        canonical_name: format!("node-{}", self.nodes.len()),
                        properties,
                    }));
                    written.insert(id);
                    self.nodes.push(id);
                }
                Step::NewEdge { from, to } => {
                    let id = EdgeId::mint();
                    operations.push(GraphOperation::CreateEdge(EdgeDraft {
                        id,
                        root_id: root,
                        type_id: world.edge_type,
                        source: self.nodes[from % self.nodes.len()],
                        target: self.nodes[to % self.nodes.len()],
                        properties: BTreeMap::new(),
                    }));
                    self.edges.push(id);
                }
                Step::Claim { about, label } => {
                    let held = claim(
                        root,
                        world.label,
                        self.nodes[about % self.nodes.len()],
                        LABELS[*label],
                        world.evidence,
                    );
                    self.active.push(held.id);
                    operations.push(GraphOperation::AddAssertion(Box::new(held)));
                    cites = true;
                }
                Step::Relabel {
                    node,
                    labels: picked,
                } => {
                    let node = self.nodes[node % self.nodes.len()];
                    if !written.insert(node) {
                        continue;
                    }
                    operations.push(GraphOperation::UpdateProperty(PropertyMutation {
                        node,
                        property: world.label,
                        values: labels(picked),
                    }));
                }
                Step::Unlink { edge } => {
                    if self.edges.is_empty() {
                        continue;
                    }
                    let id = self.edges[edge % self.edges.len()];
                    if unlinked.insert(id) {
                        operations.push(GraphOperation::DeleteEdge(id));
                    }
                }
                Step::Retract { assertion } => {
                    if self.active.is_empty() {
                        continue;
                    }
                    let id = self.active[assertion % self.active.len()];
                    if retracted.insert(id) {
                        operations.push(GraphOperation::RetractAssertion(Retraction {
                            assertion: id,
                            reason: RetractionReason::new("withdrawn"),
                        }));
                    }
                }
            }
        }
        self.edges.retain(|edge| !unlinked.contains(edge));
        self.active
            .retain(|assertion| !retracted.contains(assertion));
        if operations.is_empty() {
            operations.push(GraphOperation::CreateNode(NodeDraft {
                id: NodeId::mint(),
                root_id: root,
                type_id: world.node_type,
                canonical_name: "filler".into(),
                properties: BTreeMap::new(),
            }));
        }
        GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            operations,
            evidence: if cites {
                BTreeSet::from([world.evidence])
            } else {
                BTreeSet::new()
            },
        }
    }
}

proptest! {
    // A fixed seed, for the reason `SEED` is one: a red lineage is the same lineage when rerun.
    #![proptest_config(ProptestConfig {
        cases: 16,
        failure_persistence: None,
        rng_seed: proptest::test_runner::RngSeed::Fixed(SEED),
        ..ProptestConfig::default()
    })]

    /// P1 exit: replay from the seed reproduces the head root, for generated lineages.
    ///
    /// What this asserts: every generated transaction validates and commits; the root each commit
    /// returned is the root `read(Some(revision))` recomputes for that revision, in the process
    /// that committed it **and** in a runtime freshly opened over the same directory, which
    /// replays the whole log from the seed through the kernel authority; the fresh runtime's head
    /// is the last committed root; and the File and SQLite providers, given one lineage, record
    /// the same root at every revision.
    #[test]
    fn replay_from_the_seed_reproduces_every_root_of_a_generated_lineage_on_both_providers(
        lineage in proptest::collection::vec(proptest::collection::vec(step(), 1..4), 1..5),
    ) {
        let world = world();
        let mut made = Lineage {
            nodes: world.nodes.to_vec(),
            edges: vec![world.edge],
            active: vec![world.assertion],
        };
        let transactions: Vec<GraphTransaction> =
            lineage.iter().map(|steps| made.transaction(&world, steps)).collect();

        let mut recorded: Vec<Vec<Root>> = Vec::new();
        for file in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let kernel = open(directory.path(), file);
            let seeded = kernel.seed(world.seed.clone(), || Timestamp::from_millis(10)).unwrap();
            let mut roots = vec![kernel.head().unwrap().expect("a seeded lineage has a head")];
            prop_assert_eq!(roots[0].knowledge_root, seeded.result.knowledge_root);
            for (step, tx) in transactions.iter().enumerate() {
                let at = 100 * (step as i64 + 1);
                kernel
                    .propose(&encode(tx), context().operator, || Timestamp::from_millis(at))
                    .unwrap();
                let verdict = kernel
                    .validate(tx.id, RevisionNumber::new(step as u64), || {
                        Timestamp::from_millis(at + 1)
                    })
                    .unwrap();
                prop_assert!(
                    matches!(verdict, ValidationCommandResult::Validated(_)),
                    "file={}: generated step {} was refused: {:?} for {:?}",
                    file, step, verdict, tx
                );
                let CommitCommandResult::Committed(receipt) = kernel
                    .commit(tx.id, context().operator, || Timestamp::from_millis(at + 2))
                    .unwrap()
                else {
                    return Err(TestCaseError::fail("a lone committer went stale"));
                };
                prop_assert_eq!(receipt.result.revision, RevisionNumber::new(step as u64 + 1));
                roots.push(receipt.result);
            }
            prop_assert_eq!(kernel.head().unwrap(), roots.last().cloned());
            for (revision, root) in roots.iter().enumerate() {
                let revision = RevisionNumber::new(revision as u64);
                prop_assert_eq!(&kernel.read(Some(revision)).unwrap().root, root,
                    "file={} the committing process recomputes revision {}", file, revision);
                prop_assert_eq!(
                    kernel.replay(revision).unwrap(),
                    kernel.read(Some(revision)).unwrap().graph,
                    "file={} replay and a verified read agree at revision {}", file, revision
                );
            }
            drop(kernel);

            let reopened = open(directory.path(), file);
            prop_assert_eq!(reopened.head().unwrap(), roots.last().cloned(),
                "file={} a fresh runtime replays to the committed head", file);
            for (revision, root) in roots.iter().enumerate() {
                let revision = RevisionNumber::new(revision as u64);
                prop_assert_eq!(&reopened.read(Some(revision)).unwrap().root, root,
                    "file={} a fresh runtime recomputes revision {}", file, revision);
            }
            recorded.push(roots);
        }
        prop_assert_eq!(&recorded[0], &recorded[1], "the providers disagree on the lineage's roots");
    }
}
