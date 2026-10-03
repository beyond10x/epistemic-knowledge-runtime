// generated from ekr v1
// model digest 76cac94520a78179089e10b0871c7979eacd35197c8e7b584616fac60d49e705
// contract digest c0793b41517a2b54756a5ae9c6d0c85aa53885defe0f88328a27a2c46c880d14
// do not edit: regenerate with `ess synthesize`

//! Semantic types synthesised from the `ekr` specification, v1.
//!
//! The Epistemic Knowledge Runtime's own contract: what its verbs accept, what each outcome means, and which crate answers which command. Written before the crates, so the Rust implements this document and a conformance suite can hold it to it. Nothing here names a domain concept such as a person or a project; those are graph state typed by an ontology the runtime carries as data.
//!
//! Generated, not written: the specification is the source of truth, and the door to changing
//! anything here is `ess synthesize`. What is deliberately absent — behaviour, queries,
//! escalations — is listed with reasons in the `PLAN.md` beside this workspace, and every entry
//! there is owed through a typed seam in an `obligations` module here.

// `deny`, not the source workspace's lint set: this crate must hold on its own, and an undocumented
// public item here is an emitter defect worth failing the gate over.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod actor;
pub mod behaviour;
pub mod graph;
pub mod integrate;
pub mod kernel;
pub mod obligation;
pub mod observe;
pub mod ontology;
pub mod primitives;
pub mod store;
pub mod views;
