---
format: aep.planning-md/2
id: review-result:adversary-p2p3p4-01-observe-pass-2
kind: review-result
status: active
title: Adversary pass 2 on unit O (observe domain)
relations:
- reviews: story:observe-domain-model
revision: 1
---
## Adversary pass 2 — unit O (`story:observe-domain-model`), HEAD `600cdfe8`

Verdict: NEEDS-CHANGE. Cases executed 46→50, red 6 (4 new in `observe/scratch/adversary-observe-2.sh`, plus the
two pass-1 cases already routed). Origin: introduced 4. Ledger against pass 1: carried 0, new 4, resolved 4.

Coordinator routing (2026-09-27):
- `observe.yaml:63` (INFEASIBLE — acceptance "references ekr.graph.Observation"): **decided** — a reference through
  the declared type `ekr.graph.ObservationId` meets the clause; a declared entity relation to Observation would
  decide what `decision-blocker:observation-retention-path` holds open. No-op.
- `observe.yaml:72` / `:4` (SourceUnit defined as "what a source is polled in"): back to the implementor.
- `registration.patch:+8` (component summary gives checkpoints and poll health to units): the coordinator's file;
  the coordinator writes a neutral summary when registering. No-op for the unit.
- `observe.yaml:63` note (SourceRecordObservation repeats key fields): no-op; nothing consumes it yet.

```findings
- file: systems/ekr/domains/observe.yaml
  line: 63
  category: acceptance
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: after the correction no ekr.observe entity declares a relation to ekr.graph.Observation, only a struct field typed ObservationId, which the story itself says is not a declared relation, and every entity-level link is blocked
- file: systems/ekr/domains/observe.yaml
  line: 72
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: SourceUnit and the domain summary define a unit as what a source is polled in, which is option 1 of source-unit-granularity, contradicting the marker that holds it open
- file: systems/ekr/conformance/registration.patch
  line: 8
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the ekr-observe component summary gives checkpoints and poll health to source units, restoring in components.yaml the link and placement the correction removed from the domain
- file: systems/ekr/domains/observe.yaml
  line: 63
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: SourceRecordObservation repeats source and source_native_id from the observation it names, with nothing requiring them to agree, and exists only to carry the reference
```
