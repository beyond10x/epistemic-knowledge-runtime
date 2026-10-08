//! Adversary pass on wave correct-07 unit S: `task:sqlite-store-replaced-in-place` with
//! `task:held-bytes-notice-deleted-blobs`.
//!
//! The unit checks the SQLite database file before every store read and write
//! (`crates/ekr-store/src/replaced.rs`): its device and inode, then — after any `stat` change — the
//! log's first event and the newest event the handle saw, read from the file alone with
//! `immutable=1`. These cases drive that check from outside: a second handle writing the same
//! store (supported) beside a checkpointer, and a database renamed over the path.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ekr_core::TransactionId;
use ekr_core::{AgentId, ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, Initialize, Publication, PublicationObject,
    RetainedHistory, RevisionLog, SqliteStore, StorageClass, StoreError,
};
use tempfile::TempDir;

const TENANT: &str = "ekr";

/// Admits nothing and reads every record, as `history_cache.rs`'s stand-in does.
struct Touch;

impl CommitAuthority for Touch {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }

    fn replay(
        &self,
        history: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        for occurrence in &history.occurrences {
            history.content(occurrence.event.record_hash, StorageClass::Canonical)?;
        }
        Ok(None)
    }
}

fn sqlite(path: &Path) -> SqliteStore {
    SqliteStore::sqlite(path, TENANT, None)
        .expect("the SQLite provider opens")
        .under(Touch)
}

/// Distinct bytes of about `size` bytes, so that each publication dirties several pages.
fn payload(label: &str, size: usize) -> Vec<u8> {
    let unit = format!("adversary-c7-s {label} {} ", EventId::mint());
    unit.repeat(size / unit.len() + 1).into_bytes()
}

fn occurrence(payload: RevisionPayload, expected_version: u64, record: Vec<u8>) -> Publication {
    let record_hash = ContentHash::of_bytes(&record);
    let objects = std::collections::BTreeMap::from([(
        record_hash,
        PublicationObject {
            storage_class: StorageClass::Canonical,
            stored_at: Timestamp::EPOCH,
            bytes: record,
        },
    )]);
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id: EventId::mint(),
            record_hash,
            payload,
        },
        objects,
        expected_version,
    }
}

fn seeded(store: &SqliteStore, label: &str) {
    let record = payload(&format!("{label} seed"), 64);
    let seed = occurrence(
        RevisionPayload::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash: ContentHash::of_bytes(&record),
        },
        0,
        record,
    );
    assert_eq!(store.initialize(&seed).unwrap(), Appended::Written);
}

fn proposal(store: &SqliteStore, at: u64, size: usize) -> Result<Appended, StoreError> {
    let publication = occurrence(
        RevisionPayload::TransactionProposed {
            transaction_id: TransactionId::mint(),
            proposer: AgentId::mint(),
            operations_hash: None,
        },
        at,
        payload(&format!("proposal {at}"), size),
    );
    store.publish(&publication)
}

fn proposed(store: &SqliteStore, at: u64, size: usize) {
    assert_eq!(proposal(store, at, size).unwrap(), Appended::Written);
}

/// What one race of a reader against a writer on one store counted.
struct Raced {
    reads: u64,
    read_refused: Vec<String>,
    read_other: Vec<String>,
    write_refused: Vec<String>,
    checkpoints: u64,
}

impl Raced {
    fn verdict(&self, publications: u64) -> Result<(), String> {
        eprintln!(
            "raced: {} reads, {} external checkpoints, {publications} publications; refused: {} \
             reads, {} writes; other read failures: {}",
            self.reads,
            self.checkpoints,
            self.read_refused.len(),
            self.write_refused.len(),
            self.read_other.len()
        );
        if self.read_refused.is_empty()
            && self.read_other.is_empty()
            && self.write_refused.is_empty()
        {
            return Ok(());
        }
        Err(format!(
            "beside {} external checkpoints, {} of {} reads and {} attempts at the writer's \
             {publications} publications were refused as store-replaced, {} reads failed \
             otherwise; first read refusal: {:?}; first write refusal: {:?}; first other: {:?}",
            self.checkpoints,
            self.read_refused.len(),
            self.reads,
            self.write_refused.len(),
            self.read_other.len(),
            self.read_refused.first(),
            self.write_refused.first(),
            self.read_other.first()
        ))
    }
}

/// One handle publishes `publications` records of `size` bytes to a store a second handle reads
/// throughout, each on its own thread; with `checkpointer`, a third, plain SQLite connection runs
/// `PRAGMA wal_checkpoint(PASSIVE)` throughout too. With `link`, both handles open the store
/// through a symlink to the database file, and the checkpointer opens the file itself. No file is
/// ever replaced.
///
/// With `opening`, the reading side opens the store again and again with
/// `SqliteStore::sqlite_existing`, the open every `ekr` store verb makes on a store it may write
/// (`Store::open_existing` in `crates/ekr/src/cli/mod.rs`). That open takes no write lock, so it
/// never waits for the writer's
/// ([`an_open_of_an_existing_store_does_not_wait_for_a_held_write_lock`]). `SqliteStore::sqlite`
/// also creates the owner tables, so it takes the write lock on every open, and beside a writer
/// that publishes back to back it can lose that lock for its whole retry window: an outcome of
/// load, not of the replacement check these races are about.
fn race(link: bool, checkpointer: bool, opening: bool, publications: u64, size: usize) -> Raced {
    let directory = TempDir::new().unwrap();
    let target = directory.path().join("state.db");
    let path = if link {
        let link = directory.path().join("linked.db");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        link
    } else {
        target.clone()
    };
    let writer = sqlite(&path);
    seeded(&writer, "shared");
    let done = Arc::new(AtomicBool::new(false));
    let checkpoints = checkpointer.then(|| {
        let done = Arc::clone(&done);
        let target = target.clone();
        std::thread::spawn(move || {
            let connection = rusqlite::Connection::open(&target).unwrap();
            connection
                .busy_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            let mut checkpoints = 0_u64;
            while !done.load(Ordering::SeqCst) {
                let _: (i64, i64, i64) = connection
                    .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })
                    .unwrap();
                checkpoints += 1;
            }
            checkpoints
        })
    });
    let (ready, opened) = std::sync::mpsc::channel();
    let reading = {
        let done = Arc::clone(&done);
        let path = path.clone();
        std::thread::spawn(move || {
            let reader = sqlite(&path);
            reader.history().unwrap();
            ready.send(()).unwrap();
            let (mut reads, mut refused, mut other) = (0_u64, Vec::new(), Vec::new());
            while !done.load(Ordering::SeqCst) {
                reads += 1;
                let read = if opening {
                    SqliteStore::sqlite_existing(&path, TENANT, None).map(|_| ())
                } else {
                    reader.history().map(|_| ())
                };
                match read {
                    Ok(_) => {}
                    Err(StoreError::Replaced(why)) => refused.push(why),
                    Err(error) => other.push(error.to_string()),
                }
            }
            (reads, refused, other)
        })
    };
    opened.recv().unwrap();
    let mut write_refused = Vec::new();
    for at in 1..=publications {
        loop {
            match proposal(&writer, at, size) {
                Ok(Appended::Written) => break,
                Ok(other) => panic!("proposal {at}: {other:?}"),
                Err(StoreError::Replaced(why)) if write_refused.len() < 10_000 => {
                    write_refused.push(why);
                }
                Err(error) => panic!(
                    "proposal {at}, after {} store-replaced refusals: {error}",
                    write_refused.len()
                ),
            }
        }
    }
    done.store(true, Ordering::SeqCst);
    let checkpoints = checkpoints.map_or(0, |thread| thread.join().unwrap());
    let (reads, read_refused, read_other) = reading.join().unwrap();
    Raced {
        reads,
        read_refused,
        read_other,
        write_refused,
        checkpoints,
    }
}

/// Two handles on one store is supported, and so is any other SQLite client beside them. One
/// handle publishes, a plain connection checkpoints the WAL into the database file as any SQLite
/// client may (`sqlite3`, a backup tool, the last connection to close), and the other handle reads
/// throughout. The file is never replaced, so nothing may be refused as `store-replaced`: each
/// such refusal makes a session, MCP or view host drop and reopen a live store, and a session
/// holding a proposal refuse every store verb instead. The header is read from a database file a
/// checkpoint is half-way through writing, which reads as malformed while its `stat` holds still.
#[test]
fn adversary_c7_s_a_reader_beside_a_writer_and_a_checkpointer_is_never_refused_as_replaced() {
    let raced = race(false, true, false, 150, 24 * 1024);
    if let Err(failure) = raced.verdict(150) {
        panic!("{failure}");
    }
}

/// The same race without the third connection: the writer's own automatic checkpoints (every
/// 1000 WAL pages, SQLite's default, which eventlog keeps) are the only writes to the database
/// file. A long-running host beside `ekr` processes that commit is this.
#[test]
fn adversary_c7_s_a_reader_beside_a_writer_that_checkpoints_itself_is_never_refused_as_replaced() {
    let raced = race(false, false, false, 400, 24 * 1024);
    if let Err(failure) = raced.verdict(400) {
        panic!("{failure}");
    }
}

/// The same race, the other side opening the store again and again, as one `ekr` command after
/// another does beside a writer (two writers on one store are supported). An open checks the file
/// once (`EventlogStore::at`), so an open that meets the writer's checkpoint is refused as
/// `store-replaced`, and that command fails. Measured red in 1 of 5 runs: 15937 of
/// 18533 opens refused, in the package suite run.
///
/// The opens are `SqliteStore::sqlite_existing`, as [`race`] says. Opened with
/// `SqliteStore::sqlite` on 2026-10-07, this case and its symlinked twin failed in 12 of 18 runs at
/// load 27–40, every time with `database is locked` and never with an open refused as
/// `store-replaced`; instrumented, each such open waited 5.0–5.3 s in `BEGIN IMMEDIATE` while the
/// writer completed 7–47 publications.
#[test]
fn adversary_c7_s_an_open_beside_a_writer_that_checkpoints_itself_is_never_refused_as_replaced() {
    let raced = race(false, false, true, 400, 24 * 1024);
    if let Err(failure) = raced.verdict(400) {
        panic!("{failure}");
    }
}

/// GUARD: a database renamed over the path (`mv other.db state.db`) is another inode, and the
/// handle's next read and write are refused as `store-replaced`; its connection still reads the
/// unlinked file it opened.
#[test]
fn adversary_c7_s_a_database_renamed_over_the_path_is_refused_as_replaced() {
    let held = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    let path = held.path().join("state.db");
    let reader = sqlite(&path);
    seeded(&reader, "held");
    reader.history().unwrap();
    {
        let replacement = sqlite(&other.path().join("state.db"));
        seeded(&replacement, "other");
        proposed(&replacement, 1, 64);
    }
    std::fs::rename(other.path().join("state.db"), &path).unwrap();
    let read = reader.history().map(|history| history.occurrences.len());
    assert!(
        matches!(read, Err(StoreError::Replaced(_))),
        "the read answered {read:?}"
    );
}

/// A writable store opened through a symlink to its database file, beside a writer and a plain
/// connection checkpointing the file. SQLite resolves the link and keeps the `-wal` and `-shm`
/// beside the file it names, so the check must read the checkpoint record there: read beside the
/// link it finds no `-shm`, takes that as "no checkpoint in progress", and settles on a header
/// read from a file a checkpoint is half-way through writing. Nothing is replaced, so nothing may
/// be refused as `store-replaced`.
#[test]
fn a_symlinked_store_beside_a_checkpointing_writer_is_never_refused_as_replaced() {
    let raced = race(true, true, false, 150, 24 * 1024);
    if let Err(failure) = raced.verdict(150) {
        panic!("{failure}");
    }
}

/// The same through a symlink, the reading side opening the store again and again.
#[test]
fn a_symlinked_store_opened_beside_a_checkpointing_writer_is_never_refused_as_replaced() {
    let raced = race(true, true, true, 150, 24 * 1024);
    if let Err(failure) = raced.verdict(150) {
        panic!("{failure}");
    }
}

/// GUARD for the two opening races above, which count on it: opening an existing store, directly
/// or through a symlink, takes no write lock. A plain connection holds the write lock
/// (`BEGIN IMMEDIATE`) through each whole open, so an open that needed it would fail
/// `database is locked` however long it waited, and one that does not answers.
#[test]
fn an_open_of_an_existing_store_does_not_wait_for_a_held_write_lock() {
    let directory = TempDir::new().unwrap();
    let target = directory.path().join("state.db");
    let link = directory.path().join("linked.db");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let writer = sqlite(&target);
    seeded(&writer, "write lock");
    proposed(&writer, 1, 4096);
    for path in [&target, &link] {
        let holder = rusqlite::Connection::open(&target).unwrap();
        holder.execute_batch("BEGIN IMMEDIATE").unwrap();
        let existing = SqliteStore::sqlite_existing(path, TENANT, None)
            .map(|_| ())
            .map_err(|error| error.to_string());
        let reading = SqliteStore::sqlite_reading(path, TENANT, None)
            .map(|_| ())
            .map_err(|error| error.to_string());
        holder.execute_batch("ROLLBACK").unwrap();
        assert!(
            existing.is_ok() && reading.is_ok(),
            "beside a held write lock, opening {} answered {existing:?} for an existing store and \
             {reading:?} for reading",
            path.display()
        );
    }
}

/// GUARD: a different database copied over the file a symlinked store names — `cp` over it,
/// which keeps the device and inode — is refused as `store-replaced` through the link, for a read
/// and a write alike.
#[test]
fn a_database_copied_over_a_symlinked_store_is_refused_as_replaced() {
    let held = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    let target = held.path().join("state.db");
    let link = held.path().join("linked.db");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let reader = sqlite(&link);
    seeded(&reader, "held");
    proposed(&reader, 1, 64);
    reader.history().unwrap();
    {
        let replacement = sqlite(&other.path().join("state.db"));
        seeded(&replacement, "other");
        proposed(&replacement, 1, 64);
        proposed(&replacement, 2, 64);
    }
    std::fs::copy(other.path().join("state.db"), &target).unwrap();
    let read = reader.history().map(|history| history.occurrences.len());
    assert!(
        matches!(read, Err(StoreError::Replaced(_))),
        "the read through the link answered {read:?}"
    );
    let wrote = proposal(&reader, 2, 64);
    assert!(
        matches!(wrote, Err(StoreError::Replaced(_))),
        "the write through the link answered {wrote:?}"
    );
}

/// The environment variable that turns [`another_process_takes_the_shm_exclusively`] into the
/// child its parent case starts: the `-shm` path to try.
const SHM_CHILD: &str = "EKR_C7S_SHM_CHILD";

/// Child half of the case below, a no-op unless [`SHM_CHILD`] names a `-shm`: tries, without
/// waiting, the exclusive record lock SQLite's `unixLockSharedMemory` takes on the `-shm` before it
/// truncates it, over the whole file, and prints whether it was granted.
#[test]
fn another_process_takes_the_shm_exclusively() {
    let Some(shm) = std::env::var_os(SHM_CHILD) else {
        return;
    };
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&shm)
        .expect("the -shm opens");
    match rustix::fs::fcntl_lock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => println!("SHM-LOCK granted"),
        Err(error) => println!("SHM-LOCK refused {error}"),
    }
}

/// `wave/correct-07` CI: `ekr seed`, racing `ekr head`, died of SIGBUS. POSIX record locks belong
/// to the process and the file, and closing any descriptor of the file releases all of them: the
/// replacement check read the `-shm` with `std::fs` inside the writer's process, so its close
/// dropped the writer connection's `DMS` lock; another process then took the `-shm` exclusively
/// and truncated it under the writer's mapping, and the writer's next access faulted.
///
/// Deterministic, no timing: a writer handle writes, the database file changes under it (a
/// checkpoint), and the writer and a second handle in the same process both run the check, which
/// reads the file. Then another process asks for the `-shm` exclusively, without waiting: it must
/// be refused, because the writer's connection still holds its lock — and the writer writes again.
#[cfg(unix)]
#[test]
fn no_replacement_check_releases_the_writer_process_lock_on_the_shm() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("state.db");
    let writer = sqlite(&path);
    seeded(&writer, "locks");
    proposed(&writer, 1, 4096);
    writer.history().unwrap();
    {
        let checkpointer = rusqlite::Connection::open(&path).unwrap();
        let _: (i64, i64, i64) = checkpointer
            .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .unwrap();
    }
    writer.history().unwrap();
    let second = sqlite(&path);
    second.history().unwrap();
    drop(second);
    writer.history().unwrap();

    let mut shm = path.as_os_str().to_owned();
    shm.push("-shm");
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "another_process_takes_the_shm_exclusively",
            "--nocapture",
        ])
        .env(SHM_CHILD, &shm)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&child.stdout);
    assert!(child.status.success(), "the child failed: {said}");
    assert!(
        said.contains("SHM-LOCK refused"),
        "another process took the -shm exclusively while the writer's connection holds it: {said}"
    );
    proposed(&writer, 2, 4096);
    assert_eq!(writer.history().unwrap().occurrences.len(), 3);
}
