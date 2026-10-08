---
format: aep.planning-md/3
id: story:extraction-admits-url-evidence
kind: story
status: draft
title: Extraction admits a URL as evidence for an extracted fact
tags:
- consumer:cortex
revision: 1
---
## Outcome

`ekr apply-extraction` admits an extracted fact whose evidence source is a URL (`!Url`), so a
consumer that extracts from web pages cites the page instead of filing it as a statement a person
made.

## Starting point (0.0.30)

- The graph model declares `!Url` (`crates/ekr-graph/src/evidence.rs:46`).
- Extraction refuses every source but `!HumanStatement` with `extraction-evidence-kind-unsupported`
  (`crates/ekr-integrate/src/extraction.rs:722`), and the kernel's provenance check says the same
  (`crates/ekr-kernel/src/validate/provenance.rs:129`).
- Consumer: `beyond10x/cortex` files each web page as `!HumanStatement {identity: <url>}`
  (`src/evidence.rs`) until this lands.

## Acceptance

An extraction document with `!Url` evidence applies and `ekr view` shows the URL as the fact's
evidence; a document with an unsupported kind is still refused with the same code; the extraction
schema (`ekr schema ekr.extraction-document/1`) lists `!Url`.

## Open question for the owner

What the kernel's provenance rule requires of a URL source (content hash, retrieval time) before it
counts as evidence. That decides whether this is one story or needs a design first.
