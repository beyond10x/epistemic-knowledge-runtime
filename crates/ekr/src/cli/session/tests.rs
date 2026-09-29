//! Without a replacement, following the store costs one identity check per request and no store
//! open; a replacement costs one reopen. Counted with [`reader_work`], on both providers.

use std::collections::BTreeSet;

use ekr_core::Timestamp;

use super::fixture::{replace, seeded, BACKENDS};
use super::{identity, reader_work, respond, Checked, Held, ReaderWork, Watch};
use crate::cli::Session;

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
                reopens: 0
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
                reopens: 1
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
                reopens: 2
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
                reopens: 0
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
                reopens: 1
            },
            "{backend:?}"
        );
    }
}
