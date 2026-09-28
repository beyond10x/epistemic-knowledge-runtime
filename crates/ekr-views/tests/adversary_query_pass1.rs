//! Adversary, story:view-streams-overview-and-expansion, pass 1: the `ekr-views` query engine.
//!
//! Each case drives the engine from `systems/ekr/domains/views.yaml` rather than from the tests
//! the engine was written with:
//!
//! * the record sequence and its paging (`ekr.views.GraphSliceV1`) against an oracle written from
//!   the specification's own words, over generated graphs with self-loops, parallel edges,
//!   isolated nodes and repeated seeds;
//! * the same revision answered under two heads gives the same bytes;
//! * `ekr.node-detail/1` carries every assertion about, and every assertion pointing at, the node;
//! * `ekr.node-matches/1`'s tiers and lowercase folding;

mod support;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use ekr_core::{EdgeId, NodeId, RevisionNumber, Timestamp, TypeId};
use ekr_graph::{Edge, Node, Object, Subject};
use ekr_kernel::Runtime;
use ekr_ontology::{Cardinality, EdgeType, NodeType, Value};
use ekr_views::{ExpandRequest, Index, OverviewRequest, SearchRequest, SliceRecord};
use serde_json::Value as Json;

use support::fixtures::{self, id, Fixture, Provider};

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

// ---- the record sequence against an oracle written from views.yaml -----------------------------

/// A fixed linear congruential sequence.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, below: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % below.max(1)
    }
}

const VERTEX: u64 = 0xab_0001;
const JOINS: u64 = 0xab_0002;

/// A generated graph: node ids scattered so id order is not creation order, edges between any
/// two of them including a node and itself, and some pairs joined more than once.
struct Generated {
    nodes: Vec<NodeId>,
    edges: Vec<(EdgeId, NodeId, NodeId)>,
}

fn generate(rng: &mut Lcg) -> Generated {
    let count = 1 + rng.next(24);
    let mut nodes: Vec<NodeId> = Vec::new();
    let mut taken = BTreeSet::new();
    while (nodes.len() as u64) < count {
        let n = 0xc0_0000 + rng.next(0xffff);
        if taken.insert(n) {
            nodes.push(id(n));
        }
    }
    let edge_count = rng.next(3 * count + 1);
    let mut edges = Vec::new();
    let mut edge_ids = BTreeSet::new();
    while (edges.len() as u64) < edge_count {
        let n = 0xd0_0000 + rng.next(0xffff);
        if !edge_ids.insert(n) {
            continue;
        }
        let source = nodes[rng.next(count) as usize];
        let target = if rng.next(6) == 0 {
            source
        } else {
            nodes[rng.next(count) as usize]
        };
        edges.push((id(n), source, target));
        // Now and then the same pair again, under another id.
        if rng.next(5) == 0 && (edges.len() as u64) < edge_count {
            let n = 0xe0_0000 + rng.next(0xffff);
            if edge_ids.insert(n) {
                edges.push((id(n), source, target));
            }
        }
    }
    Generated { nodes, edges }
}

fn seed_generated(generated: &Generated) -> (tempfile::TempDir, Runtime) {
    let mut document = ekr_kernel::SeedDocument::from_yaml(
        &std::fs::read_to_string(
            std::path::PathBuf::from(
                std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"),
            )
            .join("tests/fixtures/seed-empty.yaml"),
        )
        .unwrap(),
    )
    .unwrap();
    let root = document.graph.root.id;
    document
        .ontology
        .node_types
        .push(NodeType::new(id::<TypeId>(VERTEX), "vertex"));
    let mut joins = EdgeType::new(id::<TypeId>(JOINS), "joins");
    joins.source_types = [id(VERTEX)].into_iter().collect();
    joins.target_types = [id(VERTEX)].into_iter().collect();
    joins.cardinality = Cardinality::Many;
    document.ontology.edge_types.push(joins);
    for (n, node) in generated.nodes.iter().enumerate() {
        let held = Node::<Value>::new(*node, root, id(VERTEX), format!("n{n}"));
        document.graph.nodes.insert(held.id, held);
    }
    for (edge, source, target) in &generated.edges {
        document.graph.edges.insert(
            *edge,
            Edge::<Value> {
                id: *edge,
                root_id: root,
                type_id: id(JOINS),
                source: *source,
                target: *target,
                properties: BTreeMap::new(),
            },
        );
    }
    let work = tempfile::tempdir().unwrap();
    let runtime = fixtures::open(work.path(), Provider::File);
    runtime
        .seed(document, || Timestamp::from_millis(1_800_000_000_001))
        .expect("the generated seed is admitted");
    (work, runtime)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Expected {
    Node(NodeId, u64),
    Edge(EdgeId),
}

/// views.yaml, ekr.views.GraphSliceV1, read literally.
fn oracle(generated: &Generated, seeds: &[NodeId], depth: u64) -> Vec<Expected> {
    let mut degree: HashMap<NodeId, u64> = generated.nodes.iter().map(|n| (*n, 0)).collect();
    for (_, source, target) in &generated.edges {
        if source != target {
            *degree.get_mut(source).unwrap() += 1;
            *degree.get_mut(target).unwrap() += 1;
        }
    }
    let mut distance: BTreeMap<NodeId, u64> = seeds.iter().map(|seed| (*seed, 0)).collect();
    for hop in 1..=depth {
        let reached: Vec<NodeId> = distance
            .iter()
            .filter(|(_, at)| **at == hop - 1)
            .map(|(node, _)| *node)
            .collect();
        for node in reached {
            for (_, source, target) in &generated.edges {
                for (here, there) in [(source, target), (target, source)] {
                    if *here == node && !distance.contains_key(there) {
                        distance.insert(*there, hop);
                    }
                }
            }
        }
    }
    let mut order: Vec<NodeId> = distance.keys().copied().collect();
    order.sort_by(|a, b| {
        distance[a]
            .cmp(&distance[b])
            .then(degree[b].cmp(&degree[a]))
            .then(a.to_string().cmp(&b.to_string()))
    });
    let place: HashMap<NodeId, usize> = order.iter().enumerate().map(|(at, n)| (*n, at)).collect();
    let mut sequence = Vec::new();
    for (at, node) in order.iter().enumerate() {
        sequence.push(Expected::Node(*node, distance[node]));
        let mut closes: Vec<EdgeId> = generated
            .edges
            .iter()
            .filter(|(_, source, target)| {
                let other = if source == node {
                    target
                } else if target == node {
                    source
                } else {
                    return false;
                };
                other == node || place.get(other).is_some_and(|there| *there < at)
            })
            .map(|(edge, _, _)| *edge)
            .collect();
        closes.sort_by_key(ToString::to_string);
        sequence.extend(closes.into_iter().map(Expected::Edge));
    }
    sequence
}

/// The page views.yaml describes: from `after`, stopping before the first record that would
/// hold more than `limit` nodes or `edge_limit` edges.
fn oracle_page(
    sequence: &[Expected],
    after: u64,
    limit: u64,
    edge_limit: u64,
) -> (Vec<Expected>, Option<u64>, u64) {
    let total = sequence.len() as u64;
    let mut page = Vec::new();
    let (mut nodes, mut edges) = (0, 0);
    for record in sequence
        .iter()
        .skip(usize::try_from(after.min(total)).unwrap())
    {
        match record {
            Expected::Node(..) if nodes == limit => break,
            Expected::Edge(_) if edges == edge_limit => break,
            Expected::Node(..) => nodes += 1,
            Expected::Edge(_) => edges += 1,
        }
        page.push(record.clone());
    }
    let end = after + page.len() as u64;
    (
        (page),
        (end < total).then_some(end),
        total.saturating_sub(end),
    )
}

fn observed(records: &[SliceRecord]) -> Vec<Expected> {
    records
        .iter()
        .map(|record| match record {
            SliceRecord::Node(node) => Expected::Node(node.id, node.distance),
            SliceRecord::Edge(edge) => Expected::Edge(edge.id),
        })
        .collect()
}

/// Generated graphs, generated requests: every page the engine answers is the page the
/// specification describes, and paging from 0 by `next` delivers the sequence exactly once.
#[test]
fn every_page_of_generated_graphs_is_the_page_views_yaml_describes() {
    let mut rng = Lcg(0x5eed_0001);
    for graph in 0..24 {
        let generated = generate(&mut rng);
        let (_work, runtime) = seed_generated(&generated);
        let index = Index::load(&runtime, None).unwrap();
        for request in 0..12 {
            let count = generated.nodes.len() as u64;
            let seeds: Vec<NodeId> = (0..1 + rng.next(3))
                .map(|_| generated.nodes[rng.next(count) as usize])
                .collect();
            let depth = rng.next(3);
            let limit = 1 + rng.next(6);
            let edge_limit = 1 + rng.next(6);
            let mut unique = seeds.clone();
            unique.sort();
            unique.dedup();
            let sequence = oracle(&generated, &unique, depth);
            let what = format!(
                "graph {graph} request {request}: seeds {seeds:?} depth {depth} limit {limit} \
                 edges {edge_limit}; {} nodes {} edges",
                generated.nodes.len(),
                generated.edges.len()
            );
            let whole = index
                .page(&ExpandRequest::new(seeds.clone(), depth as i64, 2_000, None, None).unwrap())
                .unwrap();
            assert_eq!(observed(whole.records()), sequence, "{what}: the sequence");
            let meta = whole.meta();
            let nodes = sequence
                .iter()
                .filter(|r| matches!(r, Expected::Node(..)))
                .count() as u64;
            assert_eq!(
                (meta.node_total, meta.edge_total),
                (nodes, sequence.len() as u64 - nodes),
                "{what}: totals"
            );
            let mut after = 0_u64;
            let mut delivered = Vec::new();
            loop {
                let page = index
                    .page(
                        &ExpandRequest::new(
                            seeds.clone(),
                            depth as i64,
                            limit as i64,
                            Some(edge_limit as i64),
                            Some(after as i64),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                let (expected, next, remaining) = oracle_page(&sequence, after, limit, edge_limit);
                assert_eq!(
                    observed(page.records()),
                    expected,
                    "{what}: page at {after}"
                );
                assert_eq!(page.next(), next, "{what}: next at {after}");
                assert_eq!(page.remaining(), remaining, "{what}: remaining at {after}");
                delivered.extend(observed(page.records()));
                match page.next() {
                    Some(next) => after = next,
                    None => break,
                }
            }
            assert_eq!(delivered, sequence, "{what}: paged once each");
        }
    }
}

// ---- one revision under two heads ---------------------------------------------------------------

/// An answer's exact text: since `task:historical-projection-carries-the-head` no format names
/// the head, so nothing is masked.
fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap()
}

fn four_answers(index: &Index) -> Vec<(String, String)> {
    let mut answers = vec![(
        "overview".to_owned(),
        text(
            index
                .overview(&OverviewRequest::new(None).unwrap())
                .unwrap()
                .bytes,
        ),
    )];
    for node in [0x110_u64, 0x111] {
        answers.push((
            format!("describe {node:x}"),
            text(index.describe(id(node)).unwrap().bytes),
        ));
        for depth in 0..=2 {
            answers.push((
                format!("expand {node:x} {depth}"),
                text(
                    index
                        .expand(&ExpandRequest::new(vec![id(node)], depth, 10, None, None).unwrap())
                        .unwrap()
                        .bytes,
                ),
            ));
        }
    }
    for query in ["", "a", "ALPHA", "-alias-"] {
        answers.push((
            format!("search {query:?}"),
            text(
                index
                    .search(&SearchRequest::new(query.to_owned(), 100).unwrap())
                    .unwrap()
                    .bytes,
            ),
        ));
    }
    answers
}

/// A store at head 1 and the same store three unrelated commits later answer revisions 0 and 1
/// with the same bytes.
#[test]
fn a_revision_answers_the_same_bytes_after_unrelated_commits() {
    let (short_dir, long_dir) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let short = fixtures::open(short_dir.path(), Provider::File);
    fixtures::build_long(&short, 0);
    let long = fixtures::open(long_dir.path(), Provider::Sqlite);
    fixtures::build_long(&long, 3);
    for at in [0_u64, 1] {
        let before = Index::load(&short, Some(RevisionNumber::new(at))).unwrap();
        let after = Index::load(&long, Some(RevisionNumber::new(at))).unwrap();
        let head = |runtime: &Runtime| runtime.head().unwrap().unwrap().revision.get();
        assert_eq!((head(&short), head(&long)), (1, 4));
        for ((what, one), (_, other)) in four_answers(&before).iter().zip(four_answers(&after)) {
            assert_eq!(one, &other, "revision {at}: {what}");
        }
    }
}

// ---- ekr.node-detail/1 carries every assertion --------------------------------------------------

/// For every node of every revision of `store`: `assertions` is every assertion whose subject is
/// the node, `referencing` every one whose object is the node, `edges` every edge at it with
/// every assertion about that edge, and `neighbours` every other node those name — by id.
#[test]
fn every_detail_of_every_revision_carries_every_assertion_about_and_at_the_node() {
    for fixture in [Fixture::Evolved, Fixture::Growth, Fixture::Timeline] {
        let work = tempfile::tempdir().unwrap();
        let runtime = fixtures::open(work.path(), Provider::File);
        fixture.build(&runtime);
        let head = runtime.head().unwrap().unwrap().revision.get();
        for at in 0..=head {
            let index = Index::load(&runtime, Some(RevisionNumber::new(at))).unwrap();
            let graph = &index.loaded().graph;
            for node in graph.nodes.keys() {
                let what = format!("{fixture:?} revision {at} node {node}");
                let detail: Json =
                    serde_json::from_slice(&index.describe(*node).unwrap().bytes).unwrap();
                let listed = |list: &Json, key: &dyn Fn(&Json) -> String| -> Vec<String> {
                    list.as_array().unwrap().iter().map(key).collect()
                };
                let sorted = |mut ids: Vec<String>| {
                    ids.sort();
                    ids
                };
                let about: Vec<String> = sorted(
                    graph
                        .assertions
                        .values()
                        .filter(|c| matches!(&c.subject, Subject::Node(n) if n.id() == *node))
                        .map(|c| c.id.to_string())
                        .collect(),
                );
                let pointing: Vec<String> = sorted(
                    graph
                        .assertions
                        .values()
                        .filter(|c| matches!(&c.object, Object::Node(n) if n.id() == *node))
                        .map(|c| c.id.to_string())
                        .collect(),
                );
                let incident: Vec<&Edge<ekr_graph::CanonicalValue>> = graph
                    .edges
                    .values()
                    .filter(|e| e.source.id() == *node || e.target.id() == *node)
                    .collect();
                assert_eq!(
                    listed(&detail["assertions"], &|a| a["id"]
                        .as_str()
                        .unwrap()
                        .to_owned()),
                    about,
                    "{what}: assertions"
                );
                assert_eq!(
                    listed(&detail["referencing"], &|r| r["assertion"]["id"]
                        .as_str()
                        .unwrap()
                        .to_owned()),
                    pointing,
                    "{what}: referencing"
                );
                assert_eq!(
                    listed(&detail["edges"], &|e| e["id"].as_str().unwrap().to_owned()),
                    sorted(incident.iter().map(|e| e.id.to_string()).collect()),
                    "{what}: edges"
                );
                for edge in detail["edges"].as_array().unwrap() {
                    let edge_id: EdgeId = edge["id"].as_str().unwrap().parse().unwrap();
                    let expected: Vec<String> = sorted(
                        graph
                            .assertions
                            .values()
                            .filter(|c| matches!(&c.subject, Subject::Edge(e) if e.id() == edge_id))
                            .map(|c| c.id.to_string())
                            .collect(),
                    );
                    assert_eq!(
                        listed(&edge["assertions"], &|a| a["id"]
                            .as_str()
                            .unwrap()
                            .to_owned()),
                        expected,
                        "{what}: edge {edge_id}'s assertions"
                    );
                }
                let mut named: BTreeSet<String> = BTreeSet::new();
                for edge in &incident {
                    named.insert(edge.source.id().to_string());
                    named.insert(edge.target.id().to_string());
                }
                for claim in graph.assertions.values() {
                    if let (Subject::Node(s), Object::Node(o)) = (&claim.subject, &claim.object) {
                        if s.id() == *node && graph.nodes.contains_key(&o.id()) {
                            named.insert(o.id().to_string());
                        }
                        if o.id() == *node && graph.nodes.contains_key(&s.id()) {
                            named.insert(s.id().to_string());
                        }
                    }
                }
                named.remove(&node.to_string());
                assert_eq!(
                    listed(&detail["neighbours"], &|n| n["id"]
                        .as_str()
                        .unwrap()
                        .to_owned()),
                    named.into_iter().collect::<Vec<_>>(),
                    "{what}: neighbours"
                );
            }
        }
    }
}

// ---- ekr.node-matches/1 -------------------------------------------------------------------------

fn named_seed(names: &[(&str, &[&str])]) -> (tempfile::TempDir, Runtime) {
    let mut document = ekr_kernel::SeedDocument::from_yaml(
        &std::fs::read_to_string(
            std::path::PathBuf::from(
                std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"),
            )
            .join("tests/fixtures/seed-empty.yaml"),
        )
        .unwrap(),
    )
    .unwrap();
    let root = document.graph.root.id;
    document
        .ontology
        .node_types
        .push(NodeType::new(id::<TypeId>(VERTEX), "vertex"));
    for (n, (name, aliases)) in names.iter().enumerate() {
        let mut node = Node::<Value>::new(id(0xf0_0000 + n as u64), root, id(VERTEX), *name);
        node.aliases = aliases.iter().map(|a| (*a).to_owned()).collect();
        document.graph.nodes.insert(node.id, node);
    }
    let work = tempfile::tempdir().unwrap();
    let runtime = fixtures::open(work.path(), Provider::File);
    runtime
        .seed(document, || Timestamp::from_millis(1_800_000_000_001))
        .expect("the named seed is admitted");
    (work, runtime)
}

/// views.yaml, ekr.views.MatchTier and ekr.views.NodeMatch, read literally, over names whose
/// lowercase mapping changes their length or has no single-character form.
#[test]
fn search_tiers_fields_and_aliases_are_the_ones_views_yaml_names() {
    let names: &[(&str, &[&str])] = &[
        ("İstanbul", &["ISTANBUL", "istanbul"]),
        ("STRASSE", &["Straẞe", "strasse"]),
        ("ǅemal", &[]),
        ("ΟΔΟΣ", &["οδος"]),
        ("plain", &["PLAIN", "Plain"]),
        ("Mixed Case", &["mixed case"]),
    ];
    let (_work, runtime) = named_seed(names);
    let index = Index::load(&runtime, None).unwrap();
    for text in [
        "", "i", "İ", "i̇", "ISTANBUL", "istanbul", "ß", "STRASSE", "ǆ", "ǅ", "σ", "ς", "Σ",
        "PLAIN", "plain", "Plain", "case", "CASE",
    ] {
        let answer: Json = serde_json::from_slice(
            &index
                .search(&SearchRequest::new(text.to_owned(), 100).unwrap())
                .unwrap()
                .bytes,
        )
        .unwrap();
        let mut expected: Vec<(u8, String, &str, Option<&str>)> = Vec::new();
        let folded = text.to_lowercase();
        for (n, (name, aliases)) in names.iter().enumerate() {
            let node = uuid(0xf0_0000 + n as u64);
            let hit = if name.contains(text) {
                Some((0, "Name", None))
            } else if let Some(alias) = aliases.iter().find(|a| a.contains(text)) {
                Some((0, "Alias", Some(*alias)))
            } else if name.to_lowercase().contains(&folded) {
                Some((1, "Name", None))
            } else {
                aliases
                    .iter()
                    .find(|a| a.to_lowercase().contains(&folded))
                    .map(|alias| (1, "Alias", Some(*alias)))
            };
            if let Some((tier, field, alias)) = hit {
                expected.push((tier, node, field, alias));
            }
        }
        // Every degree is 0, so the order is tier, then id.
        expected.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        let got: Vec<(u8, String, &str, Option<&str>)> = answer["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| {
                (
                    u8::from(m["tier"] == "Folded"),
                    m["id"].as_str().unwrap().to_owned(),
                    m["field"].as_str().unwrap(),
                    m.get("alias").map(|a| a.as_str().unwrap()),
                )
            })
            .collect();
        assert_eq!(got, expected, "text {text:?}");
        assert_eq!(answer["meta"]["total"], expected.len(), "text {text:?}");
        assert_eq!(
            answer["meta"]["exact_total"],
            expected.iter().filter(|e| e.0 == 0).count(),
            "text {text:?}"
        );
    }
}

// ---- the overview's size ------------------------------------------------------------------------

/// The page's first load is the overview alone, and the story's acceptance holds it under 300 KB
/// (`index_time.rs` names the same cap as the approved plan's). `limit` bounds only `top`: the
/// timeline has one bucket per week per node type once the dated facts span more than 120 days,
/// so a store of ten node types with a fact a week for ten years answers an overview far past
/// the cap at the default request — nothing the request can lower.
#[test]
fn the_default_overview_of_ten_types_dated_weekly_for_ten_years_stays_under_300_kb() {
    use ekr_core::{AssertionId, ContentHash, EvidenceId, PropertyId};
    use ekr_graph::{
        Assertion, AssertionLifecycle, Assessment, Confidence, Evidence, EvidenceSource, Predicate,
        TemporalRange, TransactionTime,
    };
    use ekr_ontology::{PropertyDefinition, ValueType};

    const TYPES: u64 = 10;
    const WEEKS: i64 = 520;
    const WEEK_MS: i64 = 604_800_000;
    let mut document = ekr_kernel::SeedDocument::from_yaml(
        &std::fs::read_to_string(
            std::path::PathBuf::from(
                std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"),
            )
            .join("tests/fixtures/seed-empty.yaml"),
        )
        .unwrap(),
    )
    .unwrap();
    let root = document.graph.root.id;
    let bytes = b"a weekly statement".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let evidence: EvidenceId = id(0xee_0001);
    document.graph.evidence.insert(
        evidence,
        Evidence {
            id: evidence,
            source: EvidenceSource::HumanStatement {
                identity: Some("operator".into()),
            },
            content_hash: hash,
            extracted_by: fixtures::context().operator,
            observed_at: Timestamp::from_millis(1_000),
            confidence: Confidence::from_basis_points(9_000).unwrap(),
        },
    );
    document.evidence_payloads.insert(hash, bytes);
    for t in 0..TYPES {
        let type_id: TypeId = id(0xee_1000 + t);
        let property: PropertyId = id(0xee_2000 + t);
        let mut declared = NodeType::new(type_id, format!("kind-{t}"));
        declared.properties.insert(
            property,
            PropertyDefinition::new(property, "reading", ValueType::String),
        );
        document.ontology.node_types.push(declared);
        let node = Node::<Value>::new(id(0xee_3000 + t), root, type_id, format!("subject-{t}"));
        let node_id = node.id;
        document.graph.nodes.insert(node_id, node);
        for week in 0..WEEKS {
            let assertion = Assertion {
                id: id::<AssertionId>(0xee_0000_0000 + t * 0x1_0000 + week as u64),
                root_id: root,
                subject: Subject::Node(node_id),
                predicate: Predicate::Property(property),
                object: Object::Value(Value::String(format!("week {week}"))),
                evidence: [evidence].into_iter().collect(),
                proposed_by: fixtures::context().operator,
                assessment: Assessment::Proposed,
                lifecycle: AssertionLifecycle::Active,
                valid_time: TemporalRange::since(Timestamp::from_millis(
                    1_500_000_000_000 + week * WEEK_MS,
                )),
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            };
            document.graph.assertions.insert(assertion.id, assertion);
        }
    }
    let work = tempfile::tempdir().unwrap();
    let runtime = fixtures::open(work.path(), Provider::File);
    runtime
        .seed(document, || Timestamp::from_millis(1_800_000_000_001))
        .expect("the weekly seed is admitted");
    let index = Index::load(&runtime, None).unwrap();
    let overview = index
        .overview(&OverviewRequest::new(None).unwrap())
        .unwrap();
    let smallest = index
        .overview(&OverviewRequest::new(Some(1)).unwrap())
        .unwrap();
    assert!(
        overview.bytes.len() < 300 * 1024,
        "{TYPES} types and {} assertions: the default overview is {} bytes, {} timeline buckets \
         (limit 1: {} bytes)",
        TYPES * WEEKS as u64,
        overview.bytes.len(),
        overview.summary.timeline_buckets,
        smallest.bytes.len()
    );
}
