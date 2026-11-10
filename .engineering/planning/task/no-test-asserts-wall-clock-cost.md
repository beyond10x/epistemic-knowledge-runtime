---
format: aep.planning-md/3
id: task:no-test-asserts-wall-clock-cost
kind: task
status: active
title: No test asserts a cost in wall-clock time
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o5
- derived_from: task:rendering-cost-test-passes-under-load
- decomposes: epic:zero-open-issues-2026-11
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:55:01Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-09T21:55:01Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 5}
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

## Scope

Derived 2026-11-09 by `story-scoper` at `03efc1950`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** integration tests of `crates/ekr-sdk` and `crates/ekr` — cited
- **Already converted:** `crates/ekr-views/tests/adversary_code_names.rs` (was `:121`, `:148`): the only clock use left is `fastest` at `:113-122` feeding the ratio check `grows_linearly` at `:128-134`, with no absolute bound — cited
- **Still exists, retry window:** `crates/ekr-store/src/eventlog.rs:2757-2758` (was `:2148-2150`), `last` and `elapsed` `< SQLITE_OPEN_RETRY_WINDOW` in `a_held_lock_is_retried_only_inside_the_window_and_the_bound_is_its_sum` (`:2741`) — cited
- **Product site for the injected clock:** `waiting_out_the_lock`, `crates/ekr-store/src/eventlog.rs:493-501`, reads `Instant::now()` directly — cited
- **Injected-clock pattern to copy:** `retry_unknown` takes an elapsed closure (`eventlog.rs:1913-1915`) and its test drives a `Cell` clock (`:2781-2788`) — cited
- **Still exists, `crates/ekr-sdk/tests/spawn_busy.rs`:** `:127-135` (was `:131`; under `hold`, under `BOUND` = 1 s, at least 300 ms), `:153` and `:168` (`< BOUND / 5`, unmoved) — cited
- **Still exists, `crates/ekr-sdk/tests/adversary_spawn_busy.rs`:** `:212` (`after_cancel < 1 s`, unmoved) — cited
- **Still exists, `crates/ekr-sdk/tests/session.rs`:** `:495` (was `:494`, `< 3 s`), `:709` (was `:708`, `< 1 s`) — cited
- **Still exists, `crates/ekr-sdk/tests/adversary_session_close.rs`:** `:218` (`< 3 s`, unmoved) — cited
- **Still exists, `crates/ekr/tests/view_page.rs`:** `:4038-4039` (was `:3394`), `wait_for(SETTLED && renderer, 90)` "the page settled" in `a_narrow_window_opens_compact_and_the_graph_keeps_its_width` (`:4029`) — cited
- **Still exists, `crates/ekr/tests/view_cli.rs`:** `:588` (`< 2 s`, unmoved) — cited
- **Still exists, `crates/ekr/tests/view_stream.rs`:** `:1063` (was `:1040`, `< 2 s`) — cited
- **Still exists, `crates/ekr/tests/adversary_v_view_r2.rs`:** `:198` (`< 7 s`), `:275` (`< 6500 ms`), both unmoved — cited
- **Still exists, `crates/ekr/tests/adversary_code_names_cli.rs`:** `:244` (`< 60 s`, unmoved) — cited
- **Hang guards that stay:** `crates/ekr-kernel/tests/adversary_p1_12_recovery_1.rs:363` (60 s deadline, "never released"), `crates/ekr-integrate/tests/adversary_x6_d.rs:470` (`recv_timeout` 30 s), `crates/ekr/tests/adversary_page_stream_1.rs` (`STALL_WINDOW` `:692`, `DUMP_CEILING` `:694`, `:833`); the first two are not under `crates/ekr/tests` as the body implies — cited
- **Reads as a hang guard already:** `view_page.rs:4038` is a 90 s poll with no cost claim, so it probably needs only the message and an allow-list entry; `adversary_code_names_cli.rs:244` likewise at 60 s — inferred
- **Unlisted but in the same files:** `adversary_spawn_busy.rs:181` and `:228`, `session.rs:730-736` — inferred, the acceptance grep will list them
- **Unlisted, other files matching the acceptance grep:** `crates/ekr-sdk/tests/adversary_session_transport.rs`, `crates/ekr/tests/adversary_p_page.rs`, `crates/ekr/tests/search_live.rs`, the other `wait_for` calls in `view_page.rs` — inferred, not read; `search_live.rs` may belong to `story:search-live-typing-waits-on-the-page`
- **New file:** a source guard with the hang-guard allow-list; no existing guard covers clock bounds, the nearest model is `crates/ekr/tests/adversary_x7_g_source_guard.rs` — inferred
- **Also likely:** `crates/ekr-views/src` for a work counter if a product count replaces the ratio — inferred, the converted tests may make it unnecessary
- **Documents:** `AGENTS.md:199-203` states the rule and needs no change unless the guard is to be named there — inferred
- **Confidence:** medium — every listed site was re-located by reading, but the acceptance grep reaches files the body does not list and the guard's home is a guess
- **Would collide with:** any unit touching `crates/ekr/tests/view_page.rs`, `view_stream.rs`, `view_cli.rs`, `crates/ekr-sdk/tests/session.rs`, or the open path in `crates/ekr-store/src/eventlog.rs` — cited
- **Safety fact:** `waiting_out_the_lock` is private to `eventlog.rs`, so giving it a clock parameter changes no caller outside that file — step 1 (stated from the `fn` having no `pub` at `:493`; callers not enumerated), unproven
