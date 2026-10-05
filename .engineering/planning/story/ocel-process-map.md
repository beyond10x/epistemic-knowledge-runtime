---
format: aep.planning-md/3
id: story:ocel-process-map
kind: story
status: draft
title: A store's OCEL log can be read as variants and a directly-follows graph
tags:
- consumer:cortex
revision: 1
---
## Outcome

A store's events can be read as a process: its variants and a directly-follows graph, derived from
the store's own OCEL 2.0 log.

## Starting point (0.0.30)

`ekr ocel` exports one revision as an OCEL 2.0 log (`ekr.ocel/1`). Turning that log into process
views (variants, directly-follows counts, handovers between actors) is done today by each consumer
in its own code.

## Acceptance

`ekr process-map` (or a view of the same output) over a store whose OCEL log has 3 cases following
two variants prints both variants with their counts and a directly-follows graph whose edge counts
match the log; the output format is documented like `ekr.ocel/1`.

## Consumer

An organisation-scale consumer that builds these views from its OCEL export today; moving the
generic part here lets it drop that code.
