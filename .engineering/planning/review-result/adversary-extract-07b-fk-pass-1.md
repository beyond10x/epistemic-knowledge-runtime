---
format: aep.planning-md/3
id: review-result:adversary-extract-07b-fk-pass-1
kind: review-result
status: active
title: Empty intervals and judged SDK samples, adversary pass
relations:
- reviews: story:sdk-store-checks
- reviews: task:fact-quality-states-the-empty-interval
revision: 1
---
unit: task:fact-quality-states-the-empty-interval + story:sdk-store-checks at 4360a9f1c1bfb8b67da59ce01a058f34e7e4c6a6 plus test-only additions
verdict: nothing found
cases: executed 18→22, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned FK scratch and build directories (private full inventory retained)
needs-coordinator: record review, integrate tests and run combined task check

1. Test-only diff

```text
 crates/ekr-sdk/tests/checks.rs | 292 +++++++++++++++++++++++++++++++++++++++++
 1 file changed, 292 insertions(+)
```

The complete F/K delta, current acceptance, new specification/documentation, original tests and actual read/CLI/sample callers were read before writing these four cases. No implementation, pre-existing case or planning file was edited. Before counts come from the implementor: SDK checks 3, views sample 15. These are focused lanes, not a whole-workspace count.

2. First execution of each added case

All four cases were authored before the first cargo test. Each was selected alone with `--exact --nocapture`, before either suite. Every first execution passed; no red count is inferred. The first report case checks 19,998 individual reports inside one Rust case (two populations, confidence 1–9999).

- `checks.rs:302`: judge batches retain assertion identity and order, report the exact second-batch error/count offset, and skip calling the judge for an empty sample.
- `checks.rs:403`: typed reports reserialize to the engine's exact bytes at every admitted confidence, with extreme origin integers and optional type; empty reports state explicit null/0/1, including summary bounds, while nonempty reports contain the expected counts and enclose 3/7.
- `checks.rs:454`: on File and SQLite, an actual proposed/validated/committed retraction changes the current population by exactly one while both SDK transports preserve the type-filtered historical sample and its printed document bytes.
- `checks.rs:525`: size/confidence extremes and duplicate judgements return exact named refusals; the same session then answers a valid report and unchanged draw. A nonexistent exact type yields an empty sample with explicit zero population/drawn, after a nonempty control.

Exact first commands have prefix `cargo test --locked -p ekr-sdk --test checks`, the case name printed below, and suffix `-- --exact --nocapture`. First-case exit markers are all 0. Build progress is retained in the corresponding full raw logs; test output follows verbatim with public-safe path aliases.

Log: `first-report-bytes.log` (exit 0).

```text
running 1 test
test adversary_reports_preserve_exact_bytes_at_every_confidence_and_empty_bounds ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.72s

```

Log: `first-adversary_judge_stops_at_the_first_bad_batch_and_preserves_identity_order.log` (exit 0).

```text
running 1 test
   Compiling hashbrown v0.17.1
   Compiling indexmap v2.14.2
   Compiling hashlink v0.12.2
   Compiling rusqlite v0.40.2
   Compiling serde_yaml_ng v0.10.0 (<worktree-FK>/vendor/serde_yaml_ng)
   Compiling serde_yaml v0.9.34+deprecated
   Compiling eventlog-sqlite v0.5.0 (https://github.com/beyond10x/eventlog?rev=fe8a0a7e6e97afde87b349f0840d6e2ed28df3f8#fe8a0a7e)
   Compiling ess-primitives v0.36.0 (https://github.com/beyond10x/ess?rev=be44a3365eb273cb3d447b74cd5b0e75181d284e#be44a336)
   Compiling ekr-core v0.0.26 (<worktree-FK>/crates/ekr-core)
   Compiling ekr-ontology v0.0.26 (<worktree-FK>/crates/ekr-ontology)
   Compiling ekr-graph v0.0.26 (<worktree-FK>/crates/ekr-graph)
   Compiling ess-domain v0.36.0 (https://github.com/beyond10x/ess?rev=be44a3365eb273cb3d447b74cd5b0e75181d284e#be44a336)
   Compiling ekr-integrate v0.0.26 (<worktree-FK>/crates/ekr-integrate)
   Compiling ekr-store v0.0.26 (<worktree-FK>/crates/ekr-store)
   Compiling ekr-sdk v0.0.26 (<worktree-FK>/crates/ekr-sdk)
   Compiling ekr-kernel v0.0.26 (<worktree-FK>/crates/ekr-kernel)
   Compiling ekr-views v0.0.26 (<worktree-FK>/crates/ekr-views)
   Compiling ess-compiler v0.36.0 (https://github.com/beyond10x/ess?rev=be44a3365eb273cb3d447b74cd5b0e75181d284e#be44a336)
   Compiling ess-gen v0.36.0 (https://github.com/beyond10x/ess?rev=be44a3365eb273cb3d447b74cd5b0e75181d284e#be44a336)
   Compiling ess-conformance v0.36.0 (https://github.com/beyond10x/ess?rev=be44a3365eb273cb3d447b74cd5b0e75181d284e#be44a336)
   Compiling ekr v0.0.26 (<worktree-FK>/crates/ekr)
test adversary_judge_stops_at_the_first_bad_batch_and_preserves_identity_order has been running for over 60 seconds
    Finished `dev` profile [unoptimized] target(s) in 1m 00s
test adversary_judge_stops_at_the_first_bad_batch_and_preserves_identity_order ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 60.73s

```

Log: `first-adversary_historical_type_filtered_draw_survives_a_later_retraction.log` (exit 0).

```text
running 1 test
    Finished `dev` profile [unoptimized] target(s) in 0.09s
test adversary_historical_type_filtered_draw_survives_a_later_retraction ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.29s

```

Log: `first-adversary_named_refusals_leave_the_session_usable_and_empty_draws_are_real.log` (exit 0).

```text
running 1 test
    Finished `dev` profile [unoptimized] target(s) in 0.09s
test adversary_named_refusals_leave_the_session_usable_and_empty_draws_are_real ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.23s

```

3. Subsequent suites

Both ran after every first-case run completed. SDK command: `cargo test --locked -p ekr-sdk --test checks`. Views command: `cargo test --locked -p ekr-views --test sample`. Runtime output is retained below; full compiler output and matching exit markers remain in the named logs.

Log: `suite-sdk.log` (exit 0).

```text
running 7 tests
    Finished `dev` profile [unoptimized] target(s) in 0.08s
test adversary_judge_stops_at_the_first_bad_batch_and_preserves_identity_order ... ok
test adversary_named_refusals_leave_the_session_usable_and_empty_draws_are_real ... ok
test adversary_historical_type_filtered_draw_survives_a_later_retraction ... ok
test fixed_judge_gets_closed_form_bounds_and_an_error_returns_no_judgement_set ... ok
test every_sample_fixture_document_is_typed_without_dropping_fields ... ok
test sample_and_report_roundtrip_the_verbs_on_both_providers_and_transports ... ok
test adversary_reports_preserve_exact_bytes_at_every_confidence_and_empty_bounds ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.85s

```

Log: `suite-views.log` (exit 0).

```text
running 15 tests
test a_confidence_outside_1_to_9999_and_a_fact_judged_twice_are_refused ... ok
test a_size_outside_1_to_1000_is_refused_before_the_store_is_read ... ok
test a_fact_about_an_edge_names_its_edge_type_and_no_subject_name ... ok
test an_unseeded_store_and_a_revision_beyond_the_head_are_refused_by_name ... ok
test a_later_commit_leaves_an_earlier_revisions_sample_as_it_was ... ok
test for_a_judged_fixture_of_known_results_the_rate_and_interval_equal_the_closed_form_wilson_values ... ok
test no_judgement_reports_an_explicit_null_rate_and_the_vacuous_interval ... ok
test the_document_carries_the_request_and_each_fact_with_its_names_and_evidence_bytes ... ok
test a_type_filter_draws_only_the_facts_whose_subject_is_of_that_type ... ok
test the_judgements_document_reads_and_writes_its_format_and_refuses_anything_else ... ok
test the_report_is_the_same_bytes_for_the_same_input_and_echoes_the_sample ... ok
test another_seed_draws_another_sample_and_a_larger_size_extends_the_smaller ... ok
test z_is_the_published_normal_quantile_at_the_stated_confidence ... ok
test the_draw_key_bounds_formats_and_interval_functions_are_what_the_draw_and_report_use ... ok
test the_same_seed_size_and_revision_draw_the_same_sample_on_both_providers ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s

```

Formatting (`cargo fmt --all -- --check`) and `git diff --check` exit 0. No whole-workspace gate or additional lint run is claimed by this test-only pass. Build settings: three compile/test jobs, sccache, incremental/debug disabled, assigned distinct target; free space checked before starting. No timing assertion was added.

After execution, the new case's even-index expression was spelled with `is_multiple_of(2)` instead of the equivalent `% 2 == 0` to match the pinned lint convention; formatting was rechecked. The behavioral assertions are unchanged; the coordinator's combined gate will compile that final spelling.

4. Findings

Nothing found.

| File:line | Verdict | Origin | Measured | What reaches it |
|---|---|---|---|---|
| — | — | — | No failing behavioral case | — |

Owners: 0 findings, 0 coordinator, 0 implementor. This result covers F/K at the exact reviewed commit plus these tests, not later consumer work or the combined wave.

5. Attacks without a finding

- Exact binary64 document bytes and vacuous empty interval across every admitted confidence.
- Batch ordering, assertion identity, second-batch failure/count reporting and empty input.
- Historical exact-type sampling after real retraction on both native providers and both SDK transports.
- Named size/confidence/duplicate-judgement refusals, session recovery and empty type populations.

6. Outside-tree writes and handoff

All raw logs, exit markers, attack.patch, diff-stat.txt, diff-numstat.txt, commit-message.txt and this report stay under <cache>/ekr-extract-07b/review-fk. Disposable compilation and test output stay under <cache>/b10x-target/ekr-x7b-review-fk. The private full-path inventory is outside-paths.txt. No push, cleanup or planning write occurred. Root owns recording and integration.

Test commit: 9bf163c65ff06a7a29bfa04f5c628744b96c80a6 on review/x7b-fk. Author and committer both verified b10x-bot[bot]. The tree is clean and own lease released. The build slot was returned to W before committing.

```findings
[]
```
