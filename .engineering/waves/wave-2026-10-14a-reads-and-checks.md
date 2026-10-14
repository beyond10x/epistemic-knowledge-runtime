# Wave 2026-10-14a — reads that stay cheap, and checks a long run needs

Skill: aep:implementing 0.22.8 (wave mode). Coordinator: the repository's controlling Claude session
of 2026-10-14.
Status: **integrated**: units T, R, O, C, E and V merged into `wave/20261014a`; stories R, O, C, E and V implemented, T stays active; the pull request is next.

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
| int | `wave/20261014a` | `75304ceec81` | `ekr-wave-20261014a` | `<int>/target` | `~/.cache/epistemic-knowledge-runtime-w20261014a/int` | open until the merge |
| T | `unit/w20261014a-t` | `a4998099755` | `ekr-w20261014a-t` | `<tree>/target` | `~/.cache/epistemic-knowledge-runtime-w20261014a/t` | finished |
| R | `unit/w20261014a-r` | `33110753070` | `ekr-w20261014a-r` | `<tree>/target` | `~/.cache/epistemic-knowledge-runtime-w20261014a/r` | finished |
| O | `unit/w20261014a-o` | `057a3c92fcb` | `ekr-w20261014a-o` | `<tree>/target` | `~/.cache/epistemic-knowledge-runtime-w20261014a/o` | finished |
| C | `unit/w20261014a-c` | `916c635f45a` | `ekr-w20261014a-c` | `<tree>/target` | `~/.cache/epistemic-knowledge-runtime-w20261014a/c` | finished |
| E | `unit/w20261014a-e` | `b08a621b2ee` | `ekr-w20261014a-e` | `<tree>/target` | `~/.cache/epistemic-knowledge-runtime-w20261014a/e` | finished |
| V | `unit/w20261014a-v` | `1ffc4bf3a9a` | `ekr-w20261014a-v` | `<tree>/target` | `~/.cache/epistemic-knowledge-runtime-w20261014a/v` | finished |

## Gate

One full `task check` on `0b1ed6a28` (the six units merged): clippy, format and the search-web
check green; 2391 tests passed and 6 test targets failed. Four of them (`ekr` conformance and
conformance_integrate, `ekr-views` conformance and determinism) had one cause: unit C declared
`start >= 0` and `length >= 1` on `ekr.observe.RedactionSpan`, a type no view publishes, so ESS
refused the claim (ESS-SYNTH-013) and every suite's coverage inventory read incomplete, which
makes a run's conformance Inconclusive although every scenario passed. The invariants moved to
`ekr_core::RedactionSpan`, which already holds them; the three suites were re-synthesized with
no refusal (`939dd73bc`). The fifth, `public_surface`, named `ekr_store::evidence_root_of`
as untested; `current_root_sensitivity.rs` now calls it by path (`75304ceec81`). The sixth,
the code-names adversary, failed a wall-clock bound at 5.48 s under load; its two bounds are
now ratios of two measurements in one process (`939dd73bc`).

After the fixes, each failed target was rerun alone and passed, then the package gates of the
crates the fixes touch, on `75304ceec81`: `cargo clippy -D warnings` and `cargo test --no-fail-fast`
for `ekr-store`, `ekr-views` and `ekr`, all green. The pull request's CI is the gate for the merge.
