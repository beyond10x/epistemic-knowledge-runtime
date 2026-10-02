---
format: aep.planning-md/3
id: task:seed-race-test-names-its-signal
kind: task
status: draft
title: The seed and head race test names the signal a process ended by
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o5
revision: 1
---
## What is wrong

The 0.0.26 pull request's CI run (PR #58, run 36960449801, 2026-10-02) failed
`crates/ekr/tests/adversary2_p5_01_store_open.rs`
`sqlite_head_racing_the_first_seed_answers_only_documented_states`: the `ekr seed` process racing six
`ekr head` processes ended by a signal (`status.code()` None) with empty stderr. Locally it passed 8
of 8 runs (192 rounds); the job was re-run. The test prints no signal number, so the cause (an
outside kill, a crash) is not established; a panic or stack overflow would have printed to stderr.

## Build

The race case keeps what it failed with (AGENTS.md: a concurrency test keeps the error in its
message): the signal number via `ExitStatusExt::signal` and any core or stderr, for the seed and
each head. If it recurs, find the cause from that.

## Acceptance

- A process ending by a signal in the case names the signal in the failure message.
- The case passes 50 runs in a row at load above 30, or the cause is named here and fixed.
