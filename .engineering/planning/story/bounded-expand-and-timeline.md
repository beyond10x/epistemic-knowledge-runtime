---
format: aep.planning-md/3
id: story:bounded-expand-and-timeline
kind: story
status: draft
title: expand and timeline answer hub nodes in named, paged groups
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
- informed_by: story:bounded-node-detail
revision: 1
---
## Context

After the bounded node detail (`ekr.node-detail/2`), a consumer measured the MCP tools again on the
same store with an agent client: failed tool calls halved and the largest `describe_node` answer on a
hub node fell from 53 KB to 18 KB, but the 7 answers still over the per-result budget all came from
`expand` and `timeline` on hub nodes. Both list everything at a hub: `expand` up to its edge limit,
much of it parallel edges between the same pair; `timeline` every row of the subject's
neighbourhood.

## Build (spec first)

1. `systems/ekr/domains/views.yaml` first, green under `ess specify validate --path systems/ekr` and
   the views conformance suite: new versions of the slice and timeline formats (the next versions
   after those this repository serves when the story starts) shaped as `ekr.node-detail/2` is:
   - `expand`: neighbours grouped by edge type and direction at each hop, each group named with its
     `total` and its first `limit` neighbours by degree, parallel edges between one pair collapsed to
     one entry with their count; a group pages with `edge_type` and `after`, and a group's `next` is
     its next page;
   - `timeline`: rows grouped by row type with `total` and the newest `limit` rows each, paged per
     group the same way;
   - a whole-answer bound stated in the spec, with the marker an answer carries when it is reached.
2. Served byte for byte the same by the MCP tools and `ekr view` (and the session verbs) for the same
   request; the MCP answers the new versions by default, every other surface when asked; the earlier
   versions stay served unchanged.

## Acceptance

- A conformance scenario: a hub node with 120 edges of one type and one of another; `expand` at depth
  1 answers two named groups, `{total 120, limit entries, next}` and `{total 1}`, and names the rare
  neighbour on the first page.
- A conformance scenario: two nodes joined by 50 parallel edges of one type answer one entry with
  count 50.
- A conformance scenario: a subject with rows of three types answers three named groups, newest
  first, each paged by its `next`.
- An MCP test: on a hub fixture both tools answer within the per-result budget, and each answer
  equals `ekr view` with the same format and arguments named.

## Out of scope

Ranking by relevance to a question; merging duplicate entities.
