//! Adversary pass 1 on `task:sdk-close-reports-the-child-stderr`: `ProcessSession::close`
//! against the table in `docs/sdk.md` ("Failure, the latch and cancellation") and the field
//! documentation of `TransportError::CloseFailed`.
//!
//! Every child here is a stand-in shell script that answers the binary handshake (`--version`)
//! and, where it writes stderr, copies a file the test wrote beside it, so the exact bytes are
//! under the test's control.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::process::ExitStatusExt as _;
use std::path::Path;
use std::time::{Duration, Instant};

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig, STDERR_TAIL_BYTES};
use ekr_sdk::transport::{Request, Transport as _, TransportError};

const HANDSHAKE: &str = "#!/bin/sh\ncase \"$1\" in --version) echo 'ekr 0.0.20'; exit 0 ;; esac\n";

fn stand_in(directory: &Path, body: &str) -> EkrBinary {
    let path = directory.join("ekr");
    std::fs::write(&path, format!("{HANDSHAKE}{body}")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    EkrBinary::open(&path).expect("the stand-in answers the handshake")
}

fn store(directory: &Path) -> StoreConfig {
    StoreConfig {
        host: directory.join("host.json"),
        store: directory.join("store"),
        backend: Backend::File,
    }
}

fn start(directory: &Path, body: &str) -> ProcessSession {
    let binary = stand_in(directory, body);
    ProcessSession::start(&binary, store(directory), SessionOptions::default()).unwrap()
}

/// The stand-in reads its input to the end, copies `stderr.bin` beside it to stderr, exits 3.
const COPIES_STDERR_AND_EXITS_THREE: &str =
    "cat >/dev/null\ncat \"$(dirname \"$0\")/stderr.bin\" >&2\nexit 3\n";

fn close_failed(session: ProcessSession) -> (std::process::ExitStatus, bool, String) {
    match session.close() {
        Err(TransportError::CloseFailed {
            status,
            killed,
            stderr_tail,
        }) => (status, killed, stderr_tail),
        other => panic!("expected CloseFailed, got {other:?}"),
    }
}

/// Mutant probe: without the `cancelled()` branch in `close`, a cancelled session would close
/// as `CloseFailed { killed: true }`. Nothing in `session_close.rs` or the cancel-while-closing
/// case (which discards the result) asserts the `Cancelled` row of the table.
#[test]
fn a_cancelled_session_closes_as_cancelled() {
    let directory = tempfile::tempdir().unwrap();
    let session = start(directory.path(), "exec sleep 30\n");
    session.cancel_handle().cancel();
    std::thread::sleep(Duration::from_millis(200));
    match session.close() {
        Err(TransportError::Cancelled { verb }) => assert_eq!(verb, "session"),
        other => panic!("expected Cancelled, got {other:?}"),
    }
}

/// Mutant probe: without the `latched` branch, a failed session would close as
/// `CloseFailed { status: signal 9, killed: false }`, a kill the SDK made reported as not one.
#[test]
fn a_session_failed_by_a_non_reply_closes_as_latched() {
    let directory = tempfile::tempdir().unwrap();
    let mut session = start(
        directory.path(),
        "read line\necho 'not a reply'\nexec cat >/dev/null\n",
    );
    let failed = session.request(&Request::new(["ontology"]));
    assert!(
        matches!(failed, Err(TransportError::Protocol { .. })),
        "{failed:?}"
    );
    match session.close() {
        Err(TransportError::Latched { verb, cause }) => {
            assert_eq!(verb, "session");
            assert!(cause.contains("not a reply"), "{cause}");
        }
        other => panic!("expected Latched, got {other:?}"),
    }
}

/// `docs/sdk.md`: `TransportError::Latched` — "an earlier call failed the session". `close`
/// tests `status.success()` before the latch, so a failed session whose child had exited 0
/// closes as `Ok(())`: `session.close()?` reports a clean close of a session that failed.
#[test]
#[ignore = "defect: close() returns Ok(()) for a latched session whose child exited 0; the latch is checked only after success"]
fn a_failed_session_whose_child_exited_zero_still_closes_as_latched() {
    let directory = tempfile::tempdir().unwrap();
    let mut session = start(directory.path(), "exit 0\n");
    std::thread::sleep(Duration::from_millis(200));
    let failed = session.request(&Request::new(["ontology"]));
    assert!(
        matches!(failed, Err(TransportError::Died { .. })),
        "{failed:?}"
    );
    match session.close() {
        Err(TransportError::Latched { .. }) => {}
        other => panic!("expected Latched for a session whose request failed, got {other:?}"),
    }
}

/// `killed` is "whether the SDK killed the process": a child that ends itself by a signal was
/// not killed by the SDK.
#[test]
fn a_child_ending_itself_by_a_signal_is_not_reported_as_killed() {
    let directory = tempfile::tempdir().unwrap();
    let session = start(
        directory.path(),
        "cat >/dev/null\necho 'ekr: terminating' >&2\nkill -TERM $$\nsleep 5\n",
    );
    let (status, killed, stderr_tail) = close_failed(session);
    assert_eq!(status.signal(), Some(15), "{status}");
    assert!(!killed, "the SDK did not kill a child that signalled itself");
    assert_eq!(stderr_tail.trim_end(), "ekr: terminating");
}

/// More than the bound, all ASCII: the tail is the last `STDERR_TAIL_BYTES` bytes, ending with
/// the line written last.
#[test]
fn a_long_stderr_is_cut_to_its_last_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let mut written = "a".repeat(3 * STDERR_TAIL_BYTES);
    written.push_str("\nekr: the last line\n");
    std::fs::write(directory.path().join("stderr.bin"), &written).unwrap();
    let session = start(directory.path(), COPIES_STDERR_AND_EXITS_THREE);
    let (status, _, stderr_tail) = close_failed(session);
    assert_eq!(status.code(), Some(3));
    assert_eq!(stderr_tail.len(), STDERR_TAIL_BYTES);
    assert_eq!(stderr_tail, written[written.len() - STDERR_TAIL_BYTES..]);
}

/// `CloseFailed::stderr_tail` is documented as "the last bytes the process wrote to stderr, at
/// most `STDERR_TAIL_BYTES`" (`transport.rs`), and `docs/sdk.md` as "the last 4096 bytes it
/// wrote". The collector cuts at a byte, and `from_utf8_lossy` turns the half character left at
/// the front into U+FFFD, three bytes the child never wrote, so the tail is 4098 bytes.
#[test]
#[ignore = "defect: a tail cut inside a UTF-8 character is 4098 bytes and starts with U+FFFD the child never wrote"]
fn a_tail_cut_inside_a_character_stays_within_the_bound() {
    let directory = tempfile::tempdir().unwrap();
    // 3000 two-byte characters, then one byte: the last 4096 bytes start on a continuation byte.
    let mut written = "é".repeat(3000);
    written.push('x');
    std::fs::write(directory.path().join("stderr.bin"), &written).unwrap();
    let session = start(directory.path(), COPIES_STDERR_AND_EXITS_THREE);
    let (_, _, stderr_tail) = close_failed(session);
    assert!(
        stderr_tail.len() <= STDERR_TAIL_BYTES,
        "the tail is {} bytes, over the documented {STDERR_TAIL_BYTES}",
        stderr_tail.len()
    );
    assert!(
        !stderr_tail.contains('\u{FFFD}'),
        "the tail starts with a replacement character the child never wrote: {:?}",
        &stderr_tail[..8]
    );
}

/// Bytes that are not UTF-8 do not cost the line after them.
#[test]
fn a_tail_with_invalid_utf8_keeps_the_line_after_it() {
    let directory = tempfile::tempdir().unwrap();
    let mut written = vec![0xff, 0xfe, b'\n'];
    written.extend_from_slice(b"ekr: the last line\n");
    std::fs::write(directory.path().join("stderr.bin"), &written).unwrap();
    let session = start(directory.path(), COPIES_STDERR_AND_EXITS_THREE);
    let (_, _, stderr_tail) = close_failed(session);
    assert!(stderr_tail.ends_with("ekr: the last line\n"), "{stderr_tail:?}");
}

/// The collector thread races the exit: a child that writes a burst and exits at once still has
/// its last line in the tail, every time.
#[test]
fn a_child_that_writes_and_exits_at_once_has_its_last_line_in_the_tail() {
    let mut written = "b".repeat(64 * 1024);
    written.push_str("\nekr: written just before the exit\n");
    for round in 0..20 {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("stderr.bin"), &written).unwrap();
        let session = start(directory.path(), COPIES_STDERR_AND_EXITS_THREE);
        let (_, _, stderr_tail) = close_failed(session);
        assert!(
            stderr_tail.ends_with("ekr: written just before the exit\n"),
            "round {round}: {:?}",
            &stderr_tail[stderr_tail.len().saturating_sub(60)..]
        );
    }
}

/// A grandchild holding the child's stderr open does not hold `close` beyond the collector's
/// one-second wait, and the line written before the exit is kept.
#[test]
fn a_grandchild_holding_stderr_does_not_hold_close() {
    let directory = tempfile::tempdir().unwrap();
    let session = start(
        directory.path(),
        "cat >/dev/null\nsleep 5 &\necho 'ekr: left a child' >&2\nexit 3\n",
    );
    let started = Instant::now();
    let (status, killed, stderr_tail) = close_failed(session);
    assert!(started.elapsed() < Duration::from_secs(3), "{:?}", started.elapsed());
    assert_eq!(status.code(), Some(3));
    assert!(!killed);
    assert_eq!(stderr_tail.trim_end(), "ekr: left a child");
}
