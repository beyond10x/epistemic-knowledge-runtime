---
format: aep.planning-md/2
id: story:p1-exit-properties
kind: story
status: implemented
title: 'P1 exit properties: no dangling reference commits; the membrane holds for every reference'
relations:
- decomposes: epic:p1-kernel-ontology-core
- serves: vision:o2
scope:
- confidence: cited
  path: crates/ekr-graph/src/assertion.rs
- confidence: cited
  path: crates/ekr-graph/src/canonical.rs
- confidence: inferred
  path: crates/ekr-graph/src/lib.rs
- confidence: cited
  path: crates/ekr-graph/tests/adversary_p1_06_reference_markers.rs
- confidence: cited
  path: crates/ekr-graph/tests/compile_fail
- confidence: cited
  path: crates/ekr-graph/tests/domain_projection.rs
- confidence: inferred
  path: crates/ekr-kernel/src/document/shape.rs
- confidence: inferred
  path: crates/ekr-kernel/src/explain.rs
- confidence: inferred
  path: crates/ekr-kernel/src/replay.rs
- confidence: inferred
  path: crates/ekr-kernel/src/seed.rs
- confidence: inferred
  path: crates/ekr-kernel/src/transaction.rs
- confidence: inferred
  path: crates/ekr-kernel/src/validate
- confidence: inferred
  path: crates/ekr-kernel/tests
- confidence: inferred
  path: crates/ekr-store/src/log.rs
- confidence: inferred
  path: crates/ekr-store/src/snapshot.rs
revision: 21
---
## Acceptance

Property tests, on both providers through the real kernel authority:

- no transaction whose operations reference an absent node, edge, assertion or evidence commits;
  the refusal is named and nothing is published;
- every canonical reference kind (`Subject::Edge`, `Assertion.evidence`, `CanonicalRef<Evidence>`)
  is typed so a transient identity cannot inhabit it, held by compile-fail cases (AGENTS.md
  invariant 2); today only node references are typed;
- replay from the seed reproduces the head root for generated lineages.

Then a `verification-report` maps each P1 exit clause in `docs/roadmap.md` to the case that
executes it.

## Scope

Derived 2026-09-23 by `story-scoper` (wave p1-14). Folded with `task:canonical-reference-holds-a-node-id-for-every-target` into unit p1-14-exit; the story needs the associated-`Id` design, so the narrow rename option is out.

- `crates/ekr-graph/src/canonical.rs` (`node: NodeId` :204, `resolve` :338), `crates/ekr-graph/src/assertion.rs` (`Subject::Edge(EdgeId)` :37, `Assertion.evidence: BTreeSet<EvidenceId>` :545) — cited
- `crates/ekr-graph/tests/compile_fail/` (4 cases, node only), `crates/ekr-graph/tests/adversary_p1_06_reference_markers.rs` — cited
- `crates/ekr-graph/tests/domain_projection.rs` FUSIONS tokens `"Edge(EdgeId)"` :533, :539, :605 — cited
- new property tests under `crates/ekr-kernel/tests/` using the both-provider harnesses of `validate_properties.rs` / `durable_commands.rs` — inferred
- call-site spread: `crates/ekr-kernel/src/{validate/,seed.rs,transaction.rs,explain.rs,replay.rs,document/shape.rs}`, `crates/ekr-store/src/{snapshot.rs,log.rs}`, `crates/ekr-graph/src/lib.rs`, about 25 test files — inferred
- `systems/ekr/domains/graph.yaml` unchanged while references encode the bare id — inferred
- `docs/roadmap.md:144-146` read for the verification report — cited
- already held: `crates/ekr-kernel/tests/validation.rs::every_kind_of_dangling_reference_is_refused` (hand-written, pipeline only); `crates/ekr-kernel/tests/seed.rs::a_seed_with_a_dangling_edge_is_refused_by_both_backends`; `crates/ekr/tests/retraction_example.rs::the_retraction_example_runs_through_fresh_processes_on_both_providers` (fixed lineage) — cited
- Confidence: medium

## Scope confirmation

Confirmed by the implementor of unit p1-14-exit at close of wave p1-14:

- confirmed: `crates/ekr-kernel/src/{explain.rs,seed.rs,transaction.rs,validate/reference.rs}`, `crates/ekr-store/src/snapshot.rs`, `crates/ekr-graph/src/lib.rs` (doc only); `graph.yaml` unchanged
- wrong: `crates/ekr-kernel/src/document/shape.rs`, `replay.rs`, `crates/ekr-store/src/log.rs` needed no change; `validate/ontology.rs`, `validate/types.rs` match only proposal types
- wrong: the both-provider harness is `durable_commands.rs`, not `validate_properties.rs`; forced test edits were 15 files, not about 25
- not listed: `crates/ekr-graph/src/transient.rs`, `crates/ekr-kernel/src/apply.rs`, `validate/lifecycle.rs`, four ekr-store test files, `crates/ekr/tests/graph_assertion_serde.rs`, `crates/ekr-graph/tests/current_vectors.rs:63` (construction only)
