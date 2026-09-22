---
format: aep.planning-md/1
id: review-result:p1-writer-activation-adversary
kind: review-result
status: active
title: Reconcile retained retries and strict dispatch before activation
relations:
- reviews: story:version-persisted-contracts
- reviews: story:commit-and-revision-lineage
revision: 1
---
Owners: 4 findings, 4 coordinator, 0 implementor.

Independent static review of the unapplied activation patch is retained in
.engineering/reviews/p1-writer-activation-adversary.md, including exact generated
retry scenario observations, origin comparisons and the installed-compiler probe.
No runtime case or production source was changed or executed. Proposed-file line
numbers below refer to the reviewed proposal, not current production declarations.

The coordinator verified the repeated Commit and Seed conflicts, existing inline
ObjectStored/schema1, and the absent Validate basis. Correction must preserve
retained own-result retries, explicit schema2 metadata dispatch, and a no-write
RevisionNotFound refusal. Installed ESS rejected silent preserving outcomes;
a checksum-verified newer published executable supports their vocabulary, with
its exact state/payload rules still requiring correct declarations. This is not
an upstream implementation blocker and no generated suite may be hand-edited.

```findings
- file: systems/ekr/domains/kernel.yaml
  line: 938
  category: correctness
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "Commit in Committed is declared wrong-state, contradicting return of the exact retained result. Correct only that state; other invalid states still refuse without events."
- file: systems/ekr/domains/kernel.yaml
  line: 841
  category: correctness
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "Seed reports AlreadySeeded for exact matching retained input and anchor retries, including after head advancement. Return the original Root0 silently; changed input or anchor refuses."
- file: systems/ekr/domains/store.yaml
  line: 96
  category: correctness
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Metadata-only ObjectStored drops inline bytes without its own new schema discriminator. Use schema2, retain frozen schema1 verification and unchanged retention-event schema, and refuse mixed shapes."
- file: systems/ekr/domains/kernel.yaml
  line: 879
  category: correctness
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "Validate against an absent revision has no truthful outcome and cannot construct mandatory requested_basis. Declare RevisionNotFound without terminal mutation; retain validation against older existing revisions."
```
