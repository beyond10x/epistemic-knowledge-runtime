//! `ekr migrate --to <path>`: the kernel's preserving migration (design § 100.3) of the configured
//! store into a new store of the same provider, through `Runtime::migrate_into`.
//!
//! The configured store is opened as every store verb opens it and is only read. The destination
//! is created at `--to` under the same host; a path that already holds a store is refused by the
//! kernel as `migrate-destination-not-empty` and left as it was, and a `--to` that is the
//! configured store itself is refused before it is opened a second time.

use std::path::Path;

use ekr_kernel::{Runtime, StoreMigrationV1};

use crate::exit::Failure;

/// The same path, where both exist and resolve to one file or directory.
fn same(store: &Path, to: &Path) -> bool {
    match (std::fs::canonicalize(store), std::fs::canonicalize(to)) {
        (Ok(store), Ok(to)) => store == to,
        _ => false,
    }
}

pub(super) fn run(
    store: &Path,
    to: &Path,
    source: impl FnOnce() -> Result<Runtime, Failure>,
    destination: impl FnOnce() -> Result<Runtime, Failure>,
) -> Result<StoreMigrationV1, Failure> {
    if same(store, to) {
        return Err(Failure::fault(format!(
            "migrate-destination-is-source: --to {} is the configured store; a migration writes \
             a new store beside it",
            to.display()
        )));
    }
    let source = source()?;
    let destination = destination()?;
    Ok(source.migrate_into(&destination)?)
}
