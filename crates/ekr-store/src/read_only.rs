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
//!   exclusive lock excludes. The directory is named `ekr-read-only-<pid>-…` and removed when the
//!   store drops, by [`remove_read_only_copies`] (which `ekr view` and `ekr mcp` call when they
//!   are sent SIGTERM, SIGINT or SIGHUP), or by the next read-only open in the same temporary
//!   directory once no process of that pid is alive;
//! * a SQLite store is read through a read-only connection into a database image held in memory,
//!   which `SqliteEventStore::from_image` opens, and no file is created beside the database. With
//!   no `-wal` file beside it the database file is the whole committed state, and it is read
//!   `immutable=1`: SQLite then takes no lock and makes no `-shm` or `-wal`. A process that may not
//!   write the file cannot change it, and a writer that may is detected rather than excluded: the
//!   file's size and modification time, and the absence of a `-wal`, are the same after the read as
//!   before it, or the read is taken again. With a `-wal` beside it the database is read
//!   `mode=ro&readonly_shm=1`, through SQLite's own locks, opening the `-wal` and `-shm` that are
//!   there and creating neither.
//!
//! Nothing is written under the store's path. The copy costs the store's size once per open, in
//! the temporary directory for the File provider and in memory for SQLite. A long-lived reader
//! asks [`ReadOnly::changed`] before each read and opens the store again when it has changed.

use crate::StoreError;
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// A store opened read-only: where it was read from, what its files looked like when it was read,
/// and the private copy a File store was read into, removed when this drops.
pub(super) struct ReadOnly {
    pub(super) path: PathBuf,
    pub(super) signature: Signature,
    /// Whether it is a SQLite store, whose signature is [`Signature::sqlite`].
    pub(super) sqlite: bool,
    pub(super) _copy: Option<PrivateCopy>,
}

/// The size and modification time of each file a commit changes, `None` for one that is absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Signature(Vec<Option<(u64, SystemTime)>>);

fn stamp(path: &Path) -> Option<(u64, SystemTime)> {
    let found = std::fs::metadata(path).ok()?;
    Some((found.len(), found.modified().ok()?))
}

impl Signature {
    /// A File store's: `events.jsonl`, which every commit appends to, `manifest.json`, which it
    /// replaces, and the `blobs` directory, whose entries it adds.
    pub(super) fn file(root: &Path) -> Self {
        Self(
            ["events.jsonl", "manifest.json", "blobs"]
                .iter()
                .map(|name| stamp(&root.join(name)))
                .collect(),
        )
    }

    /// A SQLite store's: the database and its `-wal`, where a commit lands first.
    pub(super) fn sqlite(database: &Path) -> Self {
        Self(vec![stamp(database), stamp(&sidecar(database, "-wal"))])
    }
}

/// The copy directories of this process's read-only File stores that are still open.
static LIVE_COPIES: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

/// The directory prefix of every private copy.
const COPY_PREFIX: &str = "ekr-read-only-";

/// One read-only File store's private copy: registered while it lives, removed when it drops.
pub(super) struct PrivateCopy(tempfile::TempDir);

impl PrivateCopy {
    pub(super) fn path(&self) -> &Path {
        self.0.path()
    }
}

impl Drop for PrivateCopy {
    fn drop(&mut self) {
        if let Ok(mut live) = LIVE_COPIES.lock() {
            live.remove(self.0.path());
        }
    }
}

/// Removes the private copy of every read-only File store this process holds open. For a process
/// about to end without dropping them — a signal handler's last act; a store read after this
/// fails to read its copy.
pub fn remove_read_only_copies() {
    let live = match LIVE_COPIES.lock() {
        Ok(mut live) => std::mem::take(&mut *live),
        Err(_) => return,
    };
    for copy in live {
        let _ = std::fs::remove_dir_all(copy);
    }
}

/// Removes the copies in `directory` that a process no longer alive left behind: a directory
/// `ekr-read-only-<pid>-…` whose pid names no process. A copy whose owner cannot be told, or that
/// cannot be removed, is left as it is.
fn remove_stale_copies(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|name| name.strip_prefix(COPY_PREFIX))
            .and_then(|rest| rest.split_once('-'))
            .and_then(|(pid, _)| pid.parse::<i32>().ok())
        else {
            continue;
        };
        if pid != std::process::id() as i32 && !alive(pid) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Whether a process of `pid` exists: one this process may not signal exists.
#[cfg(unix)]
fn alive(pid: i32) -> bool {
    match rustix::process::Pid::from_raw(pid) {
        Some(pid) => !matches!(
            rustix::process::test_kill_process(pid),
            Err(rustix::io::Errno::SRCH)
        ),
        None => true,
    }
}

/// Without a way to ask, every owner is alive.
#[cfg(not(unix))]
fn alive(_: i32) -> bool {
    true
}

impl ReadOnly {
    /// Whether the store's files have changed since it was read: a long-lived reader then opens it
    /// again.
    pub(super) fn changed(&self) -> bool {
        let now = if self.sqlite {
            Signature::sqlite(&self.path)
        } else {
            Signature::file(&self.path)
        };
        now != self.signature
    }

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
pub(super) fn copy_file_store(root: &Path) -> Result<PrivateCopy, StoreError> {
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
    let temporary = std::env::temp_dir();
    remove_stale_copies(&temporary);
    // Made and registered under the registry's lock, so a signal handler's removal either sees
    // it or runs before it exists.
    let copy = {
        let mut live = LIVE_COPIES
            .lock()
            .map_err(|_| StoreError::Backend("the copy registry is poisoned".into()))?;
        let copy = tempfile::Builder::new()
            .prefix(&format!("{COPY_PREFIX}{}-", std::process::id()))
            .tempdir_in(&temporary)
            .map_err(backend)?;
        live.insert(copy.path().to_owned());
        PrivateCopy(copy)
    };
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
    let backend = |error: rusqlite::Error| StoreError::Backend(error.to_string());
    let wal = sidecar(database, "-wal");
    for _ in 0..IMMUTABLE_ATTEMPTS {
        if std::fs::symlink_metadata(&wal).is_ok() {
            return read(false).map_err(backend);
        }
        // No `-wal`: the file is the whole committed state. Read without locks and without
        // creating a file, then check that nothing wrote it meanwhile.
        let before = Signature::sqlite(database);
        let image = read(true).map_err(backend)?;
        if Signature::sqlite(database) == before {
            return Ok(image);
        }
    }
    Err(StoreError::Backend(format!(
        "{} changed during each of {IMMUTABLE_ATTEMPTS} read-only reads",
        database.display()
    )))
}

/// How many times an immutable read is taken again when the database changed while it was read.
const IMMUTABLE_ATTEMPTS: usize = 8;

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
    // `readonly_shm=1`: SQLite opens an existing `-shm` read-only and never creates one.
    uri.push_str(if immutable {
        "?mode=ro&immutable=1"
    } else {
        "?mode=ro&readonly_shm=1"
    });
    Ok(uri)
}
