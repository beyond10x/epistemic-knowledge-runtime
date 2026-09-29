//! Adversary probes for `story:changes-since-read` (wave read-03, unit C): the boundaries of
//! `since`, `at` and the cursor on the `changes` fixture, and byte equality between the two native
//! providers, which `tests/changes.rs` compares only through parsed JSON per provider.

mod support;

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use ekr_views::{ChangesError, ChangesRequest, Index, ProjectError, SinceKind};
use serde_json::Value;

use support::fixtures::{
    self, Fixture, Provider, CHANGED_NODE_VALID_MS, CLOCK_START_MS, REPLACED_AT_MS,
};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

fn built(provider: Provider) -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    Fixture::Changes.build(&runtime);
    (work, runtime)
}

fn asked(kind: SinceKind, since: i64, limit: Option<i64>, after: Option<i64>) -> ChangesRequest {
    ChangesRequest::new(kind, since, limit, after).expect("within bounds")
}

fn answer(
    runtime: &Runtime,
    at: Option<u64>,
    request: &ChangesRequest,
) -> Result<Vec<u8>, ChangesError> {
    let index = Index::load(runtime, at.map(RevisionNumber::new)).expect("the revision indexes");
    index.changes(runtime, request).map(|answer| answer.bytes)
}

fn parsed(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("JSON")
}

fn ids(value: &Value) -> Vec<(u64, String, String)> {
    value["changes"]
        .as_array()
        .expect("changes")
        .iter()
        .map(|change| {
            (
                change["revision"].as_u64().expect("revision"),
                change["change"].as_str().expect("change").to_owned(),
                change["id"].as_str().expect("id").to_owned(),
            )
        })
        .collect()
}

/// When revision `n` of a fixture was committed.
fn committed(n: i64) -> i64 {
    CLOCK_START_MS + 1 + 3 * n
}

fn requests() -> Vec<(Option<u64>, ChangesRequest)> {
    let mut all = Vec::new();
    for at in [None, Some(0), Some(1), Some(2), Some(3)] {
        for since in [0, 1, 2, 3] {
            all.push((at, asked(SinceKind::Revision, since, None, None)));
        }
        for since in [i64::MIN, 0, 500, 1_999, 2_000, 2_999, 3_000, i64::MAX] {
            all.push((at, asked(SinceKind::ValidTime, since, None, None)));
        }
        for since in [
            i64::MIN,
            committed(0) - 1,
            committed(0),
            committed(1),
            committed(2),
            i64::MAX,
        ] {
            all.push((
                at,
                asked(SinceKind::TransactionTime, since, Some(3), Some(2)),
            ));
        }
    }
    all
}

#[test]
fn the_file_and_sqlite_providers_answer_the_same_bytes_for_every_request() {
    let (_file_work, file) = built(Provider::File);
    let (_sqlite_work, sqlite) = built(Provider::Sqlite);
    for (at, request) in requests() {
        let on_file = answer(&file, at, &request).expect("file answers");
        let on_sqlite = answer(&sqlite, at, &request).expect("sqlite answers");
        assert_eq!(
            String::from_utf8_lossy(&on_file),
            String::from_utf8_lossy(&on_sqlite),
            "at {at:?} {request:?}"
        );
    }
}

#[test]
fn since_equal_to_or_after_at_chooses_nothing_and_past_the_head_is_refused() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        for at in 0..=3_u64 {
            for since in at..=3 {
                let since = i64::try_from(since).expect("small");
                let bytes = answer(
                    &runtime,
                    Some(at),
                    &asked(SinceKind::Revision, since, None, None),
                )
                .expect("a since the store holds is no refusal");
                let value = parsed(&bytes);
                assert_eq!(
                    value["meta"]["total"], 0,
                    "{provider:?} since {since} at {at}"
                );
                assert_eq!(value["meta"]["revision"], at);
                assert!(value.get("next").is_none());
            }
            for since in [4, 5, i64::MAX] {
                match answer(
                    &runtime,
                    Some(at),
                    &asked(SinceKind::Revision, since, None, None),
                ) {
                    Err(ChangesError::Project(ProjectError::RevisionNotFound {
                        requested,
                        head,
                    })) => {
                        assert_eq!(
                            (requested.get(), head.get()),
                            (since.unsigned_abs(), 3),
                            "{provider:?}"
                        );
                    }
                    other => panic!("{provider:?} since {since} at {at}: {other:?}"),
                }
            }
        }
        // A transaction time or valid time past everything is no refusal.
        for kind in [SinceKind::TransactionTime, SinceKind::ValidTime] {
            let value = parsed(
                &answer(&runtime, Some(3), &asked(kind, i64::MAX, None, None)).expect("answered"),
            );
            assert_eq!(value["meta"]["total"], 0, "{provider:?} {kind:?}");
        }
    }
}

#[test]
fn a_cursor_at_the_last_change_and_at_a_revision_boundary_pages_exactly() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let page = |limit: i64, after: i64| {
            parsed(
                &answer(
                    &runtime,
                    Some(3),
                    &asked(
                        SinceKind::TransactionTime,
                        CLOCK_START_MS,
                        Some(limit),
                        Some(after),
                    ),
                )
                .expect("answered"),
            )
        };
        let whole = ids(&page(2_000, 0));
        assert_eq!(whole.len(), 12);
        // The last change alone.
        let last = page(1, 11);
        assert_eq!(ids(&last), whole[11..].to_vec(), "{provider:?}");
        assert!(last.get("next").is_none(), "{provider:?}: {last}");
        assert_eq!(last["remaining"], 0);
        // The seed holds positions 0 to 4; a page starting at 5 starts revision 1 exactly.
        let boundary = page(4, 5);
        assert_eq!(ids(&boundary), whole[5..9].to_vec(), "{provider:?}");
        assert!(ids(&boundary).iter().all(|(revision, ..)| *revision == 1));
        assert_eq!(boundary["next"], 9);
        assert_eq!(boundary["remaining"], 3);
        // A page that ends on the seed's last change leaves revision 1 whole for the next.
        let seed = page(5, 0);
        assert_eq!(ids(&seed), whole[..5].to_vec());
        assert_eq!(seed["next"], 5);
        // A forged cursor far past the end.
        let forged = page(2_000, i64::MAX);
        assert_eq!(ids(&forged), Vec::new());
        assert!(forged.get("next").is_none());
        assert_eq!(forged["meta"]["after"], i64::MAX);
        assert_eq!(forged["meta"]["total"], 12);
    }
}

#[test]
fn a_valid_time_since_reads_a_supersession_only_from_the_revisions_up_to_at() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let at = |at: u64, since: i64| {
            ids(&parsed(
                &answer(
                    &runtime,
                    Some(at),
                    &asked(SinceKind::ValidTime, since, None, None),
                )
                .expect("answered"),
            ))
        };
        // Before revision 2 nothing is valid after 2,500.
        assert_eq!(at(1, 2_500), Vec::new(), "{provider:?}");
        // At 2 the replacing claim and the supersession, whose effective_from is 3,000.
        let straddle = at(2, 2_500);
        assert_eq!(
            straddle
                .iter()
                .map(|(revision, change, _)| (*revision, change.as_str()))
                .collect::<Vec<_>>(),
            vec![(2, "AssertionAdded"), (2, "AssertionSuperseded")],
            "{provider:?}"
        );
        assert_eq!(
            at(3, 2_500),
            straddle,
            "{provider:?}: a retraction valid from 2,000"
        );
        // valid_time > T is strict at both valid instants.
        assert_eq!(at(3, REPLACED_AT_MS - 1), straddle);
        assert_eq!(at(3, CHANGED_NODE_VALID_MS), straddle);
        assert_eq!(at(3, CHANGED_NODE_VALID_MS - 1).len(), 4);
    }
}

#[test]
fn every_limit_from_one_to_the_most_pages_the_same_sequence() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let whole = ids(&parsed(
            &answer(
                &runtime,
                None,
                &asked(SinceKind::ValidTime, i64::MIN, Some(2_000), None),
            )
            .expect("answered"),
        ));
        assert_eq!(whole.len(), 6);
        for limit in [1_i64, 2, 5, 6, 7, 2_000] {
            let mut paged = Vec::new();
            let mut after = Some(0_i64);
            while let Some(from) = after {
                let value = parsed(
                    &answer(
                        &runtime,
                        Some(3),
                        &asked(SinceKind::ValidTime, i64::MIN, Some(limit), Some(from)),
                    )
                    .expect("answered"),
                );
                paged.extend(ids(&value));
                after = value.get("next").map(|next| next.as_i64().expect("next"));
            }
            assert_eq!(paged, whole, "{provider:?}: limit {limit}");
        }
    }
}

/// docs/cli.md: a revision is loaded once, and `/changes` "replays the seed only when it chooses
/// the seed". A read that does not choose the seed begins no replay at the seed, however often it
/// is asked; one that does begins at most one per read.
#[test]
fn a_read_that_does_not_choose_the_seed_begins_no_replay_at_the_seed() {
    for provider in PROVIDERS {
        let (work, built) = built(provider);
        drop(built);
        let runtime = fixtures::reopen(work.path(), provider);
        let index = Index::load(&runtime, Some(RevisionNumber::new(3))).expect("revision 3");
        let before = runtime.seed_replays();
        for _ in 0..5 {
            index
                .changes(&runtime, &asked(SinceKind::Revision, 0, None, None))
                .expect("answered");
        }
        assert_eq!(
            runtime.seed_replays() - before,
            0,
            "{provider:?}: five revision-since reads"
        );
        let before = runtime.seed_replays();
        for _ in 0..5 {
            index
                .changes(
                    &runtime,
                    &asked(SinceKind::TransactionTime, CLOCK_START_MS, None, None),
                )
                .expect("answered");
        }
        assert!(
            runtime.seed_replays() - before <= 5,
            "{provider:?}: five seed-choosing reads began {} replays",
            runtime.seed_replays() - before
        );
    }
}
