---
format: aep.planning-md/3
id: review-result:adversary-input-08-yaml-pass-1
kind: review-result
status: active
title: Input YAML adversary pass 1
relations:
- reviews: task:every-yaml-reader-is-bounded
revision: 1
---
unit: task:every-yaml-reader-is-bounded at 9b65ea399ad0b8ee920f6d923741ad6e0bc63c7f plus the tests-only diff below
verdict: NEEDS-CHANGE
cases: executed 261→266, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 16 scratch files and 4 tool-managed roots, listed below
needs-coordinator: route the source-inventory acceptance finding to the implementor; retain the combined integration gate

```text
 crates/ekr-core/tests/decode.rs            | 57 ++++++++++++++++++++++++++++++
 crates/ekr-ontology/tests/ontology_load.rs | 34 ++++++++++++++++++
 crates/ekr-ontology/tests/yaml_ingress.rs  | 20 +++++++++++
 3 files changed, 111 insertions(+)
```

Owners: 1 findings, 0 coordinator, 1 implementor.

This is adversary pass 1, covering the exact candidate above against base bffda70c92aed99cdcef6742dd647026460b092a. The before count is the implementing report's completed core 128 plus ontology 133 cases, including doctests. No preemptive baseline suite was run. The after count is measured from the complete core and ontology run: 265 passed, 1 failed, 0 ignored. The implementor's separate kernel 548 passing cases and 6 ignored cases are not counted as tests executed by this pass. Host token and tool counters are unavailable. Generic host adapter; inherited model, with the skill's Sonnet model unavailable.

1. Cases were written before their first execution. All five remain as additions; no existing case was altered or disabled.

- crates/ekr-ontology/tests/yaml_ingress.rs:347 — RED. An ordinary call to the new public shared loader successfully materializes 65 nested containers (131 events) with a caller-selected depth of usize::MAX. Passing the same new-source caller to the inventory must change its classification; it does not. The first direct-call assertion fails at :362. The later alias-import variant is written but not reached after that assertion fails, so no executed result is claimed for that variant.
- crates/ekr-core/tests/decode.rs:11 — GREEN. A literal Unicode alias fixture counts 5 expanded nodes and 11 UTF-8 bytes, including keys, accepts inclusive limits, and refuses one-less node/text budgets.
- crates/ekr-core/tests/decode.rs:46 — GREEN. An alias used deeper than its anchor retains the full expanded height: depth 5 succeeds and depth 4 refuses.
- crates/ekr-ontology/tests/ontology_load.rs:687 — GREEN. A nonempty ontology with a bounded scalar alias produces the expected typed ontology; duplicate identity still produces the same semantic refusal.
- crates/ekr-ontology/tests/ontology_load.rs:715 — GREEN. Source nesting within 64 containers can expand through an alias to 65 and is refused with ontology-too-deep.

The first focused outputs follow verbatim. All Cargo test and clippy commands used this environment:

```sh
export CARGO_BUILD_JOBS=3 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export RUSTC_WRAPPER=/usr/bin/sccache RUST_TEST_THREADS=3
export CARGO_TARGET_DIR=<cache>/b10x-target/ekr-extract-07b
export TMPDIR=<cache>/ekr-next-three/tmp
```

```text
$ cargo test -p ekr-ontology --test yaml_ingress adversary_input08 -- --nocapture
   Compiling ekr-ontology v0.0.27 (<worktrees>/ekr-input-08-yaml/crates/ekr-ontology)
    Finished `test` profile [unoptimized] target(s) in 0.52s
     Running tests/yaml_ingress.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/yaml_ingress-1a84909006a56908)

running 1 test

thread 'adversary_input08_the_new_shared_loader_is_an_ingress_too' (131677) panicked at crates/ekr-ontology/tests/yaml_ingress.rs:362:9:
assertion `left != right` failed: new unclassified shared-loader ingress escaped: fn read(s: &str) { let mut d = ekr_core::decode::yaml::load(s, usize::MAX).unwrap(); d.next_document(); }
  left: {("crates/ekr-core/src/decode.rs", "serde_yaml_ng : : observation : : Event"): 1, ("crates/ekr-core/src/decode/yaml.rs", "Documents : : from_str_within_depth"): 1, ("crates/ekr-core/src/decode/yaml.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr-core/src/decode/yaml.rs", "serde_yaml_ng : : observation : : { Document , Documents , Event }"): 1, ("crates/ekr-integrate/src/extraction.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-kernel/src/checkpoint.rs", "serde_yaml_ng : : to_string"): 1, ("crates/ekr-kernel/src/document.rs", "serde_yaml_ng : : Deserializer : : from_str"): 1, ("crates/ekr-kernel/src/document.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr-kernel/src/document/checked.rs", "serde_yaml_ng : : Deserializer : : from_str"): 2, ("crates/ekr-kernel/src/document/shape.rs", "serde_yaml_ng : : { observation : : { Document , Event , Tag } , Error , }"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : from_slice"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : from_value"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : to_string"): 2, ("crates/ekr-kernel/src/seed.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-kernel/src/yaml.rs", "serde_yaml_ng : : observation : : { Document , Documents }"): 1, ("crates/ekr-ontology/src/schema.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-sdk/src/document/extraction.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-sdk/src/document/extraction.rs", "serde_yaml_ng : : Value : : Tagged"): 1, ("crates/ekr-sdk/src/document/extraction.rs", "serde_yaml_ng : : from_str"): 2, ("crates/ekr-sdk/src/document/limits.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-sdk/src/document/limits.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-sdk/src/document/mod.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr-sdk/src/document/yaml.rs", "serde_yaml_ng : : to_string"): 1, ("crates/ekr-views/src/code_names.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-views/src/code_names.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr/src/cli/resolve.rs", "Documents : : from_str"): 1, ("crates/ekr/src/cli/resolve.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr/src/cli/resolve.rs", "serde_yaml_ng : : observation : : { Documents , Event , ScalarKind }"): 1}
 right: {("crates/ekr-core/src/decode.rs", "serde_yaml_ng : : observation : : Event"): 1, ("crates/ekr-core/src/decode/yaml.rs", "Documents : : from_str_within_depth"): 1, ("crates/ekr-core/src/decode/yaml.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr-core/src/decode/yaml.rs", "serde_yaml_ng : : observation : : { Document , Documents , Event }"): 1, ("crates/ekr-integrate/src/extraction.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-kernel/src/checkpoint.rs", "serde_yaml_ng : : to_string"): 1, ("crates/ekr-kernel/src/document.rs", "serde_yaml_ng : : Deserializer : : from_str"): 1, ("crates/ekr-kernel/src/document.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr-kernel/src/document/checked.rs", "serde_yaml_ng : : Deserializer : : from_str"): 2, ("crates/ekr-kernel/src/document/shape.rs", "serde_yaml_ng : : { observation : : { Document , Event , Tag } , Error , }"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : from_slice"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : from_value"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : to_string"): 2, ("crates/ekr-kernel/src/seed.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-kernel/src/yaml.rs", "serde_yaml_ng : : observation : : { Document , Documents }"): 1, ("crates/ekr-ontology/src/schema.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-sdk/src/document/extraction.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-sdk/src/document/extraction.rs", "serde_yaml_ng : : Value : : Tagged"): 1, ("crates/ekr-sdk/src/document/extraction.rs", "serde_yaml_ng : : from_str"): 2, ("crates/ekr-sdk/src/document/limits.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-sdk/src/document/limits.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-sdk/src/document/mod.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr-sdk/src/document/yaml.rs", "serde_yaml_ng : : to_string"): 1, ("crates/ekr-views/src/code_names.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-views/src/code_names.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr/src/cli/resolve.rs", "Documents : : from_str"): 1, ("crates/ekr/src/cli/resolve.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr/src/cli/resolve.rs", "serde_yaml_ng : : observation : : { Documents , Event , ScalarKind }"): 1}
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test adversary_input08_the_new_shared_loader_is_an_ingress_too ... FAILED

failures:

failures:
    adversary_input08_the_new_shared_loader_is_an_ingress_too

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr-ontology --test yaml_ingress`
exit: 101

$ cargo test -p ekr-core --test decode adversary_input08 -- --nocapture
   Compiling ekr-core v0.0.27 (<worktrees>/ekr-input-08-yaml/crates/ekr-core)
    Finished `test` profile [unoptimized] target(s) in 0.31s
     Running tests/decode.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/decode-9ce12c1de55e4549)

running 2 tests
test adversary_input08_alias_accounting_counts_utf8_and_keys_at_inclusive_bounds ... ok
test adversary_input08_an_alias_at_a_deeper_site_counts_its_whole_height ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s

exit: 0

$ cargo test -p ekr-ontology --test ontology_load adversary_input08 -- --nocapture
   Compiling ekr-ontology v0.0.27 (<worktrees>/ekr-input-08-yaml/crates/ekr-ontology)
    Finished `test` profile [unoptimized] target(s) in 0.41s
     Running tests/ontology_load.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/ontology_load-fcd4a3597da12997)

running 2 tests
test adversary_input08_an_alias_can_exceed_depth_without_deep_source_nesting ... ok
test adversary_input08_aliases_preserve_nonempty_ontology_and_semantic_refusals ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 21 filtered out; finished in 0.00s

exit: 0
```

2. The complete affected core and ontology suite ran after every focused case. The unchanged kernel suite was not repeated, per coordinator direction after the observed inventory failure; the implementor already completed it, and this pass changed tests only. A combined integration gate remains required.

```text
$ cargo test -p ekr-core -p ekr-ontology --no-fail-fast
   Compiling ekr-ontology v0.0.27 (<worktrees>/ekr-input-08-yaml/crates/ekr-ontology)
   Compiling ekr-core v0.0.27 (<worktrees>/ekr-input-08-yaml/crates/ekr-core)
    Finished `test` profile [unoptimized] target(s) in 2.85s
     Running unittests src/lib.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/ekr_core-00091f378fbcd48f)

running 7 tests
test decode::yaml::tests::a_nested_alias_bomb_is_refused_by_count_without_expanding ... ok
test decode::yaml::tests::an_alias_counts_as_what_it_repeats_and_is_never_re_walked ... ok
test decode::yaml::tests::an_alias_inside_its_own_anchor_is_recursive ... ok
test decode::yaml::tests::depth_is_counted_at_the_cut_and_through_an_alias ... ok
test bytes::tests::round_trips_every_length_and_refuses_every_other_spelling ... ok
test canonical::tests::hashing_a_value_holds_a_bounded_window_of_its_encoding ... ok
test hash::tests::the_streamed_address_is_the_address_over_the_whole_encoding ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/adversary2_encoding_vector.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/adversary2_encoding_vector-eeceb73c533afe74)

running 6 tests
test a_map_with_a_repeated_key_encodes_the_same_whatever_order_it_is_handed ... ok
test a_content_address_over_the_remaining_tags_matches_its_published_digest ... ok
test a_content_hash_encodes_as_its_published_bytes ... ok
test a_raw_payload_never_shares_an_address_with_a_canonical_value ... ok
test an_id_encodes_as_its_published_bytes ... ok
test the_remaining_tags_match_their_published_bytes ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_public_surface.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/adversary2_public_surface-f8a9e986424ec50a)

running 1 test
test every_public_item_is_used_by_a_case_and_not_only_named_in_prose ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_revision_number_text_form.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/adversary2_revision_number_text_form-61105e400275943d)

running 3 tests
test a_leading_zero_is_refused_at_every_length ... ok
test the_two_reading_paths_agree_about_what_a_revision_number_is ... ok
test serde_reads_the_number_it_writes_and_no_other_spelling_of_it ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_encoder_order.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/adversary_encoder_order-d12b4aa0caa9728b)

running 3 tests
test a_set_encodes_in_sorted_order_whatever_order_it_is_handed ... ok
test a_map_encodes_in_key_order_whatever_order_it_is_handed ... ok
test the_btreemap_path_is_not_what_makes_the_encoding_ordered ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_encoding_vector.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/adversary_encoding_vector-9b58ed9b41030c28)

running 3 tests
test a_content_address_matches_its_published_digest ... ok
test to_hex_is_the_form_from_str_reads ... ok
test the_canonical_encoding_matches_its_published_bytes ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_id_text_form.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/adversary_id_text_form-2493ff83b058a312)

running 3 tests
test a_second_spelling_does_not_round_trip_to_itself ... ok
test serde_reads_the_form_it_writes_and_no_other ... ok
test an_id_parses_from_the_form_it_writes_and_from_no_other ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_perf02_streaming_hash.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/adversary_perf02_streaming_hash-5bf7acddb13b6331)

running 3 tests
test the_address_is_the_digest_over_canonical_bytes_for_every_implementation - should panic ... ok
test repeated_keys_with_values_larger_than_the_window_order_by_value ... ok
test every_container_path_encodes_and_streams_to_the_documented_bytes ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/bytes.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/bytes-4fff277199dde2a0)

running 4 tests
test a_reader_learns_the_spelling_and_refuses_a_loose_one ... ok
test the_serde_module_writes_and_reads_the_one_string ... ok
test base64_is_a_third_larger_and_a_number_array_several_times_larger ... ok
test every_byte_string_round_trips_through_its_one_spelling ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/canonical_encoding.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/canonical_encoding-718dbdcc8accfd81)

running 10 tests
test a_type_is_part_of_the_encoding ... ok
test a_variant_marker_is_its_own_shape ... ok
test a_map_that_differs_in_one_value_encodes_differently ... ok
test field_boundaries_are_not_ambiguous ... ok
test list_order_is_meaning_and_survives ... ok
test the_primitives_of_this_crate_encode ... ok
test two_btreemaps_with_the_same_entries_encode_to_the_same_bytes ... ok
test two_variants_with_byte_identical_payloads_encode_differently ... ok
test encoding_distinguishes_exactly_what_equality_does ... ok
test insertion_order_never_reaches_the_bytes ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/content_hash.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/content_hash-389be073b2ed5eb7)

running 10 tests
test a_malformed_hex_form_is_refused ... ok
test a_value_and_a_payload_are_addressed_in_different_domains ... ok
test formatting_a_hash_emits_one_complete_hex_string ... ok
test no_payload_can_reach_a_value_address ... ok
test any_hash_round_trips_through_its_hex ... ok
test the_hash_is_sha256_of_its_domain_and_the_bytes_it_is_given ... ok
test the_hex_form_round_trips ... ok
test the_same_canonical_bytes_hash_the_same ... ok
test hash_display_preserves_text_flags_and_propagates_writer_errors ... ok
test the_address_is_a_function_of_the_content ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/decode.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/decode-9ce12c1de55e4549)

running 11 tests
test a_duplicate_id_is_refused_before_its_invalid_second_value_is_decoded ... ok
test adversary_input08_alias_accounting_counts_utf8_and_keys_at_inclusive_bounds ... ok
test adversary_input08_an_alias_at_a_deeper_site_counts_its_whole_height ... ok
test distinct_set_members_and_the_empty_set_decode_without_losing_values ... ok
test duplicate_set_identity_is_refused_instead_of_discarded ... ok
test set_uniqueness_uses_decoded_values_instead_of_input_spelling ... ok
test distinct_typed_keys_and_empty_maps_decode_normally ... ok
test strings_decode_only_strings ... ok
test shared_yaml_observation_bounds_loading_and_counts_aliases_across_documents ... ok
test the_bounded_yaml_observation_refuses_each_bound_by_name ... ok
test uniqueness_uses_the_decoded_key_type_not_its_input_spelling ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/hashing_encoder_contract.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/hashing_encoder_contract-cbdc3513ce83c14c)

running 4 tests
test finishing_a_hashing_encoder_is_refused - should panic ... ok
test replacing_the_encoder_addresses_what_is_left_in_its_place ... ok
test an_encoder_reads_the_same_through_debug_on_both ... ok
test swapping_an_encoder_there_and_back_changes_no_address ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/identity_serde.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/identity_serde-ef9f83f67186804f)

running 42 tests
test agent_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test assertion_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test adversary_i_macro_doc_counts_the_id_newtypes_it_declares ... ok
test attachment_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test agent_id::round_trips_for_any_bits ... ok
test edge_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test assertion_id::round_trips_for_any_bits ... ok
test attachment_id::round_trips_for_any_bits ... ok
test event_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test event_id::round_trips_for_any_bits ... ok
test evidence_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test edge_id::round_trips_for_any_bits ... ok
test graph_root_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test evidence_id::round_trips_for_any_bits ... ok
test issue_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test graph_root_id::round_trips_for_any_bits ... ok
test merge_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test issue_id::round_trips_for_any_bits ... ok
test node_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test merge_id::round_trips_for_any_bits ... ok
test observation_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test node_id::round_trips_for_any_bits ... ok
test property_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test every_ess_id_type_exists_in_the_crate ... ok
test revision_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test observation_id::round_trips_for_any_bits ... ok
test revision_number::round_trips_as_a_number_and_refuses_a_malformed_one ... ok
test revision_number::round_trips_for_any_number ... ok
test revision_number_text_form::a_second_spelling_of_a_number_is_refused ... ok
test revision_number_text_form::the_written_form_is_the_only_form ... ok
test property_id::round_trips_for_any_bits ... ok
test schema_version_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test split_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test revision_id::round_trips_for_any_bits ... ok
test support_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test schema_version_id::round_trips_for_any_bits ... ok
test transaction_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test split_id::round_trips_for_any_bits ... ok
test type_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test support_id::round_trips_for_any_bits ... ok
test transaction_id::round_trips_for_any_bits ... ok
test type_id::round_trips_for_any_bits ... ok

test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/public_surface.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/public_surface-6c0d5f3499f988bd)

running 5 tests
test a_refusal_carries_the_text_it_refused ... ok
test a_revision_number_counts_from_the_seed_and_stops_at_the_end ... ok
test an_id_carries_its_bits_through_uuid_and_back ... ok
test the_encoder_writes_each_shape_it_publishes ... ok
test no_public_item_is_untested ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/rename_stability.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/rename_stability-f4f0c621f5f2c2e4)

running 5 tests
test no_id_type_mints_a_constant ... ok
test renaming_two_things_to_one_name_does_not_merge_them ... ok
test the_same_name_twice_is_two_identities ... ok
test a_burst_of_mints_is_all_distinct ... ok
test two_values_that_share_a_name_never_share_an_id ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/sha256_backend.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/sha256_backend-22681c8ff89ee53d)

running 1 test
test payload_and_streamed_value_digests_keep_every_boundary ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running tests/timestamp.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/timestamp-06a898b0a382a192)

running 7 tests
test a_timestamp_crosses_serde_as_the_number_it_is ... ok
test a_refusal_carries_the_text_and_nothing_else ... ok
test a_timestamp_encodes_as_the_integer_it_wraps ... ok
test a_timestamp_is_milliseconds_since_the_unix_epoch ... ok
test one_value_has_one_text ... ok
test the_range_is_far_wider_than_nanoseconds_would_give ... ok
test the_text_form_refuses_every_other_spelling ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/ekr_ontology-395eb885ce76984d)

running 1 test
test schema::bounded_yaml::ontology_yaml_stops_loading_at_the_depth_cut ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_p5_01_ontology.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/adversary2_p5_01_ontology-0877c8d000e7cc0b)

running 3 tests
test narrowing_a_reference_from_an_abstract_type_to_its_only_concrete_subtype_admits_every_value ... ok
test the_soundness_property_catches_a_state_blind_to_its_value_kinds ... ok
test a_redeclaration_found_compatible_leaves_every_node_valid_under_the_next_version ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.12s

     Running tests/adversary_decoder.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/adversary_decoder-97aee6367c0f5525)

running 3 tests
test unknown_value_members_are_refused_beneath_a_record_key_and_list_element ... ok
test record_keys_named_like_semantic_members_survive_nested_roundtrips ... ok
test every_nested_ontology_set_and_map_refuses_duplicate_decoded_members ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_p5_01_ontology.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/adversary_p5_01_ontology-f1ba8d003e7dc092)

running 3 tests
test a_change_list_that_changes_nothing_is_not_a_next_version ... ok
test a_parent_dropped_by_an_abstract_type_moves_the_conformance_of_its_concrete_children ... ok
test a_constraint_added_to_a_property_instances_hold_is_not_passed_as_compatible ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/canonical_admissibility.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/canonical_admissibility-97edf19775777dd5)

running 6 tests
test a_bare_float_is_refused_and_the_path_is_the_value_itself ... ok
test a_float_inside_a_compound_is_refused_at_the_path_it_sits_at ... ok
test every_kind_but_a_float_is_admissible_on_its_own ... ok
test a_path_reads_from_the_whole_value_down_to_the_part ... ok
test the_first_refused_value_in_the_values_own_order_is_the_one_named ... ok
test nesting_does_not_hide_a_float_at_any_depth ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/document_round_trip.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/document_round_trip-95ed626d5e7b7b27)

running 3 tests
test an_empty_seed_writes_back_only_its_version ... ok
test a_loaded_ontology_writes_back_every_declaration_in_identity_order ... ok
test the_written_document_reloads_to_the_same_ontology_and_is_a_fixed_point ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/domain_projection.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/domain_projection-824dfddeab7c26ee)

running 4 tests
test a_timestamp_value_carries_a_timestamp ... ok
test every_timestamp_the_domain_declares_is_carried_as_a_timestamp ... ok
test the_domain_and_current_codec_retain_all_recursive_compound_parameters ... ok
test every_value_kind_the_domain_declares_is_the_crates_and_has_its_carrier ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/hierarchy_specificity.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/hierarchy_specificity-78064d3bea8d0e2a)

running 7 tests
test a_hierarchy_fault_is_reported_before_a_property_resolution_fault ... ok
test a_parent_that_specialises_another_parent_is_not_an_ambiguous_declaration ... ok
test the_ambiguity_refusal_only_fires_when_neither_declaring_type_is_an_ancestor_of_the_other ... ok
test a_redeclaration_is_never_overridden_by_the_declaration_it_redeclares ... ok
test the_reason_a_two_fault_document_is_refused_does_not_depend_on_id_order ... ok
test the_checker_resolves_allowed_types_by_the_same_order ... ok
test a_declaration_is_chosen_by_the_specialisation_order_and_by_nothing_else ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/inheritance_and_declaration_coherence.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/inheritance_and_declaration_coherence-0c1b7f48b9393e3c)

running 14 tests
test an_operation_argument_naming_an_undeclared_type_is_refused_at_load ... ok
test a_redeclared_property_is_resolved_by_the_hierarchy_and_not_by_type_id_order ... ok
test a_single_fault_hierarchy_is_refused_for_that_fault_whatever_the_id_order ... ok
test an_undeclared_grandparent_is_refused_as_an_unknown_type_and_not_as_a_cycle ... ok
test an_operation_argument_no_value_inhabits_is_refused_at_load ... ok
test every_position_a_value_type_is_declared_in_is_walked_for_inhabitability ... ok
test property_resolution_is_a_function_of_the_hierarchy_and_never_of_id_order ... ok
test the_nearest_declaration_wins_at_every_depth ... ok
test the_public_node_type_index_distinguishes_present_and_absent_nodes ... ok
test two_ancestors_at_one_distance_declaring_a_property_differently_are_refused_at_load ... ok
test two_ancestors_declaring_one_property_identically_are_not_ambiguous ... ok
test the_crate_declares_no_value_type_field_the_walk_does_not_reach ... ok
test every_domain_name_this_crate_cites_is_declared_by_the_domain ... ok
test value_rs_does_not_attribute_its_serde_shape_to_the_ess_domain ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/lifecycle_transitions.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/lifecycle_transitions-b4239e25c301f93b)

running 8 tests
test a_move_from_a_state_the_lifecycle_does_not_have_is_refused ... ok
test a_lifecycle_names_its_initial_state_among_its_states ... ok
test a_move_from_a_state_the_node_is_not_in_is_refused ... ok
test an_operation_that_declares_no_transition_leaves_the_state_alone ... ok
test declares_answers_for_the_pair_and_not_for_its_endpoints ... ok
test every_declared_transition_is_accepted ... ok
test preconditions_are_carried_as_opaque_text_and_refuse_nothing ... ok
test a_move_is_accepted_exactly_when_the_lifecycle_declares_it ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ontology_adversary_edge_type_widen_p2e.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/ontology_adversary_edge_type_widen_p2e-f140e8d48d95c048)

running 2 tests
test a_source_end_replaced_rather_than_widened_is_a_declaration_change_under_edges ... ok
test an_end_gained_beside_any_other_declaration_change_is_still_a_declaration_change ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ontology_load.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/ontology_load-fcd4a3597da12997)

running 23 tests
test a_cycle_in_the_parent_graph_is_refused_at_load ... ok
test a_lifecycle_naming_a_state_it_does_not_have_is_refused_at_load ... ok
test a_list_and_an_empty_record_are_not_in_that_class ... ok
test a_node_ref_to_a_type_the_ontology_does_not_declare_is_refused_at_load ... ok
test a_node_ref_with_no_allowed_types_is_refused_at_load ... ok
test a_parent_the_ontology_does_not_declare_is_refused_at_load ... ok
test a_type_declared_twice_is_refused_at_load ... ok
test a_node_property_definition_filed_under_another_id_is_refused_at_load ... ok
test adversary_input08_an_alias_can_exceed_depth_without_deep_source_nesting ... ok
test an_edge_type_with_no_source_or_no_target_types_is_refused_at_load ... ok
test an_empty_compound_value_type_is_refused_at_every_depth_it_is_declared ... ok
test adversary_input08_aliases_preserve_nonempty_ontology_and_semantic_refusals ... ok
test an_operation_whose_move_the_lifecycle_does_not_declare_is_refused_at_load ... ok
test an_edge_property_definition_filed_under_another_id_is_refused_at_load ... ok
test an_operation_with_a_transition_and_no_lifecycle_at_all_is_refused_at_load ... ok
test an_ontology_loads_from_yaml_and_refuses_the_same_documents ... ok
test ontology_yaml_refuses_excessive_container_depth ... ok
test ontology_yaml_preserves_bounded_aliases_and_existing_refusals ... ok
test unknown_semantic_members_of_ontology_records_are_refused ... ok
test unknown_semantics_on_compound_value_types_are_refused ... ok
test value_envelopes_do_not_discard_unknown_semantics ... ok
test ontology_yaml_bounds_expanded_alias_nodes_and_text ... ok
test ontology_yaml_refuses_input_bytes_before_decoding ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s

     Running tests/schema_evolution.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/schema_evolution-e4b10edc6d401485)

running 43 tests
test a_change_list_whose_result_declares_what_the_prior_did_is_refused ... ok
test a_cardinality_narrowed_below_what_instances_hold_is_incompatible ... ok
test a_new_required_property_on_a_type_with_instances_is_incompatible ... ok
test a_constraint_changed_on_a_property_is_incompatible_wherever_instances_resolve_it ... ok
test a_prior_at_the_largest_version_number_is_refused_rather_than_wrapped ... ok
test a_next_that_is_not_the_priors_successor_is_incompatible ... ok
test a_property_made_required_on_a_parent_reaches_the_instances_of_its_children ... ok
test a_property_made_required_on_a_type_whose_instances_lack_it_is_incompatible ... ok
test a_property_made_required_on_an_edge_type_counts_edges ... ok
test a_property_made_required_while_one_instance_of_several_lacks_it_is_incompatible ... ok
test a_redeclaration_that_makes_an_inherited_property_ambiguous_is_refused ... ok
test a_property_made_required_on_an_edge_type_some_edges_lack_is_incompatible ... ok
test a_result_that_fails_a_load_refusal_is_refused_with_that_refusal ... ok
test a_type_changed_in_more_than_its_properties_is_incompatible_while_it_has_instances ... ok
test a_type_defined_twice_in_one_change_list_is_refused ... ok
test a_same_kind_value_type_narrowed_under_held_values_is_incompatible_and_a_widening_is_not ... ok
test a_type_level_change_counts_the_instances_of_every_type_conforming_to_it ... ok
test a_value_type_change_is_refused_over_the_kind_of_an_assertion_object_alone ... ok
test a_value_type_changed_under_values_of_the_old_kind_is_incompatible ... ok
test a_reference_narrowing_is_judged_by_the_concrete_types_it_admits ... ok
test a_version_that_names_itself_as_its_parent_is_refused ... ok
test a_version_that_changes_nothing_anyone_holds_is_compatible ... ok
test a_version_that_reuses_its_grandparent_id_is_refused ... ok
test a_widening_may_name_a_node_type_defined_earlier_in_the_list ... ok
test a_widening_that_adds_nothing_is_without_effect ... ok
test a_widened_edge_type_replaces_the_prior_one_over_the_edges_it_holds ... ok
test a_widening_that_drops_a_type_from_either_end_is_refused ... ok
test an_empty_change_list_is_refused ... ok
test an_evolved_version_is_exactly_what_load_would_hold ... ok
test changes_apply_in_order_so_a_type_defined_earlier_owns_a_later_property ... ok
test define_edge_type_adds_an_edge_type_between_existing_node_types ... ok
test evolution_chains_and_each_version_names_the_one_before ... ok
test evolve_derives_the_next_version_from_the_prior_one ... ok
test modify_property_adds_a_property_and_redeclares_one ... ok
test modify_property_reaches_an_edge_type ... ok
test modifying_a_property_of_an_undeclared_owner_is_refused ... ok
test redefining_a_type_the_prior_version_declares_is_refused_for_every_kind_of_declaration ... ok
test removing_a_type_or_property_that_instances_hold_is_incompatible ... ok
test widening_an_edge_type_adds_a_declared_node_type_to_its_ends_as_the_next_version ... ok
test widening_an_edge_type_the_version_does_not_declare_is_refused ... ok
test widening_to_a_type_that_is_not_a_declared_node_type_is_refused ... ok
test the_domain_states_the_lineage_rule_and_declares_the_schema_change ... ok
test every_refusal_code_is_kebab_case_and_declared_by_the_domain ... ok

test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/type_hierarchy.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/type_hierarchy-cc2500d3407bf1c9)

running 4 tests
test a_type_conforms_to_itself_and_to_every_ancestor_and_to_nothing_else ... ok
test a_node_ref_accepts_a_node_whose_type_conforms_to_an_allowed_type ... ok
test a_types_properties_are_its_own_and_its_ancestors ... ok
test an_inherited_required_property_is_required_of_the_descendant ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/value_type_checking.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/value_type_checking-5e9de5ea3f0d7ca3)

running 8 tests
test a_value_is_checkable_against_a_type_on_its_own ... ok
test a_refusal_names_the_property_and_the_reason ... ok
test a_well_typed_value_checks_ok ... ok
test an_abstract_or_unknown_type_is_not_instantiable ... ok
test cardinality_permits_the_counts_it_names ... ok
test every_value_kind_mirrors_its_value_type ... ok
test enum_variants_and_record_fields_are_enforced_inside_a_compound_value ... ok
test a_value_breaking_exactly_one_property_of_its_declared_type_is_refused ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/yaml_ingress.rs (<cache>/b10x-target/ekr-extract-07b/debug/deps/yaml_ingress-1a84909006a56908)

running 3 tests
test adversary_input08_the_new_shared_loader_is_an_ingress_too ... FAILED
test the_yaml_inventory_rejects_new_readers_and_import_aliases ... ok
test every_production_yaml_reader_has_an_explicit_input_policy ... ok

failures:

---- adversary_input08_the_new_shared_loader_is_an_ingress_too stdout ----

thread 'adversary_input08_the_new_shared_loader_is_an_ingress_too' (165454) panicked at crates/ekr-ontology/tests/yaml_ingress.rs:362:9:
assertion `left != right` failed: new unclassified shared-loader ingress escaped: fn read(s: &str) { let mut d = ekr_core::decode::yaml::load(s, usize::MAX).unwrap(); d.next_document(); }
  left: {("crates/ekr-core/src/decode.rs", "serde_yaml_ng : : observation : : Event"): 1, ("crates/ekr-core/src/decode/yaml.rs", "Documents : : from_str_within_depth"): 1, ("crates/ekr-core/src/decode/yaml.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr-core/src/decode/yaml.rs", "serde_yaml_ng : : observation : : { Document , Documents , Event }"): 1, ("crates/ekr-integrate/src/extraction.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-kernel/src/checkpoint.rs", "serde_yaml_ng : : to_string"): 1, ("crates/ekr-kernel/src/document.rs", "serde_yaml_ng : : Deserializer : : from_str"): 1, ("crates/ekr-kernel/src/document.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr-kernel/src/document/checked.rs", "serde_yaml_ng : : Deserializer : : from_str"): 2, ("crates/ekr-kernel/src/document/shape.rs", "serde_yaml_ng : : { observation : : { Document , Event , Tag } , Error , }"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : from_slice"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : from_value"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : to_string"): 2, ("crates/ekr-kernel/src/seed.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-kernel/src/yaml.rs", "serde_yaml_ng : : observation : : { Document , Documents }"): 1, ("crates/ekr-ontology/src/schema.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-sdk/src/document/extraction.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-sdk/src/document/extraction.rs", "serde_yaml_ng : : Value : : Tagged"): 1, ("crates/ekr-sdk/src/document/extraction.rs", "serde_yaml_ng : : from_str"): 2, ("crates/ekr-sdk/src/document/limits.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-sdk/src/document/limits.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-sdk/src/document/mod.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr-sdk/src/document/yaml.rs", "serde_yaml_ng : : to_string"): 1, ("crates/ekr-views/src/code_names.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-views/src/code_names.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr/src/cli/resolve.rs", "Documents : : from_str"): 1, ("crates/ekr/src/cli/resolve.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr/src/cli/resolve.rs", "serde_yaml_ng : : observation : : { Documents , Event , ScalarKind }"): 1}
 right: {("crates/ekr-core/src/decode.rs", "serde_yaml_ng : : observation : : Event"): 1, ("crates/ekr-core/src/decode/yaml.rs", "Documents : : from_str_within_depth"): 1, ("crates/ekr-core/src/decode/yaml.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr-core/src/decode/yaml.rs", "serde_yaml_ng : : observation : : { Document , Documents , Event }"): 1, ("crates/ekr-integrate/src/extraction.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-kernel/src/checkpoint.rs", "serde_yaml_ng : : to_string"): 1, ("crates/ekr-kernel/src/document.rs", "serde_yaml_ng : : Deserializer : : from_str"): 1, ("crates/ekr-kernel/src/document.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr-kernel/src/document/checked.rs", "serde_yaml_ng : : Deserializer : : from_str"): 2, ("crates/ekr-kernel/src/document/shape.rs", "serde_yaml_ng : : { observation : : { Document , Event , Tag } , Error , }"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : from_slice"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : from_value"): 1, ("crates/ekr-kernel/src/replay.rs", "serde_yaml_ng : : to_string"): 2, ("crates/ekr-kernel/src/seed.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-kernel/src/yaml.rs", "serde_yaml_ng : : observation : : { Document , Documents }"): 1, ("crates/ekr-ontology/src/schema.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-sdk/src/document/extraction.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-sdk/src/document/extraction.rs", "serde_yaml_ng : : Value : : Tagged"): 1, ("crates/ekr-sdk/src/document/extraction.rs", "serde_yaml_ng : : from_str"): 2, ("crates/ekr-sdk/src/document/limits.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-sdk/src/document/limits.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr-sdk/src/document/mod.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr-sdk/src/document/yaml.rs", "serde_yaml_ng : : to_string"): 1, ("crates/ekr-views/src/code_names.rs", "serde_yaml_ng : : Value"): 1, ("crates/ekr-views/src/code_names.rs", "serde_yaml_ng : : from_str"): 1, ("crates/ekr/src/cli/resolve.rs", "Documents : : from_str"): 1, ("crates/ekr/src/cli/resolve.rs", "serde_yaml_ng : : Error"): 1, ("crates/ekr/src/cli/resolve.rs", "serde_yaml_ng : : observation : : { Documents , Event , ScalarKind }"): 1}
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    adversary_input08_the_new_shared_loader_is_an_ingress_too

test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

error: test failed, to rerun pass `-p ekr-ontology --test yaml_ingress`
   Doc-tests ekr_core

running 2 tests
test crates/ekr-core/src/bytes.rs - bytes (line 13) ... ok
test crates/ekr-core/src/lib.rs - (line 19) ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

   Doc-tests ekr_ontology

running 1 test
test crates/ekr-ontology/src/lib.rs - (line 24) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

error: 1 target failed:
    `-p ekr-ontology --test yaml_ingress`
exit: 101

$ cargo fmt -p ekr-core -p ekr-ontology
exit: 0

$ cargo clippy -p ekr-core -p ekr-ontology --all-targets -- -D warnings
    Checking serde_core v1.0.229
    Checking serde v1.0.229
    Checking serde_json v1.0.151
    Checking serde_yaml_ng v0.10.0 (<worktrees>/ekr-input-08-yaml/vendor/serde_yaml_ng)
    Checking ekr-core v0.0.27 (<worktrees>/ekr-input-08-yaml/crates/ekr-core)
    Checking ekr-ontology v0.0.27 (<worktrees>/ekr-input-08-yaml/crates/ekr-ontology)
    Finished `dev` profile [unoptimized] target(s) in 3.84s
exit: 0

$ cargo fmt -p ekr-core -p ekr-ontology --check
exit: 0
```

3. Finding, covering the candidate and test additions above:

| Measured location | Category / severity | Verdict / origin | Finding |
| --- | --- | --- | --- |
| crates/ekr-ontology/tests/yaml_ingress.rs:362 | acceptance / blocker | NEEDS-CHANGE / introduced | The ingress inventory ignores the new public shared YAML loader, allowing a newly added reader with caller-selected unbounded limits to remain unclassified. |

What was measured: the actual loader call succeeds and counts 131 events, then the guard's classification remains exactly unchanged when given that ordinary new-source direct call; the first focused run exits 101. The cause is the token filter at crates/ekr-ontology/tests/yaml_ingress.rs:100, which recognizes only serde_yaml_ng and Documents.

What reaches it: the public API at crates/ekr-core/src/decode/yaml.rs:18 explicitly accepts caller-selected depth. Existing production examples using the shared facade are crates/ekr-core/src/decode.rs:117 and crates/ekr-ontology/src/schema.rs:98. The reproducible acceptance counterexample models a contributor adding a new source reader through this advertised API. No existing unbounded production caller is claimed. The task explicitly requires detecting newly added unclassified outside-input YAML decoding, making this a guard-coverage defect rather than a claimed production exploit.

Origin: the candidate introduces both the public facade and this inventory guard. Reading the base with git ls-tree confirms neither crates/ekr-core/src/decode/yaml.rs nor crates/ekr-ontology/tests/yaml_ingress.rs exists there; the candidate diff adds them. No base checkout or mutation was performed. The implementor should cover the shared facade's supported call/import forms and classify current users; this pass does not fix the guard.

4. Attacked without breaking:

- Inclusive expanded-node and UTF-8 text budgets count alias values and mapping keys correctly in the executed fixture.
- Aliases at deeper sites retain their full height during expanded-depth checking.
- Nonempty bounded aliases preserve typed ontology output, and duplicate identity preserves its semantic refusal.
- Alias-expanded ontology depth is refused even when source nesting itself stays within the loader bound.

5. Outside-worktree paths written or used as writable tool roots:
- <cache>/ekr-next-three/input-08/yaml/adversary-1/first-inventory.log
- <cache>/ekr-next-three/input-08/yaml/adversary-1/first-inventory.exit
- <cache>/ekr-next-three/input-08/yaml/adversary-1/first-core.log
- <cache>/ekr-next-three/input-08/yaml/adversary-1/first-core.exit
- <cache>/ekr-next-three/input-08/yaml/adversary-1/first-ontology.log
- <cache>/ekr-next-three/input-08/yaml/adversary-1/first-ontology.exit
- <cache>/ekr-next-three/input-08/yaml/adversary-1/suite.log
- <cache>/ekr-next-three/input-08/yaml/adversary-1/suite.exit
- <cache>/ekr-next-three/input-08/yaml/adversary-1/fmt.log
- <cache>/ekr-next-three/input-08/yaml/adversary-1/fmt.exit
- <cache>/ekr-next-three/input-08/yaml/adversary-1/clippy.log
- <cache>/ekr-next-three/input-08/yaml/adversary-1/clippy.exit
- <cache>/ekr-next-three/input-08/yaml/adversary-1/fmt-check.log
- <cache>/ekr-next-three/input-08/yaml/adversary-1/fmt-check.exit
- <cache>/ekr-next-three/input-08/yaml/adversary-1/report.md
- <cache>/ekr-next-three/input-08/yaml/adversary-1/public-report.md
- <cache>/b10x-target/ekr-extract-07b — exclusive Cargo build output.
- <cache>/ekr-next-three/tmp — assigned temporary tool output.
- <cache>/sccache — compiler-cache tool state.
- <worktree-state> — managed lease tool state, session codex-input08-yaml-adversary1 only.

The coordinator's brief.md was read, not written. Disk was 33 GiB free before the package run, above the 12 GiB floor. No implementation, planning, systems or documentation file was edited; no staging, commit, push, branch change, cleanup or AEP command was performed. The owned lease ended successfully. No build is running; the exclusive target is released to the coordinator. The tests remain unstaged for coordinator handoff. Raw logs are retained unchanged; the public report substitutes home paths only.

```findings
- file: crates/ekr-ontology/tests/yaml_ingress.rs
  line: 362
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The ingress inventory ignores the new public shared YAML loader, allowing a newly added reader with caller-selected unbounded limits to remain unclassified.
```
