unit: task:bounded-transaction-document-parser, correction R1 frozen uncommitted source over 078bd7d0c944544a87f60c3c77753e2ca8c6de54, identified by correction-r1/source-final.sha256 plus inherited Taskfile
verdict: nothing found
cases: executed 328→334, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 assigned output roots, with full private inventory retained
needs-coordinator: integrate the reviewed source and retained standalone vendor lock; broader gate and writer acceptance remain outside this review

```text
$ git --no-pager diff --stat
 .../waves/p1-transaction-document-parser-brief.md  | 10 +++++++++
 Cargo.lock                                         |  2 --
 Cargo.toml                                         |  4 ++++
 Taskfile.yml                                       | 25 ++++++++++++++++++++++
 crates/ekr-core/src/lib.rs                         |  1 +
 crates/ekr-kernel/src/lib.rs                       |  4 ++++
 crates/ekr-kernel/src/transaction.rs               | 18 ++++++++++++++++
 crates/ekr-ontology/src/types.rs                   |  4 ++--
 8 files changed, 64 insertions(+), 4 deletions(-)
```

Every tracked change above was inherited. The coordinator owns the briefs/reviews and Taskfile gate; the implementor owns the parser, shared helper and vendor changes. Ordinary unstaged diff omits their new files and my added file. **The only worktree path authored in this second review is the new 182-line `crates/ekr-kernel/tests/adversary_transaction_document_r2.rs`**, SHA-256 `c0cfdd88bbf3b025871e190c5dfedcd6f3f9e3c21a7d44fac84ed1b5c162997e`. No existing source, test, manifest, document or fixture was modified.

All 49 implementor manifest entries verify before and after execution. The inherited Taskfile hash also matches afterward. Original independent test SHA-256 `3527686215250e52b1cea3b28b1102f73821e483a5b62117671aa9126cad8ba3` and original exact YAML SHA-256 `6ff71359ad6d5edf003b28b86f19c4636c40d11d3cfb57d3e19cba7345d4fdce` remain unchanged. Historical reports, the mechanism addendum and first red logs are preserved.

## Cases first

The six new deterministic cases were written before any of their executions and before the package/vendor suites. Each first run used:

```sh
cargo test --offline --locked -p ekr-kernel --test adversary_transaction_document_r2 CASE -- --exact --nocapture
```

Every selected first run reports **1 passed, 0 failed, 0 ignored, 5 filtered out**, exit **0**. Each complete first-run log and separate status is retained as `first-CASE.log` and `first-CASE.status`.

| New case | Actual assertion exercised |
|---|---|
| `directive_expanded_tags_and_aliases_share_an_exact_total` | A directive expands a 32,000-byte global tag at 32 operation occurrences; the independently tallied 1,048,576-byte total loads, one extra scalar byte refuses TotalStringBytes. Exact bytes and payload hash survive. |
| `percent_decoded_utf8_tag_bytes_have_their_own_inclusive_limit` | Percent-encoded multibyte tag text counts decoded UTF-8 bytes: 65,536 loads, 65,537 refuses StringBytes while raw input remains below its cap. |
| `tagged_alias_keys_refuse_duplicates_before_malformed_values` | Aliased global/local tagged keys refuse a duplicate before its unknown-alias or recursive second value. Distinct requested String keys remain data. |
| `reordered_enum_content_matches_the_shared_typed_decoder` | Eight value-before-discriminant inputs exercise buffered scalar/collection decoding, including tags and null. Both admitted typed values and invalid-document refusals agree with the shared decoder; positive and negative branches are required to execute. |
| `assertion_object_enum_boundaries_contribute_semantic_depth` | The actual assertion-object enum boundary counts: depth 32 loads, an additional nested Value map refuses Depth. |
| `unknown_alias_after_an_otherwise_complete_document_never_disappears` | Malformed or unknown-alias second documents return Documents; a missing alias inside the first document remains InvalidDocument. |

Before those isolated executions, the original unchanged case was rerun exactly:

```sh
cargo test --offline --locked -p ekr-kernel --test adversary_transaction_document tags_and_typed_scalar_strings_share_one_expanded_string_budget -- --exact --nocapture
```

`original-witness.log`, exit **0**, reports:

```text
running 1 test
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.02s
```

The prior acceptance failure is closed for that retained witness: the same positive controls still execute and the oversized document now returns its required limit. The preserved mechanism addendum remains necessary; this passing result does not retroactively validate the first report's retracted explanation.

## Scoped suites and gate

The baseline **328 passed, 0 failed, 0 ignored** comes from the implementor's final R1 handback, not a pre-addition suite run. All commands used the assigned sequential target, task-owned TMPDIR, two build jobs/test threads, offline resolution, no incremental compilation and no dev/test debug info.

```sh
cargo test --offline --locked -p ekr-core -p ekr-ontology -p ekr-kernel --no-fail-fast
```

`packages.log`, exit **0**, contains 42 runner summaries totaling **334 passed, 0 failed, 0 ignored**. The new target's exact summary is:

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

The same run executes all five original independent cases, 36 implementor parser cases, the allocating-visitor witnesses, legacy controls and unchanged compile-fail cases.

`task vendor-check` actually ran, exit **0**, with `CARGO_NET_OFFLINE=true`. Its logged steps were:

```sh
rustfmt --edition 2021 --check vendor/serde_yaml_ng/src/observation.rs vendor/serde_yaml_ng/tests/test_observation.rs
cargo test --manifest-path vendor/serde_yaml_ng/Cargo.toml --locked --target-dir "${CARGO_TARGET_DIR:-target}/vendor-serde_yaml_ng"
cargo doc --manifest-path vendor/serde_yaml_ng/Cargo.toml --no-deps --locked --target-dir "${CARGO_TARGET_DIR:-target}/vendor-serde_yaml_ng"
```

The vendor tests report **182 passed, 0 failed, 0 ignored** across seven runner summaries: 176 retained upstream cases including 85 doctests, plus six observation cases. Strict documentation runs with the Taskfile's `RUSTDOCFLAGS=-D warnings`. Full raw output is in `vendor-check.log`.

Additional executed checks:

| Command | Exit |
|---|---:|
| `cargo clippy --offline --locked -p ekr-core -p ekr-ontology -p ekr-kernel --all-targets -- -D warnings` | 0 |
| `cargo fmt --all -- --check` | 0 |
| `git diff --check` | 0 |
| Original upstream four-test-file SHA-256 manifest check | 0 |

Whole-vendor formatting and strict Clippy were not newly claimed or rerun here. Their unchanged upstream baseline limitations remain explicitly documented in R1's handback. The actual adopted gate's focused formatting, compatibility tests and strict docs completed.

## Vendor and source assessment

The cached published crate archive hashes to `7b4db627b98b36d4203a7b458cf3573730f2bb591b28871d916dfa9efabfd41f`, matching retained provenance and the original root lock. Its VCS metadata names `45bce7bd7efb754ffc9dc910f3d1bae7edd0adf4`. Original manifest, original manifest source, LICENSE, README and .gitignore compare byte-for-byte with the installed package. All four original upstream test files match their retained SHA-256 manifest. An initial read-only comparison used the nonexistent name LICENSE-MIT; the corrected comparison against the actual LICENSE file succeeds. No file was changed for that command error.

I inspected the complete vendor source delta: the new observation facade/export, crate visibility for the existing scalar/tag helpers, and the enumerated explicit reference/lifetime spellings. No default scalar/tag resolution branch was altered. The facade borrows the existing loader's metadata and alias positions without eager expansion; its optional classification calls the existing resolver. The kernel instead leaves actual typed scalar interpretation to the original deserializer, charging scalar String callbacks and actual enum boundaries there. It does not introduce a second scalar grammar.

No concrete new discrepancy was found. The original missing-tag acceptance finding has a passing unchanged regression; this is a bounded test outcome, not approval or a universal correctness claim.

Owners: the implementor owns the corrected parser/dependency source and its earlier defects; I own only the six new cases, this report and my previously corrected mechanism attribution. The coordinator owns the inherited gate/contract changes, vendoring adoption, standalone lock integration, full project gate, commits and publication. No new implementor- or coordinator-owned finding is returned.

## Coverage limits and handback

- Expanded directive/global/local tag bytes, typed scalar credits, alias key identity and duplicate-before-value refusal were exercised without changing decoder semantics.
- Actual assertion enum depth and reordered buffered enum content were exercised against the shared typed decoder.
- Inclusive raw/string/key/node/container/operation/evidence bounds, allocation-order witnesses, nonfinite spellings and legacy controls ran through the inherited scoped suite.
- The raw cap still bounds eager loader input; this review establishes no pre-loader allocation quota.
- No writer, persisted proposal, validation/replay/restart/provider contract, real store or external service was tested. No full-project or phase-completion claim is made.
- No source repair, AEP command, commit, publication, cleanup or third review dispatch occurred. All invoked builds have returned; no process started by this review remains running.

Three external roots contain the assigned review scratch (including its own tmp directory), the assigned Cargo target (including the vendor subtarget), and managed lease bookkeeping. Exact private host paths/environment and retained scratch-file inventory are separate in `private-paths.md` and `retained-scratch-files.txt`. Lease release and final status are retained in `lease-release.log` and `handback-status.md`; only this review's lease is released.

```findings
[]
```

