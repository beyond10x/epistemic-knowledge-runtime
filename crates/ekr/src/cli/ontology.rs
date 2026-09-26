//! `ekr ontology`: the node types, edge types and properties at the head or at one committed
//! revision, by name and id, with the schema version in force there, through one `Runtime::read`.

use std::collections::BTreeMap;

use ekr_core::{RevisionNumber, SchemaVersionId, TypeId};
use ekr_kernel::Runtime;
use ekr_ontology::{Cardinality, PropertyDefinition};
use serde::Serialize;

use crate::exit::Failure;

#[derive(Serialize)]
pub(super) struct Ontology {
    revision: u64,
    schema_version: SchemaVersionId,
    /// The version's place in the lineage: 0 at the seed, one more per committed schema change.
    schema_version_number: u64,
    /// The version it was derived from; `null` at the seed.
    schema_version_parent: Option<SchemaVersionId>,
    node_types: Vec<NodeType>,
    edge_types: Vec<EdgeType>,
}

#[derive(Serialize)]
struct Named {
    id: TypeId,
    name: Option<String>,
}

#[derive(Serialize)]
struct NodeType {
    id: TypeId,
    name: String,
    parents: Vec<Named>,
    abstract_type: bool,
    properties: Vec<PropertyDefinition>,
}

#[derive(Serialize)]
struct EdgeType {
    id: TypeId,
    name: String,
    source_types: Vec<Named>,
    target_types: Vec<Named>,
    cardinality: Cardinality,
    properties: Vec<PropertyDefinition>,
}

/// The ontology of the requested (or newest) committed revision; a missing revision is the
/// kernel's `RevisionNotFound`, as for `ekr snapshot --at`.
pub(super) fn run(runtime: &Runtime, at: Option<u64>) -> Result<Ontology, Failure> {
    let read = runtime.read(at.map(RevisionNumber::new))?;
    let document = read.graph.ontology.to_document();
    let names: BTreeMap<TypeId, String> = document
        .node_types
        .iter()
        .map(|t| (t.id, t.name.clone()))
        .collect();
    let named = |ids: &std::collections::BTreeSet<TypeId>| -> Vec<Named> {
        ids.iter()
            .map(|id| Named {
                id: *id,
                name: names.get(id).cloned(),
            })
            .collect()
    };
    Ok(Ontology {
        revision: read.root.revision.get(),
        schema_version: document.version.id,
        schema_version_number: document.version.number,
        schema_version_parent: document.version.parent,
        node_types: document
            .node_types
            .iter()
            .map(|t| NodeType {
                id: t.id,
                name: t.name.clone(),
                parents: named(&t.parents),
                abstract_type: t.abstract_type,
                properties: t.properties.values().cloned().collect(),
            })
            .collect(),
        edge_types: document
            .edge_types
            .iter()
            .map(|t| EdgeType {
                id: t.id,
                name: t.name.clone(),
                source_types: named(&t.source_types),
                target_types: named(&t.target_types),
                cardinality: t.cardinality,
                properties: t.properties.values().cloned().collect(),
            })
            .collect(),
    })
}
