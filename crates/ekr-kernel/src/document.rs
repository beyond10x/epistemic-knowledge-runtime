//! Bounded, exact-byte `ekr.transaction-document/1` and `ekr.transaction-document/2` ingress.
//!
//! The version selects an immutable parser profile, independently of a host's
//! upload cap. The YAML loader buffers events under the raw input cap. A bounded
//! representation pass then checks expanded syntax, including alias occurrences,
//! before strict typed decoding reads the original bytes. The typed traversal
//! adds actual enum boundaries; ignored tag metadata adds no semantic depth.
//! Neither pass claims a
//! pre-loader scalar/event quota or an exact allocator-byte bound.
//!
//! The two versions share one envelope and one grammar and differ only in their
//! frozen limits (design § 91.3 for `/1`, § 97 for `/2`). The profile is chosen
//! from the envelope's own top-level `format:` line before any parsing; a
//! document that names no version there is held to the `/1` byte cap, and every
//! document is finally held to the profile of the version it declares.
//!
//! Parsing creates no proposal, receipt, store object or canonical value. In
//! particular, nonfinite Float proposals remain available to kernel validation.

mod checked;
mod shape;

use std::{cell::RefCell, io::Read};

use ekr_core::ContentHash;
use serde::{de, Deserialize};

use crate::GraphTransaction;

/// Inclusive frozen limits of one transaction-document format version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocumentLimits {
    /// Raw input bytes, including comments, markers and whitespace.
    pub input_bytes: usize,
    /// Nested containers, including explicit enum payload boundaries; root is one.
    pub depth: usize,
    /// Expanded containers, enum boundaries, scalar values and map keys.
    pub nodes: usize,
    /// Entries in each mapping.
    pub mapping_entries: usize,
    /// Elements in each sequence.
    pub sequence_elements: usize,
    /// Bytes in one decoded string.
    pub string_bytes: usize,
    /// Bytes in one decoded mapping key.
    pub key_bytes: usize,
    /// Cumulative expanded decoded string bytes, including keys and tags.
    pub total_string_bytes: usize,
    /// Operations in a transaction; at least one is required.
    pub operations: usize,
    /// Input evidence entries, before set normalization.
    pub evidence: usize,
}

/// The historical profile selected by `ekr.transaction-document/1`.
/// Changing it requires a version/migration decision; callers cannot override it.
pub const DOCUMENT_V1_LIMITS: DocumentLimits = DocumentLimits {
    input_bytes: 262_144,
    depth: 32,
    nodes: 32_768,
    mapping_entries: 4_096,
    sequence_elements: 4_096,
    string_bytes: 65_536,
    key_bytes: 4_096,
    total_string_bytes: 1_048_576,
    operations: 256,
    evidence: 1_024,
};

/// The profile selected by `ekr.transaction-document/2` (design § 97): 10,000 operations in
/// 8 MiB. Every limit is at least its `/1` value, so a document within the `/1` profile is within
/// this one. Frozen like `/1`: changing it is a further version.
pub const DOCUMENT_V2_LIMITS: DocumentLimits = DocumentLimits {
    input_bytes: 8_388_608,
    depth: 32,
    nodes: 1_048_576,
    mapping_entries: 4_096,
    sequence_elements: 16_384,
    string_bytes: 65_536,
    key_bytes: 4_096,
    total_string_bytes: 33_554_432,
    operations: 10_000,
    evidence: 10_000,
};

/// A transaction-document format version: the envelope's `format` and the profile it selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocumentFormat {
    /// `ekr.transaction-document/1`, the frozen profile of design § 91.3.
    V1,
    /// `ekr.transaction-document/2`, the frozen profile of design § 97.
    V2,
}

impl DocumentFormat {
    /// Every admitted version, oldest first.
    pub const ALL: [Self; 2] = [Self::V1, Self::V2];
    /// The version new documents are written in.
    pub const CURRENT: Self = Self::V2;

    /// The envelope's exact `format` value.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::V1 => "ekr.transaction-document/1",
            Self::V2 => "ekr.transaction-document/2",
        }
    }

    /// The frozen limits this version selects.
    #[must_use]
    pub const fn limits(self) -> DocumentLimits {
        match self {
            Self::V1 => DOCUMENT_V1_LIMITS,
            Self::V2 => DOCUMENT_V2_LIMITS,
        }
    }

    /// The version an exact `format` value names, if it is admitted.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|format| format.name() == name)
    }

    /// The version the envelope's top-level `format:` line names, read without parsing: the first
    /// line that starts at column 0 with the key `format` (plain or quoted) and whose value, with
    /// quotes and a trailing comment removed, is an admitted version.
    fn sniff(bytes: &[u8]) -> Option<Self> {
        bytes.split(|byte| *byte == b'\n').find_map(|line| {
            let line = std::str::from_utf8(line).ok()?;
            let rest = ["format:", "\"format\":", "'format':"]
                .into_iter()
                .find_map(|key| line.strip_prefix(key))?;
            let value = rest.split(" #").next().unwrap_or(rest).trim();
            let value = value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .or_else(|| {
                    value
                        .strip_prefix('\'')
                        .and_then(|value| value.strip_suffix('\''))
                })
                .unwrap_or(value);
            Self::from_name(value)
        })
    }
}

impl std::fmt::Display for DocumentFormat {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// Stable names for document resource refusals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DocumentLimit {
    /// A second document, even if empty.
    #[error("documents")]
    Documents,
    /// The frozen raw input cap.
    #[error("input_bytes")]
    InputBytes,
    /// A stricter cap selected by the current upload host.
    #[error("upload_bytes")]
    UploadBytes,
    /// Expanded container nesting.
    #[error("container_depth")]
    Depth,
    /// Expanded syntax nodes.
    #[error("expanded_nodes")]
    Nodes,
    /// Entries in one mapping.
    #[error("mapping_entries")]
    MappingEntries,
    /// Elements in one sequence.
    #[error("sequence_elements")]
    SequenceElements,
    /// Bytes in a decoded string.
    #[error("string_bytes")]
    StringBytes,
    /// Bytes in a decoded key.
    #[error("key_bytes")]
    KeyBytes,
    /// Cumulative expanded string bytes.
    #[error("total_string_bytes")]
    TotalStringBytes,
    /// More than the maximum operation count.
    #[error("operations")]
    Operations,
    /// No operation is present.
    #[error("empty_operations")]
    EmptyOperations,
    /// Too many input evidence entries.
    #[error("evidence_elements")]
    Evidence,
}

impl DocumentLimit {
    /// The frozen bound this limit names under `format`, in words: what a refusal prints after the
    /// limit's stable name, so the author learns the number without the source.
    #[must_use]
    pub fn bound(self, format: DocumentFormat) -> String {
        let limits = format.limits();
        match self {
            Self::Documents => "exactly one YAML document".to_owned(),
            Self::InputBytes => format!("at most {} bytes", limits.input_bytes),
            Self::UploadBytes => "the host's upload cap".to_owned(),
            Self::Depth => format!("nesting at most {} deep", limits.depth),
            Self::Nodes => format!("at most {} values and keys", limits.nodes),
            Self::MappingEntries => format!("at most {} entries per map", limits.mapping_entries),
            Self::SequenceElements => {
                format!("at most {} elements per sequence", limits.sequence_elements)
            }
            Self::StringBytes => format!("at most {} bytes per string", limits.string_bytes),
            Self::KeyBytes => format!("at most {} bytes per key", limits.key_bytes),
            Self::TotalStringBytes => {
                format!("at most {} bytes of text in all", limits.total_string_bytes)
            }
            Self::Operations => match format {
                DocumentFormat::V1 => format!(
                    "at most {} operations per document; {} admits {}, or split the change \
                     into several transactions",
                    limits.operations,
                    DocumentFormat::V2,
                    DOCUMENT_V2_LIMITS.operations
                ),
                DocumentFormat::V2 => format!(
                    "at most {} operations per document; split the change into several \
                     transactions",
                    limits.operations
                ),
            },
            Self::EmptyOperations => "at least 1 operation".to_owned(),
            Self::Evidence => format!("at most {} evidence entries", limits.evidence),
        }
    }
}

/// A refusal before any durable proposal exists.
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// A named inclusive limit of the format's profile was exceeded (or the operation minimum
    /// was missed).
    #[error("transaction document limit: {0} ({bound})", bound = .0.bound(*.1))]
    Limit(DocumentLimit, DocumentFormat),
    /// Exact bytes must be UTF-8; replacement decoding is forbidden.
    #[error("transaction document is not UTF-8")]
    InvalidUtf8,
    /// The supplied version has no admitted decoder.
    #[error("unsupported transaction document format: {0}")]
    UnsupportedFormat(String),
    /// Invalid YAML, unknown/duplicate fields, wrong shapes or invalid typed data.
    #[error("invalid transaction document: {0}")]
    InvalidDocument(String),
    /// Reading the bounded ingress failed.
    #[error("transaction document read failed: {0}")]
    Io(#[from] std::io::Error),
}

/// The original input and its parsed proposal, without any validation authority.
#[derive(Clone, Debug)]
pub struct TransactionDocument {
    bytes: Vec<u8>,
    transaction: GraphTransaction,
    hash: ContentHash,
    format: DocumentFormat,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub(crate) struct Envelope {
    #[cfg_attr(feature = "schema", schemars(extend("const" = "ekr.transaction-document/2")))]
    format: String,
    transaction: GraphTransaction,
}

impl TransactionDocument {
    /// Parse one UTF-8 document under the immutable profile of the version it declares.
    ///
    /// # Errors
    /// Returns a named resource refusal or invalid syntax/shape/version error.
    pub fn parse(bytes: &[u8]) -> Result<Self, DocumentError> {
        let (format, transaction) = decode(bytes)?;
        Ok(Self {
            bytes: bytes.to_vec(),
            transaction,
            hash: ContentHash::of_bytes(bytes),
            format,
        })
    }

    /// Read no more than the document's frozen input cap plus one byte, then parse.
    ///
    /// # Errors
    /// Returns ingress I/O errors and the same refusals as [`Self::parse`].
    pub fn read(reader: impl Read) -> Result<Self, DocumentError> {
        Self::read_with_upload_limit(reader, DOCUMENT_V2_LIMITS.input_bytes)
    }

    /// Apply a host's stricter upload cap without changing historical parsing.
    /// At most `min(upload_limit, historical_limit) + 1` bytes are read, where the historical
    /// limit is the `/1` cap until the bytes read so far name `/2` on their `format:` line.
    ///
    /// # Errors
    /// Returns a named upload or historical limit, I/O error or parser refusal.
    pub fn read_with_upload_limit(
        mut reader: impl Read,
        upload_limit: usize,
    ) -> Result<Self, DocumentError> {
        let mut profile = DocumentFormat::V1;
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 8_192];
        loop {
            let historical = profile.limits().input_bytes;
            let cap = upload_limit.min(historical);
            if bytes.len() > cap {
                if profile == DocumentFormat::V1
                    && upload_limit > historical
                    && DocumentFormat::sniff(&bytes) == Some(DocumentFormat::V2)
                {
                    profile = DocumentFormat::V2;
                    continue;
                }
                let limit = if upload_limit < historical {
                    DocumentLimit::UploadBytes
                } else {
                    DocumentLimit::InputBytes
                };
                return Err(DocumentError::Limit(limit, profile));
            }
            let remaining = cap + 1 - bytes.len();
            let width = remaining.min(buffer.len());
            match reader.read(&mut buffer[..width]) {
                Ok(0) => break,
                Ok(count) => {
                    if count > width {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "reader exceeded its buffer",
                        )
                        .into());
                    }
                    bytes.extend_from_slice(&buffer[..count]);
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            }
        }
        let (format, transaction) = decode(&bytes)?;
        let hash = ContentHash::of_bytes(&bytes);
        Ok(Self {
            bytes,
            transaction,
            hash,
            format,
        })
    }

    /// Original submitted bytes, including permitted whitespace and comments.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// The typed, unvalidated proposal.
    #[must_use]
    pub const fn transaction(&self) -> &GraphTransaction {
        &self.transaction
    }
    /// Payload-domain hash of [`Self::bytes`], never a canonical transaction hash.
    #[must_use]
    pub const fn hash(&self) -> ContentHash {
        self.hash
    }
    /// The format version the document declares, whose profile it was parsed under.
    #[must_use]
    pub const fn format(&self) -> DocumentFormat {
        self.format
    }
}

/// Select the profile, decode under it, and hold the document to the profile of the version it
/// declares. A document whose `format:` line names no admitted version is decoded under `/1`
/// first, and under `/2` only if `/1` refuses a limit and the document then declares `/2`.
fn decode(bytes: &[u8]) -> Result<(DocumentFormat, GraphTransaction), DocumentError> {
    let sniffed = DocumentFormat::sniff(bytes);
    let candidate = sniffed.unwrap_or(DocumentFormat::V1);
    let (declared, transaction) = match decode_under(bytes, candidate) {
        Ok(decoded) => decoded,
        Err(DocumentError::Limit(limit, DocumentFormat::V1))
            if sniffed.is_none() && limit != DocumentLimit::InputBytes =>
        {
            match decode_under(bytes, DocumentFormat::V2) {
                Ok((declared, transaction)) if declared == DocumentFormat::V2.name() => {
                    return Ok((DocumentFormat::V2, transaction))
                }
                _ => return Err(DocumentError::Limit(limit, DocumentFormat::V1)),
            }
        }
        Err(error) => return Err(error),
    };
    match DocumentFormat::from_name(&declared) {
        Some(format) if format == candidate => Ok((format, transaction)),
        Some(format) => {
            let (again, transaction) = decode_under(bytes, format)?;
            if again == declared {
                Ok((format, transaction))
            } else {
                Err(DocumentError::UnsupportedFormat(again))
            }
        }
        None => Err(DocumentError::UnsupportedFormat(declared)),
    }
}

/// Decode under one profile, returning the declared `format` string unchecked.
fn decode_under(
    bytes: &[u8],
    profile: DocumentFormat,
) -> Result<(String, GraphTransaction), DocumentError> {
    if bytes.len() > profile.limits().input_bytes {
        return Err(DocumentError::Limit(DocumentLimit::InputBytes, profile));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| DocumentError::InvalidUtf8)?;
    let budget = RefCell::new(Budget::new(profile));
    let shape = shape::read(text, &budget).map_err(|error| budget.borrow().error(error))?;
    // Keep one tally. Preflight charges base nodes, keys and tags; actual typed
    // visitors charge scalar Strings and enum boundaries. Shape credits shared
    // key/enum-tag occurrences.
    // Explicit tags remain charged even when a typed scalar ignores its tag.
    let envelope = Envelope::deserialize(checked::Checked::new(
        serde_yaml_ng::Deserializer::from_str(text),
        &shape,
        &budget,
    ))
    .map_err(|error| budget.borrow().error(error))?;
    Ok((envelope.format, envelope.transaction))
}

struct Budget {
    profile: DocumentFormat,
    limits: DocumentLimits,
    nodes: usize,
    strings: usize,
    refused: Option<DocumentLimit>,
}
impl Default for Budget {
    fn default() -> Self {
        Self::new(DocumentFormat::V1)
    }
}
impl Budget {
    const fn new(profile: DocumentFormat) -> Self {
        Self {
            profile,
            limits: profile.limits(),
            nodes: 0,
            strings: 0,
            refused: None,
        }
    }
    fn refuse<E: de::Error>(&mut self, limit: DocumentLimit) -> E {
        self.refused = Some(limit);
        E::custom(format_args!(
            "transaction document limit: {limit} ({})",
            limit.bound(self.profile)
        ))
    }
    fn node<E: de::Error>(&mut self) -> Result<(), E> {
        let cap = self.limits.nodes;
        self.nodes = self
            .nodes
            .checked_add(1)
            .filter(|count| *count <= cap)
            .ok_or_else(|| self.refuse(DocumentLimit::Nodes))?;
        Ok(())
    }
    fn string<E: de::Error>(&mut self, value: &str, key: bool) -> Result<(), E> {
        self.string_with_credit(value, key, 0)
    }
    fn string_with_credit<E: de::Error>(
        &mut self,
        value: &str,
        key: bool,
        credit: usize,
    ) -> Result<(), E> {
        if key && value.len() > self.limits.key_bytes {
            return Err(self.refuse(DocumentLimit::KeyBytes));
        }
        if value.len() > self.limits.string_bytes {
            return Err(self.refuse(DocumentLimit::StringBytes));
        }
        let additional = value
            .len()
            .checked_sub(credit)
            .ok_or_else(|| E::custom("typed string is shorter than its charged representation"))?;
        let cap = self.limits.total_string_bytes;
        self.strings = self
            .strings
            .checked_add(additional)
            .filter(|count| *count <= cap)
            .ok_or_else(|| self.refuse(DocumentLimit::TotalStringBytes))?;
        Ok(())
    }
    fn error(&self, error: serde_yaml_ng::Error) -> DocumentError {
        self.refused.map_or_else(
            || DocumentError::InvalidDocument(error.to_string()),
            |limit| DocumentError::Limit(limit, self.profile),
        )
    }
}
