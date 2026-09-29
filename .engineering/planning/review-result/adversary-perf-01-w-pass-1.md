---
format: aep.planning-md/3
id: review-result:adversary-perf-01-w-pass-1
kind: review-result
status: active
title: Adversary, wave perf-01 unit W, pass 1
relations:
- reviews: story:validation-without-whole-graph-scans
revision: 1
---
## Verdict

Nothing found: no admission or refusal changed. One judgement note.

```
unit: W, story:validation-without-whole-graph-scans, fb4850a8 (base c81a426a)
verdict: no defect
cases: executed 412→419, red 0
```

## Cases added (`crates/ekr-kernel/tests/adversary_perf_01_validation_scans.rs`, kept in `036de706`)

A differential over 3,000 seeded graph-and-transaction pairs under profiles v1–v3 whose issue text
(validator, code, message, order) hashes to the digest base produced; cardinality with edges created
and deleted in either order; delete-and-recreate; byte-exact aliases (not by case or normalisation);
an edge deleted in the transaction; AddEvidence cited in the same transaction; a retraction beside a
named operation. Head and base dumps identical (53,289 lines). Five planted faults each caught; the
unit's own property saw three of them.

```findings
- file: crates/ekr-kernel/src/validate/candidate.rs
  line: 31
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the commit claims validation without whole-graph copies, yet Candidate::of still copies every node and edge five times per validation and reference.rs:180 still loops over every assertion; this is linear and within the acceptance bounds, but the claim overstates the change"
```
