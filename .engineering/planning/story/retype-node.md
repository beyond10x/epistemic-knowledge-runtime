---
format: aep.planning-md/3
id: story:retype-node
kind: story
status: draft
title: RetypeNode moves a node to a conforming or explicitly named type and keeps its id
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- depends_on: story:merge-entity-applies
revision: 2
---
## Context

The operator decided on 2026-10-17 that a node may be retyped by its own operation, after
`MergeEntity`, to a conforming type or an explicitly named target type, with its own decision
record: `architecture-decision-record:0016-retype-keeps-node-identity` (proposed). The merge
story refuses a merge across unrelated types and leaves the fix to "an explicit retype or
retraction"; this story builds the retype. Today a mistyped node can only be retracted and
recreated under a new id.

## Build (spec first)

1. `systems/ekr/domains/kernel.yaml` and `graph.yaml` first, green under `ess specify validate
   --path systems/ekr` (the pinned `ess` and the newest) before any code: the `RetypeNode`
   operation (`node`, `to`, `unrelated`, `reason`), the node's type-history record, the validation
   rules and named refusals (node not present, node absorbed, same type, unrelated target without
   `unrelated`, a property `to` does not declare, an edge whose endpoint type no longer conforms),
   and how reads at earlier revisions answer the earlier type.
2. `ekr-kernel` validates and applies `RetypeNode`; replay applies it identically; both providers.
3. Reads and views answer the node's current type at the head and its type then at earlier
   revisions; search counts the node under its current type.
4. `docs/cli.md` (the operation, its refusals and a worked example), `CHANGELOG.md`.

## Acceptance

Conformance scenarios and kernel tests, on both providers:

- `a-retype-to-a-parent-type-keeps-the-node-id`.
- `a-retype-to-an-unrelated-type-is-refused-unless-named-unrelated`.
- `a-retype-is-refused-while-the-node-holds-an-undeclared-property`; after the retraction it
  commits.
- `a-retype-is-refused-when-an-edge-no-longer-conforms`, naming the edge.
- `a-retype-of-an-absorbed-node-is-refused`.
- `a-read-at-an-earlier-revision-answers-the-earlier-type`; replay from the seed gives the same
  roots.

## Not in scope

Merges (`story:merge-entity-applies`), splits, bulk retype by predicate, ontology changes to the
types themselves.
