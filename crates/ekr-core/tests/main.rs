//! The one integration-test binary of `ekr-core`: every test file under `tests/` is a module of
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

#[path = "adversary2_encoding_vector.rs"]
mod adversary2_encoding_vector;
#[path = "adversary2_public_surface.rs"]
mod adversary2_public_surface;
#[path = "adversary2_revision_number_text_form.rs"]
mod adversary2_revision_number_text_form;
#[path = "adversary_encoder_order.rs"]
mod adversary_encoder_order;
#[path = "adversary_encoding_vector.rs"]
mod adversary_encoding_vector;
#[path = "adversary_id_text_form.rs"]
mod adversary_id_text_form;
#[path = "adversary_perf02_streaming_hash.rs"]
mod adversary_perf02_streaming_hash;
#[path = "bytes.rs"]
mod bytes;
#[path = "canonical_encoding.rs"]
mod canonical_encoding;
#[path = "content_hash.rs"]
mod content_hash;
#[path = "decode.rs"]
mod decode;
#[path = "hashing_encoder_contract.rs"]
mod hashing_encoder_contract;
#[path = "identity_serde.rs"]
mod identity_serde;
#[path = "public_surface.rs"]
mod public_surface;
#[path = "redact.rs"]
mod redact;
#[path = "rename_stability.rs"]
mod rename_stability;
#[path = "security_w20261014a_c.rs"]
mod security_w20261014a_c;
#[path = "sha256_backend.rs"]
mod sha256_backend;
#[path = "timestamp.rs"]
mod timestamp;
