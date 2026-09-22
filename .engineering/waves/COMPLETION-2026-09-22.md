# Completion through P7

The operator approved the completion plan and then instructed implementation. This supersedes
the earlier handoff's assumption that each new wave needs a separate work request. Source
publication and the verified release are included; no extra scope or unapproved outward
messages are implied.

## Contract

The architecture, its append-only amendments, roadmap exits and predecessor capability list
remain normative. Preserve existing stores. Refuse migration when required history cannot
be verified; do not replace it with an invented snapshot history. Completion includes the
verified 0.1.0 release, live v2 cutover and seven days of observed unattended operation.

## Sequence

1. Preserve the handoff, reviews and cached cases; reconcile stale planning claims.
2. Repair the transaction membrane and add repository correctness CI; independent review.
3. Validate seed admission in the kernel and remove access to the raw store writer.
4. Version persisted contracts, preserving acceptance evidence and event identity.
5. Apply and persist operations, ontology and durable validation receipts; reconstruct through
   both real backends after restart. Cover concurrency, interruption and historical queries.
6. Close guard/projection debt; deliver six CLI verbs and a real Rust ESS conformance target;
   independent review and P1 exit evidence.
7. P2 observations and adapters, then P3 interpretation and integration, followed by P4 operator
   tools, P5 schema/scheduling/budgets and P6 maintenance/metrics under roadmap dependencies.
8. P7 migration rehearsal, count/provenance reconciliation, verified release and service cutover,
   followed by the actual seven-day observation window.

Each phase stays open until its roadmap exit has executable evidence. A missing live provider
or credential is recorded against the dependent operation while independent implementation
continues. Never count unsupported scenarios as passed.

## Recovery

Integration tree id: `ekr-completion-20260922`.
Integration branch: `codex/ekr-completion-20260922`.
Tree: `<worktrees>/ekr-completion-20260922`.
Build directory: `<cache>/b10x-target/ekr-completion-20260922`.
Scratch root: `<cache>/ekr-completion-20260922`.
Coordinator lease: `codex-ekr-completion-20260922`.
Planning actor: `agent:codex-ekr-completion-20260922`.

The original handoff is now committed on main. Its formerly untracked primary copy was
byte-compared before and after integration and retained in the completion scratch root.
All planning mutations use AEP. The gates-policy baseline and required signer enrollment
were integrated independently; unrelated policy changes and backup files remain preserved.

## Current implementation boundary

The membrane repair, required repository correctness CI and kernel seed admission are merged.
The seed source is on main through PR #8; its implementation and review evidence belong to
the P1-08 wave page. Original-format verification reached main through PR #9 with 431
passing cases. Runtime-context refusal is independently reviewed and published in PR #10.
The source-guard review reproduced two escapes and a type-use false-positive class.
All original cases now pass unchanged; independent correction re-review found no
further defect. The combined gate passed and PR #10 is merged at 9a15b96 with its
required correctness and common checks green. P1-10 records the executed gate and
bounded task closures; both completed unit trees and their build output are removed.

Gates' historical-merge delivery repair is released and adopted, and actual EKR publication
passed. Eventlog's strict history inspector and atomic blob/event capability are published
at 4ee3dc2 through PR #11, with required production, comparative and restart CI green.
EKR's three dependency selectors and lock now select that published source; eleven
dependency-contract and 195 kernel/store cases passed before the combined integration gate.

The activation review found four contract mismatches. Explicit object-event schema dispatch
and absent-revision refusal are corrected in the retained draft. Exact Seed/Commit retry
requires a measured ESS extension. Its source correction and independent re-review are
integrated through [ESS PR #59](https://github.com/beyond10x/ess/pull/59), merged at
8bef63a21766c54f0d809b4decf1d9f7bd587118 after the required CI checks passed.
The operator approved the existing CI/release profile for this bounded 0.29.0
release, retaining the separate local consumer-accounting refusal. Exact-tag
qualification passed, and [ESS 0.29.0](https://github.com/beyond10x/ess/releases/tag/0.29.0)
is verified: required release checks passed, downloaded archives match their
checksums, the host binary runs and the release-status audit passes. The delivery branch preserves
the reviewed tree and governed journal; the
original development branches and a verified Git bundle retain the intermediate history.
The source review is recorded there as `review-result:retained-replay-source-r2` and in
`.engineering/reviews/ekr-retained-replay-source-r2.md`. Its bounded runtime findings are
addressed; it does not establish EKR durability. Generated scenario counts are not executed
EKR conformance.
The subsequent ess/7 draft is retained in
`.engineering/waves/p1-writer-activation-ess7-draft.patch`, with a static review and
development preflight under `.engineering/reviews/`. It declares own-result retries,
complete original-revision observations and the frozen transaction parser profile.
The first development synthesis refused the Stale state's Commit and Validate
cases through the external Commit/stale arrangement. The upstream correction now
generates both cases; repeated development validation, compilation and synthesis
pass with no refused obligations. The exact failed and corrected outputs remain
retained, and the same adopter shape is covered by upstream regression fixtures.
The preflight with the downloaded release passed; its commands and outputs are recorded in
`.engineering/reviews/p1-writer-released-compiler-preflight.md`.
The qualified declarations are now activated in the coordinator and durable
source unit together. Validation, compilation and synthesis passed against the
activated specification; no EKR runtime success is inferred from that evidence.
The bounded transaction-document parser is merged through
[EKR PR #11](https://github.com/beyond10x/epistemic-knowledge-runtime/pull/11).
Its independent review reproduced admission of a combined string total above
the frozen profile. The mechanism addendum retracts the original attribution to
separate traversals: the dependency's public deserializer hides global tag text.
The original failing case is preserved under
review-result:p1-transaction-parser-adversary-r1. The bounded observation facade
and counting correction passed the second independent review, retained verbatim
in review-result:p1-transaction-parser-adversary-r2. Its source unit is 93ec8a4
and integration commit 0444fa9. The original regression remains unchanged.
The full project gate and required CI passed; integration corrections and the
measured results are retained in
`.engineering/reviews/p1-parser-integration-corrections.md` and
`.engineering/reviews/p1-parser-integration-gate.md`. The parser task stays active
until the writer's final operation-shape integration also passes. Its source
worktree was finished and removed through the managed lifecycle after publication.

The newly published Eventlog 0.3.0 passed current consumer and cross-version seed
checks, but a fresh SQLite atomic group can reuse corrupt blob metadata without
refusing. The exact probe is retained in
.engineering/reviews/eventlog-030-integrity-probe.md. The repair passed independent
review and required production-backend CI and is merged through
[Eventlog PR #15](https://github.com/beyond10x/eventlog/pull/15). The coordinator
and durable source unit select that reviewed source. Full EKR qualification and
cross-version seed reopening passed, recorded in
`.engineering/reviews/eventlog-repair-adoption.md`; the bounded adoption is merged through
[EKR PR #12](https://github.com/beyond10x/epistemic-knowledge-runtime/pull/12).
Its required repository correctness and common checks passed. The upstream
completion record is merged through
[Eventlog PR #17](https://github.com/beyond10x/eventlog/pull/17), and that source
worktree is finished and removed after publication. Both primary checkouts are
advanced only by clean fast-forward to the merged source.
No new Eventlog release tag or ordinary-transaction durability is inferred.

The coupled persisted-contract and durable-kernel implementation is active in
managed tree `ekr-durable-activation-20260922`, following
`p1-durable-activation-brief.md` and `p1-durable-activation-20260922.md`.
The first checkpoint covers the new formats, kernel seed and atomic retained
blobs; ordinary application and full replay acceptance follow in the same unit.
Its exclusive compiler output uses temporary-memory storage; source and evidence
remain persistent. The resource page records the measured reservation and floors.
Durable application and the CLI remain unfinished.
The conformance and migration preparation reports are measured prerequisites, not executed
exits. No later phase or live cutover has been claimed complete.
