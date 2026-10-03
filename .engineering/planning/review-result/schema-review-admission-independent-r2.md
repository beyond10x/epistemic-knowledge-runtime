---
format: aep.planning-md/3
id: review-result:schema-review-admission-independent-r2
kind: review-result
status: active
title: Signed schema review admission and replay approved
relations:
- reviews: story:propose-better-vocabulary
revision: 1
---
approve

Owners: 0 outstanding coordinator/kernel implementor findings; 0 outstanding delegated implementor findings. The coordinator-owned audience-wide decision-identity finding in independent-r1 is resolved for this bounded signed-review slice.

Independent source and retained-log review of the integrated E signed-review kernel, runtime, CLI and SDK changes in the candidate based on 21668c56c. No builds, tests, repository edits or AEP mutations were performed by this reviewer. This is not full E/F or PR acceptance.

The approval and rejection paths bind the operation, exact proposal identity and byte digest, historical material basis, external proof, statement and predecessor. The verified operator comes from the independently enrolled policy. Before retention, the kernel checks both current material compatibility and the material at the signed historical revision. Replay independently reconstructs that historical basis, authenticates the proof and predecessor, and compares the retained generated decision and statement projection with the verified result. The previously acknowledged historical-basis publication defect is therefore addressed in source and covered by the retained regression.

Audience-wide identity now has both the shared physical reservation and kernel authentication. New canonical upgrade and answer publications explicitly select the /5 signed envelope. Proposal-review replay checks the audience-wide binding against the retained generated record, then authenticates the underlying proof rather than treating index equality as endorsement. The shared atomic singleton prevents changed decisions from winning through different operation kinds; per-proposal predecessor CAS remains separate. The integrated cases cover upgrade-to-proposal reuse and answer/proposal reuse in both directions, exact retries and full-replay reopen behavior. The earlier cross-domain identity finding is closed within these implemented operations.

An exact old approval retry returns its original authenticated snapshot, even after a rejection, without appending a new approval or changing the latest decision. Concurrent identical submissions recover the independently verified elected record after a physical conflict, so independently allocated recording IDs and times do not create a second review. The current regression requires both callers to return the same record and the retained history to contain exactly one review. Canonical application and its required publication-time material checks remain F work; retaining a review does not publish schema changes or mapped facts.

The generated approve/reject adapters compare the typed command with the transport projection and check the returned review identity. The show projection includes typed review history and continues to refuse unsupported application history. CLI approve/reject commands require write access, bound input size, generated input decoding and provisioned runtime trust. SDK wrappers pass the same generated request and response types. The viewer reads authenticated retained statement history and escapes statement text, operator text and serialized provenance. Rejection removes the proposal from the open attention list while retaining inspectable history; replaying an old approval does not reverse that filtering.

Evidence inspected: <retained-evidence>/integrated-kernel-3.log records 34 passed, 0 failed: 7 schema-review cases, 18 verifier cases, 7 upgrade cases and 2 answer recovery cases. Source inspection confirms provider loops and the relevant reopen, full-replay, cross-kind reuse and concurrent exact-retry assertions. <retained-evidence>/integrated-cli.log records the integrated CLI approval/rejection/viewer case passing; its docs and knowledge_cli runners were filtered to zero executed cases, so this log is not evidence that those suites passed. These are coordinator executions inspected by this reviewer, not independent execution.

No additional supported findings remain in the reviewed slice. Correction-material authoring received separate review; this review inspected its integration into historical/current basis checking and authenticated approval replay, not a fresh exhaustive audit of every correction operation. The generic physical store checkpoint finding and its corrected real-kernel reachability classification remain documented in the separate human-decision-identity reviews. Legacy unpublished signed preparations still require an explicit compatibility disposition; no automatic migration is claimed. Final broader CLI/SDK checks, lint, ESS conformance and the full workspace gate are not established by this report. F application is absent and remains outside this approval.

```findings
[]
```
