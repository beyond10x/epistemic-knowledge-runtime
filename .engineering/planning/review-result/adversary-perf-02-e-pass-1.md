---
format: aep.planning-md/3
id: review-result:adversary-perf-02-e-pass-1
kind: review-result
status: active
title: Adversary, wave perf-02 unit E, pass 1
relations:
- reviews: story:edge-type-endpoints-widen
revision: 1
---
## Verdict

Nothing blocks the unit. Every probe of admission held; one mutant the suite let through, now
pinned in `1066c15c`.

Attacked and held: widening under profile v1 (`unsupported-operation`); mixed with data
(`mixed-schema-transaction`); DefineNodeType before and after the widening in one transaction; an
abstract end; cardinality One after widening; Stale across a widening; full replay and migration;
an end gained beside transitive, symmetric, inverse, cardinality or name changes (still
`type-declaration-changed`); duplicate and empty ends; operation index 13 and the document round
trips.

```findings
- file: crates/ekr-ontology/src/evolve.rs
  line: 480
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "dropping the source_types subset check leaves the whole ekr-ontology suite green; only a route to incompatibilities other than evolve reaches it, and none exists"
```
