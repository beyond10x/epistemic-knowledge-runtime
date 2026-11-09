# Wave plan 2026-11-09 — zero open issues, the store's open defects fixed

Skill: aep:implementing 0.26.0 (wave mode), aep:planning 0.26.0, worktree:managing-worktrees.
Run: **non-interactive** (a sub-agent dispatch with no operator turn). Every bypassed stop is an
`approval-record` tagged `non-interactive`; they are listed under *Approvals*.

Source: `epic:zero-open-issues-2026-11`. Operator's goal, verbatim: "zero remaining issues on
github, all bugs fixed from planning store".

Base: `origin/main` at `57244640`. Integration branch `wave/20261109-zero-issues`, one pull request
for all three waves, no release.

## The plan: three waves

Stories were sequenced by `aep plan artifact waves --kind story --status active`. Its lines,
verbatim:

```
wave 1
  story:changes-since-by-time-within-deadline
  story:explain-names-the-merge (inferred)
  story:search-live-typing-waits-on-the-page (inferred)
wave 2
  story:view-historical-changes-and-cancelled-reads (inferred)
collision: story:changes-since-by-time-within-deadline story:view-historical-changes-and-cancelled-reads crates/ekr-views/src/changes.rs (inferred)
2 wave(s), 1 collision(s), 0 unassessed
```

Tasks were sequenced **by reading pairs**, because `aep plan artifact scope` (aep 0.79.0) refuses
a task: "`scope` is a field of `story` ... a task inherits the surface of the story it
decomposes". The seven tasks here decompose an epic, not a story, so the verb returns each as
`unassessed`. Their surfaces are the `## Scope` / `## Surface` sections the `aep:story-scoper`
agents returned on 2026-11-09, written into each body. Pairs the reading keeps apart:

| pair | shared surface |
|---|---|
| seed-authority, explain-names-the-merge | `systems/ekr/domains/kernel.yaml`, `systems/ekr/conformance/suite.json` |
| seed-authority, seed-race | `crates/ekr/tests/adversary2_p5_01_store_open.rs` |
| seed-authority, held-file-payload | `docs/epistemic-knowledge-runtime-design.md` |
| seed-authority, migration-marker | `crates/ekr-kernel/src/commit.rs` (inferred) |
| wall-clock, far-valid-times | `crates/ekr/tests/view_page.rs` |
| wall-clock, held-file-payload | `crates/ekr-store/src/eventlog.rs` |
| wall-clock, #96 | `crates/ekr/tests/view_cli.rs` |
| wall-clock, #104 | `crates/ekr/tests/search_live.rs` |

`CHANGELOG.md` is the coordinator's: no unit edits it.

| wave | unit | artifact | objective | scope |
|---|---|---|---|---|
| 1 | A | `story:search-live-typing-waits-on-the-page` (#104) | o5 | cited file, inferred page side |
| 1 | B | `story:explain-names-the-merge` | o2 | cited |
| 1 | C | `story:changes-since-by-time-within-deadline` | o5 | cited |
| 1 | D | `task:migration-marker-cannot-be-evidence` | o2 | cited; the scoper reads it as already fixed |
| 1 | E | `task:sdk-consumer-yaml-dependency` | o5 | cited manifests, fix shape open |
| 1 | F | `task:held-file-payload-overwrite` | o2 | cited read path, outcome open |
| 1 | G | `task:seed-race-test-names-its-signal` | o5 | cited; only the stress acceptance remains |
| 2 | H | `story:view-historical-changes-and-cancelled-reads` (#96) | o5 | cited route, inferred fix site |
| 2 | I | `task:seed-authority-refusal-cli-and-kernel-disagree` | o5 | cited |
| 2 | J | `task:streamed-page-shows-far-valid-times` | o5 | cited; holds off `docs/cli.md` |
| 3 | K | `task:no-test-asserts-wall-clock-cost` | o5 | cited |

Not in any wave, each stopped by an open typed blocker:
`story:supersession-verdict-fixes-replay-safe` and `story:merge-entity-review-follow-ups`
(`decision-blocker:rules-era-for-changed-verdicts`);
`task:replaced-store-close-keeps-the-restored-file`
(`dependency-blocker:eventlog-close-without-checkpoint`).

Dispatch: `aep:implementor` per unit, `aep:adversary` per returned unit (two passes at most).

Commits the approval authorises: one or more commits per unit on its branch, the merges into
`wave/20261109-zero-issues`, the store commits, the push of that branch, one pull request, and its
squash merge into `main` by the bot on green checks. No tag, no version bump, no release.

## Pre-flight (2026-11-09)

| check | reading |
|---|---|
| primary checkout | on `main`, 16 behind `origin/main`, one untracked directory `.agents/` (not this run's; left alone). Every tree is created from `origin/main` `57244640` |
| `git worktree list` | only the primary before this run |
| free disk | 80G on `/`; `b10x-build-slot` starts a build only above 20G |
| builds | every cargo command through `b10x-build-slot` (3 slots machine-wide, under the limit of 4) |
| build settings | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `RUSTC_WRAPPER=sccache`, `SCCACHE_DIR=<home>/.cache/sccache/epistemic-knowledge-runtime`, `SCCACHE_SERVER_PORT=4302`, `SCCACHE_CACHE_SIZE=2G` |
| last comparable cost | wave 2026-10-20b's page: one full `cargo test --workspace` writes about 45G of test binaries. Units therefore run named test targets only; the whole gate is the pull request's `Repository correctness` job |

## Trees

`<trees>` is the worktree manager's directory for this repository; `<scratch>` is this run's
scratch root for this repository.

| unit | branch | worktree id | build dir | scratch | state |
|---|---|---|---|---|---|
| integration | `wave/20261109-zero-issues` | `ekr-zero-issues-int` | none | `<scratch>/int` | store writes |
| A | `unit/20261109-a-search-live` | `ekr-zi-ua` | `<trees>/ekr-zi-ua/target` | `<scratch>/unit-a` | implementor green at `3b2c2f7a1` (30 of 30 loop runs); docs line `ab9ea16ae` by coordinator; adversary not run (wind-down) |
| B | `unit/20261109-b-explain-merge` | `ekr-zi-ub` | `<trees>/ekr-zi-ub/target` | `<scratch>/unit-b` | tree and brief ready; not yet dispatched |
| E | `unit/20261109-e-sdk-yaml` | `ekr-zi-ue` | `<trees>/ekr-zi-ue/target` | `<scratch>/unit-e` | tree and brief ready; not yet dispatched |
| F | `unit/20261109-f-held-payload` | `ekr-zi-uf` | `<trees>/ekr-zi-uf/target` | `<scratch>/unit-f` | first implementor paused before any change; to be re-dispatched |
| H | `unit/20261109-h-view-changes` | `ekr-zi-uh` | `<trees>/ekr-zi-uh/target` | `<scratch>/unit-h` | implementor told to stop at its next commit (wind-down) |
| verify | `verify/20261109-dg` | `ekr-zi-verify` | `<trees>/ekr-zi-verify/target` | `<scratch>/verify` | D, G, C verified at `d48cf0320`; evidence recorded |

## Approvals

- `approval-record:zero-issues-acceptance-2026-11-09` — the acceptance of the eleven artifacts.
- `approval-record:zero-issues-wave-1-proposal-2026-11-09` — the plan and wave 1.
- `approval-record:zero-issues-wave-2-proposal-2026-11-09` — wave 2 and the dispatch order of the taking-over coordinator (one sub-agent at a time; H first, then B, E, F, C, I, J, K).

## Reviews

| unit | pass | record | findings | outcome |
|---|---|---|---|---|

## Gate

Not run. The run was wound down on 2026-11-09 on the operator's correction ("the goal applies to ess only"): no pull request, no merge, no issue closed. See *Wind-down* below.

## Wind-down (2026-11-09)

Operator correction, relayed by the caller: "the goal applies to ess only". Nothing new is
dispatched. Branches pushed as b10x-bot: `wave/20261109-zero-issues`, `unit/20261109-a-search-live`
(unit A green, no adversary pass yet), and unit H's branch at whatever its implementor committed.
No story moved to `implemented`; C, D and G carry their verification evidence and wait for the
close. Next owner: merge A after one adversary pass; finish H; run B, E, F, I, J, K from the briefs
in the run's scratch.
