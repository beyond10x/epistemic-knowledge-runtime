unit: story:apply-approved-refinement — application read projection unit
verdict: blocked
cases: executed 37→42, red 4
origin: n/a
wrote-outside-worktree: assigned application-projection scratch only; full private inventory beside report
needs-coordinator: yes, projection.patch plus manual mod application_projection registration; strict clippy after real codec callers

Strict clippy remains blocked solely by the frozen sibling transaction codec's 15 dead-code errors in this isolated tree. No suppression, fake caller, or export was added. Projection tests and the complete kernel library suite pass. This presentation unit is not F behavioral acceptance.

1. Unit and acceptance

Project ApplicationRead and all reachable records losslessly from generated data types to generated semantic types, then expose nonempty receipts/application through ShowSchemaProposal. The module covers elections, steps, attempts, full canonical transaction projections and all 16 operation alternatives, publication guards/records, qualified remaining items, mapping/derivation evidence, claim replacements and receipts.

All source record fields are destructured without `..`; all destination fields are explicitly constructed. Enum matches are exhaustive, including literal union tags. Source or destination field additions therefore require a reviewed projection change. No duplicate models or generated source changes were introduced. Existing byte and timestamp presentation helpers are reused without modification. Macro adapters implement projection only, not new data models or authority.

The exact source scope was checked. Only the new module/tests and shown() call site changed for this unit. The codec remains byte-identical to its earlier frozen checksums. Local test registration was explicitly authorized; root integrates that line manually and it is excluded from the incremental patch.

2. Exact incremental patch shape

 .../ekr-kernel/src/schema_proposal_projection.rs   |   15 -
 crates/ekr-kernel/src/application_projection.rs    |  382 ++++++++++++++++++++
 .../ekr-kernel/src/application_projection/tests.rs |  355 +++++++++++++++++++
 3 files changed, 746 insertions(+), 6 deletions(-)


Patch SHA256: 8da7b81955dd90e7eb43bb3e9aaa1f3e83afef36429e634e7c35dcf6ecf19bb8.

3. Observed red

Command: cargo test --locked -p ekr-kernel --lib application_projection::tests
Exit: 101. Five tests compiled; four failed at the absent projection or the actual ShowSchemaProposal unsupported-history refusal. The negative-conversion test passed against the fail-closed stub.

```text
   Compiling ekr-kernel v0.0.27 (<managed-worktree>)
warning: function `receipt` is never used
  --> crates/ekr-kernel/src/application_projection.rs:11:15
   |
11 | pub(super) fn receipt(
   |               ^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `ekr-kernel` (lib test) generated 1 warning
    Finished `test` profile [unoptimized] target(s) in 7.58s
     Running unittests src/lib.rs (<shared-target>)

running 5 tests
test application_projection::tests::absent_optionals_and_empty_collections_remain_distinct ... FAILED
test application_projection::tests::mixed_application_history_keeps_nested_payloads_and_coordinates ... FAILED
test application_projection::tests::proposal_show_exposes_receipts_and_application_history ... FAILED
test application_projection::tests::every_operation_union_alternative_keeps_its_tag ... FAILED
test application_projection::tests::numeric_overflow_and_invalid_bytes_refuse_instead_of_narrowing ... ok

failures:

---- application_projection::tests::absent_optionals_and_empty_collections_remain_distinct stdout ----

thread 'application_projection::tests::absent_optionals_and_empty_collections_remain_distinct' (166273) panicked at crates/ekr-kernel/src/application_projection/tests.rs:246:39:
called `Result::unwrap()` on an `Err` value: Document("application-projection: absent")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- application_projection::tests::mixed_application_history_keeps_nested_payloads_and_coordinates stdout ----

thread 'application_projection::tests::mixed_application_history_keeps_nested_payloads_and_coordinates' (166275) panicked at crates/ekr-kernel/src/application_projection/tests.rs:55:47:
called `Result::unwrap()` on an `Err` value: Document("application-projection: absent")

---- application_projection::tests::proposal_show_exposes_receipts_and_application_history stdout ----

thread 'application_projection::tests::proposal_show_exposes_receipts_and_application_history' (166277) panicked at crates/ekr-kernel/src/application_projection/tests.rs:265:10:
application history remains inspectable: Document("schema-proposal-refused: unsupported application history projection")

---- application_projection::tests::every_operation_union_alternative_keeps_its_tag stdout ----

thread 'application_projection::tests::every_operation_union_alternative_keeps_its_tag' (166274) panicked at crates/ekr-kernel/src/application_projection/tests.rs:303:39:
called `Result::unwrap()` on an `Err` value: Document("application-projection: absent")


failures:
    application_projection::tests::absent_optionals_and_empty_collections_remain_distinct
    application_projection::tests::every_operation_union_alternative_keeps_its_tag
    application_projection::tests::mixed_application_history_keeps_nested_payloads_and_coordinates
    application_projection::tests::proposal_show_exposes_receipts_and_application_history

test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 37 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p ekr-kernel --lib`
```

4. Green and verification

Focused command: cargo test --locked -p ekr-kernel --lib application_projection::tests
Focused lane: executed 5 → 5, exit 0. The same five preimplementation tests went from 1 pass/4 failures to 5 passes; no tests were added after red.

```text
   Compiling ekr-graph v0.0.27 (<managed-worktree>)
   Compiling ekr-store v0.0.27 (<managed-worktree>)
   Compiling ekr-integrate v0.0.27 (<managed-worktree>)
   Compiling ekr-kernel v0.0.27 (<managed-worktree>)
    Finished `test` profile [unoptimized] target(s) in 13.11s
     Running unittests src/lib.rs (<shared-target>)

running 5 tests
test application_projection::tests::absent_optionals_and_empty_collections_remain_distinct ... ok
test application_projection::tests::mixed_application_history_keeps_nested_payloads_and_coordinates ... ok
test application_projection::tests::every_operation_union_alternative_keeps_its_tag ... ok
test application_projection::tests::proposal_show_exposes_receipts_and_application_history ... ok
test application_projection::tests::numeric_overflow_and_invalid_bytes_refuse_instead_of_narrowing ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 37 filtered out; finished in 0.01s

```

Full library command: cargo test --locked -p ekr-kernel --lib
Library lane: executed 37 → 42, exit 0. The before count is the prior frozen-codec library runner summary, retained as library-before.log; this worktree's only changes since that run are this projection unit and its local test registration.

```text
    Finished `test` profile [unoptimized] target(s) in 0.11s
     Running unittests src/lib.rs (<shared-target>)

running 42 tests
test application_projection::tests::absent_optionals_and_empty_collections_remain_distinct ... ok
test application_projection::tests::mixed_application_history_keeps_nested_payloads_and_coordinates ... ok
test document::checked::tests::string_refusals_precede_the_allocating_application_visitor ... ok
test application_projection::tests::every_operation_union_alternative_keeps_its_tag ... ok
test document::checked::tests::mixed_accounting_refuses_before_allocating_a_coerced_string ... ok
test application_transaction::tests::unrepresentable_timestamps_and_nested_values_fail_closed ... ok
test incubation_value::tests::existing_canonical_encoder_is_the_oracle_for_every_value_kind ... ok
test document::bounded_load::a_transaction_document_nested_past_the_depth_is_refused_at_the_first_container_past_it ... ok
test application_projection::tests::proposal_show_exposes_receipts_and_application_history ... ok
test application_transaction::tests::canonical_value_bytes_and_kind_are_both_checked ... ok
test application_transaction::tests::optional_owner_schema_and_empty_aliases_preserve_historical_meaning ... ok
test application_transaction::tests::normalization_is_limited_to_empty_aliases_and_equivalent_instants ... ok
test application_transaction::tests::every_assertion_and_evidence_payload_variant_round_trips ... ok
test application_transaction::tests::all_operations_preserve_native_fields_and_canonical_bytes ... ok
test application_projection::tests::numeric_overflow_and_invalid_bytes_refuse_instead_of_narrowing ... ok
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
test replay::tests::reused_graph_preserves_inherited_attachments_and_matches_forced_clone ... ok
test replay::tests::an_unpublished_candidate_cannot_seed_reuse ... ok
test replay::tests::an_equal_root_replayed_by_another_authority_falls_back_to_clone ... ok
test checkpoint::tests::a_fresh_open_restores_the_head_and_replays_nothing_the_checkpoint_covers ... ok
test replay::tests::readers_and_checkpoint_graphs_prevent_reuse_extraction ... ok
test replay::tests::warm_alias_checks_visit_no_unchanged_nodes ... ok
test replay::tests::one_handle_holds_the_head_graph_and_not_one_graph_per_revision ... ok
test replay::tests::a_new_checkpoint_releases_its_predecessors_graph_before_the_next_command ... ok
test replay::tests::one_handle_validates_against_an_earlier_revision_whose_graph_it_released ... ok
test replay::tests::validating_one_transaction_does_not_copy_prior_document_buffers ... ok
test answers::tests::reviewed_answers_publish_retry_reopen_and_fully_replay_on_both_providers ... ok
test replay::tests::warm_validation_hashes_no_more_prefix_occurrences_as_history_grows ... ok
test replay::tests::append_commits_copy_no_more_assertions_as_the_graph_grows ... ok

test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.18s

```

Strict lint command: cargo clippy --locked -p ekr-kernel --lib --tests -- -D warnings
Exit: 101. The named failure is only the frozen sibling codec's 15 dead-code errors; strict integration clippy remains required after root's real callers are present.

```text
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

Exact owned-file rustfmt --edition 2021 --check: exit 0. git diff --check: exit 0. Frozen-codec SHA256 verification: exit 0, all three files unchanged. Shared Cargo lane ran sequentially under explicit root authorization and was handed back to root with no live Cargo.

5. Presentation boundaries

Numeric values must fit the semantic i64 exactly; fractional and out-of-range Numbers refuse. Bytes decode through the existing checked base64 codec. Timestamp strings retain nanosecond precision and timezone offset because this is generated-to-generated presentation, not conversion to native millisecond timestamps. UUID and digest wrappers preserve their existing text without asserting admission.

Optional fields, including otherwise inapplicable semantic payload combinations, remain visible. Empty lists stay empty lists; absent optional lists stay absent. The mixed fixture deliberately contains such combinations and opaque canonical bytes to show that this projection neither silently drops them nor represents them as authorized native transactions. Ordinary kernel validation, canonical byte validation, trusted review verification and cross-record consistency remain with their existing admission/replay owners.

No storage, ESS, generated models, AEP, CLI/SDK, authorization logic, remote operations, commits or publication were changed. Root still owns full integration and task check. Source and tests are left uncommitted for review/integration.

6. External paths

All unit writes outside the worktree are listed with absolute paths in private-inventory.txt. This sanitized report uses portable names. Raw logs, original call-site/lib snapshots, source hashes, exact patch and report live together in the assigned application-projection scratch directory. The earlier codec evidence directory is unchanged.
