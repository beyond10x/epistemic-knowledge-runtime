---
format: aep.planning-md/3
id: story:extraction-admits-url-evidence
kind: story
status: active
title: Extraction admits a URL as evidence for an extracted fact
tags:
- consumer:cortex
relations:
- serves: vision:o5
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T16:21:37Z", actor: "agent:url-evidence-and-kinds", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-23T16:21:38Z", actor: "agent:url-evidence-and-kinds", revision: 6}
---
## Outcome

`ekr apply-extraction` admits an extracted fact whose evidence source is a URL (`!Url`), so a
consumer that extracts from web pages cites the page instead of filing it as a statement a person
made.

## Starting point (main after 0.0.42)

- The graph model declares `!Url` (`crates/ekr-graph/src/evidence.rs:47`, `EvidenceSource::Url`).
- Extraction refuses every source but `!HumanStatement` with `extraction-evidence-kind-unsupported`
  (`crates/ekr-integrate/src/extraction.rs:858`), and so does the SDK's mirror of the document
  (`crates/ekr-sdk/src/document/extraction.rs:322`), whose `EvidenceSource` has one variant
  (`crates/ekr-sdk/src/document/graph.rs:303`).
- The kernel's provenance check refuses an `AddEvidence` from any other source with
  `evidence-unsupported-source` (`crates/ekr-kernel/src/validate/provenance.rs:122`), and
  `ekr explain` ends a chain only at a `HumanStatement` (`crates/ekr-kernel/src/explain.rs:533`,
  `evidence-source-unexplained`).
- The seed admits `HumanStatement` evidence only (`crates/ekr-kernel/src/seed.rs:838`,
  `seed-unsupported-source`); this story does not change the seed.
- Consumer: `beyond10x/cortex` files each web page as `!HumanStatement {identity: <url>}`
  (`src/evidence.rs`) until this lands. Review of 0.0.42, finding 12.

## Acceptance

An extraction document with `!Url` evidence applies and `ekr view` shows the URL as the fact's
evidence; a document with an unsupported kind is still refused with the same code; the extraction
schema (`ekr schema ekr.extraction-document/1`) lists `!Url`.

## Open question for the owner

What the kernel's provenance rule requires of a URL source (content hash, retrieval time) before it
counts as evidence. That decides whether this is one story or needs a design first.

Answer (2026-11-23): one story, no design first. A `!Url` entry is held to the profile
`retained-admissible-evidence/1` that an `AddEvidence` from a `HumanStatement` is already held to:

- the payload travels with the entry and is retained; its bytes hash to the entry's
  `content_hash` (`ContentHash::of_bytes`, what `ekr hash` prints), or the entry is refused as
  `evidence-payload-mismatch`;
- `observed_at` is the time the page was fetched; `extracted_by` is the proposer, as for every
  added entry;
- the URL is the locator, kept as given. The kernel does not fetch it; the retained bytes are
  the evidence, the URL says where they came from.

The consumer retains each page's bytes and knows their hash and fetch time, so it meets this
profile as it is. `ekr explain` ends a chain at a `!Url` entry as at a `HumanStatement`, after
the same payload check. `!Document`, `!DatabaseRecord`, `!Observation` and `!GraphAssertion`
stay refused with the same codes, and the seed stays `HumanStatement`-only.
