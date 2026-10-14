---
format: aep.planning-md/3
id: decision-blocker:projection-lineage-evidence
kind: decision-blocker
status: open
title: Nobody has decided whether the projection document's schema lineage names the cited evidence
relations:
- blocks: story:schema-transaction-cites-evidence
revision: 1
---
## Question

Does `story:schema-transaction-cites-evidence` ship the cited evidence in the projection document's
schema lineage (`ekr.graph-projection/1`, `schema.versions[]`), or is its acceptance narrowed to the
reads that already carry it?

## Why nobody can read the answer

The story's acceptance says "`ekr ontology --at <that revision>` and the lineage name the evidence",
and its context says the consumer wants the viewer's schema lineage to show which message introduced
each type. Design § 109.4 postpones the projection document: `ProjectedSchemaVersion`
(`crates/ekr-views/src/document.rs:238`) has no evidence field, because its entries are declared by
`systems/ekr/domains/views.yaml` and adding one changes that contract. The adversary case for the
lineage is red on the unit branch and committed ignored.

## Options seen

1. Ship it in this wave: an optional `evidence` list on each `schema.versions[]` entry, omitted when
   empty, declared in `views.yaml` first; documents of stores without citations stay byte-identical.
   Cost: one spec entry, one field, suite digests re-synthesized, the red case un-ignored.
2. Narrow the acceptance to `ekr ontology --at` and `ekr_views::load` (`LoadedRevision.schemas`),
   and draft a follow-up story for the projection document's lineage.
3. A new document format version (`ekr.graph-projection/2`) carrying the field, in a later wave.

## What it stops

The close of `story:schema-transaction-cites-evidence`.

## Clears when

The operator records the answer; under option 1 it lands in `views.yaml` before the field is built.
