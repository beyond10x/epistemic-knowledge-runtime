//! The one integration-test binary of `ekr-graph`: every test file under `tests/` is a module of
//! it. A new test file joins with a `#[path]` line below, in name order; the `#[path]` keeps a
//! file's own `mod` declarations resolving next to it, as when it was a binary of its own.
//! AGENTS.md, "The gate", says why and how.

// A support file that several test files load by `#[path]` is a module of each of them, and a file
// may wrap its cases in a module of its own name, as each did when it was a binary of its own.
#![allow(clippy::duplicate_mod, clippy::module_inception)]

#[path = "adversary2_guard_bounds_and_ranges.rs"]
mod adversary2_guard_bounds_and_ranges;
#[path = "adversary2_membrane_and_addresses.rs"]
mod adversary2_membrane_and_addresses;
#[path = "adversary_canonical_value_reach.rs"]
mod adversary_canonical_value_reach;
#[path = "adversary_p1_06_reference_markers.rs"]
mod adversary_p1_06_reference_markers;
#[path = "adversary_p1_14_exit_typed_assertion_refs.rs"]
mod adversary_p1_14_exit_typed_assertion_refs;
#[path = "adversary_p1_14_r2_graph_declaration_scan.rs"]
mod adversary_p1_14_r2_graph_declaration_scan;
#[path = "adversary_snapshot_and_assertion.rs"]
mod adversary_snapshot_and_assertion;
#[path = "adversary_w20261020a_retired_edges.rs"]
mod adversary_w20261020a_retired_edges;
#[path = "canonical_value_and_assertion.rs"]
mod canonical_value_and_assertion;
#[path = "current_vectors.rs"]
mod current_vectors;
#[path = "domain_projection.rs"]
mod domain_projection;
#[path = "evidence_and_observations.rs"]
mod evidence_and_observations;
#[path = "legacy_status.rs"]
mod legacy_status;
#[path = "membrane.rs"]
mod membrane;
#[path = "node_identity.rs"]
mod node_identity;
#[path = "review_p1_membrane_is_by_id.rs"]
mod review_p1_membrane_is_by_id;
#[path = "revision_events.rs"]
mod revision_events;
#[path = "snapshot_reads.rs"]
mod snapshot_reads;
