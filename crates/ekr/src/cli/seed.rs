//! `ekr seed`: the kernel's shared seed handler over the declared `ekr-seed/2` parser.

use std::io::Read;
use std::path::{Path, PathBuf};

use ekr_core::{ContentHash, Timestamp};
use ekr_kernel::{Runtime, SeedDocument, SeedError, SeedResultV1};

use crate::exit::Failure;

/// Reads the document, parses it with the kernel's own seed parser, adds each `--evidence` file's
/// bytes to `evidence_payloads` under their content hash, then seeds. A retained exact retry
/// returns the original result; the kernel samples `now` only for a new seed. `open` sees the
/// completed document, so a store is created only for a seed the kernel admits.
pub(super) fn run(
    document: &Path,
    evidence: &[PathBuf],
    stdin: &mut dyn Read,
    open: impl FnOnce(&SeedDocument) -> Result<Runtime, Failure>,
    now: &dyn Fn() -> Timestamp,
) -> Result<SeedResultV1, Failure> {
    let mut bytes = Vec::new();
    super::input::open(document, stdin)?
        .read_to_end(&mut bytes)
        .map_err(|error| Failure::fault(format!("reading {}: {error}", document.display())))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| SeedError::Invalid("seed-decode: the document is not UTF-8".to_owned()))?;
    let mut seed = SeedDocument::from_yaml(&text)?;
    for file in evidence {
        let payload = std::fs::read(file)
            .map_err(|error| Failure::fault(format!("reading {}: {error}", file.display())))?;
        // Equal bytes have an equal key, so a payload also written in the document, or a file
        // named twice, lands once; the kernel's own checks then run on the completed document.
        seed.evidence_payloads
            .insert(ContentHash::of_bytes(&payload), payload);
    }
    Ok(open(&seed)?.seed(seed, now)?)
}
