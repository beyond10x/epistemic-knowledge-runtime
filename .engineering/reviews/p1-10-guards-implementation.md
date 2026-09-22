unit:                   story:source-guard-debt — Make repository guards check the invoking tree and declared surface
verdict:                green
cases:                  executed 431→455, red 0
origin:                 n/a
wrote-outside-worktree: assigned scratch, dedicated build target, restored managed witness; PRIVATE inventory holds exact paths
needs-coordinator:      yes — independent review/integration, root-owned AGENTS update and already-applied readme-coordinator.patch

Owners: source/tests/xtask — Codex implementor; README/AGENTS/AEP/commits/integration — coordinator.

## 1. Unit and implementation

Acceptance: source guards observe the invoking checkout at runtime, recursively reject the two forbidden temporal-read expressions, require code use of exported types, and reconcile README Status with Cargo product members while preserving existing behavioral/canonical-family checks.

Opening: `4ae2023d0479f79852376c86e5bcb2d15cbeea31`, branch `codex/ekr-p1-10-guards-20260922`. The source is stable and uncommitted; no production runtime, format, ESS, dependency, AEP or AGENTS bytes changed by the implementor. The coordinator separately supplied the README edit. Ambient-runtime refusal remains a separate source unit.

Runtime paths: all 34 executable compile-time manifest-path sites in 18 files at this opening now resolve `CARGO_MANIFEST_DIR` per invocation. One raw text match remains solely in an existing documentation quotation. The new recursive guard verifies executable tokens, so the historical 19/38 count is not reused as a current inventory. `xtask doctor` uses a trusted runtime manifest or walks upward from its current directory; invalid explicit manifests and unrelated directories refuse.

The workspace temporal guard visits every product workspace member's source recursively. It rejects token sequences `valid_time.is_open()` and `valid_time.to.is_none()`, including whitespace/comments between code tokens, while literals and transaction-time calls remain legitimate. Existing graph behavioral assertions remain present.

The public-surface scanner recognizes unrestricted explicit structs, enums, traits and aliases plus existing functions/constants. It skips comments, keeps literals opaque, respects identifier boundaries, excludes restricted declarations and inaccessible inline-module types, and requires actual code positions rather than imports alone. Tests cover grouped imports, lifetime/reference positions, unit-value method calls, trait bounds, nested comments, raw/byte/C strings, char escapes and Unicode identifier boundaries. Public functions/constants retain the original path/method criterion.

Strengthening the scanner exposed previously implicit public API coverage. Existing parse-error, ontology-error, validator, provider and object-store assertions now carry explicit types. Added behavioral cases cover public node lookup, canonical target deserialization/address stability, confidence boundaries, current/frozen range error conversion, entity display and structured membrane-error conversion. No bare name-only assertions were added.

Inferred scope was checked before implementation: README Status needed a real membership statement; the new temporal source guard needed a new workspace test target. The shared lexer helper was approved as new inferred scope (rev25). Additional ontology/store test paths were approved at rev28 and the kernel Validator annotation at rev29; planning was left to the coordinator. No inferred path was silently treated as established.

Mechanism measurement: before the fix, a guard compiled in the implementation checkout passed while a second managed checkout contained an untested public function and was selected through runtime `CARGO_MANIFEST_DIR`. After the fix, the same core guard binary passed on the clean implementation checkout, failed on that second checkout, then passed when its exact original source was restored. Logs retain both the original false green and the correction.

## 2. Observed diff

`git diff --stat` output below includes the coordinator-owned README. Untracked additions are listed separately because Git's normal stat omits them. `source.patch` includes all implementor changes and new files, excluding README.

```text
 README.md                                          |  14 +-
 crates/ekr-core/tests/adversary2_public_surface.rs |   5 +-
 crates/ekr-core/tests/identity_serde.rs            |  12 +-
 crates/ekr-core/tests/public_surface.rs            |  12 +-
 .../tests/adversary2_guard_bounds_and_ranges.rs    |  76 +++--
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
 crates/ekr/tests/adversary_docs_contract.rs        | 119 +++++--
 crates/ekr/tests/msrv_contract.rs                  |  12 +-
 crates/ekr/tests/public_surface.rs                 | 351 +++++++++++++++------
 xtask/src/main.rs                                  | 120 ++++++-
 22 files changed, 787 insertions(+), 224 deletions(-)
```

New files:

- `crates/ekr/tests/support/rust_source.rs`
- `crates/ekr/tests/temporal_reads.rs`
- `crates/ekr-store/tests/document_refusals.rs`

The complete changed-file inventory is `final-status.txt`; the complete runner inventory is `runner-inventory.md`.

## 3. Red-first evidence

Commands below were run before their implementations. Output is verbatim except replacement of local absolute paths with placeholders. Each selected lane exited 101.

`cargo test -p ekr --test public_surface`

```text
   Compiling ekr v0.0.0 (<guard-tree>/crates/ekr)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.14s
     Running tests/public_surface.rs (<target>/debug/deps/public_surface-cf7751befcffc952)

running 3 tests
test exported_type_declarations_are_not_invisible ... FAILED
test strings_and_comments_do_not_exercise_public_functions ... FAILED
test no_public_item_in_any_crate_is_untested ... ok

failures:

---- exported_type_declarations_are_not_invisible stdout ----

thread 'exported_type_declarations_are_not_invisible' (743459) panicked at crates/ekr/tests/public_surface.rs:216:9:
assertion `left == right` failed
  left: None
 right: Some("UnusedType")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- strings_and_comments_do_not_exercise_public_functions stdout ----

thread 'strings_and_comments_do_not_exercise_public_functions' (743461) panicked at crates/ekr/tests/public_surface.rs:237:9:
let text = "Thing::unexercised()";


failures:
    exported_type_declarations_are_not_invisible
    strings_and_comments_do_not_exercise_public_functions

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

error: test failed, to rerun pass `-p ekr --test public_surface`

```

`cargo test -p ekr --test adversary_docs_contract`

```text
   Compiling ekr v0.0.0 (<guard-tree>/crates/ekr)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.15s
     Running tests/adversary_docs_contract.rs (<target>/debug/deps/adversary_docs_contract-e9a6b870e5121212)

running 4 tests
test status_membership_refuses_absence_omission_and_invented_crates ... FAILED
test the_readme_states_the_workspace_rust_version ... ok
test the_readme_status_matches_the_workspace_members ... ok
test every_crate_doc_comment_names_an_existing_ess_domain_file ... ok

failures:

---- status_membership_refuses_absence_omission_and_invented_crates stdout ----

thread 'status_membership_refuses_absence_omission_and_invented_crates' (747293) panicked at crates/ekr/tests/adversary_docs_contract.rs:129:9:
# Readme
No status.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    status_membership_refuses_absence_omission_and_invented_crates

test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr --test adversary_docs_contract`

```

`cargo test -p ekr --test temporal_reads`

```text
   Compiling ekr v0.0.0 (<guard-tree>/crates/ekr)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.21s
     Running tests/temporal_reads.rs (<target>/debug/deps/temporal_reads-bc1f57efcfc34a6f)

running 7 tests
test both_forbidden_spellings_are_code_not_text ... FAILED
test rust_source::tests::chars_escapes_and_lifetimes_keep_distinct_boundaries ... ok
test rust_source::tests::identifiers_do_not_join_across_comments_or_split_at_unicode ... ok
test rust_source::tests::comments_and_all_string_forms_are_not_code_identifiers ... ok
test no_product_source_rebuilds_the_open_ended_valid_time_filter ... ok
test the_same_policy_reaches_graph_kernel_store_and_nested_modules ... FAILED
test executable_source_location_macros_cannot_return ... FAILED

failures:

---- both_forbidden_spellings_are_code_not_text stdout ----

thread 'both_forbidden_spellings_are_code_not_text' (767672) panicked at crates/ekr/tests/temporal_reads.rs:64:9:
record.valid_time.is_open()
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- the_same_policy_reaches_graph_kernel_store_and_nested_modules stdout ----

thread 'the_same_policy_reaches_graph_kernel_store_and_nested_modules' (767678) panicked at crates/ekr/tests/temporal_reads.rs:98:9:
assertion `left == right` failed: crates/ekr-graph/src/lib.rs
  left: []
 right: ["<scratch>/.tmp6uGNS2/crates/ekr-graph/src/lib.rs"]

---- executable_source_location_macros_cannot_return stdout ----

thread 'executable_source_location_macros_cannot_return' (767673) panicked at crates/ekr/tests/temporal_reads.rs:122:5:
compile-time checkout readers: ["<guard-tree>/xtask/src/main.rs"]


failures:
    both_forbidden_spellings_are_code_not_text
    executable_source_location_macros_cannot_return
    the_same_policy_reaches_graph_kernel_store_and_nested_modules

test result: FAILED. 4 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

error: test failed, to rerun pass `-p ekr --test temporal_reads`

```

`cargo test -p xtask`

```text
   Compiling xtask v0.0.0 (<guard-tree>/xtask)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.12s
     Running unittests src/main.rs (<target>/debug/deps/xtask-3d6662b0e8396912)

running 3 tests
test tests::missing_runtime_manifest_and_unrelated_directory_refuse ... FAILED
test tests::runtime_manifest_selects_its_own_workspace ... FAILED
test tests::standalone_invocation_walks_from_its_current_directory ... FAILED

failures:

---- tests::missing_runtime_manifest_and_unrelated_directory_refuse stdout ----

thread 'tests::missing_runtime_manifest_and_unrelated_directory_refuse' (772147) panicked at xtask/src/main.rs:76:9:
assertion failed: resolve_workspace(None, &fixture.0).is_err()
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- tests::runtime_manifest_selects_its_own_workspace stdout ----

thread 'tests::runtime_manifest_selects_its_own_workspace' (772148) panicked at xtask/src/main.rs:62:9:
assertion `left == right` failed
  left: "<guard-tree>"
 right: "<scratch>/ekr-xtask-772146-1"

---- tests::standalone_invocation_walks_from_its_current_directory stdout ----

thread 'tests::standalone_invocation_walks_from_its_current_directory' (772149) panicked at xtask/src/main.rs:70:9:
assertion `left == right` failed
  left: "<guard-tree>"
 right: "<scratch>/ekr-xtask-772146-2"


failures:
    tests::missing_runtime_manifest_and_unrelated_directory_refuse
    tests::runtime_manifest_selects_its_own_workspace
    tests::standalone_invocation_walks_from_its_current_directory

test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p xtask --bin xtask`

```

The core same-binary witness and actual source mutations supplement these first red runs:

| Mutation | Observed refusal and recovery | Evidence logs |
|---|---|---|
| Same core guard binary, second managed checkout | Original checkout pass 1; mutated witness fail 1; exact restoration pass 1 | `runtime-current-green`, `runtime-witness-red`, `runtime-witness-restored` |
| Forbidden temporal expression in graph, kernel, store | Each actual source edit fail 1; each exact restoration pass 1 | `temporal-ekr-{graph,kernel,store}-{red,restored}` |
| Forbidden temporal expression in a new nested source file | Fail 1, exact temporary file removal pass 1 | `temporal-nested-{red,restored}` |
| README missing Status / omitted graph / invented product | Each fail 1; approved README pass 1; original witness README restored byte-identically | `readme-source-{missing,omitted,invented}-red`, `readme-source-restored-green` |
| Struct, enum, trait and alias with only comments/string mentions in tests | Fail 1 naming all four; actual typed code makes guard pass 1 and its two real Rust tests pass; original source/tests restored byte-identically | `public-source-red`, `public-source-covered-green`, `public-mutation-real-tests`, `public-source-restored-green` |

The second tree is clean at handback. Mutation fixtures were synthetic and task-owned. No primary checkout was changed. The evidence directory includes `runtime-path-before.log`, the old guard's false-green observation; it is deliberately not presented as success evidence.

## 4. Final verification

All commands ran with the assigned dedicated target, two build jobs and scratch TMPDIR. Free disk remained above the 10 GB stop threshold; last observed 16 GB.

| Command/lane | Executed before → after | Exit | Verbatim evidence |
|---|---|---:|---|
| `cargo test --workspace` | 431 → 455, zero failures/ignored | 0 | `public-evidence/workspace-test-final-exact.log` |
| public-surface target within full workspace command | 1 → 8 | 0 | runner inventory and full output |
| README target within full workspace command | 3 → 4 | 0 | runner inventory and full output |
| xtask within full workspace command | 0 → 3 | 0 | runner inventory and full output |
| graph canonical value/assertion target | 18 → 21 | 0 | runner inventory and full output |
| ontology inheritance target | 13 → 14 | 0 | runner inventory and full output |
| new temporal target | absent on base → 7 | 0 | runner inventory and full output |
| new structured document refusal target | absent on base → 2 | 0 | runner inventory and full output |
| `cargo fmt --all --check` | not a test lane | 0 | `public-evidence/fmt-final.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | not a test lane | 0 | `public-evidence/clippy-final-green.log` |
| `RUSTDOCFLAGS='-D rustdoc::broken_intra_doc_links' cargo doc --workspace --no-deps` | not a test lane | 0 | `public-evidence/doc-final.log` |
| `cargo run -q -p xtask -- doctor` | actual invocation, correct current tree | 0 | `public-evidence/xtask-cargo-doctor.log` |
| Standalone same xtask binary, manifest environment removed, cwd under witness | actual invocation, correct witness root | 0 | `public-evidence/xtask-standalone-witness.log` |
| Standalone same xtask binary, unrelated scratch cwd, manifest environment removed | explicit no-workspace refusal | 1 expected | `public-evidence/xtask-standalone-refusal.log` |

The full output in the named public-evidence files is retained, with only path redaction. `runner-inventory.md` parses all 75 baseline and 77 final runner summaries; every baseline lane remains. This unit did not claim a complete integration `task check`: planning/integration and the independent review remain coordinator work.

An initial compile trial caught a missing graph test dependency reference; the test was rewritten with the already available serde deserializer rather than changing manifests. Another trial caught an accidentally removed Path import; it was restored. Lint trials caught a redundant test clone and a modulo-expression style lint; both were corrected, with no allow attributes. Their failed logs remain in the inventory rather than being discarded.

## 5. Deliberate boundaries

These are lexical policy tripwires, not semantic Rust name resolution or proof of behavioral coverage. They do not claim to detect every equivalent temporal expression, macro-generated API, or arbitrary re-export/alias graph. External source modules are conservatively scanned; restricted inline module types are excluded. Existing compile-fail and canonical mutation tests continue to establish substantive product boundaries.

Canonical current/frozen family rosters, enum numbering, mutation lists and behavioral assertions were preserved. No production visibility was changed merely to satisfy a guard. No new dependency or product wire/schema behavior was introduced. Exact diagnostic pin policy, actual runtime refusal, ESS byte-string repair and V2 activation remain separately owned. README prose was applied by the coordinator; AGENTS current-count/policy prose and planning are coordinator-owned and were not edited here.

The two managed checkout leases are released at stable handback. No commit, publication, managed-tree removal, build-cache removal or planning mutation was performed by this unit.

## 6. Files outside the managed implementation checkout

Exact local paths are deliberately confined to `PRIVATE-inventory.md`, which must not be committed to a public repository. It enumerates scratch evidence, the assigned build target and the restored managed witness source mutations. All public evidence uses `<guard-tree>`, `<witness-tree>`, `<scratch>`, `<target>` and `<home>` placeholders. `readme-coordinator.patch` is the exact prose patch already applied by the coordinator; no outstanding source patch is required to turn this unit green.
