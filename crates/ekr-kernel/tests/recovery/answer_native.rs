//! Process-death probe using the providers' public inline-projector transaction boundary.
//! No production failpoint or modified dependency is involved. The selected kernel preparation
//! is persisted before this child opens the provider and submits that exact native request.
use eventlog_core::{
    AtomicBlobEventStore, BlobAppendGroup, BoxFuture, EventLogError, EventStore, ProjectionSpec,
    ProjectionStore, Projector, RecordedEvent,
};
use std::path::Path;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn witness(path: &Path, value: &serde_json::Value) {
    let path = path.join("native-witness.json");
    std::fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
    std::fs::File::open(path).unwrap().sync_all().unwrap();
}

struct ExitInside {
    path: std::path::PathBuf,
    request: BlobAppendGroup,
    stop: usize,
    seen: AtomicUsize,
}
impl Projector for ExitInside {
    fn name(&self) -> &'static str {
        "answer_crash_probe"
    }
    fn projections(&self) -> &'static [ProjectionSpec] {
        &[]
    }
    fn apply<'a>(
        &'a self,
        event: &'a RecordedEvent,
        store: &'a mut dyn ProjectionStore,
    ) -> BoxFuture<'a, Result<(), EventLogError>> {
        Box::pin(async move {
            let index = self.seen.fetch_add(1, Ordering::SeqCst) + 1;
            // This read takes place inside the same native transaction as its event writes.
            // A child can only reach the deliberate exit after verifying the staged blobs.
            for blob in &self.request.blobs {
                assert_eq!(
                    store.get_blob(&blob.digest).await?,
                    Some(blob.bytes.clone())
                );
            }
            if index == self.stop {
                witness(
                    &self.path,
                    &serde_json::json!({"point":"inside", "index":index, "event":event}),
                );
                std::process::exit(86);
            }
            Ok(())
        })
    }
}

async fn submit<S: AtomicBlobEventStore>(
    store: S,
    path: &Path,
    point: &str,
    request: BlobAppendGroup,
) {
    let count = request
        .group
        .appends
        .iter()
        .map(|a| a.events.len())
        .sum::<usize>();
    assert!(count > 1, "must exercise partial multi-event publication");
    if point == "before" {
        witness(path, &serde_json::json!({"point":"before"}));
        std::process::exit(86);
    }
    if matches!(point, "first" | "last") {
        store
            .register_inline(Arc::new(ExitInside {
                path: path.into(),
                request: request.clone(),
                stop: if point == "first" { 1 } else { count },
                seen: AtomicUsize::new(0),
            }))
            .await
            .unwrap();
    }
    let result = store.append_group_with_blobs(&request).await.unwrap();
    assert!(!result.deduplicated);
    assert_eq!(
        point, "after",
        "inline transaction boundary did not execute"
    );
    witness(
        path,
        &serde_json::json!({"point":"after", "events":result.appends.iter().flat_map(|a| &a.events).collect::<Vec<_>>()}),
    );
    std::process::exit(86);
}

pub fn child(path: &Path) {
    let prepared: ekr_store::PublicationPreparationV1 =
        serde_json::from_slice(&std::fs::read(path.join("elected.json")).unwrap()).unwrap();
    let request = super::recovery::native_request(&prepared.native_request);
    assert_eq!(request.fingerprint().unwrap(), prepared.native_fingerprint);
    let point = std::env::var("EKR_ANSWER_NATIVE_POINT").unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    executor.block_on(async {
        if std::env::var("EKR_ANSWER_NATIVE_FILE").unwrap() == "true" {
            submit(
                eventlog_file::FileEventStore::open(path).await.unwrap(),
                path,
                &point,
                request,
            )
            .await;
        } else {
            submit(
                eventlog_sqlite::SqliteEventStore::open(
                    &path.join("store.db").to_string_lossy(),
                    "ekr",
                )
                .await
                .unwrap(),
                path,
                &point,
                request,
            )
            .await;
        }
    });
    panic!("native child must terminate at the requested boundary");
}

pub fn crash(path: &Path, file: bool, point: &str, prepared: &ekr_store::PublicationPreparationV1) {
    let before = capture(path, file, prepared);
    std::fs::write(
        path.join("elected.json"),
        serde_json::to_vec(prepared).unwrap(),
    )
    .unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "answer_native_process_death_resumes_without_partial_corrections",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("EKR_ANSWER_NATIVE_PATH", path)
        .env("EKR_ANSWER_NATIVE_POINT", point)
        .env("EKR_ANSWER_NATIVE_FILE", file.to_string())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(86),
        "file={file} point={point}: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let witness: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path.join("native-witness.json")).unwrap()).unwrap();
    if matches!(point, "first" | "last") {
        assert_eq!(witness["point"], "inside");
        let count: usize = prepared
            .native_request
            .appends
            .iter()
            .map(|a| a.events.len())
            .sum();
        assert_eq!(witness["index"], if point == "first" { 1 } else { count });
    } else {
        assert_eq!(witness["point"], point);
    }
    let after = capture(path, file, prepared);
    if point == "after" {
        let events = after["events"].as_array().unwrap();
        let old = before["events"].as_array().unwrap();
        assert_eq!(&events[..old.len()], old);
        let actual = &events[old.len()..];
        assert_eq!(actual, witness["events"].as_array().unwrap());
        let expected = prepared
            .native_request
            .appends
            .iter()
            .flat_map(|a| a.events.iter());
        assert_eq!(actual.len(), expected.clone().count());
        for (event, expected) in actual.iter().zip(expected) {
            assert_eq!(event["name"], expected.name);
            assert_eq!(
                event["data"],
                serde_json::from_slice::<serde_json::Value>(&expected.data).unwrap()
            );
        }
        for blob in &prepared.native_request.blobs {
            assert_eq!(after["blobs"][&blob.digest], serde_json::json!(blob.bytes));
        }
    } else {
        assert_eq!(
            after, before,
            "partial native group must expose neither events nor new blob bindings"
        );
    }
}

/// Read physical event coordinates and all requested blob bindings from independent handles.
pub fn capture(
    path: &Path,
    file: bool,
    prepared: &ekr_store::PublicationPreparationV1,
) -> serde_json::Value {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    executor.block_on(async {
        let provider: Box<dyn EventStore> = if file {
            Box::new(eventlog_file::FileEventStore::open(path).await.unwrap())
        } else {
            Box::new(
                eventlog_sqlite::SqliteEventStore::open(
                    &path.join("store.db").to_string_lossy(),
                    "ekr",
                )
                .await
                .unwrap(),
            )
        };
        let tenant = eventlog_core::TenantId::new(&prepared.native_request.tenant).unwrap();
        let mut events = Vec::new();
        let mut after = 0;
        loop {
            let page = provider.read_feed(&tenant, after, 100).await.unwrap();
            events.extend(page.events);
            if !page.has_more {
                break;
            }
            after = page.next_position;
        }
        let mut blobs = std::collections::BTreeMap::new();
        for blob in &prepared.native_request.blobs {
            blobs.insert(
                &blob.digest,
                provider.get_blob(&tenant, &blob.digest).await.unwrap(),
            );
        }
        serde_json::json!({"events":events,"blobs":blobs})
    })
}
