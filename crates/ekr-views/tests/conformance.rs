//! `story:graph-projection-renderer`: the admitted, synthesized `ekr-views` suite executed
//! against `ekr_views::project` through the real kernel, on both native providers.
//!
//! Every case admits the committed `systems/ekr/conformance/views-suite.json` bytes, runs them
//! with `Runner::run_admitted`, and reads the verdict off `report/2` (`CountReport::from_run`)
//! paired with those exact bytes. Each run prints its selected/terminal counts and every scenario
//! that did not pass, and writes report/2 and run/2 under the build directory. Freshness of the
//! committed suite against the specification is `task conform-check`'s synthesis-and-`cmp` step.
//! The scenario set, total and floor are held against the hand-committed
//! `systems/ekr/conformance/views-baseline.json`, never against the suite under test, so a
//! scenario lost and regenerated away is named rather than absorbed.

mod support;

use std::collections::BTreeSet;

use support::fixtures::Provider;
use support::suite::{
    admitted, authored_files, authored_scenario_name, baseline, holds_the_baseline_scenarios,
    passes_every_admitted_scenario, provenance_authored, suite_scenarios, AUTHORED,
    AUTHORED_PREFIX, BASELINE, PROVENANCE,
};

#[test]
fn the_file_provider_passes_every_admitted_views_scenario() {
    passes_every_admitted_scenario(Provider::File);
}

#[test]
fn the_sqlite_provider_passes_every_admitted_views_scenario() {
    passes_every_admitted_scenario(Provider::Sqlite);
}

/// The committed suite is the `ekr-views` component's, complete, with nothing refused; it selects
/// exactly the scenarios the committed baseline names, which are the twenty-four generated
/// outcome scenarios and every authored one.
#[test]
fn the_committed_suite_is_the_complete_views_inventory() {
    let baseline = baseline();
    let admitted = admitted();
    let suite = admitted.suite();
    assert_eq!(suite.provenance.system.to_string(), "ekr");
    assert_eq!(
        suite.provenance.component.as_ref().map(ToString::to_string),
        Some("ekr-views".to_owned())
    );
    assert_eq!(
        suite.provenance.suite_version.to_string(),
        baseline.suite_version
    );
    let coverage = admitted.coverage().expect("declared coverage inventory");
    assert!(coverage.is_complete(), "coverage inventory is incomplete");
    assert!(coverage.refused.is_empty(), "{:?}", coverage.refused);
    let names: BTreeSet<String> = suite.scenarios.keys().map(ToString::to_string).collect();
    holds_the_baseline_scenarios(&baseline, &names, "the committed views suite");
    assert!(
        suite.len() as u64 >= baseline.total,
        "the suite selects {} scenarios, below the baseline's {}",
        suite.len(),
        baseline.total
    );
    assert_eq!(names, suite_scenarios());
    let authored = authored_files().len();
    assert_eq!(
        names
            .iter()
            .filter(|name| name.contains("/authored/"))
            .count(),
        authored,
        "every authored scenario file is selected"
    );
    assert_eq!(names.len(), authored + 24, "{names:#?}");
}

/// The authored files `views-provenance.json` records the suite was synthesized from are exactly
/// the files in the authored scenario directory, and each is one the committed baseline names.
#[test]
fn the_provenance_and_the_baseline_name_every_authored_scenario_file() {
    let recorded = provenance_authored();
    let present = authored_files();
    let absent: Vec<&String> = recorded.difference(&present).collect();
    assert!(
        absent.is_empty(),
        "{PROVENANCE} records authored file(s) the directory no longer holds: {absent:#?}"
    );
    let unrecorded: Vec<&String> = present.difference(&recorded).collect();
    assert!(
        unrecorded.is_empty(),
        "{AUTHORED} holds file(s) {PROVENANCE} does not record: {unrecorded:#?}"
    );
    let from_files: BTreeSet<String> = present
        .iter()
        .map(|path| authored_scenario_name(path))
        .collect();
    let in_baseline: BTreeSet<String> = baseline()
        .scenarios
        .into_iter()
        .filter(|name| name.starts_with(AUTHORED_PREFIX))
        .collect();
    let lost: Vec<&String> = in_baseline.difference(&from_files).collect();
    assert!(
        lost.is_empty(),
        "{BASELINE} requires authored scenario(s) with no file in {AUTHORED}: {lost:#?}"
    );
    let unlisted: Vec<&String> = from_files.difference(&in_baseline).collect();
    assert!(
        unlisted.is_empty(),
        "{AUTHORED} holds scenario(s) {BASELINE} does not name; a new scenario updates the \
         baseline in the same change: {unlisted:#?}"
    );
}
