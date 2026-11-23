//! The one integration-test binary of `ekr-store`: every test file under `tests/` is a module of
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

#[path = "adversary2_event_vocabulary.rs"]
mod adversary2_event_vocabulary;
#[path = "adversary2_p1_15_blob_batch_order.rs"]
mod adversary2_p1_15_blob_batch_order;
#[path = "adversary2_retention_event_contract.rs"]
mod adversary2_retention_event_contract;
#[path = "adversary_c7_f.rs"]
mod adversary_c7_f;
#[path = "adversary_c7_s.rs"]
mod adversary_c7_s;
#[path = "adversary_history_cache.rs"]
mod adversary_history_cache;
#[path = "adversary_objects_and_append.rs"]
mod adversary_objects_and_append;
#[path = "adversary_p1_06_reference_from_bytes.rs"]
mod adversary_p1_06_reference_from_bytes;
#[path = "adversary_p1_12_recovery_r2_identity.rs"]
mod adversary_p1_12_recovery_r2_identity;
#[path = "adversary_p1_12_vectors_identity_preparation.rs"]
mod adversary_p1_12_vectors_identity_preparation;
#[path = "adversary_p1_14_published_events_paging.rs"]
mod adversary_p1_14_published_events_paging;
#[path = "adversary_p1_14_r2_store_carriers.rs"]
mod adversary_p1_14_r2_store_carriers;
#[path = "adversary_p1_14_store_bindings.rs"]
mod adversary_p1_14_store_bindings;
#[path = "adversary_p1_15_batched_reads.rs"]
mod adversary_p1_15_batched_reads;
#[path = "adversary_read_only_open.rs"]
mod adversary_read_only_open;
#[path = "adversary_retained_bytes_shared_p1.rs"]
mod adversary_retained_bytes_shared_p1;
#[path = "adversary_runtime_context.rs"]
mod adversary_runtime_context;
#[path = "adversary_w20261007c_c_docs.rs"]
mod adversary_w20261007c_c_docs;
#[path = "adversary_w20261014a_r.rs"]
mod adversary_w20261014a_r;
#[path = "adversary_write_path_pointer.rs"]
mod adversary_write_path_pointer;
#[path = "adversary_x6_c.rs"]
mod adversary_x6_c;
#[path = "attachment_decode.rs"]
mod attachment_decode;
#[path = "authority_verify.rs"]
mod authority_verify;
#[path = "borrowed_graph.rs"]
mod borrowed_graph;
#[path = "checkpoint_pointer.rs"]
mod checkpoint_pointer;
#[path = "current_legacy_object_refusal.rs"]
mod current_legacy_object_refusal;
#[path = "current_occurrence_identity.rs"]
mod current_occurrence_identity;
#[path = "current_root_sensitivity.rs"]
mod current_root_sensitivity;
#[path = "current_vectors.rs"]
mod current_vectors;
#[path = "document_refusals.rs"]
mod document_refusals;
#[path = "domain_projection.rs"]
mod domain_projection;
#[path = "durable_objects.rs"]
mod durable_objects;
#[path = "eventlog_object_memo.rs"]
mod eventlog_object_memo;
#[path = "fold_rules.rs"]
mod fold_rules;
#[path = "history_cache.rs"]
mod history_cache;
#[path = "legacy.rs"]
mod legacy;
#[path = "membrane_boundary.rs"]
mod membrane_boundary;
#[path = "payload_blobs.rs"]
mod payload_blobs;
#[path = "postgres_password_file.rs"]
mod postgres_password_file;
#[path = "postgres_password_file_review.rs"]
mod postgres_password_file_review;
#[path = "preparation_authorized_once.rs"]
mod preparation_authorized_once;
#[path = "providers.rs"]
mod providers;
#[path = "published_events.rs"]
mod published_events;
#[path = "read_model.rs"]
mod read_model;
#[path = "read_only_copies.rs"]
mod read_only_copies;
#[path = "read_only_open.rs"]
mod read_only_open;
#[path = "review_p1_invariant_one_at_the_store.rs"]
mod review_p1_invariant_one_at_the_store;
#[path = "runtime_context.rs"]
mod runtime_context;
#[path = "seed_object_integrity.rs"]
mod seed_object_integrity;
#[path = "store_inventory.rs"]
mod store_inventory;
