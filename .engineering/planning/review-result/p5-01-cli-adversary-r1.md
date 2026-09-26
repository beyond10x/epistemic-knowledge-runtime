---
format: aep.planning-md/2
id: review-result:p5-01-cli-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p5-01 cli unit
relations:
- reviews: story:schema-evolution-transactions
revision: 1
---
Adversary pass 1 on unit p5-01-cli (wave p5-01), `aep-drive:adversary`, against `64eed89`.

Owners: 6 findings, 4 implementor (1 introduced, 3 pre-existing text made wrong by this story's profile v2), 2 coordinator (finding 3 is the coordinator's own patch application; finding 4 is compensated by docs_cli).

Verdict: CONFIRMED (1 warning, 4 notes), NEEDS-CHANGE (1 warning). cases: executed 236→238, red 2.

Cases in `crates/ekr/tests/adversary_p5_01_cli.rs` (2; 2 red): the seed-authority-profile row and the host schema description name only the P1 profile while the binary seeds under v2.

Routing (coordinator): findings 1, 2, 5, 6 back to the implementor (README.md and crates/ekr/src/host.rs added to its files); finding 3 no-op (the coordinator applied that patch to fix public_surface); finding 4 no-op (the listed-or-unreachable case in docs_cli holds the class).

```findings
- file: docs/cli.md
  line: 1390
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the seed-authority-profile row gives "not the P1 profile" as the cause while the binary seeds under profile v2
- file: crates/ekr/src/host.rs
  line: 107
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: ekr schema ekr.cli-host/1 describes authority as "the P1 validation profile" although the same schema and the binary admit v2
- file: crates/ekr-ontology/tests/adversary2_p5_01_ontology.rs
  line: 401
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: unit D edited a file under crates/ekr-ontology, which its brief marks as not its own
- file: crates/ekr/tests/adversary_p1_15_docs_r2.rs
  line: 289
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the widening admits every ontology code including unreachable ones and guards only by substring; docs_cli's listed-or-unreachable case compensates, so the suite as a whole is not weakened
- file: README.md
  line: 28
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: README still lists schema changes as refused and schema evolution as a later phase
- file: docs/cli.md
  line: 1439
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the unsupported-constraint fix says constraints must be [] in the seed, but under v2 a ModifyProperty adds them to a type with no instances and another ModifyProperty removes them
```
