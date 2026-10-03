//! Kernel-owned bootstrap admission, without a preceding committed revision.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ekr_core::{AgentId, ContentHash, GraphRootId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{
    Assertion, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue, Edge, EvidenceSource,
    GraphSnapshot, Node, Object, Space, Subject,
};
use ekr_ontology::{Ontology, OntologyDocument, Value};
use ekr_store::{Entity, GraphDocument, MembraneError, RetainedHistory, StorageClass, StoreError};
use serde::{Deserialize, Serialize};

use crate::{AuthorityStateV1, EdgeDraft, GraphOperation, GraphTransaction, NodeDraft, Pipeline};

/// Independently authenticated execution identities, supplied by the host, never the seed input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct BootstrapContext {
    /// The operator submitting the seed.
    pub operator: AgentId,
    /// The distinct agent performing deterministic validation.
    pub validator: AgentId,
}

/// Version two seed input. A graph format change must also change this version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SeedDocument {
    /// Must be `ekr-seed/2`.
    #[cfg_attr(feature = "schema", schemars(extend("const" = "ekr-seed/2")))]
    pub format: String,
    /// Complete ontology, retained rather than reconstructed from a schema identity.
    pub ontology: OntologyDocument,
    /// Proposed graph records, before the kernel attributes acceptance.
    #[cfg_attr(
        feature = "schema",
        schemars(schema_with = "crate::schema::graph_document")
    )]
    pub graph: GraphDocument,
    /// Exact retained HumanStatement bytes, keyed by their content address.
    ///
    /// Shared, not owned: the seed input a verified read returns holds each payload as the very
    /// allocation the store handle verified and retains, so no read copies them and a runtime
    /// keeps no second copy. The encoded form is unchanged: a sequence of byte values.
    #[serde(deserialize_with = "ekr_core::decode::unique_map")]
    pub evidence_payloads: BTreeMap<ContentHash, Arc<Vec<u8>>>,
}

/// Inclusive limits an `ekr-seed/2` document is held to before it is decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeedLimits {
    /// Raw input bytes, including comments and whitespace.
    pub input_bytes: usize,
    /// Nested containers, aliases expanded in place; the root is one.
    pub depth: usize,
    /// Values, keys and containers, each alias counted as the nodes it repeats.
    pub expanded_nodes: u64,
    /// Decoded scalar and key bytes, each alias counted as the text it repeats.
    pub expanded_text_bytes: u64,
}

/// The limits of every seed document. An alias may not make a document decode to more than a
/// document of the byte cap could hold written out.
pub const SEED_LIMITS: SeedLimits = SeedLimits {
    input_bytes: 16_777_216,
    depth: 64,
    expanded_nodes: 33_554_432,
    expanded_text_bytes: 16_777_216,
};

impl SeedDocument {
    /// Reads the exact bytes of a seed document: [`SEED_LIMITS`]'s byte cap first, then UTF-8,
    /// then [`Self::from_yaml`]. A reader that stops at `input_bytes + 1` bytes loses nothing.
    ///
    /// # Errors
    /// `seed-too-large`, `seed-decode` for bytes that are not UTF-8, and [`Self::from_yaml`]'s.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SeedError> {
        within_size(bytes.len())?;
        let text = std::str::from_utf8(bytes)
            .map_err(|_| SeedError::Invalid("seed-decode: the document is not UTF-8".to_owned()))?;
        Self::from_yaml(text)
    }

    /// Reads the versioned YAML input without discarding unknown semantic fields, after holding it
    /// to [`SEED_LIMITS`] on the kernel's bounded loader.
    ///
    /// # Errors
    /// `seed-too-large`, `seed-too-deep` or `seed-alias-expansion` past a limit, then malformed
    /// input or an unsupported format.
    pub fn from_yaml(yaml: &str) -> Result<Self, SeedError> {
        within_limits(yaml)?;
        let document: Self = serde_yaml_ng::from_str(yaml).map_err(decode)?;
        document.check_version()?;
        Ok(document)
    }
}

fn decode(error: impl std::fmt::Display) -> SeedError {
    SeedError::Invalid(format!("seed-decode: {error}"))
}

fn within_size(bytes: usize) -> Result<(), SeedError> {
    let cap = SEED_LIMITS.input_bytes;
    if bytes > cap {
        return Err(SeedError::Invalid(format!(
            "seed-too-large: the document is over {cap} bytes"
        )));
    }
    Ok(())
}

/// Holds `yaml` to [`SEED_LIMITS`] before the full load: the byte cap, then every document on the
/// bounded loader, which stops at the first container past the depth, walked for its expansion.
/// A document the loader could not read is left to the decoder, which reports it as before.
fn within_limits(yaml: &str) -> Result<(), SeedError> {
    use crate::yaml::{self, Expansion, Past, Tally};

    within_size(yaml.len())?;
    let limits = Expansion {
        depth: SEED_LIMITS.depth,
        nodes: SEED_LIMITS.expanded_nodes,
        text_bytes: SEED_LIMITS.expanded_text_bytes,
    };
    let mut documents = yaml::load(yaml, limits.depth).map_err(decode)?;
    let mut tally = Tally::default();
    while let Some(document) = yaml::next(&mut documents) {
        yaml::expand(&document, limits, &mut tally).map_err(|past| match past {
            Past::Depth => SeedError::Invalid(format!(
                "seed-too-deep: the document nests more than {} containers deep",
                limits.depth
            )),
            Past::Nodes(nodes) => SeedError::Invalid(format!(
                "seed-alias-expansion: aliases expand the document past {} values and keys \
                 (at least {nodes})",
                limits.nodes
            )),
            Past::Text(bytes) => SeedError::Invalid(format!(
                "seed-alias-expansion: aliases expand the document past {} bytes of text \
                 (at least {bytes})",
                limits.text_bytes
            )),
            Past::Recursive => decode("an alias repeats the node it is inside"),
            Past::Malformed(error) => decode(error),
        })?;
        if document.check().is_err() {
            break;
        }
    }
    Ok(())
}

impl SeedDocument {
    fn check_version(&self) -> Result<(), SeedError> {
        check_version(&self.format)
    }
}

fn check_version(format: &str) -> Result<(), SeedError> {
    if format != "ekr-seed/2" {
        return invalid("unsupported-seed-format");
    }
    Ok(())
}

/// A named bootstrap refusal.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SeedError {
    /// A deterministic admission rule failed.
    #[error("{0}")]
    Invalid(String),
    /// The provider could not initialize the lineage.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// A retained seed envelope, decoded: `ekr-seed-envelope/2`, which carries the evidence payloads,
/// or `ekr-seed-envelope/3`, which names them (design § 100.1).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SeedEnvelope {
    pub(crate) format: String,
    pub(crate) input: RetainedSeedInput,
    pub(crate) context: BootstrapContext,
    pub(crate) authority: AuthorityStateV1,
    pub(crate) committed_at: Timestamp,
}

/// The seed input an envelope retains: the admitted `ekr-seed/2` document, its evidence payloads
/// carried (`/2`) or named by their content hashes (`/3`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RetainedSeedInput {
    pub(crate) format: String,
    pub(crate) ontology: OntologyDocument,
    pub(crate) graph: GraphDocument,
    pub(crate) payloads: SeedPayloads,
}

/// How a retained seed input holds its evidence payloads.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SeedPayloads {
    /// `ekr-seed-envelope/2`: every payload's bytes, keyed by their content hash.
    Carried(BTreeMap<ContentHash, Arc<Vec<u8>>>),
    /// `ekr-seed-envelope/3`: every payload's content hash; the bytes are retained objects.
    Named(BTreeSet<ContentHash>),
}

impl RetainedSeedInput {
    /// The input of a new `ekr-seed-envelope/3`: `document` less its payload bytes.
    pub(crate) fn naming(document: &SeedDocument) -> Self {
        Self {
            format: document.format.clone(),
            ontology: document.ontology.clone(),
            graph: document.graph.clone(),
            payloads: SeedPayloads::Named(document.evidence_payloads.keys().copied().collect()),
        }
    }

    /// The format, ontology and graph admission reads.
    fn parts(&self) -> (&str, &OntologyDocument, &GraphDocument) {
        (&self.format, &self.ontology, &self.graph)
    }

    /// The content hashes of the evidence payloads, in ascending order.
    pub(crate) fn payload_keys(&self) -> BTreeSet<ContentHash> {
        match &self.payloads {
            SeedPayloads::Carried(payloads) => payloads.keys().copied().collect(),
            SeedPayloads::Named(keys) => keys.clone(),
        }
    }

    /// Whether this is the retained input of `document`: the same format, ontology and graph, and
    /// the same evidence payloads — compared byte for byte where carried, and where named, the
    /// same content hashes, each of which `document`'s bytes for it must have.
    pub(crate) fn holds(&self, document: &SeedDocument) -> bool {
        self.format == document.format
            && self.ontology == document.ontology
            && self.graph == document.graph
            && match &self.payloads {
                SeedPayloads::Carried(payloads) => *payloads == document.evidence_payloads,
                SeedPayloads::Named(keys) => {
                    keys.iter().eq(document.evidence_payloads.keys())
                        && document
                            .evidence_payloads
                            .iter()
                            .all(|(hash, bytes)| ContentHash::of_bytes(bytes) == *hash)
                }
            }
    }

    /// The complete `ekr-seed/2` document this input retains, a named payload's bytes read from
    /// `history`, which holds it (see [`SeedEnvelope::payload_bytes`]).
    ///
    /// Every payload is shared, never copied: a named payload is the retained object's allocation
    /// in `history`. A carried payload is that allocation too where `history` holds a verified
    /// object with exactly the carried bytes, as every `/2` history the kernel requires does, and
    /// otherwise the envelope's own; the carried bytes decide either way, so this refuses nothing
    /// [`Self::document`] did not refuse before.
    pub(crate) fn document(&self, history: &RetainedHistory) -> Result<SeedDocument, StoreError> {
        let evidence_payloads = match &self.payloads {
            SeedPayloads::Carried(payloads) => payloads
                .iter()
                .map(|(hash, carried)| {
                    let shared = history
                        .content(*hash, StorageClass::Provenance)
                        .ok()
                        .filter(|retained| *retained == carried.as_slice())
                        .and_then(|_| history.objects.get(hash))
                        .map_or_else(|| Arc::clone(carried), |held| Arc::clone(&held.bytes));
                    (*hash, shared)
                })
                .collect(),
            SeedPayloads::Named(keys) => keys
                .iter()
                .map(|hash| {
                    payload(history, *hash)?;
                    Ok((*hash, Arc::clone(&history.objects[hash].bytes)))
                })
                .collect::<Result<_, StoreError>>()?,
        };
        Ok(SeedDocument {
            format: self.format.clone(),
            ontology: self.ontology.clone(),
            graph: self.graph.clone(),
            evidence_payloads,
        })
    }

    /// Holds each carried payload as the allocation `history` retains for it, where `history`
    /// holds a verified object with exactly the carried bytes, instead of the copy decoding the
    /// envelope made. The bytes are the same, so nothing this input answers changes; an authority
    /// that keeps the decoded envelope then keeps no second copy of the seed evidence.
    pub(crate) fn share_retained(&mut self, history: &RetainedHistory) {
        self.payloads.share_retained(history);
    }

    /// What [`Self::document`] requires of `history`, checked in the same order and refused the
    /// same way, without copying any bytes: each named payload held and verified. A carried
    /// payload is read from the envelope, so it requires nothing of `history`.
    pub(crate) fn check_held(&self, history: &RetainedHistory) -> Result<(), StoreError> {
        if let SeedPayloads::Named(keys) = &self.payloads {
            for hash in keys {
                payload(history, *hash)?;
            }
        }
        Ok(())
    }
}

impl SeedPayloads {
    /// [`RetainedSeedInput::share_retained`]: each carried payload `history` holds a verified
    /// object with exactly the same bytes for becomes that object's allocation; every other
    /// payload, and every named one, is left as it is.
    fn share_retained(&mut self, history: &RetainedHistory) {
        if let Self::Carried(payloads) = self {
            for (hash, carried) in payloads.iter_mut() {
                let retained = history
                    .content(*hash, StorageClass::Provenance)
                    .ok()
                    .filter(|retained| *retained == carried.as_slice())
                    .and_then(|_| history.objects.get(hash));
                if let Some(held) = retained {
                    *carried = Arc::clone(&held.bytes);
                }
            }
        }
    }
}

/// A named evidence payload's retained bytes, or `seed-evidence-payload-absent` naming it where
/// the store holds no object for it.
fn payload(history: &RetainedHistory, hash: ContentHash) -> Result<&[u8], StoreError> {
    if !history.objects.contains_key(&hash) {
        return Err(StoreError::InvalidSeed(format!(
            "seed-evidence-payload-absent: {hash} is an evidence payload the seed envelope names \
             and the store holds no object for"
        )));
    }
    history.content(hash, StorageClass::Provenance)
}

impl SeedEnvelope {
    /// The retained bytes of every evidence payload. A carried payload's retained object must hold
    /// exactly the bytes the envelope carries; a named payload's are the retained object's.
    ///
    /// # Errors
    /// `seed-evidence-payload-mismatch` where a carried payload's object differs,
    /// `seed-evidence-payload-absent` where a named payload has no object, and every refusal of
    /// [`RetainedHistory::content`].
    pub(crate) fn payload_bytes<'a>(
        &'a self,
        history: &'a RetainedHistory,
    ) -> Result<BTreeMap<ContentHash, &'a [u8]>, StoreError> {
        match &self.input.payloads {
            SeedPayloads::Carried(payloads) => payloads
                .iter()
                .map(|(hash, original)| {
                    if history.content(*hash, StorageClass::Provenance)? != original.as_slice() {
                        return Err(StoreError::InvalidSeed(
                            "seed-evidence-payload-mismatch".into(),
                        ));
                    }
                    Ok((*hash, original.as_slice()))
                })
                .collect(),
            SeedPayloads::Named(keys) => keys
                .iter()
                .map(|hash| Ok((*hash, payload(history, *hash)?)))
                .collect(),
        }
    }

    /// The exact retained bytes of this envelope, in its own format's layout.
    ///
    /// # Errors
    /// An envelope whose format and payload holding disagree, or an encoding failure.
    pub(crate) fn to_bytes(&self) -> Result<Vec<u8>, SeedError> {
        let encoded = match (&self.input.payloads, self.format.as_str()) {
            (SeedPayloads::Named(keys), ENVELOPE_FORMAT) => serde_json::to_vec(&EnvelopeV3 {
                format: self.format.clone(),
                input: InputV3 {
                    format: self.input.format.clone(),
                    ontology: self.input.ontology.clone(),
                    graph: self.input.graph.clone(),
                    evidence_payloads: keys.clone(),
                },
                context: self.context,
                authority: self.authority.clone(),
                committed_at: self.committed_at,
            }),
            (SeedPayloads::Carried(payloads), ENVELOPE_FORMAT_V2) => {
                serde_json::to_vec(&EnvelopeV2 {
                    format: self.format.clone(),
                    input: SeedDocument {
                        format: self.input.format.clone(),
                        ontology: self.input.ontology.clone(),
                        graph: self.input.graph.clone(),
                        evidence_payloads: payloads.clone(),
                    },
                    context: self.context,
                    authority: self.authority.clone(),
                    committed_at: self.committed_at,
                })
            }
            _ => return invalid("seed-envelope-layout"),
        };
        encoded.map_err(|error| SeedError::Invalid(error.to_string()))
    }
}

/// `ekr-seed-envelope/2` as retained: the complete seed input, payload bytes included.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvelopeV2 {
    format: String,
    input: SeedDocument,
    context: BootstrapContext,
    authority: AuthorityStateV1,
    committed_at: Timestamp,
}

/// `ekr-seed-envelope/3` as retained: the seed input with its payloads named, in the field order
/// of `/2`.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvelopeV3 {
    format: String,
    input: InputV3,
    context: BootstrapContext,
    authority: AuthorityStateV1,
    committed_at: Timestamp,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputV3 {
    format: String,
    ontology: OntologyDocument,
    graph: GraphDocument,
    #[serde(deserialize_with = "ekr_core::decode::unique_set")]
    evidence_payloads: BTreeSet<ContentHash>,
}

impl From<EnvelopeV2> for SeedEnvelope {
    fn from(envelope: EnvelopeV2) -> Self {
        Self {
            format: envelope.format,
            input: RetainedSeedInput {
                format: envelope.input.format,
                ontology: envelope.input.ontology,
                graph: envelope.input.graph,
                payloads: SeedPayloads::Carried(envelope.input.evidence_payloads),
            },
            context: envelope.context,
            authority: envelope.authority,
            committed_at: envelope.committed_at,
        }
    }
}
impl From<EnvelopeV3> for SeedEnvelope {
    fn from(envelope: EnvelopeV3) -> Self {
        Self {
            format: envelope.format,
            input: RetainedSeedInput {
                format: envelope.input.format,
                ontology: envelope.input.ontology,
                graph: envelope.input.graph,
                payloads: SeedPayloads::Named(envelope.input.evidence_payloads),
            },
            context: envelope.context,
            authority: envelope.authority,
            committed_at: envelope.committed_at,
        }
    }
}

fn invalid<T>(code: &str) -> Result<T, SeedError> {
    Err(SeedError::Invalid(code.to_owned()))
}

/// `seed-evidence-payload-missing`, naming the entry's `content_hash`, every `evidence_payloads`
/// key no evidence entry names — where a half-applied correction left the payload — and the rule
/// that the two are one value.
fn payload_missing<T>(
    evidence: &ekr_graph::Evidence,
    entries: &BTreeMap<ekr_core::EvidenceId, ekr_graph::Evidence>,
    payloads: &BTreeMap<ContentHash, Option<&[u8]>>,
) -> Result<T, SeedError> {
    let named: BTreeSet<ContentHash> = entries.values().map(|e| e.content_hash).collect();
    let unnamed: Vec<String> = payloads
        .keys()
        .filter(|key| !named.contains(key))
        .map(ToString::to_string)
        .collect();
    let unnamed = if unnamed.is_empty() {
        "none".to_owned()
    } else {
        unnamed.join(", ")
    };
    Err(SeedError::Invalid(format!(
        "seed-evidence-payload-missing: evidence {} has content_hash {}, which is not a key of \
         evidence_payloads; an evidence entry's content_hash and its evidence_payloads key must \
         be the same value (`ekr hash` of the payload); keys no evidence entry names: {unnamed}",
        evidence.id, evidence.content_hash
    )))
}

/// `seed-evidence-payload-mismatch`, naming the content hash the payload's bytes have (expected)
/// and the one the seed wrote for them (found), so the author can correct the entry.
fn payload_mismatch<T>(expected: ContentHash, found: ContentHash) -> Result<T, SeedError> {
    Err(SeedError::Invalid(format!(
        "seed-evidence-payload-mismatch: expected {expected} \
         (sha256(\"ekr.payload.v1\" || the payload's bytes)), found {found}"
    )))
}

/// A payload value checked to be what `Vec<u8>` decodes — a sequence of integers each fitting a
/// byte — without allocating the bytes.
struct CheckedBytes;
impl<'de> Deserialize<'de> for CheckedBytes {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = CheckedBytes;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a sequence")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<CheckedBytes, A::Error> {
                while sequence.next_element::<u8>()?.is_some() {}
                Ok(CheckedBytes)
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}

/// What a replay checkpoint's restore reads of a retained `ekr-seed-envelope/2`: the seed graph
/// and the addresses of the evidence payloads, every other field decoded as in [`SeedEnvelope`]
/// and each payload's bytes checked as `Vec<u8>` would decode them, but not kept.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SeedEnvelopeView {
    format: String,
    input: SeedDocumentView,
    #[serde(rename = "context")]
    _context: BootstrapContext,
    #[serde(rename = "authority")]
    _authority: AuthorityStateV1,
    #[serde(rename = "committed_at")]
    _committed_at: Timestamp,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SeedDocumentView {
    #[serde(rename = "format")]
    _format: String,
    #[serde(rename = "ontology")]
    _ontology: OntologyDocument,
    graph: GraphDocument,
    #[serde(deserialize_with = "ekr_core::decode::unique_map")]
    evidence_payloads: BTreeMap<ContentHash, CheckedBytes>,
}

/// The parts of a retained seed envelope a checkpoint restore reads: the envelope this authority
/// already decoded in full for the same seed, or else a [`SeedEnvelopeView`] of its bytes.
pub(crate) enum SeedOutline {
    /// The complete envelope, as the authority holds it.
    Full(std::sync::Arc<SeedEnvelope>),
    /// The envelope without its payload bytes.
    View(Box<SeedEnvelopeView>),
}
impl SeedOutline {
    /// The seed's graph document.
    pub(crate) fn graph(&self) -> &GraphDocument {
        match self {
            Self::Full(envelope) => &envelope.input.graph,
            Self::View(view) => &view.input.graph,
        }
    }
    /// Whether the envelope names its evidence payloads (`ekr-seed-envelope/3`).
    pub(crate) fn names_payloads(&self) -> bool {
        matches!(self, Self::Full(envelope) if matches!(envelope.input.payloads, SeedPayloads::Named(_)))
    }
    /// Whether `keys` are exactly the addresses of the seed's evidence payloads.
    pub(crate) fn payloads_are(&self, keys: &BTreeSet<ContentHash>) -> bool {
        match self {
            Self::Full(envelope) => *keys == envelope.input.payload_keys(),
            Self::View(view) => keys.iter().eq(view.input.evidence_payloads.keys()),
        }
    }
}

/// The envelope format every new seed retains (design § 100.1).
pub(crate) const ENVELOPE_FORMAT: &str = "ekr-seed-envelope/3";
/// The envelope format seeds before it retained, still replayed exactly.
pub(crate) const ENVELOPE_FORMAT_V2: &str = "ekr-seed-envelope/2";

/// Decodes a retained seed envelope, typed first. Only bytes the typed decode refuses are read
/// again as a JSON value, to tell a legacy envelope (`SeedMigrationRequired`) and bytes that are
/// not JSON at all apart from any other malformed envelope, as a value-first decode did.
fn decoded<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, StoreError> {
    serde_json::from_slice(bytes).or_else(|typed| {
        let shape: serde_json::Value = serde_json::from_slice(bytes)
            .map_err(|error| StoreError::InvalidSeed(format!("seed-decode: {error}")))?;
        if shape.get("format").is_none() && shape.get("root").is_some() {
            return Err(StoreError::SeedMigrationRequired);
        }
        Err(StoreError::InvalidSeed(format!("seed-decode: {typed}")))
    })
}

fn supported(format: &str, expected: &str) -> Result<(), StoreError> {
    if format != expected {
        return Err(StoreError::InvalidSeed(
            "unsupported-seed-envelope".to_owned(),
        ));
    }
    Ok(())
}

/// Decodes bytes laid out as `ekr-seed-envelope/3`, if they are, and whatever their format says.
fn as_v3(bytes: &[u8]) -> Option<EnvelopeV3> {
    serde_json::from_slice(bytes).ok()
}

/// Decodes a complete retained seed envelope by its exact format: `/3`'s layout for
/// `ekr-seed-envelope/3`, and `/2`'s, payload bytes included, for `ekr-seed-envelope/2`. A layout
/// that disagrees with its format is `unsupported-seed-envelope`.
pub(crate) fn envelope(bytes: &[u8]) -> Result<SeedEnvelope, StoreError> {
    if let Some(envelope) = as_v3(bytes) {
        supported(&envelope.format, ENVELOPE_FORMAT)?;
        return Ok(envelope.into());
    }
    let envelope: EnvelopeV2 = decoded(bytes)?;
    supported(&envelope.format, ENVELOPE_FORMAT_V2)?;
    Ok(envelope.into())
}

/// What a checkpoint restore reads of a retained envelope: an `ekr-seed-envelope/3` in full,
/// since it carries no payload bytes, and a view of an `ekr-seed-envelope/2` without its payload
/// bytes, refusing what [`envelope`] refuses, payload values that are not bytes included.
pub(crate) fn outline(bytes: &[u8]) -> Result<SeedOutline, StoreError> {
    if let Some(envelope) = as_v3(bytes) {
        supported(&envelope.format, ENVELOPE_FORMAT)?;
        return Ok(SeedOutline::Full(std::sync::Arc::new(envelope.into())));
    }
    let view: SeedEnvelopeView = decoded(bytes)?;
    supported(&view.format, ENVELOPE_FORMAT_V2)?;
    Ok(SeedOutline::View(Box::new(view)))
}

/// Admits the retained seed `envelope` names, with `payloads` its evidence payloads' retained
/// bytes ([`SeedEnvelope::payload_bytes`]).
pub(crate) fn replay(
    envelope: &SeedEnvelope,
    payloads: &BTreeMap<ContentHash, &[u8]>,
    ontology: Option<&Ontology>,
    context: BootstrapContext,
    authority: &AuthorityStateV1,
) -> Result<CanonicalGraph, StoreError> {
    authority.check(context)?;
    if envelope.context != context || envelope.authority != *authority {
        return Err(StoreError::AuthorityMismatch);
    }
    let payloads = payloads
        .iter()
        .map(|(hash, bytes)| (*hash, Some(*bytes)))
        .collect();
    let graph = admitted(
        envelope.input.parts(),
        &payloads,
        context,
        envelope.committed_at,
    )
    .map_err(|error| StoreError::InvalidSeed(error.to_string()))?;
    if ontology.is_some_and(|expected| graph.ontology != *expected) {
        return Err(StoreError::InvalidSeed("seed-ontology-mismatch".to_owned()));
    }
    Ok(graph)
}

/// The kernel's admission of `envelope`'s retained seed before its named payloads are loaded:
/// every rule, with each carried payload's bytes checked against its address and each named one's
/// left to [`replay`], which reads them.
pub(crate) fn admitted_retained(envelope: &SeedEnvelope) -> Result<CanonicalGraph, SeedError> {
    let payloads = match &envelope.input.payloads {
        SeedPayloads::Carried(payloads) => payloads
            .iter()
            .map(|(hash, bytes)| (*hash, Some(bytes.as_slice())))
            .collect(),
        SeedPayloads::Named(keys) => keys.iter().map(|hash| (*hash, None)).collect(),
    };
    admitted(
        envelope.input.parts(),
        &payloads,
        envelope.context,
        envelope.committed_at,
    )
}

pub(crate) fn admitted_graph(
    input: &SeedDocument,
    context: BootstrapContext,
    committed_at: Timestamp,
) -> Result<CanonicalGraph, SeedError> {
    let payloads = input
        .evidence_payloads
        .iter()
        .map(|(hash, bytes)| (*hash, Some(bytes.as_slice())))
        .collect();
    admitted(
        (&input.format, &input.ontology, &input.graph),
        &payloads,
        context,
        committed_at,
    )
}

/// Seed admission of `input` with `payloads` its evidence payloads: each one's bytes where they
/// are to be checked against its address here, `None` where the store holds and checks them.
fn admitted(
    (format, ontology, graph): (&str, &OntologyDocument, &GraphDocument),
    payloads: &BTreeMap<ContentHash, Option<&[u8]>>,
    context: BootstrapContext,
    committed_at: Timestamp,
) -> Result<CanonicalGraph, SeedError> {
    check_version(format)?;
    let ontology = Ontology::load(ontology.clone())
        .map_err(|error| SeedError::Invalid(format!("seed-ontology: {error}")))?;
    if ontology.version().number != 0 || ontology.version().parent.is_some() {
        return invalid("seed-ontology-lineage");
    }
    let document = graph;
    if document.root.space != Space::Canonical {
        return invalid("seed-space");
    }
    if document.root.schema_version_id != ontology.version().id {
        return invalid("seed-schema-version");
    }
    if document.root.parent.is_some() || document.revision != RevisionNumber::SEED {
        return invalid("seed-root-lineage");
    }
    // An attachment is made by a commit, at a revision after the seed (design § 103.3).
    if !document.attachments.is_empty() {
        return invalid("seed-attachments");
    }
    if context.operator == context.validator {
        return invalid("proposer-is-validator");
    }
    for (key, node) in &document.nodes {
        if node.properties.values().any(Vec::is_empty) {
            return invalid("seed-empty-property-values");
        }
        filing(
            Entity::Node(*key),
            Entity::Node(node.id),
            node.root_id,
            document.root.id,
        )?;
        if let Some(declared) = ontology.node_type(node.type_id) {
            match (&declared.lifecycle, &node.type_state) {
                (Some(lifecycle), Some(state)) if state == &lifecycle.initial => {}
                (None, None) => {}
                _ => return invalid("seed-initial-lifecycle"),
            }
        }
    }
    for (key, edge) in &document.edges {
        if edge.properties.values().any(Vec::is_empty) {
            return invalid("seed-empty-property-values");
        }
        filing(
            Entity::Edge(*key),
            Entity::Edge(edge.id),
            edge.root_id,
            document.root.id,
        )?;
    }
    for (key, assertion) in &document.assertions {
        filing(
            Entity::Assertion(*key),
            Entity::Assertion(assertion.id),
            assertion.root_id,
            document.root.id,
        )?;
        if assertion.proposed_by != context.operator {
            return invalid("seed-attribution-mismatch");
        }
    }
    for (key, evidence) in &document.evidence {
        if *key != evidence.id {
            return invalid("seed-misfiled-evidence");
        }
        if !matches!(evidence.source, EvidenceSource::HumanStatement { .. }) {
            return invalid("seed-unsupported-source");
        }
        if evidence.extracted_by != context.operator {
            return invalid("seed-attribution-mismatch");
        }
        let Some(payload) = payloads.get(&evidence.content_hash) else {
            return payload_missing(evidence, &document.evidence, payloads);
        };
        if let Some(payload) = payload {
            let expected = ContentHash::of_bytes(payload);
            if expected != evidence.content_hash {
                return payload_mismatch(expected, evidence.content_hash);
            }
        }
    }
    // An extra map entry is still retained input: verify every content address, cited or not.
    // A named payload's bytes are the store's to hold, and are checked where they are read.
    for (hash, payload) in payloads {
        let Some(payload) = payload else { continue };
        let expected = ContentHash::of_bytes(payload);
        if expected != *hash {
            return payload_mismatch(expected, *hash);
        }
    }

    // Explicit bootstrap validation context. This empty index is not a previous committed
    // revision: it supplies the seed's ontology and independently verified evidence to the
    // invariant validators. No ValidatedTransaction is sealed and no against-revision exists.
    let mut initial = CanonicalGraph {
        attachments: Default::default(),
        root: document.root,
        revision: RevisionNumber::SEED,
        ontology: ontology.clone(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };
    initial.evidence.clone_from(&document.evidence);
    let operations = document
        .nodes
        .values()
        .map(|node| {
            GraphOperation::CreateNode(NodeDraft {
                id: node.id,
                root_id: node.root_id,
                type_id: node.type_id,
                canonical_name: node.canonical_name.clone(),
                properties: node.properties.clone(),
                // Not the node's aliases: a seed may give two nodes one alias, which a
                // transaction may not (`alias-already-exists`, `duplicate-alias`). The admitted
                // node keeps its aliases through `narrow_node`.
                aliases: Vec::new(),
            })
        })
        .chain(document.edges.values().map(|edge| {
            GraphOperation::CreateEdge(EdgeDraft {
                id: edge.id,
                root_id: edge.root_id,
                type_id: edge.type_id,
                source: edge.source,
                target: edge.target,
                properties: edge.properties.clone(),
            })
        }))
        .chain(
            document
                .assertions
                .values()
                .cloned()
                .map(|a| GraphOperation::AddAssertion(Box::new(a))),
        )
        .collect();
    let proposal = GraphTransaction {
        // Diagnostic correlation only; this request is never persisted as a transaction.
        id: TransactionId::from_uuid(document.root.id.to_uuid()),
        proposer: context.operator,
        operations,
        evidence: document
            .assertions
            .values()
            .flat_map(|a| a.evidence.iter().copied())
            .collect(),
        schema_version: None,
    };
    Pipeline::deterministic(context.validator)
        .validate_bootstrap(&GraphSnapshot::of(&initial), &proposal)
        .map_err(|issues| {
            SeedError::Invalid(
                issues
                    .iter()
                    .map(|i| format!("{}: {}", i.code, i.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;

    let narrow_error = |error: MembraneError| SeedError::Invalid(error.to_string());
    let nodes = document
        .nodes
        .iter()
        .map(|(id, node)| Ok((*id, narrow_node(node.clone())?)))
        .collect::<Result<_, MembraneError>>()
        .map_err(narrow_error)?;
    let edges = document
        .edges
        .iter()
        .map(|(id, edge)| Ok((*id, narrow_edge(edge.clone())?)))
        .collect::<Result<_, MembraneError>>()
        .map_err(narrow_error)?;
    let assertions = document
        .assertions
        .iter()
        .map(|(id, assertion)| {
            let mut admitted = narrow_assertion(assertion.clone())?;
            admitted.assessment = Assessment::Accepted {
                validators: BTreeSet::from([context.validator]),
            };
            admitted.transaction_time = ekr_graph::TransactionTime::since(committed_at);
            Ok((*id, admitted))
        })
        .collect::<Result<_, MembraneError>>()
        .map_err(narrow_error)?;
    Ok(CanonicalGraph {
        attachments: Default::default(),
        root: document.root,
        revision: RevisionNumber::SEED,
        ontology,
        nodes,
        edges,
        assertions,
        evidence: document.evidence.clone(),
    })
}

fn filing(
    key: Entity,
    found: Entity,
    root: GraphRootId,
    expected: GraphRootId,
) -> Result<(), SeedError> {
    if key != found {
        return invalid("seed-misfiled-entity");
    }
    if root != expected {
        return invalid("seed-misrooted-entity");
    }
    Ok(())
}

/// A candidate's node, as canonical state — or the refusal that says why it is not.
pub(crate) fn narrow_node(node: Node<Value>) -> Result<Node<CanonicalValue>, MembraneError> {
    let node_id = node.id;
    let mut properties = BTreeMap::new();
    for (property, value) in node.properties {
        let value = value
            .into_iter()
            .map(|value| {
                CanonicalValue::try_from(value).map_err(|cause| MembraneError::Node {
                    node_id,
                    property,
                    cause,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        properties.insert(property, value);
    }
    Ok(Node {
        id: node.id,
        root_id: node.root_id,
        type_id: node.type_id,
        canonical_name: node.canonical_name,
        aliases: node.aliases,
        type_state: node.type_state,
        properties,
    })
}

/// A candidate's edge, as canonical state — or the refusal that says why it is not.
///
/// References are narrowed only after deterministic bootstrap validation succeeds.
pub(crate) fn narrow_edge(edge: Edge<Value>) -> Result<Edge<CanonicalValue>, MembraneError> {
    let edge_id = edge.id;
    let mut properties = BTreeMap::new();
    for (property, value) in edge.properties {
        let value = value
            .into_iter()
            .map(|value| {
                CanonicalValue::try_from(value).map_err(|cause| MembraneError::Edge {
                    edge_id,
                    property,
                    cause,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        properties.insert(property, value);
    }
    Ok(Edge {
        id: edge.id,
        root_id: edge.root_id,
        type_id: edge.type_id,
        source: CanonicalRef::new(edge.source),
        target: CanonicalRef::new(edge.target),
        properties,
    })
}

/// A candidate's assertion, as canonical state — or the refusal that says why it is not.
pub(crate) fn narrow_assertion(
    assertion: Assertion<Value>,
) -> Result<Assertion<CanonicalValue>, MembraneError> {
    let assertion_id = assertion.id;
    let object = match assertion.object {
        Object::Value(value) => {
            Object::Value(CanonicalValue::try_from(value).map_err(|cause| {
                MembraneError::Assertion {
                    assertion_id,
                    cause,
                }
            })?)
        }
        Object::Node(node) => Object::Node(CanonicalRef::new(node)),
        Object::Type(type_id) => Object::Type(type_id),
    };
    Ok(Assertion {
        id: assertion.id,
        root_id: assertion.root_id,
        subject: match assertion.subject {
            Subject::Node(node) => Subject::Node(CanonicalRef::new(node)),
            Subject::Edge(edge) => Subject::Edge(CanonicalRef::new(edge)),
            Subject::Type(type_id) => Subject::Type(type_id),
        },
        predicate: assertion.predicate,
        object,
        evidence: assertion
            .evidence
            .into_iter()
            .map(CanonicalRef::new)
            .collect(),
        proposed_by: assertion.proposed_by,
        assessment: assertion.assessment.map_assertions(CanonicalRef::new),
        lifecycle: assertion.lifecycle.map_assertions(CanonicalRef::new),
        valid_time: assertion.valid_time,
        transaction_time: assertion.transaction_time,
    })
}

#[cfg(test)]
mod shared_payloads {
    use super::{SeedPayloads, StorageClass};
    use ekr_core::{ContentHash, Timestamp};
    use ekr_store::{RetainedHistory, RetainedObject, StoredObject};
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;

    fn retained(bytes: Vec<u8>, stored_as: ContentHash) -> RetainedObject {
        RetainedObject {
            metadata: StoredObject {
                content_hash: stored_as,
                storage_class: StorageClass::Provenance,
                byte_len: bytes.len() as u64,
                stored_at: Timestamp::EPOCH,
            },
            bytes: Arc::new(bytes),
        }
    }

    /// A carried payload becomes the retained allocation only where the history holds a verified
    /// object with exactly its bytes; an absent, damaged or different object leaves the carried
    /// copy as it was, and a named payload holds no bytes to share.
    #[test]
    fn a_carried_payload_becomes_the_retained_allocation_only_for_verified_equal_bytes() {
        let (equal, absent, damaged) = (
            b"carried and retained".to_vec(),
            b"carried only".to_vec(),
            b"carried, retained damaged".to_vec(),
        );
        let hash = |bytes: &Vec<u8>| ContentHash::of_bytes(bytes);
        let mut broken = damaged.clone();
        broken[3] ^= 1;
        let history = RetainedHistory {
            applications: Default::default(),
            occurrences: Vec::new(),
            objects: BTreeMap::from([
                (hash(&equal), retained(equal.clone(), hash(&equal))),
                (hash(&damaged), retained(broken, hash(&damaged))),
            ]),
        };
        let carried: BTreeMap<ContentHash, Arc<Vec<u8>>> = [&equal, &absent, &damaged]
            .into_iter()
            .map(|bytes| (hash(bytes), Arc::new(bytes.clone())))
            .collect();
        let mut payloads = SeedPayloads::Carried(carried.clone());
        payloads.share_retained(&history);
        let SeedPayloads::Carried(shared) = &payloads else {
            panic!("carried payloads stay carried");
        };
        assert_eq!(*shared, carried, "sharing changes no byte");
        assert!(Arc::ptr_eq(
            &shared[&hash(&equal)],
            &history.objects[&hash(&equal)].bytes
        ));
        for other in [&absent, &damaged] {
            assert!(
                Arc::ptr_eq(&shared[&hash(other)], &carried[&hash(other)]),
                "a payload without a verified equal object keeps the carried copy"
            );
        }

        let names = BTreeSet::from([hash(&equal)]);
        let mut named = SeedPayloads::Named(names.clone());
        named.share_retained(&history);
        assert_eq!(named, SeedPayloads::Named(names));
    }
}

#[cfg(test)]
mod bounded_load {
    use super::{SeedDocument, SeedError, SEED_LIMITS};
    use crate::yaml::loaded;

    /// A seed nested past the depth is refused before the full load: the bounded loader reads up
    /// to the first container past the limit and the parser never sees the rest, so the events
    /// buffered are counted by the limit, not by the input.
    #[test]
    fn a_seed_nested_past_the_depth_is_refused_at_the_first_container_past_it() {
        let tail = 20_000;
        let open = SEED_LIMITS.depth + tail;
        let text = format!(
            "format: ekr-seed/2\nontology: {}{}\n",
            "[".repeat(open),
            "]".repeat(open)
        );
        loaded::take();
        let refused = SeedDocument::from_yaml(&text).unwrap_err();
        let events = loaded::take();
        assert!(
            matches!(&refused, SeedError::Invalid(reason) if reason.starts_with("seed-too-deep")),
            "{refused}"
        );
        // The root map, two keys and a value, then containers up to and including the first one
        // past the limit.
        assert!(
            events > SEED_LIMITS.depth,
            "the bounded loader ran: {events} events"
        );
        assert!(
            events <= SEED_LIMITS.depth + 4,
            "the loader read {events} events of a document with {} containers",
            open + 1
        );
    }
}
