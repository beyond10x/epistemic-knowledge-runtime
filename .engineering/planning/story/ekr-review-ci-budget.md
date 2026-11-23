---
format: aep.planning-md/3
id: story:ekr-review-ci-budget
kind: story
status: active
title: The EKR test step ends within 120 s and prose-only pull requests skip the build
summary: Findings 37 and 38 of the EKR 0.0.42 review
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T08:13:54Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T08:13:55Z", actor: "human:timo", revision: 3}
---
## Outcome

The test step of `task check` and of CI ends within 120 s, no test runs longer than 60 s, and a
pull request that changes only planning or prose files passes its required checks without building.

## Work

Findings 37 and 38 of the external review of EKR 0.0.42 (2026-11-23):

- 37: the release test task took 30m47s against the 2 min budget: the workspace test command took
  28m38s, followed by 2m09s for a compile-only step.
- 38: pull request 107 changed one prose file (`AGENTS.md`); its test task alone ran 35m26s and the
  correctness job 42m22s.

- Measure first: the ten slowest tests and the number of test binaries; find what is built or run
  per test that could be built once (`~/beyond10x/AGENTS.md` "Tests stay fast").
- One integration-test binary per crate; no debug info in test builds; the runner kills a test over
  60 s, fails the step over 120 s and prints the ten slowest, locally and in CI.
- CI keeps its required checks and exits 0 early when every changed path is planning or prose
  (`.engineering/`, `*.md` outside the built site), with a test for both sides of the path list.

## Acceptance

The gate's test step prints its wall time under 120 s and its ten slowest tests; a prose-only pull
request's required checks pass without building or testing.
