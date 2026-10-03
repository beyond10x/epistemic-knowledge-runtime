//! `task:write-verbs-cost-most-of-an-ingest`: a store handle authorizes one publication
//! preparation once, and a preparation changed after that is authorized again and refused.
//!
//! A kernel command elects a preparation (`RevisionLog::prepare`), which authorizes it — a basis
//! and a candidate replay through the injected authority — before writing it, and then resumes it
//! (`RevisionLog::resume`), which reads the elected chain back before the native append. Reading
//! back bytes this handle already authorized repeated the same authorization twice more. What a
//! preparation's authorization reads besides its own bytes is fixed once it is elected: the
//! revision-stream prefix it names, the object-stream prefixes it names and the handle's tenant,
//! authority and ontology. So a read-back whose bytes hash to the payload address of a
//! preparation this handle authorized is that authorization's input exactly, and is not
//! authorized again; any other read-back is, as before.
//!
//! The first case counts the authority's replays across a resume. The second changes the
//! preparation between the two steps, with a second provider handle, the way a writer outside
//! this handle could: each change is refused by the name it was refused by before, and nothing is
//! appended to the revision stream.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ekr_core::{AgentId, ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, FileStore, NativePublicationRequest, Publication,
    PublicationCommandKey, PublicationCommandKind, PublicationObject, PublicationPreparationV1,
    RetainedHistory, RevisionLog, SqliteStore, StorageClass, StoreError,
};

const TENANT: &str = "ekr";

fn id<T: std::str::FromStr>(n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

/// Admits every history, as the store's own tests do, and counts how often it is asked to.
///
/// It stands in for the kernel only in this binary; no `src/` implements the port but the
/// kernel's (`crates/ekr/tests/story_contract.rs`).
struct Counting(Arc<AtomicUsize>);
impl CommitAuthority for Counting {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }
    fn replay(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    }
}

fn open(path: &Path, file: bool, replays: &Arc<AtomicUsize>) -> Box<dyn RevisionLog> {
    let authority = Counting(Arc::clone(replays));
    if file {
        Box::new(
            FileStore::file(&path.join("files"), TENANT, None)
                .unwrap()
                .under(authority),
        )
    } else {
        Box::new(
            SqliteStore::sqlite(&path.join("state.db"), TENANT, None)
                .unwrap()
                .under(authority),
        )
    }
}

fn object(bytes: &[u8]) -> PublicationObject {
    PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::EPOCH,
        bytes: bytes.to_vec(),
    }
}

fn decision(event_id: u64, payload: RevisionPayload, record: &[u8], at: u64) -> Publication {
    Publication {
        event: RevisionEvent {
            application: None,
            format: RevisionEvent::FORMAT.into(),
            event_id: id::<EventId>(event_id),
            record_hash: ContentHash::of_bytes(record),
            payload,
        },
        objects: BTreeMap::from([(ContentHash::of_bytes(record), object(record))]),
        expected_version: at,
    }
}

/// Seeds `store` through the preparation path, as the kernel does.
fn seed(store: &dyn RevisionLog) {
    let mut seed = decision(
        0x40,
        RevisionPayload::Seeded {
            revision_id: id::<RevisionId>(0x31),
            seed_hash: ContentHash::of_bytes(b"seed envelope"),
        },
        b"seed record",
        0,
    );
    seed.objects.insert(
        ContentHash::of_bytes(b"seed envelope"),
        object(b"seed envelope"),
    );
    let bootstrap = PublicationCommandKey {
        answer_id: None,
        kind: PublicationCommandKind::Bootstrap,
        transaction_id: None,
        predecessor_event_id: None,
        predecessor_record_hash: None,
    };
    let elected = store
        .prepare(
            &bootstrap,
            ContentHash::of_bytes(b"seed input"),
            &seed,
            None,
        )
        .unwrap();
    assert_eq!(store.resume(&elected), Ok(Appended::Written));
}

fn propose_key(transaction: u64) -> PublicationCommandKey {
    PublicationCommandKey {
        answer_id: None,
        kind: PublicationCommandKind::Propose,
        transaction_id: Some(id(transaction)),
        predecessor_event_id: None,
        predecessor_record_hash: None,
    }
}

fn proposed(transaction: u64, event: u64, record: &[u8], at: u64) -> Publication {
    decision(
        event,
        RevisionPayload::TransactionProposed {
            transaction_id: id(transaction),
            proposer: id::<AgentId>(0x03),
            operations_hash: None,
        },
        record,
        at,
    )
}

fn elect(store: &dyn RevisionLog, transaction: u64, at: u64) -> PublicationPreparationV1 {
    store
        .prepare(
            &propose_key(transaction),
            ContentHash::of_bytes(format!("input {transaction}").as_bytes()),
            &proposed(transaction, 0x41 + transaction, b"proposal record", at),
            None,
        )
        .unwrap()
}

#[test]
fn a_preparation_this_handle_authorized_is_not_authorized_again_when_read_back_identical() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let replays = Arc::new(AtomicUsize::new(0));
        let store = open(directory.path(), file, &replays);
        seed(&*store);

        let before = replays.load(Ordering::SeqCst);
        let elected = elect(&*store, 0x51, 1);
        let electing = replays.load(Ordering::SeqCst) - before;
        if electing != 2 {
            wrong.push(format!(
                "file={file}: electing replayed {electing} times, not a basis and a candidate"
            ));
        }
        let before = replays.load(Ordering::SeqCst);
        let resumed = store.resume(&elected);
        let resuming = replays.load(Ordering::SeqCst) - before;
        if resumed != Ok(Appended::Written) {
            wrong.push(format!("file={file}: resume returned {resumed:?}"));
        }
        if resuming != 0 {
            wrong.push(format!(
                "file={file}: resuming the preparation this handle authorized replayed \
                 {resuming} more times"
            ));
        }
        drop(store);

        // A handle that did not authorize a preparation authorizes it when it resumes it.
        let store = open(directory.path(), file, &replays);
        let elected = elect(&*store, 0x52, 2);
        drop(store);
        let store = open(directory.path(), file, &replays);
        let before = replays.load(Ordering::SeqCst);
        let resumed = store.resume(&elected);
        let resuming = replays.load(Ordering::SeqCst) - before;
        if resumed != Ok(Appended::Written) {
            wrong.push(format!("file={file}: a fresh resume returned {resumed:?}"));
        }
        if resuming < 2 {
            wrong.push(format!(
                "file={file}: a handle that never authorized the preparation replayed \
                 {resuming} times resuming it"
            ));
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// A second native handle on the same store, as a writer outside the store handle has.
fn native(
    path: &Path,
    file: bool,
) -> (tokio::runtime::Runtime, Box<dyn eventlog_core::EventStore>) {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let provider: Box<dyn eventlog_core::EventStore> = executor.block_on(async {
        if file {
            Box::new(
                eventlog_file::FileEventStore::open(&path.join("files"))
                    .await
                    .unwrap(),
            ) as Box<dyn eventlog_core::EventStore>
        } else {
            Box::new(
                eventlog_sqlite::SqliteEventStore::open(
                    &path.join("state.db").to_string_lossy(),
                    "ekr",
                )
                .await
                .unwrap(),
            )
        }
    });
    (executor, provider)
}

fn preparation_hash(prepared: &PublicationPreparationV1) -> ContentHash {
    ContentHash::of_bytes(&serde_json::to_vec(prepared).unwrap())
}

fn private_key(hash: ContentHash) -> String {
    format!("ekr.private.preparation.{hash}")
}

fn slot(key: &PublicationCommandKey) -> String {
    ContentHash::of_bytes(&serde_json::to_vec(key).unwrap()).to_hex()
}

/// The provider's own fingerprint of a retained native request.
fn fingerprint(request: &NativePublicationRequest) -> String {
    use eventlog_core::{
        AppendGroup, BlobAppendGroup, BlobWrite, CommandMeta, Expected, NewEvent, StreamAppend,
        StreamId, TenantId,
    };
    let appends = request
        .appends
        .iter()
        .map(|append| StreamAppend {
            stream: StreamId::new(
                TenantId::new(append.stream.tenant.clone()).unwrap(),
                &append.stream.stream_type,
                &append.stream.stream_id,
            )
            .unwrap(),
            expected: match append.expected.version {
                Some(version) => Expected::Exact(version),
                None => Expected::NoStream,
            },
            events: append
                .events
                .iter()
                .map(|event| {
                    NewEvent::new(
                        &event.name,
                        event.schema_version,
                        serde_json::from_slice(&event.data).unwrap(),
                    )
                    .unwrap()
                })
                .collect(),
        })
        .collect();
    let meta = &request.meta;
    let nanos: i128 = meta.occurred_at_unix_nanos.parse().unwrap();
    BlobAppendGroup {
        group: AppendGroup {
            tenant: TenantId::new(request.tenant.clone()).unwrap(),
            appends,
            meta: CommandMeta {
                idempotency_key: meta.idempotency_key.clone(),
                request_hash: meta.request_hash.clone(),
                subject: meta.subject.clone(),
                actor: meta.actor.clone(),
                request_id: meta.request_id.clone(),
                trace_id: meta.trace_id.clone(),
                causation_id: meta.causation_id.clone(),
                causation_depth: meta.causation_depth,
                occurred_at: time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
                    .unwrap()
                    .to_offset(
                        time::UtcOffset::from_whole_seconds(meta.occurred_at_offset_seconds)
                            .unwrap(),
                    ),
                claim: None,
            },
        },
        blobs: request
            .blobs
            .iter()
            .map(|blob| BlobWrite {
                digest: blob.digest.clone(),
                bytes: blob.bytes.clone(),
            })
            .collect(),
    }
    .fingerprint()
    .unwrap()
}

/// Appends `forged` to its slot as the successor of `elected`, bytes and selection, through a
/// second provider handle.
fn install_successor(path: &Path, file: bool, forged: &PublicationPreparationV1) {
    use eventlog_core::{CommandMeta, Expected, NewEvent, StreamId, TenantId};
    let bytes = serde_json::to_vec(forged).unwrap();
    let hash = ContentHash::of_bytes(&bytes);
    let tenant = TenantId::new(TENANT).unwrap();
    let (executor, provider) = native(path, file);
    executor.block_on(async {
        provider
            .put_blob(&tenant, &private_key(hash), &bytes)
            .await
            .unwrap();
        let stream =
            StreamId::new(tenant.clone(), "ekr.preparation", slot(&forged.command_key)).unwrap();
        let key = format!("forged.{hash}");
        let meta = CommandMeta {
            idempotency_key: key.clone(),
            request_hash: key.clone(),
            subject: "forger".into(),
            actor: "forger".into(),
            request_id: key.clone(),
            trace_id: key,
            causation_id: None,
            causation_depth: 0,
            occurred_at: time::OffsetDateTime::UNIX_EPOCH,
            claim: None,
        };
        let event = NewEvent::new(
            "ekr.store.PublicationPrepared",
            1,
            serde_json::json!({
                "preparation_hash": hash,
                "attempt_number": forged.attempt_number,
                "previous_attempt_hash": forged.previous_attempt_hash,
            }),
        )
        .unwrap();
        provider
            .append(
                &stream,
                Expected::Exact(forged.attempt_number),
                &[event],
                &meta,
            )
            .await
            .unwrap();
    });
}

#[test]
fn a_preparation_changed_after_this_handle_authorized_it_is_authorized_again_and_refused() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        for mode in ["fingerprint", "metadata"] {
            let label = format!("file={file} {mode}");
            let directory = tempfile::tempdir().unwrap();
            let replays = Arc::new(AtomicUsize::new(0));
            let store = open(directory.path(), file, &replays);
            seed(&*store);
            let elected = elect(&*store, 0x51, 1);
            let retained = store.history().unwrap();

            // A successor attempt the chain admits, whose native request this handle never
            // authorized: either its retained fingerprint is not the request's, or its request
            // carries different writer metadata under the provider's own fingerprint of it.
            let mut forged = elected.clone();
            forged.attempt_number = 1;
            forged.previous_attempt_hash = Some(preparation_hash(&elected));
            let expected = match mode {
                "fingerprint" => {
                    forged.native_fingerprint = ContentHash::of_bytes(b"another request").to_hex();
                    "preparation-fingerprint"
                }
                _ => {
                    forged.native_request.meta.actor = "forger".into();
                    forged.native_fingerprint = fingerprint(&forged.native_request);
                    "preparation-native-metadata"
                }
            };
            install_successor(directory.path(), file, &forged);

            let resumed = store.resume(&elected);
            if !matches!(&resumed, Err(StoreError::Document(code)) if code == expected) {
                wrong.push(format!(
                    "{label}: resuming the authorized attempt returned {resumed:?}, not \
                     {expected}"
                ));
            }
            let resumed = store.resume(&forged);
            if !matches!(&resumed, Err(StoreError::Document(code)) if code == expected) {
                wrong.push(format!(
                    "{label}: resuming the changed attempt returned {resumed:?}, not {expected}"
                ));
            }
            match store.history() {
                Ok(history) if history == retained => {}
                Ok(history) => wrong.push(format!(
                    "{label}: the revision stream moved to {} occurrences",
                    history.occurrences.len()
                )),
                Err(error) => wrong.push(format!("{label}: history no longer reads: {error:?}")),
            }
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
