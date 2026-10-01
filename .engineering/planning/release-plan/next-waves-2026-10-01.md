---
format: aep.planning-md/3
id: release-plan:next-waves-2026-10-01
kind: release-plan
status: active
title: Waves correct-07, extract-07b and storage-08 after 0.0.25
relations:
- serves: vision:o5
- supersedes: release-plan:next-waves-2026-09-30
revision: 4
transitions:
- {from: "draft", to: "active", at: "2026-10-01T18:42:14Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":3}}}
---
## Where this starts

0.0.25 is on main (`e92f7341`, 2026-10-01): wave extract-06, eight units, CI green on main and the
tag (runs 36881585050, 36881586170, 36881639576). `release-plan:next-waves-2026-09-30` scheduled it
and is superseded by this plan.

A store audit of 2026-10-01 (390 artifacts, 19 findings) and a consumer's reports reorder what
follows. The rule here: **a defect that makes stored state wrong, unreadable or unmigratable comes
before a feature, and a feature does not merge with an open finding of its own adversary.**

Two consumer-facing defects lead:

- `ekr migrate` fails on every store with a committed `!AddEvidence`, since that operation existed
  (`task:migrate-reads-a-current-store`; mechanism read from `migrate.rs:139–300`, not yet run).
  Every consumer store since its first evidence addition is affected.
- The extraction verb on `impl/extraction-verb` re-applies a document as new facts and creates nodes
  by document order (`review-result:adversary-extract-07-v-pass-1`, eight red cases on the branch).

Work already on branches from wave extract-07 (`.engineering/waves/wave-2026-10-01-extract-07.md`)
is kept and finished here: V (`3c3aa312`), G (`1945bf5e`), A (`68ef2ecf`, WIP), W (`39b37674`, WIP).

## Wave correct-07 (0.0.26): correctness

| unit | item | state | why first |
|---|---|---|---|
| M | `task:migrate-reads-a-current-store` | new | no consumer store with added evidence can be migrated; extract-07's evidence attachment makes more such stores |
| V | `story:extraction-verb-shares-the-sdk-path` | branch, adversary NEEDS-CHANGE | the verb duplicates facts on re-apply; its acceptance now names all six findings |
| G | `task:divergence-is-a-typed-store-error` | branch, committed | acceptance narrowed to what it built; needs its adversary pass |
| S | `task:sqlite-store-replaced-in-place` with `task:held-bytes-notice-deleted-blobs` | new, decided | a live handle answers from a replaced store; one `store.yaml` section decides both (each task's `## Decision`) |
| F | `task:read-only-open-passes-under-load` with `task:rendering-cost-test-passes-under-load` | new | the first failed the 0.0.25 gate; both follow the under-load rule now in `AGENTS.md` |
| B | `task:seed-document-bounds-alias-expansion` | new | the seed and transaction readers load YAML before bounding its depth |

Order and collisions:

- G merges first. V and G both edit `crates/ekr/src/cli/session.rs` in one region: V splits
  `respond` into a new `answer` function (`origin/impl/extraction-verb`, `session.rs:258–266`) and
  G edits the divergence match inside the old `respond` body (`origin/impl/typed-divergence`,
  `session.rs:~300`). V merges after G and carries G's change into `answer`; its adversary
  re-runs the session tests on the merged file.
- S follows G: both edit `crates/ekr-store/src/eventlog.rs` and the session's reopen path, and S
  reuses G's reopen.
- M touches `crates/ekr-kernel/src/migrate.rs` and `crates/ekr-store/src/inventory.rs`, which nothing
  else in this wave edits; S's surface is inferred, so S's scope is checked against `inventory.rs`
  before it starts.
- V touches `crates/ekr-sdk`, `crates/ekr/src/cli/{extraction,session,agent}.rs`,
  `crates/ekr/tests/agent_cli.rs` and `docs/cli.md`. B touches `crates/ekr-kernel/src/document/`,
  `crates/ekr/src/cli/seed.rs` and `vendor/serde_yaml_ng`. F touches tests only.
- `docs/cli.md` is shared by V, B and S in different sections; `CHANGELOG.md` by all.

At most two units build at once (disk: four cold builds filled 50 GB on 2026-10-01). Each build
directory is deleted when its unit merges.

## Wave extract-07b (0.0.27): the consumer's extraction path

| unit | item | depends on |
|---|---|---|
| A | `story:evidence-attaches-to-a-held-assertion` (WIP branch) | M: a store with attachments must migrate |
| W | `task:validate-cost-flat-with-store-size` (WIP branch, profile first) | — |
| K | `story:sdk-store-checks` (the judged sample) | — |
| O | `task:ocel-export-prints-its-counts` | — |

A and W both may edit `crates/ekr-kernel/src/validate/lifecycle.rs`; W keeps its change to the
function its profile names. A also edits `docs/cli.md`, `crates/ekr/src/cli/agent.rs` and
`crates/ekr/tests/agent_cli.rs`, which V changes in correct-07 (`origin/impl/evidence-attachment`
and `origin/impl/extraction-verb` both touch `docs/cli.md` and `tests/agent_cli.rs` today): A is
rebased onto main after correct-07 releases, before its adversary pass.

## Wave storage-08 (0.0.28): storage cost

| item | depends on |
|---|---|
| `story:preparation-blobs-are-reclaimed` | M (`depends_on`), S's decision on withdrawn bytes |
| `task:store-open-verifies-blobs-once` | one probe shared with `story:reads-served-from-a-persisted-read-model` |
| `task:eventlog-statements-prepared-once` | an eventlog release with `prepare_cached` (not yet filed in `beyond10x/eventlog`) |

## Not scheduled

- `story:schema-transaction-cites-evidence`: a schema-evolution change, specified in ESS first.
- The eleven open decision blockers, none of which stops a unit above:
  `source-unit-granularity`, `checkpoint-unit-cardinality` and `observation-retention-path` (P2;
  option 3 each recommended), `relation-assertion-edge-correspondence`, `merge-absorbed-node-fate`,
  `merge-across-node-types`, `split-reverting-merge-identity`, `typed-reference-identifying-keys` and
  `typed-reference-subtype-matching` (P3), `property-sensitivity-home` (type packs),
  `constraint-language` (property constraints).
- Store hygiene from the audit, not work: P1-era reviews with no recorded outcomes (592 findings
  against 294 outcomes store-wide), `verification-report:p1-exit` still draft, and epics P2, P3, P4 and
  P6 whose unclaimed promises wait on those blockers.
