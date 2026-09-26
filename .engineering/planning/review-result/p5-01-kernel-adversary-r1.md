---
format: aep.planning-md/2
id: review-result:p5-01-kernel-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p5-01 kernel unit
relations:
- reviews: story:schema-evolution-transactions
revision: 1
---
Adversary pass 1 on unit p5-01-kernel (wave p5-01), `aep-drive:adversary`, against `b8ee8fa`.

Owners: 2 findings, 2 implementor (introduced).

Verdict: NEEDS-CHANGE (1 blocker), CONFIRMED (1 warning). cases: executed 411→413, red 1.

Cases in `crates/ekr-kernel/tests/adversary_p5_01_kernel.rs` (2; 1 red): a v1 file store written by the base kernel (`cee0cae`) holding a P1-shape ownerless ModifyProperty rejection must reopen (red: proposal-document unknown field id); data after a schema change held to it on both providers (green).

Routing (coordinator): both back to the implementor. Decision: under v1 the ownerless P1 shape still parses and is rejected as at the base, byte for byte; under v2 it is refused with a named issue. The crates/ekr and conformance-digest edits the adversary noted were applied by the coordinator from the implementor's patches.

```findings
- file: crates/ekr-kernel/src/transaction.rs
  line: 234
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a v1 store written at the base that retains a P1-shape ModifyProperty proposal or rejection no longer replays, because replay re-parses its bytes against the new owner shape and refuses with proposal-document unknown field id
- file: docs/epistemic-knowledge-runtime-design.md
  line: 4229
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: section 95 claims v1 stores with retained schema rejections keep replaying, but the case it names builds the rejection with the new owner shape that no pre-unit store holds, and against real base bytes the claim fails
```
