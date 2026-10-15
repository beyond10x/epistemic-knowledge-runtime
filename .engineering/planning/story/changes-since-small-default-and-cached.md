---
format: aep.planning-md/3
id: story:changes-since-small-default-and-cached
kind: story
status: active
title: MCP reads default to small pages, and a revision's changes are parsed once
summary: 'MCP expand and changes_since defaults, and a per-revision change cache counted by parses (ekr-mcp-fixes-20261015 #4, #5).'
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
- depends_on: story:names-inline-in-read-answers
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/ekr-views/src/changes.rs
- confidence: inferred
  path: crates/ekr-views/src/index.rs
- confidence: inferred
  path: crates/ekr-views/src/lib.rs
- confidence: inferred
  path: crates/ekr/src/cli/mcp.rs
- confidence: inferred
  path: crates/ekr/tests/mcp.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T13:21:16Z", actor: "agent:ekr-controller-planner", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-15T13:21:16Z", actor: "agent:ekr-controller-planner", revision: 4}
---
## Context

A consumer's measured patch (ekr-mcp-fixes-20261015, items 4 and 5), measured on 0.0.34:

- #4: the MCP `expand` default edge limit is 5,000: depth 1 around a node of degree 494 answered 996
  edges and 439 KB. The `changes_since` default limit of 500 answered 132 KB to 276 KB. The budget
  per tool result is 75 KB.
- #5: every `changes_since` call parses each chosen revision's transaction document again, about
  40 ms a revision on the measured store: 290-400 ms for `since_revision` = head-7, 22.9 s and 25.4 s
  for a `since_recorded` about 3.5 weeks back, repeated. 3 of 28 agent trials timed out.

The MCP answer stays byte for byte what `ekr view` (or the session verb) answers for the same request.
The MCP's smaller defaults keep that rule: the identity holds against `ekr view` given the same
explicit limit, and the spec says so.

## Build (spec first)

1. **`systems/ekr/domains/views.yaml` first**, green before any code:
   - the per-surface defaults (#4): the MCP `expand` takes `edge_limit` 200 and `changes_since`
     `limit` 100 when the request names none; `ekr view` and the session verb keep 5,000 and 500.
     The spec states that the MCP answer equals `ekr view`'s for the same request with that limit
     named explicitly, and that a page ends at whichever of its limits comes first;
   - the cost clause of ChangesSince (#5): a revision's change list is derived from its transaction
     document once per loaded index, and a second request over the same revisions parses no
     transaction document. A cost is stated as work counted, not as time.
2. `crates/ekr-views/src/changes.rs`, `index.rs`: the per-revision change cache, dropped with the
   index it belongs to, and a parse counter the tests read (the way `reader_work()` in
   `crates/ekr/src/cli/session.rs` counts store opens).
3. `crates/ekr/src/cli/mcp.rs`: the two defaults; the `describe_node` and `expand` descriptions say
   how to ask for the next page.
4. `docs/cli.md`, `CHANGELOG.md`.

## Acceptance

Each case names "ekr-mcp-fixes-20261015 #4" or "#5". No case passes or fails on wall-clock time.

- `crates/ekr/tests/mcp.rs`, `changes_since_from_revision_zero_answers_within_the_tool_budget` (#4):
  on a store of 250 assertions, MCP `changes_since` from revision 0 with no limit answers under 75 KB
  and exactly `ekr view` `/changes` with `limit=100`; on 0.0.34 it answered 125 KB.
- `crates/ekr/tests/mcp.rs`, `expand_defaults_to_two_hundred_edges` (#4): around a node with more than
  200 incident edges, MCP `expand` with no limit answers 200 edges and a `next`, byte for byte
  `ekr view` `/expand` with `edge_limit=200`.
- `crates/ekr-views/tests`, `a_second_changes_request_parses_no_transaction_document` (#5): two
  identical `since_recorded` requests on one index; the counter rises by the number of chosen
  revisions on the first and by 0 on the second, and both answers are byte-identical. A request after
  a new commit parses only the new revision.
- Conformance: every admitted changes scenario passes unchanged (the cache changes no answer).
- `crates/ekr/tests/mcp.rs`, moved, not deleted: `changes_since_answers_and_refuses_exactly_as_get_changes_does`
  and `an_add_evidence_is_listed_once_by_changes_since_get_changes_and_the_session_verb` compare the
  MCP answer with `ekr view` and the session verb given `limit=100`; `expand` in
  `each_tool_answers_the_document_its_read_returns_on_the_same_store_state` given `edge_limit=200`.

## Scope

- `systems/ekr/domains/views.yaml`
- `crates/ekr-views/src/changes.rs`, `index.rs`, `lib.rs`
- `crates/ekr/src/cli/mcp.rs`
- `crates/ekr/tests/mcp.rs`, a new or existing `crates/ekr-views/tests` file for the counter
- `docs/cli.md`, `CHANGELOG.md`

## Overlaps and order

Shares `views.yaml`, `mcp.rs`, `crates/ekr-views/src/{lib,index}.rs` and `crates/ekr/tests/mcp.rs`
with the other stories; `changes.rs` with `story:names-inline-in-read-answers`, after which it lands
(that story moves `changes_since` to `ekr.graph-changes/2` first). Also near
`story:changes-since-by-time-within-deadline` (active, `changes.rs`): this story caches the parsed
change list and does not take that story's deadline.

## Out of scope

The cold cost (about 20 s over hundreds of revisions on first read): persisting change lists with the
replay checkpoint, or deriving changes from the index, is a separate story. The store open on a cold
host.
