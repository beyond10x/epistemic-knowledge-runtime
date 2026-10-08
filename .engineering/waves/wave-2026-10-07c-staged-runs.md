# Wave 2026-10-07c — a run is staged, and published whole or dropped whole

Skill: aep:implementing 0.21.0 (wave mode). Coordinator: the repository's controlling Claude session
of 2026-10-07.
Status: **integrated**: units S, C, P, L and the Eventlog 0.8.1 move merged into `wave/20261007c`; the pull request is next.

Approved 2026-10-07: a consumer running one-shot `ekr` verbs against PostgreSQL needs a failed run
to leave the head where it was; the shape (a stage published whole, not a rewound head) was chosen
for this repository on 2026-10-07 and is written into `story:a-run-is-staged-and-published-whole`.

Base: `main` at `2626b0ae4b`. Integration branch `wave/20261007c`. One pull request from it.

Commits this approval authorises: the coordinator's opening and closing store commits, one commit
per unit (more if an adversary's cases are added to a unit), the merge of each unit into
`wave/20261007c`, and the merge into `main` through the pull request once its checks are green.
A release after the merge follows the repository's release process.

## Units, in order

| unit | task | surface | dispatch |
|---|---|---|---|
| S | `task:stage-specified` | `systems/ekr/domains/store.yaml`, design § 107, an architecture decision record | `ess:author` |
| C | `task:postgres-source-copy` | `crates/ekr-kernel/src/migrate.rs`, `crates/ekr-store/src` (PostgreSQL read under one consistent image) | `aep:implementor`, then `aep:adversary` |
| P | `task:stage-suffix-publication` | `crates/ekr-kernel`, `crates/ekr-store/src` | `aep:implementor`, then `aep:adversary` |
| L | `task:stage-cli` | `crates/ekr/src/cli`, `docs/cli.md`, `crates/ekr/tests` | `aep:implementor` |

C, P and L share crates, so they run one after another, each forked from the integration head
that holds the one before.

| unit | branch | head | worktree id | build dir | scratch | stage |
|---|---|---|---|---|---|---|
| int | `wave/20261007c` | `d8a5f48646` | `ekr-w20261007c-int` | `<int>/target` | `<int>/.engineering/drafts` | all units merged |
| S | `unit/stage-specified` | `3359dc47d6` | `ekr-w20261007c-s` | — | — | merged; adversary fixes merged; tree finished and archived |
| C | `unit/postgres-source-copy` | `07d50be27e` | `ekr-w20261007c-c` | — | — | merged with adversary fixes (`3b12ba4381`); tree finished and archived |
| P | `unit/stage-suffix-publication` | `ed0f5b134e` | `ekr-w20261007c-p` | — | — | merged with adversary fixes (`1aa4b4ca48`); tree finished and archived |
| L | `unit/stage-cli` | `7a2756fab6` | `ekr-w20261007c-l` | — | — | merged (`9d21959a01`) |
| E | `unit/eventlog-0.8.1` | `ffae1fb371` | `ekr-w20261007c-l` (reused) | — | — | merged (`d8a5f48646`); tree finished and archived |

## Unit S

The four stage commands are in their own domain, `ekr.cli`, owned by a new component `ekr` (the
binary). The chosen handler is the binary; ESS admits only the component owning a command's domain
as its handler (`ESS-COMPONENT-004`), so the commands could not stay in `ekr.store`. The stage, its
events, refusals and view stay in `ekr.store`. `ess specify validate --path systems/ekr` reports
`ekr v1 — 10 file(s), valid` on ess 0.36.0 and 0.55.0, with no new warning.

Three guards fail on the integration branch until unit P adds the Rust carriers:
`crates/ekr-store/tests/adversary_p1_14_store_bindings.rs`, `crates/ekr-store/tests/domain_projection.rs`
and `crates/ekr-core/tests/identity_serde.rs` (`StageId`). The pull request opens only after they
pass.

PostgreSQL cases run against a disposable local container started for this wave and removed at its
close. CI does not run them (no PostgreSQL in `correctness.yml`).

## Unit P's adversary

One blocking defect: seal and abandon shared an idempotency key, so their race answered "idempotency
key already used" instead of a named refusal. Each record move now has its own key. Two further
refusals were misreported and an abandon after an interrupted publication left the stage's tenant
holding events; all fixed (`ed0f5b134e`). Two adversary cases asserted § 107 sentences the code
showed wrong (an occurrence can land after the seal; another writer's validation of a transaction
the run decides refuses the publication); the sentences were corrected and the two cases removed.
An intermittent two-publish shortfall came from the PostgreSQL change feed withholding events behind
an open transaction, not from a double publication. Review: `.engineering/reviews/w20261007c-p-adversary.md`.

## Unit L and the Eventlog move

Unit L adds `ekr stage begin|publish|abandon|list`, `--stage`/`EKR_STAGE`, and names every stage
refusal. ess 0.56.0's refusal of `kernel.yaml` is fixed by declaring `retained-commit` before `stale`.
The Eventlog crates move from 0.5.0 (`fe8a0a7e`) to the 0.8.1 release commit `d5db40da`, pinned by
rev because `story_contract` holds each pin to an immutable revision.

## Gate

Pre-push checks on the combined tree (unit E's tree, which held `d8a5f48646`'s content), with
PostgreSQL required for the PostgreSQL targets: `cargo fmt --all -- --check`, `cargo clippy --locked
--workspace --all-targets -- -D warnings`, and package-scoped `cargo test` of `ekr` (story_contract,
public_surface, docs_cli, agent_cli, stage_cli, adversary_docs_contract, postgres_cli, conformance,
conformance_integrate), all of `ekr-store`, `ekr-kernel` (lib, stage, hosted_postgres, the two
adversary files, migrate_store, store_open, seed, seed_envelope_v3), `ekr-core` identity_serde and
`ekr-views` conformance and code_names: 10 steps, 10 exit 0. The full gate is the pull request's
CI. Its workflow provides no PostgreSQL, so CI runs the SQLite half of the stage cases.
