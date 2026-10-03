---
format: aep.planning-md/3
id: review-result:application-captured-input-independent-r1
kind: review-result
status: active
title: Independent review of captured source and review material projection
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
approve

Owners: 0 coordinator/kernel implementor findings; 0 delegated implementor findings.

Bounded independent source/log review of the captured-input refactor in crates/ekr-kernel/src/incubation.rs, read.rs and schema_proposals.rs over 03dc4892bfa2b2dbda1d86bd49c99073f2673652. No Cargo, tests, source edits or AEP mutations were performed by this reviewer. The unchanged schema-plan helper and unimplemented F verifier are outside this increment.

No supported correctness findings remain. incubation::checked_document preserves coordinate checks, duplicate/declaration checks for observation references, extraction/incubation validation, exact observation evidence hash/payload comparison and local inheritance-cycle rejection. The production wrapper still supplies the authenticated observation reader. source_support retains selected-item validation, enum support derivation, competing evidence detection, duplicate source/observation/evidence checks, canonical evidence payload verification and the nonempty-support rule. Production interpretation loading still uses retained_interpretation, which verifies the stored version and checked source document before returning it.

project_captured preserves the original candidate/current-candidate fallback, strict correction validation, canonical proposal identity validation, mapping preview, relevant declaration closure and correction-component material. The source digest still serializes the same ordered documents: replacing references to Box<Document> with references to Document does not alter their JSON representation. The observation vector is now supplied as a slice but serializes as the same ordered sequence. Hash domain labels, tuple field order, retained observation payloads, canonical evidence and correction digest components remain unchanged. Validation order moves some support loading before strict correction/identity checks; no successful-path validation is removed and no write is introduced.

KernelAuthority::capture_read preserves the admitted head graph/root, seed inputs, revision records, transaction/answer snapshots and complete authority_changes map. Captured object bytes now clone their Arc handles instead of moving those handles out of the owned history; the returned read owns the handles and does not borrow the caller's history lifetime. Commit::read still performs provider history loading and authority reconstruction first. It requests shared aliases only for a current read; historical reads receive independent cells. The existing shared alias cache remains keyed by both revision identity and root, and the extracted helper releases the cache lock before constructing the rest of the snapshot. No provider recursion was introduced into capture_read.

Retained evidence inspected: <retained-evidence>/captured-input-regression.log records 31 library tests, 14 knowledge-retention tests and 8 schema-review tests passed, with no failures or ignored cases. Coverage includes retained-source checks, schema-material dependency behavior, historical review material, unrelated advancement, cross-kind decision identity and concurrent exact retries. captured-input-regression.status is zero. The subsequently completed captured-input-clippy.log records successful kernel checking and captured-input-clippy.status is zero. These are coordinator executions inspected by this reviewer, not independent execution.

Limits: these internal helpers accept already admitted/captured inputs and do not independently authenticate arbitrary loader results, mismatched history/state pairs or a caller-supplied observation slice. The current production wrappers preserve those preconditions. Future F replay must verify captured objects, coordinates, base ontology and observation ordering before using these helpers; no F verifier, application publication/recovery, new authorization or full-gate acceptance is established here. The regression is not an independent old/new byte-vector comparison, although source inspection preserves the successful-path serialization inputs described above.

```findings
[]
```
