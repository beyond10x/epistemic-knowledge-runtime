unit:                   story:source-guard-debt — correction of independent F1, F2 and F3
verdict:                green
cases:                  executed 461→466, red 0
origin:                 n/a
wrote-outside-worktree: assigned correction scratch and dedicated target; exact paths in PRIVATE-inventory.md
needs-coordinator:      yes — record dispositions and reroute stable source for independent re-review

Owners: implementor — correction/helper and original guard implementation; adversary — six preserved cases; coordinator — README/AGENTS/AEP/Cargo pins/integration/commits.

## 1. Unit and dispositions

Acceptance: resolve all four measured failures without weakening or changing any adversary case; retain complete baseline execution, restore mutation sources byte-identically, and leave compiler/source guards meaningful.

Source remains uncommitted in the assigned guard tree on `4ae2023d0479f79852376c86e5bcb2d15cbeea31`. This correction changes only `crates/ekr/tests/temporal_reads.rs`, `public_surface.rs`, `adversary_docs_contract.rs` and new `support/workspace_manifest.rs`. The new helper was approved and recorded by the coordinator at story scope rev30. The coordinator's newer Eventlog pin was not copied into this tree. No manifest/dependency, product runtime, ESS, AEP, README or AGENTS bytes were changed in this correction.

| Finding | Fix | Class and enumeration |
|---|---|---|
| F1 blocker | Replace comma splitting with one bounded member parser shared by temporal and README guards. | Comments may occur beside the workspace table, array opening, member entries and closing; `#` inside a quoted path remains data. Both basic and literal strings and ordinary whitespace are admitted. Escapes, globs, multiline strings, duplicate declarations/members, malformed separators and unsupported path grammar refuse explicitly. The same parser weakness in README membership is closed in the approved existing test path. |
| F2 blocker | Recognize parentheses, braces and brackets in the executable env macro guard. | All three delimiter forms execute the unchanged adversary child guard. Raw string arguments and qualified/comment-separated invocations are also recognized; Rust-only escapes or nonliteral argument forms that the bounded decoder cannot classify fail closed. Ordinary unrelated env literals and opaque literal/comment controls remain accepted. |
| F3 warning | Recognize array/tuple/slice element positions and an impl's balanced generic parameter prefix. | Both unqualified and qualified generic trait implementations pass; array, tuple and slice controls all execute. The matcher does not treat every preceding `>` as sufficient use. Existing restricted visibility, import-only, partial identifier, comment and string controls remain green. This is still a lexical tripwire, not semantic Rust name resolution. |

All six adversary additions are byte-identical to their correction-opening versions. `witnesses.rs` extracted the two appended blocks and the README case, compared them exactly, and retained the compared bytes; `mutation-summary.log` records the checks. No assertion was removed or weakened.

## 2. Diff

`correction.patch` is the delta from the adversary handback, including the approved new helper. `full-unit-source.patch` includes the complete stable unit and adversary additions, excluding coordinator-owned README. The standard full Git stat below omits untracked files; `status.txt` retains their names.

```text
 README.md                                          |  14 +-
 crates/ekr-core/tests/adversary2_public_surface.rs |   5 +-
 crates/ekr-core/tests/identity_serde.rs            |  12 +-
 crates/ekr-core/tests/public_surface.rs            |  12 +-
 .../tests/adversary2_guard_bounds_and_ranges.rs    |  76 ++--
 .../tests/adversary2_membrane_and_addresses.rs     |   7 +-
 .../tests/adversary_canonical_value_reach.rs       |   6 +-
 .../tests/adversary_snapshot_and_assertion.rs      |  33 +-
 .../tests/canonical_value_and_assertion.rs         | 140 ++++++-
 crates/ekr-graph/tests/domain_projection.rs        |  18 +-
 crates/ekr-graph/tests/revision_events.rs          |  10 +-
 crates/ekr-kernel/tests/validation.rs              |   3 +-
 crates/ekr-ontology/tests/domain_projection.rs     |  19 +-
 .../tests/inheritance_and_declaration_coherence.rs |  24 +-
 crates/ekr-ontology/tests/value_type_checking.rs   |   2 +-
 .../tests/adversary2_retention_event_contract.rs   |   8 +-
 crates/ekr-store/tests/domain_projection.rs        |  13 +-
 crates/ekr-store/tests/providers.rs                |   7 +-
 crates/ekr/tests/adversary_docs_contract.rs        | 154 ++++++--
 crates/ekr/tests/msrv_contract.rs                  |  12 +-
 crates/ekr/tests/public_surface.rs                 | 410 ++++++++++++++++-----
 xtask/src/main.rs                                  | 120 +++++-
 22 files changed, 874 insertions(+), 231 deletions(-)
```

## 3. Observed red baseline and mutations

`cargo test --workspace --offline --no-fail-fast` before correction exited 101: 457 passed, 4 failed, 0 ignored; 461 executed in 77 top-level runner summaries. Its complete path-sanitized output is `public-evidence/baseline-workspace.log`. The nested child guard summary inside the failure diagnostic has filtered cases and is not counted as another top-level runner. The four original failures are the exact cases listed below.

Each original exact case was first rerun after its fix with `cargo test -p ekr --offline --test <target> <case> -- --exact --nocapture`: one executed, one passed, exit 0. This reaches the bracket, qualified-path, tuple and slice expectations that the original failure had aborted before.

`cargo test -p ekr --offline --test temporal_reads adversary_cargo_member_comment_cannot_hide_a_product_source -- --exact --nocapture`

```text
   Compiling ekr v0.0.0 (<guard-tree>/crates/ekr)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.22s
     Running tests/temporal_reads.rs (<target>/debug/deps/temporal_reads-bc1f57efcfc34a6f)

running 1 test
test adversary_cargo_member_comment_cannot_hide_a_product_source ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.01s


```

`cargo test -p ekr --offline --test temporal_reads adversary_manifest_macro_guard_rejects_all_rust_delimiters -- --exact --nocapture`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/temporal_reads.rs (<target>/debug/deps/temporal_reads-bc1f57efcfc34a6f)

running 1 test
test adversary_manifest_macro_guard_rejects_all_rust_delimiters ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s


```

`cargo test -p ekr --offline --test public_surface adversary_generic_trait_implementation_is_an_actual_type_use -- --exact --nocapture`

```text
   Compiling ekr v0.0.0 (<guard-tree>/crates/ekr)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.16s
     Running tests/public_surface.rs (<target>/debug/deps/public_surface-cf7751befcffc952)

running 1 test
test adversary_generic_trait_implementation_is_an_actual_type_use ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s


```

`cargo test -p ekr --offline --test public_surface adversary_tuple_and_array_elements_are_actual_type_uses -- --exact --nocapture`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/public_surface.rs (<target>/debug/deps/public_surface-cf7751befcffc952)

running 1 test
test adversary_tuple_and_array_elements_are_actual_type_uses ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s


```

The additional macro argument control was run before its correction and failed on the raw-string form. Command: `cargo test -p ekr --offline --test temporal_reads macro_argument_spellings_cannot_hide_compile_time_manifest_authority -- --exact --nocapture`, exit 101.

```text
   Compiling ekr v0.0.0 (<guard-tree>/crates/ekr)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.18s
     Running tests/temporal_reads.rs (<target>/debug/deps/temporal_reads-bc1f57efcfc34a6f)

running 1 test

thread 'macro_argument_spellings_cannot_hide_compile_time_manifest_authority' (943670) panicked at crates/ekr/tests/temporal_reads.rs:90:9:
std::env! { r#"CARGO_MANIFEST_DIR"# }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test macro_argument_spellings_cannot_hide_compile_time_manifest_authority ... FAILED

failures:

failures:
    macro_argument_spellings_cannot_hide_compile_time_manifest_authority

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr --test temporal_reads`
```

Four temporary correction reversions then made each original case fail again, one case per exact command: disabling comment recognition, restoring parenthesis-only macro recognition, removing generic-impl recognition, and removing array/tuple recognition. Each exited 101 with `0 passed; 1 failed`. Their complete outputs are `public-evidence/mutation-F1.log`, `mutation-F2.log`, `mutation-F3-generic.log` and `mutation-F3-elements.log`. Restoration is byte-identical, independently checked against pre-mutation SHA-256 values:

```text
public_surface: adversary cases preserved exactly
temporal_reads: adversary cases preserved exactly
adversary_docs_contract: adversary cases preserved exactly
adversary_cargo_member_comment_cannot_hide_a_product_source: mutation exit101, one failed; source restored byte-identically
adversary_manifest_macro_guard_rejects_all_rust_delimiters: mutation exit101, one failed; source restored byte-identically
adversary_generic_trait_implementation_is_an_actual_type_use: mutation exit101, one failed; source restored byte-identically
adversary_tuple_and_array_elements_are_actual_type_uses: mutation exit101, one failed; source restored byte-identically
crates/ekr/tests/temporal_reads.rs: OK
crates/ekr/tests/public_surface.rs: OK
crates/ekr/tests/adversary_docs_contract.rs: OK
crates/ekr/tests/support/workspace_manifest.rs: OK
```

## 4. Final green execution

All Cargo runs used the assigned target, two build jobs and the original assigned test TMPDIR. Last free-space observation was 15 GB, above the 10 GB stop threshold.

| Command / lane | Executed baseline → final | Exit |
|---|---|---:|
| `cargo test --workspace --offline --no-fail-fast` | 461 → 466; 457 pass/4 fail → 466 pass/0 fail; 0 ignored | 0 |
| `public_surface` within that command | 11 → 11; 9 pass/2 fail → 11 pass | 0 |
| `temporal_reads` within that command | 9 → 12; 7 pass/2 fail → 12 pass | 0 |
| `adversary_docs_contract` within that command | 5 → 7, all pass | 0 |
| `cargo fmt --all --check` | non-test check | 0 |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | non-test check | 0 |

The five additional executed cases are two shared manifest-parser controls compiled in each of the two guard targets, plus one macro-argument control. No baseline runner disappeared. The final complete raw command output is retained in `workspace-restored-final.log`; its public copy in `public-evidence/` differs only in local path placeholders. `fmt-final.log` and `clippy-final.log` retain their command output. The final whole suite ran after all mutation sources were restored; no further source edits followed it.

This correction did not rerun the root integration gate or claim its result. The original unit's rustdoc check was green; this round's changed files are integration-test guards only. The coordinator owns complete integration checks after its disjoint runtime/provider pin changes are combined.

## 5. Boundaries and handback

The workspace helper is deliberately a bounded literal-member parser. It supports the repository's ordinary member-array grammar and comments, and refuses unsupported syntax instead of claiming incomplete coverage. It does not expand globs or become a general TOML/Cargo graph resolver. The public type scan remains a lexical tripwire; no name-resolution or macro-expansion proof is claimed. Unsupported env argument syntax is a conservative source-guard refusal, not permission to embed a checkout path.

All inherited current/frozen canonical rosters and behavioral tests remain unchanged. No customer payload, source store, external integration or service was accessed. No commits, publication, planning mutation or tree cleanup was performed. The correction lease is released at stable handback; the coordinator owns re-review and integration.

## 6. Outside paths

All explicit writes outside the managed implementation tree are in the assigned correction scratch and the dedicated build target; test-owned temporary fixtures use the already assigned TMPDIR and clean themselves up. `PRIVATE-inventory.md` records the exact local paths and is not for public commit. Public logs use placeholders. The scratch Rust witness/report tools are not product or gate code.
