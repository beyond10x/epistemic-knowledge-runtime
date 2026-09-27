---
format: aep.planning-md/1
id: review-result:p5-01-store-open-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p5-01 store-open unit
relations:
- reviews: story:store-open-semantics
revision: 1
---
Adversary pass 1 on unit p5-01-store-open (wave p5-01), `aep-drive:adversary`, against `b186c86`.

Owners: 7 findings, 7 implementor (3 introduced; 4 pre-existing but inside this story's acceptance "a refused seed leaves no store behind", so routed to the implementor, not filed).

Verdict: NEEDS-CHANGE (1 blocker, 2 warnings), CONFIRMED (1 warning, 3 notes). cases: executed 616→621, red 5.

Cases in `crates/ekr/tests/adversary_p5_01_store_open.rs` (5; 5 red): a refused seed into an existing empty directory (file) and an existing empty file (SQLite), through a dangling symlink, refused for its tenant; a read verb on an existing empty path. Origins measured by running the file against a `git archive 468534e` copy.

Routing (coordinator): all seven back to the implementor. The adversary was stopped once by an HTTP 429 and resumed.

```findings
- file: crates/ekr/src/cli/mod.rs
  line: 451
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: open_to_seed skips seed admission when the store path already exists, so a refused seed into an existing empty directory or empty SQLite file provisions a store
- file: crates/ekr/src/cli/mod.rs
  line: 451
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a dangling symlink at --store skips admission and SQLite creates the link target for a refused seed
- file: crates/ekr-store/src/eventlog.rs
  line: 220
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: the tenant is validated in assemble after the provider has created the store, so a seed refused for an empty tenant leaves a store behind
- file: docs/cli.md
  line: 1155
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: the store-not-found row claims a refused seed creates no store, which cases 1-4 show is false
- file: crates/ekr/src/cli/mod.rs
  line: 424
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a read verb on an existing empty directory or file is reported as a provider-open fault, not the documented store-not-found
- file: crates/ekr/src/conformance.rs
  line: 218
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the doc comment says conformance opens the provider exactly as the CLI does, but it still opens with open-or-create for every verb
- file: crates/ekr-store/src/eventlog.rs
  line: 147
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the retry deadline is checked only between attempts, each of which can wait 5 s, so the stated 10 s bound is about 15 s
```
