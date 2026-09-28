//! The measurement for the bounded reads: a generated store of 50,000 nodes and 150,000 edges,
//! seeded through the real kernel, loaded, indexed once, and read by each of the four queries.
//! Printed rather than bounded, except the overview's size, which the approved plan caps at
//! 300 KB.
//!
//! Behind the `bench` feature, as `render_time` is: seeding 200,000 records through the debug
//! kernel takes longer than the package gate should. Run it with
//! `cargo test -p ekr-views --release --features bench --test index_time -- --nocapture`.

mod support;

use std::time::{Duration, Instant};

use ekr_views::{ExpandRequest, Index, OverviewRequest, SearchRequest};

use support::fixtures::{self, Provider};

const NODES: u64 = 50_000;
const EDGES: u64 = 150_000;
const OVERVIEW_CAP_BYTES: usize = 300 * 1024;

fn timed<T>(what: impl FnOnce() -> T) -> (T, Duration) {
    let started = Instant::now();
    let value = what();
    (value, started.elapsed())
}

#[test]
fn the_index_of_a_fifty_thousand_node_store_answers_each_read_in_proportion_to_its_answer() {
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), provider);
        let ((), seeded) = timed(|| fixtures::build_large(&runtime, NODES, EDGES));
        drop(runtime);
        let runtime = fixtures::reopen(work.path(), provider);

        let (loaded, load_time) = timed(|| ekr_views::load(&runtime, None).expect("load"));
        assert_eq!(loaded.graph.nodes.len() as u64, NODES);
        assert_eq!(loaded.graph.edges.len() as u64, EDGES);
        let (index, build_time) = timed(|| Index::build(loaded).expect("index"));

        let request = OverviewRequest::new(None).unwrap();
        let (overview, overview_time) = timed(|| index.overview(&request).expect("overview"));
        let (again, overview_again) = timed(|| index.overview(&request).expect("overview"));
        assert_eq!(overview.bytes, again.bytes);
        assert!(
            overview.bytes.len() < OVERVIEW_CAP_BYTES,
            "the overview is {} bytes",
            overview.bytes.len()
        );
        let largest = OverviewRequest::new(Some(500)).unwrap();
        let (widest, widest_time) = timed(|| index.overview(&largest).expect("overview"));
        assert!(widest.bytes.len() < OVERVIEW_CAP_BYTES);

        // The hub (highest degree) and a node of median degree, one hop each.
        let order: Vec<_> = {
            let top = index
                .overview(&OverviewRequest::new(Some(1)).unwrap())
                .unwrap();
            let value: serde_json::Value = serde_json::from_slice(&top.bytes).unwrap();
            vec![value["top"][0]["id"].as_str().unwrap().parse().unwrap()]
        };
        let hub = order[0];
        let ordinary = fixtures::id(0x60_0000_0000 + NODES / 2);
        let mut expand_times = Vec::new();
        for (what, seed) in [("hub", hub), ("median node", ordinary)] {
            let request = ExpandRequest::new(vec![seed], 1, 2_000, None, None).unwrap();
            let (page, time) = timed(|| index.expand(&request).expect("expand"));
            expand_times.push(format!(
                "{what}: {} nodes {} edges of {} + {} in {time:?} ({} bytes)",
                page.summary.nodes,
                page.summary.edges,
                page.summary.node_total,
                page.summary.edge_total,
                page.bytes.len()
            ));
        }
        let (detail, describe_time) = timed(|| index.describe(ordinary).expect("describe"));
        let search = SearchRequest::new("node 4999".into(), 100).unwrap();
        let (matches, search_time) = timed(|| index.search(&search).expect("search"));

        println!(
            "{} provider, {NODES} nodes / {EDGES} edges / {} assertions (seeded in {seeded:?}):\n\
             \x20 load {load_time:?}, index build {build_time:?}\n\
             \x20 overview (limit 300) {} bytes in {overview_time:?}, again {overview_again:?}; \
             limit 500 {} bytes in {widest_time:?}\n\
             \x20 expand depth 1, {}\n\
             \x20 describe median node: {} edges, {} bytes in {describe_time:?}\n\
             \x20 search `node 4999`: {} of {} matches in {search_time:?}",
            provider.name(),
            index.loaded().graph.assertions.len(),
            overview.bytes.len(),
            widest.bytes.len(),
            expand_times.join("; "),
            detail.summary.edges,
            detail.bytes.len(),
            matches.summary.matches,
            matches.summary.total,
        );
    }
}
