---
format: aep.planning-md/2
id: story:ekr-resolve-verb
kind: story
status: active
title: 'ekr resolve: an agent resolves a typed reference before it proposes a node'
relations:
- depends_on: story:typed-reference-resolver
- decomposes: epic:p3-incubation-integration
- serves: vision:o5
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: cited
  path: crates/ekr/Cargo.toml
- confidence: cited
  path: crates/ekr/src/cli/agent.rs
- confidence: inferred
  path: crates/ekr/src/cli/examples/typed-reference.yaml
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: cited
  path: crates/ekr/src/cli/resolve.rs
- confidence: inferred
  path: crates/ekr/src/cli/schema.rs
- confidence: cited
  path: crates/ekr/tests/agent_cli.rs
- confidence: cited
  path: crates/ekr/tests/docs_cli.rs
- confidence: inferred
  path: crates/ekr/tests/schema_cli.rs
- confidence: cited
  path: docs/cli.md
revision: 16
---
## Context

An agent writes through `ekr propose`, `ekr validate` and `ekr commit` and takes ids from `ekr mint`, `ekr ontology` and `ekr snapshot` (AGENTS.md, "Agents that use the ekr binary"). Today its only way to find an existing node is to read `ekr snapshot` and match by eye or by name, which is the name matching the resolver replaces. This story puts `story:typed-reference-resolver` behind a read verb.

- `ekr resolve <reference.yaml> [--at <revision>]` reads a typed-reference document and prints the `ResolutionOutcome` as one JSON document (exit 0), or a named refusal (exit 2, its name on stderr). It opens only an existing store and writes nothing.
- `ekr schema` prints the typed-reference JSON Schema; `ekr example typed-reference` prints one.
- `ekr guide` tells an agent to resolve before `CreateNode`, and that `ProposeNew` means "mint an id and create", not "retry with a looser reference".
- AGENTS.md requires `docs/cli.md` to change with the verb; `crates/ekr/tests/docs_cli.rs` runs the worked example on both providers.

## Design sections

§ 45, § 63 step 3, § 70; A10.

## Domain relations

- `Node → NodeType`, many-to-one, a reference. Inferable (`systems/ekr/domains/graph.yaml`, `ekr.graph.Node` relation `type`, via `type_id`). The verb adds no relation of its own; the rest are those of `story:typed-reference-resolver`.

## Scope

Derived 2026-09-27 by `story-scoper`. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- `crates/ekr/src/cli/resolve.rs` — cited
- `crates/ekr/src/cli/mod.rs` — cited
- `crates/ekr/src/cli/agent.rs` — cited
- `crates/ekr/Cargo.toml` — cited
- `docs/cli.md` — cited
- `crates/ekr/tests/docs_cli.rs` — cited
- `crates/ekr/tests/agent_cli.rs` — cited
- `crates/ekr/src/cli/schema.rs` — inferred
- `crates/ekr/src/cli/examples/typed-reference.yaml` — inferred
- `crates/ekr/tests/schema_cli.rs` — inferred
- `Cargo.lock` — inferred
- `CHANGELOG.md` — inferred
- **Would collide with:** any unit changing an `ekr` subcommand, example or schema format, or `docs/cli.md`
- **Confidence:** high for the paths the story names; medium for the inferred lines

## Acceptance

`ekr resolve` given a typed-reference document prints its resolution as one JSON document and leaves the store byte-identical, and the `docs/cli.md` worked example that runs it passes on both providers.
