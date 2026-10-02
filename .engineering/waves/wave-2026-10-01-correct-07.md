# Wave correct-07 — migrate with added evidence, the extraction verb's findings, typed divergence, a replaced SQLite store, tests under load, document bounds

Skill: aep:implementing 0.15.0 (wave mode). Coordinator: the orchestrating Claude session.
Status: **open** (2026-10-01), dispatched on the operator's instruction ("dispatch next wave, take
decisions yourself") from `release-plan:next-waves-2026-10-01` (revision 4, critic panel two rounds).

Base: `main` at `e92f7341` (0.0.25) plus this opening commit, which merges `plan/review-0930`
(the store at `fdc5ba6d`; every conflict was a squash artefact, main's copy equal to the plan
branch at `7f27af49`, so the plan branch's copy was taken). Integration branch `wave/correct-07` in
worktree `ekr-correct-07`.

| unit | item | branch | stage |
|---|---|---|---|
| G | `task:divergence-is-a-typed-store-error` | `impl/typed-divergence` (`1945bf5e`, base `30703729`) | adversary pass |
| M | `task:migrate-reads-a-current-store` | `impl/migrate-added-evidence` | implementing |
| V | `story:extraction-verb-shares-the-sdk-path` | `impl/extraction-verb` (`3c3aa312`) | fixes after G merges |
| S | `task:sqlite-store-replaced-in-place` with `task:held-bytes-notice-deleted-blobs` | `impl/store-replaced` | after G merges |
| F | `task:read-only-open-passes-under-load` with `task:rendering-cost-test-passes-under-load` | `impl/tests-under-load` | queued |
| B | `task:seed-document-bounds-alias-expansion` | `impl/document-bounds` | queued |

Build directories `~/.cache/b10x-target/ekr-c7-<unit>`, debug only, at most two building at once,
each deleted at its merge. No `git stash` (AGENTS.md).

## Order (release plan § Order and collisions)

G merges first; V merges after G and carries G's change into its `answer` function
(`crates/ekr/src/cli/session.rs`); S follows G (`eventlog.rs`, the reopen path). M is alone on
`migrate.rs` and `inventory.rs`. `docs/cli.md` is shared by V, B and S; `CHANGELOG.md` by all.

## Decisions taken by the coordinator

- The live-handle rule is `task:sqlite-store-replaced-in-place` § Decision: a live handle never
  answers from bytes a fresh handle would refuse; a replaced store is refused as `store-replaced`
  and the hosts reopen; withdrawing retained bytes appends an event to the object's stream.
- M fixes the order of publication in `Migration::migrate_into` (objects an occurrence needs are in
  the destination before it is re-derived) after a failing test reproduces the consumer's minimal
  case; the consumer's synthetic reproduction is `~/.cache/ekr-migrate-repro/cb3-store/`.
- V's acceptance names all six adversary findings; it does not merge with any open.
- 0.0.26 is cut when all six units merge and the gate is green.

## Adversary passes

| unit | record | outcome |
|---|---|---|
| V | `review-result:adversary-extract-07-v-pass-1` | NEEDS-CHANGE; fixes in this wave |

## Commits this wave makes

The opening commit; one commit per unit and at most one adversary-fix commit per unit; the merges
into `wave/correct-07`; the suite regeneration; the closing store commit; the merge into `main`
through a bot pull request; release 0.0.26 under the standing rule that a green `main` is released.

Agents: `aep:implementor` per unit, `aep:adversary` once per unit.
