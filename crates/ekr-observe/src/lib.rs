//! The observation layer: source records mapped to deterministic observations.
//!
//! One line of a JSONL source is one source record — a source identity, the source's own id for
//! the record, the time it was captured and its text — and becomes exactly one
//! [`ekr_graph::Observation`] (design § 15). The observation is `ekr.graph.Observation` of
//! `systems/ekr/domains/graph.yaml` and is not redeclared here.
//!
//! What an observation carries of its line:
//!
//! - its content address, [`ContentHash::of_bytes`] over the line's bytes without the `\n` that
//!   ends it (design § 57);
//! - `source`, `source_native_id` and `captured_at`, as the record states them (design § 54);
//! - an id derived from [`ObservationIdempotencyKey`], so the same record always yields the same
//!   id (design § 56).
//!
//! Nothing here persists or deduplicates an observation, and no evidence cites one yet: those
//! wait on `decision-blocker:observation-retention-path` and
//! `decision-blocker:evidence-entry-after-seed`. There is no source adapter and no checkpoint.

use ekr_core::{Canonical, ContentHash, Encoder, ObservationId, Timestamp};
use ekr_graph::{Observation, ObservationContent};
use serde::Deserialize;
use uuid::Builder;

/// What identifies the source record an observation was made from:
/// `ekr.observe.ObservationIdempotencyKey`, `systems/ekr/domains/observe.yaml:45-56`.
///
/// Design § 56: source identity, source-native id, content hash. Two observations of one source
/// record carry equal keys, and so equal ids. `source` and `source_native_id` mean what the fields
/// of that name on [`Observation`] mean.
///
/// The key identifies the record's bytes: `content_hash` is taken over the whole line,
/// `captured_at` included, so the same text under a different timestamp is a different record
/// with a different id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationIdempotencyKey {
    /// The source, by the name the runtime knows it as.
    pub source: String,
    /// The identifier the source itself uses for the record, where it has one.
    pub source_native_id: Option<String>,
    /// The content address of the record's bytes.
    pub content_hash: ContentHash,
}

impl Canonical for ObservationIdempotencyKey {
    /// The three fields in declaration order, structural and untagged.
    fn encode(&self, out: &mut Encoder) {
        self.source.encode(out);
        out.option(self.source_native_id.as_ref());
        self.content_hash.encode(out);
    }
}

impl ObservationIdempotencyKey {
    /// The observation id this key determines.
    ///
    /// The first sixteen bytes of the key's value address, [`ContentHash::of`], marked as a
    /// custom (version 8) UUID: not a minted UUIDv7, because it is derived and must not claim to
    /// carry a creation time. Equal keys give equal ids; a key that differs in any field gives a
    /// different one unless SHA-256 collides in its leading 128 bits.
    #[must_use]
    pub fn observation_id(&self) -> ObservationId {
        let address = ContentHash::of(self);
        let mut leading = [0_u8; 16];
        leading.copy_from_slice(&address.as_bytes()[..16]);
        ObservationId::from_uuid(Builder::from_custom_bytes(leading).into_uuid())
    }
}

/// One JSONL line as the fixture format writes it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceRecord {
    source: String,
    source_native_id: Option<String>,
    captured_at: Timestamp,
    // Required so a line without text is refused; the observation carries its bytes' address.
    #[allow(dead_code)]
    text: String,
}

/// Why a JSONL source did not map.
#[derive(Debug, thiserror::Error)]
pub enum ObserveError {
    /// A line held nothing, so it is not a record.
    #[error("line {line}: a blank line is not a source record")]
    Blank {
        /// The 1-based line number.
        line: usize,
    },
    /// A line was not a source record.
    #[error("line {line}: not a source record: {source}")]
    NotARecord {
        /// The 1-based line number.
        line: usize,
        /// What the JSON reader refused.
        source: serde_json::Error,
    },
}

impl ObserveError {
    /// The 1-based number of the line that was refused.
    #[must_use]
    pub const fn line(&self) -> usize {
        match self {
            Self::Blank { line } | Self::NotARecord { line, .. } => *line,
        }
    }
}

/// Maps one source record line to its observation.
///
/// # Errors
///
/// [`ObserveError`] when the line is blank or is not a source record.
pub fn observe_line(line: &[u8], number: usize) -> Result<Observation, ObserveError> {
    if line.iter().all(u8::is_ascii_whitespace) {
        return Err(ObserveError::Blank { line: number });
    }
    let record: SourceRecord =
        serde_json::from_slice(line).map_err(|source| ObserveError::NotARecord {
            line: number,
            source,
        })?;
    let key = ObservationIdempotencyKey {
        source: record.source,
        source_native_id: record.source_native_id,
        content_hash: ContentHash::of_bytes(line),
    };
    Ok(Observation {
        id: key.observation_id(),
        content: ObservationContent::FeedItem(key.content_hash),
        source: key.source,
        source_native_id: key.source_native_id,
        captured_at: record.captured_at,
    })
}

/// Maps a JSONL source to one observation per line, in line order.
///
/// Empty input has no lines. A final `\n` ends the last line and does not start another; every
/// other line must be a record.
///
/// # Errors
///
/// [`ObserveError`] for the first line that is blank or is not a source record.
pub fn observe_jsonl(bytes: &[u8]) -> Result<Vec<Observation>, ObserveError> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let body = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    body.split(|byte| *byte == b'\n')
        .enumerate()
        .map(|(index, line)| observe_line(line, index + 1))
        .collect()
}
