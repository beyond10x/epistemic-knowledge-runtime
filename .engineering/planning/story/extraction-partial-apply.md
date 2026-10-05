---
format: aep.planning-md/3
id: story:extraction-partial-apply
kind: story
status: draft
title: One bad extracted fact is skipped and the rest applies
tags:
- consumer:cortex
revision: 1
---
## Outcome

One bad fact in an extraction document is skipped with a reason, and the rest of the document
applies.

## Starting point (0.0.30)

Reading and planning an extraction document refuse the whole document on the first bad fact
(`crates/ekr-integrate/src/extraction.rs:565-733`; `crates/ekr-sdk/src/extraction.rs` `Plan::of`,
234-353). A model-written document of a few hundred facts then applies nothing because of one
dangling reference or one undeclared property.

## Acceptance

An extraction document of 10 facts, 1 of which cites evidence the document does not carry, applies
9 facts, and the `ExtractionReport` lists the 1 skipped fact with its index and refusal code. A
caller that wants the old behaviour asks for it (strict mode) and gets the whole-document refusal.

## Consumer

`beyond10x/cortex` (one model call per batch; a whole-batch refusal today costs the batch).
