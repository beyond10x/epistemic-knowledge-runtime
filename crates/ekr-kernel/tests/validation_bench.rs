//! Validation cost against graph size. Not part of the default test run.
//!
//! `story:validation-without-whole-graph-scans`: every created edge counted the edges of its
//! source by scanning every edge of the graph, and every created node looked for its aliases by
//! scanning every node, so a batch that creates many edges cost its size times the graph's. This
//! harness builds canonical state in memory in an ingestion shape — 57 batches of actors, spaces,
//! items, topics and changes, each with its edges and relation and property assertions — at graph
//! scales 1, 3 and 10, and times the full profile-v3 pipeline over one fixed edge-heavy batch (the
//! nodes, edges and relation assertions of a scale-10 batch) against each.
//!
//! It is compiled only with the `bench` feature (`[[test]] required-features`), so neither
//! `cargo test` nor the gate runs it. Run it with the release profile:
//!
//! ```text
//! cargo test --release --locked -p ekr-kernel --features bench --test validation_bench -- --nocapture
//! ```
//!
//! It prints every round and asserts, on the fastest of `ROUNDS` per scale, the story's two
//! bounds: at scale 10 the batch validates in at most one second, and from scale 1 to scale 3
//! the cost grows at most linearly — at most three times the scale-1 cost. Timings are
//! wall-clock and only ever grow with the machine's load.
use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionNumber, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue,
    Confidence, Edge, Evidence, EvidenceSource, GraphRoot, GraphSnapshot, Node, Object, Predicate,
    Space, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::validate::HeldIdentities;
use ekr_kernel::{
    Authorization, Cardinality as CardinalityValidator, EdgeDraft, GraphOperation,
    GraphTransaction, NodeDraft, OntologyConstraint, Pipeline, Provenance, Reference, Structural,
    Types, Validator,
};
use ekr_ontology::{
    Cardinality, EdgeType, NodeType, Ontology, OntologyDocument, PropertyDefinition, SchemaVersion,
    ValueType,
};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

const ROUNDS: usize = 3;
const BATCHES: u64 = 57;
const SCALES: [u64; 3] = [1, 3, 10];
const BOUND_AT_TEN: Duration = Duration::from_secs(1);

const K_TYPE: u16 = 1;
const K_PROP: u16 = 2;
const K_NODE: u16 = 3;
const K_EDGE: u16 = 4;
const K_ASSERT: u16 = 5;
const K_EVID: u16 = 7;

const T_ACTOR: u64 = 1;
const T_SPACE: u64 = 2;
const T_ITEM: u64 = 3;
const T_TOPIC: u64 = 4;
const T_CHANGE: u64 = 5;
const E_AUTHORED: u64 = 11;
const E_IN: u64 = 12;
const E_REPLIES: u64 = 13;
const E_MENTIONS: u64 = 14;
const E_TAGGED: u64 = 15;
const E_REFERS: u64 = 16;
const E_OWNS: u64 = 17;
const P_HANDLE: u64 = 1;
const P_TITLE: u64 = 2;
const P_BODY: u64 = 3;
const P_URL: u64 = 4;
const P_LABEL: u64 = 5;
const P_STATE: u64 = 6;
const P_NOTE: u64 = 7;

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
fn operator() -> AgentId {
    "00000000-0008-4000-8000-000000000001".parse().unwrap()
}
fn validator() -> AgentId {
    "00000000-0008-4000-8000-000000000002".parse().unwrap()
}
fn root() -> GraphRootId {
    "00000000-0009-4000-8000-000000000001".parse().unwrap()
}
fn schema() -> SchemaVersionId {
    "00000000-000a-4000-8000-000000000001".parse().unwrap()
}

fn seed_node(t: u64, n: u64) -> u64 {
    0xf000_0000 + t * 0x1000 + n
}
fn batch_node(b: u64, t: u64, k: u64) -> u64 {
    0x1_0000_0000 + b * 0x10000 + t * 0x1000 + k
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn words(&mut self, n: usize) -> String {
        const W: &[&str] = &[
            "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india",
            "juliet", "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo",
        ];
        (0..n)
            .map(|_| W[usize::try_from(self.next() % W.len() as u64).unwrap()])
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn ontology() -> Ontology {
    let node_type = |t: u64, name: &str, properties: &[(u64, &str)]| {
        let mut declared = NodeType::new(type_id(t), name);
        for (p, name) in properties {
            let mut definition = PropertyDefinition::new(property(*p), *name, ValueType::String);
            definition.cardinality = Cardinality::Many;
            declared.properties.insert(property(*p), definition);
        }
        declared
    };
    let edge_type = |t: u64, name: &str, source: u64, target: u64| {
        let mut declared = EdgeType::new(type_id(t), name);
        declared.source_types = [type_id(source)].into_iter().collect();
        declared.target_types = [type_id(target)].into_iter().collect();
        declared.cardinality = Cardinality::Many;
        declared
    };
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(schema(), Timestamp::EPOCH),
        node_types: vec![
            node_type(T_ACTOR, "Actor", &[(P_HANDLE, "handle"), (P_NOTE, "note")]),
            node_type(T_SPACE, "Space", &[(P_TITLE, "title")]),
            node_type(T_ITEM, "Item", &[(P_BODY, "body"), (P_URL, "url")]),
            node_type(T_TOPIC, "Topic", &[(P_LABEL, "label")]),
            node_type(T_CHANGE, "Change", &[(P_STATE, "state")]),
        ],
        edge_types: vec![
            edge_type(E_AUTHORED, "AUTHORED", T_ACTOR, T_ITEM),
            edge_type(E_IN, "IN", T_ITEM, T_SPACE),
            edge_type(E_REPLIES, "REPLIES", T_ITEM, T_ITEM),
            edge_type(E_MENTIONS, "MENTIONS", T_ITEM, T_ACTOR),
            edge_type(E_TAGGED, "TAGGED", T_ITEM, T_TOPIC),
            edge_type(E_REFERS, "REFERS", T_ITEM, T_CHANGE),
            edge_type(E_OWNS, "OWNS", T_ACTOR, T_CHANGE),
        ],
    })
    .expect("the bench ontology coheres")
}

/// One batch: its nodes `(id, type, alias)`, its edges `(id, type, source, target)`, and its
/// relation and property assertions.
struct Batch {
    nodes: Vec<(u64, u64, String)>,
    edges: Vec<(u64, u64, u64, u64)>,
    relations: Vec<(u64, u64, u64, u64)>,
    properties: Vec<(u64, u64, u64, String)>,
    evidence: u64,
}

/// Batch `b` at batch scale `k`, whose predecessor was built at batch scale `previous`.
fn batch(b: u64, k: u64, previous: u64, rng: &mut Rng) -> Batch {
    let mut nodes = Vec::new();
    for (t, count) in [
        (T_ACTOR, 2 * k),
        (T_SPACE, 1),
        (T_ITEM, 55 * k),
        (T_TOPIC, 8 * k),
        (T_CHANGE, 6 * k),
    ] {
        for j in 0..count {
            nodes.push((batch_node(b, t, j), t, format!("n{b}-{t}-{j}")));
        }
    }
    let items: Vec<u64> = (0..55 * k).map(|j| batch_node(b, T_ITEM, j)).collect();
    let changes: Vec<u64> = (0..6 * k).map(|j| batch_node(b, T_CHANGE, j)).collect();
    let topics: Vec<u64> = (0..8 * k).map(|j| batch_node(b, T_TOPIC, j)).collect();
    let space = batch_node(b, T_SPACE, 0);
    let mut pairs = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let i = i as u64;
        let author = if i.is_multiple_of(3) {
            batch_node(b, T_ACTOR, i % 2)
        } else {
            seed_node(T_ACTOR, rng.next() % 12)
        };
        pairs.push((E_AUTHORED, author, *item));
        let into = if i.is_multiple_of(2) {
            space
        } else {
            seed_node(T_SPACE, 0)
        };
        pairs.push((E_IN, *item, into));
        if i > 0 {
            pairs.push((E_REPLIES, *item, items[usize::try_from(i).unwrap() - 1]));
        } else if b > 0 {
            pairs.push((
                E_REPLIES,
                *item,
                batch_node(b - 1, T_ITEM, 55 * previous - 1),
            ));
        }
        pairs.push((E_MENTIONS, *item, seed_node(T_ACTOR, rng.next() % 12)));
        let i = usize::try_from(i).unwrap();
        if i % 2 == 0 {
            pairs.push((E_TAGGED, *item, topics[i % topics.len()]));
        } else {
            pairs.push((E_REFERS, *item, changes[i % changes.len()]));
        }
    }
    for change in &changes {
        pairs.push((E_OWNS, seed_node(T_ACTOR, rng.next() % 12), *change));
    }
    let mut a = b * 100_000;
    let edges = pairs
        .iter()
        .enumerate()
        .map(|(e, (t, s, d))| (b * 10_000 + e as u64, *t, *s, *d))
        .collect();
    let relations = pairs
        .iter()
        .map(|(t, s, d)| {
            a += 1;
            (a, *t, *s, *d)
        })
        .collect();
    let mut properties = Vec::new();
    for (n, t, _) in &nodes {
        let (declared, count): (&[u64], usize) = match *t {
            T_ITEM => (&[P_BODY, P_URL], 15),
            T_ACTOR => (&[P_HANDLE, P_NOTE], 6),
            T_SPACE => (&[P_TITLE], 6),
            T_TOPIC => (&[P_LABEL], 6),
            _ => (&[P_STATE], 6),
        };
        for j in 0..count {
            a += 1;
            let words = 6 + usize::try_from(rng.next() % 20).unwrap();
            properties.push((a, declared[j % declared.len()], *n, rng.words(words)));
        }
    }
    Batch {
        nodes,
        edges,
        relations,
        properties,
        evidence: b % 40 + 1,
    }
}

fn held_assertion(
    id: u64,
    subject: u64,
    predicate: Predicate,
    object: Object<CanonicalValue>,
    evidence_id: u64,
) -> Assertion<CanonicalValue> {
    Assertion {
        id: assertion(id),
        root_id: root(),
        subject: Subject::Node(CanonicalRef::new(node(subject))),
        predicate,
        object,
        evidence: [CanonicalRef::new(evidence(evidence_id))]
            .into_iter()
            .collect(),
        proposed_by: operator(),
        lifecycle: AssertionLifecycle::Active,
        assessment: Assessment::Proposed,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}

/// Canonical state after `BATCHES` batches at batch scale `scale`.
fn graph(scale: u64) -> CanonicalGraph {
    let mut graph = CanonicalGraph {
        attachments: Default::default(),
        root: GraphRoot {
            id: root(),
            space: Space::Canonical,
            schema_version_id: schema(),
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::new(BATCHES),
        ontology: ontology(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };
    let mut seed = Vec::new();
    for a in 0..12 {
        seed.push((seed_node(T_ACTOR, a), T_ACTOR, format!("actor-{a}")));
    }
    seed.push((seed_node(T_SPACE, 0), T_SPACE, "space-0".to_owned()));
    for t in 0..10 {
        seed.push((seed_node(T_TOPIC, t), T_TOPIC, format!("topic-{t}")));
    }
    for i in 1..=40 {
        graph.evidence.insert(
            evidence(i),
            Evidence {
                id: evidence(i),
                source: EvidenceSource::HumanStatement {
                    identity: Some("operator".to_owned()),
                },
                content_hash: ContentHash::of(&format!("payload {i}")),
                extracted_by: operator(),
                observed_at: Timestamp::EPOCH,
                confidence: Confidence::CERTAIN,
            },
        );
    }
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let batches = (0..BATCHES).map(|b| batch(b, scale, scale, &mut rng));
    let add_node = |graph: &mut CanonicalGraph, (n, t, alias): (u64, u64, String)| {
        let mut held = Node::new(node(n), root(), type_id(t), alias.clone());
        held.aliases = vec![alias];
        graph.nodes.insert(node(n), held);
    };
    for entry in seed {
        add_node(&mut graph, entry);
    }
    for batch in batches {
        for entry in batch.nodes {
            add_node(&mut graph, entry);
        }
        for (e, t, s, d) in batch.edges {
            graph.edges.insert(
                edge(e),
                Edge::new(
                    edge(e),
                    root(),
                    type_id(t),
                    CanonicalRef::new(node(s)),
                    CanonicalRef::new(node(d)),
                ),
            );
        }
        for (a, t, s, d) in batch.relations {
            graph.assertions.insert(
                assertion(a),
                held_assertion(
                    a,
                    s,
                    Predicate::Relation(type_id(t)),
                    Object::Node(CanonicalRef::new(node(d))),
                    batch.evidence,
                ),
            );
        }
        for (a, p, n, text) in batch.properties {
            graph.assertions.insert(
                assertion(a),
                held_assertion(
                    a,
                    n,
                    Predicate::Property(property(p)),
                    Object::Value(CanonicalValue::String(text)),
                    batch.evidence,
                ),
            );
        }
    }
    graph
}

/// The edge-heavy half of batch `BATCHES` at batch scale 10, after a graph at `graph_scale`: its
/// nodes, its edges and its relation assertions, as the ingestion shape proposes them.
fn edge_heavy(graph_scale: u64) -> GraphTransaction {
    let batch = batch(BATCHES, 10, graph_scale, &mut Rng(0x5eed));
    let mut operations = Vec::new();
    for (n, t, alias) in batch.nodes {
        operations.push(GraphOperation::CreateNode(NodeDraft {
            id: node(n),
            root_id: root(),
            type_id: type_id(t),
            canonical_name: alias.clone(),
            properties: BTreeMap::new(),
            aliases: vec![alias],
        }));
    }
    for (e, t, s, d) in batch.edges {
        operations.push(GraphOperation::CreateEdge(EdgeDraft {
            id: edge(e),
            root_id: root(),
            type_id: type_id(t),
            source: node(s),
            target: node(d),
            properties: BTreeMap::new(),
        }));
    }
    for (a, t, s, d) in batch.relations {
        operations.push(GraphOperation::AddAssertion(Box::new(Assertion {
            id: assertion(a),
            root_id: root(),
            subject: Subject::Node(node(s)),
            predicate: Predicate::Relation(type_id(t)),
            object: Object::Node(node(d)),
            evidence: [evidence(batch.evidence)].into_iter().collect(),
            proposed_by: operator(),
            lifecycle: AssertionLifecycle::Active,
            assessment: Assessment::Proposed,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        })));
    }
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: operator(),
        operations,
        evidence: [evidence(batch.evidence)].into_iter().collect(),
        schema_version: None,
    }
}

#[test]
fn validating_one_batch_costs_the_batch_and_not_the_batch_times_the_graph() {
    let mut fastest = BTreeMap::new();
    for scale in SCALES {
        let built = Instant::now();
        let graph = graph(scale);
        let proposal = edge_heavy(scale);
        let held = HeldIdentities::of([&graph]);
        let lineage: BTreeSet<SchemaVersionId> = [schema()].into_iter().collect();
        println!(
            "scale {scale:>2}: {} nodes, {} edges, {} assertions, batch of {} operations, built in \
             {:?}",
            graph.nodes.len(),
            graph.edges.len(),
            graph.assertions.len(),
            proposal.operations.len(),
            built.elapsed()
        );
        let pipeline = Pipeline::identity_keeping(validator(), lineage, held);
        let snapshot = GraphSnapshot::of(&graph);
        // Where the time goes, one validator at a time; the profile-v1 structural validator
        // stands in for profile v3's, which is not public and adds only the history check.
        let each: [(&str, &dyn Validator); 7] = [
            ("structural", &Structural),
            ("reference", &Reference),
            ("types", &Types),
            ("cardinality", &CardinalityValidator),
            ("ontology", &OntologyConstraint),
            ("provenance", &Provenance),
            ("authorization", &Authorization { actor: validator() }),
        ];
        let split: Vec<String> = each
            .iter()
            .map(|(name, check)| {
                let started = Instant::now();
                let _ = check.validate(&snapshot, &proposal);
                format!("{name} {:?}", started.elapsed())
            })
            .collect();
        println!("scale {scale:>2} by validator: {}", split.join(", "));
        for round in 0..ROUNDS {
            let started = Instant::now();
            let verdict = pipeline.validate(&snapshot, &proposal);
            let took = started.elapsed();
            if let Err(issues) = &verdict {
                panic!(
                    "the bench batch is valid; it was refused with {} issues, first {:?}",
                    issues.len(),
                    issues.first()
                );
            }
            println!("scale {scale:>2} round {round}: validate {took:?}");
            fastest
                .entry(scale)
                .and_modify(|best: &mut Duration| *best = (*best).min(took))
                .or_insert(took);
        }
    }
    for (scale, took) in &fastest {
        println!("fastest at scale {scale:>2}: {took:?}");
    }
    assert!(
        fastest[&10] <= BOUND_AT_TEN,
        "the edge-heavy batch at scale 10 validates in at most {BOUND_AT_TEN:?}; it took {:?}",
        fastest[&10]
    );
    assert!(
        fastest[&3] <= fastest[&1] * 3,
        "from scale 1 to scale 3 validation grows at most linearly: {:?} at 1, {:?} at 3",
        fastest[&1],
        fastest[&3]
    );
}
