---
format: aep.planning-md/3
id: story:schema-transaction-cites-evidence
kind: story
status: draft
title: A schema transaction cites the evidence that introduced it
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o2
revision: 1
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
