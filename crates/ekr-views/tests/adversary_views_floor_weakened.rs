//! Adversary, wave read-01 unit F (`task:views-conformance-has-no-independent-floor`), pass 1.
//!
//! The committed baseline holds scenario *names* and counts. A scenario kept by name whose
//! expectations were stripped keeps every name, every count and every pass, so the baseline
//! cannot tell it from the full one.
//!
//! The weakened scenario below is exactly what `ess 0.36.0 conform synthesize` (the command
//! `views-provenance.json` records) writes when
//! `a-search-with-no-match-answers-an-empty-list.yaml` is cut down to its first timeline entry
//! with no `events:`: the suite still selects 41 scenarios, 17 authored, 0 refusals, and this
//! scenario's steps are the committed scenario's first two (`execute_command`,
//! `expect_outcome searched`), with the three `expect_event NodesSearched` payload checks and the
//! two later searches gone. Its `source` drops `ekr.graph.NodeId`, `ekr.kernel.ContentHash` and
//! `ekr.views.NodesSearched`, as the synthesis does.

mod support;

use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};

use ess_conformance::{AdmittedSuite, CountReport, Runner};
use support::fixtures::Provider;
use support::suite::{baseline, holds_the_baseline_content, holds_the_baseline_scenarios, SUITE};
use support::target::ViewsTarget;

const WEAKENED: &str = "ekr.views/authored/a-search-with-no-match-answers-an-empty-list";

fn weakened_suite() -> (String, usize, usize) {
    let mut suite: serde_json::Value =
        serde_json::from_str(&support::read(SUITE)).expect("committed views suite");
    let scenario = &mut suite["scenarios"][WEAKENED];
    let steps = scenario["steps"].as_array_mut().expect("steps");
    let before = steps.len();
    let events_before = steps
        .iter()
        .filter(|step| step["step"] == "expect_event")
        .count();
    steps.truncate(2);
    assert_eq!(steps[0]["step"], "execute_command");
    assert_eq!(steps[1]["step"], "expect_outcome");
    let dropped = [
        "ekr.graph.NodeId",
        "ekr.kernel.ContentHash",
        "ekr.views.NodesSearched",
    ];
    scenario["source"]
        .as_array_mut()
        .expect("source")
        .retain(|entry| !dropped.iter().any(|name| entry["name"] == *name));
    (
        serde_json::to_string(&suite).expect("weakened suite JSON"),
        before,
        events_before,
    )
}

#[test]
fn adversary_views_floor_a_scenario_kept_by_name_with_its_expectations_stripped_fails_the_baseline()
{
    let (json, steps_before, events_before) = weakened_suite();
    let admitted = AdmittedSuite::from_json(&json).expect("the weakened suite admits");
    let committed = baseline();

    let names: BTreeSet<String> = admitted
        .suite()
        .scenarios
        .keys()
        .map(ToString::to_string)
        .collect();
    let names_hold = catch_unwind(AssertUnwindSafe(|| {
        holds_the_baseline_scenarios(&committed, &names, "the weakened views suite")
    }))
    .is_ok();
    let weakened: serde_json::Value = serde_json::from_str(&json).expect("weakened suite JSON");
    let content_holds = catch_unwind(AssertUnwindSafe(|| {
        holds_the_baseline_content(&committed, &weakened, "the weakened views suite")
    }))
    .is_ok();

    let work = tempfile::TempDir::new().expect("isolated provider root");
    let target = ViewsTarget::new(Provider::File, work.path().to_path_buf());
    let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &target);
    let report =
        CountReport::from_run(&run, &admitted).expect("report/2 pairs with the weakened suite");
    let counts = report.counts();

    let total_holds = counts.total >= committed.total;
    let floor_holds = counts.passed >= committed.answered_floor;
    let ceiling_holds =
        counts.error + counts.unsupported + counts.skipped <= committed.unavailable_ceiling;
    let all_pass = counts.failed == 0 && run.is_conformant();

    assert!(
        !(names_hold && content_holds && total_holds && floor_holds && ceiling_holds && all_pass),
        "{WEAKENED} kept its name but lost {} of its {steps_before} steps ({events_before} of \
         {events_before} expect_event checks), and the weakened suite still holds the committed \
         baseline: names {names_hold}, content {content_holds}, total {} >= {}, passed {} >= \
         floor {}, unavailable {} <= ceiling {}, failed {}",
        steps_before - 2,
        counts.total,
        committed.total,
        counts.passed,
        committed.answered_floor,
        counts.error + counts.unsupported + counts.skipped,
        committed.unavailable_ceiling,
        counts.failed,
    );
}
