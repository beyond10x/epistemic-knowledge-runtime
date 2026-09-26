---
format: aep.planning-md/2
id: review-result:adversary-skeleton-pass-1
kind: review-result
status: active
title: Adversary, story:workspace-crate-skeleton, pass 1
tags:
- adversary:aep-drive:adversary
relations:
- reviews: story:workspace-crate-skeleton
revision: 1
---
```
unit: story:workspace-crate-skeleton — uncommitted working tree at .../ekr-impl-workspace-crate-skeleton, base 3310fb0
verdict: red
cases: executed 0→6, red 2
origin: introduced 3, pre-existing 1, undecided 0
wrote-outside-worktree: 3 paths (part 6)
needs-coordinator: yes — the serde_yaml drift is a store write (story amendment or review-result), which is not mine to make
```

## 1. `git --no-pager diff --stat`

```
 Cargo.lock | 1139 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++--
 Cargo.toml |   22 ++
 2 files changed, 1124 insertions(+), 37 deletions(-)
```

`crates/` is untracked, so the tracked diff shows none of it. `git status --porcelain -uall` — my two additions are the last two lines; every other path is the implementor's, untouched by me:

```
 M Cargo.lock
 M Cargo.toml
?? crates/ekr-core/Cargo.toml          ?? crates/ekr-core/src/lib.rs
?? crates/ekr-graph/Cargo.toml         ?? crates/ekr-graph/src/lib.rs
?? crates/ekr-kernel/Cargo.toml        ?? crates/ekr-kernel/src/lib.rs
?? crates/ekr-ontology/Cargo.toml      ?? crates/ekr-ontology/src/lib.rs
?? crates/ekr-store/Cargo.toml         ?? crates/ekr-store/src/lib.rs
?? crates/ekr/Cargo.toml               ?? crates/ekr/src/main.rs
?? crates/ekr/tests/msrv_contract.rs   ?? crates/ekr/tests/story_contract.rs
```

No non-test path is mine. `rustfmt --edition 2021` was run on my two files only.

## 2. Cases added, and their red output captured alone, before the suite

`home-path:sha256:90d68f8473dd758c8a5bc2c933b3c4beb261c77a8ef34330f7ba5ab17b7cac07`

`cargo test -p ekr --locked --test msrv_contract` → **RED**, exit 101:

```
thread 'no_resolved_dependency_needs_more_than_the_declared_rust_version' panicked at crates/ekr/tests/msrv_contract.rs:76:5:
the workspace declares rust-version = "1.85", but the lockfile this unit wrote resolves 8 package(s) that need more:
  eventlog-core 0.2.1 needs rust 1.91
  eventlog-file 0.2.1 needs rust 1.91
  eventlog-sqlite 0.2.1 needs rust 1.91
  time 0.3.55 needs rust 1.88.0
  time-core 0.1.9 needs rust 1.88.0
  time-macros 0.2.32 needs rust 1.88.0
  trybuild 1.0.121 needs rust 1.88
  wasip2 1.0.4+wasi-0.2.12 needs rust 1.87.0
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

`.../crates/ekr/tests/story_contract.rs` — five cases; expectations are **parsed out of `.engineering/planning/story/workspace-crate-skeleton.md` at run time**, not copied in, so amending the story amends the test. `cargo test -p ekr --locked --test story_contract` → **1 RED, 4 green**, exit 101:

```
running 5 tests
test every_crate_opts_into_workspace_lints ... ok
test the_six_crates_are_workspace_members ... ok
test crate_dependency_edges_match_the_story ... ok
test no_manifest_carries_a_comment ... ok
test external_dependencies_match_the_story ... FAILED

thread 'external_dependencies_match_the_story' panicked at crates/ekr/tests/story_contract.rs:203:5:
the manifests and story:workspace-crate-skeleton disagree:
  crates/ekr-ontology/Cargo.toml [dependencies] lacks `serde_yaml`, which the story declares
  crates/ekr-ontology/Cargo.toml [dependencies] declares `serde_yaml_ng`, which the story does not
  crates/ekr-kernel/Cargo.toml [dependencies] lacks `serde_yaml`, which the story declares
  crates/ekr-kernel/Cargo.toml [dependencies] declares `serde_yaml_ng`, which the story does not
test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

Mutant probe, on a **copy in scratch** (never the tree): deleting `"crates/ekr-store"` from `members` leaves `cargo clippy --workspace --all-targets --locked -- -D warnings` at **exit 0** and `cargo test --workspace --locked` at **exit 0** — the acceptance's "six present as workspace members" is enforced by no gate step. The same probe against my case:

```
thread 'the_six_crates_are_workspace_members' panicked at crates/ekr/tests/story_contract.rs:281:5:
the acceptance names six workspace members; Cargo.toml omits ["ekr-store"]
```

Second probe, same copy: deleting the `//!` block from `crates/ekr/src/main.rs` **does** fail clippy (`error: missing documentation for the crate`, exit 101) — that guard is live on the binary crate, so the story's `missing_docs` claim holds.

## 3. The suite, after the cases existed

`export CARGO_TARGET_DIR=home-path:sha256:b75f9a061b971f23993e7f85d8bf63ed6eff15c82c05ab13d3f33e64656bc3fe && task check`

```
task: [fmt-check] cargo fmt --all -- --check
task: [clippy] cargo clippy --workspace --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
task: [test] cargo test --workspace --locked
...
thread 'no_resolved_dependency_needs_more_than_the_declared_rust_version' panicked at crates/ekr/tests/msrv_contract.rs:76:5:
the workspace declares rust-version = "1.85", but the lockfile this unit wrote resolves 8 package(s) that need more:
  eventlog-core 0.2.1 needs rust 1.91
...
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
error: test failed, to rerun pass `-p ekr --test msrv_contract`
task: Failed to run task "check": task: Failed to run task "test": exit status 101
EXIT=201
```

`task check` **exits 201**. The gate fails fast at the first red binary; `cargo test --workspace --locked --no-fail-fast` gives the honest count — 6 executed, 2 red. `<before>` is 0, read from the implementor's `gate-green.log` (every binary "running 0 tests"), not from a pre-emptive run.

## 4. Findings

| # | file:line | what is wrong | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|---|
| 1 | `Cargo.toml:17` (`rust-version = "1.85"`) | every one of the six crates inherits a minimum-toolchain promise the unit's own lockfile falsifies: `ekr-store`'s direct, non-optional `eventlog-core/file/sqlite 0.2.1` need **1.91**, and `trybuild`/`time` need 1.88 | `msrv_contract.rs:76`, exit 101, 8 packages listed | cargo enforces `rust-version` at build time — anyone on the declared minimum (1.85–1.90) gets a hard refusal on `ekr-store` and `ekr`. **No CI job and no gate step builds at 1.85** (`.github/workflows/shared-gates.yml` pins no toolchain; local rustc is 1.98.1), so this repo's pipeline never reaches it — a false promise to readers and consumers, not a broken build here | NEEDS-CHANGE | introduced |
| 2 | `.engineering/planning/story/workspace-crate-skeleton.md:75,77` | the story declares `serde_yaml` for `ekr-ontology` and `ekr-kernel`; the manifests declare `serde_yaml_ng`. The implementor brief directed the fork, but the story — revision 8, committed at base, the document later P1 stories read — was never amended, and nothing in the gate compares the two | `story_contract.rs:203`, exit 101, 4 disagreements | the story's own closing constraint routes later authors through this list ("a later P1 story that needs a dependency not listed here adds `Cargo.lock` to its scope"); a later author reading it declares `serde_yaml` and collides | NEEDS-CHANGE | introduced |
| 3 | `Taskfile.yml:6` (`check`) | the acceptance is "`task check` exits zero **with the six present as workspace members**"; no step asserts membership. Dropping `crates/ekr-store` from `members` keeps clippy and test at exit 0, because the path deps still build it | mutant probe on the scratch copy: clippy exit 0, test exit 0; my `story_contract.rs:281` catches it | the acceptance statement itself; the coordinator reads the member list by eye. Members are correct **today** — this is the missing guard, now supplied | CONFIRMED | introduced |
| 4 | `.engineering/planning/story/workspace-crate-skeleton.md:6` | title says "**five** empty workspace members"; the scope, the constraints and the acceptance all say six. Residue — no test written | read at base and at HEAD: identical string | anything printing the story title, including `aep plan artifact show` | CONFIRMED | pre-existing |

Named fixes (not applied, per charter): #1 — raise `[workspace.package] rust-version` to `1.91`, which is what the eventlog tag this story pins actually requires. #2 — amend the story's two bullets to `serde_yaml_ng`, or record a `review-result` naming the substitution. #3 — keep `the_six_crates_are_workspace_members`. #4 — retitle to "six".

## 5. Attacked, not broken

- Crate dependency edges: all six match the story exactly, both directions, no extras — `crate_dependency_edges_match_the_story` green.
- External dependencies per crate: every other name matches, including `uuid` features `["v7"]`, and the dev-dependency split.
- `[lints] workspace = true`: present in all six.
- Comments in any `Cargo.toml`: none, root or crate.
- Inherited fields: `version`, `edition`, `license`, `repository`, `rust-version` all defined in `[workspace.package]`; no crate inherits an undefined field.
- `missing_docs` guard: live on the library crates **and** the binary — probe confirmed red when the `//!` is removed.
- Doc comments: each names the right ESS domain; `ekr-core` and `ekr-kernel` both name `kernel`, matching `systems/ekr/components.yaml`.
- Lockfile completeness: `--locked --offline` resolves clean; eventlog pinned to `77cda08` at tag `0.2.1`, all three crates.
- Code beyond the doc comment: only the empty clap `Cli` and `main` in `crates/ekr/src/main.rs`.
- Layering acyclicity: cargo would have rejected a cycle; the declared order is the story's.

## 6. Paths written outside the worktree

| path | what |
|---|---|
| `home-path:sha256:49505b01ff339da643749523e35fa6cd1f14f0aeac6faef8b9e87776d7a25520` | 1.1M mutable copy of the tree for the two probes; **still present** (the probes reproduce from it) |
| `home-path:sha256:8628b27229725551c68e140cc40727afeb0bd25106c7873a4f28797012330bf9` | the part-3 gate run |
| `home-path:sha256:f10fd4d36ec7912e31388a35b1d1d1795835fb420015d5e509de860af7decd9d` | the `--no-fail-fast` case count |

Also created and **already removed by me**: `home-path:sha256:5b52c6016eefc62fa97638188d12fb86448a0625d592ddee2ad8ddf23a079867` (571M probe build dir). The shared `home-path:sha256:b75f9a061b971f23993e7f85d8bf63ed6eff15c82c05ab13d3f33e64656bc3fe` was used as the brief directs and left in place. Lease `ekr-adversary-skeleton-1` taken and released; no other session's lease touched.

```findings
- file: Cargo.toml
  line: 17
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "every crate inherits rust-version = 1.85 while the lockfile this unit wrote resolves eventlog-core/file/sqlite 0.2.1 at rust-version 1.91 as direct non-optional dependencies of ekr-store, so the declared minimum toolchain cannot build the workspace and no gate step or CI job builds at it."
- file: .engineering/planning/story/workspace-crate-skeleton.md
  line: 75
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the story declares serde_yaml for ekr-ontology and ekr-kernel while both manifests declare serde_yaml_ng, and nothing in task check compares the story to the manifests, so the document later P1 stories are routed through is wrong."
- file: Taskfile.yml
  line: 6
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "no gate step asserts the acceptance's six workspace members - deleting crates/ekr-store from members leaves clippy and test at exit 0 because the path dependencies still build it."
- file: .engineering/planning/story/workspace-crate-skeleton.md
  line: 6
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: "the story title says five empty workspace members while its scope, constraints and acceptance all say six."
```
