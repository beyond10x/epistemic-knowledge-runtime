//! The one integration-test binary of `ekr-kernel`: every test file under `tests/` is a module of
//! it. A new test file joins with a `#[path]` line below, in name order; the `#[path]` keeps a
//! file's own `mod` declarations resolving next to it, as when it was a binary of its own.
//! AGENTS.md, "The gate", says why and how.

// A support file that several test files load by `#[path]` is a module of each of them, and a file
// may wrap its cases in a module of its own name, as each did when it was a binary of its own.
#![allow(clippy::duplicate_mod, clippy::module_inception)]

/// The libtest name of `case`, a test of the module whose `module_path!()` is `module`: the module
/// path without this binary's crate name, then the case. A test that runs one case of this binary
/// in a child process names it this way, since `--exact` matches the whole name.
fn case_name(module: &str, case: &str) -> String {
    let (_, path) = module
        .split_once("::")
        .expect("a test module of this binary");
    format!("{path}::{case}")
}

#[path = "add_alias.rs"]
mod add_alias;
#[path = "add_evidence.rs"]
mod add_evidence;
#[path = "adversary2_p1_14_exit_readback_and_assertion_refs.rs"]
mod adversary2_p1_14_exit_readback_and_assertion_refs;
#[path = "adversary2_p5_01_kernel.rs"]
mod adversary2_p5_01_kernel;
#[path = "adversary2_w20261019a_value_positions.rs"]
mod adversary2_w20261019a_value_positions;
#[path = "adversary_add_alias_l.rs"]
mod adversary_add_alias_l;
#[path = "adversary_add_evidence_1.rs"]
mod adversary_add_evidence_1;
#[path = "adversary_c7_b.rs"]
mod adversary_c7_b;
#[path = "adversary_c7_m.rs"]
mod adversary_c7_m;
#[path = "adversary_checkpoint_by_size_p2.rs"]
mod adversary_checkpoint_by_size_p2;
#[path = "adversary_checkpoint_cadence_p1.rs"]
mod adversary_checkpoint_cadence_p1;
#[path = "adversary_envelope_once_p1.rs"]
mod adversary_envelope_once_p1;
#[path = "adversary_legacy_freeze.rs"]
mod adversary_legacy_freeze;
#[path = "adversary_membrane.rs"]
mod adversary_membrane;
#[path = "adversary_membrane_pass_two.rs"]
mod adversary_membrane_pass_two;
#[path = "adversary_p1_07.rs"]
mod adversary_p1_07;
#[path = "adversary_p1_08_seed.rs"]
mod adversary_p1_08_seed;
#[path = "adversary_p1_12_recovery_1.rs"]
mod adversary_p1_12_recovery_1;
#[path = "adversary_p1_12_recovery_r2_anchor_race.rs"]
mod adversary_p1_12_recovery_r2_anchor_race;
#[path = "adversary_p1_12_replay_consistent_lie.rs"]
mod adversary_p1_12_replay_consistent_lie;
#[path = "adversary_p1_12_replay_r2_named_order.rs"]
mod adversary_p1_12_replay_r2_named_order;
#[path = "adversary_p1_12_vectors_seed_envelope.rs"]
mod adversary_p1_12_vectors_seed_envelope;
#[path = "adversary_p1_12_vectors_seed_sensitivity.rs"]
mod adversary_p1_12_vectors_seed_sensitivity;
#[path = "adversary_p1_13_explain_1.rs"]
mod adversary_p1_13_explain_1;
#[path = "adversary_p1_13_explain_r2_bound.rs"]
mod adversary_p1_13_explain_r2_bound;
#[path = "adversary_p1_14_exit_wire_shape.rs"]
mod adversary_p1_14_exit_wire_shape;
#[path = "adversary_p5_01_kernel.rs"]
mod adversary_p5_01_kernel;
#[path = "adversary_perf01_r_shared_reads.rs"]
mod adversary_perf01_r_shared_reads;
#[path = "adversary_perf02_m_head_graph.rs"]
mod adversary_perf02_m_head_graph;
#[path = "adversary_perf02_memo_across_stores.rs"]
mod adversary_perf02_memo_across_stores;
#[path = "adversary_perf_01_validation_scans.rs"]
mod adversary_perf_01_validation_scans;
#[path = "adversary_retained_bytes_shared_p1.rs"]
mod adversary_retained_bytes_shared_p1;
#[path = "adversary_sdk03_p.rs"]
mod adversary_sdk03_p;
#[path = "adversary_seed_envelope_v3.rs"]
mod adversary_seed_envelope_v3;
#[path = "adversary_seed_envelope_v3_elected.rs"]
mod adversary_seed_envelope_v3_elected;
#[path = "adversary_transaction_document.rs"]
mod adversary_transaction_document;
#[path = "adversary_transaction_document_r2.rs"]
mod adversary_transaction_document_r2;
#[path = "adversary_validate_once.rs"]
mod adversary_validate_once;
#[path = "adversary_w20261007c_c_capture.rs"]
mod adversary_w20261007c_c_capture;
#[path = "adversary_w20261007c_p.rs"]
mod adversary_w20261007c_p;
#[path = "adversary_w20261014a_r.rs"]
mod adversary_w20261014a_r;
#[path = "adversary_w20261014a_v.rs"]
mod adversary_w20261014a_v;
#[path = "adversary_w20261016b_u2.rs"]
mod adversary_w20261016b_u2;
#[path = "adversary_w20261019a_supersede_node.rs"]
mod adversary_w20261019a_supersede_node;
#[path = "adversary_x6_x.rs"]
mod adversary_x6_x;
#[path = "attach_evidence.rs"]
mod attach_evidence;
#[path = "attachment_base_store.rs"]
mod attachment_base_store;
#[path = "base_era_v1_replay.rs"]
mod base_era_v1_replay;
#[path = "candidate_built_once.rs"]
mod candidate_built_once;
#[path = "commit_applies_once.rs"]
mod commit_applies_once;
#[path = "commit_hashes_once.rs"]
mod commit_hashes_once;
#[path = "commit_path.rs"]
mod commit_path;
#[path = "current_records.rs"]
mod current_records;
#[path = "current_root_sensitivity.rs"]
mod current_root_sensitivity;
#[path = "current_validation_hash.rs"]
mod current_validation_hash;
#[path = "current_vectors.rs"]
mod current_vectors;
#[path = "durable_commands.rs"]
mod durable_commands;
#[path = "durable_seed.rs"]
mod durable_seed;
#[path = "edge_type_widening.rs"]
mod edge_type_widening;
#[path = "encoding_field_order.rs"]
mod encoding_field_order;
#[path = "explain.rs"]
mod explain;
#[path = "explain_index.rs"]
mod explain_index;
#[path = "hosted_postgres.rs"]
mod hosted_postgres;
#[path = "identity_history.rs"]
mod identity_history;
#[path = "identity_history_replay.rs"]
mod identity_history_replay;
#[path = "legacy.rs"]
mod legacy;
#[path = "merge_entity.rs"]
mod merge_entity;
#[path = "migrate_store.rs"]
mod migrate_store;
#[path = "p1_exit_properties.rs"]
mod p1_exit_properties;
#[path = "read_model.rs"]
mod read_model;
#[path = "recovery.rs"]
mod recovery;
#[path = "replay_checkpoint.rs"]
mod replay_checkpoint;
#[path = "replay_forgery.rs"]
mod replay_forgery;
#[path = "retained_bytes_shared.rs"]
mod retained_bytes_shared;
#[path = "runtime_published_events.rs"]
mod runtime_published_events;
#[path = "schema_evidence.rs"]
mod schema_evidence;
#[path = "schema_evolution.rs"]
mod schema_evolution;
#[path = "schema_evolution_replay.rs"]
mod schema_evolution_replay;
#[path = "seed.rs"]
mod seed;
#[path = "seed_envelope_once.rs"]
mod seed_envelope_once;
#[path = "seed_envelope_v3.rs"]
mod seed_envelope_v3;
#[path = "stage.rs"]
mod stage;
#[path = "stage_begin_copy_failure.rs"]
mod stage_begin_copy_failure;
#[path = "stage_hosted_limits.rs"]
mod stage_hosted_limits;
#[path = "store_open.rs"]
mod store_open;
#[path = "supersede_node.rs"]
mod supersede_node;
#[path = "transaction_document.rs"]
mod transaction_document;
#[path = "transaction_document_v2.rs"]
mod transaction_document_v2;
#[path = "validate_once_per_command.rs"]
mod validate_once_per_command;
#[path = "validate_properties.rs"]
mod validate_properties;
#[path = "validation.rs"]
mod validation;
#[path = "verified_read.rs"]
mod verified_read;
