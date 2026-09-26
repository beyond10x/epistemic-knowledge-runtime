//! Schema changes under validation profile v2: design § 26, and wave p5-01's decisions 1–6.
//!
//! Profile v1 (`ekr.p1-deterministic/1`) refuses the three schema kinds with the retained P1
//! issue, `unsupported-operation`, byte for byte, because replay compares every retained
//! rejection's code and message to what the ruleset says now (`replay.rs`,
//! `rejection-issue-disagrees`): a v1 store holding such a rejection keeps reopening only while the
//! ruleset that wrote it still says the same thing.
//!
//! Profile v2 (`ekr.p2-deterministic/1`) admits them. The structural half — the transaction names
//! the version it produces, and a schema change travels alone — is the structural validator's, as
//! a question about the transaction's shape. The ontology half is here and reports as
//! [`ValidatorName::OntologyConstraint`]: the candidate ontology is built with
//! [`Ontology::evolve`], its version id is held new against every version on the lineage, and
//! [`incompatibilities`] is asked whether it can replace the prior one over the canonical state
//! there is. Each refusal carries the ontology's own code.
//!
//! **Silent where another validator has spoken.** This stage runs only on a transaction the
//! structural validator has nothing to say about in this respect: schema-only, carrying a version,
//! and declaring no type id twice or over one the ontology holds. Otherwise the structural refusal
//! is the one a proposer reads, and `Ontology::evolve` repeating it as `type-already-declared`
//! would be the same defect reported by two validators.

use std::collections::BTreeSet;

use ekr_core::{PropertyId, SchemaVersionId, TypeId};
use ekr_graph::{
    AssertionLifecycle, CanonicalGraph, CanonicalValue, GraphSnapshot, Object, Predicate, Subject,
    ValueSpace,
};
use ekr_ontology::{
    incompatibilities, EvolveError, InstanceState, Ontology, SchemaChange, Value, ValueKind,
};

use super::{finish, issue, OntologyConstraint, Validator};
use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::{GraphOperation, GraphTransaction};

/// Whether an operation is one of the three schema kinds.
pub(crate) const fn is_schema_change<V: ValueSpace>(operation: &GraphOperation<V>) -> bool {
    matches!(
        operation,
        GraphOperation::DefineNodeType(_)
            | GraphOperation::DefineEdgeType(_)
            | GraphOperation::ModifyProperty(_)
    )
}

/// The operations of `tx` as the ontology's changes, in the order written, or `None` if any
/// operation is of another kind, or a `ModifyProperty` in the ownerless P1 shape.
pub(crate) fn changes<V: ValueSpace>(tx: &GraphTransaction<V>) -> Option<Vec<SchemaChange>> {
    tx.operations
        .iter()
        .map(|operation| match operation {
            GraphOperation::DefineNodeType(declared) => {
                Some(SchemaChange::DefineNodeType((**declared).clone()))
            }
            GraphOperation::DefineEdgeType(declared) => {
                Some(SchemaChange::DefineEdgeType((**declared).clone()))
            }
            // The ownerless P1 shape names no owner to evolve; the structural validator refuses it
            // under v2, and this stage stays silent.
            GraphOperation::ModifyProperty(modification) => {
                modification
                    .owner
                    .map(|owner| SchemaChange::ModifyProperty {
                        owner,
                        property: modification.property.clone(),
                    })
            }
            _ => None,
        })
        .collect()
}

/// Every schema version id on the lineage of the given ontologies: each one's own id and its
/// parent's. The kernel holds every committed revision's ontology, so the ontologies of revisions
/// seed through `n` name every version from the seed to the one current at `n`.
pub(crate) fn lineage<'a>(
    ontologies: impl IntoIterator<Item = &'a Ontology>,
) -> BTreeSet<SchemaVersionId> {
    ontologies
        .into_iter()
        .flat_map(|ontology| {
            let version = ontology.version();
            [Some(version.id), version.parent]
        })
        .flatten()
        .collect()
}

/// Validator 5 under profile v2: the ontology constraints [`OntologyConstraint`] checks, and then
/// the schema change a transaction proposes.
pub(crate) struct SchemaOntology {
    /// Every schema version id already on the lineage of the snapshot validated against.
    pub(crate) lineage: BTreeSet<SchemaVersionId>,
}

impl Validator for SchemaOntology {
    fn name(&self) -> ValidatorName {
        ValidatorName::OntologyConstraint
    }

    fn validate(
        &self,
        graph: &GraphSnapshot<'_>,
        tx: &GraphTransaction,
    ) -> Result<(), Vec<ValidationIssue>> {
        let mut issues = OntologyConstraint
            .validate(graph, tx)
            .err()
            .unwrap_or_default();
        issues.extend(admission(graph.graph(), tx, &self.lineage));
        finish(issues)
    }
}

/// The refusals of one schema change against the canonical state it would replace the ontology of.
fn admission(
    graph: &CanonicalGraph,
    tx: &GraphTransaction,
    lineage: &BTreeSet<SchemaVersionId>,
) -> Vec<ValidationIssue> {
    let (Some(version), Some(changes)) = (tx.schema_version, changes(tx)) else {
        return Vec::new();
    };
    let prior = &graph.ontology;
    let mut defined = BTreeSet::new();
    for change in &changes {
        let type_id = match change {
            SchemaChange::DefineNodeType(declared) => declared.id,
            SchemaChange::DefineEdgeType(declared) => declared.id,
            SchemaChange::ModifyProperty { .. } => continue,
        };
        let held = prior.node_type(type_id).is_some() || prior.edge_type(type_id).is_some();
        if held || !defined.insert(type_id) {
            return Vec::new();
        }
    }

    let refusal =
        |code: &str, message: String| issue(tx, ValidatorName::OntologyConstraint, code, message);
    if lineage.contains(&version) {
        let reused = EvolveError::VersionIdReused { id: version };
        return vec![refusal(
            reused.code(),
            format!(
                "{}: {version} is already a schema version of this lineage; mint a new one with \
                 `ekr mint schema-version`",
                reused.code()
            ),
        )];
    }
    // The version record's time is the commit's, set when the change is applied; nothing below
    // reads it, so the prior version's time stands in for it here and no clock is consulted.
    let next = match prior.evolve(version, prior.version().created_at, &changes) {
        Ok(next) => next,
        Err(error) => return vec![refusal(error.code(), error.to_string())],
    };
    incompatibilities(prior, &next, &Held(graph))
        .into_iter()
        .map(|found| refusal(found.code(), found.to_string()))
        .collect()
}

/// Canonical state as [`InstanceState`] reads it, following the trait's documentation: an instance
/// of `owner` is a node or an edge whose own type is `owner`; counts are of held top-level values;
/// kinds cover held values and the objects of active property assertions.
struct Held<'a>(&'a CanonicalGraph);

impl Held<'_> {
    /// The property bags of every instance whose own type is `owner`.
    fn instances(
        &self,
        owner: TypeId,
    ) -> impl Iterator<Item = &std::collections::BTreeMap<PropertyId, Vec<CanonicalValue>>> {
        let nodes = self
            .0
            .nodes
            .values()
            .filter(move |node| node.type_id == owner)
            .map(|node| &node.properties);
        let edges = self
            .0
            .edges
            .values()
            .filter(move |edge| edge.type_id == owner)
            .map(|edge| &edge.properties);
        nodes.chain(edges)
    }

    /// The number of values `property` holds on one instance.
    fn held(
        properties: &std::collections::BTreeMap<PropertyId, Vec<CanonicalValue>>,
        property: PropertyId,
    ) -> u64 {
        properties
            .get(&property)
            .map_or(0, |values| values.len() as u64)
    }

    /// Whether an assertion's subject is an instance whose own type is `owner`.
    fn subject_is_of(&self, subject: &Subject, owner: TypeId) -> bool {
        match subject {
            Subject::Node(node) => self
                .0
                .nodes
                .get(&node.id())
                .is_some_and(|node| node.type_id == owner),
            Subject::Edge(edge) => self
                .0
                .edges
                .get(&edge.id())
                .is_some_and(|edge| edge.type_id == owner),
            Subject::Type(_) => false,
        }
    }
}

fn kind_of(value: &CanonicalValue) -> ValueKind {
    Value::from(value.clone()).kind()
}

impl InstanceState for Held<'_> {
    fn node_count(&self, node_type: TypeId) -> u64 {
        self.0
            .nodes
            .values()
            .filter(|node| node.type_id == node_type)
            .count() as u64
    }

    fn edge_count(&self, edge_type: TypeId) -> u64 {
        self.0
            .edges
            .values()
            .filter(|edge| edge.type_id == edge_type)
            .count() as u64
    }

    fn max_values(&self, owner: TypeId, property: PropertyId) -> u64 {
        self.instances(owner)
            .map(|properties| Self::held(properties, property))
            .max()
            .unwrap_or(0)
    }

    fn min_values(&self, owner: TypeId, property: PropertyId) -> u64 {
        self.instances(owner)
            .map(|properties| Self::held(properties, property))
            .min()
            .unwrap_or(0)
    }

    fn value_kinds(&self, owner: TypeId, property: PropertyId) -> BTreeSet<ValueKind> {
        let held = self
            .instances(owner)
            .filter_map(|properties| properties.get(&property))
            .flatten()
            .map(kind_of);
        let asserted = self
            .0
            .assertions
            .values()
            .filter(|assertion| {
                matches!(assertion.lifecycle, AssertionLifecycle::Active)
                    && assertion.predicate == Predicate::Property(property)
                    && self.subject_is_of(&assertion.subject, owner)
            })
            .filter_map(|assertion| match &assertion.object {
                Object::Value(value) => Some(kind_of(value)),
                Object::Node(_) => Some(ValueKind::NodeRef),
                // No property value type admits a type; the type validator refuses one as an
                // object, so none is held.
                Object::Type(_) => None,
            });
        held.chain(asserted).collect()
    }
}
