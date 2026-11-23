//! The one integration-test binary of `ekr-sdk`: every test file under `tests/` is a module of
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

#[path = "adversary_check_reads.rs"]
mod adversary_check_reads;
#[path = "adversary_docs_examples.rs"]
mod adversary_docs_examples;
#[path = "adversary_document_roundtrip.rs"]
mod adversary_document_roundtrip;
#[path = "adversary_evidence.rs"]
mod adversary_evidence;
#[path = "adversary_read.rs"]
mod adversary_read;
#[path = "adversary_read_enums.rs"]
mod adversary_read_enums;
#[path = "adversary_resolve_batch.rs"]
mod adversary_resolve_batch;
#[path = "adversary_session_close.rs"]
mod adversary_session_close;
#[path = "adversary_session_transport.rs"]
mod adversary_session_transport;
#[path = "adversary_spawn_busy.rs"]
mod adversary_spawn_busy;
#[path = "adversary_w20261014a_v.rs"]
mod adversary_w20261014a_v;
#[path = "adversary_x6_p.rs"]
mod adversary_x6_p;
#[path = "adversary_x6_r.rs"]
mod adversary_x6_r;
#[path = "batch_bisect.rs"]
mod batch_bisect;
#[path = "check_reads.rs"]
mod check_reads;
#[path = "checks.rs"]
mod checks;
#[path = "commit_scaling.rs"]
mod commit_scaling;
#[path = "docs_examples.rs"]
mod docs_examples;
#[path = "document_drift.rs"]
mod document_drift;
#[path = "evidence.rs"]
mod evidence;
#[path = "extraction_document.rs"]
mod extraction_document;
#[path = "protocol_answer.rs"]
mod protocol_answer;
#[path = "read.rs"]
mod read;
#[path = "redact.rs"]
mod redact;
#[path = "resolve_cache.rs"]
mod resolve_cache;
#[path = "session.rs"]
mod session;
#[path = "session_close.rs"]
mod session_close;
#[path = "spawn_busy.rs"]
mod spawn_busy;
