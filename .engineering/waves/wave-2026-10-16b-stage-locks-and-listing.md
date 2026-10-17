# Wave 2026-10-16b — a stage begin within the lock table, a stage listing that reads its writes

Skill: aep:implementing (wave mode). Coordinator: the repository's controlling Claude session of
2026-10-16.
Status: **gated**: U1 and U2 merged into `wave/20261016b`; the full gate is green on its head;
the pull request follows.

Source: two reports from a consumer running 0.0.35 on PostgreSQL, reproduced here first as red tests
on a PostgreSQL 17 container (`crates/ekr-kernel/tests/stage_hosted_limits.rs`): issue #93 (a stage
begin refused `stage-copy-failed`, SQLSTATE 53200, when one seed or commit carries more payloads
than the server's lock table holds) and issue #94 (`ekr stage list` prints `[]` while another
transaction is open on the server).

Base: `main` at `48e35b1f` (release 0.0.36). Integration branch `wave/20261016b`. One pull request
from it.

Width: 2 units, every ready unit of this source; the units share no source file.

## Units

| unit | story | issue | main surface | spec change | dispatch |
|---|---|---|---|---|---|
| U1 | `story:stage-copy-bounded-locks` | #93 | `crates/ekr-kernel/src/migrate.rs`; `docs/cli.md` | `store.yaml`: the bound on streams per copy transaction | `aep:implementor` |
| U2 | `story:stage-list-reads-committed` | #94 | `crates/ekr-store/src/stage_record.rs` | `cli.yaml` / `store.yaml`: how `ekr.cli.Stages` reads its writes | `aep:implementor` |

Both units add to `crates/ekr-kernel/tests/stage_hosted_limits.rs` only through the tests already
committed on the integration branch; each runs them on its own PostgreSQL container.

## Trees

| unit | branch | worktree id | build dir | stage |
|---|---|---|---|---|
| integration | `wave/20261016b` | `ekr-stage-locks` | `<tree>/target` | both units merged; suites regenerated from the merged specification; release 0.0.37 prepared |
| U1 | `unit/20261016b-u1` | `ekr-w16b-u1` | `<tree>/target` | merged at `72fd90f2` (`90e54186`, `cc154f3d`); adversary NEEDS-CHANGE on three findings, all fixed |
| U2 | `unit/20261016b-u2` | `ekr-w16b-u2` | `<tree>/target` | merged at `eeb0c261` (`762e642d`); adversary found nothing; tree finished |

## Gate

One full `task check` on the combined head 4437fdca, in this tree's own `target/`, 2026-10-17,
with the pinned `ess` 0.36.0 and `aep` 0.64.0 first on the path: `fmt-check`, `search-web-check`
and `clippy` passed; `test` ran every target (`--no-fail-fast`): 2477 passed, 13 ignored, 1
failed: `ekr`'s `public_surface::no_public_item_in_any_crate_is_untested`, because
`ekr-store`'s `CopyGroupStreams` and its `BOUND` were used by no case outside a comment.
0e17c7d7 adds `domain_projection::the_copy_group_bound_is_the_one_the_domain_states`, which reads
the bound from `store.yaml` and holds both to it. On 0e17c7d7 the failed step and what follows it
were rerun and exited zero: `fmt-check`, `ekr-store` clippy on all targets, the `public_surface`
target (12 passed) and the `domain_projection` target (9 passed), then `doc-check`,
`vendor-check`, `spec-check`, `conform-fresh`, `plan-check` and `site-check`.
`cargo test --workspace -- --list` in this tree lists 2489 workspace tests, both named cases among
them. The cross-crate checks of `COORDINATOR.md` check 9 (format, workspace clippy, the `ekr`
contract targets) are part of that run.
