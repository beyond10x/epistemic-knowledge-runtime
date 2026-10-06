//! The four bounded reads of `ekr.views` over one [`Index`]: ekr.graph-overview/1,
//! ekr.graph-slice/1, ekr.node-detail/1 and ekr.node-matches/1, held against
//! `systems/ekr/domains/views.yaml`'s determinism rules, bounds and refusal order on the `hub`,
//! `growth` and `timeline` fixtures; and the revision cache `ekr view` keeps them in.

mod support;

use std::sync::Arc;

use ekr_core::{NodeId, RevisionNumber};
use ekr_views::{
    ExpandRequest, Index, IndexCache, LimitExceeded, Lineage, OverviewRequest, ProjectError,
    QueryError, RevisionIdentity, SearchRequest, SliceRecord,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use support::fixtures::{
    self, id, Fixture, Provider, GROWTH_EDGE, GROWTH_FIRST, GROWTH_SECOND, HUB, HUB_LEAVES,
    JANUARY_FIRST_0900_MS, KIND_A, KIND_B, KIND_C,
};

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn built(fixture: Fixture, provider: Provider) -> (tempfile::TempDir, ekr_kernel::Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    fixture.build(&runtime);
    (work, runtime)
}

fn index(fixture: Fixture, at: Option<u64>) -> Index {
    let (_work, runtime) = built(fixture, Provider::File);
    Index::load(&runtime, at.map(RevisionNumber::new)).expect("the revision indexes")
}

fn json(bytes: &[u8]) -> (String, Value) {
    let text = String::from_utf8(bytes.to_vec()).expect("UTF-8");
    let value = serde_json::from_str(&text).expect("JSON");
    (text, value)
}

fn ids(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("an array")
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map_or_else(|| entry["id"].as_str().unwrap().to_owned(), str::to_owned)
        })
        .collect()
}

fn expand(
    seeds: &[u64],
    depth: i64,
    limit: i64,
    edges: Option<i64>,
    after: Option<i64>,
) -> ExpandRequest {
    ExpandRequest::new(
        seeds.iter().map(|n| id::<NodeId>(*n)).collect(),
        depth,
        limit,
        edges,
        after,
    )
    .expect("within bounds")
}

// ---- ekr.graph-overview/1 -----------------------------------------------------------------------

#[test]
fn the_overview_opens_with_its_meta_and_lists_its_sections_in_declared_order() {
    let hub = index(Fixture::Hub, None);
    let answer = hub
        .overview(&OverviewRequest::new(Some(2)).unwrap())
        .expect("overview");
    let (text, value) = json(&answer.bytes);
    assert!(
        text.starts_with(
            "{\"meta\":{\"format\":\"ekr.graph-overview/1\",\"revision\":0,\
             \"node_count\":601,\"edge_count\":600,\"assertion_count\":0,\"evidence_count\":0,\
             \"limit\":2},\"ontology\":{\"node_types\":["
        ),
        "{text}"
    );
    assert!(!text.contains('\n') && !text.contains("\": ") && !text.contains("null"));
    let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
    let order: Vec<usize> = [
        "\"meta\":",
        "\"ontology\":",
        "\"schema\":",
        "\"node_types\":[{\"type\"",
        "\"edge_types\":[{\"type\"",
        "\"roles\":",
        "\"timeline\":",
        "\"top\":",
    ]
    .iter()
    .map(|key| {
        text.find(key)
            .unwrap_or_else(|| panic!("{key} missing in {text}"))
    })
    .collect();
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
    assert_eq!(keys.len(), 8);
    // The hub, degree 600, then leaf-001: degree descending, then id.
    assert_eq!(
        value["top"],
        json!([
            {"id": uuid(HUB), "type": uuid(0x11_0000), "name": "hub", "degree": 600},
            {"id": uuid(HUB + 1), "type": uuid(0x11_0000), "name": "leaf-001", "degree": 1},
        ])
    );
    assert!(text.ends_with(&format!(
        "\"top\":[{{\"id\":\"{}\",\"type\":\"{}\",\"name\":\"hub\",\"degree\":600}},\
         {{\"id\":\"{}\",\"type\":\"{}\",\"name\":\"leaf-001\",\"degree\":1}}]}}",
        uuid(HUB),
        uuid(0x11_0000),
        uuid(HUB + 1),
        uuid(0x11_0000)
    )));
    assert_eq!(
        value["schema"]["versions"],
        json!([{
            "number": 0, "id": uuid(1), "revision": 0,
            "added": [
                {"id": uuid(0x11_0000), "kind": "NodeType", "name": "vertex"},
                {"id": uuid(0x11_0001), "kind": "EdgeType", "name": "joins"},
            ],
            "removed": [],
        }])
    );
    assert_eq!(
        value["node_types"],
        json!([{"type": uuid(0x11_0000), "count": 601}])
    );
    assert_eq!(
        value["edge_types"],
        json!([{"type": uuid(0x11_0001), "count": 600}])
    );
    assert_eq!(
        value["timeline"],
        json!({"bucket_ms": 86_400_000, "undated": 0, "buckets": []})
    );
    assert_eq!(
        value["roles"],
        json!({"types": [{
            "type": uuid(0x11_0000), "nodes": 601, "timestamped": 0, "judged": 0,
            "within_hour": 0, "instant": 0, "neighbour_types": 601, "event": false,
        }]})
    );
    let summary = answer.summary;
    assert_eq!(
        summary.overview_hash,
        hex::encode(Sha256::digest(&answer.bytes))
    );
    assert_eq!((summary.nodes, summary.edges, summary.top), (601, 600, 2));
    assert_eq!((summary.added, summary.removed), (2, 0));
    assert_eq!(
        (summary.unrecorded_nodes, summary.unrecorded_edges),
        (601, 600)
    );
    assert_eq!(summary.observation_type, None);
}

#[test]
fn the_overview_counts_growth_per_revision_from_recorded_time() {
    let growth = index(Fixture::Growth, None);
    let (_, value) = json(
        &growth
            .overview(&OverviewRequest::new(None).unwrap())
            .unwrap()
            .bytes,
    );
    assert_eq!(value["meta"]["limit"], 300);
    let revisions = value["schema"]["revisions"].as_array().unwrap();
    assert_eq!(revisions.len(), 2);
    assert!(revisions[0].get("transaction_id").is_none());
    assert_eq!(
        (
            &revisions[0]["nodes"],
            &revisions[0]["edges"],
            &revisions[0]["assertions"]
        ),
        (&json!(1), &json!(0), &json!(0))
    );
    assert_eq!(
        (
            &revisions[1]["nodes"],
            &revisions[1]["edges"],
            &revisions[1]["assertions"]
        ),
        (&json!(2), &json!(1), &json!(2))
    );
    assert_eq!(revisions[1]["transaction_id"], uuid(0x201));
    assert_eq!(value["schema"]["unrecorded_nodes"], 1);
    assert_eq!(value["schema"]["unrecorded_edges"], 0);
    // Both nodes have degree 1; id decides.
    assert_eq!(
        ids(&value["top"]),
        [uuid(GROWTH_FIRST), uuid(GROWTH_SECOND)]
    );
}

#[test]
fn the_overview_derives_roles_and_monday_weeks_from_valid_time() {
    let at_head = index(Fixture::Timeline, None);
    let (_, value) = json(
        &at_head
            .overview(&OverviewRequest::new(None).unwrap())
            .unwrap()
            .bytes,
    );
    let roles = &value["roles"];
    assert_eq!(roles["observation_type"], uuid(KIND_A));
    assert_eq!(
        roles["types"],
        json!([
            {"type": uuid(KIND_A), "nodes": 5, "timestamped": 0, "judged": 5, "within_hour": 5,
             "instant": 5, "neighbour_types": 5, "event": true},
            {"type": uuid(KIND_B), "nodes": 2, "timestamped": 0, "judged": 2, "within_hour": 0,
             "instant": 0, "neighbour_types": 2, "event": false},
            {"type": uuid(KIND_C), "nodes": 6, "timestamped": 0, "judged": 6, "within_hour": 6,
             "instant": 6, "neighbour_types": 0, "event": true},
        ])
    );
    let timeline = &value["timeline"];
    assert_eq!(timeline["bucket_ms"], 604_800_000);
    assert_eq!(timeline["first"], JANUARY_FIRST_0900_MS);
    assert_eq!(timeline["last"], 1_811_851_200_000_i64);
    assert_eq!(timeline["undated"], 1);
    // Mondays 00:00 UTC: 2026-12-28, 2027-01-04, 2027-01-11, 2027-05-31.
    let (dec_28, jan_4) = (1_798_416_000_000_i64, 1_799_020_800_000_i64);
    let (jan_11, may_31) = (1_799_625_600_000_i64, 1_811_721_600_000_i64);
    assert_eq!(
        timeline["buckets"],
        json!([
            {"start": dec_28, "type": uuid(KIND_A), "assertions": 6},
            {"start": dec_28, "type": uuid(KIND_B), "assertions": 2},
            {"start": dec_28, "type": uuid(KIND_C), "assertions": 12},
            {"start": jan_4, "type": uuid(KIND_A), "assertions": 4},
            {"start": jan_11, "type": uuid(KIND_B), "assertions": 2},
            {"start": may_31, "type": uuid(KIND_B), "assertions": 1},
        ])
    );

    let seed = index(Fixture::Timeline, Some(0));
    let (_, value) = json(
        &seed
            .overview(&OverviewRequest::new(None).unwrap())
            .unwrap()
            .bytes,
    );
    assert_eq!(value["timeline"]["bucket_ms"], 86_400_000);
    assert_eq!(value["timeline"]["buckets"].as_array().unwrap().len(), 7);
    assert_eq!(
        value["roles"]["types"][1]["judged"], 0,
        "one dated fact each"
    );
}

// ---- ekr.graph-slice/1 --------------------------------------------------------------------------

#[test]
fn a_slice_lists_each_node_before_the_edges_it_closes_and_pages_by_record() {
    let hub = index(Fixture::Hub, None);
    let answer = hub
        .expand(&expand(&[HUB, HUB], 1, 3, None, None))
        .expect("expanded");
    let (text, value) = json(&answer.bytes);
    assert_eq!(
        text,
        format!(
            "{{\"meta\":{{\"format\":\"ekr.graph-slice/1\",\"revision\":0,\
             \"seeds\":[\"{hub}\"],\"depth\":1,\"after\":0,\"node_total\":601,\"edge_total\":600}},\
             \"nodes\":[\
             {{\"id\":\"{hub}\",\"type\":\"{vertex}\",\"name\":\"hub\",\"degree\":600,\"distance\":0}},\
             {{\"id\":\"{l1}\",\"type\":\"{vertex}\",\"name\":\"leaf-001\",\"degree\":1,\"distance\":1}},\
             {{\"id\":\"{l2}\",\"type\":\"{vertex}\",\"name\":\"leaf-002\",\"degree\":1,\"distance\":1}}],\
             \"edges\":[\
             {{\"id\":\"{e1}\",\"source\":\"{hub}\",\"target\":\"{l1}\",\"type\":\"{joins}\",\"assertions\":[]}},\
             {{\"id\":\"{e2}\",\"source\":\"{hub}\",\"target\":\"{l2}\",\"type\":\"{joins}\",\"assertions\":[]}}],\
             \"next\":5}}",
            hub = uuid(HUB),
            vertex = uuid(0x11_0000),
            joins = uuid(0x11_0001),
            l1 = uuid(HUB + 1),
            l2 = uuid(HUB + 2),
            e1 = uuid(0x20_0001),
            e2 = uuid(0x20_0002),
        )
    );
    assert_eq!(value["next"], 5);
    let summary = answer.summary;
    assert_eq!(
        (summary.nodes, summary.edges, summary.remaining),
        (3, 2, 1_196)
    );
    assert_eq!(
        summary.slice_hash,
        hex::encode(Sha256::digest(&answer.bytes))
    );

    // The record sequence is one sequence: the pages of any size concatenate to it.
    let whole = hub.page(&expand(&[HUB], 1, 2_000, None, None)).unwrap();
    assert_eq!(whole.records().len(), 1_201);
    assert!(whole.next().is_none());
    let mut paged = Vec::new();
    let mut after = None;
    loop {
        let page = hub.page(&expand(&[HUB], 1, 7, Some(5), after)).unwrap();
        paged.extend(page.records().iter().cloned());
        match page.next() {
            Some(next) => after = Some(i64::try_from(next).unwrap()),
            None => break,
        }
    }
    assert_eq!(paged, whole.records());
    let end = hub
        .expand(&expand(&[HUB], 1, 7, None, Some(1_201)))
        .unwrap();
    let (_, value) = json(&end.bytes);
    assert_eq!(
        (value["nodes"].clone(), value.get("next")),
        (json!([]), None)
    );
    assert_eq!(end.summary.remaining, 0);
    let past = hub
        .expand(&expand(&[HUB], 1, 7, None, Some(5_000)))
        .unwrap();
    assert_eq!(past.summary.after, 5_000);
    assert_eq!((past.summary.nodes, past.summary.remaining), (0, 0));
}

#[test]
fn distance_orders_before_degree_and_a_seed_set_ignores_order_and_repetition() {
    let hub = index(Fixture::Hub, None);
    let page = hub.page(&expand(&[HUB + 1], 2, 3, None, None)).unwrap();
    let records: Vec<String> = page
        .records()
        .iter()
        .map(|record| match record {
            SliceRecord::Node(node) => format!("n{}:{}", node.distance, node.id),
            SliceRecord::Edge(edge) => format!("e:{}", edge.id),
        })
        .collect();
    assert_eq!(
        records,
        [
            format!("n0:{}", uuid(HUB + 1)),
            format!("n1:{}", uuid(HUB)),
            format!("e:{}", uuid(0x20_0001)),
            format!("n2:{}", uuid(HUB + 2)),
            format!("e:{}", uuid(0x20_0002)),
        ]
    );
    let one = hub
        .expand(&expand(&[HUB + 2, HUB + 1], 0, 10, None, None))
        .unwrap();
    let other = hub
        .expand(&expand(&[HUB + 1, HUB + 2, HUB + 1], 0, 10, None, None))
        .unwrap();
    assert_eq!(one.bytes, other.bytes);
    let (_, value) = json(&one.bytes);
    assert_eq!(ids(&value["meta"]["seeds"]), [uuid(HUB + 1), uuid(HUB + 2)]);
    assert_eq!(value["edges"], json!([]), "two leaves share no edge");
    let none = hub.expand(&expand(&[], 2, 10, None, None)).unwrap();
    assert_eq!((none.summary.node_total, none.summary.nodes), (0, 0));
}

#[test]
fn a_slice_edge_carries_the_ids_of_the_assertions_about_it() {
    let (_work, runtime) = built(Fixture::EdgeAssertion, Provider::File);
    let index = Index::load(&runtime, None).unwrap();
    let (_, value) = json(
        &index
            .expand(&expand(&[0x110], 1, 10, None, None))
            .unwrap()
            .bytes,
    );
    assert_eq!(value["edges"][0]["id"], uuid(0x140));
    assert_eq!(value["edges"][0]["assertions"], json!([uuid(0x131)]));
}

// ---- ekr.node-detail/1 --------------------------------------------------------------------------

#[test]
fn a_detail_carries_the_node_its_own_and_incoming_assertions_its_edges_and_neighbours() {
    let growth = index(Fixture::Growth, None);
    let second = growth.describe(id(GROWTH_SECOND)).expect("described");
    let (text, value) = json(&second.bytes);
    assert!(
        text.starts_with(&format!(
            "{{\"meta\":{{\"format\":\"ekr.node-detail/1\",\"revision\":1}},\
             \"node\":{{\"id\":\"{}\",\"name\":\"second\",\"type\":\"{}\",\"aliases\":[],\
             \"props\":{{}},\"degree\":1}},\"assertions\":[{{\"id\":\"{}\",",
            uuid(GROWTH_SECOND),
            uuid(0x40_0100),
            uuid(0x40_0011)
        )),
        "{text}"
    );
    assert_eq!(
        ids(&value["assertions"]),
        [uuid(0x40_0011), uuid(0x40_0012)]
    );
    assert_eq!(value["assertions"][1]["predicate_kind"], "Relation");
    assert_eq!(value["assertions"][1]["object_ref"], uuid(GROWTH_FIRST));
    assert_eq!(value["referencing"], json!([]));
    assert_eq!(ids(&value["edges"]), [uuid(GROWTH_EDGE)]);
    assert_eq!(value["edges"][0]["props"], json!({}));
    assert_eq!(
        value["neighbours"],
        json!([{"id": uuid(GROWTH_FIRST), "type": uuid(0x40_0100), "name": "first", "degree": 1}])
    );
    assert_eq!(
        (
            second.summary.assertions,
            second.summary.referencing,
            second.summary.edges
        ),
        (2, 0, 1)
    );

    let first = growth.describe(id(GROWTH_FIRST)).unwrap();
    let (_, value) = json(&first.bytes);
    assert_eq!(value["assertions"], json!([]));
    let referencing = &value["referencing"][0];
    assert_eq!(referencing["subject_kind"], "Node");
    assert_eq!(referencing["subject"], uuid(GROWTH_SECOND));
    assert_eq!(referencing["assertion"]["id"], uuid(0x40_0012));
    assert_eq!(ids(&value["neighbours"]), [uuid(GROWTH_SECOND)]);
    assert_eq!(
        first.summary.detail_hash,
        hex::encode(Sha256::digest(&first.bytes))
    );
}

#[test]
fn a_node_created_later_is_not_found_at_an_earlier_revision() {
    let seed = index(Fixture::Growth, Some(0));
    match seed.describe(id(GROWTH_SECOND)) {
        Err(QueryError::NodeNotFound { node, revision }) => {
            assert_eq!((node, revision.get()), (id(GROWTH_SECOND), 0));
        }
        other => panic!("{other:?}"),
    }
    match seed.expand(&expand(
        &[GROWTH_FIRST, GROWTH_SECOND, 0x40_0009],
        1,
        10,
        None,
        None,
    )) {
        Err(QueryError::NodeNotFound { node, .. }) => assert_eq!(node, id(GROWTH_SECOND)),
        other => panic!("{other:?}"),
    }
}

// ---- ekr.node-matches/1 -------------------------------------------------------------------------

#[test]
fn exact_matches_come_before_folded_ones_whatever_their_degree() {
    let hub = index(Fixture::Hub, None);
    let answer = hub
        .search(&SearchRequest::new("Hub".into(), 10).unwrap())
        .unwrap();
    let (text, _) = json(&answer.bytes);
    assert_eq!(
        text,
        format!(
            "{{\"meta\":{{\"format\":\"ekr.node-matches/1\",\"revision\":0,\
             \"text\":\"Hub\",\"total\":2,\"exact_total\":1}},\"matches\":[\
             {{\"id\":\"{leaf}\",\"type\":\"{vertex}\",\"name\":\"leaf-600\",\"degree\":1,\
             \"tier\":\"Exact\",\"field\":\"Alias\",\"alias\":\"Hub-Leaf\"}},\
             {{\"id\":\"{hub}\",\"type\":\"{vertex}\",\"name\":\"hub\",\"degree\":600,\
             \"tier\":\"Folded\",\"field\":\"Name\"}}]}}",
            leaf = uuid(HUB + HUB_LEAVES),
            hub = uuid(HUB),
            vertex = uuid(0x11_0000),
        )
    );
    assert_eq!(answer.summary.first_match, Some(id(HUB + HUB_LEAVES)));
    let every = hub
        .search(&SearchRequest::new(String::new(), 100).unwrap())
        .unwrap();
    assert_eq!((every.summary.total, every.summary.exact_total), (601, 601));
    let (_, value) = json(&every.bytes);
    assert_eq!(value["matches"][0]["id"], uuid(HUB), "highest degree first");
    assert_eq!(value["matches"][0]["field"], "Name");
    let none = hub
        .search(&SearchRequest::new("absent".into(), 1).unwrap())
        .unwrap();
    assert_eq!((none.summary.matches, none.summary.first_match), (0, None));
}

// ---- bounds and refusals ------------------------------------------------------------------------

fn limit(
    parameter: &'static str,
    requested: i64,
    minimum: i64,
    maximum: Option<i64>,
) -> LimitExceeded {
    LimitExceeded {
        parameter,
        requested,
        minimum,
        maximum,
    }
}

#[test]
fn every_bound_refuses_one_past_it_naming_the_first_broken_input() {
    assert!(OverviewRequest::new(Some(500)).is_ok() && OverviewRequest::new(Some(1)).is_ok());
    assert_eq!(
        OverviewRequest::new(Some(501)).unwrap_err(),
        limit("limit", 501, 1, Some(500))
    );
    assert_eq!(
        OverviewRequest::new(Some(0)).unwrap_err(),
        limit("limit", 0, 1, Some(500))
    );
    assert_eq!(OverviewRequest::new(None).unwrap().limit(), 300);
    assert_eq!(
        SearchRequest::new("x".into(), 101).unwrap_err(),
        limit("limit", 101, 1, Some(100))
    );
    let seeds = || vec![id::<NodeId>(HUB)];
    let refused = |depth, limit_, edges, after| {
        ExpandRequest::new(seeds(), depth, limit_, edges, after).unwrap_err()
    };
    assert_eq!(
        refused(3, 0, Some(0), Some(-1)),
        limit("depth", 3, 0, Some(2))
    );
    assert_eq!(refused(-1, 10, None, None), limit("depth", -1, 0, Some(2)));
    assert_eq!(
        refused(2, 2_001, Some(0), None),
        limit("limit", 2_001, 1, Some(2_000))
    );
    assert_eq!(
        refused(2, 1, Some(5_001), Some(-1)),
        limit("edge_limit", 5_001, 1, Some(5_000))
    );
    assert_eq!(refused(2, 1, None, Some(-1)), limit("after", -1, 0, None));
    let widest = ExpandRequest::new(seeds(), 2, 2_000, None, None).unwrap();
    assert_eq!(widest.edge_limit(), 5_000);
}

#[test]
fn an_unseeded_store_and_an_absent_revision_are_refused_before_any_node_is_looked_up() {
    let work = tempfile::tempdir().expect("work directory");
    let empty = fixtures::open(&work.path().join("empty"), Provider::Sqlite);
    assert!(matches!(
        Index::load(&empty, Some(RevisionNumber::new(4))),
        Err(ProjectError::NotSeeded { requested: Some(_) })
    ));
    let (_seed, runtime) = built(Fixture::SeedOnly, Provider::Sqlite);
    assert!(matches!(
        Index::load(&runtime, Some(RevisionNumber::new(1))),
        Err(ProjectError::RevisionNotFound { .. })
    ));
}

// ---- determinism --------------------------------------------------------------------------------

/// Every answer of every fixture revision, as bytes.
fn every_answer(runtime: &ekr_kernel::Runtime, nodes: &[u64]) -> Vec<Vec<u8>> {
    let head = runtime.head().unwrap().unwrap().revision.get();
    let mut answers = Vec::new();
    for at in 0..=head {
        let index = Index::load(runtime, Some(RevisionNumber::new(at))).unwrap();
        answers.push(
            index
                .overview(&OverviewRequest::new(None).unwrap())
                .unwrap()
                .bytes,
        );
        for node in nodes {
            if let Ok(detail) = index.describe(id(*node)) {
                answers.push(detail.bytes);
                answers.push(
                    index
                        .expand(&expand(&[*node], 2, 2_000, None, None))
                        .unwrap()
                        .bytes,
                );
            }
        }
        for text in ["", "a", "B-", "first"] {
            answers.push(
                index
                    .search(&SearchRequest::new(text.into(), 100).unwrap())
                    .unwrap()
                    .bytes,
            );
        }
    }
    answers
}

#[test]
fn both_providers_answer_every_read_of_every_revision_with_the_same_bytes_every_time() {
    let fixtures: [(Fixture, &[u64]); 3] = [
        (Fixture::Growth, &[GROWTH_FIRST, GROWTH_SECOND]),
        (Fixture::Timeline, &[0x30_0101, 0x30_0201, 0x30_0306]),
        (
            Fixture::Evolved,
            &[0x110, 0x111, 0x112, fixtures::DESCRIBED],
        ),
    ];
    for (fixture, nodes) in fixtures {
        let (_file_dir, file) = built(fixture, Provider::File);
        let (_sqlite_dir, sqlite) = built(fixture, Provider::Sqlite);
        let first = every_answer(&file, nodes);
        assert_eq!(
            first,
            every_answer(&file, nodes),
            "{fixture:?}: twice in one process"
        );
        assert_eq!(
            first,
            every_answer(&sqlite, nodes),
            "{fixture:?}: file and SQLite"
        );
        assert!(first.len() > 4);
    }
}

// ---- the cache ----------------------------------------------------------------------------------

#[test]
fn the_cache_keeps_the_three_most_recently_used_revisions_by_revision() {
    fn shared<T: Send + Sync>(_: &T) {}
    let (_work, runtime) = built(Fixture::Evolved, Provider::File);
    let mut cache = IndexCache::new(IndexCache::DEFAULT_CAPACITY);
    assert_eq!(IndexCache::DEFAULT_CAPACITY, 3);
    let at = |n| Some(RevisionNumber::new(n));
    let head = cache.index(&runtime, None).unwrap();
    shared(&head);
    assert_eq!(head.revision().get(), 5);
    assert!(
        Arc::ptr_eq(&head, &cache.index(&runtime, at(5)).unwrap()),
        "None is the head"
    );
    let one = cache.index(&runtime, at(1)).unwrap();
    let two = cache.index(&runtime, at(2)).unwrap();
    assert_eq!(cache.len(), 3);
    // Touch 5, so 1 is now the least recently used, and 3 evicts it.
    assert!(Arc::ptr_eq(&head, &cache.index(&runtime, at(5)).unwrap()));
    let three = cache.index(&runtime, at(3)).unwrap();
    assert_eq!(cache.len(), 3);
    assert!(cache.get(RevisionNumber::new(1)).is_none());
    assert!(Arc::ptr_eq(
        &two,
        &cache.get(RevisionNumber::new(2)).unwrap()
    ));
    assert!(Arc::ptr_eq(&three, &cache.index(&runtime, at(3)).unwrap()));
    assert!(
        !Arc::ptr_eq(&one, &cache.index(&runtime, at(1)).unwrap()),
        "1 was evicted"
    );
    assert!(matches!(
        cache.index(&runtime, at(6)),
        Err(ProjectError::RevisionNotFound { .. })
    ));
}

/// `task:historical-projection-carries-the-head`: no answer names the head, so a commit leaves a
/// held revision's index current — it is answered from memory, not loaded again — while the
/// head itself is read on every call: `None` names the new head and it is no longer refused.
#[test]
fn a_commit_keeps_every_held_index_and_moves_only_what_none_names() {
    let (_work, runtime) = built(Fixture::Evolved, Provider::Sqlite);
    let mut cache = IndexCache::new(IndexCache::DEFAULT_CAPACITY);
    let at = |n| Some(RevisionNumber::new(n));
    let one = cache.index(&runtime, at(1)).unwrap();
    let five = cache.index(&runtime, None).unwrap();
    assert!(matches!(
        cache.index(&runtime, at(6)),
        Err(ProjectError::RevisionNotFound { requested, head })
            if (requested.get(), head.get()) == (6, 5)
    ));
    fixtures::commit_unrelated(&runtime, 0);
    assert!(Arc::ptr_eq(&one, &cache.index(&runtime, at(1)).unwrap()));
    assert!(Arc::ptr_eq(&five, &cache.index(&runtime, at(5)).unwrap()));
    let six = cache.index(&runtime, None).unwrap();
    assert_eq!(six.revision().get(), 6, "None is the new head");
    assert!(Arc::ptr_eq(&six, &cache.index(&runtime, at(6)).unwrap()));
    assert_eq!(cache.len(), 3);
}

/// Copies the SQLite database at `from` into the one at `to` through SQLite's online backup, the
/// way `sqlite3 .backup` and `.restore` do, and truncates the target's write-ahead log after it.
fn online_backup(from: &std::path::Path, to: &std::path::Path) {
    let source = rusqlite::Connection::open(from).expect("the source database opens");
    let mut target = rusqlite::Connection::open(to).expect("the target database opens");
    let step = rusqlite::backup::Backup::new(&source, &mut target)
        .expect("the backup starts")
        .step(-1)
        .expect("the backup copies every page");
    assert_eq!(step, rusqlite::backup::StepResult::Done);
    target
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .expect("the target checkpoints");
}

/// Issue #74: a store restored to an older snapshot through SQLite's online backup, and committed
/// to after, holds a different revision under a number the cache already holds. The cache answers
/// the revision the store holds under that number — what a fresh handle loads — and drops the
/// discarded one; a revision the restore left in place stays held.
#[test]
fn a_backup_restore_then_a_commit_is_answered_from_the_new_revision_of_the_reused_number() {
    let (work, runtime) = built(Fixture::Evolved, Provider::Sqlite);
    let live = work.path().join("state.db");
    let snapshot = work.path().join("snapshot.db");
    let at = |n| Some(RevisionNumber::new(n));
    let beta = id::<NodeId>(fixtures::BETA_NODE);
    online_backup(&live, &snapshot);

    let mut cache = IndexCache::new(IndexCache::DEFAULT_CAPACITY);
    let seed = Index::load(&runtime, at(0)).unwrap();
    let one = cache.index(&runtime, at(1)).unwrap();
    fixtures::commit_unrelated(&fixtures::reopen(work.path(), Provider::Sqlite), 1);
    let discarded = cache.index(&runtime, None).unwrap();
    assert_eq!(discarded.revision().get(), 6);
    let discarded_identity: RevisionIdentity = discarded.identity();
    assert_eq!(discarded_identity.number.get(), 6);
    let discarded_note = discarded.describe(beta).unwrap().bytes;
    assert!(String::from_utf8_lossy(&discarded_note).contains("note 1"));

    online_backup(&snapshot, &live);
    fixtures::commit_unrelated(&fixtures::reopen(work.path(), Provider::Sqlite), 2);
    let fresh = Index::load(&fixtures::reopen(work.path(), Provider::Sqlite), None).unwrap();
    assert_eq!(fresh.revision().get(), 6, "the new commit reuses number 6");
    let expected = fresh.describe(beta).unwrap().bytes;
    assert!(String::from_utf8_lossy(&expected).contains("note 2"));

    // The store's lineage now: the new revision 6, not the discarded one; the seed and revision
    // 1 are what they were before the restore.
    let lineage = Lineage::read(&runtime, runtime.head().unwrap().unwrap()).unwrap();
    assert_ne!(fresh.identity(), discarded_identity);
    assert_eq!(fresh.identity().number, discarded_identity.number);
    assert!(lineage.holds(&fresh.identity()), "the new revision 6");
    assert!(
        !lineage.holds(&discarded_identity),
        "the discarded revision 6"
    );
    assert!(lineage.holds(&one.identity()), "revision 1");
    assert!(lineage.holds(&seed.identity()), "the seed");

    for asked in [None, at(6)] {
        let answered = cache.index(&runtime, asked).unwrap();
        assert!(
            !Arc::ptr_eq(&discarded, &answered),
            "{asked:?}: the discarded revision 6 is still answered"
        );
        assert_eq!(answered.revision().get(), 6, "{asked:?}");
        assert!(
            answered.loaded() == fresh.loaded(),
            "{asked:?}: not what a fresh handle loads"
        );
        assert_eq!(
            String::from_utf8_lossy(&answered.describe(beta).unwrap().bytes),
            String::from_utf8_lossy(&expected),
            "{asked:?}"
        );
    }
    assert!(
        Arc::ptr_eq(&one, &cache.index(&runtime, at(1)).unwrap()),
        "revision 1 is the same revision after the restore and stays held"
    );
}

// ---- the public names a host builds on ----------------------------------------------------------

/// Every public name of the four reads, used by path the way a host such as `ekr view` would:
/// the bounds a request is held to, the page a host streams record by record, and the answer's
/// bytes and format tag.
#[test]
fn a_host_pages_a_slice_record_by_record_and_reads_each_format_tag() {
    assert_eq!(ekr_views::OverviewRequest::DEFAULT_LIMIT, 300);
    assert_eq!(ekr_views::OverviewRequest::MAX_LIMIT, 500);
    assert_eq!(ekr_views::ExpandRequest::MAX_DEPTH, 2);
    assert_eq!(ekr_views::ExpandRequest::MAX_LIMIT, 2_000);
    assert_eq!(ekr_views::ExpandRequest::MAX_EDGE_LIMIT, 5_000);

    let hub = index(Fixture::Hub, None);
    let overview: ekr_views::Answer<ekr_views::GraphOverviewed> = hub
        .overview(&OverviewRequest::new(Some(2)).expect("within bounds"))
        .expect("overviewed");
    assert_eq!(
        json(&overview.bytes).1["meta"]["format"],
        ekr_views::OVERVIEW_FORMAT
    );

    let page: ekr_views::SlicePage = hub.page(&expand(&[HUB], 1, 3, None, None)).expect("paged");
    let meta: &ekr_views::SliceMeta = page.meta();
    assert_eq!(meta.format, ekr_views::SLICE_FORMAT);
    let mut nodes: Vec<&ekr_views::SliceNode> = Vec::new();
    let mut edges: Vec<&ekr_views::SliceEdge> = Vec::new();
    for record in page.records() {
        match record {
            SliceRecord::Node(node) => nodes.push(node),
            SliceRecord::Edge(edge) => edges.push(edge),
        }
    }
    assert_eq!((nodes.len(), edges.len()), (3, 2));
    let summary = ekr_views::NodeSummary {
        id: nodes[0].id,
        type_id: nodes[0].type_id,
        name: nodes[0].name.clone(),
        degree: nodes[0].degree,
    };
    assert_eq!((summary.name.as_str(), summary.degree), ("hub", 600));

    let detail = hub.describe(id::<NodeId>(HUB)).expect("described");
    assert_eq!(
        json(&detail.bytes).1["meta"]["format"],
        ekr_views::DETAIL_FORMAT
    );
    let matches = hub
        .search(&SearchRequest::new("hub".to_owned(), 1).expect("within bounds"))
        .expect("searched");
    assert_eq!(
        json(&matches.bytes).1["meta"]["format"],
        ekr_views::MATCHES_FORMAT
    );
}
