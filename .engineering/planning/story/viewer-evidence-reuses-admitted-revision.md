---
format: aep.planning-md/3
id: story:viewer-evidence-reuses-admitted-revision
kind: story
status: active
title: Serve retained evidence from the admitted viewer revision
relations:
- informed_by: story:hosted-read-serving
- serves: vision:o2
scope:
- confidence: cited
  path: crates/ekr/src/cli/view.rs
- confidence: inferred
  path: crates/ekr/tests/search_page.rs
- confidence: inferred
  path: crates/ekr/tests/view_cli.rs
- confidence: inferred
  path: docs/cli.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T06:18:03Z", actor: "agent:codex", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-03T06:19:09Z", actor: "agent:codex", revision: 7}
---
## Acceptance

A retained-evidence request for a revision already admitted by the viewer returns the exact retained bytes without an additional seed replay, while missing revisions, missing evidence, changed stores and invalid retained content keep their existing refusals.

## Context

The viewer routes search and graph reads through its revision index cache, but its evidence handler calls the kernel's selected-revision read directly. The selected replay path deliberately reconstructs from the seed. A pinned evidence link can therefore repeat work even when that same viewer already holds the verified revision. This is a generic serving defect; no consumer data is needed to reproduce it.

## Scope

- Cited: `crates/ekr/src/cli/view.rs`, evidence dispatch, evidence handler and existing viewer tests.
- Inferred: `docs/cli.md`, only if a clarification of existing cache behavior is needed.
- Cited read surface: `crates/ekr-views/src/index.rs`, the revision cache and loaded graph accessors.
- Cited read surface: `crates/ekr-kernel/src/read.rs` and `replay.rs`, selected read and replay work counters.
- No provider, canonical history, kernel replay semantics, transport deadline or admission bypass is in scope.

## Verification

First add and run a deterministic regression at the viewer request seam. Warm a verified revision through a normal viewer request, follow its pinned evidence request, assert exact bytes and unchanged seed-replay count. The test must fail on the current implementation; elapsed wall time is not the assertion. Preserve unseeded, malformed, missing, historical and replacement behavior in the existing suite. Package tests, formatter and lint precede independent adversarial review and the full repository gate.

## Objective and authority

This serves evidence access and the generic agent platform. The operator's standing authorization covers implementing and publishing upstream capabilities needed for hosted read serving, including a corrective release after verification. This story introduces no new domain entity or wire format.
