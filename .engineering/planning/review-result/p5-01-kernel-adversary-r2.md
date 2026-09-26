---
format: aep.planning-md/1
id: review-result:p5-01-kernel-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p5-01 kernel unit
relations:
- reviews: story:schema-evolution-transactions
revision: 1
---
Adversary pass 2 on unit p5-01-kernel (wave p5-01), `aep-drive:adversary`, against `6538628`.

Owners: 2 findings, 1 implementor, 1 coordinator (finding 1 names the case the coordinator kept out of the tree after the secret scan refused its fixture).

Verdict: NEEDS-CHANGE (1 blocker), CONFIRMED (1 note). cases: executed 417→425, red 1.

Cases in `crates/ekr-kernel/tests/adversary2_p5_01_kernel.rs` (8; 1 red): section 95 names only cases that exist (red); the ModifyProperty parser refuses every ambiguous mapping; owned and ownerless modifications never share an address; InstanceState over edges, edge assertions and inherited types; a property redeclared across versions replays on both providers; a schema commit lost before its write resumes; two schema changes against one head commit once; a property redeclared twice in one transaction (records last-wins).

Routing (coordinator): finding 1: section 95 marks the file-provider base-era claim unexecuted in the tree and names the SQLite case; finding 2: decided, two ModifyProperty of one (owner, property) in one transaction are refused as conflicting-write, and the last-wins case is rewritten to the decision.

```findings
- file: docs/epistemic-knowledge-runtime-design.md
  line: 4249
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: §95 claims the file-provider base-era replay is executed by a case that no test file declares, so the claim is unexecuted and must say so or the case must land
- file: crates/ekr-kernel/src/validate/structural.rs
  line: 280
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: two ModifyProperty of one property with different declarations in one transaction are admitted last-wins, while the equivalent competing data writes are refused as conflicting-write
```
