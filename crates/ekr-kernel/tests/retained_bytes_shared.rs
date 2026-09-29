//! `story:retained-bytes-shared-not-copied`: a request on an opened runtime copies no retained
//! object, on both providers and for a seed retained as `ekr-seed-envelope/2` or `/3`.
//!
//! A verified read holds its retained bytes — the seed envelope, every record, every evidence
//! payload and the seed input's payloads — as the very allocations the store handle verified and
//! holds. A second request therefore answers from the same bytes at the same addresses as the
//! first, and neither fetches, hashes nor compares any of them again. Address equality of two
//! requests' bytes is what "not copied" means here: a copy, however made, lives elsewhere.
mod current_fixture;
#[allow(dead_code)]
#[path = "support/v2_store.rs"]
mod v2;

use std::collections::BTreeSet;
use std::path::Path;

use current_fixture::{anchor, context, seed, SEEDED_AT};
use ekr_core::ContentHash;
use ekr_kernel::Runtime;

fn existing(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file_existing(path, "ekr", context(), anchor())
    } else {
        Runtime::sqlite_existing(&path.join("state.db"), "ekr", context(), anchor())
    }
    .unwrap()
}

/// A `/2` store of `current_fixture::seed()` holding every transaction state, written as the base
/// kernel wrote it.
fn v2_store(file: bool) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    v2::fixture_store(
        directory.path(),
        file,
        &seed(),
        context(),
        &anchor(),
        SEEDED_AT,
    );
    directory
}

/// A `/3` store of `current_fixture::seed()`, seeded through the real kernel.
fn v3_store(file: bool) -> tempfile::TempDir {
    let (directory, _, _) = current_fixture::seeded(seed(), context(), anchor(), file, SEEDED_AT);
    directory
}

/// Two verified reads of the head through one runtime, as two requests of one session are: the
/// second shares every retained byte of the first and does no work on any of them.
fn a_request_copies_no_retained_object(path: &Path, file: bool, label: &str) {
    let runtime = existing(path, file);
    let first = runtime.read(None).unwrap();
    let _ = ekr_store::read_work();
    let second = runtime.read(None).unwrap();
    let work = ekr_store::read_work();

    let mut hashes: BTreeSet<ContentHash> = second
        .revisions
        .values()
        .map(|revision| revision.record_hash)
        .collect();
    hashes.insert(second.seed.seed_hash);
    hashes.extend(
        second
            .graph
            .evidence
            .values()
            .map(|evidence| evidence.content_hash),
    );
    assert!(hashes.len() >= 3, "{label}: {hashes:?}");
    for hash in &hashes {
        let (earlier, later) = (first.content(hash), second.content(hash));
        assert!(later.is_some(), "{label}: {hash} is not held");
        assert_eq!(
            later.map(<[u8]>::as_ptr),
            earlier.map(<[u8]>::as_ptr),
            "{label}: the second request copied the retained object {hash}"
        );
    }
    assert!(
        !second.seed_input.evidence_payloads.is_empty(),
        "{label}: the fixture seed carries evidence"
    );
    for (hash, bytes) in &second.seed_input.evidence_payloads {
        assert_eq!(
            bytes.as_ptr(),
            first.seed_input.evidence_payloads[hash].as_ptr(),
            "{label}: the second request copied the seed input's payload {hash}"
        );
        assert_eq!(
            Some(bytes.as_ptr()),
            second.content(hash).map(<[u8]>::as_ptr),
            "{label}: the seed input's payload {hash} is a copy of the retained object"
        );
    }
    assert_eq!(
        (work.blobs_read, work.blobs_hashed, work.bytes_compared),
        (0, 0, 0),
        "{label}: the second request fetched, hashed or compared retained bytes again"
    );
}

#[test]
fn a_request_copies_no_retained_object_on_either_provider_or_envelope() {
    for file in [true, false] {
        let directory = v2_store(file);
        a_request_copies_no_retained_object(directory.path(), file, &format!("/2 file={file}"));
        let directory = v3_store(file);
        a_request_copies_no_retained_object(directory.path(), file, &format!("/3 file={file}"));
    }
}
