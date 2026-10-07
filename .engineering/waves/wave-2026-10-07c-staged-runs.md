# Wave 2026-10-07c — a run is staged, and published whole or dropped whole

Skill: aep:implementing 0.21.0 (wave mode). Coordinator: the repository's controlling Claude session
of 2026-10-07.
Status: **open**: units S and C merged; C's adversary running; unit P dispatched.

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
| int | `wave/20261007c` | `5906588c60` | `ekr-w20261007c-int` | `<int>/target` | `<int>/.engineering/drafts` | S, C merged |
| S | `unit/stage-specified` | `3359dc47d6` | `ekr-w20261007c-s` | — | — | merged; adversary fixes merged; tree finished and archived |
| C | `unit/postgres-source-copy` | `3c96c31e52` | `ekr-w20261007c-c` | `<c>/target` | `<c>/.engineering/drafts` | merged (`4276a3cae9`); adversary running |
| P | `unit/stage-suffix-publication` | `5906588c60` | `ekr-w20261007c-p` | `<p>/target` | `<p>/.engineering/drafts` | implementor running |

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

## Gate

Not run yet.
