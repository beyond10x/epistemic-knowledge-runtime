//! The one integration-test binary of `ekr-views`: every test file under `tests/` is a module of
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

#[path = "adversary_add_evidence_views.rs"]
mod adversary_add_evidence_views;
#[path = "adversary_changes_evidence.rs"]
mod adversary_changes_evidence;
#[path = "adversary_code_names.rs"]
mod adversary_code_names;
#[path = "adversary_lineage.rs"]
mod adversary_lineage;
#[path = "adversary_ocel.rs"]
mod adversary_ocel;
#[path = "adversary_pass2.rs"]
mod adversary_pass2;
#[path = "adversary_perf02_m_past_revisions.rs"]
mod adversary_perf02_m_past_revisions;
#[path = "adversary_property_redeclaration.rs"]
mod adversary_property_redeclaration;
#[path = "adversary_quality.rs"]
mod adversary_quality;
#[path = "adversary_query_pass1.rs"]
mod adversary_query_pass1;
#[path = "adversary_views_floor_weakened.rs"]
mod adversary_views_floor_weakened;
#[path = "adversary_views_head_pass1.rs"]
mod adversary_views_head_pass1;
#[path = "adversary_w20261014a_v.rs"]
mod adversary_w20261014a_v;
#[path = "adversary_w20261019a_supersession_reads.rs"]
mod adversary_w20261019a_supersession_reads;
#[path = "adversary_w20261020b_merge_changes.rs"]
mod adversary_w20261020b_merge_changes;
#[path = "adversary_x6_j.rs"]
mod adversary_x6_j;
#[path = "adversary_x6_q.rs"]
mod adversary_x6_q;
#[path = "changes.rs"]
mod changes;
#[path = "changes_adversary.rs"]
mod changes_adversary;
#[path = "changes_attachments_aliases.rs"]
mod changes_attachments_aliases;
#[path = "changes_by_time_cost.rs"]
mod changes_by_time_cost;
#[path = "code_names.rs"]
mod code_names;
#[path = "conformance.rs"]
mod conformance;
#[path = "determinism.rs"]
mod determinism;
#[path = "document.rs"]
mod document;
#[path = "event_time_assertions.rs"]
mod event_time_assertions;
#[path = "lineage.rs"]
mod lineage;
#[path = "merge.rs"]
mod merge;
#[path = "ocel.rs"]
mod ocel;
#[path = "one_event_type_rule.rs"]
mod one_event_type_rule;
#[path = "process_map.rs"]
mod process_map;
#[path = "pure_render.rs"]
mod pure_render;
#[path = "quality.rs"]
mod quality;
#[path = "query.rs"]
mod query;
#[path = "reads_only.rs"]
mod reads_only;
#[path = "replays.rs"]
mod replays;
#[path = "sample.rs"]
mod sample;
#[path = "shared_id.rs"]
mod shared_id;
#[path = "supersession.rs"]
mod supersession;
#[path = "timeline.rs"]
mod timeline;
