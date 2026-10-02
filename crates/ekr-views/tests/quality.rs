//! `story:store-quality-report`: ReportStoreQuality and its format ekr.store-quality/1, held
//! against `systems/ekr/domains/views.yaml` on the `quality` fixture, whose figures per revision
//! are known (a-quality-report-counts-evidence-constraints-and-shared-names.yaml states them), on
//! both native providers:
//!
//! * every figure of every revision equals the fixture's count, and the document of revision 0
//!   is exactly the bytes the format's rules give;
//! * two reads of one revision are byte-identical, on one provider and across both, before and
//!   after a later commit, and a read naming no revision is byte for byte the read naming the
//!   head it read;
//! * an unseeded store and a revision beyond the head are refused by name.

mod support;

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use ekr_views::{report_quality, ProjectError, StoreQualityReported};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use support::fixtures::{self, Fixture, Provider, Q_ITEM, Q_ITEMS, Q_OTHER, Q_OTHERS};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

#[test]
fn retained_seed_attachments_count_once_and_declaring_types_do_not_count_inheritance() {
    for provider in PROVIDERS {
        let work = tempfile::tempdir().unwrap();
        let runtime = fixtures::open(work.path(), provider);
        Fixture::QualitySeedAttachment.build(&runtime);
        assert_eq!(
            read(&runtime, Some(3)).1["assertions"]["with_seed_evidence"],
            2
        );
        let (_, head, summary) = read(&runtime, Some(4));
        assert_eq!(head["assertions"]["with_seed_evidence"], 3);
        assert_eq!(summary.with_seed_evidence, 3);
        assert_eq!(head["assertions"]["with_item_evidence"], 3);
        let seed = [
            fixtures::id(fixtures::Q_EVIDENCE),
            fixtures::id(fixtures::Q_EVIDENCE + 1),
        ]
        .into_iter()
        .collect();
        let mut loaded = ekr_views::load(&runtime, None).unwrap();
        loaded.retained.clear();
        assert_eq!(
            ekr_views::quality(&loaded, &seed)
                .unwrap()
                .summary
                .with_seed_evidence,
            0
        );

        let constraints = tempfile::tempdir().unwrap();
        let runtime = fixtures::open(constraints.path(), provider);
        Fixture::QualityConstraints.build(&runtime);
        let (_, document, summary) = read(&runtime, None);
        assert_eq!(
            document["properties"],
            json!({"declared":3,"constrained":3,"constrained_types":2,"constrained_share":10000})
        );
        assert_eq!(summary.constrained_types, 2);
    }
}

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn built(provider: Provider) -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    Fixture::Quality.build(&runtime);
    (work, runtime)
}

/// The report of revision `at` (the head when `None`): its bytes, parsed, and its event.
fn read(runtime: &Runtime, at: Option<u64>) -> (Vec<u8>, Value, StoreQualityReported) {
    let answer = ekr_views::report_quality(runtime, at.map(RevisionNumber::new))
        .expect("the revision is reported");
    let value: Value = serde_json::from_slice(&answer.bytes).expect("JSON");
    (answer.bytes, value, answer.summary)
}

fn shared(type_id: u64, name: &str, nodes: &[u64]) -> Value {
    json!({
        "type": uuid(type_id),
        "name": name,
        "nodes": nodes.iter().map(|n| uuid(*n)).collect::<Vec<_>>(),
    })
}

/// The names two or more nodes of one type hold at revision 0, in the format's order.
fn seed_shared() -> Vec<Value> {
    vec![
        shared(Q_ITEM, "A. Lovelace", &[Q_ITEMS + 1, Q_ITEMS + 3]),
        shared(Q_ITEM, "Ada", &[Q_ITEMS + 1, Q_ITEMS + 2]),
        shared(Q_ITEM, "Countess", &[Q_ITEMS + 2, Q_ITEMS + 4]),
        shared(Q_OTHER, "byron", &[Q_OTHERS + 2, Q_OTHERS + 3]),
    ]
}

/// From revision 1, i6 "Babbage" shares i5's name.
fn later_shared() -> Vec<Value> {
    let mut names = seed_shared();
    names.insert(2, shared(Q_ITEM, "Babbage", &[Q_ITEMS + 5, Q_ITEMS + 6]));
    names
}

fn assertions(active: u64, with_item_evidence: u64, with_seed_evidence: u64) -> Value {
    json!({
        "active": active,
        "with_evidence": active,
        "with_item_evidence": with_item_evidence,
        "with_seed_evidence": with_seed_evidence,
        "with_evidence_share": 10_000,
        "with_item_evidence_share": with_item_evidence * 10_000 / active,
    })
}

fn properties() -> Value {
    json!({"declared": 5, "constrained": 2, "constrained_types": 2, "constrained_share": 4_000})
}

fn expected(revision: u64, active: u64, item: u64, names: Vec<Value>, nodes: u64) -> Value {
    json!({
        "meta": {"format": "ekr.store-quality/1", "revision": revision},
        "assertions": assertions(active, item, [3, 4, 3, 2][revision as usize]),
        "properties": properties(),
        "shared_names": names,
        "sharing_nodes": nodes,
    })
}

#[test]
fn every_figure_of_every_revision_equals_the_fixtures_count_on_both_providers() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let revisions = [
            expected(0, 3, 0, seed_shared(), 6),
            expected(1, 5, 2, later_shared(), 8),
            expected(2, 4, 2, later_shared(), 8),
            expected(3, 4, 3, later_shared(), 8),
        ];
        for (at, want) in revisions.iter().enumerate() {
            let (bytes, value, summary) = read(&runtime, Some(at as u64));
            assert_eq!(&value, want, "{provider:?} revision {at}");
            assert_eq!(
                summary,
                StoreQualityReported {
                    revision: at as u64,
                    active_assertions: want["assertions"]["active"].as_u64().unwrap(),
                    with_evidence: want["assertions"]["with_evidence"].as_u64().unwrap(),
                    with_item_evidence: want["assertions"]["with_item_evidence"].as_u64().unwrap(),
                    with_seed_evidence: want["assertions"]["with_seed_evidence"].as_u64().unwrap(),
                    properties: 5,
                    constrained_properties: 2,
                    constrained_types: 2,
                    shared_names: want["shared_names"].as_array().unwrap().len() as u64,
                    sharing_nodes: want["sharing_nodes"].as_u64().unwrap(),
                    quality_hash: hex::encode(Sha256::digest(&bytes)),
                },
                "{provider:?} revision {at}"
            );
        }
    }
}

/// Rules (2) key order, (3) encoding and (4) presence, on one document written out whole.
#[test]
fn the_document_of_revision_zero_is_exactly_the_formats_bytes() {
    let (_work, runtime) = built(Provider::File);
    let (bytes, _, _) = read(&runtime, Some(0));
    let (item, other) = (uuid(Q_ITEM), uuid(Q_OTHER));
    let node = |n: u64| uuid(n);
    let want = format!(
        concat!(
            r#"{{"meta":{{"format":"ekr.store-quality/1","revision":0}},"#,
            r#""assertions":{{"active":3,"with_evidence":3,"with_item_evidence":0,"with_seed_evidence":3,"#,
            r#""with_evidence_share":10000,"with_item_evidence_share":0}},"#,
            r#""properties":{{"declared":5,"constrained":2,"constrained_types":2,"constrained_share":4000}},"#,
            r#""shared_names":["#,
            r#"{{"type":"{item}","name":"A. Lovelace","nodes":["{i1}","{i3}"]}},"#,
            r#"{{"type":"{item}","name":"Ada","nodes":["{i1}","{i2}"]}},"#,
            r#"{{"type":"{item}","name":"Countess","nodes":["{i2}","{i4}"]}},"#,
            r#"{{"type":"{other}","name":"byron","nodes":["{o2}","{o3}"]}}],"#,
            r#""sharing_nodes":6}}"#
        ),
        item = item,
        other = other,
        i1 = node(Q_ITEMS + 1),
        i2 = node(Q_ITEMS + 2),
        i3 = node(Q_ITEMS + 3),
        i4 = node(Q_ITEMS + 4),
        o2 = node(Q_OTHERS + 2),
        o3 = node(Q_OTHERS + 3),
    );
    assert_eq!(String::from_utf8(bytes).expect("UTF-8"), want);
}

#[test]
fn two_reads_of_one_revision_are_byte_identical_on_both_providers_and_after_a_later_commit() {
    let (_file_work, file) = built(Provider::File);
    let (_sqlite_work, sqlite) = built(Provider::Sqlite);
    let mut before = Vec::new();
    for at in 0..=3 {
        let (first, _, first_summary) = read(&file, Some(at));
        let (second, _, second_summary) = read(&file, Some(at));
        assert_eq!(first, second, "two reads of revision {at}");
        assert_eq!(first_summary, second_summary, "revision {at}");
        let (other, _, _) = read(&sqlite, Some(at));
        assert_eq!(first, other, "the file and SQLite reads of revision {at}");
        before.push(first);
    }
    let (head, _, head_summary) = read(&file, None);
    assert_eq!(head_summary.revision, 3);
    assert_eq!(head, before[3], "no revision named reads the head, 3");

    for runtime in [&file, &sqlite] {
        fixtures::commit_later_quality(runtime);
        for (at, bytes) in before.iter().enumerate() {
            let (after, _, _) = read(runtime, Some(at as u64));
            assert_eq!(&after, bytes, "revision {at} after a later commit");
        }
        let (_, value, summary) = read(runtime, None);
        assert_eq!(summary.revision, 4);
        assert_eq!(value["assertions"]["active"], 5);
        assert_eq!(value["assertions"]["with_item_evidence"], 3);
    }
}

/// The pure half answers what the read answers from the same loaded revision and the seed's
/// evidence, and nothing else moves it: evidence the seed admitted is never item evidence.
#[test]
fn the_pure_half_answers_the_reads_bytes_from_the_loaded_revision_and_the_seeds_evidence() {
    let (_work, runtime) = built(Provider::Sqlite);
    let seed: std::collections::BTreeSet<ekr_core::EvidenceId> =
        [fixtures::Q_EVIDENCE, fixtures::Q_EVIDENCE + 1]
            .into_iter()
            .map(fixtures::id)
            .collect();
    for at in 0..=3 {
        let loaded = ekr_views::load(&runtime, Some(RevisionNumber::new(at))).expect("loads");
        let pure = ekr_views::quality(&loaded, &seed).expect("the pure half answers");
        let (bytes, value, _) = read(&runtime, Some(at));
        assert_eq!(pure.bytes, bytes, "revision {at}");
        assert_eq!(value["meta"]["format"], ekr_views::QUALITY_FORMAT);
    }
    // Were every evidence record the seed's, no assertion would cite item evidence.
    let loaded = ekr_views::load(&runtime, None).expect("loads");
    let all = loaded.graph.evidence.keys().copied().collect();
    let answer = ekr_views::quality(&loaded, &all).expect("the pure half answers");
    assert_eq!(answer.summary.with_item_evidence, 0);
    assert_eq!(answer.summary.active_assertions, 4);
}

#[test]
fn an_unseeded_store_and_a_revision_beyond_the_head_are_refused_by_name() {
    for provider in PROVIDERS {
        let work = tempfile::tempdir().expect("work directory");
        let unseeded = fixtures::open(work.path(), provider);
        assert!(matches!(
            report_quality(&unseeded, None),
            Err(ProjectError::NotSeeded { requested: None })
        ));
        assert!(matches!(
            report_quality(&unseeded, Some(RevisionNumber::new(2))),
            Err(ProjectError::NotSeeded { requested: Some(r) }) if r.get() == 2
        ));
        let (_work, runtime) = built(provider);
        assert!(matches!(
            report_quality(&runtime, Some(RevisionNumber::new(9))),
            Err(ProjectError::RevisionNotFound { requested, head })
                if requested.get() == 9 && head.get() == 3
        ));
    }
}

/// `story:evidence-attaches-to-a-held-assertion`: evidence attached to an assertion after it was
/// added counts as cited evidence does, in `with_item_evidence` and in `with_evidence`.
///
/// a3 cites only the seed's E1; at revision 4 the item statement X2 is attached to it. Read
/// through the store, a3 is item-evidenced from revision 4 on and not before. Through the pure
/// half, with E1's bytes taken out of what the store retains, a3 is evidenced only by the attached
/// X2: not counted in `with_evidence` at revision 3, counted at revision 4.
#[test]
fn attached_evidence_counts_in_both_evidence_figures() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        fixtures::commit_attachment_quality(&runtime);
        let (_, head, summary) = read(&runtime, None);
        assert_eq!(summary.revision, 4, "{provider:?}");
        assert_eq!(head["assertions"]["active"], 4, "{provider:?}");
        assert_eq!(head["assertions"]["with_item_evidence"], 4, "{provider:?}");
        let (_, before, _) = read(&runtime, Some(3));
        assert_eq!(
            before["assertions"]["with_item_evidence"], 3,
            "{provider:?}"
        );
    }

    let (_work, runtime) = built(Provider::Sqlite);
    fixtures::commit_attachment_quality(&runtime);
    let seed: std::collections::BTreeSet<ekr_core::EvidenceId> =
        [fixtures::Q_EVIDENCE, fixtures::Q_EVIDENCE + 1]
            .into_iter()
            .map(fixtures::id)
            .collect();
    let e1: ekr_core::EvidenceId = fixtures::id(fixtures::Q_EVIDENCE + 1);
    for (at, evidenced) in [(3, 3), (4, 4)] {
        let mut loaded = ekr_views::load(&runtime, Some(RevisionNumber::new(at))).expect("loads");
        let withheld = loaded.graph.evidence[&e1].content_hash;
        assert!(loaded.retained.remove(&withheld), "revision {at}");
        let answer = ekr_views::quality(&loaded, &seed).expect("the pure half answers");
        assert_eq!(answer.summary.active_assertions, 4, "revision {at}");
        assert_eq!(answer.summary.with_evidence, evidenced, "revision {at}");
    }
}
