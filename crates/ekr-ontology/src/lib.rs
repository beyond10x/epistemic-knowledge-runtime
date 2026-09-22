//! The ontology of the Epistemic Knowledge Runtime.
//!
//! Implements the `ekr.ontology` domain, `systems/ekr/domains/ontology.yaml`: schema versions,
//! node and edge types with their properties, per-type lifecycles and named operations. The
//! ontology evolves only through schema transactions the kernel commits.
//!
//! Five modules, in dependency order:
//!
//! * [`value`] — [`ValueType`] and the [`Value`] that mirrors it (design § 11.3), [`ValuePath`]
//!   — where a value canonical state does not admit sits inside the one that carries it — and
//!   [`Cardinality`].
//! * [`types`] — [`NodeType`], [`EdgeType`], [`PropertyDefinition`] (design § 11.1–11.2, § 12).
//! * [`lifecycle`] — [`Lifecycle`], [`Transition`], [`OperationDefinition`] (amendment 87).
//! * [`schema`] — [`SchemaVersion`] and [`Ontology`], the registry of one version's types.
//! * [`check`] — the type checker, which is what validators 3 to 5 of design § 20 call.
//!
//! An ontology is only ever held through [`Ontology::load`], so a declaration that would leave the
//! checker unable to answer — a `NodeRef` allowed to point at nothing, a parent cycle, a lifecycle
//! naming a state it does not have — is refused before any value is checked against it.
//!
//! ```
//! use std::collections::BTreeMap;
//!
//! use ekr_core::{NodeId, PropertyId, SchemaVersionId, Timestamp, TypeId};
//! use ekr_ontology::{
//!     NodeType, Ontology, OntologyDocument, PropertyDefinition, SchemaVersion, Value, ValueType,
//! };
//!
//! let (organisation, person, employer) = (TypeId::mint(), TypeId::mint(), PropertyId::mint());
//! let mut person_type = NodeType::new(person, "Person");
//! person_type.properties.insert(
//!     employer,
//!     PropertyDefinition::new(
//!         employer,
//!         "employer",
//!         ValueType::NodeRef { allowed_types: [organisation].into_iter().collect() },
//!     ),
//! );
//!
//! let ontology = Ontology::load(OntologyDocument {
//!     version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
//!     node_types: vec![person_type, NodeType::new(organisation, "Organisation")],
//!     edge_types: Vec::new(),
//! })?;
//!
//! // `employer = NodeRef(organization/openai)` is checkable; `employer = "OpenAI"` is not.
//! let openai = NodeId::mint();
//! let nodes: BTreeMap<NodeId, TypeId> = [(openai, organisation)].into_iter().collect();
//!
//! let referenced: BTreeMap<_, _> = [(employer, vec![Value::NodeRef(openai)])].into_iter().collect();
//! assert!(ontology.check_node(person, &referenced, &nodes).is_ok());
//!
//! let named: BTreeMap<_, _> =
//!     [(employer, vec![Value::String("OpenAI".to_owned())])].into_iter().collect();
//! assert!(ontology.check_node(person, &named, &nodes).is_err());
//! # Ok::<(), ekr_ontology::OntologyError>(())
//! ```

mod canonical;
pub mod check;
pub mod lifecycle;
pub mod schema;
pub mod types;
pub mod value;

pub use check::{CheckError, CheckReason, NodeTypes};
pub use lifecycle::{Lifecycle, LifecycleError, OperationDefinition, Transition};
pub use schema::{DeclarationSite, Ontology, OntologyDocument, OntologyError, SchemaVersion};
pub use types::{EdgeType, NodeType, PropertyDefinition};
pub use value::{Cardinality, Value, ValueKind, ValuePath, ValueType};
