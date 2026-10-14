//! `RevisionLog::read_model`, design § 108: what the store hands the kernel for a one-shot read.
//!
//! The store reads the revision stream, the newest checkpoint pointer, the checkpoint it names and
//! the objects the kernel names, and nothing else; it judges none of it. It hands over nothing
//! where it keeps no checkpoint for the handle: no pointer, a handle opened for full replay, a
//! store with no authority to answer to, or a checkpoint whose bytes are not the ones the pointer
//! names.
use std::collections::BTreeSet;

use ekr_core::{ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, EventlogStore, Initialize, Publication,
    PublicationObject, ReadModel, RetainedHistory, RevisionLog, StorageClass, StoreError,
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

const RECORD: &[u8] = b"seed record";
const DOCUMENT: &[u8] = b"seed document";

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

/// Both providers' stores at `directory`, seeded, under the admitting authority.
fn stores(directory: &std::path::Path) -> Vec<(&'static str, Box<dyn RevisionLog>)> {
    let sqlite = ekr_store::SqliteStore::sqlite(&directory.join("state.db"), "ekr", None)
        .unwrap()
        .under(Admitting);
    assert_eq!(sqlite.initialize(&seed()), Ok(Appended::Written));
    let file = ekr_store::FileStore::file(&directory.join("files"), "ekr", None)
        .unwrap()
        .under(Admitting);
    assert_eq!(file.initialize(&seed()), Ok(Appended::Written));
    vec![("sqlite", Box::new(sqlite)), ("file", Box::new(file))]
}

fn reopen<S>(store: EventlogStore<S>, full: bool) -> EventlogStore<S>
where
    S: eventlog_core::AtomicBlobEventStore,
{
    let mut store = store.under(Admitting);
    store.set_full_replay(full);
    store
}

#[test]
fn a_read_model_is_the_stream_the_pointer_its_checkpoint_and_only_the_named_objects() {
    let directory = tempfile::tempdir().unwrap();
    for (provider, store) in stores(directory.path()) {
        let none = |_: &[ekr_store::RecordedOccurrence]| BTreeSet::new();
        assert_eq!(
            store.read_model(&none).unwrap(),
            None,
            "{provider}: no pointer, no model"
        );
        let binding = ContentHash::of_bytes(b"binding");
        assert_eq!(
            store.write_checkpoint(1, binding, Some(b"a checkpoint")),
            Ok(true)
        );
        let _ = ekr_store::objects_loaded();
        let (history, model) = store
            .read_model(&|occurrences| {
                assert_eq!(occurrences.len(), 1, "{provider}: the stream is read whole");
                BTreeSet::from([occurrences[0].event.record_hash])
            })
            .unwrap()
            .expect("a pointer and its checkpoint are a model");
        assert_eq!(
            model,
            ReadModel {
                covered: 1,
                binding,
                checkpoint_hash: ContentHash::of_bytes(b"a checkpoint"),
                checkpoint: b"a checkpoint".to_vec(),
            },
            "{provider}"
        );
        assert_eq!(history.occurrences.len(), 1, "{provider}");
        assert_eq!(
            history.objects.keys().copied().collect::<Vec<_>>(),
            vec![ContentHash::of_bytes(RECORD)],
            "{provider}: only the named object is loaded"
        );
        assert_eq!(
            ekr_store::objects_loaded(),
            1,
            "{provider}: one object loaded"
        );
        let (unnamed, _) = store.read_model(&none).unwrap().unwrap();
        assert!(
            unnamed.objects.is_empty(),
            "{provider}: nothing named, nothing loaded"
        );
        // A named object the store does not hold is no model: the complete read decides.
        let absent = |_: &[ekr_store::RecordedOccurrence]| {
            BTreeSet::from([ContentHash::of_bytes(b"never stored")])
        };
        assert_eq!(store.read_model(&absent).unwrap(), None, "{provider}");
    }
}

#[test]
fn a_handle_that_keeps_no_checkpoint_hands_over_no_model() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    let _ = stores(path);
    let binding = ContentHash::of_bytes(b"binding");
    let none = |_: &[ekr_store::RecordedOccurrence]| BTreeSet::new();
    let sqlite = || ekr_store::SqliteStore::sqlite(&path.join("state.db"), "ekr", None).unwrap();
    let file = || ekr_store::FileStore::file(&path.join("files"), "ekr", None).unwrap();
    assert_eq!(
        reopen(sqlite(), false).write_checkpoint(1, binding, Some(b"one")),
        Ok(true)
    );
    assert_eq!(
        reopen(file(), false).write_checkpoint(1, binding, Some(b"one")),
        Ok(true)
    );
    assert!(reopen(sqlite(), false).read_model(&none).unwrap().is_some());
    assert!(reopen(file(), false).read_model(&none).unwrap().is_some());
    assert_eq!(
        reopen(sqlite(), true).read_model(&none).unwrap(),
        None,
        "sqlite: full replay reads no model"
    );
    assert_eq!(
        reopen(file(), true).read_model(&none).unwrap(),
        None,
        "file: full replay reads no model"
    );
    assert_eq!(
        sqlite().read_model(&none).unwrap(),
        None,
        "sqlite: a store with no authority hands over no model"
    );
    assert_eq!(
        file().read_model(&none).unwrap(),
        None,
        "file: a store with no authority hands over no model"
    );
}

/// `RevisionLog::load_required`, design § 108.2 item 7: once the kernel has read from the model
/// which evidence payloads a complete read of the revision requires, the store loads exactly
/// those into the model's history, each verified as a required object is, and refuses one it
/// cannot load rather than handing the history over without it.
#[test]
fn objects_named_after_the_model_is_read_are_loaded_verified_and_an_absent_one_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    for (provider, store) in stores(directory.path()) {
        let binding = ContentHash::of_bytes(b"binding");
        assert_eq!(
            store.write_checkpoint(1, binding, Some(b"a checkpoint")),
            Ok(true)
        );
        let none = |_: &[ekr_store::RecordedOccurrence]| BTreeSet::new();
        let (mut history, _) = store.read_model(&none).unwrap().unwrap();
        let _ = ekr_store::objects_loaded();
        let document = ContentHash::of_bytes(DOCUMENT);
        store
            .load_required(&mut history, BTreeSet::from([document]))
            .unwrap();
        assert_eq!(
            history.objects.keys().copied().collect::<Vec<_>>(),
            vec![document],
            "{provider}: only the named object is added"
        );
        assert_eq!(
            history.content(document, StorageClass::Canonical),
            Ok(DOCUMENT),
            "{provider}: the object is the verified bytes"
        );
        assert_eq!(ekr_store::objects_loaded(), 1, "{provider}: one load");
        store
            .load_required(&mut history, BTreeSet::from([document]))
            .unwrap();
        assert_eq!(
            ekr_store::objects_loaded(),
            0,
            "{provider}: an object the history holds is not loaded again"
        );
        let absent = ContentHash::of_bytes(b"never stored");
        assert!(
            store
                .load_required(&mut history, BTreeSet::from([absent]))
                .is_err(),
            "{provider}: an object the store does not hold is refused"
        );
        assert!(
            !history.objects.contains_key(&absent),
            "{provider}: nothing is added for a refused object"
        );
    }
}
