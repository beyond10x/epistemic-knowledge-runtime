//! The one document input handler Seed and Propose share: a path, or `-` for stdin.

use std::io::Read;
use std::path::Path;

use crate::exit::Failure;

/// Opens the document without reading it. Propose hands the reader straight to the kernel's
/// bounded ingress, so no unlimited pre-read happens here.
pub(super) fn open<'a>(
    document: &Path,
    stdin: &'a mut dyn Read,
) -> Result<Box<dyn Read + 'a>, Failure> {
    if document == Path::new("-") {
        return Ok(Box::new(stdin));
    }
    std::fs::File::open(document)
        .map(|file| Box::new(file) as Box<dyn Read>)
        .map_err(|error| Failure::fault(format!("opening {}: {error}", document.display())))
}
