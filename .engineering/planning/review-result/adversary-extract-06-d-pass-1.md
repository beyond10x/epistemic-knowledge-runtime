---
format: aep.planning-md/3
id: review-result:adversary-extract-06-d-pass-1
kind: review-result
status: active
title: Adversary, extract-06 unit D, pass 1
relations:
- reviews: story:extraction-document-applies-to-a-store
revision: 1
---
NEEDS-CHANGE

Adversary pass 1 on unit D (`impl/extraction-document` at `1cf0c718`; 18 cases committed as `dd8c4a77`, 12 red and ignored). One blocker (the value-type shape disagrees with its documentation in a published `/1` format), eight warnings, three notes.

Held: nesting to the recursion limit within a 2 MiB stack, merge keys refused, round trip through `ekr example`, tags agreed between schema and reader except on scalars, byte-for-byte name comparison, every `extraction-type-undeclared` site.

```findings
- file: crates/ekr-integrate/src/extraction.rs
  line: 83
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "ValueSpec Enum/NodeRef take `parameters: [..]` while extraction.rs:60 and docs/cli.md say the value is written as a value type, whose Enum is `parameters: {variants: [..]}`; the reader refuses the documented form"
- file: crates/ekr-integrate/src/extraction.rs
  line: 357
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the only bound before the YAML load is 8 MiB, and load time grows with the square of flow-nesting depth (6 s at 64 KiB in debug), so a document under the cap is not answered in 30 s; a depth limit like the transaction document's is missing
- file: crates/ekr-integrate/src/extraction.rs
  line: 501
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a node type, property or edge type declared twice is merged and accepted, though names are unique and the SDK OntologySpec refuses it as DuplicateName
- file: crates/ekr-integrate/src/extraction.rs
  line: 503
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a store type redeclared with another parent gains that parent's properties in the check, where the SDK refuses the redeclaration as Conflict
- file: crates/ekr-integrate/src/extraction.rs
  line: 165
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: ExtractedReference.aliases reads ~, null, true, 1.0 and 0x10 as text, while TypedReference refuses them
- file: crates/ekr-integrate/src/extraction.rs
  line: 81
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: an empty or repeated Enum list and an empty NodeRef list are accepted, though ValueType refuses all three
- file: crates/ekr-integrate/src/extraction.rs
  line: 87
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a Record value type with one field written twice keeps the last silently, unlike the repository's unique_map rule
- file: crates/ekr-integrate/src/extraction.rs
  line: 415
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: two evidence items under one id are accepted, so a cited id names two different payloads
- file: crates/ekr-integrate/src/extraction.rs
  line: 230
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a payload that does not hash to its content_hash is accepted, though docs/cli.md and the schema say it must
- file: crates/ekr-integrate/src/extraction.rs
  line: 412
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a named thing with aliases [] or [""] is accepted, though the resolver refuses it as reference-without-identity
- file: crates/ekr-integrate/src/extraction.rs
  line: 419
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a property fact's value is not checked against the property's type, so a document no store can take is accepted
- file: crates/ekr-integrate/src/extraction.rs
  line: 428
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a relation's subject and object are not checked against the edge type's ends
```
