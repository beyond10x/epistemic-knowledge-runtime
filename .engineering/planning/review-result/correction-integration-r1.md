---
format: aep.planning-md/3
id: review-result:correction-integration-r1
kind: review-result
status: active
title: Schema correction recovery and retained review integrity review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
needs-revision

Owners: 2 coordinator/kernel implementor findings; 0 delegated implementor findings. Finding 1 is independently source-derived; finding 2 was discovered and executed by the coordinator, with its source and failure log inspected by this reviewer.

Bounded source review of uncommitted correction integration over `c16643b7fca7003e7e4f1724d5425c7b96ceffc3`: correction construction, signed review capabilities, guarded publication and replay, final-step ordering, private withdrawal admission and captured Explain provenance. This is not full F or full-gate acceptance.

A renewed approval cannot resume a previously elected correction step even when the residual correction effects have not changed. `application_correction.rs:34-39` requires the supplied approval to predate the frozen step, but `application_auth.rs:1071-1107` supplies the latest effective approval at every publication. `schema_application.rs:439-443` correctly reuses the existing step, so a later approval deterministically encounters "correction step has no prior exact approval". Removing only that comparison is insufficient: lines 49 and 83-106 regenerate the transaction from the latest approval's freshly allocated statement evidence, and line 212 compares those new bytes with the immutable old transaction.

Concrete reproduction: apply a Choose proposal through its schema commit, retain the genuine correction step/attempt, and interrupt before its guarded Propose. Record a newer trusted approval for the same immutable proposal, unchanged current residual basis and same operator, then resume with that approval. No intended withdrawal or allocation changed, but the retained step cannot publish and the application remains partial. The same obstacle applies after an already guarded proposal when a newer review becomes effective. This is a source-derived recovery defect, not an independently executed reproduction or an unauthorized-write finding. It conflicts with the same-election renewed-approval behavior described in design 105.17 and the ApplySchemaProposal contract.

Keep the correction's authenticated originating approval/statement and frozen operations as immutable provenance, while independently requiring the latest effective approval to authorize the current residual effects. Do not rebuild the frozen statement or weaken current approval checks. Resolve provenance from authenticated retained decision/publication links and exact frozen bytes, not an invented timestamp-derived canonical revision. Add an unchanged-effects renewed-approval recovery control and a changed-effects control that still refuses without amending the step or allocating replacements again. Merely removing the time check would not close the finding.

A second, distinct retained-input integrity defect was reproduced by the coordinator. `application_auth.rs:494-519` verifies each embedded review through `schema_proposal_review::verify_captured_review`, but does not bind that review's embedded proof/policy/statement bytes to their independently retained objects in the supplied history. The verifier parses the embedded proof and statement (`schema_proposal_review.rs:93-98`), so removing the proof object can leave verification successful. The coordinator's temporal/multi-dispute test clones an actual retained history, removes the original review proof object, and expects `authority.verify` to refuse; `<retained-evidence>/schema-application-kernel/correction-integration-3.log` shows that assertion failing, with two other cases passing. This is an executed immutable-capture integrity defect, not demonstrated proof forgery or a public Runtime path for deleting provider objects. The run stops in the first provider branch. Bind every review's embedded proof, policy and statement to its declared captured-history object address and exact bytes before producing the signature capability; retain missing/altered-object negative controls on warm and cold authorities. No base reproduction was executed, so this finding's origin is undecided rather than asserted pre-existing.

No additional concrete authority bypass was established in the inspected paths. Corrections follow the schema and all selected mapping commits; one final correction step is enforced and Unresolved remains pending. Withdrawal capability is private and derived after exact signed proposal/transaction checks, with ordinary guarded publication still checking the effective review and residual basis. Captured correction explanations come only from verified committed prefixes and check the actual attempt's record/event addresses, frozen operation bytes, retained proposal/proof/policy/statement payloads and canonical statement evidence; those later explanation checks do not replace replay's missing retained-object checks in finding 2.

Inspected implementor evidence: `<retained-evidence>/schema-application-kernel/correction-integration-1.log` has one pass and the known Unresolved V4-versus-V3 failure. `correction-integration-2.log` is terminal with two passed, zero failed. Source loops cover File and SQLite, Choose completion/retry/full reopen and Unresolved remaining pending; they do not reproduce renewed approval after correction-step election. These executions belong to the implementor; this reviewer ran no Cargo, builds, tests or mutations.

Inspected source fingerprints (SHA-256, captured twice with identical results during review):

- `crates/ekr-kernel/src/application_correction.rs`: `e01f07511dec65848ad8a8b3cd0f339ebf919e4d23c022b7b4fc30d163e58ef4`
- `crates/ekr-kernel/src/application_auth.rs`: `0e4adebfedf31c0d6d354c7f5aec0fe20a444d878849e5cb96c16300080a0481`
- `crates/ekr-kernel/src/schema_application.rs`: `f083948ca38f78b31cee47fd60bdb5e989a5fc72440191a0d6103ea745b75f72`
- `crates/ekr-kernel/src/validate/mod.rs`: `9c3a5f9cde3b51a4ff1fc46e82a85095c73edb119a5b137eed604256679260b2`
- `crates/ekr-kernel/src/explain/schema_corrections.rs`: `eae32dc162d866f75f7fb188e3c087c2ea17c951a4282b622a193cac60fcb281`

Limitations: source/log review only of the identified WIP. Broader correction variants, interrupted preparations, mutation coverage, detached provenance tests and integration regressions were not independently executed or accepted here. No repository, AEP or publication writes were made; only this external review report was saved.

```findings
[
  {
    "file": "crates/ekr-kernel/src/application_correction.rs",
    "line": 34,
    "category": "correctness",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "A frozen correction cannot resume after a newer approval of unchanged residual effects. Publication passes the latest approval to verify_semantic, but transaction rejects approvals recorded after step election and regenerates statement evidence from that latest review before comparing with frozen bytes. Retain a genuine Choose correction step, interrupt before Propose, approve the unchanged proposal/basis again, and resume: the existing election remains partial despite unchanged withdrawals and allocations. Authenticate frozen originating review/statement provenance separately from the latest approval of current residual effects, preserving exact operations and allocation IDs. Removing only the chronology check is insufficient. Source-derived recovery defect; no independent execution or unauthorized-write claim."
  },
  {
    "file": "crates/ekr-kernel/src/application_auth.rs",
    "line": 494,
    "category": "integrity",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "undecided",
    "message": "review_decisions verifies embedded review proof/policy/statement data without requiring the matching independently retained objects from captured history. The coordinator's correction-integration-3.log reproduces authority.verify accepting actual captured history after removal of the original review proof object; this reviewer inspected the source and failure but did not execute it. Bind each review's embedded proof, policy and statement to its declared retained object address and exact bytes before yielding a verified capability. This is immutable-capture integrity failure, not demonstrated signature forgery or public Runtime object deletion. No base reproduction establishes origin."
  }
]
```
