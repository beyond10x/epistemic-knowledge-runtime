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
requires a measured ESS extension; its independently critiqued implementation contract is
now being implemented upstream. Generated scenario counts are not executed conformance.
The subsequent ess/7 draft is retained in
`.engineering/waves/p1-writer-activation-ess7-draft.patch`, with a static review and
development preflight under `.engineering/reviews/`. It declares own-result retries,
complete original-revision observations and the frozen transaction parser profile.
Development validation and compilation pass, but synthesis still refuses the Stale
state's Commit and Validate cases through the external Commit/stale arrangement.
That concrete adopter shape is being added to the upstream regression fixtures.
This draft is unactivated and no released-compiler or runtime success is inferred.
Persisted-contract activation, durable application and the CLI remain unfinished.
The conformance and migration preparation reports are measured prerequisites, not executed
exits. No later phase or live cutover has been claimed complete.
