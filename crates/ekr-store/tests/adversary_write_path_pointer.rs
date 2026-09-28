//! Adversary, wave read-01 unit W (`task:write-verbs-cost-most-of-an-ingest`), pass 1.
//!
//! Design § 98.1 item 5 and the `pointer` field's own documentation: a handle's pointer write
//! continues from its own last one, and "when another handle has written since, its conditional
//! append loses, and the write reads the stream and is made again, as a handle without that record
//! makes it". The memo is also kept after a write that wrote nothing: a verification pointer
//! (`checkpoint: None`) on a store that holds no pointer yet — a store seeded before § 96.3, or one
//! whose best-effort checkpoint write did not happen — records "no pointer, length 0". The next
//! verification write then returns from that record without any append, so the loss is never
//! detected and the pointer another handle's checkpoint made possible is never written.
//!
//! Before 659a913f every write read the pointer stream (`checkpoint_pointer`) and wrote this
//! pointer, naming the other handle's checkpoint.

use ekr_core::ContentHash;
use ekr_store::{FileStore, RevisionLog, SqliteStore};
use std::path::Path;

const TENANT: &str = "ekr";

fn open(path: &Path, file: bool) -> Box<dyn RevisionLog> {
    if file {
        Box::new(FileStore::file(&path.join("files"), TENANT, None).unwrap())
    } else {
        Box::new(SqliteStore::sqlite(&path.join("state.db"), TENANT, None).unwrap())
    }
}

/// Every pointer the log holds, oldest first, as `(covered, binding, checkpoint_hash)`.
fn pointers(path: &Path, file: bool) -> Vec<(u64, String, String)> {
    let events = if file {
        FileStore::file(&path.join("files"), TENANT, None)
            .unwrap()
            .published_events()
    } else {
        SqliteStore::sqlite(&path.join("state.db"), TENANT, None)
            .unwrap()
            .published_events()
    }
    .unwrap();
    events
        .into_iter()
        .filter(|event| event.name == "ekr.store.CheckpointWritten")
        .map(|event| {
            (
                event.data["covered"].as_u64().unwrap(),
                event.data["binding"].as_str().unwrap().to_owned(),
                event.data["checkpoint_hash"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn binding(label: &str) -> ContentHash {
    ContentHash::of_bytes(label.as_bytes())
}

#[test]
fn adversary_write_path_a_verification_pointer_after_another_handles_checkpoint_is_written() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let mine = open(directory.path(), file);
        // No pointer is held yet: a verification pointer has no checkpoint to name, and nothing
        // is written. The handle keeps "no pointer, length 0".
        mine.write_checkpoint(1, binding("mine before"), None)
            .unwrap();
        // Another handle retains a checkpoint.
        let theirs = b"their checkpoint";
        open(directory.path(), file)
            .write_checkpoint(2, binding("theirs"), Some(theirs))
            .unwrap();
        // This handle's next verification now has a checkpoint to name, as a fresh handle's does.
        mine.write_checkpoint(3, binding("mine after"), None)
            .unwrap();

        let their_hash = ContentHash::of_bytes(theirs).to_hex();
        let expected = vec![
            (2, binding("theirs").to_hex(), their_hash.clone()),
            (3, binding("mine after").to_hex(), their_hash),
        ];
        let written = pointers(directory.path(), file);
        if written != expected {
            wrong.push(format!(
                "file={file}: the log holds {written:?}, want {expected:?} — the write after \
                 another handle's pointer returned from this handle's record of an empty stream"
            ));
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
