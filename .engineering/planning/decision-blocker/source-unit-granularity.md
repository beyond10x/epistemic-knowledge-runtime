---
format: aep.planning-md/3
id: decision-blocker:source-unit-granularity
kind: decision-blocker
status: open
title: Nobody has decided whether a source unit is one observed item or the stream a checkpoint advances over
relations:
- blocks: epic:p2-observation-layer
revision: 2
---
## Question

What is a **source unit**? Is it the single item that becomes one observation (one chat message)? Or is it the stream a checkpoint advances over and poll health reports on (one channel, one thread)?

Relation: `SourceUnit → Observation`. It is one-to-one if a unit is an item, and one-to-many if a unit is a stream.

## Why nobody can read the answer

The two readings both appear, and nothing typed settles them:

- The operator brief for the first P2 slice (2026-09-26) says both "one observation per source unit (for example one chat message)" and "checkpoints per source unit". A checkpoint per message is not a delta cursor in the sense of design § 55 ("messages after timestamp X").
- `docs/predecessors.md` § 8 (A8) says "replies are their own units", and the epic acceptance asks for "the checked-through cutoff per unit", which reads a unit as a thread or a channel.
- No ess/1 document declares a `SourceUnit`. `systems/ekr/domains/graph.yaml:590` gives `ekr.graph.Observation` only `source: String` and `source_native_id: Optional<String>`.

## Options

1. **Unit = stream.** A unit is a channel or a thread, with its own checkpoint and poll health. It yields many observations, one per item. Items are not units.
2. **Unit = item.** A unit is one message, with one observation each. Checkpoints and poll health then need a separate noun (for example "stream").
3. **Two levels, named separately.** For example `SourceStream` (checkpointed, health-reported) and `SourceItem` (observed), with an explicit one-to-many relation between them.

## What it stops

The `SourceAdapter` trait signature, per-unit checkpoints, poll health (`checked_through`, attempt | complete | partial | failed), and the coverage denominators. `story:observe-domain-model` carries this as an `UNMAPPED:` marker. `story:fixture-records-become-observations` uses one observation per fixture record and does not use the word unit.
