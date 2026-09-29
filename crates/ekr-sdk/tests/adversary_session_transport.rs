//! Adversary cases for `story:sdk-session-transport`, wave sdk-01, unit T.
//!
//! Each case drives the SDK against a real `ekr` built from this checkout, or against a stand-in
//! executable written into the case's own temporary directory. No case prints a value of any
//! environment variable: a child's environment is read by name only.

#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig, LINE_CAP};
use ekr_sdk::transport::{Request, Transport};
use serde_json::Value;

const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const GLOBEX: (&str, &str) = (
    "00000000-0000-4000-8000-000000000902",
    "00000000-0000-4000-8000-000000000901",
);

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

/// The `ekr` binary of this checkout, built once per test process.
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

fn binary() -> EkrBinary {
    EkrBinary::open(ekr_path()).expect("the built ekr meets the SDK's minimum")
}

fn ekr_text(args: &[&str]) -> String {
    let output = std::process::Command::new(ekr_path())
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn create(name: &str, (transaction, node): (&str, &str)) -> String {
    format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {transaction}\n  proposer: \
         {OPERATOR}\n  operations:\n  - !CreateNode\n    id: {node}\n    root_id: {ROOT}\n    \
         type_id: {ORGANIZATION}\n    canonical_name: {name}\n    properties: {{}}\n    \
         aliases: [{name}]\n  evidence: []\n"
    )
}

/// A temporary directory with the example host, `create.yaml`, and a file store path that holds
/// no store yet.
struct World {
    directory: tempfile::TempDir,
}

impl World {
    fn new() -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        world.file("host.json", &ekr_text(&["example", "ekr.cli-host/1"]));
        world.file("create.yaml", &create("Globex", GLOBEX));
        world
    }

    fn path(&self) -> &Path {
        self.directory.path()
    }

    fn file(&self, name: &str, contents: &str) {
        std::fs::write(self.path().join(name), contents).unwrap();
    }

    fn store(&self) -> StoreConfig {
        StoreConfig {
            host: self.path().join("host.json"),
            store: self.path().join("store"),
            backend: Backend::File,
        }
    }

    fn options(&self) -> SessionOptions {
        SessionOptions {
            current_dir: Some(self.path().to_path_buf()),
            ..SessionOptions::default()
        }
    }

    fn session(&self) -> ProcessSession {
        ProcessSession::start(&binary(), self.store(), self.options()).unwrap()
    }

    /// An executable stand-in for `ekr`, a `/bin/sh` script, written into this world.
    fn stand_in(&self, name: &str, script: &str) -> PathBuf {
        let path = self.path().join(name);
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
}

fn request(argv: &[&str], stdin: Option<&str>) -> Request {
    let request = Request::new(argv.iter().copied());
    match stdin {
        Some(stdin) => request.with_stdin(stdin),
        None => request,
    }
}

fn exit_zero(transport: &mut dyn Transport, argv: &[&str], stdin: Option<&str>) -> Value {
    let reply = transport.request(&request(argv, stdin)).unwrap();
    assert_eq!(reply.exit, 0, "{argv:?}: {}", reply.stderr);
    reply.document.unwrap_or(Value::Null)
}

fn signal(name: &str, pid: u32) {
    let status = std::process::Command::new("kill")
        .arg(format!("-{name}"))
        .arg(pid.to_string())
        .status()
        .unwrap();
    assert!(status.success(), "kill -{name} {pid}");
}

fn gone(pid: u32) -> bool {
    !Path::new(&format!("/proc/{pid}")).exists()
}

/// The head revision of each replay checkpoint the file provider at `store` retains, read the
/// way `crates/ekr/tests/session.rs` reads them.
fn retained_checkpoint_revisions(store: &Path) -> Vec<u64> {
    let mut revisions: Vec<u64> = std::fs::read_dir(store.join("blobs"))
        .unwrap()
        .filter_map(|entry| {
            let bytes = std::fs::read(entry.unwrap().path()).unwrap();
            if !bytes.starts_with(br#"{"format":"ekr.replay-checkpoint/1""#) {
                return None;
            }
            let checkpoint: Value = serde_json::from_slice(&bytes).unwrap();
            checkpoint["revision"].as_u64()
        })
        .collect();
    revisions.sort_unstable();
    revisions
}

/// The unhandled edge the implementor named. A `--create` session started on a path holding no
/// store gets a `seed -` whose line is over the cap, so the SDK runs it as a one-shot process.
/// `docs/sdk.md` promises the session "stays open and serves the next request", and `docs/cli.md`
/// § `ekr session` promises that the seed that creates the store "leaves the session holding
/// it" and that the session writes the checkpoint of the head it reached when its input ends.
/// The one-shot seed happens outside the child, so the child never holds the store: every later
/// request reopens it, and `close` leaves the seed's checkpoint instead of the session's head.
/// The same requests with the seed under the cap leave `[1]`
/// (`crates/ekr/tests/session.rs`, `a_session_leaves_the_checkpoint_of_its_head_when_its_input_ends`).
#[test]
fn a_seed_over_the_line_cap_leaves_the_create_session_holding_the_store() {
    let world = World::new();
    let mut session = world.session();
    // A YAML comment of JSON-escaped quotes: two bytes of line per byte of text.
    let mut seed = ekr_text(&["example", "ekr-seed/2"]);
    if !seed.ends_with('\n') {
        seed.push('\n');
    }
    seed.push_str("# ");
    seed.push_str(&"\"".repeat(LINE_CAP / 2 + 1));
    seed.push('\n');

    let seeded = exit_zero(&mut session, &["seed", "-"], Some(&seed));
    assert_eq!(seeded["result"]["revision"], 0);
    assert_eq!(
        session.processes_started(),
        1,
        "the seed read its over-cap text from a file inside the session"
    );

    exit_zero(&mut session, &["propose", "create.yaml"], None);
    exit_zero(&mut session, &["validate", GLOBEX.0], None);
    let committed = exit_zero(&mut session, &["commit", GLOBEX.0], None);
    assert_eq!(committed["result"]["revision"], 1);
    assert!(session.close().unwrap().success());

    assert_eq!(
        retained_checkpoint_revisions(&world.store().store),
        [1],
        "a closed session leaves the checkpoint of the head it reached, as it does when the \
         seed runs inside it"
    );
}

/// `CancelHandle::cancel` is documented to "stop the session's child", and `docs/sdk.md` says a
/// thread in the session checks the flag every 20 ms and kills the child. `close` (and `Drop` of
/// a healthy session) first sets `closing`, which ends the watcher thread, then waits up to the
/// request timeout without looking at the flag. A SIGTERM whose handler cancels while the
/// consumer is closing a session with a slow child is ignored until the timeout runs out.
#[test]
fn a_cancel_while_closing_ends_the_child_within_one_second() {
    let world = World::new();
    let options = SessionOptions {
        timeout: Duration::from_secs(6),
        ..world.options()
    };
    let mut session = ProcessSession::start(&binary(), world.store(), options).unwrap();
    exit_zero(&mut session, &["mint", "node"], None);
    let pid = session.id();
    let handle = session.cancel_handle();
    // Stopped, the child cannot finish exiting: `close` waits on it as on a slow checkpoint.
    signal("STOP", pid);
    let closer = std::thread::spawn(move || session.close());
    std::thread::sleep(Duration::from_millis(300));
    handle.cancel();
    let cancelled = Instant::now();
    while !gone(pid) && cancelled.elapsed() < Duration::from_secs(1) {
        std::thread::sleep(Duration::from_millis(10));
    }
    let ended_in = cancelled.elapsed();
    let ended = gone(pid);
    let _ = closer.join();
    assert!(
        ended,
        "the child was still running {ended_in:?} after the cancel; it ended only when close's \
         timeout ran out"
    );
}

/// `EkrBinary::open` runs `--version` with `Command::output` and no bound, so a binary that does
/// not answer blocks the consumer forever. Every other process the SDK starts runs under a
/// timeout (`SessionOptions::timeout`, the viewer's 120 s URL wait).
#[test]
fn opening_a_binary_that_never_answers_version_returns() {
    let world = World::new();
    let hangs = world.stand_in("ekr", "#!/bin/sh\nexec sleep 20\n");
    let (sent, received) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sent.send(EkrBinary::open(&hangs).map(|binary| binary.version()));
    });
    let opened = received.recv_timeout(Duration::from_secs(10));
    assert!(
        opened.is_ok(),
        "EkrBinary::open had not returned 10 s after starting a binary that never answers \
         --version"
    );
}

/// `docs/sdk.md` § Requests over the line cap: the SDK "returns that process's exit status,
/// document and stderr as the reply", and `Reply::stderr` is "the text the verb wrote to stderr".
/// The one-shot path keeps only the last `STDERR_TAIL_BYTES` (4096) of it, so a one-shot reply's
/// stderr differs from the session reply to the same argv whenever the verb writes more. The
/// argv here is identical in both requests and does not read `-`; only the one-shot request's
/// unread stdin puts it over the cap.
#[test]
fn a_one_shot_reply_carries_the_whole_stderr_the_verb_wrote() {
    let world = World::new();
    let mut session = world.session();
    // A missing document whose path is longer than 4096 bytes: the fault names the path.
    let long = format!("{}missing.yaml", "d/".repeat(2_200));
    let argv = ["propose", long.as_str()];

    let in_session = session.request(&request(&argv, None)).unwrap();
    assert_eq!(session.processes_started(), 1);
    assert!(
        in_session.stderr.len() > ekr_sdk::session::STDERR_TAIL_BYTES,
        "the verb wrote {} bytes of stderr",
        in_session.stderr.len()
    );

    let unread = "x".repeat(LINE_CAP + 1);
    let one_shot = session.request(&request(&argv, Some(&unread))).unwrap();
    assert_eq!(
        session.processes_started(),
        2,
        "the second ran as a one-shot"
    );
    assert_eq!(one_shot.exit, in_session.exit);
    assert_eq!(
        one_shot.stderr.len(),
        in_session.stderr.len(),
        "the one-shot reply's stderr is the whole of what the verb wrote; it starts {:?}",
        one_shot.stderr.chars().take(40).collect::<String>()
    );
    assert_eq!(one_shot.answer(), in_session.answer());
}
