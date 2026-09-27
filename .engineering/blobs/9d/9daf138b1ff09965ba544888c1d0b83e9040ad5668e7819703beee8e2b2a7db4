---
format: aep.planning-md/1
id: review-result:p5-01-store-open-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p5-01 store-open unit
relations:
- reviews: story:store-open-semantics
revision: 1
---
Adversary pass 2 on unit p5-01-store-open (wave p5-01), `aep-drive:adversary`, against `5280795`.

Owners: 4 findings, 4 implementor (2 introduced, 1 introduced by the pass-1 correction, 1 undecided and routed to the implementor by the coordinator).

Verdict: NEEDS-CHANGE (2 warnings), CONFIRMED (1 warning), INFEASIBLE (1 note). cases: executed 624→629, red 4; a race case red 1 time in 6 runs.

Cases in `crates/ekr/tests/adversary2_p5_01_store_open.rs` (5): seed under a host naming another operator, and renaming the validator (bootstrap-authority-mismatch expected); a SQLite database with no tables; a file-store directory holding only its lock; head racing the first seed on SQLite.

Decision (coordinator): findings 3 and 4 are fixed in code to match docs/cli.md: on an existing store, seed checks the retained authority before admission.

```findings
- file: crates/ekr-store/src/eventlog.rs
  line: 141
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a SQLite database with a header but no owner tables, which a read racing the first seed observes, is refused with the provider's "owner table ekr_events is absent" instead of store-not-found
- file: crates/ekr-store/src/eventlog.rs
  line: 141
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a file-store directory holding only writer.lock reads as "journal integrity check failed" instead of store-not-found, reachable only by a seed killed in a microsecond window
- file: crates/ekr/src/cli/mod.rs
  line: 452
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: admission before open makes `ekr seed` under a host with a different operator report InvalidSeed seed-attribution-mismatch (exit 2), which points at the seed document rather than the documented bootstrap-authority-mismatch (exit 1)
- file: docs/cli.md
  line: 86
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: the page promises bootstrap-authority-mismatch exit 1 for every store verb under a different host authority, but `ekr seed` with an admissible seed reports AlreadySeeded exit 2 via commit.rs:207
```
