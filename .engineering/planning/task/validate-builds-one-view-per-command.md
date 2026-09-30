---
format: aep.planning-md/3
id: task:validate-builds-one-view-per-command
kind: task
status: active
title: A validate command builds one candidate view and each revision one edge index
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T08:12:46Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-30T08:12:46Z", actor: "human:timo", revision: 3}
---
## Context

Wave sdk-03 unit P made each validation build one candidate view (was five) and each commit apply
the head graph once (was twice). Its adversary pass (2026-09-30) found three things left:

- One `ekr validate` command still runs the validation pipeline twice: once to decide and again in
  the replay that admits the publication (`crates/ekr-kernel/src/replay.rs:840`), so it builds two
  candidate views. This predates the wave; the base built ten.
- The per-edge assertion index (`crates/ekr-kernel/src/validate/candidate.rs:126`, used at
  `reference.rs:197`) is rebuilt for every validation by a full pass over every assertion, so the scan
  `task:candidate-built-once-per-validation` named remains, with an allocation per edge assertion.
- No test observes the decided graph held in the kernel's thread-local
  (`crates/ekr-kernel/src/apply.rs:77`); removing its release keeps the suite green.

## Build

- The admitting replay reuses the decision's validation outcome for the same key instead of running
  the pipeline again, or the double run is shown to be required and the reason written into the
  design document.
- The per-edge index is kept with the graph it was built from, so a session builds it once per
  revision.
- A test observes that a graph a commit decided and did not take is released when the command ends.

## Acceptance

- One candidate view per `validate` command (the adversary's ignored case
  `a_validate_command_builds_its_candidate_view_once` passes).
- The per-edge index is built once per revision in a session (a counter).
- The release test fails when the release is removed.
- Refusals and roots byte-identical (the perf-01 differential and the replay cases).
