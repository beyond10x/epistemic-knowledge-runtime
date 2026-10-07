---
format: aep.planning-md/3
id: story:c7-reads-beside-a-checkpointing-writer-do-not-depend-on-load
kind: story
status: active
title: The C7 reads-beside-a-checkpointing-writer cases do not depend on how long a lock wait takes
relations:
- serves: vision:o5
scope:
- confidence: inferred
  path: crates/ekr-store/src/eventlog.rs
- confidence: cited
  path: crates/ekr-store/tests/adversary_c7_s.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T06:22:52Z", actor: "agent:claude-ekr-controller", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-07T06:22:52Z", actor: "agent:claude-ekr-controller", revision: 5}
---
## What is wrong

Two cases in `crates/ekr-store/tests/adversary_c7_s.rs` assert that no read opened beside a
checkpointing writer fails for any reason. On a loaded machine one read can fail because the SQLite
lock is held longer than the open's retry window:

```text
first other: Some("the store is unavailable: event store is unavailable: database is locked:
still held when the retry window closed; a SQLite open waits at most 10.1s")
```

Observed in a local `task check` on 2026-10-07 at 00:41Z, load average about 30:

| case | line | result |
|---|---|---|
| `a_symlinked_store_opened_beside_a_checkpointing_writer_is_never_refused_as_replaced` | 328 | 1 of 15 reads failed, beside 507,116 external checkpoints |
| `adversary_c7_s_an_open_beside_a_writer_that_checkpoints_itself_is_never_refused_as_replaced` | 281 | 1 of 281 reads failed |

The same binary alone passed 9 of 9 three times in a row at load 28, and the five `main`
Correctness logs read on 2026-10-07 show it green. The outcome therefore depends on how long the
lock is held against a wall-clock window, which `AGENTS.md` § What must not happen here forbids for
a test.

Not established: whether a lock held over 10 s is the test's external checkpoint loop starving the
reader, or the store's own open path. That is the first thing to find out.

## Build

Find which holder keeps the lock past the retry window, then make the cases assert what they are
named for (no read refused as replaced) without depending on how long a lock wait takes under load.

## Acceptance

- Both cases pass, and their pass or fail does not depend on elapsed time: a lock-wait failure is
  either impossible by construction or counted and reported separately from the store-replaced
  refusals the cases are about.

## Scope

- `crates/ekr-store/tests/adversary_c7_s.rs` — cited (lines 281 and 328)
- `crates/ekr-store/src/eventlog.rs` — inferred (the retry window's message at line 444)
