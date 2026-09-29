---
format: aep.planning-md/3
id: review-result:adversary-sdk-02-b-pass-1
kind: review-result
status: active
title: Adversary, wave sdk-02 unit B, pass 1
relations:
- reviews: task:sdk-spawn-retries-a-busy-binary
revision: 1
---
## Verdict

CONFIRMED on `f02247aa`, no blocker; two defects fixed in `32a05803` (the timeout clock starts before
the spawn; the busy wait checks the cancel flag), two mutants covered by the adversary's cases in
`6f47e267`. The two timing cases were reworked in `7db519b6` to act on the SDK's observed wait
(`/proc/<tid>/wchan`), after 3 of 10 full runs went red under load; 10 of 10 at load 10.5 to 23.1
afterwards. Held under attack: fresh pipes per attempt, no child code before a refused exec, all
four spawn sites covered, other errors refused at once, the over-cap stdin file never executed.

```findings
- file: crates/ekr-sdk/tests/spawn_busy.rs
  line: 133
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the 300 ms floor let a retry cut to 310 ms pass while docs and CHANGELOG promise 630 ms"
- file: crates/ekr-sdk/src/session.rs
  line: 389
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "no unit test reached the one-shot retry, so reverting it to command.spawn() kept the suite green"
- file: crates/ekr-sdk/src/binary.rs
  line: 269
  category: concurrency
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "a cancel during a one-shot's busy wait was ignored until the wait ended and the call failed with Io, not Cancelled"
- file: crates/ekr-sdk/src/binary.rs
  line: 305
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the busy wait ran before probe_timeout started, so open_with with a 50 ms limit returned after 632 ms"
```
