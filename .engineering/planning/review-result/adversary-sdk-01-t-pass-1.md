---
format: aep.planning-md/3
id: review-result:adversary-sdk-01-t-pass-1
kind: review-result
status: active
title: Adversary, wave sdk-01 unit T, pass 1
relations:
- reviews: story:sdk-session-transport
revision: 1
---
## Verdict

CONFIRMED, two warnings and two notes, sent back for fixing. The line-cap boundary, the child's
inherited state (no extra fds, empty signal mask), a partial line then exit, a taken viewer port, a
cancel during the one-shot fallback and replay refusals held.

```findings
- file: crates/ekr-sdk/src/session.rs
  line: 460
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "a seed over the line cap runs as a one-shot, so the --create session never holds the store it created"
- file: crates/ekr-sdk/src/session.rs
  line: 314
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "close and Drop never read the cancel flag, so a cancel during close waits out the request timeout"
- file: crates/ekr-sdk/src/binary.rs
  line: 233
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "EkrBinary::open and operations block forever on a binary that never answers"
- file: crates/ekr-sdk/src/session.rs
  line: 416
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "a one-shot reply keeps only the last 4096 bytes of stderr, contrary to docs/sdk.md"
```
