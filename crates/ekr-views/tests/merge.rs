//! `story:merge-entity-applies`: what `ekr.views` reads of a merge, on both providers, over the
//! `merged` fixture.
//!
//! The fixture: the seed holds `alpha` and `beta` (Subject), an edge from alpha to beta with an
//! assertion about it, and an assertion about alpha; revision 1 creates `beta, kept` and an
//! assertion about beta; revision 2 merges beta into `beta, kept`. The edge keeps its id and runs
//! to `beta, kept`, and the assertion about beta is about `beta, kept`.

mod support;

use ekr_core::{NodeId, RevisionNumber};
use ekr_views::{ChangesRequest, DetailRequest, ExpandRequest, Index, QueryError, SinceKind};
use serde_json::Value;

use support::fixtures::{
    self, id, Fixture, Provider, ALPHA_NODE, BETA_NODE, SEEDED_LINK, SUPERSEDED_CLAIM,
    SUPERSEDED_KEPT,
};

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("JSON")
}

/// Each provider's `merged` store, with its head index and revision 1's.
fn each(mut run: impl FnMut(&str, &ekr_kernel::Runtime, &Index, &Index)) {
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), provider);
        Fixture::Merged.build(&runtime);
        let head = Index::load(&runtime, None).expect("the head indexes");
        let before = Index::load(&runtime, Some(RevisionNumber::new(1))).expect("revision 1");
        run(provider.name(), &runtime, &head, &before);
    }
}

/// The changes since revision 1 list the merge once, as a NodeMerged of beta naming `beta, kept`
/// in `target`, carrying beta's type and name and the merge's evidence; nothing is listed as
/// superseded.
#[test]
fn the_changes_list_a_merge() {
    each(|provider, runtime, head, _| {
        let page = head
            .changes(
                runtime,
                &ChangesRequest::new(SinceKind::Revision, 1, None, None).expect("bounds"),
            )
            .expect("the changes answer");
        let document = json(&page.bytes);
        let changes = document["changes"].as_array().expect("changes");
        assert_eq!(changes.len(), 1, "{provider}: {document}");
        let change = &changes[0];
        assert_eq!(change["change"], "NodeMerged", "{provider}: {document}");
        assert_eq!(change["id"], Value::String(uuid(BETA_NODE)), "{provider}");
        assert_eq!(change["revision"], 2, "{provider}");
        assert_eq!(
            change["target"],
            Value::String(uuid(SUPERSEDED_KEPT)),
            "{provider}"
        );
        assert_eq!(change["name"], "beta", "{provider}");
        assert_eq!(
            change["type"],
            Value::String(uuid(fixtures::SUBJECT_TYPE)),
            "{provider}"
        );
        assert_eq!(
            change["evidence"],
            serde_json::json!([fixtures::statement_id(0).to_string()]),
            "{provider}"
        );
        assert_eq!(page.summary.nodes_merged, 1, "{provider}");
        assert_eq!(page.summary.nodes_superseded, 0, "{provider}");
    });
}

/// At the head, beta is refused as `NodeSuperseded` naming `beta, kept`, in either detail format
/// and as an expansion seed; at revision 1 it is described with its edge.
#[test]
fn a_merged_node_is_refused_naming_the_node_it_was_merged_into() {
    each(|provider, _, head, before| {
        let beta: NodeId = id(BETA_NODE);
        let refused = |error: QueryError, what: &str| match error {
            QueryError::NodeSuperseded { node, by, revision } => {
                assert_eq!(node, beta, "{provider} {what}");
                assert_eq!(by, id::<NodeId>(SUPERSEDED_KEPT), "{provider} {what}");
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
        let described = before.describe(beta).expect("revision 1 describes beta");
        assert!(
            String::from_utf8_lossy(&described.bytes).contains(&uuid(SEEDED_LINK)),
            "{provider}: revision 1 lists beta's edge"
        );
    });
}

/// The rewritten edge keeps its id and reaches `beta, kept` from alpha, and `beta, kept` is
/// described with the assertion the merge moved to it.
#[test]
fn the_rewritten_references_reach_the_node_kept() {
    each(|provider, _, head, _| {
        let expanded = head
            .expand(
                &ExpandRequest::new(vec![id::<NodeId>(ALPHA_NODE)], 1, 100, None, None)
                    .expect("bounds"),
            )
            .expect("the expansion answers");
        let text = String::from_utf8_lossy(&expanded.bytes).into_owned();
        assert!(text.contains(&uuid(SEEDED_LINK)), "{provider}: {text}");
        assert!(text.contains(&uuid(SUPERSEDED_KEPT)), "{provider}: {text}");
        assert!(!text.contains(&uuid(BETA_NODE)), "{provider}: {text}");
        let described = head
            .describe(id(SUPERSEDED_KEPT))
            .expect("the node kept is described");
        assert!(
            String::from_utf8_lossy(&described.bytes).contains(&uuid(SUPERSEDED_CLAIM)),
            "{provider}: the moved assertion is listed under the node kept"
        );
    });
}
