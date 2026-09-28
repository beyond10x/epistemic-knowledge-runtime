---
format: aep.planning-md/3
id: task:views-conformance-has-no-independent-floor
kind: task
status: draft
title: The views conformance suite's expected scenario set is read from the suite under test
summary: 'Coverage concern: deleting an authored views scenario and regenerating the suite keeps every gate green'
tags:
- review-2026-09-28
- severity-coverage
relations:
- serves: vision:o5
- derived_from: story:graph-projection-renderer
- derived_from: story:view-streams-overview-and-expansion
revision: 1
---
## Severity

Coverage concern (the review's label; no defect in shipped behaviour). Deleting an authored views
scenario and regenerating the suite keeps every gate green, so lost coverage is not reported.

## The review (external review of EKR, 2026-09-28, verbatim)

> Coverage concern — views conformance lacks an independent regression baseline. Expected
> scenarios and passing counts are derived from the suite being tested, allowing deleted coverage to
> redefine its own floor after regeneration. Established by inspection; the mutation experiment was
> interrupted. `crates/ekr-views/tests/support/suite.rs:99`

## What was verified (code reading at `wave/p2p3p4-06` 69e5b994; not run)

- **Verified**: `passes_every_admitted_scenario` takes `expected` from `suite_scenarios()`, which
  reads the scenario names off the committed `systems/ekr/conformance/views-suite.json`
  (`crates/ekr-views/tests/support/suite.rs:20-28`, `:59`), and every count assertion compares
  against `expected.len()` (`suite.rs:99-106`).
- **Verified**: `the_committed_suite_is_the_complete_views_inventory` compares the suite's names
  with the same `suite_scenarios()` and counts authored scenarios by listing
  `crates/ekr-views/tests/fixtures/conformance/scenarios` (`crates/ekr-views/tests/conformance.rs:42-56`).
  Deleting an authored scenario file and regenerating the suite (`task conform-fresh`,
  `Taskfile.yml:88-89`, which checks only that the committed suite is fresh synthesis) lowers
  both sides together.
- **Partial floor, which the review does not mention**: `names.len() == authored + 20`
  (`conformance.rs:56`) pins the generated outcome scenarios at twenty, so removing a generated
  one is caught. Authored scenarios have no floor.
- **Inferred**: the mutation experiment the review names (delete, regenerate, stay green) was
  interrupted in the review and not run by me.

## Reproduce

Delete one file under `crates/ekr-views/tests/fixtures/conformance/scenarios`, regenerate
`systems/ekr/conformance/views-suite.json` with the synthesize command from `task conform-fresh`,
then run `cargo test -p ekr-views --test conformance`.

## Acceptance

A committed baseline that the suite does not generate — a list of the authored scenario names, or
a minimum count per kind — is held by `cargo test -p ekr-views --test conformance`, and the
reproduction above (one authored scenario deleted, suite regenerated) makes that test fail with a
message naming the missing scenario. Adding a scenario requires updating the baseline in the same
change.
