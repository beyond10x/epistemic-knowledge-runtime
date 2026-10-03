unit: story:apply-approved-refinement — generated canonical transaction codec
verdict: blocked
cases: executed unmeasured→37, red 5
origin: n/a
wrote-outside-worktree: assigned scratch logs, patch and reports (private inventory)
needs-coordinator: yes, codec.patch; connect real production callers and rerun strict clippy

Strict clippy currently refuses dead code because this independently authored module has no production callers yet. No suppression, fake caller or export was introduced. The full kernel library suite passes. This bounded codec does not establish F conformance or full task-check acceptance.

1. Unit and acceptance

Pure bidirectional checked transport between native GraphTransaction<CanonicalValue> and the generated EkrKernelCanonicalTransactionProjection, preserving all 16 operations, nested canonical bytes, identities, schema ownership/version, evidence and timestamps; malformed representations fail closed. No admission or ValidatedTransaction construction occurs.

Scope checked against actual source: the new module and its children hold the codec/tests. Parent explicitly authorized exactly one registration line in lib.rs. Existing incubation canonical-value decoding is reused. Checked serde bridges cover equivalent fields only, with explicit transforms at every known differing representation; full generated typed equality after decode/re-encode refuses discarded optional payloads and duplicate/unsorted set projections.

2. Exact patch shape

 crates/ekr-kernel/src/lib.rs                       |    1
 crates/ekr-kernel/src/application_transaction.rs   |  110 +++++
 .../src/application_transaction/projection.rs      |  313 +++++++++++++
 .../src/application_transaction/tests.rs           |  476 ++++++++++++++++++++
 4 files changed, 900 insertions(+)


3. Red

Command: cargo test --locked -p ekr-kernel --lib application_transaction::tests
Exit: 101. The five acceptance tests compiled and failed at the absent implementation before the codec was written.

```text
   Compiling ekr-kernel v0.0.27 (<managed-worktree>)
    Finished `test` profile [unoptimized] target(s) in 6.65s
     Running unittests src/lib.rs (<shared-target>)

running 5 tests
test application_transaction::tests::canonical_value_bytes_and_kind_are_both_checked ... FAILED
test application_transaction::tests::optional_owner_schema_and_empty_aliases_preserve_historical_meaning ... FAILED
test application_transaction::tests::all_operations_preserve_native_fields_and_canonical_bytes ... FAILED
test application_transaction::tests::inconsistent_projection_payloads_are_not_silently_dropped ... FAILED
test application_transaction::tests::malformed_identity_and_duplicate_manifest_are_refused ... FAILED

failures:

---- application_transaction::tests::canonical_value_bytes_and_kind_are_both_checked stdout ----

thread 'application_transaction::tests::canonical_value_bytes_and_kind_are_both_checked' (4091773) panicked at crates/ekr-kernel/src/application_transaction/tests.rs:278:64:
called `Result::unwrap()` on an `Err` value: Document("application-transaction: codec absent")

---- application_transaction::tests::optional_owner_schema_and_empty_aliases_preserve_historical_meaning stdout ----

thread 'application_transaction::tests::optional_owner_schema_and_empty_aliases_preserve_historical_meaning' (4091776) panicked at crates/ekr-kernel/src/application_transaction/tests.rs:302:37:
called `Result::unwrap()` on an `Err` value: Document("application-transaction: codec absent")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- application_transaction::tests::all_operations_preserve_native_fields_and_canonical_bytes stdout ----

thread 'application_transaction::tests::all_operations_preserve_native_fields_and_canonical_bytes' (4091772) panicked at crates/ekr-kernel/src/application_transaction/tests.rs:212:37:
all operations have a generated transport: Document("application-transaction: codec absent")

---- application_transaction::tests::inconsistent_projection_payloads_are_not_silently_dropped stdout ----

thread 'application_transaction::tests::inconsistent_projection_payloads_are_not_silently_dropped' (4091774) panicked at crates/ekr-kernel/src/application_transaction/tests.rs:313:64:
called `Result::unwrap()` on an `Err` value: Document("application-transaction: codec absent")

---- application_transaction::tests::malformed_identity_and_duplicate_manifest_are_refused stdout ----

thread 'application_transaction::tests::malformed_identity_and_duplicate_manifest_are_refused' (4091775) panicked at crates/ekr-kernel/src/application_transaction/tests.rs:256:63:
called `Result::unwrap()` on an `Err` value: Document("application-transaction: codec absent")


failures:
    application_transaction::tests::all_operations_preserve_native_fields_and_canonical_bytes
    application_transaction::tests::canonical_value_bytes_and_kind_are_both_checked
    application_transaction::tests::inconsistent_projection_payloads_are_not_silently_dropped
    application_transaction::tests::malformed_identity_and_duplicate_manifest_are_refused
    application_transaction::tests::optional_owner_schema_and_empty_aliases_preserve_historical_meaning

test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 28 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr-kernel --lib`
```

4. Green and verification

Command: cargo test --locked -p ekr-kernel --lib application_transaction::tests
Exit: 0. Focused lane: executed 5 → 9, exit 0 (five observed-red tests plus four added boundary/variant tests).

```text
   Compiling ekr-kernel v0.0.27 (<managed-worktree>)
    Finished `test` profile [unoptimized] target(s) in 7.22s
     Running unittests src/lib.rs (<shared-target>)

running 9 tests
test application_transaction::tests::unrepresentable_timestamps_and_nested_values_fail_closed ... ok
test application_transaction::tests::canonical_value_bytes_and_kind_are_both_checked ... ok
test application_transaction::tests::optional_owner_schema_and_empty_aliases_preserve_historical_meaning ... ok
test application_transaction::tests::all_operations_preserve_native_fields_and_canonical_bytes ... ok
test application_transaction::tests::every_assertion_and_evidence_payload_variant_round_trips ... ok
test application_transaction::tests::inconsistent_projection_payloads_are_not_silently_dropped ... ok
test application_transaction::tests::normalization_is_limited_to_empty_aliases_and_equivalent_instants ... ok
test application_transaction::tests::invalid_numeric_ranges_and_interval_order_are_refused ... ok
test application_transaction::tests::malformed_identity_and_duplicate_manifest_are_refused ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out; finished in 0.01s

```

Command: cargo test --locked -p ekr-kernel --lib
Exit: 0. Complete library lane: executed unmeasured → 37, exit 0. An actual base-run count was not collected; no inferred count is presented as measured evidence.

```text
    Finished `test` profile [unoptimized] target(s) in 0.11s
     Running unittests src/lib.rs (<shared-target>)

running 37 tests
test document::checked::tests::string_refusals_precede_the_allocating_application_visitor ... ok
test document::checked::tests::mixed_accounting_refuses_before_allocating_a_coerced_string ... ok
test incubation_value::tests::existing_canonical_encoder_is_the_oracle_for_every_value_kind ... ok
test document::bounded_load::a_transaction_document_nested_past_the_depth_is_refused_at_the_first_container_past_it ... ok
test application_transaction::tests::unrepresentable_timestamps_and_nested_values_fail_closed ... ok
test application_transaction::tests::canonical_value_bytes_and_kind_are_both_checked ... ok
test application_transaction::tests::optional_owner_schema_and_empty_aliases_preserve_historical_meaning ... ok
test application_transaction::tests::all_operations_preserve_native_fields_and_canonical_bytes ... ok
test application_transaction::tests::normalization_is_limited_to_empty_aliases_and_equivalent_instants ... ok
test application_transaction::tests::every_assertion_and_evidence_payload_variant_round_trips ... ok
test application_transaction::tests::inconsistent_projection_payloads_are_not_silently_dropped ... ok
test application_transaction::tests::invalid_numeric_ranges_and_interval_order_are_refused ... ok
test application_transaction::tests::malformed_identity_and_duplicate_manifest_are_refused ... ok
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
test replay::tests::warm_validation_hashes_no_more_prefix_occurrences_as_history_grows ... ok
test answers::tests::reviewed_answers_publish_retry_reopen_and_fully_replay_on_both_providers ... ok
test replay::tests::validating_one_transaction_does_not_copy_prior_document_buffers ... ok
test replay::tests::append_commits_copy_no_more_assertions_as_the_graph_grows ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.07s

```

Command: cargo clippy --locked -p ekr-kernel --lib --tests -- -D warnings
Exit: 101. Fifteen dead-code diagnostics name only this unwired module. Real root orchestration must consume encode/decode before the strict gate is rerun.

```text
    Checking serde_yaml_ng v0.10.0 (<managed-worktree>)
    Checking ekr-core v0.0.27 (<managed-worktree>)
    Checking ekr-ontology v0.0.27 (<managed-worktree>)
    Checking ekr-graph v0.0.27 (<managed-worktree>)
    Checking ekr-store v0.0.27 (<managed-worktree>)
    Checking ekr-integrate v0.0.27 (<managed-worktree>)
    Checking ekr-kernel v0.0.27 (<managed-worktree>)
error: function `refused` is never used
  --> crates/ekr-kernel/src/application_transaction.rs:19:4
   |
19 | fn refused(reason: impl std::fmt::Display) -> StoreError {
   |    ^^^^^^^
   |
   = note: `-D dead-code` implied by `-D warnings`
   = help: to override `-D warnings` add `#[expect(dead_code)]` or `#[allow(dead_code)]`

error: function `read` is never used
  --> crates/ekr-kernel/src/application_transaction.rs:22:4
   |
22 | fn read<T: DeserializeOwned>(value: Value) -> Result<T, StoreError> {
   |    ^^^^

error: function `write` is never used
  --> crates/ekr-kernel/src/application_transaction.rs:25:4
   |
25 | fn write<T: Serialize + ?Sized>(value: &T) -> Result<Value, StoreError> {
   |    ^^^^^

error: function `field` is never used
  --> crates/ekr-kernel/src/application_transaction.rs:28:4
   |
28 | fn field<'a>(value: &'a mut Value, name: &str) -> Result<&'a mut Value, StoreError> {
   |    ^^^^^

error: function `object` is never used
  --> crates/ekr-kernel/src/application_transaction.rs:33:4
   |
33 | fn object(value: &mut Value) -> Result<&mut serde_json::Map<String, Value>, StoreError> {
   |    ^^^^^^

error: function `array` is never used
  --> crates/ekr-kernel/src/application_transaction.rs:38:4
   |
38 | fn array(value: &mut Value) -> Result<&mut Vec<Value>, StoreError> {
   |    ^^^^^

error: function `encode` is never used
  --> crates/ekr-kernel/src/application_transaction.rs:42:15
   |
42 | pub(crate) fn encode(
   |               ^^^^^^

error: function `decode` is never used
  --> crates/ekr-kernel/src/application_transaction.rs:81:15
   |
81 | pub(crate) fn decode(
   |               ^^^^^^

error: function `operation` is never used
 --> crates/ekr-kernel/src/application_transaction/projection.rs:5:15
  |
5 | pub(super) fn operation(kind: &str, value: &mut Value, encode: bool) -> Result<(), StoreError> {
  |               ^^^^^^^^^

error: function `optional` is never used
  --> crates/ekr-kernel/src/application_transaction/projection.rs:98:4
   |
98 | fn optional(value: &mut Value, name: &str, encode: bool) -> Result<(), StoreError> {
   |    ^^^^^^^^

error: function `timestamp` is never used
   --> crates/ekr-kernel/src/application_transaction/projection.rs:105:4
    |
105 | fn timestamp(value: &mut Value, encode: bool) -> Result<(), StoreError> {
    |    ^^^^^^^^^

error: function `canonical_value` is never used
   --> crates/ekr-kernel/src/application_transaction/projection.rs:125:4
    |
125 | fn canonical_value(value: &mut Value, encode: bool) -> Result<(), StoreError> {
    |    ^^^^^^^^^^^^^^^

error: function `value_type` is never used
   --> crates/ekr-kernel/src/application_transaction/projection.rs:150:4
    |
150 | fn value_type(value: &mut Value, encode: bool) -> Result<(), StoreError> {
    |    ^^^^^^^^^^

error: function `sum` is never used
   --> crates/ekr-kernel/src/application_transaction/projection.rs:205:4
    |
205 | fn sum(
    |    ^^^

error: function `assertion` is never used
   --> crates/ekr-kernel/src/application_transaction/projection.rs:257:4
    |
257 | fn assertion(value: &mut Value, encode: bool) -> Result<(), StoreError> {
    |    ^^^^^^^^^

error: could not compile `ekr-kernel` (lib) due to 15 previous errors
```

Exact owned files rustfmt --edition 2021 --check: exit 0. git diff --check: exit 0. An initial green attempt refused Rust-2024-only let chains under this workspace's edition 2021 and a test import; both were corrected before the successful run.

5. Deliberate boundaries

The generated calendar timestamp range is narrower than native i64 milliseconds; encoding outside it refuses with an explicit calendar-range error. Fractional milliseconds refuse. Canonical values reuse the existing bounded decoder (nesting depth 32); encoding checks the same bound, so it never emits an unreadable value. These are explicit transport representability limits, not transaction validation rules.

Only empty node aliases versus absent aliases, and timestamp offsets representing the same exact millisecond instant, normalize. Native canonical bytes are unchanged. Ordered duplicate alias entries are preserved. Canonical value Float remains prohibited while ontology Float declarations retain their existing meaning. Schema enum sets are transported without inferring intent.

The module is not wired into runtime orchestration in this unit. No store/graph/ESS/AEP/CLI/SDK files, generated models, commits or publication were changed. Full integration suite and task check remain root responsibilities. The shared Cargo target was used sequentially under the parent's explicit coordination exception; the lane is released to root.

6. Private evidence inventory

The complete absolute-path inventory is in private-inventory.txt beside this report. Published report text deliberately uses repository-relative and portable names. Raw logs preserve original output and command environment; source-sha256.txt identifies frozen source and codec.patch is the exact integration patch.
