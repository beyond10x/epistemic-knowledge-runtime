//! `story:supersede-node`: every head read of `ekr.views` omits a superseded node, a read of its
//! id at the head is refused as `NodeSuperseded` naming the node it is superseded by, and every
//! earlier revision still reads it — on both providers, over the `superseded` fixture.
//!
//! The fixture: the seed holds `alpha` and `beta` (Subject), an edge from alpha to beta with an
//! assertion about it, and an assertion about alpha; revision 1 creates `beta, kept` and an
//! assertion about beta; revision 2 supersedes beta by `beta, kept`.

mod support;

use ekr_core::{NodeId, RevisionNumber};
use ekr_views::{
    ChangesRequest, DetailRequest, ExpandRequest, Index, MatchesFormat, OverviewRequest,
    QueryError, SearchFilter, SearchRequest, SinceKind, TimelineRequest,
};
use serde_json::Value;

use support::fixtures::{
    self, id, Fixture, Provider, ALPHA_NODE, BETA_NODE, REFERENCING_EDGE_CLAIM,
    REFERENCING_RELATION, SEEDED_LINK, SUPERSEDED_KEPT,
};

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("JSON")
}

/// Each provider's `superseded` store, with its head index and revision 1's.
fn each(mut run: impl FnMut(&str, &ekr_kernel::Runtime, &Index, &Index)) {
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), provider);
        Fixture::Superseded.build(&runtime);
        let head = Index::load(&runtime, None).expect("the head indexes");
        let before = Index::load(&runtime, Some(RevisionNumber::new(1))).expect("revision 1");
        run(provider.name(), &runtime, &head, &before);
    }
}

/// The node count of an overview, and its per-type count of `Subject`.
fn counts(overview: &Value) -> (u64, u64) {
    let types = overview["node_types"].as_array().expect("node types");
    let subject = types
        .iter()
        .find(|entry| entry["type"] == Value::String(uuid(fixtures::SUBJECT_TYPE)))
        .expect("the Subject type");
    (
        overview["meta"]["node_count"].as_u64().expect("node count"),
        subject["count"].as_u64().expect("a type count"),
    )
}

/// `a-superseded-node-leaves-every-head-read`: search (/1, /2 and /3) for beta's name and alias
/// returns no beta; the overview counts one node and one Subject fewer than revision 1; an
/// expansion from alpha, beta's former neighbour, returns neither beta nor an edge to it; the
/// projection and the timeline hold no beta.
#[test]
fn a_superseded_node_leaves_every_head_read() {
    each(|provider, runtime, head, before| {
        let beta = uuid(BETA_NODE);
        for text in ["beta", "beta-alias-a"] {
            for format in [MatchesFormat::V1, MatchesFormat::V2, MatchesFormat::V3] {
                let request = SearchRequest::new(text.to_owned(), 20).expect("bounds");
                let filter = SearchFilter::new(None, None, None);
                let found = head
                    .search_as(format, &request, &filter)
                    .expect("the search answers");
                let earlier = before
                    .search_as(format, &request, &filter)
                    .expect("the search answers");
                assert!(
                    !String::from_utf8_lossy(&found.bytes).contains(&beta),
                    "{provider} {format:?} {text}: the head's search finds the superseded node"
                );
                assert!(
                    String::from_utf8_lossy(&earlier.bytes).contains(&beta),
                    "{provider} {format:?} {text}: revision 1 finds it"
                );
            }
        }

        let overview = |index: &Index| {
            json(
                &index
                    .overview(&OverviewRequest::new(None).expect("bounds"))
                    .expect("the overview answers")
                    .bytes,
            )
        };
        let (now, then) = (counts(&overview(head)), counts(&overview(before)));
        assert_eq!(now.0 + 1, then.0, "{provider}: node count");
        assert_eq!(now.1 + 1, then.1, "{provider}: Subject count");
        assert!(
            !overview(head).to_string().contains(&beta),
            "{provider}: the overview names the superseded node"
        );

        let expanded = head
            .expand(
                &ExpandRequest::new(vec![id::<NodeId>(ALPHA_NODE)], 2, 100, None, None)
                    .expect("bounds"),
            )
            .expect("the expansion answers");
        let text = String::from_utf8_lossy(&expanded.bytes).into_owned();
        assert!(
            !text.contains(&beta),
            "{provider}: the expansion reaches beta"
        );
        assert!(
            !text.contains(&uuid(SEEDED_LINK)),
            "{provider}: the expansion lists the retired edge"
        );

        let projected = ekr_views::project(runtime, None).expect("the projection answers");
        let text = String::from_utf8_lossy(&projected.bytes).into_owned();
        assert!(
            !text.contains(&beta),
            "{provider}: the projection holds beta"
        );
        assert!(
            !text.contains(&uuid(SEEDED_LINK)),
            "{provider}: the projection holds the retired edge"
        );
        let earlier =
            ekr_views::project(runtime, Some(RevisionNumber::new(1))).expect("revision 1 projects");
        assert!(String::from_utf8_lossy(&earlier.bytes).contains(&beta));

        let timeline = head
            .timeline(&TimelineRequest::new(None, 1, 100, None, None).expect("bounds"))
            .expect("the timeline answers");
        assert!(
            !String::from_utf8_lossy(&timeline.bytes).contains(&beta),
            "{provider}: the timeline holds beta"
        );
    });
}

/// `a-read-of-a-superseded-id-names-its-successor`: `DescribeNode` (/1 and /2) and an expansion
/// seeded with beta, at the head, are refused as `NodeSuperseded` naming `beta, kept`.
#[test]
fn a_read_of_a_superseded_id_names_its_successor() {
    each(|provider, _, head, _| {
        let beta: NodeId = id(BETA_NODE);
        let kept: NodeId = id(SUPERSEDED_KEPT);
        let refused = |error: QueryError, what: &str| match error {
            QueryError::NodeSuperseded { node, by, revision } => {
                assert_eq!(node, beta, "{provider} {what}");
                assert_eq!(by, kept, "{provider} {what}");
                assert_eq!(revision, RevisionNumber::new(2), "{provider} {what}");
            }
            other => panic!("{provider} {what}: {other:?}"),
        };
        refused(head.describe(beta).expect_err("refused"), "describe /1");
        refused(
            head.detail_v2(beta, &DetailRequest::new(None, None, None).expect("bounds"))
                .expect_err("refused"),
            "describe /2",
        );
        refused(
            head.expand(&ExpandRequest::new(vec![beta], 1, 10, None, None).expect("bounds"))
                .expect_err("refused"),
            "expand",
        );
        let message = head.describe(beta).expect_err("refused").to_string();
        assert!(
            message.contains(&uuid(SUPERSEDED_KEPT)),
            "{provider}: {message}"
        );
    });
}

/// `a-superseded-node-reads-at-an-earlier-revision`: at revision 1, `DescribeNode` answers beta
/// with its edge, its assertion and that assertion's evidence.
#[test]
fn a_superseded_node_reads_at_an_earlier_revision() {
    each(|provider, _, _, before| {
        let detail = json(
            &before
                .describe(id(BETA_NODE))
                .expect("revision 1 describes beta")
                .bytes,
        );
        let text = detail.to_string();
        assert!(text.contains(&uuid(SEEDED_LINK)), "{provider}: its edge");
        assert!(
            text.contains(&uuid(fixtures::SUPERSEDED_CLAIM)),
            "{provider}: its assertion"
        );
        assert!(
            text.contains(&fixtures::statement_id(0).to_string()),
            "{provider}: its evidence"
        );
    });
}

/// The changes since revision 1 list the supersession once, as a NodeSuperseded of beta naming
/// `beta, kept` in `target`, carrying the supersession's evidence.
#[test]
fn the_changes_list_a_supersession() {
    each(|provider, runtime, head, _| {
        let page = head
            .changes(
                runtime,
                &ChangesRequest::new(SinceKind::Revision, 1, None, None).expect("bounds"),
            )
            .expect("the changes answer");
        let document = json(&page.bytes);
        let changes = document["changes"].as_array().expect("changes");
        let superseded: Vec<&Value> = changes
            .iter()
            .filter(|change| change["change"] == "NodeSuperseded")
            .collect();
        assert_eq!(superseded.len(), 1, "{provider}: {document}");
        let change = superseded[0];
        assert_eq!(change["id"], Value::String(uuid(BETA_NODE)), "{provider}");
        assert_eq!(change["revision"], 2, "{provider}");
        assert_eq!(
            change["target"],
            Value::String(uuid(SUPERSEDED_KEPT)),
            "{provider}"
        );
        assert_eq!(change["name"], "beta", "{provider}");
        assert_eq!(
            change["evidence"],
            serde_json::json!([fixtures::statement_id(0).to_string()]),
            "{provider}"
        );
        assert_eq!(page.summary.nodes_superseded, 1, "{provider}");
    });
}

/// A timeline whose `subject` is beta is refused at the head as `NodeSuperseded` naming
/// `beta, kept`, as `DescribeNode` refuses it, in both formats; at revision 1 it answers beta's
/// row; a subject no revision holds still answers no row.
#[test]
fn a_timeline_of_a_superseded_subject_names_its_successor() {
    each(|provider, _, head, before| {
        let request = |subject: u64| {
            TimelineRequest::new(None, 1, 10, None, Some(id(subject))).expect("bounds")
        };
        for format in [ekr_views::TimelineFormat::V1, ekr_views::TimelineFormat::V2] {
            match head.timeline_as(format, &request(BETA_NODE)) {
                Err(QueryError::NodeSuperseded { node, by, revision }) => {
                    assert_eq!(node, id::<NodeId>(BETA_NODE), "{provider} {format:?}");
                    assert_eq!(by, id::<NodeId>(SUPERSEDED_KEPT), "{provider} {format:?}");
                    assert_eq!(revision, RevisionNumber::new(2), "{provider} {format:?}");
                }
                other => panic!("{provider} {format:?}: {:?}", other.map(|a| a.summary)),
            }
        }
        let earlier = before
            .timeline(&request(BETA_NODE))
            .expect("revision 1 answers beta's row");
        assert_eq!(earlier.summary.rows, 1, "{provider}: {:?}", earlier.summary);
        let unknown = head
            .timeline(&request(0xdead_beef))
            .expect("an unheld subject is no refusal");
        assert_eq!(unknown.summary.rows, 0, "{provider}");
    });
}

/// Each provider's [`fixtures::build_superseded_referencing`] store, with its head index and
/// revision 1's, at which beta and its edge are still held.
fn each_referencing(mut run: impl FnMut(&str, &Index, &Index)) {
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), provider);
        fixtures::build_superseded_referencing(&runtime);
        let head = Index::load(&runtime, None).expect("the head indexes");
        let before = Index::load(&runtime, Some(RevisionNumber::new(1))).expect("revision 1");
        run(provider.name(), &head, &before);
    }
}

/// `story:supersede-node-review-fixes` D1 (`a-superseded-subject-references-nothing`): alpha is
/// the object of a Relation assertion about beta and of an assertion about the edge from alpha to
/// beta. Once beta is superseded, `DescribeNode` /1 of alpha lists neither under `referencing` and
/// counts neither, as it lists neither under any other key; at revision 1 it lists both.
#[test]
fn a_live_nodes_referencing_omits_assertions_about_a_superseded_subject() {
    each_referencing(|provider, head, before| {
        let alpha: NodeId = id(ALPHA_NODE);
        let listed = |index: &Index| {
            let answer = index.describe(alpha).expect("alpha is held");
            let document = json(&answer.bytes);
            let ids: Vec<String> = document["referencing"]
                .as_array()
                .expect("referencing")
                .iter()
                .map(|entry| entry["assertion"]["id"].as_str().expect("an id").to_owned())
                .collect();
            (ids, answer.summary.referencing)
        };
        let kept = [uuid(REFERENCING_RELATION), uuid(REFERENCING_EDGE_CLAIM)];
        let (earlier, counted) = listed(before);
        for claim in &kept {
            assert!(
                earlier.contains(claim),
                "{provider}: revision 1 lists {claim} under referencing: {earlier:?}"
            );
        }
        assert_eq!(counted, 2, "{provider}: revision 1 counts both");
        let (now, counted) = listed(head);
        for claim in &kept {
            assert!(
                !now.contains(claim),
                "{provider}: the head lists {claim}, about a superseded subject: {now:?}"
            );
        }
        assert_eq!(counted, 0, "{provider}: the head counts {now:?}");
    });
}

/// `story:supersede-node-review-fixes` D3 (`a-superseded-subject-dates-nothing`): the Relation
/// assertion `beta links alpha`, valid from an hour after alpha's own claim, is alpha's second
/// dated fact while beta is held, so revision 1 judges one `Subject` by its facts. Once beta is
/// superseded the head dates nothing from it — the overview's timeline already omits it — so alpha
/// has one dated fact and no `Subject` is judged.
#[test]
fn an_assertion_about_a_superseded_subject_dates_no_node() {
    each_referencing(|provider, head, before| {
        let subject_timing = |index: &Index| {
            let overview = json(
                &index
                    .overview(&OverviewRequest::new(None).expect("bounds"))
                    .expect("the overview answers")
                    .bytes,
            );
            overview["roles"]["types"]
                .as_array()
                .expect("roles")
                .iter()
                .find(|timing| timing["type"] == Value::String(uuid(fixtures::SUBJECT_TYPE)))
                .expect("the Subject type")
                .clone()
        };
        assert_eq!(
            subject_timing(before)["judged"],
            1,
            "{provider}: alpha has two dated facts while beta is held"
        );
        let timing = subject_timing(head);
        assert_eq!(
            timing["judged"], 0,
            "{provider}: a fact about the superseded beta dates alpha: {timing}"
        );
        assert_eq!(timing["within_hour"], 0, "{provider}: {timing}");
    });
}

/// `story:supersede-node-review-fixes` D9 (`a-read-indexes-retired-edges-once`): a sample filtered
/// to the `links` edge type reads two assertions about the edge retired with beta — the seeded
/// one and `about` — and looks the edge up for its type and its pointer. Counted, not timed: the
/// read reads each supersession record once, however many lookups it makes.
#[test]
fn a_sample_reads_each_supersession_record_once() {
    for provider in [Provider::File, Provider::Sqlite] {
        let name = provider.name();
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), provider);
        fixtures::build_superseded_referencing(&runtime);
        let read = runtime.read(None).expect("the head reads");
        assert_eq!(read.graph.superseded.len(), 1, "{name}");
        let request =
            ekr_views::SampleRequest::new(7, 10, Some(id(fixtures::LINKS_TYPE))).expect("bounds");
        let before = ekr_graph::supersession_records_read();
        let drawn = ekr_views::sample(&read, &request).expect("the sample draws");
        let reads = ekr_graph::supersession_records_read() - before;
        assert_eq!(drawn.summary.population, 2, "{name}: {:?}", drawn.summary);
        assert_eq!(reads, 1, "{name}: supersession records read for one sample");
        let document = json(&drawn.bytes);
        for item in document["items"].as_array().expect("items") {
            assert_eq!(
                item["subject_superseded_by"],
                Value::String(uuid(SUPERSEDED_KEPT)),
                "{name}: {item}"
            );
            assert_eq!(
                item["subject_type"],
                Value::String(uuid(fixtures::LINKS_TYPE)),
                "{name}: {item}"
            );
        }
    }
}
