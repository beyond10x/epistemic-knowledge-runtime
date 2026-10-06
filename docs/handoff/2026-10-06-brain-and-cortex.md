# Hand-over: session brain-and-cortex, 2026-10-06

Written at the session's wrap-up. The session ends after this file; the next owner starts from
here.

## Release 0.0.32: blocked on red CI

| item | state |
|---|---|
| PR | https://github.com/beyond10x/epistemic-knowledge-runtime/pull/80, open |
| branch | `release/0.0.32` at `7e47e9abc`, pushed |
| contents | the 0.0.32 section of `CHANGELOG.md`: bounded `explain --documents` with `--offset`/`--limit`, `password_file` for `ekr.postgres/1`, the views-cache revision identity fix (#74) |
| last CI | run 37531347192: `Repository correctness` failed at `crates/ekr/tests/adversary_viewer_compact.rs:662` ("the page settled") |

`7e47e9abc` merges `main` into the release branch to bring in
https://github.com/beyond10x/epistemic-knowledge-runtime/pull/82 (browser tests wait on what the
page does). The three test files #82 changed passed locally on that commit (9, 11 and 36 passed).

`main` itself is red on `Correctness` for its last three merges, each time in a different browser
test:

| commit | failing test |
|---|---|
| `28c49c0de` | not read |
| `5a35fd399` | `adversary_page_stream_1.rs:646` (the case #82 fixed) |
| `e8383dc9c` (#82) | `view_page.rs:2964`, `compact_mode_wakes_the_resting_3d_loop_for_the_resize_and_it_rests_again` (run 37529812043) |

Issue https://github.com/beyond10x/epistemic-knowledge-runtime/issues/81 was closed by #82, but
the same class of failure (a browser wait that times out on a loaded runner) remains in
`adversary_viewer_compact.rs` and `view_page.rs`.

Next step:

1. Fix the remaining browser waits the way #82 did, on a branch from `main`, until `Correctness`
   is green on `main`.
2. Merge `main` into `release/0.0.32` locally (not through the update-branch button: the push gate
   refuses a bot push over a web-flow merge commit), push, wait for green.
3. Merge PR 80, tag `0.0.32` on the merge commit, create the GitHub Release from the 0.0.32
   section of `CHANGELOG.md`, comment the release on #74 and #81.
4. cortex wave 20261006i waits for this release (see cortex's hand-over).

## Queued work

| item | state |
|---|---|
| build rule | not started: rewrite the build rule in `AGENTS.md` (line 127) to "build into this tree's `target/`; end each tree with `worktree finish --discard-cache --archive <tree>`". It was to follow 0.0.32 |
| hand-over | this file |

## Worktrees

This session's trees are finished with `--discard-cache --archive` and removed by `worktree gc`:
`ekr-release-0032`, `ekr-fix-74`, `ekr-fix-74b`, `ekr-fix-74c`, `ekr-fix-74d`, `ekr-fix-81`,
`ekr-w6a-int`, `ekr-w6a-a`, `ekr-w6b-int`, `ekr-w6b-a`. The other trees of this repository
(dated 20261003 and 20261004) are not this session's and were not touched.

The primary checkout holds an untracked `.agents/` directory that this session did not create.

## Open pull requests

| PR | owner |
|---|---|
| 80 | this session (above) |
| 64, 59 | not this session |
