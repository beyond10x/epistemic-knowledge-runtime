//! `story:commit-cost-flat-with-store-size`: a history load reads the stream of a verified
//! non-canonical object again only when that object's stream has moved since this handle
//! verified it.
//!
//! Evidence payloads are `Provenance` objects, and a history load required every one of them. The
//! handle's memo served only `Canonical` objects, because another handle may raise any other
//! class, so every load read one object stream per evidence object and the cost of a verb grew
//! with the evidence in the store. A raise appends an event to the object's stream, so the log
//! says when a held class may have moved. So does a withdrawal of retained bytes, which the store
//! domain requires to append an event there too (held bytes); section 3 holds that no source
//! withdraws bytes in place.
//!
//! Every case runs on the SQLite and the file provider. Reads are counted with
//! `ekr_store::stream_reads`, the per-thread tally of provider stream reads.

use std::collections::BTreeSet;
use std::path::Path;

use ekr_core::TransactionId;
use ekr_core::{AgentId, ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, FileStore, Initialize, ObjectStore, Publication,
    PublicationObject, RetainedHistory, RevisionLog, SqliteStore, StorageClass, StoreError,
};
use tempfile::TempDir;

const TENANT: &str = "ekr";

/// Requires the one extra object it names with every history, as the kernel requires an evidence
/// payload, and admits nothing.
struct Requires(Option<ContentHash>);

impl CommitAuthority for Requires {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(self.0.into_iter().collect())
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

fn sqlite(root: &Path, required: Option<ContentHash>) -> SqliteStore {
    std::fs::create_dir_all(root).unwrap();
    SqliteStore::sqlite(&root.join("state.db"), TENANT, None)
        .expect("the SQLite provider opens")
        .under(Requires(required))
}

fn file(root: &Path, required: Option<ContentHash>) -> FileStore {
    FileStore::file(&root.join("state"), TENANT, None)
        .expect("the file provider opens")
        .under(Requires(required))
}

fn payload(label: &str) -> Vec<u8> {
    format!("object memo {label} {} ", EventId::mint())
        .repeat(4)
        .into_bytes()
}

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

/// Seeds the store, stores `evidence` as a `Provenance` object and publishes one proposal.
fn seeded_with_evidence<S: RevisionLog + ObjectStore + Initialize>(store: &S, evidence: &[u8]) {
    let (record, document) = (payload("seed record"), payload("seed document"));
    let seed = occurrence(
        RevisionPayload::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash: ContentHash::of_bytes(&document),
        },
        0,
        record,
        Some(document),
    );
    assert_eq!(store.initialize(&seed).unwrap(), Appended::Written);
    let stored = store
        .put(StorageClass::Provenance, evidence, Timestamp::EPOCH)
        .unwrap();
    assert_eq!(stored.storage_class, StorageClass::Provenance);
    assert_eq!(
        store.publish(&proposal(1, payload("record 1"))).unwrap(),
        Appended::Written
    );
}

fn class_of(history: &RetainedHistory, address: ContentHash) -> StorageClass {
    history.objects[&address].metadata.storage_class
}

// ---------------------------------------------------------------------------------------------
// 1. A second load reads no object stream while the log has not moved; a raise by another handle
//    is read again and seen.
// ---------------------------------------------------------------------------------------------

fn read_again_only_after_a_raise<S: RevisionLog + ObjectStore + Initialize>(
    evidence: &[u8],
    open: impl Fn(Option<ContentHash>) -> S,
) -> Vec<String> {
    let mut wrong = Vec::new();
    let address = ContentHash::of_bytes(evidence);
    seeded_with_evidence(&open(None), evidence);

    let reader = open(Some(address));
    let _ = ekr_store::stream_reads();
    let first = reader.history().unwrap();
    let first_reads = ekr_store::stream_reads();
    if class_of(&first, address) != StorageClass::Provenance {
        wrong.push(format!(
            "the first load holds the evidence at {:?}",
            class_of(&first, address)
        ));
    }
    if first_reads.object == 0 {
        wrong.push("the first load read no object stream".into());
    }

    let second = reader.history().unwrap();
    let second_reads = ekr_store::stream_reads();
    if second_reads.object != 0 {
        wrong.push(format!(
            "a second load with the log not advanced read {} object streams (first load: {})",
            second_reads.object, first_reads.object
        ));
    }
    if second_reads.feed != 1 {
        wrong.push(format!(
            "a second load with the log not advanced read the log {} times, not once",
            second_reads.feed
        ));
    }
    if second != first {
        wrong.push("the second load differs from the first".into());
    }

    let raised = open(Some(address))
        .put(StorageClass::Canonical, evidence, Timestamp::EPOCH)
        .unwrap();
    assert_eq!(raised.storage_class, StorageClass::Canonical);

    let _ = ekr_store::stream_reads();
    let third = reader.history().unwrap();
    let third_reads = ekr_store::stream_reads();
    if third_reads.object != 1 {
        wrong.push(format!(
            "the load after another handle raised the evidence read {} object streams, not its one",
            third_reads.object
        ));
    }
    if class_of(&third, address) != StorageClass::Canonical {
        wrong.push(format!(
            "the load after another handle raised the evidence holds it at {:?}",
            class_of(&third, address)
        ));
    }
    if third != open(Some(address)).history().unwrap() {
        wrong.push("the load after the raise differs from a fresh handle's".into());
    }
    wrong
}

#[test]
fn a_held_provenance_object_is_read_again_only_after_another_handle_raises_it() {
    let directory = TempDir::new().unwrap();
    let evidence = payload("raised evidence");
    let mut wrong = Vec::new();
    let sqlite_root = directory.path().join("sqlite");
    let file_root = directory.path().join("file");
    wrong.extend(
        read_again_only_after_a_raise(&evidence, |required| sqlite(&sqlite_root, required))
            .into_iter()
            .map(|line| format!("sqlite: {line}")),
    );
    wrong.extend(
        read_again_only_after_a_raise(&evidence, |required| file(&file_root, required))
            .into_iter()
            .map(|line| format!("file: {line}")),
    );
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// 2. A write that does not touch the held object moves the log but not the object's stream, so
//    the next load reads only what that write added. Every transaction commits, so a memo that
//    is dropped whenever the log moves would still read every evidence stream once per
//    transaction.
// ---------------------------------------------------------------------------------------------

fn not_read_again_after_an_unrelated_write<S: RevisionLog + ObjectStore + Initialize>(
    evidence: &[u8],
    open: impl Fn(Option<ContentHash>) -> S,
) -> Vec<String> {
    let mut wrong = Vec::new();
    let address = ContentHash::of_bytes(evidence);
    seeded_with_evidence(&open(None), evidence);

    let reader = open(Some(address));
    let before = reader.history().unwrap();
    let written = before.occurrences.len() as u64;

    // Another handle's write: the next load reads the one record that write added.
    assert_eq!(
        open(Some(address))
            .publish(&proposal(written, payload("their record")))
            .unwrap(),
        Appended::Written
    );
    let _ = ekr_store::stream_reads();
    reader.history().unwrap();
    let reads = ekr_store::stream_reads();
    if reads.object != 1 {
        wrong.push(format!(
            "the load after another handle's write that left the evidence alone read {} object \
             streams, not the new record's one",
            reads.object
        ));
    }

    // This handle's own write: the next load reads the record it forgot on writing it.
    assert_eq!(
        reader
            .publish(&proposal(written + 1, payload("own record")))
            .unwrap(),
        Appended::Written
    );
    let _ = ekr_store::stream_reads();
    let after = reader.history().unwrap();
    let reads = ekr_store::stream_reads();
    if reads.object != 1 {
        wrong.push(format!(
            "the load after this handle's own write that left the evidence alone read {} object \
             streams, not its record's one",
            reads.object
        ));
    }
    if class_of(&after, address) != StorageClass::Provenance {
        wrong.push(format!(
            "the evidence is held at {:?}",
            class_of(&after, address)
        ));
    }
    if after != open(Some(address)).history().unwrap() {
        wrong.push("the load after the writes differs from a fresh handle's".into());
    }
    wrong
}

#[test]
fn a_held_provenance_object_is_not_read_again_after_a_write_that_leaves_it_alone() {
    let directory = TempDir::new().unwrap();
    let evidence = payload("untouched evidence");
    let mut wrong = Vec::new();
    let sqlite_root = directory.path().join("sqlite");
    let file_root = directory.path().join("file");
    wrong.extend(
        not_read_again_after_an_unrelated_write(&evidence, |required| {
            sqlite(&sqlite_root, required)
        })
        .into_iter()
        .map(|line| format!("sqlite: {line}")),
    );
    wrong.extend(
        not_read_again_after_an_unrelated_write(&evidence, |required| file(&file_root, required))
            .into_iter()
            .map(|line| format!("file: {line}")),
    );
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// 3. Nothing withdraws retained bytes in place (`systems/ekr/domains/store.yaml`, held bytes):
//    the memo above sees a withdrawal only as an event on the object's stream.
// ---------------------------------------------------------------------------------------------

/// The workspace's `crates` directory, found from the invoking checkout at run time.
fn crates_directory() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    std::path::Path::new(&manifest)
        .parent()
        .expect("the crate sits in the workspace's crates directory")
        .to_owned()
}

/// Every `.rs` file under `directory`.
fn sources(directory: &Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("a source directory") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// The provider calls that take retained bytes away in place: a redaction, a blob deletion, and
/// `forget_tenant`, which removes every event and blob of a tenant.
const WITHDRAWALS: [&str; 3] = ["redact", "delete_blob", "forget_tenant"];

/// How many provider withdrawals `source` reaches outside line comments, in any form: a method
/// call (`.redact(`), a qualified or fully qualified call (`EventStore::redact(`,
/// `<P as EventStore>::delete_blob(`), a turbofish (`forget_tenant::<`) and a path used as a value
/// (`EventStore::redact` passed along). An identifier is counted when it is the whole word, and is
/// reached through `.` or `::` or followed by `(` or `::`; a definition (`fn redact`) is not
/// counted.
fn withdrawals(source: &str) -> usize {
    let identifier = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut found = 0;
    for line in source.lines() {
        let code = line.split("//").next().unwrap_or_default();
        for name in WITHDRAWALS {
            for (at, _) in code.match_indices(name) {
                let before = code[..at].trim_end();
                let after = code[at + name.len()..].trim_start();
                if before.chars().next_back().is_some_and(identifier)
                    || code[at + name.len()..]
                        .chars()
                        .next()
                        .is_some_and(identifier)
                    || before.ends_with("fn")
                        && !before[..before.len() - 2]
                            .chars()
                            .next_back()
                            .is_some_and(identifier)
                {
                    continue;
                }
                let reached = before.ends_with('.') || before.ends_with("::");
                let called = after.starts_with('(') || after.starts_with("::");
                if reached || called {
                    found += 1;
                }
            }
        }
    }
    found
}

/// Every product source of the workspace calling a provider withdrawal, with how many it calls,
/// is one this list names: the checkpoint pointer's deletion of the cache blob it has just
/// replaced, after appending the pointer event that names its successor, and the test-only
/// provider wrapper of `eventlog_reads.rs` (one `redact`, one `forget_tenant`, three `delete_blob`).
/// A new call — a redaction or a deletion path — must
/// append an event to the object's stream and is added here with that said.
///
/// `stage_record.rs` holds the one `forget_tenant` held-bytes rule 2 admits (design § 107.7): a
/// stage's own tenant, after the stage record's `StagePublished` or `StageAbandoned` in the
/// store's tenant, taken from the stage's `StageBegun` only where it is the tenant derived from
/// the store's tenant and the stage id, and never the store's own. A forgotten tenant has no
/// stream to append to; a handle joined to the stage reads that record before every read and
/// write instead.
#[test]
fn no_source_withdraws_retained_bytes_without_an_event_on_the_object_stream() {
    let allowed = [
        ("ekr-store/src/eventlog.rs", 1),
        ("ekr-store/src/eventlog_reads.rs", 5),
        ("ekr-store/src/stage_record.rs", 1),
    ];
    let root = crates_directory();
    let mut all = Vec::new();
    for entry in std::fs::read_dir(&root).expect("the crates directory") {
        let source = entry.expect("an entry").path().join("src");
        if source.is_dir() {
            sources(&source, &mut all);
        }
    }
    assert!(
        all.len() > 50,
        "the workspace's sources are found: {}",
        all.len()
    );
    let mut found: Vec<(String, usize)> = all
        .iter()
        .filter_map(|path| {
            let calls = withdrawals(&std::fs::read_to_string(path).expect("a UTF-8 source"));
            let name = path
                .strip_prefix(&root)
                .expect("under the crates directory")
                .to_string_lossy()
                .replace('\\', "/");
            (calls > 0).then_some((name, calls))
        })
        .collect();
    let expected: Vec<(String, usize)> = allowed
        .iter()
        .map(|(name, calls)| ((*name).to_owned(), *calls))
        .collect();
    found.sort();
    assert_eq!(
        found, expected,
        "provider withdrawals in product sources; each must append an event to the object's \
         stream (systems/ekr/domains/store.yaml, held bytes)"
    );
    assert_eq!(
        withdrawals("        provider.redact(&stream, 1, \"why\").await?; // .delete_blob("),
        1,
        "the guard finds a call, and not one in a comment"
    );
    assert_eq!(
        withdrawals(
            "    fn redact<'a>(&self) {}\n    let redacted = event.is_redacted();\n    \"delete_blob\";"
        ),
        0,
        "a definition, a longer word and a string are not calls"
    );
}

/// Adversary pass c7-s on the guard above: it counts `.redact(` and `.delete_blob(` as text, so a
/// withdrawal written as a qualified call — the form a generic helper over `EventStore` takes — or
/// as the provider's `forget_tenant`, which removes every event and blob of the tenant and leaves
/// no stream to append the event to, is not counted, and a new product source calling one passes
/// the guard.
#[test]
fn adversary_c7_s_the_withdrawal_guard_counts_every_form_of_a_provider_withdrawal() {
    let forms = [
        "        EventStore::redact(&provider, &stream, 1, \"why\").await?;",
        "        <P as EventStore>::delete_blob(&provider, &tenant, &digest).await?;",
        "        provider.forget_tenant(&tenant).await?;",
    ];
    let missed: Vec<&str> = forms
        .iter()
        .filter(|form| withdrawals(form) == 0)
        .map(|form| form.trim())
        .collect();
    assert!(
        missed.is_empty(),
        "the guard counts no withdrawal in: {missed:#?}"
    );
}
