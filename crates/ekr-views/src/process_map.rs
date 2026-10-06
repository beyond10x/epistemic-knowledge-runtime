//! `ProjectProcessMap` and its format `ekr.process-map/1` ([`export_process_map`]): one
//! revision's OCEL 2.0 log read as a process (`views.yaml`, `ekr.views.ProcessMapV1`), per object
//! type its variants and its directly-follows graph.
//!
//! The map is derived from the `ekr.ocel/1` document [`crate::export_ocel_with_event_time`]
//! answers for the same request, and from nothing else: `meta.ocel_hash` names that document by
//! its hash. Each object of the log is a case, and its trace the types of the events relating to
//! it, in the log's event order — by time, then id — an event relating to it under two
//! qualifiers being one step. A variant is a distinct trace with the number of cases following
//! it; a directly-follows edge a pair of event types one case's trace holds one after the other,
//! counted once per occurrence. An object no event relates to is no case.
//!
//! [`process_map`] is the pure half: the same `ekr.ocel/1` bytes give the same map bytes.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use serde::{Deserialize, Serialize};

use crate::ocel::{ocel_with_event_time, OcelError, OCEL_FORMAT};
use crate::query::{encode, hash, Answer};
use crate::{Index, ProjectError};

/// The format literal every map carries in `meta.format`.
pub const PROCESS_MAP_FORMAT: &str = "ekr.process-map/1";

/// What one map returned, counted over the document: `ekr.views.ProcessMapped`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ProcessMapped {
    /// `meta.revision`.
    pub revision: u64,
    /// Entries of `object_types`.
    pub object_types: u64,
    /// Cases over every object type: objects some event relates to.
    pub cases: u64,
    /// Variant entries over every object type.
    pub variants: u64,
    /// Directly-follows entries over every object type.
    pub directly_follows: u64,
    /// `meta.ocel_hash`: the hex SHA-256 of the `ekr.ocel/1` document the map was derived from.
    pub ocel_hash: String,
    /// The lowercase hex SHA-256 of the map's exact bytes.
    pub process_map_hash: String,
}

/// The bytes [`process_map`] was given are not an `ekr.ocel/1` document.
#[derive(Debug, thiserror::Error)]
#[error("not an ekr.ocel/1 document: {0}")]
pub struct OcelMalformed(pub String);

/// The part of `ekr.views.OcelLogV1` the map reads; every other member is ignored.
#[derive(Deserialize)]
struct OcelInput {
    meta: OcelInputMeta,
    names: OcelInputNames,
    ocel: OcelInputLog,
}

#[derive(Deserialize)]
struct OcelInputMeta {
    format: String,
    revision: u64,
}

#[derive(Deserialize)]
struct OcelInputNames {
    node_types: Vec<ProcessMapName>,
}

#[derive(Deserialize)]
struct OcelInputLog {
    #[serde(rename = "objectTypes")]
    object_types: Vec<OcelInputType>,
    events: Vec<OcelInputEvent>,
    objects: Vec<OcelInputObject>,
}

#[derive(Deserialize)]
struct OcelInputType {
    name: String,
}

#[derive(Deserialize)]
struct OcelInputEvent {
    id: String,
    #[serde(rename = "type")]
    event_type: String,
    time: String,
    relationships: Vec<OcelInputRelationship>,
}

#[derive(Deserialize)]
struct OcelInputRelationship {
    #[serde(rename = "objectId")]
    object_id: String,
}

#[derive(Deserialize)]
struct OcelInputObject {
    id: String,
    #[serde(rename = "type")]
    object_type: String,
}

/// `ekr.views.ProcessMapMeta`.
#[derive(Serialize)]
struct ProcessMapMeta {
    format: &'static str,
    revision: u64,
    ocel_hash: String,
}

/// `ekr.views.OcelName`, as the log's `names` holds it.
#[derive(Serialize, Deserialize)]
struct ProcessMapName {
    id: String,
    name: String,
}

/// `ekr.views.ProcessMapNames`.
#[derive(Serialize)]
struct ProcessMapNames {
    node_types: Vec<ProcessMapName>,
}

/// `ekr.views.ProcessVariant`.
#[derive(Serialize)]
struct ProcessVariant {
    activities: Vec<String>,
    cases: u64,
}

/// `ekr.views.DirectlyFollows`.
#[derive(Serialize)]
struct DirectlyFollows {
    from: String,
    to: String,
    count: u64,
}

/// `ekr.views.ObjectTypeProcess`.
#[derive(Serialize)]
struct ObjectTypeProcess {
    object_type: String,
    objects: u64,
    cases: u64,
    variants: Vec<ProcessVariant>,
    directly_follows: Vec<DirectlyFollows>,
}

/// `ekr.views.ProcessMapV1`.
#[derive(Serialize)]
struct ProcessMapV1 {
    meta: ProcessMapMeta,
    names: ProcessMapNames,
    object_types: Vec<ObjectTypeProcess>,
}

/// One object type's figures while they are counted.
#[derive(Default)]
struct Counted {
    objects: u64,
    cases: u64,
    variants: BTreeMap<Vec<String>, u64>,
    directly_follows: BTreeMap<(String, String), u64>,
}

/// Maps revision `at` of `runtime`'s store, or its head when `at` is `None`: the
/// `ekr.process-map/1` document of the `ekr.ocel/1` log [`crate::export_ocel_with_event_time`]
/// answers for `events` and `event_time`.
///
/// # Errors
///
/// What [`crate::export_ocel_with_event_time`] refuses, as it refuses it;
/// [`ProjectError::Inconsistent`] when that log does not read back, which an export never gives.
pub fn export_process_map(
    runtime: &Runtime,
    at: Option<RevisionNumber>,
    events: &[String],
    event_time: &[String],
) -> Result<Answer<ProcessMapped>, OcelError> {
    let log = ocel_with_event_time(&Index::load(runtime, at)?, events, event_time)?;
    process_map(&log.bytes)
        .map_err(|malformed| OcelError::Project(ProjectError::Inconsistent(malformed.to_string())))
}

/// The pure half of [`export_process_map`]: the `ekr.process-map/1` document of `ocel`, the bytes
/// of an `ekr.ocel/1` document.
///
/// # Errors
///
/// [`OcelMalformed`] when `ocel` is not JSON, lacks a member the map reads, or names another
/// format in `meta.format`.
pub fn process_map(ocel: &[u8]) -> Result<Answer<ProcessMapped>, OcelMalformed> {
    let log: OcelInput =
        serde_json::from_slice(ocel).map_err(|error| OcelMalformed(error.to_string()))?;
    if log.meta.format != OCEL_FORMAT {
        return Err(OcelMalformed(format!(
            "meta.format is {:?}",
            log.meta.format
        )));
    }

    // Each object's trace: the events relating to it, in the log's order.
    let mut events: Vec<&OcelInputEvent> = log.ocel.events.iter().collect();
    events.sort_by(|left, right| {
        (left.time.as_str(), left.id.as_str()).cmp(&(right.time.as_str(), right.id.as_str()))
    });
    let mut traces: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for event in events {
        let related: BTreeSet<&str> = event
            .relationships
            .iter()
            .map(|relationship| relationship.object_id.as_str())
            .collect();
        for object in related {
            traces
                .entry(object)
                .or_default()
                .push(event.event_type.as_str());
        }
    }

    let mut counted: BTreeMap<&str, Counted> = log
        .ocel
        .object_types
        .iter()
        .map(|object_type| (object_type.name.as_str(), Counted::default()))
        .collect();
    for object in &log.ocel.objects {
        let figures = counted.entry(object.object_type.as_str()).or_default();
        figures.objects += 1;
        let Some(trace) = traces.get(object.id.as_str()) else {
            continue;
        };
        figures.cases += 1;
        for pair in trace.windows(2) {
            *figures
                .directly_follows
                .entry((pair[0].to_owned(), pair[1].to_owned()))
                .or_default() += 1;
        }
        *figures
            .variants
            .entry(trace.iter().map(|kind| (*kind).to_owned()).collect())
            .or_default() += 1;
    }

    let (mut cases, mut variant_count, mut edge_count) = (0_u64, 0_u64, 0_u64);
    let object_types: Vec<ObjectTypeProcess> = counted
        .into_iter()
        .map(|(object_type, figures)| {
            let mut variants: Vec<ProcessVariant> = figures
                .variants
                .into_iter()
                .map(|(activities, cases)| ProcessVariant { activities, cases })
                .collect();
            // The most followed first; variants of one count by their activities.
            variants.sort_by(|left, right| {
                right
                    .cases
                    .cmp(&left.cases)
                    .then_with(|| left.activities.cmp(&right.activities))
            });
            cases += figures.cases;
            variant_count += variants.len() as u64;
            edge_count += figures.directly_follows.len() as u64;
            ObjectTypeProcess {
                object_type: object_type.to_owned(),
                objects: figures.objects,
                cases: figures.cases,
                variants,
                directly_follows: figures
                    .directly_follows
                    .into_iter()
                    .map(|((from, to), count)| DirectlyFollows { from, to, count })
                    .collect(),
            }
        })
        .collect();

    let document = ProcessMapV1 {
        meta: ProcessMapMeta {
            format: PROCESS_MAP_FORMAT,
            revision: log.meta.revision,
            ocel_hash: hash(ocel),
        },
        names: ProcessMapNames {
            node_types: log.names.node_types,
        },
        object_types,
    };
    let bytes = encode(&document).map_err(|error| OcelMalformed(error.to_string()))?;
    let summary = ProcessMapped {
        revision: document.meta.revision,
        object_types: document.object_types.len() as u64,
        cases,
        variants: variant_count,
        directly_follows: edge_count,
        ocel_hash: document.meta.ocel_hash.clone(),
        process_map_hash: hash(&bytes),
    };
    Ok(Answer { bytes, summary })
}
