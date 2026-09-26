---
format: aep.planning-md/2
id: task:open-ended-read-ban-binds-one-crate
kind: task
status: implemented
title: The ban on filtering valid time with no known end reaches ekr-graph only
relations:
- derived_from: story:source-guard-debt
- serves: vision:o2
revision: 4
---
## The decision this records

Wave p1-04 deleted `GraphSnapshot::active()`. It filtered assertions the runtime currently believes
whose valid time has no known end, which the adversary measured to be the same read as `valid_at`
asked at the largest representable instant. Two ordinary facts fall on the wrong side of it: a
fixed-term fact true today is excluded because its end is known, and an announced successor whose
tenure has not begun is included. `valid_at(t)` is now the only read.

## What is not enforced

The decision binds `ekr-graph` and nothing else.

`Assertion::valid_time` is a public field and `TemporalRange::to` is a public field, so
`assertions.filter(|a| a.is_current() && a.valid_time.to.is_none())` rebuilds the deleted filter in
one line. `crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs` scans every module of
`crates/ekr-graph/src/` for it, and that is the whole of its reach: a text search over one crate's
source. `ekr-kernel` and `ekr-store`, which are the next two stories and are the crates that will
actually read a snapshot, are not covered by it and cannot be from here.

`TemporalRange::is_open` was deleted in the same round for having no caller. That removed the
convenience, not the capability.

## What closes this

One of:

- the guard is lifted to the workspace the way `crates/ekr/tests/public_surface.rs` was in wave
  p1-03, scanning every crate's `src/` rather than `ekr-graph`'s;
- or `valid_time` stops being a public field and the only way to ask a temporal question is a
  method that takes an instant;
- or a decision is recorded that consumers above `ekr-graph` may build the filter, with the reason.

The first is cheapest and has a working precedent in this repository.

## Where it is named

`crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs` calls it "an unfiled follow-up named in
this unit's report". That sentence takes this id.
