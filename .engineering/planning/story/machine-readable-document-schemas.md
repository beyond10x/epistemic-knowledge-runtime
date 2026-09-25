---
format: aep.planning-md/1
id: story:machine-readable-document-schemas
kind: story
status: active
title: ekr schema prints JSON Schema for the documents an agent writes
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
scope:
- confidence: cited
  path: Cargo.lock
- confidence: inferred
  path: crates/ekr-graph/src
- confidence: inferred
  path: crates/ekr-kernel/src/seed.rs
- confidence: inferred
  path: crates/ekr-kernel/src/transaction.rs
- confidence: inferred
  path: crates/ekr-ontology/src
- confidence: cited
  path: crates/ekr/Cargo.toml
- confidence: cited
  path: crates/ekr/src/cli
- confidence: cited
  path: crates/ekr/tests/schema_cli.rs
- confidence: cited
  path: docs/cli.md
revision: 12
---
## Context

Operator request (2026-09-25): a machine-readable schema for the documents an agent writes, so a schema author does not depend on prose. Today only printed examples (`ekr example`) and the readers' Rust types describe `ekr-seed/2`, `ekr.transaction-document/1` and `ekr.cli-host/1`.

## Acceptance

- `ekr schema <format>` prints a JSON Schema (draft 2020-12) for each of the three formats, generated from the same Rust types the readers deserialize, so it cannot drift from them.
- A test validates every `ekr example` document, every operation example and every document block in `docs/cli.md` against the printed schema, and holds that each schema refuses a document the reader refuses (a missing required field, an unknown field, a wrong value kind) on at least one case per format.
- `ekr guide`, `ekr --help` and `docs/cli.md` name `ekr schema`.
