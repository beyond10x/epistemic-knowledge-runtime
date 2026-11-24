---
format: aep.planning-md/3
id: story:ekr-views-detail-fixes
kind: story
status: active
title: Node detail and filters return what the store holds
summary: Node detail and edge-type filters return what the store holds, with one text limit rule
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T08:05:03Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T08:05:04Z", actor: "human:timo", revision: 3}
---
## Outcome

Node detail and edge-type filters return what the store holds, with one text limit rule.

## Work

Review of EKR 0.0.42, findings 27 28 30. Each finding below is quoted from the review packet
(the review packet); the cited links point at the 0.0.42 source.

27. The default node detail can omit all of a node’s relation claims. The shipped seed gave Bob two active CEO_OF assertions and no structural edge. Full detail returned both. The compact CLI and default MCP result listed no relation and marked the result untruncated. Include assertion-only relation groups or report how many were omitted with a supported follow-up.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-views/src/detail.rs#L625
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/mcp.rs#L620
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L2749


28. Filtering by an edge type’s valid UUID-shaped name returns no matches. An accepted edge type had an ID ending in 206 and a display name shaped like a different UUID ending in 999. Bounded node detail returned its edge by ID and zero groups by its exact name. After an ID lookup misses, check the name index or provide an explicit name selector.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-views/src/detail.rs#L310
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L1467


30. The bounded detail format applies its text limit inconsistently. A 2,403-byte string returned as a scalar was clipped to 2,000 bytes and marked truncated. The same text nested in a list was returned in full with no truncation flag. The format documents the exception. Apply a byte budget to nested values or expose their size before an agent requests them.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-views/src/detail.rs#L386
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/systems/ekr/domains/views.yaml#L1378
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L1458


## Acceptance

For each finding, a test that is red on main and green after the change, run on both the File and
SQLite stores where the finding concerns a store; no existing test weakened.
