//! `task:read-verbs-open-a-read-only-store`: the private copies a read-only File store is read
//! through do not outlive their process. One case in a binary of its own, because
//! `remove_read_only_copies` removes every copy the process holds, and a concurrent case's copy
//! would go with it.

use std::path::{Path, PathBuf};

use ekr_core::{SchemaVersionId, Timestamp};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{FileStore, ObjectStore, StorageClass};

fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .unwrap()
}

/// The entries of the temporary directory named `ekr-read-only-<pid>-…`.
fn copies_of(pid: u32) -> Vec<PathBuf> {
    let prefix = format!("ekr-read-only-{pid}-");
    std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(&prefix))
        })
        .collect()
}

fn planted(pid: u32) -> PathBuf {
    let path = std::env::temp_dir().join(format!("ekr-read-only-{pid}-planted"));
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("events.jsonl"), b"left behind").unwrap();
    path
}

fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// A read-only open names its copy after this process and removes a copy whose process is gone,
/// keeping one whose process is alive; `remove_read_only_copies`, what `ekr view` and `ekr mcp`
/// call when they are terminated, removes this process's copies at once.
#[test]
fn copies_are_named_by_their_process_and_removed_once_it_is_gone() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("store");
    FileStore::file(&path, "ekr", ontology())
        .unwrap()
        .put(StorageClass::Provenance, b"held", Timestamp::EPOCH)
        .unwrap();

    let mut ended = std::process::Command::new("true").spawn().unwrap();
    let gone = ended.id();
    ended.wait().unwrap();
    let stale = planted(gone);
    // Process 1 is alive and not this process's to signal: its copy stays.
    let alive = planted(1);

    let store = FileStore::file_read_only(&path, "ekr", ontology()).unwrap();
    assert!(
        !exists(&stale),
        "the copy of process {gone}, which ended, is removed"
    );
    assert!(exists(&alive), "the copy of a live process stays");
    let own = copies_of(std::process::id());
    assert_eq!(own.len(), 1, "this process's copy: {own:?}");

    ekr_store::remove_read_only_copies();
    assert!(!exists(&own[0]), "removed while the store is still open");
    drop(store);
    assert_eq!(copies_of(std::process::id()), Vec::<PathBuf>::new());
    std::fs::remove_dir_all(alive).unwrap();
}
