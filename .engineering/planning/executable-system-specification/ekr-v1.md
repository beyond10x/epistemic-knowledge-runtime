---
format: aep.planning-md/1
id: executable-system-specification:ekr-v1
kind: executable-system-specification
status: validated
title: systems/ekr — the runtime's executable system specification, v1
relations:
- specifies: epic:p1-kernel-ontology-core
revision: 2
---
## What this is

The runtime's own executable system specification, `systems/ekr/`, format `ess/1`: four domains
(`ekr.kernel`, `ekr.ontology`, `ekr.graph`, `ekr.store`) and the components that own them.
Written before the crates, so the Rust implements the document.

## Validation

`ess specify validate --path systems/ekr` → `ekr v1 — 6 file(s), valid`, exit 0, on 2026-09-21
with ESS 0.26.0.

## Commands it obliges

`ekr.kernel.Seed`, `Propose`, `Validate`, `Commit`, `Snapshot`, `Explain`, with the
`GraphTransaction` ladder `Proposed → Validated | Rejected`, `Validated → Committed | Stale`.

## Open markers

Two `UNMAPPED:` markers in `domains/ontology.yaml`: `EdgeCardinality` beyond One/Many, and the
`Constraint` language of design § 11.2. Both wait on a design decision, not on code.

## Conformance

Moves to `conforming` on the report `story:ess-conformance-kernel` records.
