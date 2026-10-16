# Wave 2026-10-16b — a stage begin within the lock table, a stage listing that reads its writes

Skill: aep:implementing (wave mode). Coordinator: the repository's controlling Claude session of
2026-10-16.
Status: **building**: U1 and U2 in their own trees.

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
| integration | `wave/20261016b` | `ekr-stage-locks` | `<tree>/target` | red tests and stories committed |
| U1 | `unit/20261016b-u1` | `ekr-w16b-u1` | `<tree>/target` | building |
| U2 | `unit/20261016b-u2` | `ekr-w16b-u2` | `<tree>/target` | building |

## Gate

Not run. Each unit runs its package gates; one full `task check` on the combined head, then the
cross-crate checks of `COORDINATOR.md` check 9, before the pull request.
