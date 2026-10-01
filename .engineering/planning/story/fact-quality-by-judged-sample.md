---
format: aep.planning-md/3
id: story:fact-quality-by-judged-sample
kind: story
status: implemented
title: Fact quality is reported from a judged, reproducible sample with a Wilson interval
relations:
- serves: vision:o6
- decomposes: epic:p6-maintenance-observability
scope:
- confidence: inferred
  path: crates/ekr-views/src/lib.rs
- confidence: inferred
  path: crates/ekr-views/src/sample.rs
- confidence: inferred
  path: crates/ekr-views/tests/sample.rs
- confidence: inferred
  path: crates/ekr-views/tests/support/target.rs
- confidence: cited
  path: crates/ekr/src/cli/agent.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: inferred
  path: crates/ekr/src/cli/sample.rs
- confidence: cited
  path: crates/ekr/src/cli/session.rs
- confidence: cited
  path: crates/ekr/tests/agent_cli.rs
- confidence: inferred
  path: crates/ekr/tests/sample_cli.rs
- confidence: cited
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/conformance/baseline.json
- confidence: inferred
  path: systems/ekr/conformance/suite.json
- confidence: inferred
  path: systems/ekr/conformance/views-baseline.json
- confidence: inferred
  path: systems/ekr/conformance/views-provenance.json
- confidence: inferred
  path: systems/ekr/conformance/views-suite.json
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T12:26:22Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-09-30T18:15:40Z", actor: "human:timo", revision: 10}
- {from: "active", to: "implemented", at: "2026-10-01T18:09:55Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

A consumer instance judges the quality of extracted facts by drawing a seeded sample, having each
fact judged against its cited evidence, and reporting the pass rate with a Wilson interval; the
consumer sets the sample size and the bar (reported 2026-09-29). Epic P6 plans epistemic health
metrics; invariant 7 keeps deterministic validators model-free, so the judging itself belongs to
an agent, not to a validator.

## Build

A read that draws a reproducible sample of assertions from a revision (seed, size, optional type
filter) and prints each with its evidence bytes, and a verb that takes the judged sample back and
reports the pass rate with its Wilson interval at a stated confidence. The runtime does no
judging.

## Acceptance

- The same seed, size and revision draw the same sample on both providers.
- For a judged fixture of known results the reported rate and interval equal the closed-form
  Wilson values.

## Scope

Derived 2026-09-30 by `story-scoper`, replacing the 2026-09-29 scope that assumed a new `ekr-metrics`
crate: `story:store-quality-report` (0.0.21, `582347de`) placed P6's metric in `ekr-views` and
`views.yaml`, and this story follows that precedent. Every line is **cited** or **inferred**.

- **Primary surface:** `crates/ekr/src/cli` — cited; every verb is a `Command` variant (`crates/ekr/src/cli/mod.rs:122`)
- **Files:** `crates/ekr/src/cli/mod.rs` (mod list :44, `Command` :122, `Access` :442, dispatch :736), `crates/ekr/src/cli/session.rs` (verb match :690–706), `crates/ekr/src/cli/agent.rs` (:63), `crates/ekr/tests/agent_cli.rs` (:818) — cited
- **Documents:** `docs/cli.md` (verb table :157, a verb section, session verb list :884) — cited
- **Also likely:** `crates/ekr/src/cli/sample.rs`, `crates/ekr-views/src/sample.rs`, `crates/ekr-views/src/lib.rs` (:54, :67) — inferred from the precedent
- **Also likely:** `systems/ekr/domains/views.yaml`, both conformance suites and baselines, `views-provenance.json` — inferred
- **Also likely (tests):** `crates/ekr-views/tests/support/target.rs`, a scenario under `crates/ekr-views/tests/fixtures/conformance/scenarios/`, `crates/ekr-views/tests/sample.rs`, `crates/ekr/tests/sample_cli.rs` — inferred
- **Read, not changed:** `Runtime::content` (`crates/ekr-kernel/src/read.rs:60`) — cited
- **Confidence:** medium — the CLI, guide and docs files are forced by exhaustive matches; the `ekr-views` placement is precedent
- **Would collide with:** any unit adding a `Command` variant; any unit changing `views.yaml` or regenerating the suites; `ekr-views/src/lib.rs` re-exports; `tests/support/target.rs`
- **Safety fact:** the sample is a pure function of (seed, size, type filter, revision) over one read, and the Wilson figure is arithmetic over a judged document the caller supplies, so invariant 7 is untouched — inferred
- **Not established:** a Wilson interval at an arbitrary confidence (no stats crate; `sha2` only); whether rates are basis points like `ekr quality` or floats; whether the judged document is a file argument or stdin; a new refusal name for a malformed judged document
