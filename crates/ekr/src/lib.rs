//! Shared input types for the Epistemic Knowledge Runtime command-line host.
//!
//! The kernel, not host-input parsing, checks the retained authority anchor. [`conformance`] is
//! the ESS conformance target over the same kernel handlers.

pub mod cli;
pub mod conformance;
pub mod exit;
pub mod host;
