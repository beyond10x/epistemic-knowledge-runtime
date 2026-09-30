# Wave extract-06 — commit cost, SDK resolver and protocol fixes, the extraction document, fact quality, an explain index, one event-type rule, per-type property definitions

Skill: aep:implementing 0.15.0 (wave mode). Coordinator: the orchestrating Claude session.
Status: **open** (2026-09-30), dispatched on the operator's instruction of 2026-09-30 ("continue, do
it"), after the plan review recorded in `release-plan:next-waves-2026-09-30` (revision 4).

Base: `plan/review-0930` (`2b3c6c1e`, main `ec6bbf38` plus the plan-review commit) plus this opening
commit. Integration branch `wave/extract-06` in worktree `ekr-extract-06`.

| unit | branch | worktree | build dir | stage |
|---|---|---|---|---|
| C | `impl/commit-cost-flat` | `ekr-x6-c` | `~/.cache/b10x-target/ekr-x6-c` | dispatched |
| R | `impl/resolver-queue-invalidates` | `ekr-x6-r` | `~/.cache/b10x-target/ekr-x6-r` | dispatched |
| P | `impl/protocol-error-answer` | `ekr-x6-p` | `~/.cache/b10x-target/ekr-x6-p` | dispatched |
| X | `impl/explain-index` | `ekr-x6-x` | `~/.cache/b10x-target/ekr-x6-x` | dispatched |
| D | `impl/extraction-document` | `ekr-x6-d` | — | second batch |
| Q | `impl/fact-quality-sample` | `ekr-x6-q` | — | second batch |
| E | `impl/one-event-type-rule` | `ekr-x6-e` | — | second batch |
| J | `impl/per-type-property-definitions` | `ekr-x6-j` | — | second batch |

At most four units build at once; the second batch starts as the first finishes. `/` had 47 GB free
at the opening.

## Units

| unit | item | surface | confidence |
|---|---|---|---|
| C | `story:commit-cost-flat-with-store-size` | `crates/ekr-store/src/{eventlog,verified}.rs`, `crates/ekr-store/tests/`, one ignored scaling test in `crates/ekr/tests/` | cited (`eventlog.rs`), rest inferred |
| R | `task:resolver-queue-drops-shared-alias-answers` | `crates/ekr-sdk/src/resolve.rs`, its tests | cited |
| P | `task:protocol-error-keeps-the-answer` | `crates/ekr-sdk/src/{transport,session}.rs`, `crates/ekr-sdk/src/read/one_shot.rs`, their tests | cited |
| X | `story:explain-reads-an-index` | `crates/ekr-kernel/src/{explain,read,replay,checkpoint}.rs`, `crates/ekr-sdk/src/read/{kernel,mod}.rs`, `crates/ekr/src/cli/{explain,mcp,agent,mod}.rs`, kernel suite | cited and inferred (story § Scope) |
| D | `story:extraction-document-applies-to-a-store` | `crates/ekr-integrate/src/extraction.rs`, `crates/ekr/src/cli/{schema,agent,mod}.rs`, `docs/cli.md` | cited and inferred (story § Scope) |
| Q | `story:fact-quality-by-judged-sample` | `crates/ekr-views/src/sample.rs`, `systems/ekr/domains/views.yaml`, `crates/ekr/src/cli/{sample,session,agent,mod}.rs`, `docs/cli.md` | cited and inferred (story § Scope) |
| E | `task:one-event-type-rule` | `crates/ekr-views/src/index.rs`, `crates/ekr/src/cli/view_roles.rs`, the OCEL export, `views.yaml`, `docs/cli.md` § Roles and § `ekr ocel` | cited |
| J | `task:projection-carries-per-type-property-definitions` | `ekr.graph-projection/1` in `views.yaml`, `crates/ekr-views`, the viewer | cited |

Collisions, all in different sections, merged by the coordinator: the verb registry
(`crates/ekr/src/cli/mod.rs`, `agent.rs`, `tests/agent_cli.rs`, `docs/cli.md`) for X, D, Q;
`views.yaml` for E, J, Q; `docs/cli.md` for E as well; `CHANGELOG.md` for every unit. The
coordinator regenerates the conformance suites once after the merges. P and R both edit
`crates/ekr-sdk` in different files.

Left out: extract-07's five items (next wave, `release-plan:next-waves-2026-09-30`); the eventlog
statement cache (`prepare_cached`), which is a change in `beyond10x/eventlog`.

## Adversary passes

| unit | record | outcome |
|---|---|---|

## Merge notes

## Decisions taken by the coordinator

- C's fix is chosen from the 2026-09-30 profile (story § Probe result): a verified non-canonical
  object is re-read only when the log has advanced since it was verified. The upstream statement
  cache is not part of this wave.
- The two SDK tasks ship in 0.0.25 because a consumer is parked on each.

## Commits this wave makes

The opening commit; one commit per unit and at most one adversary-fix commit per unit; the merges
into `wave/extract-06`; the closing store commit; the merge into `main` through a bot pull request;
a release (0.0.25), cut under the standing rule that a green `main` is released.

Agents: `aep:implementor` per unit, then one `aep:adversary` pass per unit. Build directories under
`~/.cache/b10x-target`, one per unit, deleted when its unit merges; `task check` on the combined
tree, its build directory deleted right after.
