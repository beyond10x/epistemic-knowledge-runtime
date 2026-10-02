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
//!   which `SqliteEventStore::from_image` opens. With no `-wal` beside it, or one of zero bytes,
//!   which holds no frame, the database file is the whole committed state, and it is read
//!   `immutable=1`: SQLite then takes no lock and makes no `-shm` or `-wal`. A process that may not
//!   write the file cannot change it, and a writer that may is detected rather than excluded: the
//!   size and modification time of the file and its `-wal` are the same after the read as before
//!   it, or the read is taken again. With a `-wal` holding bytes the database is read
//!   `mode=ro&readonly_shm=1`, through SQLite's own locks, opening the `-wal` and `-shm` that are
//!   there and creating no `-shm`. A `-wal` holding bytes without its `-shm` is read again, a
//!   bounded number of times, after a pause ([`Attempt::ShmAbsent`]). The `-wal` and `-shm` are
//!   looked for beside the database's [`resolved`] path, as SQLite names them: beside the file a
//!   symlinked path names, not beside the link.
//!
//! What can appear under a SQLite store's path is exactly one file: an empty `-wal`. SQLite opens
//! a WAL database's `-wal` with `O_CREAT` whatever the connection's flags (`sqlite3WalOpen`), and
//! no URI parameter turns that off. So a writer that closes between this open's look at the `-wal`
//! and SQLite's own open of it — unlinking the `-shm` and then the `-wal` — leaves SQLite to
//! create a `-wal` of zero bytes, owned by this process's user, wherever this process may write
//! the database's directory; SQLite then reports `SQLITE_CANTOPEN` for the `-shm` it may not
//! create, and the read taken again finds that `-wal` empty and reads the file alone. The empty
//! `-wal` holds no frame and changes no read: the next writer's open uses it, the next read-only
//! open reads past it. Where this process may not write the directory, nothing can appear.
//! Nothing else is written under the store's path. The copy costs the store's size once per open,
//! in the temporary directory for the File provider and in memory for SQLite. A long-lived reader
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
        let database = resolved(database);
        Self(vec![stamp(&database), stamp(&sidecar(&database, "-wal"))])
    }

    /// Whether a SQLite store's `-wal` held a byte when this was taken: only then can it hold a
    /// frame the database file does not.
    fn wal_holds_bytes(&self) -> bool {
        matches!(self.0.get(1), Some(Some((length, _))) if *length > 0)
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
    let target = resolved(database);
    let directory = match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_owned(),
        _ => PathBuf::from("."),
    };
    first_denied(
        &[database.to_owned(), directory],
        &[sidecar(&target, "-wal"), sidecar(&target, "-shm")],
    )
}

/// The file beside `database` SQLite names with `suffix`. Call it with a [`resolved`] path: SQLite
/// keeps a database's `-wal` and `-shm` beside the file a symlinked path names, not beside the link.
fn sidecar(database: &Path, suffix: &str) -> PathBuf {
    let mut name = database.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// `database` with every symlink resolved, as SQLite resolves it before it names the `-wal` and
/// `-shm`; the path as given when it cannot be resolved, where nothing is beside it to find.
fn resolved(database: &Path) -> PathBuf {
    std::fs::canonicalize(database).unwrap_or_else(|_| database.to_owned())
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
    let shown = database;
    let database = &resolved(database);
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
    let backend = |error: rusqlite::Error| StoreError::Backend(described(&error));
    let shm = sidecar(database, "-shm");
    let present = |path: &Path| std::fs::symlink_metadata(path).is_ok();
    reading(
        || {
            let before = Signature::sqlite(database);
            if before.wal_holds_bytes() {
                return match read(false) {
                    Err(error) if cant_open(&error) && !present(&shm) => Attempt::ShmAbsent(error),
                    result => Attempt::Done(result.map_err(backend)),
                };
            }
            // No `-wal`, or one of zero bytes, which holds no frame: the file is the whole
            // committed state. Read without locks and without creating a file, then check that
            // nothing wrote it, or its `-wal`, meanwhile.
            match read(true) {
                Err(error) => Attempt::Done(Err(backend(error))),
                Ok(image) if Signature::sqlite(database) == before => Attempt::Done(Ok(image)),
                Ok(_) => Attempt::Changed,
            }
        },
        std::thread::sleep,
    )
    .map_err(|error| match error {
        StoreError::Backend(message) => {
            StoreError::Backend(format!("{}: {message}", shown.display()))
        }
        error => error,
    })
}

/// What one read of a SQLite store came to.
enum Attempt<T> {
    /// Its result, which ends the read.
    Done(Result<T, StoreError>),
    /// No `-wal` holding bytes, and the database file or its `-wal` changed while it was read
    /// immutable.
    Changed,
    /// SQLite could not open the database through its `-wal`, and no `-shm` was beside it.
    ///
    /// A `readonly_shm` connection opens an existing `-shm` and never creates one, so it reports
    /// `SQLITE_CANTOPEN` (extended code 14) whenever a `-wal` is there without its `-shm`. A writer
    /// leaves the two that way for a moment twice: when its open has created the `-wal` and not
    /// yet the `-shm`, and when its close has unlinked the `-shm` and not yet the `-wal`; and
    /// SQLite's own read-only open leaves an empty `-wal` without a `-shm` when the writer's close
    /// unlinks the `-wal` under it. Measured on 2026-10-02 with `tests/adversary_read_only_open.rs`
    /// run 48 at a time at load 115 to 183, before an empty `-wal` was read as no `-wal`: 6 of 1440
    /// runs failed, each with this error and no `-shm` after it, 3 while the writer opened and 3
    /// while it closed, and one later run failed on the empty `-wal` in each of its reads.
    ShmAbsent(rusqlite::Error),
}

/// How many reads a read-only open of a SQLite store takes before it reports the last one's
/// failure: a database that changed while it was read is read again at once, a `-wal` without its
/// `-shm` after a pause (see [`Attempt::ShmAbsent`]).
const SQLITE_READ_ATTEMPTS: usize = 12;

/// The first pause before a `-wal` without its `-shm` is read again; each later pause doubles,
/// up to [`SQLITE_SHM_MAX_PAUSE`]. Eleven pauses add up to at most 527 ms.
const SQLITE_SHM_FIRST_PAUSE: std::time::Duration = std::time::Duration::from_millis(1);

/// The longest pause between two reads of a `-wal` without its `-shm`.
const SQLITE_SHM_MAX_PAUSE: std::time::Duration = std::time::Duration::from_millis(100);

/// Takes `attempt` until it is [`Attempt::Done`], at most [`SQLITE_READ_ATTEMPTS`] times, calling
/// `pause` before each read taken again after [`Attempt::ShmAbsent`].
fn reading<T>(
    mut attempt: impl FnMut() -> Attempt<T>,
    mut pause: impl FnMut(std::time::Duration),
) -> Result<T, StoreError> {
    let mut wait = SQLITE_SHM_FIRST_PAUSE;
    let mut last = attempt();
    for _ in 1..SQLITE_READ_ATTEMPTS {
        match last {
            Attempt::Done(result) => return result,
            Attempt::Changed => {}
            Attempt::ShmAbsent(_) => {
                pause(wait);
                wait = (wait * 2).min(SQLITE_SHM_MAX_PAUSE);
            }
        }
        last = attempt();
    }
    match last {
        Attempt::Done(result) => result,
        Attempt::Changed => Err(StoreError::Backend(format!(
            "changed during each of {SQLITE_READ_ATTEMPTS} read-only reads"
        ))),
        Attempt::ShmAbsent(error) => Err(StoreError::Backend(format!(
            "{}; its -wal was there without the -shm a writer creates beside it in each of \
             {SQLITE_READ_ATTEMPTS} read-only reads",
            described(&error)
        ))),
    }
}

/// Whether SQLite reported that it could not open a file: `SQLITE_CANTOPEN` with any extended code.
fn cant_open(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.code == rusqlite::ErrorCode::CannotOpen
    )
}

/// `error` as text that keeps SQLite's extended result code, which its message alone drops.
fn described(error: &rusqlite::Error) -> String {
    match error {
        rusqlite::Error::SqliteFailure(failure, _) => {
            format!("{error} (SQLite extended code {})", failure.extended_code)
        }
        error => error.to_string(),
    }
}

/// `database` as a SQLite URI filename, read-only and, when asked, immutable. Every byte that is
/// not unreserved is percent-encoded, so no path can add a query parameter of its own.
pub(super) fn uri(database: &Path, immutable: bool) -> Result<String, rusqlite::Error> {
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

#[cfg(test)]
mod sidecars_in_flux {
    use super::{reading, Attempt, SQLITE_READ_ATTEMPTS};
    use crate::StoreError;
    use std::time::Duration;

    fn cant_open() -> rusqlite::Error {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
            Some("unable to open database file".into()),
        )
    }

    /// A writer between creating the `-wal` and its `-shm`, or between unlinking them: the read is
    /// taken again after a pause, and the one that succeeds is returned.
    #[test]
    fn a_wal_without_its_shm_is_read_again_until_the_shm_is_there() {
        let mut calls = 0;
        let mut pauses = Vec::new();
        let read = reading(
            || {
                calls += 1;
                if calls < 4 {
                    Attempt::ShmAbsent(cant_open())
                } else {
                    Attempt::Done(Ok(7))
                }
            },
            |pause| pauses.push(pause),
        );
        assert_eq!(read.unwrap(), 7);
        assert_eq!(calls, 4);
        assert_eq!(pauses.len(), 3, "one pause before each read taken again");
        assert!(
            pauses.windows(2).all(|pair| pair[0] < pair[1]),
            "{pauses:?}"
        );
    }

    /// A `-shm` that never appears is reported after [`SQLITE_READ_ATTEMPTS`] reads, naming
    /// SQLite's own error, its extended code and the missing file.
    #[test]
    fn a_shm_that_never_appears_is_reported_after_the_bounded_reads() {
        let mut calls = 0;
        let mut waited = Duration::ZERO;
        let read: Result<(), _> = reading(
            || {
                calls += 1;
                Attempt::ShmAbsent(cant_open())
            },
            |pause| waited += pause,
        );
        assert_eq!(calls, SQLITE_READ_ATTEMPTS);
        let Err(StoreError::Backend(message)) = read else {
            panic!("{read:?}")
        };
        assert!(
            message.contains("unable to open database file")
                && message.contains("extended code 14")
                && message.contains("-shm")
                && message.contains(&format!("each of {SQLITE_READ_ATTEMPTS} read-only reads")),
            "{message}"
        );
        assert!(waited < Duration::from_secs(1), "{waited:?} in pauses");
    }

    /// A database that changed while it was read immutable is read again at once, as before.
    #[test]
    fn a_database_that_changed_is_read_again_without_a_pause() {
        let mut calls = 0;
        let mut pauses = 0;
        let read: Result<(), _> = reading(
            || {
                calls += 1;
                Attempt::Changed
            },
            |_| pauses += 1,
        );
        assert_eq!(calls, SQLITE_READ_ATTEMPTS);
        assert_eq!(pauses, 0);
        assert!(
            matches!(&read, Err(StoreError::Backend(message))
                if message.contains("changed during each")),
            "{read:?}"
        );
    }

    /// Any other result, success or failure, ends the read at the first attempt.
    #[test]
    fn any_other_result_is_returned_at_once() {
        let mut calls = 0;
        let read: Result<(), _> = reading(
            || {
                calls += 1;
                Attempt::Done(Err(StoreError::Backend("disk I/O error".into())))
            },
            |_| panic!("no pause"),
        );
        assert!(read.is_err());
        assert_eq!(
            calls, 1,
            "only a -wal without its -shm, or a change, is read again"
        );
    }
}
