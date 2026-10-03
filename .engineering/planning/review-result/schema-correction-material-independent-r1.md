---
format: aep.planning-md/3
id: review-result:schema-correction-material-independent-r1
kind: review-result
status: active
title: Schema correction support and applicability review
relations:
- reviews: story:propose-better-vocabulary
revision: 1
---
# Schema proposal correction review

Verdict: needs-revision

Owners: Both findings are implementation defects in the coordinator-authored E schema-proposal material/admission path; root owns correction and verification. This read-only review was performed by author_contracts.

Scope: schema_proposal_mapping.rs, schema_proposals.rs, schema_proposal_material.rs, schema_proposal_review.rs, existing attention material projection, and human_review/corrections.rs.

Evidence: Read-only source inspection. No code changes, builds, test execution, provider operations or conformance execution were performed by this reviewer. The reachable cases below are source-derived counterexamples, not executed reproductions.

## Findings

### 1. Selected correction evidence and competing options are incompletely bound

- file: crates/ekr-kernel/src/schema_proposals.rs
- category: review-material-integrity
- severity: high
- verdict: needs-revision
- origin: implementation
- message: Proposal ReviewBasis includes explicitly cited proposal evidence and mapping effects, but selected corrections add only their named Assertion rows. For a proposal with no mappings and Choose(A), AttachEvidence to A or its competitor B need not change any of the three proposal material digests. Full competitor records, per-claim support links, attached evidence and their exact retained payloads are missing from correction-specific material. An old human proof can therefore remain admissible after the evidence the correction should review changes.

Required correction: Project complete selected dispute components from the same VerifiedRead using dispute_attention/disputes_at. Fold their evidence/options/effects digests, keyed and ordered by dispute identity, into the corresponding proposal digests. Exclude observation revision and presentation text from nested material so unrelated revisions and renames do not force another human answer. Preserve historical verification of a once-valid decision at its signed observed revision.

### 2. Proposal admission does not validate correction applicability

- file: crates/ekr-kernel/src/schema_proposal_mapping.rs
- category: admission-validation
- severity: high
- verdict: needs-revision
- origin: implementation
- message: The correction path checks canonical serialization, duplicate claim IDs and assertion existence, but does not reuse existing correction applicability checks. It can admit blank explanations, settled/retracted/non-disputed claims, incompatible Choose instructions, bounds on non-temporal instructions, inverted or unchanged CorrectTime intervals, and uncertainty combined with claim changes. Re-signing current proposal material does not make these effects applicable. Non-strict preview fallback must not become an approval bypass.

Required correction: Extract the allocation-independent applicability validation from human_review/corrections.rs::derive and interval, preserving attention-answer semantics. Apply it at proposal submission and approval, and when independently verifying historical approvals at their observed revision. Keep replacement identity allocation/freshness checks in actual operation derivation. Do not mint temporary replacement identities or fabricate AnswerAttention authorization for E. Stale retained proposals should remain inspectable, and rejection should remain possible without endorsing inapplicable effects.

## Verification status

Root reports authoring these tests for both file and SQLite providers:

- proposal_correction_review_binds_support_attached_to_either_competing_claim
- proposal_submission_refuses_inapplicable_corrections

At this review recording, neither test has run; execution is pending Cargo-lane handback. Their existence is not passing evidence, and this review remains needs-revision until the fixes and relevant regression results are independently assessed.
