# Wave 2026-10-20a — the 0.0.39 review's SupersedeNode defects fixed

Skill: aep:implementing 0.23.2 (wave mode). Coordinator: the Claude session of wave 2026-10-19a.
Status: **gated**: CI `Repository correctness` green on `f982c340` (PR 100); the story is
implemented; U1's tree `ekr-w20a-u1` is finished and removed (archive kept).

Source: `review-result:haiku-panel-0-0-39-supersede-node`, 20 read-only reviewers on release 0.0.39,
54 findings. Story: `story:supersede-node-review-fixes`.

Approval: the operator chose option A on 2026-10-20: record the panel, file the story, run it as a
wave of one and release 0.0.40. That authorises: the unit's commits on `unit/20261020a-u1`, its merge
into `wave/20261020a`, the store commits on `wave/20261020a`, the push of `wave/20261020a`, one pull
request, its merge into `main` once CI is green, a `Release 0.0.40` commit, the bot tag `0.0.40` on the
merge commit and the GitHub release `0.0.40`.

Base: `main` at `67bd5555` (release 0.0.39). Integration branch `wave/20261020a`.

Width: 1 unit. The story's scope was written from the panel's cited `file:line`s (23 cited, 3
inferred); no `aep:story-scoper` ran, because every surface is a reviewer's citation.

## Units

| unit | story | main surface | dispatch |
|---|---|---|---|
| U1 | `story:supersede-node-review-fixes` | `ekr-views` index/sample, `ekr-sdk` resolve/extraction, `ekr-kernel` validate, `ekr-graph` canonical; documents | `aep:implementor` |

## Trees

| unit | branch | worktree id | build dir | scratch | state |
|---|---|---|---|---|---|
| integration | `wave/20261020a` | `ekr-w20261019a` (reused) | none | none | store writes |
| U1 | `unit/20261020a-u1` | `ekr-w20a-u1` | `<tree>/target` | `<home>/.cache/ekr-w20a-u1-scratch` | implementor green at `5127539c` after correction 1 (lane 3 1429→1445, lane 4 138→143); D7, D8, D10, Q7 held back as `<scratch>/verdict-changes.patch`; adversary pass 1 dispatched |

## Incidents

- The implementor stopped once on HTTP 429 and was resumed with its context.
- `target/` reached 47G (the `adr-changes` monitor session's `du`); `cargo clean -p` of the 11
  workspace members brought it to 3.7G and `/` to 77G free; later builds run with
  `CARGO_INCREMENTAL=0`, one at a time. The disk watch logged no stop.
- The first full gate on `478807e5` was stopped by the coordinator in its `test` step, after
  `fmt-check`, `search-web-check` and `clippy` exited 0, at `adr-changes`' alarm: `/` at 18G free,
  `target/debug/deps` 44G (`du`) holding 508 entries without an extension. The watch had logged no
  stop (it polled every 20 s); it now polls every 5 s. The cache was discarded with
  `worktree discard-cache` (`/` to 89G free) and the gate rerun with `CARGO_PROFILE_DEV_DEBUG=0`.
  Cause: `cargo test --workspace` links about 250 integration-test binaries, each with the
  opt-level-3 dependency tree; one run writes about 45G.

## Pre-flight

| check | reading |
|---|---|
| free disk on `/` | 65G (`df -h /`, 2026-10-20); the U1 build of wave 2026-10-19a reached 26–43G |
| disk watch | `<home>/.cache/ekr-w20a-u1-scratch/disk-watch.sh` stops U1's cargo and rustc processes under 20G free (operator rule, 2026-10-20) |
| pinned tools | `ess` 0.36.0, `aep` 0.64.0 through `pinned-task.sh` |

## Gate

Local rerun on `478807e5` in the U1 tree (`<home>/.cache/ekr-w20a-u1-scratch/gate-478807e5-2/status.txt`):
`fmt-check`, `search-web-check`, `clippy`, `spec-check`, `conform-fresh`, `plan-check`, `site-check`
and `cargo fmt --all -- --check` exit 0. `test` ran 2077 cases before the disk watch stopped it at
under 20G free (six STOP lines, 08:15–08:17Z, `/` down to 17G with other sessions building): 2076
passed, 1 failed, `ekr-ontology`'s `yaml_ingress::every_production_yaml_reader_has_an_explicit_input_policy`,
because the T6 test in `replay.rs`'s test module is a third `serde_yaml_ng::to_string` and the
inventory pinned two (fixed in `52dcc3cc`). `doc-check`, `vendor-check` and the `ekr` cross-crate
lane were stopped by the watch (exit 143/201), not failed. The implementor's lanes had not run
`ekr-ontology`'s tests, which is how the count slipped through. The full gate runs again as the pull
request's `Repository correctness` job.
