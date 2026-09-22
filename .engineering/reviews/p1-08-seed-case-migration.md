# Kernel seed boundary: original case → replacement evidence

These substitutions follow story:kernel-validated-seed. Direct unchecked conversion was removed
from store source; provider tests use an explicitly labeled substitute authority. Every semantic
replacement below is in crates/ekr-kernel/tests/seed.rs and runs SQLite and File. The old store
lane loses 13 cases; the aggregate touched-package lane rises 203 → 217 (25 kernel seed cases,
2 provider object-integrity cases, minus those 13).

| Original source / test | Replacement test(s) and assertion |
| --- | --- |
| membrane_boundary.rs::a_candidate_node_and_a_canonical_node_write_the_same_document | Remains in its original file, same byte-equality assertion. |
| membrane_boundary.rs::the_crossing_refuses_a_float_and_says_which_property_carried_it | floats_are_refused_in_seed_nodes_edges_and_assertion_objects — inadmissible-value and the actual property ID in the refusal; invalid object/event absent. |
| membrane_boundary.rs::the_crossing_refuses_a_float_in_an_assertions_object | floats_are_refused_in_seed_nodes_edges_and_assertion_objects — inadmissible-value on assertion object. |
| membrane_boundary.rs::the_crossing_refuses_a_float_on_an_edge | floats_are_refused_in_seed_nodes_edges_and_assertion_objects — inadmissible-value on edge property. |
| membrane_boundary.rs::a_canonical_graph_round_trips_through_its_document | an_evidence_seed_reopens_with_identical_roots_fields_and_retained_bytes — all GraphDocument fields equal after actual kernel acceptance, except actual validator attribution which is checked explicitly; reopen and replay equal the whole canonical graph. |
| membrane_boundary.rs::an_empty_document_crosses | empty_bootstrap_does_not_make_empty_transactions_valid; the_versioned_minimal_yaml_fixture_initializes_both_real_providers — admitted revision zero, ordinary empty transaction still refused. |
| membrane_boundary.rs::the_crossing_refuses_a_record_filed_under_another_records_id | every_seed_entity_map_checks_key_and_root_identity — seed-misfiled-entity. |
| membrane_boundary.rs::the_crossing_refuses_a_record_that_belongs_to_another_graph_root | every_seed_entity_map_checks_key_and_root_identity — seed-misrooted-entity. |
| membrane_boundary.rs::every_map_of_a_document_is_checked_and_not_only_the_first | every_seed_entity_map_checks_key_and_root_identity — all node/edge/assertion keys and roots plus evidence filing, each against both backends. |
| membrane_boundary.rs::the_crossing_does_not_check_what_it_has_nothing_to_check_against | Its intentional old gap assertions now invert: canonical_seed_root_filing_schema_and_genesis_are_mandatory refuses any seed root parent and nonzero revision; seed_support_checks_missing_uncited_and_unsupported_evidence refuses unresolved evidence. Arbitrary created_at remains preserved by an_evidence_seed_reopens_with_identical_roots_fields_and_retained_bytes. |
| adversary2_membrane_bounds.rs::the_crossing_refuses_a_root_that_is_its_own_parent | canonical_seed_root_filing_schema_and_genesis_are_mandatory — seed-root-lineage, self-parent and unrelated predecessor separately. |
| adversary_membrane_and_schema.rs::the_crossing_refuses_a_document_whose_root_declares_transient_space | canonical_seed_root_filing_schema_and_genesis_are_mandatory — seed-space. |
| adversary_membrane_and_schema.rs::a_fold_does_not_return_canonical_state_rooted_in_a_transient_root | legacy_and_tampered_seed_envelopes_are_preserved_but_never_admitted — raw independently opened provider publishes transient envelope; real kernel replay refuses seed-space and original bytes remain retained. |
| adversary_membrane_and_schema.rs::a_fold_does_not_retype_stored_state_under_a_schema_version_it_was_not_written_against | legacy_and_tampered_seed_envelopes_are_preserved_but_never_admitted — seed-schema-version for an injected conflicting root; reopen_checks_full_ontology_and_execution_context additionally rejects changed ontology content under the SAME schema ID. |
| adversary_p1_06_reference_from_bytes.rs::a_document_naming_an_edge_target_it_does_not_carry_becomes_canonical_state_unrefused | Old explicitly temporary acceptance-of-defect assertion is replaced by a_store_cannot_turn_dangling_document_bytes_into_canonical_state in that file (NoSeedAuthority); a_seed_with_a_dangling_edge_is_refused_by_both_backends and named_seed_refusals_write_neither_the_object_nor_the_revision prove real kernel unresolved-node and no-write outcomes. |
| fold_rules.rs::a_store_with_no_commit_authority_refuses_to_say_what_canonical_state_is | Same case now asserts earlier NoSeedAuthority from fold, replay AND head. Its authoritative provider control still reaches revision one. |
| review_p1_invariant_one_at_the_store.rs two transaction forgery/staleness cases | Remain. Seed-only substitute authority admits fixture bytes but attests no commit in first case; the second separately uses its existing attestation policy. Transaction tests do not pass solely because seed authority is absent. |
| kernel/tests/commit_path.rs all five cases | Remain, now initialized via Commit::seed; raw forgery uses an independently opened provider handle instead of a production writer accessor. |

No ignored tests or disabled assertions were introduced. The two files whose complete behavior
moved to the kernel were removed instead of keeping dead wrappers around an API that no longer
exists. Provider authority helpers explicitly state that their graph construction is fixture
mechanics and is not semantic admission evidence.
