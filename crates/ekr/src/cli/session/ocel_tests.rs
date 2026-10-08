//! OCEL success diagnostics belong to the same in-process request as their document.
use ekr_core::Timestamp;
use ekr_sdk::transport::{Request, Transport};

use super::fixture::{seeded, BACKENDS};

#[test]
fn in_process_ocel_returns_engine_counts_with_its_document_and_leaves_the_next_stderr_empty() {
    for backend in BACKENDS {
        let directory = tempfile::tempdir().unwrap();
        let store = seeded(directory.path(), backend, "ocel");
        let runtime = store.open().unwrap();
        let expected = ekr_views::export_ocel(&runtime, None, &[]).unwrap();
        let clock = || Timestamp::from_millis(0);
        let mut session = super::InProcess::new(store, runtime, &clock);
        let reply = session.request(&Request::new(["ocel"])).unwrap();
        assert_eq!(reply.exit, 0);
        assert_eq!(
            reply.document,
            Some(serde_json::from_slice(&expected.bytes).unwrap())
        );
        let counts: serde_json::Value =
            serde_json::from_str(&reply.stderr).expect("the request's counts");
        assert_eq!(counts["events"], expected.summary.events);
        assert_eq!(counts["objects"], expected.summary.objects);
        assert_eq!(counts["ocel_hash"], expected.summary.ocel_hash);
        assert_eq!(session.request(&Request::new(["head"])).unwrap().stderr, "");
        session.close();
    }
}
