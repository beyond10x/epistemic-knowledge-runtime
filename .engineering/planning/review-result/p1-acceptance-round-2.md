---
format: aep.planning-md/2
id: review-result:p1-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, P1 decomposition, round 2
tags:
- critic:aep-plan:plan-critic-acceptance
relations:
- reviews: story:workspace-crate-skeleton
- reviews: story:kernel-identity-and-hashing
- reviews: story:ontology-types-and-values
- reviews: story:graph-model-and-assertions
- reviews: story:transaction-and-validators
- reviews: story:eventlog-store
- reviews: story:commit-and-revision-lineage
- reviews: story:seed-and-explain
- reviews: story:ekr-cli
- reviews: story:ess-conformance-kernel
revision: 1
---
approve — no.

needs-revision

story:ontology-types-and-values — round-1's join to the unrelated `Lifecycle::transition` claim is fixed (that clause now sits alone in Tests the story ships, ontology-types-and-values.md:43-44), but the acceptance itself still joins two independent claims with "and": `Ontology::check` returning `Ok` exactly when a value satisfies its declared type, and, separately, naming the property and the reason when it does not — the checker can correctly reject an invalid value while misreporting which property or why, so one can hold while the other fails — .engineering/planning/story/ontology-types-and-values.md:36-38

story:graph-model-and-assertions — round-1's join to the trybuild membrane test and the third query claim is fixed (the compile-fail test and the retracted/superseded check now sit in Tests the story ships, graph-model-and-assertions.md:52-53), but the acceptance still joins two independent method-call claims with "and": `valid_at(2025-06-01)` answering Alice and `active()` answering Bob are two separate reads of the snapshot that can diverge — .engineering/planning/story/graph-model-and-assertions.md:47-48

story:ess-conformance-kernel — round-1's join to the separate "records evidence" store action is fixed (that step now sits in Notes and is explicitly named as the store's decision, not this story's acceptance, ess-conformance-kernel.md:50-52), but the acceptance still joins two independent claims with "and": "every synthesised scenario executed" (no skips) and "none failed" (no failures) are separate fields of the same report that can diverge, e.g. a run that skips a scenario while every executed one passes — .engineering/planning/story/ess-conformance-kernel.md:31-32

story:ekr-cli — the acceptance joins two independent claims with "and": the two snapshots naming Alice and Bob respectively (query correctness) and exit 0 at every step (CLI plumbing) — a step can exit 0 while the snapshot content is wrong, or a step can fail for an unrelated reason while the query logic is correct, so one can hold while the other fails — .engineering/planning/story/ekr-cli.md:35-36

What I read: all 11 given ids via `aep plan artifact show <id>` (full body), plus `aep plan artifact kinds`, `aep plan artifact lifecycle story`, `aep plan artifact lifecycle executable-system-specification`, and `review-result:p1-acceptance-round-1` for the round-1 findings to diff against; `grep -n` over each story file under `.engineering/planning/story/` for exact line citations. 11 of 11 read.

What I could not establish: whether `story:ekr-cli`'s acceptance text changed since round 1 — round 1's closing line explicitly chose not to flag it ("one test file over one end-to-end scenario, not two independent components") without quoting the sentence, so I cannot tell if my finding is new content or a defect round 1 saw and excused; I judged the text as it now reads. I did not run `task check`, the proptests, or `ess verify conform run`, so "observable" is judged from what the sentence names, not from executing it. Whether `story:transaction-and-validators`'s disjunctive property ("a reference that does not resolve … or a canonical assertion with no evidence") is implemented as one proptest generator or several independent test functions is outside what the story body states; I read it as one quantified property and did not flag it, but I could not confirm the implementation shape.

```findings
- file: .engineering/planning/story/ontology-types-and-values.md
  line: 36
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the round-1 join to Lifecycle::transition is fixed (moved to Tests the story ships, line 43), but the acceptance still joins two independent claims with and — Ontology::check returning Ok exactly when a value satisfies its declared type, and separately naming the property and reason when it does not — so one can hold while the other fails"
- file: .engineering/planning/story/graph-model-and-assertions.md
  line: 47
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the round-1 join to the trybuild membrane test and the retracted/superseded claim is fixed (moved to Tests the story ships, line 52), but the acceptance still joins two independent method-call claims with and — valid_at(2025-06-01) answering Alice and active() answering Bob are two separate reads of the snapshot that can diverge"
- file: .engineering/planning/story/ess-conformance-kernel.md
  line: 31
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the round-1 join to the records evidence store action is fixed (moved to Notes, line 50), but the acceptance still joins two independent claims with and — every synthesised scenario executed (no skips) and none failed (no failures) are separate fields of the report that can diverge"
- file: .engineering/planning/story/ekr-cli.md
  line: 35
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins two independent claims with and — the two snapshots naming Alice and Bob respectively and exit 0 at every step — a step can exit 0 while the snapshot content is wrong, or fail for an unrelated reason while the content is correct"
```
