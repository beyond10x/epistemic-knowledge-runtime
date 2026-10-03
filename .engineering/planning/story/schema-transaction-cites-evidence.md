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
revision: 13
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
