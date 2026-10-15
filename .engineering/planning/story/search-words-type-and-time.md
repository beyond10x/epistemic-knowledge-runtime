---
format: aep.planning-md/3
id: story:search-words-type-and-time
kind: story
status: active
title: 'search answers ekr.node-matches/2: every word, a whole name first, by type and time'
summary: 'Word matching, type and time filters and a query alias (ekr-mcp-fixes-20261015 #2, #6, #9).'
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
- depends_on: story:bounded-node-detail
scope:
- confidence: inferred
  path: CHANGELOG.md
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
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T13:21:15Z", actor: "agent:ekr-controller-planner", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-15T13:21:16Z", actor: "agent:ekr-controller-planner", revision: 4}
---
## Context

A consumer's measured patch (ekr-mcp-fixes-20261015, items 2, 6 and 9), measured on 0.0.34:

- #2: the search text must be one substring of a name or alias. "build runner" finds nothing beside
  "Build-Runner", and "disk 507" nothing beside "…-disk-507". Within a tier the order is degree, so a
  node named exactly "west" ranks below a higher-degree "prod-west-…". 8 of 28 agent trials were hurt.
- #6: an agent called `search` with `query` instead of `text` and was answered -32602.
- #9: no read lists the nodes of one type or of one period; 5 of 28 trials were hurt.

The patch is a proposal; this repository owns the format. Every MCP answer stays byte for byte what
`ekr view` (or the session verb) answers for the same request.

## Build (spec first)

1. **`systems/ekr/domains/views.yaml` first**, green under `ess specify validate --path systems/ekr`
   and the views conformance suite before any code:
   - a new format `ekr.node-matches/2` (`ekr.views.NodeMatchesV2`, its format variant and a
     determinism rule (1) entry), served by the existing SearchNodes command through a format
     selector; `ekr.node-matches/1` stays declared and served unchanged;
   - a third `ekr.views.MatchTier` variant for /2, `Words` (#2): for a text of two or more
     whitespace-separated words, not Exact or Folded, every word under the Folded lowercase mapping
     is a substring of the name or of one alias, in any order. A one-word text has only Exact and
     Folded, as today. Words are split on whitespace only, never on punctuation (the patch's earlier
     punctuation split broke the admitted no-match scenario and the `İ` case);
   - /2's order: tier, then a match whose whole name or whole alias equals the text (under the tier's
     comparison) before one that only contains it, then degree descending, then id; with `from` or
     `to`, newest first on the node's timeline time before degree. The spec writes the order out;
   - every /2 match carries `type_name` (#2);
   - the inputs `type` (a node type id or name) and `from`/`to` (epoch milliseconds or `YYYY-MM-DD`,
     a date naming its whole UTC day), filtering on the node's timeline time as
     `ekr.graph-timeline/1` defines it (#9); an empty text with `type` lists that type's nodes; a
     type name the ontology does not hold is a named refusal;
   - the per-surface default: the MCP `search` answers /2 unless asked for /1; `ekr view` `/search`
     and the session verb answer /1 unless asked for /2; the spec states that the MCP answer equals
     `ekr view`'s for the same request with the format named explicitly;
   - the MCP binding note that `text` is also accepted as `query` and `q` (#6), an argument alias
     that changes no answer.
2. `crates/ekr-views`: the tier, the order, the filters, `type_name`.
3. `crates/ekr`: the selector and the new inputs on `ekr view` `/search`, the session verb and the MCP
   tool; the aliases and the tool's input schema naming them.
4. `docs/cli.md` and `CHANGELOG.md`.

## Acceptance

Each case names the item it carries as "ekr-mcp-fixes-20261015 #N".

Conformance scenarios:

- `a-search-v2-matches-every-word-in-any-order` (#2): "widget router" finds the node named
  "Widget-Router" in the first 3 matches with tier `Words`; under /1 the same text finds nothing.
- `a-search-v2-ranks-a-whole-name-first-within-its-tier` (#2): "eu" answers the node named "eu" before
  a higher-degree "prod-eu-central-9".
- `a-search-v2-lists-one-type-by-degree` (#9): empty text with `type: "Incident"` (by name, and again
  by id) lists exactly the store's Incident nodes, each with `type_name` "Incident".
- `a-search-v2-filters-on-timeline-time` (#9): `from`/`to` as a date and as milliseconds admit the
  node whose timeline time falls in the window and no other.
- The existing `a-search-with-no-match-answers-an-empty-list` and the `İ` case keep passing under /2.

Tests:

- `crates/ekr/tests/mcp.rs`: `search {"query": "Ada"}` and `search {"q": "Ada"}` answer what
  `search {"text": "Ada"}` answers (#6); on 0.0.34 the first is -32602.
- `crates/ekr-views/tests/query.rs`:
  `exact_matches_come_before_folded_ones_whatever_their_degree` is moved to `ekr.node-matches/2`
  (it gains `type_name`), with a /1 copy kept that holds /1's bytes unchanged.
- `crates/ekr/tests/mcp.rs`, moved to /2, not deleted: `search` in
  `each_tool_answers_the_document_its_read_returns_on_the_same_store_state` and in
  `a_hundred_searches_and_node_descriptions_over_one_server_are_timed`, each compared byte for byte
  with `ekr view` asked for /2 explicitly.
- The search adversary (`crates/ekr-views/tests`, five cases) passes on /1 and /2.

## Scope

- `systems/ekr/domains/views.yaml`; `crates/ekr-views/tests/fixtures/conformance/scenarios`
- `crates/ekr-views/src/lib.rs`, `index.rs`, `query.rs`
- `crates/ekr/src/cli/mcp.rs`, `view.rs`, `session/views.rs`
- `crates/ekr/tests/mcp.rs`, `crates/ekr-views/tests/query.rs`
- `docs/cli.md`, `CHANGELOG.md`

## Overlaps and order

Shares `views.yaml`, `crates/ekr/src/cli/mcp.rs`, `crates/ekr-views/src/{lib,index,query}.rs`,
`crates/ekr/tests/mcp.rs`, `crates/ekr-views/tests/query.rs`, `docs/cli.md` and `CHANGELOG.md` with
the other five stories of the wave. Lands after `story:bounded-node-detail`, whose /2 refusal points
at `search` with `type`.

## Out of scope

Fuzzy (edit-distance) matching; search over property values; the SDK's typed `NodeMatches`, which
keeps reading /1 (its `MatchTier` already reads an unknown tier as `Other`).
