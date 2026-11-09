---
format: aep.planning-md/3
id: task:seed-race-test-names-its-signal
kind: task
status: active
title: The seed and head race test names the signal a process ended by
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o5
- decomposes: epic:zero-open-issues-2026-11
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:55:01Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-09T21:55:01Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
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

## Released diagnostic correction

Source inspection at released main confirms that
crates/ekr/tests/adversary2_p5_01_store_open.rs prints seed and head signal numbers.
The earlier missing-signal description is obsolete. This is partial completion, not closure:
the repeated stress acceptance remains unverified. No stress runs were performed in this review.

## Scope

Derived 2026-11-09 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr` integration tests — cited
- **Files:** `<tree>/crates/ekr/tests/adversary2_p5_01_store_open.rs:189-239`, case `sqlite_head_racing_the_first_seed_answers_only_documented_states` — cited
- **Acceptance 1 (signal named), satisfied:** seed at `:209-211` and each head at `:222-224` print `ExitStatusExt::signal`; the last commit on the file, `4832d892e` (#58), is an ancestor of `origin/main` — cited
- **Acceptance 2 (50 runs in a row at load above 30), open:** no stress run is recorded; the body says none were performed — cited
- **Build gap:** the message carries stderr but nothing about a core file, which the Build paragraph also asks for — cited
- **Also likely:** `crates/ekr/src` seed and SQLite store-open path, only if a stress run reproduces the signal death and names a cause — inferred
- **Documents:** the task body, to record the stress result or the named cause — inferred
- **Confidence:** high — the story names the file and case, and the tree shows the signal lines
- **Would collide with:** any unit editing `crates/ekr/tests/adversary2_p5_01_store_open.rs`; if a cause is found, any unit on `ekr seed` or SQLite store open — inferred
- **Safety fact:** the remaining acceptance is a verification run that edits no source unless it fails; the file is referenced only by planning documents — step 2 (`git grep -n adversary2_p5_01_store_open`), unproven
