---
format: aep.planning-md/3
id: story:evidence-erasure
kind: story
status: draft
title: An evidence record or a span within it is erased from what is stored and served, and its dependants are marked
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o2
- depends_on: story:merge-entity-applies
revision: 1
---
## Context

A consumer must remove evidence bytes from a store for a privacy cleanup, without losing the store's
revision history. `RetractAssertion` keeps the evidence stored and served; only a rebuild removes it,
which replaces history. The design is proposed in
`architecture-decision-record:0015-evidence-erasure-keeps-history`; this story waits for it to be
accepted and builds the accepted option. The Build below assumes its proposed option A.

## Build (spec first)

1. `systems/ekr/domains/kernel.yaml`, `store.yaml` and `views.yaml` first, green under
   `ess specify validate --path systems/ekr` (the pinned `ess` and the newest) and their
   conformance suites before any code: `EraseEvidence` (an evidence id, an optional byte span, a
   reason), its audit stub (evidence id, span, actor, time, reason; no erased byte and no hash of
   one), its refusals (unknown evidence, a span outside the record, an already erased span), the
   mark on dependent assertions, and how every read answers an erased item.
2. `ekr-kernel` validates and commits the erasure as its own revision; the maintenance path removes
   the bytes from the payload, retained preparations, receipt copies and checkpoints; replay accepts
   the stub; both providers.
3. A design amendment (a new dated section) relating erasure to § 35.3 and § 60.
4. `docs/cli.md` (the operation, a dry run, its refusals), `CHANGELOG.md`.

## Acceptance

Synthetic fixtures in the runtime's own vocabulary only; no data or name from any consumer's store.

- `an-erased-record-leaves-no-byte-in-the-store`: after the erasure and the maintenance pass, no
  stored file or row on either provider holds a marker string the record carried.
- `an-erased-span-keeps-the-rest-of-the-record`: the bytes outside the span still read.
- `every-read-answers-the-stub`: every MCP tool and `ekr view` route, at the head and at an earlier
  revision, answers the stub (who, when, why) and never the bytes; a test enumerates the read
  surface so a new read is covered.
- `dependent-assertions-are-marked-not-deleted`: each assertion citing the erased evidence is
  still present and marked; one with other evidence keeps it.
- `an-erased-store-reopens-and-replays`: reopen, with and without `--full-replay`, succeeds and the
  checkpoint records the erasure.
- `erasure-is-its-own-revision`: the head moves by one revision naming the erasure and its reason.

## Out of scope

Selecting evidence by predicate (`story:evidence-purge-keeps-history`); erasing an assertion's own
text; outward deletion in a source system.
