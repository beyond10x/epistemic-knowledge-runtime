---
format: aep.planning-md/3
id: task:ocel-times-events-by-a-named-property
kind: task
status: active
title: ekr ocel times events by a named date property
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
- informed_by: task:ocel-export-prints-its-counts
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T10:34:31Z", actor: "agent:codex-ekr-x7b", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T10:34:31Z", actor: "agent:codex-ekr-x7b", revision: 4}
---
## What is wrong

A consumer (2026-10-02) cannot adopt `ekr ocel`. Under its default rule an event's time is the
fact's valid time, which in the consumer's store is when a message stated a thing, not when it
happened, and the default makes seven whole node types events (34,535 events, 8,460 objects on its
live store, against 31,981 and 11,120 by its own rule). Its own rule times three properties:
`Alert.fired_at`, `Decision.decided_at`, `Incident.reported_at`.

## Decision (coordinator, 2026-10-02)

`ekr ocel --event-time <NodeType>.<property>` (repeatable) names the event types and, for each, the
date-valued property that gives an event's time. Nodes of those types without a value for that
property are left out and counted in the export's counts; every other node type is an object. The
default rule and `--events` stay as they are; `--event-time` and `--events` are refused together.
A named property that is not date-valued, or not declared on the type, is a named refusal.

## Acceptance

- Specified in `views.yaml` (`ekr.views.ExportOcel`) before code; the one-shot verb, the session
  verb and the SDK accept it.
- On a fixture with a dated property, each event's time is that property's value, undated nodes are
  left out and counted, and the default output is byte-identical to today's.

## Resume scope (2026-10-02)

Read-only story-scoper inspected main 4832d892. Primary paths, cited unless explicitly new:

- `crates/ekr-views/src/ocel.rs`
- `crates/ekr/src/cli/mod.rs`
- `crates/ekr/src/cli/ocel.rs`
- `crates/ekr-sdk/src/read/ocel.rs`
- `crates/ekr-sdk/src/read/mod.rs`
- `systems/ekr/domains/views.yaml`

The consumer group shares views.yaml, CLI dispatch, typed SDK exports and documentation;
its artifacts are implemented serially in one managed consumer unit. Generated conformance
suites and planning writes belong to the coordinator. New SDK modules are inferred.
Typed task scope is unavailable: AEP 0.64.0 restricts the scope field to stories.
