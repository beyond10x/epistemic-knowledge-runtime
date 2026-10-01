//! `story:extraction-verb-shares-the-sdk-path`: the `ekr-integrate` component's conformance target,
//! [`IntegrateTarget`], answers both branches of `ekr.integrate.ApplyExtraction` on both native
//! providers, and publishes `ekr.integrate.ExtractionApplied` for an applied document.
//!
//! `systems/ekr` declares the command on the `ekr-integrate` component; the suite ESS synthesizes
//! for it (`ess conform synthesize --component ekr-integrate`) holds two scenarios,
//! `ekr.integrate.ApplyExtraction/outcome/applied` and `.../outcome/refused`. Until that suite is
//! committed, these cases drive the target through the same `ConformanceTarget` calls the runner
//! makes for those two scenarios, with the fixtures the kernel suite's manifest names.

use std::collections::BTreeMap;
use std::path::PathBuf;

use ekr::conformance::{IntegrateTarget, Provider};
use ess_conformance::target::{
    ConformanceTarget, Deadline, EventObservationRequest, ExternalOutcomeControl, ScenarioContext,
    SemanticCommandRequest, SemanticCommandResult,
};
use ess_primitives::ids::CorrelationId;
use ess_primitives::node::Node;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/conformance")
}

fn correlation() -> CorrelationId {
    CorrelationId::new("ekr-integrate").unwrap()
}

/// One scenario: `refused` forces that branch first, as the synthesized scenario does.
fn run(target: &IntegrateTarget, outcome: &str) -> SemanticCommandResult {
    let scenario = ScenarioContext::new(
        format!("ekr.integrate.ApplyExtraction/outcome/{outcome}")
            .parse()
            .unwrap(),
        correlation(),
    );
    target.begin_scenario(&scenario).unwrap();
    if outcome == "refused" {
        target
            .configure_external_outcome(ExternalOutcomeControl {
                force: serde_json::from_value(serde_json::json!({
                    "command": "ekr.integrate.ApplyExtraction",
                    "outcome": "refused",
                }))
                .unwrap(),
                correlation: correlation(),
            })
            .unwrap();
    }
    let result = target
        .execute_command(SemanticCommandRequest {
            command: "ekr.integrate.ApplyExtraction".parse().unwrap(),
            actor: Some("ekr.integrate.Applier".parse().unwrap()),
            input: BTreeMap::from([(
                "extraction_document".to_owned(),
                Node::Text("extraction_document".to_owned()),
            )]),
            correlation: correlation(),
        })
        .unwrap();
    let observed = target
        .observe_events(EventObservationRequest {
            event: "ekr.integrate.ExtractionApplied".parse().unwrap(),
            correlation: correlation(),
            // A fixed instant, not a clock: the target answers at once from what it published.
            deadline: Deadline::at(ess_primitives::time::Timestamp::EPOCH),
        })
        .unwrap();
    assert_eq!(observed, result.direct_events, "{outcome}");
    target.end_scenario(&scenario).unwrap();
    result
}

#[test]
fn the_integrate_target_answers_both_branches_of_apply_extraction() {
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().unwrap();
        let target: IntegrateTarget =
            IntegrateTarget::new(provider, &fixtures(), work.path()).unwrap();
        assert!(target
            .identity()
            .unwrap()
            .to_string()
            .contains("ekr-integrate"));

        // Dana is new and CEO of Initech, also new: two nodes, then the fact with its evidence.
        let applied = run(&target, "applied");
        assert_eq!(
            applied.outcome.as_ref().map(ToString::to_string).as_deref(),
            Some("ekr.integrate.ApplyExtraction/applied"),
            "{provider:?}: {applied:?}"
        );
        assert_eq!(applied.error, None, "{provider:?}");
        let [event] = applied.direct_events.as_slice() else {
            panic!("{provider:?}: one event, {applied:?}");
        };
        let count = |field: &str| event.payload.get(field).cloned();
        let integer = |value: i64| Some(Node::Number(value.into()));
        assert_eq!(count("committed"), integer(2), "{provider:?}: {event:?}");
        assert_eq!(count("rejected"), integer(0), "{provider:?}");
        assert_eq!(count("ambiguous"), integer(0), "{provider:?}");
        assert_eq!(count("held"), integer(0), "{provider:?}");

        // Person has a subtype in the refusing seed: the reader refuses before any write.
        let refused = run(&target, "refused");
        assert_eq!(
            refused.outcome.as_ref().map(ToString::to_string).as_deref(),
            Some("ekr.integrate.ApplyExtraction/refused"),
            "{provider:?}: {refused:?}"
        );
        let error = refused.error.expect("a declared error");
        assert_eq!(error.error.to_string(), "ekr.integrate.ExtractionRefused");
        assert_eq!(
            error.fields.get("code"),
            Some(&Node::Text("reference-type-has-subtypes".to_owned())),
            "{provider:?}: {error:?}"
        );
        assert!(refused.direct_events.is_empty(), "{provider:?}");
    }
}
