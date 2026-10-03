//! Deterministic read-only analysis for the explicit knowledge-authority upgrade.
use ekr_core::contract_data::{
    EkrGraphAssertionId, EkrGraphPredicateKind, EkrGraphPredicateProjection, EkrGraphSubjectKind,
    EkrGraphSubjectProjection, EkrKernelUpgradeContradiction,
};
use ekr_core::Timestamp;
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, CanonicalValue, Object, Predicate,
    Subject, TemporalRange,
};
use ekr_ontology::Cardinality;
use std::collections::BTreeMap;

/// Recompute assessments from admitted claims and retained validation attribution.
/// Only the explicit knowledge authority calls this while constructing a new revision.
pub(crate) fn recompute(
    graph: &mut CanonicalGraph,
    validators: &mut BTreeMap<ekr_core::AssertionId, std::collections::BTreeSet<ekr_core::AgentId>>,
) -> Result<(), ekr_store::StoreError> {
    for assertion in graph.assertions.values() {
        if let Assessment::Accepted {
            validators: accepted,
        } = &assertion.assessment
        {
            validators.insert(assertion.id, accepted.clone());
        }
    }
    let mut competitors: BTreeMap<ekr_core::AssertionId, Vec<ekr_core::AssertionId>> =
        BTreeMap::new();
    for pair in preview_contradictions(graph) {
        let left = pair
            .left
            .0
            .parse()
            .map_err(|_| crate::replay::refuse("dispute-assertion-id"))?;
        let right = pair
            .right
            .0
            .parse()
            .map_err(|_| crate::replay::refuse("dispute-assertion-id"))?;
        competitors.entry(left).or_default().push(right);
        competitors.entry(right).or_default().push(left);
    }
    for assertion in graph.assertions.values_mut() {
        if let Some(ids) = competitors.get_mut(&assertion.id) {
            ids.sort();
            ids.dedup();
            assertion.assessment = Assessment::Disputed {
                competing_assertions: ids
                    .iter()
                    .map(|id| ekr_graph::CanonicalRef::new(*id))
                    .collect(),
            };
        } else if matches!(assertion.assessment, Assessment::Disputed { .. }) {
            assertion.assessment = Assessment::Accepted {
                validators: validators.get(&assertion.id).cloned().ok_or_else(|| {
                    crate::replay::refuse("dispute-validation-attribution-missing")
                })?,
            };
        }
    }
    Ok(())
}

/// Preview conflicting active claims under declared One semantics in an admitted graph.
///
/// Both accepted and already-disputed claims participate. Retracted, superseded, rejected and
/// unfinished claims do not. Property lookup uses the subject's effective declaration, including
/// inherited node properties and edge properties. Relation cardinality is per source node.
/// Equal node-reference property values compare equally through either admitted object encoding.
/// Results contain each pair once, ordered by assertion identity, independent of insertion order.
/// This does not mutate assessments, authority, history or physical node/edge properties.
#[must_use]
pub fn preview_contradictions(graph: &CanonicalGraph) -> Vec<EkrKernelUpgradeContradiction> {
    let mut groups: BTreeMap<(Subject, Predicate), Vec<&Assertion>> = BTreeMap::new();
    for claim in graph.assertions.values() {
        if matches!(claim.lifecycle, AssertionLifecycle::Active)
            && claim.transaction_time.is_open()
            && matches!(
                claim.assessment,
                Assessment::Accepted { .. } | Assessment::Disputed { .. }
            )
        {
            groups
                .entry((claim.subject, claim.predicate))
                .or_default()
                .push(claim);
        }
    }
    let mut conflicts = Vec::new();
    for ((subject, predicate), claims) in groups {
        if !single_valued(graph, subject, predicate) {
            continue;
        }
        for (index, left) in claims.iter().enumerate() {
            for right in &claims[index + 1..] {
                if !same_value(&left.object, &right.object)
                    && overlaps(left.valid_time, right.valid_time)
                {
                    conflicts.push((
                        left.id.min(right.id),
                        left.id.max(right.id),
                        subject,
                        predicate,
                    ));
                }
            }
        }
    }
    conflicts.sort_by_key(|(left, right, _, _)| (*left, *right));
    conflicts
        .into_iter()
        .map(
            |(left, right, subject, predicate)| EkrKernelUpgradeContradiction {
                left: Box::new(EkrGraphAssertionId(left.to_string())),
                right: Box::new(EkrGraphAssertionId(right.to_string())),
                subject: Box::new(project_subject(subject)),
                predicate: Box::new(project_predicate(predicate)),
            },
        )
        .collect()
}

fn single_valued(graph: &CanonicalGraph, subject: Subject, predicate: Predicate) -> bool {
    match predicate {
        Predicate::Relation(id) => {
            matches!(subject, Subject::Node(_))
                && graph
                    .ontology
                    .edge_type(id)
                    .is_some_and(|definition| definition.cardinality == Cardinality::One)
        }
        Predicate::Property(property) => match subject {
            Subject::Node(node) => graph.nodes.get(&node.id()).is_some_and(|node| {
                graph
                    .ontology
                    .properties_of(node.type_id)
                    .get(&property)
                    .is_some_and(|definition| definition.cardinality == Cardinality::One)
            }),
            Subject::Edge(edge) => graph
                .edges
                .get(&edge.id())
                .and_then(|edge| graph.ontology.edge_type(edge.type_id))
                .and_then(|definition| definition.properties.get(&property))
                .is_some_and(|definition| definition.cardinality == Cardinality::One),
            // No metatype declares properties of ontology definitions in the current runtime.
            Subject::Type(_) => false,
        },
    }
}
fn same_value(left: &Object, right: &Object) -> bool {
    match (left, right) {
        (Object::Node(left), Object::Value(CanonicalValue::NodeRef(right)))
        | (Object::Value(CanonicalValue::NodeRef(left)), Object::Node(right)) => left == right,
        _ => left == right,
    }
}
fn overlaps(left: TemporalRange, right: TemporalRange) -> bool {
    // None at the lower bound includes the first representable instant. None at the upper
    // bound includes the last one; using MAX as an exclusive sentinel would lose that instant.
    let first = Timestamp::from_millis(i64::MIN);
    let from = left.from.unwrap_or(first).max(right.from.unwrap_or(first));
    let to = match (left.to, right.to) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (left, None) => left,
        (None, right) => right,
    };
    to.is_none_or(|to| from < to)
}
fn project_subject(subject: Subject) -> EkrGraphSubjectProjection {
    let (kind, id) = match subject {
        Subject::Node(node) => (EkrGraphSubjectKind::V1, node.id().to_string()),
        Subject::Edge(edge) => (EkrGraphSubjectKind::V0, edge.id().to_string()),
        Subject::Type(id) => (EkrGraphSubjectKind::V2, id.to_string()),
    };
    EkrGraphSubjectProjection {
        kind: Box::new(kind),
        id,
    }
}
fn project_predicate(predicate: Predicate) -> EkrGraphPredicateProjection {
    let (kind, id) = match predicate {
        Predicate::Property(id) => (EkrGraphPredicateKind::V0, id.to_string()),
        Predicate::Relation(id) => (EkrGraphPredicateKind::V1, id.to_string()),
    };
    EkrGraphPredicateProjection {
        kind: Box::new(kind),
        id,
    }
}
