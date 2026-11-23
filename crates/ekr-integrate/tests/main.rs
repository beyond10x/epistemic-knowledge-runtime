//! The one integration-test binary of `ekr-integrate`: every test file under `tests/` is a module of
//! it. A new test file joins with a `#[path]` line below, in name order; the `#[path]` keeps a
//! file's own `mod` declarations resolving next to it, as when it was a binary of its own.
//! AGENTS.md, "The gate", says why and how.

// A support file that several test files load by `#[path]` is a module of each of them, and a file
// may wrap its cases in a module of its own name, as each did when it was a binary of its own.
#![allow(clippy::duplicate_mod, clippy::module_inception)]

#[path = "adversary_resolve.rs"]
mod adversary_resolve;
#[path = "adversary_x6_d.rs"]
mod adversary_x6_d;
#[path = "domain_projection.rs"]
mod domain_projection;
#[path = "extraction.rs"]
mod extraction;
#[path = "manifest.rs"]
mod manifest;
#[path = "resolve.rs"]
mod resolve;
