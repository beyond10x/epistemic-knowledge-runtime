# Wave 2026-10-20b — MergeEntity applies, built on the supersession record

Skill: aep:implementing 0.23.2 (wave mode). Coordinator: the Claude session of waves 2026-10-19a and
2026-10-20a.
Status: **gated locally**; U1 merged into `wave/20261020b`; `story:merge-entity-applies`
implemented; the pull request's CI runs the full gate again before the merge into `main`.

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
| U1 | `unit/20261020b-u1` | `ekr-w20b-u1` | `<tree>/target` | `<home>/.cache/ekr-w20b-scratch` | implementor `05bd7050`, correction 1 `5b59a139`; merged |

## Reviews

| pass | record | findings | outcome |
|---|---|---|---|
| adversary 1 on `05bd7050` | `review-result:adversary-w20261020b-u1-pass-1` | 6 introduced | F1, F2, J2, J3, J4 fixed in `5b59a139`; J1 filed as `story:explain-names-the-merge` (no-op) |
| adversary 2 on `5b59a139` | `review-result:adversary-w20261020b-u1-pass-2` | 1 note (N1) | coordinator's decision: no change (no-op); 12 green cases kept in scratch |
| haiku panel (10, operator's request) on PR head `7a7e885b`, before the merge | `review-result:haiku-panel-pr-101-merge-entity` | 26: 3 blocker, 12 warning, 11 note | H1 to H5 fixed on `wave/20261020b` (2 blockers: a created-and-deleted edge in the merge check, per-merge checks before the more-than-one-merge refusal; guide text; module doc); H6 to H12 filed as `story:merge-entity-review-follow-ups` |

The panel's fixes were gated in the integration tree through `b10x-build-slot`: `cargo fmt --all -- --check`
and `cargo clippy --workspace --all-targets --locked -- -D warnings` exit 0; `cargo test -p ekr-kernel` and
the `ekr` CLI suites `agent_cli`, `docs_cli`, `adversary_p4_01_agent_cli`, `conformance` and
`adversary_w20261020b_merge_cli`: 99 binaries, 760 passed, 0 failed. The two new kernel cases failed on
`7a7e885b` first (one Validated where a rejection was expected, one with an extra `merge-types-unrelated`).
The pull request's `Repository correctness` job runs `task check` whole.

## Incidents

- The implementor stopped five times on HTTP 429 (session and weekly limits); each time the account
  was switched by `b10x-chef-orchestrator` and the agent was resumed with its context.
- `/` fell to 13G free while the implementor was stopped; the coordinator stopped its orphaned
  `cargo test -p ekr-views` and discarded the tree's cache (11,233 MiB) on the orchestrator's
  request; the next build was a full one.
- Sizing of the operator's ess-upgrade wish (ess 0.66.0) refused 52, 1021 and 5 times in the three
  syntheses; it is `story:ekr-on-ess-0-66`, outside this wave and release 0.0.41.

## Pre-flight

| check | reading |
|---|---|
| builds | every cargo command through `b10x-build-slot` (3 machine-wide, none under 20G free); disk and accounts are `b10x-chef-orchestrator`'s, so no watch of this wave's own |
| build settings | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, this repository's sccache on port 4302 |
| last comparable cost | wave 2026-10-20a: one full `cargo test --workspace` writes about 45G of test binaries |

## Gate

On `5b59a139` in the U1 tree, through `b10x-build-slot`: `cargo fmt --all -- --check`,
`cargo clippy --locked --workspace --all-targets -- -D warnings`, `task spec-check`,
`task conform-fresh` (kernel 80, views 110, integrate 2) and `task doc-check` exit 0;
`cargo test --locked --workspace --no-fail-fast` 2578 passed, 0 failed, 13 ignored (implementor), and
2590 passed, 0 failed with adversary pass 2's 12 cases (`ekr-ontology` included). `task site-check`
and `vendor-check` were not run locally; the pull request's `Repository correctness` job runs
`task check` whole.
