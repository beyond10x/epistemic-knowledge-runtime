# Legacy integration coverage correction

Story: version-persisted-contracts, reviewed original-format preparation slice.
Base: 5ae71d16f1119daaadaf16d9b362162a342839be.
Scope: crates/ekr-store/tests/legacy.rs only.
Status: five direct boundary tests added; all requested checks green.
Findings: no production defect found or corrected.
Owners: coordinator owns the integrated public-surface failure and final gate; this worker owns the five added tests and their observed results.

The integrated gate reported six public APIs without direct usage: graph legacy unique_map/unique_set and store legacy verify_payload/verify_value/GraphDocument::check_identities/knowledge_bytes. Existing kernel tests called imported verification functions and serde used collection helpers indirectly, which did not meet the current guard's path/method criterion. The APIs are actually public cross-crate boundaries, so this correction tests them directly rather than reducing visibility or adding inert name mentions.

## Added acceptance

| Case | Boundary and independent expectation |
|---|---|
| unique_map_refuses_decoded_duplicate_keys_and_preserves_unique_entries | Direct unique_map calls refuse repeated literal and escaped-equivalent keys; empty and unsorted unique maps preserve their complete contents. |
| unique_set_refuses_decoded_duplicate_members_and_preserves_unique_entries | Direct unique_set calls refuse repeated literal and escaped-equivalent string members; empty and unsorted distinct collections remain admitted. |
| direct_address_verifiers_distinguish_exact_payloads_from_frozen_values | Direct payload/value verification uses original scalar-node capture hashes. Swapping hash domains returns exact mismatch variants with expected/observed fields. The same canonical bytes hashed in the payload domain are refused as a value address. JSON whitespace changes the payload address while preserving decoded value identity. |
| direct_identity_check_refuses_each_misfiled_collection | Bypass verify_bytes and call check_identities on an intact decoded document, then independently corrupt node, edge, assertion and evidence record identities while retaining their map keys. Every collection returns exactly InvalidData("misfiled-identity"). |
| knowledge_bytes_keep_original_map_framing_and_captured_record_order | Compare the complete emitted knowledge bytes against fixed original map tags/counts/UUID keys plus original 73ab8b0 captured node/edge/assertion canonical bytes. No encoder manufactures the expected record bytes. Root/revision/evidence changes leave this sub-root's bytes unchanged, and empty collections match fixed empty-map bytes. |

The existing fixture-reading helper now selects named original vectors and checks source_commit, retaining runtime CARGO_MANIFEST_DIR lookup. No fixture, original adversary test, production source, manifest, planning record or document was changed by this worker. Tests only; no claim of new format activation, migrated history or provider behavior.

## Observed checks

All cargo commands used the coordinator target serially, CARGO_BUILD_JOBS=2 and this report directory as TMPDIR. Disk checks preceded compiler invocations: available space remained above the 10 GiB floor (observed 19 → 17 GiB).

| Exact command (common environment below) | Exit | Observed result | Log |
|---|---:|---|---|
| cargo test --locked -p ekr-store --test legacy | 0 | 9 passed, 0 failed/ignored; existing 4 plus new 5 | store-legacy.log |
| cargo test --locked -p ekr --test public_surface | 0 | 1 passed, 0 failed/ignored | public-surface.log |
| cargo test --locked -p ekr-kernel --test legacy --test adversary_legacy_freeze | 0 | 13 legacy + 6 retained adversary passed, 0 failed/ignored | kernel-legacy.log |
| cargo clippy --locked -p ekr-store --test legacy -- -D warnings | 0 | no warnings | clippy.log |
| rustfmt --edition 2021 --check crates/ekr-store/tests/legacy.rs | 0 | formatted | fmt.log |
| git diff --check -- crates/ekr-store/tests/legacy.rs | 0 | no whitespace errors | diff-check.log |

Common environment, with public-safe path aliases:

```sh
CARGO_TARGET_DIR=<cache>/b10x-target/ekr-completion-20260922
CARGO_BUILD_JOBS=2
TMPDIR=<cache>/ekr-completion-20260922/guard-debt/legacy-integration-correction
```

Total targeted execution: 29 test cases passed, including 5 new cases; no ignored cases. The original public-surface red was reported by the coordinator before dispatch, not independently rerun here before editing. The new behavioral cases passed on their first execution, so there is no claimed production behavioral red or test-driven source correction.

A help discovery attempt used unsupported worktree lease; the documented worktree hook session-start/session-end interface was used for the actual lease. Read-only searches encountered two obsolete wave filenames and a nonexistent revision.rs; actual inspected locations were current wave documentation and core/identity.rs. No repository mutation resulted from those discovery misses.

## Handback

Stable test file SHA-256:
a976cebb56f4d83ef80be264e002d37d3c0cef788d8c676679f19660aca76cf2.

One test file changed, 204 insertions / 7 deletions. Root's concurrent planning/scope edits are outside this worker's diff. Own lease codex-ekr-legacy-integration-correction is released at handback; no compiler remains running. Coordinator owns the final full gate and all commits.

Retained outside the managed source tree: this report and six named logs in
<cache>/ekr-completion-20260922/guard-debt/legacy-integration-correction.
Compiler artifacts are under <cache>/b10x-target/ekr-completion-20260922.
No scratch Rust project, captured provider store, fixture copy or temporary production mutation was created.

