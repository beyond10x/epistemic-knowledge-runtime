---
format: aep.planning-md/2
id: review-result:p1-design-round-2
kind: review-result
status: active
title: Design critic, P1 decomposition, round 2
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
Reviewed via `aep plan artifact show` on all 10 stories, `epic:p1-kernel-ontology-core`, `executable-system-specification:ekr-v1`, and `review-result:p1-design-round-1`; `aep plan artifact relations`, `aep plan artifact graph` (66 edges walked, including the 8 to artifacts outside the set — epic, initiative-adjacent epics, the ESS, and the four other review-results); `aep plan artifact validate`; direct reads of `systems/ekr/components.yaml`, `systems/ekr/domains/kernel.yaml`, and `docs/epistemic-knowledge-runtime-design.md` § 68 (Rust Domain Sketch).

needs-revision

story:graph-model-and-assertions — its Notes (`.engineering/planning/story/graph-model-and-assertions.md:84-86`) claim `RevisionEvent`'s move here lets `ekr-kernel` and `ekr-store` "share one definition without either depending on the other," but `RevisionEvent`'s own field shapes (lines 67-70, drawn from `systems/ekr/domains/kernel.yaml:429-461`) are typed `ekr.kernel.TransactionId`, `RevisionId`, `RevisionNumber`, `ContentHash` — newtypes minted nowhere but `ekr-kernel/src/identity.rs` and `hash.rs` (`.engineering/planning/story/kernel-identity-and-hashing.md:43-47`) — so `ekr-graph` needs a Cargo dependency on `ekr-kernel` to compile this file. Meanwhile `story:transaction-and-validators` (`.engineering/planning/story/transaction-and-validators.md:34-37`, validating "a reference that does not resolve in the snapshot") and `story:commit-and-revision-lineage` (`.engineering/planning/story/commit-and-revision-lineage.md:44-46`, "applies each `GraphOperation` to the canonical graph; computes ... the new `Root`") both live in `crates/ekr-kernel/` and need `GraphSnapshot`/`CanonicalGraph`/`Root` — types this same story defines in `ekr-graph`. That is a two-way Cargo package dependency, which Cargo refuses to build, and `story:workspace-crate-skeleton`'s per-crate dependency list — the one place meant to be complete (`.engineering/planning/story/workspace-crate-skeleton.md:54-60`) — declares neither direction; `transaction-and-validators.md:67-68` and `commit-and-revision-lineage.md:58` each independently claim "adds no dependency beyond the skeleton's," which is false given what their own bodies describe using. Design § 68 (`docs/epistemic-knowledge-runtime-design.md:2655-2723`) sketches `kernel/`, `graph/` etc. as modules inside one crate, where cross-references in both directions are unproblematic; splitting them into separate library crates while keeping the bidirectional type references is what turns a safe module layout into an unbuildable package cycle — citation: the five files above, `aep plan artifact graph`.

What I read: 12 artifacts in full (10 stories, the epic, the ESS) plus `review-result:p1-design-round-1`; `aep plan artifact relations`, `aep plan artifact graph`, `aep plan artifact validate`; `systems/ekr/components.yaml`, `systems/ekr/domains/kernel.yaml`; `docs/epistemic-knowledge-runtime-design.md` § 68; and, for context on how other lanes already treated the dependency-list gap, `review-result:p1-scope-round-2` and `review-result:p1-parallel-safety-round-2`.

What I could not establish: whether the fix intended for `ekr-graph`'s `events.rs` to use raw primitive types (`Uuid`, `[u8; 32]`) rather than the concrete `ekr-kernel` newtypes for `RevisionEvent`'s fields — the story body cites `systems/ekr/domains/kernel.yaml`'s field *shapes* directly, which is the stronger, newtype reading, but a looser reading would remove half of the cycle I found. Out of my lane and not weighed into this verdict: `review-result:p1-parallel-safety-round-2`'s open finding (an external `trybuild` dev-dependency mismatch between `ekr-kernel` and `ekr-graph`) touches the same `workspace-crate-skeleton` dependency list but is a different defect (a missing external dev-dependency, not a package cycle); `review-result:p1-scope-round-2` approved and explicitly treated "the dependency additions" as "`Cargo.toml` mechanics, not new outcomes," so it did not check Cargo buildability — neither overlaps this finding. Round 1's single design finding (`eventlog-store` defining kernel event shapes it did not own) is fixed and I do not repeat it: `story:eventlog-store`'s current Notes correctly disclaim owning any event shape and state the kernel fills and publishes them.

```findings
- file: .engineering/planning/story/graph-model-and-assertions.md
  line: 84
  category: design
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "RevisionEvent's field types (ekr.kernel.TransactionId, RevisionId, RevisionNumber, ContentHash) are minted only in ekr-kernel's identity.rs/hash.rs, so ekr-graph needs a Cargo dependency on ekr-kernel to compile this file; but story:transaction-and-validators and story:commit-and-revision-lineage, both scoped inside ekr-kernel, need ekr-graph's GraphSnapshot/CanonicalGraph/Root, so ekr-kernel needs ekr-graph too. That is a two-way Cargo package dependency Cargo cannot build, workspace-crate-skeleton's per-crate dependency list declares neither direction, and this story's own Notes claim ('without either depending on the other') and two other stories' claim ('adds no dependency beyond the skeleton's') are contradicted by what their own bodies describe using."
```
