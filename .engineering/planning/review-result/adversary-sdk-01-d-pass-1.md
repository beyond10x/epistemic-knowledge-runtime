---
format: aep.planning-md/3
id: review-result:adversary-sdk-01-d-pass-1
kind: review-result
status: active
title: Adversary, wave sdk-01 unit D, pass 1
relations:
- reviews: story:sdk-typed-documents
revision: 1
---
## Verdict

CONFIRMED, four defects and one weak check; 102 awkward strings, i64 bounds, floats, seeds, the
payload hash on binary input, 320,000 minted ids and `Ontology::read` at every version held.
The coordinator decided the narrowing question: a spec narrower than the store is already met.

```findings
- file: crates/ekr-sdk/src/document/transaction.rs
  line: 105
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "TransactionBuilder::build returns Ok for documents past the frozen /2 limits and the kernel reader refuses each by name"
- file: crates/ekr-sdk/src/document/ontology.rs
  line: 625
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "Ontology::ensure yields one inherited property or two same-named properties depending on the order the spec lists parent and child"
- file: crates/ekr-sdk/src/document/ontology.rs
  line: 626
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the documented Conflict for a property an ancestor declares differently is raised for existing types only"
- file: crates/ekr-sdk/src/document/ontology.rs
  line: 749
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "a spec narrowing an edge type ends is treated as already met; the coordinator keeps this as the contract"
- file: crates/ekr-sdk/tests/document_drift.rs
  line: 572
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the manifest check matches dependency keys only, so a renamed package = ekr-kernel passes it"
```
