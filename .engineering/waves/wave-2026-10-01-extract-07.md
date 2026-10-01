# Wave extract-07 — the extraction verb, the SDK's judged sample, evidence attachment, validate cost, typed divergence, document bounds

Skill: aep:implementing 0.15.0 (wave mode). Coordinator: the orchestrating Claude session.
Status: **open** (2026-10-01), dispatched on the operator's instruction of 2026-10-01 ("integrate
completed work, cleanup, dispatch next /wave"), from `release-plan:next-waves-2026-09-30`
(revision 5) plus `task:validate-cost-flat-with-store-size`.

Base: `wave/extract-06` at `30703729` (all eight extract-06 units merged; its integration fixes and
release 0.0.25 follow on that branch). Units branch from `30703729`; the integration branch
`wave/extract-07` is created from `main` after 0.0.25 lands, and the units merge into it.

| unit | item | branch | worktree | stage |
|---|---|---|---|---|
| A | `story:evidence-attaches-to-a-held-assertion` | `impl/evidence-attachment` | `ekr-x7-a` | dispatched |
| V | `story:extraction-verb-shares-the-sdk-path` | `impl/extraction-verb` | `ekr-x7-v` | dispatched |
| W | `task:validate-cost-flat-with-store-size` | `impl/validate-cost-flat` | `ekr-x7-w` | dispatched |
| G | `task:divergence-is-a-typed-store-error` | `impl/typed-divergence` | `ekr-x7-g` | dispatched |
| K | `story:sdk-store-checks` | `impl/sdk-judged-sample` | `ekr-x7-k` | second batch |
| B | `task:seed-document-bounds-alias-expansion` | `impl/document-bounds` | `ekr-x7-b` | second batch |
| L | `task:rendering-cost-test-passes-under-load` | `impl/rendering-test-load` | `ekr-x7-l` | second batch |

Build directories: `~/.cache/b10x-target/ekr-x7-<unit>`, debug only, deleted at merge. At most four
build at once.

## Collisions (different sections; merged by the coordinator)

- `docs/cli.md`: A, V, B.
- `crates/ekr/src/cli/agent.rs`, `crates/ekr/tests/agent_cli.rs`, the kernel conformance manifest and
  suites: A, V.
- `crates/ekr/src/cli/session.rs`: V (`respond`, `admit`), G (`:622`).
- `crates/ekr-sdk/src/lib.rs`: V and K, one `pub mod` line each.
- `crates/ekr-kernel/src/validate/lifecycle.rs`: A (attachment checks, inferred) and W (if the
  profile confirms the claim copy at `:20–27`).
- `crates/ekr-sdk/src/document/`: A (the `Operation` variant) and V (the extraction mirror).

## Decisions taken by the coordinator

- Evidence attachment: index 15, a canonical-graph collection omitted when empty so existing stores
  encode unchanged, reused `unresolved-*` codes, refused beside a retract or supersede of the same
  assertion in one transaction, counted by both quality counters (story § Coordinator decisions).
- W is probe-first: the profile on the large SQLite delta is written into the task before a change.
- `story:preparation-blobs-are-reclaimed` and `task:store-open-verifies-blobs-once` wait for
  extract-08 (shared `eventlog.rs`, `explain.rs`, `kernel.yaml`, `log.rs`).

## Adversary passes

| unit | record | outcome |
|---|---|---|

## Commits this wave makes

The opening store commit; one commit per unit and at most one adversary-fix commit per unit; the
merges into `wave/extract-07`; the suite regeneration; the closing store commit; the merge into
`main` through a bot pull request; a release (0.0.26), cut under the standing rule that a green
`main` is released.

Agents: `aep:implementor` per unit, then one `aep:adversary` pass per unit.
