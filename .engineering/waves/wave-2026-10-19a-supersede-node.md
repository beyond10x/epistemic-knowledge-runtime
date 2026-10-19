# Wave 2026-10-19a — a duplicate node is superseded by the node that replaces it

Skill: aep:implementing 0.23.2 (wave mode). Coordinator: a Claude session the operator started on
2026-10-19 for this one change.
Status: **gated**: U1 merged into `wave/20261019a` at `a3d90018`; the full gate is green on it;
`story:supersede-node` is implemented. Pull request from `wave/20261019a`; no merge into `main`.

Source: a consumer's ingestion wrote duplicate nodes and needs one released write that retires a
node in favour of another (`story:supersede-node`). The decision between finishing `MergeEntity`
and a smaller operation is `architecture-decision-record:0017-a-node-is-superseded-by-another`,
accepted under the operator's delegation.

Approval: the operator authorised, in the session's opening instruction, planning this change,
implementing it in a managed worktree with its tests, committing on a branch and opening a pull
request through the bot route; no tag, version bump or release. That instruction is the stage-1
approval. It authorises: the unit's commits on `unit/20261019a-u1`, its merge into
`wave/20261019a`, the store commits on `wave/20261019a`, the push of `wave/20261019a` and one
pull request from it. Not a merge into `main`, a tag, a version bump or a release.

Base: `main` at `2dd09407` (release 0.0.38). Integration branch `wave/20261019a`.

Width: 1 unit. The planning critic panel was not run: one story sits under the decision, and the
panel compares items with each other.

Selection: `aep plan artifact waves --kind story --status draft --format json` (aep 0.74.0) after
the scope entries were written: 4 waves, 23 collisions, 20 unassessed, 0 cycles.
`story:supersede-node` sits in its wave 3, on 47 paths, 41 cited. Its 15 collisions are with three
drafts no session has taken up: `story:describe-type-read` (`CHANGELOG.md`, `ekr-views/src/query.rs`,
`ekr/src/cli/mcp.rs`, `ekr/src/cli/view.rs`, `docs/cli.md`, `views.yaml`, all inferred),
`story:property-constraints-are-enforced` (`ekr/src/cli/agent.rs` and `docs/cli.md` cited,
`ekr/tests/docs_cli.rs` inferred) and `story:read-answers-name-types` (the same six paths as the
first, inferred). The operator asked for this one change, so no other story joins the wave.

Concurrent work in this repository, not this wave's: wave 2026-10-17b (`wave/20261017b`,
`unit/20261017b-u1`, issue 96 historical changes reads) in trees `ekr-plan-96` and `ekr-w17b-u1`,
and `fix/leak-scrub-20261019`. Wave 17b specifies a read in `views.yaml` and may touch `docs/cli.md`
and `CHANGELOG.md`; whichever pull request lands second resolves those files.

## Units

| unit | story | main surface | spec change | dispatch |
|---|---|---|---|---|
| U1 | `story:supersede-node` | `crates/ekr-kernel` (validate, apply, replay), `crates/ekr-graph` (`CanonicalGraph`, `AliasIndex`), `crates/ekr-store` (knowledge root, checkpoint serde), `crates/ekr-integrate`, `crates/ekr-views`, `crates/ekr` | `kernel.yaml`, `graph.yaml`, `integrate.yaml`, `views.yaml` | `aep:implementor` |

## Trees

| unit | branch | worktree id | build dir | scratch | state |
|---|---|---|---|---|---|
| integration | `wave/20261019a` | `ekr-w20261019a` | none: this tree does not build | none | store writes |
| U1 | `unit/20261019a-u1` | `ekr-w19a-u1` | `<tree>/target` | `<home>/.cache/ekr-w19a-u1-scratch` | merged at `a3d90018`: implementor `7f0deb56`, correction 1 `35fe0874`, correction 2 `1a716c1a`; adversary scratch `<home>/.cache/ekr-w19a-u1-adversary`; this tree ran the full gate |

Worktree paths: `<worktrees>/b10x/epistemic-knowledge-runtime/<worktree id>`.

## Pre-flight

| check | reading |
|---|---|
| free disk on `/` | 20G (`df -h /`, 2026-10-19); floor 10G (operator rule) |
| build cache | `RUSTC_WRAPPER=sccache`, `SCCACHE_DIR=<home>/.cache/sccache/epistemic-knowledge-runtime`, port 4302, 2G cap (AGENTS.md); the directory did not exist before this wave |
| pinned tools | `ess` 0.36.0 and `aep` 0.64.0 from `<home>/.cache/epistemic-knowledge-runtime-tools/*/bin`, first on the path for `spec-check`, `conform-fresh`, `plan-check` |
| one build | not measured before dispatch; only one tree builds, and the full gate runs in the U1 tree on the integrated head |

## Incidents

- During the implementor's base run free space on `/` fell to 65M and once to 0 B (ENOSPC), as the
  implementor reported; its build directory was 26G, other sessions the rest; it deleted nothing and
  stopped its run. Free space later read 110G (`df -h /`). `<tree>/target` read 43G at one point
  (`du -sh`). The base before-counts therefore come from `cargo test -- --list`, not from a run.

## Reviews

| pass | record | findings | outcome |
|---|---|---|---|
| adversary 1 on `7f0deb56` | `review-result:adversary-w20261019a-u1-pass-1` | 5: 4 introduced, 1 undecided | F1–F4 fixed in `35fe0874`; J1 filed into `story:merge-entity-applies` (no-op) |
| adversary 2 on `35fe0874` | `review-result:adversary-w20261019a-u1-pass-2` | 4 introduced; carried 0, new 4, resolved 5 (`aep plan artifact findings`) | F5, F6, N1, N2 fixed in `1a716c1a` |

The coordinator read the correction-2 diff against `35fe0874`: no assertion removed, no `#[ignore]`
added, the only removed test lines raise a count (views generated scenarios 58 → 59, views baseline
total and floor 108 → 109). No third attack.

## Gate

The full gate ran once on the integrated head `a3d90018`, in the U1 tree's own `target/`, with the
pinned `ess` 0.36.0 and `aep` 0.64.0 first on the path, one exit status per step
(`<home>/.cache/ekr-w19a-u1-scratch/gate-a3d90018/status.txt`): `fmt-check`, `search-web-check`,
`clippy`, `test`, `doc-check`, `vendor-check`, `spec-check`, `conform-fresh`, `plan-check` and
`site-check` each `EXIT=0`. `test`: 2526 passed, 0 failed, 13 ignored, summed over its
`test result:` lines. `cargo test --workspace -- --list` in the same tree lists 2537 tests, 63 of
them adversary cases. The cross-crate checks of `COORDINATOR.md` check 9 (`cargo fmt --all --
--check`; `cargo test -p ekr --test story_contract --test public_surface --test docs_cli --test
agent_cli`) each `EXIT=0`; workspace clippy is the `clippy` step.

## Cost

| agent | tokens | tool uses | wall |
|---|---|---|---|
| read-surface survey (`Explore`) | 117,716 | 56 | 4.4 min |
| `aep:story-scoper` | 97,436 (after a 429 resume) | 10 | 2.1 min (final leg) |
| `aep:implementor` (dispatch and two corrections) | 831,461 + 478,992 + 605,780 | 542 + 61 + 130 | 190 + 47 + 81 min |
| `aep:adversary` pass 1 | 274,266 | 109 | 49 min |
| `aep:adversary` pass 2 | 270,792 | 106 | 67 min |
| ADR 0018 draft (`Plan`) | 100,354 | 44 | 6.8 min |
