//! The one integration-test binary of `ekr`: every test file under `tests/` is a module of
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

#[path = "adversary2_p1_15_schema.rs"]
mod adversary2_p1_15_schema;
#[path = "adversary2_p5_01_cli.rs"]
mod adversary2_p5_01_cli;
#[path = "adversary2_p5_01_store_open.rs"]
mod adversary2_p5_01_store_open;
#[path = "adversary2_w20261019a_supersede_cli.rs"]
mod adversary2_w20261019a_supersede_cli;
#[path = "adversary_add_alias_l_cli.rs"]
mod adversary_add_alias_l_cli;
#[path = "adversary_agent_guidance.rs"]
mod adversary_agent_guidance;
#[path = "adversary_alias_race.rs"]
mod adversary_alias_race;
#[path = "adversary_alias_replay.rs"]
mod adversary_alias_replay;
#[path = "adversary_c7_b_cli.rs"]
mod adversary_c7_b_cli;
#[path = "adversary_c7_m.rs"]
mod adversary_c7_m;
#[path = "adversary_c7_s_session.rs"]
mod adversary_c7_s_session;
#[path = "adversary_code_names_cli.rs"]
mod adversary_code_names_cli;
#[path = "adversary_docs_contract.rs"]
mod adversary_docs_contract;
#[path = "adversary_mcp_protocol.rs"]
mod adversary_mcp_protocol;
#[path = "adversary_mcp_resolve.rs"]
mod adversary_mcp_resolve;
#[path = "adversary_migrate_v3_cli.rs"]
mod adversary_migrate_v3_cli;
#[path = "adversary_p1_06_authority_guard.rs"]
mod adversary_p1_06_authority_guard;
#[path = "adversary_p1_13_cli_exit_contract.rs"]
mod adversary_p1_13_cli_exit_contract;
#[path = "adversary_p1_13_cli_r2_exit_map.rs"]
mod adversary_p1_13_cli_r2_exit_map;
#[path = "adversary_p1_13_cli_r2_reads.rs"]
mod adversary_p1_13_cli_r2_reads;
#[path = "adversary_p1_14_conformance.rs"]
mod adversary_p1_14_conformance;
#[path = "adversary_p1_14_conformance_r2.rs"]
mod adversary_p1_14_conformance_r2;
#[path = "adversary_p1_15_docs.rs"]
mod adversary_p1_15_docs;
#[path = "adversary_p1_15_docs_r2.rs"]
mod adversary_p1_15_docs_r2;
#[path = "adversary_p1_15_ess_pin.rs"]
mod adversary_p1_15_ess_pin;
#[path = "adversary_p1_15_ess_pin_r2.rs"]
mod adversary_p1_15_ess_pin_r2;
#[path = "adversary_p1_15_schema.rs"]
mod adversary_p1_15_schema;
#[path = "adversary_p1_15_seed_hash.rs"]
mod adversary_p1_15_seed_hash;
#[path = "adversary_p1_15_seed_hash_2.rs"]
mod adversary_p1_15_seed_hash_2;
#[path = "adversary_p4_01_agent_cli.rs"]
mod adversary_p4_01_agent_cli;
#[path = "adversary_p4_01_agent_cli_r2.rs"]
mod adversary_p4_01_agent_cli_r2;
#[path = "adversary_p5_01_cli.rs"]
mod adversary_p5_01_cli;
#[path = "adversary_p5_01_store_open.rs"]
mod adversary_p5_01_store_open;
#[path = "adversary_p_page.rs"]
mod adversary_p_page;
#[path = "adversary_page_stream_1.rs"]
mod adversary_page_stream_1;
#[path = "adversary_read_only_store.rs"]
mod adversary_read_only_store;
#[path = "adversary_rejections.rs"]
mod adversary_rejections;
#[path = "adversary_roles_rule.rs"]
mod adversary_roles_rule;
#[path = "adversary_roles_rule_2.rs"]
mod adversary_roles_rule_2;
#[path = "adversary_s_resolve.rs"]
mod adversary_s_resolve;
#[path = "adversary_s_resolve_pass2.rs"]
mod adversary_s_resolve_pass2;
#[path = "adversary_sdk01_h_replaced_store.rs"]
mod adversary_sdk01_h_replaced_store;
#[path = "adversary_session_create_pass1.rs"]
mod adversary_session_create_pass1;
#[path = "adversary_session_pass1.rs"]
mod adversary_session_pass1;
#[path = "adversary_session_views.rs"]
mod adversary_session_views;
#[path = "adversary_stream_pass1.rs"]
mod adversary_stream_pass1;
#[path = "adversary_tests_under_load.rs"]
mod adversary_tests_under_load;
#[path = "adversary_v_view.rs"]
mod adversary_v_view;
#[path = "adversary_v_view_r2.rs"]
mod adversary_v_view_r2;
#[path = "adversary_viewer_compact.rs"]
mod adversary_viewer_compact;
#[path = "adversary_viewer_follow_ups.rs"]
mod adversary_viewer_follow_ups;
#[path = "adversary_w20261019a_supersede_extraction.rs"]
mod adversary_w20261019a_supersede_extraction;
#[path = "adversary_w20261020a_extraction_notes.rs"]
mod adversary_w20261020a_extraction_notes;
#[path = "adversary_w20261020b_merge_cli.rs"]
mod adversary_w20261020b_merge_cli;
#[path = "adversary_x6_d_schema.rs"]
mod adversary_x6_d_schema;
#[path = "adversary_x6_e.rs"]
mod adversary_x6_e;
#[path = "adversary_x6_q_cli.rs"]
mod adversary_x6_q_cli;
#[path = "adversary_x6_x_documents.rs"]
mod adversary_x6_x_documents;
#[path = "adversary_x7_g_session.rs"]
mod adversary_x7_g_session;
#[path = "adversary_x7_g_source_guard.rs"]
mod adversary_x7_g_source_guard;
#[path = "adversary_x7_v.rs"]
mod adversary_x7_v;
#[path = "adversary_x7_v_2.rs"]
mod adversary_x7_v_2;
#[path = "agent_cli.rs"]
mod agent_cli;
#[path = "code_names_cli.rs"]
mod code_names_cli;
#[path = "conformance.rs"]
mod conformance;
#[path = "conformance_cli.rs"]
mod conformance_cli;
#[path = "conformance_integrate.rs"]
mod conformance_integrate;
#[path = "created_node_aliases.rs"]
mod created_node_aliases;
#[path = "docs_cli.rs"]
mod docs_cli;
#[path = "explain_bounds_documents.rs"]
mod explain_bounds_documents;
#[path = "explain_by_reference.rs"]
mod explain_by_reference;
#[path = "explain_pinned_evidence.rs"]
mod explain_pinned_evidence;
#[path = "explain_scope.rs"]
mod explain_scope;
#[path = "extraction_cli.rs"]
mod extraction_cli;
#[path = "extraction_validation.rs"]
mod extraction_validation;
#[path = "extraction_write_integrity.rs"]
mod extraction_write_integrity;
#[path = "gate_layout.rs"]
mod gate_layout;
#[path = "graph_assertion_serde.rs"]
mod graph_assertion_serde;
#[path = "graph_events_serde.rs"]
mod graph_events_serde;
#[path = "host_formats.rs"]
mod host_formats;
#[path = "host_input_adversary.rs"]
mod host_input_adversary;
#[path = "hosted_http.rs"]
mod hosted_http;
#[path = "mcp.rs"]
mod mcp;
#[path = "mcp_list_nodes.rs"]
mod mcp_list_nodes;
#[path = "migrate_cli.rs"]
mod migrate_cli;
#[path = "msrv_contract.rs"]
mod msrv_contract;
#[path = "named_formats.rs"]
mod named_formats;
#[path = "ocel_cli.rs"]
mod ocel_cli;
#[path = "postgres_cli.rs"]
mod postgres_cli;
#[path = "process_map_cli.rs"]
mod process_map_cli;
#[path = "public_surface.rs"]
mod public_surface;
#[path = "quality_cli.rs"]
mod quality_cli;
#[path = "read_only_store.rs"]
mod read_only_store;
#[path = "reader_metrics.rs"]
mod reader_metrics;
#[path = "rejections_cli.rs"]
mod rejections_cli;
#[path = "resolve_cli.rs"]
mod resolve_cli;
#[path = "retraction_example.rs"]
mod retraction_example;
#[path = "sample_cli.rs"]
mod sample_cli;
#[path = "schema_cli.rs"]
mod schema_cli;
#[path = "search_live.rs"]
mod search_live;
#[path = "search_page.rs"]
mod search_page;
#[path = "seed_document_bounds.rs"]
mod seed_document_bounds;
#[path = "seed_evidence_files.rs"]
mod seed_evidence_files;
#[path = "seed_if_absent.rs"]
mod seed_if_absent;
#[path = "session.rs"]
mod session;
#[path = "stage_cli.rs"]
mod stage_cli;
#[path = "store_comparison.rs"]
mod store_comparison;
#[path = "store_fault_reopens.rs"]
mod store_fault_reopens;
#[path = "store_open.rs"]
mod store_open;
#[path = "store_outcome_kinds.rs"]
mod store_outcome_kinds;
#[path = "story_contract.rs"]
mod story_contract;
#[path = "temporal_reads.rs"]
mod temporal_reads;
#[path = "view_cli.rs"]
mod view_cli;
#[path = "view_page.rs"]
mod view_page;
#[path = "view_roles.rs"]
mod view_roles;
#[path = "view_stream.rs"]
mod view_stream;
