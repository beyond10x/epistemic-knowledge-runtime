//! `story:seed-envelope-v3-references-payloads`: the seed envelope names each evidence payload by
//! its content hash (`ekr-seed-envelope/3`) instead of carrying it as a JSON number array, and the
//! publication preparation holds no payload bytes either. A retained `/2` envelope still replays.
//!
//! The `/2` stores are built at test time by `support/v2_store.rs`, which seeds exactly as the
//! kernel at base `36e87d41` (release 0.0.14, whose `/2` envelope layout is the one 0.0.11 wrote)
//! did and then records every transaction state; `current_vectors.rs` holds its envelope to the
//! address that base kernel produced.
mod current_fixture;
#[allow(dead_code)]
#[path = "support/v2_store.rs"]
mod v2;

use std::collections::BTreeSet;
use std::path::Path;

use current_fixture::{anchor, context, id, open, seed, seeded, SEEDED_AT};
use ekr_core::{ContentHash, EventId, RevisionId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{Confidence, Evidence, EvidenceSource, RevisionEvent, RevisionPayload};
use ekr_kernel::{Runtime, SeedDocument, SeedResultV1, TransactionState};
use ekr_store::{
    AdmittedRevision, CommitAuthority, FileStore, Initialize, Publication, PublicationObject,
    RetainedHistory, RevisionLog, SqliteStore, StorageClass, StoreError,
};

/// The `/2` envelope address the writer's seed retains: the value `current_vectors.rs` pinned
/// for a real seed of `current_fixture::seed()` before this story.
const V2_SEED_HASH: &str = "d9fc8c0fe513570feebbfd97837e79ee8e7636b657ee7b6974a9c5e892d51dde";

/// The head root of that store: the address the store the base kernel itself wrote reached,
/// measured on `36e87d41` before this story.
const V2_HEAD_ROOT: &str = "b8fb29f3abd83a10dce6ed8c005e03981c9ee9a67c84ebb95e66fff5b2964ad5";

/// A `/2` store of `current_fixture::seed()` holding every transaction state, written as the base
/// kernel wrote it (`support/v2_store.rs`).
fn v2_store(file: bool) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    v2::fixture_store(
        directory.path(),
        file,
        &current_fixture::seed(),
        context(),
        &anchor(),
        current_fixture::SEEDED_AT,
    );
    directory
}

fn existing(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file_existing(path, "ekr", context(), anchor())
    } else {
        Runtime::sqlite_existing(&path.join("state.db"), "ekr", context(), anchor())
    }
    .unwrap()
}

fn fresh(path: &Path, file: bool) -> Runtime {
    open(path, file, context(), anchor())
}

/// Deterministic incompressible bytes, so a copy of them in any spelling is large and findable.
fn noise(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect()
}

/// The current fixture's seed with four more evidence entries, 2.5 MiB of payload each — 10 MiB
/// in all — every one cited by the seed's assertion.
fn heavy_seed() -> (SeedDocument, Vec<Vec<u8>>) {
    let mut document = seed();
    let assertion = *document.graph.assertions.keys().next().unwrap();
    let mut payloads = Vec::new();
    for n in 0..4_u64 {
        let payload = noise(n + 1, 2_621_440);
        let evidence = Evidence {
            id: id(0x40 + n),
            source: EvidenceSource::HumanStatement {
                identity: Some("operator".into()),
            },
            content_hash: ContentHash::of_bytes(&payload),
            extracted_by: context().operator,
            observed_at: Timestamp::EPOCH,
            confidence: Confidence::CERTAIN,
        };
        document
            .graph
            .assertions
            .get_mut(&assertion)
            .unwrap()
            .evidence
            .insert(evidence.id);
        document.graph.evidence.insert(evidence.id, evidence);
        document
            .evidence_payloads
            .insert(ContentHash::of_bytes(&payload), payload.clone());
        payloads.push(payload);
    }
    (document, payloads)
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let triple = chunk.iter().enumerate().fold(0_u32, |acc, (at, byte)| {
            acc | (u32::from(*byte) << (16 - 8 * at))
        });
        for position in 0..4 {
            if position <= chunk.len() {
                out.push(char::from(
                    ALPHABET[((triple >> (18 - 6 * position)) & 0x3f) as usize],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// A 3 KiB window of `payload` as raw bytes, as the base64 an encoder of the whole payload writes
/// for it, and as the comma-separated decimals a JSON number array writes for it.
fn spellings(payload: &[u8]) -> [(&'static str, Vec<u8>); 3] {
    let window = &payload[3_000..3_000 + 3_072];
    [
        ("raw", window.to_vec()),
        ("base64", base64(window).into_bytes()),
        (
            "numbers",
            window
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",")
                .into_bytes(),
        ),
    ]
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Every retained byte sequence of the store with where it came from: for the file provider every
/// file under the store directory; for SQLite every event body in the provider feed and every blob
/// the feed names (object, preparation and checkpoint bindings).
fn retained(path: &Path, file: bool) -> Vec<(String, Vec<u8>)> {
    if file {
        let mut found = Vec::new();
        fn visit(directory: &Path, found: &mut Vec<(String, Vec<u8>)>) {
            for entry in std::fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    visit(&path, found);
                } else {
                    found.push((path.display().to_string(), std::fs::read(&path).unwrap()));
                }
            }
        }
        visit(path, &mut found);
        return found;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        use eventlog_core::EventStore;
        let provider =
            eventlog_sqlite::SqliteEventStore::open(path.join("state.db").to_str().unwrap(), "ekr")
                .await
                .unwrap();
        let tenant = eventlog_core::TenantId::new("ekr").unwrap();
        let mut found = Vec::new();
        let mut digests = BTreeSet::new();
        let mut after = 0;
        loop {
            let page = provider.read_feed(&tenant, after, 100).await.unwrap();
            for event in page.events {
                let data = &event.data;
                match event.name.as_str() {
                    "ekr.store.ObjectStored" => {
                        digests.insert(data["content_hash"].as_str().unwrap().to_owned());
                    }
                    "ekr.store.PublicationPrepared" => {
                        digests.insert(format!(
                            "ekr.private.preparation.{}",
                            data["preparation_hash"].as_str().unwrap()
                        ));
                    }
                    "ekr.store.CheckpointWritten" => {
                        digests.insert(format!(
                            "ekr.private.checkpoint.{}",
                            data["checkpoint_hash"].as_str().unwrap()
                        ));
                    }
                    _ => {}
                }
                found.push((
                    format!("event {} {}", event.name, event.event_id),
                    serde_json::to_vec(&event.data).unwrap(),
                ));
            }
            if !page.has_more {
                break;
            }
            after = page.next_position;
        }
        for digest in digests {
            if let Some(bytes) = provider.get_blob(&tenant, &digest).await.unwrap() {
                found.push((format!("blob {digest}"), bytes));
            }
        }
        found
    })
}

#[test]
fn a_seed_with_ten_megabytes_of_evidence_retains_an_envelope_under_one_megabyte_on_both_providers()
{
    for file in [false, true] {
        let (document, payloads) = heavy_seed();
        let total: usize = payloads.iter().map(Vec::len).sum();
        assert!(total >= 10 * 1024 * 1024, "{total}");
        let (_directory, runtime, result) = seeded(document, context(), anchor(), file, SEEDED_AT);
        let envelope = runtime.content(&result.seed_hash).unwrap().unwrap();
        let value: serde_json::Value = serde_json::from_slice(&envelope).unwrap();
        assert_eq!(value["format"], "ekr-seed-envelope/3", "file={file}");
        assert!(
            envelope.len() < 1_000_000,
            "file={file}: the envelope blob is {} bytes",
            envelope.len()
        );
        // Every payload is still retained under its own address, byte for byte.
        for payload in &payloads {
            assert_eq!(
                runtime
                    .content(&ContentHash::of_bytes(payload))
                    .unwrap()
                    .as_deref(),
                Some(payload.as_slice()),
                "file={file}"
            );
        }
    }
}

#[test]
fn no_retained_blob_or_event_but_the_payload_blobs_holds_the_payload_bytes_on_both_providers() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let runtime = fresh(directory.path(), file);
        let (document, payloads) = heavy_seed();
        runtime.seed(document, || SEEDED_AT).unwrap();
        drop(runtime);
        let retained = retained(directory.path(), file);
        assert!(
            retained.len() > 3,
            "file={file}: {} retained",
            retained.len()
        );
        for (at, bytes) in &retained {
            if payloads.iter().any(|payload| payload == bytes) {
                continue;
            }
            for (n, payload) in payloads.iter().enumerate() {
                for (spelling, needle) in spellings(payload) {
                    if contains(bytes, &needle) {
                        wrong.push(format!(
                            "file={file}: {at} ({} bytes) holds payload {n} as {spelling}",
                            bytes.len()
                        ));
                    }
                }
            }
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

fn tx(n: u64) -> TransactionId {
    id(n)
}

#[test]
fn a_v2_store_written_before_this_story_reopens_and_replays_with_identical_roots_on_both_providers()
{
    for file in [false, true] {
        let directory = v2_store(file);
        let runtime = existing(directory.path(), file);
        let head = runtime.head().unwrap().unwrap();
        assert_eq!(head.revision, RevisionNumber::new(2), "file={file}");
        assert_eq!(ContentHash::of(&head).to_hex(), V2_HEAD_ROOT, "file={file}");
        let read = runtime.read(None).unwrap();
        assert_eq!(read.root, head);
        assert_eq!(read.seed.seed_hash.to_hex(), V2_SEED_HASH, "file={file}");
        let envelope = runtime.content(&read.seed.seed_hash).unwrap().unwrap();
        assert!(
            envelope.starts_with(br#"{"format":"ekr-seed-envelope/2","#),
            "file={file}"
        );
        assert_eq!(read.seed_input, seed(), "file={file}");
        let states: Vec<(TransactionId, TransactionState)> = read
            .transactions
            .iter()
            .map(|(id, record)| (*id, record.state()))
            .collect();
        assert_eq!(
            states,
            [
                (tx(0x601), TransactionState::Committed),
                (tx(0x602), TransactionState::Stale),
                (tx(0x603), TransactionState::Rejected),
                (tx(0x604), TransactionState::Proposed),
                (tx(0x605), TransactionState::Committed),
            ],
            "file={file}"
        );
        // Every revision replays to the root it was committed with, from the seed.
        let mut full = existing(directory.path(), file);
        full.set_full_replay(true);
        assert_eq!(full.head().unwrap(), Some(head), "file={file}");
        for number in 0..=2 {
            let revision = RevisionNumber::new(number);
            assert_eq!(
                full.replay(revision).unwrap(),
                runtime.replay(revision).unwrap(),
                "file={file} revision {number}"
            );
            assert_eq!(read.revisions[&revision].root.revision, revision);
        }
        // The /2 store stays writable: a further transaction commits against its head.
        drop((runtime, full));
        let runtime = existing(directory.path(), file);
        let operator = context().operator;
        let document = format!(
            "format: ekr.transaction-document/2\ntransaction:\n  id: {}\n  proposer: {}\n  \
             operations:\n  - !CreateNode\n    id: {}\n    root_id: {}\n    type_id: {}\n    \
             canonical_name: seventh\n    properties: {{}}\n  evidence: []\n",
            tx(0x606),
            operator,
            id::<ekr_core::NodeId>(0x24),
            id::<ekr_core::GraphRootId>(0x02),
            id::<ekr_core::TypeId>(0x05),
        );
        runtime
            .propose(document.as_bytes(), operator, || Timestamp::from_millis(40))
            .unwrap();
        runtime
            .validate(tx(0x606), RevisionNumber::new(2), || {
                Timestamp::from_millis(41)
            })
            .unwrap();
        runtime
            .commit(tx(0x606), operator, || Timestamp::from_millis(42))
            .unwrap();
        assert_eq!(
            runtime.head().unwrap().unwrap().revision,
            RevisionNumber::new(3),
            "file={file}"
        );
    }
}

/// Admits every history: a stand-in that lets a test write a store the kernel then reads.
struct AdmitAll;
impl CommitAuthority for AdmitAll {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }
    fn replay(
        &self,
        _: &RetainedHistory,
        _: Option<&ekr_ontology::Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        Ok(None)
    }
}

/// A `/3` seed publication for `current_fixture::seed()` whose envelope names the statement's
/// payload and whose objects omit it: the envelope, and a seed result the kernel would compute for
/// it, derived from the base kernel's `/2` seed of the same input.
fn unbacked_v3_seed() -> (Publication, ContentHash) {
    let directory = v2_store(true);
    let runtime = existing(directory.path(), true);
    let read = runtime.read(Some(RevisionNumber::SEED)).unwrap();
    let v2 = runtime.content(&read.seed.seed_hash).unwrap().unwrap();
    let mut envelope: serde_json::Value = serde_json::from_slice(&v2).unwrap();
    envelope["format"] = "ekr-seed-envelope/3".into();
    let payloads = envelope["input"]["evidence_payloads"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(payloads.len(), 1);
    envelope["input"]["evidence_payloads"] = serde_json::json!(payloads);
    let bytes = serde_json::to_vec(&envelope).unwrap();
    let seed_hash = ContentHash::of_bytes(&bytes);
    let mut root = read.seed.result;
    root.transaction = seed_hash;
    let record = SeedResultV1 {
        seed_hash,
        result: root,
        result_hash: ContentHash::of(&root),
        event_id: EventId::mint(),
        revision_id: RevisionId::mint(),
        ..read.seed
    };
    let record_bytes = record.to_bytes().unwrap();
    let record_hash = ContentHash::of_bytes(&record_bytes);
    let object = |bytes: Vec<u8>| PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: SEEDED_AT,
        bytes,
    };
    let publication = Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id: record.event_id,
            record_hash,
            payload: RevisionPayload::Seeded {
                revision_id: record.revision_id,
                seed_hash,
            },
        },
        objects: [
            (seed_hash, object(bytes)),
            (record_hash, object(record_bytes)),
        ]
        .into_iter()
        .collect(),
        expected_version: 0,
    };
    (publication, payloads[0].parse().unwrap())
}

#[test]
fn a_v3_envelope_naming_a_payload_the_store_does_not_hold_is_refused_by_name_on_both_providers() {
    let (publication, absent) = unbacked_v3_seed();
    let name = format!("seed-evidence-payload-absent: {absent}");
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        // Written past the kernel, as a damaged or hand-assembled store would be.
        let _written = if file {
            FileStore::file(path, "ekr", None)
                .unwrap()
                .under(AdmitAll)
                .initialize(&publication)
                .unwrap()
        } else {
            SqliteStore::sqlite(&path.join("state.db"), "ekr", None)
                .unwrap()
                .under(AdmitAll)
                .initialize(&publication)
                .unwrap()
        };
        let runtime = existing(path, file);
        for (read, outcome) in [
            ("head", runtime.head().map(|_| ())),
            ("snapshot", runtime.snapshot().map(|_| ())),
        ] {
            match outcome {
                Err(error) if error.to_string().contains(&name) => {}
                other => wrong.push(format!("file={file} {read}: {other:?}")),
            }
        }
        // And the kernel refuses to publish such a seed at all.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let mut authority = None;
        let _kernel = ekr_kernel::Commit::over_with_authority(context(), anchor(), |kernel| {
            authority = Some(kernel.clone());
            FileStore::file(&path.join("unused"), "ekr", None).map(|store| store.under(kernel))
        })
        .unwrap();
        let authority = authority.unwrap();
        let outcome = if file {
            let store = FileStore::file(path, "ekr", None).unwrap().under(authority);
            store.initialize(&publication).map(|_| store.head())
        } else {
            let store = SqliteStore::sqlite(&path.join("state.db"), "ekr", None)
                .unwrap()
                .under(authority);
            store.initialize(&publication).map(|_| store.head())
        };
        match outcome {
            Err(error) if error.to_string().contains(&name) => {}
            other => wrong.push(format!("file={file} initialize: {other:?}")),
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
