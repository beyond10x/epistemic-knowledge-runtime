---
format: aep.planning-md/3
id: task:timeline-rows-are-subjects
kind: task
status: active
title: The viewer's timeline rows are subjects again, from a ProjectTimeline read
relations:
- serves: vision:o5
- derived_from: story:view-streams-overview-and-expansion
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T03:47:00Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T03:47:00Z", actor: "human:timo", revision: 3}
---
## Context

The operator's prototype timeline shows one row per subject (each node of the chosen subject type,
for example every environment), with that subject's related events counted per day or week within
N hops, heatmap cells, row totals and per-row swimlanes; the type chips choose the row type, and
sort is by activity, recent or first. After the streamed data layer (wave p2p3p4-06, step 4,
commit 7afe8262) the timeline's rows are node types, because `ekr.graph-overview/1` carries timeline
buckets per node type only. This is a regression against the prototype the operator chose as the
UI.

## Build

- ESS `ekr.views`: a read `ProjectTimeline(store, at?, row_type, hops ≤ 3, limit ≤ 500, bucket?)`
  → `ekr.graph-timeline/1`: one row per node of `row_type` (the top `limit` by activity, ties by
  id), each with its buckets of related dated facts within `hops`, per event type, and its total;
  the bucket rule and roles as the overview states them; determinism rules in the style of the
  other formats; authored scenarios; the views suite regenerated.
- `ekr-views` engine: the query over the per-revision Index (cost proportional to the rows and
  their neighbourhoods).
- `ekr view`: `GET /timeline?type=<id>&hops=H&limit=L[&bucket=day|week][&revision=N]`, JSON.
- Page: the timeline reads it; rows are subjects again, with the prototype's type chips, hops
  control, sort and swimlanes; data-free as before.

## Acceptance

On the bench store, the timeline for a subject type shows one row per subject with its per-bucket
counts and totals equal to what the prototype showed for the same store and hops, and the page
still never loads `/projection`.
