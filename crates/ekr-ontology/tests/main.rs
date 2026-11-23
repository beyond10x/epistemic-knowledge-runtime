//! The one integration-test binary of `ekr-ontology`: every test file under `tests/` is a module of
//! it. A new test file joins with a `#[path]` line below, in name order; the `#[path]` keeps a
//! file's own `mod` declarations resolving next to it, as when it was a binary of its own.
//! AGENTS.md, "The gate", says why and how.

// A support file that several test files load by `#[path]` is a module of each of them, and a file
// may wrap its cases in a module of its own name, as each did when it was a binary of its own.
#![allow(clippy::duplicate_mod, clippy::module_inception)]

#[path = "adversary2_p5_01_ontology.rs"]
mod adversary2_p5_01_ontology;
#[path = "adversary_decoder.rs"]
mod adversary_decoder;
#[path = "adversary_p5_01_ontology.rs"]
mod adversary_p5_01_ontology;
#[path = "canonical_admissibility.rs"]
mod canonical_admissibility;
#[path = "document_round_trip.rs"]
mod document_round_trip;
#[path = "domain_projection.rs"]
mod domain_projection;
#[path = "hierarchy_specificity.rs"]
mod hierarchy_specificity;
#[path = "inheritance_and_declaration_coherence.rs"]
mod inheritance_and_declaration_coherence;
#[path = "lifecycle_transitions.rs"]
mod lifecycle_transitions;
#[path = "ontology_adversary_edge_type_widen_p2e.rs"]
mod ontology_adversary_edge_type_widen_p2e;
#[path = "ontology_load.rs"]
mod ontology_load;
#[path = "schema_evolution.rs"]
mod schema_evolution;
#[path = "schema_version_evidence.rs"]
mod schema_version_evidence;
#[path = "type_hierarchy.rs"]
mod type_hierarchy;
#[path = "value_type_checking.rs"]
mod value_type_checking;
#[path = "yaml_ingress.rs"]
mod yaml_ingress;
