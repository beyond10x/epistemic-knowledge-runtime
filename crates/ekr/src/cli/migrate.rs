//! `ekr migrate --to <path>`: the kernel's preserving migration (design § 100.3) of the configured
//! store into a new store of the same provider, through `Runtime::migrate_into`.
//!
//! The configured store is opened as every store verb opens it and is only read. The destination
//! is created at `--to` under the same host; a path that already holds a store is refused by the
//! kernel as `migrate-destination-not-empty` and left as it was. A `--to` that is the configured
//! store, lies inside it or contains it is refused before anything is opened or created, so that
//! no byte is written into `--store`.

use std::path::{Path, PathBuf};

use ekr_kernel::{Runtime, StoreMigrationV1};

use crate::exit::Failure;

/// `path` with every symbolic link and `..` resolved: its nearest existing ancestor made
/// canonical, and the components below that ancestor, which do not exist yet, appended as given.
fn resolved(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut missing = Vec::new();
    let mut existing = absolute.as_path();
    loop {
        if let Ok(canonical) = std::fs::canonicalize(existing) {
            return missing
                .iter()
                .rev()
                .fold(canonical, |at: PathBuf, part| at.join(part));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name.to_owned());
                existing = parent;
            }
            _ => return absolute,
        }
    }
}

/// The refusal for a `--to` that would write into `--store`, or over it, if any.
fn overlaps(store: &Path, to: &Path) -> Option<Failure> {
    let (store, to) = (resolved(store), resolved(to));
    // A SQLite store is its database file and the side files beside it.
    let sides = ["-wal", "-shm", "-journal"].map(|side| {
        let mut name = store.clone().into_os_string();
        name.push(side);
        PathBuf::from(name)
    });
    let (code, what) = if to == store || sides.contains(&to) {
        (
            "migrate-destination-is-source",
            "is the configured store; a migration writes a new store beside it",
        )
    } else if to.starts_with(&store) {
        (
            "migrate-destination-inside-source",
            "lies inside the configured store; a migration writes nothing into it",
        )
    } else if store.starts_with(&to) {
        (
            "migrate-destination-contains-source",
            "contains the configured store; a migration writes a new store beside it",
        )
    } else {
        return None;
    };
    Some(Failure::fault(format!(
        "{code}: --to {} {what}",
        to.display()
    )))
}

pub(super) fn run(
    store: &Path,
    to: &Path,
    source: impl FnOnce() -> Result<Runtime, Failure>,
    destination: impl FnOnce() -> Result<Runtime, Failure>,
) -> Result<StoreMigrationV1, Failure> {
    if let Some(refused) = overlaps(store, to) {
        return Err(refused);
    }
    let source = source()?;
    let destination = destination()?;
    Ok(source.migrate_into(&destination)?)
}
