---
format: aep.planning-md/2
id: review-result:p1-10-guards-rereview
kind: review-result
status: active
title: 'Guard correction re-review: all original findings resolved'
relations:
- reviews: story:source-guard-debt
revision: 1
---
Owners: 0 findings; no new coordinator or implementor defect observed.

unit: story:source-guard-debt — one correction re-review of the uncommitted diff on 4ae2023d0479f79852376c86e5bcb2d15cbeea31
verdict: nothing found; original F1/F2/F3 closed by the executed correction cases
cases: executed 466→468, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned re-review evidence, target and temporary fixtures; full paths in PRIVATE-inventory.md
needs-coordinator: integration with separately owned runtime/provider changes and the final integration gate

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
 crates/ekr/tests/public_surface.rs                 | 433 ++++++++++++++++-----
 xtask/src/main.rs                                  | 120 +++++-
 22 files changed, 897 insertions(+), 231 deletions(-)
```

Owners: inherited implementation/helper and correction tests — implementor; inherited README/AGENTS/AEP/dependency and integration ownership — coordinator; exactly two appended test cases — this adversary re-review. The non-test paths in the displayed full-unit diff were inherited; this pass did not edit them. Git's stat omits inherited untracked support files; `status.txt` and full before/after hash manifests retain them.

## 1. Bound and byte preservation

Read the complete installed adversary charter and applicable AGENTS before starting, then the correction patch/report and original findings. No AEP command, dependency/manifest/document edit, source repair, commit, publication, cleanup, provider-pin copy or subagent dispatch occurred. Free disk was 15 GB, above the 10 GB floor.

Before additions, captured every tracked/untracked nonignored file hash. After the run, exactly `crates/ekr/tests/public_surface.rs` and `temporal_reads.rs` differ, and their complete dispatch-time bytes remain unchanged prefixes; only the new EOF cases follow them. The new manifest helper and every other inherited source/test/README byte match. Exact test delta: `test-additions.patch`.

The implementor's pre-correction snapshots match this adversary's previous final SHA-256 values for all three original files. A separate read-only comparison then extracted the six original cases and compared their bytes to current source. All six are identical, not merely assertion-equivalent:

```text
public_surface: original adversary cases byte-identical
public_surface: all correction-opening bytes preserved; appended case only
temporal_reads: original adversary cases byte-identical
temporal_reads: all correction-opening bytes preserved; appended case only
adversary_docs_contract: original adversary cases byte-identical
```

`source-before.sha256`, `source-after.sha256`, `source-hash-check.log`, `check-preservation.rs` and `preservation.log` retain the exact checks. The hash-check's two expected changed-file reports are accounted for by the prefix comparisons.

## 2. Focused cases before the full suite

The baseline 466 executed cases came from the correction handback. Two cases were appended before any test execution in this pass:

- `temporal_reads::rereview_member_inventory_reads_literal_hash_paths_and_refuses_partial_globs`: Cargo metadata accepts literal/basic member paths containing `#` and spaces; the actual recursive source scan finds forbidden code in the nested module. Globs, parent-relative paths and duplicate member declarations refuse the inventory instead of reporting a partial scan.
- `public_surface::rereview_generic_impl_recognition_balances_nested_types_without_comparison_mentions`: nested generic bounds, lifetimes, const parameters and qualified array/tuple positions count as use; comparison expressions, imports, suffix identifiers, comments and strings do not.

Each first command was `cargo test -p ekr --offline --test <target> <case> -- --exact --nocapture`, selecting exactly one case and exiting 0. The first raw outputs follow; only private paths are redacted. The expected caught panics are the source guard's explicit unsupported-inventory refusals, not test failures.

```text
   Compiling ekr v0.0.0 (<guard-tree>/crates/ekr)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.22s
     Running tests/temporal_reads.rs (<target>/debug/deps/temporal_reads-bc1f57efcfc34a6f)

running 1 test

thread 'rereview_member_inventory_reads_literal_hash_paths_and_refuses_partial_globs' (971587) panicked at crates/ekr/tests/temporal_reads.rs:38:10:
workspace member grammar must be supported before scanning sources: "unsupported workspace member path: \"crates/*\""
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'rereview_member_inventory_reads_literal_hash_paths_and_refuses_partial_globs' (971587) panicked at crates/ekr/tests/temporal_reads.rs:38:10:
workspace member grammar must be supported before scanning sources: "unsupported workspace member path: \"../outside\""

thread 'rereview_member_inventory_reads_literal_hash_paths_and_refuses_partial_globs' (971587) panicked at crates/ekr/tests/temporal_reads.rs:38:10:
workspace member grammar must be supported before scanning sources: "duplicate workspace members assignment"
test rereview_member_inventory_reads_literal_hash_paths_and_refuses_partial_globs ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.01s

```

```text
   Compiling ekr v0.0.0 (<guard-tree>/crates/ekr)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.14s
     Running tests/public_surface.rs (<target>/debug/deps/public_surface-cf7751befcffc952)

running 1 test
test rereview_generic_impl_recognition_balances_nested_types_without_comparison_mentions ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.00s

```

Then each of the six original adversary cases ran alone with the same exact-selection command. Every log contains `1 passed; 0 failed; 0 ignored`; every command exited 0. `first-status.txt` records all eight exact selections and exits; `first-<case>.log` retains each full raw output.

## 3. Whole workspace and lint results

All Cargo commands used the assigned target, `CARGO_BUILD_JOBS=2` and assigned TMPDIR.

| Command | Actual result | Exit |
|---|---|---:|
| `cargo test --workspace --offline --no-fail-fast` | 77 top-level runners; 468 passed, 0 failed, 0 ignored | 0 |
| `cargo fmt --all --check` | clean | 0 |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | clean | 0 |

Selected runner summaries from the full workspace run:
     Running tests/adversary_docs_contract.rs (<target>/debug/deps/adversary_docs_contract-e9a6b870e5121212)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/public_surface.rs (<target>/debug/deps/public_surface-cf7751befcffc952)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
     Running tests/temporal_reads.rs (<target>/debug/deps/temporal_reads-bc1f57efcfc34a6f)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/public_surface.rs (<target>/debug/deps/public_surface-ad9e3658fc4a0e0a)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

The full raw suite output and exit are `workspace.log` / `workspace.exit`. No baseline runner disappeared. The two additions account for 466→468. A preliminary formatting check found only line wraps in the new temporal case; those were fixed before the full workspace run. No source or test edit followed that final suite, and no second broad suite was run.

This is the guard unit's suite, not a claim about the coordinator's newer provider pin/runtime integration. Those separate changes were intentionally absent here.

## 4. Original finding dispositions

| Finding | Disposition and measured scope |
|---|---|
| F1: Cargo comment hides following member | Closed: unchanged original real-Cargo fixture passes; added quoted-hash/space/nested-source fixture passes. The shared bounded helper is called by both temporal and README guards. Unsupported member grammar explicitly refuses. |
| F2: braces bypass the manifest macro guard | Closed: unchanged original case now executes parentheses, braces and brackets, all refused by the actual child guard. Existing correction controls for raw/escaped/comment-separated arguments and harmless text run in the full suite. |
| F3: genuine type positions rejected | Closed: unchanged original cases now reach generic/qualified impl and array/tuple/slice expectations. Added nested generic and lexical-negative controls pass. |

No remaining blocker or new finding was observed. Limits remain explicit: the manifest helper is a bounded literal-member reader, and API coverage is lexical, not full Rust name resolution or behavioral proof. No demand for unrelated syntax expansion or semantic analysis follows from this pass.

## 5. Outside paths and stable handback

`PRIVATE-inventory.md` lists complete local paths for retained logs/tools, assigned target and temporary fixture root. It is private operational evidence, not a public artifact. Public report and test patch use relative paths/placeholders. The original-case comparison helper only reads repository files.

Own lease `codex-ekr-guard-debt-rereview` is released (`lease-release.log`). No process started by this pass remains running. The implementation and two additive cases are stable for coordinator integration.

```findings
[]
```
