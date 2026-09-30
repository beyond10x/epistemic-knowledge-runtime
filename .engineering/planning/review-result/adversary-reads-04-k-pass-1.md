---
format: aep.planning-md/3
id: review-result:adversary-reads-04-k-pass-1
kind: review-result
status: active
title: Adversary, wave reads-04 unit K, pass 1
relations:
- reviews: task:changes-since-lists-added-evidence
revision: 1
---
## Verdict

NEEDS-CHANGE on `73097dee`, on the SDK side only; the changes read held (several AddEvidence in one
revision, paging across it, since boundaries, valid-time since, file and SQLite bytes, byte identity
for ranges without one). Fixed in `ff205866`: `ontology`, `snapshot` and `explain` read into
read-side `OntologyCardinality`, `SnapshotSubject` and `SnapshotPredicate` with `Other`, and
`ViewValue::deserialize` is the tolerant reader.

```findings
- file: docs/sdk.md
  line: 670
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Ontology's Cardinality and Snapshot's Subject/Predicate had no Other, so a new kind still failed the whole read the docs said tolerates it"
- file: crates/ekr-sdk/src/read/views.rs
  line: 21
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "remote = Self published a strict inherent ViewValue::deserialize that shadowed the tolerant trait impl"
- file: crates/ekr-sdk/tests/read.rs
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the unit's tolerant cases stayed green without the is_other guard"
```
