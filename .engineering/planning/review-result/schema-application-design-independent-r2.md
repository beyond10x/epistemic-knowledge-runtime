---
format: aep.planning-md/3
id: review-result:schema-application-design-independent-r2
kind: review-result
status: active
title: Independent revised application contract design review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
approve

Owners: 0 outstanding coordinator/design findings; 0 outstanding delegated implementor findings. The 3 coordinator/design findings in design-review.md are addressed by the amended candidate and authoritative ESS/design declarations.

Bounded source-only design review of the amended .engineering/reviews/knowledge-schema-application-design/minimal-contract-proposal.md and the F contract candidate in systems/ekr/domains/integrate.yaml, systems/ekr/domains/kernel.yaml, systems/ekr/domains/store.yaml and docs/epistemic-knowledge-runtime-design.md section 105.17. No Cargo, tests, source changes or upstream actions were performed. This verdict approves the reviewed design direction, not runtime implementation or F acceptance.

The transaction-lifecycle finding is resolved. RetainedApplicationAttempt and ApplicationStepAttempt distinguish an immutable semantic step template from ordinary transaction attempts. The first attempt equals the template; a successor names the latest predecessor transaction and verified terminal Stale record, changes only the transaction identity, and preserves other fields, operations and allocated identities. ApplicationAttemptRetention requires atomic latest-attempt election. Merely proposed, validated, rejected or uncertain attempts cannot authorize a successor, and uncertain publication is resolved first. These rules preserve the existing terminal stale lifecycle and immutable Validate input rather than attempting to revive a stale transaction. Receipts name the actual committed attempt.

The partial-progress approval finding is resolved at the contract level. The schema commits first, every selected mapping must commit next, and all selected corrections form one final atomic transaction. Unresolved mappings cannot be discarded to reach corrections. After the correction commit there is no further canonical application work; lost reporting receipts are recovered from actual linked commits. Application-aware Show and approval material bind verified own-prefix publication references and residual evidence/options/effects, preserving the original proposal and correction instructions. Section 105.17 also requires historical review interpretation at the review's own observed basis and permits only exact approved remaining effects. Thus an initial review is not invalidated solely by its own elected progress, a new approval can resume partial mapping work after rejection, and coincidental external changes cannot be credited as completed application steps.

The preparation compatibility finding is resolved. Guarded ordinary events use revision-event/6 with the guard in event.application; the authoritative store.Publication declaration carries it through that event rather than duplicating the guard. Guarded preparations use publication-preparation/7, preserving existing /1 through /6 bytes and authorization, particularly /6 shared human-decision binding. The candidate now acknowledges the existing authenticated identity append and permits only explicitly declared extra-stream composition.

The design retains the required authorization boundary: human review and application markers compare the same physical proposal-stream position, while marker-only advancement leaves the human predecessor unchanged. Every guarded ordinary publication, including stale or rejected outcomes, links its exact attempt to an effective historical approval and a nonempty atomic marker. Full replay must reject missing, duplicated, mismatched and out-of-order links. Ordinary generic entry points retain the elected transaction's application linkage, so omission of the optional event field is explicitly forbidden as a bypass. Exact recovery of an already committed request after rejection is distinguished from authority for a new write.

Retained tool evidence inspected: <retained-evidence>/f-validate-final.log reports 9 files valid; f-compile-final.log reports 563 declarations compiled; f-data-final.log reports 474 model types generated; f-semantic-final.log reports generated semantic artifacts and outstanding runtime obligations. These are author executions inspected by this reviewer, not independent execution or conformance evidence.

No additional concrete design findings remain within this bounded round. Cross-record checks, exact hashing/serialization compatibility, versioned preparation succession, generated-model adaptation, corruption refusal, provider atomicity, generic-commit guard enforcement and the prescribed crash/concurrency probes still require implementation and executable evidence. No runtime security, full workspace gate or complete F/PR approval is claimed.

```findings
[]
```
