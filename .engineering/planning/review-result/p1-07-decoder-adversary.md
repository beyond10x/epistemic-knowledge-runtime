---
format: aep.planning-md/2
id: review-result:p1-07-decoder-adversary
kind: review-result
status: active
title: Adversary of the ontology decoder correction
relations:
- reviews: story:refuse-discarded-ontology-semantics
revision: 1
---
unit: story:refuse-discarded-ontology-semantics; working tree ekr-p1-07-membrane on ed9aff46238c0a7a1cbe96de57935c96dad69d4b
verdict: nothing found
cases: executed 141→143, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 8 paths, listed below with private prefixes elided
needs-coordinator: record this review and the new test-file scope before integration
 crates/ekr-kernel/tests/validation.rs      |  43 +++++++++
 crates/ekr-ontology/src/lifecycle.rs       |   3 +
 crates/ekr-ontology/src/schema.rs          |   2 +
 crates/ekr-ontology/src/types.rs           |   3 +
 crates/ekr-ontology/src/value.rs           |   4 +-
 crates/ekr-ontology/tests/ontology_load.rs | 146 +++++++++++++++++++++++++++++
 6 files changed, 199 insertions(+), 2 deletions(-)

The raw tracked diff above was inherited intact from the implementor. Its source changes are not adversary edits. This pass authored only the new test file below; no existing test, source, planning record, or shared document was edited. No commit was made.
 .../ekr-ontology/tests/adversary_decoder.rs        | 96 ++++++++++++++++++++++
 1 file changed, 96 insertions(+)

1. Added cases and first executions

File: crates/ekr-ontology/tests/adversary_decoder.rs.

- record_keys_named_like_semantic_members_survive_nested_roundtrips: record fields named sets, constraints, value_kind, parameters, value, and unit remain data. Nested Record/List schema and values roundtrip without rejection or field loss through supported YAML and JSON forms.
- unknown_value_members_are_refused_beneath_a_record_key_and_list_element: the legitimate Record key sets remains permitted, but an unsupported unit member inserted into its nested List envelope or Integer element is refused and named in both YAML and JSON.

These are distinct boundaries beyond the implementation's top-level Value-envelope regression. Both cases were written before any test run in this pass. Each was first run alone; both are green, so no red output exists to report.

Every cargo command used CARGO_BUILD_JOBS=2, CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-07-membrane and TMPDIR=<cache>/ekr-completion-20260922/membrane/decoder-adversary. Free disk before compilation was 33 GB, above the 10 GB floor.

Command: cargo test -p ekr-ontology --test adversary_decoder record_keys_named_like_semantic_members_survive_nested_roundtrips -- --exact
Exit: 0
   Compiling ekr-ontology v0.0.0 (<worktrees>/ekr-p1-07-membrane/crates/ekr-ontology)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.10s
     Running tests/adversary_decoder.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/adversary_decoder-4cc40562133d594d)

running 1 test
test record_keys_named_like_semantic_members_survive_nested_roundtrips ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

Command: cargo test -p ekr-ontology --test adversary_decoder unknown_value_members_are_refused_beneath_a_record_key_and_list_element -- --exact
Exit: 0
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.08s
     Running tests/adversary_decoder.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/adversary_decoder-4cc40562133d594d)

running 1 test
test unknown_value_members_are_refused_beneath_a_record_key_and_list_element ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


2. Suite after the individual cases

The before count of 141 comes from the implementing agent's supplied report. This pass ran no baseline suite before adding its cases. The after count is the sum of runner summaries below, including doc tests and excluding separately counting nested trybuild fixtures.
Command: cargo test -p ekr-ontology -p ekr-kernel
Exit: 0
   Compiling ekr-ontology v0.0.0 (<worktrees>/ekr-p1-07-membrane/crates/ekr-ontology)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.54s
     Running unittests src/lib.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/ekr_kernel-dcebe1934f67cb0b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_membrane.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/adversary_membrane-f528e4a1cc1a1079)

running 5 tests
test an_add_assertion_over_an_id_canonical_state_already_holds_is_refused ... ok
test an_assertion_whose_only_evidence_the_proposer_invented_is_refused ... ok
test a_node_created_under_a_graph_root_that_does_not_exist_is_refused ... ok
test a_create_node_over_an_id_canonical_state_already_holds_is_refused ... ok
test reordering_the_operations_of_one_transaction_does_not_change_the_verdict ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_membrane_pass_two.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/adversary_membrane_pass_two-7a85a72ebf8dcd25)

running 4 tests
test a_node_is_not_merged_into_itself ... ok
test a_type_the_ontology_already_declares_is_not_declared_again ... ok
test a_proposer_cannot_mark_its_own_assertion_accepted ... ok
test an_assertion_naming_a_type_the_ontology_does_not_declare_is_refused ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_07.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/adversary_p1_07-d7562ee04b18eba3)

running 5 tests
test inherited_opaque_constraints_are_refused_on_child_node_properties ... ok
test retracting_a_retained_edge_assertion_does_not_erase_its_reference ... ok
test cancellation_cannot_leave_a_new_assertion_referring_to_the_cancelled_edge ... ok
test independent_property_and_lifecycle_writes_are_order_invariant ... ok
test assertion_subject_predicate_object_cross_product_obeys_declarations ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/commit_path.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/commit_path-816fb6c18858336f)

running 5 tests
test a_commit_into_an_unseeded_lineage_is_refused ... ok
test the_same_lineage_written_by_hand_does_not_advance_anything ... ok
test the_kernels_authority_stands_behind_exactly_what_it_validated ... ok
test a_transaction_validated_against_a_revision_the_lineage_moved_past_is_refused ... ok
test a_validated_transaction_commits_and_the_lineage_advances ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

     Running tests/encoding_field_order.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/encoding_field_order-2a7ec28608d4a18e)

running 1 test
test every_field_of_the_kernels_encodings_is_written_in_declaration_order ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/validate_properties.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/validate_properties-bebd2f6d483597a0)

running 5 tests
test the_two_walks_of_one_rule_agree ... ok
test no_transaction_is_ever_validated_by_its_own_proposer ... ok
test a_float_anywhere_is_always_refused ... ok
test validating_twice_gives_the_same_answer ... ok
test the_encoding_separates_transactions_that_differ ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/validation.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/validation-09bc7f3eafde1f2a)

running 49 tests
test a_transaction_with_no_operations_is_refused_by_the_structural_validator ... ok
test a_node_of_an_abstract_type_is_refused ... ok
test a_canonical_assertion_without_evidence_is_refused_by_the_provenance_validator ... ok
test a_merge_names_two_nodes ... ok
test a_reference_to_something_the_same_transaction_creates_resolves ... ok
test a_value_that_fails_its_type_is_refused_by_the_type_validator ... ok
test a_transaction_whose_validator_is_its_proposer_is_refused ... ok
test a_record_under_another_graph_root_is_refused ... ok
test a_value_canonical_state_does_not_admit_is_refused_with_its_path ... ok
test an_edge_cardinality_the_type_forbids_is_refused_by_the_cardinality_validator ... ok
test an_edge_endpoint_of_the_wrong_type_is_refused ... ok
test an_empty_property_assignment_still_requires_a_declared_property ... ok
test a_type_the_ontology_already_declares_is_not_declared_again ... ok
test a_valid_transaction_validates ... ok
test an_evidence_id_the_transaction_declares_does_not_make_it_exist ... ok
test a_valid_transaction_validates_to_the_same_hash_twice ... ok
test an_invoke_the_lifecycle_does_not_declare_is_refused_by_the_ontology_validator ... ok
test an_operation_the_type_does_not_declare_is_refused ... ok
test an_unresolvable_reference_is_refused_by_the_reference_validator ... ok
test an_identity_created_twice_in_one_transaction_is_refused ... ok
test an_invocation_carries_exactly_the_arguments_its_operation_declares ... ok
test an_unresolved_assertion_endpoint_does_not_acquire_an_unrelated_type_refusal ... ok
test deleting_an_edge_referenced_by_a_new_or_retained_assertion_is_refused ... ok
test competing_lifecycle_writes_are_refused ... ok
test every_type_refusal_the_validator_can_make_is_reachable ... ok
test opaque_edge_property_constraints_refuse_creation_and_assertions ... ok
test an_identity_canonical_state_already_holds_is_not_created_again ... ok
test an_assertion_cannot_arrive_carrying_its_own_verdict ... ok
test opaque_preconditions_and_emissions_are_not_silently_accepted ... ok
test an_assertions_types_are_resolved_whatever_shape_its_object_has ... ok
test applicable_opaque_property_constraints_refuse_all_node_write_paths ... ok
test property_assertions_check_type_objects_edge_properties_and_type_subjects ... ok
test competing_property_writes_are_refused_in_every_order ... ok
test every_field_of_every_encoded_type_reaches_its_encoding ... ok
test property_cardinality_uses_the_candidate_node ... ok
test property_cardinality_and_required_presence_are_refused ... ok
test every_kind_of_dangling_reference_is_refused ... ok
test the_encoding_writes_id_bearing_fields_in_declaration_order ... ok
test the_pipeline_runs_the_seven_deterministic_validators_in_order ... ok
test the_declared_evidence_set_is_the_evidence_the_assertions_cite ... ok
test the_eleven_operation_numbers_are_the_domains_and_the_declarations ... ok
test a_serialized_lifecycle_invokes_but_unsupported_sets_refuse_at_decode ... ok
test relation_assertions_check_both_endpoint_types_and_allow_inherited_types ... ok
test relation_assertions_require_compatible_node_endpoints ... ok
test unsupported_schema_changes_and_merges_refuse_explicitly ... ok
test the_validation_hash_covers_the_revision_it_was_validated_against ... ok
test two_transactions_that_differ_validate_to_different_hashes ... ok
test every_issue_code_the_kernel_can_raise_is_raised_by_a_case ... ok
    Checking ekr-kernel-tests v0.0.0 (<cache>/b10x-target/ekr-p1-07-membrane/tests/trybuild/ekr-kernel)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.07s


test tests/compile_fail/validated_transaction_cannot_be_deserialised.rs ... ok
test tests/compile_fail/validated_transaction_has_no_constructor_outside_the_kernel.rs ... ok


test only_the_kernel_constructs_a_validated_transaction ... ok

test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s

     Running unittests src/lib.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/ekr_ontology-a33d8483ba66af11)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_decoder.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/adversary_decoder-3f0e5810a44a9e85)

running 2 tests
test unknown_value_members_are_refused_beneath_a_record_key_and_list_element ... ok
test record_keys_named_like_semantic_members_survive_nested_roundtrips ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/canonical_admissibility.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/canonical_admissibility-26be980bc2e08ca8)

running 6 tests
test a_bare_float_is_refused_and_the_path_is_the_value_itself ... ok
test a_path_reads_from_the_whole_value_down_to_the_part ... ok
test a_float_inside_a_compound_is_refused_at_the_path_it_sits_at ... ok
test the_first_refused_value_in_the_values_own_order_is_the_one_named ... ok
test nesting_does_not_hide_a_float_at_any_depth ... ok
test every_kind_but_a_float_is_admissible_on_its_own ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/domain_projection.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/domain_projection-dd4be6766e122248)

running 3 tests
test a_timestamp_value_carries_a_timestamp ... ok
test the_domain_carries_node_ref_and_enum_parameters_and_no_other_compound_kind ... ok
test every_timestamp_the_domain_declares_is_carried_as_a_timestamp ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/hierarchy_specificity.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/hierarchy_specificity-cf37b6959f776402)

running 7 tests
test a_parent_that_specialises_another_parent_is_not_an_ambiguous_declaration ... ok
test the_reason_a_two_fault_document_is_refused_does_not_depend_on_id_order ... ok
test the_ambiguity_refusal_only_fires_when_neither_declaring_type_is_an_ancestor_of_the_other ... ok
test the_checker_resolves_allowed_types_by_the_same_order ... ok
test a_hierarchy_fault_is_reported_before_a_property_resolution_fault ... ok
test a_redeclaration_is_never_overridden_by_the_declaration_it_redeclares ... ok
test a_declaration_is_chosen_by_the_specialisation_order_and_by_nothing_else ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/inheritance_and_declaration_coherence.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/inheritance_and_declaration_coherence-15a311c7a8eecd5f)

running 13 tests
test an_operation_argument_no_value_inhabits_is_refused_at_load ... ok
test an_undeclared_grandparent_is_refused_as_an_unknown_type_and_not_as_a_cycle ... ok
test an_operation_argument_naming_an_undeclared_type_is_refused_at_load ... ok
test a_redeclared_property_is_resolved_by_the_hierarchy_and_not_by_type_id_order ... ok
test a_single_fault_hierarchy_is_refused_for_that_fault_whatever_the_id_order ... ok
test every_position_a_value_type_is_declared_in_is_walked_for_inhabitability ... ok
test the_nearest_declaration_wins_at_every_depth ... ok
test two_ancestors_declaring_one_property_identically_are_not_ambiguous ... ok
test two_ancestors_at_one_distance_declaring_a_property_differently_are_refused_at_load ... ok
test property_resolution_is_a_function_of_the_hierarchy_and_never_of_id_order ... ok
test the_crate_declares_no_value_type_field_the_walk_does_not_reach ... ok
test every_domain_name_this_crate_cites_is_declared_by_the_domain ... ok
test value_rs_does_not_attribute_its_serde_shape_to_the_ess_domain ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/lifecycle_transitions.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/lifecycle_transitions-8de4d97cb81233f0)

running 8 tests
test a_move_from_a_state_the_lifecycle_does_not_have_is_refused ... ok
test a_lifecycle_names_its_initial_state_among_its_states ... ok
test an_operation_that_declares_no_transition_leaves_the_state_alone ... ok
test a_move_from_a_state_the_node_is_not_in_is_refused ... ok
test every_declared_transition_is_accepted ... ok
test declares_answers_for_the_pair_and_not_for_its_endpoints ... ok
test preconditions_are_carried_as_opaque_text_and_refuse_nothing ... ok
test a_move_is_accepted_exactly_when_the_lifecycle_declares_it ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ontology_load.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/ontology_load-f6488c4d7e47bbcc)

running 15 tests
test a_cycle_in_the_parent_graph_is_refused_at_load ... ok
test a_node_ref_to_a_type_the_ontology_does_not_declare_is_refused_at_load ... ok
test a_node_ref_with_no_allowed_types_is_refused_at_load ... ok
test a_type_declared_twice_is_refused_at_load ... ok
test a_parent_the_ontology_does_not_declare_is_refused_at_load ... ok
test a_list_and_an_empty_record_are_not_in_that_class ... ok
test a_lifecycle_naming_a_state_it_does_not_have_is_refused_at_load ... ok
test an_edge_type_with_no_source_or_no_target_types_is_refused_at_load ... ok
test an_empty_compound_value_type_is_refused_at_every_depth_it_is_declared ... ok
test an_operation_with_a_transition_and_no_lifecycle_at_all_is_refused_at_load ... ok
test an_operation_whose_move_the_lifecycle_does_not_declare_is_refused_at_load ... ok
test value_envelopes_do_not_discard_unknown_semantics ... ok
test an_ontology_loads_from_yaml_and_refuses_the_same_documents ... ok
test unknown_semantics_on_compound_value_types_are_refused ... ok
test unknown_semantic_members_of_ontology_records_are_refused ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/type_hierarchy.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/type_hierarchy-6777223b6cfef7ba)

running 4 tests
test a_type_conforms_to_itself_and_to_every_ancestor_and_to_nothing_else ... ok
test a_types_properties_are_its_own_and_its_ancestors ... ok
test a_node_ref_accepts_a_node_whose_type_conforms_to_an_allowed_type ... ok
test an_inherited_required_property_is_required_of_the_descendant ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/value_type_checking.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/value_type_checking-df5650e8555792e0)

running 8 tests
test a_value_is_checkable_against_a_type_on_its_own ... ok
test cardinality_permits_the_counts_it_names ... ok
test an_abstract_or_unknown_type_is_not_instantiable ... ok
test a_refusal_names_the_property_and_the_reason ... ok
test every_value_kind_mirrors_its_value_type ... ok
test a_well_typed_value_checks_ok ... ok
test enum_variants_and_record_fields_are_enforced_inside_a_compound_value ... ok
test a_value_breaking_exactly_one_property_of_its_declared_type_is_refused ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

   Doc-tests ekr_kernel

running 2 tests
test crates/ekr-kernel/src/commit.rs - commit::Commit<S>::over (line 165) ... ok
test crates/ekr-kernel/src/lib.rs - (line 38) ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

   Doc-tests ekr_ontology

running 1 test
test crates/ekr-ontology/src/lib.rs - (line 21) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s

Command: cargo clippy -p ekr-ontology -p ekr-kernel --all-targets -- -D warnings
Exit: 0
    Checking ekr-ontology v0.0.0 (<worktrees>/ekr-p1-07-membrane/crates/ekr-ontology)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
Command: cargo fmt --all --check
Exit: 0; no output.

3. Findings

Nothing found in the bounded decoder repair. The supported lifecycle regression still passes, the serialized sets regression refuses, and the new nested data/semantic-member distinctions hold. This report does not approve the work or claim independent verification.

Owners: no findings to attribute. This adversary owns only adversary_decoder.rs; the inherited implementation belongs to the implementor and the integration records belong to the coordinator.

4. Attack coverage and limits

Read the full six-file implementation diff, current acceptance, implementation report, decoder brief, and recorded independent checkpoint. Attacked legitimate user map keys and nested unknown Value members; could not break either. The combined suite re-executed the existing nested ValueType and ontology-record refusal matrix plus real serialized lifecycle invocation control. No source mutation was performed. No other capability or later-phase completion is claimed.

5. Outside-worktree writes

The public report elides private prefixes; original logs retain actual runner paths in the assigned local scratch directory. No external path outside these assigned locations was used.

<cache>/b10x-target/ekr-p1-07-membrane
<cache>/ekr-completion-20260922/membrane/decoder-adversary
<cache>/ekr-completion-20260922/membrane/decoder-adversary/record-keys-first.log
<cache>/ekr-completion-20260922/membrane/decoder-adversary/nested-envelope-first.log
<cache>/ekr-completion-20260922/membrane/decoder-adversary/suite.log
<cache>/ekr-completion-20260922/membrane/decoder-adversary/clippy.log
<cache>/ekr-completion-20260922/membrane/decoder-adversary/fmt.log
<cache>/ekr-completion-20260922/membrane/decoder-adversary/report.md

```findings
[]
```
