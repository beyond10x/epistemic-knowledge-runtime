//! `story:graph-projection-renderer`: the admitted, synthesized `ekr-views` suite executed
//! against `ekr_views::project` through the real kernel, on both native providers.
//!
//! Every case admits the committed `systems/ekr/conformance/views-suite.json` bytes, runs them
//! with `Runner::run_admitted`, and reads the verdict off `report/2` (`CountReport::from_run`)
//! paired with those exact bytes. Each run prints its selected/terminal counts and every scenario
//! that did not pass, and writes report/2 and run/2 under the build directory. Freshness of the
//! committed suite against the specification is `task conform-check`'s synthesis-and-`cmp` step.

mod support;

use std::collections::BTreeSet;

use support::fixtures::Provider;
use support::suite::{admitted, passes_every_admitted_scenario, suite_scenarios};

#[test]
fn the_file_provider_passes_every_admitted_views_scenario() {
    passes_every_admitted_scenario(Provider::File);
}

#[test]
fn the_sqlite_provider_passes_every_admitted_views_scenario() {
    passes_every_admitted_scenario(Provider::Sqlite);
}

/// The committed suite is the `ekr-views` component's, complete, with nothing refused, and it
/// selects the twenty-four generated outcome scenarios and every authored one.
#[test]
fn the_committed_suite_is_the_complete_views_inventory() {
    let admitted = admitted();
    let suite = admitted.suite();
    assert_eq!(suite.provenance.system.to_string(), "ekr");
    assert_eq!(
        suite.provenance.component.as_ref().map(ToString::to_string),
        Some("ekr-views".to_owned())
    );
    let coverage = admitted.coverage().expect("declared coverage inventory");
    assert!(coverage.is_complete(), "coverage inventory is incomplete");
    assert!(coverage.refused.is_empty(), "{:?}", coverage.refused);
    let names: BTreeSet<String> = suite.scenarios.keys().map(ToString::to_string).collect();
    assert_eq!(names, suite_scenarios());
    let authored = std::fs::read_dir(
        support::workspace_root().join("crates/ekr-views/tests/fixtures/conformance/scenarios"),
    )
    .expect("authored scenarios")
    .count();
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
