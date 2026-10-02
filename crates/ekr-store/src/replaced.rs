//! A SQLite store file replaced in place under a live handle
//! (`task:sqlite-store-replaced-in-place`).
//!
//! A database copied over the file a handle opened — `cp` over it, which keeps the file's device
//! and inode — is not seen by the handle's connection: SQLite in WAL mode keeps its page cache
//! while the WAL index has not moved, so the connection goes on answering the database it opened.
//! The eventlog SQLite provider has no check of its own for this, so the store makes one, beside
//! the connection and without it.
//!
//! Only positive evidence proves a replacement:
//!
//! * the **file identity**: the device and inode at the path are not those the handle opened;
//! * a **foreign event**: the database file, read alone (`immutable=1`: no lock, no WAL, no
//!   `-shm`), holds as its first or its newest event for the tenant one that this handle's own log
//!   does not hold at that position — asked of the handle's own connection, once per event — and
//!   a second, independent read of the file shows a foreign event again.
//!
//! The file is read only when its `stat` (identity, size, modification and change time) differs
//! from the one the check last passed at, so a read of an unchanged store costs one `stat`.
//!
//! A checkpoint writes the database file page by page, and a file read meanwhile can read as
//! malformed, or as a mix of two versions, while its `stat` holds still (file times are coarse).
//! Every page of such a mix is a page of this handle's own database, so every event it shows is
//! one this log holds at that position: a mix proves nothing, and neither does a read error. A read
//! that errs is taken again, a bounded number of times ([`ATTEMPTS`]), counted and never timed;
//! one still unreadable after them proves nothing, the check passes without recording the `stat`,
//! and the next entry checks again. A database that is really damaged is refused by the read that
//! follows, under its own code.
//!
//! **No file of the store is opened outside SQLite.** POSIX record locks belong to the process
//! and the file, and closing any descriptor of the file releases every one of them: an `open` and
//! `close` of the `-shm` (or the database) in a process holding a SQLite connection on the store
//! drops that connection's `DMS` lock on the `-shm`, another process then takes it exclusively and
//! truncates the `-shm` the connection has mapped, and the connection's next access faults with
//! SIGBUS (`crates/ekr-store/tests/adversary_c7_s.rs`,
//! `no_replacement_check_releases_the_writer_process_lock_on_the_shm`). Only `stat`
//! and SQLite connections touch these files; SQLite defers the close of a descriptor while its own
//! connections in the process hold locks on the file.
//!
//! Not seen, and recorded in `systems/ekr/domains/store.yaml`: a backup of this same store
//! restored over its file, whose first and newest events this log holds at their positions; and
//! a database whose file holds no event yet (all of it still in its own WAL) copied over the store.
use crate::StoreError;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// An event as the file or the log holds it: its log position and provider event id.
pub(super) type Event = (u64, String);

/// One SQLite handle's record of the database it opened.
pub(super) struct AtPath {
    path: PathBuf,
    tenant: String,
    identity: (u64, u64),
    seen: Mutex<Seen>,
}

/// What the check last passed at: the file's `stat`, and the events of the file it has found this
/// log to hold.
#[derive(Default)]
struct Seen {
    stamp: Option<Stamp>,
    ours: Vec<Event>,
}

/// A file's `stat`: identity, length, and modification and change time to the nanosecond.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    device: u64,
    inode: u64,
    len: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

#[cfg(unix)]
fn stamp(path: &Path) -> std::io::Result<Stamp> {
    use std::os::unix::fs::MetadataExt as _;
    let found = std::fs::metadata(path)?;
    Ok(Stamp {
        device: found.dev(),
        inode: found.ino(),
        len: found.len(),
        modified: (found.mtime(), found.mtime_nsec()),
        changed: (found.ctime(), found.ctime_nsec()),
    })
}

/// Without a device and inode to read, the length and modification time alone.
#[cfg(not(unix))]
fn stamp(path: &Path) -> std::io::Result<Stamp> {
    let found = std::fs::metadata(path)?;
    let modified = found
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    Ok(Stamp {
        device: 0,
        inode: 0,
        len: found.len(),
        modified: (
            i64::try_from(modified.as_secs()).unwrap_or(i64::MAX),
            i64::from(modified.subsec_nanos()),
        ),
        changed: (0, 0),
    })
}

/// How many times the file is read before an unreadable file is left to the next check: a count,
/// never a duration.
pub(super) const ATTEMPTS: usize = 64;

impl AtPath {
    /// The record of the database at `path` as it is now.
    pub(super) fn opened(path: &Path, tenant: &str) -> Result<Self, StoreError> {
        let now = stamp(path).map_err(|error| replaced(path, &error.to_string()))?;
        Ok(Self {
            path: path.to_owned(),
            tenant: tenant.to_owned(),
            identity: (now.device, now.inode),
            seen: Mutex::default(),
        })
    }

    /// Refuses as [`StoreError::Replaced`] when the database at the path is positively not the one
    /// this handle opened. `held_at` answers, through the handle's own connection, the provider
    /// event id this log holds at a position, if any.
    pub(super) fn check(
        &self,
        held_at: impl Fn(u64) -> Result<Option<String>, StoreError>,
    ) -> Result<(), StoreError> {
        let mut seen = self
            .seen
            .lock()
            .map_err(|_| StoreError::Document("replacement-check-poisoned".into()))?;
        let now = stamp(&self.path).map_err(|error| replaced(&self.path, &error.to_string()))?;
        if (now.device, now.inode) != self.identity {
            return Err(replaced(
                &self.path,
                "another file is at the path: its device and inode are not the ones opened",
            ));
        }
        if seen.stamp.as_ref() == Some(&now) {
            return Ok(());
        }
        // Unreadable after every attempt: no evidence either way. Nothing is recorded, so the next
        // entry checks again; a damaged database is refused by the read that follows.
        let Some(events) = self.readable_events() else {
            return Ok(());
        };
        if let Some(foreign) = self.foreign(&seen, &events, &held_at)? {
            // A second, independent read must show a foreign event too.
            let again = self.readable_events();
            if let Some(again) = again {
                if self.foreign(&seen, &again, &held_at)?.is_some() {
                    return Err(replaced(
                        &self.path,
                        &format!(
                            "the file holds event {} ({}) at a position where this handle's log \
                             holds another",
                            foreign.0, foreign.1
                        ),
                    ));
                }
            }
            return Ok(());
        }
        // The events just found to be this log's; a file showing none keeps those already found.
        if !events.is_empty() {
            seen.ours = events;
        }
        seen.stamp = Some(now);
        Ok(())
    }

    /// The first of `events` that this log does not hold at its position, if any. An event at a
    /// position where this check already found the log to hold another is foreign without asking
    /// the log again — a log holds one event at a position — so a connection that can no longer
    /// read what it opened is not asked; any other is asked of the log.
    fn foreign(
        &self,
        seen: &Seen,
        events: &[Event],
        held_at: &impl Fn(u64) -> Result<Option<String>, StoreError>,
    ) -> Result<Option<Event>, StoreError> {
        for event in events {
            if seen.ours.contains(event) {
                continue;
            }
            if seen.ours.iter().any(|ours| ours.0 == event.0) {
                return Ok(Some(event.clone()));
            }
            if held_at(event.0)?.as_ref() != Some(&event.1) {
                return Ok(Some(event.clone()));
            }
        }
        Ok(None)
    }

    /// The tenant's first and newest events as the database file alone holds them — none while
    /// everything is still in the WAL — from the first of [`ATTEMPTS`] reads that does not err.
    fn readable_events(&self) -> Option<Vec<Event>> {
        (0..ATTEMPTS).find_map(|_| self.file_events().ok())
    }

    /// The database file's first and newest events for the tenant, read from the file alone.
    fn file_events(&self) -> Result<Vec<Event>, rusqlite::Error> {
        let connection = rusqlite::Connection::open_with_flags(
            super::read_only::uri(&self.path, true)?,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                | rusqlite::OpenFlags::SQLITE_OPEN_URI
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let owned: bool = connection.query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?1)",
            [EVENTS],
            |row| row.get(0),
        )?;
        if !owned {
            return Ok(Vec::new());
        }
        let mut events = Vec::with_capacity(2);
        for order in ["ASC", "DESC"] {
            let mut statement = connection.prepare(&format!(
                "SELECT global_seq, event_id FROM {EVENTS} WHERE tenant_id = ?1 \
                 ORDER BY global_seq {order} LIMIT 1"
            ))?;
            let mut rows = statement.query([&self.tenant])?;
            if let Some(row) = rows.next()? {
                let event: Event = (row.get::<_, i64>(0)?.unsigned_abs(), row.get(1)?);
                if !events.contains(&event) {
                    events.push(event);
                }
            }
        }
        Ok(events)
    }
}

/// The provider's event table, under the `ekr` owner prefix every store opens with.
const EVENTS: &str = "ekr_events";

fn replaced(path: &Path, why: &str) -> StoreError {
    StoreError::Replaced(format!("{}: {why}", path.display()))
}
