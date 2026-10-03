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
//! This adapter maps records; the kernel's `import_observation` operation independently retains
//! observations through `ekr-store` without a canonical revision. Both use the unchanged key
//! implementation re-exported here from `ekr-core`. Retention does not itself admit an evidence
//! interpretation into canonical state. There is no live source connector or source checkpoint.

pub use ekr_core::ObservationIdempotencyKey;
use ekr_core::{ContentHash, Timestamp};
use ekr_graph::{Observation, ObservationContent};
use serde::Deserialize;

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
    /// A line held a `\n`, so it is not one line: a record's bytes end before its terminator.
    #[error("line {line}: holds a newline, so it is not one source record line")]
    HoldsNewline {
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
            Self::Blank { line } | Self::HoldsNewline { line } | Self::NotARecord { line, .. } => {
                *line
            }
        }
    }
}

/// Maps one source record line, without its terminating `\n`, to its observation.
///
/// A line holding a `\n` anywhere is refused rather than mapped: the content hash covers every
/// byte, so accepting a terminated line would give the record a second id beside the one
/// [`observe_jsonl`] gives it.
///
/// # Errors
///
/// [`ObserveError`] when the line holds a `\n`, is blank or is not a source record.
pub fn observe_line(line: &[u8], number: usize) -> Result<Observation, ObserveError> {
    if line.contains(&b'\n') {
        return Err(ObserveError::HoldsNewline { line: number });
    }
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
