//! A SQLite store file replaced in place under a live handle
//! (`task:sqlite-store-replaced-in-place`).
//!
//! A database copied over the file a handle opened — `cp` over it, which keeps the file's device
//! and inode — is not seen by the handle's connection: SQLite in WAL mode keeps its page cache
//! while the WAL index has not moved, so the connection goes on answering the database it opened.
//! The eventlog SQLite provider has no check of its own for this, so the store makes one, beside
//! the connection and without it:
//!
//! * the **file identity**: the device and inode at the path must be those the handle opened;
//! * the **log header**: the log's first event, and the newest event this handle has seen in the
//!   database file, read from the file alone (`immutable=1`: no lock, no WAL, no `-shm`). The first
//!   event must be the one the handle's own connection reads, and once seen must stay; the newest
//!   must stay at its place and never move back. A checkpoint only ever moves the file forward
//!   through committed history, so neither changes while the file is the database the handle
//!   opened.
//!
//! The header is read only when the file's `stat` (identity, size, modification and change time)
//! differs from the one it last matched at, so a read of an unchanged store costs one `stat`.
//!
//! Only positive evidence proves a replacement: another device or inode, or a header that reads
//! successfully, **settled**, and differs from the one recorded. A checkpoint writes the database
//! file page by page, and a file read meanwhile can read as malformed, or as a mix of two
//! versions, while its `stat` holds still (file times are coarse). A read is therefore settled
//! only when no checkpoint ran across it: the WAL index's checkpoint record (`nBackfill` and
//! `nBackfillAttempted` in the `-shm` file, SQLite's documented WAL-index format) is the same
//! before and after the read and shows none in progress, and the file's `stat` is the same too. A
//! read that errs or is not settled is taken again, a bounded number of times ([`ATTEMPTS`]),
//! counted and never timed; one still unsettled after them proves nothing, the check passes
//! without recording the `stat`, and the next entry checks again. A read error never concludes
//! `store-replaced`: a database that is really damaged is refused by the read that follows, under
//! its own code.
//!
//! Whatever settled header disagrees is [`StoreError::Replaced`], `store-replaced`, and nothing is
//! answered from the handle; a store opened at the path again reads what is there, or is refused
//! the same way.
//!
//! A plain read-only connection would read a consistent snapshot, but through the WAL beside the
//! file: over a database copied onto a store whose WAL still holds frames, it reads the replaced
//! database's pages, and the replacement it is meant to find is hidden. The file alone is read.
//!
//! Not seen: a backup of this same store restored over it, holding no event older than the newest
//! one this handle has seen in the file, has the same header, and passes.
use crate::StoreError;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The provider's first event for this handle's tenant, or the file's: its log position and
/// provider event id.
pub(super) type Event = (u64, String);

/// One SQLite handle's record of the database it opened.
pub(super) struct AtPath {
    path: PathBuf,
    tenant: String,
    identity: (u64, u64),
    seen: Mutex<Seen>,
}

/// What the handle last saw of the file. `stamp` is `None` until the header has been read once.
#[derive(Default)]
struct Seen {
    stamp: Option<Stamp>,
    first: Option<Event>,
    newest: Option<Event>,
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

/// How many times the header is read before an unsettled or unreadable file is left to the next
/// check: a count, never a duration.
pub(super) const ATTEMPTS: usize = 64;

/// The WAL index's checkpoint record: `nBackfill` and `nBackfillAttempted` (`-shm` offsets 96 and
/// 128, native byte order). A checkpoint sets the second before it writes the database file and
/// the first, to the same frame, after; they differ while one is in progress. `None` without a
/// `-shm` file, where no connection holds the WAL open and nothing checkpoints.
fn checkpoint_record(database: &Path) -> Option<(u32, u32)> {
    let mut name = database.as_os_str().to_owned();
    name.push("-shm");
    let index = std::fs::read(PathBuf::from(name)).ok()?;
    let word = |at: usize| -> Option<u32> {
        Some(u32::from_ne_bytes(index.get(at..at + 4)?.try_into().ok()?))
    };
    Some((word(96)?, word(128)?))
}

/// The file's `stat` and its checkpoint record, when no checkpoint is in progress.
fn quiet(database: &Path) -> Option<(Stamp, Option<(u32, u32)>)> {
    let record = checkpoint_record(database);
    if record.is_some_and(|(backfilled, attempted)| backfilled != attempted) {
        return None;
    }
    Some((stamp(database).ok()?, record))
}

/// What the database file alone says.
enum Header {
    /// It holds no `ekr` events for the tenant yet: everything is still in the WAL.
    Empty,
    /// Its first and newest events for the tenant, and the event now at `at`'s position.
    Events {
        first: Event,
        newest: Event,
        at: Option<String>,
    },
}

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

    /// Refuses as [`StoreError::Replaced`] when the database at the path is no longer the one this
    /// handle opened. `provider_first` is the first event of the tenant's log as the handle's own
    /// connection reads it; it is asked once, the first time the file shows an event.
    pub(super) fn check(
        &self,
        provider_first: impl FnOnce() -> Result<Option<Event>, StoreError>,
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
        let at = seen.newest.as_ref().map(|(position, _)| *position);
        // Unsettled or unreadable after every attempt: no evidence either way. Nothing is recorded,
        // so the next entry checks again; a damaged database is refused by the read that follows.
        let Some((header, steady)) = self.settled_header(at) else {
            return Ok(());
        };
        match header {
            Header::Empty if seen.first.is_some() => {
                return Err(replaced(
                    &self.path,
                    "the file holds none of the events this handle saw in it",
                ))
            }
            Header::Empty => {}
            Header::Events { first, newest, at } => {
                if let Some(held) = &seen.first {
                    if *held != first {
                        return Err(replaced(&self.path, "the log's first event is another"));
                    }
                } else if provider_first()?.as_ref() != Some(&first) {
                    return Err(replaced(
                        &self.path,
                        "the file's first event is not the one this handle's log begins with",
                    ));
                }
                if let Some((position, id)) = &seen.newest {
                    if newest.0 < *position || at.as_ref() != Some(id) {
                        return Err(replaced(
                            &self.path,
                            "the newest event this handle saw in the file is no longer there",
                        ));
                    }
                }
                seen.first = Some(first);
                seen.newest = Some(newest);
            }
        }
        seen.stamp = Some(steady);
        Ok(())
    }

    /// The first header that reads successfully and settled — no checkpoint in progress before
    /// it, and the file's `stat` and checkpoint record the same after it as before — with that
    /// `stat`; `None` when none of [`ATTEMPTS`] reads is. A read that errs is one that did not
    /// settle.
    fn settled_header(&self, at: Option<u64>) -> Option<(Header, Stamp)> {
        for _ in 0..ATTEMPTS {
            let Some(before) = quiet(&self.path) else {
                continue;
            };
            let header = self.header(at);
            if quiet(&self.path).as_ref() != Some(&before) {
                continue;
            }
            if let Ok(header) = header {
                return Some((header, before.0));
            }
        }
        None
    }

    /// The database file's header, read from the file alone.
    fn header(&self, at: Option<u64>) -> Result<Header, rusqlite::Error> {
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
            return Ok(Header::Empty);
        }
        let event = |order: &str| -> Result<Option<Event>, rusqlite::Error> {
            let mut statement = connection.prepare(&format!(
                "SELECT global_seq, event_id FROM {EVENTS} WHERE tenant_id = ?1 \
                 ORDER BY global_seq {order} LIMIT 1"
            ))?;
            let mut rows = statement.query([&self.tenant])?;
            rows.next()?
                .map(|row| Ok((row.get::<_, i64>(0)?.unsigned_abs(), row.get(1)?)))
                .transpose()
        };
        let (Some(first), Some(newest)) = (event("ASC")?, event("DESC")?) else {
            return Ok(Header::Empty);
        };
        let at = match at {
            None => None,
            Some(position) => {
                let mut statement = connection.prepare(&format!(
                    "SELECT event_id FROM {EVENTS} WHERE tenant_id = ?1 AND global_seq = ?2"
                ))?;
                let mut rows = statement.query(rusqlite::params![
                    &self.tenant,
                    i64::try_from(position).unwrap_or(i64::MAX)
                ])?;
                rows.next()?.map(|row| row.get(0)).transpose()?
            }
        };
        Ok(Header::Events { first, newest, at })
    }
}

/// The provider's event table, under the `ekr` owner prefix every store opens with.
const EVENTS: &str = "ekr_events";

fn replaced(path: &Path, why: &str) -> StoreError {
    StoreError::Replaced(format!("{}: {why}", path.display()))
}
