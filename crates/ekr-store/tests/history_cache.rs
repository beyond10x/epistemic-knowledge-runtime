//! `story:history-loaded-once-per-process`: one store handle reads each revision occurrence and
//! verifies each retained blob once, and a later read loads only what it has not seen.
//!
//! Every case runs on the SQLite and the file provider, which share `EventlogStore<S>`. Work is
//! counted with `ekr_store::read_work`, the per-thread tally of blobs fetched, blobs hashed and
//! revision events fetched; nothing here relies on timing, except the ignored measurement at the
//! end, which prints and asserts nothing about speed.
//!
//! The authority is [`Touch`], a stand-in that admits no semantics: it asks the history for every
//! record and seed payload through `RetainedHistory::content`, as the kernel's replay does, and
//! admits nothing. It is not acceptance evidence for replay; `ekr --test conformance` is.

use std::collections::BTreeSet;
use std::path::Path;

use ekr_core::TransactionId;
use ekr_core::{AgentId, ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    read_work, AdmittedRevision, Appended, CommitAuthority, FileStore, Initialize, ObjectStore,
    Publication, PublicationObject, ReadWork, RetainedHistory, RevisionLog, SqliteStore,
    StorageClass, StoreError,
};
use tempfile::TempDir;

const TENANT: &str = "ekr";

/// Discards the work counted on this thread so far.
fn restart_count() {
    let _ = ekr_store::read_work();
}

/// Reads every record and seed payload the way a replay does, and admits nothing.
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

fn sqlite(root: &Path) -> SqliteStore {
    SqliteStore::sqlite(&root.join("state.db"), TENANT, None)
        .expect("the SQLite provider opens")
        .under(Touch)
}

fn file(root: &Path) -> FileStore {
    FileStore::file(&root.join("state"), TENANT, None)
        .expect("the file provider opens")
        .under(Touch)
}

/// Distinctive bytes, short enough that SQLite keeps them contiguous on one page.
fn payload(label: &str) -> Vec<u8> {
    format!("history-cache {label} {} ", EventId::mint())
        .repeat(4)
        .into_bytes()
}

/// One occurrence at stream position `expected_version`, whose record is `record` and which
/// publishes `extra` beside it.
fn occurrence(
    payload: RevisionPayload,
    expected_version: u64,
    record: Vec<u8>,
    extra: Option<Vec<u8>>,
) -> Publication {
    let record_hash = ContentHash::of_bytes(&record);
    let canonical = |bytes| PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::EPOCH,
        bytes,
    };
    let mut objects = std::collections::BTreeMap::from([(record_hash, canonical(record))]);
    if let Some(bytes) = extra {
        objects.insert(ContentHash::of_bytes(&bytes), canonical(bytes));
    }
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id: EventId::mint(),
            record_hash,
            payload,
        },
        objects,
        expected_version,
    }
}

fn seed(record: Vec<u8>, document: Vec<u8>) -> Publication {
    occurrence(
        RevisionPayload::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash: ContentHash::of_bytes(&document),
        },
        0,
        record,
        Some(document),
    )
}

fn proposal(expected_version: u64, record: Vec<u8>) -> Publication {
    occurrence(
        RevisionPayload::TransactionProposed {
            transaction_id: TransactionId::mint(),
            proposer: AgentId::mint(),
            operations_hash: None,
        },
        expected_version,
        record,
        None,
    )
}

/// Seeds the store and publishes `later` proposals after it, each with a record of its own.
/// Returns every published payload.
fn written<S: RevisionLog + Initialize>(store: &S, label: &str, later: u64) -> Vec<Vec<u8>> {
    let (record, document) = (payload(&format!("{label} seed record")), payload(label));
    let mut written = vec![record.clone(), document.clone()];
    assert_eq!(
        store.initialize(&seed(record, document)).unwrap(),
        Appended::Written
    );
    for at in 1..=later {
        let record = payload(&format!("{label} record {at}"));
        written.push(record.clone());
        assert_eq!(
            store.publish(&proposal(at, record)).unwrap(),
            Appended::Written
        );
    }
    written
}

/// Flips one byte in every copy of `bytes` under `root`: the provider's blob file, or the SQLite
/// database and its WAL, as damage on disk would.
fn damage(root: &Path, bytes: &[u8]) {
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
    assert!(damaged > 0, "the blob's bytes were found on disk");
}

// ---------------------------------------------------------------------------------------------
// 1. A second read on one handle reads and hashes no blob the first read verified.
// ---------------------------------------------------------------------------------------------

fn a_second_read_repeats_no_work<S: RevisionLog + ObjectStore + Initialize>(open: impl Fn() -> S) {
    let payloads = {
        let writer = open();
        written(&writer, "second read", 2)
    };
    let reader = open();
    restart_count();

    let first = reader.history().unwrap();
    let work = read_work();
    assert_eq!(first.occurrences.len(), 3);
    assert_eq!(first.objects.len(), 4);
    assert_eq!(
        (work.occurrences_read, work.blobs_read),
        (3, 4),
        "the first read fetches every occurrence and every required blob once"
    );
    assert_eq!(
        work.blobs_hashed, 4,
        "the first read hashes each blob once: the replay's `content` does not hash again what \
         the load verified"
    );

    let second = reader.history().unwrap();
    assert_eq!(second, first, "a later read returns the same history");
    assert_eq!(
        read_work(),
        ReadWork::default(),
        "a second read on one handle fetched or hashed again what the first read verified"
    );

    let at_seed = reader.history_at(RevisionNumber::SEED).unwrap();
    assert_eq!(
        read_work(),
        ReadWork::default(),
        "a read at an earlier revision repeated work the first read did"
    );
    assert_eq!(
        at_seed,
        open().history_at(RevisionNumber::SEED).unwrap(),
        "a read at the seed served from what the handle holds differs from a fresh handle's"
    );
    restart_count();

    for bytes in &payloads {
        assert_eq!(
            reader
                .get(&ContentHash::of_bytes(bytes))
                .unwrap()
                .as_deref(),
            Some(bytes.as_slice())
        );
    }
    assert_eq!(
        read_work(),
        ReadWork::default(),
        "a single-object read fetched or hashed again an object the history read verified"
    );
}

#[test]
fn a_second_read_on_one_handle_reads_and_hashes_no_blob_the_first_verified_sqlite() {
    let directory = TempDir::new().unwrap();
    a_second_read_repeats_no_work(|| sqlite(directory.path()));
}

#[test]
fn a_second_read_on_one_handle_reads_and_hashes_no_blob_the_first_verified_file() {
    let directory = TempDir::new().unwrap();
    a_second_read_repeats_no_work(|| file(directory.path()));
}

// ---------------------------------------------------------------------------------------------
// 1b. A later read shares the bytes the handle holds (`story:retained-bytes-shared-not-copied`).
// ---------------------------------------------------------------------------------------------

/// Every object of a later history read on one handle is the allocation the handle already
/// holds, the one the first read returned, and not a copy of it; and the replay's `content`
/// checks of those bytes compare none of them, because they are the registered verified copy.
fn a_later_read_copies_no_retained_object<S: RevisionLog + ObjectStore + Initialize>(
    open: impl Fn() -> S,
) {
    {
        let writer = open();
        written(&writer, "shared bytes", 2);
    }
    let reader = open();
    let first = reader.history().unwrap();
    assert_eq!(first.objects.len(), 4);
    restart_count();

    let second = reader.history().unwrap();
    let at_seed = reader.history_at(RevisionNumber::SEED).unwrap();
    let work = read_work();
    for later in [&second, &at_seed] {
        for (hash, object) in &later.objects {
            assert_eq!(
                object.bytes.as_ptr(),
                first.objects[hash].bytes.as_ptr(),
                "a later read copied the retained object {hash} instead of sharing it"
            );
        }
    }
    assert_eq!(at_seed.objects.len(), 2);
    assert_eq!(
        work,
        ReadWork::default(),
        "a later read fetched, hashed or compared bytes the handle already verified"
    );
}

#[test]
fn a_later_read_on_one_handle_copies_no_retained_object_sqlite() {
    let directory = TempDir::new().unwrap();
    a_later_read_copies_no_retained_object(|| sqlite(directory.path()));
}

#[test]
fn a_later_read_on_one_handle_copies_no_retained_object_file() {
    let directory = TempDir::new().unwrap();
    a_later_read_copies_no_retained_object(|| file(directory.path()));
}

// ---------------------------------------------------------------------------------------------
// 2. An occurrence appended through another handle is seen by the next read.
// ---------------------------------------------------------------------------------------------

fn another_handles_append_is_seen<S: RevisionLog + ObjectStore + Initialize>(open: impl Fn() -> S) {
    let reader = open();
    written(&reader, "another handle", 1);
    let before = reader.history().unwrap();
    assert_eq!(before.occurrences.len(), 2);

    let record = payload("another handle late record");
    let late = proposal(2, record.clone());
    {
        let other = open();
        assert_eq!(other.publish(&late).unwrap(), Appended::Written);
    }
    restart_count();

    let after = reader.history().unwrap();
    let work = read_work();
    assert_eq!(
        after.occurrences.len(),
        3,
        "the other handle's occurrence is read"
    );
    assert_eq!(after.occurrences[..2], before.occurrences[..]);
    assert_eq!(after.occurrences[2].event, late.event);
    assert_eq!(
        *after.objects[&late.event.record_hash].bytes, record,
        "the other handle's record is loaded"
    );
    assert_eq!(
        (work.occurrences_read, work.blobs_read, work.blobs_hashed),
        (1, 1, 1),
        "the next read fetches only the new occurrence and verifies only its new blob"
    );
    assert_eq!(
        after,
        open().history().unwrap(),
        "the extended history differs from a fresh handle's"
    );
}

#[test]
fn an_occurrence_appended_through_another_handle_is_seen_by_the_next_read_sqlite() {
    let directory = TempDir::new().unwrap();
    another_handles_append_is_seen(|| sqlite(directory.path()));
}

#[test]
fn an_occurrence_appended_through_another_handle_is_seen_by_the_next_read_file() {
    let directory = TempDir::new().unwrap();
    another_handles_append_is_seen(|| file(directory.path()));
}

/// `task:write-verbs-cost-most-of-an-ingest`: on the file provider every call re-hashes the log
/// while it is younger than two seconds, so a head read that confirmed the last held occurrence
/// in one call and read on in another paid for the log twice. The call that reads on confirms it.
fn a_read_on_confirms_the_held_prefix_in_the_same_call<
    S: RevisionLog + ObjectStore + Initialize,
>(
    open: impl Fn() -> S,
) {
    let reader = open();
    written(&reader, "one call", 1);
    let before = reader.history().unwrap();
    let _ = ekr_store::stream_reads();

    assert_eq!(reader.history().unwrap(), before);
    assert_eq!(
        ekr_store::stream_reads().revision,
        1,
        "a head read that found nothing new made more than one revision-stream read"
    );

    let late = proposal(2, payload("one call late record"));
    {
        let other = open();
        assert_eq!(other.publish(&late).unwrap(), Appended::Written);
    }
    let _ = ekr_store::stream_reads();
    let after = reader.history().unwrap();
    assert_eq!(
        ekr_store::stream_reads().revision,
        1,
        "a head read that found a new occurrence made more than one revision-stream read"
    );
    assert_eq!(after.occurrences.len(), 3);
    assert_eq!(after.occurrences[2].event, late.event);
    assert_eq!(
        after,
        open().history().unwrap(),
        "the extended history differs from a fresh handle's"
    );
}

#[test]
fn a_read_on_confirms_the_held_prefix_in_the_same_call_sqlite() {
    let directory = TempDir::new().unwrap();
    a_read_on_confirms_the_held_prefix_in_the_same_call(|| sqlite(directory.path()));
}

#[test]
fn a_read_on_confirms_the_held_prefix_in_the_same_call_file() {
    let directory = TempDir::new().unwrap();
    a_read_on_confirms_the_held_prefix_in_the_same_call(|| file(directory.path()));
}

// ---------------------------------------------------------------------------------------------
// 3. A blob damaged on disk before its first load is still refused, as before.
// ---------------------------------------------------------------------------------------------

/// The reader has verified the lineage so far; another handle appends an occurrence whose record
/// is then damaged on disk before anything loads it. The reader must refuse it exactly as a handle
/// that holds nothing does.
fn a_damaged_blob_is_refused<S: RevisionLog + ObjectStore + Initialize>(
    root: &Path,
    open: impl Fn() -> S,
) {
    let reader = open();
    written(&reader, "damage", 1);
    reader.history().unwrap();

    let record = payload("damage late record");
    assert_eq!(
        open().publish(&proposal(2, record.clone())).unwrap(),
        Appended::Written
    );
    let fresh = open();
    damage(root, &record);

    let refused = reader.history().unwrap_err();
    let expected = fresh.history().unwrap_err();
    assert_eq!(
        refused, expected,
        "a handle holding verified history refused a damaged new blob differently from one \
         holding nothing"
    );
    assert!(
        refused.to_string().contains("integrity"),
        "the damaged blob was refused, but not for its integrity: {refused}"
    );
}

#[test]
fn a_blob_damaged_before_its_first_load_is_still_refused_sqlite() {
    let directory = TempDir::new().unwrap();
    a_damaged_blob_is_refused(directory.path(), || sqlite(directory.path()));
}

#[test]
fn a_blob_damaged_before_its_first_load_is_still_refused_file() {
    let directory = TempDir::new().unwrap();
    a_damaged_blob_is_refused(directory.path(), || file(directory.path()));
}

/// `RetainedHistory::content` is the check every replay reads through, and its fields are public:
/// a copy of a verified object whose bytes were changed afterwards must still be refused, whatever
/// the process has verified before.
fn a_tampered_copy_is_refused<S: RevisionLog + ObjectStore + Initialize>(open: impl Fn() -> S) {
    let store = open();
    let payloads = written(&store, "tampered copy", 1);
    let history = store.history().unwrap();
    let hash = ContentHash::of_bytes(&payloads[0]);
    assert!(history.content(hash, StorageClass::Canonical).is_ok());

    let refused = |change: &dyn Fn(&mut Vec<u8>)| {
        let mut copy = history.clone();
        change(std::sync::Arc::make_mut(
            &mut copy.objects.get_mut(&hash).unwrap().bytes,
        ));
        let length = copy.objects[&hash].bytes.len() as u64;
        copy.objects.get_mut(&hash).unwrap().metadata.byte_len = length;
        copy.content(hash, StorageClass::Canonical).unwrap_err()
    };
    let integrity = StoreError::Document("required-object-integrity".into());
    assert_eq!(refused(&|bytes| bytes[3] ^= 1), integrity);
    assert_eq!(refused(&|bytes| bytes.truncate(bytes.len() - 1)), integrity);
    assert_eq!(refused(&|bytes| bytes.push(b'!')), integrity);
    assert!(
        history.content(hash, StorageClass::Canonical).is_ok(),
        "the untouched history still reads"
    );
}

#[test]
fn a_tampered_copy_of_a_verified_object_is_refused_by_content_sqlite() {
    let directory = TempDir::new().unwrap();
    a_tampered_copy_is_refused(|| sqlite(directory.path()));
}

#[test]
fn a_tampered_copy_of_a_verified_object_is_refused_by_content_file() {
    let directory = TempDir::new().unwrap();
    a_tampered_copy_is_refused(|| file(directory.path()));
}

// ---------------------------------------------------------------------------------------------
// Nothing a handle holds answers for a store the provider says has diverged from it.
// ---------------------------------------------------------------------------------------------

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A file store replaced under a live handle by a different history. The provider refuses the
/// replacement to a handle that observed the original; a single-object read of an object the
/// handle verified must refuse too, rather than answer from what the handle holds.
#[test]
fn a_held_object_is_not_served_from_a_file_store_that_diverged() {
    let held_dir = TempDir::new().unwrap();
    let other_dir = TempDir::new().unwrap();
    let reader = file(held_dir.path());
    let payloads = written(&reader, "diverged get", 1);
    reader.history().unwrap();
    let address = ContentHash::of_bytes(&payloads[0]);
    assert!(reader.get(&address).unwrap().is_some());
    written(&file(other_dir.path()), "diverged replacement", 1);
    std::fs::remove_dir_all(held_dir.path().join("state")).unwrap();
    copy_tree(
        &other_dir.path().join("state"),
        &held_dir.path().join("state"),
    );

    let head = reader.history().unwrap_err();
    let got = reader.get(&address);
    assert_eq!(
        got,
        Err(head),
        "a get answered from the handle's memo where the same handle's head read refuses"
    );
}

/// The file provider's refusal of a history that diverged from what a live handle observed
/// reaches a caller as [`StoreError::Diverged`], on a head read and on a single-object read alike:
/// a variant to match, so no caller depends on how the provider words it.
#[test]
fn a_file_store_that_diverged_is_refused_as_a_typed_divergence() {
    let held_dir = TempDir::new().unwrap();
    let other_dir = TempDir::new().unwrap();
    let reader = file(held_dir.path());
    let payloads = written(&reader, "typed divergence", 1);
    reader.history().unwrap();
    written(&file(other_dir.path()), "typed divergence replacement", 1);
    std::fs::remove_dir_all(held_dir.path().join("state")).unwrap();
    copy_tree(
        &other_dir.path().join("state"),
        &held_dir.path().join("state"),
    );

    let head = reader.history();
    assert!(
        matches!(head, Err(StoreError::Diverged(_))),
        "a head read of a diverged file store answered {head:?}"
    );
    let got = reader.get(&ContentHash::of_bytes(&payloads[0]));
    assert!(
        matches!(got, Err(StoreError::Diverged(_))),
        "an object read of a diverged file store answered {got:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Measurement: two consecutive history reads on one handle over a large store. Ignored; run with
// `cargo test -p ekr-store --test history_cache -- --ignored --nocapture`.
// ---------------------------------------------------------------------------------------------

/// `size` bytes that differ from any other call's.
fn large(label: &str, size: usize) -> Vec<u8> {
    let mut bytes = format!("history-cache {label} {} ", EventId::mint()).into_bytes();
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    while bytes.len() < size {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        bytes.extend_from_slice(&state.to_le_bytes());
    }
    bytes.truncate(size);
    bytes
}

fn measure<S: RevisionLog + ObjectStore + Initialize>(provider: &str, open: impl Fn() -> S) {
    const MIB: usize = 1 << 20;
    let (document_size, record_size, later) = (48 * MIB, 8 * MIB, 6_u64);
    let document = large("seed document", document_size);
    {
        let writer = open();
        assert_eq!(
            writer
                .initialize(&seed(large("seed record", MIB), document.clone()))
                .unwrap(),
            Appended::Written
        );
        for at in 1..=later {
            assert_eq!(
                writer
                    .publish(&proposal(at, large("record", record_size)))
                    .unwrap(),
                Appended::Written
            );
        }
    }
    let total = document_size + MIB + record_size * later as usize;
    let reader = open();
    restart_count();
    for call in 1..=2 {
        let started = std::time::Instant::now();
        let history = reader.history().unwrap();
        let elapsed = started.elapsed();
        let work = read_work();
        println!(
            "{provider}: history() #{call}: {:>8.1} ms  occurrences={} objects={} \
             blobs_read={} blobs_hashed={} occurrences_read={} store={} MiB",
            elapsed.as_secs_f64() * 1e3,
            history.occurrences.len(),
            history.objects.len(),
            work.blobs_read,
            work.blobs_hashed,
            work.occurrences_read,
            total / MIB,
        );
    }
    let address = ContentHash::of_bytes(&document);
    let started = std::time::Instant::now();
    let got = reader.get(&address).unwrap();
    let elapsed = started.elapsed();
    let work = read_work();
    assert_eq!(got.as_deref(), Some(document.as_slice()));
    println!(
        "{provider}: get(seed document, {} MiB): {:>8.1} ms  blobs_read={} blobs_hashed={}",
        document_size / MIB,
        elapsed.as_secs_f64() * 1e3,
        work.blobs_read,
        work.blobs_hashed,
    );
}

#[test]
#[ignore = "measurement; prints timings"]
fn measure_two_history_reads_on_one_handle() {
    let directory = TempDir::new().unwrap();
    measure("sqlite", || sqlite(directory.path()));
    let directory = TempDir::new().unwrap();
    measure("file", || file(directory.path()));
}
