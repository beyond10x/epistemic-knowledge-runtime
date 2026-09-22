//! Occurrence identity (§ 89): "Reusing an identity for different content refuses." Publishing an
//! already retained `event_id` with any changed content is refused with
//! `occurrence-identity-conflict` on both providers, writes nothing, and leaves the retained
//! history exactly as it was; the exact retry of the same occurrence is `AlreadyRecorded`.
//!
//! The substitute authority admits every history, so what is measured is the store's own
//! identity check (`crates/ekr-store/src/eventlog.rs`, `publish`), not kernel admission.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use ekr_core::{AgentId, ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, FileStore, Initialize, ObjectStore, Publication,
    PublicationObject, RetainedHistory, RevisionLog, SqliteStore, StorageClass, StoreError,
};

fn id<T: std::str::FromStr>(n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

struct AdmitAll;
impl CommitAuthority for AdmitAll {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }
    fn replay(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        Ok(None)
    }
}

trait Store: RevisionLog + ObjectStore + Initialize {}
impl<S: RevisionLog + ObjectStore + Initialize> Store for S {}

fn open(path: &Path, file: bool) -> Box<dyn Store> {
    if file {
        Box::new(FileStore::file(path, "ekr", None).unwrap().under(AdmitAll))
    } else {
        Box::new(
            SqliteStore::sqlite(&path.join("state.db"), "ekr", None)
                .unwrap()
                .under(AdmitAll),
        )
    }
}

fn publication(event_id: u64, payload: RevisionPayload, record: &[u8], at: u64) -> Publication {
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id: id::<EventId>(event_id),
            record_hash: ContentHash::of_bytes(record),
            payload,
        },
        objects: BTreeMap::from([(
            ContentHash::of_bytes(record),
            PublicationObject {
                storage_class: StorageClass::Canonical,
                stored_at: Timestamp::EPOCH,
                bytes: record.to_vec(),
            },
        )]),
        expected_version: at,
    }
}

fn seed() -> Publication {
    let mut seed = publication(
        0x40,
        RevisionPayload::Seeded {
            revision_id: id::<RevisionId>(0x31),
            seed_hash: ContentHash::of_bytes(b"identity seed envelope"),
        },
        b"identity seed record",
        0,
    );
    seed.objects.insert(
        ContentHash::of_bytes(b"identity seed envelope"),
        PublicationObject {
            storage_class: StorageClass::Canonical,
            stored_at: Timestamp::EPOCH,
            bytes: b"identity seed envelope".to_vec(),
        },
    );
    seed
}

fn proposed(proposer: u64, record: &[u8], at: u64) -> Publication {
    publication(
        0x41,
        RevisionPayload::TransactionProposed {
            transaction_id: id(0x51),
            proposer: id::<AgentId>(proposer),
            operations_hash: None,
        },
        record,
        at,
    )
}

fn bytes_under(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, into: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, into);
            } else {
                into.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut into = BTreeMap::new();
    visit(root, root, &mut into);
    into
}

#[test]
fn a_retained_event_id_with_changed_content_is_refused_on_both_providers() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let store = open(directory.path(), file);
        assert_eq!(store.initialize(&seed()).unwrap(), Appended::Written);
        let original = proposed(0x03, b"identity proposal record", 1);
        assert_eq!(store.publish(&original).unwrap(), Appended::Written);
        let retained = store.history().unwrap();
        assert_eq!(retained.occurrences.len(), 2);
        let files = file.then(|| bytes_under(directory.path()));

        let mut rejected = original.clone();
        rejected.event.payload = RevisionPayload::TransactionRejected {
            transaction_id: id(0x51),
            issues: 1,
        };
        let mut reformatted = original.clone();
        reformatted.event.format = "ekr.revision-event/3".into();
        let changes = [
            (
                "changed payload field",
                proposed(0x09, b"identity proposal record", 2),
            ),
            ("changed record", proposed(0x03, b"a different record", 2)),
            (
                "changed record at the original position",
                proposed(0x03, b"a different record", 1),
            ),
            ("changed payload kind", rejected),
            ("changed format", reformatted),
        ];
        for (name, changed) in changes {
            let outcome = store.publish(&changed);
            if !matches!(&outcome, Err(StoreError::Document(code)) if code == "occurrence-identity-conflict")
            {
                wrong.push(format!("file={file} {name}: {outcome:?}"));
            }
            for hash in changed.objects.keys() {
                if !original.objects.contains_key(hash) && store.get(hash).unwrap().is_some() {
                    wrong.push(format!("file={file} {name}: wrote {hash}"));
                }
            }
        }
        assert_eq!(
            store.publish(&original).unwrap(),
            Appended::AlreadyRecorded,
            "file={file}: the exact retry is the same occurrence"
        );
        assert_eq!(store.history().unwrap(), retained, "file={file}");
        if let Some(files) = files {
            assert_eq!(bytes_under(directory.path()), files, "file provider bytes");
        }
        drop(store);
        assert_eq!(
            open(directory.path(), file).history().unwrap(),
            retained,
            "file={file}: a fresh open reads the same history"
        );
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
