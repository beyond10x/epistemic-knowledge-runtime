---
format: aep.planning-md/2
id: review-result:p1-design-round-1
kind: review-result
status: active
title: Design critic, P1 decomposition, round 1
tags:
- critic:aep-plan:plan-critic-design
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

story:eventlog-store — its scope's `events.rs` (`.engineering/planning/story/eventlog-store.md:46-48`) commits to the full `RevisionEvent` type — `TransactionValidated`, `TransactionRejected`, `TransactionStale` with the field shapes `systems/ekr/domains/kernel.yaml` gives those events (lines 429-452: `TransactionRejected.issues`, `TransactionStale.validated_against`/`current`) — but `components.yaml:18-27` assigns publishing of exactly these events to component `ekr-kernel`, not `ekr-store`, and the semantics that fill those fields belong to `story:transaction-and-validators` (what counts as an issue) and `story:commit-and-revision-lineage` (when `validated_against != head`, i.e. `Stale`). No edge records eventlog-store's outcome depending on either story's internals, and eventlog-store's own notes assert independence ("Depends on `story:graph-model-and-assertions` only, so it runs beside the validators" — eventlog-store.md:53-54). Recording the dependency directly would cycle, since `story:commit-and-revision-lineage` already `depends_on` `story:eventlog-store` — citation: `systems/ekr/components.yaml:18-27`, `systems/ekr/domains/kernel.yaml:429-452`, `aep plan artifact graph`.

What I read: all 10 story artifacts plus `epic:p1-kernel-ontology-core` and `executable-system-specification:ekr-v1`, full body via `aep plan artifact show <id>` (12 artifacts). Commands run: `aep plan artifact relations`, `aep plan artifact graph` (walked all 57 printed edges, including the 7 edges to artifacts outside the set — the epic, the initiative, the two prior review-results, the ESS), `aep plan artifact validate`, `aep plan artifact list --kind review-result`, plus direct reads of `systems/ekr/components.yaml` and `systems/ekr/domains/{kernel,store}.yaml`.

What I could not establish: whether the drafter intends `ekr-store`'s `RevisionEvent` to be a distinct wire/persistence type that merely mirrors `ekr.kernel`'s event names by convention (in which case the coupling is real but perhaps deliberately loose) rather than a literal re-derivation of the kernel domain's semantics — the story body does not say which, and I read the stronger reading because the scope line cites `systems/ekr/domains/kernel.yaml events` directly as its source. Out of my lane, and not weighed into this verdict: `review-result:p1-acceptance-round-1` already found each story's acceptance joins independent claims (9 blockers, acceptance lane) and `review-result:p1-parallel-safety-round-1` found `story:transaction-and-validators` and `story:eventlog-store` collide unrecorded on the workspace `Cargo.lock` (parallel-safety lane) — neither overlaps this finding, and I did not let either set my verdict. The single depends_on chain from `workspace-crate-skeleton` through `ess-conformance-kernel` (with one 2-wide branch at `transaction-and-validators`/`eventlog-store`) tracks the design document's own layering (§ 9-21, § 34) and each link carries its own independently-checkable acceptance test, so I did not treat the chain's depth itself as a "serialises the set" defect.

```findings
- file: .engineering/planning/story/eventlog-store.md
  line: 46
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its scope's events.rs commits to the full RevisionEvent type (TransactionValidated, TransactionRejected, TransactionStale) with field shapes systems/ekr/domains/kernel.yaml gives those events, but components.yaml assigns publishing of those same events to component ekr-kernel not ekr-store, and the semantics that fill their fields belong to story:transaction-and-validators (issues) and story:commit-and-revision-lineage (staleness) with no edge recording either coupling, and recording it as depends_on would cycle since commit-and-revision-lineage already depends_on eventlog-store"
```
