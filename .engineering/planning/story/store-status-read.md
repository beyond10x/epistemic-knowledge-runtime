---
format: aep.planning-md/3
id: story:store-status-read
kind: story
status: draft
title: An agent reads how far the store's knowledge reaches and how current it is, in one cheap call
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
revision: 1
---
## Context

A consumer running agents against a store over the MCP asked for one cheap answer to "how far does
this store's knowledge reach, and how current is it". Two agents derived the knowledge cutoff two
ways from `overview` with `sections: ["timeline"]` and got different dates, and an agent reads "no
results" as a negative when the store simply ends earlier than the question.

Requested:

1. A `status` tool that does no graph traversal, answering: the knowledge window (the minimum and
   maximum valid time of the revision's assertions), overall and per evidence source kind; the head
   revision and when it was recorded; when the store was built (seeded); counts of nodes, edges,
   assertions and evidence.
2. The same facts in the MCP `initialize` answer (the `instructions` text or the server info), so a
   client sees them without a call.
3. An optional operator-supplied metadata document (a file-path option such as
   `--instance-metadata <file>`), merged verbatim into `status` and `initialize`: sources included
   and excluded, last ingest per source, whether ingestion is paused, rate-limit hints, a contact.
   The runtime passes it through, checks that it is JSON, and caps its size.

## Build (spec first)

1. `systems/ekr/domains/views.yaml` first, green under `ess specify validate --path systems/ekr` and
   the views conformance suite before any code: a new read and its versioned format (for example
   `ekr.store-status/1`), its determinism rules, and how the operator document is carried (opaque
   JSON, size bound, named refusal when it is not JSON or over the bound, checked at start-up).
2. Served byte for byte the same by the MCP `status` tool and `ekr view` (and the session verb), as
   every other read; the `initialize` answer carries the same facts.
3. The cost is counted, not timed: answering `status` reads no node's neighbourhood; the window and
   counts come from what the index already holds, or are kept per revision.

## Acceptance

- A conformance scenario: on a seeded store with assertions at two valid times and two evidence
  source kinds, `status` answers the window overall and per kind, the head revision with its
  recorded time, the seed time, and the four counts.
- `ekr view status` and the MCP `status` tool answer the same bytes for the same store state.
- `initialize` on a store with an operator document names the window and carries the document.
- An operator document that is not JSON, or over the bound, is refused at start-up with its named
  refusal; with no document, `status` answers without the field.
- A test counts that `status` performs no neighbourhood expansion.

## Out of scope

Deriving source health from observations; anything the operator document says is passed through,
not interpreted.
