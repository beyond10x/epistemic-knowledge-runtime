//! The graph state of the Epistemic Knowledge Runtime.
//!
//! Implements the `ekr.graph` domain, `systems/ekr/domains/graph.yaml`: graph roots, nodes,
//! edges and bitemporal assertions with their validation state, evidence and observations. The
//! crate holds no writer of its own; every change arrives as a transaction the kernel committed.
