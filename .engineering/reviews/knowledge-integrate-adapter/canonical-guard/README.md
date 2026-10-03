# Incremental A/E canonical-write observation correction

Source frozen. This corrects the adapter observation gap identified in independent-review.md; it is not evidence that the kernel previously performed an unexpected write. The entire integrate gate remains held for the existing Submit witness and F application limitations.

Only crates/ekr/src/conformance/knowledge.rs changes in this correction. Apply incremental.patch on top of the original source.patch (SHA256 86affa74ac99c9d7cd0348dce58e18fee1c95316949cdcd19461b6602527f205), not directly on the original Git base. The exact prior knowledge.rs is retained in baseline/knowledge.rs. Incremental patch SHA256: 78b4c8a1549755d8e09d1cc6fb4d92e936d8552d177bcf7c0a4bc287a843d0e7. No specification, AEP, generated contract, fixture, dependency, kernel or CLI changes. No commit or publication.

CanonicalObservation captures Runtime.head() and the complete actual Runtime.published_events() records filtered by stream_type == ekr.revision and stream_id == canonical. Before dispatch of each A/E native command the adapter captures that observation. After every returned success or refusal it drops Runtime, reopens with full replay enabled, captures again and requires exact equality of both head and occurrences before reporting the outcome. An unexpected canonical publication therefore fails observation even if graph head is unchanged. Allowed observation/incubation/proposal/review metadata remains outside this comparison. Existing retained-review id/decision checks still run using the reopened Runtime.

The new unit test canonical_guard_rejects_real_proposal_with_unchanged_head_on_both_providers invokes the same capture/comparison helper used by dispatch. It establishes a real native fixture, verifies unchanged observations pass, publishes an ordinary transaction through Runtime.propose(), closes/reopens with full replay, and independently asserts unchanged head and one extra canonical occurrence. It then requires the guard to reject. Both providers execute before the final aggregate assertion.

Red evidence is explicit about the intermediate implementation: the test first ran against an intermediate head-only guard, not a claim that the original source had that guard. red-behavior.log/status captures both File and Sqlite escaping that guard; exit 101, 0 passed / 1 failed / 0 ignored / 66 filtered. This demonstrates why head equality is insufficient using a real publication, not an invented counter. The earlier red.log/status is a separate compile error for the PublishedEvent export namespace; it is not behavioral red evidence. After adding exact occurrence equality, green.log/status records exit 0, 1 passed / 0 failed / 0 ignored / 66 filtered, exercising both providers.

Commands under the coordinator's sole compiler lane environment:

- cargo test -p ekr --lib canonical_guard_rejects_real_proposal -- --nocapture
- cargo test -p ekr --test conformance_integrate -- --nocapture (EKR_INTEGRATE_REPORT_DIR set to this unit's reports directory)
- cargo clippy -p ekr --lib --test conformance_integrate -- -D warnings
- rustfmt --edition 2021 --config skip_children=true --check crates/ekr/src/conformance/knowledge.rs
- git apply --reverse --check incremental.patch (verifies the incremental patch against current source without mutating it)

Final focused Rust target remains 5 passed / 3 failed / 0 ignored / 0 filtered, exit 101. Both provider report/2 documents remain exactly 19 total, 16 passed, 1 failed, 0 error, 2 unsupported and 0 skipped: 17 answered. SubmitSchemaProposal/outcome/answered is the sole failure. ApplySchemaProposal/outcome/answered and /refused remain unsupported. Floor 19, zero unavailable and zero quarantine are untouched. These are bounded observations, not full conformance or CI stability claims. The existing event-suppression and signature-corruption tests still pass, as do retained review reopen/full-replay checks. Permitted metadata writes therefore coexist with the new canonical guard in the exercised success paths.

Clippy exit 0; exact-file formatting exit 0; reverse patch applicability check exit 0. No broader package or task check was run. Terminal worker sessions: 33489 (compile red 101), 3612 (behavior red 101), 4780 (focused guard green 0), 5655 (existing conformance target 101), 42225 (clippy 0). No live Cargo remains; compiler lane was explicitly returned to root.

Eight report/2 JSON documents in sanitized-reports/ are byte-identical to reports/ after inspection found no local home/tmp/user/worktree paths. Preserve the exact report bytes. Their completed_at=1700000003900 is the fixed synthetic runner clock, NOT observed execution UTC or fresh AEP evidence. Original evidence is preserved unchanged. Raw logs may contain local paths and are private evidence, not sanitized publication artifacts. Relative-path SHA256SUMS records this correction's patch/report/reports and raw log hashes.

Managed tree ekr-knowledge-conformance-20261003 and branch ekr/knowledge-conformance-20261003 are preserved. Own correction lease codex-ekr-integrate-conformance-write-guard-20261003 is released at handoff. Root owns applying/reviewing the incremental patch and subsequent integration. No further edits or builds are planned by this worker.
