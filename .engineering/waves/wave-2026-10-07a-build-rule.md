# Wave 2026-10-07a — every tree builds into its own `target/`; the unified-site manifest goes

Skill: aep:implementing 0.21.0 (wave mode). Coordinator: the repository's controlling Claude session
of 2026-10-07.
Status: **closed**: merged to `main` in
https://github.com/beyond10x/epistemic-knowledge-runtime/pull/85 (`2626b0ae4b`); both tasks are
implemented. Its worktrees are finished and removed.

Approved 2026-10-07: both changes were queued for this repository after release 0.0.32. Unit A
follows the operator's rule of 2026-10-06 that every tree builds into its own `target/` and ends
with `worktree finish --discard-cache --archive`. Unit B joined the wave once Atlas listed this
repository as documented independently.

Base: `main` at `c3d9da3a53` (release 0.0.32). Integration branch `wave/20261007a`. One pull
request from it.

Commits this approval authorises: the coordinator's opening store commit, one commit per unit, the
merge of each unit into `wave/20261007a`, the coordinator's store commits, and the merge into
`main` through the pull request once its checks are green.

## Units

| unit | task | surface |
|---|---|---|
| A | `task:build-into-the-trees-own-target` (serves `vision:o2`) | `AGENTS.md` § The gate (cited) |
| B | `task:remove-unified-site-manifest` (serves `vision:o5`) | `b10x.docs.yaml` (cited) |

Dispatch: `aep:implementor` for each. No adversary: neither unit changes code.

| unit | branch | head | worktree id | build dir | scratch | stage |
|---|---|---|---|---|---|---|
| int | `wave/20261007a` | `66e4416190` | `ekr-w20261007a-int` | `<int>/target` | `<int>/.engineering/drafts` | closing |
| A | `unit/build-into-the-trees-own-target` | `0b607af14a` | `ekr-w20261007a-a` | none (no build) | none | merged |
| B | `unit/remove-unified-site-manifest` | `b685243f35` | `ekr-w20261007a-b` | `<b>/target` (xtask only) | none | merged |

Unit B's order: the documentation procedure deletes a repository's unified-site manifest only
after Atlas lists the repository as documented independently. Atlas `main` did not at
`5a2c04e463` (read 2026-10-07); it did at `1cfb16a7dbfb` (committed 2026-10-07T01:57:14Z), which
cleared `dependency-blocker:atlas-lists-ekr-as-independent`. The repository had no unified-site
bundle caller, façade caller or unified-site check; `b10x-docs-site.yml` is the own-site caller and
stays. `docs/roadmap.md:126` and `story:cli-user-documentation` name the manifest as P0 history and
stay.

## Planning records written with this wave

Draft stories from what release 0.0.32 and the worktree clean-up of 2026-10-07 found. None is in
this wave.

- `story:browser-tests-need-no-network`: the browser tests still fetch the pinned graph libraries
  from the CDN on every launch.
- `story:c7-reads-beside-a-checkpointing-writer-do-not-depend-on-load`: two `adversary_c7_s` cases
  fail when a SQLite lock wait passes 10.1 s under load.
- `story:knowledge-stack-integrated-or-retired`: where the unmerged 110-commit stack of 2026-10-03
  and 2026-10-04 was published, and what it waits on.

## Gate

The wave changes `AGENTS.md`, deletes `b10x.docs.yaml` and writes planning records; it changes no
code, so the full local gate was waived for it and the pull request's CI runs the full gate. Run on
`wave/20261007a` at `66e4416190`, each step's own exit status:

| step | exit |
|---|---|
| `cargo test --locked -p xtask --bin ekr-docs` (3 passed) | 0 |
| `cargo run --locked -p xtask --bin ekr-docs -- build` | 0 |
| `cargo run --locked -p xtask --bin ekr-docs -- check` (`task site-check`) | 0 |
| `aep plan artifact validate` (aep 0.64.0) | 0 |

The three tests that read `AGENTS.md` (`crates/ekr/tests/docs_cli.rs:1427`,
`crates/ekr-sdk/tests/document_drift.rs:575`, `crates/ekr/tests/schema_cli.rs:930`) were read, not
run: none asserts anything in § The gate. Nothing in `crates/`, `xtask/`, `website/`, `.github/`
or `Taskfile.yml` reads `b10x.docs.yaml`.

Cost: unit A's `aep:implementor`, 52,888 tokens, 19 tool uses, 91 s; unit B's, 54,220 tokens,
21 tool uses, 142 s.
