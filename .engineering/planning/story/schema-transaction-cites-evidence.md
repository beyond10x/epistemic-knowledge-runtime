---
format: aep.planning-md/3
id: story:schema-transaction-cites-evidence
kind: story
status: active
title: A schema transaction cites the evidence that introduced it
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o2
- depends_on: story:show-disputed-knowledge
- derived_from: release-plan:knowledge-inbox-schema-learning
scope:
- confidence: inferred
  path: crates/ekr
- confidence: inferred
  path: crates/ekr-kernel
- confidence: inferred
  path: crates/ekr-ontology
- confidence: inferred
  path: crates/ekr-sdk
- confidence: inferred
  path: crates/ekr-views
- confidence: cited
  path: docs/cli.md
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
- confidence: cited
  path: systems/ekr/domains/views.yaml
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T14:55:25Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-10-03T14:55:26Z", actor: "agent:codex-ekr-knowledge", revision: 10, decided_on: {"recorded":{"approval":1}}}
---
## Context

A consumer asked on 2026-10-01 for schema transactions that cite evidence.

The consumer grows its schema one type, edge type or property per version, each just before the
first fact that uses it. It wants each version to cite that fact's message, so that
`ekr ontology --at` and the viewer's schema lineage show which message introduced each type. Today
it keeps that link in its own `apply.json`.

Three rules block this. A transaction's `evidence` must be exactly the set its `AddAssertion`s
cite. `AddEvidence` is a data operation, so carrying it in a schema transaction refuses as
`MixedSchemaTransaction`. And `Transaction` denies unknown fields.

## Build

Let a schema transaction cite evidence the store retains, or carry an `AddEvidence`, without counting
as mixed. The version records the cited evidence, and the schema reads (`ekr ontology --at`, the
projection's schema lineage) show it. This is a schema-evolution change: it is specified in ESS
first, and its design section sits beside the schema-evolution sections.

## Acceptance

- A schema transaction that cites one retained evidence entry, and one that adds and cites one,
  each commit as a new schema version on both providers.
- `ekr ontology --at <that revision>` and the lineage name the evidence.
- A schema transaction that carries any other data operation is still refused as
  `MixedSchemaTransaction`.

## Knowledge inbox delivery scope

Reuse this story as D of release-plan:knowledge-inbox-schema-learning; do not replace its original consumer request. It follows story:show-disputed-knowledge's authority/evidence foundations and precedes approved proposal integration. Source contract commit 07604bf1a5e6c19ef86b78846dc99a603f3d2bfb, spec_digest 29e2f5d0af0bb99248bb46d0dc1f538dedab7d5d209ca07c0fce5d0ac60a1a4e, validates before implementation.

Named scenario schema_change_exposes_supporting_evidence runs against both file and SQLite, including reopen/full replay. Keep mixed schema/data refusals except explicitly admitted evidence operations. Expose supporting evidence through typed SDK, ontology CLI and read-only viewer schema history. Use generated contracts, preserve canonical evidence retention and ordinary validation; full task check plus real conformance closes this story.

## Scope

Scoper source review: ekr-sdk/src/document/transaction.rs:60,115–128 currently derives assertion/attachment evidence and refuses mixed schema transactions; ekr-kernel/src/validate/provenance.rs:98 and reference.rs:25 admit evidence; read.rs:93 schema history lacks per-version evidence links. Inferred implementation: kernel admission/replay under new profile, schema metadata, SDK supporting-evidence builder, cli/ontology.rs:52, ekr-views/src/document.rs:238,499 and index.rs:356. Extend schema_evolution.rs:382, schema_evolution_replay.rs:455 and add_evidence.rs:204,436. Preserve old-profile refusals and all other mixed schema/data refusals.


## Kernel checkpoint in progress

Kernel authority/evidence commit dded81ab56351b2b705d62dd2d6b34732856780a and SDK builder commit 24002eeca1ca487671dde6c4146d33600c514a69 are retained on managed unit branch ekr/schema-evidence-20261003. Native compatibility fixtures were written by the previous implementation and upgraded by the new one; the original Gates hook admitted them after the operator's exact fixture-exception approval.

Presentation work now derives schema citations from verified immutable manifests through the ESS-generated SchemaEvidenceEntry declared in systems/ekr/domains/views.yaml. The ontology CLI, SDK typed reads, graph projection, overview and Rust-rendered inbox expose those records. The focused cli::ontology::tests::schema_change_exposes_supporting_evidence case passes after normal and full replay on both providers, including historical selection and HTML escaping. Broader regressions are in progress; this is not yet authored ESS conformance.

Independent review-result:schema-evidence-kernel-independent-r1 found a kernel test whose manifest mismatch masked removal of the mixed-data guard. The unit now clears that manifest and requires the named mixed-schema-transaction issue; execution and review disposition remain pending. The named ESS scenario, review closure, viewer inspection and complete integrated gate remain required. This partial checkpoint does not complete story D or PR64.

Attribution correction: the earlier draft-to-proposed transition attributed to human:timo was issued by the coordinating agent without setting AEP_ACTOR. It relied on the existing operator execution request and was not a separate human action. The immutable transition is preserved; subsequent writes explicitly identify agent:codex-ekr-knowledge.
