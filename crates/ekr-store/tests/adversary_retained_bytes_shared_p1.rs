//! Adversary pass, wave perf-01 unit B (`story:retained-bytes-shared-not-copied`), at `7f299158`.
//!
//! A history now holds the very allocation its handle verified, and the process registry holds a
//! weak reference to it. Once the handle and every history it returned are dropped, no retained
//! byte may stay alive: the registry must neither keep nor resurrect them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Weak;

use ekr_core::{ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, FileStore, Initialize, Publication,
    PublicationObject, RetainedHistory, RevisionLog, SqliteStore, StorageClass, StoreError,
};
use tempfile::TempDir;

/// Reads every record and seed envelope through `RetainedHistory::content`, and admits nothing.
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
            if let RevisionPayload::Seeded { seed_hash, .. } = occurrence.event.payload {
                history.content(seed_hash, StorageClass::Canonical)?;
            }
        }
        Ok(None)
    }
}

fn seed_publication() -> Publication {
    let record = format!("adversary shared bytes record {}", EventId::mint()).into_bytes();
    let document = format!("adversary shared bytes seed {}", EventId::mint()).into_bytes();
    let record_hash = ContentHash::of_bytes(&record);
    let canonical = |bytes| PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::EPOCH,
        bytes,
    };
    Publication {
        event: RevisionEvent {
            application: None,
            format: RevisionEvent::FORMAT.into(),
            event_id: EventId::mint(),
            record_hash,
            payload: RevisionPayload::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash: ContentHash::of_bytes(&document),
            },
        },
        objects: BTreeMap::from([
            (record_hash, canonical(record)),
            (ContentHash::of_bytes(&document), canonical(document)),
        ]),
        expected_version: 0,
    }
}

fn no_retained_byte_outlives_its_handle<S: RevisionLog + Initialize>(open: impl Fn() -> S) {
    {
        let writer = open();
        assert_eq!(
            writer.initialize(&seed_publication()).unwrap(),
            Appended::Written
        );
    }
    let reader = open();
    let first = reader.history().unwrap();
    let second = reader.history().unwrap();
    assert_eq!(first.objects.len(), 2);
    let kept: Vec<(ContentHash, Weak<Vec<u8>>)> = first
        .objects
        .iter()
        .map(|(hash, object)| (*hash, std::sync::Arc::downgrade(&object.bytes)))
        .collect();
    drop(first);
    assert!(
        kept.iter().all(|(_, bytes)| bytes.upgrade().is_some()),
        "the handle and the later history keep the verified bytes"
    );
    drop(second);
    drop(reader);
    let alive: Vec<ContentHash> = kept
        .iter()
        .filter(|(_, bytes)| bytes.upgrade().is_some())
        .map(|(hash, _)| *hash)
        .collect();
    assert!(
        alive.is_empty(),
        "retained bytes outlived the handle and every history it returned: {alive:?}"
    );
}

fn sqlite(root: &Path) -> SqliteStore {
    SqliteStore::sqlite(&root.join("state.db"), "ekr", None)
        .unwrap()
        .under(Touch)
}

fn file(root: &Path) -> FileStore {
    FileStore::file(&root.join("state"), "ekr", None)
        .unwrap()
        .under(Touch)
}

#[test]
fn no_retained_byte_outlives_its_handle_sqlite() {
    let directory = TempDir::new().unwrap();
    no_retained_byte_outlives_its_handle(|| sqlite(directory.path()));
}

#[test]
fn no_retained_byte_outlives_its_handle_file() {
    let directory = TempDir::new().unwrap();
    no_retained_byte_outlives_its_handle(|| file(directory.path()));
}
