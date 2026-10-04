// generated from ekr v1
// model digest 95e0cc738a4105082ac236fd226f432bb61b31bd4c0c3f8bafd2688d0a257609
// contract digest db51800daf271146ca5948dce9aaf371fac263e98eae1f4dd09a51739c94022c
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
