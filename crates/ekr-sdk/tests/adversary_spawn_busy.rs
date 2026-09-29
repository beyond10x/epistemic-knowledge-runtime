//! Adversary cases for `task:sdk-spawn-retries-a-busy-binary`.
//!
//! `spawn_busy.rs` holds the probe, the session and the viewer against a busy binary. These cases
//! hold what it leaves open: the one-shot path over the line cap, the documented 630 ms bound as a
//! number rather than a range, a cancel that arrives during the busy wait, and a probe timeout
//! shorter than the busy wait.

use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ekr_sdk::binary::{BinaryError, EkrBinary, Version};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig, LINE_CAP};
use ekr_sdk::transport::{Request, Transport, TransportError};

/// What `docs/sdk.md` and the CHANGELOG say the SDK waits in all before refusing a busy binary.
const DOCUMENTED_WAIT: Duration = Duration::from_millis(630);

/// Answers `--version` as `ekr 0.0.19` and exits 0, printing nothing, to anything else.
const STAND_IN: &str =
    "#!/bin/sh\ncase \"$1\" in --version) echo 'ekr 0.0.19'; exit 0 ;; esac\nexit 0\n";

fn stand_in(directory: &Path) -> PathBuf {
    let path = directory.join("ekr");
    std::fs::write(&path, STAND_IN).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn store(directory: &Path) -> StoreConfig {
    StoreConfig {
        host: directory.join("host.json"),
        store: directory.join("store"),
        backend: Backend::File,
    }
}

/// Open `path` for writing from another thread, return once it is open, close it after `hold`.
fn hold_open_for_writing(path: &Path, hold: Duration) -> JoinHandle<()> {
    let (opened, is_open) = mpsc::channel();
    let path = path.to_path_buf();
    let writer = thread::spawn(move || {
        let file = OpenOptions::new().write(true).open(&path).unwrap();
        opened.send(()).unwrap();
        thread::sleep(hold);
        drop(file);
    });
    is_open.recv().unwrap();
    writer
}

/// A request whose line is over [`LINE_CAP`] and whose argv reads no `-`: it goes to a one-shot
/// `ekr <argv>` process, not into the session.
fn over_the_cap() -> Request {
    Request::new(["commit"]).with_stdin("a".repeat(LINE_CAP + 1))
}

#[test]
fn a_one_shot_request_starts_from_a_binary_held_open_for_writing_once_the_writer_closes() {
    let directory = tempfile::tempdir().unwrap();
    let path = stand_in(directory.path());
    let binary = EkrBinary::open(&path).unwrap();
    let mut session =
        ProcessSession::start(&binary, store(directory.path()), SessionOptions::default()).unwrap();
    let request = over_the_cap();
    // The SDK serialises the 25 MB line before it starts the one-shot, so the writer must still
    // hold the binary open when that is done: 300 ms past the time serialising takes here.
    let serialising = Instant::now();
    drop(serde_json::to_vec(&request).unwrap());
    let hold = serialising.elapsed() + Duration::from_millis(300);
    let writer = hold_open_for_writing(&path, hold);
    let started = Instant::now();
    let replied = session.request(&request);
    let elapsed = started.elapsed();
    writer.join().unwrap();
    let reply = replied.unwrap_or_else(|error| panic!("the one-shot was refused: {error:?}"));
    assert!(
        elapsed >= hold,
        "the one-shot answered after {elapsed:?}, before the writer closed at {hold:?}"
    );
    assert_eq!(reply.exit, 0);
    assert_eq!(reply.document, None);
    assert_eq!(session.processes_started(), 2);
}

#[test]
fn a_binary_busy_past_the_bound_is_refused_after_the_documented_630_ms_and_not_much_later() {
    let directory = tempfile::tempdir().unwrap();
    let path = stand_in(directory.path());
    let writer = hold_open_for_writing(&path, Duration::from_millis(1500));
    let started = Instant::now();
    let refused = EkrBinary::open(&path).unwrap_err();
    let elapsed = started.elapsed();
    writer.join().unwrap();
    match &refused {
        BinaryError::Run { source, .. } => {
            assert_eq!(source.kind(), ErrorKind::ExecutableFileBusy, "{refused:?}");
        }
        other => panic!("expected BinaryError::Run, got {other:?}"),
    }
    assert!(
        elapsed >= DOCUMENTED_WAIT,
        "refused after {elapsed:?}, before the documented {DOCUMENTED_WAIT:?}"
    );
    assert!(
        elapsed < DOCUMENTED_WAIT + Duration::from_millis(270),
        "refused after {elapsed:?}, well past the documented {DOCUMENTED_WAIT:?}"
    );
}

#[test]
fn a_cancel_during_the_busy_wait_of_a_one_shot_ends_the_call_as_cancelled() {
    let directory = tempfile::tempdir().unwrap();
    let path = stand_in(directory.path());
    let binary = EkrBinary::open(&path).unwrap();
    let mut session =
        ProcessSession::start(&binary, store(directory.path()), SessionOptions::default()).unwrap();
    let cancel = session.cancel_handle();
    let request = over_the_cap();
    // The SDK serialises the 25 MB line before it starts the one-shot; the cancel is timed to land
    // 150 ms after that, inside the busy wait.
    let serialising = Instant::now();
    let line = serde_json::to_vec(&request).unwrap();
    let serialised = serialising.elapsed();
    drop(line);
    let cancel_after = serialised + Duration::from_millis(150);
    let writer = hold_open_for_writing(&path, Duration::from_millis(3000));
    let started = Instant::now();
    let canceller = thread::spawn(move || {
        thread::sleep(cancel_after);
        cancel.cancel();
        Instant::now()
    });
    let replied = session.request(&request);
    let returned = Instant::now();
    let cancelled_at = canceller.join().unwrap();
    writer.join().unwrap();
    let after_cancel = returned.saturating_duration_since(cancelled_at);
    assert!(
        after_cancel < Duration::from_millis(100),
        "the call returned {after_cancel:?} after the cancel ({:?} in all): {replied:?}",
        returned - started
    );
    if returned > cancelled_at {
        assert!(
            matches!(replied, Err(TransportError::Cancelled { .. })),
            "docs/sdk.md: a call in flight fails with Cancelled; got {replied:?}"
        );
    }
}

#[test]
fn a_probe_timeout_shorter_than_the_busy_wait_bounds_the_probe() {
    let directory = tempfile::tempdir().unwrap();
    let path = stand_in(directory.path());
    let probe_timeout = Duration::from_millis(50);
    let writer = hold_open_for_writing(&path, Duration::from_millis(1500));
    let started = Instant::now();
    let refused = EkrBinary::open_with(&path, Version::new(0, 0, 14), probe_timeout).unwrap_err();
    let elapsed = started.elapsed();
    writer.join().unwrap();
    assert!(
        elapsed < probe_timeout + Duration::from_millis(200),
        "open_with(probe_timeout = {probe_timeout:?}) returned after {elapsed:?}: {refused:?}"
    );
}
