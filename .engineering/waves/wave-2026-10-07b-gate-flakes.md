# Wave 2026-10-07b — the gate stops depending on the network and on lock-wait time

Skill: aep:implementing 0.21.0 (wave mode). Coordinator: the repository's controlling Claude session
of 2026-10-07.
Status: **open**.

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
| int | `wave/20261007b` | `2626b0ae4b` | `ekr-w20261007b-int` | `<int>/target` | `<int>/.engineering/drafts` | opening |
| A | `unit/browser-tests-need-no-network` | — | `ekr-w20261007b-a` | `<a>/target` | `<a>/.engineering/drafts` | planned |
| B | `unit/c7-lock-wait-under-load` | — | `ekr-w20261007b-b` | `<b>/target` | `<b>/.engineering/drafts` | planned |

## Gate

Not run yet.
