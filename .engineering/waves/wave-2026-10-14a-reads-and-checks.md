# Wave 2026-10-14a — reads that stay cheap, and checks a long run needs

Skill: aep:implementing 0.22.8 (wave mode). Coordinator: the repository's controlling Claude session
of 2026-10-14.
Status: **running**.

Approved 2026-10-14: every ready unit with disjoint scope, up to six, preferring what a consumer's
seven-day unattended run needs. The stories that run needs most (source-adapter contract, retained
observations, poll health, the SDK adapter runner) are blocked by three open decisions
(`decision-blocker:checkpoint-unit-cardinality`, `decision-blocker:observation-retention-path`,
`decision-blocker:source-unit-granularity`) and are not in this wave.

Base: `main` at `ed01b09654e`. Integration branch `wave/20261014a`. One pull request from it.

Commits this approval authorises: the coordinator's opening and closing store commits, the CI commit
already on the branch (`task test` with `--no-fail-fast`; the docs-site workflow re-pinned to
website `7d3d414`), one commit per unit (more if an adversary's cases are added), the merge of each
unit into `wave/20261014a`, and the merge into `main` through the pull request once its checks are
green. A release after the merge follows the repository's release process.

Width: 6, the limit; every other ready story overlaps one of these on a named file (below).

## Units

| unit | story | serves | main surface | spec owner of | dispatch |
|---|---|---|---|---|---|
| T | `story:changes-since-by-time-within-deadline` | o5 | `crates/ekr-views/src/changes.rs`; `ekr-store` read path only if diagnosed | `views.yaml` cost clause only, through unit O | `aep:implementor` |
| R | `story:reads-served-from-a-persisted-read-model` | o5 | `crates/ekr-store/src` snapshot/verified/eventlog/postgres; `crates/ekr-kernel/src` checkpoint/runtime/read/replay | `store.yaml` | `aep:implementor`, `aep:adversary`, `aep:security-reviewer` |
| O | `story:overview-sections` | o5 | `crates/ekr-views/src` query/index/lib; `crates/ekr/src/cli` mcp/view/session/views; `crates/ekr-sdk/src/read/views.rs` | `views.yaml` | `aep:implementor` |
| C | `story:credential-redaction-before-model-input` | o5 | `crates/ekr-observe`; new `crates/ekr-sdk/src/redact.rs` | `observe.yaml` | `aep:implementor`, `aep:security-reviewer` |
| E | `story:store-equivalence-check` | o2 | new `crates/ekr/src/cli` verb module, `cli/mod.rs` | `cli.yaml` | `aep:implementor` |
| V | `story:schema-transaction-cites-evidence` | o2 | `crates/ekr-kernel/src` apply/validate; `crates/ekr-ontology/src` evolve/schema; `crates/ekr-sdk/src/document/transaction.rs`; `crates/ekr/src/cli/ontology.rs`; `crates/ekr-views/src/lib.rs` only | `kernel.yaml`, `ontology.yaml` | `aep:implementor`, `aep:adversary` |

Scopes are inferred from a read-only triage of the stories; none is cited by a scoper yet.

Conditions:

- Unit V keeps the evidence citation out of the overview (`crates/ekr-views/src/index.rs`,
  `query.rs`, `views.yaml`); if it cannot, it stops and reports.
- Unit T stops after its diagnosis if the cause is in `crates/ekr-store`; its fix then lands after R.
- Unit R treats the schema-version record as opaque, so V's new field needs no change in R.
- `docs/cli.md` and `CHANGELOG.md` are shared by O, E, V and possibly C; they merge in order and
  `crates/ekr/tests/docs_cli.rs` runs after each merge.

Deferred, with the file they share with a unit here: `story:read-answers-name-types` (O's
`query.rs`, `mcp.rs`), `story:resolve-by-type-name` (O's `mcp.rs`), `story:store-provider-migration`
(R's `ekr-store`, E's `cli/mod.rs`).

## Trees

| unit | branch | head | worktree id | build dir | scratch | stage |
|---|---|---|---|---|---|---|
| int | `wave/20261014a` | — | `ekr-wave-20261014a` | `<int>/target` | `~/.cache/ekr-w20261014a/int` | open |
