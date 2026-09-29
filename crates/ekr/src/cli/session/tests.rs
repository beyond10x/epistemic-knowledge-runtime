//! Without a replacement, following the store costs one identity check per request and no store
//! open; a replacement costs one reopen, and a file store replaced inside its directory — the same
//! device and inode — is reopened once its held history is refused as diverged. A session reads its
//! transactions only while it tracks a proposal. Counted with [`reader_work`].

use std::collections::BTreeSet;

use ekr_core::Timestamp;

use super::fixture::{replace, replace_inside, seeded, seeded_with_a_commit, BACKENDS};
use super::{identity, reader_work, respond, Checked, Held, ReaderWork, Watch};
use crate::cli::{Backend, Session, Store};

/// A session over `store`, holding it, and what it watches.
fn session(store: Store) -> (Session, Watch) {
    let watch = Watch {
        held: true,
        opened: identity(&store.store),
        proposed: BTreeSet::new(),
    };
    let session = Session {
        runtime: Some(store.open().expect("the store opens")),
        store,
        create: false,
    };
    (session, watch)
}

/// One request line for `argv`.
fn line(argv: &[&str]) -> Vec<u8> {
    serde_json::json!({ "argv": argv }).to_string().into_bytes()
}

/// A file store whose files are replaced inside its directory keeps its device and inode: the
/// held history is refused as diverged, the session reopens the store once and answers there.
#[test]
fn a_session_follows_a_file_store_replaced_inside_its_directory() {
    let clock = || Timestamp::from_millis(0);
    let directory = tempfile::tempdir().expect("a temporary directory");
    let store = seeded(directory.path(), Backend::File, "store");
    let path = store.store.clone();
    let (mut session, mut watch) = session(store);
    let head = respond(&line(&["head"]), &mut session, &mut watch, &clock).expect("head");
    assert_eq!(head["revision"], 0);
    let next = seeded_with_a_commit(directory.path(), Backend::File, "next");
    let before = identity(&path);
    replace_inside(&path, &next.store);
    assert_eq!(
        identity(&path),
        before,
        "precondition: the same device and inode"
    );
    let _ = reader_work();
    let head = respond(&line(&["head"]), &mut session, &mut watch, &clock).expect("head");
    assert_eq!(head["revision"], 1, "the store now at the path");
    assert_eq!(reader_work().reopens, 1);
}

/// The proposals a session tracks are settled from its transactions only while there are some:
/// one read per store verb from its `propose` until its `commit`, none before or after.
#[test]
fn a_session_reads_its_transactions_only_while_it_tracks_a_proposal() {
    let clock = || Timestamp::from_millis(0);
    for backend in BACKENDS {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let store = seeded(directory.path(), backend, "store");
        let transaction = "00000000-0000-4000-8000-00000000f912";
        let document = directory.path().join("create.yaml");
        std::fs::write(
            &document,
            format!(
                "format: ekr.transaction-document/2\ntransaction:\n  id: {transaction}\n  \
                 proposer: 00000000-0000-4000-8000-000000000101\n  operations:\n  - !CreateNode\n    \
                 id: 00000000-0000-4000-8000-00000000f911\n    root_id: \
                 00000000-0000-4000-8000-000000000002\n    type_id: \
                 00000000-0000-4000-8000-000000000202\n    canonical_name: Hooli\n    \
                 properties: {{}}\n    aliases: [Hooli]\n  evidence: []\n"
            ),
        )
        .expect("writing the transaction");
        let document = document.to_str().expect("a UTF-8 path").to_owned();
        let (mut session, mut watch) = session(store);
        let _ = reader_work();
        for argv in [
            &["head"][..],
            &["propose", &document],
            &["head"],
            &["validate", transaction],
            &["commit", transaction],
            &["head"],
        ] {
            respond(&line(argv), &mut session, &mut watch, &clock)
                .unwrap_or_else(|failure| panic!("{backend:?} {argv:?}: {failure}"));
        }
        assert!(watch.proposed.is_empty(), "{backend:?}");
        assert_eq!(
            reader_work(),
            ReaderWork {
                checks: 6,
                reopens: 0,
                settles: 3
            },
            "{backend:?}: a settle before head, validate and commit, none before the proposal \
             or after its commit"
        );
    }
}

#[test]
fn a_held_store_is_checked_without_a_store_open_and_reopened_once_per_replacement() {
    for backend in BACKENDS {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let store = seeded(directory.path(), backend, "store");
        let path = store.store.clone();
        let mut held = Held::open(store).expect("the store opens");
        let _ = reader_work();
        for _ in 0..5 {
            assert!(
                matches!(held.current(), Ok(Checked::Same(_))),
                "{backend:?}"
            );
        }
        assert_eq!(
            reader_work(),
            ReaderWork {
                checks: 5,
                reopens: 0,
                settles: 0
            },
            "{backend:?}: five requests, no replacement"
        );

        let next = seeded(directory.path(), backend, "next");
        replace(&path, &directory.path().join("replaced"), &next.store);
        assert!(
            matches!(held.current(), Ok(Checked::Reopened(_))),
            "{backend:?}"
        );
        assert!(
            matches!(held.current(), Ok(Checked::Same(_))),
            "{backend:?}"
        );
        assert_eq!(
            reader_work(),
            ReaderWork {
                checks: 2,
                reopens: 1,
                settles: 0
            },
            "{backend:?}: one replacement, one reopen"
        );

        std::fs::write(directory.path().join("junk"), "not a store\n").expect("a file");
        replace(
            &path,
            &directory.path().join("second"),
            &directory.path().join("junk"),
        );
        for _ in 0..2 {
            let refused = held
                .current()
                .err()
                .expect("a file that is no store is refused");
            assert!(
                refused.message.contains(&path.display().to_string()),
                "{backend:?}: {}",
                refused.message
            );
        }
        assert_eq!(
            reader_work(),
            ReaderWork {
                checks: 2,
                reopens: 2,
                settles: 0
            },
            "{backend:?}: every request after a failed reopen tries again"
        );
    }
}

#[test]
fn a_session_request_checks_the_store_only_for_a_store_verb_and_opens_it_only_when_replaced() {
    let clock = || Timestamp::from_millis(0);
    for backend in BACKENDS {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let store = seeded(directory.path(), backend, "store");
        let path = store.store.clone();
        let mut watch = Watch {
            held: true,
            opened: identity(&path),
            proposed: BTreeSet::new(),
        };
        let mut session = Session {
            runtime: Some(store.open().expect("the store opens")),
            store,
            create: false,
        };
        let _ = reader_work();
        for _ in 0..3 {
            respond(br#"{"argv":["head"]}"#, &mut session, &mut watch, &clock)
                .expect("head answers");
            respond(
                br#"{"argv":["mint","node"]}"#,
                &mut session,
                &mut watch,
                &clock,
            )
            .expect("mint answers");
        }
        assert_eq!(
            reader_work(),
            ReaderWork {
                checks: 3,
                reopens: 0,
                settles: 0
            },
            "{backend:?}: a check per head, none per mint, no open"
        );

        let next = seeded(directory.path(), backend, "next");
        replace(&path, &directory.path().join("replaced"), &next.store);
        respond(br#"{"argv":["head"]}"#, &mut session, &mut watch, &clock).expect("head answers");
        respond(br#"{"argv":["head"]}"#, &mut session, &mut watch, &clock).expect("head answers");
        assert_eq!(
            reader_work(),
            ReaderWork {
                checks: 2,
                reopens: 1,
                settles: 0
            },
            "{backend:?}"
        );
    }
}
