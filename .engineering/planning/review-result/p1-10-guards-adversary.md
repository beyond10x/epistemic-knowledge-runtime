---
format: aep.planning-md/2
id: review-result:p1-10-guards-adversary
kind: review-result
status: active
title: 'Independent source guard review: two escapes and type-use false positives'
relations:
- reviews: story:source-guard-debt
revision: 1
---
unit: story:source-guard-debt — stable working diff on 4ae2023d0479f79852376c86e5bcb2d15cbeea31
verdict: CONFIRMED — two missed guard violations and one type-use false-positive class
cases: executed 455→461, red 4
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned evidence directory, build target and temporary fixture directory; exact private inventory retained separately
needs-coordinator: route the two guard corrections and resolve the type-position cases before integration

```text
 README.md                                          |  14 +-
 crates/ekr-core/tests/adversary2_public_surface.rs |   5 +-
 crates/ekr-core/tests/identity_serde.rs            |  12 +-
 crates/ekr-core/tests/public_surface.rs            |  12 +-
 .../tests/adversary2_guard_bounds_and_ranges.rs    |  76 ++--
 .../tests/adversary2_membrane_and_addresses.rs     |   7 +-
 .../tests/adversary_canonical_value_reach.rs       |   6 +-
 .../tests/adversary_snapshot_and_assertion.rs      |  33 +-
 .../tests/canonical_value_and_assertion.rs         | 140 +++++++-
 crates/ekr-graph/tests/domain_projection.rs        |  18 +-
 crates/ekr-graph/tests/revision_events.rs          |  10 +-
 crates/ekr-kernel/tests/validation.rs              |   3 +-
 crates/ekr-ontology/tests/domain_projection.rs     |  19 +-
 .../tests/inheritance_and_declaration_coherence.rs |  24 +-
 crates/ekr-ontology/tests/value_type_checking.rs   |   2 +-
 .../tests/adversary2_retention_event_contract.rs   |   8 +-
 crates/ekr-store/tests/domain_projection.rs        |  13 +-
 crates/ekr-store/tests/providers.rs                |   7 +-
 crates/ekr/tests/adversary_docs_contract.rs        | 139 ++++++--
 crates/ekr/tests/msrv_contract.rs                  |  12 +-
 crates/ekr/tests/public_surface.rs                 | 392 ++++++++++++++++-----
 xtask/src/main.rs                                  | 120 ++++++-
 22 files changed, 848 insertions(+), 224 deletions(-)
```

Owners: all inherited guard implementation and substantive tests — implementor; inherited README/AGENTS/AEP/integration — coordinator; six appended cases in three authorized test files — adversary. The non-test paths in the displayed diff were already dirty at dispatch. This adversary did not edit them.

## 1. Diff and source preservation

The initial status, raw binary patch and SHA-256 manifest were retained before test additions. The complete manifest check afterward differs in exactly three files: `crates/ekr/tests/public_surface.rs`, `temporal_reads.rs` and `adversary_docs_contract.rs`. Removing only the six added test cases reconstructs the original bytes and their exact initial hashes:

```text
7eb24e966a5515e0f865abd0355daa4e97dee3e5e0f31e0ac97a93042cf32744 public_surface.rs
d58e81b2e14ac5136d69b91cc1b9d61c9cfba7866d354d8563b3ccae5d4115d0 temporal_reads.rs
1d1d9caf33da953ee2e7a485c853db71c308b3e66c7baead92df424091d44e42 adversary_docs_contract.rs
```

`adversary-test-additions.patch` contains only those added cases. All implementation, inherited assertions, README and planning bytes remain unchanged. The normal Git stat omits inherited untracked files, including `temporal_reads.rs`; `final-status.txt` and the hash manifest retain them. No AEP command, source mutation, commit, publication or managed-tree cleanup was performed.

## 2. Cases written before isolated execution

Each command used `cargo test -p ekr --offline --test <target> <case> -- --exact --nocapture`. Every first run selected exactly one case. The inherited 455-case baseline was supplied by the coordinator; it was not rerun before writing the cases.

| Target / added case | Result | Boundary |
|---|---|---|
| temporal_reads / adversary_cargo_member_comment_cannot_hide_a_product_source | red, exit 101 | Cargo accepts and lists both members, but a trailing member comment hides the next product source from the actual scan. |
| temporal_reads / adversary_manifest_macro_guard_rejects_all_rust_delimiters | red, exit 101 | The actual guard binary rejects the parentheses control then accepts the brace spelling in a scratch source file. |
| public_surface / adversary_generic_trait_implementation_is_an_actual_type_use | red, exit 101 | A generic trait implementation is not recognized as type use. |
| public_surface / adversary_tuple_and_array_elements_are_actual_type_uses | red, exit 101 | The first array-element control is not recognized as type use. |
| public_surface / adversary_opaque_literals_keep_comment_and_identifier_boundaries | green, exit 0 | Opaque strings, nested comments, imports, identifier boundaries and positive raw-identifier uses. |
| adversary_docs_contract / adversary_readme_membership_requires_one_unambiguous_product_inventory | green, exit 0 | Duplicate sections/products/labels, malformed quoting, extra utilities, wrong-section utility and order-independent valid membership. |

The macro case stops at the first incorrectly accepted spelling (`{}`); the `[]` expectation is retained but was not reached. The array case stops at `[Widget; 2]`; its tuple/slice expectations remain unexecuted after that failure. The generic case's qualified-path expectation likewise follows its first failure. A separate scratch `rustc --edition 2021 --crate-type lib --emit metadata` probe compiled all three macro delimiters plus generic implementation and array/tuple/slice type syntax, exit 0. It establishes that the examples are Rust syntax, not that later expectations executed.

First-run output follows, with only private paths replaced by placeholders. Full raw logs remain in assigned scratch.

```text
   Compiling ekr v0.0.0 (<guard-tree>/crates/ekr)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.19s
     Running tests/temporal_reads.rs (<target>/debug/deps/temporal_reads-bc1f57efcfc34a6f)

running 1 test

thread 'adversary_cargo_member_comment_cannot_hide_a_product_source' (882200) panicked at crates/ekr/tests/temporal_reads.rs:200:5:
assertion `left == right` failed
  left: []
 right: ["<tmp>/.tmp2UYamv/crates/ekr-kernel/src/lib.rs"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test adversary_cargo_member_comment_cannot_hide_a_product_source ... FAILED

failures:

failures:
    adversary_cargo_member_comment_cannot_hide_a_product_source

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p ekr --test temporal_reads`
```

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/temporal_reads.rs (<target>/debug/deps/temporal_reads-bc1f57efcfc34a6f)

running 1 test

thread 'adversary_manifest_macro_guard_rejects_all_rust_delimiters' (882223) panicked at crates/ekr/tests/temporal_reads.rs:228:9:
the actual guard accepted env!{"CARGO_MANIFEST_DIR"}:

running 1 test
test executable_source_location_macros_cannot_return ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s



note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test adversary_manifest_macro_guard_rejects_all_rust_delimiters ... FAILED

failures:

failures:
    adversary_manifest_macro_guard_rejects_all_rust_delimiters

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr --test temporal_reads`
```

```text
   Compiling ekr v0.0.0 (<guard-tree>/crates/ekr)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.14s
     Running tests/public_surface.rs (<target>/debug/deps/public_surface-cf7751befcffc952)

running 1 test

thread 'adversary_generic_trait_implementation_is_an_actual_type_use' (882375) panicked at crates/ekr/tests/public_surface.rs:379:5:
assertion failed: is_type_used("impl<T> Widget for Adapter<T> {}", "Widget")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test adversary_generic_trait_implementation_is_an_actual_type_use ... FAILED

failures:

failures:
    adversary_generic_trait_implementation_is_an_actual_type_use

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr --test public_surface`
```

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/public_surface.rs (<target>/debug/deps/public_surface-cf7751befcffc952)

running 1 test

thread 'adversary_tuple_and_array_elements_are_actual_type_uses' (882389) panicked at crates/ekr/tests/public_surface.rs:390:9:
fn consume(_: [Widget; 2]) {}
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test adversary_tuple_and_array_elements_are_actual_type_uses ... FAILED

failures:

failures:
    adversary_tuple_and_array_elements_are_actual_type_uses

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr --test public_surface`
```

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/public_surface.rs (<target>/debug/deps/public_surface-cf7751befcffc952)

running 1 test
test adversary_opaque_literals_keep_comment_and_identifier_boundaries ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s

```

```text
   Compiling ekr v0.0.0 (<guard-tree>/crates/ekr)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.14s
     Running tests/adversary_docs_contract.rs (<target>/debug/deps/adversary_docs_contract-e9a6b870e5121212)

running 1 test
test adversary_readme_membership_requires_one_unambiguous_product_inventory ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s

```

## 3. Suite and controls after case creation

Commands ran with the assigned target, `CARGO_BUILD_JOBS=2`, and assigned TMPDIR.

`cargo test --workspace --offline --no-fail-fast` exited 101. All 77 top-level runners completed: **457 passed, 4 failed, 0 ignored; 461 executed**. Before additions: 455 passed as reported by the implementor/coordinator. The full raw output is `workspace-test.log`. A nested child guard's successful summary appears inside the macro failure diagnostic; it has eight filtered cases and is not counted as another top-level case.

Relevant verbatim runner summaries:

```text
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: FAILED. 9 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: FAILED. 7 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
error: 2 targets failed:
    `-p ekr --test public_surface`
    `-p ekr --test temporal_reads`
```

`cargo fmt --all --check`: final exit 0. The first check found formatting only in the appended cases; those line wraps were corrected without altering assertions. The suite above preceded only this formatting. `cargo clippy --workspace --all-targets --offline -- -D warnings`: exit 0. No second full suite was claimed.

The same existing `xtask doctor` binary was invoked four ways: no manifest environment from a nested scratch workspace selected that scratch root (exit 0); explicit runtime manifest from the same cwd selected the actual assigned tree (exit 0); unrelated cwd with no manifest refused (exit 1); nonexistent explicit manifest refused (exit 1). Exact output and statuses are retained in `xtask-*.log` / `xtask-status.txt`. These are command controls, not extra Cargo case counts.

## 4. Findings

| ID | File:line | Severity / verdict / origin | What was measured | What reaches it |
|---|---|---|---|---|
| F1 | crates/ekr/tests/temporal_reads.rs:44 | blocker / CONFIRMED / introduced | A valid trailing Cargo member comment hides the following product member from the temporal source scan. Exact case exits 101 after Cargo metadata successfully lists both members; the scan returns an empty violations list although the kernel fixture contains `valid_time.to.is_none()`. | An ordinary comment after one workspace-member entry, followed by prohibited code in the next member. Both Cargo and the source guard run through their real entry points; no unsupported runtime configuration is needed. The new scan function is entirely introduced by this diff. |
| F2 | crates/ekr/tests/temporal_reads.rs:153 | blocker / CONFIRMED / introduced | The compile-time source-location macro guard accepts the valid brace-delimited `env!` spelling. Exact case exits 101 after the actual child guard reports 1 passed against the forbidden scratch source; its parentheses control first refuses. | A repository author uses `env!{"CARGO_MANIFEST_DIR"}` in a source-reading test, which compiles and embeds the checkout path just like the prohibited parentheses spelling. This is a prospective policy bypass, not a claim that today's source still uses it. The guard is new in this diff. |
| F3 | crates/ekr/tests/public_surface.rs:94 | warning / CONFIRMED / introduced | The type-use scanner rejects actual generic trait implementations and array-element type positions. Two exact cases exit 101; a scratch Rust compilation confirms the syntax is valid. | A test exercises an exported trait via `impl<T> Widget for Adapter<T>` or accepts an exported type as `[Widget; 2]`. The scanner now considers public types but omits these lexical positions, causing false failures on legitimate coverage. This needs no full Rust name resolution. The new type-use path is introduced by this diff. |

Corrections can remain bounded: prevent supported Cargo comments from silently dropping members (or refuse unsupported syntax explicitly rather than report coverage); recognize Rust's three macro delimiters; recognize the demonstrated type positions without loosening comment/import/identifier controls. No finding requires a semantic Rust resolver, arbitrary re-export traversal, a new runtime feature or a production-format change.

## 5. Attacked without another finding

- Literal/comment opacity and identifier boundaries held for the added controls; existing lexical and behavioral cases still ran.
- README member order is irrelevant, but duplicates, section ambiguity, missing utility and invented utility are refused.
- The same xtask binary follows runtime workspace selection and clearly refuses missing authority paths.
- All inherited assertions and source bytes were preserved; full workspace execution exposed only the four added failures.

## 6. Outside paths and handback

`PRIVATE-inventory.md` records every full scratch path plus the assigned target, temporary fixture root and managed lease scope. It is private operational evidence and is not for a public repository. Public report paths are repository-relative or placeholders. No process started by this review remains running. The adversary lease is released in the retained `lease-release.log`; source and test additions are stable for the next owner.

```findings
- file: crates/ekr/tests/temporal_reads.rs
  line: 44
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: A valid trailing Cargo member comment hides the following product member from the temporal source scan.
- file: crates/ekr/tests/temporal_reads.rs
  line: 153
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: The compile-time source-location macro guard accepts the valid brace-delimited env spelling.
- file: crates/ekr/tests/public_surface.rs
  line: 94
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The type-use scanner rejects actual generic trait implementations and array-element type positions.
```
