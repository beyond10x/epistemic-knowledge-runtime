# Wave 2026-10-07a — every tree builds into its own `target/`

Skill: aep:implementing 0.21.0 (wave mode). Coordinator: the repository's controlling Claude session
of 2026-10-07.
Status: **closing**: unit A merged into `wave/20261007a`; the pull request into `main` is open.

Approved 2026-10-07: the change was queued for this repository after release 0.0.32, under the
operator's rule of 2026-10-06 that every tree builds into its own `target/` and ends with
`worktree finish --discard-cache --archive`.

Base: `main` at `c3d9da3a53` (release 0.0.32). Integration branch `wave/20261007a`. One pull
request from it.

Commits this approval authorises: the coordinator's opening store commit, one commit for unit A,
the merge of unit A into `wave/20261007a`, the closing store commit, and the merge into `main`
through the pull request once its checks are green.

## Unit

| unit | task | surface |
|---|---|---|
| A | `task:build-into-the-trees-own-target` (serves `vision:o2`) | `AGENTS.md` § The gate (cited) |

Dispatch: `aep:implementor`. No adversary: the unit changes contributor guidance in one file and no
code.

| unit | branch | head | worktree id | build dir | scratch | stage |
|---|---|---|---|---|---|---|
| int | `wave/20261007a` | `dd7eb2cf0a` | `ekr-w20261007a-int` | `<int>/target` | `<int>/.engineering/drafts` | closing |
| A | `unit/build-into-the-trees-own-target` | `0b607af14a` | `ekr-w20261007a-a` | none (no build) | none | merged |

## Planning records written with this wave

Draft stories from what release 0.0.32 and the worktree clean-up of 2026-10-07 found. None is in
this wave.

- `story:browser-tests-need-no-network`: the browser tests still fetch the pinned graph libraries
  from the CDN on every launch.
- `story:c7-reads-beside-a-checkpointing-writer-do-not-depend-on-load`: two `adversary_c7_s` cases
  fail when a SQLite lock wait passes 10.1 s under load.
- `story:knowledge-stack-integrated-or-retired`: where the unmerged 110-commit stack of 2026-10-03
  and 2026-10-04 was published, and what it waits on.

## Also recorded, not in this wave

- `task:remove-unified-site-manifest` (draft): delete `b10x.docs.yaml`. It waits on
  `dependency-blocker:atlas-lists-ekr-as-independent`: the documentation procedure deletes a
  repository's unified-site manifest only after Atlas lists the repository as documented
  independently, and on 2026-10-07 Atlas `main` (`5a2c04e463`) did not.

## Gate

The wave changes `AGENTS.md` and planning records and no code, so the full local gate was waived for
it; the pull request's CI runs the full gate. Run on `wave/20261007a` at `dd7eb2cf0a`, each step's
own exit status:

| step | exit |
|---|---|
| `cargo test --locked -p xtask --bin ekr-docs` (3 passed) | 0 |
| `cargo run --locked -p xtask --bin ekr-docs -- build` | 0 |
| `cargo run --locked -p xtask --bin ekr-docs -- check` (`task site-check`) | 0 |
| `aep plan artifact validate` (aep 0.64.0) | 0 |

The three tests that read `AGENTS.md` (`crates/ekr/tests/docs_cli.rs:1427`,
`crates/ekr-sdk/tests/document_drift.rs:575`, `crates/ekr/tests/schema_cli.rs:930`) were read, not
run: none asserts anything in § The gate.

Cost: one `aep:implementor` run, 52,888 tokens, 19 tool uses, 91 s.
