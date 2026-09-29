//! `task:sdk-close-reports-the-child-stderr`: `ProcessSession::close` reports the child's stderr
//! tail when the session does not exit 0.
//!
//! The non-zero exit and the kill at the deadline are stand-in shell scripts that answer the
//! binary handshake (`--version`); the clean exit is a real `ekr`, built once from this checkout
//! (`ekr_path`).

#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::process::ExitStatusExt as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport as _, TransportError};
use serde_json::Value;

/// The line both stand-ins write to stderr.
const STDERR_LINE: &str = "ekr: cannot write the replay checkpoint: permission denied";

/// Answers `--version` as `ekr 0.0.20`; as a session, reads its input to the end, writes
/// [`STDERR_LINE`] to stderr and exits 3.
const EXITS_THREE: &str = r#"#!/bin/sh
case "$1" in --version) echo 'ekr 0.0.20'; exit 0 ;; esac
cat >/dev/null
echo 'ekr: cannot write the replay checkpoint: permission denied' >&2
exit 3
"#;

/// Answers `--version` as `ekr 0.0.20`; as a session, writes [`STDERR_LINE`] to stderr and then
/// never exits on its own, whatever its input does.
const NEVER_EXITS: &str = r#"#!/bin/sh
case "$1" in --version) echo 'ekr 0.0.20'; exit 0 ;; esac
echo 'ekr: cannot write the replay checkpoint: permission denied' >&2
exec sleep 30
"#;

fn stand_in(directory: &Path, script: &str) -> EkrBinary {
    let path = directory.join("ekr");
    std::fs::write(&path, script).unwrap();
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

/// The workspace root, found at run time from the directory holding `Cargo.lock`: never
/// `env!("CARGO_MANIFEST_DIR")`, which a binary built in another checkout would carry
/// (`AGENTS.md` § The gate).
fn workspace_root() -> PathBuf {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    manifest
        .ancestors()
        .find(|directory| directory.join("Cargo.lock").is_file())
        .expect("a workspace root above the crate")
        .to_path_buf()
}

/// The `ekr` binary of this checkout, built once per test process through cargo, as
/// `tests/session.rs` builds it.
fn ekr_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
        let output = std::process::Command::new(cargo)
            .arg("build")
            .arg("--manifest-path")
            .arg(workspace_root().join("Cargo.toml"))
            .args(["--locked", "-p", "ekr", "--bin", "ekr"])
            .arg("--message-format=json-render-diagnostics")
            .stderr(std::process::Stdio::inherit())
            .output()
            .expect("running cargo build -p ekr");
        assert!(output.status.success(), "cargo build -p ekr failed");
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|message| {
                message["reason"] == "compiler-artifact" && message["target"]["name"] == "ekr"
            })
            .find_map(|message| message["executable"].as_str().map(PathBuf::from))
            .expect("cargo named the ekr executable it built")
    })
}

/// Exit 0 and stdout as text, from the built binary run directly.
fn ekr_text(args: &[&str]) -> String {
    let output = std::process::Command::new(ekr_path())
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn a_session_that_exits_three_at_the_end_of_its_input_reports_exit_three_and_its_stderr() {
    let directory = tempfile::tempdir().unwrap();
    let binary = stand_in(directory.path(), EXITS_THREE);
    let session =
        ProcessSession::start(&binary, store(directory.path()), SessionOptions::default()).unwrap();
    match session.close() {
        Err(TransportError::CloseFailed {
            status,
            killed,
            stderr_tail,
        }) => {
            assert_eq!(status.code(), Some(3), "{status}");
            assert!(!killed, "the child exited on its own");
            assert_eq!(stderr_tail.trim_end(), STDERR_LINE);
        }
        other => panic!("expected CloseFailed with exit 3, got {other:?}"),
    }
}

#[test]
fn the_close_failure_message_names_the_exit_and_the_stderr_line() {
    let directory = tempfile::tempdir().unwrap();
    let binary = stand_in(directory.path(), EXITS_THREE);
    let session =
        ProcessSession::start(&binary, store(directory.path()), SessionOptions::default()).unwrap();
    let message = session.close().unwrap_err().to_string();
    assert!(message.contains("exit status: 3"), "{message}");
    assert!(message.contains(STDERR_LINE), "{message}");
}

#[test]
fn a_real_session_that_exits_zero_closes_as_success() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("host.json"),
        ekr_text(&["example", "ekr.cli-host/1"]),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("seed.yaml"),
        ekr_text(&["example", "ekr-seed/2"]),
    )
    .unwrap();
    let binary = EkrBinary::open(ekr_path()).expect("the built ekr meets the SDK's minimum");
    let options = SessionOptions {
        current_dir: Some(directory.path().to_path_buf()),
        ..SessionOptions::default()
    };
    let mut session = ProcessSession::start(&binary, store(directory.path()), options).unwrap();
    let seeded = session
        .request(&Request::new(["seed", "seed.yaml"]))
        .unwrap();
    assert_eq!(seeded.exit, 0, "{seeded:?}");
    session
        .close()
        .unwrap_or_else(|error| panic!("a clean session did not close as success: {error}"));
}

#[test]
fn a_session_killed_at_the_close_deadline_reports_the_kill_and_its_stderr() {
    let directory = tempfile::tempdir().unwrap();
    let binary = stand_in(directory.path(), NEVER_EXITS);
    let timeout = Duration::from_millis(500);
    let options = SessionOptions {
        timeout,
        ..SessionOptions::default()
    };
    let session = ProcessSession::start(&binary, store(directory.path()), options).unwrap();
    // The stand-in has written its line before `close` starts waiting.
    std::thread::sleep(Duration::from_millis(200));
    let started = Instant::now();
    let closed = session.close();
    let elapsed = started.elapsed();
    match closed {
        Err(TransportError::CloseFailed {
            status,
            killed,
            stderr_tail,
        }) => {
            assert!(killed, "the SDK killed the child at the deadline");
            assert_eq!(status.signal(), Some(9), "{status}");
            assert_eq!(stderr_tail.trim_end(), STDERR_LINE);
        }
        other => panic!("expected CloseFailed with a kill, got {other:?}"),
    }
    assert!(
        elapsed >= timeout,
        "close gave up after {elapsed:?}, before its deadline"
    );
    assert!(
        elapsed < timeout + Duration::from_secs(2),
        "close took {elapsed:?}"
    );
}
