unit: story:version-persisted-contracts — P1-09 original-format freeze only
verdict: green (bounded preparation; combined format story remains active)
cases: executed 221→238, red 4 targeted/mutation cases
origin: n/a
wrote-outside-worktree: assigned scratch, dedicated Cargo target, own managed worktree lease; private inventory retained separately
needs-coordinator: yes — cite realized guard/diagnostic paths and remeasure source-macro counts; no source patch remains outstanding

The bounded acceptance is implemented: original graph/value/assertion/ontology/transaction/event/seed representations and encoders are independent of the current graph, ontology and transaction codecs, with fixed original bytes and addresses. The only shared encoding dependencies are ekr-core scalar identities, timestamps, hashes and canonical primitives; the immutable vectors pin their observed behavior as well. Current production codecs and publication behavior are unchanged.

Source provenance is 73ab8b0a5aa5c670bacbc4c1abdca02f87b62bf0. Before capture, the relevant current core, graph, ontology, transaction, seed, snapshot and fold source was compared against that revision and had no differences. The tree remains uncommitted at opening 151be425fafb9013368d1a74d2d3d1f1019834ce. No provider was opened by implementation, fixture capture or these new verification cases. Existing package tests continue to exercise their disposable providers.

There are 52 fixed vectors plus the original validation construction: all ten hashable value kinds; scalar Node/Edge including List, empty List and Record; all seven combined assertion states, subject/object/time alternatives and actual bootstrap acceptance; six evidence sources; six revision facts; eleven operations including AddAssertion and complete transitive ontology fields; a full ordered transaction; unversioned GraphDocument; Seed/1; SeedEnvelope/1; and the original seven-field revision root. Each JSON string preserves the exact original serializer bytes. Canonical hex/value hashes exist only for types the original source encoded canonically; no whole-document canonical encoding was invented for graph/seed JSON.

The one-shot Rust capture invoked the original kernel seed validate and replay functions directly, without a provider. Its proposed input assertion and resulting Accepted assertion/knowledge/evidence roots are all retained. The original validation hash was taken from the original ValidatedTransaction seal and checked against transaction bytes followed by revision, in the payload domain. Subsequent capture additions checked every previously recorded vector remained identical. The capture program is scratch evidence, not an installed regeneration test; final tests only consume fixed constants.

The original operation vector encoding is ordered even though application semantics treat transactions atomically as sets. The frozen encoder preserves that divergence. Retracted/Superseded original states retain only what the old bytes held; missing prior acceptance is never invented. Frozen references are bare identities and no CanonicalGraph, CanonicalRef, validated transaction, authority or writer capability is constructed by the verifier.

Supplied-byte APIs check payload addresses before decoding, separately verify value addresses, reproduce historical validation addresses against supplied revisions, and reproduce bootstrap attribution on a plain document. Named refusals cover duplicate JSON keys (including user Record keys), duplicate set members/type identities, unknown fields/formats, misfiled identities, changed addresses and missing seed payload evidence (affected EvidenceId plus expected address). Arbitrary unique Record keys named event, format and operation survive unchanged. These are address/shape checks, not a rerun of deterministic admission or proof of a complete export, authenticated identity or valid history.

Realized scope:

- New frozen modules: crates/ekr-graph/src/legacy.rs, crates/ekr-store/src/legacy.rs, crates/ekr-kernel/src/legacy.rs; additive exports in their three lib.rs files.
- New fixed-vector/refusal tests: crates/ekr-kernel/tests/legacy.rs (13 cases), crates/ekr-store/tests/legacy.rs (4 cases).
- Fixed evidence: crates/ekr-store/tests/fixtures/legacy/vectors.json and README.md.
- Existing guard extended: crates/ekr-graph/tests/canonical_value_and_assertion.rs. It scans and tests both current and legacy families, with distinct fixture rosters and unchanged payload-distinguishability checks. Its 18 case functions remain 18; six additional frozen sum-type fixture families run inside them. Both touched source-read sites now resolve CARGO_MANIFEST_DIR at runtime. Coordinator macro-count prose must be remeasured, not inferred from an old count.
- Existing compile-fail snapshots refreshed: canonical_ref_cannot_target_a_transient_type.stderr and transient_state_has_no_content_address.stderr under crates/ekr-graph/tests/compile_fail. The exact E0277 refusals and input cases remain unchanged; new legacy names only qualify/reorder rustc's type-help lists. All four graph compile-fail inputs remain active.

No fixture/type scope inference needed a provider adapter, new dependency, ESS change or current ontology mutation. All new runtime data representations are historical plain data. The scope expansion into the existing guard was measured by its red 13-versus-7 type roster and agreed by the coordinator. Diagnostic refresh was reviewed against unchanged compiler refusal clauses, not accepted as a blanket overwrite.

Actual tracked diff stat (new files are listed separately below):

```text
 crates/ekr-graph/src/lib.rs                        |   2 +
 .../tests/canonical_value_and_assertion.rs         | 275 ++++++++++++++++++++-
 ...nical_ref_cannot_target_a_transient_type.stderr |  18 +-
 .../transient_state_has_no_content_address.stderr  |   8 +-
 crates/ekr-kernel/src/lib.rs                       |   2 +
 crates/ekr-store/src/lib.rs                        |   2 +
 6 files changed, 287 insertions(+), 20 deletions(-)
```

Actual working-tree inventory:

```text
 M crates/ekr-graph/src/lib.rs
 M crates/ekr-graph/tests/canonical_value_and_assertion.rs
 M crates/ekr-graph/tests/compile_fail/canonical_ref_cannot_target_a_transient_type.stderr
 M crates/ekr-graph/tests/compile_fail/transient_state_has_no_content_address.stderr
 M crates/ekr-kernel/src/lib.rs
 M crates/ekr-store/src/lib.rs
?? crates/ekr-graph/src/legacy.rs
?? crates/ekr-kernel/src/legacy.rs
?? crates/ekr-kernel/tests/legacy.rs
?? crates/ekr-store/src/legacy.rs
?? crates/ekr-store/tests/fixtures/
?? crates/ekr-store/tests/legacy.rs
```

Actual untracked-file stats:

```text
 /dev/null => crates/ekr-graph/src/legacy.rs | 1292 +++++++++++++++++++++++++++
 1 file changed, 1292 insertions(+)
 /dev/null => crates/ekr-kernel/src/legacy.rs | 798 +++++++++++++++++++++++++++
 1 file changed, 798 insertions(+)
 /dev/null => crates/ekr-kernel/tests/legacy.rs | 364 +++++++++++++++++++++++++
 1 file changed, 364 insertions(+)
 /dev/null => crates/ekr-store/src/legacy.rs | 232 ++++++++++++++++++++++++++++
 1 file changed, 232 insertions(+)
 .../ekr-store/tests/fixtures/legacy/README.md      | 38 ++++++++++++++++++++++
 1 file changed, 38 insertions(+)
 .../ekr-store/tests/fixtures/legacy/vectors.json   | 423 +++++++++++++++++++++
 1 file changed, 423 insertions(+)
 /dev/null => crates/ekr-store/tests/legacy.rs | 79 +++++++++++++++++++++++++++
 1 file changed, 79 insertions(+)
```

Red-first and mutation evidence (each targeted command exited 101):

```text
$ cargo test -p ekr-store --test legacy
   Compiling libc v0.2.189
   Compiling getrandom v0.4.3
   Compiling uuid v1.26.1
   Compiling tempfile v3.27.0
   Compiling eventlog-core v0.2.1 (https://github.com/beyond10x/eventlog?tag=0.2.1#77cda080)
   Compiling ekr-core v0.0.0 (<worktree>/crates/ekr-core)
   Compiling ekr-ontology v0.0.0 (<worktree>/crates/ekr-ontology)
   Compiling eventlog-file v0.2.1 (https://github.com/beyond10x/eventlog?tag=0.2.1#77cda080)
   Compiling ekr-graph v0.0.0 (<worktree>/crates/ekr-graph)
   Compiling eventlog-sqlite v0.2.1 (https://github.com/beyond10x/eventlog?tag=0.2.1#77cda080)
   Compiling ekr-store v0.0.0 (<worktree>/crates/ekr-store)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.58s
     Running tests/legacy.rs (<dedicated-target>/debug/deps/legacy-28046b461dbd8f6f)

running 1 test
test duplicate_keys_are_named_refusals_even_inside_user_records ... FAILED

failures:

---- duplicate_keys_are_named_refusals_even_inside_user_records stdout ----

thread 'duplicate_keys_are_named_refusals_even_inside_user_records' (387947) panicked at crates/ekr-store/tests/legacy.rs:7:5:
verification silently discarded a duplicate key
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    duplicate_keys_are_named_refusals_even_inside_user_records

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr-store --test legacy`

exit: 101
```

```text
$ cargo test -p ekr-kernel --test legacy frozen_contracts_refuse_unknown_fields_and_duplicate_set_members
   Compiling ekr-kernel v0.0.0 (<worktree>/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.41s
     Running tests/legacy.rs (<dedicated-target>/debug/deps/legacy-1ef43957b620c981)

running 1 test
test frozen_contracts_refuse_unknown_fields_and_duplicate_set_members ... FAILED

failures:

---- frozen_contracts_refuse_unknown_fields_and_duplicate_set_members stdout ----

thread 'frozen_contracts_refuse_unknown_fields_and_duplicate_set_members' (496208) panicked at crates/ekr-kernel/tests/legacy.rs:274:5:
assertion failed: serde_json::from_slice::<kernel::GraphOperation>(repeated_arguments).is_err()
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    frozen_contracts_refuse_unknown_fields_and_duplicate_set_members

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr-kernel --test legacy`

exit: 101
```

```text
$ cargo test -p ekr-kernel --test legacy all_original_operations_include_transitive_ontology_and_assertion_bytes
   Compiling ekr-kernel v0.0.0 (<worktree>/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.31s
     Running tests/legacy.rs (<dedicated-target>/debug/deps/legacy-1ef43957b620c981)

running 1 test
test all_original_operations_include_transitive_ontology_and_assertion_bytes ... FAILED

failures:

---- all_original_operations_include_transitive_ontology_and_assertion_bytes stdout ----

thread 'all_original_operations_include_transitive_ontology_and_assertion_bytes' (512045) panicked at crates/ekr-kernel/tests/legacy.rs:48:5:
assertion `left == right` failed: operation-4 bytes
  left: "0f0000000401"
 right: "0f000000040d0000000000004000800000000000000d0d000000000000400080000000000000020f000000000d0000000000004000800000000000000a0f000000010d000000000000400080000000000000040f000000010d0000000000004000800000000000000a0900000000000000010d0000000000004000800000000000000c0d000000000000400080000000000000050f000000000b0b05000000000000000000000000000000000b"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    all_original_operations_include_transitive_ontology_and_assertion_bytes

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr-kernel --test legacy`

exit: 101
```

```text
$ cargo test -p ekr-kernel --test legacy original_transaction_validation_uses_payload_domain_and_ordered_operations
   Compiling ekr-kernel v0.0.0 (<worktree>/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.91s
     Running tests/legacy.rs (<dedicated-target>/debug/deps/legacy-1ef43957b620c981)

running 1 test
test original_transaction_validation_uses_payload_domain_and_ordered_operations ... FAILED

failures:

---- original_transaction_validation_uses_payload_domain_and_ordered_operations stdout ----

thread 'original_transaction_validation_uses_payload_domain_and_ordered_operations' (513661) panicked at crates/ekr-kernel/tests/legacy.rs:145:5:
assertion `left == right` failed
  left: ContentHash([179, 255, 203, 151, 152, 15, 123, 183, 86, 87, 216, 0, 1, 116, 21, 155, 2, 66, 63, 143, 103, 195, 99, 27, 211, 70, 80, 112, 22, 187, 209, 191])
 right: ContentHash([116, 22, 5, 5, 12, 154, 168, 48, 126, 70, 130, 122, 89, 249, 62, 21, 247, 157, 223, 140, 129, 172, 250, 65, 255, 28, 205, 200, 191, 130, 27, 244])
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    original_transaction_validation_uses_payload_domain_and_ordered_operations

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p ekr-kernel --test legacy`

exit: 101
```

The first two are executable refusals written before their fixes. The latter two intentionally removed AddAssertion's payload or changed the validation hash construction; both mutations were restored before final verification. Intermediate full runs also caught the newly expanded source roster and rustc diagnostic qualification changes; their full logs are retained in the private evidence inventory. No test input, refusal assertion or existing case was removed.

Final whole touched-package run; exit 0:

```text
$ CARGO_TARGET_DIR=<dedicated-target> CARGO_BUILD_JOBS=2 TMPDIR=<assigned-scratch> cargo test -p ekr-graph -p ekr-store -p ekr-kernel
   Compiling ekr-store v0.0.0 (<worktree>/crates/ekr-store)
   Compiling ekr-kernel v0.0.0 (<worktree>/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.99s
     Running unittests src/lib.rs (<dedicated-target>/debug/deps/ekr_graph-4851d02fd854460b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_guard_bounds_and_ranges.rs (<dedicated-target>/debug/deps/adversary2_guard_bounds_and_ranges-ade199b341027c49)

running 4 tests
test an_inverted_range_is_refused_or_describes_some_instant ... ok
test the_item_scanner_is_the_same_text_in_both_files ... ok
test the_guard_against_a_returning_open_ended_read_covers_the_whole_crate ... ok
test the_field_guard_catches_a_new_field_whatever_it_is_called ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_membrane_and_addresses.rs (<dedicated-target>/debug/deps/adversary2_membrane_and_addresses-eb0fa4f6b69b2bc0)

running 2 tests
test every_entity_canonical_state_holds_has_a_content_address ... ok
test the_seal_of_canonical_dependency_is_carried_only_by_a_canonical_dependency ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_canonical_value_reach.rs (<dedicated-target>/debug/deps/adversary_canonical_value_reach-f38bc0a0dfec49f2)

running 3 tests
test a_transient_candidate_assertion_may_carry_an_approximate_measurement ... ok
test a_transient_candidate_node_may_hold_an_approximate_measurement ... ok
test every_part_of_graph_state_has_a_canonical_encoding ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_06_reference_markers.rs (<dedicated-target>/debug/deps/adversary_p1_06_reference_markers-5448aa5e495d6e6b)

running 2 tests
test the_default_reference_of_a_transient_claim_is_the_transient_reference ... ok
test a_reference_to_evidence_resolves_to_a_node_because_the_marker_is_decoration ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_snapshot_and_assertion.rs (<dedicated-target>/debug/deps/adversary_snapshot_and_assertion-11ab10473e2780ec)

running 4 tests
test the_domain_requires_a_recorded_from_and_the_crate_cannot_omit_one ... ok
test a_record_with_no_transaction_time_at_all_is_not_a_current_belief ... ok
test is_current_is_exactly_acceptance_and_an_open_transaction_time ... ok
test active_is_not_merely_valid_at_the_end_of_representable_time ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/canonical_value_and_assertion.rs (<dedicated-target>/debug/deps/canonical_value_and_assertion-4145a44c64ff8db4)

running 18 tests
test a_node_and_an_edge_property_carry_only_an_admissible_value ... ok
test a_float_at_any_depth_cannot_inhabit_a_canonical_value_and_the_refusal_names_it ... ok
test no_two_nodes_that_differ_share_a_content_address ... ok
test no_two_assertions_that_differ_share_a_content_address ... ok
test every_field_of_a_node_and_an_edge_reaches_the_encoding ... ok
test no_two_pieces_of_evidence_that_differ_share_a_content_address ... ok
test no_two_observations_that_differ_share_a_content_address ... ok
test the_conversion_refuses_exactly_where_the_ontology_says_it_must ... ok
test graph_state_equal_in_every_field_hashes_equally ... ok
test an_admissible_value_round_trips_through_the_newtype ... ok
test no_two_support_links_that_differ_share_a_content_address ... ok
test two_values_that_differ_do_not_encode_alike ... ok
test every_field_of_an_assertion_reaches_the_encoding ... ok
test two_assertions_equal_in_every_field_hash_equally ... ok
test no_two_edges_that_differ_share_a_content_address ... ok
test every_field_of_evidence_state_reaches_the_encoding ... ok
test every_variant_of_every_sum_type_opens_with_its_own_marker ... ok
test the_declaration_order_of_every_sum_type_equals_its_numbering ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/domain_projection.rs (<dedicated-target>/debug/deps/domain_projection-9784aef31543abe4)

running 3 tests
test every_enumeration_the_domain_declares_is_carried_variant_for_variant ... ok
test the_crate_quotes_the_domains_own_sentence_about_validation_state_payloads ... ok
test every_declaration_of_the_domain_is_carried_field_for_field ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/evidence_and_observations.rs (<dedicated-target>/debug/deps/evidence_and_observations-8349d28dea5445a2)

running 6 tests
test a_blob_observation_carries_its_media_type_and_length ... ok
test confidence_outside_its_declared_range_is_not_constructible ... ok
test every_evidence_source_answers_the_flat_fields_the_domain_declares ... ok
test every_observation_form_answers_its_kind_and_its_content_hash ... ok
test evidence_carries_its_provenance ... ok
test support_links_one_assertion_to_one_piece_of_evidence ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/membrane.rs (<dedicated-target>/debug/deps/membrane-232536dd185bd11a)

running 5 tests
test a_candidate_and_a_canonical_node_that_share_an_id_do_not_resolve_alike ... ok
test a_canonical_reference_is_the_only_canonical_dependency ... ok
test canonical_state_resolves_a_canonical_reference ... ok
test transient_state_may_depend_on_canonical_state_and_on_its_own ... ok
    Checking ekr-graph-tests v0.0.0 (<dedicated-target>/tests/trybuild/ekr-graph)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s


test tests/compile_fail/canonical_dependency_is_sealed.rs ... ok
test tests/compile_fail/canonical_graph_rejects_a_transient_ref.rs ... ok
test tests/compile_fail/canonical_ref_cannot_target_a_transient_type.rs ... ok
test tests/compile_fail/transient_state_has_no_content_address.rs ... ok


test the_membrane_is_a_set_of_build_failures ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s

     Running tests/node_identity.rs (<dedicated-target>/debug/deps/node_identity-cc0577f9dcabf6fc)

running 3 tests
test the_lifecycle_state_of_a_node_is_a_property_and_not_its_identity ... ok
test two_nodes_that_share_a_name_do_not_share_an_id ... ok
test a_node_renamed_a_thousand_times_keeps_its_id ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/review_p1_membrane_is_by_id.rs (<dedicated-target>/debug/deps/review_p1_membrane_is_by_id-7b5a918e521e9c90)

running 2 tests
test canonical_state_resolves_the_references_it_holds_and_answers_none_for_one_it_does_not ... ok
    Checking ekr-graph-tests v0.0.0 (<dedicated-target>/tests/trybuild/ekr-graph)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s


test tests/review_p1_compile_fail/a_canonical_edge_may_target_a_candidate_node.rs ... ok


test the_sealed_reference_types_are_sound_and_canonical_state_holds_none_of_them ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/revision_events.rs (<dedicated-target>/debug/deps/revision_events-8b30a7d9b5976684)

running 5 tests
test no_two_variants_share_an_encoding ... ok
test every_variant_carries_its_declared_index_and_domain_name ... ok
test an_event_encodes_as_a_function_of_its_value ... ok
test the_variant_marker_and_not_the_payload_is_what_separates_two_events ... ok
test the_declaration_order_of_the_variants_equals_their_numbering ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/snapshot_reads.rs (<dedicated-target>/debug/deps/snapshot_reads-7c27a9f0cbcff8a8)

running 7 tests
test a_snapshot_names_the_revision_it_reads ... ok
test the_historical_query_returns_alice ... ok
test a_proposed_assertion_is_never_answered ... ok
test the_handover_instant_belongs_to_exactly_one_of_them ... ok
test a_record_whose_transaction_time_is_closed_is_not_current ... ok
test valid_at_never_returns_a_retracted_or_superseded_assertion ... ok
test valid_at_answers_alice_before_the_handover_and_bob_at_or_after_it ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (<dedicated-target>/debug/deps/ekr_kernel-dcebe1934f67cb0b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_membrane.rs (<dedicated-target>/debug/deps/adversary_membrane-f528e4a1cc1a1079)

running 5 tests
test an_assertion_whose_only_evidence_the_proposer_invented_is_refused ... ok
test an_add_assertion_over_an_id_canonical_state_already_holds_is_refused ... ok
test a_node_created_under_a_graph_root_that_does_not_exist_is_refused ... ok
test a_create_node_over_an_id_canonical_state_already_holds_is_refused ... ok
test reordering_the_operations_of_one_transaction_does_not_change_the_verdict ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_membrane_pass_two.rs (<dedicated-target>/debug/deps/adversary_membrane_pass_two-7a85a72ebf8dcd25)

running 4 tests
test a_node_is_not_merged_into_itself ... ok
test a_type_the_ontology_already_declares_is_not_declared_again ... ok
test a_proposer_cannot_mark_its_own_assertion_accepted ... ok
test an_assertion_naming_a_type_the_ontology_does_not_declare_is_refused ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_07.rs (<dedicated-target>/debug/deps/adversary_p1_07-d7562ee04b18eba3)

running 5 tests
test inherited_opaque_constraints_are_refused_on_child_node_properties ... ok
test retracting_a_retained_edge_assertion_does_not_erase_its_reference ... ok
test cancellation_cannot_leave_a_new_assertion_referring_to_the_cancelled_edge ... ok
test independent_property_and_lifecycle_writes_are_order_invariant ... ok
test assertion_subject_predicate_object_cross_product_obeys_declarations ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_08_seed.rs (<dedicated-target>/debug/deps/adversary_p1_08_seed-b13a50007eca94e0)

running 4 tests
test seed_ontology_property_definitions_must_be_filed_under_their_own_ids ... ok
test inherited_record_properties_remain_typed_and_constraints_cannot_disappear ... ok
test losing_cached_seed_keeps_its_original_retention_class ... ok
test persisted_seed_format_and_context_fields_refuse_before_admission ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.78s

     Running tests/commit_path.rs (<dedicated-target>/debug/deps/commit_path-816fb6c18858336f)

running 5 tests
test a_commit_into_an_unseeded_lineage_is_refused ... ok
test a_transaction_validated_against_a_revision_the_lineage_moved_past_is_refused ... ok
test the_same_lineage_written_by_hand_does_not_advance_anything ... ok
test the_kernels_authority_stands_behind_exactly_what_it_validated ... ok
test a_validated_transaction_commits_and_the_lineage_advances ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/encoding_field_order.rs (<dedicated-target>/debug/deps/encoding_field_order-2a7ec28608d4a18e)

running 1 test
test every_field_of_the_kernels_encodings_is_written_in_declaration_order ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/legacy.rs (<dedicated-target>/debug/deps/legacy-1ef43957b620c981)

running 13 tests
test original_revision_root_fields_are_fixed ... ok
test all_six_original_event_discriminants_and_fields_are_fixed ... ok
test seed_envelope_reproduces_actual_original_bootstrap_attribution ... ok
test user_record_discriminator_keys_survive_and_floats_do_not_gain_an_encoding ... ok
test all_original_operations_include_transitive_ontology_and_assertion_bytes ... ok
test every_original_combined_assertion_state_and_evidence_source_is_fixed ... ok
test original_value_scalar_node_and_edge_bytes_are_fixed ... ok
test withdrawn_legacy_assertions_never_acquire_invented_acceptance_history ... ok
test frozen_contracts_refuse_unknown_fields_and_duplicate_set_members ... ok
test original_transaction_validation_uses_payload_domain_and_ordered_operations ... ok
test missing_and_corrupt_original_evidence_are_named_refusals ... ok
test historical_claims_require_the_supplied_bytes_and_revision ... ok
test original_formats_unknown_fields_and_duplicate_semantic_identities_refuse ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/seed.rs (<dedicated-target>/debug/deps/seed-db0d520306fccdc5)

running 25 tests
test legitimate_record_keys_remain_data_in_strict_seed_decoding ... ok
test seed_decoding_refuses_unknown_semantic_fields_at_every_record_boundary ... ok
test a_seed_ontology_cannot_claim_a_predecessor ... ok
test a_seed_with_a_dangling_edge_is_refused_by_both_backends ... ok
test a_seed_ontology_cannot_claim_a_later_version ... ok
test a_seed_with_a_caller_verdict_is_refused_by_both_backends ... ok
test a_seed_with_an_undeclared_type_is_refused_by_both_backends ... ok
test a_valid_seed_has_a_positive_control ... ok
test empty_bootstrap_does_not_make_empty_transactions_valid ... ok
test repeated_initialization_preserves_the_lineage_and_writes_no_second_object ... ok
test reopen_checks_full_ontology_and_execution_context ... ok
test concurrent_independent_handles_publish_exactly_one_seed_and_no_losing_object ... ok
test initialization_reuses_exact_cached_seed_bytes_atomically ... ok
test required_properties_and_unsupported_constraints_keep_their_refusals ... ok
test the_versioned_minimal_yaml_fixture_initializes_both_real_providers ... ok
test seed_multiplicity_and_property_types_are_checked ... ok
test actual_bootstrap_identities_refuse_self_validation_and_false_attribution ... ok
test named_seed_refusals_write_neither_the_object_nor_the_revision ... ok
test seed_lifecycle_is_the_declared_initial_state_and_is_preserved ... ok
test an_evidence_seed_reopens_with_identical_roots_fields_and_retained_bytes ... ok
test floats_are_refused_in_seed_nodes_edges_and_assertion_objects ... ok
test canonical_seed_root_filing_schema_and_genesis_are_mandatory ... ok
test legacy_and_tampered_seed_envelopes_are_preserved_but_never_admitted ... ok
test seed_support_checks_missing_uncited_and_unsupported_evidence ... ok
test every_seed_entity_map_checks_key_and_root_identity ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.31s

     Running tests/validate_properties.rs (<dedicated-target>/debug/deps/validate_properties-bebd2f6d483597a0)

running 5 tests
test the_two_walks_of_one_rule_agree ... ok
test no_transaction_is_ever_validated_by_its_own_proposer ... ok
test a_float_anywhere_is_always_refused ... ok
test validating_twice_gives_the_same_answer ... ok
test the_encoding_separates_transactions_that_differ ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/validation.rs (<dedicated-target>/debug/deps/validation-09bc7f3eafde1f2a)

running 49 tests
test a_node_of_an_abstract_type_is_refused ... ok
test a_merge_names_two_nodes ... ok
test a_canonical_assertion_without_evidence_is_refused_by_the_provenance_validator ... ok
test a_reference_to_something_the_same_transaction_creates_resolves ... ok
test a_transaction_with_no_operations_is_refused_by_the_structural_validator ... ok
test a_transaction_whose_validator_is_its_proposer_is_refused ... ok
test a_value_that_fails_its_type_is_refused_by_the_type_validator ... ok
test a_type_the_ontology_already_declares_is_not_declared_again ... ok
test a_value_canonical_state_does_not_admit_is_refused_with_its_path ... ok
test a_record_under_another_graph_root_is_refused ... ok
test an_edge_cardinality_the_type_forbids_is_refused_by_the_cardinality_validator ... ok
test a_valid_transaction_validates ... ok
test an_edge_endpoint_of_the_wrong_type_is_refused ... ok
test a_valid_transaction_validates_to_the_same_hash_twice ... ok
test an_empty_property_assignment_still_requires_a_declared_property ... ok
test an_evidence_id_the_transaction_declares_does_not_make_it_exist ... ok
test an_invoke_the_lifecycle_does_not_declare_is_refused_by_the_ontology_validator ... ok
test an_identity_created_twice_in_one_transaction_is_refused ... ok
test an_unresolvable_reference_is_refused_by_the_reference_validator ... ok
test an_operation_the_type_does_not_declare_is_refused ... ok
test an_invocation_carries_exactly_the_arguments_its_operation_declares ... ok
test deleting_an_edge_referenced_by_a_new_or_retained_assertion_is_refused ... ok
test an_unresolved_assertion_endpoint_does_not_acquire_an_unrelated_type_refusal ... ok
test competing_lifecycle_writes_are_refused ... ok
test an_identity_canonical_state_already_holds_is_not_created_again ... ok
test an_assertion_cannot_arrive_carrying_its_own_verdict ... ok
test property_cardinality_and_required_presence_are_refused ... ok
test every_type_refusal_the_validator_can_make_is_reachable ... ok
test an_assertions_types_are_resolved_whatever_shape_its_object_has ... ok
test opaque_preconditions_and_emissions_are_not_silently_accepted ... ok
test applicable_opaque_property_constraints_refuse_all_node_write_paths ... ok
test the_encoding_writes_id_bearing_fields_in_declaration_order ... ok
test property_assertions_check_type_objects_edge_properties_and_type_subjects ... ok
test every_field_of_every_encoded_type_reaches_its_encoding ... ok
test property_cardinality_uses_the_candidate_node ... ok
test the_pipeline_runs_the_seven_deterministic_validators_in_order ... ok
test competing_property_writes_are_refused_in_every_order ... ok
test every_kind_of_dangling_reference_is_refused ... ok
test the_declared_evidence_set_is_the_evidence_the_assertions_cite ... ok
test the_eleven_operation_numbers_are_the_domains_and_the_declarations ... ok
test relation_assertions_check_both_endpoint_types_and_allow_inherited_types ... ok
test opaque_edge_property_constraints_refuse_creation_and_assertions ... ok
test unsupported_schema_changes_and_merges_refuse_explicitly ... ok
test the_validation_hash_covers_the_revision_it_was_validated_against ... ok
test two_transactions_that_differ_validate_to_different_hashes ... ok
test relation_assertions_require_compatible_node_endpoints ... ok
test a_serialized_lifecycle_invokes_but_unsupported_sets_refuse_at_decode ... ok
test every_issue_code_the_kernel_can_raise_is_raised_by_a_case ... ok
    Checking ekr-store v0.0.0 (<worktree>/crates/ekr-store)
    Checking ekr-kernel v0.0.0 (<worktree>/crates/ekr-kernel)
    Checking ekr-kernel-tests v0.0.0 (<dedicated-target>/tests/trybuild/ekr-kernel)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.02s


test tests/compile_fail/validated_transaction_cannot_be_deserialised.rs ... ok
test tests/compile_fail/validated_transaction_has_no_constructor_outside_the_kernel.rs ... ok


test only_the_kernel_constructs_a_validated_transaction ... ok

test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.21s

     Running unittests src/lib.rs (<dedicated-target>/debug/deps/ekr_store-faf645b78e873b91)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_event_vocabulary.rs (<dedicated-target>/debug/deps/adversary2_event_vocabulary-5c08c3becd8c2526)

running 1 test
test a_second_rejection_of_a_re_proposed_transaction_is_not_swallowed_as_a_retry ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/adversary2_retention_event_contract.rs (<dedicated-target>/debug/deps/adversary2_retention_event_contract-a82a827ca939b61d)

running 2 tests
test the_store_domain_declares_the_retention_raise_event_this_crate_writes ... ok
test the_retention_raise_in_stored_bytes_carries_the_fields_the_domain_declares ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/adversary_objects_and_append.rs (<dedicated-target>/debug/deps/adversary_objects_and_append-4988f0c4b1471b34)

running 2 tests
test storing_canonical_bytes_that_were_cached_earlier_records_them_as_canonical ... ok
test retrying_an_append_writes_the_same_event_once_rather_than_twice ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running tests/adversary_p1_06_reference_from_bytes.rs (<dedicated-target>/debug/deps/adversary_p1_06_reference_from_bytes-720826f958dc140b)

running 1 test
test a_store_cannot_turn_dangling_document_bytes_into_canonical_state ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running tests/domain_projection.rs (<dedicated-target>/debug/deps/domain_projection-26a37c3aca18a7fa)

running 7 tests
test the_storage_classes_are_the_ones_the_domain_declares ... ok
test the_derived_ordering_is_not_the_retention_ordering ... ok
test every_storage_class_has_its_own_retention_rank ... ok
test the_strongest_of_two_classes_does_not_depend_on_which_arrived_first ... ok
test the_retention_ladder_is_the_one_section_thirty_seven_describes ... ok
test every_event_the_crate_writes_is_declared_by_the_domain ... ok
test every_event_the_crate_writes_carries_the_fields_the_domain_declares ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/fold_rules.rs (<dedicated-target>/debug/deps/fold_rules-f25e2bdb78fb33f7)

running 18 tests
test evidence_is_addressed_apart_from_knowledge ... ok
test the_knowledge_root_is_a_function_of_the_graph_state ... ok
test an_empty_log_has_no_head_and_does_not_fold ... ok
test a_lineage_that_does_not_begin_at_a_seed_is_refused ... ok
test a_seed_naming_bytes_the_store_does_not_hold_is_refused ... ok
test a_commit_of_a_transaction_that_was_never_validated_is_refused ... ok
test a_commit_whose_validation_the_authority_stands_behind_advances_the_lineage ... ok
test validating_a_transaction_that_was_never_proposed_is_refused ... ok
test a_revision_that_does_not_follow_its_parent_is_refused ... ok
test a_commit_whose_validation_the_authority_does_not_stand_behind_does_not_advance_the_lineage ... ok
test a_commit_publishing_a_knowledge_root_the_fold_does_not_reach_is_refused ... ok
test two_of_the_five_sub_roots_are_the_placeholder_and_not_derived ... ok
test a_store_with_no_commit_authority_refuses_to_say_what_canonical_state_is ... ok
test a_second_seed_is_refused ... ok
test a_transaction_that_went_stale_cannot_then_commit ... ok
test a_rejected_transaction_cannot_then_commit ... ok
test a_commit_validated_against_a_revision_the_lineage_has_moved_past_does_not_advance_it ... ok
test two_stores_seeded_from_different_state_have_different_heads ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s

     Running tests/legacy.rs (<dedicated-target>/debug/deps/legacy-b369c36c4f7eb88e)

running 4 tests
test all_user_record_keys_are_preserved_when_unique ... ok
test duplicate_keys_are_named_refusals_even_inside_user_records ... ok
test original_document_fixture_verifies_exact_payload_bytes ... ok
test original_documents_refuse_unknown_fields_and_misfiled_identities ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/membrane_boundary.rs (<dedicated-target>/debug/deps/membrane_boundary-4fd8b8cc0e26267e)

running 1 test
test a_candidate_node_and_a_canonical_node_write_the_same_document ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/providers.rs (<dedicated-target>/debug/deps/providers-92620baa01a15f37)

running 15 tests
test a_file_store_stores_identical_bytes_once ... ok
test a_cache_write_after_a_canonical_one_leaves_the_bytes_canonical ... ok
test a_raised_retention_class_survives_a_reopen ... ok
test storing_a_graph_is_storing_its_document ... ok
test sqlite_stores_identical_bytes_once ... ok
test a_stored_object_carries_the_class_it_was_written_under ... ok
test a_replay_from_a_revision_with_no_materialised_state_is_refused ... ok
test every_event_shape_reports_a_write_once_and_a_recognition_after ... ok
test the_fold_carries_the_seed_the_log_named ... ok
test a_sqlite_store_reopened_folds_to_the_same_head_root ... ok
test the_recorded_class_is_the_strongest_requested_whichever_order_they_arrive_in ... ok
test two_different_events_appended_in_a_row_both_land ... ok
test sqlite_replay_from_the_seed_equals_the_fold ... ok
test a_file_store_reopened_folds_to_the_same_head_root ... ok
test a_file_store_replay_from_the_seed_equals_the_fold ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s

     Running tests/review_p1_invariant_one_at_the_store.rs (<dedicated-target>/debug/deps/review_p1_invariant_one_at_the_store-82651e84b28006c4)

running 2 tests
test a_commit_lands_with_no_validated_transaction_anywhere_in_the_process ... ok
test a_commit_validated_against_a_revision_that_is_no_longer_the_head_replays_as_valid ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/seed_object_integrity.rs (<dedicated-target>/debug/deps/seed_object_integrity-d7384f3e5473b860)

running 2 tests
test sqlite_seed_objects_verify_address_length_and_retention_before_admission ... ok
test file_seed_objects_verify_address_length_and_retention_before_admission ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s

   Doc-tests ekr_graph

running 1 test
test crates/ekr-graph/src/lib.rs - (line 57) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

   Doc-tests ekr_kernel

running 2 tests
test crates/ekr-kernel/src/commit.rs - commit::Commit<S>::over (line 180) ... ok
test crates/ekr-kernel/src/lib.rs - (line 38) ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

   Doc-tests ekr_store

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


exit: 0
```

Counts are taken from the runner summary lines, including doctests and the trybuild wrapper cases (nested diagnostic inputs are not double-counted). New test targets were absent from the base; every pre-existing target retains its count:

```text
TOTAL 221 -> 238
Doc-tests ekr_graph: 1 -> 1
Doc-tests ekr_kernel: 2 -> 2
Doc-tests ekr_store: 0 -> 0
ekr_graph lib: 0 -> 0
ekr_graph tests/adversary2_guard_bounds_and_ranges.rs: 4 -> 4
ekr_graph tests/adversary2_membrane_and_addresses.rs: 2 -> 2
ekr_graph tests/adversary_canonical_value_reach.rs: 3 -> 3
ekr_graph tests/adversary_p1_06_reference_markers.rs: 2 -> 2
ekr_graph tests/adversary_snapshot_and_assertion.rs: 4 -> 4
ekr_graph tests/canonical_value_and_assertion.rs: 18 -> 18
ekr_graph tests/domain_projection.rs: 3 -> 3
ekr_graph tests/evidence_and_observations.rs: 6 -> 6
ekr_graph tests/membrane.rs: 5 -> 5
ekr_graph tests/node_identity.rs: 3 -> 3
ekr_graph tests/review_p1_membrane_is_by_id.rs: 2 -> 2
ekr_graph tests/revision_events.rs: 5 -> 5
ekr_graph tests/snapshot_reads.rs: 7 -> 7
ekr_kernel lib: 0 -> 0
ekr_kernel tests/adversary_membrane.rs: 5 -> 5
ekr_kernel tests/adversary_membrane_pass_two.rs: 4 -> 4
ekr_kernel tests/adversary_p1_07.rs: 5 -> 5
ekr_kernel tests/adversary_p1_08_seed.rs: 4 -> 4
ekr_kernel tests/commit_path.rs: 5 -> 5
ekr_kernel tests/encoding_field_order.rs: 1 -> 1
ekr_kernel tests/legacy.rs: absent -> 13
ekr_kernel tests/seed.rs: 25 -> 25
ekr_kernel tests/validate_properties.rs: 5 -> 5
ekr_kernel tests/validation.rs: 49 -> 49
ekr_store lib: 0 -> 0
ekr_store tests/adversary2_event_vocabulary.rs: 1 -> 1
ekr_store tests/adversary2_retention_event_contract.rs: 2 -> 2
ekr_store tests/adversary_objects_and_append.rs: 2 -> 2
ekr_store tests/adversary_p1_06_reference_from_bytes.rs: 1 -> 1
ekr_store tests/domain_projection.rs: 7 -> 7
ekr_store tests/fold_rules.rs: 18 -> 18
ekr_store tests/legacy.rs: absent -> 4
ekr_store tests/membrane_boundary.rs: 1 -> 1
ekr_store tests/providers.rs: 15 -> 15
ekr_store tests/review_p1_invariant_one_at_the_store.rs: 2 -> 2
ekr_store tests/seed_object_integrity.rs: 2 -> 2
```

```text
$ cargo clippy -p ekr-graph -p ekr-store -p ekr-kernel --all-targets -- -D warnings
    Checking ekr-store v0.0.0 (<worktree>/crates/ekr-store)
    Checking ekr-kernel v0.0.0 (<worktree>/crates/ekr-kernel)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.30s

exit: 0
```

```text
$ cargo fmt --all --check

exit: 0
```

```text
$ RUSTDOCFLAGS='-D rustdoc::broken_intra_doc_links -D warnings' cargo doc -p ekr-graph -p ekr-store -p ekr-kernel --no-deps
    Blocking waiting for file lock on build directory
    Checking ekr-graph v0.0.0 (<worktree>/crates/ekr-graph)
 Documenting ekr-graph v0.0.0 (<worktree>/crates/ekr-graph)
    Checking ekr-store v0.0.0 (<worktree>/crates/ekr-store)
 Documenting ekr-store v0.0.0 (<worktree>/crates/ekr-store)
 Documenting ekr-kernel v0.0.0 (<worktree>/crates/ekr-kernel)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.67s
   Generated <dedicated-target>/doc/ekr_graph/index.html and 2 other files

exit: 0
```

git diff --check exited 0. The whole repository task check and independent review remain the coordinator's integration gate. Final free-space observation was 16 GB, above the 10 GB build floor. The dedicated target and retained scratch evidence were not removed.

Deliberate boundaries:

- No production V2 activation, canonical property multiplicity change, retraction repair, event occurrence envelope, validation-domain repair, current seed API change or new publication behavior.
- No provider opening/inventory/copy, backend metadata parser, complete-history exporter, live data migration, destination publication or store reset. The pure helpers cover exactly their supplied bytes and addresses; transport coordinates/completeness stay with the later inventory adapter.
- No missing original operations, prior acceptance, ontology or validation authority is synthesized. Those remain necessary for a later complete preserving migration.
- Frozen seed verification reads the persisted JSON representation; it is not a replacement for the production YAML CLI input parser or current seed admission.
- No AEP/ESS/design edits, dependency changes, commits or publication. Combined story remains active for production activation and the still-owned provider/migration acceptance.

Raw producer logs are retained verbatim in assigned scratch. This public report normalizes private checkout/cache prefixes only; that normalization is explicit and does not change test names, error messages, counts or verdicts. The separate private run record retains unnormalized output and full external paths.
