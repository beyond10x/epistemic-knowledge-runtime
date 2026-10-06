//! `ekr seed`: the kernel's shared seed handler over the declared `ekr-seed/2` parser.

use std::io::Read;
use std::path::{Path, PathBuf};

use ekr_core::{ContentHash, Timestamp};
use ekr_kernel::{Runtime, SeedDocument, SeedResultV1, SEED_LIMITS};

use crate::exit::Failure;

/// Reads the document, no more of it than the seed byte cap and one byte, parses it with the
/// kernel's own seed parser, which refuses a document over the cap, adds each `--evidence` file's
/// bytes to `evidence_payloads` under their content hash, then seeds. A retained exact retry
/// returns the original result, unless `if_absent` asks for a seed only if the lineage has none,
/// when any seed already there refuses as `AlreadySeeded`; the kernel samples `now` only for a new
/// seed. `open` sees the completed document, so a store is created only for a seed the kernel
/// admits.
pub(super) fn run(
    document: &Path,
    evidence: &[PathBuf],
    if_absent: bool,
    stdin: &mut dyn Read,
    open: impl FnOnce(&SeedDocument) -> Result<Runtime, Failure>,
    now: &dyn Fn() -> Timestamp,
) -> Result<SeedResultV1, Failure> {
    let mut bytes = Vec::new();
    let cap = u64::try_from(SEED_LIMITS.input_bytes).unwrap_or(u64::MAX);
    super::input::open(document, stdin)?
        .take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| Failure::fault(format!("reading {}: {error}", document.display())))?;
    let mut seed = SeedDocument::from_bytes(&bytes)?;
    for file in evidence {
        let payload = std::fs::read(file)
            .map_err(|error| Failure::fault(format!("reading {}: {error}", file.display())))?;
        // Equal bytes have an equal key, so a payload also written in the document, or a file
        // named twice, lands once; the kernel's own checks then run on the completed document.
        seed.evidence_payloads
            .insert(ContentHash::of_bytes(&payload), payload.into());
    }
    let runtime = open(&seed)?;
    Ok(if if_absent {
        runtime.seed_if_absent(seed, now)?
    } else {
        runtime.seed(seed, now)?
    })
}
