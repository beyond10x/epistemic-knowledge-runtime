//! `ekr hash`: the content hash an `ekr-seed/2` evidence entry and its `evidence_payloads` key
//! carry for a payload. Config-free; opens no provider.

use std::io::Read;
use std::path::Path;

use ekr_core::ContentHash;
use serde::Serialize;

use crate::exit::Failure;

/// How [`ContentHash::of_bytes`] addresses a payload: its domain label, then the bytes.
pub(super) const ALGORITHM: &str = "sha256(\"ekr.payload.v1\" || bytes)";

/// `ekr hash`'s result.
#[derive(Serialize)]
pub(super) struct Hashed {
    content_hash: String,
    byte_len: usize,
    algorithm: &'static str,
    /// The bytes as the YAML flow sequence an `evidence_payloads` value is written as, for
    /// example `[67, 97, 114]`. One string, so no verb prints a number array.
    payload_yaml: String,
}

/// Reads the payload exactly as given — no newline added or removed — and addresses it.
pub(super) fn run(payload: &Path, stdin: &mut dyn Read) -> Result<Hashed, Failure> {
    let mut bytes = Vec::new();
    super::input::open(payload, stdin)?
        .read_to_end(&mut bytes)
        .map_err(|error| Failure::fault(format!("reading {}: {error}", payload.display())))?;
    Ok(Hashed {
        content_hash: ContentHash::of_bytes(&bytes).to_hex(),
        byte_len: bytes.len(),
        algorithm: ALGORITHM,
        payload_yaml: flow_sequence(&bytes),
    })
}

/// `[b0, b1, ...]`: each byte in decimal, comma and space separated; `[]` when empty.
fn flow_sequence(bytes: &[u8]) -> String {
    let values: Vec<String> = bytes.iter().map(ToString::to_string).collect();
    format!("[{}]", values.join(", "))
}
