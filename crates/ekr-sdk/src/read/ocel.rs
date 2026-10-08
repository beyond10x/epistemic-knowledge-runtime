//! The OCEL document and the engine counts accompanying it on request stderr.
use serde::{Deserialize, Serialize};

use super::{OneShotReader, ReadError, Reader};
use crate::reply::Answer;
use crate::transport::{Request, Transport};

/// OCEL event selection; empty lists use the default rule. The two lists are mutually exclusive.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OcelQuery {
    /// Requested revision, or the head.
    pub revision: Option<u64>,
    /// Event type names using the default time rule.
    pub events: Vec<String>,
    /// TypeName.propertyName selectors for Timestamp values, split at the last dot.
    pub event_time: Vec<String>,
}

/// A successful OCEL request, its stdout document and its stderr counts kept separately.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OcelExport {
    /// The `ekr.ocel/1` document.
    pub document: OcelDocument,
    /// The engine's `ekr.views.OcelExported` summary.
    pub counts: OcelCounts,
}

/// The engine's counts and document hash, carried as a compact JSON line on stderr.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcelCounts {
    /// Revision exported.
    pub revision: u64,
    /// Event types declared in the log.
    pub event_types: u64,
    /// Object types declared in the log.
    pub object_types: u64,
    /// Events exported.
    pub events: u64,
    /// Objects exported.
    pub objects: u64,
    /// Event-to-object relationships.
    pub event_object_relationships: u64,
    /// Object-to-object relationships.
    pub object_object_relationships: u64,
    /// Omitted edges between events.
    pub edges_between_events: u64,
    /// Omitted event-type nodes without a writable timestamp.
    pub undated_events: u64,
    /// Omitted edges involving those nodes.
    pub edges_of_undated_events: u64,
    /// Parallel edges represented by one relationship.
    pub parallel_edges_merged: u64,
    /// Attributes omitted for unrepresentable values.
    pub attribute_values_out_of_range: u64,
    /// SHA-256 of the engine document bytes, before CLI pretty printing.
    pub ocel_hash: String,
}

/// The `ekr.ocel/1` wrapper.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcelDocument {
    /// Format and revision.
    pub meta: OcelMeta,
    /// Display names of stable identifiers.
    pub names: OcelNames,
    /// The OCEL 2.0 log.
    pub ocel: OcelLog,
}

/// Format and revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcelMeta {
    /// `ekr.ocel/1`.
    pub format: String,
    /// Exported revision.
    pub revision: u64,
}

/// Display names for OCEL's stable identifiers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcelNames {
    /// Node types.
    pub node_types: Vec<OcelName>,
    /// Relationship qualifiers.
    pub edge_types: Vec<OcelName>,
    /// Attributes.
    pub properties: Vec<OcelName>,
}

/// An identifier and its display name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcelName {
    /// Stable identifier.
    pub id: String,
    /// Display name.
    pub name: String,
}

/// OCEL 2.0's log document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcelLog {
    /// Event type declarations.
    pub event_types: Vec<OcelType>,
    /// Object type declarations.
    pub object_types: Vec<OcelType>,
    /// Events, in time and identifier order.
    pub events: Vec<OcelEvent>,
    /// Objects, in identifier order.
    pub objects: Vec<OcelObject>,
}

/// A declared OCEL type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcelType {
    /// Stable type identifier.
    pub name: String,
    /// Its declared attributes.
    pub attributes: Vec<OcelTypeAttribute>,
}

/// An attribute declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcelTypeAttribute {
    /// Stable property identifier.
    pub name: String,
    /// OCEL attribute type.
    #[serde(rename = "type")]
    pub value_type: String,
}

/// An event's attribute value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcelEventAttribute {
    /// Property identifier.
    pub name: String,
    /// Encoded value.
    pub value: String,
}

/// An object's time-indexed attribute value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcelObjectAttribute {
    /// Property identifier.
    pub name: String,
    /// Encoded value.
    pub value: String,
    /// Value's initial time.
    pub time: String,
}

/// A qualified relationship to an object.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcelRelationship {
    /// Object identifier.
    #[serde(rename = "objectId")]
    pub object_id: String,
    /// Edge type identifier.
    pub qualifier: String,
}

/// A timestamped event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcelEvent {
    /// Node identifier.
    pub id: String,
    /// Node type identifier.
    #[serde(rename = "type")]
    pub event_type: String,
    /// RFC 3339 timestamp.
    pub time: String,
    /// Event attributes.
    pub attributes: Vec<OcelEventAttribute>,
    /// Related objects.
    pub relationships: Vec<OcelRelationship>,
}

/// An object participating in events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcelObject {
    /// Node identifier.
    pub id: String,
    /// Node type identifier.
    #[serde(rename = "type")]
    pub object_type: String,
    /// Object attributes.
    pub attributes: Vec<OcelObjectAttribute>,
    /// Related objects.
    pub relationships: Vec<OcelRelationship>,
}

impl<T: Transport> Reader<T> {
    /// Export OCEL with the engine's counts, without recounting the document.
    ///
    /// # Errors
    /// [`ReadError`], including invalid selectors or malformed stdout/stderr documents.
    pub fn ocel(&mut self, query: &OcelQuery) -> Result<OcelExport, ReadError> {
        let mut argv = vec!["ocel".to_owned()];
        if let Some(revision) = query.revision {
            argv.extend(["--revision".to_owned(), revision.to_string()]);
        }
        for name in &query.events {
            argv.push(format!("--events={name}"));
        }
        for selector in &query.event_time {
            argv.push(format!("--event-time={selector}"));
        }
        let reply = self.transport.request(&Request::new(argv))?;
        let verb = "ocel".to_owned();
        let malformed = |source| ReadError::Document {
            verb: verb.clone(),
            source,
        };
        match reply.answer() {
            Answer::Outcome(outcome) => Ok(OcelExport {
                document: serde_json::from_value(outcome.document().clone()).map_err(malformed)?,
                counts: serde_json::from_str(&reply.stderr).map_err(malformed)?,
            }),
            Answer::Refusal(refusal) => Err(ReadError::Refused { verb, refusal }),
            Answer::Usage(message) => Err(ReadError::Usage { verb, message }),
            Answer::Fault(fault) => Err(ReadError::Fault { verb, fault }),
        }
    }
}

impl OneShotReader {
    /// [`Reader::ocel`], through a one-shot process.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn ocel(&mut self, query: &OcelQuery) -> Result<OcelExport, ReadError> {
        self.reader.ocel(query)
    }
}
