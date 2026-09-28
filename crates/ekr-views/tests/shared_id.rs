//! A node and an edge that hold one UUID are two records: the projection and the four bounded
//! reads list under each only the assertions whose subject is that record, on both providers.
//! The case of the 2026-09-28 external review, where the projection refused the revision
//! because the two records' assertions were bucketed under one key.

mod support;

use ekr_core::NodeId;
use ekr_views::{ExpandRequest, Index, OverviewRequest, SearchRequest, SliceRecord};
use serde_json::Value;

use support::fixtures::{
    self, id, Provider, SEEDED_EDGE_CLAIM, SEEDED_EDGE_ID, SHARED, SHARED_EDGE_CLAIM,
    SHARED_NODE_CLAIM,
};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn built(provider: Provider) -> (tempfile::TempDir, ekr_kernel::Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    fixtures::build_shared_id(&runtime);
    (work, runtime)
}

fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("JSON")
}

fn ids(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("an array")
        .iter()
        .map(|entry| entry["id"].as_str().expect("an id").to_owned())
        .collect()
}

/// The entry of `records` whose `id` is `wanted`.
fn with_id<'a>(records: &'a Value, wanted: &str) -> &'a Value {
    records
        .as_array()
        .expect("an array")
        .iter()
        .find(|entry| entry["id"] == wanted)
        .unwrap_or_else(|| panic!("no record {wanted} in {records}"))
}

#[test]
fn the_projection_lists_each_records_own_assertions_when_a_node_and_an_edge_share_an_id() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let rendered = ekr_views::project(&runtime, None)
            .unwrap_or_else(|e| panic!("{}: the revision projects: {e}", provider.name()));
        let document = json(&rendered.bytes);
        let shared = uuid(SHARED);
        assert_eq!(
            document["meta"]["assertion_count"],
            3,
            "{}",
            provider.name()
        );
        let node = with_id(&document["nodes"], &shared);
        assert_eq!(
            ids(&node["assertions"]),
            [uuid(SHARED_NODE_CLAIM)],
            "{}",
            provider.name()
        );
        let edge = with_id(&document["edges"], &shared);
        assert_eq!(
            ids(&edge["assertions"]),
            [uuid(SHARED_EDGE_CLAIM)],
            "{}",
            provider.name()
        );
        let seeded = with_id(&document["edges"], &uuid(SEEDED_EDGE_ID));
        assert_eq!(
            ids(&seeded["assertions"]),
            [uuid(SEEDED_EDGE_CLAIM)],
            "{}",
            provider.name()
        );
    }
}

#[test]
fn the_four_reads_keep_a_node_and_an_edge_that_share_an_id_apart() {
    for provider in PROVIDERS {
        let name = provider.name();
        let (_work, runtime) = built(provider);
        let index = Index::load(&runtime, None)
            .unwrap_or_else(|e| panic!("{name}: the revision indexes: {e}"));
        let shared = uuid(SHARED);

        let overview = index
            .overview(&OverviewRequest::new(None).expect("default limit"))
            .unwrap_or_else(|e| panic!("{name}: overview: {e}"));
        assert_eq!(
            (
                overview.summary.nodes,
                overview.summary.edges,
                overview.summary.assertions
            ),
            (3, 2, 3),
            "{name}"
        );

        let detail = index
            .describe(id::<NodeId>(SHARED))
            .unwrap_or_else(|e| panic!("{name}: describe: {e}"));
        let detail = json(&detail.bytes);
        assert_eq!(detail["node"]["id"], shared.as_str(), "{name}");
        assert_eq!(
            ids(&detail["assertions"]),
            [uuid(SHARED_NODE_CLAIM)],
            "{name}"
        );
        assert_eq!(detail["referencing"], Value::Array(Vec::new()), "{name}");
        let edge = with_id(&detail["edges"], &shared);
        assert_eq!(
            ids(&edge["assertions"]),
            [uuid(SHARED_EDGE_CLAIM)],
            "{name}"
        );

        let page = index
            .page(&ExpandRequest::new(vec![id(SHARED)], 1, 100, None, None).expect("within bounds"))
            .unwrap_or_else(|e| panic!("{name}: expand: {e}"));
        let edge = page
            .records()
            .iter()
            .find_map(|record| match record {
                SliceRecord::Edge(edge) if edge.id.to_string() == shared => Some(edge),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{name}: the shared edge is in the slice"));
        assert_eq!(
            edge.assertions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            [uuid(SHARED_EDGE_CLAIM)],
            "{name}"
        );
        let slice = json(&page.render().expect("the page encodes").bytes);
        let edge = with_id(&slice["edges"], &shared);
        assert_eq!(
            edge["assertions"],
            Value::Array(vec![Value::String(uuid(SHARED_EDGE_CLAIM))]),
            "{name}"
        );

        let found = index
            .search(&SearchRequest::new("shared".into(), 10).expect("within bounds"))
            .unwrap_or_else(|e| panic!("{name}: search: {e}"));
        assert_eq!(found.summary.total, 1, "{name}");
        assert_eq!(found.summary.first_match, Some(id(SHARED)), "{name}");
    }
}
