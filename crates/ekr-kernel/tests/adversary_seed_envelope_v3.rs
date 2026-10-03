//! Adversary pass, wave ingest-02 unit V (`story:seed-envelope-v3-references-payloads`,
//! `task:object-payloads-belong-in-provider-blobs`): design § 100 driven from its own text.
//!
//! Every store here is written at test time, on both providers: `/3` stores by the real kernel,
//! `/2` stores by `support/v2_store.rs`.
#[allow(dead_code)]
mod current_fixture;
#[allow(dead_code)]
#[path = "support/v2_store.rs"]
mod v2;

use std::cell::Cell;
use std::collections::BTreeSet;
use std::path::Path;

use current_fixture::{anchor, context, id, open, seed, seeded, SEEDED_AT, STATEMENT};
use ekr_core::{ContentHash, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{Confidence, Evidence, EvidenceSource};
use ekr_kernel::{Commit, CommitCommandResult, Runtime, SeedDocument, ValidationCommandResult};
use ekr_store::{
    FileStore, Initialize, Inventory, ObjectStore, Publication, PublicationCommandKey,
    PublicationPreparationV1, RevisionLog, SqliteStore, StorageClass, StoreError, StoreInventory,
};

const TENANT: &str = "ekr";

fn existing(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file_existing(path, TENANT, context(), anchor())
    } else {
        Runtime::sqlite_existing(&path.join("state.db"), TENANT, context(), anchor())
    }
    .unwrap()
}

fn provider(
    path: &Path,
    file: bool,
) -> (tokio::runtime::Runtime, Box<dyn eventlog_core::EventStore>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let provider: Box<dyn eventlog_core::EventStore> = runtime.block_on(async {
        if file {
            Box::new(eventlog_file::FileEventStore::open(path).await.unwrap())
                as Box<dyn eventlog_core::EventStore>
        } else {
            Box::new(
                eventlog_sqlite::SqliteEventStore::open(
                    path.join("state.db").to_str().unwrap(),
                    TENANT,
                )
                .await
                .unwrap(),
            )
        }
    });
    (runtime, provider)
}

fn tenant() -> eventlog_core::TenantId {
    eventlog_core::TenantId::new(TENANT).unwrap()
}

/// Deterministic incompressible bytes.
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

/// `seed()` with one more evidence entry, citing `payload`, cited by the seed's assertion.
fn seed_with(payload: &[u8], evidence_id: u64) -> SeedDocument {
    let mut document = seed();
    let assertion = *document.graph.assertions.keys().next().unwrap();
    let evidence = Evidence {
        id: id(evidence_id),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: ContentHash::of_bytes(payload),
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
        .insert(ContentHash::of_bytes(payload), payload.to_vec().into());
    document
}

/// `seed()` without its assertion, evidence or payload.
fn seed_without_evidence() -> SeedDocument {
    let mut document = seed();
    document.graph.assertions.clear();
    document.graph.evidence.clear();
    document.evidence_payloads.clear();
    document
}

fn envelope_json(runtime: &Runtime) -> serde_json::Value {
    let read = runtime.read(None).unwrap();
    serde_json::from_slice(&runtime.content(&read.seed.seed_hash).unwrap().unwrap()).unwrap()
}

fn object_streams(runtime: &Runtime, hash: ContentHash) -> usize {
    runtime
        .published_events()
        .unwrap()
        .iter()
        .filter(|event| event.name == "ekr.store.ObjectStored" && event.stream_id == hash.to_hex())
        .count()
}

// --- tampered and deleted payload blobs --------------------------------------------------------

/// Every read a `/3` store answers, on a fresh handle, both with and without a full replay.
fn every_read_refuses(path: &Path, file: bool, what: &str) {
    for full in [false, true] {
        let mut runtime = existing(path, file);
        runtime.set_full_replay(full);
        let read = runtime.read(None);
        assert!(
            read.is_err(),
            "file={file} full={full}: {what}: read(None) answered"
        );
        assert!(
            runtime.snapshot().is_err(),
            "file={file} full={full}: {what}: snapshot answered"
        );
        assert!(
            runtime.replay(RevisionNumber::SEED).is_err(),
            "file={file} full={full}: {what}: replay(0) answered"
        );
        assert!(
            runtime.transactions().is_err(),
            "file={file} full={full}: {what}: transactions answered"
        );
    }
}

#[test]
fn a_named_payload_blob_replaced_by_other_bytes_is_refused_by_every_read_on_both_providers() {
    for file in [false, true] {
        let (directory, runtime, _) = seeded(seed(), context(), anchor(), file, SEEDED_AT);
        drop(runtime);
        let hash = ContentHash::of_bytes(STATEMENT);
        let forged = b"a forged statement standing where the seed's payload was".to_vec();
        {
            let (executor, provider) = provider(directory.path(), file);
            executor
                .block_on(provider.delete_blob(&tenant(), &hash.to_hex()))
                .unwrap();
            executor
                .block_on(provider.put_blob(&tenant(), &hash.to_hex(), &forged))
                .unwrap();
            assert_eq!(
                executor
                    .block_on(provider.get_blob(&tenant(), &hash.to_hex()))
                    .unwrap(),
                Some(forged.clone()),
                "file={file}: the forgery is in place"
            );
        }
        every_read_refuses(directory.path(), file, "forged payload");
        let runtime = existing(directory.path(), file);
        assert_ne!(
            runtime.content(&hash).ok().flatten(),
            Some(forged),
            "file={file}: content() served the forged bytes under the payload's address"
        );
    }
}

#[test]
fn a_named_payload_blob_deleted_after_seeding_is_refused_by_every_read_on_both_providers() {
    for file in [false, true] {
        let (directory, runtime, _) = seeded(seed(), context(), anchor(), file, SEEDED_AT);
        // One commit, so that a replay checkpoint and its pointer exist when the blob goes.
        commit_one(&runtime, 0x701, 0x31, "committed");
        drop(runtime);
        let hash = ContentHash::of_bytes(STATEMENT);
        {
            let (executor, provider) = provider(directory.path(), file);
            executor
                .block_on(provider.delete_blob(&tenant(), &hash.to_hex()))
                .unwrap();
        }
        every_read_refuses(directory.path(), file, "deleted payload");
    }
}

// --- determinism, zero evidence, one payload under two evidence ids ---------------------------

#[test]
fn a_v3_envelope_is_byte_identical_on_both_providers_for_every_evidence_shape() {
    let shapes = [
        ("the fixture", seed()),
        ("no evidence", seed_without_evidence()),
        ("one payload, two ids", seed_with(STATEMENT, 0x50)),
    ];
    for (shape, document) in shapes {
        let mut envelopes = Vec::new();
        for file in [false, true] {
            let (_directory, runtime, result) =
                seeded(document.clone(), context(), anchor(), file, SEEDED_AT);
            envelopes.push((
                result.seed_hash,
                runtime.content(&result.seed_hash).unwrap().unwrap(),
            ));
        }
        assert_eq!(envelopes[0], envelopes[1], "{shape}");
    }
}

#[test]
fn a_seed_without_evidence_names_an_empty_list_replays_and_migrates_on_both_providers() {
    for file in [false, true] {
        let (directory, runtime, result) = seeded(
            seed_without_evidence(),
            context(),
            anchor(),
            file,
            SEEDED_AT,
        );
        drop(runtime);
        let mut reopened = existing(directory.path(), file);
        reopened.set_full_replay(true);
        assert_eq!(
            reopened.read(None).unwrap().seed.seed_hash,
            result.seed_hash
        );
        let envelope = envelope_json(&reopened);
        assert_eq!(envelope["format"], "ekr-seed-envelope/3", "file={file}");
        assert_eq!(
            envelope["input"]["evidence_payloads"],
            serde_json::json!([]),
            "file={file}"
        );

        let source_directory = tempfile::tempdir().unwrap();
        v2::seed_v2(
            source_directory.path(),
            file,
            &seed_without_evidence(),
            context(),
            &anchor(),
            SEEDED_AT,
        );
        let source = existing(source_directory.path(), file);
        let destination_directory = tempfile::tempdir().unwrap();
        let destination = open(destination_directory.path(), file, context(), anchor());
        let report = source.migrate_into(&destination).unwrap();
        // The same input, context, authority and time, with a fresh seed-bound copy claim.
        assert_ne!(
            report.destination_seed_hash, result.seed_hash,
            "file={file}"
        );
        assert_claimed_envelope(&destination, &reopened, &report);
    }
}

#[test]
fn one_payload_cited_by_two_evidence_ids_is_named_and_retained_once_and_migrates() {
    for file in [false, true] {
        let document = seed_with(STATEMENT, 0x50);
        assert_eq!(document.graph.evidence.len(), 2);
        assert_eq!(document.evidence_payloads.len(), 1);
        let (directory, runtime, result) =
            seeded(document.clone(), context(), anchor(), file, SEEDED_AT);
        drop(runtime);
        let mut reopened = existing(directory.path(), file);
        reopened.set_full_replay(true);
        let envelope = envelope_json(&reopened);
        assert_eq!(
            envelope["input"]["evidence_payloads"],
            serde_json::json!([ContentHash::of_bytes(STATEMENT)]),
            "file={file}"
        );
        assert_eq!(
            object_streams(&reopened, ContentHash::of_bytes(STATEMENT)),
            1,
            "file={file}"
        );

        let source_directory = tempfile::tempdir().unwrap();
        v2::seed_v2(
            source_directory.path(),
            file,
            &document,
            context(),
            &anchor(),
            SEEDED_AT,
        );
        let source = existing(source_directory.path(), file);
        let destination_directory = tempfile::tempdir().unwrap();
        let destination = open(destination_directory.path(), file, context(), anchor());
        let report = source.migrate_into(&destination).unwrap();
        assert_ne!(
            report.destination_seed_hash, result.seed_hash,
            "file={file}"
        );
        assert_claimed_envelope(&destination, &reopened, &report);
        assert_eq!(
            destination.read(None).unwrap().seed_input,
            reopened.read(None).unwrap().seed_input,
            "file={file}"
        );
    }
}

// --- payload bytes retained once, after a commit and after a migration ------------------------

fn assert_claimed_envelope(
    destination: &Runtime,
    ordinary: &Runtime,
    report: &ekr_kernel::StoreMigrationV1,
) {
    let mut copied = envelope_json(destination);
    assert_eq!(copied["format"], "ekr-seed-envelope/4");
    copied["migration"]
        .as_str()
        .unwrap()
        .parse::<ekr_core::EventId>()
        .unwrap();
    copied.as_object_mut().unwrap().remove("migration");
    copied["format"] = serde_json::json!("ekr-seed-envelope/3");
    assert_eq!(copied, envelope_json(ordinary));
    let after = destination.read(None).unwrap();
    let before = ordinary.read(None).unwrap();
    assert_eq!(after.seed.seed_hash, report.destination_seed_hash);
    assert_eq!(after.seed_input, before.seed_input);
    assert_eq!(after.graph, before.graph);
    let mut expected = before.root;
    expected.transaction = report.destination_seed_hash;
    assert_eq!(after.root, expected);
}

fn transaction(transaction: u64, node: u64, name: &str) -> String {
    let id = |n: u64| format!("00000000-0000-4000-8000-{n:012x}");
    format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {}\n  proposer: {}\n  \
         operations:\n  - !CreateNode\n    id: {}\n    root_id: {}\n    type_id: {}\n    \
         canonical_name: {name}\n    properties: {{}}\n  evidence: []\n",
        id(transaction),
        context().operator,
        id(node),
        id(0x02),
        id(0x05),
    )
}

fn commit_one(runtime: &Runtime, tx: u64, node: u64, name: &str) {
    let operator = context().operator;
    runtime
        .propose(transaction(tx, node, name).as_bytes(), operator, || {
            Timestamp::from_millis(20)
        })
        .unwrap();
    let id: TransactionId = format!("00000000-0000-4000-8000-{tx:012x}")
        .parse()
        .unwrap();
    let head = runtime.head().unwrap().unwrap().revision;
    assert!(matches!(
        runtime
            .validate(id, head, || Timestamp::from_millis(21))
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        runtime
            .commit(id, operator, || Timestamp::from_millis(22))
            .unwrap(),
        CommitCommandResult::Committed(_)
    ));
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

/// Every retained byte sequence: every file of a file store; every event body and every blob the
/// feed names (objects, preparations, checkpoints) of a SQLite store.
fn retained(path: &Path, file: bool) -> Vec<(String, Vec<u8>)> {
    if file {
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
        let mut found = Vec::new();
        visit(path, &mut found);
        return found;
    }
    let (executor, provider) = provider(path, false);
    executor.block_on(async {
        let mut found = Vec::new();
        let mut digests = BTreeSet::new();
        let mut after = 0;
        loop {
            let page = provider.read_feed(&tenant(), after, 100).await.unwrap();
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
            if let Some(bytes) = provider.get_blob(&tenant(), &digest).await.unwrap() {
                found.push((format!("blob {digest}"), bytes));
            }
        }
        found
    })
}

fn copies(path: &Path, file: bool, payload: &[u8], label: &str) -> Vec<String> {
    let mut wrong = Vec::new();
    for (at, bytes) in retained(path, file) {
        if bytes == payload {
            continue;
        }
        for (spelling, needle) in spellings(payload) {
            if contains(&bytes, &needle) {
                wrong.push(format!(
                    "{label} file={file}: {at} ({} bytes) holds the payload as {spelling}",
                    bytes.len()
                ));
            }
        }
    }
    wrong
}

#[test]
fn no_retained_byte_but_the_payload_blob_holds_the_payload_after_a_commit_or_a_migration() {
    let payload = noise(7, 16_384);
    let document = seed_with(&payload, 0x51);
    let mut wrong = Vec::new();
    for file in [false, true] {
        // A /3 store after a commit, which retains a replay checkpoint, and a reopen.
        let (directory, runtime, _) =
            seeded(document.clone(), context(), anchor(), file, SEEDED_AT);
        commit_one(&runtime, 0x702, 0x32, "committed");
        drop(runtime);
        let reopened = existing(directory.path(), file);
        assert_eq!(
            reopened.head().unwrap().unwrap().revision,
            RevisionNumber::new(1)
        );
        drop(reopened);
        wrong.extend(copies(directory.path(), file, &payload, "committed /3"));

        // A /2 store with the same seed and a commit, migrated.
        let source_directory = tempfile::tempdir().unwrap();
        v2::seed_v2(
            source_directory.path(),
            file,
            &document,
            context(),
            &anchor(),
            SEEDED_AT,
        );
        let source = existing(source_directory.path(), file);
        commit_one(&source, 0x703, 0x33, "committed");
        let destination_directory = tempfile::tempdir().unwrap();
        let destination = open(destination_directory.path(), file, context(), anchor());
        source.migrate_into(&destination).unwrap();
        drop(destination);
        wrong.extend(copies(
            destination_directory.path(),
            file,
            &payload,
            "migrated",
        ));
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

// --- an interrupted migration -----------------------------------------------------------------

/// The destination store with a provider that stops accepting publications after `remaining`
/// of them: the process was interrupted, or the disk filled, mid-migration.
struct Interrupted<S> {
    inner: S,
    remaining: Cell<usize>,
}

impl<S: RevisionLog> RevisionLog for Interrupted<S> {
    fn preparation(
        &self,
        key: &PublicationCommandKey,
    ) -> Result<Option<PublicationPreparationV1>, StoreError> {
        self.inner.preparation(key)
    }
    fn prepare(
        &self,
        key: &PublicationCommandKey,
        input: ContentHash,
        decision: &Publication,
        previous: Option<&PublicationPreparationV1>,
    ) -> Result<PublicationPreparationV1, StoreError> {
        self.inner.prepare(key, input, decision, previous)
    }
    fn resume(&self, p: &PublicationPreparationV1) -> Result<ekr_store::Appended, StoreError> {
        self.inner.resume(p)
    }
    fn history(&self) -> Result<ekr_store::RetainedHistory, StoreError> {
        self.inner.history()
    }
    fn history_at(&self, r: RevisionNumber) -> Result<ekr_store::RetainedHistory, StoreError> {
        self.inner.history_at(r)
    }
    fn publish(&self, p: &Publication) -> Result<ekr_store::Appended, StoreError> {
        match self.remaining.get() {
            0 => Err(StoreError::Backend("interrupted".into())),
            n => {
                self.remaining.set(n - 1);
                self.inner.publish(p)
            }
        }
    }
    fn seed_bytes(&self) -> Result<Option<Vec<u8>>, StoreError> {
        self.inner.seed_bytes()
    }
    fn fold(&self) -> Result<ekr_graph::CanonicalGraph, StoreError> {
        self.inner.fold()
    }
    fn head(&self) -> Result<Option<ekr_graph::Root>, StoreError> {
        self.inner.head()
    }
    fn replay(&self, r: RevisionNumber) -> Result<ekr_graph::CanonicalGraph, StoreError> {
        self.inner.replay(r)
    }
    fn write_checkpoint(
        &self,
        covered: u64,
        binding: ContentHash,
        checkpoint: Option<&[u8]>,
    ) -> Result<bool, StoreError> {
        self.inner.write_checkpoint(covered, binding, checkpoint)
    }
}
impl<S: Initialize> Initialize for Interrupted<S> {
    fn initialize(&self, p: &Publication) -> Result<ekr_store::Appended, StoreError> {
        self.inner.initialize(p)
    }
}
impl<S: ObjectStore> ObjectStore for Interrupted<S> {
    fn put(
        &self,
        c: StorageClass,
        b: &[u8],
        t: Timestamp,
    ) -> Result<ekr_store::StoredObject, StoreError> {
        self.inner.put(c, b, t)
    }
    fn get(&self, h: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        self.inner.get(h)
    }
}
impl<S: Inventory> Inventory for Interrupted<S> {
    fn inventory(&self) -> Result<StoreInventory, StoreError> {
        self.inner.inventory()
    }
}

/// Migrates the `/2` fixture store at `source` into `destination`, interrupted after the seed and
/// eight more publications: the destination then holds revision 1 of the source's 2.
fn migrate_interrupted(source: &Path, destination: &Path, file: bool) -> String {
    let error = if file {
        let from = Commit::over_with_authority(context(), anchor(), |authority| {
            Ok(FileStore::file(source, TENANT, None)?.under(authority))
        })
        .unwrap();
        let into = Commit::over_with_authority(context(), anchor(), |authority| {
            Ok(Interrupted {
                inner: FileStore::file(destination, TENANT, None)?.under(authority),
                remaining: Cell::new(8),
            })
        })
        .unwrap();
        from.migrate_into(&into).unwrap_err()
    } else {
        let from = Commit::over_with_authority(context(), anchor(), |authority| {
            Ok(SqliteStore::sqlite(&source.join("state.db"), TENANT, None)?.under(authority))
        })
        .unwrap();
        let into = Commit::over_with_authority(context(), anchor(), |authority| {
            Ok(Interrupted {
                inner: SqliteStore::sqlite(&destination.join("state.db"), TENANT, None)?
                    .under(authority),
                remaining: Cell::new(8),
            })
        })
        .unwrap();
        from.migrate_into(&into).unwrap_err()
    };
    error.to_string()
}

#[test]
fn an_interrupted_migration_leaves_no_store_that_opens_as_if_it_were_the_migrated_one() {
    for file in [false, true] {
        let source_directory = tempfile::tempdir().unwrap();
        v2::fixture_store(
            source_directory.path(),
            file,
            &seed(),
            context(),
            &anchor(),
            SEEDED_AT,
        );
        let destination_directory = tempfile::tempdir().unwrap();
        let refused =
            migrate_interrupted(source_directory.path(), destination_directory.path(), file);
        assert!(refused.contains("interrupted"), "file={file}: {refused}");

        let source = existing(source_directory.path(), file);
        let source_head = source.head().unwrap().unwrap();
        assert_eq!(source_head.revision, RevisionNumber::new(2));

        // A second run does not write over the partial store: it refuses it.
        let partial = existing(destination_directory.path(), file);
        let again = source.migrate_into(&partial).unwrap_err().to_string();
        assert!(
            again.contains("migrate-destination-not-empty"),
            "file={file}: {again}"
        );

        // No migration map was retained, and nothing else marks the store as incomplete: a store
        // an interrupted migration left must not answer as an ordinary store of a different head.
        let answered = partial.head();
        assert!(
            answered.is_err() || answered.as_ref().unwrap().as_ref() == Some(&source_head),
            "file={file}: the interrupted destination opens and answers head revision {:?} while \
             its source is at {:?}; nothing in it says the migration did not finish",
            answered.map(|root| root.map(|root| root.revision)),
            source_head.revision
        );
    }
}
