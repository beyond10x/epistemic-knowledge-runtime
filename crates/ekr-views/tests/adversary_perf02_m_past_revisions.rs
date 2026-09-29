//! Adversary pass, wave perf-02, unit M (`story:session-keeps-only-the-head-graph`), acceptance
//! "reads of past revisions answer byte-identically".
//!
//! A session now keeps only the head graph (and the retained checkpoint's), and a read of any
//! other revision rebuilds it. On each provider, every revision of the evolved fixture is
//! projected by the session that built it while it was the head or close to it, then again after
//! nine later commits have moved the head past two checkpoints — by that session, by a fresh
//! open restored from the checkpoint (in ascending and descending order of revision), and by a
//! full replay from the seed — and every projection keeps its bytes.

mod support;

use std::path::Path;

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;

use support::fixtures::{self, Fixture, Provider};

const REVISIONS: u64 = 5;

/// Every revision's projection, by revision.
type Answers = Vec<(u64, Vec<u8>)>;

fn projection(runtime: &Runtime, at: u64) -> Vec<u8> {
    ekr_views::project(runtime, Some(RevisionNumber::new(at)))
        .unwrap_or_else(|error| panic!("revision {at}: {error}"))
        .bytes
}

fn every(runtime: &Runtime, order: impl Iterator<Item = u64>) -> Answers {
    let mut answers: Answers = order.map(|at| (at, projection(runtime, at))).collect();
    answers.sort_by_key(|(at, _)| *at);
    answers
}

fn past_revisions_keep_their_bytes(provider: Provider) {
    let directory = tempfile::tempdir().unwrap();
    let path: &Path = directory.path();
    let session = fixtures::open(path, provider);
    Fixture::Evolved.build(&session);
    let before = every(&session, 0..=REVISIONS);
    for n in 0..9 {
        fixtures::commit_unrelated(&session, n);
    }
    assert_eq!(
        session.head().unwrap().unwrap().revision.get(),
        REVISIONS + 9
    );
    let mut full = fixtures::reopen(path, provider);
    full.set_full_replay(true);
    let readers: [(&str, Answers); 4] = [
        (
            "the session, after nine later commits",
            every(&session, 0..=REVISIONS),
        ),
        (
            "a fresh open, ascending",
            every(&fixtures::reopen(path, provider), 0..=REVISIONS),
        ),
        (
            "a fresh open, descending",
            every(&fixtures::reopen(path, provider), (0..=REVISIONS).rev()),
        ),
        ("a full replay", every(&full, 0..=REVISIONS)),
    ];
    for (reader, answers) in readers {
        let changed: Vec<u64> = before
            .iter()
            .zip(&answers)
            .filter(|((_, one), (_, other))| one != other)
            .map(|((at, _), _)| *at)
            .collect();
        assert!(
            changed.is_empty(),
            "{} provider, {reader}: revisions {changed:?} changed bytes",
            provider.name()
        );
    }
}

#[test]
fn the_file_provider_projects_past_revisions_identically_warm_and_cold() {
    past_revisions_keep_their_bytes(Provider::File);
}

#[test]
fn the_sqlite_provider_projects_past_revisions_identically_warm_and_cold() {
    past_revisions_keep_their_bytes(Provider::Sqlite);
}
