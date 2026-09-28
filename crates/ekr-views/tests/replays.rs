//! `task:view-load-replays-once`: [`ekr_views::load`] replays a store's history at most once,
//! whatever the number of schema versions the loaded revision's lineage passes through.
//!
//! The count is the kernel's own, [`ekr_kernel::Runtime::seed_replays`]: each replay that began
//! at the seed rather than at a state the runtime had already reached or at the store's replay
//! checkpoint. Each load runs on a freshly reopened runtime, so no earlier read of this test has
//! reached anything for it.

mod support;

use ekr_core::RevisionNumber;

use support::fixtures::{self, Fixture, Provider};

/// The evolved store's lineage: six revisions under three schema versions (the seed's, one from
/// revision 1, one from revision 4).
const HEAD: u64 = 5;

fn replays_of_one_load(
    store: &std::path::Path,
    provider: Provider,
    at: Option<RevisionNumber>,
    full: bool,
) -> (u64, ekr_views::LoadedRevision) {
    let mut runtime = fixtures::reopen(store, provider);
    runtime.set_full_replay(full);
    let before = runtime.seed_replays();
    let loaded = ekr_views::load(&runtime, at).expect("the revision loads");
    (runtime.seed_replays() - before, loaded)
}

#[test]
fn a_load_replays_at_most_once_whatever_the_schema_versions_and_revision() {
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().expect("work directory");
        Fixture::Evolved.build(&fixtures::open(work.path(), provider));

        let requests =
            std::iter::once(None).chain((0..=HEAD).map(|number| Some(RevisionNumber::new(number))));
        for at in requests {
            let (replays, loaded) = replays_of_one_load(work.path(), provider, at, false);
            let (full_replays, from_seed) = replays_of_one_load(work.path(), provider, at, true);
            let versions = loaded.schemas.len();
            assert!(
                replays <= 1,
                "{} provider, revision {at:?} under {versions} schema versions: {replays} replays",
                provider.name()
            );
            assert_eq!(
                full_replays,
                1,
                "{} provider, revision {at:?} under {versions} schema versions, full replay",
                provider.name()
            );
            assert_eq!(
                loaded,
                from_seed,
                "{} provider, revision {at:?}: the checkpointed and the full load differ",
                provider.name()
            );
        }
        let (_, head) = replays_of_one_load(work.path(), provider, None, false);
        assert_eq!(head.schemas.len(), 3, "the fixture's three schema versions");
    }
}
