//! Corrupt address/length/retention metadata is refused before seed authority admission.
//! This is provider integrity coverage; semantic admission remains the real kernel's suite.
mod fixture;
use ekr_core::{ContentHash, RevisionId, Timestamp};
use ekr_graph::RevisionEvent;
use ekr_store::{FileStore, GraphDocument, ObjectStore, RevisionLog, SqliteStore};
use eventlog_core::{CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId};
use tempfile::TempDir;

trait Provider: RevisionLog + ObjectStore {}
impl<S: RevisionLog + ObjectStore> Provider for S {}

fn metadata() -> CommandMeta {
    CommandMeta {
        idempotency_key: "integrity-probe".to_owned(),
        request_hash: "integrity-probe".to_owned(),
        subject: "ekr.test".to_owned(),
        actor: "ekr.test".to_owned(),
        request_id: "integrity-probe".to_owned(),
        trace_id: "integrity-probe".to_owned(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: time::OffsetDateTime::UNIX_EPOCH,
        claim: None,
    }
}

fn run(file: bool) {
    for fault in ["control", "hash", "length", "bytes", "retention", "unknown"] {
        let directory = TempDir::new().unwrap();
        let ontology = fixture::ontology();
        let graph = fixture::seed_graph(&ontology);
        let bytes = GraphDocument::of(&graph).to_bytes().unwrap();
        let hash = ContentHash::of_bytes(&bytes);
        let mut body = serde_json::json!({
            "content_hash": hash, "storage_class": "Canonical", "byte_len": bytes.len(),
            "stored_at": Timestamp::EPOCH, "bytes": bytes,
        });
        let expected = match fault {
            "control" => "",
            "hash" => {
                body["content_hash"] =
                    serde_json::to_value(ContentHash::of_bytes(b"other")).unwrap();
                "object-integrity"
            }
            "length" => {
                body["byte_len"] = 1.into();
                "object-integrity"
            }
            "bytes" => {
                body["bytes"] = serde_json::json!([1, 2, 3]);
                "object-integrity"
            }
            "retention" => {
                body["storage_class"] = "Cache".into();
                "seed-retention"
            }
            "unknown" => {
                body["unexpected_rule"] = true.into();
                "unknown field"
            }
            _ => unreachable!(),
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let stream = StreamId::new(
            TenantId::new("ekr").unwrap(),
            "ekr.store.object",
            hash.to_hex(),
        )
        .unwrap();
        let event = NewEvent::new("ekr.store.ObjectStored", 1, body).unwrap();
        let provider: Box<dyn Provider> = if file {
            let raw = runtime
                .block_on(eventlog_file::FileEventStore::open(directory.path()))
                .unwrap();
            runtime
                .block_on(raw.append(&stream, Expected::NoStream, &[event], &metadata()))
                .unwrap();
            drop(raw);
            Box::new(
                FileStore::file(directory.path(), "ekr", ontology)
                    .unwrap()
                    .under(fixture::SeedOnly),
            )
        } else {
            let path = directory.path().join("state.db");
            let raw = runtime
                .block_on(eventlog_sqlite::SqliteEventStore::open(
                    path.to_str().unwrap(),
                    "ekr",
                ))
                .unwrap();
            runtime
                .block_on(raw.append(&stream, Expected::NoStream, &[event], &metadata()))
                .unwrap();
            drop(raw);
            Box::new(
                SqliteStore::sqlite(&path, "ekr", ontology)
                    .unwrap()
                    .under(fixture::SeedOnly),
            )
        };
        let _appended = provider
            .append(&RevisionEvent::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash: hash,
            })
            .unwrap();
        if fault == "control" {
            assert_eq!(provider.fold().unwrap(), graph);
        } else {
            let error = provider.fold().unwrap_err().to_string();
            assert!(error.contains(expected), "{fault}: {error}");
        }
    }
}

#[test]
fn sqlite_seed_objects_verify_address_length_and_retention_before_admission() {
    run(false);
}
#[test]
fn file_seed_objects_verify_address_length_and_retention_before_admission() {
    run(true);
}
