unit:                   story:kernel-validated-seed — property-definition filing correction
verdict:                green
cases:                  executed 288→290, red 3
origin:                 n/a
wrote-outside-worktree: assigned seed scratch logs/report and dedicated seed target
needs-coordinator:      no extra patch or scope; ready for independent re-review

## 1. Finding and correction

The seed adversary's sole blocker is corrected in the shared loader. Ontology::check_properties
now compares each property-map key to PropertyDefinition.id before examining its declared value
type. Both NodeType.properties and EdgeType.properties call this common method. Refusal is the
typed OntologyError::MisfiledProperty { key: PropertyId, declared: PropertyId }; Display names both.

Class: a property definition indexed by one stable identity must declare that same identity.
Enumeration: crates/ekr-ontology/src/types.rs has two BTreeMap<PropertyId, PropertyDefinition>
fields, NodeType.properties and EdgeType.properties. Both use the corrected common check.
Inherited node properties are declarations on one of those node types and remain checked at load.
Record value-type field names remain ordinary user map keys, unchanged. No second kernel-only
schema validator was introduced.

Scope read and confirmed:
- crates/ekr-ontology/src/schema.rs: existing values-only common property check and typed error enum;
  this is the shared correction point the coordinator identified.
- crates/ekr-ontology/tests/ontology_load.rs: existing loader regression suite; two independent
  exact-variant cases added with in-memory and YAML paths plus matching-identity positive controls.
- crates/ekr-kernel/tests/adversary_p1_08_seed.rs: read only, unchanged. Before and after SHA-256:
  a3152f04d6a02c28be6dc3b4327a7f8dec3fcf1ec1177c98a7d4fba78c4920ce.

The independent seed case still opens a valid compatibility ontology, submits the malformed seed
input and requires seed-ontology (not seed-ontology-mismatch), with no seed object/event. It passes
on SQLite/node, SQLite/edge, File/node and File/edge. No panic, skip or weakened witness.

## 2. Actual correction diff

Only the two ontology files were edited by this correction dispatch. Earlier seed implementation,
independent adversary case, coordinator ESS/AGENTS work and all other inherited changes remain.
```
 crates/ekr-ontology/src/schema.rs          | 18 +++++++++++--
 crates/ekr-ontology/tests/ontology_load.rs | 43 ++++++++++++++++++++++++++++++
 2 files changed, 59 insertions(+), 2 deletions(-)
```

## 3. Red first

Before edits, the four-package suite ran with --no-fail-fast and exited 101: 288 executed,
287 passed, one failed (the unchanged independent filing witness). Full original output:
<cache>/ekr-completion-20260922/seed/correction-baseline.log.

The new typed error variant was staged without changing loader behavior, so the exact-variant
tests compiled and their failures were behavioral, not missing-symbol errors.

```sh
CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-08-seed CARGO_BUILD_JOBS=2 TMPDIR=<cache>/ekr-completion-20260922/seed cargo test -p ekr-ontology --test ontology_load property_definition_filed
# exit 101; 2 executed, both failed
```
```
   Compiling serde_core v1.0.229
   Compiling serde v1.0.229
   Compiling serde_json v1.0.151
   Compiling serde_yaml_ng v0.10.0
   Compiling ekr-core v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-core)
   Compiling ekr-ontology v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-ontology)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.48s
     Running tests/ontology_load.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/ontology_load-772bbf00ea3f3baa)

running 2 tests
test an_edge_property_definition_filed_under_another_id_is_refused_at_load ... FAILED
test a_node_property_definition_filed_under_another_id_is_refused_at_load ... FAILED

failures:

---- an_edge_property_definition_filed_under_another_id_is_refused_at_load stdout ----

thread 'an_edge_property_definition_filed_under_another_id_is_refused_at_load' (255147) panicked at crates/ekr-ontology/tests/ontology_load.rs:98:49:
called `Result::unwrap_err()` on an `Ok` value: Ontology { version: SchemaVersion { id: SchemaVersionId(2164032953588709760371108244551982737), number: 0, parent: None, created_at: Timestamp(0) }, node_types: {TypeId(2164032953588709760371108226579424380): NodeType { id: TypeId(2164032953588709760371108226579424380), name: "Subject", parents: {}, properties: {PropertyId(2164032953588709760371108237382579353): PropertyDefinition { id: PropertyId(2164032953588709760371108237382579353), name: "p", value_type: String, cardinality: One, required: false, constraints: [] }}, abstract_type: false, lifecycle: None, operations: {} }, TypeId(2164032953588709760371108230584549592): NodeType { id: TypeId(2164032953588709760371108230584549592), name: "Other", parents: {}, properties: {}, abstract_type: false, lifecycle: None, operations: {} }}, edge_types: {TypeId(2164032953588709760371108261350145535): EdgeType { id: TypeId(2164032953588709760371108261350145535), name: "depends_on", source_types: {TypeId(2164032953588709760371108226579424380)}, target_types: {TypeId(2164032953588709760371108230584549592)}, cardinality: One, properties: {PropertyId(2164032953588709760371108253373722875): PropertyDefinition { id: PropertyId(2164032953588709760371108256621453393), name: "support", value_type: String, cardinality: One, required: false, constraints: [] }}, inverse: None, symmetric: false, transitive: false }} }

---- a_node_property_definition_filed_under_another_id_is_refused_at_load stdout ----

thread 'a_node_property_definition_filed_under_another_id_is_refused_at_load' (255146) panicked at crates/ekr-ontology/tests/ontology_load.rs:78:49:
called `Result::unwrap_err()` on an `Ok` value: Ontology { version: SchemaVersion { id: SchemaVersionId(2164032953588709760371108242678918682), number: 0, parent: None, created_at: Timestamp(0) }, node_types: {TypeId(2164032953588709760371108215588049067): NodeType { id: TypeId(2164032953588709760371108215588049067), name: "Subject", parents: {}, properties: {PropertyId(2164032953588709760371108222230043741): PropertyDefinition { id: PropertyId(2164032953588709760371108248956381764), name: "p", value_type: String, cardinality: One, required: false, constraints: [] }}, abstract_type: false, lifecycle: None, operations: {} }, TypeId(2164032953588709760371108220367359844): NodeType { id: TypeId(2164032953588709760371108220367359844), name: "Other", parents: {}, properties: {}, abstract_type: false, lifecycle: None, operations: {} }}, edge_types: {} }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    a_node_property_definition_filed_under_another_id_is_refused_at_load
    an_edge_property_definition_filed_under_another_id_is_refused_at_load

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr-ontology --test ontology_load`
```

Only after observing these failures was the common check_properties comparison added.

## 4. Green suite, counts and checks

```sh
CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-08-seed CARGO_BUILD_JOBS=2 TMPDIR=<cache>/ekr-completion-20260922/seed cargo test -p ekr-ontology -p ekr-graph -p ekr-store -p ekr-kernel --no-fail-fast
# before: exit 101, 288 executed (287 passed / 1 failed)
# after: exit 0, 290 executed (290 passed / 0 failed)
```

Per-binary counts from the runner's own summary lines:
```
ekr_graph/unit: executed 0 → 0, final exit 0
ekr_graph/adversary2_guard_bounds_and_ranges.rs: executed 4 → 4, final exit 0
ekr_graph/adversary2_membrane_and_addresses.rs: executed 2 → 2, final exit 0
ekr_graph/adversary_canonical_value_reach.rs: executed 3 → 3, final exit 0
ekr_graph/adversary_p1_06_reference_markers.rs: executed 2 → 2, final exit 0
ekr_graph/adversary_snapshot_and_assertion.rs: executed 4 → 4, final exit 0
ekr_graph/canonical_value_and_assertion.rs: executed 18 → 18, final exit 0
ekr_graph/domain_projection.rs: executed 3 → 3, final exit 0
ekr_graph/evidence_and_observations.rs: executed 6 → 6, final exit 0
ekr_graph/membrane.rs: executed 5 → 5, final exit 0
ekr_graph/node_identity.rs: executed 3 → 3, final exit 0
ekr_graph/review_p1_membrane_is_by_id.rs: executed 2 → 2, final exit 0
ekr_graph/revision_events.rs: executed 5 → 5, final exit 0
ekr_graph/snapshot_reads.rs: executed 7 → 7, final exit 0
ekr_kernel/unit: executed 0 → 0, final exit 0
ekr_kernel/adversary_membrane.rs: executed 5 → 5, final exit 0
ekr_kernel/adversary_membrane_pass_two.rs: executed 4 → 4, final exit 0
ekr_kernel/adversary_p1_07.rs: executed 5 → 5, final exit 0
ekr_kernel/adversary_p1_08_seed.rs: executed 4 → 4, final exit 0; baseline 1 failed
ekr_kernel/commit_path.rs: executed 5 → 5, final exit 0
ekr_kernel/encoding_field_order.rs: executed 1 → 1, final exit 0
ekr_kernel/seed.rs: executed 25 → 25, final exit 0
ekr_kernel/validate_properties.rs: executed 5 → 5, final exit 0
ekr_kernel/validation.rs: executed 49 → 49, final exit 0
ekr_ontology/unit: executed 0 → 0, final exit 0
ekr_ontology/adversary_decoder.rs: executed 2 → 2, final exit 0
ekr_ontology/canonical_admissibility.rs: executed 6 → 6, final exit 0
ekr_ontology/domain_projection.rs: executed 3 → 3, final exit 0
ekr_ontology/hierarchy_specificity.rs: executed 7 → 7, final exit 0
ekr_ontology/inheritance_and_declaration_coherence.rs: executed 13 → 13, final exit 0
ekr_ontology/lifecycle_transitions.rs: executed 8 → 8, final exit 0
ekr_ontology/ontology_load.rs: executed 15 → 17, final exit 0
ekr_ontology/type_hierarchy.rs: executed 4 → 4, final exit 0
ekr_ontology/value_type_checking.rs: executed 8 → 8, final exit 0
ekr_store/unit: executed 0 → 0, final exit 0
ekr_store/adversary2_event_vocabulary.rs: executed 1 → 1, final exit 0
ekr_store/adversary2_retention_event_contract.rs: executed 2 → 2, final exit 0
ekr_store/adversary_objects_and_append.rs: executed 2 → 2, final exit 0
ekr_store/adversary_p1_06_reference_from_bytes.rs: executed 1 → 1, final exit 0
ekr_store/domain_projection.rs: executed 7 → 7, final exit 0
ekr_store/fold_rules.rs: executed 18 → 18, final exit 0
ekr_store/membrane_boundary.rs: executed 1 → 1, final exit 0
ekr_store/providers.rs: executed 15 → 15, final exit 0
ekr_store/review_p1_invariant_one_at_the_store.rs: executed 2 → 2, final exit 0
ekr_store/seed_object_integrity.rs: executed 2 → 2, final exit 0
ekr_graph/doc: executed 1 → 1, final exit 0
ekr_kernel/doc: executed 2 → 2, final exit 0
ekr_ontology/doc: executed 1 → 1, final exit 0
ekr_store/doc: executed 0 → 0, final exit 0
```

The unchanged adversary suite executes 4→4 and flips 3-pass/1-fail to 4-pass. New loader regression
selection executed 2 red before repair; ontology_load's whole lane increases 15→17. No counts
fall and no new tests are ignored or filtered out of the full lane.

Final full runner output:
```
   Compiling ekr-ontology v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-ontology)
   Compiling ekr-graph v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-graph)
   Compiling ekr-store v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-store)
   Compiling ekr-kernel v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.37s
     Running unittests src/lib.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/ekr_graph-4851d02fd854460b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_guard_bounds_and_ranges.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary2_guard_bounds_and_ranges-ade199b341027c49)

running 4 tests
test an_inverted_range_is_refused_or_describes_some_instant ... ok
test the_item_scanner_is_the_same_text_in_both_files ... ok
test the_guard_against_a_returning_open_ended_read_covers_the_whole_crate ... ok
test the_field_guard_catches_a_new_field_whatever_it_is_called ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_membrane_and_addresses.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary2_membrane_and_addresses-eb0fa4f6b69b2bc0)

running 2 tests
test every_entity_canonical_state_holds_has_a_content_address ... ok
test the_seal_of_canonical_dependency_is_carried_only_by_a_canonical_dependency ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_canonical_value_reach.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_canonical_value_reach-f38bc0a0dfec49f2)

running 3 tests
test a_transient_candidate_assertion_may_carry_an_approximate_measurement ... ok
test a_transient_candidate_node_may_hold_an_approximate_measurement ... ok
test every_part_of_graph_state_has_a_canonical_encoding ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_06_reference_markers.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_p1_06_reference_markers-5448aa5e495d6e6b)

running 2 tests
test the_default_reference_of_a_transient_claim_is_the_transient_reference ... ok
test a_reference_to_evidence_resolves_to_a_node_because_the_marker_is_decoration ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_snapshot_and_assertion.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_snapshot_and_assertion-11ab10473e2780ec)

running 4 tests
test a_record_with_no_transaction_time_at_all_is_not_a_current_belief ... ok
test is_current_is_exactly_acceptance_and_an_open_transaction_time ... ok
test the_domain_requires_a_recorded_from_and_the_crate_cannot_omit_one ... ok
test active_is_not_merely_valid_at_the_end_of_representable_time ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/canonical_value_and_assertion.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/canonical_value_and_assertion-4145a44c64ff8db4)

running 18 tests
test a_node_and_an_edge_property_carry_only_an_admissible_value ... ok
test a_float_at_any_depth_cannot_inhabit_a_canonical_value_and_the_refusal_names_it ... ok
test an_admissible_value_round_trips_through_the_newtype ... ok
test every_field_of_a_node_and_an_edge_reaches_the_encoding ... ok
test every_field_of_an_assertion_reaches_the_encoding ... ok
test every_field_of_evidence_state_reaches_the_encoding ... ok
test no_two_observations_that_differ_share_a_content_address ... ok
test no_two_edges_that_differ_share_a_content_address ... ok
test no_two_support_links_that_differ_share_a_content_address ... ok
test no_two_nodes_that_differ_share_a_content_address ... ok
test graph_state_equal_in_every_field_hashes_equally ... ok
test no_two_pieces_of_evidence_that_differ_share_a_content_address ... ok
test every_variant_of_every_sum_type_opens_with_its_own_marker ... ok
test the_conversion_refuses_exactly_where_the_ontology_says_it_must ... ok
test no_two_assertions_that_differ_share_a_content_address ... ok
test two_values_that_differ_do_not_encode_alike ... ok
test two_assertions_equal_in_every_field_hash_equally ... ok
test the_declaration_order_of_every_sum_type_equals_its_numbering ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/domain_projection.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/domain_projection-9784aef31543abe4)

running 3 tests
test every_enumeration_the_domain_declares_is_carried_variant_for_variant ... ok
test the_crate_quotes_the_domains_own_sentence_about_validation_state_payloads ... ok
test every_declaration_of_the_domain_is_carried_field_for_field ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/evidence_and_observations.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/evidence_and_observations-8349d28dea5445a2)

running 6 tests
test a_blob_observation_carries_its_media_type_and_length ... ok
test evidence_carries_its_provenance ... ok
test every_evidence_source_answers_the_flat_fields_the_domain_declares ... ok
test confidence_outside_its_declared_range_is_not_constructible ... ok
test support_links_one_assertion_to_one_piece_of_evidence ... ok
test every_observation_form_answers_its_kind_and_its_content_hash ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/membrane.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/membrane-232536dd185bd11a)

running 5 tests
test a_canonical_reference_is_the_only_canonical_dependency ... ok
test canonical_state_resolves_a_canonical_reference ... ok
test a_candidate_and_a_canonical_node_that_share_an_id_do_not_resolve_alike ... ok
test transient_state_may_depend_on_canonical_state_and_on_its_own ... ok
    Checking ekr-ontology v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-ontology)
    Checking ekr-graph v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-graph)
    Checking ekr-graph-tests v0.0.0 (<cache>/b10x-target/ekr-p1-08-seed/tests/trybuild/ekr-graph)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.87s


test tests/compile_fail/canonical_dependency_is_sealed.rs ... ok
test tests/compile_fail/canonical_graph_rejects_a_transient_ref.rs ... ok
test tests/compile_fail/canonical_ref_cannot_target_a_transient_type.rs ... ok
test tests/compile_fail/transient_state_has_no_content_address.rs ... ok


test the_membrane_is_a_set_of_build_failures ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.92s

     Running tests/node_identity.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/node_identity-cc0577f9dcabf6fc)

running 3 tests
test the_lifecycle_state_of_a_node_is_a_property_and_not_its_identity ... ok
test two_nodes_that_share_a_name_do_not_share_an_id ... ok
test a_node_renamed_a_thousand_times_keeps_its_id ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/review_p1_membrane_is_by_id.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/review_p1_membrane_is_by_id-7b5a918e521e9c90)

running 2 tests
test canonical_state_resolves_the_references_it_holds_and_answers_none_for_one_it_does_not ... ok
    Checking ekr-graph-tests v0.0.0 (<cache>/b10x-target/ekr-p1-08-seed/tests/trybuild/ekr-graph)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s


test tests/review_p1_compile_fail/a_canonical_edge_may_target_a_candidate_node.rs ... ok


test the_sealed_reference_types_are_sound_and_canonical_state_holds_none_of_them ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/revision_events.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/revision_events-8b30a7d9b5976684)

running 5 tests
test an_event_encodes_as_a_function_of_its_value ... ok
test no_two_variants_share_an_encoding ... ok
test the_variant_marker_and_not_the_payload_is_what_separates_two_events ... ok
test every_variant_carries_its_declared_index_and_domain_name ... ok
test the_declaration_order_of_the_variants_equals_their_numbering ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/snapshot_reads.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/snapshot_reads-7c27a9f0cbcff8a8)

running 7 tests
test a_snapshot_names_the_revision_it_reads ... ok
test a_proposed_assertion_is_never_answered ... ok
test a_record_whose_transaction_time_is_closed_is_not_current ... ok
test the_historical_query_returns_alice ... ok
test valid_at_never_returns_a_retracted_or_superseded_assertion ... ok
test the_handover_instant_belongs_to_exactly_one_of_them ... ok
test valid_at_answers_alice_before_the_handover_and_bob_at_or_after_it ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/ekr_kernel-dcebe1934f67cb0b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_membrane.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_membrane-f528e4a1cc1a1079)

running 5 tests
test an_assertion_whose_only_evidence_the_proposer_invented_is_refused ... ok
test a_create_node_over_an_id_canonical_state_already_holds_is_refused ... ok
test an_add_assertion_over_an_id_canonical_state_already_holds_is_refused ... ok
test a_node_created_under_a_graph_root_that_does_not_exist_is_refused ... ok
test reordering_the_operations_of_one_transaction_does_not_change_the_verdict ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_membrane_pass_two.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_membrane_pass_two-7a85a72ebf8dcd25)

running 4 tests
test a_node_is_not_merged_into_itself ... ok
test a_type_the_ontology_already_declares_is_not_declared_again ... ok
test a_proposer_cannot_mark_its_own_assertion_accepted ... ok
test an_assertion_naming_a_type_the_ontology_does_not_declare_is_refused ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_07.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_p1_07-d7562ee04b18eba3)

running 5 tests
test inherited_opaque_constraints_are_refused_on_child_node_properties ... ok
test retracting_a_retained_edge_assertion_does_not_erase_its_reference ... ok
test cancellation_cannot_leave_a_new_assertion_referring_to_the_cancelled_edge ... ok
test independent_property_and_lifecycle_writes_are_order_invariant ... ok
test assertion_subject_predicate_object_cross_product_obeys_declarations ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_08_seed.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_p1_08_seed-b13a50007eca94e0)

running 4 tests
test seed_ontology_property_definitions_must_be_filed_under_their_own_ids ... ok
test inherited_record_properties_remain_typed_and_constraints_cannot_disappear ... ok
test losing_cached_seed_keeps_its_original_retention_class ... ok
test persisted_seed_format_and_context_fields_refuse_before_admission ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.71s

     Running tests/commit_path.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/commit_path-816fb6c18858336f)

running 5 tests
test a_commit_into_an_unseeded_lineage_is_refused ... ok
test the_kernels_authority_stands_behind_exactly_what_it_validated ... ok
test a_validated_transaction_commits_and_the_lineage_advances ... ok
test a_transaction_validated_against_a_revision_the_lineage_moved_past_is_refused ... ok
test the_same_lineage_written_by_hand_does_not_advance_anything ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/encoding_field_order.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/encoding_field_order-2a7ec28608d4a18e)

running 1 test
test every_field_of_the_kernels_encodings_is_written_in_declaration_order ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/seed.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/seed-db0d520306fccdc5)

running 25 tests
test legitimate_record_keys_remain_data_in_strict_seed_decoding ... ok
test seed_decoding_refuses_unknown_semantic_fields_at_every_record_boundary ... ok
test a_seed_with_an_undeclared_type_is_refused_by_both_backends ... ok
test a_seed_ontology_cannot_claim_a_predecessor ... ok
test a_seed_ontology_cannot_claim_a_later_version ... ok
test a_seed_with_a_dangling_edge_is_refused_by_both_backends ... ok
test a_seed_with_a_caller_verdict_is_refused_by_both_backends ... ok
test a_valid_seed_has_a_positive_control ... ok
test empty_bootstrap_does_not_make_empty_transactions_valid ... ok
test repeated_initialization_preserves_the_lineage_and_writes_no_second_object ... ok
test concurrent_independent_handles_publish_exactly_one_seed_and_no_losing_object ... ok
test reopen_checks_full_ontology_and_execution_context ... ok
test initialization_reuses_exact_cached_seed_bytes_atomically ... ok
test required_properties_and_unsupported_constraints_keep_their_refusals ... ok
test the_versioned_minimal_yaml_fixture_initializes_both_real_providers ... ok
test seed_multiplicity_and_property_types_are_checked ... ok
test actual_bootstrap_identities_refuse_self_validation_and_false_attribution ... ok
test named_seed_refusals_write_neither_the_object_nor_the_revision ... ok
test an_evidence_seed_reopens_with_identical_roots_fields_and_retained_bytes ... ok
test seed_lifecycle_is_the_declared_initial_state_and_is_preserved ... ok
test floats_are_refused_in_seed_nodes_edges_and_assertion_objects ... ok
test canonical_seed_root_filing_schema_and_genesis_are_mandatory ... ok
test seed_support_checks_missing_uncited_and_unsupported_evidence ... ok
test legacy_and_tampered_seed_envelopes_are_preserved_but_never_admitted ... ok
test every_seed_entity_map_checks_key_and_root_identity ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.34s

     Running tests/validate_properties.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/validate_properties-bebd2f6d483597a0)

running 5 tests
test the_two_walks_of_one_rule_agree ... ok
test a_float_anywhere_is_always_refused ... ok
test no_transaction_is_ever_validated_by_its_own_proposer ... ok
test validating_twice_gives_the_same_answer ... ok
test the_encoding_separates_transactions_that_differ ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/validation.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/validation-09bc7f3eafde1f2a)

running 49 tests
test a_canonical_assertion_without_evidence_is_refused_by_the_provenance_validator ... ok
test a_reference_to_something_the_same_transaction_creates_resolves ... ok
test a_merge_names_two_nodes ... ok
test a_value_canonical_state_does_not_admit_is_refused_with_its_path ... ok
test a_transaction_with_no_operations_is_refused_by_the_structural_validator ... ok
test a_node_of_an_abstract_type_is_refused ... ok
test a_value_that_fails_its_type_is_refused_by_the_type_validator ... ok
test an_edge_cardinality_the_type_forbids_is_refused_by_the_cardinality_validator ... ok
test a_record_under_another_graph_root_is_refused ... ok
test a_transaction_whose_validator_is_its_proposer_is_refused ... ok
test an_edge_endpoint_of_the_wrong_type_is_refused ... ok
test an_empty_property_assignment_still_requires_a_declared_property ... ok
test an_evidence_id_the_transaction_declares_does_not_make_it_exist ... ok
test a_valid_transaction_validates_to_the_same_hash_twice ... ok
test a_type_the_ontology_already_declares_is_not_declared_again ... ok
test a_valid_transaction_validates ... ok
test an_invoke_the_lifecycle_does_not_declare_is_refused_by_the_ontology_validator ... ok
test an_operation_the_type_does_not_declare_is_refused ... ok
test an_unresolvable_reference_is_refused_by_the_reference_validator ... ok
test an_identity_created_twice_in_one_transaction_is_refused ... ok
test competing_lifecycle_writes_are_refused ... ok
test an_invocation_carries_exactly_the_arguments_its_operation_declares ... ok
test an_unresolved_assertion_endpoint_does_not_acquire_an_unrelated_type_refusal ... ok
test deleting_an_edge_referenced_by_a_new_or_retained_assertion_is_refused ... ok
test opaque_edge_property_constraints_refuse_creation_and_assertions ... ok
test an_identity_canonical_state_already_holds_is_not_created_again ... ok
test an_assertions_types_are_resolved_whatever_shape_its_object_has ... ok
test an_assertion_cannot_arrive_carrying_its_own_verdict ... ok
test every_type_refusal_the_validator_can_make_is_reachable ... ok
test opaque_preconditions_and_emissions_are_not_silently_accepted ... ok
test property_cardinality_and_required_presence_are_refused ... ok
test relation_assertions_check_both_endpoint_types_and_allow_inherited_types ... ok
test property_assertions_check_type_objects_edge_properties_and_type_subjects ... ok
test every_field_of_every_encoded_type_reaches_its_encoding ... ok
test competing_property_writes_are_refused_in_every_order ... ok
test applicable_opaque_property_constraints_refuse_all_node_write_paths ... ok
test property_cardinality_uses_the_candidate_node ... ok
test the_encoding_writes_id_bearing_fields_in_declaration_order ... ok
test the_pipeline_runs_the_seven_deterministic_validators_in_order ... ok
test the_declared_evidence_set_is_the_evidence_the_assertions_cite ... ok
test the_eleven_operation_numbers_are_the_domains_and_the_declarations ... ok
test a_serialized_lifecycle_invokes_but_unsupported_sets_refuse_at_decode ... ok
test every_kind_of_dangling_reference_is_refused ... ok
test unsupported_schema_changes_and_merges_refuse_explicitly ... ok
test the_validation_hash_covers_the_revision_it_was_validated_against ... ok
test two_transactions_that_differ_validate_to_different_hashes ... ok
test relation_assertions_require_compatible_node_endpoints ... ok
test every_issue_code_the_kernel_can_raise_is_raised_by_a_case ... ok
    Checking ekr-ontology v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-ontology)
    Checking ekr-graph v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-graph)
    Checking ekr-store v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-store)
    Checking ekr-kernel v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-kernel)
    Checking ekr-kernel-tests v0.0.0 (<cache>/b10x-target/ekr-p1-08-seed/tests/trybuild/ekr-kernel)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.49s


test tests/compile_fail/validated_transaction_cannot_be_deserialised.rs ... ok
test tests/compile_fail/validated_transaction_has_no_constructor_outside_the_kernel.rs ... ok


test only_the_kernel_constructs_a_validated_transaction ... ok

test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.15s

     Running unittests src/lib.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/ekr_ontology-a33d8483ba66af11)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_decoder.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_decoder-3f0e5810a44a9e85)

running 2 tests
test unknown_value_members_are_refused_beneath_a_record_key_and_list_element ... ok
test record_keys_named_like_semantic_members_survive_nested_roundtrips ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/canonical_admissibility.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/canonical_admissibility-26be980bc2e08ca8)

running 6 tests
test a_bare_float_is_refused_and_the_path_is_the_value_itself ... ok
test a_float_inside_a_compound_is_refused_at_the_path_it_sits_at ... ok
test every_kind_but_a_float_is_admissible_on_its_own ... ok
test a_path_reads_from_the_whole_value_down_to_the_part ... ok
test nesting_does_not_hide_a_float_at_any_depth ... ok
test the_first_refused_value_in_the_values_own_order_is_the_one_named ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/domain_projection.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/domain_projection-dd4be6766e122248)

running 3 tests
test a_timestamp_value_carries_a_timestamp ... ok
test the_domain_carries_node_ref_and_enum_parameters_and_no_other_compound_kind ... ok
test every_timestamp_the_domain_declares_is_carried_as_a_timestamp ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/hierarchy_specificity.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/hierarchy_specificity-cf37b6959f776402)

running 7 tests
test a_hierarchy_fault_is_reported_before_a_property_resolution_fault ... ok
test the_ambiguity_refusal_only_fires_when_neither_declaring_type_is_an_ancestor_of_the_other ... ok
test the_checker_resolves_allowed_types_by_the_same_order ... ok
test the_reason_a_two_fault_document_is_refused_does_not_depend_on_id_order ... ok
test a_parent_that_specialises_another_parent_is_not_an_ambiguous_declaration ... ok
test a_redeclaration_is_never_overridden_by_the_declaration_it_redeclares ... ok
test a_declaration_is_chosen_by_the_specialisation_order_and_by_nothing_else ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/inheritance_and_declaration_coherence.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/inheritance_and_declaration_coherence-15a311c7a8eecd5f)

running 13 tests
test an_operation_argument_no_value_inhabits_is_refused_at_load ... ok
test an_operation_argument_naming_an_undeclared_type_is_refused_at_load ... ok
test an_undeclared_grandparent_is_refused_as_an_unknown_type_and_not_as_a_cycle ... ok
test a_redeclared_property_is_resolved_by_the_hierarchy_and_not_by_type_id_order ... ok
test a_single_fault_hierarchy_is_refused_for_that_fault_whatever_the_id_order ... ok
test two_ancestors_declaring_one_property_identically_are_not_ambiguous ... ok
test every_position_a_value_type_is_declared_in_is_walked_for_inhabitability ... ok
test the_nearest_declaration_wins_at_every_depth ... ok
test two_ancestors_at_one_distance_declaring_a_property_differently_are_refused_at_load ... ok
test property_resolution_is_a_function_of_the_hierarchy_and_never_of_id_order ... ok
test the_crate_declares_no_value_type_field_the_walk_does_not_reach ... ok
test every_domain_name_this_crate_cites_is_declared_by_the_domain ... ok
test value_rs_does_not_attribute_its_serde_shape_to_the_ess_domain ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/lifecycle_transitions.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/lifecycle_transitions-8de4d97cb81233f0)

running 8 tests
test a_lifecycle_names_its_initial_state_among_its_states ... ok
test a_move_from_a_state_the_lifecycle_does_not_have_is_refused ... ok
test an_operation_that_declares_no_transition_leaves_the_state_alone ... ok
test a_move_from_a_state_the_node_is_not_in_is_refused ... ok
test declares_answers_for_the_pair_and_not_for_its_endpoints ... ok
test every_declared_transition_is_accepted ... ok
test preconditions_are_carried_as_opaque_text_and_refuse_nothing ... ok
test a_move_is_accepted_exactly_when_the_lifecycle_declares_it ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ontology_load.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/ontology_load-f6488c4d7e47bbcc)

running 17 tests
test a_cycle_in_the_parent_graph_is_refused_at_load ... ok
test a_node_ref_to_a_type_the_ontology_does_not_declare_is_refused_at_load ... ok
test a_lifecycle_naming_a_state_it_does_not_have_is_refused_at_load ... ok
test a_list_and_an_empty_record_are_not_in_that_class ... ok
test a_type_declared_twice_is_refused_at_load ... ok
test a_node_ref_with_no_allowed_types_is_refused_at_load ... ok
test a_parent_the_ontology_does_not_declare_is_refused_at_load ... ok
test an_edge_type_with_no_source_or_no_target_types_is_refused_at_load ... ok
test an_empty_compound_value_type_is_refused_at_every_depth_it_is_declared ... ok
test an_operation_with_a_transition_and_no_lifecycle_at_all_is_refused_at_load ... ok
test an_operation_whose_move_the_lifecycle_does_not_declare_is_refused_at_load ... ok
test value_envelopes_do_not_discard_unknown_semantics ... ok
test a_node_property_definition_filed_under_another_id_is_refused_at_load ... ok
test an_ontology_loads_from_yaml_and_refuses_the_same_documents ... ok
test an_edge_property_definition_filed_under_another_id_is_refused_at_load ... ok
test unknown_semantics_on_compound_value_types_are_refused ... ok
test unknown_semantic_members_of_ontology_records_are_refused ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/type_hierarchy.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/type_hierarchy-6777223b6cfef7ba)

running 4 tests
test an_inherited_required_property_is_required_of_the_descendant ... ok
test a_type_conforms_to_itself_and_to_every_ancestor_and_to_nothing_else ... ok
test a_types_properties_are_its_own_and_its_ancestors ... ok
test a_node_ref_accepts_a_node_whose_type_conforms_to_an_allowed_type ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/value_type_checking.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/value_type_checking-df5650e8555792e0)

running 8 tests
test a_value_is_checkable_against_a_type_on_its_own ... ok
test a_refusal_names_the_property_and_the_reason ... ok
test an_abstract_or_unknown_type_is_not_instantiable ... ok
test a_well_typed_value_checks_ok ... ok
test cardinality_permits_the_counts_it_names ... ok
test every_value_kind_mirrors_its_value_type ... ok
test enum_variants_and_record_fields_are_enforced_inside_a_compound_value ... ok
test a_value_breaking_exactly_one_property_of_its_declared_type_is_refused ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running unittests src/lib.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/ekr_store-faf645b78e873b91)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_event_vocabulary.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary2_event_vocabulary-5c08c3becd8c2526)

running 1 test
test a_second_rejection_of_a_re_proposed_transaction_is_not_swallowed_as_a_retry ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/adversary2_retention_event_contract.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary2_retention_event_contract-a82a827ca939b61d)

running 2 tests
test the_store_domain_declares_the_retention_raise_event_this_crate_writes ... ok
test the_retention_raise_in_stored_bytes_carries_the_fields_the_domain_declares ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/adversary_objects_and_append.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_objects_and_append-4988f0c4b1471b34)

running 2 tests
test retrying_an_append_writes_the_same_event_once_rather_than_twice ... ok
test storing_canonical_bytes_that_were_cached_earlier_records_them_as_canonical ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/adversary_p1_06_reference_from_bytes.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_p1_06_reference_from_bytes-720826f958dc140b)

running 1 test
test a_store_cannot_turn_dangling_document_bytes_into_canonical_state ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/domain_projection.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/domain_projection-26a37c3aca18a7fa)

running 7 tests
test every_storage_class_has_its_own_retention_rank ... ok
test the_strongest_of_two_classes_does_not_depend_on_which_arrived_first ... ok
test the_retention_ladder_is_the_one_section_thirty_seven_describes ... ok
test the_derived_ordering_is_not_the_retention_ordering ... ok
test the_storage_classes_are_the_ones_the_domain_declares ... ok
test every_event_the_crate_writes_is_declared_by_the_domain ... ok
test every_event_the_crate_writes_carries_the_fields_the_domain_declares ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/fold_rules.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/fold_rules-f25e2bdb78fb33f7)

running 18 tests
test evidence_is_addressed_apart_from_knowledge ... ok
test the_knowledge_root_is_a_function_of_the_graph_state ... ok
test a_lineage_that_does_not_begin_at_a_seed_is_refused ... ok
test a_seed_naming_bytes_the_store_does_not_hold_is_refused ... ok
test an_empty_log_has_no_head_and_does_not_fold ... ok
test validating_a_transaction_that_was_never_proposed_is_refused ... ok
test a_store_with_no_commit_authority_refuses_to_say_what_canonical_state_is ... ok
test a_commit_of_a_transaction_that_was_never_validated_is_refused ... ok
test a_commit_whose_validation_the_authority_stands_behind_advances_the_lineage ... ok
test two_of_the_five_sub_roots_are_the_placeholder_and_not_derived ... ok
test a_commit_publishing_a_knowledge_root_the_fold_does_not_reach_is_refused ... ok
test a_revision_that_does_not_follow_its_parent_is_refused ... ok
test a_second_seed_is_refused ... ok
test a_rejected_transaction_cannot_then_commit ... ok
test a_transaction_that_went_stale_cannot_then_commit ... ok
test a_commit_whose_validation_the_authority_does_not_stand_behind_does_not_advance_the_lineage ... ok
test a_commit_validated_against_a_revision_the_lineage_has_moved_past_does_not_advance_it ... ok
test two_stores_seeded_from_different_state_have_different_heads ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s

     Running tests/membrane_boundary.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/membrane_boundary-4fd8b8cc0e26267e)

running 1 test
test a_candidate_node_and_a_canonical_node_write_the_same_document ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/providers.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/providers-92620baa01a15f37)

running 15 tests
test a_file_store_stores_identical_bytes_once ... ok
test a_cache_write_after_a_canonical_one_leaves_the_bytes_canonical ... ok
test storing_a_graph_is_storing_its_document ... ok
test sqlite_stores_identical_bytes_once ... ok
test a_stored_object_carries_the_class_it_was_written_under ... ok
test a_raised_retention_class_survives_a_reopen ... ok
test a_replay_from_a_revision_with_no_materialised_state_is_refused ... ok
test the_fold_carries_the_seed_the_log_named ... ok
test a_sqlite_store_reopened_folds_to_the_same_head_root ... ok
test every_event_shape_reports_a_write_once_and_a_recognition_after ... ok
test the_recorded_class_is_the_strongest_requested_whichever_order_they_arrive_in ... ok
test two_different_events_appended_in_a_row_both_land ... ok
test sqlite_replay_from_the_seed_equals_the_fold ... ok
test a_file_store_reopened_folds_to_the_same_head_root ... ok
test a_file_store_replay_from_the_seed_equals_the_fold ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s

     Running tests/review_p1_invariant_one_at_the_store.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/review_p1_invariant_one_at_the_store-82651e84b28006c4)

running 2 tests
test a_commit_lands_with_no_validated_transaction_anywhere_in_the_process ... ok
test a_commit_validated_against_a_revision_that_is_no_longer_the_head_replays_as_valid ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/seed_object_integrity.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/seed_object_integrity-d7384f3e5473b860)

running 2 tests
test sqlite_seed_objects_verify_address_length_and_retention_before_admission ... ok
test file_seed_objects_verify_address_length_and_retention_before_admission ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.70s

   Doc-tests ekr_graph

running 1 test
test crates/ekr-graph/src/lib.rs - (line 57) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

   Doc-tests ekr_kernel

running 2 tests
test crates/ekr-kernel/src/commit.rs - commit::Commit<S>::over (line 180) ... ok
test crates/ekr-kernel/src/lib.rs - (line 38) ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

   Doc-tests ekr_ontology

running 1 test
test crates/ekr-ontology/src/lib.rs - (line 21) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

   Doc-tests ekr_store

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```sh
CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-08-seed CARGO_BUILD_JOBS=2 TMPDIR=<cache>/ekr-completion-20260922/seed cargo clippy -p ekr-ontology -p ekr-graph -p ekr-store -p ekr-kernel --all-targets -- -D warnings
# exit 0
```
```
    Checking ekr-ontology v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-ontology)
    Checking ekr-graph v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-graph)
    Checking ekr-store v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-store)
    Checking ekr-kernel v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-kernel)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.05s
```

```sh
cargo fmt --all --check
# exit 0, empty output
git diff --check
# exit 0, empty output
```

No full task check or independent re-review is claimed by this correction.

## 5. Exclusions and clean handback

No AEP/ESS/AGENTS or other normative-document edits, blob migration, commits, publication or cleanup.
Historical origin remains the adversary's undecided finding; no base-commit origin probe was run.
All compiler/test processes completed. Managed lease codex-ekr-p1-08-seed-correction was renewed
and released at handback. “Clean handback” means no active writer/process/lease and no unreported
correction changes; the assigned tree deliberately retains the complete uncommitted unit for review.

## 6. Outside writes

- <cache>/b10x-target/ekr-p1-08-seed/
- <cache>/ekr-completion-20260922/seed/correction-baseline.log
- <cache>/ekr-completion-20260922/seed/correction-red.log
- <cache>/ekr-completion-20260922/seed/correction-green.log
- <cache>/ekr-completion-20260922/seed/correction-clippy.log
- <cache>/ekr-completion-20260922/seed/correction-fmt.log
- <cache>/ekr-completion-20260922/seed/correction-report.md

TMPDIR remains <cache>/ekr-completion-20260922/seed; temporary provider data is cleaned by each test's TempDir. Managed lease
metadata changed only through worktree CLI session hooks. No other process/cache/tmp content was
touched.
