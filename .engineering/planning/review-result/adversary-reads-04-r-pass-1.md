---
format: aep.planning-md/3
id: review-result:adversary-reads-04-r-pass-1
kind: review-result
status: active
title: Adversary, wave reads-04 unit R, pass 1
relations:
- reviews: task:read-verbs-open-a-read-only-store
revision: 1
---
## Verdict

NEEDS-CHANGE on `7416ed61`, two blockers and a warning, fixed in `8149b9e0`: long-lived readers now
re-read a store whose files changed; a read-only SQLite read never creates `-wal` or `-shm` beside
the database (`immutable=1` without a `-wal`, `mode=ro&readonly_shm=1` with one); `view` and `mcp`
remove their private copy on SIGTERM, SIGINT and SIGHUP, and stale copies of dead processes are
removed at the next read-only open. Held under attack: the file copy racing a writer, the SQLite
image with a live WAL, write verbs refused by name in every path, copies removed on normal exits.
The whole-store copy per open remains until eventlog opens read-only natively
(`task:eventlog-read-only-open`).

```findings
- file: crates/ekr-store/src/eventlog.rs
  line: 460
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a session or mcp server on a read-only store answered from the copy taken at open and never saw another process's commit"
- file: crates/ekr-store/src/read_only.rs
  line: 217
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "reading a read-only database in a writable directory created -wal and -shm and then locked out the owner's writes"
- file: crates/ekr-store/src/read_only.rs
  line: 151
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "ekr view left its full private copy in TMPDIR when sent SIGTERM"
- file: crates/ekr-store/src/read_only.rs
  line: 81
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the unit's cases always made the directory read-only too, so a check of the directory alone passed them"
- file: docs/cli.md
  line: 64
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the docs omitted the immutable read's no-lock property and the TMPDIR space a file-store copy needs"
```
