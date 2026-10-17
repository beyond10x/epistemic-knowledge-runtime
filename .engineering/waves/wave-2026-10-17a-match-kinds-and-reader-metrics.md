# Wave 2026-10-17a — search names each match's kind; the readers emit metrics

Skill: aep:implementing (wave mode). Coordinator: the repository's controlling Claude session of
2026-10-17.
Status: **integrating**: U2 merged into `wave/20261017a`; U1 runs its package tests.

Source: two consumer requests. A reader that reads only the first five matches per word misses a
node whose whole name matches in another case (`story:search-match-kinds`). An operator scraping
Prometheus metrics finds none on the readers (`story:readers-emit-metrics-and-request-logs`).

Base: `main` at `2f233f8f` (release 0.0.37). Integration branch `wave/20261017a`. One pull request
from it.

Width: 2 units, every ready unit of this source. Both declare their change in `views.yaml` and touch
`docs/cli.md`, so each runs in its own tree; the coordinator resolves those two files at merge.

## Units

| unit | story | main surface | spec change | dispatch |
|---|---|---|---|---|
| U1 | `story:search-match-kinds` | `crates/ekr-views/src/search.rs`; the format selection in `crates/ekr/src/cli/` and `crates/ekr-sdk/src/read/` | `views.yaml`: `ekr.node-matches/3` | `aep:implementor` |
| U2 | `story:readers-emit-metrics-and-request-logs` | `crates/ekr/src/cli/http.rs`, `mcp.rs`, `view.rs` | `views.yaml`: metrics, `/metrics`, request log; `cli.yaml`: the options | `aep:implementor` |

## Trees

| unit | branch | worktree id | build dir | state |
|---|---|---|---|---|
| integration | `wave/20261017a` | `ekr-w20261017a` | `<tree>/target` | U2 merged at `4ed86ce8` |
| U1 | `unit/20261017a-u1` | `ekr-w17a-u1` | `<tree>/target` | at `91bea139`; package tests running |
| U2 | `unit/20261017a-u2` | `ekr-w17a-u2` | `<tree>/target` | merged at `9eab295c`; adversary found two introduced defects, both fixed |

## Gate

Not run.
