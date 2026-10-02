---
format: aep.planning-md/3
id: review-result:adversary-extract-07b-on-pass-1
kind: review-result
status: active
title: OCEL and word matching adversary pass one
relations:
- reviews: task:ocel-export-prints-its-counts
- reviews: task:ocel-times-events-by-a-named-property
- reviews: task:code-names-matches-whole-words
revision: 1
---
unit: OCEL counts/named event time and whole-word code names at c27f3c53efd6754c65f638212d9621777c16dfd8 plus test-only additions
verdict: NEEDS-CHANGE
cases: executed 15→19, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned FK scratch and build directories (private full inventory retained)
needs-coordinator: route SDK argv finding to consumer implementor, record review and integrate tests

1. Test-only diff

```text
 crates/ekr-sdk/tests/checks.rs | 211 +++++++++++++++++++++++++++++++++++++++++
 crates/ekr/tests/ocel_cli.rs   |  63 ++++++++++++
 2 files changed, 274 insertions(+)
```

No source or existing test was modified. The complete O/N delta, accepted task decisions, specification, authored scenarios, original tests and engine/CLI/SDK callers were read first. Prior F/K tests remain unchanged.

2. First-case executions

All four cases were authored before any execution. Each ran alone with `--exact --nocapture` before a suite.

- SDK `checks.rs:762`: an actual CLI seed admits type `-Alert.Kind` with Timestamp property `at`. The real one-shot CLI accepts `--event-time=-Alert.Kind.at` and returns an event at 2 seconds. Both SDK transports on both native providers must accept the same selector. RED: all four return Usage, `unexpected argument '-A'`. First exit 101. This asserts reachable committed state, not a fabricated canonical graph.
- SDK `checks.rs:802`: duplicate equal timestamp values and identical selectors produce one event; years 0000/9999 are writable; i64 extremes are undated and counted. Both transports/providers. First exit 0.
- SDK `checks.rs:840`: overlapping punctuation names, Unicode scalar columns, underscore/alphanumeric exclusions, combining-mark boundary, exact case and literal-mode bytes. Both transports/providers. First exit 0.
- CLI `ocel_cli.rs:399`: a session interleaves OCEL success, a named refusal, raw fact-quality output, another OCEL selection and head. Successful counts/documents match one-shot exactly and process-global stderr stays empty. Both providers. First exit 0.

SDK command prefix: `cargo test --locked -p ekr-sdk --test checks`; CLI prefix: `cargo test --locked -p ekr --test ocel_cli`. Each uses the full test name below and suffix `-- --exact --nocapture`. Complete raw logs include compiler progress. Test output follows verbatim with public-safe path aliases.

Log: `on-first-leading-dash.log` (exit 101).

```text
running 1 test
   Compiling ekr-views v0.0.26 (<worktree-FK>/crates/ekr-views)
   Compiling ekr-sdk v0.0.26 (<worktree-FK>/crates/ekr-sdk)
   Compiling ekr v0.0.26 (<worktree-FK>/crates/ekr)
    Finished `dev` profile [unoptimized] target(s) in 9.94s

thread 'adversary_sdk_event_time_treats_a_leading_dash_in_a_type_name_as_data' (530528) panicked at crates/ekr-sdk/tests/checks.rs:795:5:
valid selector names must survive SDK argv encoding: ["file session: `ocel`: error: unexpected argument '-A' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n", "file one-shot: `ocel`: error: unexpected argument '-A' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n", "sqlite session: `ocel`: error: unexpected argument '-A' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n", "sqlite one-shot: `ocel`: error: unexpected argument '-A' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test adversary_sdk_event_time_treats_a_leading_dash_in_a_type_name_as_data ... FAILED

failures:

failures:
    adversary_sdk_event_time_treats_a_leading_dash_in_a_type_name_as_data

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out; finished in 10.09s

error: test failed, to rerun pass `-p ekr-sdk --test checks`
```

Log: `on-first-adversary_named_times_deduplicate_equal_values_and_count_unwritable_instants.log` (exit 0).

```text
running 1 test
    Finished `dev` profile [unoptimized] target(s) in 0.12s
test adversary_named_times_deduplicate_equal_values_and_count_unwritable_instants ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.85s

```

Log: `on-first-adversary_word_mode_handles_overlapping_punctuation_names_and_scalar_columns.log` (exit 0).

```text
running 1 test
    Finished `dev` profile [unoptimized] target(s) in 0.10s
test adversary_word_mode_handles_overlapping_punctuation_names_and_scalar_columns ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.39s

```

Log: `on-first-diagnostics.log` (exit 0).

```text
running 1 test
test adversary_ocel_diagnostics_stay_inside_their_request_across_errors_and_raw_reports ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.09s

```

3. Subsequent suites

After all first executions: `cargo test --locked -p ekr-sdk --test checks` executed 11, with 10 passed/1 failed, exit 101. `cargo test --locked -p ekr --test ocel_cli` executed 8, all passed, exit 0. SDK before count 8 was measured afterward with precisely the three newly added SDK cases deselected (`on-baseline-sdk.log`, exit 0, 8 passed/3 filtered). CLI before count 7 is the implementor's measured lane. Thus these focused lanes executed 15→19, one red. No whole-workspace count is claimed.

Log: `on-suite-sdk.log` (exit 101).

```text
running 11 tests
    Finished `dev` profile [unoptimized] target(s) in 0.13s
test adversary_judge_stops_at_the_first_bad_batch_and_preserves_identity_order ... ok
test adversary_named_refusals_leave_the_session_usable_and_empty_draws_are_real ... ok
test adversary_historical_type_filtered_draw_survives_a_later_retraction ... ok
test adversary_sdk_event_time_treats_a_leading_dash_in_a_type_name_as_data ... FAILED
test adversary_word_mode_handles_overlapping_punctuation_names_and_scalar_columns ... ok
test adversary_named_times_deduplicate_equal_values_and_count_unwritable_instants ... ok
test fixed_judge_gets_closed_form_bounds_and_an_error_returns_no_judgement_set ... ok
test every_sample_fixture_document_is_typed_without_dropping_fields ... ok
test adversary_reports_preserve_exact_bytes_at_every_confidence_and_empty_bounds ... ok
test typed_ocel_preserves_document_and_engine_counts_in_both_transports ... ok
test sample_and_report_roundtrip_the_verbs_on_both_providers_and_transports ... ok

failures:

---- adversary_sdk_event_time_treats_a_leading_dash_in_a_type_name_as_data stdout ----

thread 'adversary_sdk_event_time_treats_a_leading_dash_in_a_type_name_as_data' (569735) panicked at crates/ekr-sdk/tests/checks.rs:795:5:
valid selector names must survive SDK argv encoding: ["file session: `ocel`: error: unexpected argument '-A' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n", "file one-shot: `ocel`: error: unexpected argument '-A' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n", "sqlite session: `ocel`: error: unexpected argument '-A' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n", "sqlite one-shot: `ocel`: error: unexpected argument '-A' found\n\nUsage: ekr ocel [OPTIONS]\n\nFor more information, try '--help'.\n"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    adversary_sdk_event_time_treats_a_leading_dash_in_a_type_name_as_data

test result: FAILED. 10 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.00s

error: test failed, to rerun pass `-p ekr-sdk --test checks`
```

Log: `on-suite-cli.log` (exit 0).

```text
running 8 tests
test a_name_no_node_type_holds_is_refused_by_name ... ok
test a_revision_beyond_the_head_is_refused_by_name_and_no_store_is_a_fault ... ok
test adversary_ocel_diagnostics_stay_inside_their_request_across_errors_and_raw_reports ... ok
test ocel_counts_are_returned_on_each_requests_stderr_without_leaking_to_the_next_request ... ok
test a_session_serves_ocel_as_the_one_shot_verb_prints_it ... ok
test events_names_the_event_types_instead_of_the_rule ... ok
test ocel_prints_each_revision_of_the_example_store_as_an_ocel_2_0_log_on_both_providers ... ok
test two_reads_of_one_revision_print_the_same_bytes_on_both_providers ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.83s

```

Log: `on-baseline-sdk.log` (exit 0).

```text
running 8 tests
    Finished `dev` profile [unoptimized] target(s) in 0.10s
test adversary_judge_stops_at_the_first_bad_batch_and_preserves_identity_order ... ok
test adversary_named_refusals_leave_the_session_usable_and_empty_draws_are_real ... ok
test adversary_historical_type_filtered_draw_survives_a_later_retraction ... ok
test fixed_judge_gets_closed_form_bounds_and_an_error_returns_no_judgement_set ... ok
test every_sample_fixture_document_is_typed_without_dropping_fields ... ok
test typed_ocel_preserves_document_and_engine_counts_in_both_transports ... ok
test sample_and_report_roundtrip_the_verbs_on_both_providers_and_transports ... ok
test adversary_reports_preserve_exact_bytes_at_every_confidence_and_empty_bounds ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 1.38s

```

Formatting (`cargo fmt --all -- --check`, on-fmt.log/.exit) and `git diff --check` exited 0. No implementation fix, weakened assertion or lint suppression was applied.

4. Finding

| File:line | Verdict | Origin | Measured | What reaches it |
|---|---|---|---|---|
| crates/ekr-sdk/src/read/ocel.rs:207 | NEEDS-CHANGE | introduced | checks.rs:795 fails after successful seed and exact CLI control; first run exits 101 and both SDK transports on File/SQLite report Usage | public Reader::ocel / OneShotReader::ocel with an admitted type name beginning `-`; the identical selector succeeds as `--event-time=-Alert.Kind.at` |

The SDK appends a selector as a separate argv element. Clap interprets its leading dash as an option. Encode each selector as one `--event-time=<selector>` argument (or provide equivalent value-safe parsing); do not narrow the admitted ontology naming contract. The new SDK OCEL method did not exist before O/N, so the origin is introduced. This finding makes no claim about malformed names or an unsafe write: it prevents a valid read.

Owners: 1 finding, 0 coordinator, 1 implementor. Root has routed the source repair to the consumer implementor. This pass did not apply or inspect that correction.

5. Attacks without a finding

- Equal timestamp values/selectors deduplicate; boundary years export and out-of-range timestamps become counted undated events.
- Whole-word matching honors Unicode alphanumerics, underscores, combining-mark boundaries, overlapping punctuation names and scalar columns; old literal mode remains byte-identical for the probe.
- OCEL counts remain per-request through success/refusal/raw-report/success sequences; process-global stderr stays empty and one-shot documents/counts match.

6. Outside-tree files and handoff

Test-only commit ba27141b34c5ced39ae90198e4bc1776b05ff656 on review/x7b-fk; author and committer verified b10x-bot[bot]. Tree clean; own lease released; build slot returned to consumer. No push, cleanup or planning write occurred.

All `on-*.log`, matching `.exit`, on-attack.patch, on-diff-stat.txt, on-commit-message.txt, report-on.md and on-outside-paths.txt are under <cache>/ekr-extract-07b/review-fk. Assigned compilation/test outputs remain under <cache>/b10x-target/ekr-x7b-review-fk. Private full paths are retained in on-outside-paths.txt; public report paths use aliases. Prior F/K evidence is preserved.

```findings
- file: crates/ekr-sdk/src/read/ocel.rs
  line: 207
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The SDK passes valid leading-dash event-time selectors as separate argv values, so both transports refuse names the CLI accepts with equals syntax.
```
