---
format: aep.planning-md/3
id: decision-blocker:rules-era-for-changed-verdicts
kind: decision-blocker
status: open
title: Nobody has decided how a changed validation verdict reaches stores that recorded the old one
relations:
- blocks: story:supersession-verdict-fixes-replay-safe
- blocks: story:merge-entity-review-follow-ups
revision: 1
---
## What is missing

`story:supersession-verdict-fixes-replay-safe` Build step 1: "Decide and record how a rule change
reaches stores that recorded verdicts under the old rule: a rules era (or validation-profile
revision) recorded with each verdict and replayed under the era it was recorded in, or another
mechanism the design already has for this. This is an architecture decision; it needs an ADR and
the operator's acceptance."

`story:merge-entity-review-follow-ups` rows H6 and H7 "wait on the rules-era decision of
`story:supersession-verdict-fixes-replay-safe`" (its Context).

Replay compares every retained rejection's issues with what the current rules answer, byte for
byte (`crates/ekr-kernel/src/replay.rs:1202-1215`, as both stories cite; not re-read for this
record). Releases 0.0.39 to 0.0.41 reached a consumer, so landing D7, D8, D10, Q7, H6 or H7
without the decision can stop a store that recorded an old verdict from reopening.

## Why this run does not decide it

The run of 2026-11-09 is non-interactive. The operator's goal ("zero remaining issues on github,
all bugs fixed from planning store") approves fixing defects; it does not choose between a rules
era, a validation-profile revision or another mechanism, and the story reserves that choice for
the operator by name. A bypass record replaces a question, never an answer.

## What clears it

An accepted architecture-decision-record that states how a changed validation verdict reaches
stores that recorded the old one. Then both stories can be accepted and waved.

## Asked of the operator

Which mechanism: (a) a rules era recorded with each verdict and replayed under its era, (b) a
validation-profile revision (v4) that carries the six fixes, or (c) another mechanism?
