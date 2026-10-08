# Wave 2026-10-07b — the gate stops depending on the network and on lock-wait time

Skill: aep:implementing 0.21.0 (wave mode). Coordinator: the repository's controlling Claude session
of 2026-10-07.
Status: **closed**: merged to `main` in https://github.com/beyond10x/epistemic-knowledge-runtime/pull/86 (`2d0bab6f4e`); both stories are implemented. Its worktrees are finished and removed.

Approved 2026-10-07: these two stories are the next wave after 2026-10-07a, one pull request, with
a full gate on the integration branch.

Base: `main` at `2626b0ae4b`. Integration branch `wave/20261007b`. One pull request from it.

Commits this approval authorises: the coordinator's opening store commit, one commit per unit (more
if an adversary's cases are added to a unit), the merge of each unit into `wave/20261007b`, the
coordinator's closing store commit, and the merge into `main` through the pull request once its
checks are green.

## Why

Release 0.0.32 took four local full-gate runs on one tree. Run 2 failed in
`crates/ekr-store/tests/adversary_c7_s.rs` on a SQLite lock wait past 10.1 s; run 3 failed in
`crates/ekr/tests/adversary_page_stream_1.rs` because the page's graph libraries did not load from
the CDN. Each binary passed alone. Every release and wave pays for these reruns.

## Units

| unit | story | surface | scope |
|---|---|---|---|
| A | `story:browser-tests-need-no-network` (serves `vision:o5`) | the six browser-driving files in `crates/ekr/tests`, a fixture directory for the four pinned libraries, a shared support module | cited |
| B | `story:c7-reads-beside-a-checkpointing-writer-do-not-depend-on-load` (serves `vision:o5`) | `crates/ekr-store/tests/adversary_c7_s.rs` (cited); `crates/ekr-store/src/eventlog.rs` if the store's open path holds the lock (inferred) | cited and inferred |

The two surfaces share no file. Dispatch: `aep:implementor` for each. `aep:adversary` on unit B if
it changes anything under `crates/ekr-store/src`; unit A changes tests only.

| unit | branch | head | worktree id | build dir | scratch | stage |
|---|---|---|---|---|---|---|
| int | `wave/20261007b` | `bca067e032` | `ekr-w20261007b-int` | `<int>/target` | `<int>/.engineering/drafts` | gate |
| A | `unit/browser-tests-need-no-network` | `841edb1b17` | `ekr-w20261007b-a` | `<a>/target` (2,195 MiB, discarded) | `<a>/.engineering/drafts` (archived) | merged |
| B | `unit/c7-lock-wait-under-load` | `233261c312` | `ekr-w20261007b-b` | `<b>/target` (3,701 MiB, discarded) | `<b>/.engineering/drafts` (archived) | merged |

## What the units found

- Unit B: the lock that the two `adversary_c7_s` opening races lost was the test's own writer's.
  The races opened with `SqliteStore::sqlite`, which creates the owner tables through
  eventlog-sqlite and so takes `BEGIN IMMEDIATE` on every open; instrumented at load 39–44, each
  failing open waited 5.0–5.3 s there while the writer completed 7–47 publications, and no single
  publication held the lock past 2.5 s. Every `ekr` store verb opens an existing store with
  `sqlite_existing`, which takes no write lock. The races now open that way; no source changed, so
  no adversary ran. `SqliteStore::sqlite` still takes the write lock on an existing store; it is
  used only to create one.
- Unit A: the `--dump-dom` launches in `view_page.rs` and `adversary_p_page.rs` could not be
  intercepted, so they are now driven over the DevTools protocol under the same virtual-time
  policy. SHA-384 is implemented in the test support module rather than taken from `sha2`, because
  a new dev-dependency would change the dependency list `crates/ekr/tests/story_contract.rs`
  holds.

| unit | red | green |
|---|---|---|
| A | `view_page` 2 passed, 5 failed with the CDN unresolvable and nothing served | six binaries, 81 cases, 0 failed, CDN unresolvable |
| B | 1 of 9 runs passed at load 27–40 | 41 of 41 runs at load 17–44; `ekr-store` 198 passed |

Cost: unit A's `aep:implementor`, 368,123 tokens, 168 tool uses, 2,197 s; unit B's, 169,462
tokens, 65 tool uses, 1,741 s.

## Gate

Every `task check` step, one at a time, on `wave/20261007b` at `56d2f32383` (tree `abe7b2c7e2`),
2026-10-07 07:34–08:35Z, with the pinned `ess 0.36.0` and `aep 0.64.0`, the browser required,
`cargo test --workspace --no-fail-fast`, and debug info off (`CARGO_PROFILE_DEV_DEBUG=false`) to
keep the build directory within the machine's free space (it peaked at 10,550 MiB). The run was
stopped (SIGSTOP) for a time when free space fell under 20 GiB and continued afterwards.

| step | exit |
|---|---|
| fmt-check | 0 |
| search-web-check | 0 |
| clippy | 0 |
| test: 375 suites, 2,477 cases passed, 0 failed | 0 |
| bench `--no-run` | 0 |
| doc-check | 0 |
| vendor-check | 0 |
| spec-check | 0 |
| conform-fresh | 0 |
| plan-check | 0 |
| site-check | 0 |
