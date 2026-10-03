unit:                   story:apply-approved-refinement — explicit application capture boundary correction
verdict:                green
cases:                  focused application 11→13 passed; final whole package 316 passed, 0 failed, 3 ignored
origin:                 n/a
wrote-outside-worktree: assigned evidence directory; exact private inventory outside-paths.txt
needs-coordinator:      yes — capture-final.patch or capture-checkpoint-to-final.patch; kernel integration and independent review remain

1. Unit and acceptance

Correct the selected-history blocker recorded in report.md without altering the frozen source-final.patch baseline. Selected history and original preparation authorization must load only the required physical application closure, preserve exact post-canonical markers, expose explicit canonical-prefix metadata, and keep full-capture missing-link refusal. The parent delegated this correction and confirmed exact marker-closed capture because the native read port does not expose authenticated group ranges.

This report supersedes report.md's selected-history blocker for the physical implementation, not the complete F story. Parent kernel integration must independently verify retries with real signature/operation authority. No semantic approval/conformance claim follows from the synthetic-authority store tests.

2. Change

ApplicationHistory::canonical_through() at applications.rs:65 returns None for complete capture, Some(n) for the exact selected/original-preparation canonical occurrence prefix. Scope remains private to store construction and participates in Eq. Pending candidate staging advances the explicit prefix by its actual staged occurrence.

load_application_material at applications.rs:1057 receives Complete, Prefix, or Candidate scope from eventlog.rs:1547/1549 and preparation.rs:1219. A selected cutoff matches the actual canonical provider_event_id, stream, version and full event payload in the tenant feed; it never uses a cached newer tail. Selected markers close over exact selected event identities and undergo event/record/guard/transaction/proposal/cursor validation. A marker physically after the canonical event remains included. Candidate capture additionally selects its exact immutable application/step/attempt and reviewed prefix.

application_rows_capture filters metadata before fetching pinned row payloads. Schema proposal and observation lookup select requested identities before decoding/verifying their payload objects. Review-prefix validation uses only selected recorded events and preserves identity/proof binding, native coordinates, contiguous cursor and byte checks. Full capture continues auditing unknown proposal markers and unknown/unsupported retention envelopes. No missing transaction heuristic or skip-by-absence permission was introduced.

Native limitation: RecordedEvent has no immutable group identity, and EventStore has no recorded-group lookup. request_id is caller metadata, so replay does not use it as proof of atomic grouping. Creation still uses a single native atomic append group; replay proves the exact visible marker closure and refuses incomplete links. Complete tenant-feed metadata is still scanned. This correction excludes unrelated future pinned-object payload fetches, not every byte embedded inside raw feed event metadata.

Provider tests now compare every retained material component separately and explicitly require selected Some(version) versus complete None. Their former whole-value Eq assertion is incompatible with deliberately distinct capture metadata; canonical occurrences, objects, application data and observations remain exactly equal at the current head.

Exact delta against frozen source-final baseline:
 crates/ekr-store/src/eventlog.rs                |    9 +
 crates/ekr-store/src/observations.rs            |   13 +
 crates/ekr-store/src/preparation.rs             |    5
 crates/ekr-store/src/proposal_reviews.rs        |   27 ++
 crates/ekr-store/src/schema_proposals.rs        |   10 +
 crates/ekr-store/tests/providers.rs             |   25 ++
 crates/ekr-store/src/applications.rs            |  269 ++++++++++++++++++-----
 crates/ekr-store/tests/application_retention.rs |  152 +++++++++++++
 8 files changed, 444 insertions(+), 66 deletions(-)

3. Red evidence

Parent-owned actual kernel witness was read at ../schema-application-kernel/application-implementation-2.log: both schema_only_application_is_durable_and_idempotent_on_file/sqlite failed on retry with receipt transaction is unavailable; runner1passed2failed0ignored. It is parent evidence, not a test executed by this worker.

Local command: cargo test -p ekr-store --test application_retention selected_history
Initial API-red (capture-red.log): missing canonical_through getter. Then getter-only compiled red below. Its first assertion detects the missing explicit scope, not by itself a missing-blob observation; the final deleted-future-blob control establishes the latter. No conflation of compile-red and semantic evidence.
   Compiling ekr-graph v0.0.27 (<managed-worktree>/crates/ekr-graph)
   Compiling ekr-store v0.0.27 (<managed-worktree>/crates/ekr-store)
    Finished `test` profile [unoptimized] target(s) in 5.90s
     Running tests/application_retention.rs (<shared-build-lane>/target/debug/deps/application_retention-bc4308b5a907b9ae)

running 1 test
test selected_history_closes_exact_marker_without_loading_future_application_inputs ... FAILED

failures:

---- selected_history_closes_exact_marker_without_loading_future_application_inputs stdout ----

thread 'selected_history_closes_exact_marker_without_loading_future_application_inputs' (358047) panicked at crates/ekr-store/tests/application_retention.rs:769:9:
assertion `left == right` failed
  left: None
 right: Some(1)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    selected_history_closes_exact_marker_without_loading_future_application_inputs

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.10s

error: test failed, to rerun pass `-p ekr-store --test application_retention`

Whole regression initially returned314passed2failed3ignored (capture-package.log), solely provider equality assertions because Some(13) differs from None; all material values were unchanged. Explicit material-equality+scope assertions corrected that expectation. No cases were ignored or dropped.

4. Green evidence

Same whole command: cargo test -p ekr-store -p ekr-graph --no-fail-fast
Capture package executed316(314passed2failed)→316(316passed0failed),3ignored in both, exit101→0. This final count is unchanged because the intervening changes strengthened existing assertions, not new test functions. Relative to the frozen physical focused target, application tests11→13passed,0ignored: selected historical closure and old preparation capture are two new cases. Earlier package-3 was313passed before the final physical envelope case and these two capture cases; it is not a full base count. Original base run stopped at missing carriers and remains explicitly incomplete.

All13 focused application cases loop both providers. The selected-history case cold-reopens after a later election, excludes its proposal object, and includes the earlier commit's marker. It then deletes only the future proposal blob using the native test provider: history_at(earlier revision) succeeds, while complete history refuses. The old-preparation case elects an unrelated later application, reopens with an authority that refuses unscoped/unrelated capture, and recovers the exact original preparation. Existing full-capture missing/orphan/mismatched/zero-cursor marker cases remain green.

Final whole package stdout (capture-package-final.log, session66196, exit0):
    Finished `test` profile [unoptimized] target(s) in 0.11s
     Running unittests src/lib.rs (<shared-build-lane>/target/debug/deps/ekr_graph-5c61ebaffa0c5661)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_guard_bounds_and_ranges.rs (<shared-build-lane>/target/debug/deps/adversary2_guard_bounds_and_ranges-1e7ca4d949f1e786)

running 4 tests
test an_inverted_range_is_refused_or_describes_some_instant ... ok
test the_item_scanner_is_the_same_text_in_both_files ... ok
test the_guard_against_a_returning_open_ended_read_covers_the_whole_crate ... ok
test the_field_guard_catches_a_new_field_whatever_it_is_called ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/adversary2_membrane_and_addresses.rs (<shared-build-lane>/target/debug/deps/adversary2_membrane_and_addresses-02c0d1c1ce219425)

running 2 tests
test every_entity_canonical_state_holds_has_a_content_address ... ok
test the_seal_of_canonical_dependency_is_carried_only_by_a_canonical_dependency ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_canonical_value_reach.rs (<shared-build-lane>/target/debug/deps/adversary_canonical_value_reach-a92e2dc2fea16a61)

running 3 tests
test a_transient_candidate_assertion_may_carry_an_approximate_measurement ... ok
test a_transient_candidate_node_may_hold_an_approximate_measurement ... ok
test every_part_of_graph_state_has_a_canonical_encoding ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_06_reference_markers.rs (<shared-build-lane>/target/debug/deps/adversary_p1_06_reference_markers-5195ab45ea811f49)

running 2 tests
test the_default_reference_of_a_transient_claim_is_the_transient_reference ... ok
test a_reference_to_evidence_resolves_to_evidence_and_never_to_a_node ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_14_exit_typed_assertion_refs.rs (<shared-build-lane>/target/debug/deps/adversary_p1_14_exit_typed_assertion_refs-1cb07eda286a01b9)

running 1 test
    Checking ekr-graph-tests v0.0.0 (<shared-build-lane>/target/tests/trybuild/ekr-graph)
    Finished `dev` profile [unoptimized] target(s) in 0.04s


test tests/adversary_p1_14_exit_compile_fail/a_canonical_dispute_names_its_competitors_by_canonical_reference.rs ... ok
test tests/adversary_p1_14_exit_compile_fail/a_canonical_supersession_names_its_replacement_by_canonical_reference.rs ... ok
test tests/adversary_p1_14_exit_compile_fail/retained_evidence_names_its_source_assertion_by_canonical_reference.rs ... ok


test every_assertion_reference_canonical_state_holds_refuses_a_transient_identity ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s

     Running tests/adversary_p1_14_r2_graph_declaration_scan.rs (<shared-build-lane>/target/debug/deps/adversary_p1_14_r2_graph_declaration_scan-8539d662165a6a5b)

running 1 test
test every_graph_declaration_the_line_scans_must_see_opens_with_its_name ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_snapshot_and_assertion.rs (<shared-build-lane>/target/debug/deps/adversary_snapshot_and_assertion-51678dee1750f797)

running 4 tests
test is_current_is_exactly_acceptance_an_active_lifecycle_and_an_open_transaction_time ... ok
test a_record_with_no_transaction_time_at_all_is_not_a_current_belief ... ok
test the_domain_requires_a_recorded_from_and_the_crate_cannot_omit_one ... ok
test active_is_not_merely_valid_at_the_end_of_representable_time ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/application_guard.rs (<shared-build-lane>/target/debug/deps/application_guard-717d8da198d0a613)

running 2 tests
test guard_codec_refuses_invalid_cursor_identity_and_step_shape ... ok
test guarded_ordinary_format_is_closed_and_old_hash_input_is_unchanged ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/canonical_value_and_assertion.rs (<shared-build-lane>/target/debug/deps/canonical_value_and_assertion-f91f7da165244b62)

running 21 tests
test a_node_and_an_edge_property_carry_only_an_admissible_value ... ok
test confidence_refusal_preserves_the_public_error_and_its_bound ... ok
test a_float_at_any_depth_cannot_inhabit_a_canonical_value_and_the_refusal_names_it ... ok
test an_admissible_value_round_trips_through_the_newtype ... ok
test every_field_of_a_node_and_an_edge_reaches_the_encoding ... ok
test every_field_of_an_assertion_reaches_the_encoding ... ok
test no_two_edges_that_differ_share_a_content_address ... ok
test no_two_observations_that_differ_share_a_content_address ... ok
test graph_state_equal_in_every_field_hashes_equally ... ok
test every_field_of_evidence_state_reaches_the_encoding ... ok
test no_two_nodes_that_differ_share_a_content_address ... ok
test no_two_assertions_that_differ_share_a_content_address ... ok
test no_two_support_links_that_differ_share_a_content_address ... ok
test public_canonical_targets_preserve_deserialized_identity_and_address ... ok
test no_two_pieces_of_evidence_that_differ_share_a_content_address ... ok
test every_variant_of_every_sum_type_opens_with_its_own_marker ... ok
test range_serde_refusals_keep_the_actual_current_and_frozen_error_type ... ok
test the_conversion_refuses_exactly_where_the_ontology_says_it_must ... ok
test two_values_that_differ_do_not_encode_alike ... ok
test two_assertions_equal_in_every_field_hash_equally ... ok
test the_declaration_order_of_every_sum_type_equals_its_numbering ... ok

test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/current_vectors.rs (<shared-build-lane>/target/debug/deps/current_vectors-bec691471ad00b41)

running 7 tests
test a_rejected_event_matches_its_hand_derived_bytes ... ok
test every_current_event_payload_kind_has_a_fixed_envelope_address ... ok
test seed_and_successor_roots_have_fixed_addresses ... ok
test the_base_assertion_has_fixed_canonical_bytes ... ok
test the_seed_root_matches_its_hand_derived_bytes ... ok
test every_current_event_payload_kind_has_fixed_envelope_bytes ... ok
test every_current_assessment_and_lifecycle_kind_has_a_fixed_assertion_address ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/domain_projection.rs (<shared-build-lane>/target/debug/deps/domain_projection-3251f6e6106bb5be)

running 6 tests
test field_binding_reads_inline_and_block_fields_without_borrowing_a_neighbor ... ok
test assessment_and_lifecycle_retain_independent_payloads ... ok
test current_type_regions_cannot_borrow_legacy_or_unrelated_fields ... ok
test property_maps_are_keyed_by_property_id_in_the_domain_and_the_crate ... ok
test every_enumeration_the_domain_declares_is_carried_variant_for_variant ... ok
test every_declaration_of_the_domain_is_carried_field_for_field ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s

     Running tests/evidence_and_observations.rs (<shared-build-lane>/target/debug/deps/evidence_and_observations-9ee5768518d2a783)

running 6 tests
test a_blob_observation_carries_its_media_type_and_length ... ok
test confidence_outside_its_declared_range_is_not_constructible ... ok
test every_evidence_source_answers_the_flat_fields_the_domain_declares ... ok
test every_observation_form_answers_its_kind_and_its_content_hash ... ok
test evidence_carries_its_provenance ... ok
test support_links_one_assertion_to_one_piece_of_evidence ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/legacy_status.rs (<shared-build-lane>/target/debug/deps/legacy_status-c401899978fcfdc5)

running 3 tests
test active_says_nothing_about_acceptance_or_belief_time ... ok
test every_frozen_validation_state_maps_to_the_documented_status ... ok
test removal_statuses_carry_exactly_the_payload_of_their_validation_state ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/membrane.rs (<shared-build-lane>/target/debug/deps/membrane-cf6dd3cf8fd7e012)

running 5 tests
test a_canonical_reference_is_the_only_canonical_dependency ... ok
test a_candidate_and_a_canonical_node_that_share_an_id_do_not_resolve_alike ... ok
test canonical_state_resolves_a_canonical_reference ... ok
test transient_state_may_depend_on_canonical_state_and_on_its_own ... ok
    Checking ekr-graph-tests v0.0.0 (<shared-build-lane>/target/tests/trybuild/ekr-graph)
    Finished `dev` profile [unoptimized] target(s) in 0.04s


test tests/compile_fail/a_canonical_assertion_cites_evidence_by_canonical_reference.rs ... ok
test tests/compile_fail/a_canonical_claim_names_its_nodes_by_canonical_reference.rs ... ok
test tests/compile_fail/a_canonical_reference_holds_the_id_of_its_kind.rs ... ok
test tests/compile_fail/a_canonical_reference_targets_only_what_canonical_state_holds.rs ... ok
test tests/compile_fail/a_canonical_subject_names_its_edge_by_canonical_reference.rs ... ok
test tests/compile_fail/canonical_dependency_is_sealed.rs ... ok
test tests/compile_fail/canonical_graph_rejects_a_transient_ref.rs ... ok
test tests/compile_fail/canonical_ref_cannot_target_a_transient_type.rs ... ok
test tests/compile_fail/transient_state_has_no_content_address.rs ... ok


test the_membrane_is_a_set_of_build_failures ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s

     Running tests/node_identity.rs (<shared-build-lane>/target/debug/deps/node_identity-563e8296a4f98e7c)

running 3 tests
test the_lifecycle_state_of_a_node_is_a_property_and_not_its_identity ... ok
test two_nodes_that_share_a_name_do_not_share_an_id ... ok
test a_node_renamed_a_thousand_times_keeps_its_id ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/review_p1_membrane_is_by_id.rs (<shared-build-lane>/target/debug/deps/review_p1_membrane_is_by_id-1b8d99c0dbefe0a6)

running 2 tests
test canonical_state_resolves_the_references_it_holds_and_answers_none_for_one_it_does_not ... ok
    Checking ekr-graph-tests v0.0.0 (<shared-build-lane>/target/tests/trybuild/ekr-graph)
    Finished `dev` profile [unoptimized] target(s) in 0.04s


test tests/review_p1_compile_fail/a_canonical_edge_may_target_a_candidate_node.rs ... ok


test the_sealed_reference_types_are_sound_and_canonical_state_holds_none_of_them ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s

     Running tests/revision_events.rs (<shared-build-lane>/target/debug/deps/revision_events-650af5429b80491a)

running 11 tests
test every_variant_carries_its_declared_index_and_domain_name ... ok
test no_two_variants_share_an_encoding ... ok
test an_event_encodes_as_a_function_of_its_value ... ok
test the_current_envelope_binds_format_occurrence_record_and_payload ... ok
test proposal_payload_distinguishes_absent_and_present_operations_hashes ... ok
test the_variant_marker_and_not_the_payload_is_what_separates_two_events ... ok
test answer_metadata_is_generated_checked_and_version_four_only ... ok
test transition_requires_version_three_and_historical_payloads_keep_version_two ... ok
test transition_metadata_is_a_lossless_bridge_to_the_generated_contract ... ok
test the_declaration_order_of_the_variants_equals_their_numbering ... ok
test shared_human_identity_envelope_is_explicit_and_preserves_old_event_bytes ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/snapshot_reads.rs (<shared-build-lane>/target/debug/deps/snapshot_reads-de8fac5b7ce1a059)

running 9 tests
test a_snapshot_names_the_revision_it_reads ... ok
test a_proposed_assertion_is_never_answered ... ok
test a_record_whose_transaction_time_is_closed_is_not_current ... ok
test a_superseded_assertion_answers_only_inside_its_closed_interval ... ok
test attached_evidence_is_ordered_and_scoped_without_changing_assertions_or_older_graphs ... ok
test the_historical_query_returns_alice ... ok
test valid_at_answers_alice_before_the_handover_and_bob_at_or_after_it ... ok
test the_handover_instant_belongs_to_exactly_one_of_them ... ok
test valid_at_never_returns_a_retracted_assertion ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (<shared-build-lane>/target/debug/deps/ekr_store-c541b8ab35ac80a0)

running 18 tests
test eventlog::read_only::sidecars_in_flux::a_database_that_changed_is_read_again_without_a_pause ... ok
test eventlog::read_only::sidecars_in_flux::a_shm_that_never_appears_is_reported_after_the_bounded_reads ... ok
test eventlog::read_only::sidecars_in_flux::a_wal_without_its_shm_is_read_again_until_the_shm_is_there ... ok
test eventlog::read_only::sidecars_in_flux::any_other_result_is_returned_at_once ... ok
test eventlog::reads::a_merge_expectation_is_refused_by_the_preparation_capture ... ok
test eventlog::retention_faults::retention_crash_child ... ignored, child-process entry point; executed by abrupt_exit_retention_reopens_without_duplicates
test verified::registry::bytes_changed_in_place_are_hashed_again ... ok
test verified::registry::equal_bytes_skip_the_hash_and_any_other_bytes_are_hashed ... ok
test eventlog::reads::a_missing_blob_is_refused_alike_by_the_single_and_the_batched_path ... ok
test eventlog::reads::of_two_faulty_objects_the_first_in_order_decides_the_refusal ... ok
test eventlog::reads::replay_without_a_readable_feed_refuses_application_audit ... ok
test eventlog::reads::application_audit_refuses_unreadable_feed_without_weakening_object_memo_checks ... ok
test eventlog::preparation::human_identity_tests::new_signed_preparations_require_exact_shared_index_and_retry_idempotently ... ok
test eventlog::preparation::human_identity_tests::legacy_signed_preparations_recover_only_already_published_occurrences ... ok
test eventlog::retention_faults::before_and_after_native_response_loss_reopens_without_duplicate_knowledge ... ok
test eventlog::reads::a_history_load_including_application_audit_costs_seven_calls_within_one_feed_page ... ok
test eventlog::retention_faults::abrupt_exit_retention_reopens_without_duplicates ... ok
test eventlog::lock_wait::a_held_lock_is_retried_only_inside_the_window_and_the_bound_is_its_sum ... ok

test result: ok. 17 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.96s

     Running tests/adversary2_event_vocabulary.rs (<shared-build-lane>/target/debug/deps/adversary2_event_vocabulary-f5065bc9371fa03e)

running 1 test
test a_second_rejection_of_a_re_proposed_transaction_is_not_swallowed_as_a_retry ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/adversary2_p1_15_blob_batch_order.rs (<shared-build-lane>/target/debug/deps/adversary2_p1_15_blob_batch_order-55b7641fc5ace5cf)

running 1 test
test a_later_blob_provider_failure_comes_before_an_earlier_objects_refusal_as_documented ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/adversary2_retention_event_contract.rs (<shared-build-lane>/target/debug/deps/adversary2_retention_event_contract-4f7245c48ffb2a6c)

running 2 tests
test the_store_domain_declares_the_retention_raise_event_this_crate_writes ... ok
test the_retention_raise_in_stored_bytes_carries_the_fields_the_domain_declares ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_c7_f.rs (<shared-build-lane>/target/debug/deps/adversary_c7_f-cd401c25564266c9)

running 5 tests
test writer_process_beside_the_read_only_opens ... ignored, helper: runs only as the writer process another case starts
test a_read_only_open_through_a_symlinked_database_holds_every_acknowledged_object ... ok
test a_read_only_open_beside_a_live_writer_changes_nothing_under_the_store_path ... ok
test a_read_only_open_where_a_closing_writer_unlinked_the_wal_leaves_at_most_an_empty_wal ... ok
test a_wal_without_its_shm_is_read_twelve_times_then_refused ... ok

test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.53s

     Running tests/adversary_c7_s.rs (<shared-build-lane>/target/debug/deps/adversary_c7_s-94c56558510210b4)

running 9 tests
test another_process_takes_the_shm_exclusively ... ok
test adversary_c7_s_a_database_renamed_over_the_path_is_refused_as_replaced ... ok
test a_database_copied_over_a_symlinked_store_is_refused_as_replaced ... ok
test no_replacement_check_releases_the_writer_process_lock_on_the_shm ... ok
test adversary_c7_s_a_reader_beside_a_writer_and_a_checkpointer_is_never_refused_as_replaced ... ok
test a_symlinked_store_beside_a_checkpointing_writer_is_never_refused_as_replaced ... ok
test a_symlinked_store_opened_beside_a_checkpointing_writer_is_never_refused_as_replaced ... ok
test adversary_c7_s_an_open_beside_a_writer_that_checkpoints_itself_is_never_refused_as_replaced ... ok
test adversary_c7_s_a_reader_beside_a_writer_that_checkpoints_itself_is_never_refused_as_replaced ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.62s

     Running tests/adversary_history_cache.rs (<shared-build-lane>/target/debug/deps/adversary_history_cache-3839a05a7dd1eb86)

running 5 tests
test adversary_history_replaced_file_store_selected_read_refuses_like_head_read ... ok
test adversary_history_second_handle_second_read_hashes_nothing_file ... ok
test adversary_history_get_then_foreign_raise_still_reads_file ... ok
test adversary_history_second_handle_second_read_hashes_nothing_sqlite ... ok
test adversary_history_get_then_foreign_raise_still_reads_sqlite ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/adversary_objects_and_append.rs (<shared-build-lane>/target/debug/deps/adversary_objects_and_append-d50a78c095f3a08b)

running 2 tests
test storing_canonical_bytes_that_were_cached_earlier_records_them_as_canonical ... ok
test retrying_an_append_writes_the_same_event_once_rather_than_twice ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_06_reference_from_bytes.rs (<shared-build-lane>/target/debug/deps/adversary_p1_06_reference_from_bytes-da0c6d0eb08cc5eb)

running 1 test
test a_store_cannot_turn_dangling_document_bytes_into_canonical_state ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_12_recovery_r2_identity.rs (<shared-build-lane>/target/debug/deps/adversary_p1_12_recovery_r2_identity-307b11eb2d0cda01)

running 2 tests
test adv2_an_identical_retained_decision_under_another_slot_is_refused_as_already_retained ... ok
test adv2_a_landed_third_attempt_retried_from_a_fresh_handle_is_already_recorded ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/adversary_p1_12_vectors_identity_preparation.rs (<shared-build-lane>/target/debug/deps/adversary_p1_12_vectors_identity_preparation-03faf70cc68f19cf)

running 2 tests
test a_retained_event_id_with_changed_content_is_refused_on_the_preparation_path_on_both_providers ... ok
test a_reused_event_id_after_a_retried_original_is_refused_and_history_stays_readable_on_both_providers ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/adversary_p1_14_published_events_paging.rs (<shared-build-lane>/target/debug/deps/adversary_p1_14_published_events_paging-2abe01df131b1007)

running 2 tests
test the_sqlite_log_reads_back_past_one_feed_page ... ok
test the_file_log_reads_back_past_one_feed_page ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.03s

     Running tests/adversary_p1_14_r2_store_carriers.rs (<shared-build-lane>/target/debug/deps/adversary_p1_14_r2_store_carriers-ad83135b0920522e)

running 2 tests
test every_store_field_is_carried_with_the_type_the_domain_declares ... ok
test every_store_carrier_is_written_under_the_member_names_the_binding_compares ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/adversary_p1_14_store_bindings.rs (<shared-build-lane>/target/debug/deps/adversary_p1_14_store_bindings-a768a9add907c507)

running 2 tests
test every_store_declaration_names_a_carrier_whatever_key_its_mapping_opens_with ... ok
test every_store_declaration_agrees_member_for_member_with_its_carrier ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running tests/adversary_p1_15_batched_reads.rs (<shared-build-lane>/target/debug/deps/adversary_p1_15_batched_reads-0c62f7438e7f4576)

running 4 tests
test a_redacted_object_event_is_refused_alike_by_the_single_and_the_batched_path ... ok
test an_absent_object_is_refused_as_absent_by_the_batched_load_as_by_the_single_read ... ok
test the_first_object_in_order_decides_the_refusal_of_a_batched_load ... ok
test a_write_by_another_handle_after_the_stamp_is_trusted_is_read ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.30s

     Running tests/adversary_read_only_open.rs (<shared-build-lane>/target/debug/deps/adversary_read_only_open-5d8cfb4ea17b37f4)

running 2 tests
test a_sqlite_store_opened_read_only_while_a_writer_writes_holds_every_acknowledged_object ... ok
test a_file_store_opened_read_only_while_a_writer_writes_holds_every_acknowledged_object ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/adversary_retained_bytes_shared_p1.rs (<shared-build-lane>/target/debug/deps/adversary_retained_bytes_shared_p1-7f55b36939fcdd0f)

running 2 tests
test no_retained_byte_outlives_its_handle_file ... ok
test no_retained_byte_outlives_its_handle_sqlite ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_runtime_context.rs (<shared-build-lane>/target/debug/deps/adversary_runtime_context-24a720799d36c235)

running 5 tests
test plain_worker_thread_can_open_and_write_while_another_thread_runs_tokio ... ok
test store_drop_during_caller_unwind_in_entered_handle_keeps_completed_data_reopenable ... ok
test entered_handle_constructor_refusal_preserves_existing_provider_bytes_and_missing_parents ... ok
test populated_file_lineage_refuses_before_seed_and_commit_callbacks_or_disk_changes ... ok
test populated_sqlite_lineage_refuses_before_seed_and_commit_callbacks_or_disk_changes ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/adversary_write_path_pointer.rs (<shared-build-lane>/target/debug/deps/adversary_write_path_pointer-0db6d06798915430)

running 1 test
test adversary_write_path_a_verification_pointer_after_another_handles_checkpoint_is_written ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/adversary_x6_c.rs (<shared-build-lane>/target/debug/deps/adversary_x6_c-e1e2ccfaf8c32ca4)

running 2 tests
test a_held_evidence_payload_whose_redaction_is_recorded_is_refused_as_a_fresh_handle_refuses_it_on_sqlite ... ok
test a_held_evidence_payload_whose_event_is_redacted_is_not_served_on_file ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/application_retention.rs (<shared-build-lane>/target/debug/deps/application_retention-bc4308b5a907b9ae)

running 13 tests
test closed_application_retention_feed_refuses_unknown_tables_and_versions ... ok
test processing_receipt_requires_retained_source_and_mapping_bytes ... ok
test missing_independent_source_bytes_refuse_election_before_any_record ... ok
test concurrent_elections_preserve_one_winner_on_both_providers ... ok
test independent_source_is_pinned_and_replayed_without_an_incubation_root ... ok
test immutable_election_step_attempt_retry_and_reopen_preserve_winners ... ok
test default_semantic_authority_refuses_retained_application_history_and_new_attempts ... ok
test prepared_marker_loses_to_rejection_without_orphan_ordinary_event ... ok
test guarded_publication_has_exact_marker_and_marker_does_not_change_human_predecessor ... ok
test old_preparation_captures_only_its_original_prefix_and_exact_candidate ... ok
test renewed_review_reprepares_same_ordinary_input_without_mutating_prior_attempt ... ok
test cold_replay_refuses_missing_extra_mismatched_and_zero_cursor_markers ... ok
test selected_history_closes_exact_marker_without_loading_future_application_inputs ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s

     Running tests/attachment_decode.rs (<shared-build-lane>/target/debug/deps/attachment_decode-8dcf2a07e3f9f1fe)

running 3 tests
test duplicate_decoded_assertion_keys_are_refused_before_their_second_value ... ok
test unique_attachment_records_round_trip_without_changing_the_wire_shape ... ok
test omitted_and_empty_attachment_collections_still_decode ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/authority_verify.rs (<shared-build-lane>/target/debug/deps/authority_verify-72e3e85ea70a2a95)

running 3 tests
test verify_answers_as_replay_does_for_an_authority_that_implements_only_replay ... ok
test replay_root_answers_as_replay_does_for_an_authority_that_implements_only_replay ... ok
test a_head_no_pointer_answers_asks_the_authority_for_the_root_alone ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/borrowed_graph.rs (<shared-build-lane>/target/debug/deps/borrowed_graph-9ec76f57a49c192c)

running 3 tests
test borrowed_graph_returns_the_serializer_write_error ... ok
test borrowed_graph_preserves_serde_envelope_and_graph_struct_names ... ok
test borrowed_graph_matches_owned_bytes_for_every_value_and_retained_record_field ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/checkpoint_pointer.rs (<shared-build-lane>/target/debug/deps/checkpoint_pointer-5b97c53d482c5201)

running 4 tests
test a_handle_continues_from_the_pointer_it_last_wrote ... ok
test a_pointer_another_handle_wrote_since_is_kept_and_the_next_write_still_lands ... ok
test a_repeat_of_the_handles_own_pointer_after_another_handles_is_written ... ok
test a_write_says_whether_its_pointer_stands ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/current_legacy_object_refusal.rs (<shared-build-lane>/target/debug/deps/current_legacy_object_refusal-b5e64d9aad60b933)

running 1 test
test a_schema_one_inline_object_is_refused_on_read_and_left_untouched_on_both_providers ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/current_occurrence_identity.rs (<shared-build-lane>/target/debug/deps/current_occurrence_identity-7c844a1e05619af1)

running 1 test
test a_retained_event_id_with_changed_content_is_refused_on_both_providers ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/current_root_sensitivity.rs (<shared-build-lane>/target/debug/deps/current_root_sensitivity-53dbebf364d40458)

running 5 tests
test the_two_roots_are_distinct_value_addresses ... ok
test every_attachment_coordinate_reaches_only_the_knowledge_root ... ok
test graph_fields_neither_root_reads_move_neither_root ... ok
test every_evidence_field_reaches_evidence_root_and_not_knowledge_root ... ok
test every_node_edge_and_assertion_field_reaches_knowledge_root_and_not_evidence_root ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/current_vectors.rs (<shared-build-lane>/target/debug/deps/current_vectors-9d95699e341e80d4)

running 3 tests
test the_command_key_has_fixed_slot_bytes ... ok
test answer_identity_is_absent_from_historical_slots_and_never_explicit_null ... ok
test an_elected_bootstrap_preparation_has_fixed_bytes_on_both_providers ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/document_refusals.rs (<shared-build-lane>/target/debug/deps/document_refusals-377266f4f0c72313)

running 2 tests
test entity_display_retains_each_kind_and_stable_identity ... ok
test structured_membrane_refusal_survives_store_error_conversion ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/domain_projection.rs (<shared-build-lane>/target/debug/deps/domain_projection-5e57233b07496963)

running 8 tests
test every_storage_class_has_its_own_retention_rank ... ok
test the_retention_ladder_is_the_one_section_thirty_seven_describes ... ok
test the_derived_ordering_is_not_the_retention_ordering ... ok
test the_strongest_of_two_classes_does_not_depend_on_which_arrived_first ... ok
test the_storage_classes_are_the_ones_the_domain_declares ... ok
test every_event_the_crate_writes_is_declared_by_the_domain ... ok
test every_event_the_crate_writes_carries_the_fields_the_domain_declares ... ok
test every_type_and_entity_the_domain_declares_names_a_rust_carrier ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s

     Running tests/durable_objects.rs (<shared-build-lane>/target/debug/deps/durable_objects-8c79e58db69b7047)

running 1 test
test new_object_events_are_schema_two_metadata_with_verified_native_blobs ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/eventlog_object_memo.rs (<shared-build-lane>/target/debug/deps/eventlog_object_memo-377d51ae275f41d1)

running 4 tests
test adversary_c7_s_the_withdrawal_guard_counts_every_form_of_a_provider_withdrawal ... ok
test a_held_provenance_object_is_read_again_only_after_another_handle_raises_it ... ok
test a_held_provenance_object_is_not_read_again_after_a_write_that_leaves_it_alone ... ok
test no_source_withdraws_retained_bytes_without_an_event_on_the_object_stream ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running tests/fold_rules.rs (<shared-build-lane>/target/debug/deps/fold_rules-b5ea9cedd5173f79)

running 10 tests
test evidence_is_addressed_apart_from_knowledge ... ok
test the_knowledge_root_is_a_function_of_the_graph_state ... ok
test an_empty_log_has_no_head_and_does_not_fold ... ok
test a_lineage_that_does_not_begin_at_a_seed_is_refused ... ok
test a_second_seed_is_refused ... ok
test a_seed_naming_bytes_the_store_does_not_hold_is_refused ... ok
test a_commit_whose_validation_the_authority_stands_behind_advances_the_lineage ... ok
test the_store_reports_the_sub_roots_its_authority_derived_and_writes_no_placeholder ... ok
test a_store_with_no_commit_authority_refuses_to_say_what_canonical_state_is ... ok
test a_commit_whose_validation_the_authority_does_not_stand_behind_is_refused_not_folded_away ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/history_cache.rs (<shared-build-lane>/target/debug/deps/history_cache-a61dde487b0f8f32)

running 18 tests
test measure_two_history_reads_on_one_handle ... ignored, measurement; prints timings
test a_tampered_copy_of_a_verified_object_is_refused_by_content_file ... ok
test a_file_store_that_diverged_is_refused_as_a_typed_divergence ... ok
test a_later_read_on_one_handle_copies_no_retained_object_file ... ok
test a_held_object_is_not_served_from_a_file_store_that_diverged ... ok
test a_blob_damaged_before_its_first_load_is_still_refused_file ... ok
test a_read_on_confirms_the_held_prefix_in_the_same_call_file ... ok
test a_second_read_on_one_handle_reads_and_hashes_no_blob_the_first_verified_file ... ok
test an_occurrence_appended_through_another_handle_is_seen_by_the_next_read_file ... ok
test a_tampered_copy_of_a_verified_object_is_refused_by_content_sqlite ... ok
test a_later_read_on_one_handle_copies_no_retained_object_sqlite ... ok
test a_blob_damaged_before_its_first_load_is_still_refused_sqlite ... ok
test an_occurrence_appended_through_another_handle_is_seen_by_the_next_read_sqlite ... ok
test a_read_on_confirms_the_held_prefix_in_the_same_call_sqlite ... ok
test a_second_read_on_one_handle_reads_and_hashes_no_blob_the_first_verified_sqlite ... ok
test adversary_a_spurious_missing_replay_object_cannot_refuse_a_valid_history ... ok
test an_incomplete_replay_hint_falls_back_to_the_complete_history ... ok
test a_sqlite_store_overwritten_in_place_is_refused_as_replaced ... ok

test result: ok. 17 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/human_decision_identity.rs (<shared-build-lane>/target/debug/deps/human_decision_identity-5d69fbe52c85deac)

running 6 tests
test legacy_direct_publication_refuses_before_any_mutation ... ok
test ordinary_checkpoint_head_keeps_the_existing_shortcut ... ok
test new_signed_occurrences_require_matching_index_on_cold_replay ... ok
test checkpoint_head_after_signed_answer_checks_its_mandatory_identity_binding ... ok
test historical_signed_decisions_reserve_ids_without_requiring_new_indexes ... ok
test canonical_and_proposal_decisions_race_on_one_atomic_identity ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/knowledge_retention.rs (<shared-build-lane>/target/debug/deps/knowledge_retention-d51337cdbfef3428)

running 1 test
test incubation_port_pins_document_bytes_on_both_providers ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/legacy.rs (<shared-build-lane>/target/debug/deps/legacy-84afaf9fe72e366d)

running 9 tests
test all_user_record_keys_are_preserved_when_unique ... ok
test duplicate_keys_are_named_refusals_even_inside_user_records ... ok
test unique_map_refuses_decoded_duplicate_keys_and_preserves_unique_entries ... ok
test unique_set_refuses_decoded_duplicate_members_and_preserves_unique_entries ... ok
test direct_address_verifiers_distinguish_exact_payloads_from_frozen_values ... ok
test original_document_fixture_verifies_exact_payload_bytes ... ok
test original_documents_refuse_unknown_fields_and_misfiled_identities ... ok
test direct_identity_check_refuses_each_misfiled_collection ... ok
test knowledge_bytes_keep_original_map_framing_and_captured_record_order ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/membrane_boundary.rs (<shared-build-lane>/target/debug/deps/membrane_boundary-a1b5544603fedaa4)

running 1 test
test a_candidate_node_and_a_canonical_node_write_the_same_document ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/payload_blobs.rs (<shared-build-lane>/target/debug/deps/payload_blobs-3ab3c6724246a10e)

running 3 tests
test a_decision_without_an_evidence_payload_is_still_elected_in_format_two_on_both_providers ... ok
test a_preparation_whose_staged_payload_is_gone_is_refused_by_name_on_both_providers ... ok
test a_preparation_names_its_evidence_payload_and_stages_the_bytes_once_on_both_providers ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/preparation_authorized_once.rs (<shared-build-lane>/target/debug/deps/preparation_authorized_once-db0e920435a02bc9)

running 2 tests
test a_preparation_this_handle_authorized_is_not_authorized_again_when_read_back_identical ... ok
test a_preparation_changed_after_this_handle_authorized_it_is_authorized_again_and_refused ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/proposal_review_retention.rs (<shared-build-lane>/target/debug/deps/proposal_review_retention-9fb76c9c440d739e)

running 9 tests
test changed_proof_policy_statement_or_duplicated_fields_refuse_before_append ... ok
test concurrent_identical_reviews_deduplicate ... ok
test concurrent_decision_identity_reuse_across_proposals_has_one_winner ... ok
test decision_and_review_identities_are_independently_unique_across_proposals ... ok
test concurrent_distinct_decisions_have_one_winner ... ok
test missing_or_changed_proposal_and_stale_predecessor_leave_no_review_or_orphan_bytes ... ok
test all_review_bytes_are_promoted_and_stronger_retention_survives ... ok
test legacy_review_events_keep_their_original_identity_rules_and_reserve_decisions ... ok
test exact_retry_after_later_decision_and_reopen_returns_original ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running tests/providers.rs (<shared-build-lane>/target/debug/deps/providers-22b9fa6819f38d2f)

running 19 tests
test every_constructor_refuses_an_invalid_tenant_before_creating_anything ... ok
test an_existing_path_holding_no_store_is_no_store_and_is_left_as_it_was ... ok
test a_file_store_stores_identical_bytes_once ... ok
test a_store_whose_creation_has_not_committed_is_no_store_and_is_left_as_it_was ... ok
test a_file_store_reopened_folds_to_the_same_head_root ... ok
test a_cache_write_after_a_canonical_one_leaves_the_bytes_canonical ... ok
test a_stored_object_carries_the_class_it_was_written_under ... ok
test sqlite_stores_identical_bytes_once ... ok
test opening_an_existing_store_refuses_a_path_with_none_and_creates_nothing ... ok
test storing_a_graph_is_storing_its_document ... ok
test a_raised_retention_class_survives_a_reopen ... ok
test the_recorded_class_is_the_strongest_requested_whichever_order_they_arrive_in ... ok
test the_fold_carries_the_seed_the_log_named ... ok
test a_replay_of_a_revision_the_lineage_has_not_reached_is_refused ... ok
test a_file_store_replay_of_the_head_revision_equals_the_fold ... ok
test a_sqlite_store_reopened_folds_to_the_same_head_root ... ok
test two_different_events_appended_in_a_row_both_land ... ok
test every_event_shape_reports_a_write_once_and_a_recognition_after ... ok
test sqlite_replay_of_the_head_revision_equals_the_fold ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/published_events.rs (<shared-build-lane>/target/debug/deps/published_events-498ecd3a576bc3b9)

running 2 tests
test the_file_log_is_every_published_event_in_order ... ok
test the_sqlite_log_is_every_published_event_in_order ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/read_only_copies.rs (<shared-build-lane>/target/debug/deps/read_only_copies-8a6936247b14807d)

running 1 test
test copies_are_named_by_their_process_and_removed_once_it_is_gone ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/read_only_open.rs (<shared-build-lane>/target/debug/deps/read_only_open-1c7e87487bd7c439)

running 7 tests
test a_read_only_path_holding_no_store_is_no_store ... ok
test a_read_only_file_store_removes_its_copy_when_it_drops ... ok
test a_sqlite_store_beside_an_empty_wal_is_read_from_the_database_alone ... ok
test a_sqlite_store_with_a_live_wal_is_read_with_its_newest_commit ... ok
test a_read_only_store_opens_for_reading_answers_and_refuses_every_write ... ok
test a_read_only_store_says_when_the_store_at_its_path_changed ... ok
test a_read_only_open_creates_no_shm_beside_a_wal_that_has_none ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s

     Running tests/review_p1_invariant_one_at_the_store.rs (<shared-build-lane>/target/debug/deps/review_p1_invariant_one_at_the_store-0002b6edcf9e6b64)

running 1 test
test a_commit_lands_with_no_validated_transaction_anywhere_in_the_process ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/runtime_context.rs (<shared-build-lane>/target/debug/deps/runtime_context-f136a378d91129ce)

running 6 tests
test constructors_refuse_before_creating_paths ... ok
test running_runtime_reads_return_a_refusal_instead_of_panicking ... ok
test entered_runtime_reads_refuse_and_the_store_remains_usable ... ok
test every_file_operation_refuses_before_authority_or_persistence ... ok
test dropping_inside_a_running_runtime_preserves_completed_writes ... ok
test every_sqlite_operation_refuses_before_authority_or_persistence ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/schema_proposal_retention.rs (<shared-build-lane>/target/debug/deps/schema_proposal_retention-9f76f74d74fc57bb)

running 6 tests
test changed_bytes_under_one_id_refuse_without_pinning_the_loser ... ok
test malformed_mismatched_and_noncanonical_identity_inputs_publish_nothing ... ok
test exact_retries_and_reopen_preserve_the_original_on_both_providers ... ok
test concurrent_identical_retries_elect_one_retained_record ... ok
test concurrent_changed_input_elects_one_and_refuses_the_other ... ok
test retained_payload_is_pinned_and_never_downgrades_stronger_retention ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/seed_object_integrity.rs (<shared-build-lane>/target/debug/deps/seed_object_integrity-f6c8317320883431)

running 2 tests
test file_seed_objects_verify_address_length_and_retention_before_admission ... ok
test sqlite_seed_objects_verify_address_length_and_retention_before_admission ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/store_inventory.rs (<shared-build-lane>/target/debug/deps/store_inventory-1412dfa06a928d10)

running 2 tests
test an_inline_object_whose_bytes_are_not_its_address_is_refused_on_both_providers ... ok
test an_inventory_reads_legacy_and_current_objects_and_writes_nothing_on_both_providers ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests ekr_graph

running 1 test
test crates/ekr-graph/src/lib.rs - (line 79) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

   Doc-tests ekr_store

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


Final lint: cargo clippy -p ekr-store -p ekr-graph --all-targets -- -D warnings, exit0.
    Checking ekr-store v0.0.27 (<managed-worktree>/crates/ekr-store)
    Finished `dev` profile [unoptimized] target(s) in 0.41s
Format: exact changed Rust paths checked using rustfmt --check --edition2021 --config skip_children=true, exit0 (capture-fmt.status). git diff --check exit0. Every final source change was included in the final whole package run.

5. Handoff and limits

Source frozen, no commits, no ESS/AEP/GitHub writes, no cleanup. No live Cargo remains here. Session66196 package terminal0; final clippy command terminal0; own lease codex-ekr-schema-application-store-20261003 released. Root confirmed both patches applied after exact digest checks and owns Cargo for real schema-application/review regression.

Full correction over source-final.patch: capture-final.patch SHA25668f4ce20dda6062bbffd638b05e8934a7d28addd62ba838bf028966b0b65cd02.
If capture-checkpoint.patch8129b7451cd4f5b7310e544adea62d73316fdcfd51e62823bf41db38360feaa0 is already applied, use ONLY capture-checkpoint-to-final.patch SHA25664594e0d57d9eb219fc270dbb36df79992790f64fde3e77d11bce973cd384402. This final delta contains two test files only; do not reapply the full correction.

Root remains responsible for actual kernel authority, mapped-fact integration, CLI/SDK/viewer and generated implementation conformance, independent adversary review, task check and bot publication. Read-port atomic grouping and complete-feed cost/availability limitations remain explicit. No upstream provider capability was invented or required here.

6. Outside-worktree writes

All worker artifacts remain in assigned retained-evidence: logs/statuses, frozen source snapshots, patches, raw/sanitized reports and outside-paths.txt. The latter records full private filesystem paths; reports use placeholders for AEP/public review. Shared compiler output stayed in the authorized sequential build lane; native temporary fixtures used the authorized temp/inode directories. Existing parent brief is not a worker-authored artifact. Parent owns cleanup and next action.
