---
format: aep.planning-md/1
id: review-result:p1-scope-round-1
kind: review-result
status: active
title: Scope critic, P1 decomposition, round 1
tags:
- critic:aep-plan:plan-critic-scope
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
needs-revision

- story:graph-model-and-assertions — the story's Notes commit `KnowledgeState` (design § 29) as a live field on `GraphRoot` in P1, but § 29 is the roadmap's P3 territory ("Design § 22–29 ... root utility metadata") and no sentence in the parent epic names § 29 or `KnowledgeState` — .engineering/planning/story/graph-model-and-assertions.md:70, parent Outcome at .engineering/planning/epic/p1-kernel-ontology-core.md:18-30 (silent on § 29), docs/roadmap.md:162
- story:graph-model-and-assertions — the story's Scope builds `QueryScope` on design § 47 ("active, as-of revision, valid-at time, include disputed"), but § 47 is what the roadmap names for P4 ("Query scopes of § 47; the § 62 explain chain", docs/roadmap.md:179), the parent epic cites only § 71–72 for reads (design § 71's `GraphSnapshot` carries only a revision, no scope selector — docs/epistemic-knowledge-runtime-design.md:2803-2811), and the story's own Acceptance exercises only two of the four variants it defines — .engineering/planning/story/graph-model-and-assertions.md (Scope section, `snapshot.rs` line), docs/roadmap.md:179

What I read: the parent epic body in full (`aep plan artifact show epic:p1-kernel-ontology-core`, Outcome/Acceptance/Carries/Notes) plus its raw file at `.engineering/planning/epic/p1-kernel-ontology-core.md`; `docs/roadmap.md` § 4 (P0–P7) and § 5–7; `aep plan artifact graph` to check for prior claimants; `aep plan artifact kinds` and `aep plan artifact relations`; all 11 drafted artifacts via `aep plan artifact show <id>` in full; and the design document sections each story cites (`docs/epistemic-knowledge-runtime-design.md` §§ 17, 19, 21, 29, 34, 37, 47, 71–72) to check the phase assignment each citation implied. Extracted 20 distinct promises from the parent (Outcome bullets, Acceptance clauses, Carries, Notes); all 20 trace to at least one drafted item, so no gap findings — the two findings above are both reach beyond the parent, not missing coverage. No exclusion in the parent's own Notes ("no crate names a domain concept") was violated by any crate name in the set.

What I could not establish: whether design § 69 (`SchemaRegistry`, cited by story:ontology-types-and-values) or § 73 (cited by story:transaction-and-validators) are assigned to a later phase by the roadmap — I found no roadmap sentence naming either section for any phase, so I did not treat their P1 presence as reach; an operator with fuller context on those two sections might disagree. Whether the roadmap's own tension — its P4 bullet names § 62 (explain) while the parent epic already promises `explain` as a P1 CLI verb — should be read as the parent overriding the roadmap or a pre-existing inconsistency between the two documents is outside my lane (I judged the drafted set against the parent epic, which explicitly claims `explain`, so I did not flag it). Whether the design-coherence or acceptance-testability angles on the two findings above are also live is plan-critic-design's and plan-critic-acceptance's respectively, not mine.

```findings
- file: .engineering/planning/story/graph-model-and-assertions.md
  line: 70
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the story's Notes commit KnowledgeState (design § 29) as a field on GraphRoot in P1, but § 29 is roadmap-assigned to P3 (root utility metadata, docs/roadmap.md:162) and no sentence in the parent epic's Outcome/Acceptance/Carries/Notes names § 29 or KnowledgeState"
- file: .engineering/planning/story/graph-model-and-assertions.md
  line: 46
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story builds QueryScope on design § 47, which the roadmap names for P4 (Query scopes of § 47, docs/roadmap.md:179); the parent epic cites only § 71-72 for reads, whose GraphSnapshot carries no scope selector, and the story's own acceptance exercises only two of the four QueryScope variants it defines"
```
