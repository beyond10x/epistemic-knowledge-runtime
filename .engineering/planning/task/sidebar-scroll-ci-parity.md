---
format: aep.planning-md/3
id: task:sidebar-scroll-ci-parity
kind: task
status: active
title: Preserve sidebar scroll position across CI browser collapse and restore
relations:
- informed_by: task:viewer-compact-mode
- informed_by: story:live-search-agent-entry
- serves: vision:o5
- decomposes: story:live-search-agent-entry
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T12:49:30Z", actor: "agent:codex", revision: 2, decided_on: {"recorded":{"test_result":1}}}
- {from: "proposed", to: "active", at: "2026-10-03T12:49:30Z", actor: "agent:codex", revision: 3, decided_on: {"recorded":{"test_result":1}}}
---
## Observed failure

The required Repository correctness job on candidate 24f7714475c363701c93b07bf6782cc5ce46d9bf fails `a_scrolled_sidebar_keeps_its_scroll_position_through_collapse_and_restore` in `crates/ekr/tests/adversary_viewer_compact.rs`. The initial run and its single unchanged-source retry both report left scroll position 137 after restoration versus 120 before it; the right stays 40. Evidence: https://github.com/beyond10x/epistemic-knowledge-runtime/actions/runs/37122410085 (attempts 1 and 2). No more blind retries.

The same-source local full-gate target and bounded direct headless-shell repetitions passed. A local full-Chromium comparison stopped earlier in browser discovery because no page target was listed yet. It did not exercise scroll preservation. These results do not establish a cause or a fix. Neither the existing compact test nor graph page changed from the pull request base.

## Engine or instance

This is generic browser/viewer behavior and its test harness. No consumer identity, host, store or data is involved. Work belongs in EKR; use only the existing synthetic sounding fixture.

## Diagnosis and acceptance

Follow loop-first diagnosis: establish a fast local reproducer with the same symptom and distinguish browser-discovery failure from scroll failure. Then rank falsifiable hypotheses and change one variable at a time. Preserve each attempt. Do not relax the exact scroll equality, add skips, select a different browser merely to pass, or replace the failed behavior with a narrower claim.

If correction is needed, retain a regression that fails before the fix and passes afterwards. Re-run the original compact target and relevant viewer/search tests, then required CI on the integrated source. No authored JavaScript may be introduced, including strings hidden in Rust. Rust test/probe code and declarative layout corrections are the available implementation paths; any different language requirement is reported rather than inferred.

## Scope and ownership

Cited source: `crates/ekr/tests/adversary_viewer_compact.rs` and `crates/ekr/src/cli/viewer/index.html`. CI selects `/usr/bin/google-chrome` through `EKR_VIEW_BROWSER`; local headless-shell behavior is not parity proof. The coordinator owns this task's AEP record and integration. Diagnosis uses its own managed tree and scratch; the running release gate keeps the coordinator tree and existing target frozen. Reuse linked test binaries read-only where sufficient, and use a separate bounded target if compilation is necessary.

The source release stays pending until this repeated required-check failure is resolved and the integrated gate is verified.
