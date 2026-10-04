unit: story:apply-approved-refinement — pure application fact step builder and verifier
verdict: blocked
cases: executed 51→60, red 5
origin: n/a
wrote-outside-worktree: private assigned application-fact evidence directory; full inventory retained privately
needs-coordinator: yes, fact.patch and lib-registration.patch; real production callers and strict clippy rerun

1. Unit and scope

Build an immutable generated mapping step from authenticated inputs with exact mapped evidence, checked UUID identities, and source/mapping/proposal provenance. Verify frozen templates structurally and at the actual first guarded Propose semantic read. This bounded unit does not establish F acceptance. Scope resolves to the two new application_fact source files and one module registration. Prior mapping changes and synchronized ESS amendments are excluded.

2. Actual patch shape

```
 crates/ekr-kernel/src/application_fact.rs       |  334 ++++++++++++++++++++
 crates/ekr-kernel/src/application_fact/tests.rs |  391 +++++++++++++++++++++++
 2 files changed, 725 insertions(+)
```

3. Initial red

Command: cargo test -p ekr-kernel --lib application_fact -- --nocapture
Exit: 101. Six tests existed before implementation: five failed at the missing builder; the fail-closed negative-support case already passed. Three additional tamper/boundary cases were added after the first green and are not claimed red-first.

```
   Compiling ekr-kernel v0.0.27 (<worktree>/crates/ekr-kernel)
warning: multiple fields are never read
  --> crates/ekr-kernel/src/application_fact.rs:8:9
   |
 7 | pub(crate) struct BuildInputs<'a> {
   |                   ----------- fields in this struct
 8 |     pub read: &'a VerifiedRead,
   |         ^^^^
 9 |     pub proposal: &'a w::EkrIntegrateRetainedSchemaProposal,
   |         ^^^^^^^^
10 |     pub mapping: &'a w::EkrIntegrateKnowledgeMapping,
   |         ^^^^^^^
11 |     pub sources: &'a BTreeMap<String, Source>,
   |         ^^^^^^^
12 |     pub evidence_map: &'a BTreeMap<EvidenceId, EvidenceId>,
   |         ^^^^^^^^^^^^
13 |     pub application_id: &'a w::EkrIntegrateSchemaApplicationId,
   |         ^^^^^^^^^^^^^^
14 |     pub proposer: AgentId,
   |         ^^^^^^^^
15 |     pub elected_at: Timestamp,
   |         ^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: variants `Ready` and `Blocked` are never constructed
  --> crates/ekr-kernel/src/application_fact.rs:19:5
   |
18 | pub(crate) enum FactStepOutcome {
   |                 --------------- variants in this enum
19 |     Ready(Box<w::EkrIntegrateRetainedApplicationStep>),
   |     ^^^^^
20 |     Blocked(Vec<String>),
   |     ^^^^^^^
   |
   = note: `FactStepOutcome` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis

warning: fields `step`, `transaction`, `assertion`, `mapping`, and `derivations` are never read
  --> crates/ekr-kernel/src/application_fact.rs:23:5
   |
22 | struct Identities {
   |        ---------- fields in this struct
23 |     step: w::EkrIntegrateApplicationStepId,
   |     ^^^^
24 |     transaction: ekr_core::TransactionId,
   |     ^^^^^^^^^^^
25 |     assertion: ekr_core::AssertionId,
   |     ^^^^^^^^^
26 |     mapping: w::EkrIntegrateMappingRecordId,
   |     ^^^^^^^
27 |     derivations: Vec<String>,
   |     ^^^^^^^^^^^

warning: `ekr-kernel` (lib test) generated 3 warnings
    Finished `test` profile [unoptimized] target(s) in 8.27s
     Running unittests src/lib.rs (<shared-build>/target/debug/deps/ekr_kernel-f8587ff637e2bcbf)

running 6 tests
test application_fact::tests::frozen_template_tampering_is_rejected ... FAILED
test application_fact::tests::fact_step_contains_only_supported_assertion_and_exact_mapping_provenance ... FAILED
test application_fact::tests::blocked_resolution_does_not_even_invoke_the_identity_allocator ... FAILED
test application_fact::tests::semantic_verification_uses_frozen_ids_but_not_timestamp_guessed_revision ... FAILED
test application_fact::tests::blocked_mapping_returns_blockers_without_a_step ... FAILED
test application_fact::tests::no_support_or_incomplete_correspondence_never_creates_fresh_evidence ... ok

failures:

---- application_fact::tests::frozen_template_tampering_is_rejected stdout ----

thread 'application_fact::tests::frozen_template_tampering_is_rejected' (1379905) panicked at crates/ekr-kernel/src/application_fact/tests.rs:136:66:
called `Result::unwrap()` on an `Err` value: Document("application-fact: absent")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- application_fact::tests::fact_step_contains_only_supported_assertion_and_exact_mapping_provenance stdout ----

thread 'application_fact::tests::fact_step_contains_only_supported_assertion_and_exact_mapping_provenance' (1379904) panicked at crates/ekr-kernel/src/application_fact/tests.rs:136:66:
called `Result::unwrap()` on an `Err` value: Document("application-fact: absent")

---- application_fact::tests::blocked_resolution_does_not_even_invoke_the_identity_allocator stdout ----

thread 'application_fact::tests::blocked_resolution_does_not_even_invoke_the_identity_allocator' (1379903) panicked at crates/ekr-kernel/src/application_fact/tests.rs:305:6:
called `Result::unwrap()` on an `Err` value: Document("application-fact: absent")

---- application_fact::tests::semantic_verification_uses_frozen_ids_but_not_timestamp_guessed_revision stdout ----

thread 'application_fact::tests::semantic_verification_uses_frozen_ids_but_not_timestamp_guessed_revision' (1379907) panicked at crates/ekr-kernel/src/application_fact/tests.rs:136:66:
called `Result::unwrap()` on an `Err` value: Document("application-fact: absent")

---- application_fact::tests::blocked_mapping_returns_blockers_without_a_step stdout ----

thread 'application_fact::tests::blocked_mapping_returns_blockers_without_a_step' (1379902) panicked at crates/ekr-kernel/src/application_fact/tests.rs:223:44:
called `Result::unwrap()` on an `Err` value: Document("application-fact: absent")


failures:
    application_fact::tests::blocked_mapping_returns_blockers_without_a_step
    application_fact::tests::blocked_resolution_does_not_even_invoke_the_identity_allocator
    application_fact::tests::fact_step_contains_only_supported_assertion_and_exact_mapping_provenance
    application_fact::tests::frozen_template_tampering_is_rejected
    application_fact::tests::semantic_verification_uses_frozen_ids_but_not_timestamp_guessed_revision

test result: FAILED. 1 passed; 5 failed; 0 ignored; 0 measured; 51 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p ekr-kernel --lib`
```

4. Verification

Focused lane: executed 6 → 9, exit 0. Initial six-case postimplementation run was green; final nine-case run follows.

Command: cargo test -p ekr-kernel --lib application_fact -- --nocapture
```
   Compiling ekr-kernel v0.0.27 (<worktree>/crates/ekr-kernel)
    Finished `test` profile [unoptimized] target(s) in 8.64s
     Running unittests src/lib.rs (<shared-build>/target/debug/deps/ekr_kernel-f8587ff637e2bcbf)

running 9 tests
test application_fact::tests::blocked_resolution_does_not_even_invoke_the_identity_allocator ... ok
test application_fact::tests::blocked_mapping_returns_blockers_without_a_step ... ok
test application_fact::tests::semantic_verification_checks_the_claim_value_beyond_structural_shape ... ok
test application_fact::tests::fact_step_contains_only_supported_assertion_and_exact_mapping_provenance ... ok
test application_fact::tests::semantic_verification_uses_frozen_ids_but_not_timestamp_guessed_revision ... ok
test application_fact::tests::no_support_or_incomplete_correspondence_never_creates_fresh_evidence ... ok
test application_fact::tests::unselected_mapping_and_missing_correspondence_fail_before_allocating ... ok
test application_fact::tests::frozen_template_rejects_duplicate_identities_and_incomplete_support ... ok
test application_fact::tests::frozen_template_tampering_is_rejected ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 51 filtered out; finished in 0.02s

```

Library lane: executed 51 → 60, exit 0. Before count is the prior mapping unit's actual regression runner summary; after is this run. Prior combined regression also ran knowledge_retention and schema_proposal_reviews, but those targets were not rerun or counted in this bounded library lane.

Command: cargo test -p ekr-kernel --lib
```
    Finished `test` profile [unoptimized] target(s) in 0.07s
     Running unittests src/lib.rs (<shared-build>/target/debug/deps/ekr_kernel-f8587ff637e2bcbf)

running 60 tests
test application_plan::tests::application_schema_plan_cannot_redeclare_an_existing_property ... ok
test application_plan::tests::application_schema_plan_refuses_destructive_changes_and_empty_effects ... ok
test application_plan::tests::frozen_schema_plan_preserves_allocations_and_existing_declarations ... ok
test application_projection::tests::absent_optionals_and_empty_collections_remain_distinct ... ok
test application_projection::tests::mixed_application_history_keeps_nested_payloads_and_coordinates ... ok
test application_projection::tests::every_operation_union_alternative_keeps_its_tag ... ok
test application_projection::tests::proposal_show_exposes_receipts_and_application_history ... ok
test application_fact::tests::blocked_resolution_does_not_even_invoke_the_identity_allocator ... ok
test application_fact::tests::blocked_mapping_returns_blockers_without_a_step ... ok
test application_mapping::tests::relation_copy_and_noderef_constant_check_both_endpoints ... ok
test application_mapping::tests::resolved_property_uses_alias_and_exact_selected_fact_evidence ... ok
test application_mapping::tests::mapping_payload_digest_and_qualified_keys_distinguish_each_mapping_and_source ... ok
test application_transaction::tests::all_operations_preserve_native_fields_and_canonical_bytes ... ok
test application_projection::tests::numeric_overflow_and_invalid_bytes_refuse_instead_of_narrowing ... ok
test application_transaction::tests::canonical_value_bytes_and_kind_are_both_checked ... ok
test application_mapping::tests::all_canonical_value_kinds_work_for_constant_and_copy_field ... ok
test application_fact::tests::semantic_verification_checks_the_claim_value_beyond_structural_shape ... ok
test application_fact::tests::fact_step_contains_only_supported_assertion_and_exact_mapping_provenance ... ok
test application_transaction::tests::unrepresentable_timestamps_and_nested_values_fail_closed ... ok
test document::checked::tests::mixed_accounting_refuses_before_allocating_a_coerced_string ... ok
test document::checked::tests::string_refusals_precede_the_allocating_application_visitor ... ok
test incubation_value::tests::existing_canonical_encoder_is_the_oracle_for_every_value_kind ... ok
test document::bounded_load::a_transaction_document_nested_past_the_depth_is_refused_at_the_first_container_past_it ... ok
test application_transaction::tests::every_assertion_and_evidence_payload_variant_round_trips ... ok
test application_fact::tests::semantic_verification_uses_frozen_ids_but_not_timestamp_guessed_revision ... ok
test application_fact::tests::no_support_or_incomplete_correspondence_never_creates_fresh_evidence ... ok
test application_transaction::tests::optional_owner_schema_and_empty_aliases_preserve_historical_meaning ... ok
test application_fact::tests::frozen_template_rejects_duplicate_identities_and_incomplete_support ... ok
test application_transaction::tests::inconsistent_projection_payloads_are_not_silently_dropped ... ok
test application_transaction::tests::normalization_is_limited_to_empty_aliases_and_equivalent_instants ... ok
test application_fact::tests::unselected_mapping_and_missing_correspondence_fail_before_allocating ... ok
test application_transaction::tests::invalid_numeric_ranges_and_interval_order_are_refused ... ok
test application_fact::tests::frozen_template_tampering_is_rejected ... ok
test application_transaction::tests::malformed_identity_and_duplicate_manifest_are_refused ... ok
test application_mapping::tests::unresolved_ambiguous_and_invalid_selectors_match_preview_blockers ... ok
test application_mapping::tests::relation_missing_ambiguous_and_outside_endpoints_stay_blocked ... ok
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
test replay::tests::reused_graph_preserves_inherited_attachments_and_matches_forced_clone ... ok
test replay::tests::an_equal_root_replayed_by_another_authority_falls_back_to_clone ... ok
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

test result: ok. 60 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.60s

```

Command: cargo clippy -p ekr-kernel --lib -- -D warnings
Exit: 101. All 18 diagnostics are dead-code in the isolated unwired mapping/fact call graph. No suppression, fake caller, or public export was added. Integration with actual callers must pass this check.
```
    Checking ekr-kernel v0.0.27 (<worktree>/crates/ekr-kernel)
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

error: struct `BuildInputs` is never constructed
  --> crates/ekr-kernel/src/application_fact.rs:18:19
   |
18 | pub(crate) struct BuildInputs<'a> {
   |                   ^^^^^^^^^^^

error: enum `FactStepOutcome` is never used
  --> crates/ekr-kernel/src/application_fact.rs:29:17
   |
29 | pub(crate) enum FactStepOutcome {
   |                 ^^^^^^^^^^^^^^^

error: struct `Identities` is never constructed
  --> crates/ekr-kernel/src/application_fact.rs:34:8
   |
34 | struct Identities {
   |        ^^^^^^^^^^

error: associated items `mint` and `check` are never used
  --> crates/ekr-kernel/src/application_fact.rs:42:8
   |
41 | impl Identities {
   | --------------- associated items in this implementation
42 |     fn mint(supports: usize) -> Self {
   |        ^^^^
...
52 |     fn check(&self, supports: usize) -> Result<(), StoreError> {
   |        ^^^^^

error: function `error` is never used
  --> crates/ekr-kernel/src/application_fact.rs:77:4
   |
77 | fn error(reason: impl std::fmt::Display) -> StoreError {
   |    ^^^^^

error: function `build` is never used
  --> crates/ekr-kernel/src/application_fact.rs:82:15
   |
82 | pub(crate) fn build(input: &BuildInputs<'_>) -> Result<FactStepOutcome, StoreError> {
   |               ^^^^^

error: function `build_using` is never used
  --> crates/ekr-kernel/src/application_fact.rs:85:4
   |
85 | fn build_using(
   |    ^^^^^^^^^^^

error: function `selected_evidence` is never used
   --> crates/ekr-kernel/src/application_fact.rs:109:4
    |
109 | fn selected_evidence(input: &BuildInputs<'_>) -> Result<Vec<EvidenceId>, StoreError> {
    |    ^^^^^^^^^^^^^^^^^

error: function `support` is never used
   --> crates/ekr-kernel/src/application_fact.rs:153:4
    |
153 | fn support(
    |    ^^^^^^^

error: function `wire_time` is never used
   --> crates/ekr-kernel/src/application_fact.rs:177:4
    |
177 | fn wire_time(time: Timestamp) -> Result<w::EssTimestamp, StoreError> {
    |    ^^^^^^^^^

error: function `render` is never used
   --> crates/ekr-kernel/src/application_fact.rs:182:4
    |
182 | fn render(
    |    ^^^^^^

error: function `frozen` is never used
   --> crates/ekr-kernel/src/application_fact.rs:272:4
    |
272 | fn frozen(
    |    ^^^^^^

error: function `verify_structure` is never used
   --> crates/ekr-kernel/src/application_fact.rs:299:15
    |
299 | pub(crate) fn verify_structure(
    |               ^^^^^^^^^^^^^^^^

error: function `verify_semantic` is never used
   --> crates/ekr-kernel/src/application_fact.rs:320:15
    |
320 | pub(crate) fn verify_semantic(
    |               ^^^^^^^^^^^^^^^

error: could not compile `ekr-kernel` (lib) due to 18 previous errors
```

Exact-file rustfmt --edition 2021 --check: exit 0. git diff --check: exit 0. Builds used the coordinator-authorized exclusive sequential shared target and one job; disk floors were checked, with 12 GiB persistent and 7.7 GiB tmpfs available. Cargo lane returned before packaging.

5. Boundaries and deferred verification

- Input read, source documents, review authority, and evidence correspondence are caller-authenticated. The pure fixture injects canonical Evidence rows to test support rendering; it is not a source-retention/provenance admission test.
- Structural verification deliberately does not re-resolve aliases or source values. It checks exact selected membership, payload/digests, operation shape, canonical root, identities, mapped support, derivations and timestamps. A test demonstrates a changed value is structurally consistent but rejected by semantic verification. Only the first guarded Propose read proves that value's semantic mapping.
- Builder mints identities only after resolution/support succeed. Blocked resolution and invalid selections/correspondence are tested with a panicking allocator. Derivation IDs are fresh canonical UUIDs, never content hashes, and verification reuses the frozen IDs.
- The input elected_at must be the retained step election time when verifying. No revision is guessed from that timestamp. An unrelated revision change remains acceptable, whereas changed alias resolution refuses semantic verification.
- No persistence, authorization, publication, residual review, successor attempt, runtime dispatch, CLI/SDK, ESS, generated model, AEP, commit, or publication change. Parent handles integration, independent review and full gates.

6. Outside-worktree writes

Only the assigned private application-fact evidence directory and coordinator-authorized shared Cargo build area were written. This publishable report intentionally redacts personal absolute prefixes. Full exact private artifact paths are in outside-paths.txt alongside this report; raw logs remain unmodified. The prior mapping handoff remains separate and unchanged.
