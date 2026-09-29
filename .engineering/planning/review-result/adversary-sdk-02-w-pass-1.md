---
format: aep.planning-md/3
id: review-result:adversary-sdk-02-w-pass-1
kind: review-result
status: active
title: Adversary, wave sdk-02 unit W, pass 1
relations:
- reviews: task:sdk-docs-cover-the-document-builders
revision: 1
---
## Verdict

NEEDS-CHANGE on `c5895f27`: the section's claims held against the code (the ten limits, refusal names,
the `ensure` emit table and order, `OntologyError`, profile rulesets, minting equal to `ekr mint`,
hashing equal to `ekr hash`, every anchor), but the example guard let six page mutants through.
The adversary's cases in `4002b000` check the limits table, per-block imports, field lists and
fences; `374452f3` fixed the name check and three sentences. The case asserting the old
`SeedBuilder::node` sentence was deleted by the coordinator's decision, since the sentence was
corrected to what the builder does.

```findings
- file: crates/ekr-sdk/tests/docs_examples.rs
  line: 212
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the name check skipped the limits table, variant field lists and test-local names, so four page mutants passed the guard"
- file: crates/ekr-sdk/tests/docs_examples.rs
  line: 26
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "module-level imports supplied names the example regions omitted, so a copied example could fail to compile"
- file: crates/ekr-sdk/tests/docs_examples.rs
  line: 72
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "a tilde or indented fence was invisible to the guard"
- file: docs/sdk.md
  line: 177
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "SeedBuilder::node stores the caller's type_state, not its type's initial state, as the page said"
- file: docs/sdk.md
  line: 146
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "to_yaml() was said to write a schema change, which has none"
- file: docs/sdk.md
  line: 211
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "every payload was said to convert into its Operation, but DeleteEdge's EdgeId has no From"
```
