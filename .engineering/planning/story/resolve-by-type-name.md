---
format: aep.planning-md/3
id: story:resolve-by-type-name
kind: story
status: draft
title: A typed reference names its type by name as well as by id
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
revision: 1
---
## Context

The `resolve` tool takes a typed reference, `type_id` (a UUID) and `aliases` (`crates/ekr/src/cli/mcp.rs`,
tool schema at tag 0.0.30). The type UUIDs are listed in `overview`'s ontology. An agent that knows only
the type's name therefore has to read the overview before it can resolve anything.

## Build

- A typed reference names its type by `type_id` or by `type_name`, exactly one of the two.
- A name matching more than one concrete type is refused, listing every matching id. A name matching an
  abstract type, or one with a subtype, is refused like the same `type_id` is today.
- `ekr resolve`, the MCP tool and the HTTP operation take the same reference; `ekr example typed-reference`
  shows both forms.

## Acceptance

- Resolving by a type's name answers byte-identically to resolving by its id, for every outcome
  (`Resolved`, `ProposeNew`, `Ambiguous`).
- Both keys, or neither, is refused before the store is read.
- A name two types share is refused, naming both ids.
