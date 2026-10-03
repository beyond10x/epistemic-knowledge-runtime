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
  path: crates/ekr-ontology/tests/yaml_ingress.rs
- confidence: cited
  path: crates/ekr/src/cli/view.rs
- confidence: inferred
  path: crates/ekr/tests/search_page.rs
- confidence: inferred
  path: crates/ekr/tests/view_cli.rs
- confidence: inferred
  path: docs/cli.md
revision: 9
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

## Cross-crate inventory correction

The required CI run for source candidate 033817eec8b4f5defcf3c79dcb968deeefa619aa failed `every_production_yaml_reader_has_an_explicit_input_policy`; the local full gate reproduced the same failure. The new historical-membership fixture serializes a synthetic transaction inside the viewer's cfg(test) module. The YAML guard inventories test-only source-module references too, but that writer had not been classified. Add its exact single `to_string` entry beside the other test-only fixture writers, retaining the guard's refusal of additional readers/imports. No production reader, input bound or runtime behavior changes in this correction.

The first local full-gate process was stopped after the reproduced failure, with observed exit 143 and no surviving process-group members; its incomplete log is preserved under `<cache>/ekr-evidence-cache/gate/before-inventory-correction`. It is not a completed or passing full gate. Focused inventory verification and a fresh full gate are required on the corrected candidate.
