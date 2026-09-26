//! `ekr seed`: the kernel's shared seed handler over the declared `ekr-seed/2` parser.

use std::io::Read;
use std::path::Path;

use ekr_core::Timestamp;
use ekr_kernel::{Runtime, SeedDocument, SeedError, SeedResultV1};

use crate::exit::Failure;

/// Reads the document, parses it with the kernel's own seed parser, then seeds. A retained
/// exact retry returns the original result; the kernel samples `now` only for a new seed. `open`
/// sees the parsed document, so a store is created only for a seed the kernel admits.
pub(super) fn run(
    document: &Path,
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
    let seed = SeedDocument::from_yaml(&text)?;
    Ok(open(&seed)?.seed(seed, now)?)
}
