# Checkpoint shortcut correction

Follow-up to independent review of the human-decision identity slice. Same preserved managed tree and branch; no commit, AEP, generated, ESS or kernel changes. Only `crates/ekr-store/src/eventlog.rs` and `crates/ekr-store/tests/human_decision_identity.rs` changed for this follow-up.

The store's checkpoint-only `head()` shortcut previously bypassed the new physical signed-identity binding check. A synthetic authority returning a checkpoint root reproduced a /5 AttentionAnswered followed by an ordinary commit, a covering checkpoint pointer and a missing signed identity singleton: cold `history()` refused, but `head()` returned the root. Both file and SQLite reproduced this. This proves the store-port gap only. Current kernel admission requires a preceding AuthorityUpgraded event before an answer, and KernelAuthority already declines the shortcut whenever an upgrade occurs; no reachable real-kernel exploit was established.

Fix: `eventlog.rs:677` declines the shortcut if any occurrence's `requires_human_binding()` is true, sending the caller through the verified read path. This covers both new signed payload kinds and later ordinary commits, rather than checking only the current head. Ordinary and legacy histories retain the old eligibility and shortcut performance. No persistent bytes/formats changed in this follow-up.

Tests added before the fix:

- `checkpoint_head_after_signed_answer_checks_its_mandatory_identity_binding` (human_decision_identity.rs:441) checks cold history and head agree on named binding refusal, on both providers. It accumulates failures so red evidence covers both providers.
- `ordinary_checkpoint_head_keeps_the_existing_shortcut` (:462) uses an authority whose full replay refuses but checkpoint root lookup succeeds; both providers still return the root through the ordinary shortcut. This control passed both before and after.

Evidence lives beside this report. The build used exactly the previously assigned sole tmpfs target and bounded/offline settings. All ten changed files were touched after root returned the lane, ensuring the candidate's store and graph sources rebuilt instead of reusing another worktree's artifacts.

- Red: `cargo test -p ekr-store --test human_decision_identity checkpoint_head`; `checkpoint-red.log`, status101. Runner: 1 passed, 1 failed, 0 ignored, 4 filtered. Failure prints successful checkpoint head results on both providers while history already refused.
- Green: same command; `checkpoint-green.log`, status0. Runner: 2 passed, 0 failed, 0 ignored, 4 filtered.
- Regression: `cargo test -p ekr-store -p ekr-graph`; `checkpoint-package.log`, status0. Previous candidate 299 passed → final301 passed, 0 failed, 3 ignored unchanged. Original base was291 passed, 3ignored. Counts sum runner summaries, including doc tests.
- Lint: `cargo clippy -p ekr-store -p ekr-graph --all-targets -- -D warnings`; `checkpoint-clippy.log`, status0.
- Exact-file rustfmt check (edition2021, skip_children=true) and `git diff --check`: `checkpoint-format.status=0`, `checkpoint-diff-check.status=0`.

Initial report correction: its original cross-domain race red was `[Err(Document("invalid-publication-envelope")), Ok(true)]`, not two accepted duplicate decisions. The initial `report.md` now says this explicitly. Raw evidence was not changed. This checkpoint report supersedes the earlier report's unresolved checkpoint caveat; its separate unpublished-legacy-preparation migration limitation remains parent-owned.

Sessions49124(red),81279(focused green),8428(package),58251(clippy) all terminal. Compiler lane returned explicitly to root after clippy. Own lease released on handoff; tree remains uncommitted and preserved. No full task check or ESS conformance claimed. Next owner root: source review, kernel integration, migration disposition and bot commit.
