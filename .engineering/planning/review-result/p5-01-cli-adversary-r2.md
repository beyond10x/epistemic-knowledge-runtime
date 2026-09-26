---
format: aep.planning-md/2
id: review-result:p5-01-cli-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p5-01 cli unit
relations:
- reviews: story:schema-evolution-transactions
revision: 1
---
Adversary pass 2 on unit p5-01-cli (wave p5-01), `aep-drive:adversary`, against `ae6a3b5` (the cli unit at `64ec35f` with the kernel unit at `6538628` merged).

Owners: 3 findings, 3 implementor (introduced); the description half of finding 2 is in the kernel unit's file and is applied by the coordinator from a patch.

Verdict: NEEDS-CHANGE (1 blocker, 1 warning), CONFIRMED (1 note). cases: executed 242→244, red 2.

Cases in `crates/ekr/tests/adversary2_p5_01_cli.rs` (2; 2 red): the refusal table lists modify-property-without-owner; the schema gap for the P1 ModifyProperty shape is named.

Routing (coordinator): findings 1 and 3 back to the implementor; finding 2: docs row by the implementor, printed description by patch; the CHANGELOG entry is the coordinator's at the close.

```findings
- file: docs/cli.md
  line: 1399
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: profile v2 rejects the ownerless P1 ModifyProperty as modify-property-without-owner, which the refusal table omits, and docs_cli.rs's hand-written SCHEMA_SHAPE_CODES (three codes) lets the coverage test pass without it
- file: docs/cli.md
  line: 250
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the reader now accepts a ModifyProperty written as the bare P1 declaration while the printed schema refuses it, and neither the page's schema-difference table nor the printed description names that difference as both claim to
- file: README.md
  line: 26
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the "works in 0.0.2" table no longer lists the schema-change refusal that 0.0.2 and 0.0.3 still have, and the README sends readers to a CHANGELOG whose Unreleased section is empty
```
