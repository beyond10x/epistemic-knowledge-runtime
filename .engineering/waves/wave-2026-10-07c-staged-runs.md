# Wave 2026-10-07c — a run is staged, and published whole or dropped whole

Skill: aep:implementing 0.21.0 (wave mode). Coordinator: the repository's controlling Claude session
of 2026-10-07.
Status: **open**: unit S dispatched.

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
| int | `wave/20261007c` | `2626b0ae4b` | `ekr-w20261007c-int` | `<int>/target` | `<int>/.engineering/drafts` | opening |
| S | `unit/stage-specified` | — | `ekr-w20261007c-s` | none (no build) | `<s>/.engineering/drafts` | planned |

## Gate

Not run yet.
