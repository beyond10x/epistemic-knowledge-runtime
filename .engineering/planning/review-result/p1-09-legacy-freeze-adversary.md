---
format: aep.planning-md/2
id: review-result:p1-09-legacy-freeze-adversary
kind: review-result
status: active
title: Independent original-format freeze review
relations:
- reviews: story:version-persisted-contracts
revision: 1
---
unit: P1-09 original-format freeze preparation, opening 151be425fafb9013368d1a74d2d3d1f1019834ce plus stable implementation
verdict: nothing found within the bounded frozen-format preparation
cases: executed 238→244, red 0, ignored 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned scratch files and dedicated compiler target; exact private path manifest retained
needs-coordinator: integrated full gate and remaining production format/migration prerequisites; no source correction requested

```text
git --no-pager diff --stat
 crates/ekr-graph/src/lib.rs                        |   2 +
 .../tests/canonical_value_and_assertion.rs         | 275 ++++++++++++++++++++-
 ...nical_ref_cannot_target_a_transient_type.stderr |  18 +-
 .../transient_state_has_no_content_address.stderr  |   8 +-
 crates/ekr-kernel/src/lib.rs                       |   2 +
 crates/ekr-store/src/lib.rs                        |   2 +
 6 files changed, 287 insertions(+), 20 deletions(-)

git diff --no-index --stat /dev/null crates/ekr-kernel/tests/adversary_legacy_freeze.rs
 .../ekr-kernel/tests/adversary_legacy_freeze.rs | 297 +++++++++++++++++++++
 1 file changed, 297 insertions(+)
```

Owners: zero findings. The implementor owns the frozen modules, original fixed-vector tests and fixtures, explicit dual-family encoding guard, and diagnostic-only snapshot updates. The coordinator owns normative scope, planning, integration and subsequent production activation/migration. This reviewer owns only the new untracked adversary test file and assigned scratch outputs. The whole-tree diff above is inherited implementation work, not reviewer-owned source work; the only repository write made by this reviewer is the 297-line test file shown separately.

Stable test SHA256:
`2a1d5278aed73d6ce5e896e4560b9ce6b0603fbdb4a5898b2be040070957cecb`.

## 1. Scope and source assessment

The source reviewed is additive frozen-original DTO/encoding/address verification. It does not activate V2, admit canonical state, authenticate a historical actor, prove complete transport history, open a source provider, migrate a live source or publish a destination. Those are explicitly separate obligations in the story and implementation report, and were not inferred from address reproduction.

The three frozen modules were read together with current transaction/seed admission, original capture program and retained capture provenance, fixed fixture metadata, and existing regression/guard changes. Frozen graph types use their own values, assertions, evidence, roots and events. Frozen kernel operations use their own recursive ontology declarations and the frozen assertion types. Store verification uses frozen graph records. Current graph/ontology/kernel codecs appear only in historical explanatory references, not as encoding calls. Shared encoding dependencies are the documented ekr-core scalar identities, timestamps, hashes and primitive containers; fixed vectors pin their observed behavior.

A read-only production-source comparison against the capture revision exited 0 with no output:

```sh
git diff --exit-code 73ab8b0a5aa5c670bacbc4c1abdca02f87b62bf0 -- \
  crates/ekr-core/src crates/ekr-ontology/src crates/ekr-graph/src \
  ':!crates/ekr-graph/src/lib.rs' ':!crates/ekr-graph/src/legacy.rs' \
  crates/ekr-kernel/src/transaction.rs crates/ekr-kernel/src/seed.rs \
  crates/ekr-kernel/src/validate crates/ekr-store/src/snapshot.rs crates/ekr-store/src/log.rs
```

The omitted graph lib change only exports the new historical module. No capture fixture was regenerated. The one-shot capture program was inspected: it calls the original seed validator/replayer for the seed acceptance vectors and the original ValidatedTransaction seal for the validation hash. This review does not claim to have rerun that producer; it verifies its retained results independently against the frozen decoder/encoder and confirms the relevant production source still matches the stated capture revision.

The graph encoding guard continues to exercise both current and frozen sum-type families with distinct fixture names. The two E0277 snapshot changes preserve their rejected input and error/trait-bound substance; only type qualification/help ordering changes. The full package run executes those compile-fail wrappers. No capability-construction escape was added by plain historical references.

## 2. New cases, written before suite execution

All new cases are in crates/ekr-kernel/tests/adversary_legacy_freeze.rs. They use immutable fixture bytes and synthetic in-memory mutations only. No current production serializer/encoder is used as the expected output.

| Case | Executed assertion | Result |
|---|---|---|
| adversary_nested_original_fields_refuse_unknown_semantics | Unknown fields nested in lifecycle, operation argument type, assertion time and evidence source refuse. | Green, individually selected before suite. |
| adversary_frozen_serializers_preserve_all_original_capture_bytes | Every one of the 52 captured JSON strings reproduces exactly when serialized from its frozen type; unknown fixture kinds cannot skip silently. | Green, individually selected before suite. |
| adversary_validation_receipts_refuse_identical_bytes_in_wrong_hash_domain_and_order | The exact validation encoding hashed in the value domain refuses; rotating operations or duplicating AddAssertion cannot reuse the old validation hash. Newly computed supplied-byte addresses remain reproducible as data. | Green, individually selected before suite. |
| adversary_seed_attribution_preserves_input_and_refuses_mismatched_original_contexts | Attribution leaves retained Proposed input unchanged, changes knowledge but not evidence root, and refuses ten mismatched original bootstrap context/attribution/source shapes. | Green, individually selected before suite. |
| adversary_nonproposed_seed_states_never_receive_reconstructed_acceptance | Every six non-Proposed combined states—including caller Accepted, Superseded and Retracted—refuses invented bootstrap acceptance. | Green, individually selected before suite. |
| adversary_noncanonical_identity_spelling_and_escaped_duplicate_keys_refuse | Uppercase noncanonical UUID spelling refuses and escaped duplicate user Record keys are detected after JSON unescaping. | Green after correcting the test's initial error-category expectation. |

First exact case:
`cargo test -p ekr-kernel --test adversary_legacy_freeze adversary_nested_original_fields_refuse_unknown_semantics -- --exact --nocapture`

Exit 0, one executed. Retained output with private path prefixes substituted:

```text
   Compiling ekr-kernel v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-09-format-20260922/crates/ekr-kernel)
warning: unused imports: `Canonical` and `Encoder`
 --> crates/ekr-kernel/tests/adversary_legacy_freeze.rs:2:27
  |
2 | use ekr_core::canonical::{Canonical, Encoder};
  |                           ^^^^^^^^^  ^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused imports: `ContentHash` and `RevisionNumber`
 --> crates/ekr-kernel/tests/adversary_legacy_freeze.rs:3:16
  |
3 | use ekr_core::{ContentHash, RevisionNumber};
  |                ^^^^^^^^^^^  ^^^^^^^^^^^^^^

warning: unused imports: `GraphDocument` and `Refusal`
 --> crates/ekr-kernel/tests/adversary_legacy_freeze.rs:6:38
  |
6 | use ekr_store::legacy::{decode_json, GraphDocument, Refusal};
  |                                      ^^^^^^^^^^^^^  ^^^^^^^

warning: unused import: `serde::de::DeserializeOwned`
 --> crates/ekr-kernel/tests/adversary_legacy_freeze.rs:7:5
  |
7 | use serde::de::DeserializeOwned;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `serde::Serialize`
 --> crates/ekr-kernel/tests/adversary_legacy_freeze.rs:8:5
  |
8 | use serde::Serialize;
  |     ^^^^^^^^^^^^^^^^

warning: `ekr-kernel` (test "adversary_legacy_freeze") generated 5 warnings (run `cargo fix --test "adversary_legacy_freeze" -p ekr-kernel` to apply 5 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.24s
     Running tests/adversary_legacy_freeze.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_legacy_freeze-ce0092a9bb93b269)

running 1 test
test adversary_nested_original_fields_refuse_unknown_semantics ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


```

The staged imports in that first file were used by the five following cases; final clippy is clean.

Each subsequent case ran alone with the same command form and its full name before the suite. Their exact raw outputs are retained under the correspondingly named logs listed below. No source defect produced a red case.

One failed probe was a reviewer harness mistake, not a finding. Its initial expectation was that uppercase/lowercase UUID keys decode to the same identity and should therefore yield DuplicateKey. ekr-core already refuses the uppercase spelling with InvalidData before that point. The result correctly refuses the input. The case was renamed to state that actual invariant, then separately checked escaped duplicate user keys against DuplicateKey. Both initial diagnostic logs remain retained; no production source or pre-existing assertion was changed to make them pass.

Measured diagnostic, excluded from findings:

```text
   Compiling ekr-kernel v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-09-format-20260922/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.25s
     Running tests/adversary_legacy_freeze.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_legacy_freeze-ce0092a9bb93b269)

running 1 test

thread 'adversary_duplicate_decoded_identities_and_escaped_user_keys_are_not_lost' (594765) panicked at crates/ekr-kernel/tests/adversary_legacy_freeze.rs:288:5:
semantic duplicate: Err(InvalidData("\"00000000-0000-4000-8000-00000000000C\" is not a UUID identifier"))
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test adversary_duplicate_decoded_identities_and_escaped_user_keys_are_not_lost ... FAILED

failures:

failures:
    adversary_duplicate_decoded_identities_and_escaped_user_keys_are_not_lost

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr-kernel --test adversary_legacy_freeze`

```

Corrected exact case, exit 0:

```text
   Compiling ekr-kernel v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-09-format-20260922/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.24s
     Running tests/adversary_legacy_freeze.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_legacy_freeze-ce0092a9bb93b269)

running 1 test
test adversary_noncanonical_identity_spelling_and_escaped_duplicate_keys_refuse ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s


```

## 3. Package suite after all cases existed

Baseline 238 is the implementor's retained final runner count; this reviewer did not run a preemptive baseline suite. Six new top-level cases yield 244 executed. Counted from the runner summary lines: 244 pass, 0 fail, 0 ignored. Counts include doctests and compile-fail wrapper cases, without double-counting nested trybuild inputs. All 238 prior cases remain selected and green.

Environment:
- CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-09-format-20260922
- CARGO_BUILD_JOBS=2
- TMPDIR=<cache>/ekr-completion-20260922/persisted-contract/adversary

Free disk was measured before compiler calls at 20 GB, then 19 GB, above the assigned 10 GB floor.

Command: `cargo test -p ekr-graph -p ekr-store -p ekr-kernel --no-fail-fast`.
Exit 0. Complete retained output, private prefixes substituted:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.06s
     Running unittests src/lib.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/ekr_graph-4851d02fd854460b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_guard_bounds_and_ranges.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary2_guard_bounds_and_ranges-ade199b341027c49)

running 4 tests
test an_inverted_range_is_refused_or_describes_some_instant ... ok
test the_item_scanner_is_the_same_text_in_both_files ... ok
test the_guard_against_a_returning_open_ended_read_covers_the_whole_crate ... ok
test the_field_guard_catches_a_new_field_whatever_it_is_called ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_membrane_and_addresses.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary2_membrane_and_addresses-eb0fa4f6b69b2bc0)

running 2 tests
test every_entity_canonical_state_holds_has_a_content_address ... ok
test the_seal_of_canonical_dependency_is_carried_only_by_a_canonical_dependency ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_canonical_value_reach.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_canonical_value_reach-f38bc0a0dfec49f2)

running 3 tests
test a_transient_candidate_assertion_may_carry_an_approximate_measurement ... ok
test a_transient_candidate_node_may_hold_an_approximate_measurement ... ok
test every_part_of_graph_state_has_a_canonical_encoding ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_06_reference_markers.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_p1_06_reference_markers-5448aa5e495d6e6b)

running 2 tests
test a_reference_to_evidence_resolves_to_a_node_because_the_marker_is_decoration ... ok
test the_default_reference_of_a_transient_claim_is_the_transient_reference ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_snapshot_and_assertion.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_snapshot_and_assertion-11ab10473e2780ec)

running 4 tests
test a_record_with_no_transaction_time_at_all_is_not_a_current_belief ... ok
test is_current_is_exactly_acceptance_and_an_open_transaction_time ... ok
test the_domain_requires_a_recorded_from_and_the_crate_cannot_omit_one ... ok
test active_is_not_merely_valid_at_the_end_of_representable_time ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/canonical_value_and_assertion.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/canonical_value_and_assertion-4145a44c64ff8db4)

running 18 tests
test a_float_at_any_depth_cannot_inhabit_a_canonical_value_and_the_refusal_names_it ... ok
test a_node_and_an_edge_property_carry_only_an_admissible_value ... ok
test an_admissible_value_round_trips_through_the_newtype ... ok
test every_field_of_a_node_and_an_edge_reaches_the_encoding ... ok
test every_field_of_an_assertion_reaches_the_encoding ... ok
test no_two_edges_that_differ_share_a_content_address ... ok
test every_field_of_evidence_state_reaches_the_encoding ... ok
test no_two_nodes_that_differ_share_a_content_address ... ok
test graph_state_equal_in_every_field_hashes_equally ... ok
test no_two_observations_that_differ_share_a_content_address ... ok
test no_two_pieces_of_evidence_that_differ_share_a_content_address ... ok
test the_conversion_refuses_exactly_where_the_ontology_says_it_must ... ok
test no_two_support_links_that_differ_share_a_content_address ... ok
test no_two_assertions_that_differ_share_a_content_address ... ok
test every_variant_of_every_sum_type_opens_with_its_own_marker ... ok
test two_values_that_differ_do_not_encode_alike ... ok
test two_assertions_equal_in_every_field_hash_equally ... ok
test the_declaration_order_of_every_sum_type_equals_its_numbering ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/domain_projection.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/domain_projection-9784aef31543abe4)

running 3 tests
test every_enumeration_the_domain_declares_is_carried_variant_for_variant ... ok
test the_crate_quotes_the_domains_own_sentence_about_validation_state_payloads ... ok
test every_declaration_of_the_domain_is_carried_field_for_field ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/evidence_and_observations.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/evidence_and_observations-8349d28dea5445a2)

running 6 tests
test confidence_outside_its_declared_range_is_not_constructible ... ok
test a_blob_observation_carries_its_media_type_and_length ... ok
test every_evidence_source_answers_the_flat_fields_the_domain_declares ... ok
test every_observation_form_answers_its_kind_and_its_content_hash ... ok
test support_links_one_assertion_to_one_piece_of_evidence ... ok
test evidence_carries_its_provenance ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/membrane.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/membrane-232536dd185bd11a)

running 5 tests
test a_canonical_reference_is_the_only_canonical_dependency ... ok
test canonical_state_resolves_a_canonical_reference ... ok
test a_candidate_and_a_canonical_node_that_share_an_id_do_not_resolve_alike ... ok
test transient_state_may_depend_on_canonical_state_and_on_its_own ... ok
    Checking ekr-graph-tests v0.0.0 (<cache>/b10x-target/ekr-p1-09-format-20260922/tests/trybuild/ekr-graph)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s


test tests/compile_fail/canonical_dependency_is_sealed.rs ... ok
test tests/compile_fail/canonical_graph_rejects_a_transient_ref.rs ... ok
test tests/compile_fail/canonical_ref_cannot_target_a_transient_type.rs ... ok
test tests/compile_fail/transient_state_has_no_content_address.rs ... ok


test the_membrane_is_a_set_of_build_failures ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s

     Running tests/node_identity.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/node_identity-cc0577f9dcabf6fc)

running 3 tests
test the_lifecycle_state_of_a_node_is_a_property_and_not_its_identity ... ok
test two_nodes_that_share_a_name_do_not_share_an_id ... ok
test a_node_renamed_a_thousand_times_keeps_its_id ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/review_p1_membrane_is_by_id.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/review_p1_membrane_is_by_id-7b5a918e521e9c90)

running 2 tests
test canonical_state_resolves_the_references_it_holds_and_answers_none_for_one_it_does_not ... ok
    Checking ekr-graph-tests v0.0.0 (<cache>/b10x-target/ekr-p1-09-format-20260922/tests/trybuild/ekr-graph)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s


test tests/review_p1_compile_fail/a_canonical_edge_may_target_a_candidate_node.rs ... ok


test the_sealed_reference_types_are_sound_and_canonical_state_holds_none_of_them ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/revision_events.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/revision_events-8b30a7d9b5976684)

running 5 tests
test every_variant_carries_its_declared_index_and_domain_name ... ok
test an_event_encodes_as_a_function_of_its_value ... ok
test the_variant_marker_and_not_the_payload_is_what_separates_two_events ... ok
test no_two_variants_share_an_encoding ... ok
test the_declaration_order_of_the_variants_equals_their_numbering ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/snapshot_reads.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/snapshot_reads-7c27a9f0cbcff8a8)

running 7 tests
test a_proposed_assertion_is_never_answered ... ok
test a_snapshot_names_the_revision_it_reads ... ok
test a_record_whose_transaction_time_is_closed_is_not_current ... ok
test the_historical_query_returns_alice ... ok
test the_handover_instant_belongs_to_exactly_one_of_them ... ok
test valid_at_answers_alice_before_the_handover_and_bob_at_or_after_it ... ok
test valid_at_never_returns_a_retracted_or_superseded_assertion ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/ekr_kernel-dcebe1934f67cb0b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_legacy_freeze.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_legacy_freeze-ce0092a9bb93b269)

running 6 tests
test adversary_noncanonical_identity_spelling_and_escaped_duplicate_keys_refuse ... ok
test adversary_nested_original_fields_refuse_unknown_semantics ... ok
test adversary_frozen_serializers_preserve_all_original_capture_bytes ... ok
test adversary_seed_attribution_preserves_input_and_refuses_mismatched_original_contexts ... ok
test adversary_validation_receipts_refuse_identical_bytes_in_wrong_hash_domain_and_order ... ok
test adversary_nonproposed_seed_states_never_receive_reconstructed_acceptance ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/adversary_membrane.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_membrane-f528e4a1cc1a1079)

running 5 tests
test an_add_assertion_over_an_id_canonical_state_already_holds_is_refused ... ok
test an_assertion_whose_only_evidence_the_proposer_invented_is_refused ... ok
test a_create_node_over_an_id_canonical_state_already_holds_is_refused ... ok
test a_node_created_under_a_graph_root_that_does_not_exist_is_refused ... ok
test reordering_the_operations_of_one_transaction_does_not_change_the_verdict ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_membrane_pass_two.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_membrane_pass_two-7a85a72ebf8dcd25)

running 4 tests
test a_node_is_not_merged_into_itself ... ok
test a_type_the_ontology_already_declares_is_not_declared_again ... ok
test a_proposer_cannot_mark_its_own_assertion_accepted ... ok
test an_assertion_naming_a_type_the_ontology_does_not_declare_is_refused ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_07.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_p1_07-d7562ee04b18eba3)

running 5 tests
test inherited_opaque_constraints_are_refused_on_child_node_properties ... ok
test retracting_a_retained_edge_assertion_does_not_erase_its_reference ... ok
test cancellation_cannot_leave_a_new_assertion_referring_to_the_cancelled_edge ... ok
test independent_property_and_lifecycle_writes_are_order_invariant ... ok
test assertion_subject_predicate_object_cross_product_obeys_declarations ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p1_08_seed.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_p1_08_seed-b13a50007eca94e0)

running 4 tests
test seed_ontology_property_definitions_must_be_filed_under_their_own_ids ... ok
test inherited_record_properties_remain_typed_and_constraints_cannot_disappear ... ok
test losing_cached_seed_keeps_its_original_retention_class ... ok
test persisted_seed_format_and_context_fields_refuse_before_admission ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.70s

     Running tests/commit_path.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/commit_path-816fb6c18858336f)

running 5 tests
test a_commit_into_an_unseeded_lineage_is_refused ... ok
test the_same_lineage_written_by_hand_does_not_advance_anything ... ok
test the_kernels_authority_stands_behind_exactly_what_it_validated ... ok
test a_validated_transaction_commits_and_the_lineage_advances ... ok
test a_transaction_validated_against_a_revision_the_lineage_moved_past_is_refused ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/encoding_field_order.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/encoding_field_order-2a7ec28608d4a18e)

running 1 test
test every_field_of_the_kernels_encodings_is_written_in_declaration_order ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/legacy.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/legacy-1ef43957b620c981)

running 13 tests
test all_six_original_event_discriminants_and_fields_are_fixed ... ok
test original_revision_root_fields_are_fixed ... ok
test user_record_discriminator_keys_survive_and_floats_do_not_gain_an_encoding ... ok
test all_original_operations_include_transitive_ontology_and_assertion_bytes ... ok
test seed_envelope_reproduces_actual_original_bootstrap_attribution ... ok
test historical_claims_require_the_supplied_bytes_and_revision ... ok
test every_original_combined_assertion_state_and_evidence_source_is_fixed ... ok
test original_value_scalar_node_and_edge_bytes_are_fixed ... ok
test withdrawn_legacy_assertions_never_acquire_invented_acceptance_history ... ok
test original_transaction_validation_uses_payload_domain_and_ordered_operations ... ok
test frozen_contracts_refuse_unknown_fields_and_duplicate_set_members ... ok
test missing_and_corrupt_original_evidence_are_named_refusals ... ok
test original_formats_unknown_fields_and_duplicate_semantic_identities_refuse ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/seed.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/seed-db0d520306fccdc5)

running 25 tests
test legitimate_record_keys_remain_data_in_strict_seed_decoding ... ok
test seed_decoding_refuses_unknown_semantic_fields_at_every_record_boundary ... ok
test a_seed_with_an_undeclared_type_is_refused_by_both_backends ... ok
test a_seed_ontology_cannot_claim_a_later_version ... ok
test a_seed_ontology_cannot_claim_a_predecessor ... ok
test a_seed_with_a_dangling_edge_is_refused_by_both_backends ... ok
test a_seed_with_a_caller_verdict_is_refused_by_both_backends ... ok
test empty_bootstrap_does_not_make_empty_transactions_valid ... ok
test a_valid_seed_has_a_positive_control ... ok
test repeated_initialization_preserves_the_lineage_and_writes_no_second_object ... ok
test concurrent_independent_handles_publish_exactly_one_seed_and_no_losing_object ... ok
test reopen_checks_full_ontology_and_execution_context ... ok
test initialization_reuses_exact_cached_seed_bytes_atomically ... ok
test required_properties_and_unsupported_constraints_keep_their_refusals ... ok
test the_versioned_minimal_yaml_fixture_initializes_both_real_providers ... ok
test seed_multiplicity_and_property_types_are_checked ... ok
test named_seed_refusals_write_neither_the_object_nor_the_revision ... ok
test actual_bootstrap_identities_refuse_self_validation_and_false_attribution ... ok
test seed_lifecycle_is_the_declared_initial_state_and_is_preserved ... ok
test an_evidence_seed_reopens_with_identical_roots_fields_and_retained_bytes ... ok
test floats_are_refused_in_seed_nodes_edges_and_assertion_objects ... ok
test canonical_seed_root_filing_schema_and_genesis_are_mandatory ... ok
test legacy_and_tampered_seed_envelopes_are_preserved_but_never_admitted ... ok
test seed_support_checks_missing_uncited_and_unsupported_evidence ... ok
test every_seed_entity_map_checks_key_and_root_identity ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.08s

     Running tests/validate_properties.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/validate_properties-bebd2f6d483597a0)

running 5 tests
test the_two_walks_of_one_rule_agree ... ok
test no_transaction_is_ever_validated_by_its_own_proposer ... ok
test a_float_anywhere_is_always_refused ... ok
test validating_twice_gives_the_same_answer ... ok
test the_encoding_separates_transactions_that_differ ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/validation.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/validation-09bc7f3eafde1f2a)

running 49 tests
test a_node_of_an_abstract_type_is_refused ... ok
test a_canonical_assertion_without_evidence_is_refused_by_the_provenance_validator ... ok
test a_transaction_with_no_operations_is_refused_by_the_structural_validator ... ok
test a_merge_names_two_nodes ... ok
test a_reference_to_something_the_same_transaction_creates_resolves ... ok
test a_record_under_another_graph_root_is_refused ... ok
test a_value_canonical_state_does_not_admit_is_refused_with_its_path ... ok
test a_value_that_fails_its_type_is_refused_by_the_type_validator ... ok
test a_transaction_whose_validator_is_its_proposer_is_refused ... ok
test a_valid_transaction_validates ... ok
test a_valid_transaction_validates_to_the_same_hash_twice ... ok
test an_edge_endpoint_of_the_wrong_type_is_refused ... ok
test an_edge_cardinality_the_type_forbids_is_refused_by_the_cardinality_validator ... ok
test an_empty_property_assignment_still_requires_a_declared_property ... ok
test an_evidence_id_the_transaction_declares_does_not_make_it_exist ... ok
test a_type_the_ontology_already_declares_is_not_declared_again ... ok
test an_invoke_the_lifecycle_does_not_declare_is_refused_by_the_ontology_validator ... ok
test an_operation_the_type_does_not_declare_is_refused ... ok
test an_unresolvable_reference_is_refused_by_the_reference_validator ... ok
test an_identity_created_twice_in_one_transaction_is_refused ... ok
test competing_lifecycle_writes_are_refused ... ok
test an_unresolved_assertion_endpoint_does_not_acquire_an_unrelated_type_refusal ... ok
test an_invocation_carries_exactly_the_arguments_its_operation_declares ... ok
test deleting_an_edge_referenced_by_a_new_or_retained_assertion_is_refused ... ok
test an_assertion_cannot_arrive_carrying_its_own_verdict ... ok
test opaque_edge_property_constraints_refuse_creation_and_assertions ... ok
test an_identity_canonical_state_already_holds_is_not_created_again ... ok
test an_assertions_types_are_resolved_whatever_shape_its_object_has ... ok
test every_type_refusal_the_validator_can_make_is_reachable ... ok
test applicable_opaque_property_constraints_refuse_all_node_write_paths ... ok
test opaque_preconditions_and_emissions_are_not_silently_accepted ... ok
test property_cardinality_uses_the_candidate_node ... ok
test every_field_of_every_encoded_type_reaches_its_encoding ... ok
test competing_property_writes_are_refused_in_every_order ... ok
test relation_assertions_check_both_endpoint_types_and_allow_inherited_types ... ok
test every_kind_of_dangling_reference_is_refused ... ok
test property_assertions_check_type_objects_edge_properties_and_type_subjects ... ok
test property_cardinality_and_required_presence_are_refused ... ok
test relation_assertions_require_compatible_node_endpoints ... ok
test the_encoding_writes_id_bearing_fields_in_declaration_order ... ok
test the_pipeline_runs_the_seven_deterministic_validators_in_order ... ok
test the_declared_evidence_set_is_the_evidence_the_assertions_cite ... ok
test the_eleven_operation_numbers_are_the_domains_and_the_declarations ... ok
test a_serialized_lifecycle_invokes_but_unsupported_sets_refuse_at_decode ... ok
test two_transactions_that_differ_validate_to_different_hashes ... ok
test unsupported_schema_changes_and_merges_refuse_explicitly ... ok
test the_validation_hash_covers_the_revision_it_was_validated_against ... ok
test every_issue_code_the_kernel_can_raise_is_raised_by_a_case ... ok
    Checking ekr-kernel-tests v0.0.0 (<cache>/b10x-target/ekr-p1-09-format-20260922/tests/trybuild/ekr-kernel)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s


test tests/compile_fail/validated_transaction_cannot_be_deserialised.rs ... ok
test tests/compile_fail/validated_transaction_has_no_constructor_outside_the_kernel.rs ... ok


test only_the_kernel_constructs_a_validated_transaction ... ok

test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s

     Running unittests src/lib.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/ekr_store-faf645b78e873b91)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_event_vocabulary.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary2_event_vocabulary-5c08c3becd8c2526)

running 1 test
test a_second_rejection_of_a_re_proposed_transaction_is_not_swallowed_as_a_retry ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running tests/adversary2_retention_event_contract.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary2_retention_event_contract-a82a827ca939b61d)

running 2 tests
test the_store_domain_declares_the_retention_raise_event_this_crate_writes ... ok
test the_retention_raise_in_stored_bytes_carries_the_fields_the_domain_declares ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/adversary_objects_and_append.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_objects_and_append-4988f0c4b1471b34)

running 2 tests
test storing_canonical_bytes_that_were_cached_earlier_records_them_as_canonical ... ok
test retrying_an_append_writes_the_same_event_once_rather_than_twice ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running tests/adversary_p1_06_reference_from_bytes.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/adversary_p1_06_reference_from_bytes-720826f958dc140b)

running 1 test
test a_store_cannot_turn_dangling_document_bytes_into_canonical_state ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/domain_projection.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/domain_projection-26a37c3aca18a7fa)

running 7 tests
test the_derived_ordering_is_not_the_retention_ordering ... ok
test the_retention_ladder_is_the_one_section_thirty_seven_describes ... ok
test the_strongest_of_two_classes_does_not_depend_on_which_arrived_first ... ok
test every_storage_class_has_its_own_retention_rank ... ok
test the_storage_classes_are_the_ones_the_domain_declares ... ok
test every_event_the_crate_writes_is_declared_by_the_domain ... ok
test every_event_the_crate_writes_carries_the_fields_the_domain_declares ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/fold_rules.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/fold_rules-f25e2bdb78fb33f7)

running 18 tests
test evidence_is_addressed_apart_from_knowledge ... ok
test the_knowledge_root_is_a_function_of_the_graph_state ... ok
test an_empty_log_has_no_head_and_does_not_fold ... ok
test a_seed_naming_bytes_the_store_does_not_hold_is_refused ... ok
test a_lineage_that_does_not_begin_at_a_seed_is_refused ... ok
test a_commit_of_a_transaction_that_was_never_validated_is_refused ... ok
test validating_a_transaction_that_was_never_proposed_is_refused ... ok
test a_revision_that_does_not_follow_its_parent_is_refused ... ok
test a_commit_publishing_a_knowledge_root_the_fold_does_not_reach_is_refused ... ok
test a_commit_whose_validation_the_authority_stands_behind_advances_the_lineage ... ok
test a_store_with_no_commit_authority_refuses_to_say_what_canonical_state_is ... ok
test two_of_the_five_sub_roots_are_the_placeholder_and_not_derived ... ok
test a_commit_whose_validation_the_authority_does_not_stand_behind_does_not_advance_the_lineage ... ok
test a_rejected_transaction_cannot_then_commit ... ok
test a_second_seed_is_refused ... ok
test a_transaction_that_went_stale_cannot_then_commit ... ok
test a_commit_validated_against_a_revision_the_lineage_has_moved_past_does_not_advance_it ... ok
test two_stores_seeded_from_different_state_have_different_heads ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s

     Running tests/legacy.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/legacy-b369c36c4f7eb88e)

running 4 tests
test all_user_record_keys_are_preserved_when_unique ... ok
test duplicate_keys_are_named_refusals_even_inside_user_records ... ok
test original_document_fixture_verifies_exact_payload_bytes ... ok
test original_documents_refuse_unknown_fields_and_misfiled_identities ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/membrane_boundary.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/membrane_boundary-4fd8b8cc0e26267e)

running 1 test
test a_candidate_node_and_a_canonical_node_write_the_same_document ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/providers.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/providers-92620baa01a15f37)

running 15 tests
test a_file_store_stores_identical_bytes_once ... ok
test sqlite_stores_identical_bytes_once ... ok
test a_cache_write_after_a_canonical_one_leaves_the_bytes_canonical ... ok
test a_stored_object_carries_the_class_it_was_written_under ... ok
test storing_a_graph_is_storing_its_document ... ok
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

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s

     Running tests/review_p1_invariant_one_at_the_store.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/review_p1_invariant_one_at_the_store-82651e84b28006c4)

running 2 tests
test a_commit_lands_with_no_validated_transaction_anywhere_in_the_process ... ok
test a_commit_validated_against_a_revision_that_is_no_longer_the_head_replays_as_valid ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running tests/seed_object_integrity.rs (<cache>/b10x-target/ekr-p1-09-format-20260922/debug/deps/seed_object_integrity-d7384f3e5473b860)

running 2 tests
test sqlite_seed_objects_verify_address_length_and_retention_before_admission ... ok
test file_seed_objects_verify_address_length_and_retention_before_admission ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.67s

   Doc-tests ekr_graph

running 1 test
test crates/ekr-graph/src/lib.rs - (line 57) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

   Doc-tests ekr_kernel

running 2 tests
test crates/ekr-kernel/src/commit.rs - commit::Commit<S>::over (line 180) ... ok
test crates/ekr-kernel/src/lib.rs - (line 38) ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

   Doc-tests ekr_store

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


```

Additional checks:
- `cargo clippy -p ekr-graph -p ekr-store -p ekr-kernel --all-targets -- -D warnings`: exit 0, clippy.log.
- `cargo fmt --all --check`: exit 0, fmt.log.
- `git diff --check`: exit 0.

Existing package tests use their disposable provider fixtures. The new adversary cases themselves use no provider, and no live source or operator store was inspected. This is a three-package verification lane, not the full integrated task check or a migration-completeness claim.

## 4. Result and limits

No source findings remain in this bounded pass. The code and observed cases preserve supplied original byte addresses, original ordered operation encoding, the historical payload-domain validation construction, transitive frozen declaration/assertion encoding, and the actual captured Proposed-to-Accepted attribution. Unknown nested semantics and duplicate textual keys refuse without turning generic user Record names into reserved fields.

The seed helper's result is plain historical graph data. It checks the stated original attribution/context envelope but does not rerun full historical deterministic admission or authenticate the named identities. The helper and report explicitly limit that claim. A verified payload/value address similarly proves neither full history nor the availability of omitted operations/receipts/ontology. No new future format, history reconstruction or live-provider requirement was imposed during this review.

No source mutation was made, including temporarily. The mutation probes change only values inside the added tests. No fixture expectation was regenerated from current encoders. This report offers observed outcomes, not approval or a proof covering unexecuted production paths.

## 5. Outside inventory and handback

Public aliases omit private machine paths. Exact paths are retained beside this report in outside-paths.txt.

- `<cache>/ekr-completion-20260922/persisted-contract/adversary/nested-first.log`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/adversary_frozen_serializers_preserve_all_original_capture_bytes.log`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/adversary_validation_receipts_refuse_identical_bytes_in_wrong_hash_domain_and_order.log`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/adversary_seed_attribution_preserves_input_and_refuses_mismatched_original_contexts.log`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/adversary_nonproposed_seed_states_never_receive_reconstructed_acceptance.log`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/adversary_duplicate_decoded_identities_and_escaped_user_keys_are_not_lost.log`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/duplicate-diagnostic.log`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/duplicate-corrected.log`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/original-production-diff.log`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/packages.log`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/clippy.log`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/fmt.log`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/adversary-report.md`
- `<cache>/ekr-completion-20260922/persisted-contract/adversary/outside-paths.txt`

Compiler, rustdoc dependencies and trybuild outputs remain under <cache>/b10x-target/ekr-p1-09-format-20260922. Temporary test fixtures stay beneath the assigned TMPDIR and use normal fixture lifetime cleanup. Worktree lease metadata was maintained only through managed hook commands for this reviewer's session, codex-ekr-legacy-adversary; that session is released at stable handback. No source/AEP/ESS/doc edit, commit, publication or managed-tree cleanup occurred.

The added test file is stable. No compiler process remains active. Coordinator integration and the combined format story's remaining prerequisites are separate work.

```findings
[]
```

