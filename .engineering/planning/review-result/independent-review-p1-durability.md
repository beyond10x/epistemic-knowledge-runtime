---
format: aep.planning-md/2
id: review-result:independent-review-p1-durability
kind: review-result
status: active
title: 'Independent review of 209dd5e: membrane, durability and CI'
relations:
- reviews: epic:p1-kernel-ontology-core
revision: 1
---
## Source and attribution

This records the review already performed against 209dd5e, not a new execution against the completion branch. The full report, Rust probe source and observed output are retained under `.engineering/reviews/p1-baseline-209dd5e/`.

Owners: the original report did not assign an owner split; no count by owner is asserted.

The original report has seven numbered findings, some containing multiple reproduced defects. Its additional alignment and planning observations remain in the preserved report. The handoff's different count is not used as an acceptance denominator.

## Routing

Durable commit and reopen: `story:commit-and-revision-lineage`. Assertion typing, post-state references, unsupported semantics and conflicting writes: `story:p1-transaction-membrane-repair`. Seed admission: the seed portion of `story:seed-and-explain`. Required CI: `task:p1-required-correctness-ci`. Root/projection, source-guard and submitter-identity obligations retain their existing tasks and must be bound to the relevant implementation stories.

No finding is closed by this record. Fresh membrane and P1 exit reviews remain required.

```findings
- file: crates/ekr-kernel/src/commit.rs
  line: 26
  category: correctness
  severity: blocker
  verdict: CONFIRMED
  origin: pre-existing
  message: "A real kernel commit reports a new revision without applying CreateNode; reopening loses the committed head because validation authority is process-local."
- file: crates/ekr-kernel/src/validate/types.rs
  line: 230
  category: correctness
  severity: blocker
  verdict: CONFIRMED
  origin: pre-existing
  message: "Assertion property object types and relation endpoint restrictions are not enforced on every branch."
- file: crates/ekr-kernel/src/validate/reference.rs
  line: 176
  category: correctness
  severity: blocker
  verdict: CONFIRMED
  origin: pre-existing
  message: "Reference validation retains deleted edges while cardinality uses their removal, accepting a dangling assertion subject."
- file: crates/ekr-kernel/src/validate/ontology.rs
  line: 9
  category: correctness
  severity: blocker
  verdict: CONFIRMED
  origin: pre-existing
  message: "Unsupported schema changes and nonempty operation preconditions are accepted rather than explicitly refused."
- file: crates/ekr-store/src/snapshot.rs
  line: 222
  category: correctness
  severity: blocker
  verdict: CONFIRMED
  origin: pre-existing
  message: "Seed deserialization admits canonical graphs without semantic graph validation."
- file: crates/ekr-kernel/src/transaction.rs
  line: 168
  category: correctness
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: "Conflicting property updates in the documented unordered operation set have no defined application semantics."
- file: .github/workflows/shared-gates.yml
  line: 14
  category: correctness
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: "The sole CI workflow checks privacy and does not compile or test the candidate repository."
```
