//! Adversary cases for `task:sdk-spawn-retries-a-busy-binary`.
//!
//! `spawn_busy.rs` holds the probe, the session and the viewer against a busy binary. These cases
//! hold what it leaves open: the one-shot path over the line cap, the documented 630 ms bound as a
//! number rather than a range, a cancel that arrives during the busy wait, and a probe timeout
//! shorter than the busy wait.
//!
//! A one-shot starts only after the SDK has serialised a 25 MB request line, which takes a time
//! that load stretches by seconds. So the one-shot cases do not time their writer or their cancel
//! against a clock: a watcher reads the calling thread's `/proc/<tid>/wchan` and acts once that
//! thread sleeps in `nanosleep`. Nothing on the one-shot path sleeps before its spawn, so that
//! sleep is the busy wait itself, and a one-shot that does not retry never reaches it.

use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ekr_sdk::binary::{BinaryError, EkrBinary, Version};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig, LINE_CAP};
use ekr_sdk::transport::{Request, Transport, TransportError};

/// What `docs/sdk.md` and the CHANGELOG say the SDK waits in all before refusing a busy binary.
const DOCUMENTED_WAIT: Duration = Duration::from_millis(630);

/// How long a watcher looks for the busy wait before it gives up.
const WATCH_LIMIT: Duration = Duration::from_secs(60);

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

/// The `wchan` file of the thread that calls this.
fn own_wchan() -> PathBuf {
    let task = std::fs::read_link("/proc/thread-self").unwrap();
    Path::new("/proc").join(task).join("wchan")
}

fn sleeping(wchan: &Path) -> bool {
    std::fs::read_to_string(wchan).is_ok_and(|symbol| symbol.contains("nanosleep"))
}

/// Refuse to run a case whose watcher could never see a sleep: `/proc/<tid>/wchan` must name
/// `nanosleep` for a thread in `thread::sleep`.
fn assert_wchan_shows_a_sleep() {
    let (sent, received) = mpsc::channel();
    let sleeper = thread::spawn(move || {
        sent.send(own_wchan()).unwrap();
        thread::sleep(Duration::from_millis(500));
    });
    let wchan = received.recv().unwrap();
    let mut seen = false;
    while !sleeper.is_finished() && !seen {
        seen = sleeping(&wchan);
        thread::sleep(Duration::from_millis(1));
    }
    sleeper.join().unwrap();
    assert!(
        seen,
        "{} never named nanosleep for a sleeping thread",
        wchan.display()
    );
}

/// Watches the thread whose `wchan` is given, and runs `action` the first time it sleeps.
struct Watcher {
    done: Arc<AtomicBool>,
    thread: JoinHandle<Option<Instant>>,
}

impl Watcher {
    fn start(wchan: PathBuf, action: impl FnOnce() + Send + 'static) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&done);
        let thread = thread::spawn(move || {
            let give_up = Instant::now() + WATCH_LIMIT;
            while !stop.load(Ordering::SeqCst) && Instant::now() < give_up {
                if sleeping(&wchan) {
                    action();
                    return Some(Instant::now());
                }
                thread::sleep(Duration::from_millis(1));
            }
            None
        });
        Self { done, thread }
    }

    /// When `action` ran, or `None` if the watched thread never slept.
    fn finish(self) -> Option<Instant> {
        self.done.store(true, Ordering::SeqCst);
        self.thread.join().unwrap()
    }
}

#[test]
fn a_one_shot_request_starts_from_a_binary_held_open_for_writing_once_the_writer_closes() {
    assert_wchan_shows_a_sleep();
    let directory = tempfile::tempdir().unwrap();
    let path = stand_in(directory.path());
    let binary = EkrBinary::open(&path).unwrap();
    let mut session =
        ProcessSession::start(&binary, store(directory.path()), SessionOptions::default()).unwrap();
    let request = over_the_cap();
    // The binary is busy from before the request until the SDK is seen in its busy wait.
    let file: File = OpenOptions::new().write(true).open(&path).unwrap();
    let watcher = Watcher::start(own_wchan(), move || drop(file));
    let replied = session.request(&request);
    let closed = watcher.finish();
    let reply = replied.unwrap_or_else(|error| {
        panic!("the one-shot was refused (writer closed: {closed:?}): {error:?}")
    });
    assert!(
        closed.is_some(),
        "the one-shot answered without ever waiting on the busy binary"
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
    assert_wchan_shows_a_sleep();
    let directory = tempfile::tempdir().unwrap();
    let path = stand_in(directory.path());
    let binary = EkrBinary::open(&path).unwrap();
    let mut session =
        ProcessSession::start(&binary, store(directory.path()), SessionOptions::default()).unwrap();
    let cancel = session.cancel_handle();
    let request = over_the_cap();
    // The binary stays busy for the whole call; the cancel lands once the SDK is in its busy wait.
    let file: File = OpenOptions::new().write(true).open(&path).unwrap();
    let watcher = Watcher::start(own_wchan(), move || cancel.cancel());
    let replied = session.request(&request);
    let returned = Instant::now();
    let cancelled_at = watcher.finish();
    drop(file);
    let Some(cancelled_at) = cancelled_at else {
        panic!("the one-shot never waited on the busy binary: {replied:?}");
    };
    assert!(
        matches!(replied, Err(TransportError::Cancelled { .. })),
        "docs/sdk.md: a call in flight fails with Cancelled; got {replied:?}"
    );
    let after_cancel = returned.saturating_duration_since(cancelled_at);
    assert!(
        after_cancel < Duration::from_secs(1),
        "the call returned {after_cancel:?} after the cancel"
    );
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
