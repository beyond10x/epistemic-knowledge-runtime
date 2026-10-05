---
format: aep.planning-md/3
id: story:extracted-relations-visible-to-graph-reads
kind: story
status: draft
title: A relation applied through extraction is visible to graph reads
tags:
- consumer:cortex
revision: 1
---
## Outcome

A relation applied through `ekr apply-extraction` is visible to graph reads: `search` counts it in a
node's `degree`, and `expand` walks it.

## Observed (ekr 0.0.30, 2026-10-05)

A store built only by `apply-extraction` from 8 web documents (consumer: `beyond10x/cortex`):

| read | result |
|---|---|
| `ekr snapshot` | 26 assertions: 25 with a `Relation` predicate, 1 `Property`; `graph.edges` holds 0 entries |
| MCP `search` `{"text":"Qdrant"}` | both `Qdrant` nodes answer `"degree": 0` |
| MCP `expand` `{"seeds":[<node>],"depth":1,"limit":50}` on the node that is the subject of 18 relation assertions | `node_total: 1`, `edge_total: 0` |

So the relations reach canonical state as assertions, and every read that walks the graph misses
them.

## Not established

Why. Two readings fit the observation and neither has been checked against the code: the extraction
path writes a relation as an assertion without creating the `Edge` the graph projection reads
(`crates/ekr-graph/src/canonical.rs:451` holds `edges`), or the read side builds `degree` and
`expand` from `edges` only and ignores `Relation` assertions. Which one decides where the fix goes.

## Acceptance

An extraction document with one `!Relation` fact applies, and afterwards `search` reports `degree` 1
for both ends and `expand` from either end returns the other node and one edge.
