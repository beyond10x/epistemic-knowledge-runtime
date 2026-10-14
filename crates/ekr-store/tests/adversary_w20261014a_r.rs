//! Adversary pass, wave 2026-10-14a unit R (`story:reads-served-from-a-persisted-read-model`), at
//! `264fbbe16`: `RevisionLog::read_model`'s two refusals the unit's own store test never reaches.
//!
//! `ekr_store::ReadModel` documents `checkpoint` as "that checkpoint's bytes, which hash to
//! `checkpoint_hash`", and `RevisionLog::read_model` says a named object is "loaded and verified as
//! a required object is" and that a checkpoint or object that cannot be read hands over no model.
//! The unit's `read_model.rs` writes every checkpoint through the store's own writer and names
//! only undamaged or absent objects, so neither the checkpoint's address check nor a named
//! object's verification is driven. These cases damage the retained bytes on disk, on both
//! providers, and hold the store to its own documentation.
use std::collections::BTreeSet;
use std::path::Path;

use ekr_core::{ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, Initialize, Publication, PublicationObject,
    RetainedHistory, RevisionLog, StorageClass, StoreError,
};

/// Admits every history and requires nothing beyond its records.
struct Admitting;
impl CommitAuthority for Admitting {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }
    fn replay(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        Err(StoreError::Document("not asked for".into()))
    }
    fn verify(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<(), StoreError> {
        Ok(())
    }
}

const RECORD: &[u8] = b"adversary w20261014a-r seed record bytes, unique 5d21";
const DOCUMENT: &[u8] = b"adversary w20261014a-r seed document bytes, unique 0be8";
const CHECKPOINT: &[u8] = b"adversary w20261014a-r checkpoint bytes, unique 66af";

fn seed() -> Publication {
    let canonical = |bytes: &[u8]| PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::EPOCH,
        bytes: bytes.to_vec(),
    };
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id: EventId::mint(),
            record_hash: ContentHash::of_bytes(RECORD),
            payload: RevisionPayload::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash: ContentHash::of_bytes(DOCUMENT),
            },
        },
        objects: [RECORD, DOCUMENT]
            .into_iter()
            .map(|bytes| (ContentHash::of_bytes(bytes), canonical(bytes)))
            .collect(),
        expected_version: 0,
    }
}

/// A fresh handle on the store of `file`'s provider at `path`, under the admitting authority.
fn store(path: &Path, file: bool) -> Box<dyn RevisionLog> {
    if file {
        Box::new(
            ekr_store::FileStore::file(&path.join("files"), "ekr", None)
                .unwrap()
                .under(Admitting),
        )
    } else {
        Box::new(
            ekr_store::SqliteStore::sqlite(&path.join("state.db"), "ekr", None)
                .unwrap()
                .under(Admitting),
        )
    }
}
/// Seeds the store at `path` and retains [`CHECKPOINT`] as the newest checkpoint over its one
/// occurrence.
fn seeded(path: &Path, file: bool) {
    let written = store(path, file);
    assert_eq!(written.initialize(&seed()), Ok(Appended::Written));
    assert_eq!(
        written.write_checkpoint(1, ContentHash::of_bytes(b"binding"), Some(CHECKPOINT)),
        Ok(true)
    );
}
fn provider(file: bool) -> &'static str {
    if file {
        "file"
    } else {
        "sqlite"
    }
}

/// Flips one bit in the middle of every occurrence of `bytes` in every file under `root`.
fn damage(root: &Path, bytes: &[u8]) -> usize {
    fn walk(path: &Path, bytes: &[u8], damaged: &mut usize) {
        if path.is_dir() {
            for entry in std::fs::read_dir(path).unwrap() {
                walk(&entry.unwrap().path(), bytes, damaged);
            }
            return;
        }
        let mut contents = std::fs::read(path).unwrap();
        let mut at = 0;
        let mut changed = false;
        while let Some(found) = contents[at..]
            .windows(bytes.len())
            .position(|window| window == bytes)
        {
            contents[at + found + bytes.len() / 2] ^= 1;
            at += found + bytes.len();
            changed = true;
            *damaged += 1;
        }
        if changed {
            std::fs::write(path, contents).unwrap();
        }
    }
    let mut damaged = 0;
    walk(root, bytes, &mut damaged);
    damaged
}

/// A checkpoint whose retained bytes no longer hash to the address the pointer names is no
/// model, on both providers: the store never hands the kernel bytes `ReadModel` says hash to
/// `checkpoint_hash` and do not.
#[test]
fn a_checkpoint_damaged_on_disk_hands_over_no_model() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        seeded(path, file);
        let none = |_: &[ekr_store::RecordedOccurrence]| BTreeSet::new();
        assert!(
            store(path, file).read_model(&none).unwrap().is_some(),
            "{}: precondition: the undamaged checkpoint is a model",
            provider(file)
        );
        assert!(
            damage(path, CHECKPOINT) >= 1,
            "{}: precondition: the checkpoint is retained as raw bytes",
            provider(file)
        );
        let read = store(path, file).read_model(&none);
        match read {
            Ok(None) => {}
            Ok(Some((_, model))) => panic!(
                "{}: a damaged checkpoint is handed over: hash of bytes {} named {}",
                provider(file),
                ContentHash::of_bytes(&model.checkpoint),
                model.checkpoint_hash
            ),
            Err(error) => panic!(
                "{}: a damaged checkpoint is an error, not the absent model the complete read \
                 answers past: {error}",
                provider(file)
            ),
        }
    }
}

/// A named object whose retained bytes are damaged is no model, on both providers: the store
/// hands over no object it has not verified against its address.
#[test]
fn a_named_object_damaged_on_disk_hands_over_no_model() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        seeded(path, file);
        let record = |occurrences: &[ekr_store::RecordedOccurrence]| {
            BTreeSet::from([occurrences[0].event.record_hash])
        };
        assert!(
            store(path, file).read_model(&record).unwrap().is_some(),
            "{}: precondition: the undamaged record is loaded",
            provider(file)
        );
        assert!(
            damage(path, RECORD) >= 1,
            "{}: precondition: the record is retained as raw bytes",
            provider(file)
        );
        let read = store(path, file).read_model(&record);
        match read {
            Ok(None) => {}
            Ok(Some((history, _))) => panic!(
                "{}: a damaged record is handed over as verified: {:?}",
                provider(file),
                history
                    .objects
                    .values()
                    .map(|object| ContentHash::of_bytes(&object.bytes))
                    .collect::<Vec<_>>()
            ),
            Err(error) => panic!(
                "{}: a damaged named object is an error, not the absent model the complete read \
                 answers past: {error}",
                provider(file)
            ),
        }
    }
}
