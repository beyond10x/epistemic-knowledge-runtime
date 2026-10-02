---
format: aep.planning-md/3
id: task:ocel-times-events-by-a-named-property
kind: task
status: draft
title: ekr ocel times events by a named date property
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
- informed_by: task:ocel-export-prints-its-counts
revision: 1
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
