//! Adversary, `task:historical-projection-carries-the-head`, pass 1: the `ekr-views` engine.
//!
//! The unit's own determinism case answers every revision before and after one commit through
//! one runtime handle. These cases attack what that case does not say:
//!
//! * the store is closed and opened again after the commits, so a past revision is read back
//!   from the provider (and whatever replay checkpoint the commits left) rather than from the
//!   authority state the handle already held, and it must still answer the same bytes;
//! * the revision that was the head is answered by `None` before the commits and by its number
//!   after them, which must be the same bytes;
//! * an index held by an [`IndexCache`] from while its revision was the head must be exactly —
//!   as a [`ekr_views::LoadedRevision`] — what a fresh load of that revision gives after the
//!   commits, from a fresh handle;
//! * paging cursors (`next`, `remaining`), every subject's timeline, both bucket widths and
//!   small search limits keep their bytes too.

mod support;

use std::path::Path;

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use ekr_views::{
    BucketWidth, ExpandRequest, Index, IndexCache, OverviewRequest, SearchRequest, TimelineRequest,
};

use support::fixtures::{self, Fixture, Provider};

/// Every answer about the index's revision, by what was asked: overview at two limits, a detail
/// of every node, every page of an expansion from every node paged two nodes at a time (the
/// cursors included), searches at small limits and the timeline over every subject and bucket.
fn answers_of(index: &Index) -> Vec<(String, Vec<u8>)> {
    let mut answers = Vec::new();
    for limit in [None, Some(1)] {
        let request = OverviewRequest::new(limit).unwrap();
        let answer = index.overview(&request).unwrap();
        answers.push((format!("overview {limit:?}"), answer.bytes));
    }
    let nodes: Vec<_> = index.loaded().graph.nodes.keys().copied().collect();
    for node in &nodes {
        answers.push((
            format!("describe {node}"),
            index.describe(*node).unwrap().bytes,
        ));
        let mut after = None;
        loop {
            let request = ExpandRequest::new(vec![*node], 2, 2, Some(1), after).unwrap();
            let page = index.page(&request).unwrap();
            let next = page.next();
            let remaining = page.remaining();
            answers.push((
                format!("expand {node} after {after:?} (next {next:?}, remaining {remaining})"),
                page.render().unwrap().bytes,
            ));
            match next {
                Some(next) => after = Some(i64::try_from(next).unwrap()),
                None => break,
            }
        }
        for bucket in [None, Some(BucketWidth::Day), Some(BucketWidth::Week)] {
            let request = TimelineRequest::new(None, 3, 500, bucket, Some(*node)).unwrap();
            answers.push((
                format!("timeline of {node} {bucket:?}"),
                index.timeline(&request).unwrap().bytes,
            ));
        }
    }
    for text in ["", "a", "e", "ALPHA", "note"] {
        for limit in [1, 100] {
            let request = SearchRequest::new(text.to_owned(), limit).unwrap();
            answers.push((
                format!("search {text:?} limit {limit}"),
                index.search(&request).unwrap().bytes,
            ));
        }
    }
    for bucket in [None, Some(BucketWidth::Day), Some(BucketWidth::Week)] {
        for hops in 1..=3 {
            let request = TimelineRequest::new(None, hops, 500, bucket, None).unwrap();
            answers.push((
                format!("timeline hops {hops} {bucket:?}"),
                index.timeline(&request).unwrap().bytes,
            ));
        }
    }
    answers
}

/// The projection and every bounded answer of revision `at` (the head when `None`).
fn every_answer(runtime: &Runtime, at: Option<u64>) -> Vec<(String, Vec<u8>)> {
    let at = at.map(RevisionNumber::new);
    let mut answers = vec![(
        "projection".to_owned(),
        ekr_views::project(runtime, at).unwrap().bytes,
    )];
    answers.extend(answers_of(&Index::load(runtime, at).unwrap()));
    answers
}

fn differences(before: &[(String, Vec<u8>)], after: &[(String, Vec<u8>)], at: u64) -> Vec<String> {
    let mut changed = Vec::new();
    if before.len() != after.len() {
        changed.push(format!(
            "revision {at}: {} answers before, {} after",
            before.len(),
            after.len()
        ));
    }
    for ((what, one), (other_what, other)) in before.iter().zip(after) {
        if what != other_what || one != other {
            changed.push(format!(
                "revision {at}, {what} / {other_what}:\n  before {}\n  after  {}",
                String::from_utf8_lossy(&one[..one.len().min(160)]),
                String::from_utf8_lossy(&other[..other.len().min(160)]),
            ));
        }
    }
    changed
}

/// Every revision answered through one handle before three commits, then through a handle opened
/// afresh on the same provider after them: the same bytes, the head's `None` answer included.
fn a_reopened_store_answers_every_past_revision_as_before(provider: Provider) {
    let work = tempfile::tempdir().unwrap();
    let store = work.path().join("store");
    let (head_before, before, as_head) = {
        let runtime = fixtures::open(&store, provider);
        Fixture::Evolved.build(&runtime);
        let head = runtime.head().unwrap().unwrap().revision.get();
        let before: Vec<_> = (0..=head)
            .map(|at| every_answer(&runtime, Some(at)))
            .collect();
        let as_head = every_answer(&runtime, None);
        for n in 0..3 {
            fixtures::commit_unrelated(&runtime, n);
        }
        (head, before, as_head)
    };
    let runtime = reopen(&store, provider);
    assert_eq!(
        runtime.head().unwrap().unwrap().revision.get(),
        head_before + 3
    );
    let mut changed = Vec::new();
    for (at, before) in (0_u64..).zip(&before) {
        changed.extend(differences(before, &every_answer(&runtime, Some(at)), at));
    }
    changed.extend(differences(
        &as_head,
        &every_answer(&runtime, Some(head_before)),
        head_before,
    ));
    assert!(
        changed.is_empty(),
        "{} provider: {} answers changed after three commits and a reopen:\n{}",
        provider.name(),
        changed.len(),
        changed.join("\n")
    );
}

fn reopen(store: &Path, provider: Provider) -> Runtime {
    fixtures::reopen(store, provider)
}

#[test]
fn adversary_views_head_the_file_provider_reopened_after_commits_answers_every_past_revision_as_before(
) {
    a_reopened_store_answers_every_past_revision_as_before(Provider::File);
}

#[test]
fn adversary_views_head_the_sqlite_provider_reopened_after_commits_answers_every_past_revision_as_before(
) {
    a_reopened_store_answers_every_past_revision_as_before(Provider::Sqlite);
}

/// An index an [`IndexCache`] loaded while its revision was the head, and one of every earlier
/// revision, held across three commits: each is served from memory afterwards and each is, as a
/// loaded revision and in every answer, what a handle opened afresh after the commits loads.
fn a_held_index_is_what_a_fresh_handle_loads_after_commits(provider: Provider) {
    let work = tempfile::tempdir().unwrap();
    let store = work.path().join("store");
    let runtime = fixtures::open(&store, provider);
    Fixture::Evolved.build(&runtime);
    let head = runtime.head().unwrap().unwrap().revision.get();
    let mut cache = IndexCache::new(usize::try_from(head).unwrap() + 4);
    let held_head = cache.index(&runtime, None).unwrap();
    assert_eq!(held_head.revision().get(), head);
    let held: Vec<_> = (0..head)
        .map(|at| {
            cache
                .index(&runtime, Some(RevisionNumber::new(at)))
                .unwrap()
        })
        .collect();
    for n in 0..3 {
        fixtures::commit_unrelated(&runtime, n);
    }
    let fresh_runtime = reopen(&store, provider);
    for index in held.iter().chain([&held_head]) {
        let at = index.revision();
        let again = cache.index(&runtime, Some(at)).unwrap();
        assert!(
            std::sync::Arc::ptr_eq(index, &again),
            "{} provider: revision {at} was loaded again",
            provider.name()
        );
        let fresh = Index::load(&fresh_runtime, Some(at)).unwrap();
        assert!(
            index.loaded() == fresh.loaded(),
            "{} provider: the index held for revision {at} is not what a fresh handle loads",
            provider.name()
        );
        let changed = differences(&answers_of(index), &answers_of(&fresh), at.get());
        assert!(
            changed.is_empty(),
            "{} provider:\n{}",
            provider.name(),
            changed.join("\n")
        );
    }
    let newest = cache.index(&runtime, None).unwrap();
    assert_eq!(newest.revision().get(), head + 3);
    assert!(
        newest.loaded() == Index::load(&fresh_runtime, None).unwrap().loaded(),
        "{} provider: the new head's index",
        provider.name()
    );
}

#[test]
fn adversary_views_head_a_file_index_held_across_commits_is_a_fresh_load() {
    a_held_index_is_what_a_fresh_handle_loads_after_commits(Provider::File);
}

#[test]
fn adversary_views_head_a_sqlite_index_held_across_commits_is_a_fresh_load() {
    a_held_index_is_what_a_fresh_handle_loads_after_commits(Provider::Sqlite);
}
