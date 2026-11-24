//! Shared input types for the Epistemic Knowledge Runtime command-line host.
//!
//! The kernel, not host-input parsing, checks the retained authority anchor. `conformance`, built
//! only with the opt-in `conformance` feature, is the ESS conformance target over the same kernel
//! handlers: a user build of the binary compiles no part of ESS. This crate's own tests turn the
//! feature on through a dev-dependency on the crate itself.

pub mod cli;
#[cfg(feature = "conformance")]
pub mod conformance;
pub mod exit;
pub mod host;
