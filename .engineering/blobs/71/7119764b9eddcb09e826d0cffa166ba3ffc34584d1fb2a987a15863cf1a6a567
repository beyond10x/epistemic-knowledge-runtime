---
format: aep.planning-md/1
id: review-result:p1-07-membrane-adversary
kind: review-result
status: active
title: P1-07 membrane adversary
relations:
- reviews: story:p1-transaction-membrane-repair
revision: 1
---
unit: p1-transaction-membrane-repair; working tree ekr-p1-07-membrane on 581005b9a1239866c674418aeece170bb7031002
verdict: nothing found
cases: executed 70→75, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 11 paths (10 retained files and the assigned build directory)
needs-coordinator: preserve the new test file and integrate the inherited uncommitted implementation
 crates/ekr-kernel/src/validate/cardinality.rs |  69 +---
 crates/ekr-kernel/src/validate/mod.rs         |  16 +-
 crates/ekr-kernel/src/validate/ontology.rs    |  59 ++-
 crates/ekr-kernel/src/validate/reference.rs   |  43 ++-
 crates/ekr-kernel/src/validate/structural.rs  |  75 +++-
 crates/ekr-kernel/src/validate/types.rs       | 214 ++++++++---
 crates/ekr-kernel/tests/adversary_membrane.rs |   9 +
 crates/ekr-kernel/tests/validation.rs         | 501 ++++++++++++++++++++++++--
 8 files changed, 809 insertions(+), 177 deletions(-)

The raw tracked diff above was already present when this adversary started. It contains the implementor's source and test edits; candidate.rs was also already untracked. This pass did not edit any of those files. Since the coordinator supplied an uncommitted implementation, this raw diff cannot alone distinguish authorship. This pass's only repository write is the following new test file:
 .../ekr-kernel/tests/adversary_p1_07.rs            | 497 +++++++++++++++++++++
 1 file changed, 497 insertions(+)

1. Cases added and first executions

All five cases are in crates/ekr-kernel/tests/adversary_p1_07.rs, written before the first compiler invocation in this pass. All are green now. No red output exists: these attacks did not break the change.

- assertion_subject_predicate_object_cross_product_obeys_declarations: 45 combinations of three subjects, three predicates (string property, reference property, relation), and five objects. Accepted combinations succeed; rejected combinations belong solely to the type validator.
- cancellation_cannot_leave_a_new_assertion_referring_to_the_cancelled_edge: creation/deletion cancellation succeeds in both orders; adding an assertion pointing to the cancelled edge refuses solely with unresolved-edge in three permutations.
- retracting_a_retained_edge_assertion_does_not_erase_its_reference: deletion without a dependent assertion succeeds; deletion plus retraction of a retained dependent assertion refuses in both orders because retraction retains the reference.
- independent_property_and_lifecycle_writes_are_order_invariant: a property update and a declared lifecycle transition coexist in both orders; identical property assignments succeed; creating and invoking a node succeeds in both orders.
- inherited_opaque_constraints_are_refused_on_child_node_properties: an inherited opaque constraint refuses a child-node property update solely through unsupported-constraint.

Each first command was CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-07-membrane cargo test -p ekr-kernel --test adversary_p1_07 CASE -- --exact. Each exited 0. First-run outputs, verbatim:

   Compiling ekr-kernel v0.0.0 (<worktrees>/ekr-p1-07-membrane/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.65s
     Running tests/adversary_p1_07.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/adversary_p1_07-d7562ee04b18eba3)

running 1 test
test assertion_subject_predicate_object_cross_product_obeys_declarations ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/adversary_p1_07.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/adversary_p1_07-d7562ee04b18eba3)

running 1 test
test cancellation_cannot_leave_a_new_assertion_referring_to_the_cancelled_edge ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/adversary_p1_07.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/adversary_p1_07-d7562ee04b18eba3)

running 1 test
test retracting_a_retained_edge_assertion_does_not_erase_its_reference ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/adversary_p1_07.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/adversary_p1_07-d7562ee04b18eba3)

running 1 test
test independent_property_and_lifecycle_writes_are_order_invariant ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/adversary_p1_07.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/adversary_p1_07-d7562ee04b18eba3)

running 1 test
test inherited_opaque_constraints_are_refused_on_child_node_properties ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s


2. Package suite, after the new cases' individual executions

Command: CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-07-membrane cargo test -p ekr-kernel
Exit status: 0
Before count: 70, from the implementor's declared green report, not a pre-attack suite run.
After count: 75, the sum of the runner result counts below, including two doc tests. Nested trybuild fixtures are not added separately to their enclosing case.

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running unittests src/lib.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/ekr_kernel-dcebe1934f67cb0b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_membrane.rs (<cache>/b10x-target/ekr-p1-07-membrane/debug/deps/adversary_membrane-f528e4a1cc1a1079)

running 5 tests
test an_add_assertion_over_an_id_canonical_state_already_holds_is_refused ... ok
test an_assertion_whose_only_evidence_the_proposer_invented_is_refused ... ok
test a_create_node_over_an_id_canonical_state_already_holds_is_refused ... ok
test a_node_created_under_a_graph_root_that_does_not_exist_is_refused ... ok
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
test a_validated_transaction_commits_and_the_lineage_advances ... ok
test the_kernels_authority_stands_behind_exactly_what_it_validated ... ok
test a_transaction_validated_against_a_revision_the_lineage_moved_past_is_refused ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

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

running 48 tests
test a_transaction_with_no_operations_is_refused_by_the_structural_validator ... ok
test a_merge_names_two_nodes ... ok
test a_canonical_assertion_without_evidence_is_refused_by_the_provenance_validator ... ok
test a_reference_to_something_the_same_transaction_creates_resolves ... ok
test a_node_of_an_abstract_type_is_refused ... ok
test a_transaction_whose_validator_is_its_proposer_is_refused ... ok
test a_value_that_fails_its_type_is_refused_by_the_type_validator ... ok
test a_value_canonical_state_does_not_admit_is_refused_with_its_path ... ok
test a_record_under_another_graph_root_is_refused ... ok
test a_type_the_ontology_already_declares_is_not_declared_again ... ok
test an_edge_cardinality_the_type_forbids_is_refused_by_the_cardinality_validator ... ok
test an_edge_endpoint_of_the_wrong_type_is_refused ... ok
test a_valid_transaction_validates_to_the_same_hash_twice ... ok
test an_evidence_id_the_transaction_declares_does_not_make_it_exist ... ok
test a_valid_transaction_validates ... ok
test an_empty_property_assignment_still_requires_a_declared_property ... ok
test an_invoke_the_lifecycle_does_not_declare_is_refused_by_the_ontology_validator ... ok
test an_identity_created_twice_in_one_transaction_is_refused ... ok
test an_unresolvable_reference_is_refused_by_the_reference_validator ... ok
test an_operation_the_type_does_not_declare_is_refused ... ok
test competing_lifecycle_writes_are_refused ... ok
test an_invocation_carries_exactly_the_arguments_its_operation_declares ... ok
test an_unresolved_assertion_endpoint_does_not_acquire_an_unrelated_type_refusal ... ok
test an_identity_canonical_state_already_holds_is_not_created_again ... ok
test an_assertions_types_are_resolved_whatever_shape_its_object_has ... ok
test deleting_an_edge_referenced_by_a_new_or_retained_assertion_is_refused ... ok
test opaque_edge_property_constraints_refuse_creation_and_assertions ... ok
test applicable_opaque_property_constraints_refuse_all_node_write_paths ... ok
test property_cardinality_and_required_presence_are_refused ... ok
test an_assertion_cannot_arrive_carrying_its_own_verdict ... ok
test every_type_refusal_the_validator_can_make_is_reachable ... ok
test opaque_preconditions_and_emissions_are_not_silently_accepted ... ok
test relation_assertions_check_both_endpoint_types_and_allow_inherited_types ... ok
test every_kind_of_dangling_reference_is_refused ... ok
test competing_property_writes_are_refused_in_every_order ... ok
test property_cardinality_uses_the_candidate_node ... ok
test every_field_of_every_encoded_type_reaches_its_encoding ... ok
test property_assertions_check_type_objects_edge_properties_and_type_subjects ... ok
test relation_assertions_require_compatible_node_endpoints ... ok
test the_encoding_writes_id_bearing_fields_in_declaration_order ... ok
test the_pipeline_runs_the_seven_deterministic_validators_in_order ... ok
test the_declared_evidence_set_is_the_evidence_the_assertions_cite ... ok
test unsupported_schema_changes_and_merges_refuse_explicitly ... ok
test two_transactions_that_differ_validate_to_different_hashes ... ok
test the_validation_hash_covers_the_revision_it_was_validated_against ... ok
test the_eleven_operation_numbers_are_the_domains_and_the_declarations ... ok
test every_issue_code_the_kernel_can_raise_is_raised_by_a_case ... ok
    Checking ekr-kernel-tests v0.0.0 (<cache>/b10x-target/ekr-p1-07-membrane/tests/trybuild/ekr-kernel)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s


test tests/compile_fail/validated_transaction_cannot_be_deserialised.rs ... ok
test tests/compile_fail/validated_transaction_has_no_constructor_outside_the_kernel.rs ... ok


test only_the_kernel_constructs_a_validated_transaction ... ok

test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s

   Doc-tests ekr_kernel

running 2 tests
test crates/ekr-kernel/src/commit.rs - commit::Commit<S>::over (line 165) ... ok
test crates/ekr-kernel/src/lib.rs - (line 38) ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s


Additional checks: cargo clippy -p ekr-kernel --all-targets -- -D warnings exited 0; cargo fmt --all --check exited 0. The same assigned build directory and two-job limit applied to clippy.

3. Findings

Nothing found within the assigned membrane acceptance. No source mutation was used. No implementation or existing test was edited.

Owners: no findings to attribute; the new adversarial test file belongs to this adversary pass. The inherited implementation remains the implementor's work, and integration/planning records remain the coordinator's.

4. Attack coverage and limits

Assertion typing: positive and negative combinations against the actual declared ontology; could not break.
Surviving references: deleted/cancelled edges, new assertions, retained assertions, and retraction permutations; could not break.
Unordered writes: independent property/lifecycle operations, identical property updates, create/invoke orderings; could not break.
Opaque constraints: inherited declarations remain refused by the intended validator; could not break.
Unsupported schema/merge operations and opaque invocation effects: inspected complete changed source and implementation regressions, all exercised again by the package suite; no additional failing case found.
Seed admission, durable application, persisted authority, authentication at submission, migrations, and complete graph projections were excluded by the brief. This report makes no completion claim about them.

5. Paths written outside the worktree

<cache>/b10x-target/ekr-p1-07-membrane
<cache>/ekr-completion-20260922/membrane/adversary-fixture-fragment.rs
<cache>/ekr-completion-20260922/membrane/adversary-matrix-first.log
<cache>/ekr-completion-20260922/membrane/adversary-cancellation_cannot_leave_a_new_assertion_referring_to_the_cancelled_edge-first.log
<cache>/ekr-completion-20260922/membrane/adversary-retracting_a_retained_edge_assertion_does_not_erase_its_reference-first.log
<cache>/ekr-completion-20260922/membrane/adversary-independent_property_and_lifecycle_writes_are_order_invariant-first.log
<cache>/ekr-completion-20260922/membrane/adversary-inherited_opaque_constraints_are_refused_on_child_node_properties-first.log
<cache>/ekr-completion-20260922/membrane/adversary-suite.log
<cache>/ekr-completion-20260922/membrane/adversary-clippy.log
<cache>/ekr-completion-20260922/membrane/adversary-fmt.log
<cache>/ekr-completion-20260922/membrane/adversary-report.md

```findings
[]
```
