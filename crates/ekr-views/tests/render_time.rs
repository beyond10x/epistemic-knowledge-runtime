//! `story:graph-projection-renderer`, the measurement: render time on a store of a few hundred
//! revisions, on both providers, printed rather than bounded. The store is not changed here.
//!
//! Behind the `bench` feature, as `ekr-kernel`'s `command_bench` is: building 300 revisions
//! through the debug kernel takes longer than the package gate should. Run it with
//! `cargo test -p ekr-views --release --features bench --test render_time -- --nocapture`.

mod support;

use std::time::Instant;

use ekr_core::RevisionNumber;

use support::fixtures::{self, Provider};

/// Revisions after the schema change: with the seed and that change, 302 revisions.
const EXTRA: u64 = 300;

#[test]
fn render_time_on_a_store_of_three_hundred_revisions_is_measured_on_both_providers() {
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), provider);
        let built = Instant::now();
        fixtures::build_long(&runtime, EXTRA);
        let built = built.elapsed();
        drop(runtime);
        let runtime = fixtures::reopen(work.path(), provider);
        let head = EXTRA + 1;
        for at in [RevisionNumber::new(head), RevisionNumber::new(head / 2)] {
            let started = Instant::now();
            let first = ekr_views::project(&runtime, Some(at)).expect("render");
            let first_time = started.elapsed();
            let started = Instant::now();
            let second = ekr_views::project(&runtime, Some(at)).expect("render");
            let second_time = started.elapsed();
            assert_eq!(first.bytes, second.bytes);
            assert_eq!(first.summary.revisions, at.get() + 1);
            println!(
                "{} provider, {} revisions (built in {built:?}): revision {at} rendered {} bytes \
                 in {first_time:?} after reopening, {second_time:?} again",
                provider.name(),
                head + 1,
                first.bytes.len()
            );
        }
    }
}
