//! `story:graph-projection-renderer`, acceptance: on each of the file and SQLite providers, one
//! revision of a seeded, schema-evolved fixture store is rendered twice in one process and once
//! more in a fresh process after reopening; the three renders are byte-identical, and the
//! renderer passes the committed `ekr-views` suite with no skipped or unsupported scenario — its
//! answer for the suite's own `store` at that revision carrying the same projection hash.
//!
//! The fresh process is this test binary run again with [`CHILD`] set, selecting only
//! [`render_in_a_child_process`], which reopens the provider without creating anything, renders
//! and writes the bytes. Unset, that case returns at once; the parent refuses a child run that
//! wrote nothing, so a filter that selected no case cannot pass for a render.
//!
//! `task:historical-projection-carries-the-head`: on each provider, every revision of the same
//! store is answered — projected, and read through all five bounded formats — before and after
//! an unrelated commit, and every answer keeps its bytes.

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;

use ekr_core::RevisionNumber;
use ekr_views::{
    ExpandRequest, Index, IndexCache, OverviewRequest, SearchRequest, TimelineRequest,
};
use sha2::{Digest, Sha256};

use support::fixtures::{self, Fixture, Provider};
use support::suite::passes_every_admitted_scenario;

/// `<provider> <store directory> <revision> <output file>`, tab-separated.
const CHILD: &str = "EKR_VIEWS_RENDER_CHILD";

/// Revision 1 of the six-revision store: a past revision, one schema change in, so the render
/// is scoped below a head that has moved on through data, a retraction and a second version.
const REVISION: u64 = 1;

fn provider_named(name: &str) -> Provider {
    match name {
        "file" => Provider::File,
        "sqlite" => Provider::Sqlite,
        other => panic!("no provider {other}"),
    }
}

#[test]
fn render_in_a_child_process() {
    let Ok(spec) = std::env::var(CHILD) else {
        return;
    };
    let parts: Vec<&str> = spec.split('\t').collect();
    let [provider, store, revision, out] = parts[..] else {
        panic!("{CHILD} is `{spec}`");
    };
    let runtime = fixtures::reopen(Path::new(store), provider_named(provider));
    let revision = RevisionNumber::new(revision.parse().expect("a revision"));
    let rendered = ekr_views::project(&runtime, Some(revision)).expect("the child renders");
    std::fs::write(out, rendered.bytes).expect("the child writes its render");
}

fn render_in_a_fresh_process(provider: Provider, store: &Path, out: &Path) -> Vec<u8> {
    let status = Command::new(std::env::current_exe().expect("this test binary"))
        .args(["--exact", "render_in_a_child_process", "--test-threads=1"])
        .env(
            CHILD,
            format!(
                "{}\t{}\t{REVISION}\t{}",
                provider.name(),
                store.display(),
                out.display()
            ),
        )
        .status()
        .expect("the child process runs");
    assert!(status.success(), "the child render failed: {status}");
    std::fs::read(out).expect("the child wrote a render: its case was selected and ran")
}

fn hex_sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn three_renders_are_one(provider: Provider) {
    let work = tempfile::tempdir().expect("work directory");
    let store: PathBuf = work.path().join("store");
    let at = Some(RevisionNumber::new(REVISION));
    let runtime = fixtures::open(&store, provider);
    Fixture::Evolved.build(&runtime);
    assert_eq!(runtime.head().unwrap().unwrap().revision.get(), 5);

    let first = ekr_views::project(&runtime, at).expect("first render");
    let second = ekr_views::project(&runtime, at).expect("second render");
    drop(runtime);
    let third = render_in_a_fresh_process(provider, &store, &work.path().join("child.json"));

    assert!(!first.bytes.is_empty());
    assert_eq!(first.bytes, second.bytes, "two renders in one process");
    assert_eq!(
        first.bytes, third,
        "a render in a fresh process after reopening"
    );
    assert_eq!(first.summary, second.summary);
    assert_eq!(first.summary.projection_hash, hex_sha256(&third));
    assert_eq!(first.summary.revision, REVISION);
    println!(
        "{} provider: revision {REVISION} renders {} bytes, sha256 {}",
        provider.name(),
        third.len(),
        first.summary.projection_hash
    );

    let answered = passes_every_admitted_scenario(provider);
    let suites: Vec<_> = answered
        .iter()
        .filter(|answer| answer.store == "store" && answer.summary.revision == REVISION)
        .collect();
    assert!(
        !suites.is_empty(),
        "the suite projected its `store` at revision {REVISION}: {answered:#?}"
    );
    for answer in suites {
        assert_eq!(
            answer.summary, first.summary,
            "the suite's render of `store`"
        );
    }
}

#[test]
fn the_file_provider_renders_one_revision_identically_three_times() {
    three_renders_are_one(Provider::File);
}

#[test]
fn the_sqlite_provider_renders_one_revision_identically_three_times() {
    three_renders_are_one(Provider::Sqlite);
}

/// Every answer this crate gives about revision `at`, by what was asked: the projection, and the
/// five bounded reads from a freshly loaded [`Index`] and from `cache` — the overview, an
/// expansion and a detail of every node, two searches and the timeline.
fn every_answer(
    runtime: &ekr_kernel::Runtime,
    cache: &mut IndexCache,
    at: u64,
) -> Vec<(String, Vec<u8>)> {
    let at = Some(RevisionNumber::new(at));
    let projected = ekr_views::project(runtime, at).expect("projection");
    let mut answers = vec![("projection".to_owned(), projected.bytes)];
    let fresh = Index::load(runtime, at).expect("index");
    let held = cache.index(runtime, at).expect("cached index");
    for (how, index) in [("fresh", &fresh), ("cached", &*held)] {
        let mut answer =
            |what: String, bytes: Vec<u8>| answers.push((format!("{how} {what}"), bytes));
        let overview = OverviewRequest::new(None).unwrap();
        answer("overview".into(), index.overview(&overview).unwrap().bytes);
        let nodes: Vec<_> = index.loaded().graph.nodes.keys().copied().collect();
        for node in &nodes {
            answer(
                format!("describe {node}"),
                index.describe(*node).unwrap().bytes,
            );
        }
        let expand = ExpandRequest::new(nodes.clone(), 2, 10, None, None).unwrap();
        answer("expand".into(), index.expand(&expand).unwrap().bytes);
        for text in ["", "a"] {
            let search = SearchRequest::new(text.to_owned(), 100).unwrap();
            answer(
                format!("search {text:?}"),
                index.search(&search).unwrap().bytes,
            );
        }
        let timeline = TimelineRequest::new(None, 2, 500, None, None).unwrap();
        answer("timeline".into(), index.timeline(&timeline).unwrap().bytes);
    }
    answers
}

/// `task:historical-projection-carries-the-head`: on one store, every revision answered before
/// an unrelated commit and again after it gives the same bytes — the projection and every one of
/// the five bounded formats, through a fresh index and through an [`IndexCache`] large enough to
/// hold all six revisions across the commit. A document that named the head would differ after
/// the commit.
fn a_later_commit_changes_no_answer_about_an_earlier_revision(provider: Provider) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(&work.path().join("store"), provider);
    Fixture::Evolved.build(&runtime);
    let mut cache = IndexCache::new(6);
    let before: Vec<_> = (0..=5)
        .map(|at| every_answer(&runtime, &mut cache, at))
        .collect();
    fixtures::commit_unrelated(&runtime, 0);
    assert_eq!(runtime.head().unwrap().unwrap().revision.get(), 6);
    let mut changed = Vec::new();
    let mut compared = 0;
    for (at, before) in (0_u64..).zip(before) {
        let after = every_answer(&runtime, &mut cache, at);
        assert_eq!(before.len(), after.len(), "revision {at}");
        for ((what, one), (_, other)) in before.iter().zip(&after) {
            compared += 1;
            if one != other {
                changed.push(format!(
                    "revision {at}, {what}:\n  before {}\n  after  {}",
                    String::from_utf8_lossy(&one[..one.len().min(120)]),
                    String::from_utf8_lossy(&other[..other.len().min(120)]),
                ));
            }
        }
    }
    assert!(
        changed.is_empty(),
        "{} provider: {} of {compared} answers changed bytes after an unrelated commit:\n{}",
        provider.name(),
        changed.len(),
        changed.join("\n")
    );
}

#[test]
fn the_file_provider_answers_a_past_revision_identically_after_a_later_commit() {
    a_later_commit_changes_no_answer_about_an_earlier_revision(Provider::File);
}

#[test]
fn the_sqlite_provider_answers_a_past_revision_identically_after_a_later_commit() {
    a_later_commit_changes_no_answer_about_an_earlier_revision(Provider::Sqlite);
}

/// Two providers holding the same fixture render every one of its revisions to the same bytes:
/// nothing about where the store lives reaches the document.
#[test]
fn both_providers_render_every_revision_of_one_fixture_to_the_same_bytes() {
    let work = tempfile::tempdir().expect("work directory");
    let file = fixtures::open(&work.path().join("file"), Provider::File);
    let sqlite = fixtures::open(&work.path().join("sqlite"), Provider::Sqlite);
    Fixture::Evolved.build(&file);
    Fixture::Evolved.build(&sqlite);
    for revision in 0..=5 {
        let at = Some(RevisionNumber::new(revision));
        let from_file = ekr_views::project(&file, at).expect("file render");
        let from_sqlite = ekr_views::project(&sqlite, at).expect("sqlite render");
        assert_eq!(from_file.bytes, from_sqlite.bytes, "revision {revision}");
    }
    assert_eq!(
        ekr_views::project(&file, None).unwrap().bytes,
        ekr_views::project(&file, Some(RevisionNumber::new(5)))
            .unwrap()
            .bytes,
        "no revision named renders the head"
    );
}
