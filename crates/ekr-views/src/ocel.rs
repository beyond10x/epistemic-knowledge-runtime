//! `ExportOcel` and its format `ekr.ocel/1` ([`export_ocel`]): one revision as an OCEL 2.0
//! object-centric event log (`views.yaml`, `ekr.views.OcelLogV1`), wrapped with its revision and
//! the names of the ids it uses.
//!
//! The event types are the overview's — the reference viewer's valid-time rule the timeline uses
//! (`ekr.views.TypeTiming`) — unless the request names them; every other node type is an object
//! type. Each node of an event type is an event at its timeline time; one without a time is left
//! out. Edges are the log's relationships, qualified by their type id, and a node's properties its
//! attributes. Types, attributes and qualifiers are named by id, because two of the store's types
//! or properties may share a name and OCEL 2.0 identifies both by it; `names` labels them.
//!
//! [`export_ocel`] reads the revision as [`crate::Index::load`] does; [`ocel`] is the pure half:
//! the same index and request give the same bytes.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{NodeId, PropertyId, RevisionNumber, TypeId};
use ekr_graph::CanonicalValue;
use ekr_kernel::Runtime;
use ekr_ontology::{Cardinality, PropertyDefinition, ValueType};
use serde::Serialize;

use crate::document::ProjectedValue;
use crate::query::{encode, hash, Answer};
use crate::{Index, ProjectError};

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
    /// The revision's edges both of whose ends are events in the log, which it does not carry.
    pub edges_between_events: u64,
    /// Nodes of an event type left out of the log for want of a writable time.
    pub undated_events: u64,
    /// Edges with such a node at an end, which the log does not carry.
    pub edges_of_undated_events: u64,
    /// Edges not carried as a relationship of their own: one of the same type between the same
    /// ends, holder and object, is already one.
    pub parallel_edges_merged: u64,
    /// Values of a property typed `time` the time form cannot write, left out of the log.
    pub attribute_values_out_of_range: u64,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub ocel_hash: String,
}

/// Why an export answered nothing.
#[derive(Debug, thiserror::Error)]
pub enum OcelError {
    /// What the revision's read refuses: `ekr.views.NotSeeded`, `ekr.views.RevisionNotFound`, or
    /// a store or revision that cannot be read.
    #[error(transparent)]
    Project(#[from] ProjectError),
    /// `ekr.views.EventTypeNotFound`: no node type of the revision's ontology holds `name`, the
    /// first such name the request lists as an event type.
    #[error("revision {revision} has no node type named {name:?}")]
    EventTypeNotFound {
        /// The name, as requested.
        name: String,
        /// The revision read.
        revision: RevisionNumber,
    },
}

/// `ekr.views.OcelMeta`.
#[derive(Serialize)]
struct OcelMeta {
    format: &'static str,
    revision: u64,
}

/// `ekr.views.OcelName`.
#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct OcelName {
    id: String,
    name: String,
}

/// `ekr.views.OcelNames`.
#[derive(Serialize)]
struct OcelNames {
    node_types: Vec<OcelName>,
    edge_types: Vec<OcelName>,
    properties: Vec<OcelName>,
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
    time: String,
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
    names: OcelNames,
    ocel: Ocel20Log,
}

/// Where a node stands in the log.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    /// An event at this time, in milliseconds.
    Event(i64),
    /// A node of an event type with no writable time: not in the log.
    Undated,
    Object,
}

/// Exports revision `at` of `runtime`'s store, or its head when `at` is `None`, as an OCEL 2.0
/// event log. `events` names the event types by node type name; empty, the overview's rule
/// decides them.
///
/// # Errors
///
/// [`OcelError::Project`] for what [`crate::Index::load`] refuses — [`ProjectError::NotSeeded`],
/// [`ProjectError::RevisionNotFound`] or the kernel's refusal of the store's history — and what
/// [`ocel`] refuses.
pub fn export_ocel(
    runtime: &Runtime,
    at: Option<RevisionNumber>,
    events: &[String],
) -> Result<Answer<OcelExported>, OcelError> {
    ocel(&Index::load(runtime, at)?, events)
}

/// The pure half of [`export_ocel`]: the `ekr.ocel/1` document of `index`'s revision, with the
/// event types `events` names, or the overview's when it names none.
///
/// # Errors
///
/// [`OcelError::EventTypeNotFound`] for the first name of `events` no node type holds;
/// [`ProjectError::Inconsistent`] for an edge whose end the revision does not hold, which a
/// revision the kernel admitted never has, or a document that does not encode.
pub fn ocel(index: &Index, events: &[String]) -> Result<Answer<OcelExported>, OcelError> {
    let graph = &index.loaded.graph;
    let declared = graph.ontology.to_document();

    // Every declared node type and every type a node has, by id text.
    let mut types: BTreeMap<String, TypeId> = declared
        .node_types
        .iter()
        .map(|node_type| (node_type.id.to_string(), node_type.id))
        .collect();
    for node in graph.nodes.values() {
        types
            .entry(node.type_id.to_string())
            .or_insert(node.type_id);
    }

    // The event types: those named, or the overview's.
    let event_types: BTreeSet<TypeId> = if events.is_empty() {
        index
            .overview
            .roles
            .types
            .iter()
            .filter(|timing| timing.event)
            .map(|timing| timing.type_id)
            .collect()
    } else {
        let mut named = BTreeSet::new();
        for name in events {
            let holders: Vec<TypeId> = declared
                .node_types
                .iter()
                .filter(|node_type| node_type.name == *name)
                .map(|node_type| node_type.id)
                .collect();
            if holders.is_empty() {
                return Err(OcelError::EventTypeNotFound {
                    name: name.clone(),
                    revision: graph.revision,
                });
            }
            named.extend(holders);
        }
        named
    };

    // Each node's role, by its index position.
    let roles: BTreeMap<NodeId, Role> = index
        .node_ids
        .iter()
        .enumerate()
        .map(|(at, node)| {
            let type_id = index.node_type[at];
            let role = if !event_types.contains(&type_id) {
                Role::Object
            } else {
                match index.node_time[at] {
                    Some((time, _)) if (FIRST_WRITABLE_MS..=LAST_WRITABLE_MS).contains(&time) => {
                        Role::Event(time)
                    }
                    _ => Role::Undated,
                }
            };
            (*node, role)
        })
        .collect();

    // Each type's attributes: its properties, its ancestors' included, by id.
    let declarations: BTreeMap<TypeId, BTreeMap<PropertyId, &PropertyDefinition>> = types
        .values()
        .map(|type_id| (*type_id, graph.ontology.properties_of(*type_id)))
        .collect();
    let (mut ocel_event_types, mut ocel_object_types) = (Vec::new(), Vec::new());
    for (name, type_id) in &types {
        let described = OcelType {
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
            ocel_event_types.push(described);
        } else {
            ocel_object_types.push(described);
        }
    }

    // Each edge, once per (holder, object, qualifier).
    let mut related: BTreeMap<NodeId, BTreeSet<(String, String)>> = BTreeMap::new();
    let (mut edges_between_events, mut edges_of_undated_events) = (0_u64, 0_u64);
    let mut parallel_edges_merged = 0_u64;
    for edge in graph.edges.values() {
        let (source, target) = (edge.source.id(), edge.target.id());
        let role = |end: NodeId| {
            roles.get(&end).copied().ok_or_else(|| {
                ProjectError::Inconsistent(format!(
                    "edge {} names node {end}, which revision {} does not hold",
                    edge.id, graph.revision
                ))
            })
        };
        let (from, to) = (role(source)?, role(target)?);
        let (holder, object) = match (from, to) {
            (Role::Undated, _) | (_, Role::Undated) => {
                edges_of_undated_events += 1;
                continue;
            }
            (Role::Event(_), Role::Event(_)) => {
                edges_between_events += 1;
                continue;
            }
            (Role::Object, Role::Event(_)) => (target, source),
            (Role::Event(_) | Role::Object, Role::Object) => (source, target),
        };
        let fresh = related
            .entry(holder)
            .or_default()
            .insert((object.to_string(), edge.type_id.to_string()));
        if !fresh {
            parallel_edges_merged += 1;
        }
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

    let mut ocel_events = Vec::new();
    let mut objects = Vec::new();
    let (mut event_object, mut object_object, mut undated_events) = (0_u64, 0_u64, 0_u64);
    let mut attribute_values_out_of_range = 0_u64;
    // An object's attribute values start at 1970-01-01, OCEL 2.0's initial time, or at the log's
    // earliest event when that is before it, so that every event sees its objects' values.
    let initial = roles
        .values()
        .filter_map(|role| match role {
            Role::Event(time) => Some(*time),
            Role::Undated | Role::Object => None,
        })
        .min()
        .filter(|earliest| *earliest < 0)
        .and_then(rfc3339)
        .unwrap_or_else(|| INITIAL.to_owned());
    for node in graph.nodes.values() {
        let role = roles[&node.id];
        if role == Role::Undated {
            undated_events += 1;
            continue;
        }
        let described = &declarations[&node.type_id];
        let mut attributes = Vec::with_capacity(node.properties.len());
        for (property, values) in &node.properties {
            let Some(value) = value_text(described.get(property).copied(), values)? else {
                attribute_values_out_of_range += 1;
                continue;
            };
            attributes.push((property.to_string(), value));
        }
        let held = relationships(&node.id);
        if let Role::Event(time) = role {
            event_object += held.len() as u64;
            ocel_events.push((
                time,
                OcelEvent {
                    id: node.id.to_string(),
                    event_type: node.type_id.to_string(),
                    time: rfc3339(time).unwrap_or_default(),
                    attributes: attributes
                        .into_iter()
                        .map(|(name, value)| OcelEventAttribute { name, value })
                        .collect(),
                    relationships: held,
                },
            ));
        } else {
            object_object += held.len() as u64;
            objects.push(OcelObject {
                id: node.id.to_string(),
                object_type: node.type_id.to_string(),
                attributes: attributes
                    .into_iter()
                    .map(|(name, value)| OcelObjectAttribute {
                        name,
                        value,
                        time: initial.clone(),
                    })
                    .collect(),
                relationships: held,
            });
        }
    }
    ocel_events.sort_by(|(a, left), (b, right)| a.cmp(b).then_with(|| left.id.cmp(&right.id)));
    objects.sort_by(|left, right| left.id.cmp(&right.id));

    // The names of the ids the log uses, as the revision's ontology holds them.
    let named = |id: String, name: &str| OcelName {
        id,
        name: name.to_owned(),
    };
    let mut node_type_names: Vec<OcelName> = declared
        .node_types
        .iter()
        .map(|node_type| named(node_type.id.to_string(), &node_type.name))
        .collect();
    node_type_names.sort();
    let mut edge_type_names: Vec<OcelName> = declared
        .edge_types
        .iter()
        .map(|edge_type| named(edge_type.id.to_string(), &edge_type.name))
        .collect();
    edge_type_names.sort();
    let property_names: BTreeSet<OcelName> = declared
        .node_types
        .iter()
        .flat_map(|node_type| node_type.properties.values())
        .chain(
            declared
                .edge_types
                .iter()
                .flat_map(|edge_type| edge_type.properties.values()),
        )
        .map(|property| named(property.id.to_string(), &property.name))
        .collect();

    let document = OcelLogV1 {
        meta: OcelMeta {
            format: OCEL_FORMAT,
            revision: graph.revision.get(),
        },
        names: OcelNames {
            node_types: node_type_names,
            edge_types: edge_type_names,
            properties: property_names.into_iter().collect(),
        },
        ocel: Ocel20Log {
            event_types: ocel_event_types,
            object_types: ocel_object_types,
            events: ocel_events.into_iter().map(|(_, event)| event).collect(),
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
        undated_events,
        edges_of_undated_events,
        parallel_edges_merged,
        attribute_values_out_of_range,
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
/// writes it in `props` — except under a declaration typed `time`, whose attribute holds a time
/// or is left out (`None`).
fn value_text(
    declaration: Option<&PropertyDefinition>,
    values: &[CanonicalValue],
) -> Result<Option<String>, ProjectError> {
    if let Some(declaration) = declaration {
        let (typed, scalar) = attribute_type(declaration);
        let text = match values {
            [value] if scalar => scalar_text(&declaration.value_type, value),
            _ => None,
        };
        if text.is_some() || typed == "time" {
            return Ok(text);
        }
    }
    let projected: Vec<ProjectedValue> = values.iter().map(ProjectedValue::from).collect();
    serde_json::to_string(&projected)
        .map(Some)
        .map_err(|error| ProjectError::Inconsistent(format!("encoding a value list: {error}")))
}

/// A value's text under the scalar value type `declared`, when it is of that kind and, for a
/// Timestamp, when the time form writes it.
fn scalar_text(declared: &ValueType, value: &CanonicalValue) -> Option<String> {
    Some(match (declared, value) {
        (ValueType::String, CanonicalValue::String(text))
        | (ValueType::Enum { .. }, CanonicalValue::Enum(text))
        | (ValueType::Decimal, CanonicalValue::Decimal(text)) => text.clone(),
        (ValueType::Boolean, CanonicalValue::Boolean(truth)) => truth.to_string(),
        (ValueType::Integer, CanonicalValue::Integer(number))
        | (ValueType::Duration, CanonicalValue::Duration(number)) => number.to_string(),
        (ValueType::Timestamp, CanonicalValue::Timestamp(at)) => rfc3339(at.millis())?,
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
