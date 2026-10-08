---
format: aep.planning-md/3
id: review-result:adversary-input-08-identity-pass-1
kind: review-result
status: active
title: Input identity adversary pass one
relations:
- reviews: task:identity-scan-reads-a-hand-kept-domain-list
revision: 1
---
unit: input-08 I identity, candidate e2aa1b16c196fa78bd8e5ebdbd7c67be05e57773 plus adversary test additions
verdict: green
cases: executed 129→133, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned adversary scratch, assigned target and assigned TMPDIR; exact paths below
needs-coordinator: retain and commit test-only changes; combined wave gate remains

```text
 crates/ekr-core/tests/identity_serde.rs | 145 ++++++++++++++++++++++++++++++++
 1 file changed, 145 insertions(+)
```

Owners: 0 findings, 0 coordinator, 0 implementor.

1. Added cases before running tests

Four cases in crates/ekr-core/tests/identity_serde.rs::adversary_input08 exercise the real guard through its test subprocess and the public newtypes:
- renamed_domain_files_preserve_the_identity_inventory: every current domain is renamed in a disposable fixture, with identical contents; discovery must still pass. This is reached when a contributor renames domain files.
- semantic_yaml_spelling_does_not_hide_a_new_identity: a newly added domain with an absent UUID carrier must fail for block and flow mappings with reordered and quoted fields. This is reached when a contributor adds an ESS type using either supported YAML spelling.
- removing_a_declared_identity_is_not_hidden_by_the_typed_inventory: deleting SourceCheckpointId from a copied domain must make the actual comparison fail rather than leave a stale typed inventory green. This is reached when a declared type is removed.
- public_observation_ids_keep_uuid_bits_and_canonical_text: both public newtypes preserve all-one UUID bits, serialize to literal lowercase canonical strings, refuse uppercase/unhyphenated alternatives and mint UUIDv7 values. This reaches the newly exported API directly.

The first focused run passed all four cases. The nested missing/removal fixtures intentionally make the scanner subprocess fail; these are passing negative tests, not product findings. No red case was found. After the first suite run, the renamed-file fixture was tightened to collect paths before mutating directory entries; the complete core suite was rerun afterwards.

First execution command: cargo test -p ekr-core --test identity_serde adversary_input08 -- --nocapture
Exit: 0. Raw output:
```text
   Compiling libc v0.2.189
   Compiling cfg-if v1.0.5
   Compiling proc-macro2 v1.0.107
   Compiling quote v1.0.47
   Compiling unicode-ident v1.0.26
   Compiling getrandom v0.3.4
   Compiling getrandom v0.4.3
   Compiling serde_core v1.0.229
   Compiling find-msvc-tools v0.1.14
   Compiling rustix v1.1.5
   Compiling zerocopy v0.8.59
   Compiling typenum v1.20.1
   Compiling syn v3.0.6
   Compiling shlex v2.0.1
   Compiling cc v1.5.1
   Compiling hybrid-array v0.4.15
   Compiling rand_core v0.9.5
   Compiling bitflags v2.13.2
   Compiling serde v1.0.229
   Compiling linux-raw-sys v0.12.1
   Compiling zmij v1.0.23
   Compiling autocfg v1.5.1
   Compiling ring v0.17.14
   Compiling num-traits v0.2.19
   Compiling serde_derive v1.0.229
   Compiling hashbrown v0.17.1
   Compiling fastrand v2.5.0
   Compiling serde_json v1.0.151
   Compiling thiserror v2.0.21
   Compiling equivalent v1.0.2
   Compiling once_cell v1.21.4
   Compiling itoa v1.0.18
   Compiling tempfile v3.27.0
   Compiling indexmap v2.14.2
   Compiling ppv-lite86 v0.2.21
   Compiling thiserror-impl v2.0.21
   Compiling block-buffer v0.12.1
   Compiling crypto-common v0.2.2
   Compiling getrandom v0.2.17
   Compiling wait-timeout v0.2.1
   Compiling memchr v2.8.3
   Compiling unsafe-libyaml v0.2.11
   Compiling const-oid v0.10.2
   Compiling ryu v1.0.23
   Compiling bit-vec v0.8.0
   Compiling fnv v1.0.7
   Compiling untrusted v0.9.0
   Compiling quick-error v1.2.3
   Compiling rusty-fork v0.3.1
   Compiling serde_yaml_ng v0.10.0 (<worktrees>/ekr-input-08-identity/vendor/serde_yaml_ng)
   Compiling bit-set v0.8.0
   Compiling digest v0.11.3
   Compiling rand_chacha v0.9.0
   Compiling rand_xorshift v0.4.0
   Compiling rand v0.9.5
   Compiling uuid v1.26.1
   Compiling hex v0.4.3
   Compiling regex-syntax v0.8.11
   Compiling unarray v0.1.4
   Compiling cpufeatures v0.3.1
   Compiling sha2 v0.11.0
   Compiling ekr-core v0.0.27 (<worktrees>/ekr-input-08-identity/crates/ekr-core)
   Compiling proptest v1.11.0
    Finished `test` profile [unoptimized] target(s) in 23.18s
     Running tests/identity_serde.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/identity_serde-ef9f83f67186804f)

running 4 tests
test adversary_input08::public_observation_ids_keep_uuid_bits_and_canonical_text ... ok
test adversary_input08::renamed_domain_files_preserve_the_identity_inventory ... ok
test adversary_input08::removing_a_declared_identity_is_not_hidden_by_the_typed_inventory ... ok
test adversary_input08::semantic_yaml_spelling_does_not_hide_a_new_identity ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 48 filtered out; finished in 0.06s

```

2. Final package suite

The before count of 129 is attributed to the implementor report. The final count is measured from the following test-run output, including documentation cases. No suite ran before the added cases existed.

Command: cargo test -p ekr-core
Exit: 0. Raw output:
```text
   Compiling ekr-core v0.0.27 (<worktrees>/ekr-input-08-identity/crates/ekr-core)
    Finished `test` profile [unoptimized] target(s) in 1.05s
     Running unittests src/lib.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/ekr_core-00091f378fbcd48f)

running 3 tests
test bytes::tests::round_trips_every_length_and_refuses_every_other_spelling ... ok
test canonical::tests::hashing_a_value_holds_a_bounded_window_of_its_encoding ... ok
test hash::tests::the_streamed_address_is_the_address_over_the_whole_encoding ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running tests/adversary2_encoding_vector.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/adversary2_encoding_vector-eeceb73c533afe74)

running 6 tests
test a_content_hash_encodes_as_its_published_bytes ... ok
test a_content_address_over_the_remaining_tags_matches_its_published_digest ... ok
test a_map_with_a_repeated_key_encodes_the_same_whatever_order_it_is_handed ... ok
test a_raw_payload_never_shares_an_address_with_a_canonical_value ... ok
test an_id_encodes_as_its_published_bytes ... ok
test the_remaining_tags_match_their_published_bytes ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_public_surface.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/adversary2_public_surface-f8a9e986424ec50a)

running 1 test
test every_public_item_is_used_by_a_case_and_not_only_named_in_prose ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_revision_number_text_form.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/adversary2_revision_number_text_form-61105e400275943d)

running 3 tests
test a_leading_zero_is_refused_at_every_length ... ok
test serde_reads_the_number_it_writes_and_no_other_spelling_of_it ... ok
test the_two_reading_paths_agree_about_what_a_revision_number_is ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_encoder_order.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/adversary_encoder_order-d12b4aa0caa9728b)

running 3 tests
test a_map_encodes_in_key_order_whatever_order_it_is_handed ... ok
test a_set_encodes_in_sorted_order_whatever_order_it_is_handed ... ok
test the_btreemap_path_is_not_what_makes_the_encoding_ordered ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_encoding_vector.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/adversary_encoding_vector-9b58ed9b41030c28)

running 3 tests
test a_content_address_matches_its_published_digest ... ok
test the_canonical_encoding_matches_its_published_bytes ... ok
test to_hex_is_the_form_from_str_reads ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_id_text_form.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/adversary_id_text_form-2493ff83b058a312)

running 3 tests
test an_id_parses_from_the_form_it_writes_and_from_no_other ... ok
test a_second_spelling_does_not_round_trip_to_itself ... ok
test serde_reads_the_form_it_writes_and_no_other ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_perf02_streaming_hash.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/adversary_perf02_streaming_hash-5bf7acddb13b6331)

running 3 tests
test the_address_is_the_digest_over_canonical_bytes_for_every_implementation - should panic ... ok
test repeated_keys_with_values_larger_than_the_window_order_by_value ... ok
test every_container_path_encodes_and_streams_to_the_documented_bytes ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running tests/bytes.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/bytes-4fff277199dde2a0)

running 4 tests
test a_reader_learns_the_spelling_and_refuses_a_loose_one ... ok
test the_serde_module_writes_and_reads_the_one_string ... ok
test base64_is_a_third_larger_and_a_number_array_several_times_larger ... ok
test every_byte_string_round_trips_through_its_one_spelling ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/canonical_encoding.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/canonical_encoding-718dbdcc8accfd81)

running 10 tests
test a_map_that_differs_in_one_value_encodes_differently ... ok
test a_type_is_part_of_the_encoding ... ok
test a_variant_marker_is_its_own_shape ... ok
test field_boundaries_are_not_ambiguous ... ok
test list_order_is_meaning_and_survives ... ok
test the_primitives_of_this_crate_encode ... ok
test two_btreemaps_with_the_same_entries_encode_to_the_same_bytes ... ok
test two_variants_with_byte_identical_payloads_encode_differently ... ok
test encoding_distinguishes_exactly_what_equality_does ... ok
test insertion_order_never_reaches_the_bytes ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/content_hash.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/content_hash-389be073b2ed5eb7)

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

     Running tests/decode.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/decode-9ce12c1de55e4549)

running 8 tests
test distinct_typed_keys_and_empty_maps_decode_normally ... ok
test a_duplicate_id_is_refused_before_its_invalid_second_value_is_decoded ... ok
test distinct_set_members_and_the_empty_set_decode_without_losing_values ... ok
test duplicate_set_identity_is_refused_instead_of_discarded ... ok
test set_uniqueness_uses_decoded_values_instead_of_input_spelling ... ok
test strings_decode_only_strings ... ok
test uniqueness_uses_the_decoded_key_type_not_its_input_spelling ... ok
test the_bounded_yaml_observation_refuses_each_bound_by_name ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/hashing_encoder_contract.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/hashing_encoder_contract-cbdc3513ce83c14c)

running 4 tests
test replacing_the_encoder_addresses_what_is_left_in_its_place ... ok
test finishing_a_hashing_encoder_is_refused - should panic ... ok
test an_encoder_reads_the_same_through_debug_on_both ... ok
test swapping_an_encoder_there_and_back_changes_no_address ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/identity_serde.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/identity_serde-ef9f83f67186804f)

running 52 tests
test adversary_input08::public_observation_ids_keep_uuid_bits_and_canonical_text ... ok
test adversary_i_macro_doc_counts_the_id_newtypes_it_declares ... ok
test adversary_input08::removing_a_declared_identity_is_not_hidden_by_the_typed_inventory ... ok
test agent_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test agent_id::round_trips_for_any_bits ... ok
test adversary_input08::renamed_domain_files_preserve_the_identity_inventory ... ok
test assertion_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test assertion_id::round_trips_for_any_bits ... ok
test attachment_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test attachment_id::round_trips_for_any_bits ... ok
test edge_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test edge_id::round_trips_for_any_bits ... ok
test event_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test event_id::round_trips_for_any_bits ... ok
test every_ess_id_type_exists_in_the_crate ... ok
test evidence_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test evidence_id::round_trips_for_any_bits ... ok
test existing_observe_identities_have_typed_contract_cases ... ok
test graph_root_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test graph_root_id::round_trips_for_any_bits ... ok
test issue_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test an_unknown_domain_filename_with_a_missing_identity_fails_the_actual_scan ... ok
test merge_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test issue_id::round_trips_for_any_bits ... ok
test node_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test merge_id::round_trips_for_any_bits ... ok
test observation_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test node_id::round_trips_for_any_bits ... ok
test property_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test observation_id::round_trips_for_any_bits ... ok
test revision_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test property_id::round_trips_for_any_bits ... ok
test revision_number::round_trips_as_a_number_and_refuses_a_malformed_one ... ok
test revision_number::round_trips_for_any_number ... ok
test revision_number_text_form::a_second_spelling_of_a_number_is_refused ... ok
test revision_id::round_trips_for_any_bits ... ok
test schema_version_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test revision_number_text_form::the_written_form_is_the_only_form ... ok
test source_checkpoint_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test adversary_input08::semantic_yaml_spelling_does_not_hide_a_new_identity ... ok
test source_unit_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test schema_version_id::round_trips_for_any_bits ... ok
test split_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test source_checkpoint_id::round_trips_for_any_bits ... ok
test support_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test source_unit_id::round_trips_for_any_bits ... ok
test transaction_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test split_id::round_trips_for_any_bits ... ok
test type_id::round_trips_as_a_string_and_refuses_a_malformed_one ... ok
test support_id::round_trips_for_any_bits ... ok
test transaction_id::round_trips_for_any_bits ... ok
test type_id::round_trips_for_any_bits ... ok

test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running tests/public_surface.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/public_surface-6c0d5f3499f988bd)

running 5 tests
test a_revision_number_counts_from_the_seed_and_stops_at_the_end ... ok
test a_refusal_carries_the_text_it_refused ... ok
test an_id_carries_its_bits_through_uuid_and_back ... ok
test the_encoder_writes_each_shape_it_publishes ... ok
test no_public_item_is_untested ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/rename_stability.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/rename_stability-f4f0c621f5f2c2e4)

running 5 tests
test no_id_type_mints_a_constant ... ok
test renaming_two_things_to_one_name_does_not_merge_them ... ok
test the_same_name_twice_is_two_identities ... ok
test a_burst_of_mints_is_all_distinct ... ok
test two_values_that_share_a_name_never_share_an_id ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/sha256_backend.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/sha256_backend-22681c8ff89ee53d)

running 1 test
test payload_and_streamed_value_digests_keep_every_boundary ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/timestamp.rs (<cache>/b10x-target/ekr-input-08-identity/debug/deps/timestamp-06a898b0a382a192)

running 7 tests
test a_refusal_carries_the_text_and_nothing_else ... ok
test a_timestamp_crosses_serde_as_the_number_it_is ... ok
test a_timestamp_encodes_as_the_integer_it_wraps ... ok
test a_timestamp_is_milliseconds_since_the_unix_epoch ... ok
test one_value_has_one_text ... ok
test the_range_is_far_wider_than_nanoseconds_would_give ... ok
test the_text_form_refuses_every_other_spelling ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests ekr_core

running 2 tests
test crates/ekr-core/src/bytes.rs - bytes (line 13) ... ok
test crates/ekr-core/src/lib.rs - (line 19) ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

```

Command: cargo clippy -p ekr-core --all-targets --all-features -- -D warnings
Exit: 0. Raw output:
```text
    Checking cfg-if v1.0.5
    Checking libc v0.2.189
   Compiling serde_core v1.0.229
   Compiling serde v1.0.229
    Checking typenum v1.20.1
    Checking getrandom v0.4.3
    Checking getrandom v0.3.4
    Checking hybrid-array v0.4.15
    Checking rand_core v0.9.5
    Checking bitflags v2.13.2
    Checking itoa v1.0.18
    Checking linux-raw-sys v0.12.1
   Compiling serde_json v1.0.151
   Compiling ref-cast v1.0.27
    Checking rustix v1.1.5
    Checking zmij v1.0.23
    Checking zerocopy v0.8.59
   Compiling ref-cast-impl v1.0.27
   Compiling serde_derive_internals v0.30.0
    Checking fastrand v2.5.0
    Checking equivalent v1.0.2
    Checking memchr v2.8.3
    Checking hashbrown v0.17.1
    Checking once_cell v1.21.4
    Checking tempfile v3.27.0
   Compiling schemars_derive v1.2.2
    Checking indexmap v2.14.2
    Checking ppv-lite86 v0.2.21
    Checking crypto-common v0.2.2
    Checking block-buffer v0.12.1
    Checking wait-timeout v0.2.1
    Checking getrandom v0.2.17
    Checking untrusted v0.9.0
    Checking bit-vec v0.8.0
    Checking quick-error v1.2.3
    Checking unsafe-libyaml v0.2.11
    Checking dyn-clone v1.0.20
    Checking fnv v1.0.7
    Checking ryu v1.0.23
    Checking const-oid v0.10.2
    Checking schemars v1.2.2
    Checking digest v0.11.3
    Checking serde_yaml_ng v0.10.0 (<worktrees>/ekr-input-08-identity/vendor/serde_yaml_ng)
    Checking rusty-fork v0.3.1
    Checking bit-set v0.8.0
    Checking ring v0.17.14
    Checking thiserror v2.0.21
    Checking num-traits v0.2.19
    Checking rand_chacha v0.9.0
    Checking rand_xorshift v0.4.0
    Checking rand v0.9.5
    Checking uuid v1.26.1
    Checking unarray v0.1.4
    Checking hex v0.4.3
    Checking cpufeatures v0.3.1
    Checking regex-syntax v0.8.11
    Checking sha2 v0.11.0
    Checking ekr-core v0.0.27 (<worktrees>/ekr-input-08-identity/crates/ekr-core)
    Checking proptest v1.11.0
    Finished `dev` profile [unoptimized] target(s) in 14.32s
```

Command: cargo fmt -p ekr-core --check
Exit: 0; no output. git diff --check also exited 0.

Build environment: CARGO_BUILD_JOBS=3 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 RUSTC_WRAPPER=/usr/bin/sccache RUST_TEST_THREADS=3 CARGO_TARGET_DIR=<cache>/b10x-target/ekr-input-08-identity TMPDIR=<cache>/ekr-next-three/tmp.

3. Findings and limits

Nothing found in the bounded attacks. Read the complete candidate diff, task acceptance, typed serde/mint inventory and scanner callers. The unrelated ontology-domain comments were coordinator-propagated contract material, not identity implementation. No implementation file, existing assertion, planning artifact or Git index was changed. No permission, publication or independence claim is made.

This attack covers flat *.yaml domain discovery used by the repository, semantic UUID declarations, two-way inventory drift and the new public carriers; it makes no claim about unsupported recursive domain layouts. The exact baseline commit was read without moving this worktree. There is no finding requiring an origin comparison.

4. Outside-tree paths and handoff

- <cache>/ekr-next-three/input-08/identity/adversary-1/focused.log
- <cache>/ekr-next-three/input-08/identity/adversary-1/focused.exit
- <cache>/ekr-next-three/input-08/identity/adversary-1/core.log
- <cache>/ekr-next-three/input-08/identity/adversary-1/core.exit
- <cache>/ekr-next-three/input-08/identity/adversary-1/core-final.log
- <cache>/ekr-next-three/input-08/identity/adversary-1/core-final.exit
- <cache>/ekr-next-three/input-08/identity/adversary-1/clippy.log
- <cache>/ekr-next-three/input-08/identity/adversary-1/clippy.exit
- <cache>/ekr-next-three/input-08/identity/adversary-1/fmt.log
- <cache>/ekr-next-three/input-08/identity/adversary-1/fmt.exit
- <cache>/ekr-next-three/input-08/identity/adversary-1/report.md
- <cache>/ekr-next-three/input-08/identity/adversary-1/public-report.md
- <cache>/b10x-target/ekr-input-08-identity (compiler output; target now released with no running build)
- <cache>/ekr-next-three/tmp (Cargo/test runtime scratch and disposable test fixture directories; added fixtures removed themselves through their Drop guard)

Own lease codex-input08-identity-adversary1 was ended successfully. Target ownership is released. Coordinator owns the unstaged test-only diff, final gate and subsequent lifecycle. Tool/token metrics are unavailable and are not claimed.

```findings
[]
```
