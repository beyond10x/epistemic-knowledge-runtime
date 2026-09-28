//! `task:write-verbs-cost-most-of-an-ingest`: the checkpoint pointer a handle last wrote is where
//! its next pointer write continues from, and a pointer another handle wrote in between is not
//! lost to that.
//!
//! Every write verb ends with a pointer (design § 96.3), and each first read the whole pointer
//! stream to find the newest pointer and the stream's length, one provider call that on the file
//! provider re-hashes the log. A handle's own last write says both. When another handle has
//! written since, the conditional append is refused as stale, and the write reads the stream and
//! is made again, as it would have been without the handle's record.

use ekr_core::ContentHash;
use ekr_store::{FileStore, RevisionLog, SqliteStore, StreamReads};
use std::path::Path;

const TENANT: &str = "ekr";

fn open(path: &Path, file: bool) -> Box<dyn RevisionLog> {
    if file {
        Box::new(FileStore::file(&path.join("files"), TENANT, None).unwrap())
    } else {
        Box::new(SqliteStore::sqlite(&path.join("state.db"), TENANT, None).unwrap())
    }
}

/// Every pointer the log holds, oldest first, as `(covered, binding)`.
fn pointers(path: &Path, file: bool) -> Vec<(u64, String)> {
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
            )
        })
        .collect()
}

fn binding(label: &str) -> ContentHash {
    ContentHash::of_bytes(label.as_bytes())
}

#[test]
fn a_handle_continues_from_the_pointer_it_last_wrote() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let store = open(directory.path(), file);
        let _ = ekr_store::stream_reads();
        store
            .write_checkpoint(1, binding("first"), Some(b"a first checkpoint"))
            .unwrap();
        let first = ekr_store::stream_reads();
        store.write_checkpoint(2, binding("second"), None).unwrap();
        store
            .write_checkpoint(3, binding("third"), Some(b"a third checkpoint"))
            .unwrap();
        let later = ekr_store::stream_reads();
        if first.checkpoint != 1 {
            wrong.push(format!(
                "file={file}: the first write read the pointer stream {} times",
                first.checkpoint
            ));
        }
        if later != StreamReads::default() {
            wrong.push(format!(
                "file={file}: writes after the handle's own read the streams again: {later:?}"
            ));
        }
        let written = pointers(directory.path(), file);
        let expected = vec![
            (1, binding("first").to_hex()),
            (2, binding("second").to_hex()),
            (3, binding("third").to_hex()),
        ];
        if written != expected {
            wrong.push(format!("file={file}: the log holds {written:?}"));
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn a_pointer_another_handle_wrote_since_is_kept_and_the_next_write_still_lands() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let store = open(directory.path(), file);
        store
            .write_checkpoint(1, binding("mine"), Some(b"my checkpoint"))
            .unwrap();
        open(directory.path(), file)
            .write_checkpoint(2, binding("theirs"), Some(b"their checkpoint"))
            .unwrap();
        store
            .write_checkpoint(3, binding("mine again"), None)
            .unwrap();
        let written = pointers(directory.path(), file);
        let expected = vec![
            (1, binding("mine").to_hex()),
            (2, binding("theirs").to_hex()),
            (3, binding("mine again").to_hex()),
        ];
        if written != expected {
            wrong.push(format!("file={file}: the log holds {written:?}"));
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
