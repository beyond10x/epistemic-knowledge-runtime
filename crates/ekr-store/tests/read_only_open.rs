//! `task:read-verbs-open-a-read-only-store`, at the store: a store this process may read but not
//! write opens for reading on both providers and answers what it holds, refuses every write as
//! `StoreError::ReadOnly`, writes no checkpoint, and leaves every byte at its path as it was; a
//! writing open of it is refused as `StoreError::ReadOnly` before anything is opened.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use ekr_core::{ContentHash, SchemaVersionId, Timestamp};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{FileStore, ObjectStore, RevisionLog, SqliteStore, StorageClass, StoreError};

fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .unwrap()
}

const HELD: &[u8] = b"bytes the store holds before it is made read-only";
const LATER: &[u8] = b"bytes a write would add";

#[derive(Clone, Copy, Debug)]
enum Provider {
    File,
    Sqlite,
}

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

/// A store in `directory/world`, whose directory is what is made read-only.
struct World {
    _outer: tempfile::TempDir,
    directory: PathBuf,
    provider: Provider,
}

impl World {
    fn new(provider: Provider) -> Self {
        let outer = tempfile::tempdir().unwrap();
        let directory = outer.path().join("world");
        std::fs::create_dir(&directory).unwrap();
        Self {
            _outer: outer,
            directory,
            provider,
        }
    }

    fn path(&self) -> PathBuf {
        match self.provider {
            Provider::File => self.directory.join("store"),
            Provider::Sqlite => self.directory.join("state.db"),
        }
    }

    fn create(&self) -> Store {
        match self.provider {
            Provider::File => Store::File(Box::new(
                FileStore::file(&self.path(), "ekr", ontology()).unwrap(),
            )),
            Provider::Sqlite => Store::Sqlite(Box::new(
                SqliteStore::sqlite(&self.path(), "ekr", ontology()).unwrap(),
            )),
        }
    }

    fn reading(&self) -> Result<Store, StoreError> {
        Ok(match self.provider {
            Provider::File => Store::File(Box::new(FileStore::file_reading(
                &self.path(),
                "ekr",
                ontology(),
            )?)),
            Provider::Sqlite => Store::Sqlite(Box::new(SqliteStore::sqlite_reading(
                &self.path(),
                "ekr",
                ontology(),
            )?)),
        })
    }

    fn existing(&self) -> Result<Store, StoreError> {
        Ok(match self.provider {
            Provider::File => Store::File(Box::new(FileStore::file_existing(
                &self.path(),
                "ekr",
                ontology(),
            )?)),
            Provider::Sqlite => Store::Sqlite(Box::new(SqliteStore::sqlite_existing(
                &self.path(),
                "ekr",
                ontology(),
            )?)),
        })
    }

    fn bytes(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        walk(&self.directory)
            .into_iter()
            .filter(|path| path.is_file())
            .map(|path| {
                let bytes = std::fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect()
    }

    fn read_only(&self) -> ReadOnly {
        set_modes(&self.directory, 0o555, 0o444);
        ReadOnly(self.directory.clone())
    }
}

enum Store {
    File(Box<FileStore>),
    Sqlite(Box<SqliteStore>),
}

impl Store {
    fn put(&self, bytes: &[u8]) -> Result<(), StoreError> {
        match self {
            Self::File(store) => store.put(StorageClass::Provenance, bytes, Timestamp::EPOCH),
            Self::Sqlite(store) => store.put(StorageClass::Provenance, bytes, Timestamp::EPOCH),
        }
        .map(|_| ())
    }

    fn get(&self, bytes: &[u8]) -> Option<Vec<u8>> {
        let hash = ContentHash::of_bytes(bytes);
        match self {
            Self::File(store) => store.get(&hash),
            Self::Sqlite(store) => store.get(&hash),
        }
        .unwrap()
    }

    fn is_read_only(&self) -> bool {
        match self {
            Self::File(store) => store.is_read_only(),
            Self::Sqlite(store) => store.is_read_only(),
        }
    }

    fn write_checkpoint(&self) -> Result<bool, StoreError> {
        let binding = ContentHash::of_bytes(b"binding");
        match self {
            Self::File(store) => store.write_checkpoint(1, binding, Some(b"checkpoint")),
            Self::Sqlite(store) => store.write_checkpoint(1, binding, Some(b"checkpoint")),
        }
    }
}

/// Restores write permission on drop.
struct ReadOnly(PathBuf);

impl Drop for ReadOnly {
    fn drop(&mut self) {
        set_modes(&self.0, 0o755, 0o644);
    }
}

fn set_modes(at: &Path, directory: u32, file: u32) {
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(0o755)).unwrap();
    for path in walk(at) {
        let mode = if path.is_dir() { directory } else { file };
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
    }
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(directory)).unwrap();
}

fn walk(at: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(at).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(walk(&path));
        }
        found.push(path);
    }
    found
}

#[test]
fn a_read_only_store_opens_for_reading_answers_and_refuses_every_write() {
    for provider in PROVIDERS {
        let world = World::new(provider);
        world.create().put(HELD).unwrap();
        let writable = world.reading().unwrap();
        assert!(
            !writable.is_read_only(),
            "{provider:?}: a writable store opens writable"
        );
        drop(writable);
        let before = world.bytes();
        let guard = world.read_only();

        let store = world.reading().unwrap();
        assert!(store.is_read_only(), "{provider:?}");
        assert_eq!(store.get(HELD).as_deref(), Some(HELD), "{provider:?}");
        match store.put(LATER) {
            Err(StoreError::ReadOnly(why)) => assert!(
                why.contains(&world.path().display().to_string()),
                "{provider:?}: the refusal names the store: {why}"
            ),
            other => panic!("{provider:?}: a write to a read-only store: {other:?}"),
        }
        assert_eq!(
            store.get(LATER),
            None,
            "{provider:?}: the refused write left nothing"
        );
        assert_eq!(
            store.write_checkpoint(),
            Ok(false),
            "{provider:?}: a read-only store keeps no checkpoint"
        );
        match world.existing() {
            Err(StoreError::ReadOnly(why)) => assert!(
                why.contains(&world.path().display().to_string())
                    || why.contains(&world.directory.display().to_string()),
                "{provider:?}: {why}"
            ),
            Err(other) => panic!("{provider:?}: a writing open: {other:?}"),
            Ok(_) => panic!("{provider:?}: a writing open of a read-only store opened"),
        }
        drop(store);
        drop(guard);
        assert!(
            world.bytes() == before,
            "{provider:?}: the store's bytes changed"
        );
    }
}

/// A SQLite store another connection is writing through holds its newest commit in its `-wal`
/// file: a read-only open reads it there, through the shared memory file it may only read.
#[test]
fn a_sqlite_store_with_a_live_wal_is_read_with_its_newest_commit() {
    let world = World::new(Provider::Sqlite);
    let writer = world.create();
    writer.put(HELD).unwrap();
    let wal = world.directory.join("state.db-wal");
    assert!(
        std::fs::metadata(&wal).is_ok_and(|found| found.len() > 0),
        "the writer's commit is in the WAL"
    );
    let guard = world.read_only();
    let store = world.reading().unwrap();
    assert!(store.is_read_only());
    assert_eq!(store.get(HELD).as_deref(), Some(HELD));
    drop(store);
    drop(guard);
    // A read-only open of a store this process may write reads the same way.
    let direct = SqliteStore::sqlite_read_only(&world.path(), "ekr", ontology()).unwrap();
    assert!(direct.is_read_only());
    assert_eq!(
        direct.get(&ContentHash::of_bytes(HELD)).unwrap().as_deref(),
        Some(HELD)
    );
    drop(writer);
}

/// The File provider reads a private copy, and the copy is gone once the store is dropped.
#[test]
fn a_read_only_file_store_removes_its_copy_when_it_drops() {
    let world = World::new(Provider::File);
    world.create().put(HELD).unwrap();
    let manifest = std::fs::read(world.path().join("manifest.json")).unwrap();
    let copies = || -> Vec<PathBuf> {
        std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("ekr-read-only-"))
                    && std::fs::read(path.join("manifest.json")).is_ok_and(|held| held == manifest)
            })
            .collect()
    };
    let guard = world.read_only();
    let store = world.reading().unwrap();
    assert_eq!(copies().len(), 1, "one copy while the store is open");
    drop(store);
    drop(guard);
    assert_eq!(copies(), Vec::<PathBuf>::new(), "no copy after it drops");
    // Opened read-only directly, on a store this process may write: a copy, then none.
    let direct = FileStore::file_read_only(&world.path(), "ekr", ontology()).unwrap();
    assert!(direct.is_read_only());
    assert_eq!(copies().len(), 1, "one copy while the store is open");
    drop(direct);
    assert_eq!(copies(), Vec::<PathBuf>::new(), "no copy after it drops");
}

/// A read-only path that holds no store is `NoStore`, as a writable one is: an empty directory,
/// and a SQLite database without the owner tables.
#[test]
fn a_read_only_path_holding_no_store_is_no_store() {
    let world = World::new(Provider::File);
    std::fs::create_dir(world.path()).unwrap();
    let guard = world.read_only();
    assert!(matches!(world.reading(), Err(StoreError::NoStore(_))));
    drop(guard);

    let world = World::new(Provider::Sqlite);
    rusqlite::Connection::open(world.path())
        .unwrap()
        .execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE other (a INTEGER);")
        .unwrap();
    let guard = world.read_only();
    assert!(matches!(world.reading(), Err(StoreError::NoStore(_))));
    drop(guard);
}
