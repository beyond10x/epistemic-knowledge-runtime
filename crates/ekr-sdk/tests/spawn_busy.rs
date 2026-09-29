//! Starting an `ekr` binary that is busy being written (`task:sdk-spawn-retries-a-busy-binary`).
//!
//! Linux refuses to execute a file while any process holds it open for writing (`ETXTBSY`). A
//! consumer that has just written its binary meets that refusal for as long as the write
//! descriptor lives, including in a child another thread forked before it execs. The SDK retries
//! that one refusal for a bounded time and returns every other error at once.

use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ekr_sdk::binary::{BinaryError, EkrBinary, Version};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};

/// The SDK's retries stop within this; a refusal returned without retrying comes well inside it.
const BOUND: Duration = Duration::from_secs(1);

/// A stand-in that answers `--version` as `ekr 0.0.19` and exits 0 to anything else.
const STAND_IN: &str = "#!/bin/sh\ncase \"$1\" in --version) echo 'ekr 0.0.19' ;; esac\nexit 0\n";

fn stand_in(directory: &Path, mode: u32) -> PathBuf {
    let path = directory.join("ekr");
    std::fs::write(&path, STAND_IN).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
    path
}

/// Open `path` for writing from another thread, return once it is open, and close it after
/// `hold`.
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

fn probe(path: &Path) -> Result<EkrBinary, BinaryError> {
    EkrBinary::open_with(path, Version::new(0, 0, 14), Duration::from_secs(5))
}

fn run_source(error: &BinaryError) -> &std::io::Error {
    match error {
        BinaryError::Run { what, source, .. } if what == "--version" => source,
        other => panic!("expected BinaryError::Run for --version, got {other:?}"),
    }
}

#[test]
fn a_probe_of_a_binary_held_open_for_writing_succeeds_once_the_writer_closes() {
    let directory = tempfile::tempdir().unwrap();
    let path = stand_in(directory.path(), 0o755);
    let writer = hold_open_for_writing(&path, Duration::from_millis(100));
    let probed = probe(&path);
    writer.join().unwrap();
    let binary = probed.unwrap_or_else(|error| panic!("the probe was refused: {error:?}"));
    assert_eq!(binary.version(), Version::new(0, 0, 19));
}

#[test]
fn a_session_starts_from_a_binary_held_open_for_writing_once_the_writer_closes() {
    let directory = tempfile::tempdir().unwrap();
    let path = stand_in(directory.path(), 0o755);
    let binary = probe(&path).unwrap();
    let store = StoreConfig {
        host: directory.path().join("host.json"),
        store: directory.path().join("store"),
        backend: Backend::File,
    };
    let writer = hold_open_for_writing(&path, Duration::from_millis(100));
    let started = ProcessSession::start(&binary, store, SessionOptions::default());
    writer.join().unwrap();
    let session = started.unwrap_or_else(|error| panic!("the session did not start: {error:?}"));
    assert!(session.close().unwrap().success());
}

#[test]
fn a_binary_busy_past_the_bound_is_refused_as_before_with_the_busy_source() {
    let directory = tempfile::tempdir().unwrap();
    let path = stand_in(directory.path(), 0o755);
    let hold = Duration::from_millis(1500);
    let writer = hold_open_for_writing(&path, hold);
    let started = Instant::now();
    let refused = probe(&path).unwrap_err();
    let elapsed = started.elapsed();
    writer.join().unwrap();
    let source = run_source(&refused);
    assert_eq!(source.kind(), ErrorKind::ExecutableFileBusy, "{refused:?}");
    assert!(
        elapsed < hold,
        "gave up after {elapsed:?}, not before the writer closed"
    );
    assert!(elapsed < BOUND, "retried for {elapsed:?}");
    assert!(
        elapsed >= Duration::from_millis(300),
        "gave up after {elapsed:?} without waiting"
    );
    let message = refused.to_string();
    assert!(message.contains(&path.display().to_string()), "{message}");
    assert!(message.contains("--version"), "{message}");
}

#[test]
fn a_missing_binary_is_refused_on_the_first_attempt() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("absent");
    let started = Instant::now();
    let refused = probe(&path).unwrap_err();
    let elapsed = started.elapsed();
    assert_eq!(
        run_source(&refused).kind(),
        ErrorKind::NotFound,
        "{refused:?}"
    );
    assert!(elapsed < BOUND / 5, "refused after {elapsed:?}");
}

#[test]
fn a_binary_that_is_not_executable_is_refused_on_the_first_attempt() {
    let directory = tempfile::tempdir().unwrap();
    let path = stand_in(directory.path(), 0o644);
    let started = Instant::now();
    let refused = probe(&path).unwrap_err();
    let elapsed = started.elapsed();
    assert_eq!(
        run_source(&refused).kind(),
        ErrorKind::PermissionDenied,
        "{refused:?}"
    );
    assert!(elapsed < BOUND / 5, "refused after {elapsed:?}");
}
