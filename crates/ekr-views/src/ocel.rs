//! `ExportOcel` and its format `ekr.ocel/1` ([`export_ocel`]): one revision as an OCEL 2.0
//! object-centric event log (`views.yaml`, `ekr.views.OcelLogV1`), wrapped with its revision.
//!
//! Every role comes from the store's shape, never from a name. A node type whose every node has a
//! valid-time start — the least `valid_from` of the unretracted assertions about the node — is an
//! event type, each of its nodes an event at that start; every other node type is an object type.
//! Edges are the log's relationships, qualified by their type id, and a node's properties its
//! attributes. Types, attributes and qualifiers are named by id, because two of the store's types
//! or properties may share a name and OCEL 2.0 identifies both by it.
//!
//! [`export_ocel`] reads the revision as [`crate::load`] does; [`ocel`] is the pure half: the same
//! revision gives the same bytes.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{NodeId, PropertyId, RevisionNumber, TypeId};
use ekr_graph::{AssertionLifecycle, CanonicalValue, Subject};
use ekr_kernel::Runtime;
use ekr_ontology::{Cardinality, PropertyDefinition, ValueType};
use serde::Serialize;

use crate::document::ProjectedValue;
use crate::query::{encode, hash, Answer};
use crate::{LoadedRevision, ProjectError};

/// The format literal every export carries in `meta.format`.
pub const OCEL_FORMAT: &str = "ekr.ocel/1";

/// The time OCEL 2.0 gives an object's initial attribute values (its specification, § 6.4).
const INITIAL: &str = "1970-01-01T00:00:00.000Z";

/// 0000-01-01T00:00:00.000Z and 9999-12-31T23:59:59.999Z: the instants RFC 3339 writes.
const FIRST_WRITABLE_MS: i64 = -62_167_219_200_000;
const LAST_WRITABLE_MS: i64 = 253_402_300_799_999;

/// What one export returned, counted over the document: `ekr.views.OcelExported`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OcelExported {
    /// `meta.revision`.
    pub revision: u64,
    /// Entries of `ocel.eventTypes`.
    pub event_types: u64,
    /// Entries of `ocel.objectTypes`.
    pub object_types: u64,
    /// Entries of `ocel.events`.
    pub events: u64,
    /// Entries of `ocel.objects`.
    pub objects: u64,
    /// Relationships the events hold.
    pub event_object_relationships: u64,
    /// Relationships the objects hold.
    pub object_object_relationships: u64,
    /// The revision's edges both of whose ends are events, which the log does not carry.
    pub edges_between_events: u64,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub ocel_hash: String,
}

/// `ekr.views.OcelMeta`.
#[derive(Serialize)]
struct OcelMeta {
    format: &'static str,
    revision: u64,
}

/// `ekr.views.OcelTypeAttribute`.
#[derive(Serialize)]
struct OcelTypeAttribute {
    name: String,
    #[serde(rename = "type")]
    value_type: &'static str,
}

/// `ekr.views.OcelType`.
#[derive(Serialize)]
struct OcelType {
    name: String,
    attributes: Vec<OcelTypeAttribute>,
}

/// `ekr.views.OcelEventAttribute`.
#[derive(Serialize)]
struct OcelEventAttribute {
    name: String,
    value: String,
}

/// `ekr.views.OcelObjectAttribute`.
#[derive(Serialize)]
struct OcelObjectAttribute {
    name: String,
    value: String,
    time: &'static str,
}

/// `ekr.views.OcelRelationship`.
#[derive(Serialize)]
struct OcelRelationship {
    #[serde(rename = "objectId")]
    object_id: String,
    qualifier: String,
}

/// `ekr.views.OcelEvent`.
#[derive(Serialize)]
struct OcelEvent {
    id: String,
    #[serde(rename = "type")]
    event_type: String,
    time: String,
    attributes: Vec<OcelEventAttribute>,
    relationships: Vec<OcelRelationship>,
}

/// `ekr.views.OcelObject`.
#[derive(Serialize)]
struct OcelObject {
    id: String,
    #[serde(rename = "type")]
    object_type: String,
    attributes: Vec<OcelObjectAttribute>,
    relationships: Vec<OcelRelationship>,
}

/// `ekr.views.Ocel20Log`.
#[derive(Serialize)]
struct Ocel20Log {
    #[serde(rename = "eventTypes")]
    event_types: Vec<OcelType>,
    #[serde(rename = "objectTypes")]
    object_types: Vec<OcelType>,
    events: Vec<OcelEvent>,
    objects: Vec<OcelObject>,
}

/// `ekr.views.OcelLogV1`.
#[derive(Serialize)]
struct OcelLogV1 {
    meta: OcelMeta,
    ocel: Ocel20Log,
}

/// Exports revision `at` of `runtime`'s store, or its head when `at` is `None`, as an OCEL 2.0
/// event log.
///
/// # Errors
///
/// What [`crate::load`] refuses — [`ProjectError::NotSeeded`], [`ProjectError::RevisionNotFound`]
/// or the kernel's refusal of the store's history — and what [`ocel`] refuses.
pub fn export_ocel(
    runtime: &Runtime,
    at: Option<RevisionNumber>,
) -> Result<Answer<OcelExported>, ProjectError> {
    ocel(&crate::load(runtime, at)?)
}

/// The pure half of [`export_ocel`]: the `ekr.ocel/1` document of `loaded`.
///
/// # Errors
///
/// [`ProjectError::Inconsistent`] for an edge whose end the revision does not hold, which a
/// revision the kernel admitted never has, or a document that does not encode.
pub fn ocel(loaded: &LoadedRevision) -> Result<Answer<OcelExported>, ProjectError> {
    let graph = &loaded.graph;

    // Each node's valid-time start: the least writable valid_from of the unretracted assertions
    // about it.
    let mut starts: BTreeMap<NodeId, i64> = BTreeMap::new();
    for assertion in graph.assertions.values() {
        let Subject::Node(node) = &assertion.subject else {
            continue;
        };
        if matches!(assertion.lifecycle, AssertionLifecycle::Retracted { .. }) {
            continue;
        }
        let Some(from) = assertion.valid_time.from.map(|at| at.millis()) else {
            continue;
        };
        if !(FIRST_WRITABLE_MS..=LAST_WRITABLE_MS).contains(&from) {
            continue;
        }
        starts
            .entry(node.id())
            .and_modify(|start| *start = (*start).min(from))
            .or_insert(from);
    }

    // Every declared node type and every type a node has, by id text; an event type has a node
    // and every node of it a start.
    let mut types: BTreeMap<String, TypeId> = graph
        .ontology
        .to_document()
        .node_types
        .iter()
        .map(|declared| (declared.id.to_string(), declared.id))
        .collect();
    let mut dated: BTreeMap<TypeId, (u64, u64)> = BTreeMap::new();
    for node in graph.nodes.values() {
        types
            .entry(node.type_id.to_string())
            .or_insert(node.type_id);
        let (nodes, with_start) = dated.entry(node.type_id).or_default();
        *nodes += 1;
        if starts.contains_key(&node.id) {
            *with_start += 1;
        }
    }
    let event_types: BTreeSet<TypeId> = dated
        .iter()
        .filter(|(_, (nodes, with_start))| *nodes > 0 && nodes == with_start)
        .map(|(type_id, _)| *type_id)
        .collect();
    let is_event = |node: &NodeId| {
        graph
            .nodes
            .get(node)
            .is_some_and(|held| event_types.contains(&held.type_id))
    };

    // Each type's attributes: its properties, its ancestors' included, by id.
    let declarations: BTreeMap<TypeId, BTreeMap<PropertyId, &PropertyDefinition>> = types
        .values()
        .map(|type_id| (*type_id, graph.ontology.properties_of(*type_id)))
        .collect();
    let (mut ocel_event_types, mut ocel_object_types) = (Vec::new(), Vec::new());
    for (name, type_id) in &types {
        let declared = OcelType {
            name: name.clone(),
            attributes: declarations[type_id]
                .iter()
                .map(|(property, definition)| OcelTypeAttribute {
                    name: property.to_string(),
                    value_type: attribute_type(definition).0,
                })
                .collect(),
        };
        if event_types.contains(type_id) {
            ocel_event_types.push(declared);
        } else {
            ocel_object_types.push(declared);
        }
    }

    // Each edge, once per (holder, object, qualifier).
    let mut related: BTreeMap<NodeId, BTreeSet<(String, String)>> = BTreeMap::new();
    let mut edges_between_events = 0_u64;
    for edge in graph.edges.values() {
        let (source, target) = (edge.source.id(), edge.target.id());
        for end in [source, target] {
            if !graph.nodes.contains_key(&end) {
                return Err(ProjectError::Inconsistent(format!(
                    "edge {} names node {end}, which revision {} does not hold",
                    edge.id, graph.revision
                )));
            }
        }
        let (holder, object) = match (is_event(&source), is_event(&target)) {
            (true, true) => {
                edges_between_events += 1;
                continue;
            }
            (false, true) => (target, source),
            (true, false) | (false, false) => (source, target),
        };
        related
            .entry(holder)
            .or_default()
            .insert((object.to_string(), edge.type_id.to_string()));
    }
    let relationships = |node: &NodeId| -> Vec<OcelRelationship> {
        related
            .get(node)
            .map(|held| {
                held.iter()
                    .map(|(object_id, qualifier)| OcelRelationship {
                        object_id: object_id.clone(),
                        qualifier: qualifier.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let mut events = Vec::new();
    let mut objects = Vec::new();
    let (mut event_object, mut object_object) = (0_u64, 0_u64);
    for node in graph.nodes.values() {
        let declared = &declarations[&node.type_id];
        let mut attributes = Vec::with_capacity(node.properties.len());
        for (property, values) in &node.properties {
            attributes.push((
                property.to_string(),
                value_text(declared.get(property).copied(), values)?,
            ));
        }
        let held = relationships(&node.id);
        match starts.get(&node.id) {
            Some(start) if event_types.contains(&node.type_id) => {
                event_object += held.len() as u64;
                events.push((
                    *start,
                    OcelEvent {
                        id: node.id.to_string(),
                        event_type: node.type_id.to_string(),
                        time: rfc3339(*start).unwrap_or_default(),
                        attributes: attributes
                            .into_iter()
                            .map(|(name, value)| OcelEventAttribute { name, value })
                            .collect(),
                        relationships: held,
                    },
                ));
            }
            _ => {
                object_object += held.len() as u64;
                objects.push(OcelObject {
                    id: node.id.to_string(),
                    object_type: node.type_id.to_string(),
                    attributes: attributes
                        .into_iter()
                        .map(|(name, value)| OcelObjectAttribute {
                            name,
                            value,
                            time: INITIAL,
                        })
                        .collect(),
                    relationships: held,
                });
            }
        }
    }
    events.sort_by(|(a, left), (b, right)| a.cmp(b).then_with(|| left.id.cmp(&right.id)));
    objects.sort_by(|left, right| left.id.cmp(&right.id));

    let document = OcelLogV1 {
        meta: OcelMeta {
            format: OCEL_FORMAT,
            revision: graph.revision.get(),
        },
        ocel: Ocel20Log {
            event_types: ocel_event_types,
            object_types: ocel_object_types,
            events: events.into_iter().map(|(_, event)| event).collect(),
            objects,
        },
    };
    let bytes = encode(&document)?;
    let summary = OcelExported {
        revision: document.meta.revision,
        event_types: document.ocel.event_types.len() as u64,
        object_types: document.ocel.object_types.len() as u64,
        events: document.ocel.events.len() as u64,
        objects: document.ocel.objects.len() as u64,
        event_object_relationships: event_object,
        object_object_relationships: object_object,
        edges_between_events,
        ocel_hash: hash(&bytes),
    };
    Ok(Answer { bytes, summary })
}

/// A declaration's OCEL 2.0 attribute type, and whether it is scalar: cardinality One and a value
/// type OCEL 2.0 has a type for.
fn attribute_type(definition: &PropertyDefinition) -> (&'static str, bool) {
    if definition.cardinality != Cardinality::One {
        return ("string", false);
    }
    match definition.value_type {
        ValueType::String | ValueType::Enum { .. } | ValueType::NodeRef { .. } => ("string", true),
        ValueType::Boolean => ("boolean", true),
        ValueType::Integer | ValueType::Duration => ("integer", true),
        ValueType::Float | ValueType::Decimal => ("float", true),
        ValueType::Timestamp => ("time", true),
        ValueType::List(_) | ValueType::Record(_) => ("string", false),
    }
}

/// A node's values of one property as an attribute value: a single value of a scalar
/// declaration's kind as its text, anything else as the compact JSON of the list as a projection
/// writes it in `props`.
fn value_text(
    declaration: Option<&PropertyDefinition>,
    values: &[CanonicalValue],
) -> Result<String, ProjectError> {
    if let (Some(declaration), [value]) = (declaration, values) {
        if attribute_type(declaration).1 {
            if let Some(text) = scalar_text(&declaration.value_type, value) {
                return Ok(text);
            }
        }
    }
    let projected: Vec<ProjectedValue> = values.iter().map(ProjectedValue::from).collect();
    serde_json::to_string(&projected)
        .map_err(|error| ProjectError::Inconsistent(format!("encoding a value list: {error}")))
}

/// A value's text under the scalar value type `declared`, when it is of that kind.
fn scalar_text(declared: &ValueType, value: &CanonicalValue) -> Option<String> {
    Some(match (declared, value) {
        (ValueType::String, CanonicalValue::String(text))
        | (ValueType::Enum { .. }, CanonicalValue::Enum(text))
        | (ValueType::Decimal, CanonicalValue::Decimal(text)) => text.clone(),
        (ValueType::Boolean, CanonicalValue::Boolean(truth)) => truth.to_string(),
        (ValueType::Integer, CanonicalValue::Integer(number))
        | (ValueType::Duration, CanonicalValue::Duration(number)) => number.to_string(),
        (ValueType::Timestamp, CanonicalValue::Timestamp(at)) => {
            rfc3339(at.millis()).unwrap_or_else(|| at.millis().to_string())
        }
        (ValueType::NodeRef { .. }, CanonicalValue::NodeRef(node)) => node.id().to_string(),
        _ => return None,
    })
}

/// `millis` as `YYYY-MM-DDTHH:MM:SS.mmmZ`, or `None` outside the years 0000 to 9999.
fn rfc3339(millis: i64) -> Option<String> {
    if !(FIRST_WRITABLE_MS..=LAST_WRITABLE_MS).contains(&millis) {
        return None;
    }
    let (days, within) = (millis.div_euclid(86_400_000), millis.rem_euclid(86_400_000));
    // Days since the epoch to a proleptic Gregorian date (H. Hinnant, `civil_from_days`).
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1_460 + of_era / 36_524 - of_era / 146_096) / 365;
    let day_of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        within / 3_600_000,
        within / 60_000 % 60,
        within / 1_000 % 60,
        within % 1_000
    ))
}

#[cfg(test)]
mod tests {
    use super::rfc3339;

    #[test]
    fn times_are_rfc_3339_in_utc_with_milliseconds_within_years_0000_to_9999() {
        assert_eq!(rfc3339(0).as_deref(), Some("1970-01-01T00:00:00.000Z"));
        assert_eq!(
            rfc3339(1_798_794_000_123).as_deref(),
            Some("2027-01-01T09:00:00.123Z")
        );
        assert_eq!(rfc3339(-1).as_deref(), Some("1969-12-31T23:59:59.999Z"));
        assert_eq!(
            rfc3339(951_782_400_000).as_deref(),
            Some("2000-02-29T00:00:00.000Z")
        );
        assert_eq!(
            rfc3339(-62_167_219_200_000).as_deref(),
            Some("0000-01-01T00:00:00.000Z")
        );
        assert_eq!(
            rfc3339(253_402_300_799_999).as_deref(),
            Some("9999-12-31T23:59:59.999Z")
        );
        assert_eq!(rfc3339(-62_167_219_200_001), None);
        assert_eq!(rfc3339(253_402_300_800_000), None);
    }
}
