//! Test support only: stores seeded as the kernel at base `36e87d41` (release 0.0.14, whose layout
//! 0.0.11 already wrote) seeded one, built at test time on either provider.
//!
//! The kernel no longer writes `ekr-seed-envelope/2` or elects an evidence-staging decision in
//! `ekr.publication-preparation/2` (design § 100). This writer does both exactly as
//! `36e87d41:crates/ekr-kernel/src/commit.rs` (`Commit::seed`) and
//! `36e87d41:crates/ekr-store/src/preparation.rs` (`elect_preparation`, `native_for`) did: the
//! `/2` envelope carrying the complete input, its seed result, the payloads as Provenance objects,
//! the native request and fingerprint, and the preparation's own atomic group, written straight
//! through the provider. It then hands the store to the real kernel, whose `seed` finds that
//! pending `/2` preparation, authorizes it through its own replay and publishes it — the path an
//! interrupted base seed resumes by — so every retained byte of the seed is the base's and the
//! kernel admitted it. Nothing here is part of any crate's public surface.
use std::path::Path;

use ekr_core::{Canonical, ContentHash, Encoder, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_kernel::{
    AuthorityStateV1, BootstrapContext, CommitCommandResult, Runtime, SeedDocument, SeedResultV1,
    ValidationCommandResult,
};
use ekr_store::{
    NativeCommandMeta, NativeExpected, NativeExpectedKind, NativeNewEvent,
    NativePublicationRequest, NativeStreamAppend, NativeStreamId, Publication,
    PublicationCommandKey, PublicationCommandKind, PublicationObject, PublicationPreparationV1,
    StorageClass,
};
use eventlog_core::{
    AppendGroup, AtomicBlobEventStore, BlobAppendGroup, BlobWrite, CommandMeta, Expected, NewEvent,
    StreamAppend, StreamId, TenantId,
};

const TENANT: &str = "ekr";

/// The `ekr-seed-envelope/2` bytes `36e87d41` retained for `document`: the complete input, its
/// payloads carried as number arrays.
pub fn envelope_v2(
    document: &SeedDocument,
    context: BootstrapContext,
    anchor: &AuthorityStateV1,
    committed_at: Timestamp,
) -> Vec<u8> {
    #[derive(serde::Serialize)]
    struct Envelope<'a> {
        format: &'a str,
        input: &'a SeedDocument,
        context: BootstrapContext,
        authority: &'a AuthorityStateV1,
        committed_at: Timestamp,
    }
    serde_json::to_vec(&Envelope {
        format: "ekr-seed-envelope/2",
        input: document,
        context,
        authority: anchor,
        committed_at,
    })
    .unwrap()
}

/// `36e87d41:crates/ekr-kernel/src/commands.rs`'s `input_hash("Seed", …)` for `document`.
fn seed_input_hash(
    document: &SeedDocument,
    context: BootstrapContext,
    anchor: &AuthorityStateV1,
) -> ContentHash {
    struct Input<'a> {
        material: &'a [u8],
        context: BootstrapContext,
        anchor: &'a AuthorityStateV1,
    }
    impl Canonical for Input<'_> {
        fn encode(&self, out: &mut Encoder) {
            "ekr.publication-input/1".encode(out);
            "Seed".encode(out);
            self.material.encode(out);
            self.context.operator.encode(out);
            self.context.operator.encode(out);
            self.context.validator.encode(out);
            self.anchor.encode(out);
        }
    }
    let material = serde_json::to_vec(document).unwrap();
    ContentHash::of(&Input {
        material: &material,
        context,
        anchor,
    })
}

/// `36e87d41:crates/ekr-store/src/eventlog.rs`'s `envelope`: the native metadata of a group.
fn meta(key: &str, hash: String) -> CommandMeta {
    CommandMeta {
        idempotency_key: key.into(),
        request_hash: hash,
        subject: "ekr.store".into(),
        actor: "ekr.store".into(),
        request_id: key.into(),
        trace_id: key.into(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: time::OffsetDateTime::UNIX_EPOCH,
        claim: None,
    }
}

fn stream(stream_type: &str, stream_id: &str) -> StreamId {
    StreamId::new(TenantId::new(TENANT).unwrap(), stream_type, stream_id).unwrap()
}

/// `36e87d41`'s `native_for(decision, 0)` on a store that holds nothing: the revision append, one
/// `ObjectStored`/schema-2 append per object, and every object's bytes as a blob binding.
fn native_for(decision: &Publication) -> BlobAppendGroup {
    let mut appends = vec![StreamAppend {
        stream: stream("ekr.revision", "canonical"),
        expected: Expected::NoStream,
        events: vec![NewEvent::new(
            decision.event.name(),
            2,
            serde_json::to_value(&decision.event).unwrap(),
        )
        .unwrap()],
    }];
    for (hash, object) in &decision.objects {
        appends.push(StreamAppend {
            stream: stream("ekr.store.object", &hash.to_hex()),
            expected: Expected::NoStream,
            events: vec![NewEvent::new(
                "ekr.store.ObjectStored",
                2,
                serde_json::json!({
                    "content_hash": hash,
                    "storage_class": object.storage_class,
                    "byte_len": object.bytes.len() as u64,
                    "stored_at": object.stored_at,
                }),
            )
            .unwrap()],
        });
    }
    BlobAppendGroup {
        group: AppendGroup {
            tenant: TenantId::new(TENANT).unwrap(),
            appends,
            meta: meta(
                &format!("ekr.occurrence.{}.0", decision.event.event_id),
                ContentHash::of(&decision.event).to_hex(),
            ),
        },
        blobs: decision
            .objects
            .iter()
            .map(|(hash, object)| BlobWrite {
                digest: hash.to_hex(),
                bytes: object.bytes.clone(),
            })
            .collect(),
    }
}

/// `36e87d41`'s `NativePublicationRequest::capture`.
fn capture(request: &BlobAppendGroup) -> NativePublicationRequest {
    NativePublicationRequest {
        tenant: TENANT.into(),
        appends: request
            .group
            .appends
            .iter()
            .map(|append| NativeStreamAppend {
                stream: NativeStreamId {
                    tenant: TENANT.into(),
                    stream_type: append.stream.stream_type().into(),
                    stream_id: append.stream.stream_id().into(),
                },
                expected: NativeExpected {
                    kind: NativeExpectedKind::NoStream,
                    version: None,
                },
                events: append
                    .events
                    .iter()
                    .map(|event| NativeNewEvent {
                        name: event.name.clone(),
                        schema_version: event.schema_version,
                        data: serde_json::to_vec(&event.data).unwrap(),
                    })
                    .collect(),
            })
            .collect(),
        meta: NativeCommandMeta {
            idempotency_key: request.group.meta.idempotency_key.clone(),
            request_hash: request.group.meta.request_hash.clone(),
            subject: "ekr.store".into(),
            actor: "ekr.store".into(),
            request_id: request.group.meta.request_id.clone(),
            trace_id: request.group.meta.trace_id.clone(),
            causation_id: None,
            causation_depth: 0,
            occurred_at_unix_nanos: "0".into(),
            occurred_at_offset_seconds: 0,
            claim: None,
        },
        blobs: request
            .blobs
            .iter()
            .map(|blob| ekr_store::NativeBlobWrite {
                digest: blob.digest.clone(),
                bytes: blob.bytes.clone(),
            })
            .collect(),
    }
}

async fn append_group(path: &Path, file: bool, group: &BlobAppendGroup) {
    if file {
        let provider = eventlog_file::FileEventStore::open(path).await.unwrap();
        AtomicBlobEventStore::append_group_with_blobs(&provider, group)
            .await
            .unwrap();
    } else {
        let provider = eventlog_sqlite::SqliteEventStore::open(
            path.join("state.db").to_str().unwrap(),
            TENANT,
        )
        .await
        .unwrap();
        AtomicBlobEventStore::append_group_with_blobs(&provider, group)
            .await
            .unwrap();
    }
}

fn existing(
    path: &Path,
    file: bool,
    context: BootstrapContext,
    anchor: AuthorityStateV1,
) -> Runtime {
    if file {
        Runtime::file_existing(path, TENANT, context, anchor)
    } else {
        Runtime::sqlite_existing(&path.join("state.db"), TENANT, context, anchor)
    }
    .unwrap()
}

/// Seeds an empty store at `path` with `document` exactly as `36e87d41` did at `committed_at`: an
/// `ekr-seed-envelope/2` elected in an `ekr.publication-preparation/2` attempt, published by the
/// real kernel. Returns the seed's result.
pub fn seed_v2(
    path: &Path,
    file: bool,
    document: &SeedDocument,
    context: BootstrapContext,
    anchor: &AuthorityStateV1,
    committed_at: Timestamp,
) -> SeedResultV1 {
    // The admitted roots of a seed are its input's, whatever envelope names it: take them from a
    // real seed of the same input in a scratch store, and name the `/2` envelope instead.
    let scratch = tempfile::tempdir().unwrap();
    let admitted = Runtime::file(scratch.path(), TENANT, context, anchor.clone())
        .unwrap()
        .seed(document.clone(), || committed_at)
        .unwrap();
    let bytes = envelope_v2(document, context, anchor, committed_at);
    let seed_hash = ContentHash::of_bytes(&bytes);
    let mut root = admitted.result;
    root.transaction = seed_hash;
    let record = SeedResultV1 {
        format: SeedResultV1::FORMAT.into(),
        event_id: EventId::mint(),
        revision_id: RevisionId::mint(),
        seed_hash,
        authority_root: root.agent_root,
        committed_at,
        result: root,
        result_hash: ContentHash::of(&root),
    };
    let record_bytes = record.to_bytes().unwrap();
    let record_hash = ContentHash::of_bytes(&record_bytes);
    let object = |storage_class, bytes| PublicationObject {
        storage_class,
        stored_at: committed_at,
        bytes,
    };
    let mut objects = std::collections::BTreeMap::new();
    objects.insert(seed_hash, object(StorageClass::Canonical, bytes));
    objects.insert(record_hash, object(StorageClass::Canonical, record_bytes));
    for (hash, payload) in &document.evidence_payloads {
        objects
            .entry(*hash)
            .or_insert_with(|| object(StorageClass::Provenance, payload.to_vec()));
    }
    let decision = Publication {
        event: RevisionEvent {
            application: None,
            format: RevisionEvent::FORMAT.into(),
            event_id: record.event_id,
            record_hash,
            payload: RevisionPayload::Seeded {
                revision_id: record.revision_id,
                seed_hash,
            },
        },
        objects,
        expected_version: 0,
    };
    let key = PublicationCommandKey {
        answer_id: None,
        kind: PublicationCommandKind::Bootstrap,
        transaction_id: None,
        predecessor_event_id: None,
        predecessor_record_hash: None,
    };
    let request = native_for(&decision);
    let prepared = PublicationPreparationV1 {
        format: "ekr.publication-preparation/2".into(),
        command_key: key.clone(),
        input_hash: seed_input_hash(document, context, anchor),
        decision,
        attempt_number: 0,
        previous_attempt_hash: None,
        native_fingerprint: request.fingerprint().unwrap(),
        native_request: capture(&request),
    };
    let prepared_bytes = serde_json::to_vec(&prepared).unwrap();
    let prepared_hash = ContentHash::of_bytes(&prepared_bytes);
    let slot = ContentHash::of_bytes(&serde_json::to_vec(&key).unwrap()).to_hex();
    let election = BlobAppendGroup {
        group: AppendGroup {
            tenant: TenantId::new(TENANT).unwrap(),
            appends: vec![StreamAppend {
                stream: stream("ekr.preparation", &slot),
                expected: Expected::NoStream,
                events: vec![NewEvent::new(
                    "ekr.store.PublicationPrepared",
                    1,
                    serde_json::json!({
                        "preparation_hash": prepared_hash,
                        "attempt_number": 0,
                        "previous_attempt_hash": null,
                    }),
                )
                .unwrap()],
            }],
            meta: meta(
                &format!("ekr.prepare.{slot}.{prepared_hash}"),
                prepared_hash.to_hex(),
            ),
        },
        blobs: vec![BlobWrite {
            digest: format!("ekr.private.preparation.{prepared_hash}"),
            bytes: prepared_bytes,
        }],
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(append_group(path, file, &election));
    // The kernel resumes the elected attempt: it samples no clock and returns its record.
    let result = existing(path, file, context, anchor.clone())
        .seed(document.clone(), || {
            panic!("the elected /2 attempt is resumed, not decided again")
        })
        .unwrap();
    assert_eq!(result, record);
    result
}

/// A `CreateNode` of `current_fixture`'s `Subject` type under its root, proposed by `operator`.
fn node(transaction: u64, node: u64, name: &str, operator: ekr_core::AgentId) -> String {
    let id = |n: u64| format!("00000000-0000-4000-8000-{n:012x}");
    format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {}\n  proposer: {operator}\n  \
         operations:\n  - !CreateNode\n    id: {}\n    root_id: {}\n    type_id: {}\n    \
         canonical_name: {name}\n    properties: {{}}\n  evidence: []\n",
        id(transaction),
        id(node),
        id(0x02),
        id(0x05),
    )
}

/// A `CreateEdge` of `current_fixture`'s `relates` type to a node no revision holds.
fn dangling_edge(transaction: u64, operator: ekr_core::AgentId) -> String {
    let id = |n: u64| format!("00000000-0000-4000-8000-{n:012x}");
    format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {}\n  proposer: {operator}\n  \
         operations:\n  - !CreateEdge\n    id: {}\n    root_id: {}\n    type_id: {}\n    \
         source: {}\n    target: {}\n    properties: {{}}\n  evidence: []\n",
        id(transaction),
        id(0x30),
        id(0x02),
        id(0x07),
        id(0x10),
        id(0x99),
    )
}

fn tx(n: u64) -> ekr_core::TransactionId {
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

/// A `/2` store of `current_fixture::seed()` at `path` holding every transaction state, as the
/// base kernel wrote it: seeded at `seeded_at`, then at trusted times 20–31 ms transactions `…601`
/// and `…602` (each one `CreateNode`) proposed and both validated against revision 0; `…601`
/// committed (revision 1) and `…602` committed into `Stale`; `…603` (a `CreateEdge` to an absent
/// node) proposed and `Rejected` against revision 1; `…605` proposed, validated against 1 and
/// committed (revision 2); `…604` proposed and left `Proposed`.
pub fn fixture_store(
    path: &Path,
    file: bool,
    document: &SeedDocument,
    context: BootstrapContext,
    anchor: &AuthorityStateV1,
    seeded_at: Timestamp,
) -> SeedResultV1 {
    let seeded = seed_v2(path, file, document, context, anchor, seeded_at);
    let runtime = existing(path, file, context, anchor.clone());
    let operator = context.operator;
    let at = |n: i64| move || Timestamp::from_millis(n);
    runtime
        .propose(
            node(0x601, 0x20, "third", operator).as_bytes(),
            operator,
            at(20),
        )
        .unwrap();
    runtime
        .propose(
            node(0x602, 0x21, "fourth", operator).as_bytes(),
            operator,
            at(21),
        )
        .unwrap();
    let (zero, one) = (RevisionNumber::SEED, RevisionNumber::new(1));
    assert!(matches!(
        runtime.validate(tx(0x601), zero, at(22)).unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        runtime.validate(tx(0x602), zero, at(23)).unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        runtime.commit(tx(0x601), operator, at(24)).unwrap(),
        CommitCommandResult::Committed(_)
    ));
    assert!(matches!(
        runtime.commit(tx(0x602), operator, at(25)).unwrap(),
        CommitCommandResult::Stale(_)
    ));
    runtime
        .propose(dangling_edge(0x603, operator).as_bytes(), operator, at(26))
        .unwrap();
    assert!(matches!(
        runtime.validate(tx(0x603), one, at(27)).unwrap(),
        ValidationCommandResult::Rejected(_)
    ));
    runtime
        .propose(
            node(0x605, 0x23, "sixth", operator).as_bytes(),
            operator,
            at(28),
        )
        .unwrap();
    assert!(matches!(
        runtime.validate(tx(0x605), one, at(29)).unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        runtime.commit(tx(0x605), operator, at(30)).unwrap(),
        CommitCommandResult::Committed(_)
    ));
    runtime
        .propose(
            node(0x604, 0x22, "fifth", operator).as_bytes(),
            operator,
            at(31),
        )
        .unwrap();
    seeded
}
