---
format: aep.planning-md/3
id: review-result:adversary-sdk-01-h-pass-1
kind: review-result
status: active
title: Adversary, wave sdk-01 unit H, pass 1
relations:
- reviews: task:readers-reopen-a-replaced-store
revision: 1
---
## Verdict

NEEDS-CHANGE, two findings, both fixed in the adversary-fix commit. The suspected SQLite `-wal` loss
is ruled out (a green guard case: closing the replaced connection leaves the `-wal` at the path).
One statx per request without a replacement (strace).

```findings
- file: crates/ekr/src/cli/session.rs
  line: 303
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "a session proposal committed or rejected by another process stays tracked as open, so every later replacement is refused as store-replaced-proposals-open"
- file: crates/ekr/src/cli/session.rs
  line: 462
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: "a file store replaced under the same device and inode is never reopened and ekr mcp answers -32603 file history diverged until restarted"
```
