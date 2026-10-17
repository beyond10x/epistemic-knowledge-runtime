---
format: aep.planning-md/3
id: story:search-match-kinds
kind: story
status: active
title: 'search answers ekr.node-matches/3: each match names its kind, a whole name or alias first regardless of case'
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
- depends_on: story:search-words-type-and-time
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T10:05:36Z", actor: "agent:claude-ekr-controller", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-17T10:05:36Z", actor: "agent:claude-ekr-controller", revision: 3}
---
## Context

A reader that reads only the first few matches per word of `SearchNodes` misses a node whose whole
name matches the word when other nodes match it as part of a longer name or alias. Under
`ekr.node-matches/2` the tier comes first: an Exact match (a case-exact substring, such as the word
inside a path-like alias) ranks above a Folded match, so a node whose whole name equals the word in
another case falls below every case-exact partial match. The wholeness of a match only orders
matches within one tier.

`ekr.node-matches/2` keeps its bytes. The new ranking and the per-match kind are a new format.

## Build (spec first)

1. `systems/ekr/domains/views.yaml` first, green under `ess specify validate --path systems/ekr`
   (the pinned `ess` and the newest) before any code: `ekr.node-matches/3` (`ekr.views.NodeMatchesV3`,
   `ekr.views.NodeMatchV3`, its meta) as a third variant of `SearchNodes`, with its determinism rule.
2. Each match carries its kind, from a closed enum: whole name, whole alias, prefix, substring, and
   words (the every-word match of /2), and a boolean saying whether it matched only after
   lowercasing (case-folded). `field` and `alias` stay as in /2.
3. Order: whole name, then whole alias, then prefix, then substring, then words, regardless of
   case; within one kind a case-exact match before a case-folded one; then, when the request
   names `from` or `to`, time descending; then degree descending; then id. Never by a name.
4. Meta counts the whole matches (name or alias) before `limit`, so a reader of the first page
   knows whether more whole matches exist.
5. `ekr view`, the session verb, the MCP `search` tool and the SDK serve /3 by the same format
   selection /2 uses; /1 and /2 answer unchanged bytes. `docs/cli.md`, `docs/sdk.md` and
   `CHANGELOG.md` name the format.

## Acceptance

Conformance scenarios under `crates/ekr-views/tests/fixtures/conformance/scenarios/`, and tests:

- `a-search-v3-ranks-a-whole-name-above-case-exact-partial-matches`: a node whose name equals the
  text in another case ranks above a node whose alias holds the text case-exactly inside a path.
- `a-search-v3-ranks-a-whole-alias-above-a-prefix`: whole alias before a name that starts with the
  text.
- `a-search-v3-names-each-match-kind-and-fold`: one match of each kind, each with its kind and its
  case-folded flag.
- `a-search-v3-counts-whole-matches-before-the-limit`: a limit of 1 over two whole matches answers
  a whole count of 2.
- The /1 and /2 golden bytes and scenarios pass unchanged; MCP and `ekr view` answer /3 byte for
  byte alike on both providers.

## Out of scope

Fuzzy search; merging duplicate entities; a change to /1 or /2.
