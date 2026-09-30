---
format: aep.planning-md/3
id: review-result:adversary-ops-05-o-pass-1
kind: review-result
status: active
title: Adversary, wave ops-05 unit O, pass 1
relations:
- reviews: task:store-exports-ocel-event-log
revision: 1
---
## Verdict

NEEDS-CHANGE on `1f141d22` (after the coordinator's design round: the timeline's event rule,
`--events`, the names table), fixed in `5f7edc52`. Held under attack: every output passes the
published OCEL 2.0 JSON schema, event and object ids never collide, relationships point at objects
that exist, file and SQLite bytes are identical, names follow the requested revision, and the lineage
keyed by (kind, id) keeps earlier bytes.

```findings
- file: crates/ekr-views/src/ocel.rs
  line: 515
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a time-typed attribute value outside years 0000-9999 was written as milliseconds; now left out and counted"
- file: crates/ekr-views/src/ocel.rs
  line: 396
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "events before 1970 related objects whose values started at 1970; values now start at the log's earliest event time"
- file: crates/ekr-views/src/ocel.rs
  line: 293
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "an inherited property is declared on every inheriting type, a documented departure from OCEL 2.0 Definition 2"
- file: crates/ekr-views/src/ocel.rs
  line: 274
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "no unit case reached an event time outside years 0000-9999; covered by the adversary's case"
- file: docs/cli.md
  line: 564
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "parallel edges of one type merge into one relationship; now counted and documented"
```
