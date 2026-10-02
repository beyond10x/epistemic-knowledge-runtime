---
format: aep.planning-md/3
id: review-result:adversary-correct-07-v-pass-2
kind: review-result
status: active
title: Adversary, correct-07 unit V, pass 2
relations:
- reviews: story:extraction-verb-shares-the-sdk-path
revision: 1
---
NEEDS-CHANGE

Adversary pass 2 on unit V (`impl/extraction-verb-c7` at `ce51e93d`; cases committed as `2bc5c6db`, ten red and ignored). Lanes run with the new file: 45 → 58 cases, 10 failing.

The mirror still reads three shapes the reader refuses (a confidence past 10000, a record value or record type with a field written twice). Over a child session the SDK applies a fact citing evidence its document does not list. A stop inside the facts batch after the run's first commit returns an error, not a report, and a stop's report omits the stopping batch's commits. A second run re-asserts a fact retracted or superseded since the first (two active values for a cardinality-One property). Two reader-accepted documents fault the verb with exit 1 (non-human evidence; a subtype redeclaring an inherited property). `cargo fmt --check` fails at `extraction_cli.rs:773`.

Held: a re-run after a stop is idempotent; a fact read again from new evidence is asserted and then held; a group is named alike in every order on both providers; `ekr-core::decode` names no domain concept; G's divergence tests pass; a session refuses the verb; no child process; writes only through propose, validate and commit.

```findings
- file: crates/ekr-sdk/src/document/graph.rs
  line: 324
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the SDK mirror reads a confidence past 10000 that the reader refuses as extraction-document-malformed
- file: crates/ekr-sdk/src/document/value.rs
  line: 80
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the SDK mirror reads a record value with a field written twice, keeping the last, which the reader refuses
- file: crates/ekr-sdk/src/document/extraction.rs
  line: 273
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the mirror's ontology wire form reads a record type with a field written twice, which the reader refuses
- file: crates/ekr-sdk/src/document/extraction.rs
  line: 218
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: over a child session the SDK applies a fact citing evidence its document does not list, which the verb refuses as fact-evidence-unlisted
- file: crates/ekr-sdk/src/extraction.rs
  line: 423
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a stop inside the facts batch after the run's first commit returns ApplyError::Batch instead of a report
- file: crates/ekr-sdk/src/extraction.rs
  line: 535
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: under stopped, the report omits the transactions committed by the batch that stopped
- file: crates/ekr-sdk/src/extraction.rs
  line: 392
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a second run re-asserts a fact retracted or superseded since the first, from the same evidence
- file: crates/ekr/src/cli/extraction.rs
  line: 58
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a reader-accepted document with non-HumanStatement evidence faults the verb with exit 1, an outcome the domain does not declare
- file: crates/ekr-sdk/src/document/ontology.rs
  line: 693
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a reader-accepted subtype redeclaring an inherited property faults the verb with exit 1 instead of a refusal
- file: crates/ekr/tests/extraction_cli.rs
  line: 773
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: cargo fmt --all -- --check fails at lines 773 and 787
```
