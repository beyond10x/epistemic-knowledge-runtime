# Coordinated production acceptance

Owners: coordinator and the format, writer, shared-command and conformance implementors.

Everything below is a future acceptance requirement or static source dependency. None was compiled or executed by this preparation. Existing behavioral checks must remain effective; a stale textual assertion may be replaced only with a stronger check of the new contract, never an exclusion that makes new declarations invisible.

Correction status, 2026-09-22: F3 object-version dispatch and F4 absent-revision refusal are
declared in the new scratch candidate. F1 Commit retry and F2 Seed retry are settled in DESIGN,
but the ESS0.28 command declaration remains blocked by the measured subject/refusal capabilities.
The compiler-valid partial candidate is not activation-ready. Do not claim its generated suite
proves either retry. Preserve the original preparation and independent review beside this update.

## Existing projection checks that must move with the patch

| source | exact existing case | required coordinated change |
|---|---|---|
| crates/ekr-graph/tests/domain_projection.rs | every_enumeration_the_domain_declares_is_carried_variant_for_variant | Preserve coverage of actual Rust enums; cover AssessmentKind, AssertionLifecycleKind and CanonicalValueKind projections without dropping frozen old enums. Explicitly distinguish new format enum from old validation enum. |
| same | the_crate_quotes_the_domains_own_sentence_about_validation_state_payloads | Replace the obsolete absence-of-carriers/comment quotation contract with every-kind payload-presence/absence and round-trip projection checks. Do not keep a test asserting the gap remains. |
| same | every_declaration_of_the_domain_is_carried_field_for_field | Update PROJECTIONS/FUSIONS for assessment/lifecycle, canonical_bytes, complete record shapes and outer collections. Require every declared field to resolve to the real owner/type projection. No blanket exception for every new *Projection name. |
| crates/ekr-ontology/tests/domain_projection.rs | the_domain_carries_node_ref_and_enum_parameters_and_no_other_compound_kind | Replace the pinned partial-projection claim with recursive List/Record/NodeRef/Enum parameters, operation arguments and complete property declaration coverage; update matching source documentation. |
| same | every_timestamp_the_domain_declares_is_carried_as_a_timestamp | Include SchemaVersionRecord.created_at in actual-field coverage; new named types must not make the scanner silently skip old/new declarations. |
| crates/ekr-store/tests/domain_projection.rs | every_event_the_crate_writes_is_declared_by_the_domain | Preserve bidirectional event inventory after the atomic blob path; metadata events remain actual persisted events. |
| same | every_event_the_crate_writes_carries_the_fields_the_domain_declares | Read actual ObjectStored/schema2 metadata bodies without bytes, verify the exact atomic blob binding and byte_len/content_hash separately, keep ObjectRetentionRaised/schema1, and freeze inline ObjectStored/schema1 for verification/migration. Unknown versions and old/new shape mismatches refuse. A dummy absent event is not evidence. |
| crates/ekr-graph/tests/revision_events.rs | every_variant_carries_its_declared_index_and_domain_name | Preserve the six names/indices; distinguish event envelope fields from payload projection. |
| same | the_declaration_order_of_the_variants_equals_their_numbering | Preserve original numbering, including frozen old family; event/2 does not rename payload kinds. |
| same | no_two_variants_share_an_encoding; the_variant_marker_and_not_the_payload_is_what_separates_two_events; an_event_encodes_as_a_function_of_its_value | Add format/EventId/record_hash contribution and same-occurrence/different-content refusal; do not replace the legacy expected bytes with new encodings. |
| crates/ekr-kernel/tests/validation.rs | the_eleven_operation_numbers_are_the_domains_and_the_declarations | Current operation family becomes twelve with append-only SupersedeAssertion index11; frozen original family stays eleven. Retraction's new reason contributes at existing index5 only under the new format. |
| same | unsupported_schema_changes_and_merges_refuse_explicitly | Retain these refusals, including typed named ontology payloads in ESS; representability does not make schema evolution a P1 capability. |
| crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs | the_domain_requires_a_recorded_from_and_the_crate_cannot_omit_one | Keep the mandatory recorded_from checks while extending independent lifecycle and superseded historical reads. |

GraphDocument/seed representations presently live across store and kernel even though their graph fields have graph-domain homes. The existing source-only graph guard must not fake a graph-crate carrier or introduce an upward Rust dependency merely to satisfy its table. Put executable complete document/receipt projection checks in the actual owning store/kernel tests, and make the domain guard's explicit owner mapping prove every declaration has one corresponding check. Fundamental identifier types remain ekr-core; graph never embeds kernel receipts/issues.

Source documentation coupled to these tests includes crates/ekr-graph/src/node.rs, assertion.rs and value.rs, crates/ekr-ontology/src/value.rs, and crates/ekr-kernel/src/transaction.rs. The current statements denying ESS recursive/byte support, claiming seven combined assertion states, scalar properties, eleven current operation variants or JSON-safe proposal hashing must change with the implementation, not by silently relaxing guards.

## Existing seed and storage acceptance that must remain meaningful

In crates/ekr-kernel/tests/seed.rs, update the versioned fixtures and new envelope constructor together with:
an_evidence_seed_reopens_with_identical_roots_fields_and_retained_bytes,
reopen_checks_full_ontology_and_execution_context,
repeated_initialization_preserves_the_lineage_and_writes_no_second_object,
concurrent_independent_handles_publish_exactly_one_seed_and_no_losing_object,
legacy_and_tampered_seed_envelopes_are_preserved_but_never_admitted,
seed_decoding_refuses_unknown_semantic_fields_at_every_record_boundary,
initialization_reuses_exact_cached_seed_bytes_atomically,
seed_multiplicity_and_property_types_are_checked,
legitimate_record_keys_remain_data_in_strict_seed_decoding,
and the_versioned_minimal_yaml_fixture_initializes_both_real_providers.

Retain all seed reference/provenance/bootstrap-actor refusals, Float refusals and positive controls. Update the old scalar multiplicity expectation to the explicit outer collection without flattening a List value. Existing unsupported legacy input refusals now need the exact legacy-verifier/migration distinction; normal new ingestion must still refuse them.

crates/ekr-store/tests/seed_object_integrity.rs has
sqlite_seed_objects_verify_address_length_and_retention_before_admission and
file_seed_objects_verify_address_length_and_retention_before_admission.
They must inspect the real atomic object/blob binding, address, length and retention before kernel admission, rather than tampering with a removed inline bytes field.

crates/ekr/tests/story_contract.rs::crate_dependency_edges_match_the_story,
only_the_kernel_implements_the_commit_authority and
seed_admission_is_kernel_owned_and_the_commit_api_lends_no_writer remain the kernel boundary.
Update the authority signature recognizer if needed without expanding implementation authority below the kernel or lending a raw writer. Do not hand-edit AEP expectations.

## Proposed new runtime cases

Names below are proposed identifiers, not already-existing passing tests.

| owner | proposed case | observable assertion |
|---|---|---|
| format/kernel | transaction_document_retains_nan_bytes_until_named_type_refusal | Exact UTF-8 YAML bytes, including whitespace and NaN spelling, are identical before/after Proposed restart. No canonical transaction/operations hash exists. Real Validate rejects with the named Float/type issue and retains Rejected. |
| shared parser | transaction_document_refuses_unknown_duplicate_multiple_and_over_limit_inputs | Unknown format/fields/nested fields, duplicate semantic/map keys, multiple YAML documents, invalid UTF-8, empty operations and each selected limit boundary refuse before Proposed. Positive Record-key and exact-boundary controls remain. |
| graph projection | every_assessment_and_lifecycle_payload_projects_without_loss | Every assessment and lifecycle case retains precisely its kind's payload. Reject contradictory/extra optional fields; Accepted validators survive Retraction/Supersession. |
| graph projection | canonical_value_bytes_are_exact_and_float_is_not_projected | All ten canonical variants and nested List/Record projections match immutable encoder vectors; kind disagrees with bytes refuses; transient Float remains only the original proposal document. |
| ontology/kernel | complete_ontology_root_changes_for_every_semantic_field | Mutate each declaration field, including unused node/edge/property definitions and recursive operation arguments. The root changes or invalid declaration refuses. Empty ontology has a real defined root. |
| authority/kernel | registered_authority_and_exact_profile_survive_restart | Host registry/profile retained; capability/name/validator/check-list changes affect root or refuse. Unknown/substituted startup state cannot rewrite history. No P5 capability meaning is inferred. |
| kernel/store | seed2_returns_its_original_result_after_head_advances | Shared Seed handler parses the same full Seed2 input and checks actual BootstrapContext plus host authority anchor before allocating an occurrence/time. Retry after restart and another commit returns its own Root0, original receipt, occurrence and committed_at with unchanged objects/events. Different YAML whitespace alone still matches. Different parsed input or actual anchor/context returns AlreadySeeded with no writes. |
| shared handler/kernel | committed_transaction_retry_preserves_its_own_result_and_refuses_other_states | Repeat public Commit(transaction_id) after success, restart and a later head advance. Return the original retained CommitReceiptV1/result with no event, new timestamp or changed transaction fields. Proposed/Rejected/Stale retain TransactionStateConflict. |
| kernel/store | missing_validation_revision_keeps_proposal_without_rejection | Proposed transaction plus absent against returns RevisionNotFound with identical persisted objects/events and transaction still Proposed. No requested basis or rejection receipt is fabricated. Existing older revision validates, then Commit records Stale if the head has advanced. |
| store/providers | object_event_dispatch_keeps_explicit_versions_and_binding_integrity | New metadata-only ObjectStored is schema2, unchanged ObjectRetentionRaised is schema1, original inline ObjectStored/schema1 remains frozen verification/migration-only. Refuse unknown versions, metadata/schema1, inline/schema2, unknown fields, missing/wrong blob and address/length mismatches through actual provider readback with source unchanged. |
| kernel/store | all_six_occurrences_resolve_strict_complete_records | Each actual provider event resolves its mandated strict payload and agrees on IDs/hashes/counts/root fields. Missing record/blob, mismatched kind/version/id/address, duplicate key or unknown semantic field refuses. |
| kernel | validation_material_binds_prior_receipt_profile_and_full_transaction | Equal revision number/knowledge root is insufficient; changing any complete basis, prior time/context receipt, proposer, operation, evidence set or validator set invalidates the receipt. Distinguish value vs payload domains with fixed vectors. |
| writer/providers | changed_knowledge_survives_fresh_process_real_kernel_restart | Both real providers: seed, new node plus evidence-backed claim, validate, apply, atomic publish, fresh-process restart, query actual added values; real ontology/agent roots and knowledge hash change. |
| writer/graph | outer_multiplicity_preserves_order_duplicates_and_inner_list | Many retains every outer value/order/duplicate; One accepts one List including empty inner List; empty outer stored collection refuses; validated optional clear removes key. |
| writer/graph | supersession_and_retraction_preserve_assessment_and_bitemporal_answers | Run §65 latest-revision valid_at before/after boundary and earlier-revision reconstruction; retraction excludes all valid times only from its revision onward; acceptance/evidence/reason retained. |
| writer | admitted_operation_permutations_have_equal_post_state | Permutations yield same admission and graph; explicit AddAssertion+supersession works in one candidate; contradictory lifecycle operations/refusing replacement/cycle fail. Canonical transaction hashes may differ with vector order. |
| writer/providers | occurrence_retry_and_unknown_outcome_reconcile_exact_results | A retry preserves EventId/bytes/times and returns its own result after head moves; identity reuse with different bytes refuses; unknown publication outcome resolves exact occurrence before retry. |
| writer/providers | unrelated_stream_advance_retries_but_canonical_advance_stales | Object/noncommit event positions do not impersonate canonical revision. Competing canonical commit records full Stale and no canonical mutation by loser. |
| writer/providers | interrupted_atomic_publication_has_no_visible_partial_decision | Faults at actual provider boundaries leave either complete blobs+metadata or no visible occurrence, for success/refusal/reopen paths, on both providers. |
| writer/kernel | forged_receipt_and_missing_history_refuse_without_seed_fallback | Alter actual stored records/authority/basis/result/time/evidence or remove required history. Real reopen refuses precisely; no in-memory allowlist, false latest seed or fabricated accepted metadata. |
| writer/kernel | trusted_times_are_retained_non_decreasing_and_bound_assertions | submitted<=validated<=committed, nondecreasing commits, equal timestamps allowed with revision ordering, affected recorded_from bounds; retry/replay do not resample clock. |
| conformance | retained_transactions_include_every_terminal_state_after_restart | Transactions comes from real Proposed/Validated/Committed/Rejected/Stale records and returns captured identities/counts/basis/hashes after independent reopen. |
| conformance | generated_paths_and_role_bindings_reach_real_handlers | Exact synthesized relative paths and against=1 remain unchanged; host actors bind actual documents, no forged proposer or adapter-local result map. Actual revision1 setup uses a real validated commit. |

Use fixed final-format fixture vectors for authored hash/count/proposer/receipt checks. A changed reported hash must fail those authored checks. The old original-format vectors remain immutable and retain their original payload-domain validation hash. New current-format cases do not regenerate historical constants through current codecs.

## Conformance arrangement and future mutations

Re-synthesize the exact final declarations; the original 30-scenario count is not a permanent cap.
Until F1/F2 are representable, the partial candidate's suite is evidence of compiler behavior only:
its inherited Committed refusal is a known contradiction, and a Seed repeat branch is absent.
Authored scenarios add semantics the corrected generated suite cannot express; they must not
silently replace contradictory generated obligations. A fixture manifest may contain scenario-id
to concrete synthetic document/setup facts, never expected result objects to return. Stage only
exact admitted relative regular-file paths under a temporary allowed fixture root; reject
absolute/traversal/symlink escapes. Seed's AlreadySeeded fixture must submit genuinely different
parsed input or actual trusted anchor/context. Transaction/snapshot scenarios reach revision1 by
real seed+validated commit; Explain's exact generated identity is genuinely present or absent.

For rejected Validate, stage a structurally admissible bad reference before Propose; an external outcome control cannot replace the retained proposal afterward. For stale Commit, perform another real canonical commit after validation. Use observation cursors to separate setup events while keeping all state real. Bind roles to independently configured host identities; proposal and AddAssertion.proposed_by must match the actual submitting actor.

For revision-not-found Validate, create a real Proposed transaction and leave the exact requested
revision absent; observe the proposal and object/event counts before and after. An injected
external label must not fabricate this state. For seed and commit retries, retain the original
real return values outside the runtime and compare the subsequent returned values against them.
Mutations must make those checks fail if the runtime resamples time, allocates another occurrence,
returns latest head, emits duplicate facts, accepts a different seed/anchor, fabricates a missing
basis or infers object format from a missing bytes field.

Future mutations must demonstrate real failure: remove reference validation, remove stale detection, erase a terminal Transactions row, change a retained count to zero, forge a reported hash, remove one root field, allow identity reuse with altered bytes, bypass actual authority replay, or publish metadata without its blob. A compiler-generated reference implementation or expected-outcome switch cannot provide this evidence.

Re-synthesize the exact committed final specification, add reviewed authored scenarios beside generated obligations, execute its immutable suite through the pinned real Runner/target, retain suite plus report/2, and run the full repository gate. No scenario, mutation or gate is claimed complete here.
