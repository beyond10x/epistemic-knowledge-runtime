---
format: aep.planning-md/2
id: story:cli-user-documentation
kind: story
status: implemented
title: Document the ekr CLI and the seed schema for users and agents
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
- confidence: inferred
  path: b10x.docs.yaml
- confidence: cited
  path: crates/ekr/tests/docs_cli.rs
- confidence: cited
  path: docs/cli.md
revision: 9
---
## Context

Operator request (2026-09-25): document the `ekr` CLI for real users, so that an independent agent given only this repository's documentation and a built `ekr` binary can author a reasonable schema (the `ontology:` section of an `ekr-seed/2` document) for a use case it is given, seed it, and record knowledge against it.

## Acceptance

- `docs/cli.md` describes the CLI end to end for a user and an agent: install and configuration (flags and `EKR_*`), every verb with its input format, output and exit codes, the propose → validate → commit workflow, and the `ekr-seed/2` format field by field, with the ontology section (node types, edge types, properties and value types, cardinality, lifecycles, operations) in enough detail to write a new schema without reading source.
- It states the P1 limits plainly: a schema is declared only in the seed; `DefineNodeType`, `DefineEdgeType`, `ModifyProperty` and `MergeEntity` are refused as `unsupported-operation`.
- A worked example builds a small schema for a use case other than the example seed's, seeds it, and commits an assertion.
- Every `ekr-seed/2` and `ekr.transaction-document/1` block in `docs/cli.md` is parsed by the real readers in a test, and the worked example seeds and commits on both providers, so the page cannot drift from the binary.
- `README.md` is rewritten for users (what it is, status, install, first run, where to read next); `AGENTS.md` points agents at `docs/cli.md` and `ekr guide`; neither duplicates the other.
