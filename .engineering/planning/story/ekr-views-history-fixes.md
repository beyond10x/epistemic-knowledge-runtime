---
format: aep.planning-md/3
id: story:ekr-views-history-fixes
kind: story
status: active
title: Views follow active assertions and the asked revision
summary: Event times, the change feed and historical change queries follow the store's active assertions and the revision asked for
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T08:05:01Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T08:05:01Z", actor: "human:timo", revision: 3}
---
## Outcome

Event times, the change feed and historical change queries follow the store's active assertions and the revision asked for.

## Work

Review of EKR 0.0.42, findings 17 22 31 32. Each finding below is quoted from the review packet
(the review packet); the cited links point at the 0.0.42 source.

17. An extracted event time disappears from process exports. An extraction committed Project.started_at=2026-03-12T00:01:00Z as an active, evidenced Timestamp. With --event-time Project.started_at, ocel exported zero events, counted one undated node, and process-map returned zero cases. Recording the same time separately via UpdateProperty yielded one event at 00:01 and one case. The selector reads only values on the node. Resolve an extracted event-time assertion at the selected revision or explicitly project it when writing. Check both views with an extraction-created event.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L440
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L762
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-views/src/ocel.rs#L367
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-views/src/process_map.rs#L177


22. The change feed misses new evidence and newly learned aliases. Attaching a second source changed the assertion’s explanation but yielded zero changes since revision 1. Adding just an alias changed resolve from ProposeNew to Resolved but yielded zero changes since revision 2. A subscriber would keep both stale answers. Add attachment and alias change records with the affected IDs.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-views/src/changes.rs#L294
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-views/src/changes.rs#L612
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L1472


31. A retracted date still supplies an event’s time in the default OCEL heuristic. After the example retracted Bob’s incorrect 2020 assertion, his only active dated claim began in 2026. The exported Person event still used 2020. The documented heuristic includes withdrawn facts. Expose an active-claims date mode for current-state reports and distinguish it from the historical heuristic.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/agent.rs#L508
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-views/src/index.rs#L543
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/systems/ekr/domains/views.yaml#L865
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-views/src/ocel.rs#L386


32. One historical changes query depends on the current head. The request since_revision=1, at=0 returned RevisionNotFound while the head was 0 and an empty success after it advanced to 1. The revision-0 data stayed the same. This follows the current specification. For reproducible historical reads, define the result relative to the selected revision and test the identical request before and after a later commit.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-views/src/changes.rs#L672
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/systems/ekr/domains/views.yaml#L3680
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L1499


## Acceptance

For each finding, a test that is red on main and green after the change, run on both the File and
SQLite stores where the finding concerns a store; no existing test weakened.
