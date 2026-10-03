//! `story:seed-envelope-v3-references-payloads`, at the store: an elected publication preparation
//! whose decision stages an evidence payload (a `Provenance` object) is `ekr.publication-
//! preparation/3`. It names the payload by address, length, class and time, and the preparation's
//! own atomic group binds the payload's bytes under the payload's own address, where the later
//! publication binds the same bytes again. No second copy of the bytes is retained, in the
//! preparation or anywhere else, and nothing about the payload is visible as an object until the
//! publication. A decision that stages no payload is elected in `/2`, byte for byte as before.
//!
//! The substitute authority below admits everything: this pins the store's own mechanics, not
//! kernel admission, which `crates/ekr-kernel/tests/seed_envelope_v3.rs` holds with the real one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ekr_core::{ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, FileStore, ObjectStore, Publication,
    PublicationCommandKey, PublicationCommandKind, PublicationObject, PublicationPreparationV1,
    RetainedHistory, RevisionLog, SqliteStore, StagedPublicationObject, StorageClass, StoreError,
};
use eventlog_core::{EventStore, TenantId};

/// Admits every history. A stand-in so the store's own election path runs; it decides nothing.
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

fn noise(len: usize) -> Vec<u8> {
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect()
}

const RECORD: &[u8] = b"seed result record bytes";
const ENVELOPE: &[u8] = b"seed envelope bytes";

fn key() -> PublicationCommandKey {
    PublicationCommandKey {
        answer_id: None,
        kind: PublicationCommandKind::Bootstrap,
        transaction_id: None,
        predecessor_event_id: None,
        predecessor_record_hash: None,
    }
}

/// A bootstrap decision staging a record and an envelope, and `payload` as `Provenance` if given.
fn decision(payload: Option<&[u8]>) -> Publication {
    let object = |class, bytes: &[u8]| PublicationObject {
        storage_class: class,
        stored_at: Timestamp::from_millis(10),
        bytes: bytes.to_vec(),
    };
    let mut objects = BTreeMap::from([
        (
            ContentHash::of_bytes(RECORD),
            object(StorageClass::Canonical, RECORD),
        ),
        (
            ContentHash::of_bytes(ENVELOPE),
            object(StorageClass::Canonical, ENVELOPE),
        ),
    ]);
    if let Some(payload) = payload {
        objects.insert(
            ContentHash::of_bytes(payload),
            object(StorageClass::Provenance, payload),
        );
    }
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id: EventId::mint(),
            record_hash: ContentHash::of_bytes(RECORD),
            payload: RevisionPayload::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash: ContentHash::of_bytes(ENVELOPE),
            },
        },
        objects,
        expected_version: 0,
    }
}

fn store(path: &Path, file: bool) -> Box<dyn Store> {
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
trait Store: RevisionLog + ObjectStore {}
impl<T: RevisionLog + ObjectStore> Store for T {}

/// The provider's own view of one blob binding.
fn blob(path: &Path, file: bool, digest: &str) -> Option<Vec<u8>> {
    let (runtime, provider) = provider(path, file);
    runtime
        .block_on(provider.get_blob(&TenantId::new("ekr").unwrap(), digest))
        .unwrap()
}

fn delete_blob(path: &Path, file: bool, digest: &str) {
    let (runtime, provider) = provider(path, file);
    runtime
        .block_on(provider.delete_blob(&TenantId::new("ekr").unwrap(), digest))
        .unwrap();
}

fn provider(path: &Path, file: bool) -> (tokio::runtime::Runtime, Box<dyn EventStore>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let provider: Box<dyn EventStore> = runtime.block_on(async {
        if file {
            Box::new(eventlog_file::FileEventStore::open(path).await.unwrap())
                as Box<dyn EventStore>
        } else {
            Box::new(
                eventlog_sqlite::SqliteEventStore::open(
                    path.join("state.db").to_str().unwrap(),
                    "ekr",
                )
                .await
                .unwrap(),
            )
        }
    });
    (runtime, provider)
}

fn private_key(prepared: &PublicationPreparationV1) -> String {
    let bytes = serde_json::to_vec(prepared).unwrap();
    format!("ekr.private.preparation.{}", ContentHash::of_bytes(&bytes))
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[test]
fn a_preparation_names_its_evidence_payload_and_stages_the_bytes_once_on_both_providers() {
    let payload = noise(300_000);
    let hash = ContentHash::of_bytes(&payload);
    let window = &payload[3_000..6_072];
    let numbers = window
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",");
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let decision = decision(Some(&payload));
        let input = ContentHash::of_bytes(b"bootstrap input");
        let prepared = {
            let store = store(path, file);
            let prepared = store.prepare(&key(), input, &decision, None).unwrap();
            assert_eq!(
                prepared.format,
                PublicationPreparationV1::FORMAT_V3,
                "file={file}"
            );
            assert_eq!(prepared.format, "ekr.publication-preparation/3");
            assert_eq!(prepared.decision, decision, "file={file}");
            prepared
        };

        // The private record holds no payload bytes in any spelling, and names the payload.
        let record = blob(path, file, &private_key(&prepared)).unwrap();
        assert!(record.len() < 16_384, "file={file}: {} bytes", record.len());
        assert!(!contains(&record, window), "file={file}: raw");
        assert!(
            !contains(&record, numbers.as_bytes()),
            "file={file}: numbers"
        );
        let written: serde_json::Value = serde_json::from_slice(&record).unwrap();
        let named: StagedPublicationObject =
            serde_json::from_value(written["decision"]["objects"][hash.to_hex()].clone()).unwrap();
        assert_eq!(
            named,
            StagedPublicationObject {
                storage_class: StorageClass::Provenance,
                stored_at: Timestamp::from_millis(10),
                byte_len: payload.len() as u64,
            },
            "file={file}"
        );
        // Canonical objects are still carried, as `/2` carries them.
        assert!(
            written["decision"]["objects"][ContentHash::of_bytes(RECORD).to_hex()]["bytes"]
                .is_string()
        );

        // The bytes are bound once, under the payload's own address, and are not yet an object.
        assert_eq!(
            blob(path, file, &hash.to_hex()).as_deref(),
            Some(&payload[..])
        );
        {
            let store = store(path, file);
            assert_eq!(store.get(&hash).unwrap(), None, "file={file}");
            // A handle that did not elect it reads the whole decision back.
            assert_eq!(
                store.preparation(&key()).unwrap().as_ref(),
                Some(&prepared),
                "file={file}"
            );
            assert_eq!(store.resume(&prepared).unwrap(), Appended::Written);
            assert_eq!(store.get(&hash).unwrap().as_deref(), Some(&payload[..]));
        }
        assert_eq!(
            blob(path, file, &hash.to_hex()).as_deref(),
            Some(&payload[..])
        );
    }
}

#[test]
fn a_decision_without_an_evidence_payload_is_still_elected_in_format_two_on_both_providers() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let store = store(directory.path(), file);
        let prepared = store
            .prepare(
                &key(),
                ContentHash::of_bytes(b"input"),
                &decision(None),
                None,
            )
            .unwrap();
        assert_eq!(
            prepared.format,
            PublicationPreparationV1::FORMAT,
            "file={file}"
        );
        assert_eq!(prepared.format, "ekr.publication-preparation/2");
    }
}

#[test]
fn a_preparation_whose_staged_payload_is_gone_is_refused_by_name_on_both_providers() {
    let payload = noise(4_096);
    let hash = ContentHash::of_bytes(&payload);
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        {
            let store = store(path, file);
            store
                .prepare(
                    &key(),
                    ContentHash::of_bytes(b"input"),
                    &decision(Some(&payload)),
                    None,
                )
                .unwrap();
        }
        delete_blob(path, file, &hash.to_hex());
        let store = store(path, file);
        let refused = store.preparation(&key()).unwrap_err();
        assert_eq!(
            refused,
            StoreError::Document(format!("preparation-staged-object-missing: {hash}")),
            "file={file}"
        );
        assert_eq!(store.head(), Ok(None), "file={file}: nothing was published");
    }
}
