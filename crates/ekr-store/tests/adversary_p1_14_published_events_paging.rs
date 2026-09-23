//! Adversary pass 2 on `story:ess-conformance-kernel`: `EventlogStore::published_events` pages
//! the provider feed `MAX_READ_LIMIT` (1000) events at a time. No existing case writes more than
//! one page, so a read that returned after the first page stayed green. These cases write one
//! event past the page on each provider and require every one back, in order, exactly once.

use ekr_core::Timestamp;
use ekr_store::{FileStore, ObjectStore, PublishedEvent, SqliteStore, StorageClass};
use tempfile::TempDir;

const PAST_ONE_PAGE: usize = 1001;

fn fill<S: ObjectStore>(store: &S) {
    for n in 0..PAST_ONE_PAGE {
        store
            .put(
                StorageClass::Canonical,
                format!("object {n}").as_bytes(),
                Timestamp::from_millis(i64::try_from(n).unwrap()),
            )
            .expect("put");
    }
}

fn check(events: &[PublishedEvent]) {
    assert_eq!(
        events.len(),
        PAST_ONE_PAGE,
        "published_events returned {} of the {PAST_ONE_PAGE} logged events",
        events.len()
    );
    assert!(
        events.windows(2).all(|w| w[0].position < w[1].position),
        "positions do not strictly ascend across the page boundary"
    );
    assert!(events.iter().all(|e| e.name == "ekr.store.ObjectStored"));
}

#[test]
fn the_file_log_reads_back_past_one_feed_page() {
    let directory = TempDir::new().unwrap();
    let store = FileStore::file(&directory.path().join("store"), "ekr", None).expect("opens");
    fill(&store);
    check(&store.published_events().expect("the file log reads"));
}

#[test]
fn the_sqlite_log_reads_back_past_one_feed_page() {
    let directory = TempDir::new().unwrap();
    let store =
        SqliteStore::sqlite(&directory.path().join("store.db"), "ekr", None).expect("opens");
    fill(&store);
    check(&store.published_events().expect("the SQLite log reads"));
}
