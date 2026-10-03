# Human decision identity correction — bounded implementor report

Candidate is uncommitted in managed tree `<review-retention-worktree>`, branch `ekr/schema-review-retention-20261003`, base `89fc4337fce53b51072c62d066b3207093c32490`. Parent owns AEP story `story:propose-better-vocabulary`, review, bot commit and integration. No kernel, CLI, SDK, ESS, generated contracts or AEP changes were made by this correction.

## Result and class

UpgradeAuthority, AnswerAttention and schema proposal reviews now share one atomic decision-identity singleton, carrying the existing generated `EkrKernelHumanDecisionRecord`. New canonical signed writes and new retained reviews append that singleton in the same native atomic group as their occurrence and bytes. The CAS is `Expected::NoStream`; a precheck alone is not the uniqueness mechanism. Existing review-id singleton uniqueness remains separate and atomic.

Class: every new physical human-decision publication must participate in the same identity namespace. Enumerated members are direct canonical publication, elected canonical preparation/resume and proposal review retention. Historical canonical decisions and legacy review decision singletons reserve their identities without changing original bytes or retroactively requiring an index for old replay.

Public API: `ekr_store::HumanDecisionRetention::human_decision(&self, decision_id: &str) -> Result<Option<ekr_core::contract_data::EkrKernelHumanDecisionRecord>, StoreError>`. It is a physical lookup. Kernel owns audience, protocol digest/signature, target and operator authentication. A protocol proof digest is never treated as a plain object hash. No method grants approval or adds a canonical writer.

## Exact persistence boundary for parent specification/design update

- Shared stream: tenant-local `ekr.kernel.human-decisions` / canonical decision UUID. One `ekr.kernel.HumanDecisionBound`, native schema version 1, generated HumanDecisionRecord data.
- New canonical signed envelope: `RevisionEvent::SIGNED_FORMAT = ekr.revision-event/5`, native schema version 5, only AuthorityUpgraded and AttentionAnswered payloads. Other payloads with /5 refuse. Existing `RevisionPayload::format()` remains /3 for upgrade and /4 for answer; parent must explicitly select SIGNED_FORMAT in new kernel writes. Legacy serializers and replay rules remain available.
- New signed preparation: `ekr.publication-preparation/6`, with one exact shared index append in its fingerprinted immutable native request. Authorization rejects missing, duplicate, wrong stream, wrong event/version/data and wrong expectation. Existing /4 and /5 preparation bytes remain readable and re-encode unchanged.
- New proposal-review occurrence: existing `ekr.integrate.ProposalReviewRetained` native schema version 2. It requires shared decision binding plus the existing review-id singleton. Version 1 reads/retries preserve their original review-id and decision-id singleton requirements. An exact legacy retry returns its original receipt and does not rewrite its version or add an index.
- Full history/replay checks each /5 canonical occurrence against an exact shared record; missing/changed bindings refuse. Preparation basis authorization checks already-published /5 history too.
- Current direct old-format signed writes and new old-format signed elections refuse `human-decision-revalidation-required` before mutation. Resuming an old signed preparation first recovers an exact already-published occurrence as AlreadyRecorded; an unpublished old signed preparation refuses under that same named code.
- No empty canonical append, marker event or new semantic DTO was introduced.

## Migration and remaining boundaries

Mixed old/new writer binaries are unsupported for this transition. Legacy reservation scanning is safe only because current runtime paths cannot add unindexed old-format signed occurrences. Unsigned advancement cannot introduce a decision identity; new signed advancement participates in the shared CAS. This limitation is explicit in module documentation.

An unpublished legacy prepared upgrade remains an immutable occupied command slot. This slice refuses it; it does not implement automatic same-head re-election or a new migration command. The parent must record that narrower recovery boundary and decide its compatibility implications. It must not claim automatic revalidation of every old pending upgrade.

The initial candidate left `checkpointed_head` on its pre-existing fast root read. Independent review required the physical binding invariant there too. The follow-up correction now declines that shortcut for every history containing a /5 signed occurrence, so the verified path checks every required signed binding; ordinary and legacy histories retain their existing shortcut eligibility. See `checkpoint-report.md` for measured red/green evidence. The repro uses a synthetic store authority; current kernel admission necessarily puts an upgrade before an answer, and the kernel already declines checkpoint head shortcuts for histories containing upgrades. No real-kernel exploit was established.

Historical conflicting records for the same decision identity refuse lookup; there is no automatic repair. Physical tests use a permissive injected authority and synthetic generated records; they establish store mechanics, not cryptographic admission or ESS conformance.

## Evidence and verification

All evidence is under `<retained-evidence>/human-decision-identity/`. Original red logs are preserved.

Commands used the exclusively assigned target `<owned-build>/target`, sibling TMPDIR, `env -u RUSTC_WRAPPER`, `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, dev/test debug=0 and strip=symbols, `CARGO_NET_OFFLINE=true`. No heavy second configuration or lane was created. Persistent/tmp free space before regression was 12 GiB / 8.5 GiB.

- Baseline: `cargo test -p ekr-store -p ekr-graph`; `base.log`, `base.status=0`: 291 passed, 0 failed, 3 ignored.
- Store red: `cargo test -p ekr-store --test human_decision_identity`; `red-store.log`, `red-store.status=101`: 0 passed, 4 failed, 0 ignored. Observed missing legacy reservation, missing legacy write refusal, absent new replay-binding diagnostic, and a cross-kind race whose canonical arm refused the then-unsupported new envelope (not two accepted writes).
- Graph red: `cargo test -p ekr-graph --test revision_events shared_human_identity`; `red-graph.log`, `red-graph.status=101`: 0 passed, 1 failed, 0 ignored, 10 filtered. New signed envelope was unsupported.
- Initial implementation compile: `green-focused.log`, status 101: read-only helper incorrectly required AtomicBlobEventStore; fixed by separating EventStore read helpers from atomic write helpers. This is compile evidence, not behavior evidence.
- Focused store: `cargo test -p ekr-store --test human_decision_identity --test proposal_review_retention --lib`; `green-focused-2.log`, status 0: 30 passed, 0 failed, 1 ignored. Two test-only unused-result warnings then corrected to explicit Written assertions.
- Package regression: same baseline command, `green-package.log`, status 0: 291 → 299 passed, 0 failed, 3 ignored in both. Counts sum runner summary lines, including doc tests. The new graph test ran in this full command. All four cross-domain cases exercise both file and SQLite and both upgrade/answer kinds. Recovery tests exercise both providers and kinds; legacy review compatibility exercises both providers.
- Additional recovery and legacy review fixtures were added during implementation and passed, but were not individually observed red; no red-capability claim is made for those additional cases.
- `cargo clippy -p ekr-store -p ekr-graph --all-targets -- -D warnings`; `clippy.log`, status 0.
- Exact ten touched files: `rustfmt --edition 2021 --config skip_children=true --check ...`; `format.log`, status 0. Never ran cargo fmt --all. Final formatting after package test was formatting only, removing accidental edition-2024 formatting; clippy ran on final formatted source.
- `git diff --check`; `diff-check.log`, status 0.

No full workspace `task check`, kernel integration verification or conformance claim. Those remain parent-owned.

## Changed files

- `crates/ekr-graph/src/events.rs`
- `crates/ekr-graph/tests/revision_events.rs`
- `crates/ekr-store/src/eventlog.rs`
- `crates/ekr-store/src/lib.rs`
- `crates/ekr-store/src/human_decisions.rs` (new)
- `crates/ekr-store/src/preparation.rs`
- `crates/ekr-store/src/preparation_human_identity_tests.rs` (new)
- `crates/ekr-store/src/proposal_reviews.rs`
- `crates/ekr-store/tests/human_decision_identity.rs` (new)
- `crates/ekr-store/tests/proposal_review_retention.rs`

## Handoff

Sessions 79263 (baseline), 56424 (store red), 89441 (graph red), 12393 (compile correction), 55750 (focused green), 36204 (package green), 65468 (clippy) are terminal. Root temporarily borrowed the lane between red and green runs, then explicitly returned it. Lane returned to root again after terminal package/clippy; no live Cargo process belongs to this task. Tree preserved; no commits, pushes, publication or cleanup. Parent owns next action: independent review, exact migration disposition, bot commit and kernel integration.
