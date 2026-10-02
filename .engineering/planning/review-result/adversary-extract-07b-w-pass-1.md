---
format: aep.planning-md/3
id: review-result:adversary-extract-07b-w-pass-1
kind: review-result
status: active
title: Replay adversary and full-size cost measurement
relations:
- reviews: task:validate-cost-flat-with-store-size
revision: 1
---
unit: task:validate-cost-flat-with-store-size at a8f087094 plus test-only adversary additions
verdict: NEEDS-CHANGE (measured acceptance); no new replay correctness finding
cases: executed 25→27, red 0
origin: introduced 0 / pre-existing 0 / undecided 1
wrote-outside-worktree: assigned W scratch and build directories
needs-coordinator: keep performance task active until its measured acceptance passes

1. Diff proof

```text
 crates/ekr-kernel/tests/add_evidence.rs | 59 +++++++++++++++++++++++++++++++++
 crates/ekr-store/tests/history_cache.rs | 40 ++++++++++++++++++++++
 2 files changed, 99 insertions(+)
```

Only tests changed. The before count is the implementor's nine add-evidence and sixteen history-cache cases; the measured after count is ten and seventeen, with one existing measurement helper ignored. No implementation was changed during this attack.

2. New cases, run alone first

The first drives a checkpoint-backed handle after a peer records a validation at revision 1 while the head is revision 6. It compares complete transaction records against full replay, then requires the old-basis commit to become Stale. The second supplies an authority replay hint naming nonexistent, unnecessary bytes, requiring complete-history fallback to return the same successful occurrences on repeated calls. Both exercise both native providers.

`cargo test --locked -p ekr-kernel --test add_evidence adversary_peer_validation_at_an_old_basis_matches_full_replay -- --exact --nocapture`, exit 0:

```text
   Compiling ekr-kernel v0.0.26 (<worktree-W>/crates/ekr-kernel)
    Finished `test` profile [unoptimized] target(s) in 1.16s
     Running tests/add_evidence.rs (<cache>/b10x-target/ekr-x7b-w/debug/deps/add_evidence-5f21f3e2bbbcf436)

running 1 test
test adversary_peer_validation_at_an_old_basis_matches_full_replay ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.44s

```

`cargo test --locked -p ekr-store --test history_cache adversary_a_spurious_missing_replay_object_cannot_refuse_a_valid_history -- --exact --nocapture`, exit 0:

```text
    Blocking waiting for file lock on package cache
   Compiling ekr-store v0.0.26 (<worktree-W>/crates/ekr-store)
    Finished `test` profile [unoptimized] target(s) in 3.30s
     Running tests/history_cache.rs (<cache>/b10x-target/ekr-x7b-w/debug/deps/history_cache-0ea899aeefffe902)

running 1 test
test adversary_a_spurious_missing_replay_object_cannot_refuse_a_valid_history ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 17 filtered out; finished in 0.02s

```

3. Subsequent suite

`cargo test --locked -p ekr-store --test history_cache -p ekr-kernel --test add_evidence -- --nocapture`, exit 0:

```text
    Finished `test` profile [unoptimized] target(s) in 0.16s
     Running tests/add_evidence.rs (<cache>/b10x-target/ekr-x7b-w/debug/deps/add_evidence-5f21f3e2bbbcf436)

running 10 tests
test evidence_extracted_by_another_agent_than_the_proposer_is_misattributed ... ok
test an_assertion_citing_evidence_no_revision_holds_is_refused_as_unresolved_evidence ... ok
test evidence_from_a_source_other_than_a_human_statement_is_refused_by_name ... ok
test a_payload_that_does_not_hash_to_its_entry_is_refused_as_evidence_payload_mismatch ... ok
test a_reused_evidence_id_is_refused_by_name ... ok
test added_evidence_and_the_assertion_citing_it_commit_and_replay_on_both_providers ... ok
test a_verified_read_still_holds_every_added_payload ... ok
file false: retained objects loaded into command histories: 154 / 154
test cached_commands_refuse_withdrawn_evidence_as_complete_history_does ... ok
test adversary_peer_validation_at_an_old_basis_matches_full_replay ... ok
file true: retained objects loaded into command histories: 154 / 154
test a_command_loads_the_same_objects_however_much_evidence_the_store_holds ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s

     Running tests/history_cache.rs (<cache>/b10x-target/ekr-x7b-w/debug/deps/history_cache-0ea899aeefffe902)

running 18 tests
test measure_two_history_reads_on_one_handle ... ignored, measurement; prints timings
test a_tampered_copy_of_a_verified_object_is_refused_by_content_file ... ok
test a_later_read_on_one_handle_copies_no_retained_object_file ... ok
test a_file_store_that_diverged_is_refused_as_a_typed_divergence ... ok
test a_held_object_is_not_served_from_a_file_store_that_diverged ... ok
test a_blob_damaged_before_its_first_load_is_still_refused_file ... ok
test an_occurrence_appended_through_another_handle_is_seen_by_the_next_read_file ... ok
test a_second_read_on_one_handle_reads_and_hashes_no_blob_the_first_verified_file ... ok
test a_read_on_confirms_the_held_prefix_in_the_same_call_file ... ok
test a_tampered_copy_of_a_verified_object_is_refused_by_content_sqlite ... ok
test a_later_read_on_one_handle_copies_no_retained_object_sqlite ... ok
test an_occurrence_appended_through_another_handle_is_seen_by_the_next_read_sqlite ... ok
test a_read_on_confirms_the_held_prefix_in_the_same_call_sqlite ... ok
test adversary_a_spurious_missing_replay_object_cannot_refuse_a_valid_history ... ok
test a_second_read_on_one_handle_reads_and_hashes_no_blob_the_first_verified_sqlite ... ok
test a_blob_damaged_before_its_first_load_is_still_refused_sqlite ... ok
test an_incomplete_replay_hint_falls_back_to_the_complete_history ... ok
test a_sqlite_store_overwritten_in_place_is_refused_as_replaced ... ok

test result: ok. 17 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.05s

```

4. Acceptance finding

The implementor's separate default-size SQLite measurement finished during this review. It is not a case this adversary authored. Its release binary was built from a8f087094; later changes in this tree were the two tests above only. The 20,000-fact base, 10,000-fact small delta and 80,000-fact large delta match task acceptance. This observed result blocks completion even though the replay attack cases passed.

Measured log excerpt (scaling.log, exit 101):

```text
  large transaction 106            propose   1171.7 ms  validate    180.9 ms  commit   1158.9 ms  total   2511.5 ms
  large transaction 107            propose    934.3 ms  validate    225.0 ms  commit   1183.9 ms  total   2343.2 ms
  large transaction 108            propose    923.1 ms  validate    211.5 ms  commit   1216.4 ms  total   2351.0 ms
  large transaction 109            propose    967.8 ms  validate    175.7 ms  commit   4578.4 ms  total   5722.0 ms
  large transaction 110            propose    906.9 ms  validate    268.5 ms  commit   1244.6 ms  total   2420.0 ms
  large transaction 111            propose    995.6 ms  validate    182.7 ms  commit   1217.3 ms  total   2395.6 ms
  large transaction 112            propose   1065.7 ms  validate    192.7 ms  commit   1208.1 ms  total   2466.5 ms
  large transaction 113            propose    936.0 ms  validate    236.0 ms  commit   1155.1 ms  total   2327.1 ms
  large transaction 114            propose    851.7 ms  validate    140.7 ms  commit   4105.0 ms  total   5097.4 ms
  large transaction 115            propose    734.0 ms  validate    318.4 ms  commit    891.2 ms  total   1943.6 ms
  large transaction 116            propose    758.8 ms  validate    131.7 ms  commit    865.0 ms  total   1755.5 ms
  large transaction 117            propose    738.0 ms  validate    130.0 ms  commit    971.4 ms  total   1839.5 ms
  large transaction 118            propose    904.8 ms  validate    154.0 ms  commit    954.1 ms  total   2012.9 ms
  large transaction 119            propose    885.6 ms  validate    167.9 ms  commit   3973.3 ms  total   5026.8 ms
  large transaction 120            propose    376.2 ms  validate    224.3 ms  commit    572.8 ms  total   1173.3 ms
Sqlite first 15 medians            propose    645.5 ms  validate     76.0 ms  commit    648.5 ms  total   1370.0 ms
Sqlite last 15 medians             propose    923.1 ms  validate    182.7 ms  commit   1208.1 ms  total   2313.9 ms
Sqlite median ratios: validate 2.404x; commit 1.863x; load 14.50 13.53 14.69 8/6629 90184
Sqlite small delta, last 3         propose    612.8 ms  validate     81.6 ms  commit    885.3 ms  total   1579.7 ms
Sqlite large delta, first 3        propose    626.1 ms  validate     58.7 ms  commit    516.4 ms  total   1201.2 ms
Sqlite large delta, last 3         propose    842.8 ms  validate    150.7 ms  commit   1966.3 ms  total   2959.7 ms
Sqlite: large/small 1.87x, large last/first 2.46x

thread 'a_transaction_costs_the_same_in_a_large_delta_as_in_a_small_one' (4154941) panicked at crates/ekr-sdk/tests/commit_scaling.rs:484:5:

Sqlite: a transaction at the end of the large delta costs 1.87x one at the end of the small delta
Sqlite: the last transactions of the large delta cost 2.46x its first
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test a_transaction_costs_the_same_in_a_large_delta_as_in_a_small_one ... FAILED

failures:

failures:
    a_transaction_costs_the_same_in_a_large_delta_as_in_a_small_one

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 377.77s

error: test failed, to rerun pass `-p ekr-sdk --test commit_scaling`
[ perf record: Woken up 19 times to write data ]
[ perf record: Captured and wrote 6.043 MB <cache>/ekr-extract-07b/w/scaling.perf.data (27745 samples) ]
MEASUREMENT_EXIT=101
Fri Oct  2 11:15:03 UTC 2026
 13:15:03 up 22 days, 22:10,  1 user,  load average: 14.50, 13.53, 14.69
Filesystem      Size  Used Avail Use% Mounted on
/dev/nvme0n1p2  848G  774G   31G  97% /
```

| file | verdict | origin | measured | reaches it |
|---|---|---|---|---|
| crates/ekr-sdk/tests/commit_scaling.rs | NEEDS-CHANGE | undecided | validate first/last 15 medians 76.0→182.7 ms (2.404x); commit 648.5→1208.1 ms (1.863x); required bound 1.2x each; old aggregate assertions also fail | documented ignored release benchmark at default sizes, explicitly selecting SQLite |

This review did not execute the base benchmark, so it does not attribute the remaining cost to a new regression. It establishes that the current change has not achieved the task's acceptance. The counted-object regression passes but does not substitute for the per-verb measurement.

Owners: 1 finding, 0 coordinator, 1 implementor (remaining acceptance work; regression origin undecided).

5. Attacks without a new correctness finding

- A peer's historical validation after checkpoint restore matches full replay and cannot commit over a moved head.
- An overinclusive replay hint with missing bytes cannot change complete-history success into refusal, including retries.
- Existing withdrawal, replacement, read-cost and retained-evidence cases remain green in the attacked targets.

6. Outside-tree writes

<cache>/ekr-extract-07b/w/adversary-old-basis-first.log and .exit; adversary-spurious-first.log and .exit; adversary-suite-first.log and .exit; adversary-pass-1.raw.md; adversary-pass-1.md. Compiler output: <cache>/b10x-target/ekr-x7b-w. The measurement/profile files belong to the implementation report. Public log copies replace machine roots with declared aliases; originals remain intact.

```findings
- file: crates/ekr-sdk/tests/commit_scaling.rs
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: undecided
  message: Default-size SQLite validate and commit median ratios are 2.404x and 1.863x, exceeding the required 1.2x bound for each verb.
```
