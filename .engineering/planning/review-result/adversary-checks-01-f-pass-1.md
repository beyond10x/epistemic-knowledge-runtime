---
format: aep.planning-md/3
id: review-result:adversary-checks-01-f-pass-1
kind: review-result
status: active
title: Adversary, wave checks-01 unit F, pass 1
relations:
- reviews: story:validation-findings-read
revision: 1
---
## Verdict

NEEDS-CHANGE on `527bd4f8`: `views-suite.json` was not regenerated after `kernel.yaml` changed the
contract digest, so `task conform-fresh` failed. Both suites are regenerated once for the combined
wave tree and the adversary's case that both suites name one contract is un-ignored then. The basis
order the unit's fixtures could not distinguish is pinned by the adversary's case in `218b02ae`. Held
under attack: the key is the requested basis even after the head moved; a transaction is rejected
at most once; stale, validated and open transactions are excluded; bound edge cases; one-shot equals
session; a store replaced under a session.

```findings
- file: systems/ekr/conformance/views-suite.json
  line: 1130
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "kernel.yaml changed the ekr contract digest but views-suite.json was not regenerated, so conform-fresh failed"
- file: crates/ekr/src/cli/rejections.rs
  line: 235
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "removing the sort by basis left the unit's suite green because its fixtures' id order equals their basis order"
```
