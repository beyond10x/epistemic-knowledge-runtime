---
format: aep.planning-md/3
id: story:extraction-valid-time
kind: story
status: draft
title: An extracted fact is valid from the time its evidence was observed
tags:
- consumer:cortex
revision: 1
---
## Outcome

A fact applied through extraction is valid from the time its evidence was observed, not from an
unbounded past.

## Starting point (0.0.30)

`Assertion::new` in the extraction path sets no valid time (`crates/ekr-sdk/src/extraction.rs:560-583`;
`crates/ekr-sdk/src/document/graph.rs:273` leaves it unbounded). A consumer that extracts from
dated records (chat messages, ticket comments) loses when each fact was said, and timeline reads
cannot order them.

## Acceptance

An extraction document whose evidence entries carry an observed time applies facts whose
`valid_time.from` equals their cited evidence's observed time (the earliest, when a fact cites
several); a fact citing evidence with no observed time keeps today's behaviour.

## Consumer

`beyond10x/cortex`, `story:document-time-as-valid-time` (blocked on this through
`upstream-blocker:ekr-extraction-valid-time`).
