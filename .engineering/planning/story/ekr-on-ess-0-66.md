---
format: aep.planning-md/3
id: story:ekr-on-ess-0-66
kind: story
status: active
title: The repository's ess pin moves to 0.66.0 with its suites regenerated
relations:
- decomposes: epic:p0-bootstrap
- serves: vision:o2
scope:
- confidence: cited
  path: .github/workflows/correctness.yml
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: inferred
  path: crates/ekr-integrate/tests
- confidence: cited
  path: crates/ekr-views/tests/fixtures/conformance
- confidence: cited
  path: crates/ekr/tests/fixtures/conformance
- confidence: cited
  path: systems/ekr/conformance
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T00:27:15Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-21T00:27:15Z", actor: "human:timo", revision: 6}
---
## Context

The operator's standing wish of 2026-10-20, relayed by `b10x-chef-orchestrator`: every release moves
the repository's `ess` pin to the newest `ess` release in the same release pull request. This
repository pins `ess` 0.36.0 (`Taskfile.yml` `spec-check`, `conform-check`, `conform-fresh`
preconditions; `.github/workflows/correctness.yml` `ESS_VERSION: "0.36.0"`). The newest release on
2026-10-20 is 0.66.0 (https://github.com/beyond10x/ess/releases/tag/0.66.0).

Sized on 2026-10-20 by the coordinator of wave 2026-10-20b with a private copy of the 0.66.0 release
binary (`ess-0.66.0-x86_64-unknown-linux-gnu.tar.gz`, SHA256SUMS check OK), on the specification of
`wave/20261020b` at `2a843eb6` plus its store commits; logs under
`<home>/.cache/ekr-w20b-scratch/ess066/`:

| command | exit | refusals by code | first message |
|---|---|---|---|
| `ess specify validate --path systems/ekr` | 0 | none; 25 warning or help lines | `ekr v1 — 10 file(s), valid` |
| `ess conform synthesize --path systems/ekr --component ekr-kernel --suite-format 5 --target ir --scenarios crates/ekr/tests/fixtures/conformance/scenarios` | 1 | 52: 47 `ESS-AUTHOR-042`, 3 `ESS-SYNTH-003`, 2 `ESS-SYNTH-004` | `ESS-AUTHOR-042`: the payload of `ekr.kernel.RevisionCommitted` pins `number` to 2, a value `ekr.kernel.Commit/committed` marks `{generated: true}` |
| `ess conform synthesize … --component ekr-views … --scenarios crates/ekr-views/tests/fixtures/conformance/scenarios` | 1 | 1021: 1015 `ESS-AUTHOR-042`, 1 `ESS-AUTHOR-041`, 3 `ESS-SYNTH-003`, 2 `ESS-SYNTH-004` | `ESS-AUTHOR-042` on `a-cursor-pages-a-neighbourhood-to-its-end`; the pinned generated values are mostly revision numbers 0–4 |
| `ess conform synthesize … --component ekr-integrate` | 1 | 5: 3 `ESS-SYNTH-003`, 2 `ESS-SYNTH-004` | `ESS-SYNTH-003`: entity `ekr.kernel.GraphTransaction` has no scenario `ekr.kernel.GraphTransaction/invariant/after/ekr.kernel.Commit/stale` |

So the move is not a pin change: authored scenarios that pin `{generated: true}` fields are rewritten
(about 1,060 expectations), the `Commit/stale` branch is made reachable or given a fixture, and the
three committed suites and their baselines are regenerated.

## Build

1. Rewrite the authored kernel and views scenarios so no expectation pins a generated field (drop it,
   or assert presence and type), as `ESS-AUTHOR-042`'s help says; fix the `ESS-AUTHOR-041` case.
2. Make `ekr.kernel.Commit/stale` and the two `ESS-SYNTH-004` states reachable by a candidate, or
   supply fixtures.
3. Move the pin to the newest `ess` in `Taskfile.yml` and `correctness.yml` (version and checksum);
   regenerate the three suites and their baselines.

## Acceptance

- `task spec-check` and `task conform-fresh` exit 0 under the new pin; `task conform-check` (the
  conformance runners) exit 0; the executed scenario counts are not lower than under 0.36.0 (kernel
  80, views 110, integrate 2 after wave 2026-10-20b).

## Coordinator decisions (2026-10-21)

Approved by `b10x-chef-orchestrator` under the dependency-upgrade grant, "on the side that changes
no behaviour":

1. `ekr.kernel.Commit/stale` and the two `ESS-SYNTH-004` states it routes: reached by a fixture or
   an authored scenario. The branch's condition in `systems/ekr/domains/kernel.yaml` is not
   rewritten (a changed condition is a specification change near ADR 0102, still proposed). If the
   branch is unreachable by design, the unit stops and says so.
2. `ESS-AUTHOR-041` on `a-superseded-node-is-refused-naming-the-node-it-is-superseded-by`: the
   scenario's input changes so it passes the earlier guard; the guard order in
   `systems/ekr/domains/views.yaml` stays.
3. `ESS-AUTHOR-042`: an expectation pinning a `{generated: true}` field drops the value or asserts
   presence and type; no specification field loses `generated`.
4. The pin moves to 0.66.0 (the newest release on 2026-10-21): `Taskfile.yml` preconditions and
   `correctness.yml` `ESS_VERSION` / `ESS_SHA256`
   (`aa6b209efb5f9545c85fc31d3b66363d10be208c5bcdeaba61be0d54c50e17f6`, the release's SHA256SUMS
   line for `ess-0.66.0-x86_64-unknown-linux-gnu.tar.gz`). The `ess-conformance` and
   `ess-primitives` crate revisions in `Cargo.toml` move only if the regenerated suites need it.
5. No release: the wave lands on main by a bot fast-forward of the green head.

## Out of scope

Any behaviour change in the runtime; any change to a specification branch condition or guard order.
