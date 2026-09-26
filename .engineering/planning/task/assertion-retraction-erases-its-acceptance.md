---
format: aep.planning-md/2
id: task:assertion-retraction-erases-its-acceptance
kind: task
status: implemented
title: A retraction replaces the Accepted state and discards the validators that accepted it
relations:
- derived_from: story:version-persisted-contracts
- serves: vision:o2
revision: 4
---
## What is wrong

Design § 17 gives one enum. `Retracted { reason }` and `Superseded { by }` are variants of
`ValidationState` alongside `Accepted { validators }`, so moving an assertion to either of them
**replaces** the accepted state and the validator set goes with it.

Design § 36 asks for the opposite: canonical state "no longer treats it as true, without erasing
that it once did". `AGENTS.md` invariant 4 is the same claim at repository scope — which agents
accepted an assertion is provenance, and provenance is what a retraction most needs to keep.

Found by the adversary of wave p1-04 at `crates/ekr-graph/src/assertion.rs:136`. It could write no
failing case: the defect is that a state is unrepresentable, and there is no value to exhibit.

## Why the crate was not changed

`crates/ekr-graph` implements design § 17 as written, which is correct behaviour for an
implementing crate. The design document is normative here and an amendment adds rather than
rewrites, so the shape is a design decision and not a wave's to take.

A second consequence of the same shape, which the crate does carry: `AssertionStatus` is derived
from `ValidationState` rather than stored, so `status()` gives a `Rejected` assertion the status
`Active`, whose own doc reads "still standing".

## What closes this

A dated amendment to `docs/epistemic-knowledge-runtime-design.md` deciding one of:

- two independent fields, validation and lifecycle, which is what
  `systems/ekr/domains/graph.yaml:248-251` already declares — `validation` and `superseded_by` are
  separate there;
- `Retracted` and `Superseded` carrying the state they replaced;
- or a statement that the erasure is intended, with the reason.

Then `ekr-graph` follows the amendment.

## Why it blocks

`story:commit-and-revision-lineage` writes the retraction command. That writer is the first caller
that has to decide what happens to the validator set, and it cannot be written before the shape is
decided.
