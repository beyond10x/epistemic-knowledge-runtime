---
format: aep.planning-md/2
id: verification-report:p1-exit
kind: verification-report
status: draft
title: P1 exit clauses mapped to the cases that execute them
relations:
- verifies: story:p1-exit-properties
revision: 1
---
Filed at the close of wave p1-14 by the coordinator. The table below was produced by the exit unit on `wave/p1-14-exit` at `d104a2b`; every case it cites ran again in the closing gate on `wave/wave-p1-14` at `2644da8` (`task test`: 772 passed, 0 failed, 132 targets). The one absolute-path prefix in the unit's text is written `$HOME`.

# Verification report — P1 exit (`story:p1-exit-properties`, wave p1-14, unit p1-14-exit)

**Tree:** `wave/p1-14-exit` at `8a22a2e`, plus two uncommitted things:
- round 2 of this unit, which changes only `crates/ekr-kernel/tests/p1_exit_properties.rs`;
- the untracked adversary file `crates/ekr-kernel/tests/adversary2_p1_14_exit_readback_and_assertion_refs.rs`.

The coordinator's `systems/ekr/` changes are also present in the tree, uncommitted.

**Run:** `cargo test --locked --no-fail-fast -p ekr-graph -p ekr-kernel -p ekr-store -p ekr`, with
`CARGO_TARGET_DIR=home-path:sha256:90adafd9d0964e2d5825d4e771410292ac0c9687653de7737fb4840470b1752f`. The log is
`home-path:sha256:beb048c9ff71e28b426766b0ea8f8fc1188591d91216924ce47833a19021d0ae`: exit 0, 569 passed, 0 failed, 0 ignored,
95 `test result:` lines. "Ran" means the case appears as `... ok` in that log. A case marked `(trybuild)`
appears there as its own `test tests/<dir>/<file>.rs ... ok` line under its harness.

## `docs/roadmap.md` § 4 P1, Exit (lines 144-146)

### 1. No dangling reference can commit, shown by property tests

| Case | What it asserts | Who refuses | Ran |
|---|---|---|---|
| `crates/ekr-kernel/tests/p1_exit_properties.rs::no_transaction_naming_an_absent_identity_commits_on_either_provider` | Covers 105 strata: 21 shapes (every place an operation names a node, edge, assertion, evidence or graph root) × 5 absent-id forms (a fresh id, and the bits of an entity of each other kind the world holds). Each stratum runs 2 generated cases on File and SQLite through `Runtime`, from fixed seeds `0x10014e417 + index`. For each case: validation is `Rejected` with the kind's `unresolved-*` code; commit is refused with `TransactionStateConflict { Rejected }`; the commit appends zero physical events; head, revision count and snapshot are unchanged. The case also checks that all 105 strata ran 2 cases each. | reference validator (`crates/ekr-kernel/src/validate/reference.rs`). `MergeEntity` shapes are also refused by the structural validator, because P1 has no merge application path | yes |
| `crates/ekr-kernel/tests/adversary2_p1_14_exit_readback_and_assertion_refs.rs::the_physical_event_counter_the_exit_property_relies_on_can_see_a_publication` | The zero-events counter can see a real publication, so the zero-events assertion can fail | — | yes |
| `crates/ekr-kernel/tests/adversary2_p1_14_exit_readback_and_assertion_refs.rs::a_refused_commit_publishes_nothing_a_fresh_runtime_reads_on_either_provider` | A fresh runtime opened after a refused commit reads the seed's head and revision count, on both providers | reference validator | yes |
| `crates/ekr-kernel/tests/validation.rs::every_kind_of_dangling_reference_is_refused` | Hand-written rows, through the pipeline only | reference validator | yes |
| `crates/ekr-kernel/tests/seed.rs::a_seed_with_a_dangling_edge_is_refused_by_both_backends` | The seed path | seed admission: `crates/ekr-kernel/src/seed.rs` runs the seed through `Pipeline::validate_bootstrap`, whose reference validator refuses it | yes |

### 2. `Canonical → Transient` is unrepresentable at the type level

The graph harness `crates/ekr-graph/tests/membrane.rs::the_membrane_is_a_set_of_build_failures` globs `tests/compile_fail/`. `crates/ekr-graph/src/lib.rs:35-49` maps each reference to its case. Every row is enforced by the compiler.

| Reference | Case (trybuild) | Ran |
|---|---|---|
| `CanonicalRef<T>` holds its own kind's id | `crates/ekr-graph/tests/compile_fail/a_canonical_reference_holds_the_id_of_its_kind.rs` | yes |
| Only kinds canonical state keeps a map of are targets | `crates/ekr-graph/tests/compile_fail/a_canonical_reference_targets_only_what_canonical_state_holds.rs` | yes |
| `Subject::Node`, `Object::Node`, `CanonicalValue::NodeRef` | `crates/ekr-graph/tests/compile_fail/a_canonical_claim_names_its_nodes_by_canonical_reference.rs` | yes |
| `Edge::source`, `Edge::target` | `crates/ekr-graph/tests/review_p1_compile_fail/a_canonical_edge_may_target_a_candidate_node.rs` (harness `crates/ekr-graph/tests/review_p1_membrane_is_by_id.rs`) | yes |
| `Subject::Edge` | `crates/ekr-graph/tests/compile_fail/a_canonical_subject_names_its_edge_by_canonical_reference.rs` | yes |
| `Assertion::evidence` | `crates/ekr-graph/tests/compile_fail/a_canonical_assertion_cites_evidence_by_canonical_reference.rs` | yes |
| `AssertionLifecycle::Superseded.by` | `crates/ekr-graph/tests/adversary_p1_14_exit_compile_fail/a_canonical_supersession_names_its_replacement_by_canonical_reference.rs` (harness `crates/ekr-graph/tests/adversary_p1_14_exit_typed_assertion_refs.rs::every_assertion_reference_canonical_state_holds_refuses_a_transient_identity`) | yes |
| `Assessment::Disputed.competing_assertions` | `crates/ekr-graph/tests/adversary_p1_14_exit_compile_fail/a_canonical_dispute_names_its_competitors_by_canonical_reference.rs` (same harness) | yes |
| `EvidenceSource::GraphAssertion` | `crates/ekr-graph/tests/adversary_p1_14_exit_compile_fail/retained_evidence_names_its_source_assertion_by_canonical_reference.rs` (same harness) | yes |
| Graph resolve refuses a transient reference; the dependency trait is sealed; no `CanonicalRef<TransientRef<_>>`; transient state has no content address | `crates/ekr-graph/tests/compile_fail/canonical_graph_rejects_a_transient_ref.rs`, `crates/ekr-graph/tests/compile_fail/canonical_dependency_is_sealed.rs`, `crates/ekr-graph/tests/compile_fail/canonical_ref_cannot_target_a_transient_type.rs`, `crates/ekr-graph/tests/compile_fail/transient_state_has_no_content_address.rs` | yes |

One runtime case also belongs to this clause:

| Case | What it asserts | Ran |
|---|---|---|
| `crates/ekr-graph/tests/adversary_p1_06_reference_markers.rs::a_reference_to_evidence_resolves_to_evidence_and_never_to_a_node` | The marker, not the bits, decides which map a reference resolves in | yes |

### 3. Replay from the seed reproduces the root hash

| Case | What it asserts | Ran |
|---|---|---|
| `crates/ekr-kernel/tests/p1_exit_properties.rs::replay_from_the_seed_reproduces_every_root_of_a_generated_lineage_on_both_providers` | 16 generated lineages of 1-4 commits, fixed seed `0x10014e417`, on File and SQLite. Every commit's root equals `read(Some(r)).root`, both in the committing process and in a freshly opened runtime. The fresh head equals the last root. Both providers record identical roots. | yes |
| `crates/ekr-kernel/tests/seed.rs::an_evidence_seed_reopens_with_identical_roots_fields_and_retained_bytes` | The same, for a fixed lineage | yes |

### 4. The § 65 retraction example runs

| Case | Ran |
|---|---|
| `crates/ekr/tests/retraction_example.rs::the_retraction_example_runs_through_fresh_processes_on_both_providers` (runs `CARGO_BIN_EXE_ekr`) | yes |

## `epic:p1-kernel-ontology-core` Acceptance

| Clause | Case | Ran |
|---|---|---|
| Property tests show that no dangling reference can commit | `crates/ekr-kernel/tests/p1_exit_properties.rs::no_transaction_naming_an_absent_identity_commits_on_either_provider` (transaction path). The seed path's case is `crates/ekr-kernel/tests/seed.rs::a_seed_with_a_dangling_edge_is_refused_by_both_backends`, which is not generated. | yes |
| A `Canonical → Transient` reference is unrepresentable at the type level | `crates/ekr-graph/tests/membrane.rs::the_membrane_is_a_set_of_build_failures`, `crates/ekr-graph/tests/review_p1_membrane_is_by_id.rs`, `crates/ekr-graph/tests/adversary_p1_14_exit_typed_assertion_refs.rs::every_assertion_reference_canonical_state_holds_refuses_a_transient_identity` (the cases in the table above) | yes |
| Replay from the seed reproduces the root hash | `crates/ekr-kernel/tests/p1_exit_properties.rs::replay_from_the_seed_reproduces_every_root_of_a_generated_lineage_on_both_providers` | yes |
| The § 65 retraction example runs through the CLI | `crates/ekr/tests/retraction_example.rs::the_retraction_example_runs_through_fresh_processes_on_both_providers` | yes |

## What this report does not claim

- **The type does not stop a transient id becoming a canonical one.** What the type holds is that a `TransientRef`, `LocalRef` or bare id does not inhabit a canonical field. It does not hold that a candidate's *id* cannot be turned into a `CanonicalRef`: the public `CanonicalRef::new` takes any id of the kind.
  - `crates/ekr-kernel/tests/adversary2_p1_14_exit_readback_and_assertion_refs.rs::a_transient_identity_is_minted_into_every_canonical_reference_kind` compiles and ran green.
  - This is the serde and mint boundary that `architecture-decision-record:0008-canonical-state-references-are-typed` states. On the transaction path the kernel mints references only after the reference validator has resolved the ids.
- **Dangling references in seed documents are not generated.** The only seed case is the fixed dangling-edge seed.
- **The transaction's own `evidence` list is not resolved by the reference validator.** The structural validator compares it with the assertions' cited set. The generated `CitedEvidence` shape names the same absent id in both places.
- **Mutation evidence** that the dangling-reference property can fail:
  - `home-path:sha256:23066653d684a6fe4f9df99d8654be5379ed4d1ba6a3d42c1a345022d4a58721` (round 0);
  - `r1/stratified-mutation.log` (value and root checks disabled: 7 shapes red);
  - `r2/mutation-borrowed.log` (edges resolved against node bits: exactly `DeletedEdge/BorrowedFrom(Node)` and `SubjectEdge/BorrowedFrom(Node)` red, with the same failing input on rerun in `r2/mutation-borrowed-rerun.log`).
