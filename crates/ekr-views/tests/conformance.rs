//! `story:graph-projection-renderer`: the admitted, synthesized `ekr-views` suite executed
//! against `ekr_views::project` through the real kernel, on both native providers.
//!
//! Every case admits the committed `systems/ekr/conformance/views-suite.json` bytes, runs them
//! with `Runner::run_admitted`, and reads the verdict off `report/2` (`CountReport::from_run`)
//! paired with those exact bytes. Each run prints its selected/terminal counts and every scenario
//! that did not pass, and writes report/2 and run/2 under the build directory. Freshness of the
//! committed suite against the specification is `task conform-check`'s synthesis-and-`cmp` step.
//! The scenario set, each authored scenario's file digest, each scenario's step floors, the
//! total and the floor are held against the hand-committed
//! `systems/ekr/conformance/views-baseline.json`, never against the suite under test, so a
//! scenario lost, or kept by name with its expectations stripped, and regenerated is named
//! rather than absorbed. A deliberate change to a scenario updates the baseline in the same
//! change, where a reviewer sees it.

mod support;

use std::collections::BTreeSet;

use support::fixtures::Provider;
use support::suite::{
    admitted, authored_files, authored_scenarios_in_files, baseline, holds_the_baseline_content,
    holds_the_baseline_scenarios, passes_every_admitted_scenario, provenance_authored, suite_json,
    suite_scenarios, AUTHORED, BASELINE, PROVENANCE,
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
/// exactly the scenarios the committed baseline names, which are the thirty-nine generated
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
    holds_the_baseline_content(&baseline, &suite_json(), "the committed views suite");
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
    assert_eq!(names.len(), authored + 39, "{names:#?}");
}

/// The authored files `views-provenance.json` records the suite was synthesized from are exactly
/// the files in the authored scenario directory, and each file's scenario — named by its own
/// `domain:` and `scenario:` fields, as ESS names it — is one the committed baseline pins, in
/// that file and with those exact bytes. Editing an authored scenario updates its digest in
/// `views-baseline.json` in the same change; that update is the point a reviewer sees it.
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
    let from_files = authored_scenarios_in_files();
    let pinned = baseline().authored;
    let lost: Vec<&String> = pinned
        .keys()
        .filter(|name| !from_files.contains_key(*name))
        .collect();
    assert!(
        lost.is_empty(),
        "{BASELINE} requires authored scenario(s) no file in {AUTHORED} authors: {lost:#?}"
    );
    let unlisted: Vec<&String> = from_files
        .keys()
        .filter(|name| !pinned.contains_key(*name))
        .collect();
    assert!(
        unlisted.is_empty(),
        "{AUTHORED} authors scenario(s) {BASELINE} does not name; a new scenario updates the \
         baseline in the same change: {unlisted:#?}"
    );
    let changed: Vec<String> = pinned
        .iter()
        .filter(|(name, pin)| from_files[*name] != **pin)
        .map(|(name, pin)| {
            let found = &from_files[name];
            format!(
                "{name}: baseline pins {} {}, the directory holds {} {}",
                pin.file, pin.digest, found.file, found.digest
            )
        })
        .collect();
    assert!(
        changed.is_empty(),
        "authored scenario file(s) differ from what {BASELINE} pins; a deliberate change updates \
         the baseline in the same change: {changed:#?}"
    );
}
