---
format: aep.planning-md/3
id: review-result:adversary-extract-07b-on-pass-2
kind: review-result
status: active
title: OCEL opaque selectors adversary pass two
relations:
- reviews: task:ocel-export-prints-its-counts
- reviews: task:ocel-times-events-by-a-named-property
- reviews: task:code-names-matches-whole-words
revision: 1
---
unit: OCEL selector correction b0ac27c792054b3b7f8c4161e7a1bf46e0af68f8 in consumer tree 8ccf39a0e5bc1763e5d473976f8aab3c117f6328 plus one test-only addition
verdict: NEEDS-CHANGE
cases: executed 11→12, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned FK scratch and consumer build target
needs-coordinator: record resolved timestamp selector and route remaining events encoding to implementor

1. Test-only diff

```text
 crates/ekr-sdk/tests/checks.rs | 64 ++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 64 insertions(+)
```

Read the one-line correction before attacking: timestamp selectors now form one `--event-time=<value>` argument. No implementation or prior case was changed. The new case was written before any test execution, and tests ran before the consumer began the next source correction.

2. First case and prior regression

New case `adversary_ocel_selectors_preserve_equals_spaces_and_punctuation_as_data` seeds actual types named `=Alert.Kind`, `Alert with spaces`, and `--Alert=two Kind`, then uses actual successful CLI equals-form controls for both OCEL selection modes. Repeated identical selections are sent through session and one-shot SDK transports on File and SQLite. The first execution exits 101: only `events` with the leading-dash name fails, in all four provider/transport combinations. The `event_time` matrix passes.

The original independent `adversary_sdk_event_time_treats_a_leading_dash_in_a_type_name_as_data` then runs unchanged and passes (exit 0). The prior finding at read/ocel.rs:207 is resolved; the remaining events list at :204 was not changed by that correction. These are different argv paths of the originally introduced SDK method.

Commands: `cargo test --locked -p ekr-sdk --test checks <case> -- --exact --nocapture`. Exact runtime outputs follow; full logs retain compiler progress.

Log: `on2-first-selectors.log` (exit 101).

```text
running 1 test
    Finished `dev` profile [unoptimized] target(s) in 0.17s

thread 'adversary_ocel_selectors_preserve_equals_spaces_and_punctuation_as_data' (856484) panicked at crates/ekr-sdk/tests/checks.rs:966:5:
OCEL names must survive SDK argv encoding in both modes: ["file session events \"--Alert=two Kind\": `ocel`: error: unexpected argument '--Alert' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n", "file one-shot events \"--Alert=two Kind\": `ocel`: error: unexpected argument '--Alert' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n", "sqlite session events \"--Alert=two Kind\": `ocel`: error: unexpected argument '--Alert' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n", "sqlite one-shot events \"--Alert=two Kind\": `ocel`: error: unexpected argument '--Alert' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test adversary_ocel_selectors_preserve_equals_spaces_and_punctuation_as_data ... FAILED

failures:

failures:
    adversary_ocel_selectors_preserve_equals_spaces_and_punctuation_as_data

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 11 filtered out; finished in 1.18s

error: test failed, to rerun pass `-p ekr-sdk --test checks`
```

Log: `on2-prior-regression.log` (exit 0).

```text
running 1 test
    Finished `dev` profile [unoptimized] target(s) in 0.29s
test adversary_sdk_event_time_treats_a_leading_dash_in_a_type_name_as_data ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.51s

```

3. Subsequent suite

`cargo test --locked -p ekr-sdk --test checks`: 11 passed, 1 failed, exit 101. Before count 11 is the consumer's correction suite, reported before this pass. Formatting and diff check exit 0. No broad gate or timing result is claimed.

```text
running 12 tests
    Finished `dev` profile [unoptimized] target(s) in 0.11s
test adversary_judge_stops_at_the_first_bad_batch_and_preserves_identity_order ... ok
test adversary_named_refusals_leave_the_session_usable_and_empty_draws_are_real ... ok
test adversary_historical_type_filtered_draw_survives_a_later_retraction ... ok
test adversary_named_times_deduplicate_equal_values_and_count_unwritable_instants ... ok
test adversary_ocel_selectors_preserve_equals_spaces_and_punctuation_as_data ... FAILED
test adversary_sdk_event_time_treats_a_leading_dash_in_a_type_name_as_data ... ok
test adversary_reports_preserve_exact_bytes_at_every_confidence_and_empty_bounds ... ok
test every_sample_fixture_document_is_typed_without_dropping_fields ... ok
test fixed_judge_gets_closed_form_bounds_and_an_error_returns_no_judgement_set ... ok
test adversary_word_mode_handles_overlapping_punctuation_names_and_scalar_columns ... ok
test typed_ocel_preserves_document_and_engine_counts_in_both_transports ... ok
test sample_and_report_roundtrip_the_verbs_on_both_providers_and_transports ... ok

failures:

---- adversary_ocel_selectors_preserve_equals_spaces_and_punctuation_as_data stdout ----

thread 'adversary_ocel_selectors_preserve_equals_spaces_and_punctuation_as_data' (866899) panicked at crates/ekr-sdk/tests/checks.rs:966:5:
OCEL names must survive SDK argv encoding in both modes: ["file session events \"--Alert=two Kind\": `ocel`: error: unexpected argument '--Alert' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n", "file one-shot events \"--Alert=two Kind\": `ocel`: error: unexpected argument '--Alert' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n", "sqlite session events \"--Alert=two Kind\": `ocel`: error: unexpected argument '--Alert' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n", "sqlite one-shot events \"--Alert=two Kind\": `ocel`: error: unexpected argument '--Alert' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    adversary_ocel_selectors_preserve_equals_spaces_and_punctuation_as_data

test result: FAILED. 11 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.77s

error: test failed, to rerun pass `-p ekr-sdk --test checks`
```

4. Finding

| File:line | Verdict | Origin | Measured | What reaches it |
|---|---|---|---|---|
| crates/ekr-sdk/src/read/ocel.rs:204 | NEEDS-CHANGE | introduced | checks.rs:966 fails, first exit 101; File/SQLite and both SDK transports return Usage for the admitted name while every CLI control succeeds | public OcelQuery.events with `--Alert=two Kind`; one-shot `--events=--Alert=two Kind` exports it successfully |

Encode every events name as its own `--events=<name>` argument so names remain values. This is residue in the new SDK method, not a pre-existing engine or CLI name refusal. No source correction was applied by this reviewer.

Owners: 1 finding, 0 coordinator, 1 implementor. Prior timestamp-selector finding resolved; one newly measured events-path finding remains.

5. Attacks without a finding

- Corrected event-time encoding handles leading dashes, additional equals, spaces and punctuation.
- Repeated selectors and normal equals/space names preserve the CLI document through both SDK transports and providers.
- Requests after the tested read remain usable and carry no stale diagnostics.

6. Outside-tree files and handoff

Test commit 4ed8d5a3b3ea77d91fcac168781ce8c6d7646bec on impl/x7b-consumer; author and committer verified b10x-bot[bot]. Tree clean at handoff, own review lease released, consumer target/slot returned before its next source correction. No push, cleanup or planning write.

`on2-case.rs`, `on2-*.log`, matching `.exit`, on2-commit-message.txt and report-on2.md are under <cache>/ekr-extract-07b/review-fk. Builds/tests used the exclusively handed-over <cache>/b10x-target/ekr-x7b-consumer. The private full inventory is on2-outside-paths.txt. Other review outputs were preserved.

```findings
- file: crates/ekr-sdk/src/read/ocel.rs
  line: 204
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The SDK events list still passes leading-dash type names as separate argv values, so both transports refuse admitted names accepted by the CLI equals form.
```
