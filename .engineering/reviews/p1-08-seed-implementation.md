unit:                   story:kernel-validated-seed — Initialize and reopen seeds through kernel validation
verdict:                green
cases:                  executed 203→217, red 5
origin:                 n/a
wrote-outside-worktree: <cache>/ekr-completion-20260922/seed/; <cache>/b10x-target/ekr-p1-08-seed/
needs-coordinator:      yes — integration/full gate/adversary; review scoped AGENTS wording (agents.patch)

The store membrane_boundary lane decreased 10→1; adversary2_membrane_bounds 1→0 and
adversary_membrane_and_schema 3→0. Their semantic assertions moved to the real kernel boundary,
rather than being ignored. The exact original testcase→replacement/refusal map is
[case-migration.md](p1-08-seed-case-migration.md).
The aggregate package suite increased 203→217; no test was ignored or skipped.

## 1. Unit, acceptance and scope

Initialize versioned seed input through kernel invariant validation and publish a retained envelope
plus Seeded atomically; invalid input writes neither object nor revision, both providers replay
through real kernel authority, and initialization cannot replace a lineage.

Opening HEAD: 4032d00. Tree: <worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed.
Branch: codex/ekr-p1-08-seed. No AEP/ESS writes, commits, publication or cleanup by implementor.
Coordinator changed systems/ekr/domains/kernel.yaml comments in this tree and owns that diff.
Managed lease codex-ekr-p1-08-seed-implementor was renewed during work and released at handback.
Compiler/test processes are complete; last inspected free disk 25 GB.

Representations:
- SeedDocument { format: "ekr-seed/1", ontology: OntologyDocument, graph: GraphDocument,
  evidence_payloads: BTreeMap<ContentHash, Vec<u8>> }.
- Private SeedEnvelope { format: "ekr-seed-envelope/1", input: SeedDocument,
  context: BootstrapContext { operator, validator } }.
- The next persisted-contract story must bump both formats before interpreting changed nested
  assertion/graph shapes; no separate graph or event wrapper was introduced in this unit.

The kernel constructs a private ValidatedSeed only after checking ontology genesis, root genesis,
filing, full declared types and semantics, initial lifecycle, proposed-only assertions, distinct
actual operator/validator, attributed assertions/evidence, admissible HumanStatement support and
verified retained bytes. Explicit bootstrap mode runs the same deterministic invariant validators
without sealing a transaction against a fabricated predecessor. Empty bootstrap skips only the
transaction-specific nonempty structural rule.

Store replay asks the existing CommitAuthority port for seed admission on every read/reopen.
No accepted-seed hash set exists. Full loaded ontology equality and runtime context equality are
checked, and original input is retained. Legacy raw graph documents preserve their bytes and refuse
SeedMigrationRequired. Commit exposes head/snapshot/replay/content reads; store() is removed.

AtomicEventStore::append_group uses Expected::NoStream on the lineage in every path, including
object contention retries. Object reuse compares exact bytes; cache promotion is part of the
atomic group. Typed conflicts are classified before provider error flattening. Stored object
address, byte length and retention metadata are checked before seed admission.

Scope confirmation:

| Assigned path | Inspection / resulting change |
| --- | --- |
| AGENTS.md | Existing ordinary commit invariant required bootstrap qualification; scoped wording changed, coordinator reviews final normative wording. |
| crates/ekr-graph/src/assertion.rs | Seed-reachable temporal field helpers, validation state and assertion records now reject unknown semantic fields. |
| crates/ekr-graph/src/canonical.rs | Corrected obsolete unvalidated seed-path comments; no graph representation change. |
| crates/ekr-graph/src/edge.rs | Seed-reachable Edge record strictness. |
| crates/ekr-graph/src/evidence.rs | Evidence and EvidenceSource strictness; arbitrary user record keys remain legal. |
| crates/ekr-graph/src/node.rs | Node record strictness. |
| crates/ekr-graph/src/root.rs | GraphRoot record strictness. |
| crates/ekr-kernel/src/commit.rs | Actual authority context, seed admission/replay and read API; raw writer removed. |
| crates/ekr-kernel/src/lib.rs | Exports approved seed entry types. |
| crates/ekr-kernel/src/seed.rs (inferred) | Confirmed absent on opening tree; added bootstrap capability/admission, complete retained input, strict formats, lossless narrowing. |
| crates/ekr-kernel/src/validate | Confirmed seven shared invariant implementations; only mod.rs adds private explicit bootstrap check mode. |
| crates/ekr-kernel/tests/commit_path.rs | Existing five cases preserved through real kernel seed API. |
| crates/ekr-kernel/tests/fixtures/seed-minimal.yaml (inferred) | Confirmed new fixture path; fixed-ID empty YAML control runs both providers. |
| crates/ekr-kernel/tests/seed.rs (inferred) | Confirmed absent; behavioral red first, then 25 acceptance cases through real providers. |
| crates/ekr-store/src/eventlog.rs | Production replay crossing and atomic append port verified against pinned eventlog; changed replay admission, metadata checks, NoStream initialization. |
| crates/ekr-store/src/lib.rs | Named store refusals and initialization port export. |
| crates/ekr-store/src/log.rs | Existing CommitAuthority extended fail-closed; read seed bytes and atomic initialization ports. |
| crates/ekr-store/src/snapshot.rs | Verified unchecked public conversion was the crossing; removed construction/narrowing from store source, preserved DTO serialization. |
| crates/ekr-store/tests | Explicit substitute authorities for provider tests; semantic cases mapped to kernel; independent provider corruption tests. |
| crates/ekr/tests/story_contract.rs | Existing dependency/authority guards preserved; added seed ownership/no raw writer check. |
| systems/ekr/domains/kernel.yaml | Read only by implementor. Coordinator-owned comment clarification copied into tree; included in inventory. |

All three inferred paths were confirmed before implementation. No additional repository scope was
required. The suggested atomic mechanism was not weakened: independent provider-handle concurrency
cases observe exactly one success and one AlreadySeeded, with the losing object absent.

## 2. Actual diff inventory

Tracked-file diff (new files are shown separately because git diff --stat excludes untracked files):
```
 AGENTS.md                                          |   8 +-
 crates/ekr-graph/src/assertion.rs                  |   4 +
 crates/ekr-graph/src/canonical.rs                  |  20 +-
 crates/ekr-graph/src/edge.rs                       |   1 +
 crates/ekr-graph/src/evidence.rs                   |   2 +
 crates/ekr-graph/src/node.rs                       |   1 +
 crates/ekr-graph/src/root.rs                       |   1 +
 crates/ekr-kernel/src/commit.rs                    | 101 ++++++-
 crates/ekr-kernel/src/lib.rs                       |   2 +
 crates/ekr-kernel/src/validate/mod.rs              |  18 ++
 crates/ekr-kernel/tests/commit_path.rs             |  66 ++---
 crates/ekr-store/src/eventlog.rs                   | 175 +++++++++--
 crates/ekr-store/src/lib.rs                        |  20 +-
 crates/ekr-store/src/log.rs                        |  26 ++
 crates/ekr-store/src/snapshot.rs                   | 314 +-------------------
 .../ekr-store/tests/adversary2_event_vocabulary.rs |   7 +
 .../ekr-store/tests/adversary2_membrane_bounds.rs  |  41 ---
 .../tests/adversary_membrane_and_schema.rs         | 143 ---------
 .../tests/adversary_objects_and_append.rs          |   1 +
 .../tests/adversary_p1_06_reference_from_bytes.rs  | 120 ++------
 crates/ekr-store/tests/fixture/mod.rs              |  51 +++-
 crates/ekr-store/tests/fold_rules.rs               |  24 +-
 crates/ekr-store/tests/lineage/mod.rs              |   7 +
 crates/ekr-store/tests/membrane_boundary.rs        | 327 +--------------------
 .../tests/review_p1_invariant_one_at_the_store.rs  |  11 +-
 crates/ekr/tests/story_contract.rs                 |  22 ++
 systems/ekr/domains/kernel.yaml                    |   5 +-
 27 files changed, 495 insertions(+), 1023 deletions(-)
```

New file line counts:
```
  434 crates/ekr-kernel/src/seed.rs
  985 crates/ekr-kernel/tests/seed.rs
   22 crates/ekr-kernel/tests/fixtures/seed-minimal.yaml
  128 crates/ekr-store/tests/seed_object_integrity.rs
 1569 total
```

Full working-tree inventory:
```
 M AGENTS.md
 M crates/ekr-graph/src/assertion.rs
 M crates/ekr-graph/src/canonical.rs
 M crates/ekr-graph/src/edge.rs
 M crates/ekr-graph/src/evidence.rs
 M crates/ekr-graph/src/node.rs
 M crates/ekr-graph/src/root.rs
 M crates/ekr-kernel/src/commit.rs
 M crates/ekr-kernel/src/lib.rs
 M crates/ekr-kernel/src/validate/mod.rs
 M crates/ekr-kernel/tests/commit_path.rs
 M crates/ekr-store/src/eventlog.rs
 M crates/ekr-store/src/lib.rs
 M crates/ekr-store/src/log.rs
 M crates/ekr-store/src/snapshot.rs
 M crates/ekr-store/tests/adversary2_event_vocabulary.rs
 D crates/ekr-store/tests/adversary2_membrane_bounds.rs
 D crates/ekr-store/tests/adversary_membrane_and_schema.rs
 M crates/ekr-store/tests/adversary_objects_and_append.rs
 M crates/ekr-store/tests/adversary_p1_06_reference_from_bytes.rs
 M crates/ekr-store/tests/fixture/mod.rs
 M crates/ekr-store/tests/fold_rules.rs
 M crates/ekr-store/tests/lineage/mod.rs
 M crates/ekr-store/tests/membrane_boundary.rs
 M crates/ekr-store/tests/review_p1_invariant_one_at_the_store.rs
 M crates/ekr/tests/story_contract.rs
 M systems/ekr/domains/kernel.yaml
?? crates/ekr-kernel/src/seed.rs
?? crates/ekr-kernel/tests/fixtures/
?? crates/ekr-kernel/tests/seed.rs
?? crates/ekr-store/tests/seed_object_integrity.rs
```

## 3. Behavioral red, verbatim

Initial four-case lane ran before implementation. Positive control passed; dangling endpoint,
undeclared type and caller Accepted verdict were accepted under real kernel authority by BOTH
providers. The first preliminary red.log stopped each loop at SQLite; red-both.log below aggregates
both backend outcomes and is the retained cross-provider evidence.

```sh
CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-08-seed CARGO_BUILD_JOBS=2 TMPDIR=<cache>/ekr-completion-20260922/seed cargo test -p ekr-kernel --test seed
# exit 101
```
```
   Compiling ekr-kernel v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.43s
     Running tests/seed.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/seed-db0d520306fccdc5)

running 4 tests
test a_valid_seed_has_a_positive_control ... ok
test a_seed_with_a_caller_verdict_is_refused_by_both_backends ... FAILED
test a_seed_with_an_undeclared_type_is_refused_by_both_backends ... FAILED
test a_seed_with_a_dangling_edge_is_refused_by_both_backends ... FAILED

failures:

---- a_seed_with_a_caller_verdict_is_refused_by_both_backends stdout ----

thread 'a_seed_with_a_caller_verdict_is_refused_by_both_backends' (4128742) panicked at crates/ekr-kernel/tests/seed.rs:164:5:
accepted a caller's verdict: [Sqlite, File]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- a_seed_with_an_undeclared_type_is_refused_by_both_backends stdout ----

thread 'a_seed_with_an_undeclared_type_is_refused_by_both_backends' (4128744) panicked at crates/ekr-kernel/tests/seed.rs:147:5:
accepted an undeclared type: [Sqlite, File]

---- a_seed_with_a_dangling_edge_is_refused_by_both_backends stdout ----

thread 'a_seed_with_a_dangling_edge_is_refused_by_both_backends' (4128743) panicked at crates/ekr-kernel/tests/seed.rs:138:5:
accepted a dangling edge: [Sqlite, File]


failures:
    a_seed_with_a_caller_verdict_is_refused_by_both_backends
    a_seed_with_a_dangling_edge_is_refused_by_both_backends
    a_seed_with_an_undeclared_type_is_refused_by_both_backends

test result: FAILED. 1 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.19s

error: test failed, to rerun pass `-p ekr-kernel --test seed`
```

Coordinator source audit found the ontology-genesis class: load() allows nonzero schema version or
a predecessor. Two added tests measured red on both providers before the genesis check was added:

```sh
CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-08-seed CARGO_BUILD_JOBS=2 TMPDIR=<cache>/ekr-completion-20260922/seed cargo test -p ekr-kernel --test seed a_seed_ontology
# exit 101
```
```
   Compiling ekr-kernel v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.28s
     Running tests/seed.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/seed-db0d520306fccdc5)

running 2 tests
test a_seed_ontology_cannot_claim_a_predecessor ... FAILED
test a_seed_ontology_cannot_claim_a_later_version ... FAILED

failures:

---- a_seed_ontology_cannot_claim_a_predecessor stdout ----

thread 'a_seed_ontology_cannot_claim_a_predecessor' (147237) panicked at crates/ekr-kernel/tests/seed.rs:177:5:
accepted an ontology predecessor: [Sqlite, File]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- a_seed_ontology_cannot_claim_a_later_version stdout ----

thread 'a_seed_ontology_cannot_claim_a_later_version' (147236) panicked at crates/ekr-kernel/tests/seed.rs:168:5:
accepted a non-genesis ontology: [Sqlite, File]


failures:
    a_seed_ontology_cannot_claim_a_later_version
    a_seed_ontology_cannot_claim_a_predecessor

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.17s

error: test failed, to rerun pass `-p ekr-kernel --test seed`
```

## 4. Verification commands, runner counts and verbatim output

Same package lane on opening tree and final code:
```sh
CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-08-seed CARGO_BUILD_JOBS=2 TMPDIR=<cache>/ekr-completion-20260922/seed cargo test -p ekr-graph -p ekr-store -p ekr-kernel
# opening exit 0, executed 203 (baseline.log)
# final exit 0, executed 217 (green.log)
```

Every binary count below comes from its runner summary. New lanes were absent from the base
command; removed lanes are the explicitly mapped semantic migrations above.
```
ekr_graph/unit: executed 0 → 0, exit 0
ekr_graph/adversary2_guard_bounds_and_ranges.rs: executed 4 → 4, exit 0
ekr_graph/adversary2_membrane_and_addresses.rs: executed 2 → 2, exit 0
ekr_graph/adversary_canonical_value_reach.rs: executed 3 → 3, exit 0
ekr_graph/adversary_p1_06_reference_markers.rs: executed 2 → 2, exit 0
ekr_graph/adversary_snapshot_and_assertion.rs: executed 4 → 4, exit 0
ekr_graph/canonical_value_and_assertion.rs: executed 18 → 18, exit 0
ekr_graph/domain_projection.rs: executed 3 → 3, exit 0
ekr_graph/evidence_and_observations.rs: executed 6 → 6, exit 0
ekr_graph/membrane.rs: executed 5 → 5, exit 0
ekr_graph/node_identity.rs: executed 3 → 3, exit 0
ekr_graph/review_p1_membrane_is_by_id.rs: executed 2 → 2, exit 0
ekr_graph/revision_events.rs: executed 5 → 5, exit 0
ekr_graph/snapshot_reads.rs: executed 7 → 7, exit 0
ekr_kernel/unit: executed 0 → 0, exit 0
ekr_kernel/adversary_membrane.rs: executed 5 → 5, exit 0
ekr_kernel/adversary_membrane_pass_two.rs: executed 4 → 4, exit 0
ekr_kernel/adversary_p1_07.rs: executed 5 → 5, exit 0
ekr_kernel/commit_path.rs: executed 5 → 5, exit 0
ekr_kernel/encoding_field_order.rs: executed 1 → 1, exit 0
ekr_kernel/validate_properties.rs: executed 5 → 5, exit 0
ekr_kernel/validation.rs: executed 49 → 49, exit 0
ekr_store/unit: executed 0 → 0, exit 0
ekr_store/adversary2_event_vocabulary.rs: executed 1 → 1, exit 0
ekr_store/adversary2_membrane_bounds.rs: executed 1 → 0, exit 0 (migrated; removed lane)
ekr_store/adversary2_retention_event_contract.rs: executed 2 → 2, exit 0
ekr_store/adversary_membrane_and_schema.rs: executed 3 → 0, exit 0 (migrated; removed lane)
ekr_store/adversary_objects_and_append.rs: executed 2 → 2, exit 0
ekr_store/adversary_p1_06_reference_from_bytes.rs: executed 1 → 1, exit 0
ekr_store/domain_projection.rs: executed 7 → 7, exit 0
ekr_store/fold_rules.rs: executed 18 → 18, exit 0
ekr_store/membrane_boundary.rs: executed 10 → 1, exit 0
ekr_store/providers.rs: executed 15 → 15, exit 0
ekr_store/review_p1_invariant_one_at_the_store.rs: executed 2 → 2, exit 0
ekr_graph/doc: executed 1 → 1, exit 0
ekr_kernel/doc: executed 2 → 2, exit 0
ekr_store/doc: executed 0 → 0, exit 0
ekr_kernel/seed.rs: executed 0 → 25, exit 0 (new lane)
ekr_store/seed_object_integrity.rs: executed 0 → 2, exit 0 (new lane)
```

Final complete runner output:
```
   Compiling ekr-kernel v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-kernel)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.40s
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
test a_float_at_any_depth_cannot_inhabit_a_canonical_value_and_the_refusal_names_it ... ok
test a_node_and_an_edge_property_carry_only_an_admissible_value ... ok
test an_admissible_value_round_trips_through_the_newtype ... ok
test no_two_observations_that_differ_share_a_content_address ... ok
test every_variant_of_every_sum_type_opens_with_its_own_marker ... ok
test no_two_pieces_of_evidence_that_differ_share_a_content_address ... ok
test no_two_support_links_that_differ_share_a_content_address ... ok
test graph_state_equal_in_every_field_hashes_equally ... ok
test every_field_of_a_node_and_an_edge_reaches_the_encoding ... ok
test no_two_nodes_that_differ_share_a_content_address ... ok
test no_two_edges_that_differ_share_a_content_address ... ok
test every_field_of_evidence_state_reaches_the_encoding ... ok
test no_two_assertions_that_differ_share_a_content_address ... ok
test the_conversion_refuses_exactly_where_the_ontology_says_it_must ... ok
test every_field_of_an_assertion_reaches_the_encoding ... ok
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
test every_evidence_source_answers_the_flat_fields_the_domain_declares ... ok
test a_blob_observation_carries_its_media_type_and_length ... ok
test confidence_outside_its_declared_range_is_not_constructible ... ok
test every_observation_form_answers_its_kind_and_its_content_hash ... ok
test evidence_carries_its_provenance ... ok
test support_links_one_assertion_to_one_piece_of_evidence ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/membrane.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/membrane-232536dd185bd11a)

running 5 tests
test a_canonical_reference_is_the_only_canonical_dependency ... ok
test canonical_state_resolves_a_canonical_reference ... ok
test a_candidate_and_a_canonical_node_that_share_an_id_do_not_resolve_alike ... ok
test transient_state_may_depend_on_canonical_state_and_on_its_own ... ok
    Checking ekr-graph-tests v0.0.0 (<cache>/b10x-target/ekr-p1-08-seed/tests/trybuild/ekr-graph)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s


test tests/compile_fail/canonical_dependency_is_sealed.rs ... ok
test tests/compile_fail/canonical_graph_rejects_a_transient_ref.rs ... ok
test tests/compile_fail/canonical_ref_cannot_target_a_transient_type.rs ... ok
test tests/compile_fail/transient_state_has_no_content_address.rs ... ok


test the_membrane_is_a_set_of_build_failures ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s

     Running tests/node_identity.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/node_identity-cc0577f9dcabf6fc)

running 3 tests
test two_nodes_that_share_a_name_do_not_share_an_id ... ok
test the_lifecycle_state_of_a_node_is_a_property_and_not_its_identity ... ok
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
test every_variant_carries_its_declared_index_and_domain_name ... ok
test an_event_encodes_as_a_function_of_its_value ... ok
test no_two_variants_share_an_encoding ... ok
test the_variant_marker_and_not_the_payload_is_what_separates_two_events ... ok
test the_declaration_order_of_the_variants_equals_their_numbering ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/snapshot_reads.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/snapshot_reads-7c27a9f0cbcff8a8)

running 7 tests
test a_snapshot_names_the_revision_it_reads ... ok
test the_handover_instant_belongs_to_exactly_one_of_them ... ok
test a_proposed_assertion_is_never_answered ... ok
test a_record_whose_transaction_time_is_closed_is_not_current ... ok
test the_historical_query_returns_alice ... ok
test valid_at_answers_alice_before_the_handover_and_bob_at_or_after_it ... ok
test valid_at_never_returns_a_retracted_or_superseded_assertion ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/ekr_kernel-dcebe1934f67cb0b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_membrane.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_membrane-f528e4a1cc1a1079)

running 5 tests
test an_assertion_whose_only_evidence_the_proposer_invented_is_refused ... ok
test an_add_assertion_over_an_id_canonical_state_already_holds_is_refused ... ok
test a_create_node_over_an_id_canonical_state_already_holds_is_refused ... ok
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

     Running tests/commit_path.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/commit_path-816fb6c18858336f)

running 5 tests
test a_commit_into_an_unseeded_lineage_is_refused ... ok
test a_transaction_validated_against_a_revision_the_lineage_moved_past_is_refused ... ok
test the_same_lineage_written_by_hand_does_not_advance_anything ... ok
test the_kernels_authority_stands_behind_exactly_what_it_validated ... ok
test a_validated_transaction_commits_and_the_lineage_advances ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running tests/encoding_field_order.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/encoding_field_order-2a7ec28608d4a18e)

running 1 test
test every_field_of_the_kernels_encodings_is_written_in_declaration_order ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/seed.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/seed-db0d520306fccdc5)

running 25 tests
test legitimate_record_keys_remain_data_in_strict_seed_decoding ... ok
test seed_decoding_refuses_unknown_semantic_fields_at_every_record_boundary ... ok
test a_seed_with_a_dangling_edge_is_refused_by_both_backends ... ok
test a_seed_ontology_cannot_claim_a_predecessor ... ok
test a_seed_ontology_cannot_claim_a_later_version ... ok
test a_seed_with_an_undeclared_type_is_refused_by_both_backends ... ok
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
test seed_lifecycle_is_the_declared_initial_state_and_is_preserved ... ok
test an_evidence_seed_reopens_with_identical_roots_fields_and_retained_bytes ... ok
test floats_are_refused_in_seed_nodes_edges_and_assertion_objects ... ok
test canonical_seed_root_filing_schema_and_genesis_are_mandatory ... ok
test legacy_and_tampered_seed_envelopes_are_preserved_but_never_admitted ... ok
test every_seed_entity_map_checks_key_and_root_identity ... ok
test seed_support_checks_missing_uncited_and_unsupported_evidence ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.08s

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
test a_transaction_with_no_operations_is_refused_by_the_structural_validator ... ok
test a_node_of_an_abstract_type_is_refused ... ok
test a_canonical_assertion_without_evidence_is_refused_by_the_provenance_validator ... ok
test a_merge_names_two_nodes ... ok
test a_reference_to_something_the_same_transaction_creates_resolves ... ok
test a_transaction_whose_validator_is_its_proposer_is_refused ... ok
test a_record_under_another_graph_root_is_refused ... ok
test a_value_that_fails_its_type_is_refused_by_the_type_validator ... ok
test a_valid_transaction_validates ... ok
test a_value_canonical_state_does_not_admit_is_refused_with_its_path ... ok
test an_edge_endpoint_of_the_wrong_type_is_refused ... ok
test a_type_the_ontology_already_declares_is_not_declared_again ... ok
test an_empty_property_assignment_still_requires_a_declared_property ... ok
test an_edge_cardinality_the_type_forbids_is_refused_by_the_cardinality_validator ... ok
test a_valid_transaction_validates_to_the_same_hash_twice ... ok
test an_evidence_id_the_transaction_declares_does_not_make_it_exist ... ok
test an_operation_the_type_does_not_declare_is_refused ... ok
test an_invoke_the_lifecycle_does_not_declare_is_refused_by_the_ontology_validator ... ok
test an_unresolvable_reference_is_refused_by_the_reference_validator ... ok
test an_identity_created_twice_in_one_transaction_is_refused ... ok
test competing_lifecycle_writes_are_refused ... ok
test an_unresolved_assertion_endpoint_does_not_acquire_an_unrelated_type_refusal ... ok
test an_invocation_carries_exactly_the_arguments_its_operation_declares ... ok
test an_identity_canonical_state_already_holds_is_not_created_again ... ok
test deleting_an_edge_referenced_by_a_new_or_retained_assertion_is_refused ... ok
test an_assertion_cannot_arrive_carrying_its_own_verdict ... ok
test applicable_opaque_property_constraints_refuse_all_node_write_paths ... ok
test opaque_edge_property_constraints_refuse_creation_and_assertions ... ok
test every_type_refusal_the_validator_can_make_is_reachable ... ok
test an_assertions_types_are_resolved_whatever_shape_its_object_has ... ok
test every_field_of_every_encoded_type_reaches_its_encoding ... ok
test property_cardinality_and_required_presence_are_refused ... ok
test competing_property_writes_are_refused_in_every_order ... ok
test property_assertions_check_type_objects_edge_properties_and_type_subjects ... ok
test every_kind_of_dangling_reference_is_refused ... ok
test the_encoding_writes_id_bearing_fields_in_declaration_order ... ok
test the_pipeline_runs_the_seven_deterministic_validators_in_order ... ok
test a_serialized_lifecycle_invokes_but_unsupported_sets_refuse_at_decode ... ok
test the_declared_evidence_set_is_the_evidence_the_assertions_cite ... ok
test the_eleven_operation_numbers_are_the_domains_and_the_declarations ... ok
test opaque_preconditions_and_emissions_are_not_silently_accepted ... ok
test property_cardinality_uses_the_candidate_node ... ok
test relation_assertions_check_both_endpoint_types_and_allow_inherited_types ... ok
test unsupported_schema_changes_and_merges_refuse_explicitly ... ok
test relation_assertions_require_compatible_node_endpoints ... ok
test two_transactions_that_differ_validate_to_different_hashes ... ok
test the_validation_hash_covers_the_revision_it_was_validated_against ... ok
test every_issue_code_the_kernel_can_raise_is_raised_by_a_case ... ok
    Checking ekr-kernel-tests v0.0.0 (<cache>/b10x-target/ekr-p1-08-seed/tests/trybuild/ekr-kernel)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s


test tests/compile_fail/validated_transaction_cannot_be_deserialised.rs ... ok
test tests/compile_fail/validated_transaction_has_no_constructor_outside_the_kernel.rs ... ok


test only_the_kernel_constructs_a_validated_transaction ... ok

test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s

     Running unittests src/lib.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/ekr_store-faf645b78e873b91)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary2_event_vocabulary.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary2_event_vocabulary-5c08c3becd8c2526)

running 1 test
test a_second_rejection_of_a_re_proposed_transaction_is_not_swallowed_as_a_retry ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running tests/adversary2_retention_event_contract.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary2_retention_event_contract-a82a827ca939b61d)

running 2 tests
test the_store_domain_declares_the_retention_raise_event_this_crate_writes ... ok
test the_retention_raise_in_stored_bytes_carries_the_fields_the_domain_declares ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/adversary_objects_and_append.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_objects_and_append-4988f0c4b1471b34)

running 2 tests
test storing_canonical_bytes_that_were_cached_earlier_records_them_as_canonical ... ok
test retrying_an_append_writes_the_same_event_once_rather_than_twice ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

     Running tests/adversary_p1_06_reference_from_bytes.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/adversary_p1_06_reference_from_bytes-720826f958dc140b)

running 1 test
test a_store_cannot_turn_dangling_document_bytes_into_canonical_state ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/domain_projection.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/domain_projection-26a37c3aca18a7fa)

running 7 tests
test the_derived_ordering_is_not_the_retention_ordering ... ok
test every_storage_class_has_its_own_retention_rank ... ok
test the_storage_classes_are_the_ones_the_domain_declares ... ok
test the_retention_ladder_is_the_one_section_thirty_seven_describes ... ok
test the_strongest_of_two_classes_does_not_depend_on_which_arrived_first ... ok
test every_event_the_crate_writes_is_declared_by_the_domain ... ok
test every_event_the_crate_writes_carries_the_fields_the_domain_declares ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/fold_rules.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/fold_rules-f25e2bdb78fb33f7)

running 18 tests
test evidence_is_addressed_apart_from_knowledge ... ok
test the_knowledge_root_is_a_function_of_the_graph_state ... ok
test an_empty_log_has_no_head_and_does_not_fold ... ok
test a_lineage_that_does_not_begin_at_a_seed_is_refused ... ok
test a_seed_naming_bytes_the_store_does_not_hold_is_refused ... ok
test a_commit_whose_validation_the_authority_stands_behind_advances_the_lineage ... ok
test validating_a_transaction_that_was_never_proposed_is_refused ... ok
test a_commit_of_a_transaction_that_was_never_validated_is_refused ... ok
test a_commit_publishing_a_knowledge_root_the_fold_does_not_reach_is_refused ... ok
test two_of_the_five_sub_roots_are_the_placeholder_and_not_derived ... ok
test a_revision_that_does_not_follow_its_parent_is_refused ... ok
test a_rejected_transaction_cannot_then_commit ... ok
test a_transaction_that_went_stale_cannot_then_commit ... ok
test a_commit_whose_validation_the_authority_does_not_stand_behind_does_not_advance_the_lineage ... ok
test a_store_with_no_commit_authority_refuses_to_say_what_canonical_state_is ... ok
test a_second_seed_is_refused ... ok
test a_commit_validated_against_a_revision_the_lineage_has_moved_past_does_not_advance_it ... ok
test two_stores_seeded_from_different_state_have_different_heads ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s

     Running tests/membrane_boundary.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/membrane_boundary-4fd8b8cc0e26267e)

running 1 test
test a_candidate_node_and_a_canonical_node_write_the_same_document ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/providers.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/providers-92620baa01a15f37)

running 15 tests
test a_file_store_stores_identical_bytes_once ... ok
test sqlite_stores_identical_bytes_once ... ok
test storing_a_graph_is_storing_its_document ... ok
test a_cache_write_after_a_canonical_one_leaves_the_bytes_canonical ... ok
test a_stored_object_carries_the_class_it_was_written_under ... ok
test a_raised_retention_class_survives_a_reopen ... ok
test a_replay_from_a_revision_with_no_materialised_state_is_refused ... ok
test the_recorded_class_is_the_strongest_requested_whichever_order_they_arrive_in ... ok
test the_fold_carries_the_seed_the_log_named ... ok
test a_sqlite_store_reopened_folds_to_the_same_head_root ... ok
test every_event_shape_reports_a_write_once_and_a_recognition_after ... ok
test two_different_events_appended_in_a_row_both_land ... ok
test sqlite_replay_from_the_seed_equals_the_fold ... ok
test a_file_store_reopened_folds_to_the_same_head_root ... ok
test a_file_store_replay_from_the_seed_equals_the_fold ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s

     Running tests/review_p1_invariant_one_at_the_store.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/review_p1_invariant_one_at_the_store-82651e84b28006c4)

running 2 tests
test a_commit_lands_with_no_validated_transaction_anywhere_in_the_process ... ok
test a_commit_validated_against_a_revision_that_is_no_longer_the_head_replays_as_valid ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/seed_object_integrity.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/seed_object_integrity-d7384f3e5473b860)

running 2 tests
test sqlite_seed_objects_verify_address_length_and_retention_before_admission ... ok
test file_seed_objects_verify_address_length_and_retention_before_admission ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s

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

Ownership/public-surface checks:
```sh
CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-08-seed CARGO_BUILD_JOBS=2 TMPDIR=<cache>/ekr-completion-20260922/seed cargo test -p ekr --test story_contract --test public_surface
# exit 0, executed 12
```
The opening coordinator gate's final-p1-07-test.log printed story_contract 10 and public_surface 1;
this run prints 11 and 1 respectively, total 11→12. That before measurement was run by coordinator
under cargo test --workspace, not a separate implementor run of the targeted command.
```
   Compiling syn v3.0.6
   Compiling anstyle v1.0.14
   Compiling utf8parse v0.2.2
   Compiling anstyle-parse v1.0.0
   Compiling colorchoice v1.0.5
   Compiling anstyle-query v1.1.5
   Compiling is_terminal_polyfill v1.70.2
   Compiling anstream v1.0.0
   Compiling strsim v0.11.1
   Compiling clap_lex v1.1.1
   Compiling predicates-core v1.0.10
   Compiling heck v0.5.0
   Compiling clap_builder v4.6.7
   Compiling serde_derive v1.0.229
   Compiling serde v1.0.229
   Compiling thiserror-impl v2.0.20
   Compiling serde_yaml_ng v0.10.0
   Compiling thiserror v2.0.20
   Compiling eventlog-core v0.2.1 (https://github.com/beyond10x/eventlog?tag=0.2.1#77cda080)
   Compiling ekr-core v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-core)
   Compiling eventlog-file v0.2.1 (https://github.com/beyond10x/eventlog?tag=0.2.1#77cda080)
   Compiling ekr-ontology v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-ontology)
   Compiling ekr-graph v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-graph)
   Compiling eventlog-sqlite v0.2.1 (https://github.com/beyond10x/eventlog?tag=0.2.1#77cda080)
   Compiling clap_derive v4.6.7
   Compiling ekr-store v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-store)
   Compiling termtree v0.5.1
   Compiling regex-automata v0.4.18
   Compiling assert_cmd v2.2.2
   Compiling difflib v0.4.0
   Compiling bstr v1.13.1
   Compiling predicates v3.1.4
   Compiling ekr-kernel v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-kernel)
   Compiling predicates-tree v1.0.13
   Compiling clap v4.6.7
   Compiling ekr v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 9.08s
     Running tests/public_surface.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/public_surface-cf7751befcffc952)

running 1 test
test no_public_item_in_any_crate_is_untested ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/story_contract.rs (<cache>/b10x-target/ekr-p1-08-seed/debug/deps/story_contract-ff859193506a8029)

running 11 tests
test every_crate_opts_into_workspace_lints ... ok
test crate_dependency_edges_match_the_story ... ok
test no_manifest_carries_a_comment ... ok
test external_dependencies_match_the_story ... ok
test the_expectation_tables_cover_every_crate ... ok
test workspace_dependencies_carry_the_story_qualifiers ... ok
test the_commit_authority_matcher_reads_a_head_by_shape_and_not_by_spelling ... ok
test the_six_crates_are_workspace_members ... ok
test seed_admission_is_kernel_owned_and_the_commit_api_lends_no_writer ... ok
test no_rust_source_reads_the_planning_store ... ok
test only_the_kernel_implements_the_commit_authority ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

Formatter:
```sh
cargo fmt --all --check
# exit 0; no output
```

Package linter:
```sh
CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-08-seed CARGO_BUILD_JOBS=2 TMPDIR=<cache>/ekr-completion-20260922/seed cargo clippy -p ekr-graph -p ekr-store -p ekr-kernel --all-targets -- -D warnings
# exit 0
```
```
    Checking ekr-kernel v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-kernel)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s
```

Documentation:
```sh
RUSTDOCFLAGS='-D rustdoc::broken_intra_doc_links' CARGO_TARGET_DIR=<cache>/b10x-target/ekr-p1-08-seed CARGO_BUILD_JOBS=2 TMPDIR=<cache>/ekr-completion-20260922/seed cargo doc -p ekr-graph -p ekr-store -p ekr-kernel --no-deps
# exit 0
```
```
    Checking ekr-graph v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-graph)
 Documenting ekr-graph v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-graph)
    Checking ekr-store v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-store)
 Documenting ekr-kernel v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-kernel)
 Documenting ekr-store v0.0.0 (<worktrees>/epistemic-knowledge-runtime/ekr-p1-08-seed/crates/ekr-store)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.92s
   Generated <cache>/b10x-target/ekr-p1-08-seed/doc/ekr_graph/index.html and 2 other files
```

git diff --check exited 0. Full task check, planning reconciliation, ESS status, independent
adversary and publication belong to coordinator; no such run is claimed here.

## 5. Deliberate exclusions and handoff concerns

- The writer is not implemented here. Existing ordinary commits still record validation addresses
  and do not apply operations or retain replayable transaction receipts. Existing tests keep that
  scope explicit.
- Ontology/agent sub-roots remain the existing placeholder values; full ontology bytes are now
  retained and checked on reopen. No populated agent-root/capability policy is claimed.
- Backend event envelope name/schema validation and graph/event version wrappers belong to the
  next persisted-contract story. This unit verifies stored ObjectRecord body metadata and seed
  envelope/input version dispatch, not every backend envelope field.
- Bootstrap admits HumanStatement support only. Observation, GraphAssertion and other source
  kinds explicitly refuse until their independent support can be resolved.
- No automatic legacy migration or invented attribution/evidence is supplied.
- Existing compile-time source guard macro debt was not broadened or repaired opportunistically;
  new fixture reads use runtime CARGO_MANIFEST_DIR.
- Writer contract concern from coordinator: Commit::seed returns store.head after atomic init.
  With a future concurrent ordinary writer, that could answer a later root; writer acceptance
  should hold seed response to the initialized revision. No such applying writer exists now.
- Scoped AGENTS wording is reviewable at agents.patch; coordinator asked to tighten the final
  named testcase citation after review. The ESS comments in this tree were coordinator-written.

## 6. Every outside write

Dedicated build/doc artifacts:
- <cache>/b10x-target/ekr-p1-08-seed/

Assigned scratch and TMPDIR (temporary provider databases/directories were scoped here):
- <cache>/ekr-completion-20260922/seed/
- <cache>/ekr-completion-20260922/seed/baseline.log
- <cache>/ekr-completion-20260922/seed/red.log
- <cache>/ekr-completion-20260922/seed/red-both.log
- <cache>/ekr-completion-20260922/seed/red-genesis.log
- <cache>/ekr-completion-20260922/seed/iteration.log
- <cache>/ekr-completion-20260922/seed/green.log
- <cache>/ekr-completion-20260922/seed/ownership.log
- <cache>/ekr-completion-20260922/seed/clippy-iteration.log
- <cache>/ekr-completion-20260922/seed/clippy.log
- <cache>/ekr-completion-20260922/seed/clippy-final.log
- <cache>/ekr-completion-20260922/seed/fmt.log
- <cache>/ekr-completion-20260922/seed/doc.log
- <cache>/ekr-completion-20260922/seed/agents.patch
- <cache>/ekr-completion-20260922/seed/case-migration.md
- <cache>/ekr-completion-20260922/seed/report.md

Managed lease metadata was changed only through the worktree CLI's session hooks. No other
session's process, cache or temporary content was changed. No build is running at handback.
