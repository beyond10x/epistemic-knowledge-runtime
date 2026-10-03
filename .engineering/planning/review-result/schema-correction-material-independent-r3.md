---
format: aep.planning-md/3
id: review-result:schema-correction-material-independent-r3
kind: review-result
status: active
title: Correction material and applicability fixes reviewed
relations:
- reviews: story:propose-better-vocabulary
revision: 1
---
approve

Scope: bounded correction-material and applicability re-review only.

Owners: Root owns the reviewed implementation and executed regression evidence. Reviewer author_contracts owns this independent read-only assessment and the explicit withdrawal of the earlier attachment-route claim. No unresolved finding remains within the two correction defects reviewed here.

## Scope and method

Reviewed current source in crates/ekr-kernel/src/schema_proposal_corrections.rs,
schema_proposals.rs, schema_proposal_review.rs, human_review/corrections.rs and attention.rs,
and the corresponding cases in crates/ekr-kernel/tests/schema_proposal_reviews.rs.
The reviewer read coordinator-produced logs but ran no Cargo command, build, test, provider
operation or repository mutation. Only this report was written outside the repository.
This verdict does not approve full E, F, publication, or overall conformance.

## Findings

### CORRECTION-1 — complete affected-component review material

- file: crates/ekr-kernel/src/schema_proposals.rs
- category: review-material-integrity
- severity: high
- verdict: resolved
- origin: implementation
- message: schema_proposal_corrections::components selects each complete current dispute component touched by any selected correction from the same VerifiedRead. The proposal separately incorporates those components' evidence_digest, options_digest and effects_digest into its matching material digests. attention::disputes_at includes per-claim support links and attachments, actual evidence records and retained payloads, complete claim rows, and component identities plus the applicable declaration. Its deterministic component ordering and identity-bearing digest contents prevent omitted competitor material. Neither observed_revision nor question/presentation text enters these nested material digests. The previously omitted equal-valued third-claim case is fixed.

Execution evidence read: component-red.log records a successful ordinary third-claim admission,
unchanged selected assertion row, expanded three-claim component, then failure at
`accepted changed competing component`. corrections-green-2.log reports the same behavioral
regression passing. The test's file/SQLite loop and assertions were inspected: it verifies successful
mutation, unchanged selected row, three options, refusal of the old signed approval, unchanged
canonical event history during refusal, and changed proposal options_digest.

The original AttachEvidence probe remains withdrawn. correction-support-red-3.log stops at
assertion-not-active before exercising stale approval. It supplies no evidence of a stale-review
bypass and is not counted as the successful reproduction.

### CORRECTION-2 — shared applicability validation

- file: crates/ekr-kernel/src/human_review/corrections.rs
- category: admission-validation
- severity: high
- verdict: resolved
- origin: implementation
- message: The extracted instructions/validate_corrections path shares the original answer applicability rules without allocating replacements or inventing answer authorization. It checks active/open/disputed membership, distinct instructions, nonblank explanations, legal use of temporal bounds, uncertainty exclusivity, compatible choices, valid temporal intervals and changed temporal effect. Proposal validation partitions selected instructions by complete dispute component and refuses unassigned or duplicate selected identities. Strict proposal submission calls it. New approvals validate against both current and signed historical reads; historical retained approvals are validated at their original observed revision. Rejections and non-strict inspection do not incorrectly require current applicability. Actual operation derivation still requires fresh retained replacement allocations.

Execution evidence read: correction-applicability-red-2.log fails because the initial whitespace-only
Choose reason was accepted. corrections-green-2.log reports proposal_submission_refuses_inapplicable_corrections
passing. The current test covers six cases in both provider branches: blank reason, non-temporal
bounds, unchanged interval, inverted interval, competing Choose instructions and mixed uncertainty.
It also asserts no canonical history change on refusal.

## Verification extent and limitations

corrections-green-2.log reports exactly 3 passed, 0 failed, 3 filtered out:

- proposal_submission_refuses_inapplicable_corrections
- proposal_correction_review_binds_the_full_competing_component
- schema_proposal_requires_exact_human_approval

Both new correction cases explicitly loop over file and SQLite. This is targeted coordinator
execution evidence, not a reviewer-run suite. Source inspection verifies exclusion of unrelated
revision/presentation material and the current/historical applicability call sites; this log does
not separately demonstrate a correction-bearing approval surviving an unrelated commit, or a
previously valid correction approval reopening after its dispute settles. Those remain broader
regression coverage considerations, not claimed executions here. Existing attention-answer tests,
full E verification and the full repository gate are outside this bounded verdict.

```findings
[]
```
