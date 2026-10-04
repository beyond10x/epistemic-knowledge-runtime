---
format: aep.planning-md/3
id: review-result:mapping-integration-r1
kind: review-result
status: active
title: Mapping integration independent review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
needs-revision

Owners: 1 coordinator/kernel implementor finding; 0 delegated implementor findings. The earlier residual-progress-r1 finding is corrected in source and is not counted again.

Bounded review of the current mapping integration's application authorization, residual projection, committed progress, receipt/report handling, captured reads, replay and Explain links. Correction application remains explicitly refused and full F is outside this review.

The residual-progress-r1 correction is present: `application_material.rs:30-32` clears completed additions, and `schema_proposals.rs` still validates the immutable original candidate using `original_proposal`. The residual digest continues binding the original proposal and verified prefix while live declaration dependencies come from the remaining work. This closes the cited source defect; the dedicated unrelated-declaration versus relevant-dependency regression was not executed or inspected in this review.

One new report-consistency defect remains. `schema_application.rs:329-330` retains an application receipt and then replays again. `application_progress.rs:229` derives item outcomes through that newer head, while lines 261-266 copy the older receipt's identity, progress, remaining items and stop reason. A concurrent application can commit a remaining mapping between those observations. The returned report can then label the item integrated with its transaction/assertion IDs while also listing it as remaining under a partial progress receipt. This is a reachable concurrency window in the Runtime reporting path, established from source; it has not been independently reproduced and is not an authorization bypass.

Add a deterministic interleaving test that pauses a caller after retaining a partial receipt, lets a second caller finish the remaining mapping, then resumes the first caller's final replay/report. Require all report fields to describe one verified prefix. Derive item outcomes through the exact receipt prefix, reusing receipt verification's committed-boundary calculation, or refresh and persist a receipt for the same state used to build the report. A lower-level control can supply a genuine older partial receipt with a genuine later verified state to the report helper and assert coherent results.

Within the inspected source, guarded Propose checks exact semantic mapping resolution; later structural checks preserve frozen operations and allocations rather than reinterpreting aliases. Every guarded occurrence still checks effective signed review and continuation material. Prefixes count verified commits through the requested revision, not step elections or receipt assertions. Processing receipts are checked against actual mapping commits, and completion recovery requires a verified complete prefix. Captured Explain metadata includes committed steps only, compares exact retained mapping bytes and source hashes, and checks derivation/evidence correspondence. No additional concrete authority defect was established in these bounded paths.

Evidence inspected: `<retained-evidence>/schema-application-kernel/application-mapping-green-3.log` reports one passing test, zero failures. Its source, `approved_mapping_integrates_a_parked_fact_once_on_both_providers`, loops over File and SQLite, checks one accepted assertion, exact Mapping/Derivation Explain links, missing mapping-object refusal, no-write exact retry, full reopen equality and detached-history verification after temporary provider deletion. This is the implementor's execution, not independent test execution. The test does not cover interrupted multi-item progress, changed residual basis or the report interleaving above.

Limitations: source/log review only; no Cargo, build, test, mutation, source edit, AEP operation or publication performed. The implementation is evolving and this report does not approve full F, correction application, all recovery boundaries or the full gate.

```findings
[
  {
    "file": "crates/ekr-kernel/src/application_progress.rs",
    "line": 229,
    "category": "correctness",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "report derives item outcomes through the current replay head but copies progress, remaining_items and receipt_id from the supplied older receipt. record_application_progress retains that receipt and then replays again, so a concurrent Apply can commit another mapping between those observations. The same returned item can be integrated with transaction/assertion IDs and still appear in remaining_items under partial progress. Build the report from one verified receipt prefix, or refresh/persist a matching receipt, and add a deterministic partial-receipt/concurrent-completion control. Source-established Runtime concurrency window; not independently reproduced or an authorization bypass."
  }
]
```
