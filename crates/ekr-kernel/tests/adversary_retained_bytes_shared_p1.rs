//! Adversary pass, wave perf-01 unit B (`story:retained-bytes-shared-not-copied`), at `7f299158`.
//!
//! The unit's own test (`retained_bytes_shared.rs`) says a verified read holds "the seed input's
//! payloads" as "the very allocations the store handle verified and holds", and defines "not
//! copied" as address equality: "a copy, however made, lives elsewhere". These cases hold the
//! change to that statement, and to the integrity the shared bytes and the seed input a runtime
//! now keeps must preserve:
//!
//! * a payload damaged on disk is refused by a runtime that did not verify it, although an earlier
//!   read still holds the verified copy the process registry vouches for;
//! * the seed input a runtime keeps is released when the runtime and its reads are dropped;
//! * a runtime that holds the seed input sees another runtime's commit on the same store, also
//!   while runtimes on other threads read the store.
//!
//! Every store is built at test time, `/2` as the base kernel wrote it and `/3` through the real
//! kernel, on both providers.
mod current_fixture;
#[allow(dead_code)]
#[path = "support/v2_store.rs"]
mod v2;

use std::path::Path;
use std::sync::Arc;

use current_fixture::{anchor, context, seed, SEEDED_AT, STATEMENT};
use ekr_core::{ContentHash, EvidenceId, Timestamp, TransactionId};
use ekr_graph::{Confidence, Evidence, EvidenceSource};
use ekr_kernel::{
    CommitCommandResult, EvidenceAddition, GraphOperation, GraphTransaction, Runtime,
    ValidationCommandResult,
};

fn existing(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file_existing(path, "ekr", context(), anchor())
    } else {
        Runtime::sqlite_existing(&path.join("state.db"), "ekr", context(), anchor())
    }
    .unwrap()
}

/// A `/2` store of `current_fixture::seed()`, written as the base kernel wrote it.
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

/// Every store shape this unit claims: `/2` and `/3`, on the file and the SQLite provider.
fn stores() -> Vec<(String, bool, tempfile::TempDir)> {
    let mut stores = Vec::new();
    for file in [true, false] {
        stores.push((format!("/2 file={file}"), file, v2_store(file)));
        stores.push((format!("/3 file={file}"), file, v3_store(file)));
    }
    stores
}

/// Flips one bit in the middle of every occurrence of `bytes` in every file under `root`: the
/// same length, different bytes. Returns how many occurrences it changed.
fn damage(root: &Path, bytes: &[u8]) -> usize {
    fn walk(path: &Path, bytes: &[u8], damaged: &mut usize) {
        if path.is_dir() {
            for entry in std::fs::read_dir(path).unwrap() {
                walk(&entry.unwrap().path(), bytes, damaged);
            }
            return;
        }
        let mut contents = std::fs::read(path).unwrap();
        let mut at = 0;
        let mut changed = false;
        while let Some(found) = contents[at..]
            .windows(bytes.len())
            .position(|window| window == bytes)
        {
            contents[at + found + bytes.len() / 2] ^= 1;
            at += found + bytes.len();
            changed = true;
            *damaged += 1;
        }
        if changed {
            std::fs::write(path, contents).unwrap();
        }
    }
    let mut damaged = 0;
    walk(root, bytes, &mut damaged);
    damaged
}

// ---------------------------------------------------------------------------------------------
// 1. The unit's own statement: the seed input's payloads are the retained allocations.
// ---------------------------------------------------------------------------------------------

/// `retained_bytes_shared.rs` states that a verified read holds the seed input's payloads as the
/// very allocations the store handle verified and holds, and that a copy "lives elsewhere". The
/// seed input is now kept by the runtime for its whole life, so a copy there is not transient: it
/// is a second resident copy of every seed evidence byte, per runtime.
#[test]
fn the_seed_inputs_payloads_are_the_retained_allocations_not_a_resident_copy() {
    let mut copies = Vec::new();
    for (label, file, directory) in stores() {
        let runtime = existing(directory.path(), file);
        let read = runtime.read(None).unwrap();
        assert!(
            !read.seed_input.evidence_payloads.is_empty(),
            "{label}: the fixture seed carries evidence"
        );
        let mut copied = 0usize;
        for (hash, bytes) in &read.seed_input.evidence_payloads {
            let retained = read
                .content(hash)
                .unwrap_or_else(|| panic!("{label}: the seed payload {hash} is not held"));
            assert_eq!(bytes.as_slice(), retained, "{label}: {hash}");
            if bytes.as_ptr() != retained.as_ptr() {
                copied += bytes.len();
            }
        }
        let kept = Arc::downgrade(&read.seed_input);
        drop(read);
        let resident = kept.upgrade().map_or(0, |input| {
            input
                .evidence_payloads
                .values()
                .map(|bytes| bytes.len())
                .sum()
        });
        if copied != 0 {
            copies.push(format!(
                "{label}: {copied} seed evidence bytes are a copy of the retained objects, and \
                 the runtime keeps {resident} of them after the read is dropped"
            ));
        }
    }
    assert!(
        copies.is_empty(),
        "the seed input holds copies of retained objects, not the retained allocations:\n{}",
        copies.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------
// 2. The seed input a runtime keeps is released with it.
// ---------------------------------------------------------------------------------------------

#[test]
fn dropping_the_runtime_and_its_reads_frees_the_seed_input_it_kept() {
    for (label, file, directory) in stores() {
        let runtime = existing(directory.path(), file);
        let first = runtime.read(None).unwrap();
        let second = runtime.read(None).unwrap();
        assert!(
            Arc::ptr_eq(&first.seed_input, &second.seed_input),
            "{label}: the runtime does not keep the seed input, so there is nothing to free"
        );
        let kept = Arc::downgrade(&first.seed_input);
        drop((first, second));
        assert!(
            kept.upgrade().is_some(),
            "{label}: the runtime keeps the seed input between reads"
        );
        drop(runtime);
        assert!(
            kept.upgrade().is_none(),
            "{label}: the seed input outlived the runtime and every read of it"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 3. A payload damaged on disk is refused, whatever an earlier read still holds.
// ---------------------------------------------------------------------------------------------

/// A read of a closed runtime still holds the verified payload, so the process registry vouches
/// for its address. The payload is then damaged on disk, keeping its length. A new runtime must
/// refuse it: the registered copy may only be compared with, never stand in for, bytes it reads.
/// A runtime whose handle still holds the payload does not prevent the refusal either.
#[test]
fn a_seed_payload_damaged_on_disk_is_refused_while_a_verified_copy_is_still_held() {
    for (label, file, directory) in stores() {
        let earlier = {
            let runtime = existing(directory.path(), file);
            runtime.read(None).unwrap()
        };
        let holding = existing(directory.path(), file);
        let held = holding.read(None).unwrap();
        assert_eq!(
            earlier.content(&ContentHash::of_bytes(STATEMENT)),
            Some(STATEMENT)
        );
        assert!(
            damage(directory.path(), STATEMENT) > 0,
            "{label}: the payload's bytes were found on disk"
        );

        let _ = ekr_store::read_work();
        // A provider that checks its blobs itself may refuse already at open; that is a refusal
        // too.
        let refused = if file {
            Runtime::file_existing(directory.path(), "ekr", context(), anchor())
        } else {
            Runtime::sqlite_existing(
                &directory.path().join("state.db"),
                "ekr",
                context(),
                anchor(),
            )
        }
        .map_err(|error| error.to_string())
        .and_then(|runtime| runtime.read(None).map_err(|error| error.to_string()));
        let work = ekr_store::read_work();
        let error = match refused {
            Ok(read) => panic!(
                "{label}: a new runtime answered from a payload damaged on disk: {:?} ({work:?})",
                read.content(&ContentHash::of_bytes(STATEMENT))
                    .map(String::from_utf8_lossy)
            ),
            Err(error) => error,
        };
        assert!(
            error.contains("integrity"),
            "{label}: refused, but not for the payload's integrity: {error}"
        );
        drop((earlier, held, holding));
    }
}

// ---------------------------------------------------------------------------------------------
// 4. Two runtimes on one store, one committing, while the other holds the seed input.
// ---------------------------------------------------------------------------------------------

fn at(millis: i64) -> impl FnOnce() -> Timestamp {
    move || Timestamp::from_millis(millis)
}

fn encode(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(serde::Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}

/// Commits one `AddEvidence` of `payload` through `runtime` and returns the evidence.
fn add_evidence(runtime: &Runtime, payload: &[u8], now: i64) -> Evidence {
    let evidence = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: ContentHash::of_bytes(payload),
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(5),
        confidence: Confidence::CERTAIN,
    };
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::AddEvidence(Box::new(EvidenceAddition {
            evidence: evidence.clone(),
            payload: payload.to_vec(),
        }))],
        evidence: std::collections::BTreeSet::new(),
        schema_version: None,
    };
    runtime
        .propose(&encode(&tx), context().operator, at(now))
        .unwrap();
    let head = runtime.head().unwrap().unwrap().revision;
    let verdict = runtime.validate(tx.id, head, at(now + 1)).unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    match runtime
        .commit(tx.id, context().operator, at(now + 2))
        .unwrap()
    {
        CommitCommandResult::Committed(_) => evidence,
        CommitCommandResult::Stale(stale) => panic!("stale: {stale:?}"),
    }
}

#[test]
fn a_runtime_holding_the_seed_input_sees_another_runtimes_commit_while_threads_read() {
    for (label, file, directory) in stores() {
        let reader = existing(directory.path(), file);
        let before = reader.read(None).unwrap();
        let writer = existing(directory.path(), file);

        let payload = format!("adversary evidence after the seed, {label}").into_bytes();
        let path = directory.path();
        let (evidence, reads) = std::thread::scope(|scope| {
            // `Runtime` is not `Sync`: each reading thread holds a runtime of its own, which
            // keeps its own seed input from its first read on.
            let readers: Vec<_> = (0..3)
                .map(|_| {
                    scope.spawn(move || {
                        let runtime = existing(path, file);
                        (0..5)
                            .map(|_| {
                                runtime
                                    .read(None)
                                    .map(|read| (*read.seed_input).clone())
                                    .map_err(|error| error.to_string())
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            let evidence = add_evidence(&writer, &payload, 1_000);
            let reads: Vec<_> = readers
                .into_iter()
                .flat_map(|handle| handle.join().unwrap())
                .collect();
            (evidence, reads)
        });
        for read in reads {
            let input = read.unwrap_or_else(|error| panic!("{label}: a concurrent read: {error}"));
            assert_eq!(input, *before.seed_input, "{label}");
        }

        let after = reader.read(None).unwrap();
        assert!(
            after.revisions.len() > before.revisions.len(),
            "{label}: the reader did not see the other runtime's commit"
        );
        assert_eq!(
            after.graph.evidence.get(&evidence.id),
            Some(&evidence),
            "{label}"
        );
        assert_eq!(
            after.content(&evidence.content_hash),
            Some(payload.as_slice()),
            "{label}: the committed payload"
        );
        assert_eq!(*after.seed_input, seed(), "{label}: the seed input");
        let fresh = existing(directory.path(), file).read(None).unwrap();
        assert_eq!(fresh.root, after.root, "{label}");
        assert_eq!(*fresh.seed_input, *after.seed_input, "{label}");
    }
}
