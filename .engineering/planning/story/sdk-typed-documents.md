---
format: aep.planning-md/3
id: story:sdk-typed-documents
kind: story
status: active
title: The SDK builds every document a consumer writes and names the ontology by name
relations:
- serves: vision:o5
- decomposes: epic:consumer-sdk
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: crates/ekr-sdk/Cargo.toml
- confidence: inferred
  path: crates/ekr-sdk/src/document
- confidence: inferred
  path: crates/ekr-sdk/src/lib.rs
- confidence: inferred
  path: crates/ekr-sdk/tests/document_drift.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T15:13:55Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-09-29T15:13:55Z", actor: "human:timo", revision: 9}
---
## Context

A consumer instance hand-writes a tag-aware YAML emitter (86 lines) because the transaction
readers refuse the one-key JSON form of a tag (`docs/cli.md`), builds `ekr-seed/2` and schema
operations from name→id maps (~400 lines), and sends `mint` and `hash` requests through the
session. `ekr-core` already owns `Id::mint()` and the payload hash.

## Build

- Serde models and builders for `ekr.transaction-document/2` (every operation kind), `ekr-seed/2`
  and the typed reference.
- A YAML writer that emits `!Tag` values.
- Local minting and hashing through `ekr-core`; `AGENTS.md` (the rule that ids come from
  `ekr mint`) amended to name `ekr-core`'s `Id::mint()` as the same function.
- `OntologySpec` (types, properties with value kinds, edge endpoints, by name) → a seed plus an
  `Ontology` name→id map. It is built from the ontology operations that exist today
  (`DefineNodeType`, `DefineEdgeType`, `ModifyProperty`) and adds no noun. The extraction document's
  ontology section (`story:extraction-document-applies-to-a-store`) is written to match it later;
  this story does not wait for that format (coordinator, 2026-09-29).
- `Ontology::read` from `ekr ontology`, so a later run needs no reseed.
- `Ontology::ensure(spec)` emits the missing `DefineNodeType`, `DefineEdgeType` and
  `ModifyProperty` operations under profile v2 or v3.

## Surface (inferred)

`crates/ekr-sdk/src/document/{mod,yaml,transaction,seed,reference,ontology}.rs`,
`crates/ekr-sdk/tests/document_drift.rs` (dev-depends on `ekr-kernel` to parse every output with
the real readers), `AGENTS.md`.

## Acceptance

- Every builder output is accepted by the kernel's document reader and validates against
  `ekr schema <format>`.
- A test fails when `ekr operations` lists a kind that has no builder.
- A locally computed hash equals `ekr hash` for the same bytes, including a trailing newline.
- `ensure` against a store whose ontology already matches emits no operation.
- A fixture ingest sends zero `mint` and zero `hash` requests.
