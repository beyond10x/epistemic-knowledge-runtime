---
format: aep.planning-md/3
id: task:no-test-asserts-wall-clock-cost
kind: task
status: draft
title: No test asserts a cost in wall-clock time
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o5
- derived_from: task:rendering-cost-test-passes-under-load
revision: 2
---
## What is wrong

AGENTS.md forbids a test that passes or fails on wall-clock time. Wave correct-07 unit F
(2026-10-02, `e33241b7`) converted three and listed the rest, none converted:

- `crates/ekr-views/tests/adversary_code_names.rs:121` (under 1 s), `:148` (under 5 s): needs a work
  counter in the product.
- `crates/ekr-store/src/eventlog.rs:2148–2150`: a retry window measured by the clock; needs an
  injected clock.
- `crates/ekr-sdk/tests/spawn_busy.rs:131,153,168`, `adversary_spawn_busy.rs:212`,
  `session.rs:494,708`, `adversary_session_close.rs:218`: response-time bounds on subprocesses.
- `crates/ekr/tests/view_page.rs:3394` (`a_narrow_window_opens_compact_and_the_graph_keeps_its_width`,
  "the page settled"): timed out once in the 0.0.26 local gate at load 11–16, passed 3 of 3 alone.
- `crates/ekr/tests/view_cli.rs:588`, `view_stream.rs:1040`, `adversary_v_view_r2.rs:198,275`,
  `adversary_code_names_cli.rs:244`.

Timeouts that only stop a hang (`adversary_p1_12_recovery_1.rs:363`, `adversary_x6_d.rs:470`,
`adversary_page_stream_1.rs`) are not cost assertions and stay.

## Build

Each listed assertion counts work or compares two measurements taken in one process, or becomes a
hang guard with a generous bound and a message saying so; the clock-measured retry window takes an
injected clock.

## Acceptance

- `git grep` for `elapsed()` and `Duration` bounds in tests lists only hang guards, each named in a
  source guard's allow-list with its reason.
