//! The trusted kernel of the Epistemic Knowledge Runtime.
//!
//! Implements the command half of the `ekr.kernel` domain,
//! `systems/ekr/domains/kernel.yaml`: the transaction boundary from proposal through validation
//! to an immutable committed revision, snapshot reads and the explain chain. This is the only
//! crate that constructs a validated transaction; the domain's types live in `ekr-core`.
