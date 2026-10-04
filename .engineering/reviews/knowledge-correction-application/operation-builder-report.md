unit: story:apply-approved-refinement — schema correction operation unit (rev32)
verdict: blocked
cases: executed 62→69, red 5
origin: n/a
wrote-outside-worktree: assigned private schema-correction-ops evidence directory; inventory retained privately
needs-coordinator: yes, operations.patch; real production caller, independent review and full integration gate

1. Acceptance and checked scope

A crate-private operation builder accepts a real VerifiedDecision, authenticated read, retained exact proposal, semantic ClaimReplacement list and statement EvidenceId. It derives only the immutable proposal's corrections after checking an ApproveSchemaProposal target, payload equality, exact content digest and proposal identity. It groups corrections through the same current full dispute-component projection used by proposal material, including competitors. Global replacement coverage is checked before partitioning; missing, extra, reused and colliding IDs refuse. Per-component derivation reuses existing derive/instructions unchanged. Retractions and additions are globally sorted, and duplicate effects refuse.

Explicit Unresolved returns schema-correction-unresolved for the whole selection, without returning partial operations. This pure method neither admits a transaction nor grants publication authority. Existing AttentionAnswer methods and their target/basis checks remain unchanged. Scope is exactly the existing corrections module, its new test module and human_review module registration; coordinator-synchronized ESS changes are excluded.

2. Actual patch shape

```
 crates/ekr-kernel/src/human_review.rs              |    2
 crates/ekr-kernel/src/human_review/corrections.rs  |  162 +++++++++
 .../src/human_review/schema_corrections_tests.rs   |  363 ++++++++++++++++++++
 3 files changed, 526 insertions(+), 1 deletion(-)
```

3. Initial red

Command: cargo test -p ekr-kernel --lib schema_corrections_tests -- --nocapture
The first invocation red.log failed compilation due to an ambiguous fixture collect type. After specifying the existing Vec<Vec<AssertionId>> field type, the behavioral red ran seven cases: five failed against the fail-closed stub; two negative-only cases passed. Exit 101.

```
   Compiling ekr-kernel v0.0.27 (<worktree>/crates/ekr-kernel)
    Finished `test` profile [unoptimized] target(s) in 9.37s
     Running unittests src/lib.rs (<shared-build>/target/debug/deps/ekr_kernel-f8587ff637e2bcbf)

running 7 tests

thread 'human_review::schema_corrections_tests::schema_replacements_have_global_exact_fresh_distinct_coverage' (1653715) panicked at crates/ekr-kernel/src/human_review/schema_corrections_tests.rs:318:14:
called `Result::unwrap()` on an `Err` value: KnowledgeRefused { code: "answer-correction", reason: "schema correction builder not implemented" }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test human_review::schema_corrections_tests::schema_replacements_have_global_exact_fresh_distinct_coverage ... FAILED

thread 'human_review::schema_corrections_tests::schema_choose_and_retract_use_actual_dispute_competitors' (1653711) panicked at crates/ekr-kernel/src/human_review/schema_corrections_tests.rs:161:14:
called `Result::unwrap()` on an `Err` value: KnowledgeRefused { code: "answer-correction", reason: "schema correction builder not implemented" }
test human_review::schema_corrections_tests::schema_choose_and_retract_use_actual_dispute_competitors ... FAILED

thread 'human_review::schema_corrections_tests::schema_time_preserves_source_evidence_and_uses_exact_fresh_replacement' (1653716) panicked at crates/ekr-kernel/src/human_review/schema_corrections_tests.rs:181:10:
called `Result::unwrap()` on an `Err` value: KnowledgeRefused { code: "answer-correction", reason: "schema correction builder not implemented" }
test human_review::schema_corrections_tests::schema_time_preserves_source_evidence_and_uses_exact_fresh_replacement ... FAILED

thread 'human_review::schema_corrections_tests::schema_unresolved_refuses_the_entire_correction_step' (1653717) panicked at crates/ekr-kernel/src/human_review/schema_corrections_tests.rs:247:9:
assertion `left == right` failed
  left: "answer-correction"
 right: "schema-correction-unresolved"
test human_review::schema_corrections_tests::schema_unresolved_refuses_the_entire_correction_step ... FAILED
test human_review::schema_corrections_tests::schema_corrections_require_distinct_current_applicable_claims ... ok

thread 'human_review::schema_corrections_tests::schema_multi_dispute_effects_are_complete_and_stably_ordered' (1653713) panicked at crates/ekr-kernel/src/human_review/schema_corrections_tests.rs:219:10:
called `Result::unwrap()` on an `Err` value: KnowledgeRefused { code: "answer-correction", reason: "schema correction builder not implemented" }
test human_review::schema_corrections_tests::schema_multi_dispute_effects_are_complete_and_stably_ordered ... FAILED
test human_review::schema_corrections_tests::schema_proof_target_and_immutable_payload_are_exact ... ok

failures:

failures:
    human_review::schema_corrections_tests::schema_choose_and_retract_use_actual_dispute_competitors
    human_review::schema_corrections_tests::schema_multi_dispute_effects_are_complete_and_stably_ordered
    human_review::schema_corrections_tests::schema_replacements_have_global_exact_fresh_distinct_coverage
    human_review::schema_corrections_tests::schema_time_preserves_source_evidence_and_uses_exact_fresh_replacement
    human_review::schema_corrections_tests::schema_unresolved_refuses_the_entire_correction_step

test result: FAILED. 2 passed; 5 failed; 0 ignored; 0 measured; 62 filtered out; finished in 0.03s

error: test failed, to rerun pass `-p ekr-kernel --lib`
```

4. Verification

Focused lane: executed 7 → 7, exit 0. The same seven preimplementation cases changed from 5 failed/2 passed to 7 passed. No new cases were added after that red.

Command: cargo test -p ekr-kernel --lib schema_corrections_tests -- --nocapture
```
   Compiling ekr-kernel v0.0.27 (<worktree>/crates/ekr-kernel)
    Finished `test` profile [unoptimized] target(s) in 9.16s
     Running unittests src/lib.rs (<shared-build>/target/debug/deps/ekr_kernel-f8587ff637e2bcbf)

running 7 tests
test human_review::schema_corrections_tests::schema_choose_and_retract_use_actual_dispute_competitors ... ok
test human_review::schema_corrections_tests::schema_corrections_require_distinct_current_applicable_claims ... ok
test human_review::schema_corrections_tests::schema_replacements_have_global_exact_fresh_distinct_coverage ... ok
test human_review::schema_corrections_tests::schema_time_preserves_source_evidence_and_uses_exact_fresh_replacement ... ok
test human_review::schema_corrections_tests::schema_multi_dispute_effects_are_complete_and_stably_ordered ... ok
test human_review::schema_corrections_tests::schema_unresolved_refuses_the_entire_correction_step ... ok
test human_review::schema_corrections_tests::schema_proof_target_and_immutable_payload_are_exact ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 62 filtered out; finished in 0.03s

```

Command: cargo test -p ekr-kernel --lib --test human_review --test attention_answers_recovery
Exit 0. Kernel library executed 62 → 69. The before count comes from the coordinator's actual application-mapping-regression-1.log summary; the final run below executes all 69. Attention recovery executed 2 and human review executed 18; their prior counts were not remeasured by this unit, so no invented before counts are claimed.
```
   Compiling ekr-kernel v0.0.27 (<worktree>/crates/ekr-kernel)
warning: method `schema_correction_operations` is never used
  --> crates/ekr-kernel/src/human_review/corrections.rs:16:19
   |
11 | impl VerifiedDecision {
   | --------------------- method in this implementation
...
16 |     pub(crate) fn schema_correction_operations(
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: function `schema_replacements` is never used
   --> crates/ekr-kernel/src/human_review/corrections.rs:172:4
    |
172 | fn schema_replacements(
    |    ^^^^^^^^^^^^^^^^^^^

warning: function `schema_derive` is never used
   --> crates/ekr-kernel/src/human_review/corrections.rs:209:4
    |
209 | fn schema_derive(
    |    ^^^^^^^^^^^^^

warning: `ekr-kernel` (lib) generated 3 warnings
    Finished `test` profile [unoptimized] target(s) in 10.08s
     Running unittests src/lib.rs (<shared-build>/target/debug/deps/ekr_kernel-f8587ff637e2bcbf)

running 69 tests
test application_plan::tests::application_schema_plan_cannot_redeclare_an_existing_property ... ok
test application_plan::tests::frozen_schema_plan_preserves_allocations_and_existing_declarations ... ok
test application_plan::tests::application_schema_plan_refuses_destructive_changes_and_empty_effects ... ok
test application_projection::tests::absent_optionals_and_empty_collections_remain_distinct ... ok
test application_projection::tests::every_operation_union_alternative_keeps_its_tag ... ok
test application_projection::tests::mixed_application_history_keeps_nested_payloads_and_coordinates ... ok
test application_fact::tests::blocked_resolution_does_not_even_invoke_the_identity_allocator ... ok
test application_fact::tests::blocked_mapping_returns_blockers_without_a_step ... ok
test application_projection::tests::proposal_show_exposes_receipts_and_application_history ... ok
test application_mapping::tests::mapping_payload_digest_and_qualified_keys_distinguish_each_mapping_and_source ... ok
test application_mapping::tests::relation_copy_and_noderef_constant_check_both_endpoints ... ok
test application_mapping::tests::resolved_property_uses_alias_and_exact_selected_fact_evidence ... ok
test application_fact::tests::fact_step_contains_only_supported_assertion_and_exact_mapping_provenance ... ok
test application_fact::tests::semantic_verification_checks_the_claim_value_beyond_structural_shape ... ok
test application_mapping::tests::all_canonical_value_kinds_work_for_constant_and_copy_field ... ok
test application_transaction::tests::canonical_value_bytes_and_kind_are_both_checked ... ok
test application_fact::tests::semantic_verification_uses_frozen_ids_but_not_timestamp_guessed_revision ... ok
test application_projection::tests::numeric_overflow_and_invalid_bytes_refuse_instead_of_narrowing ... ok
test application_transaction::tests::all_operations_preserve_native_fields_and_canonical_bytes ... ok
test application_transaction::tests::unrepresentable_timestamps_and_nested_values_fail_closed ... ok
test document::checked::tests::mixed_accounting_refuses_before_allocating_a_coerced_string ... ok
test document::checked::tests::string_refusals_precede_the_allocating_application_visitor ... ok
test document::bounded_load::a_transaction_document_nested_past_the_depth_is_refused_at_the_first_container_past_it ... ok
test application_transaction::tests::optional_owner_schema_and_empty_aliases_preserve_historical_meaning ... ok
test application_transaction::tests::every_assertion_and_evidence_payload_variant_round_trips ... ok
test application_fact::tests::no_support_or_incomplete_correspondence_never_creates_fresh_evidence ... ok
test application_fact::tests::frozen_template_rejects_duplicate_identities_and_incomplete_support ... ok
test application_fact::tests::unselected_mapping_and_missing_correspondence_fail_before_allocating ... ok
test application_transaction::tests::normalization_is_limited_to_empty_aliases_and_equivalent_instants ... ok
test incubation_value::tests::existing_canonical_encoder_is_the_oracle_for_every_value_kind ... ok
test application_transaction::tests::inconsistent_projection_payloads_are_not_silently_dropped ... ok
test application_transaction::tests::invalid_numeric_ranges_and_interval_order_are_refused ... ok
test application_fact::tests::frozen_template_tampering_is_rejected ... ok
test application_transaction::tests::malformed_identity_and_duplicate_manifest_are_refused ... ok
test application_mapping::tests::relation_missing_ambiguous_and_outside_endpoints_stay_blocked ... ok
test human_review::schema_corrections_tests::schema_unresolved_refuses_the_entire_correction_step ... ok
test human_review::schema_corrections_tests::schema_choose_and_retract_use_actual_dispute_competitors ... ok
test human_review::schema_corrections_tests::schema_proof_target_and_immutable_payload_are_exact ... ok
test human_review::schema_corrections_tests::schema_corrections_require_distinct_current_applicable_claims ... ok
test human_review::schema_corrections_tests::schema_multi_dispute_effects_are_complete_and_stably_ordered ... ok
test human_review::schema_corrections_tests::schema_replacements_have_global_exact_fresh_distinct_coverage ... ok
test human_review::schema_corrections_tests::schema_time_preserves_source_evidence_and_uses_exact_fresh_replacement ... ok
test application_mapping::tests::unresolved_ambiguous_and_invalid_selectors_match_preview_blockers ... ok
test checkpoint::tests::held_identities_are_read_from_each_receipt_without_parsing_its_proposal ... ok
test replay::tests::retirement_requires_the_confirmed_occurrence_and_matching_prefix ... ok
test replay::tests::prefix_hash_memo_compares_every_occurrence_and_preserves_held_digest_vectors ... ok
test seed::bounded_load::a_seed_nested_past_the_depth_is_refused_at_the_first_container_past_it ... ok
test seed::shared_payloads::a_carried_payload_becomes_the_retained_allocation_only_for_verified_equal_bytes ... ok
test replay::tests::node_only_commits_share_the_unchanged_assertion_edge_index ... ok
test validate::candidate::tests::a_candidate_borrows_existing_node_types_at_every_graph_size ... ok
test replay::tests::confirmed_commit_releases_the_previous_head_before_the_next_command ... ok
test replay::tests::reusable_graph_belongs_to_the_authority_not_the_thread ... ok
test checkpoint::tests::checkpoint_writer_matches_the_complete_owned_document_bytes ... ok
test answers::tests::signed_answers_validate_through_the_ordinary_pipeline_with_private_reviewed_withdrawals ... ok
test replay::tests::an_unpublished_candidate_cannot_seed_reuse ... ok
test replay::tests::an_equal_root_replayed_by_another_authority_falls_back_to_clone ... ok
test replay::tests::reused_graph_preserves_inherited_attachments_and_matches_forced_clone ... ok
test checkpoint::tests::a_fresh_open_restores_the_head_and_replays_nothing_the_checkpoint_covers ... ok
test replay::tests::readers_and_checkpoint_graphs_prevent_reuse_extraction ... ok
test replay::tests::warm_alias_checks_visit_no_unchanged_nodes ... ok
test replay::tests::a_new_checkpoint_releases_its_predecessors_graph_before_the_next_command ... ok
test replay::tests::one_handle_holds_the_head_graph_and_not_one_graph_per_revision ... ok
test upgrade::version_tests::knowledge_two_history_survives_the_reviewed_third_profile ... ok
test replay::tests::one_handle_validates_against_an_earlier_revision_whose_graph_it_released ... ok
test answers::tests::reviewed_answers_publish_retry_reopen_and_fully_replay_on_both_providers ... ok
test replay::tests::warm_validation_hashes_no_more_prefix_occurrences_as_history_grows ... ok
test replay::tests::validating_one_transaction_does_not_copy_prior_document_buffers ... ok
test replay::tests::append_commits_copy_no_more_assertions_as_the_graph_grows ... ok
test application_progress::tests::fixture::older_receipt_reports_only_its_mapping_prefix_after_later_commits ... ok

test result: ok. 69 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.70s

     Running tests/attention_answers_recovery.rs (<shared-build>/target/debug/deps/attention_answers_recovery-8b39c4f5d8a37256)

running 2 tests
test answer_port_fault_resumes_exactly_in_a_fresh_process ... ok
test answer_native_process_death_resumes_without_partial_corrections ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.27s

     Running tests/human_review.rs (<shared-build>/target/debug/deps/human_review-2e1e3e62d19d9e82)

running 18 tests
test correction_codec_preserves_order_every_field_and_millisecond_instants ... ok
test independent_binding_refuses_substituted_policy_and_store ... ok
test policy_refuses_duplicate_keys_scopes_wrong_key_digest_and_noncanonical_order ... ok
test forged_truncated_and_altered_signatures_refuse ... ok
test signing_message_matches_the_fixed_binary_layout_vector ... ok
test policy_key_order_is_checked_and_an_empty_policy_never_authorizes ... ok
test signing_codec_has_explicit_domain_and_ess_field_order_and_refuses_invalid_scalars ... ok
test human_answer_cannot_change_its_corrections_subject_scope_statement_or_predecessor ... ok
test verified_operator_comes_from_policy_and_statement_and_target_are_bound ... ok
test even_valid_signatures_cannot_cross_audience_key_policy_or_predecessor ... ok
test exact_human_answer_survives_only_unrelated_basis_advancement ... ok
test retained_proof_and_every_review_basis_field_are_bound ... ok
test generated_json_review_documents_preserve_all_signed_targets_and_reject_bad_scalars ... ok
test schema_review_binds_exact_proposal_and_material_without_binding_unrelated_revisions ... ok
test canonical_readers_refuse_every_truncation_and_trailing_byte ... ok
test all_four_targets_verify_only_in_their_own_enrolled_scope ... ok
test protocol_digests_and_object_addresses_resolve_the_same_bytes_after_both_provider_reopens ... ok
test no_single_byte_change_in_retained_proof_can_replay_as_approved ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

```

Command: cargo clippy -p ekr-kernel --lib -- -D warnings
Exit 101, three dead-code diagnostics for the intentionally unwired schema method and its two private helpers. No product suppression or artificial caller was added. An initial combined shell command subsequently ran formatting, masking clippy's shell exit; clippy-exit.log below is the isolated repeat with its own observed 101 exit.
```
    Checking ekr-kernel v0.0.27 (<worktree>/crates/ekr-kernel)
error: method `schema_correction_operations` is never used
  --> crates/ekr-kernel/src/human_review/corrections.rs:16:19
   |
11 | impl VerifiedDecision {
   | --------------------- method in this implementation
...
16 |     pub(crate) fn schema_correction_operations(
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `-D dead-code` implied by `-D warnings`
   = help: to override `-D warnings` add `#[expect(dead_code)]` or `#[allow(dead_code)]`

error: function `schema_replacements` is never used
   --> crates/ekr-kernel/src/human_review/corrections.rs:172:4
    |
172 | fn schema_replacements(
    |    ^^^^^^^^^^^^^^^^^^^

error: function `schema_derive` is never used
   --> crates/ekr-kernel/src/human_review/corrections.rs:209:4
    |
209 | fn schema_derive(
    |    ^^^^^^^^^^^^^

error: could not compile `ekr-kernel` (lib) due to 3 previous errors
```

Exact-file rustfmt --edition 2021 --config skip_children=true --check and git diff --check: exit 0. Module registration ordering was formatted after the initial check named it; no behavioral source changed after the green runs. Builds used the coordinator-authorized exclusive sequential shared target, one job and no incremental debug artifacts. Observed disk floors passed: initially 11 GiB persistent/7.7 GiB tmpfs, later 12 GiB/7.7 GiB. The Cargo lane was explicitly returned after all processes terminated.

5. Limits and deferred work

- Tests obtain actual VerifiedDecision instances through Ed25519 signature verification under a host-pinned fixture policy. The graph comes from real File-provider seed and authority upgrade with two disputes. These are pure operation-boundary tests, not schema review material admission, store durability or F conformance scenarios.
- The signed proposal basis is an operation fixture; root separately verifies full schema material, effective review, continuation and atomic publication. This helper does not replace those checks.
- Statement evidence identity is caller-authenticated. Temporal additions preserve original and attached evidence through the unchanged derive function and add that statement ID. The EPOCH transaction-time placeholder matches existing attention derivation; root must bind final election time consistently.
- Pending Unresolved yields no transaction. The caller decides/report retains Partial/pending status; this unit writes no receipt.
- No generated/public model, ESS authoring, storage, schema application driver, pipeline authority, explanation, CLI/SDK, AEP, commit or remote publication changes. Synchronized coordinator ESS is deliberately absent from operations.patch. Generated drift and full task check were not run.

6. Outside-worktree writes

Only the assigned private schema-correction-ops evidence directory and coordinator-authorized shared Cargo build area were written. Outside-paths.txt retains exact private artifact paths. This portable report replaces personal prefixes in embedded output; raw logs are unchanged. The managed tree and prior mapping tree remain intact for coordinator integration and cleanup.
