//! `story:changes-since-by-time-within-deadline`: a `ChangesSince` by recorded time
//! (`since_recorded`, [`SinceKind::TransactionTime`]) that chooses the same revisions as one by
//! revision (`since_revision`) costs no more reads than it, on both providers.
//!
//! Cost is counted, never timed (`AGENTS.md`): the store's provider reads on this thread
//! (`ekr_kernel::read_work`, `ekr_kernel::stream_reads`), the kernel's replays from the seed and
//! seed-envelope decodes, the graphs it applied, and the changes the read built before paging
//! (`meta.total`). Each read runs on a freshly reopened runtime with its index already loaded, so
//! a cache one read warmed is never another's saving, and only the `changes` call is counted.
//!
//! The probes vary one input each: the window's size (1, 10 and every revision after the seed),
//! and a recorded time before the seed, which is the only `since` of either kind that chooses the
//! seed — no `since_revision` does, revision 0 being the least and choosing the revisions after
//! it.

mod support;

use std::collections::BTreeMap;
use std::path::Path;

use ekr_kernel::Runtime;
use ekr_views::{ChangesRequest, Index, SinceKind};
use serde_json::Value;
use support::fixtures::{self, Provider};

/// Revisions committed after the seed, each adding one retained statement.
const EXTRA: u64 = 40;

/// What one `changes` call cost, by name.
type Cost = BTreeMap<&'static str, u64>;

fn request(kind: SinceKind, since: i64) -> ChangesRequest {
    ChangesRequest::new(kind, since, Some(ChangesRequest::MAX_LIMIT), None).expect("in bounds")
}

/// One `changes` call on a fresh open of the store at `path`: what it cost, and the changes it
/// listed.
fn measure(path: &Path, provider: Provider, request: &ChangesRequest) -> (Cost, Vec<Value>) {
    let runtime: Runtime = fixtures::reopen(path, provider);
    let index = Index::load(&runtime, None).expect("the head indexes");
    let _ = (ekr_kernel::read_work(), ekr_kernel::stream_reads());
    let replays = runtime.seed_replays();
    let envelopes = runtime.seed_envelope_decodes();
    let applied = ekr_kernel::graphs_applied();
    let answer = index
        .changes(&runtime, request)
        .expect("the changes are read");
    let (work, streams) = (ekr_kernel::read_work(), ekr_kernel::stream_reads());
    let cost = Cost::from([
        ("blobs read", work.blobs_read),
        ("revision occurrences read", work.occurrences_read),
        ("revision-stream reads", streams.revision),
        ("checkpoint-stream reads", streams.checkpoint),
        ("object-stream reads", streams.object),
        ("tenant-log reads", streams.feed),
        ("tenant captures", streams.captures),
        ("replays from the seed", runtime.seed_replays() - replays),
        (
            "seed envelopes decoded",
            runtime.seed_envelope_decodes() - envelopes,
        ),
        ("graphs applied", ekr_kernel::graphs_applied() - applied),
        ("changes built", answer.summary.total),
    ]);
    let value: Value = serde_json::from_slice(&answer.bytes).expect("JSON");
    let changes = value["changes"].as_array().expect("changes").clone();
    (cost, changes)
}

/// Each revision's recorded time, read off a listing of every change by recorded time.
fn recorded_at(path: &Path, provider: Provider) -> BTreeMap<u64, i64> {
    let (_, changes) = measure(path, provider, &request(SinceKind::TransactionTime, 0));
    changes
        .iter()
        .map(|change| {
            (
                change["revision"].as_u64().expect("revision"),
                change["recorded_at"].as_i64().expect("recorded_at"),
            )
        })
        .collect()
}

/// The kinds `by_time` cost more of than `by_revision`, with both counts.
fn dearer(by_time: &Cost, by_revision: &Cost) -> Vec<(&'static str, u64, u64)> {
    by_time
        .iter()
        .filter(|(kind, count)| **count > by_revision[*kind])
        .map(|(kind, count)| (*kind, *count, by_revision[kind]))
        .collect()
}

fn a_time_since_costs_what_the_revision_since_choosing_its_revisions_costs(provider: Provider) {
    let directory = tempfile::tempdir().expect("work directory");
    let path = directory.path();
    fixtures::build_evidenced(&fixtures::open(path, provider), EXTRA);
    let head = EXTRA;
    let times = recorded_at(path, provider);
    assert_eq!(
        times.len() as u64,
        head + 1,
        "{provider:?}: a recorded time before every commit lists every revision, seed included"
    );
    let mut failures = Vec::new();
    for window in [1, 10, head] {
        let since = head - window;
        let first = since + 1;
        let by_revision = request(SinceKind::Revision, i64::try_from(since).expect("small"));
        // Revisions are committed at strictly increasing times, so one millisecond before the
        // first chosen revision's is at or after the revision before it.
        let by_time = request(SinceKind::TransactionTime, times[&first] - 1);
        let (revision_cost, revision_changes) = measure(path, provider, &by_revision);
        let (time_cost, time_changes) = measure(path, provider, &by_time);
        println!(
            "{provider:?}, revisions {first}..={head}:\n  since_revision {since}: \
             {revision_cost:?}\n  since_recorded {}: {time_cost:?}",
            times[&first] - 1
        );
        assert_eq!(
            time_changes, revision_changes,
            "{provider:?}: the two sinces choose revisions {first}..={head} alike"
        );
        let dearer = dearer(&time_cost, &revision_cost);
        if !dearer.is_empty() {
            failures.push(format!(
                "revisions {first}..={head}: (kind, by time, by revision) {dearer:?}"
            ));
        }
    }
    // The probe that varies the window past the seed: diagnostic, printed and not asserted, since
    // no revision since chooses what it chooses.
    let (seed_cost, seed_changes) = measure(
        path,
        provider,
        &request(SinceKind::TransactionTime, times[&0] - 1),
    );
    let (all_cost, all_changes) = measure(path, provider, &request(SinceKind::Revision, 0));
    println!(
        "{provider:?}, the seed chosen too:\n  since_revision 0 (revisions 1..={head}): \
         {all_cost:?} ({} changes)\n  since_recorded before the seed (0..={head}): {seed_cost:?} \
         ({} changes)",
        all_changes.len(),
        seed_changes.len()
    );
    assert!(
        failures.is_empty(),
        "{provider:?}: a since_recorded choosing the revisions a since_revision chooses cost more:\n{}",
        failures.join("\n")
    );
}

#[test]
fn on_the_file_provider_a_time_since_costs_no_more_than_the_revision_since() {
    a_time_since_costs_what_the_revision_since_choosing_its_revisions_costs(Provider::File);
}

#[test]
fn on_the_sqlite_provider_a_time_since_costs_no_more_than_the_revision_since() {
    a_time_since_costs_what_the_revision_since_choosing_its_revisions_costs(Provider::Sqlite);
}
