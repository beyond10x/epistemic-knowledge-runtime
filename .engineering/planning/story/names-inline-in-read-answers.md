---
format: aep.planning-md/3
id: story:names-inline-in-read-answers
kind: story
status: active
title: Read answers carry type and property names in new format versions
summary: 'type_name on slice and overview, a names map on timeline, changes, explain and resolve, committed_at in overview meta (ekr-mcp-fixes-20261015 #3, #13).'
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
- depends_on: story:search-words-type-and-time
- informed_by: story:read-answers-name-types
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
  path: crates/ekr-views/src/query.rs
- confidence: inferred
  path: crates/ekr-views/src/timeline.rs
- confidence: inferred
  path: crates/ekr-views/tests/fixtures/conformance/scenarios
- confidence: inferred
  path: crates/ekr-views/tests/lineage.rs
- confidence: inferred
  path: crates/ekr-views/tests/query.rs
- confidence: inferred
  path: crates/ekr/src/cli/explain.rs
- confidence: inferred
  path: crates/ekr/src/cli/mcp.rs
- confidence: inferred
  path: crates/ekr/src/cli/resolve.rs
- confidence: inferred
  path: crates/ekr/src/cli/session/views.rs
- confidence: inferred
  path: crates/ekr/src/cli/view.rs
- confidence: inferred
  path: crates/ekr/tests/mcp.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/integrate.yaml
- confidence: inferred
  path: systems/ekr/domains/kernel.yaml
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T13:21:16Z", actor: "agent:ekr-controller-planner", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-15T13:21:16Z", actor: "agent:ekr-controller-planner", revision: 4}
---
## Context

A consumer's measured patch (ekr-mcp-fixes-20261015, items 3 and 13), measured on 0.0.34:

- #3: every read answer names types, edge types and properties by id only. The names sit only in
  `overview`'s ontology section, 86 KB on the measured store. Agents called `describe_node` on an
  edge type id or fetched the whole ontology. 16 of 28 agent trials were hurt.
- #13: no answer says when the served revision was committed; 3 of 28 trials were hurt.

The patch gave up byte identity between the MCP and `ekr view` to add these. That rule stays: each
answer that gains names becomes a new format version, served by the MCP tool and by `ekr view` (or
the session verb, `ekr explain`, `ekr resolve`), and the old version stays served.

`story:read-answers-name-types` (draft) asked for names on five reads; this story and its two
siblings (`story:bounded-node-detail` names the detail, `story:search-words-type-and-time` names the
matches) cover what it asked. Whether it is archived or superseded is the planner's call, not this
story's.

## Build (spec first)

1. **The spec first**, green under `ess specify validate --path systems/ekr` and the conformance
   suites before any code:
   - `systems/ekr/domains/views.yaml`:
     - `ekr.graph-slice/2`: `ekr.graph-slice/1` with `type_name` on every node and edge (#3);
     - `ekr.graph-overview/3`: `ekr.graph-overview/2` with `name` on each `node_types` and
       `edge_types` entry (#3) and `meta.committed_at`, the epoch milliseconds at which the revision
       read was committed (#13). It is a fact of the revision, so rule (5) still holds: two answers
       for one revision are byte-identical;
     - `ekr.graph-timeline/2` and `ekr.graph-changes/2`: /1 with a `names` map (#3);
   - `systems/ekr/domains/kernel.yaml`: `ekr.explanation/3`, `ekr.explanation/2` with a `names` map;
   - `systems/ekr/domains/integrate.yaml`: the resolve answer with a `names` map is a named,
     versioned document (`ResolutionOutcome` has no format name today, so this declares its first,
     for example `ekr.resolution/1`), the bare outcome staying what `ekr resolve` prints by default;
   - the `names` map, declared once: `{id: name}` for every node type, edge type and property id the
     document holds, from the ontology of the revision the document reads; keys ascending (rule (1));
     present and empty when the document holds no such id (rule (4)); never a node id;
   - per format, the per-surface default: the MCP answers the new version unless asked for the old;
     `ekr view`, the session verb, `ekr explain` and `ekr resolve` answer the old unless asked for
     the new. The spec states that each MCP answer equals that surface's answer for the same request
     with the format named explicitly.
2. `crates/ekr-views`: the fields and the map. `crates/ekr` (`cli/explain.rs`, `cli/resolve.rs`,
   `view.rs`, `session/views.rs`, `mcp.rs`): the format selectors and the MCP defaults.
3. `docs/cli.md`, `CHANGELOG.md`.

## Acceptance

Each case names "ekr-mcp-fixes-20261015 #3" or "#13".

Conformance scenarios:

- `an-overview-v3-names-its-types-and-says-when-its-revision-was-committed` (#3, #13): with `limit`
  20 and no section, the /3 answer carries the type names "Person", "Service" and "Incident" under
  `node_types` and `meta.committed_at` equal to the revision's commit time; the /2 answer of the same
  request is unchanged.
- `a-timeline-v2-names-the-types-it-cites` (#3): an Incident timeline's `names` maps the Incident
  type id to "Incident" and holds an entry for every type and property id in the document and no
  other.
- `a-slice-v2-names-each-node-and-edge-type` (#3).
- `changes-v2-name-the-types-and-properties-they-cite` (#3).

Tests:

- `crates/ekr/tests`: `ekr explain --format ekr.explanation/3` and `ekr resolve` asked for the named
  format carry `names`; without the selector they print the bytes they print today.
- Golden bytes moved to the new formats, not deleted, with the old bytes still held by a kept case:
  `crates/ekr-views/tests/query.rs`
  `a_slice_lists_each_node_before_the_edges_it_closes_and_pages_by_record` (to `ekr.graph-slice/2`)
  and `the_overview_opens_with_its_meta_and_lists_its_sections_in_declared_order` (to
  `ekr.graph-overview/3`); `crates/ekr-views/tests/lineage.rs`
  `a_store_without_widened_ends_or_modified_properties_answers_the_bytes_it_answered_before` keeps
  /2's bytes and gains the /3 bytes beside them.
- `crates/ekr/tests/mcp.rs`, moved to the new formats, not deleted: `timeline`, `expand`, `overview`,
  `explain` and `resolve` in `each_tool_answers_the_document_its_read_returns_on_the_same_store_state`;
  `resolve` and `overview` in `a_commit_by_another_process_is_read_by_the_next_tool_call`; `overview`
  in `a_running_server_answers_from_a_store_replaced_by_rename`;
  `changes_since_answers_and_refuses_exactly_as_get_changes_does` and
  `an_add_evidence_is_listed_once_by_changes_since_get_changes_and_the_session_verb` (to
  `ekr.graph-changes/2`; `story:changes-since-small-default-and-cached` then moves their limit); each
  compared byte for byte with the verb asked for the new format explicitly.

## Scope

- `systems/ekr/domains/views.yaml`, `kernel.yaml`, `integrate.yaml`;
  `crates/ekr-views/tests/fixtures/conformance/scenarios`
- `crates/ekr-views/src/lib.rs`, `index.rs`, `query.rs`, `timeline.rs`, `changes.rs`
- `crates/ekr/src/cli/mcp.rs`, `view.rs`, `session/views.rs`, `explain.rs`, `resolve.rs`
- `crates/ekr/tests/mcp.rs`, `crates/ekr-views/tests/query.rs`, `crates/ekr-views/tests/lineage.rs`
- `docs/cli.md`, `CHANGELOG.md`

## Overlaps and order

The widest story of the wave. Shares `views.yaml`, `mcp.rs`, `crates/ekr-views/src/{lib,index,query}.rs`
and `crates/ekr/tests/mcp.rs` with all five others; `changes.rs` with
`story:changes-since-small-default-and-cached`; `kernel.yaml` and `cli/explain.rs` with
`story:explain-scoped-to-cited-evidence`, which extends this story's `ekr.explanation/3` before the
wave's release (as `ekr.explanation/2` was once extended without a bump, kernel.yaml), so one release
carries one /3. Lands after `story:bounded-node-detail` and `story:search-words-type-and-time`, before
the changes and explain stories.

## Out of scope

The SDK's typed reads (`crates/ekr-sdk/src/read/views.rs`) keep reading the old versions; a
`describe_type` read (`story:describe-type-read`).
