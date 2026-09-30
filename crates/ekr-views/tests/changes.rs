//! `story:changes-since-read`: ChangesSince and its format ekr.graph-changes/1, held against
//! `systems/ekr/domains/views.yaml` on the `changes` fixture, whose changes per revision are
//! known, on both native providers:
//!
//! * `since` as a revision, a transaction time and a valid time lists exactly the changes the
//!   specification chooses, each with its revision, kind and evidence ids, in the format's order;
//! * a page limit and a cursor page through the change set with nothing lost or repeated;
//! * two reads of one (since, at) pair are byte-identical, before and after an unrelated commit,
//!   and a read naming no `at` is byte for byte the read naming the head it read;
//! * the refusals come in the declared order;
//! * on the `quality` fixture, an `AddEvidence` in the range is listed once as an EvidenceAdded
//!   (`task:changes-since-lists-added-evidence`), and a range without one answers the bytes it
//!   answered before that change kind existed.

mod support;

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use ekr_views::{
    ChangesError, ChangesListed, ChangesRequest, Index, LimitExceeded, ProjectError, SinceKind,
    SinceMalformed,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use support::fixtures::{
    self, statement_id, Fixture, Provider, ALPHA_NODE, BETA_NODE, CHANGED_EDGE, CHANGED_EDGE_CLAIM,
    CHANGED_NODE, CHANGED_NODE_CLAIM, CHANGED_NODE_VALID_MS, CLOCK_START_MS, LATER_CLAIM,
    LINKS_TYPE, Q_EVIDENCE, REPLACED_AT_MS, REPLACING_CLAIM, SEEDED_LINK, SEEDED_LINK_CLAIM,
    SEEDED_NODE_CLAIM, SEEDED_VALID_MS, SUBJECT_TYPE,
};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn evidence(statements: &[u64]) -> Value {
    json!(statements
        .iter()
        .map(|n| statement_id(*n).to_string())
        .collect::<Vec<_>>())
}

/// When revision `n` of a fixture was committed: see `fixtures::CLOCK_START_MS`.
fn committed(n: i64) -> i64 {
    CLOCK_START_MS + 1 + 3 * n
}

fn built(provider: Provider) -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    Fixture::Changes.build(&runtime);
    (work, runtime)
}

fn request(kind: SinceKind, since: i64, limit: Option<i64>, after: Option<i64>) -> ChangesRequest {
    ChangesRequest::new(kind, since, limit, after).expect("within bounds")
}

/// The answer of `request` at `at` (the head when `None`): its bytes, parsed, and its event.
fn read(
    runtime: &Runtime,
    at: Option<u64>,
    request: &ChangesRequest,
) -> (Vec<u8>, Value, ChangesListed) {
    let index = Index::load(runtime, at.map(RevisionNumber::new)).expect("the revision indexes");
    let answer = index
        .changes(runtime, request)
        .expect("the changes are read");
    let value: Value = serde_json::from_slice(&answer.bytes).expect("JSON");
    (answer.bytes, value, answer.summary)
}

// ---- the changes of each revision of `changes` ---------------------------------------------

fn seed_changes() -> Vec<Value> {
    let at = committed(0);
    vec![
        json!({"revision": 0, "recorded_at": at, "change": "NodeCreated", "id": uuid(ALPHA_NODE),
               "type": uuid(SUBJECT_TYPE), "name": "alpha", "evidence": evidence(&[0, 1])}),
        json!({"revision": 0, "recorded_at": at, "change": "NodeCreated", "id": uuid(BETA_NODE),
               "type": uuid(SUBJECT_TYPE), "name": "beta", "evidence": []}),
        json!({"revision": 0, "recorded_at": at, "change": "EdgeCreated", "id": uuid(SEEDED_LINK),
               "type": uuid(LINKS_TYPE), "source": uuid(ALPHA_NODE), "target": uuid(BETA_NODE),
               "evidence": evidence(&[0])}),
        json!({"revision": 0, "recorded_at": at, "change": "AssertionAdded",
               "id": uuid(SEEDED_NODE_CLAIM), "subject_kind": "Node", "subject": uuid(ALPHA_NODE),
               "valid_time": SEEDED_VALID_MS, "evidence": evidence(&[0, 1])}),
        json!({"revision": 0, "recorded_at": at, "change": "AssertionAdded",
               "id": uuid(SEEDED_LINK_CLAIM), "subject_kind": "Edge", "subject": uuid(SEEDED_LINK),
               "valid_time": SEEDED_VALID_MS, "evidence": evidence(&[0])}),
    ]
}

fn first_changes() -> Vec<Value> {
    let at = committed(1);
    vec![
        json!({"revision": 1, "recorded_at": at, "change": "NodeCreated", "id": uuid(CHANGED_NODE),
               "type": uuid(SUBJECT_TYPE), "name": "changed", "evidence": evidence(&[1])}),
        json!({"revision": 1, "recorded_at": at, "change": "EdgeCreated", "id": uuid(CHANGED_EDGE),
               "type": uuid(LINKS_TYPE), "source": uuid(CHANGED_NODE), "target": uuid(ALPHA_NODE),
               "evidence": evidence(&[0])}),
        json!({"revision": 1, "recorded_at": at, "change": "AssertionAdded",
               "id": uuid(CHANGED_NODE_CLAIM), "subject_kind": "Node", "subject": uuid(CHANGED_NODE),
               "valid_time": CHANGED_NODE_VALID_MS, "evidence": evidence(&[1])}),
        json!({"revision": 1, "recorded_at": at, "change": "AssertionAdded",
               "id": uuid(CHANGED_EDGE_CLAIM), "subject_kind": "Edge", "subject": uuid(CHANGED_EDGE),
               "evidence": evidence(&[0])}),
    ]
}

fn second_changes() -> Vec<Value> {
    let at = committed(2);
    vec![
        json!({"revision": 2, "recorded_at": at, "change": "AssertionAdded",
               "id": uuid(REPLACING_CLAIM), "subject_kind": "Node", "subject": uuid(ALPHA_NODE),
               "valid_time": REPLACED_AT_MS, "evidence": evidence(&[1])}),
        json!({"revision": 2, "recorded_at": at, "change": "AssertionSuperseded",
               "id": uuid(SEEDED_NODE_CLAIM), "subject_kind": "Node", "subject": uuid(ALPHA_NODE),
               "by": uuid(REPLACING_CLAIM), "valid_time": REPLACED_AT_MS,
               "evidence": evidence(&[0, 1])}),
    ]
}

fn third_changes() -> Vec<Value> {
    vec![
        json!({"revision": 3, "recorded_at": committed(3), "change": "AssertionRetracted",
               "id": uuid(CHANGED_NODE_CLAIM), "subject_kind": "Node", "subject": uuid(CHANGED_NODE),
               "valid_time": CHANGED_NODE_VALID_MS, "evidence": evidence(&[1])}),
    ]
}

fn concat(parts: &[Vec<Value>]) -> Vec<Value> {
    parts.iter().flatten().cloned().collect()
}

fn changes(value: &Value) -> Vec<Value> {
    value["changes"].as_array().expect("changes").clone()
}

// ---- since as a revision, a transaction time and a valid time -------------------------------

#[test]
fn a_revision_since_lists_exactly_the_changes_of_the_revisions_after_it() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let (text, value, summary) = read(
            &runtime,
            Some(3),
            &request(SinceKind::Revision, 0, None, None),
        );
        let expected = concat(&[first_changes(), second_changes(), third_changes()]);
        assert_eq!(changes(&value), expected, "{provider:?}");
        assert_eq!(
            value["meta"],
            json!({"format": ekr_views::CHANGES_FORMAT, "revision": 3, "since_revision": 0, "limit": 500,
                   "after": 0, "total": 7}),
            "{provider:?}"
        );
        assert!(value.get("next").is_none(), "{provider:?}: {value}");
        assert_eq!(value["remaining"], 0);
        // Key order is read off the bytes: rule (2), declared order, and no `next`.
        let text = String::from_utf8(text.clone()).expect("UTF-8");
        assert!(
            text.starts_with(
                "{\"meta\":{\"format\":\"ekr.graph-changes/1\",\"revision\":3,\"since_revision\":0,\
                 \"limit\":500,\"after\":0,\"total\":7},\"changes\":[{\"revision\":1,\
                 \"recorded_at\":"
            ),
            "{text}"
        );
        assert!(text.ends_with("}],\"remaining\":0}"), "{text}");
        let created = &text[text.find("\"change\":\"EdgeCreated\"").expect("an edge")..];
        assert!(
            created.starts_with(&format!(
                "\"change\":\"EdgeCreated\",\"id\":\"{}\",\"type\":\"{}\",\"source\":\"{}\",\
                 \"target\":\"{}\",\"evidence\":[\"{}\"]}}",
                uuid(CHANGED_EDGE),
                uuid(LINKS_TYPE),
                uuid(CHANGED_NODE),
                uuid(ALPHA_NODE),
                statement_id(0)
            )),
            "{created}"
        );
        assert_eq!(
            summary,
            ChangesListed {
                revision: 3,
                after: 0,
                changes: 7,
                total: 7,
                remaining: 0,
                nodes_created: 1,
                edges_created: 1,
                assertions_added: 3,
                assertions_superseded: 1,
                assertions_retracted: 1,
                evidence_added: 0,
                first_revision: Some(1),
                last_revision: Some(3),
                changes_hash: hex::encode(Sha256::digest(&text)),
            }
        );

        let (_, later, _) = read(
            &runtime,
            Some(3),
            &request(SinceKind::Revision, 1, None, None),
        );
        assert_eq!(
            changes(&later),
            concat(&[second_changes(), third_changes()])
        );
        let (_, upto, _) = read(
            &runtime,
            Some(2),
            &request(SinceKind::Revision, 0, None, None),
        );
        assert_eq!(changes(&upto), concat(&[first_changes(), second_changes()]));
        // A since at or after `at` that the store holds chooses nothing.
        for (since, at) in [(3, 3), (3, 2), (1, 1)] {
            let (_, none, summary) = read(
                &runtime,
                Some(at),
                &request(SinceKind::Revision, since, None, None),
            );
            assert_eq!(changes(&none), Vec::<Value>::new(), "since {since} at {at}");
            assert_eq!((summary.total, summary.first_revision), (0, None));
        }
    }
}

#[test]
fn a_transaction_time_since_lists_the_changes_of_the_revisions_committed_after_it() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let at = |since: i64| {
            let (_, value, _) = read(
                &runtime,
                Some(3),
                &request(SinceKind::TransactionTime, since, None, None),
            );
            assert_eq!(value["meta"]["since_recorded"], since);
            assert!(value["meta"].get("since_revision").is_none());
            changes(&value)
        };
        let everything = concat(&[
            seed_changes(),
            first_changes(),
            second_changes(),
            third_changes(),
        ]);
        assert_eq!(
            at(CLOCK_START_MS),
            everything,
            "{provider:?}: before the seed"
        );
        assert_eq!(at(committed(0) - 1), everything, "{provider:?}");
        assert_eq!(
            at(committed(0)),
            everything[5..].to_vec(),
            "{provider:?}: at the seed"
        );
        assert_eq!(
            at(committed(1)),
            concat(&[second_changes(), third_changes()]),
            "{provider:?}: committed_at > T, not >="
        );
        assert_eq!(at(committed(3)), Vec::<Value>::new(), "{provider:?}");
    }
}

#[test]
fn a_valid_time_since_lists_the_assertion_changes_valid_after_it() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let at = |since: i64| {
            let (_, value, _) = read(
                &runtime,
                Some(3),
                &request(SinceKind::ValidTime, since, None, None),
            );
            assert_eq!(value["meta"]["since_valid"], since);
            changes(&value)
        };
        let seeded = seed_changes();
        let first = first_changes();
        let second = second_changes();
        let third = third_changes();
        // No node or edge, and not the edge claim, which has no valid time.
        assert_eq!(
            at(0),
            vec![
                seeded[3].clone(),
                seeded[4].clone(),
                first[2].clone(),
                second[0].clone(),
                second[1].clone(),
                third[0].clone(),
            ],
            "{provider:?}"
        );
        assert_eq!(
            at(SEEDED_VALID_MS),
            at(1_000),
            "{provider:?}: valid_time > T, not >="
        );
        assert_eq!(
            at(1_000),
            vec![
                first[2].clone(),
                second[0].clone(),
                second[1].clone(),
                third[0].clone()
            ],
            "{provider:?}"
        );
        assert_eq!(at(2_500), second, "{provider:?}");
        assert_eq!(at(REPLACED_AT_MS), Vec::<Value>::new(), "{provider:?}");
    }
}

// ---- paging -----------------------------------------------------------------------------------

#[test]
fn a_limit_and_a_cursor_page_through_the_changes_with_nothing_lost_or_repeated() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let whole = |limit: Option<i64>, after: Option<i64>| {
            let (_, value, summary) = read(
                &runtime,
                Some(3),
                &request(SinceKind::TransactionTime, CLOCK_START_MS, limit, after),
            );
            (value, summary)
        };
        let (all, _) = whole(None, None);
        let all = changes(&all);
        assert_eq!(all.len(), 12);
        for limit in 1..=13_i64 {
            let mut paged = Vec::new();
            let mut after = 0_i64;
            loop {
                let (page, summary) = whole(Some(limit), Some(after));
                let listed = changes(&page);
                assert!(listed.len() as i64 <= limit);
                assert_eq!(page["meta"]["after"], after);
                assert_eq!(page["meta"]["limit"], limit);
                assert_eq!(page["meta"]["total"], 12);
                let end = after + listed.len() as i64;
                assert_eq!(summary.remaining, (12 - end) as u64);
                assert_eq!(page["remaining"], 12 - end);
                paged.extend(listed);
                match page.get("next") {
                    Some(next) => {
                        assert_eq!(*next, json!(end), "next is after plus the page");
                        after = end;
                    }
                    None => {
                        assert_eq!(end, 12, "no next exactly at the end");
                        break;
                    }
                }
            }
            assert_eq!(paged, all, "{provider:?}: limit {limit}");
        }
        for after in [12, 13, 1_000] {
            let (page, summary) = whole(Some(5), Some(after));
            assert_eq!(changes(&page), Vec::<Value>::new());
            assert!(page.get("next").is_none());
            assert_eq!((summary.remaining, summary.changes), (0, 0));
        }
    }
}

// ---- determinism ------------------------------------------------------------------------------

#[test]
fn two_reads_of_one_since_and_at_are_byte_identical_before_and_after_an_unrelated_commit() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let requests = [
            request(SinceKind::Revision, 0, None, None),
            request(SinceKind::TransactionTime, CLOCK_START_MS, Some(4), Some(3)),
            request(SinceKind::ValidTime, 1_000, None, None),
        ];
        let before: Vec<Vec<u8>> = requests
            .iter()
            .map(|asked| read(&runtime, Some(3), asked).0)
            .collect();
        for (asked, bytes) in requests.iter().zip(&before) {
            assert_eq!(
                &read(&runtime, Some(3), asked).0,
                bytes,
                "{provider:?}: twice"
            );
            assert_eq!(
                &read(&runtime, None, asked).0,
                bytes,
                "{provider:?}: no `at` reads the head, and answers as naming it"
            );
        }
        fixtures::commit_later_change(&runtime);
        for (asked, bytes) in requests.iter().zip(&before) {
            assert_eq!(
                &read(&runtime, Some(3), asked).0,
                bytes,
                "{provider:?}: after a commit"
            );
        }
        let (_, head, _) = read(&runtime, None, &requests[0]);
        assert_eq!(head["meta"]["revision"], 4);
        let last = changes(&head).pop().expect("a change");
        assert_eq!(last["id"], uuid(LATER_CLAIM));
        assert_eq!(last["revision"], 4);
    }
}

// ---- refusals ---------------------------------------------------------------------------------

#[test]
fn the_refusals_come_in_the_declared_order() {
    let malformed = ChangesRequest::new(SinceKind::Revision, -1, Some(0), Some(-1));
    assert!(
        matches!(
            malformed,
            Err(ChangesError::SinceMalformed(SinceMalformed {
                kind: SinceKind::Revision,
                requested: -1
            }))
        ),
        "a revision below 0 before any bound: {malformed:?}"
    );
    for (limit, after, parameter, requested, maximum) in [
        (Some(0), None, "limit", 0, Some(2_000)),
        (Some(2_001), Some(-1), "limit", 2_001, Some(2_000)),
        (None, Some(-1), "after", -1, None),
    ] {
        let refused = ChangesRequest::new(SinceKind::ValidTime, -5, limit, after);
        let Err(ChangesError::LimitExceeded(LimitExceeded {
            parameter: named,
            requested: value,
            minimum,
            maximum: most,
        })) = refused
        else {
            panic!("{limit:?} {after:?}: {refused:?}");
        };
        assert_eq!((named, value, most), (parameter, requested, maximum));
        assert_eq!(minimum, if parameter == "limit" { 1 } else { 0 });
    }
    for (limit, after) in [(Some(1), Some(0)), (Some(2_000), None)] {
        ChangesRequest::new(SinceKind::TransactionTime, -5, limit, after).expect("in bounds");
    }
    let asked = request(SinceKind::Revision, 1, None, None);
    assert_eq!(
        (asked.kind(), asked.since(), asked.limit(), asked.after()),
        (SinceKind::Revision, 1, 500, 0)
    );
    assert_eq!(ChangesRequest::DEFAULT_LIMIT, 500);
    assert_eq!(ChangesRequest::MAX_LIMIT, 2_000);

    let (_work, runtime) = built(Provider::File);
    let index = Index::load(&runtime, Some(RevisionNumber::new(3))).expect("revision 3");
    match index.changes(&runtime, &request(SinceKind::Revision, 9, None, None)) {
        Err(ChangesError::Project(ProjectError::RevisionNotFound { requested, head })) => {
            assert_eq!((requested.get(), head.get()), (9, 3));
        }
        other => panic!("a since beyond the head: {other:?}"),
    }
    let unseeded = tempfile::tempdir().expect("work directory");
    let empty = fixtures::open(unseeded.path(), Provider::File);
    assert!(matches!(
        Index::load(&empty, None),
        Err(ProjectError::NotSeeded { requested: None })
    ));
    assert!(matches!(
        Index::load(&runtime, Some(RevisionNumber::new(4))),
        Err(ProjectError::RevisionNotFound { .. })
    ));
}

#[test]
fn a_host_takes_since_as_exactly_one_of_three_inputs() {
    assert_eq!(
        SinceKind::one_of(Some(1), None, None),
        Ok((SinceKind::Revision, 1))
    );
    assert_eq!(
        SinceKind::one_of(None, Some(-2), None),
        Ok((SinceKind::ValidTime, -2))
    );
    assert_eq!(
        SinceKind::one_of(None, None, Some(7)),
        Ok((SinceKind::TransactionTime, 7))
    );
    assert_eq!(SinceKind::one_of(None, None, None), Err(Vec::new()));
    assert_eq!(
        SinceKind::one_of(Some(1), None, Some(2)),
        Err(vec!["since_revision", "since_recorded"])
    );
    assert_eq!(
        SinceKind::one_of(Some(1), Some(1), Some(2)),
        Err(vec!["since_revision", "since_valid", "since_recorded"])
    );
    assert_eq!(
        [
            SinceKind::Revision,
            SinceKind::ValidTime,
            SinceKind::TransactionTime
        ]
        .map(SinceKind::name),
        ["Revision", "ValidTime", "TransactionTime"]
    );
}

// ---- evidence added after the seed (task:changes-since-lists-added-evidence) -------------------

fn quality(provider: Provider) -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    Fixture::Quality.build(&runtime);
    (work, runtime)
}

/// The EvidenceAdded change of the `quality` store's `AddEvidence` of evidence `n` at `revision`:
/// its id, its source identity (a human statement by `operator`) and its payload's hash.
fn evidence_added(revision: i64, n: u64) -> Value {
    let payload = format!("item statement {n:x}").into_bytes();
    json!({"revision": revision, "recorded_at": committed(revision), "change": "EvidenceAdded",
           "id": uuid(n), "locator": "operator",
           "content_hash": ekr_core::ContentHash::of_bytes(&payload).to_string(),
           "evidence": []})
}

/// The `quality` store adds X0 at revision 1 and X1 at revision 3, each with one `AddEvidence`.
/// A range holding one lists it once, as its revision's last change, on both providers; a
/// revision since, a transaction-time since and a page-by-page read agree, and a valid-time
/// since chooses no evidence, which has no valid time.
#[test]
fn an_add_evidence_in_the_range_is_listed_once_with_its_revision_source_and_payload_hash() {
    let (x0, x1) = (Q_EVIDENCE + 0x10, Q_EVIDENCE + 0x11);
    for provider in PROVIDERS {
        let (_work, runtime) = quality(provider);
        let (_, value, _) = read(
            &runtime,
            Some(3),
            &request(SinceKind::Revision, 0, None, None),
        );
        let listed = changes(&value);
        let added: Vec<Value> = listed
            .iter()
            .filter(|change| change["change"] == "EvidenceAdded")
            .cloned()
            .collect();
        assert_eq!(
            added,
            vec![evidence_added(1, x0), evidence_added(3, x1)],
            "{provider:?}: {value}"
        );
        assert_eq!(value["meta"]["total"], 8, "{provider:?}: {value}");
        for (revision, n) in [(1, x0), (3, x1)] {
            let last = listed
                .iter()
                .rev()
                .find(|change| change["revision"] == revision)
                .expect("a change of the revision");
            assert_eq!(*last, evidence_added(revision, n), "{provider:?}");
        }

        let mut paged = Vec::new();
        let mut after = 0;
        loop {
            let (_, page, _) = read(
                &runtime,
                Some(3),
                &request(SinceKind::Revision, 0, Some(1), Some(after)),
            );
            paged.extend(changes(&page));
            match page["next"].as_i64() {
                Some(next) => after = next,
                None => break,
            }
        }
        assert_eq!(paged, listed, "{provider:?}: page by page");

        let (_, recorded, _) = read(
            &runtime,
            Some(3),
            &request(SinceKind::TransactionTime, committed(0), None, None),
        );
        assert_eq!(
            changes(&recorded),
            listed,
            "{provider:?}: since the seed's time"
        );
        let (_, later, _) = read(
            &runtime,
            Some(3),
            &request(SinceKind::Revision, 1, None, None),
        );
        assert_eq!(
            changes(&later)
                .into_iter()
                .filter(|change| change["change"] == "EvidenceAdded")
                .collect::<Vec<_>>(),
            vec![evidence_added(3, x1)],
            "{provider:?}"
        );
        let (_, valid, _) = read(
            &runtime,
            Some(3),
            &request(SinceKind::ValidTime, i64::MIN, None, None),
        );
        assert!(
            changes(&valid)
                .iter()
                .all(|change| change["change"] != "EvidenceAdded"),
            "{provider:?}: {valid}"
        );
    }
}

/// A range without an `AddEvidence` answers the bytes it answered before EvidenceAdded existed:
/// each hash below was read off the base `197d93d9`, on both providers. The seed's evidence is
/// no change, so a range holding the seed is one of them, and a valid-time since chooses no
/// evidence, so it answers as before even over revisions that add some.
#[test]
fn a_range_without_an_add_evidence_answers_the_bytes_it_answered_before() {
    let pinned: [(Fixture, u64, SinceKind, i64, &str); 7] = [
        (
            Fixture::Changes,
            3,
            SinceKind::Revision,
            0,
            "c41dc12fa36c9033d9ae18572a763d8821f9b2d6f6a3402ac9c81d5adc6c4a38",
        ),
        (
            Fixture::Changes,
            3,
            SinceKind::TransactionTime,
            i64::MIN,
            "cce7e30bb6997ddbda987ae1b03ee81fc2cbc9fdc59eb03f78128e9a82dd30e9",
        ),
        (
            Fixture::Changes,
            3,
            SinceKind::ValidTime,
            i64::MIN,
            "1a74499b39bf27de50a3745fbcfa01eba5f95a9e0a6244dc7695d18ecaddf92d",
        ),
        (
            Fixture::Quality,
            0,
            SinceKind::TransactionTime,
            i64::MIN,
            "280400782b5ab177e75c6e3aae677bdffedbc3f153c7290d93ea675e0ef2a3b1",
        ),
        (
            Fixture::Quality,
            2,
            SinceKind::Revision,
            1,
            "ea7c85d2c77e88d2370eeefa9eefaff014b6a427b98d7c3caa168789e9a8d039",
        ),
        (
            Fixture::Quality,
            3,
            SinceKind::Revision,
            3,
            "ae4b2ce00fcf9bafc3acf82e5b229366cc47184acbbd4bcfb7b45847e4a309b4",
        ),
        (
            Fixture::Quality,
            3,
            SinceKind::ValidTime,
            i64::MIN,
            "f8804ee5bb407a368a54d75af21867a49dd3a567fc78e8cb670ecefca7810515",
        ),
    ];
    for provider in PROVIDERS {
        let mut read_now = Vec::new();
        for (fixture, at, kind, since, _) in pinned {
            let work = tempfile::tempdir().expect("work directory");
            let runtime = fixtures::open(work.path(), provider);
            fixture.build(&runtime);
            let (bytes, _, summary) = read(&runtime, Some(at), &request(kind, since, None, None));
            assert_eq!(summary.changes_hash, hex::encode(Sha256::digest(&bytes)));
            read_now.push(summary.changes_hash);
        }
        let expected: Vec<&str> = pinned.iter().map(|(.., hash)| *hash).collect();
        assert_eq!(read_now, expected, "{provider:?}");
    }
}
