//! Adversary pass on wave correct-07 unit F: `task:read-only-open-passes-under-load`.
//!
//! The unit reads a SQLite store whose `-wal` is absent or empty from the database file alone
//! (`immutable=1`, under the size-and-mtime check), and reads one whose `-wal` holds bytes but has
//! no `-shm` again, at most `SQLITE_READ_ATTEMPTS` (12) times. These cases drive it from outside.
//! Where a case needs an event to land between two steps of the open, it watches the store's
//! directory with inotify and acts on the open of a named file, so the interleaving is caused, not
//! hoped for; where it measures, it counts opens, never time.

use std::collections::BTreeMap;
use std::mem::MaybeUninit;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use ekr_core::{SchemaVersionId, Timestamp};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{ObjectStore, SqliteStore, StorageClass};
use rustix::fs::inotify;

fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .unwrap()
}

const HELD: &[u8] = b"adversary-c7-f: bytes a writer acknowledged before the read-only open";

fn sidecar(database: &Path, suffix: &str) -> PathBuf {
    let mut name = database.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Every entry of `directory`: name to (length, mtime in ns, inode, bytes).
fn listing(directory: &Path) -> BTreeMap<String, (u64, i64, u64, Vec<u8>)> {
    std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let found = std::fs::symlink_metadata(entry.path()).unwrap();
            let bytes = if found.is_file() {
                std::fs::read(entry.path()).unwrap()
            } else {
                Vec::new()
            };
            (
                entry.file_name().to_string_lossy().into_owned(),
                (
                    found.len(),
                    found.mtime() * 1_000_000_000 + found.mtime_nsec(),
                    found.ino(),
                    bytes,
                ),
            )
        })
        .collect()
}

/// An inotify watch on `directory` for opens and creations, non-blocking.
fn watch(directory: &Path) -> rustix::fd::OwnedFd {
    let fd = inotify::init(inotify::CreateFlags::NONBLOCK | inotify::CreateFlags::CLOEXEC).unwrap();
    inotify::add_watch(
        &fd,
        directory,
        inotify::WatchFlags::OPEN | inotify::WatchFlags::CREATE,
    )
    .unwrap();
    fd
}

/// The events queued on `fd` now: (file name, whether it was an open, whether a creation).
fn drain(fd: &rustix::fd::OwnedFd) -> Vec<(String, bool, bool)> {
    let mut buffer = [MaybeUninit::uninit(); 8192];
    let mut reader = inotify::Reader::new(fd, &mut buffer);
    let mut events = Vec::new();
    loop {
        match reader.next() {
            Ok(event) => events.push((
                event
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                event.events().contains(inotify::ReadFlags::OPEN),
                event.events().contains(inotify::ReadFlags::CREATE),
            )),
            Err(rustix::io::Errno::WOULDBLOCK) => return events,
            Err(error) => panic!("inotify read: {error}"),
        }
    }
}

/// A store at `directory/copy/state.db` holding [`HELD`], with a `-wal` that holds bytes and a
/// `-shm` beside it, every frame already copied into the database file: the files a writer's close
/// leaves just before it unlinks the `-shm` and then the `-wal`. They are copied from a live writer
/// in `directory/live`, which is then closed, so that no connection of this process holds the
/// copy's inode: SQLite shares one open file and one `-shm` mapping per inode within a process,
/// and a reader beside an in-process writer would not open them again as a reader in another
/// process does.
fn checkpointed_copy(directory: &Path) -> PathBuf {
    let live = directory.join("live");
    let copy = directory.join("copy");
    std::fs::create_dir(&live).unwrap();
    std::fs::create_dir(&copy).unwrap();
    let path = live.join("state.db");
    drop(SqliteStore::sqlite(&path, "ekr", ontology()).unwrap());
    let writer = SqliteStore::sqlite_existing(&path, "ekr", ontology()).unwrap();
    writer
        .put(StorageClass::Provenance, HELD, Timestamp::EPOCH)
        .unwrap();
    let checkpointer = rusqlite::Connection::open(&path).unwrap();
    let (busy, _, _): (i64, i64, i64) = checkpointer
        .query_row("PRAGMA wal_checkpoint(FULL)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(busy, 0, "the checkpoint completed");
    for name in ["state.db", "state.db-wal", "state.db-shm"] {
        std::fs::copy(live.join(name), copy.join(name)).unwrap();
    }
    drop(checkpointer);
    drop(writer);
    let path = copy.join("state.db");
    let wal = std::fs::metadata(sidecar(&path, "-wal")).unwrap();
    assert!(wal.len() > 0, "the -wal holds bytes");
    assert!(sidecar(&path, "-shm").exists(), "the -shm is beside it");
    path
}

/// A writer's close unlinks its `-shm` and then its `-wal` after `sqlite_image` has seen the `-wal`
/// holding bytes and before SQLite opens it. The read-only connection then opens the `-wal` with
/// `O_CREAT` in a directory it may write, fails on the absent `-shm`, and the read taken again
/// finds that `-wal` empty and reads the database alone — which succeeds, but the empty `-wal` it
/// created is left under the store's path. The module says "Nothing else is written under the
/// store's path", the unit's summary says the read-only open no longer leaves an empty `-wal`.
///
/// The unlink is caused, not hoped for: a watcher acts on the reader's first open of `state.db`,
/// which is SQLite's own open inside the read (the stat of the `-wal` opens nothing), and reads
/// page 1 before it opens the `-wal`. Counted over trials; the assertion is that no trial leaves a
/// `-wal` behind and every open holds what the writer acknowledged.
#[test]
#[ignore = "adversary c7-f: the read-only open leaves an empty -wal it created under the store's path"]
fn a_read_only_open_creates_no_wal_where_a_closing_writer_unlinked_it() {
    const TRIALS: usize = 40;
    let mut left = Vec::new();
    let mut failed = Vec::new();
    let mut raced = 0;
    for trial in 0..TRIALS {
        let directory = tempfile::tempdir().unwrap();
        let path = checkpointed_copy(directory.path());
        let (wal, shm) = (sidecar(&path, "-wal"), sidecar(&path, "-shm"));
        let fd = watch(path.parent().unwrap());
        let stop = AtomicBool::new(false);
        let unlinked = AtomicBool::new(false);
        let opened = std::thread::scope(|scope| {
            scope.spawn(|| {
                let mut buffer = [MaybeUninit::uninit(); 8192];
                let mut reader = inotify::Reader::new(&fd, &mut buffer);
                while !stop.load(Ordering::SeqCst) {
                    match reader.next() {
                        Ok(event)
                            if event.events().contains(inotify::ReadFlags::OPEN)
                                && event
                                    .file_name()
                                    .is_some_and(|name| name.to_bytes() == b"state.db")
                                && !unlinked.load(Ordering::SeqCst) =>
                        {
                            std::fs::remove_file(&shm).unwrap();
                            std::fs::remove_file(&wal).unwrap();
                            unlinked.store(true, Ordering::SeqCst);
                        }
                        Ok(_) | Err(rustix::io::Errno::WOULDBLOCK) => {}
                        Err(error) => panic!("inotify read: {error}"),
                    }
                }
            });
            let opened = SqliteStore::sqlite_read_only(&path, "ekr", ontology());
            stop.store(true, Ordering::SeqCst);
            opened
        });
        if unlinked.load(Ordering::SeqCst) {
            raced += 1;
        }
        match opened {
            Ok(store) => {
                if store
                    .get(&ekr_core::ContentHash::of_bytes(HELD))
                    .unwrap()
                    .as_deref()
                    != Some(HELD)
                {
                    failed.push(format!("trial {trial}: the acknowledged object is missing"));
                }
            }
            Err(error) => failed.push(format!("trial {trial}: {error:?}")),
        }
        if let (true, Ok(found)) = (
            unlinked.load(Ordering::SeqCst),
            std::fs::symlink_metadata(&wal),
        ) {
            left.push(format!(
                "trial {trial}: a -wal of {} bytes, -shm {}",
                found.len(),
                if shm.exists() { "present" } else { "absent" }
            ));
        }
    }
    println!("{raced} of {TRIALS} trials unlinked the sidecars during the open");
    assert!(
        raced > 0,
        "the watcher never acted; the case measured nothing"
    );
    assert!(
        failed.is_empty(),
        "read-only opens that failed: {failed:#?}"
    );
    assert!(
        left.is_empty(),
        "{} of {raced} raced read-only opens left a -wal under the store's path: {left:#?}",
        left.len()
    );
}

/// The environment variable that turns this test binary into the writer process of
/// [`a_read_only_open_beside_a_live_writer_changes_nothing_under_the_store_path`]: the database
/// path, then `:truncate` or `:frames`.
const WRITER: &str = "EKR_ADVERSARY_C7_F_WRITER";

/// The child half of the live-writer case: opens the store at the path [`WRITER`] names as a
/// writer, puts [`HELD`], leaves its `-wal` holding frames or truncates it with
/// `wal_checkpoint(TRUNCATE)`, says `ready` and stays open until its stdin closes.
#[test]
#[ignore = "helper: runs only as the writer process another case starts"]
fn writer_process_beside_the_read_only_opens() {
    let Ok(spec) = std::env::var(WRITER) else {
        return;
    };
    let (path, mode) = spec.rsplit_once(':').unwrap();
    let path = Path::new(path);
    let writer = SqliteStore::sqlite_existing(path, "ekr", ontology()).unwrap();
    writer
        .put(StorageClass::Provenance, HELD, Timestamp::EPOCH)
        .unwrap();
    if mode == "truncate" {
        writer
            .put(StorageClass::Provenance, b"second", Timestamp::EPOCH)
            .unwrap();
        let checkpointer = rusqlite::Connection::open(path).unwrap();
        checkpointer
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            .unwrap();
        drop(checkpointer);
    }
    println!("ready");
    let mut rest = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut rest).unwrap();
    drop(writer);
}

/// Beside a live, idle writer in another process — once with frames in its `-wal`, once with a
/// `-wal` that `wal_checkpoint(TRUNCATE)` left empty — read-only opens create no file and change no
/// name, length, mtime, inode or byte of any file in the store's directory, and every open holds
/// what the writer acknowledged. The writer is another process because SQLite shares one `-shm`
/// mapping per inode inside a process, so an in-process reader would not open the `-shm` as a
/// reader of a real deployment does.
#[test]
fn a_read_only_open_beside_a_live_writer_changes_nothing_under_the_store_path() {
    use std::io::BufRead as _;
    for mode in ["frames", "truncate"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        drop(SqliteStore::sqlite(&path, "ekr", ontology()).unwrap());
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "writer_process_beside_the_read_only_opens",
                "--include-ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(WRITER, format!("{}:{mode}", path.display()))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut lines = std::io::BufReader::new(child.stdout.take().unwrap()).lines();
        assert!(
            lines
                .by_ref()
                .map(Result::unwrap)
                .any(|line| line.ends_with("ready")),
            "{mode}: the writer process never became ready"
        );
        let wal = std::fs::metadata(sidecar(&path, "-wal")).unwrap().len();
        assert_eq!(wal == 0, mode == "truncate", "{mode}: -wal of {wal} bytes");
        let before = listing(directory.path());
        assert!(before.contains_key("state.db-shm"), "{:?}", before.keys());
        let fd = watch(directory.path());
        for open in 0..12 {
            let reader = SqliteStore::sqlite_read_only(&path, "ekr", ontology())
                .unwrap_or_else(|error| panic!("{mode}: open {open}: {error:?}"));
            assert_eq!(
                reader
                    .get(&ekr_core::ContentHash::of_bytes(HELD))
                    .unwrap()
                    .as_deref(),
                Some(HELD),
                "{mode}: open {open}"
            );
        }
        let events = drain(&fd);
        let after = listing(directory.path());
        drop(child.stdin.take());
        assert!(
            child.wait().unwrap().success(),
            "{mode}: the writer process"
        );
        let opened: Vec<_> = events
            .iter()
            .filter(|(_, open, _)| *open)
            .map(|(name, _, _)| name.as_str())
            .collect();
        println!("{mode}: files the read-only opens opened: {opened:?}");
        let created: Vec<_> = events.iter().filter(|(_, _, created)| *created).collect();
        assert_eq!(
            created,
            Vec::<&(String, bool, bool)>::new(),
            "{mode}: files created"
        );
        assert_eq!(
            before.keys().collect::<Vec<_>>(),
            after.keys().collect::<Vec<_>>(),
            "{mode}"
        );
        for (name, was) in &before {
            let now = &after[name];
            assert!(
                (was.0, was.1, was.2) == (now.0, now.1, now.2) && was.3 == now.3,
                "{mode}: {name} changed: {:?} -> {:?}",
                (was.0, was.1, was.2),
                (now.0, now.1, now.2)
            );
        }
    }
}

/// A `-wal` holding committed frames without its `-shm` (copied from a live writer) is refused,
/// never read from the database alone, and SQLite opens the `-wal` exactly
/// `SQLITE_READ_ATTEMPTS` (12) times before the refusal: the bound is a count of reads, observed
/// here as a count of opens of the `-wal`. inotify merges identical consecutive events; the count
/// holds because each read opens `state.db` before the `-wal`, so no two `-wal` opens are adjacent.
#[test]
fn a_wal_without_its_shm_is_read_twelve_times_then_refused() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("state.db");
    drop(SqliteStore::sqlite(&path, "ekr", ontology()).unwrap());
    let writer = SqliteStore::sqlite_existing(&path, "ekr", ontology()).unwrap();
    writer
        .put(StorageClass::Provenance, HELD, Timestamp::EPOCH)
        .unwrap();
    let copy = directory.path().join("copy");
    std::fs::create_dir(&copy).unwrap();
    for name in ["state.db", "state.db-wal"] {
        std::fs::copy(directory.path().join(name), copy.join(name)).unwrap();
    }
    drop(writer);
    let fd = watch(&copy);
    let opened = SqliteStore::sqlite_read_only(&copy.join("state.db"), "ekr", ontology());
    let message = match opened {
        Ok(store) => panic!(
            "a -wal holding frames without its -shm was read; the object is {}",
            if store
                .get(&ekr_core::ContentHash::of_bytes(HELD))
                .unwrap()
                .is_some()
            {
                "held"
            } else {
                "missing: the database file was read alone"
            }
        ),
        Err(error) => format!("{error:?}"),
    };
    let wal_opens = drain(&fd)
        .iter()
        .filter(|(name, open, _)| *open && name == "state.db-wal")
        .count();
    assert_eq!(
        wal_opens, 12,
        "opens of the -wal before the refusal: {message}"
    );
    assert!(message.contains("extended code 14"), "{message}");
}

/// A store whose database path is a symlink to the database a live writer writes. SQLite resolves
/// the symlink and keeps the writer's `-wal` beside the target; `sqlite_image` looks for the `-wal`
/// beside the path it was given, finds none, and reads the target's database file alone with
/// `immutable=1`, where no acknowledged commit that is still only in the `-wal` is. The signature
/// check passes: nothing changed. The open then misses an object the writer acknowledged before
/// it began.
#[test]
#[ignore = "adversary c7-f: a read-only open through a symlinked database misses acknowledged commits"]
fn a_read_only_open_through_a_symlinked_database_holds_every_acknowledged_object() {
    let directory = tempfile::tempdir().unwrap();
    let real = directory.path().join("real");
    std::fs::create_dir(&real).unwrap();
    let target = real.join("state.db");
    drop(SqliteStore::sqlite(&target, "ekr", ontology()).unwrap());
    let writer = SqliteStore::sqlite_existing(&target, "ekr", ontology()).unwrap();
    writer
        .put(StorageClass::Provenance, HELD, Timestamp::EPOCH)
        .unwrap();
    assert!(std::fs::metadata(sidecar(&target, "-wal")).unwrap().len() > 0);
    let link = directory.path().join("state.db");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let reader = SqliteStore::sqlite_read_only(&link, "ekr", ontology()).unwrap();
    assert_eq!(
        reader
            .get(&ekr_core::ContentHash::of_bytes(HELD))
            .unwrap()
            .as_deref(),
        Some(HELD),
        "the writer acknowledged this object before the read-only open began"
    );
    drop(writer);
}
