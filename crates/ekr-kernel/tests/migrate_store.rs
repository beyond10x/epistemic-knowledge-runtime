//! `story:seed-envelope-v3-references-payloads` and `task:object-payloads-belong-in-provider-blobs`:
//! the preserving migration (design §§ 89, 90, 100). A `/2` store written by the base kernel
//! (built at test time by `support/v2_store.rs`) becomes a claimed `/4` store at a new
//! path whose snapshot — nodes, edges, assertions, evidence ids and payload hashes — and whose
//! transaction decisions equal the source's; the source is left exactly as it was, so every original
//! object is still retained; and a legacy `ObjectStored` schema-1 inline object is carried as
//! schema-2 metadata plus a provider blob.
#[allow(dead_code)]
mod current_fixture;
#[allow(dead_code)]
#[path = "support/v2_store.rs"]
mod v2;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use current_fixture::{anchor, context, open};
use ekr_core::{ContentHash, RevisionNumber, Timestamp};
use ekr_kernel::{MigratedOccurrence, Runtime, StoreMigrationV1, TransactionState};
use eventlog_core::{CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId};

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

/// Every object the store's log says it stored, with its retained bytes.
fn objects(runtime: &Runtime) -> BTreeMap<ContentHash, Vec<u8>> {
    runtime
        .published_events()
        .unwrap()
        .into_iter()
        .filter(|event| event.name == "ekr.store.ObjectStored")
        .map(|event| {
            let hash: ContentHash = event.data["content_hash"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            (hash, runtime.content(&hash).unwrap().unwrap())
        })
        .collect()
}

#[test]
fn a_v2_store_migrates_to_a_claimed_v4_store_with_its_snapshot_and_decisions_and_the_source_unchanged(
) {
    for file in [false, true] {
        let source_directory = v2_store(file);
        let source_path = source_directory.path();
        let file_bytes = bytes_under(source_path);
        let source = existing(source_path, file);
        let events_before = source.published_events().unwrap();
        let objects_before = objects(&source);
        let before = source.read(None).unwrap();

        let destination_directory = tempfile::tempdir().unwrap();
        let destination = open(destination_directory.path(), file, context(), anchor());
        let report: StoreMigrationV1 = source.migrate_into(&destination).unwrap();
        assert_eq!(report.format, StoreMigrationV1::FORMAT);
        assert_eq!(report.format, "ekr.store-migration/1");
        assert_eq!(report.source_seed_hash, before.seed.seed_hash);

        // The destination is a /3 store.
        let after = destination.read(None).unwrap();
        assert_eq!(after.seed.seed_hash, report.destination_seed_hash);
        let envelope = destination.content(&after.seed.seed_hash).unwrap().unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&envelope).unwrap();
        assert_eq!(envelope["format"], "ekr-seed-envelope/4", "file={file}");

        // Its snapshot equals the source's: nodes, edges, assertions, evidence ids, payload hashes.
        assert_eq!(after.graph.nodes, before.graph.nodes, "file={file}");
        assert_eq!(after.graph.edges, before.graph.edges, "file={file}");
        assert_eq!(
            after.graph.assertions, before.graph.assertions,
            "file={file}"
        );
        assert_eq!(after.graph.evidence, before.graph.evidence, "file={file}");
        assert_eq!(after.seed_input, before.seed_input, "file={file}");
        let payloads: BTreeSet<ContentHash> = before
            .seed_input
            .evidence_payloads
            .keys()
            .copied()
            .collect();
        let named: BTreeSet<ContentHash> = envelope["input"]["evidence_payloads"]
            .as_array()
            .unwrap()
            .iter()
            .map(|hash| hash.as_str().unwrap().parse().unwrap())
            .collect();
        assert_eq!(named, payloads, "file={file}");
        for hash in &payloads {
            assert_eq!(
                destination.content(hash).unwrap(),
                source.content(hash).unwrap(),
                "file={file}"
            );
        }
        // And every retained decision, revision by revision, except the addresses that name the
        // seed envelope, which the report maps.
        assert_eq!(after.root.revision, before.root.revision);
        for (number, revision) in &before.revisions {
            let migrated = &after.revisions[number];
            assert_eq!(migrated.revision_id, revision.revision_id);
            assert_eq!(migrated.event_id, revision.event_id);
            assert_eq!(migrated.committed_at, revision.committed_at);
            assert_eq!(migrated.root.knowledge_root, revision.root.knowledge_root);
            assert_eq!(migrated.root.evidence_root, revision.root.evidence_root);
            assert_eq!(migrated.root.ontology_root, revision.root.ontology_root);
            assert_eq!(migrated.root.agent_root, revision.root.agent_root);
        }
        let states = |read: &ekr_kernel::VerifiedRead| -> Vec<_> {
            read.transactions
                .iter()
                .map(|(id, record)| (*id, record.state()))
                .collect()
        };
        assert_eq!(states(&after), states(&before), "file={file}");
        assert!(states(&after)
            .iter()
            .any(|(_, state)| *state == TransactionState::Stale));
        assert_eq!(
            report.occurrences.len(),
            destination
                .published_events()
                .unwrap()
                .iter()
                .filter(|event| event.stream_type == "ekr.revision")
                .count(),
        );

        // The source is exactly as it was: every original object still retained.
        assert_eq!(source.published_events().unwrap(), events_before);
        assert_eq!(objects(&source), objects_before);
        drop(source);
        if file {
            assert_eq!(bytes_under(source_path), file_bytes, "file provider bytes");
        }
        let source = existing(source_path, file);
        assert_eq!(source.head().unwrap(), Some(before.root));

        // A proposal record names nothing of the lineage and is carried byte for byte; every other
        // record names the seed envelope or an earlier root or record, and is derived again.
        for mapping in &report.occurrences {
            assert_eq!(
                mapping.source_record_hash == mapping.destination_record_hash,
                mapping.event == "ekr.kernel.TransactionProposed",
                "file={file}: {mapping:?}"
            );
        }
        // Every source object is in the destination too, unless the migration replaced it, in
        // which case the report names its replacement.
        let replaced: BTreeMap<ContentHash, ContentHash> = report
            .occurrences
            .iter()
            .filter(|mapping: &&MigratedOccurrence| {
                mapping.source_record_hash != mapping.destination_record_hash
            })
            .map(|mapping| (mapping.source_record_hash, mapping.destination_record_hash))
            .chain([(report.source_seed_hash, report.destination_seed_hash)])
            .collect();
        for (hash, bytes) in &objects_before {
            if let Some(replacement) = replaced.get(hash) {
                assert!(
                    destination.content(replacement).unwrap().is_some(),
                    "file={file}"
                );
            } else {
                assert_eq!(
                    destination.content(hash).unwrap().as_ref(),
                    Some(bytes),
                    "file={file}: {hash}"
                );
            }
        }
        // The address map itself is retained in the destination.
        let map = destination.content(&report.map_hash).unwrap().unwrap();
        let map: serde_json::Value = serde_json::from_slice(&map).unwrap();
        assert_eq!(map["format"], "ekr.store-migration/1");

        // A fresh open replaying the whole destination from its seed reaches the same head.
        drop(destination);
        let mut reopened = existing(destination_directory.path(), file);
        reopened.set_full_replay(true);
        assert_eq!(reopened.head().unwrap(), Some(after.root), "file={file}");

        // A destination that already holds a store is refused, and left as it was.
        let refused = source.migrate_into(&reopened).unwrap_err();
        assert!(
            refused
                .to_string()
                .contains("migrate-destination-not-empty"),
            "file={file}: {refused}"
        );
        assert_eq!(reopened.head().unwrap(), Some(after.root));
    }
}

const LEGACY: &[u8] = b"original inline object bytes";

/// Writes one schema-1 `ObjectStored` with its bytes inline and no blob binding, as a store from
/// before metadata-only objects holds it.
fn install_legacy(path: &Path, file: bool) -> ContentHash {
    let hash = ContentHash::of_bytes(LEGACY);
    let stream = StreamId::new(
        TenantId::new("ekr").unwrap(),
        "ekr.store.object",
        hash.to_hex(),
    )
    .unwrap();
    let body = serde_json::json!({
        "content_hash": hash,
        "storage_class": "Provenance",
        "byte_len": LEGACY.len(),
        "stored_at": Timestamp::from_millis(5),
        "bytes": LEGACY,
    });
    let event = NewEvent::new("ekr.store.ObjectStored", 1, body).unwrap();
    let meta = CommandMeta {
        idempotency_key: "legacy-inline-object".into(),
        request_hash: "legacy-inline-object".into(),
        subject: "ekr.test".into(),
        actor: "ekr.test".into(),
        request_id: "legacy-inline-object".into(),
        trace_id: "legacy-inline-object".into(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: time::OffsetDateTime::UNIX_EPOCH,
        claim: None,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        if file {
            let provider = eventlog_file::FileEventStore::open(path).await.unwrap();
            provider
                .append(&stream, Expected::NoStream, &[event], &meta)
                .await
                .unwrap();
        } else {
            let provider = eventlog_sqlite::SqliteEventStore::open(
                path.join("state.db").to_str().unwrap(),
                "ekr",
            )
            .await
            .unwrap();
            provider
                .append(&stream, Expected::NoStream, &[event], &meta)
                .await
                .unwrap();
        }
    });
    hash
}

#[test]
fn a_legacy_inline_object_is_carried_as_schema_two_metadata_and_a_blob_on_both_providers() {
    for file in [false, true] {
        let source_directory = v2_store(file);
        let hash = install_legacy(source_directory.path(), file);
        let file_bytes = bytes_under(source_directory.path());
        let source = existing(source_directory.path(), file);
        // The current read path still refuses it in the source, as before.
        assert!(source
            .content(&hash)
            .unwrap_err()
            .to_string()
            .contains("legacy inline records require migration"));

        let destination_directory = tempfile::tempdir().unwrap();
        let destination = open(destination_directory.path(), file, context(), anchor());
        let report = source.migrate_into(&destination).unwrap();
        assert_eq!(report.legacy_objects, [hash], "file={file}");
        assert_eq!(
            destination.content(&hash).unwrap().as_deref(),
            Some(LEGACY),
            "file={file}"
        );
        let stored: Vec<_> = destination
            .published_events()
            .unwrap()
            .into_iter()
            .filter(|event| {
                event.stream_type == "ekr.store.object" && event.stream_id == hash.to_hex()
            })
            .collect();
        assert_eq!(stored.len(), 1, "file={file}");
        assert_eq!(stored[0].schema_version, 2);
        assert!(stored[0].data.get("bytes").is_none());
        assert_eq!(stored[0].data["storage_class"], "Provenance");
        assert_eq!(stored[0].data["stored_at"], 5);

        drop(source);
        if file {
            assert_eq!(bytes_under(source_directory.path()), file_bytes);
        }
        let source = existing(source_directory.path(), file);
        assert_eq!(
            source.head().unwrap().unwrap().revision,
            RevisionNumber::new(2)
        );
    }
}
