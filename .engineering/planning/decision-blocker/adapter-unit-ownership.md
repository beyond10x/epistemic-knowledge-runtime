---
format: aep.planning-md/3
id: decision-blocker:adapter-unit-ownership
kind: decision-blocker
status: cleared
title: Nobody has decided whether an adapter or its declaration owns the set of source units it polls
relations:
- blocks: epic:p2-observation-layer
- blocks: story:v1-chat-raw-becomes-observations
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-09-28T23:00:28Z", actor: "agent:claude-coordinator", revision: 4}
---
## Question

For `SourceAdapter → SourceUnit` (one adapter, many units):

- Who owns the set of units: the adapter, which discovers them from the source; a declaration, which lists them for the operator (the "declared denominators" of A8); or both, reconciled?
- When a unit disappears from the source, does it stay declared, and does its coverage show a gap?

## Why nobody can read the answer

- No ess/1 document declares `SourceAdapter` or `SourceUnit`.
- Design § 54 gives `poll(checkpoint) -> ObservationBatch` with no unit parameter.
- The epic says the adapter declarations are "lifted from `org-brain-successor/adapters/*.yaml`", and the roadmap says v2 `adapters/*.yaml`. Epic "Before decomposition" item 4 records that disagreement with a coordinator default and no decision. Neither path is in this repository.
- The epic acceptance ("a coverage report prints declared denominators") implies a declared set, but it does not say whether discovery may extend that set.

## Options

1. **Declared.** Units come only from a declaration. The adapter polls those, and the coverage denominator is the declaration.
2. **Discovered.** The adapter enumerates its units, and the coverage denominator is whatever it found.
3. **Declared, with discovery as a report.** Discovered but undeclared units are reported and not polled until declared.

## What it stops

The `SourceAdapter` trait (whether `poll` takes a unit), and the fixture adapter behind that trait. `story:observe-domain-model` carries this as an `UNMAPPED:` marker. `story:fixture-records-become-observations` has a single fixture source and does not depend on it.

## Answer

Answered by `architecture-decision-record:0012-source-adapters-are-a-contract` (operator, 2026-09-29): connectors live in consumers and implement a contract the engine specifies, so the consumer's adapter owns its units and declares them to the engine for coverage, as `story:source-adapter-contract` specifies.
