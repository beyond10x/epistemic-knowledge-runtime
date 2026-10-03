# Schema proposal correction review

Verdict: needs-revision

Owners: Root owns correction and verification of the coordinator-authored E material/admission path. Reviewer author_contracts owns the erroneous initial claim that an ordinary attachment to a disputed claim was reachable; that claim is explicitly withdrawn below.

Scope: schema_proposal_mapping.rs, schema_proposals.rs, schema_proposal_material.rs, schema_proposal_review.rs, disputes.rs, attention projection, attachment admission, and human_review/corrections.rs.

Evidence: This reviewer performed read-only source inspection and read the coordinator's two execution logs. The reviewer ran no build, test or provider operation and changed no repository files. The coordinator's applicability reproduction is distinguished from the blocked attachment probe and the unexecuted replacement hypothesis.

## Findings

### 1. Selected corrections omit complete dispute-component material

- file: crates/ekr-kernel/src/schema_proposals.rs
- category: review-material-integrity
- severity: high
- verdict: needs-revision
- origin: implementation
- message: Correction-specific effects include the named assertion row, but do not include the complete connected dispute component's competing claim records, per-claim support links or evidence material. This is a source-observed omission. The originally suggested ordinary AttachEvidence route does NOT reproduce it: attachment admission requires held.is_current(), and disputed assertions are not accepted/current. correction-support-red-3.log fails with assertion-not-active before the old-review assertion is reached. That failure is a blocked probe, not evidence that stale review was accepted. The earlier description of that attachment route as reachable is withdrawn.

Replacement hypothesis, source-supported but NOT executed: Start with overlapping disputed claims A=value1 and B=value2 for the same One subject/predicate. Submit and sign a proposal with mappings=[] and a sole Choose(A) correction. Then admit a fresh ordinary assertion C with A's same subject, predicate, value and valid interval, valid existing evidence and Proposed/Active input state. disputes::preview_contradictions excludes the equal-valued A-C pair and includes B-C. recompute therefore preserves A's competitor list [B] and its full assertion row, while B's list grows to [A,C]. Attention's connected component grows from {A,B} to {A,B,C}. Proposal correction effects capture only A; explicit proposal evidence, schema and source material need not change. The current proposal digests are therefore expected to remain unchanged, allowing the old approval despite changed dispute options. This remains a hypothesis until an admissible AddAssertion and stale-approval result are observed.

Minimal meaningful probe: Assert C's ordinary commit succeeds; assert A's full row remains equal; assert the attention component grows to three claims; assert current proposal material changes and the old signed approval refuses. Use no mappings so mapping-induced claim collection cannot mask the missing correction material. Reuse retained evidence to isolate component membership from explicit proposal citations.

Required correction if this route is confirmed: Project complete selected dispute components from the same VerifiedRead using dispute_attention/disputes_at. Fold their evidence/options/effects digests, keyed and ordered by dispute identity, into the corresponding proposal digests. Exclude observation revision and presentation text so unrelated revisions and renames do not force another human answer. Preserve historical verification at a once-valid decision's observed revision. Do not widen AttachEvidence admission as part of fixing this finding.

### 2. Proposal admission does not validate correction applicability

- file: crates/ekr-kernel/src/schema_proposal_mapping.rs
- category: admission-validation
- severity: high
- verdict: needs-revision
- origin: implementation
- message: The correction path checks serialization, duplicate claim IDs and assertion existence but does not reuse existing applicability checks. The coordinator's correction-applicability-red-2.log reproduces acceptance of Choose with a whitespace-only reason: proposal_submission_refuses_inapplicable_corrections fails at the first case with accepted [{kind: Choose, reason: " ", ...}]. Subsequent table cases and the second provider were not established by that failing run. Other source-indicated omissions include non-disputed/nonactive claims, conflicting choices, inappropriate bounds, inverted/no-op temporal corrections and combined uncertainty; they remain unexecuted here.

Required correction: Extract allocation-independent applicability validation from human_review/corrections.rs::derive and interval, preserving attention-answer semantics. Apply it at submission and approval and when verifying historical approvals at their observed revision. Keep replacement identity allocation/freshness in operation derivation. Do not mint preview replacement identities or fabricate AnswerAttention authorization. Stale retained proposals should remain inspectable, and rejection should remain possible without endorsing inapplicable effects.

## Execution status

- proposal_correction_review_binds_support_attached_to_either_competing_claim: coordinator executed; blocked at mutation admission (assertion-not-active); did not test stale approval. This test design must be replaced, not counted as a defect reproduction.
- proposal_submission_refuses_inapplicable_corrections: coordinator executed; RED at the blank-reason case, demonstrating that specific defect. Other cases/provider coverage not yet proven.
- Equal-valued third-claim component expansion: source-only hypothesis; no execution performed or claimed.

This review remains needs-revision based on the reproduced blank-reason defect and the separately documented material omission pending a valid reproduction. No green verification or implementation-conformance result is claimed.
