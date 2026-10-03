unit: story:propose-better-vocabulary — bounded physical proposal review retention
verdict: green
cases: passed 193→201, ignored 3→3, behavioral red 8
origin: n/a
wrote-outside-worktree: assigned evidence directory, assigned Cargo lane, managed lease state
needs-coordinator: review and bot-commit dirty source; no out-of-scope patch

1. Unit and acceptance

Atomically retain generated review records, separate review/decision identity bindings and exact proof/policy/statement bytes at Provenance rank or stronger. Require an exact retained proposal, deduplicate exact retries before human-predecessor CAS, and refuse inconsistent input, reused identity input and stale predecessors. Both file and SQLite providers are exercised.

Tree: <review-retention-worktree>
Branch: ekr/schema-review-retention-20261003
Base: f39ea736eaafeeed87e7a1bd516dbc3973f38157

Owned files changed:
- crates/ekr-store/src/proposal_reviews.rs (new, 278 lines)
- crates/ekr-store/src/eventlog.rs (module/export only)
- crates/ekr-store/src/lib.rs (export only)
- crates/ekr-store/tests/proposal_review_retention.rs (new, 415 lines)

Read active story, scope and artifact graph; A foundation is present in source although broad story remains active. Prior inferred Taskfile/design paths remain outside this bounded owned slice; none was changed. Existing atomic blob/event CAS mechanism is reused and exercised by observed red-to-green concurrent-writer cases. No dependency, generated-contract, AEP, kernel, CLI or SDK changes.

2. Actual diff shape

The git diff --stat appended below reports tracked files only. The two new untracked files are included in the explicit inventory above; no staging or commit was performed.

3. Red evidence

Every Cargo invocation used:
env -u RUSTC_WRAPPER CARGO_TARGET_DIR=<owned-build-lane>/target TMPDIR=<owned-build-lane>/tmp CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_STRIP=symbols CARGO_PROFILE_TEST_STRIP=symbols CARGO_NET_OFFLINE=true

After adding tests and an explicitly refusing API skeleton, cargo test -p ekr-store --test proposal_review_retention executed eight cases: zero passed, eight failed, zero ignored, exit101. This is preserved verbatim in red.log and appended below. Persistence implementation was written only after that run.

4. Green evidence

cargo test -p ekr-store: base193 passed/3 ignored → implementation201 passed/3 ignored, exit0 for both (base.log, green-package.log). Totals sum each test runner's own summary, including child subprocess summaries as in the existing package reporting convention. No filtered cases.

cargo test -p ekr-store --test proposal_review_retention: executed8→8, failed8→0, final8 passed/0 ignored, exit0. Both green-target.log and green-target-final.log preserve the summaries. No target existed on base.

cargo clippy -p ekr-store --all-targets -- -D warnings: initial exit101 for replace_box in a test, fixed by mutating the existing boxed enum value; clippy-fixed.log exit0. This corrected both identical test allocations. No implementation changed after the package201 run; the final focused8 run rechecked the affected tests.

rustfmt --edition 2021 --config skip_children=true --check on exactly the four owned files: exit0 (format-final.log/status). git diff --check: exit0. No cargo fmt --all.

All build processes reached terminal completion: exec sessions 95204 (base), 40097 (red), 25793 (focused green), 85072 (package green), 1325 (initial clippy), 69012 (corrected clippy), 86333 (final focused/format). Wrapper shell exit0 is not substituted for Cargo status: each actual Cargo status is retained in its .status file. Exclusive shared lane is returned to root on handoff. No cleanup or simultaneous builds.

5. Boundaries and remaining risks

Physical retention does not authorize a decision. Protocol proof/policy/statement digests, signatures, targets, operator authentication and the proof's expected predecessor are verified by the kernel. This port validates actual ContentHash::of_bytes object addresses, canonical hash/identity spellings and duplicated generated record fields; it deliberately does not parse kernel-owned binary proof/policy encodings or recompute raw protocol SHA-256. This boundary was explicitly confirmed by the coordinator and is documented on the trait. Review and decision identities remain distinct and are each globally bound atomically.

Read paths fail closed on unknown event/version, mismatched proposals, malformed records, duplicate identity/proof digest, missing/mismatched singleton bindings and missing/changed/unpinned bytes. Direct injection of malformed native event envelopes is not part of this new target. F publication markers are not implemented or invented: future support must count physical events separately from the effective human predecessor. CAS is bounded to the existing16-retry pattern. Statement evidence admission and cross-review evidence-identity semantics remain kernel responsibilities. No canonical writer, signature approval, full task check or implementation conformance claim.

6. Outside writes and handoff

Evidence directory: <retained-evidence>/schema-review-retention/
Files written there (each is a direct child of that absolute directory): base.log, base.status, red.log, red.status, green-target.log, green-target.status, green-package.log, green-package.status, clippy.log, clippy.status, clippy-fixed.log, clippy-fixed.status, green-target-final.log, green-target-final.status, format.log, format.status, format-final.log, format-final.status, report.md.

Cargo wrote assigned <owned-build-lane>/target and <owned-build-lane>/tmp plus ordinary Cargo metadata. Worktree CLI owns lease bookkeeping. Own lease codex-ekr-schema-review-retention-20261003 is released on handoff; tree stays dirty and preserved for coordinator review and bot commit. No AEP writes, commits, pushes, cleanup or publication.

Verbatim tracked diff stat, red output, green package output, corrected clippy output, and final focused output follow:
 crates/ekr-store/src/eventlog.rs | 3 +++
 crates/ekr-store/src/lib.rs      | 1 +
 2 files changed, 4 insertions(+)
   Compiling ekr-store v0.0.27 (<review-retention-worktree>/crates/ekr-store)
    Finished `test` profile [unoptimized] target(s) in 3.03s
     Running tests/proposal_review_retention.rs (<owned-build-lane>/target/debug/deps/proposal_review_retention-44a573d0e6d9cd63)

running 8 tests
test missing_or_changed_proposal_and_stale_predecessor_leave_no_review_or_orphan_bytes ... FAILED
test exact_retry_after_later_decision_and_reopen_returns_original ... FAILED
test changed_proof_policy_statement_or_duplicated_fields_refuse_before_append ... FAILED
test decision_and_review_identities_are_independently_unique_across_proposals ... FAILED
test all_review_bytes_are_promoted_and_stronger_retention_survives ... FAILED
test concurrent_decision_identity_reuse_across_proposals_has_one_winner ... FAILED
test concurrent_distinct_decisions_have_one_winner ... FAILED
test concurrent_identical_reviews_deduplicate ... FAILED

failures:

---- missing_or_changed_proposal_and_stale_predecessor_leave_no_review_or_orphan_bytes stdout ----

thread 'missing_or_changed_proposal_and_stale_predecessor_leave_no_review_or_orphan_bytes' (2274724) panicked at crates/ekr-store/tests/proposal_review_retention.rs:131:9:
assertion `left == right` failed
  left: Err(Document("not implemented"))
 right: Err(Conflict)

---- exact_retry_after_later_decision_and_reopen_returns_original stdout ----

thread 'exact_retry_after_later_decision_and_reopen_returns_original' (2274723) panicked at crates/ekr-store/tests/proposal_review_retention.rs:74:22:
called `Result::unwrap()` on an `Err` value: Document("not implemented")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- changed_proof_policy_statement_or_duplicated_fields_refuse_before_append stdout ----

thread 'changed_proof_policy_statement_or_duplicated_fields_refuse_before_append' (2274718) panicked at crates/ekr-store/tests/proposal_review_retention.rs:221:18:
called `Result::unwrap()` on an `Err` value: Document("not implemented")

---- decision_and_review_identities_are_independently_unique_across_proposals stdout ----

thread 'decision_and_review_identities_are_independently_unique_across_proposals' (2274722) panicked at crates/ekr-store/tests/proposal_review_retention.rs:248:14:
called `Result::unwrap()` on an `Err` value: Document("not implemented")

---- all_review_bytes_are_promoted_and_stronger_retention_survives stdout ----

thread 'all_review_bytes_are_promoted_and_stronger_retention_survives' (2274717) panicked at crates/ekr-store/tests/proposal_review_retention.rs:296:18:
called `Result::unwrap()` on an `Err` value: Document("not implemented")

---- concurrent_decision_identity_reuse_across_proposals_has_one_winner stdout ----

thread 'concurrent_decision_identity_reuse_across_proposals_has_one_winner' (2274719) panicked at crates/ekr-store/tests/proposal_review_retention.rs:352:9:
assertion `left == right` failed: [Err(Document("not implemented")), Err(Document("not implemented"))]
  left: 0
 right: 1

---- concurrent_distinct_decisions_have_one_winner stdout ----

thread 'concurrent_distinct_decisions_have_one_winner' (2274720) panicked at crates/ekr-store/tests/proposal_review_retention.rs:352:9:
assertion `left == right` failed: [Err(Document("not implemented")), Err(Document("not implemented"))]
  left: 0
 right: 1

---- concurrent_identical_reviews_deduplicate stdout ----

thread 'concurrent_identical_reviews_deduplicate' (2274721) panicked at crates/ekr-store/tests/proposal_review_retention.rs:352:9:
assertion `left == right` failed: [Err(Document("not implemented")), Err(Document("not implemented"))]
  left: 0
 right: 1


failures:
    all_review_bytes_are_promoted_and_stronger_retention_survives
    changed_proof_policy_statement_or_duplicated_fields_refuse_before_append
    concurrent_decision_identity_reuse_across_proposals_has_one_winner
    concurrent_distinct_decisions_have_one_winner
    concurrent_identical_reviews_deduplicate
    decision_and_review_identities_are_independently_unique_across_proposals
    exact_retry_after_later_decision_and_reopen_returns_original
    missing_or_changed_proposal_and_stale_predecessor_leave_no_review_or_orphan_bytes

test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr-store --test proposal_review_retention`
   Compiling ekr-store v0.0.27 (<review-retention-worktree>/crates/ekr-store)
    Finished `test` profile [unoptimized] target(s) in 26.50s
     Running unittests src/lib.rs (<owned-build-lane>/target/debug/deps/ekr_store-3545281ff30f2366)

running 16 tests
test eventlog::read_only::sidecars_in_flux::a_database_that_changed_is_read_again_without_a_pause ... ok
test eventlog::read_only::sidecars_in_flux::a_shm_that_never_appears_is_reported_after_the_bounded_reads ... ok
test eventlog::read_only::sidecars_in_flux::a_wal_without_its_shm_is_read_again_until_the_shm_is_there ... ok
test eventlog::read_only::sidecars_in_flux::any_other_result_is_returned_at_once ... ok
test eventlog::reads::a_merge_expectation_is_refused_by_the_preparation_capture ... ok
test eventlog::retention_faults::retention_crash_child ... ignored, child-process entry point; executed by abrupt_exit_retention_reopens_without_duplicates
test verified::registry::bytes_changed_in_place_are_hashed_again ... ok
test verified::registry::equal_bytes_skip_the_hash_and_any_other_bytes_are_hashed ... ok
test eventlog::reads::of_two_faulty_objects_the_first_in_order_decides_the_refusal ... ok
test eventlog::reads::a_missing_blob_is_refused_alike_by_the_single_and_the_batched_path ... ok
test eventlog::reads::replay_without_a_readable_feed_keeps_the_complete_payload_set ... ok
test eventlog::reads::a_feed_the_handle_cannot_read_has_every_held_object_read_again ... ok
test eventlog::retention_faults::before_and_after_native_response_loss_reopens_without_duplicate_knowledge ... ok
test eventlog::retention_faults::abrupt_exit_retention_reopens_without_duplicates ... ok
test eventlog::reads::a_history_load_costs_five_provider_calls_whatever_its_length ... ok
test eventlog::lock_wait::a_held_lock_is_retried_only_inside_the_window_and_the_bound_is_its_sum ... ok

test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.96s

     Running tests/adversary2_event_vocabulary.rs (<owned-build-lane>/target/debug/deps/adversary2_event_vocabulary-fad616bc4afad33f)

running 1 test
test a_second_rejection_of_a_re_proposed_transaction_is_not_swallowed_as_a_retry ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/adversary2_p1_15_blob_batch_order.rs (<owned-build-lane>/target/debug/deps/adversary2_p1_15_blob_batch_order-58615d1a01a046dd)

running 1 test
test a_later_blob_provider_failure_comes_before_an_earlier_objects_refusal_as_documented ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/adversary2_retention_event_contract.rs (<owned-build-lane>/target/debug/deps/adversary2_retention_event_contract-f6e82365fcc3aa23)

running 2 tests
test the_store_domain_declares_the_retention_raise_event_this_crate_writes ... ok
test the_retention_raise_in_stored_bytes_carries_the_fields_the_domain_declares ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_c7_f.rs (<owned-build-lane>/target/debug/deps/adversary_c7_f-faacb05160b8e5ff)

running 5 tests
test writer_process_beside_the_read_only_opens ... ignored, helper: runs only as the writer process another case starts
test a_read_only_open_through_a_symlinked_database_holds_every_acknowledged_object ... ok
test a_read_only_open_beside_a_live_writer_changes_nothing_under_the_store_path ... ok
test a_read_only_open_where_a_closing_writer_unlinked_the_wal_leaves_at_most_an_empty_wal ... ok
test a_wal_without_its_shm_is_read_twelve_times_then_refused ... ok

test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.54s

     Running tests/adversary_c7_s.rs (<owned-build-lane>/target/debug/deps/adversary_c7_s-d5c878aa731217be)

running 9 tests
test another_process_takes_the_shm_exclusively ... ok
test adversary_c7_s_a_database_renamed_over_the_path_is_refused_as_replaced ... ok
test no_replacement_check_releases_the_writer_process_lock_on_the_shm ... ok
test a_database_copied_over_a_symlinked_store_is_refused_as_replaced ... ok
test a_symlinked_store_opened_beside_a_checkpointing_writer_is_never_refused_as_replaced ... ok
test a_symlinked_store_beside_a_checkpointing_writer_is_never_refused_as_replaced ... ok
test adversary_c7_s_a_reader_beside_a_writer_and_a_checkpointer_is_never_refused_as_replaced ... ok
test adversary_c7_s_a_reader_beside_a_writer_that_checkpoints_itself_is_never_refused_as_replaced ... ok
test adversary_c7_s_an_open_beside_a_writer_that_checkpoints_itself_is_never_refused_as_replaced ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.23s

     Running tests/adversary_history_cache.rs (<owned-build-lane>/target/debug/deps/adversary_history_cache-38adf0a981c741e2)

running 5 tests
test adversary_history_replaced_file_store_selected_read_refuses_like_head_read ... ok
test adversary_history_second_handle_second_read_hashes_nothing_file ... ok
test adversary_history_get_then_foreign_raise_still_reads_file ... ok
test adversary_history_second_handle_second_read_hashes_nothing_sqlite ... ok
test adversary_history_get_then_foreign_raise_still_reads_sqlite ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/adversary_objects_and_append.rs (<owned-build-lane>/target/debug/deps/adversary_objects_and_append-f4cdf098d4e0fb55)

running 2 tests
test storing_canonical_bytes_that_were_cached_earlier_records_them_as_canonical ... ok
test retrying_an_append_writes_the_same_event_once_rather_than_twice ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_06_reference_from_bytes.rs (<owned-build-lane>/target/debug/deps/adversary_p1_06_reference_from_bytes-37bcd34bda71d7ab)

running 1 test
test a_store_cannot_turn_dangling_document_bytes_into_canonical_state ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_12_recovery_r2_identity.rs (<owned-build-lane>/target/debug/deps/adversary_p1_12_recovery_r2_identity-4df3bdf7835f5899)

running 2 tests
test adv2_an_identical_retained_decision_under_another_slot_is_refused_as_already_retained ... ok
test adv2_a_landed_third_attempt_retried_from_a_fresh_handle_is_already_recorded ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/adversary_p1_12_vectors_identity_preparation.rs (<owned-build-lane>/target/debug/deps/adversary_p1_12_vectors_identity_preparation-dc8f5d96eeeb8310)

running 2 tests
test a_retained_event_id_with_changed_content_is_refused_on_the_preparation_path_on_both_providers ... ok
test a_reused_event_id_after_a_retried_original_is_refused_and_history_stays_readable_on_both_providers ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/adversary_p1_14_published_events_paging.rs (<owned-build-lane>/target/debug/deps/adversary_p1_14_published_events_paging-1af8af59b71a5f2e)

running 2 tests
test the_sqlite_log_reads_back_past_one_feed_page ... ok
test the_file_log_reads_back_past_one_feed_page ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.41s

     Running tests/adversary_p1_14_r2_store_carriers.rs (<owned-build-lane>/target/debug/deps/adversary_p1_14_r2_store_carriers-37f3e95c477d26d7)

running 2 tests
test every_store_carrier_is_written_under_the_member_names_the_binding_compares ... ok
test every_store_field_is_carried_with_the_type_the_domain_declares ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/adversary_p1_14_store_bindings.rs (<owned-build-lane>/target/debug/deps/adversary_p1_14_store_bindings-2f000c7a85a020fa)

running 2 tests
test every_store_declaration_names_a_carrier_whatever_key_its_mapping_opens_with ... ok
test every_store_declaration_agrees_member_for_member_with_its_carrier ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/adversary_p1_15_batched_reads.rs (<owned-build-lane>/target/debug/deps/adversary_p1_15_batched_reads-351bf15e04caa81d)

running 4 tests
test a_redacted_object_event_is_refused_alike_by_the_single_and_the_batched_path ... ok
test the_first_object_in_order_decides_the_refusal_of_a_batched_load ... ok
test an_absent_object_is_refused_as_absent_by_the_batched_load_as_by_the_single_read ... ok
test a_write_by_another_handle_after_the_stamp_is_trusted_is_read ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.30s

     Running tests/adversary_read_only_open.rs (<owned-build-lane>/target/debug/deps/adversary_read_only_open-b162732f42a071b9)

running 2 tests
test a_sqlite_store_opened_read_only_while_a_writer_writes_holds_every_acknowledged_object ... ok
test a_file_store_opened_read_only_while_a_writer_writes_holds_every_acknowledged_object ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running tests/adversary_retained_bytes_shared_p1.rs (<owned-build-lane>/target/debug/deps/adversary_retained_bytes_shared_p1-090bc7697b462cdb)

running 2 tests
test no_retained_byte_outlives_its_handle_file ... ok
test no_retained_byte_outlives_its_handle_sqlite ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_runtime_context.rs (<owned-build-lane>/target/debug/deps/adversary_runtime_context-b3ebe6832626c232)

running 5 tests
test plain_worker_thread_can_open_and_write_while_another_thread_runs_tokio ... ok
test store_drop_during_caller_unwind_in_entered_handle_keeps_completed_data_reopenable ... ok
test entered_handle_constructor_refusal_preserves_existing_provider_bytes_and_missing_parents ... ok
test populated_file_lineage_refuses_before_seed_and_commit_callbacks_or_disk_changes ... ok
test populated_sqlite_lineage_refuses_before_seed_and_commit_callbacks_or_disk_changes ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/adversary_write_path_pointer.rs (<owned-build-lane>/target/debug/deps/adversary_write_path_pointer-b25894dbb501da70)

running 1 test
test adversary_write_path_a_verification_pointer_after_another_handles_checkpoint_is_written ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_x6_c.rs (<owned-build-lane>/target/debug/deps/adversary_x6_c-7a7ac8cedb3bb74d)

running 2 tests
test a_held_evidence_payload_whose_redaction_is_recorded_is_refused_as_a_fresh_handle_refuses_it_on_sqlite ... ok
test a_held_evidence_payload_whose_event_is_redacted_is_not_served_on_file ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/attachment_decode.rs (<owned-build-lane>/target/debug/deps/attachment_decode-32f7439219d94c5c)

running 3 tests
test duplicate_decoded_assertion_keys_are_refused_before_their_second_value ... ok
test unique_attachment_records_round_trip_without_changing_the_wire_shape ... ok
test omitted_and_empty_attachment_collections_still_decode ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/authority_verify.rs (<owned-build-lane>/target/debug/deps/authority_verify-e58d6443db0d4846)

running 3 tests
test replay_root_answers_as_replay_does_for_an_authority_that_implements_only_replay ... ok
test verify_answers_as_replay_does_for_an_authority_that_implements_only_replay ... ok
test a_head_no_pointer_answers_asks_the_authority_for_the_root_alone ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/borrowed_graph.rs (<owned-build-lane>/target/debug/deps/borrowed_graph-260a213627ba3dae)

running 3 tests
test borrowed_graph_returns_the_serializer_write_error ... ok
test borrowed_graph_preserves_serde_envelope_and_graph_struct_names ... ok
test borrowed_graph_matches_owned_bytes_for_every_value_and_retained_record_field ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/checkpoint_pointer.rs (<owned-build-lane>/target/debug/deps/checkpoint_pointer-46a77920db404990)

running 4 tests
test a_handle_continues_from_the_pointer_it_last_wrote ... ok
test a_pointer_another_handle_wrote_since_is_kept_and_the_next_write_still_lands ... ok
test a_repeat_of_the_handles_own_pointer_after_another_handles_is_written ... ok
test a_write_says_whether_its_pointer_stands ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/current_legacy_object_refusal.rs (<owned-build-lane>/target/debug/deps/current_legacy_object_refusal-788323ccc7bb79a0)

running 1 test
test a_schema_one_inline_object_is_refused_on_read_and_left_untouched_on_both_providers ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/current_occurrence_identity.rs (<owned-build-lane>/target/debug/deps/current_occurrence_identity-bcddc669ee9996e3)

running 1 test
test a_retained_event_id_with_changed_content_is_refused_on_both_providers ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/current_root_sensitivity.rs (<owned-build-lane>/target/debug/deps/current_root_sensitivity-51852ca458f97956)

running 5 tests
test the_two_roots_are_distinct_value_addresses ... ok
test every_attachment_coordinate_reaches_only_the_knowledge_root ... ok
test every_evidence_field_reaches_evidence_root_and_not_knowledge_root ... ok
test graph_fields_neither_root_reads_move_neither_root ... ok
test every_node_edge_and_assertion_field_reaches_knowledge_root_and_not_evidence_root ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/current_vectors.rs (<owned-build-lane>/target/debug/deps/current_vectors-27b1f0ddab2b8df2)

running 3 tests
test the_command_key_has_fixed_slot_bytes ... ok
test answer_identity_is_absent_from_historical_slots_and_never_explicit_null ... ok
test an_elected_bootstrap_preparation_has_fixed_bytes_on_both_providers ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/document_refusals.rs (<owned-build-lane>/target/debug/deps/document_refusals-c2c86fdd0590036a)

running 2 tests
test entity_display_retains_each_kind_and_stable_identity ... ok
test structured_membrane_refusal_survives_store_error_conversion ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/domain_projection.rs (<owned-build-lane>/target/debug/deps/domain_projection-419f3fd4b71986bb)

running 8 tests
test every_storage_class_has_its_own_retention_rank ... ok
test the_derived_ordering_is_not_the_retention_ordering ... ok
test the_retention_ladder_is_the_one_section_thirty_seven_describes ... ok
test the_strongest_of_two_classes_does_not_depend_on_which_arrived_first ... ok
test the_storage_classes_are_the_ones_the_domain_declares ... ok
test every_event_the_crate_writes_is_declared_by_the_domain ... ok
test every_event_the_crate_writes_carries_the_fields_the_domain_declares ... ok
test every_type_and_entity_the_domain_declares_names_a_rust_carrier ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s

     Running tests/durable_objects.rs (<owned-build-lane>/target/debug/deps/durable_objects-6222d572e9d7df1c)

running 1 test
test new_object_events_are_schema_two_metadata_with_verified_native_blobs ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/eventlog_object_memo.rs (<owned-build-lane>/target/debug/deps/eventlog_object_memo-117b85149859b237)

running 4 tests
test adversary_c7_s_the_withdrawal_guard_counts_every_form_of_a_provider_withdrawal ... ok
test a_held_provenance_object_is_read_again_only_after_another_handle_raises_it ... ok
test a_held_provenance_object_is_not_read_again_after_a_write_that_leaves_it_alone ... ok
test no_source_withdraws_retained_bytes_without_an_event_on_the_object_stream ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/fold_rules.rs (<owned-build-lane>/target/debug/deps/fold_rules-3c930ee180ef986b)

running 10 tests
test evidence_is_addressed_apart_from_knowledge ... ok
test the_knowledge_root_is_a_function_of_the_graph_state ... ok
test an_empty_log_has_no_head_and_does_not_fold ... ok
test a_lineage_that_does_not_begin_at_a_seed_is_refused ... ok
test a_second_seed_is_refused ... ok
test a_seed_naming_bytes_the_store_does_not_hold_is_refused ... ok
test the_store_reports_the_sub_roots_its_authority_derived_and_writes_no_placeholder ... ok
test a_commit_whose_validation_the_authority_stands_behind_advances_the_lineage ... ok
test a_store_with_no_commit_authority_refuses_to_say_what_canonical_state_is ... ok
test a_commit_whose_validation_the_authority_does_not_stand_behind_is_refused_not_folded_away ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/history_cache.rs (<owned-build-lane>/target/debug/deps/history_cache-ecacee62068bcfe6)

running 18 tests
test measure_two_history_reads_on_one_handle ... ignored, measurement; prints timings
test a_tampered_copy_of_a_verified_object_is_refused_by_content_file ... ok
test a_held_object_is_not_served_from_a_file_store_that_diverged ... ok
test a_later_read_on_one_handle_copies_no_retained_object_file ... ok
test a_file_store_that_diverged_is_refused_as_a_typed_divergence ... ok
test a_blob_damaged_before_its_first_load_is_still_refused_file ... ok
test a_second_read_on_one_handle_reads_and_hashes_no_blob_the_first_verified_file ... ok
test a_read_on_confirms_the_held_prefix_in_the_same_call_file ... ok
test an_occurrence_appended_through_another_handle_is_seen_by_the_next_read_file ... ok
test a_tampered_copy_of_a_verified_object_is_refused_by_content_sqlite ... ok
test a_later_read_on_one_handle_copies_no_retained_object_sqlite ... ok
test an_occurrence_appended_through_another_handle_is_seen_by_the_next_read_sqlite ... ok
test adversary_a_spurious_missing_replay_object_cannot_refuse_a_valid_history ... ok
test a_read_on_confirms_the_held_prefix_in_the_same_call_sqlite ... ok
test a_second_read_on_one_handle_reads_and_hashes_no_blob_the_first_verified_sqlite ... ok
test a_blob_damaged_before_its_first_load_is_still_refused_sqlite ... ok
test an_incomplete_replay_hint_falls_back_to_the_complete_history ... ok
test a_sqlite_store_overwritten_in_place_is_refused_as_replaced ... ok

test result: ok. 17 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/knowledge_retention.rs (<owned-build-lane>/target/debug/deps/knowledge_retention-42a744ae5b494dee)

running 1 test
test incubation_port_pins_document_bytes_on_both_providers ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/legacy.rs (<owned-build-lane>/target/debug/deps/legacy-a0a23b34b9d5227a)

running 9 tests
test all_user_record_keys_are_preserved_when_unique ... ok
test duplicate_keys_are_named_refusals_even_inside_user_records ... ok
test unique_map_refuses_decoded_duplicate_keys_and_preserves_unique_entries ... ok
test unique_set_refuses_decoded_duplicate_members_and_preserves_unique_entries ... ok
test original_document_fixture_verifies_exact_payload_bytes ... ok
test original_documents_refuse_unknown_fields_and_misfiled_identities ... ok
test direct_address_verifiers_distinguish_exact_payloads_from_frozen_values ... ok
test direct_identity_check_refuses_each_misfiled_collection ... ok
test knowledge_bytes_keep_original_map_framing_and_captured_record_order ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/membrane_boundary.rs (<owned-build-lane>/target/debug/deps/membrane_boundary-974b19565af0b61a)

running 1 test
test a_candidate_node_and_a_canonical_node_write_the_same_document ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/payload_blobs.rs (<owned-build-lane>/target/debug/deps/payload_blobs-935d230b7e3e709e)

running 3 tests
test a_decision_without_an_evidence_payload_is_still_elected_in_format_two_on_both_providers ... ok
test a_preparation_whose_staged_payload_is_gone_is_refused_by_name_on_both_providers ... ok
test a_preparation_names_its_evidence_payload_and_stages_the_bytes_once_on_both_providers ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/preparation_authorized_once.rs (<owned-build-lane>/target/debug/deps/preparation_authorized_once-df336290889d2579)

running 2 tests
test a_preparation_this_handle_authorized_is_not_authorized_again_when_read_back_identical ... ok
test a_preparation_changed_after_this_handle_authorized_it_is_authorized_again_and_refused ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/proposal_review_retention.rs (<owned-build-lane>/target/debug/deps/proposal_review_retention-44a573d0e6d9cd63)

running 8 tests
test changed_proof_policy_statement_or_duplicated_fields_refuse_before_append ... ok
test missing_or_changed_proposal_and_stale_predecessor_leave_no_review_or_orphan_bytes ... ok
test concurrent_identical_reviews_deduplicate ... ok
test decision_and_review_identities_are_independently_unique_across_proposals ... ok
test concurrent_decision_identity_reuse_across_proposals_has_one_winner ... ok
test concurrent_distinct_decisions_have_one_winner ... ok
test all_review_bytes_are_promoted_and_stronger_retention_survives ... ok
test exact_retry_after_later_decision_and_reopen_returns_original ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/providers.rs (<owned-build-lane>/target/debug/deps/providers-d25d31e5611c3bd8)

running 19 tests
test every_constructor_refuses_an_invalid_tenant_before_creating_anything ... ok
test an_existing_path_holding_no_store_is_no_store_and_is_left_as_it_was ... ok
test a_store_whose_creation_has_not_committed_is_no_store_and_is_left_as_it_was ... ok
test a_file_store_stores_identical_bytes_once ... ok
test a_file_store_reopened_folds_to_the_same_head_root ... ok
test a_stored_object_carries_the_class_it_was_written_under ... ok
test a_cache_write_after_a_canonical_one_leaves_the_bytes_canonical ... ok
test opening_an_existing_store_refuses_a_path_with_none_and_creates_nothing ... ok
test sqlite_stores_identical_bytes_once ... ok
test storing_a_graph_is_storing_its_document ... ok
test a_raised_retention_class_survives_a_reopen ... ok
test a_file_store_replay_of_the_head_revision_equals_the_fold ... ok
test the_fold_carries_the_seed_the_log_named ... ok
test a_replay_of_a_revision_the_lineage_has_not_reached_is_refused ... ok
test the_recorded_class_is_the_strongest_requested_whichever_order_they_arrive_in ... ok
test two_different_events_appended_in_a_row_both_land ... ok
test a_sqlite_store_reopened_folds_to_the_same_head_root ... ok
test every_event_shape_reports_a_write_once_and_a_recognition_after ... ok
test sqlite_replay_of_the_head_revision_equals_the_fold ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/published_events.rs (<owned-build-lane>/target/debug/deps/published_events-a78709c4a9796c14)

running 2 tests
test the_file_log_is_every_published_event_in_order ... ok
test the_sqlite_log_is_every_published_event_in_order ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/read_only_copies.rs (<owned-build-lane>/target/debug/deps/read_only_copies-949a096f89b0d502)

running 1 test
test copies_are_named_by_their_process_and_removed_once_it_is_gone ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/read_only_open.rs (<owned-build-lane>/target/debug/deps/read_only_open-1db996b36736155a)

running 7 tests
test a_read_only_path_holding_no_store_is_no_store ... ok
test a_read_only_file_store_removes_its_copy_when_it_drops ... ok
test a_sqlite_store_beside_an_empty_wal_is_read_from_the_database_alone ... ok
test a_sqlite_store_with_a_live_wal_is_read_with_its_newest_commit ... ok
test a_read_only_store_opens_for_reading_answers_and_refuses_every_write ... ok
test a_read_only_store_says_when_the_store_at_its_path_changed ... ok
test a_read_only_open_creates_no_shm_beside_a_wal_that_has_none ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s

     Running tests/review_p1_invariant_one_at_the_store.rs (<owned-build-lane>/target/debug/deps/review_p1_invariant_one_at_the_store-4d993ff08dd918a9)

running 1 test
test a_commit_lands_with_no_validated_transaction_anywhere_in_the_process ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/runtime_context.rs (<owned-build-lane>/target/debug/deps/runtime_context-375caeae5cc35860)

running 6 tests
test constructors_refuse_before_creating_paths ... ok
test running_runtime_reads_return_a_refusal_instead_of_panicking ... ok
test entered_runtime_reads_refuse_and_the_store_remains_usable ... ok
test every_file_operation_refuses_before_authority_or_persistence ... ok
test dropping_inside_a_running_runtime_preserves_completed_writes ... ok
test every_sqlite_operation_refuses_before_authority_or_persistence ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/schema_proposal_retention.rs (<owned-build-lane>/target/debug/deps/schema_proposal_retention-3164b14c633b5283)

running 6 tests
test changed_bytes_under_one_id_refuse_without_pinning_the_loser ... ok
test malformed_mismatched_and_noncanonical_identity_inputs_publish_nothing ... ok
test exact_retries_and_reopen_preserve_the_original_on_both_providers ... ok
test concurrent_changed_input_elects_one_and_refuses_the_other ... ok
test concurrent_identical_retries_elect_one_retained_record ... ok
test retained_payload_is_pinned_and_never_downgrades_stronger_retention ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/seed_object_integrity.rs (<owned-build-lane>/target/debug/deps/seed_object_integrity-f3638a3522a9b45e)

running 2 tests
test file_seed_objects_verify_address_length_and_retention_before_admission ... ok
test sqlite_seed_objects_verify_address_length_and_retention_before_admission ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/store_inventory.rs (<owned-build-lane>/target/debug/deps/store_inventory-15212b1d29c42d8a)

running 2 tests
test an_inline_object_whose_bytes_are_not_its_address_is_refused_on_both_providers ... ok
test an_inventory_reads_legacy_and_current_objects_and_writes_nothing_on_both_providers ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests ekr_store

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

    Checking ekr-store v0.0.27 (<review-retention-worktree>/crates/ekr-store)
    Finished `dev` profile [unoptimized] target(s) in 1.79s
   Compiling ekr-store v0.0.27 (<review-retention-worktree>/crates/ekr-store)
    Finished `test` profile [unoptimized] target(s) in 1.19s
     Running tests/proposal_review_retention.rs (<owned-build-lane>/target/debug/deps/proposal_review_retention-44a573d0e6d9cd63)

running 8 tests
test changed_proof_policy_statement_or_duplicated_fields_refuse_before_append ... ok
test missing_or_changed_proposal_and_stale_predecessor_leave_no_review_or_orphan_bytes ... ok
test concurrent_decision_identity_reuse_across_proposals_has_one_winner ... ok
test decision_and_review_identities_are_independently_unique_across_proposals ... ok
test concurrent_identical_reviews_deduplicate ... ok
test concurrent_distinct_decisions_have_one_winner ... ok
test all_review_bytes_are_promoted_and_stronger_retention_survives ... ok
test exact_retry_after_later_decision_and_reopen_returns_original ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

