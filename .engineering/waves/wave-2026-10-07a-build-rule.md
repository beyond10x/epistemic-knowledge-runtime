# Wave 2026-10-07a — every tree builds into its own `target/`

Skill: aep:implementing 0.21.0 (wave mode). Coordinator: the repository's controlling Claude session
of 2026-10-07.
Status: **open**.

Approved 2026-10-07: the change was queued for this repository after release 0.0.32, under the
operator's rule of 2026-10-06 that every tree builds into its own `target/` and ends with
`worktree finish --discard-cache --archive`.

Base: `main` at `c3d9da3a53` (release 0.0.32). Integration branch `wave/20261007a`. One pull
request from it.

Commits this approval authorises: the coordinator's opening store commit, one commit for unit A,
the merge of unit A into `wave/20261007a`, the closing store commit, and the merge into `main`
through the pull request once its checks are green.

## Unit

| unit | task | surface |
|---|---|---|
| A | `task:build-into-the-trees-own-target` (serves `vision:o2`) | `AGENTS.md` § The gate (cited) |

Dispatch: `aep:implementor`. No adversary: the unit changes contributor guidance in one file and no
code.

| unit | branch | head | worktree id | build dir | scratch | stage |
|---|---|---|---|---|---|---|
| int | `wave/20261007a` | `c3d9da3a53` | `ekr-w20261007a-int` | `<int>/target` | `<int>/.engineering/drafts` | opening |
| A | `unit/build-into-the-trees-own-target` | `c3d9da3a53` | `ekr-w20261007a-a` | none (no build) | none | planned |

## Planning records written with this wave

Draft stories from what release 0.0.32 and the worktree clean-up of 2026-10-07 found. None is in
this wave.

- `story:browser-tests-need-no-network`: the browser tests still fetch the pinned graph libraries
  from the CDN on every launch.
- `story:c7-reads-beside-a-checkpointing-writer-do-not-depend-on-load`: two `adversary_c7_s` cases
  fail when a SQLite lock wait passes 10.1 s under load.
- `story:knowledge-stack-integrated-or-retired`: where the unmerged 110-commit stack of 2026-10-03
  and 2026-10-04 was published, and what it waits on.

## Gate

Not run yet.
