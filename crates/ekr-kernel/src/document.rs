//! Bounded, exact-byte `ekr.transaction-document/1` ingress.
//!
//! The version selects an immutable parser profile, independently of a host's
//! upload cap. The YAML loader buffers events under the raw input cap. A bounded
//! representation pass then checks expanded syntax, including alias occurrences,
//! before strict typed decoding reads the original bytes. The typed traversal
//! adds actual enum boundaries; ignored tag metadata adds no semantic depth.
//! Neither pass claims a
//! pre-loader scalar/event quota or an exact allocator-byte bound.
//!
//! Parsing creates no proposal, receipt, store object or canonical value. In
//! particular, nonfinite Float proposals remain available to kernel validation.

mod checked;
mod shape;

use std::{cell::RefCell, io::Read};

use ekr_core::ContentHash;
use serde::{de, Deserialize};

use crate::GraphTransaction;

/// Inclusive frozen limits for the original transaction-document format.
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

/// A refusal before any durable proposal exists.
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// A named inclusive limit was exceeded (or the operation minimum was missed).
    #[error("transaction document limit: {0}")]
    Limit(DocumentLimit),
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
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub(crate) struct Envelope {
    #[cfg_attr(feature = "schema", schemars(extend("const" = "ekr.transaction-document/1")))]
    format: String,
    transaction: GraphTransaction,
}

impl TransactionDocument {
    /// Parse one UTF-8 document under its immutable historical profile.
    ///
    /// # Errors
    /// Returns a named resource refusal or invalid syntax/shape/version error.
    pub fn parse(bytes: &[u8]) -> Result<Self, DocumentError> {
        Self::check_length(bytes.len())?;
        let transaction = decode(bytes)?;
        Ok(Self {
            bytes: bytes.to_vec(),
            transaction,
            hash: ContentHash::of_bytes(bytes),
        })
    }

    /// Read no more than the frozen input cap plus one byte, then parse.
    ///
    /// # Errors
    /// Returns ingress I/O errors and the same refusals as [`Self::parse`].
    pub fn read(reader: impl Read) -> Result<Self, DocumentError> {
        Self::read_with_upload_limit(reader, DOCUMENT_V1_LIMITS.input_bytes)
    }

    /// Apply a host's stricter upload cap without changing historical parsing.
    /// At most `min(upload_limit, historical_limit) + 1` bytes are read.
    ///
    /// # Errors
    /// Returns a named upload or historical limit, I/O error or parser refusal.
    pub fn read_with_upload_limit(
        mut reader: impl Read,
        upload_limit: usize,
    ) -> Result<Self, DocumentError> {
        let cap = upload_limit.min(DOCUMENT_V1_LIMITS.input_bytes);
        let limit = if upload_limit < DOCUMENT_V1_LIMITS.input_bytes {
            DocumentLimit::UploadBytes
        } else {
            DocumentLimit::InputBytes
        };
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 8_192];
        loop {
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
                    if bytes.len() > cap {
                        return Err(DocumentError::Limit(limit));
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            }
        }
        let transaction = decode(&bytes)?;
        let hash = ContentHash::of_bytes(&bytes);
        Ok(Self {
            bytes,
            transaction,
            hash,
        })
    }

    fn check_length(length: usize) -> Result<(), DocumentError> {
        if length > DOCUMENT_V1_LIMITS.input_bytes {
            Err(DocumentError::Limit(DocumentLimit::InputBytes))
        } else {
            Ok(())
        }
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
}

fn decode(bytes: &[u8]) -> Result<GraphTransaction, DocumentError> {
    let text = std::str::from_utf8(bytes).map_err(|_| DocumentError::InvalidUtf8)?;
    let budget = RefCell::new(Budget::default());
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
    if envelope.format != "ekr.transaction-document/1" {
        return Err(DocumentError::UnsupportedFormat(envelope.format));
    }
    Ok(envelope.transaction)
}

#[derive(Default)]
struct Budget {
    nodes: usize,
    strings: usize,
    refused: Option<DocumentLimit>,
}
impl Budget {
    fn refuse<E: de::Error>(&mut self, limit: DocumentLimit) -> E {
        self.refused = Some(limit);
        E::custom(format_args!("transaction document limit: {limit}"))
    }
    fn node<E: de::Error>(&mut self) -> Result<(), E> {
        self.nodes = self
            .nodes
            .checked_add(1)
            .filter(|count| *count <= DOCUMENT_V1_LIMITS.nodes)
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
        if key && value.len() > DOCUMENT_V1_LIMITS.key_bytes {
            return Err(self.refuse(DocumentLimit::KeyBytes));
        }
        if value.len() > DOCUMENT_V1_LIMITS.string_bytes {
            return Err(self.refuse(DocumentLimit::StringBytes));
        }
        let additional = value
            .len()
            .checked_sub(credit)
            .ok_or_else(|| E::custom("typed string is shorter than its charged representation"))?;
        self.strings = self
            .strings
            .checked_add(additional)
            .filter(|count| *count <= DOCUMENT_V1_LIMITS.total_string_bytes)
            .ok_or_else(|| self.refuse(DocumentLimit::TotalStringBytes))?;
        Ok(())
    }
    fn error(&self, error: serde_yaml_ng::Error) -> DocumentError {
        self.refused.map_or_else(
            || DocumentError::InvalidDocument(error.to_string()),
            DocumentError::Limit,
        )
    }
}
