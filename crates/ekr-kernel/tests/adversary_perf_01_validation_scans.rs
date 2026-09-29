//! Adversary, wave perf-01, unit W: `story:validation-without-whole-graph-scans`.
//!
//! The unit replaced whole-graph scans in validation with indexes and claims every refusal, its
//! message and its order are unchanged. The unit's own differential property compares three codes
//! against a re-implementation of the old scans, over one edge type (`One`), three aliases and no
//! deletion of an edge nothing holds. This file holds the whole pipeline to what the base commit
//! (`c81a426a`) said, over a wider generator:
//!
//! * edge types with (`One`) and without (`Many`) a cardinality limit, and one the ontology does
//!   not declare; edges created, deleted, deleted before they are created, deleted and re-created
//!   under a canonical id, and sourced at nodes the transaction creates;
//! * aliases that collide exactly, differ only in case, differ only in Unicode normalisation
//!   (NFC against NFD, a ligature against its expansion) or are empty; holders in another root and
//!   of another type; aliases taken twice in one transaction;
//! * assertions about edges the transaction deletes, citing evidence an `AddEvidence` of the same
//!   transaction brings, retracted, superseded and withdrawn twice;
//! * named operations (`Invoke`) and merges beside all of it, and basis graphs at different
//!   revisions, as `--against N` validates against an older one.
//!
//! [`the_pipeline_says_what_base_said_over_seeded_transactions`] renders every issue of three
//! profiles over `CASES` seeded transactions and pins the digest of that text to the one the base
//! commit produced. Set `EKR_ADVERSARY_DUMP` to a path to write the text itself, and diff two
//! trees' dumps to find the case that moved.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionNumber, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue,
    Confidence, Edge, Evidence, EvidenceSource, GraphRoot, GraphSnapshot, Node, Object, Predicate,
    RetractionReason, Space, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::validate::HeldIdentities;
use ekr_kernel::{
    EdgeDraft, EntityMerge, EvidenceAddition, GraphOperation, GraphTransaction, NodeDraft,
    Pipeline, PropertyMutation, Retraction, Supersession, ValidationIssue,
};
use ekr_ontology::{
    Cardinality, EdgeType, Lifecycle, NodeType, Ontology, OntologyDocument, OperationDefinition,
    PropertyDefinition, SchemaVersion, Transition, Value, ValueType,
};

/// How many seeded transactions the differential renders.
const CASES: u64 = 3000;

/// The digest of the rendering at base `c81a426a`, before the unit: every issue, in order, of
/// three profiles over `CASES` seeded transactions.
const BASE_DIGEST: &str = "80607f117007d8d13277adc696569114b3e755cd58aea6024d3a61d896c1d506";

const K_TYPE: u16 = 1;
const K_PROP: u16 = 2;
const K_NODE: u16 = 3;
const K_EDGE: u16 = 4;
const K_ASSERT: u16 = 5;
const K_EVID: u16 = 7;
const K_ROOT: u16 = 8;
const K_TX: u16 = 9;
const K_AGENT: u16 = 10;
const K_SCHEMA: u16 = 11;

const NODES: u64 = 6;
const EDGES: u64 = 6;
const ASSERTIONS: u64 = 5;
const EVIDENCE: u64 = 4;

/// Exact repeats, case variants, NFC `café` and NFD `café`, the `ﬁ` ligature and its expansion,
/// a padded variant and the empty alias.
const ALIASES: [&str; 9] = [
    "eventlog",
    "Eventlog",
    "caf\u{e9}",
    "cafe\u{301}",
    "\u{fb01}le",
    "file",
    " eventlog",
    "",
    "eventlog",
];

fn uuid(kind: u16, n: u64) -> String {
    format!("00000000-{kind:04x}-4000-8000-{n:012x}")
}
fn type_id(n: u64) -> TypeId {
    uuid(K_TYPE, n).parse().unwrap()
}
fn property(n: u64) -> PropertyId {
    uuid(K_PROP, n).parse().unwrap()
}
fn node(n: u64) -> NodeId {
    uuid(K_NODE, n).parse().unwrap()
}
fn edge(n: u64) -> EdgeId {
    uuid(K_EDGE, n).parse().unwrap()
}
fn assertion(n: u64) -> AssertionId {
    uuid(K_ASSERT, n).parse().unwrap()
}
fn evidence(n: u64) -> EvidenceId {
    uuid(K_EVID, n).parse().unwrap()
}
fn root(n: u64) -> GraphRootId {
    uuid(K_ROOT, n).parse().unwrap()
}
fn agent(n: u64) -> AgentId {
    uuid(K_AGENT, n).parse().unwrap()
}
fn schema() -> SchemaVersionId {
    uuid(K_SCHEMA, 1).parse().unwrap()
}

const DECISION: u64 = 1;
const OTHER: u64 = 2;
const ONE: u64 = 3;
const MANY: u64 = 4;
const UNDECLARED: u64 = 5;
const TITLE: u64 = 1;
const TAGS: u64 = 2;

/// SplitMix64: the same draws on every machine and every Rust release.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
    fn aliases(&mut self, most: u64) -> Vec<String> {
        (0..self.below(most + 1))
            .map(|_| ALIASES[usize::try_from(self.below(ALIASES.len() as u64)).unwrap()].to_owned())
            .collect()
    }
}

fn ontology() -> Ontology {
    let mut decision = NodeType::new(type_id(DECISION), "Decision");
    let mut title = PropertyDefinition::new(property(TITLE), "title", ValueType::String);
    title.required = true;
    decision.properties.insert(property(TITLE), title);
    let mut tags = PropertyDefinition::new(property(TAGS), "tags", ValueType::String);
    tags.cardinality = Cardinality::Many;
    decision.properties.insert(property(TAGS), tags);
    decision.lifecycle = Some(Lifecycle {
        initial: "open".to_owned(),
        states: ["open", "decided"].into_iter().map(str::to_owned).collect(),
        transitions: [Transition::new("open", "decided")].into_iter().collect(),
    });
    let mut decide = OperationDefinition::new("decide");
    decide.transition = Some(Transition::new("open", "decided"));
    decision.operations.insert("decide".to_owned(), decide);
    let other = NodeType::new(type_id(OTHER), "Other");
    let edge_type = |id: u64, name: &str, cardinality: Cardinality| {
        let mut declared = EdgeType::new(type_id(id), name);
        declared.source_types = [type_id(DECISION), type_id(OTHER)].into_iter().collect();
        declared.target_types = [type_id(DECISION), type_id(OTHER)].into_iter().collect();
        declared.cardinality = cardinality;
        declared
    };
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(schema(), Timestamp::EPOCH),
        node_types: vec![decision, other],
        edge_types: vec![
            edge_type(ONE, "depends_on", Cardinality::One),
            edge_type(MANY, "mentions", Cardinality::Many),
        ],
    })
    .expect("two node types and two edge types cohere")
}

fn stamp(rng: &mut Rng) -> Timestamp {
    Timestamp::from_millis(i64::try_from(rng.below(4) * 1000).unwrap())
}

fn range(rng: &mut Rng) -> TemporalRange {
    TemporalRange {
        from: rng.chance(50).then(|| stamp(rng)),
        to: rng.chance(20).then(|| Timestamp::from_millis(9000)),
    }
}

fn entry(id: u64, payload: &[u8]) -> Evidence {
    Evidence {
        id: evidence(id),
        source: EvidenceSource::HumanStatement {
            identity: Some(format!("statement {id}")),
        },
        content_hash: ContentHash::of_bytes(payload),
        extracted_by: agent(0),
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::CERTAIN,
    }
}

fn accepted(rng: &mut Rng) -> Assessment {
    if rng.chance(70) {
        Assessment::Accepted {
            validators: [agent(1)].into_iter().collect(),
        }
    } else {
        Assessment::Proposed
    }
}

/// Canonical state drawn from `rng`: every node, edge, evidence and assertion id of the pools
/// held or not, so the transaction's operations meet both.
fn basis(rng: &mut Rng) -> CanonicalGraph {
    let mut graph = CanonicalGraph {
        root: GraphRoot {
            id: root(1),
            space: Space::Canonical,
            schema_version_id: schema(),
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::new(1 + rng.below(9)),
        ontology: ontology(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };
    for n in 0..NODES {
        if !rng.chance(60) {
            continue;
        }
        let decision = rng.chance(70);
        let mut held = Node::new(
            node(n),
            root(if rng.chance(85) { 1 } else { 2 }),
            type_id(if decision { DECISION } else { OTHER }),
            "held",
        );
        held.aliases = rng.aliases(3);
        if decision {
            held.type_state = Some(if rng.chance(50) { "open" } else { "decided" }.to_owned());
            held.properties.insert(
                property(TITLE),
                vec![CanonicalValue::String(format!("title {n}"))],
            );
        }
        graph.nodes.insert(node(n), held);
    }
    let held_nodes: Vec<NodeId> = graph.nodes.keys().copied().collect();
    if !held_nodes.is_empty() {
        let pick = |rng: &mut Rng| {
            held_nodes[usize::try_from(rng.below(held_nodes.len() as u64)).unwrap()]
        };
        for e in 0..EDGES {
            if !rng.chance(50) {
                continue;
            }
            let kind = if rng.chance(50) { ONE } else { MANY };
            let (source, target) = (pick(rng), pick(rng));
            graph.edges.insert(
                edge(e),
                Edge::new(
                    edge(e),
                    root(1),
                    type_id(kind),
                    CanonicalRef::new(source),
                    CanonicalRef::new(target),
                ),
            );
        }
    }
    for v in 0..EVIDENCE {
        if rng.chance(50) {
            graph.evidence.insert(evidence(v), entry(v, b"held"));
        }
    }
    let held_edges: Vec<EdgeId> = graph.edges.keys().copied().collect();
    let held_evidence: Vec<EvidenceId> = graph.evidence.keys().copied().collect();
    for a in 0..ASSERTIONS {
        if held_nodes.is_empty() || !rng.chance(50) {
            continue;
        }
        let subject = if !held_edges.is_empty() && rng.chance(40) {
            Subject::Edge(CanonicalRef::new(
                held_edges[usize::try_from(rng.below(held_edges.len() as u64)).unwrap()],
            ))
        } else {
            Subject::Node(CanonicalRef::new(
                held_nodes[usize::try_from(rng.below(held_nodes.len() as u64)).unwrap()],
            ))
        };
        let lifecycle = match rng.below(4) {
            0 => AssertionLifecycle::Retracted {
                at_revision: RevisionNumber::new(1),
                reason: RetractionReason::new("held withdrawal"),
            },
            1 => AssertionLifecycle::Superseded {
                by: CanonicalRef::new(assertion(rng.below(ASSERTIONS))),
                at_revision: RevisionNumber::new(1),
                effective_from: stamp(rng),
            },
            _ => AssertionLifecycle::Active,
        };
        graph.assertions.insert(
            assertion(a),
            Assertion {
                id: assertion(a),
                root_id: root(1),
                subject,
                predicate: Predicate::Property(property(TITLE)),
                object: Object::Value(CanonicalValue::String(format!("claim {a}"))),
                evidence: held_evidence
                    .iter()
                    .take(1)
                    .map(|id| CanonicalRef::new(*id))
                    .collect(),
                proposed_by: agent(0),
                lifecycle,
                assessment: accepted(rng),
                valid_time: range(rng),
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            },
        );
    }
    graph
}

fn values(rng: &mut Rng) -> Vec<Value> {
    (0..rng.below(3))
        .map(|n| Value::String(format!("v{n}")))
        .collect()
}

/// One operation drawn from `rng`, weighted toward what the unit's indexes decide: nodes with
/// aliases, edges created and deleted.
fn operation(rng: &mut Rng) -> GraphOperation {
    match rng.below(20) {
        0..=4 => {
            let kind = match rng.below(10) {
                0..=5 => DECISION,
                6..=8 => OTHER,
                _ => UNDECLARED,
            };
            let mut properties = BTreeMap::new();
            if kind == DECISION {
                if rng.chance(85) {
                    properties.insert(property(TITLE), values(rng));
                }
                if rng.chance(40) {
                    properties.insert(property(TAGS), values(rng));
                }
            }
            GraphOperation::CreateNode(NodeDraft {
                id: node(rng.below(NODES)),
                root_id: root(if rng.chance(92) { 1 } else { 2 }),
                type_id: type_id(kind),
                canonical_name: "created".to_owned(),
                properties,
                aliases: rng.aliases(3),
            })
        }
        5..=10 => {
            let kind = match rng.below(10) {
                0..=4 => ONE,
                5..=8 => MANY,
                _ => UNDECLARED,
            };
            GraphOperation::CreateEdge(EdgeDraft {
                id: edge(rng.below(EDGES)),
                root_id: root(1),
                type_id: type_id(kind),
                source: node(rng.below(NODES)),
                target: node(rng.below(NODES)),
                properties: BTreeMap::new(),
            })
        }
        11..=13 => GraphOperation::DeleteEdge(edge(rng.below(EDGES))),
        14 => GraphOperation::UpdateProperty(PropertyMutation {
            node: node(rng.below(NODES)),
            property: property(if rng.chance(50) { TITLE } else { TAGS }),
            values: values(rng),
        }),
        15 => {
            let subject = if rng.chance(50) {
                Subject::Edge(edge(rng.below(EDGES)))
            } else {
                Subject::Node(node(rng.below(NODES)))
            };
            GraphOperation::AddAssertion(Box::new(Assertion {
                id: assertion(rng.below(ASSERTIONS)),
                root_id: root(1),
                subject,
                predicate: Predicate::Property(property(TITLE)),
                object: if rng.chance(30) {
                    Object::Node(node(rng.below(NODES)))
                } else {
                    Object::Value(Value::String("added".to_owned()))
                },
                evidence: (0..1 + rng.below(2))
                    .map(|_| evidence(rng.below(EVIDENCE)))
                    .collect(),
                proposed_by: agent(0),
                lifecycle: AssertionLifecycle::Active,
                assessment: Assessment::Proposed,
                valid_time: range(rng),
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            }))
        }
        16 => {
            let id = rng.below(EVIDENCE);
            let payload = format!("payload {id}").into_bytes();
            let mut added = entry(id, &payload);
            if rng.chance(15) {
                added.content_hash = ContentHash::of_bytes(b"something else");
            }
            GraphOperation::AddEvidence(Box::new(EvidenceAddition {
                evidence: added,
                payload,
            }))
        }
        17 => GraphOperation::RetractAssertion(Retraction {
            assertion: assertion(rng.below(ASSERTIONS)),
            reason: RetractionReason::new("withdrawn"),
        }),
        18 => GraphOperation::SupersedeAssertion(Supersession {
            assertion: assertion(rng.below(ASSERTIONS)),
            by: assertion(rng.below(ASSERTIONS)),
            effective_from: stamp(rng),
        }),
        _ => {
            if rng.chance(50) {
                GraphOperation::MergeEntity(EntityMerge {
                    absorbed: node(rng.below(NODES)),
                    into: node(rng.below(NODES)),
                })
            } else {
                GraphOperation::Invoke {
                    node: node(rng.below(NODES)),
                    operation: "decide".to_owned(),
                    arguments: BTreeMap::new(),
                }
            }
        }
    }
}

/// The transaction of case `case`: its declared evidence is what its assertions cite, as a
/// proposer would build it, except in one case in eight.
fn proposal(case: u64, rng: &mut Rng) -> GraphTransaction {
    let operations: Vec<GraphOperation> = (0..1 + rng.below(8)).map(|_| operation(rng)).collect();
    let cited: BTreeSet<EvidenceId> = operations
        .iter()
        .filter_map(|operation| match operation {
            GraphOperation::AddAssertion(assertion) => Some(assertion.evidence.iter().copied()),
            _ => None,
        })
        .flatten()
        .collect();
    GraphTransaction {
        id: uuid(K_TX, case).parse().unwrap(),
        proposer: agent(0),
        operations,
        evidence: if rng.chance(88) {
            cited
        } else {
            [evidence(0)].into_iter().collect()
        },
        schema_version: None,
    }
}

fn pipelines() -> Vec<(&'static str, Pipeline)> {
    let lineage: BTreeSet<SchemaVersionId> = [schema()].into_iter().collect();
    vec![
        ("v1", Pipeline::deterministic(agent(1))),
        ("v2", Pipeline::schema_evolving(agent(1), lineage.clone())),
        (
            "v3",
            Pipeline::identity_keeping(agent(1), lineage, HeldIdentities::default()),
        ),
    ]
}

/// Every issue of every profile over every seeded case, one line each, in the order raised; and
/// how many cases raised each code, so the generator's reach is itself checked.
fn render() -> (String, BTreeMap<String, usize>) {
    let pipelines = pipelines();
    let mut text = String::new();
    let mut reach: BTreeMap<String, usize> = BTreeMap::new();
    for case in 0..CASES {
        let mut rng = Rng(0x00ad_5e5a_0000_0000 ^ case);
        let graph = basis(&mut rng);
        let tx = proposal(case, &mut rng);
        let snapshot = GraphSnapshot::of(&graph);
        for (profile, pipeline) in &pipelines {
            match pipeline.validate(&snapshot, &tx) {
                Ok(validated) => {
                    writeln!(text, "{case} {profile} ok {}", validated.validation_hash()).unwrap();
                    *reach.entry("ok".to_owned()).or_default() += 1;
                }
                Err(issues) => {
                    for ValidationIssue {
                        transaction_id,
                        validator,
                        code,
                        message,
                    } in issues
                    {
                        writeln!(
                            text,
                            "{case} {profile} {transaction_id} {validator:?} {code} {message}"
                        )
                        .unwrap();
                        *reach.entry(code).or_default() += 1;
                    }
                }
            }
        }
    }
    (text, reach)
}

/// The whole pipeline, three profiles, over `CASES` seeded basis graphs and transactions, says
/// byte for byte what it said before the unit: every code, message and position.
#[test]
fn the_pipeline_says_what_base_said_over_seeded_transactions() {
    let (text, reach) = render();
    if let Some(path) = std::env::var_os("EKR_ADVERSARY_DUMP") {
        std::fs::write(path, &text).expect("the dump path is writable");
    }
    // The generator reaches every refusal the unit's indexes decide, and admits some cases.
    for code in [
        "ok",
        "alias-already-exists",
        "duplicate-alias",
        "edge-cardinality",
        "unresolved-edge",
        "unresolved-node",
        "unresolved-evidence",
        "unresolved-assertion",
        "identity-already-exists",
        "missing-required-property",
        "property-cardinality",
        "assertion-lifecycle-state",
        "conflicting-assertion-lifecycle",
        "invalid-supersession",
    ] {
        assert!(
            reach.get(code).copied().unwrap_or(0) >= 5,
            "the generator reaches {code} in at least five cases: {reach:?}"
        );
    }
    let digest = ContentHash::of_bytes(text.as_bytes()).to_hex();
    assert_eq!(
        digest, BASE_DIGEST,
        "the rendering moved from base c81a426a; set EKR_ADVERSARY_DUMP and diff against base"
    );
}

/// A small world for the named scenarios: two decisions, `open` holding one `depends_on` (`One`)
/// edge to `decided` and aliased `eventlog` and `café` (NFC), an assertion about that edge, and
/// one retained piece of evidence.
struct World {
    graph: CanonicalGraph,
}

const OPEN: u64 = 0;
const DECIDED: u64 = 1;
const HELD_EDGE: u64 = 0;
const HELD_EVIDENCE: u64 = 0;
const EDGE_CLAIM: u64 = 0;

impl World {
    fn new() -> Self {
        let mut graph = CanonicalGraph {
            root: GraphRoot {
                id: root(1),
                space: Space::Canonical,
                schema_version_id: schema(),
                parent: None,
                created_at: Timestamp::EPOCH,
            },
            revision: RevisionNumber::new(4),
            ontology: ontology(),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            assertions: BTreeMap::new(),
            evidence: BTreeMap::new(),
        };
        for (n, aliases) in [
            (OPEN, vec!["eventlog".to_owned(), "caf\u{e9}".to_owned()]),
            (DECIDED, vec!["\u{fb01}le".to_owned()]),
        ] {
            let mut held = Node::new(node(n), root(1), type_id(DECISION), "held");
            held.aliases = aliases;
            held.type_state = Some("open".to_owned());
            held.properties.insert(
                property(TITLE),
                vec![CanonicalValue::String(format!("title {n}"))],
            );
            graph.nodes.insert(node(n), held);
        }
        graph.edges.insert(
            edge(HELD_EDGE),
            Edge::new(
                edge(HELD_EDGE),
                root(1),
                type_id(ONE),
                CanonicalRef::new(node(OPEN)),
                CanonicalRef::new(node(DECIDED)),
            ),
        );
        graph
            .evidence
            .insert(evidence(HELD_EVIDENCE), entry(HELD_EVIDENCE, b"held"));
        graph.assertions.insert(
            assertion(EDGE_CLAIM),
            Assertion {
                id: assertion(EDGE_CLAIM),
                root_id: root(1),
                subject: Subject::Edge(CanonicalRef::new(edge(HELD_EDGE))),
                predicate: Predicate::Property(property(TITLE)),
                object: Object::Value(CanonicalValue::String("about the edge".to_owned())),
                evidence: [CanonicalRef::new(evidence(HELD_EVIDENCE))]
                    .into_iter()
                    .collect(),
                proposed_by: agent(0),
                lifecycle: AssertionLifecycle::Active,
                assessment: Assessment::Accepted {
                    validators: [agent(1)].into_iter().collect(),
                },
                valid_time: TemporalRange::UNBOUNDED,
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            },
        );
        Self { graph }
    }

    /// The codes of every issue the v1 pipeline raises, in order; empty when it validates.
    fn codes(&self, operations: Vec<GraphOperation>) -> Vec<String> {
        let cited = operations
            .iter()
            .filter_map(|operation| match operation {
                GraphOperation::AddAssertion(assertion) => Some(assertion.evidence.iter().copied()),
                _ => None,
            })
            .flatten()
            .collect();
        let tx = GraphTransaction {
            id: TransactionId::mint(),
            proposer: agent(0),
            operations,
            evidence: cited,
            schema_version: None,
        };
        match Pipeline::deterministic(agent(1)).validate(&GraphSnapshot::of(&self.graph), &tx) {
            Ok(_) => Vec::new(),
            Err(issues) => issues.into_iter().map(|issue| issue.code).collect(),
        }
    }
}

fn depends(id: u64, source: u64, target: u64) -> GraphOperation {
    GraphOperation::CreateEdge(EdgeDraft {
        id: edge(id),
        root_id: root(1),
        type_id: type_id(ONE),
        source: node(source),
        target: node(target),
        properties: BTreeMap::new(),
    })
}

fn decision(id: u64, aliases: &[&str]) -> GraphOperation {
    GraphOperation::CreateNode(NodeDraft {
        id: node(id),
        root_id: root(1),
        type_id: type_id(DECISION),
        canonical_name: "created".to_owned(),
        properties: [(property(TITLE), vec![Value::String("t".to_owned())])]
            .into_iter()
            .collect(),
        aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
    })
}

/// A `One` limit reached only by an edge the same transaction deletes is not reached, whichever
/// order the create and the delete are written in; one the transaction does not delete is.
#[test]
fn a_one_limit_counts_the_graph_after_the_transaction_in_either_order() {
    let world = World::new();
    for operations in [
        vec![
            depends(1, OPEN, DECIDED),
            GraphOperation::DeleteEdge(edge(1)),
        ],
        vec![
            GraphOperation::DeleteEdge(edge(1)),
            depends(1, OPEN, DECIDED),
        ],
        vec![
            GraphOperation::DeleteEdge(edge(HELD_EDGE)),
            depends(1, OPEN, DECIDED),
            GraphOperation::AddAssertion(Box::new(about_edge(HELD_EDGE + 1, 1))),
        ],
    ] {
        let codes = world.codes(operations.clone());
        assert!(
            !codes.contains(&"edge-cardinality".to_owned()),
            "{operations:?} raised {codes:?}"
        );
    }
    assert_eq!(
        world.codes(vec![depends(1, OPEN, DECIDED)]),
        vec!["edge-cardinality"]
    );
    // A node created in the transaction is a source like any other.
    assert_eq!(
        world.codes(vec![
            decision(4, &[]),
            depends(1, 4, OPEN),
            depends(2, 4, DECIDED),
        ]),
        vec!["edge-cardinality", "edge-cardinality"]
    );
}

/// Deleting a canonical edge and creating it again under its own id is a create over an id
/// canonical state holds, and the edge is gone from the candidate either way.
#[test]
fn deleting_an_edge_and_recreating_it_is_refused_as_base_refused_it() {
    let world = World::new();
    assert_eq!(
        world.codes(vec![
            GraphOperation::DeleteEdge(edge(HELD_EDGE)),
            depends(HELD_EDGE, OPEN, DECIDED),
        ]),
        vec!["identity-already-exists", "unresolved-edge"]
    );
}

/// An alias collides byte for byte and in no other way: case, NFC against NFD and a ligature
/// against its expansion are different aliases, as they were before the index.
#[test]
fn aliases_collide_byte_for_byte_and_not_by_case_or_normalisation() {
    let world = World::new();
    assert_eq!(
        world.codes(vec![decision(
            4,
            &["Eventlog", "cafe\u{301}", "file", "EVENTLOG"]
        )]),
        Vec::<String>::new()
    );
    assert_eq!(
        world.codes(vec![decision(4, &["caf\u{e9}", "\u{fb01}le"])]),
        vec!["alias-already-exists", "alias-already-exists"]
    );
}

fn about_edge(id: u64, subject: u64) -> Assertion<Value> {
    Assertion {
        id: assertion(id),
        root_id: root(1),
        subject: Subject::Edge(edge(subject)),
        predicate: Predicate::Property(property(TITLE)),
        object: Object::Value(Value::String("about an edge".to_owned())),
        evidence: [evidence(HELD_EVIDENCE)].into_iter().collect(),
        proposed_by: agent(0),
        lifecycle: AssertionLifecycle::Active,
        assessment: Assessment::Proposed,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}

/// An edge the transaction deletes is not there to talk about: an assertion the transaction adds
/// about it, and the one canonical state already holds, each resolve against nothing.
#[test]
fn an_edge_deleted_in_the_transaction_resolves_for_no_assertion() {
    let world = World::new();
    assert_eq!(
        world.codes(vec![
            GraphOperation::DeleteEdge(edge(HELD_EDGE)),
            GraphOperation::AddAssertion(Box::new(about_edge(1, HELD_EDGE))),
        ]),
        vec!["unresolved-edge", "unresolved-edge"]
    );
}

/// Evidence an `AddEvidence` brings resolves for an assertion of the same transaction.
#[test]
fn evidence_added_and_cited_in_one_transaction_resolves() {
    let world = World::new();
    let payload = b"brought".to_vec();
    let mut cited = about_edge(1, HELD_EDGE);
    cited.subject = Subject::Node(node(OPEN));
    cited.evidence = [evidence(2)].into_iter().collect();
    assert_eq!(
        world.codes(vec![
            GraphOperation::AddEvidence(Box::new(EvidenceAddition {
                evidence: entry(2, &payload),
                payload,
            })),
            GraphOperation::AddAssertion(Box::new(cited)),
        ]),
        Vec::<String>::new()
    );
}

/// The early return keys on the lifecycle operations themselves; a named operation beside a
/// retraction does not hide it, and one alone raises nothing.
#[test]
fn a_retraction_beside_a_named_operation_is_still_checked() {
    let world = World::new();
    let invoke = GraphOperation::Invoke {
        node: node(OPEN),
        operation: "decide".to_owned(),
        arguments: BTreeMap::new(),
    };
    let retract = || {
        GraphOperation::RetractAssertion(Retraction {
            assertion: assertion(EDGE_CLAIM),
            reason: RetractionReason::new("withdrawn"),
        })
    };
    assert_eq!(world.codes(vec![invoke.clone()]), Vec::<String>::new());
    assert_eq!(
        world.codes(vec![invoke, retract(), retract()]),
        vec!["conflicting-assertion-lifecycle"]
    );
}
