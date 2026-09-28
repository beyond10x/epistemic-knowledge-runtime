---
format: aep.planning-md/3
id: review-result:adversary-read-02-s-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit S: ekr session before a store'
relations:
- reviews: story:session-starts-before-a-store
revision: 1
---
unit: story:session-starts-before-a-store (read-02, unit S), commit 1fa25a2b plus the adversary's untracked test file, worktree ekr-r2-s
verdict: nothing found (no red case; 3 notes)
cases: executed 396→403, red 0
origin: introduced 2 / pre-existing 0 / undecided 1

## Cases added (all green; kept in the suite as crates/ekr/tests/adversary_session_create_pass1.rs)

A refused seed in a `--create` session leaves the same files and answers as one-shot (absent path,
empty directory, empty file), and a session without `--create` leaves the path unchanged; two
sessions seeding two documents at once serve one store; a plain session serves a store another
process seeded and commits into it; after its own seed the session sees another process's commit
and answers a stale commit as one-shot; on SQLite the session holds the store its seed created; a
provisioned store is seeded; `--full-replay session --create` works and request-level
`--store`/`--full-replay` stay `session-option-refused`.

## What held

Starting without `--create` creates nothing (no lock, `-wal` or directory); second seeds and
`--create` on an existing store answer as one-shot; `seed` without `--create` and every option
refusal are unchanged; docs and help rows match the code.

```findings
- file: crates/ekr/src/cli/session.rs
  line: 183
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a session whose own seed did not create the store never holds it and reopens and re-verifies the store on every later store verb
- file: crates/ekr-kernel/src/commit.rs
  line: 352
  category: concurrency
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: the loser of two concurrent seeds of different documents answers exit 1 "a publication preparation exists for different input", an answer docs/cli.md does not document
- file: crates/ekr/tests/session.rs
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: no case in session.rs observes that the session holds the store after its seed, so deleting that reopen leaves every case green
```
