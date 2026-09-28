//! `story:graph-projection-renderer`: determinism is a property of the pure half. A revision
//! loaded once through `ekr_views::load` renders the same bytes on every call to
//! `ekr_views::render`, and those are the bytes `ekr_views::project` returns.

mod support;

use ekr_core::RevisionNumber;

use support::fixtures::{self, Fixture, Provider};

#[test]
fn a_loaded_revision_renders_the_same_bytes_every_time_and_the_same_as_project() {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::File);
    Fixture::Evolved.build(&runtime);
    let at = Some(RevisionNumber::new(3));

    let loaded: ekr_views::LoadedRevision = ekr_views::load(&runtime, at).expect("load");
    assert_eq!(loaded.graph.revision.get(), 3);
    assert_eq!(loaded.revisions.len(), 4);
    for (n, entry) in loaded.revisions.iter().enumerate() {
        let entry: &ekr_views::LoadedRevisionEntry = entry;
        assert_eq!(entry.number.get(), n as u64);
        assert_eq!(entry.transaction_id.is_none(), n == 0, "revision {n}");
        assert!(loaded.schemas.contains_key(&entry.schema_version));
    }

    let first: ekr_views::Rendered = ekr_views::render(&loaded).expect("first render");
    let second: ekr_views::Rendered = ekr_views::render(&loaded).expect("second render");
    let projected: ekr_views::Rendered = ekr_views::project(&runtime, at).expect("project");
    assert!(!first.bytes.is_empty());
    assert_eq!(
        first.bytes, second.bytes,
        "two renders of one loaded revision"
    );
    assert_eq!(first.summary, second.summary);
    assert_eq!(first, projected, "render(load(..)) is project(..)");
}
