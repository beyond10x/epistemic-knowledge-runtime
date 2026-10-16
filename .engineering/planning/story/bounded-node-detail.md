---
format: aep.planning-md/3
id: story:bounded-node-detail
kind: story
status: implemented
title: describe_node answers a bounded, named ekr.node-detail/2
summary: 'Bounded node detail, same-name siblings and a refusal that names a type (ekr-mcp-fixes-20261015 #1, #10, #11).'
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/ekr-views/src/brief.rs
- confidence: inferred
  path: crates/ekr-views/src/index.rs
- confidence: inferred
  path: crates/ekr-views/src/lib.rs
- confidence: inferred
  path: crates/ekr-views/src/query.rs
- confidence: inferred
  path: crates/ekr-views/tests/fixtures/conformance/scenarios
- confidence: inferred
  path: crates/ekr-views/tests/query.rs
- confidence: inferred
  path: crates/ekr/src/cli/mcp.rs
- confidence: inferred
  path: crates/ekr/src/cli/session/views.rs
- confidence: inferred
  path: crates/ekr/src/cli/view.rs
- confidence: inferred
  path: crates/ekr/tests/mcp.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T13:21:15Z", actor: "agent:ekr-controller-planner", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-15T13:21:15Z", actor: "agent:ekr-controller-planner", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-16T16:00:38Z", actor: "agent:claude-ekr-controller", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Context

A consumer's measured patch (ekr-mcp-fixes-20261015, items 1, 10 and 11), measured on 0.0.34 against
a store of 21,913 nodes and 34,626 edges with a 75 KB budget per tool result:

- #1: `ekr.node-detail/1` lists every assertion about or naming the node and every edge at it, and
  does not page. Measured: 2.77 MB at degree 2,793, 1.23 MB at degree 494, 74 KB for an ordinary
  node; 13 of 22 `describe_node` calls exceeded the budget, and a rare relation is lost among
  hundreds of frequent ones.
- #10: `describe_node` on a node type or edge type id answers only the node-not-found refusal, so an
  agent holding a type id is not told what it holds.
- #11: one thing recorded as several same-name nodes of different types; the fact the agent needs
  sits on the sibling, and the detail does not show the sibling exists.

The patch is a proposal; this repository owns the format. The rule that every MCP answer is byte
for byte what `ekr view` (or the session verb) answers for the same request stays.

## Build (spec first)

1. **`systems/ekr/domains/views.yaml` first**, green under `ess specify validate --path systems/ekr`
   and the views conformance suite before any code:
   - a new format `ekr.node-detail/2` (`ekr.views.NodeDetailV2` and the types it needs, its
     `NodeDetailFormat` variant, and a determinism rule (1) entry for it), served by the existing
     DescribeNode command through a format selector; `ekr.node-detail/1` stays declared and served
     unchanged;
   - its content (#1): the node record with `type_name`; active property values grouped by property,
     each group named, newest first, at most `limit` (default 10) each; a string value over 2,000
     bytes cut at a UTF-8 boundary and marked `truncated`; neighbours grouped by edge type and
     direction, each group named, with its `total` and its first `limit` neighbours by degree
     descending then id, each with the id of its newest active relation assertion and how many there
     are; `edge_type` (id or name) and `after` page one group, and a group's `next` is its next page;
   - `same_name` (#11): up to `limit` other nodes whose canonical name or an alias equals this node's
     name or an alias, compared under the same lowercase mapping as `ekr.views.MatchTier` Folded,
     each with `type_name` and degree, ordered by degree descending then id;
   - the inputs `limit`, `edge_type` and `after` with their bounds, checked before the store is read
     as the existing bounds rule says;
   - a named refusal (#10) for a /2 request whose id is a node type or edge type of the revision's
     ontology, whose message names the type and points at `overview` and `search` (the patch's text:
     "<id> is the type <Name>, not a node: overview lists the types; search (with type) lists its
     nodes"). A /1 request keeps answering node-not-found.
   - the per-surface default: the MCP `describe_node` answers /2 unless asked for /1; `ekr view`
     `/node/<id>` and the session verb answer /1 unless asked for /2. The spec states that the MCP
     answer equals `ekr view`'s for the same request with the format named explicitly.
2. `crates/ekr-views`: the /2 document from the index (the patch's `Index::brief`, `brief.rs`), the
   same-name lookup, the refusal.
3. `crates/ekr`: the selector on `ekr view` `/node/<id>`, the session verb and the MCP tool; the MCP
   tool description says it answers /2 and how to page a group.
4. `docs/cli.md` (held to the binary by `crates/ekr/tests/docs_cli.rs`) and `CHANGELOG.md`.

## Acceptance

Each case names the item it carries as "ekr-mcp-fixes-20261015 #N" in its name or a comment.

Conformance scenarios (`crates/ekr-views/tests/fixtures/conformance/scenarios`):

- `a-node-detail-v2-groups-neighbours-by-edge-type-and-pages-a-group` (#1): a node with 120 incoming
  edges of one type and one incoming edge of another. The /2 answer has the groups
  `{type A, in, total 120, 10 neighbours, next}` and `{type B, in, total 1}`, names the second
  group's neighbour, and is under 75 KB; requesting the first group with its `next` answers the next
  10. After the fix it asserts the bound; on 0.0.34 there is no /2 and the /1 answer is 108 KB.
- `a-node-detail-v2-lists-same-name-nodes` (#11): a node of type S and a node of type C both named
  "Widget-Router" (one by alias, case differing). S's /2 detail lists C under `same_name` with its
  `type_name` and degree.
- `a-node-detail-v2-on-a-type-id-names-the-type` (#10): describe with a node type id and with an edge
  type id is refused by the new name, and the message carries "is the type" and "overview"; the same
  id under /1 is node-not-found as before.
- `a-node-detail-v2-cuts-a-long-string-and-marks-it` (#1): a 5,000-byte property value is answered
  cut to at most 2,000 bytes on a character boundary with `truncated` true.

Tests:

- `crates/ekr-views/tests`: a /1 request answers the same bytes as before this story (the /1 golden
  bytes do not change).
- `crates/ekr/tests/mcp.rs`, moved to the new format, not deleted: `describe_node` in
  `each_tool_answers_the_document_its_read_returns_on_the_same_store_state`,
  `the_library_seam_serves_mcp_in_process`,
  `a_running_server_answers_from_a_sqlite_database_copied_over_its_file`,
  `a_running_server_answers_from_a_store_replaced_by_rename`,
  `a_commit_by_another_process_is_read_by_the_next_tool_call` (its `describe_node` part),
  `a_hundred_searches_and_node_descriptions_over_one_server_are_timed` (its `describe_node` part) and
  `a_refusal_is_a_tool_error_with_its_name_and_the_server_keeps_serving` (its `describe_node` part).
  Each compares the MCP answer with `ekr view` (or the session verb) asked for
  `ekr.node-detail/2` explicitly, byte for byte, and one case still compares an explicit /1 request.

## Scope

- `systems/ekr/domains/views.yaml`; `crates/ekr-views/tests/fixtures/conformance/scenarios`
- `crates/ekr-views/src/lib.rs`, `index.rs`, `query.rs`; a new module for the /2 document (the patch's
  `brief.rs`)
- `crates/ekr/src/cli/mcp.rs`, `view.rs`, `session/views.rs`
- `crates/ekr/tests/mcp.rs`; `crates/ekr-views/tests/query.rs` (new /2 cases)
- `docs/cli.md`, `CHANGELOG.md`

## Overlaps and order

Shares `views.yaml`, `crates/ekr/src/cli/mcp.rs`, `crates/ekr-views/src/{lib,index,query}.rs`,
`crates/ekr/tests/mcp.rs`, `docs/cli.md` and `CHANGELOG.md` with every other story of this wave
(`story:search-words-type-and-time`, `story:names-inline-in-read-answers`,
`story:changes-since-small-default-and-cached`, `story:explain-scoped-to-cited-evidence`,
`story:mcp-aliases-and-unqueued-handshake`). It lands first: the alias story maps `id` onto `node`
and `edge` onto this story's `edge_type`.

`story:describe-type-read` (draft) answers a type id with its declaration; this story only refuses
one by name, and does not take that read.

## Out of scope

Merging same-name nodes or recording them as the same thing (extraction owns that data fix); fuzzy
matching; the SDK's typed reads (`crates/ekr-sdk/src/read/views.rs`), which keep reading /1.
