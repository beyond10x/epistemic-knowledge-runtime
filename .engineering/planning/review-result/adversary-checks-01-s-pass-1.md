---
format: aep.planning-md/3
id: review-result:adversary-checks-01-s-pass-1
kind: review-result
status: active
title: Adversary, wave checks-01 unit S, pass 1
relations:
- reviews: task:sdk-close-reports-the-child-stderr
revision: 1
---
## Verdict

NEEDS-CHANGE on `05daf2f5`, two findings, both fixed in `64ab1a0e`; the adversary's cases in `ca3ae6a6`
also pin the cancel and latch branches no unit test reached. Held under attack: the collector racing
the child's exit (20 of 20 rounds kept the last line), long and non-UTF-8 tails, `killed` for a child
that ends itself by a signal, the deadline kill, drop after close, a cancel from another thread
during close, a grandchild holding stderr.

```findings
- file: crates/ekr-sdk/src/session.rs
  line: 334
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: "close() checked success before the latch and cancel, so a failed or cancelled session whose child exited 0 closed Ok(())"
- file: crates/ekr-sdk/src/transport.rs
  line: 135
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "a stderr tail cut inside a UTF-8 character was 4098 bytes starting with U+FFFD, over the documented bound"
```
