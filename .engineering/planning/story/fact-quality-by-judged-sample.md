---
format: aep.planning-md/3
id: story:fact-quality-by-judged-sample
kind: story
status: draft
title: Fact quality is reported from a judged, reproducible sample with a Wilson interval
relations:
- serves: vision:o6
- decomposes: epic:p6-maintenance-observability
scope:
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: crates/ekr-metrics
- confidence: cited
  path: crates/ekr/src/cli/agent.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: cited
  path: crates/ekr/src/cli/session.rs
- confidence: cited
  path: crates/ekr/tests/docs_cli.rs
- confidence: cited
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/components.yaml
- confidence: inferred
  path: systems/ekr/domains/metrics.yaml
- confidence: inferred
  path: systems/ekr/system.yaml
revision: 4
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

Derived 2026-09-29 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr/src/cli` — cited, the story builds "a read" and "a verb", and every verb is a `Command` variant in `crates/ekr/src/cli/mod.rs`
- **Files:** `crates/ekr/src/cli/mod.rs` — cited, the `Command` enum (lines 107–296) and its dispatch
- **Files:** `crates/ekr/src/cli/session.rs` — cited, the match at lines 230–250 lists every `Command` variant with no wildcard
- **Files:** `docs/cli.md`, `crates/ekr/tests/docs_cli.rs` (verb list, line 1139), `crates/ekr/src/cli/agent.rs` (guide verb table, line 58) — cited
- **Also likely:** `crates/ekr/src/cli/sample.rs` — inferred, one module per verb
- **Also likely:** a new `crates/ekr-metrics/` crate and `systems/ekr/domains/metrics.yaml` (`ekr.metrics`), with `system.yaml`, `components.yaml`, a metrics conformance suite, baseline and provenance and a `conform-fresh` line in `Taskfile.yml` — inferred from epic P6 and `docs/roadmap.md:83`; the fallback is `views.yaml` and `views-suite.json`
- **Also likely:** `crates/ekr-views/src/query.rs`, `crates/ekr-kernel/src/read.rs` — inferred, read for assertions and evidence bytes
- **Confidence:** medium — the two verbs and the CLI and docs files follow from the story and the tree; crate, domain and suite are inferred
- **Would collide with:** any unit adding a `Command` variant; any unit creating `ekr-metrics` or `ekr.metrics` (`story:store-quality-report`, probably `story:validation-findings-read`)
- **Safety fact:** the sample is a pure function of (seed, size, type filter, revision) and the Wilson figure is arithmetic over a caller-supplied judged document; invariants 1 and 7 are not touched — unproven
