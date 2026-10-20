---
format: aep.planning-md/3
id: story:ekr-on-ess-0-66
kind: story
status: draft
title: The repository's ess pin moves to 0.66.0 with its suites regenerated
relations:
- decomposes: epic:p0-bootstrap
- serves: vision:o2
revision: 1
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

## Out of scope

Any behaviour change in the runtime.
