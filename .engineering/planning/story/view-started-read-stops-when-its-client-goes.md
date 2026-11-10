---
format: aep.planning-md/3
id: story:view-started-read-stops-when-its-client-goes
kind: story
status: draft
title: A started ekr view read stops when its client disconnects
refs:
- provider: github
  reference: beyond10x/epistemic-knowledge-runtime#96
relations:
- serves: vision:o5
- informed_by: story:view-historical-changes-and-cancelled-reads
revision: 1
---
## Context

`story:view-historical-changes-and-cancelled-reads` (#96) drops a queued `ekr view` read whose
client left while it waited, and answers a historical `/changes` at the head's cost. A read the store
thread has already started still runs to its end when its client disconnects (`store_loop` in
`crates/ekr/src/cli/view.rs`); its answer is not written. The adversary pass of 2026-11-10 named the
gap against that story's acceptance ("a read whose client disconnected stops"). Since every read now
costs at most the head's, a started read holds the queue for one head-cost read at most.

## Build

- Measure first: on a store of the size #96 names, the longest single read `ekr view` serves after
  #96, counted in index builds and events read.
- If one is long enough to matter, give the read path a cancellation check between units of work
  that the store thread trips when the job's client is gone, and answer nothing for it.

## Acceptance

- Either a measurement recorded as evidence shows no read long enough to matter and this story is
  archived with it, or a test holds a started read whose client disconnects and shows it stopping
  before its end (counted work, no wall-clock assertion), with a read queued behind it answered.
