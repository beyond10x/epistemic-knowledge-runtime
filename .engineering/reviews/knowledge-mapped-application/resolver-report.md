unit: story:apply-approved-refinement — shared deterministic application mapping unit
verdict: blocked
cases: executed unmeasured→73, red 6
origin: n/a
wrote-outside-worktree: assigned application-mapping scratch only; exact private inventory beside report
needs-coordinator: yes, mapping.patch and manual lib-registration.patch; connect application entry points and rerun strict clippy

Strict kernel library clippy remains blocked solely by four new crate-private entry points without application callers in this isolated tree: resolve, payload, digest and item. No suppression or artificial caller was introduced. The shared resolver is already used by the real preview path. Focused mapping, library, retention and schema-review tests pass. This is no partial F acceptance claim.

1. Unit and acceptance

Extract the existing preview's selectors, typed value checks and canonical alias/endpoint resolution into one deterministic mapping_material function. Application returns either canonical subject/predicate/object with the exact selected fact evidence sequence, or the same per-item blockers as non-strict preview. The source remains immutable, and the resolver mints no identity or entity. Compact generated KnowledgeMapping JSON supplies independently addressable payload bytes and ContentHash; ApplicationItemKey preserves full source version, canonical facts[index] and mapping digest.

Base: e2b3c499228cbb7cf09a7ba9f69b321e95552eb4. Branch: ekr/application-mapping-20261004. Scope checked against AGENTS.md, story delegation and design 105.17. Canonical names are explicitly not aliases; unknown and ambiguous references remain blocked. All ten canonical value kinds are covered through Constant and CopyField; CopyRelation and NodeRef relation constants share endpoint checks. Preview retains its existing bindings, dependent claims and current property/relation declarations for review material. The node-type map is still collected once per preview batch.

2. Exact source patch

 crates/ekr-kernel/src/schema_proposal_mapping.rs   |  350 +++++++++++-------
 crates/ekr-kernel/src/application_mapping.rs       |   61 +++
 crates/ekr-kernel/src/application_mapping/tests.rs |  387 ++++++++++++++++++++
 3 files changed, 654 insertions(+), 144 deletions(-)


mapping.patch SHA256: 9181d83a66f3a73e6d0c4432c69f62ed7016258430ceb3d8e2d80f63b841bff0.
source-sha256.txt inventory digest: f4be4b1398ecc4a86859279c758b9fbc857230879aa1fe13cf92f9d1c23e2f90.
The one-line lib.rs registration is a separate patch for root integration. Parent-synchronized kernel/integrate ESS files are excluded from both patches. Previous frozen codec/projection trees are preserved.

3. Observed red

Command: cargo test --locked -p ekr-kernel --lib application_mapping::tests
Exit: 101. An initial compile probe exposed an incorrect Subject generic in the new stub; it was corrected before this executed run.

Five tests then failed at missing resolver/hash helpers. The sixth exposed a fixture problem before reaching the new resolver: the relation object had a canonical name but lacked the explicit alias used by the unchanged preview. Source inspection confirmed AliasIndex excludes canonical_name. The positive fixture now supplies an alias, and the final test retains a separate canonical-name-only blocked control. This was not a production resolver defect.

```text
   Compiling ekr-kernel v0.0.27 (<managed-worktree>)
    Finished `test` profile [unoptimized] target(s) in 9.77s
     Running unittests src/lib.rs (<shared-target>)

running 6 tests
test application_mapping::tests::relation_missing_ambiguous_and_outside_endpoints_stay_blocked ... FAILED
test application_mapping::tests::mapping_payload_digest_and_qualified_keys_distinguish_each_mapping_and_source ... FAILED
test application_mapping::tests::resolved_property_uses_alias_and_exact_selected_fact_evidence ... FAILED
test application_mapping::tests::all_canonical_value_kinds_work_for_constant_and_copy_field ... FAILED
test application_mapping::tests::unresolved_ambiguous_and_invalid_selectors_match_preview_blockers ... FAILED
test application_mapping::tests::relation_copy_and_noderef_constant_check_both_endpoints ... FAILED

failures:

---- application_mapping::tests::relation_missing_ambiguous_and_outside_endpoints_stay_blocked stdout ----

thread 'application_mapping::tests::relation_missing_ambiguous_and_outside_endpoints_stay_blocked' (962813) panicked at crates/ekr-kernel/src/application_mapping/tests.rs:130:10:
called `Result::unwrap()` on an `Err` value: Document("application-mapping: absent")

---- application_mapping::tests::mapping_payload_digest_and_qualified_keys_distinguish_each_mapping_and_source stdout ----

thread 'application_mapping::tests::mapping_payload_digest_and_qualified_keys_distinguish_each_mapping_and_source' (962811) panicked at crates/ekr-kernel/src/application_mapping/tests.rs:302:34:
called `Result::unwrap()` on an `Err` value: Document("application-mapping: absent")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- application_mapping::tests::resolved_property_uses_alias_and_exact_selected_fact_evidence stdout ----

thread 'application_mapping::tests::resolved_property_uses_alias_and_exact_selected_fact_evidence' (962814) panicked at crates/ekr-kernel/src/application_mapping/tests.rs:130:10:
called `Result::unwrap()` on an `Err` value: Document("application-mapping: absent")

---- application_mapping::tests::all_canonical_value_kinds_work_for_constant_and_copy_field stdout ----

thread 'application_mapping::tests::all_canonical_value_kinds_work_for_constant_and_copy_field' (962810) panicked at crates/ekr-kernel/src/application_mapping/tests.rs:130:10:
called `Result::unwrap()` on an `Err` value: Document("application-mapping: absent")

---- application_mapping::tests::unresolved_ambiguous_and_invalid_selectors_match_preview_blockers stdout ----

thread 'application_mapping::tests::unresolved_ambiguous_and_invalid_selectors_match_preview_blockers' (962815) panicked at crates/ekr-kernel/src/application_mapping/tests.rs:130:10:
called `Result::unwrap()` on an `Err` value: Document("application-mapping: absent")

---- application_mapping::tests::relation_copy_and_noderef_constant_check_both_endpoints stdout ----

thread 'application_mapping::tests::relation_copy_and_noderef_constant_check_both_endpoints' (962812) panicked at crates/ekr-kernel/src/application_mapping/tests.rs:277:9:
assertion failed: fixture.preview().is_empty()


failures:
    application_mapping::tests::all_canonical_value_kinds_work_for_constant_and_copy_field
    application_mapping::tests::mapping_payload_digest_and_qualified_keys_distinguish_each_mapping_and_source
    application_mapping::tests::relation_copy_and_noderef_constant_check_both_endpoints
    application_mapping::tests::relation_missing_ambiguous_and_outside_endpoints_stay_blocked
    application_mapping::tests::resolved_property_uses_alias_and_exact_selected_fact_evidence
    application_mapping::tests::unresolved_ambiguous_and_invalid_selectors_match_preview_blockers

test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 45 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p ekr-kernel --lib`
```

4. Green and regression evidence

Focused command: cargo test --locked -p ekr-kernel --lib application_mapping::tests
Focused lane: executed 6 → 6, exit 0. Tests now cover all ten canonical value kinds, selectors, evidence retention, unknown/ambiguous entities, source/target endpoints and qualified identities; the later explicit name-only negative control also passes.

```text
   Compiling ekr-kernel v0.0.27 (<managed-worktree>)
    Finished `test` profile [unoptimized] target(s) in 8.59s
     Running unittests src/lib.rs (<shared-target>)

running 6 tests
test application_mapping::tests::mapping_payload_digest_and_qualified_keys_distinguish_each_mapping_and_source ... ok
test application_mapping::tests::resolved_property_uses_alias_and_exact_selected_fact_evidence ... ok
test application_mapping::tests::relation_copy_and_noderef_constant_check_both_endpoints ... ok
test application_mapping::tests::all_canonical_value_kinds_work_for_constant_and_copy_field ... ok
test application_mapping::tests::relation_missing_ambiguous_and_outside_endpoints_stay_blocked ... ok
test application_mapping::tests::unresolved_ambiguous_and_invalid_selectors_match_preview_blockers ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 45 filtered out; finished in 0.04s

```

Regression command: cargo test --locked -p ekr-kernel --lib --test knowledge_retention --test schema_proposal_reviews
Exit: 0. Actual runner counts: library 51, knowledge_retention 14, schema_proposal_reviews 8. No base-run counts were separately measured for this exact new base; the aggregate 73 is the sum of the three actual summary lines, not an inferred baseline comparison. The final source change after this regression run only extended one existing focused test with the canonical-name-only blocked control; that focused lane was rerun green.

```text
   Compiling ekr-kernel v0.0.27 (<managed-worktree>)
warning: function `resolve` is never used
  --> crates/ekr-kernel/src/application_mapping.rs:25:15
   |
25 | pub(crate) fn resolve(
   |               ^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: function `payload` is never used
  --> crates/ekr-kernel/src/application_mapping.rs:43:15
   |
43 | pub(crate) fn payload(mapping: &w::EkrIntegrateKnowledgeMapping) -> Result<Vec<u8>, StoreError> {
   |               ^^^^^^^

warning: function `digest` is never used
  --> crates/ekr-kernel/src/application_mapping.rs:46:15
   |
46 | pub(crate) fn digest(mapping: &w::EkrIntegrateKnowledgeMapping) -> Result<ContentHash, StoreError> {
   |               ^^^^^^

warning: function `item` is never used
  --> crates/ekr-kernel/src/application_mapping.rs:50:15
   |
50 | pub(crate) fn item(
   |               ^^^^

warning: `ekr-kernel` (lib) generated 4 warnings
    Finished `test` profile [unoptimized] target(s) in 10.36s
     Running unittests src/lib.rs (<shared-target>)

running 51 tests
test application_plan::tests::application_schema_plan_cannot_redeclare_an_existing_property ... ok
test application_plan::tests::application_schema_plan_refuses_destructive_changes_and_empty_effects ... ok
test application_plan::tests::frozen_schema_plan_preserves_allocations_and_existing_declarations ... ok
test application_projection::tests::absent_optionals_and_empty_collections_remain_distinct ... ok
test application_projection::tests::every_operation_union_alternative_keeps_its_tag ... ok
test application_projection::tests::mixed_application_history_keeps_nested_payloads_and_coordinates ... ok
test application_transaction::tests::unrepresentable_timestamps_and_nested_values_fail_closed ... ok
test application_projection::tests::proposal_show_exposes_receipts_and_application_history ... ok
test application_transaction::tests::canonical_value_bytes_and_kind_are_both_checked ... ok
test application_transaction::tests::normalization_is_limited_to_empty_aliases_and_equivalent_instants ... ok
test application_transaction::tests::all_operations_preserve_native_fields_and_canonical_bytes ... ok
test application_transaction::tests::optional_owner_schema_and_empty_aliases_preserve_historical_meaning ... ok
test application_transaction::tests::every_assertion_and_evidence_payload_variant_round_trips ... ok
test application_projection::tests::numeric_overflow_and_invalid_bytes_refuse_instead_of_narrowing ... ok
test document::checked::tests::string_refusals_precede_the_allocating_application_visitor ... ok
test document::checked::tests::mixed_accounting_refuses_before_allocating_a_coerced_string ... ok
test document::bounded_load::a_transaction_document_nested_past_the_depth_is_refused_at_the_first_container_past_it ... ok
test application_mapping::tests::resolved_property_uses_alias_and_exact_selected_fact_evidence ... ok
test application_mapping::tests::relation_copy_and_noderef_constant_check_both_endpoints ... ok
test incubation_value::tests::existing_canonical_encoder_is_the_oracle_for_every_value_kind ... ok
test application_transaction::tests::inconsistent_projection_payloads_are_not_silently_dropped ... ok
test application_mapping::tests::mapping_payload_digest_and_qualified_keys_distinguish_each_mapping_and_source ... ok
test application_mapping::tests::all_canonical_value_kinds_work_for_constant_and_copy_field ... ok
test application_transaction::tests::invalid_numeric_ranges_and_interval_order_are_refused ... ok
test application_transaction::tests::malformed_identity_and_duplicate_manifest_are_refused ... ok
test application_mapping::tests::relation_missing_ambiguous_and_outside_endpoints_stay_blocked ... ok
test application_mapping::tests::unresolved_ambiguous_and_invalid_selectors_match_preview_blockers ... ok
test replay::tests::retirement_requires_the_confirmed_occurrence_and_matching_prefix ... ok
test checkpoint::tests::held_identities_are_read_from_each_receipt_without_parsing_its_proposal ... ok
test seed::bounded_load::a_seed_nested_past_the_depth_is_refused_at_the_first_container_past_it ... ok
test seed::shared_payloads::a_carried_payload_becomes_the_retained_allocation_only_for_verified_equal_bytes ... ok
test validate::candidate::tests::a_candidate_borrows_existing_node_types_at_every_graph_size ... ok
test replay::tests::prefix_hash_memo_compares_every_occurrence_and_preserves_held_digest_vectors ... ok
test replay::tests::node_only_commits_share_the_unchanged_assertion_edge_index ... ok
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
test replay::tests::one_handle_validates_against_an_earlier_revision_whose_graph_it_released ... ok
test answers::tests::reviewed_answers_publish_retry_reopen_and_fully_replay_on_both_providers ... ok
test replay::tests::warm_validation_hashes_no_more_prefix_occurrences_as_history_grows ... ok
test replay::tests::validating_one_transaction_does_not_copy_prior_document_buffers ... ok
test replay::tests::append_commits_copy_no_more_assertions_as_the_graph_grows ... ok

test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.71s

     Running tests/knowledge_retention.rs (<shared-target>)

running 14 tests
test inconsistent_observation_is_refused_without_retaining_anything ... ok
test retained_sources_pin_exact_bytes_on_both_providers ... ok
test generated_observation_commands_preserve_provider_faults ... ok
test rejected_interpretation_retains_its_sources ... ok
test observation_retry_is_idempotent ... ok
test concurrent_imports_choose_one_immutable_record ... ok
test invalid_local_shape_and_evidence_are_refused_before_any_publication ... ok
test unmapped_knowledge_survives_reopen ... ok
test schema_proposal_submission_retains_exact_bytes_without_admitting_facts ... ok
test schema_gap_discovery_rechecks_the_current_schema_and_preserves_import_history ... ok
test schema_gap_discovery_is_stable_after_replay_and_does_not_mutate_canonical_state ... ok
test schema_proposal_refuses_unreviewable_sources_selectors_and_constants_before_retention ... ok
test schema_proposal_schema_dependencies_change_review_material_but_unrelated_schema_does_not ... ok
test schema_proposal_remains_inspectable_after_a_mapped_property_changes_kind ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s

     Running tests/schema_proposal_reviews.rs (<shared-target>)

running 8 tests
test schema_review_cannot_reuse_an_authority_upgrade_decision_identity ... ok
test proposal_submission_refuses_inapplicable_corrections ... ok
test proposal_correction_review_binds_the_full_competing_component ... ok
test concurrent_identical_schema_reviews_return_one_elected_record ... ok
test schema_reviews_and_attention_answers_share_decision_identity_in_both_directions ... ok
test correction_review_survives_unrelated_advancement_and_remains_historical_after_resolution ... ok
test schema_review_revalidates_material_and_replays_the_original_review_revision ... ok
test schema_proposal_requires_exact_human_approval ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s

```

Strict lint command: cargo clippy --locked -p ekr-kernel --lib -- -D warnings
Exit: 101, four unwired entry points only. Root must connect real application callers and repeat strict clippy.

```text
    Checking serde_yaml_ng v0.10.0 (<managed-worktree>)
    Checking ekr-contract-data v0.0.0 (<managed-worktree>)
    Checking ekr-types v1.0.0 (<managed-worktree>)
    Checking ekr-core v0.0.27 (<managed-worktree>)
    Checking ekr-ontology v0.0.27 (<managed-worktree>)
    Checking ekr-graph v0.0.27 (<managed-worktree>)
    Checking ekr-store v0.0.27 (<managed-worktree>)
    Checking ekr-integrate v0.0.27 (<managed-worktree>)
    Checking ekr-kernel v0.0.27 (<managed-worktree>)
error: function `resolve` is never used
  --> crates/ekr-kernel/src/application_mapping.rs:25:15
   |
25 | pub(crate) fn resolve(
   |               ^^^^^^^
   |
   = note: `-D dead-code` implied by `-D warnings`
   = help: to override `-D warnings` add `#[expect(dead_code)]` or `#[allow(dead_code)]`

error: function `payload` is never used
  --> crates/ekr-kernel/src/application_mapping.rs:43:15
   |
43 | pub(crate) fn payload(mapping: &w::EkrIntegrateKnowledgeMapping) -> Result<Vec<u8>, StoreError> {
   |               ^^^^^^^

error: function `digest` is never used
  --> crates/ekr-kernel/src/application_mapping.rs:46:15
   |
46 | pub(crate) fn digest(mapping: &w::EkrIntegrateKnowledgeMapping) -> Result<ContentHash, StoreError> {
   |               ^^^^^^

error: function `item` is never used
  --> crates/ekr-kernel/src/application_mapping.rs:50:15
   |
50 | pub(crate) fn item(
   |               ^^^^

error: could not compile `ekr-kernel` (lib) due to 4 previous errors
```

Owned-file rustfmt --edition 2021 --check: exit 0. git diff --check: exit 0. Cargo used the expressly authorized sequential shared target with one build job; observed free space was 21 GiB persistent and 7.8 GiB tmpfs, above the stated floors. Lane released to root with no live Cargo.

5. Boundaries and remaining work

resolve requires the caller's verified read, checked ontology and already authenticated Source map; it does not authenticate bytes, approve a proposal or validate provenance sufficiency. Returned evidence is exactly the selected fact's evidence sequence, including its order. Publication must still enforce ordinary canonical admission, retained admissible evidence, effective human review and application election/replay rules. A native Value that cannot become canonical fails closed through the shared resolver.

Per-proposal duplicate source/item/target admission and corrections remain in preview's existing batch path. Individual resolve calls carry no batch or authorization authority. Mapping bytes/digests are pure transport coordinates; item additionally refuses noncanonical facts[index] syntax. No persistence, approval, residual-review, source-evidence admission, schema_application/application_auth, CLI/SDK, AEP or remote write was implemented. ESS regeneration and the synchronized amendment belong to root. No commits or publication were made.

6. External paths

private-inventory.txt lists every absolute scratch path written by this unit. Raw logs, source hashes, formatting/diff checks and the exact patches are colocated in assigned application-mapping scratch. This report uses portable names. The managed tree remains uncommitted for root's independent review and integration; its own lease is released at handoff.
