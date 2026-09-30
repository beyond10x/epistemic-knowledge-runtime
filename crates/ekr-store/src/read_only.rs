//! Opening a store this process may read but not write (`task:read-verbs-open-a-read-only-store`).
//!
//! Neither provider at the pinned eventlog revision opens an event store without write access:
//! `FileEventStore::open_existing` opens `writer.lock` for writing and runs a transaction that may
//! recover, sweep and rewrite, and so does its strict capture reader; `SqliteEventStore` opens its
//! connection read-write and needs the database's directory for its WAL files. A read-only open
//! therefore reads the store's committed state as it stands and opens the provider over a private
//! copy of it, which it may write:
//!
//! * a File store is copied, file by file, into a new temporary directory while this process holds
//!   a shared lock on the store's `writer.lock` (opened for reading), which every writer's
//!   exclusive lock excludes; the copy is removed when the store drops;
//! * a SQLite store is read through a read-only connection (`mode=ro`) into a database image held
//!   in memory, which `SqliteEventStore::from_image` opens. A WAL database whose directory this
//!   process may not write, and which has no `-wal` file, cannot be opened `mode=ro` — SQLite has
//!   nowhere to put its `-shm` file — and is read `immutable=1` instead: without a WAL the
//!   database file is the whole committed state, but an immutable read takes no lock, so a writer
//!   with write access that checkpoints into the file during the read is not excluded. That is the
//!   one read-only open that can observe a write in progress; what it reads is still held to the
//!   store's object hashes and replayed lineage, as every read is.
//!
//! Nothing is written under the store's path. The copy costs the store's size once per open, in
//! the temporary directory for the File provider and in memory for SQLite.

use crate::StoreError;
use std::io;
use std::path::{Path, PathBuf};

/// A store opened read-only: where it was read from, and the private copy a File store was read
/// into, removed when this drops.
pub(super) struct ReadOnly {
    pub(super) path: PathBuf,
    pub(super) _copy: Option<tempfile::TempDir>,
}

impl ReadOnly {
    /// The refusal of a write reaching this store.
    pub(super) fn refusal(&self) -> StoreError {
        StoreError::ReadOnly(format!(
            "{} was opened read-only; nothing is written to it",
            self.path.display()
        ))
    }
}

fn backend(error: io::Error) -> StoreError {
    StoreError::Backend(error.to_string())
}

/// Whether this process may write `path`, by its effective identity. A path that does not exist,
/// or whose permissions cannot be read, may not be written.
#[cfg(unix)]
fn may_write(path: &Path) -> bool {
    use rustix::fs::{accessat, Access, AtFlags, CWD};
    accessat(CWD, path, Access::WRITE_OK, AtFlags::EACCESS).is_ok()
}

/// Whether `path` may be written, from its permission bits alone.
#[cfg(not(unix))]
fn may_write(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|found| !found.permissions().readonly())
}

/// The first of `required`, then of those of `if_present` that exist, that this process may not
/// write.
fn first_denied(required: &[PathBuf], if_present: &[PathBuf]) -> Option<PathBuf> {
    required
        .iter()
        .chain(
            if_present
                .iter()
                .filter(|path| std::fs::symlink_metadata(path).is_ok()),
        )
        .find(|path| !may_write(path))
        .cloned()
}

/// The first path of the File store at `root` a writer needs and this process may not write: the
/// directory itself, where every writer stages and renames, its `writer.lock` and `events.jsonl`,
/// and its `blobs` directory.
pub(super) fn file_write_denied(root: &Path) -> Option<PathBuf> {
    first_denied(
        &[root.to_owned()],
        &[
            root.join("writer.lock"),
            root.join("events.jsonl"),
            root.join("blobs"),
        ],
    )
}

/// The first path of the SQLite store at `database` a writer needs and this process may not
/// write: the database, the directory holding it, where SQLite creates its `-wal` and `-shm`
/// files, and those files where they exist.
pub(super) fn sqlite_write_denied(database: &Path) -> Option<PathBuf> {
    let directory = match database.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_owned(),
        _ => PathBuf::from("."),
    };
    first_denied(
        &[database.to_owned(), directory],
        &[sidecar(database, "-wal"), sidecar(database, "-shm")],
    )
}

fn sidecar(database: &Path, suffix: &str) -> PathBuf {
    let mut name = database.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// The refusal of a writing open of the store at `store`, whose `denied` path this process may
/// not write.
pub(super) fn denied(store: &Path, denied: &Path) -> StoreError {
    StoreError::ReadOnly(if denied == store {
        format!("this process may not write {}", store.display())
    } else {
        format!(
            "this process may not write {}, which the store at {} needs written",
            denied.display(),
            store.display()
        )
    })
}

/// Copies the File store at `root` into a new temporary directory, holding a shared lock on its
/// `writer.lock` while it copies so that no writer's transaction runs meanwhile. Only regular
/// files and directories are copied; anything else is refused, as the provider refuses it.
pub(super) fn copy_file_store(root: &Path) -> Result<tempfile::TempDir, StoreError> {
    if !std::fs::symlink_metadata(root)
        .map_err(backend)?
        .file_type()
        .is_dir()
    {
        return Err(StoreError::Backend(format!(
            "{} is not a physical directory",
            root.display()
        )));
    }
    let lock = match std::fs::File::open(root.join("writer.lock")) {
        Ok(lock) => {
            lock.lock_shared().map_err(backend)?;
            Some(lock)
        }
        // No lock to take: the provider refuses the copy as it refuses the store.
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(backend(error)),
    };
    let copy = tempfile::Builder::new()
        .prefix("ekr-read-only-")
        .tempdir()
        .map_err(backend)?;
    copy_tree(root, copy.path())?;
    drop(lock);
    Ok(copy)
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), StoreError> {
    for entry in std::fs::read_dir(from).map_err(backend)? {
        let entry = entry.map_err(backend)?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        let kind = entry.file_type().map_err(backend)?;
        if kind.is_dir() {
            std::fs::create_dir(&target).map_err(backend)?;
            copy_tree(&source, &target)?;
        } else if kind.is_file() {
            // Copied by content into a file this process creates, so the copy is writable
            // whatever the original's permissions are.
            let mut reader = std::fs::File::open(&source).map_err(backend)?;
            let mut writer = std::fs::File::create_new(&target).map_err(backend)?;
            io::copy(&mut reader, &mut writer).map_err(backend)?;
        } else {
            return Err(StoreError::Backend(format!(
                "{} is neither a file nor a directory",
                source.display()
            )));
        }
    }
    Ok(())
}

/// The SQLite database at `database` as one image, read through a read-only connection; `None`
/// when it holds no `ekr` owner tables.
pub(super) fn sqlite_image(
    database: &Path,
    owner_table: &str,
) -> Result<Option<Vec<u8>>, StoreError> {
    let read = |immutable: bool| -> Result<Option<Vec<u8>>, rusqlite::Error> {
        let connection = rusqlite::Connection::open_with_flags(
            uri(database, immutable)?,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                | rusqlite::OpenFlags::SQLITE_OPEN_URI
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection.execute_batch("BEGIN DEFERRED")?;
        let owned: bool = connection.query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?1)",
            [owner_table],
            |row| row.get(0),
        )?;
        if !owned {
            return Ok(None);
        }
        let mut image = connection.serialize("main")?.to_vec();
        connection.execute_batch("COMMIT")?;
        // The file format's read and write versions, 2 for a WAL database: an in-memory database
        // has no WAL, and SQLite opens a WAL image there only once they say rollback (1). The
        // image already holds every committed page, so nothing else changes.
        if let Some(versions) = image.get_mut(18..20) {
            if versions == [2, 2] {
                versions.copy_from_slice(&[1, 1]);
            }
        }
        Ok(Some(image))
    };
    match read(false) {
        Ok(image) => Ok(image),
        // A WAL database with no `-wal` file, in a directory this process may not write: SQLite
        // cannot create the `-shm` file a read-only WAL reader needs. Its file is all there is.
        Err(first) if std::fs::symlink_metadata(sidecar(database, "-wal")).is_err() => {
            read(true).map_err(|_| StoreError::Backend(first.to_string()))
        }
        Err(error) => Err(StoreError::Backend(error.to_string())),
    }
}

/// `database` as a SQLite URI filename, read-only and, when asked, immutable. Every byte that is
/// not unreserved is percent-encoded, so no path can add a query parameter of its own.
fn uri(database: &Path, immutable: bool) -> Result<String, rusqlite::Error> {
    let absolute = std::path::absolute(database)
        .map_err(|error| rusqlite::Error::InvalidPath(PathBuf::from(error.to_string())))?;
    let text = absolute
        .to_str()
        .ok_or_else(|| rusqlite::Error::InvalidPath(absolute.clone()))?;
    let mut uri = String::from("file:");
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri.push_str(if immutable {
        "?mode=ro&immutable=1"
    } else {
        "?mode=ro"
    });
    Ok(uri)
}
