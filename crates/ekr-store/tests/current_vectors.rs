//! Fixed current-format vector for § 94's private `ekr.publication-preparation/1` record, taken
//! from the real election path (`RevisionLog::prepare`) on both providers rather than built by
//! hand, together with the `PublicationCommandKey` bytes its private slot is derived from.
//!
//! The substitute authority below admits everything: this pins bytes, not kernel admission,
//! which `crates/ekr-kernel/tests` holds with the real authority.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, CommitAuthority, FileStore, Publication, PublicationCommandKey,
    PublicationCommandKind, PublicationObject, PublicationPreparationV1, RetainedHistory,
    RevisionLog, SqliteStore, StorageClass, StoreError,
};

fn id<T: std::str::FromStr>(n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

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

const RECORD: &[u8] = b"current seed result record bytes";
const ENVELOPE: &[u8] = b"current seed envelope bytes";

fn bootstrap() -> (PublicationCommandKey, ContentHash, Publication) {
    let object = |bytes: &[u8]| PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::from_millis(10),
        bytes: bytes.to_vec(),
    };
    let decision = Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id: id::<EventId>(0x40),
            record_hash: ContentHash::of_bytes(RECORD),
            payload: RevisionPayload::Seeded {
                revision_id: id::<RevisionId>(0x31),
                seed_hash: ContentHash::of_bytes(ENVELOPE),
            },
        },
        objects: BTreeMap::from([
            (ContentHash::of_bytes(RECORD), object(RECORD)),
            (ContentHash::of_bytes(ENVELOPE), object(ENVELOPE)),
        ]),
        expected_version: 0,
    };
    let key = PublicationCommandKey {
        kind: PublicationCommandKind::Bootstrap,
        transaction_id: None,
        predecessor_event_id: None,
        predecessor_record_hash: None,
    };
    (
        key,
        ContentHash::of_bytes(b"current bootstrap input"),
        decision,
    )
}

fn prepared(file: bool) -> PublicationPreparationV1 {
    let directory = tempfile::tempdir().unwrap();
    let (key, input, decision) = bootstrap();
    let store: Box<dyn RevisionLog> = if file {
        Box::new(
            FileStore::file(directory.path(), "ekr", None)
                .unwrap()
                .under(AdmitAll),
        )
    } else {
        Box::new(
            SqliteStore::sqlite(&directory.path().join("state.db"), "ekr", None)
                .unwrap()
                .under(AdmitAll),
        )
    };
    let prepared = store.prepare(&key, input, &decision, None).unwrap();
    assert_eq!(store.preparation(&key).unwrap().as_ref(), Some(&prepared));
    prepared
}

#[derive(Default)]
struct Pins(Vec<String>);
impl Pins {
    fn check(&mut self, name: &str, actual: impl Into<String>, expected: &str) {
        let actual = actual.into();
        if actual != expected {
            self.0.push(format!("{name}: actual {actual}"));
        }
    }
    fn finish(self) {
        assert!(self.0.is_empty(), "\n{}", self.0.join("\n"));
    }
}

#[test]
fn an_elected_bootstrap_preparation_has_fixed_bytes_on_both_providers() {
    let mut pins = Pins::default();
    for file in [false, true] {
        let provider = if file { "file" } else { "sqlite" };
        let prepared = prepared(file);
        let (key, input, decision) = bootstrap();
        assert_eq!(prepared.format, PublicationPreparationV1::FORMAT);
        assert_eq!(prepared.command_key, key);
        assert_eq!(prepared.input_hash, input);
        assert_eq!(prepared.decision, decision);
        assert_eq!(prepared.attempt_number, 0);
        assert_eq!(prepared.previous_attempt_hash, None);
        let bytes = serde_json::to_vec(&prepared).unwrap();
        let decoded: PublicationPreparationV1 = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, prepared);
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
        pins.check(
            &format!("{provider} preparation address"),
            ContentHash::of_bytes(&bytes).to_hex(),
            "e2710a346d063cf261004ff0fb4ef98b244c14209b1b923a1e1b3264ce01dd99",
        );
        pins.check(
            &format!("{provider} native fingerprint"),
            prepared.native_fingerprint.clone(),
            "c95bcc9eac4be450affd7a3400234b36a9047837654f4fd0e4c910246fcc85e9",
        );
        pins.check(
            &format!("{provider} native revision event"),
            String::from_utf8(prepared.native_request.appends[0].events[0].data.clone()).unwrap(),
            r#"{"event_id":"00000000-0000-4000-8000-000000000040","format":"ekr.revision-event/2","payload":{"event":"Seeded","revision_id":"00000000-0000-4000-8000-000000000031","seed_hash":"34f15c06110a5f20464e8c56470f7aef3987b674f3e70cf6c3daf38d5007261b"},"record_hash":"7da2b7b57032106fec457364d3c7b8d52e28143f46a3cb13c34cc7166f1da308"}"#,
        );
    }
    pins.finish();
}

#[test]
fn the_command_key_has_fixed_slot_bytes() {
    let (key, _, _) = bootstrap();
    let bytes = serde_json::to_vec(&key).unwrap();
    let mut pins = Pins::default();
    pins.check("key bytes", String::from_utf8(bytes.clone()).unwrap(), r#"{"kind":"Bootstrap","transaction_id":null,"predecessor_event_id":null,"predecessor_record_hash":null}"#);
    pins.check(
        "key slot",
        ContentHash::of_bytes(&bytes).to_hex(),
        "1f065563ff037ae605a438215b0b0bbf06dda20192fc543426629b8c854596bb",
    );
    pins.finish();
}
