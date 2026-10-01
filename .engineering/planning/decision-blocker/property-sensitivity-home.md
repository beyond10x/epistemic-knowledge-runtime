---
format: aep.planning-md/3
id: decision-blocker:property-sensitivity-home
kind: decision-blocker
status: open
title: Nobody has decided whether a property's sensitivity class lives in the store's ontology or in pack metadata
relations:
- blocks: epic:type-packs
revision: 1
---
## Question

Where does a property's sensitivity class live?

## Options

1. **In the store's ontology**: a field on `ekr_ontology::PropertyDefinition`
   (`crates/ekr-ontology/src/types.rs:21`). The store says what is sensitive and the observation
   layer reads it without pack code. Cost: the canonical ontology encoding changes (seed hashes,
   schema versions), and setting it on an existing property is a schema change (`ModifyProperty`).
2. **In pack metadata only.** No kernel change. Redaction needs the pack at runtime, and a store
   seeded before a pack classified a property carries nothing.

## Why the coordinator did not decide

Option 1 changes the canonical ontology encoding, an architecture change; it was put to the operator
on 2026-09-29 and is not answered.

## Clears when

The operator records the answer.
