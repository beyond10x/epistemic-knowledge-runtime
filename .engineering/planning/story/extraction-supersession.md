---
format: aep.planning-md/3
id: story:extraction-supersession
kind: story
status: draft
title: An extraction document can supersede an earlier assertion
tags:
- consumer:cortex
revision: 1
---
## Outcome

An extraction document can state that a fact replaces an earlier one, and applying it supersedes
the earlier assertion instead of adding a second active one.

## Starting point (0.0.30)

The extraction path only declines to re-assert claims that are already retracted or superseded
(`crates/ekr-sdk/src/extraction.rs:94-108, 399-424`); it has no way to supersede. A consumer that
imports a registry (people and their teams) needs "this value replaced that one" when a source
changes, and today must write its own `SupersedeAssertion` transactions.

## Acceptance

An extraction document with a property fact marked as replacing the active value of the same
subject and property applies one `SupersedeAssertion`: afterwards exactly one assertion for that
subject and property is active, carrying the new value, and the old one is superseded with the new
fact's evidence. A replacement naming no active assertion is refused for that fact only, with a
reason.

## Consumer

`beyond10x/cortex`, `story:structured-from-files-and-drops` (blocked on this through
`upstream-blocker:ekr-extraction-supersession`).
