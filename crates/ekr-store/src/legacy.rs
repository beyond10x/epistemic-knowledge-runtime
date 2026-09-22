//! Supplied-byte verification of the original graph format.
//!
//! Source: 73ab8b0a5aa5c670bacbc4c1abdca02f87b62bf0. This module opens no
//! provider and neither establishes history completeness nor grants commit authority.
//! Retain original bytes: serializing decoded data is not a substitute for their address.

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{AssertionId, ContentHash, EdgeId, EvidenceId, NodeId, RevisionNumber};
use ekr_graph::legacy::{unique_map, Assertion, Edge, Evidence, GraphRoot, Node};
use serde::de::{DeserializeOwned, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Conservative refusal of supplied historical evidence.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// A JSON object repeats a key, including inside a user Record.
    #[error("duplicate-key: {0}")]
    DuplicateKey(String),
    /// A collection repeats a semantic identity.
    #[error("duplicate-member: {0}")]
    DuplicateMember(String),
    /// A discriminator is not one of the frozen formats.
    #[error("unsupported-format: {0}")]
    UnsupportedFormat(String),
    /// Malformed data, unknown fields, or an unsupported historical shape.
    #[error("invalid-original-data: {0}")]
    InvalidData(String),
    /// Original bytes do not match their claimed payload address.
    #[error("payload-address-mismatch: expected {expected}, observed {observed}")]
    PayloadAddressMismatch {
        /// Claimed address.
        expected: ContentHash,
        /// Address of the exact bytes, without reserialization.
        observed: ContentHash,
    },
    /// The original encoding does not match a supplied value address.
    #[error("value-address-mismatch: expected {expected}, observed {observed}")]
    ValueAddressMismatch {
        /// Claimed canonical value address.
        expected: ContentHash,
        /// Address calculated with the frozen encoder.
        observed: ContentHash,
    },
    /// Required bytes were not supplied.
    #[error("missing-evidence: seed payload {address} for evidence {evidence}")]
    MissingEvidence {
        /// Evidence record whose payload is missing.
        evidence: EvidenceId,
        /// Payload address claimed by that record.
        address: ContentHash,
    },
}

fn decode_error(error: serde_json::Error) -> Refusal {
    let text = error.to_string();
    if text.contains("duplicate-key") {
        Refusal::DuplicateKey(text)
    } else if text.contains("duplicate-member") {
        Refusal::DuplicateMember(text)
    } else {
        Refusal::InvalidData(text)
    }
}

struct UniqueJson(serde_json::Value);
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("JSON without duplicate object keys")
            }
            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueJson(value.into()))
            }
            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueJson(value.into()))
            }
            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueJson(value.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(|n| UniqueJson(n.into()))
                    .ok_or_else(|| E::custom("non-finite-number"))
            }
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueJson(value.into()))
            }
            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueJson(value.into()))
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(serde_json::Value::Null))
            }
            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(serde_json::Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut result = Vec::new();
                while let Some(value) = seq.next_element::<UniqueJson>()? {
                    result.push(value.0);
                }
                Ok(UniqueJson(result.into()))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut result = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, UniqueJson>()? {
                    if result.insert(key.clone(), value.0).is_some() {
                        return Err(serde::de::Error::custom(format!("duplicate-key: {key}")));
                    }
                }
                Ok(UniqueJson(result.into()))
            }
        }
        deserializer.deserialize_any(JsonVisitor)
    }
}

/// Reads JSON without losing duplicate keys before typed interpretation.
/// User Record keys such as `event`, `format`, and `operation` remain ordinary data.
/// This generic syntax helper alone makes no claim about a format or address.
///
/// # Errors
/// Duplicates, malformed JSON, or refusal by the requested data type.
pub fn decode_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Refusal> {
    let json: UniqueJson = serde_json::from_slice(bytes).map_err(decode_error)?;
    serde_json::from_value(json.0).map_err(decode_error)
}

/// Checks the payload-domain address of exact supplied bytes.
///
/// # Errors
/// A supplied address does not match these bytes.
pub fn verify_payload(bytes: &[u8], expected: ContentHash) -> Result<(), Refusal> {
    let observed = ContentHash::of_bytes(bytes);
    if observed != expected {
        return Err(Refusal::PayloadAddressMismatch { expected, observed });
    }
    Ok(())
}

/// Checks a supplied value-domain address using the provided historical encoder.
///
/// # Errors
/// The address and encoded value disagree.
pub fn verify_value<T: Canonical>(value: &T, expected: ContentHash) -> Result<(), Refusal> {
    let observed = ContentHash::of(value);
    if observed != expected {
        return Err(Refusal::ValueAddressMismatch { expected, observed });
    }
    Ok(())
}

/// Original unversioned document, with scalar properties and combined assertion states.
/// Its references are identities and confer no canonical capability.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphDocument {
    /// Original graph root metadata.
    pub root: GraphRoot,
    /// Original revision coordinate.
    pub revision: RevisionNumber,
    /// Node records by identity.
    #[serde(deserialize_with = "unique_map")]
    pub nodes: BTreeMap<NodeId, Node>,
    /// Edge records by identity.
    #[serde(deserialize_with = "unique_map")]
    pub edges: BTreeMap<EdgeId, Edge>,
    /// Assertions by identity.
    #[serde(deserialize_with = "unique_map")]
    pub assertions: BTreeMap<AssertionId, Assertion>,
    /// Evidence metadata; payloads must be supplied independently.
    #[serde(deserialize_with = "unique_map")]
    pub evidence: BTreeMap<EvidenceId, Evidence>,
}
impl GraphDocument {
    /// Checks the original shape and payload address without admitting or publishing state.
    ///
    /// # Errors
    /// Address mismatch, malformed/unknown fields, duplicates, or misfiled identities.
    pub fn verify_bytes(bytes: &[u8], address: ContentHash) -> Result<Self, Refusal> {
        verify_payload(bytes, address)?;
        let document: Self = decode_json(bytes)?;
        document.check_identities()?;
        Ok(document)
    }
    /// Checks map keys against identities contained in their records.
    ///
    /// # Errors
    /// Any record filed under a different identity.
    pub fn check_identities(&self) -> Result<(), Refusal> {
        if self.nodes.iter().any(|(id, r)| *id != r.id)
            || self.edges.iter().any(|(id, r)| *id != r.id)
            || self.assertions.iter().any(|(id, r)| *id != r.id)
            || self.evidence.iter().any(|(id, r)| *id != r.id)
        {
            return Err(Refusal::InvalidData("misfiled-identity".to_owned()));
        }
        Ok(())
    }
    /// Original knowledge bytes: nodes, edges, assertions, in that order.
    #[must_use]
    pub fn knowledge_bytes(&self) -> Vec<u8> {
        let mut out = Encoder::new();
        self.nodes.encode(&mut out);
        self.edges.encode(&mut out);
        self.assertions.encode(&mut out);
        out.finish()
    }
    /// Original knowledge address, without claiming these records were admitted.
    #[must_use]
    pub fn knowledge_address(&self) -> ContentHash {
        struct Knowledge<'a>(&'a GraphDocument);
        impl Canonical for Knowledge<'_> {
            fn encode(&self, out: &mut Encoder) {
                self.0.nodes.encode(out);
                self.0.edges.encode(out);
                self.0.assertions.encode(out);
            }
        }
        ContentHash::of(&Knowledge(self))
    }
    /// Original evidence metadata address; this excludes payload bytes.
    #[must_use]
    pub fn evidence_address(&self) -> ContentHash {
        ContentHash::of(&self.evidence)
    }
}
