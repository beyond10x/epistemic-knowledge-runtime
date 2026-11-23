---
format: aep.planning-md/3
id: story:ekr-test-step-budget
kind: story
status: active
title: Test step within 120 s
summary: The test step ends within 120 s on CI and is enforced
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T23:15:42Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T23:15:42Z", actor: "human:timo", revision: 3}
---
## Outcome

EKR's test step ends within 120 s on CI and fails when it runs longer; no test runs over 60 s.

## Work

The CI "Repository correctness" test step ran 498.6 s for 2598 tests on release PR #112 (run
45260269611) and hit its 600 s global timeout on PR #113 (run 45283467352: 2484 of 2599 run;
`conformance::each_injected_kernel_defect_fails_exactly_its_named_scenarios` 34.7 s, then killed at
60 s). The workspace rule is 120 s per step and 60 s per test.

- Find where the 498 s go: the per-test times in the CI log (nextest prints each test's time), summed
  per binary and per module; the ten slowest; and whether an expensive step runs per test (spec
  synthesis, a store seeded per test, a browser per test, a server per test).
- Make each expensive product once per run (a committed generated file with a digest check, or one
  fixture built once per run keyed by a digest under `target/`; an in-process global does not count
  under nextest).
- Set the step's global timeout to 120 s once it fits.

## Acceptance

The CI test step prints a wall time under 120 s and fails over 120 s; no test over 60 s; the ten
slowest tests are printed.
