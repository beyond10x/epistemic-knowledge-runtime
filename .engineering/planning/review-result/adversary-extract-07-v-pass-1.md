---
format: aep.planning-md/3
id: review-result:adversary-extract-07-v-pass-1
kind: review-result
status: active
title: Adversary, extract-07 unit V, pass 1
relations:
- reviews: story:extraction-verb-shares-the-sdk-path
revision: 1
---
NEEDS-CHANGE

Adversary pass 1 on unit V (`impl/extraction-verb` at `8cbfd2e9`; cases committed as `3c3aa312`, 15 cases, 8 red and ignored). Fixes are for the next session.

Held: the verb's store equals the SDK-over-session store on ambiguous references, aliases shared across named things, reused and twice-cited evidence, the empty document and a half-failing document (committed per batch, the rejected fact named); no child process (strace); writes only through propose, validate and commit; a session refuses the verb (`session-verb-refused`); report keys match the domain.

```findings
- file: crates/ekr-sdk/src/extraction.rs
  line: 325
  category: property
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: applying one document a second time asserts every fact again, doubling active assertions
- file: crates/ekr-sdk/src/extraction.rs
  line: 209
  category: property
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the number of nodes created depends on the order the document lists named things that share aliases
- file: crates/ekr/src/cli/extraction.rs
  line: 61
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a document the reader accepts faults with exit 1 and no report after its schema change committed (reference-type-has-subtypes), an outcome the domain does not declare
- file: crates/ekr-sdk/src/document/extraction.rs
  line: 155
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the SDK mirror reads a YAML alias, a non-string alias, a document over 8 MiB and one past the depth bound, all refused by the engine reader
- file: crates/ekr-sdk/src/evidence.rs
  line: 201
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: over a child session two evidence items under one id resolve silently to the last one, which the reader refuses as duplicate-identity
- file: systems/ekr/domains/integrate.yaml
  line: 1
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: ApplyExtraction and its ExtractionApplied event are declared but nothing emits the event and no conformance scenario exercises the command
```
