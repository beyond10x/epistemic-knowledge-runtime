//! `EventlogStore::published_events`: the provider log's published events, read back in log order
//! through the store's own provider handle, on both providers.
//!
//! Wave p1-14, adversary pass 1 finding 1 on `story:ess-conformance-kernel`: the conformance target
//! must report the store's own events, and the SQLite provider's history inspector refuses every
//! database the SQLite store writes (WAL mode). So the store reads its own log. These cases hold
//! what that read promises: every event the tenant's log holds, in the order it was written, with
//! stream, version and payload as the provider recorded them. Reading changes nothing, and another
//! tenant's events never appear.

use ekr_core::{ContentHash, Timestamp};
use ekr_store::{FileStore, ObjectStore, PublishedEvent, SqliteStore, StorageClass};
use tempfile::TempDir;

fn file(directory: &TempDir, tenant: &str) -> FileStore {
    FileStore::file(&directory.path().join("store"), tenant, None).expect("file provider opens")
}

fn sqlite(directory: &TempDir, tenant: &str) -> SqliteStore {
    SqliteStore::sqlite(&directory.path().join("store.db"), tenant, None)
        .expect("SQLite provider opens")
}

/// Two objects stored and one retention raised, then the log read three ways.
fn the_log_is_every_published_event_in_order<S, F>(open: F)
where
    S: ObjectStore,
    F: Fn(&str) -> S,
    for<'a> &'a S: Published,
{
    let store = open("ekr");
    assert_eq!(
        (&store).published(),
        Vec::<PublishedEvent>::new(),
        "an empty log publishes nothing"
    );

    let first = b"first object";
    let second = b"second object";
    store
        .put(StorageClass::Canonical, first, Timestamp::from_millis(1))
        .expect("first put");
    store
        .put(StorageClass::Ephemeral, second, Timestamp::from_millis(2))
        .expect("second put");
    store
        .put(StorageClass::Canonical, second, Timestamp::from_millis(3))
        .expect("retention raised");

    let events: Vec<PublishedEvent> = (&store).published();
    let shape: Vec<(&str, &str, u64)> = events
        .iter()
        .map(|e| (e.name.as_str(), e.stream_id.as_str(), e.version))
        .collect();
    let (a, b) = (
        ContentHash::of_bytes(first).to_hex(),
        ContentHash::of_bytes(second).to_hex(),
    );
    assert_eq!(
        shape,
        vec![
            ("ekr.store.ObjectStored", a.as_str(), 1),
            ("ekr.store.ObjectStored", b.as_str(), 1),
            ("ekr.store.ObjectRetentionRaised", b.as_str(), 2),
        ],
        "the log, in the order it was written"
    );
    assert!(events.iter().all(|e| e.stream_type == "ekr.store.object"));
    assert!(
        events.windows(2).all(|w| w[0].position < w[1].position),
        "positions ascend: {events:?}"
    );
    assert_eq!(events[0].schema_version, 2);
    assert_eq!(events[0].data["content_hash"], a);
    assert_eq!(events[0].data["byte_len"], first.len());
    assert_eq!(events[2].schema_version, 1);
    assert_eq!(events[2].data["from"], "Ephemeral");
    assert_eq!(events[2].data["to"], "Canonical");

    assert_eq!(
        (&store).published(),
        events,
        "reading again changes nothing"
    );
    drop(store);
    assert_eq!(
        (&open("ekr")).published(),
        events,
        "a reopened provider reads the same log back"
    );

    let other = open("other-tenant");
    other
        .put(StorageClass::Canonical, b"another tenant", Timestamp::EPOCH)
        .expect("other tenant put");
    assert_eq!((&other).published().len(), 1);
    drop(other);
    assert_eq!(
        (&open("ekr")).published(),
        events,
        "another tenant's events never appear"
    );
}

/// The one method under test, named once per provider type.
trait Published {
    fn published(self) -> Vec<PublishedEvent>;
}
impl Published for &FileStore {
    fn published(self) -> Vec<PublishedEvent> {
        self.published_events().expect("the file log reads")
    }
}
impl Published for &SqliteStore {
    fn published(self) -> Vec<PublishedEvent> {
        self.published_events().expect("the SQLite log reads")
    }
}

#[test]
fn the_file_log_is_every_published_event_in_order() {
    let directory = TempDir::new().unwrap();
    the_log_is_every_published_event_in_order(|tenant| file(&directory, tenant));
}

#[test]
fn the_sqlite_log_is_every_published_event_in_order() {
    let directory = TempDir::new().unwrap();
    the_log_is_every_published_event_in_order(|tenant| sqlite(&directory, tenant));
}
