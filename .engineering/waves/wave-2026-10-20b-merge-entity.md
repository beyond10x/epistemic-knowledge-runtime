# Wave 2026-10-20b — MergeEntity applies, built on the supersession record

Skill: aep:implementing 0.23.2 (wave mode). Coordinator: the Claude session of waves 2026-10-19a and
2026-10-20a.
Status: **dispatching** U1.

Source: `story:merge-entity-applies`, designed by ADR 0018 (accepted 2026-10-19 with option A and its
six defaults), on ADR 0017's supersession record (release 0.0.39, fixed in 0.0.40).

Approval: the operator answered "A" in this session on 2026-10-20 to the proposal
`b10x-chef-orchestrator` received: the unit's commits on `unit/20261020b-u1`, its merge into
`wave/20261020b`, the store commits, the push, one pull request, the bot merge into `main` on green
CI, and release 0.0.41 (a `Release 0.0.41` commit, the bot tag on the merge commit, the GitHub
release). Reports go to `b10x-chef-orchestrator` at each step boundary.

Base: `main` at `efe53653` (release 0.0.40). Integration branch `wave/20261020b`.

Width: 1 unit. Scope from `aep:story-scoper` at `efe53653`, written into the story with its typed
entries (25 cited, 17 inferred). The coordinator's four decisions on the scoper's open questions are
the story's "Coordinator decisions" section.

## Units

| unit | story | main surface | dispatch |
|---|---|---|---|
| U1 | `story:merge-entity-applies` | `ekr-kernel` validate/apply/replay, `ekr-graph` supersession record and lineage, three spec files and their suites, views changes, SDK and CLI mirrors, documents | `aep:implementor` |

## Trees

| unit | branch | worktree id | build dir | scratch | state |
|---|---|---|---|---|---|
| integration | `wave/20261020b` | `ekr-w20261020b` | none | none | store writes |
| U1 | `unit/20261020b-u1` | `ekr-w20b-u1` | `<tree>/target` | `<home>/.cache/ekr-w20b-scratch` | dispatched |

## Pre-flight

| check | reading |
|---|---|
| builds | every cargo command through `b10x-build-slot` (3 machine-wide, none under 20G free); disk and accounts are `b10x-chef-orchestrator`'s, so no watch of this wave's own |
| build settings | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, this repository's sccache on port 4302 |
| last comparable cost | wave 2026-10-20a: one full `cargo test --workspace` writes about 45G of test binaries |

## Gate

Not run yet.
