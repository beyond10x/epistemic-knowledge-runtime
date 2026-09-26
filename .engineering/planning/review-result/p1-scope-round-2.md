---
format: aep.planning-md/2
id: review-result:p1-scope-round-2
kind: review-result
status: active
title: Scope critic, P1 decomposition, round 2
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
approve

What I read: the parent `epic:p1-kernel-ontology-core` (full body, via `aep plan artifact show`) and `docs/roadmap.md` § 4 P1 and § 3 (crate table) and § 7 (out of scope), read in full *before* opening any drafted item. Then all 11 drafted artifacts in full (`story:workspace-crate-skeleton`, `story:kernel-identity-and-hashing`, `story:ontology-types-and-values`, `story:graph-model-and-assertions`, `story:transaction-and-validators`, `story:eventlog-store`, `story:commit-and-revision-lineage`, `story:seed-and-explain`, `story:ekr-cli`, `story:ess-conformance-kernel`, `executable-system-specification:ekr-v1`). `aep plan artifact graph` (to check no outside claimant covers any promise, and to see the epic's sole `depends_on epic:p0-bootstrap`). `aep plan artifact kinds` and `aep plan artifact relations` (confirmed `decomposes` is the drafted-from edge). `review-result:p1-scope-round-1` plus the other three round-1 results (`design`, `acceptance`, `parallel-safety`) to identify what changed and check no fix introduced a new scope defect. Design doc sections § 69–73 read directly (`docs/epistemic-knowledge-runtime-design.md`) to verify the `GraphSnapshot` field shape and the `KnowledgeState`/`QueryScope` narrowing claimed in the stories' Notes.

I extracted 20 distinct promises from the parent (Outcome's 5 bullets expand to ~15 discrete claims across identity, typed values, node/edge types, lifecycles, assertions/validation-state/evidence, the transaction→commit→root pipeline, snapshot+concurrency, the eventlog store with two providers/storage classes/content addressing, and six CLI verbs; plus the four Acceptance clauses; plus Carries A13; plus the two Notes exclusions). All 20 trace to at least one drafted item; none is orphaned.

Round 1's two findings are both fixed, not merely reworded:
- `KnowledgeState` (§ 29) is gone from `story:graph-model-and-assertions`'s scope; `root.rs` now carries only the fields `graph.yaml` declares, and the Notes name § 29 as P3 territory explicitly excluded — `.engineering/planning/story/graph-model-and-assertions.md` (Scope, `root.rs`; Notes).
- `QueryScope` (§ 47) is gone from scope; `snapshot.rs` is narrowed to exactly `active()` and `valid_at(Timestamp)` per design § 71, and the Notes name § 47/P4 as explicitly excluded — same file, Scope (`snapshot.rs`) and Notes.

I checked the rest of the set for defects the other three critics' fixes might have introduced (`story:eventlog-store` lost its `events.rs`, moved to `story:graph-model-and-assertions`; `story:workspace-crate-skeleton` gained per-crate dependency declarations). Neither move adds a claim the parent did not ask for or drops one it did — `RevisionEvent`/`Root` are still defined once (in `ekr-graph`) and still traced to the same epic Outcome bullets (§ 19, § 34), and the dependency additions are `Cargo.toml` mechanics, not new outcomes.

No two items claim the same outcome (each crate/file ownership in the "Scope" sections is disjoint, and where two stories both touch `Root`/`RevisionEvent` one defines and the other computes/fills — a design-lane distinction, not a duplicate scope claim). No promise is narrowed without the Notes saying so.

What I could not establish: same two items round 1 flagged as unresolved — whether design § 69 (`SchemaRegistry`) and § 73 belong to a phase the roadmap never names; I read them as pre-existing and not reach, consistent with round 1's reading, since no roadmap sentence assigns either elsewhere and each citing story explicitly defers the transactional/evolution parts to P5. The roadmap's own tension between the parent epic's explicit P1 `explain` verb and § 4's P4 bullet ("Query scopes of § 47; the § 62 explain chain") is unchanged from round 1 and remains outside my lane to resolve — I judged against the parent epic, which claims `explain` directly. Whether the acceptance-lane and design-lane findings from the other three round-1 reviews (acceptance joins, the `eventlog-store`/`events.rs` component-ownership question, the `Cargo.lock` collision) are now fully closed is those critics' call, not mine; I note only that none of their fixes reads as a new scope defect.

```findings
[]
```
