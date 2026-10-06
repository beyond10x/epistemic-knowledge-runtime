---
format: aep.planning-md/3
id: specification:wave-20261005b-extraction-parity
kind: specification
status: implemented
title: 'Wave 20261005b: extraction parity'
relations:
- decides: story:extraction-partial-apply
revision: 7
transitions:
- {from: "draft", to: "in_review", at: "2026-10-05T11:51:15Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-05T11:51:15Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-06T06:08:55Z", actor: "agent:claude", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
---
## Wave 20261005b: extraction parity

Opened 2026-10-05 by the coordinating session, `aep:implementing` 0.19.2 wave mode. It clears the cb3 upstream blocker `ekr-extraction-parity` (valid time, supersession, partial apply, relations as graph edges) and adds `ocel-process-map`.

**Approval:** the operator approved every wave up front on 2026-10-05: "I approve all waves upfront and now. do not ask for permission, you orchestrate this". Option A of the proposal: one wave, with unit A running the four extraction stories in order.

## Units

| unit | stories, in order | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| A | `story:extraction-partial-apply` → `story:extraction-valid-time` → `story:extracted-relations-visible-to-graph-reads` → `story:extraction-supersession` | `unit/extraction-parity` | `ekr-wx-a` | `~/.cache/b10x-target/ekr-wx-a` | `~/.cache/ekr-wave-8c9138d1/a` |
| B | `story:ocel-process-map` | `unit/ocel-process-map` | `ekr-wx-b` | `~/.cache/b10x-target/ekr-wx-b` | `~/.cache/ekr-wave-8c9138d1/b` |

## Selection

- The four extraction stories all change `Run::go` in `crates/ekr-sdk/src/extraction.rs` (their Scope sections), so they run as one sequence in one unit, one commit per story. `story:extraction-supersession` depends on partial-apply and valid-time (store edges).
- `story:ocel-process-map` shares only `crates/ekr/src/cli/mod.rs`, `docs/cli.md` and `CHANGELOG.md` with unit A, in different hunks; the coordinator resolves that overlap when merging.
- Each unit has its own build directory, as `AGENTS.md` § The gate requires for concurrent trees.
- At most two adversary passes per unit.

## Commits approval authorises

One commit per story in unit A and one in unit B, each through `b10x-gates bot`; the merges of both units into `wave/20261005b`; the closing planning-store commit; the pull request into `main` and its merge; the EKR release that follows, through the repository's own release process.

## Unit C (added 2026-10-05)

## Unit C (added 2026-10-05)

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| C | `story:seed-if-absent` | `unit/seed-if-absent` | `ekr-wx-c` | `~/.cache/b10x-target/ekr-wx-c` | `~/.cache/ekr-wave-8c9138d1/c` |

Added under the operator's standing approval of every wave. It touches the seed path, not `Run::go`, so it runs beside unit A; forked from `427a211` (unit B merged).

## Shared with later stories

`story:read-answers-name-types` and `story:explain-bounds-documents` (planned 2026-10-06, not in this wave) share `docs/cli.md` and `CHANGELOG.md` with every unit here, and `systems/ekr/domains/views.yaml` with `story:ocel-process-map`. Both land after this wave merges; `story:explain-bounds-documents` carries a `depends_on` edge to `story:ocel-process-map`.

## Outcome

Closed 2026-10-06. Three units merged into `wave/20261005b`; the repository gate (`task check`) passed in the pull request's CI, run 37419743657 ("Repository correctness").

| unit | stories | commit | review |
|---|---|---|---|
| A | `extraction-partial-apply`, `extraction-valid-time`, `extraction-supersession`, `extracted-relations-visible-to-graph-reads` | `4dc3ddfa2` | two adversary passes (7 and 3 findings, all fixed) |
| B | `ocel-process-map` | `a929e666` | coordinator review |
| C | `seed-if-absent` | `9d1338583` | one adversary pass (2 findings: 1 fixed, 1 kept as documented) |

The gate ran in CI because a local `task check` on this machine needs about 12 GB of build output with debug info off, and the shared disk had under 10 GB free: the local attempt failed with "No space left on device". A coordinator gating this repository locally checks `df -h /` for at least 15 GB first.
