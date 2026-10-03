//! `story:sdk-session-transport`: the SDK drives one child `ekr session` with typed replies and a
//! binary handshake.
//!
//! Every case drives a real `ekr`, built once from this checkout (`ekr_path`), except the replay,
//! which runs after the binary it recorded has been deleted. The SDK is handed that binary by
//! path; nothing here puts it on `PATH`.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use ekr_sdk::binary::{BinaryError, EkrBinary, Version, VersionError};
use ekr_sdk::reply::{Answer, Fault, Outcome, Refusal, Reply};
use ekr_sdk::session::{
    Backend, CancelHandle, Environment, ProcessSession, SessionOptions, StoreConfig,
};
use ekr_sdk::transport::{
    Exchange, Recording, RecordingError, RecordingTransport, ReplayTransport, Request, Transport,
    TransportError,
};
use ekr_sdk::viewer::{Viewer, ViewerError};
use serde_json::{json, Value};

const BACKENDS: [Backend; 2] = [Backend::File, Backend::Sqlite];
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const GLOBEX: (&str, &str) = (
    "00000000-0000-4000-8000-000000000902",
    "00000000-0000-4000-8000-000000000901",
);
const INITECH: (&str, &str) = (
    "00000000-0000-4000-8000-000000000904",
    "00000000-0000-4000-8000-000000000903",
);
const GLOBEX_AGAIN: (&str, &str) = (
    "00000000-0000-4000-8000-000000000906",
    "00000000-0000-4000-8000-000000000905",
);
const UNKNOWN_TRANSACTION: &str = "00000000-0000-4000-8000-000000000999";

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

/// The `ekr` binary of this checkout, built once per test process through cargo: another
/// package's binary has no `CARGO_BIN_EXE_*`. A child test process started by
/// [`the_session_starts_with_an_empty_path_and_never_searches_it`] names it in `EKR_SDK_TEST_EKR`.
fn ekr_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        if let Some(path) = std::env::var_os("EKR_SDK_TEST_EKR") {
            return PathBuf::from(path);
        }
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

/// Exit 0 and stdout as text, from the built binary run directly.
fn ekr_text(args: &[&str]) -> String {
    let output = std::process::Command::new(ekr_path())
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// A `CreateNode` of an Organization known as `Globex`, under a transaction and node id.
fn create(name: &str, (transaction, node): (&str, &str)) -> String {
    format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {transaction}\n  proposer: \
         {OPERATOR}\n  operations:\n  - !CreateNode\n    id: {node}\n    root_id: {ROOT}\n    \
         type_id: {ORGANIZATION}\n    canonical_name: {name}\n    properties: {{}}\n    \
         aliases: [{name}]\n  evidence: []\n"
    )
}

/// A directory holding the example host, the example seed as `seed.yaml`, `create.yaml` and a
/// payload, and a store path on one provider that holds no store yet.
struct World {
    directory: tempfile::TempDir,
    backend: Backend,
}

impl World {
    fn new(backend: Backend) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        world.file("host.json", &ekr_text(&["example", "ekr.cli-host/1"]));
        world.file("seed.yaml", &ekr_text(&["example", "ekr-seed/2"]));
        world.file("create.yaml", &create("Globex", GLOBEX));
        world.file("payload.txt", "Alice is CEO of Acme.\n");
        world
    }

    fn path(&self) -> &Path {
        self.directory.path()
    }

    fn file(&self, name: &str, contents: &str) {
        std::fs::write(self.path().join(name), contents).unwrap();
    }

    fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.path().join(name)).unwrap()
    }

    fn store(&self) -> StoreConfig {
        let store = match self.backend {
            Backend::File => self.path().join("store"),
            Backend::Sqlite => self.path().join("state.db"),
            Backend::Postgres => panic!("local fixture requires a filesystem backend"),
        };
        StoreConfig {
            host: self.path().join("host.json"),
            store,
            backend: self.backend,
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

    /// A session that has seeded the store from `seed.yaml`.
    fn seeded(&self) -> ProcessSession {
        let mut session = self.session();
        let seeded = ok(&mut session, &["seed", "seed.yaml"], None);
        assert_eq!(seeded["result"]["revision"], 0);
        session
    }
}

fn request(argv: &[&str], stdin: Option<&str>) -> Request {
    let request = Request::new(argv.iter().copied());
    match stdin {
        Some(stdin) => request.with_stdin(stdin),
        None => request,
    }
}

/// The document of an exit-0 answer.
fn ok(transport: &mut dyn Transport, argv: &[&str], stdin: Option<&str>) -> Value {
    let reply = transport.request(&request(argv, stdin)).unwrap();
    match reply.answer() {
        Answer::Outcome(outcome) => outcome.document().clone(),
        other => panic!("{argv:?}: {other:?}"),
    }
}

/// Send `signal` to `pid` through `kill(1)`.
fn signal(name: &str, pid: u32) {
    let status = std::process::Command::new("kill")
        .arg(format!("-{name}"))
        .arg(pid.to_string())
        .status()
        .unwrap();
    assert!(status.success(), "kill -{name} {pid}");
}

/// The process `pid` has ended and been reaped: `/proc` no longer lists it.
#[cfg(target_os = "linux")]
fn gone(pid: u32) -> bool {
    !Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(not(target_os = "linux"))]
fn gone(pid: u32) -> bool {
    !std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap()
        .success()
}

#[test]
fn hash_seed_and_the_first_commit_run_through_exactly_one_ekr_process() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut session = world.session();
        let pid = session.id();
        let hashed = ok(&mut session, &["hash", "payload.txt"], None);
        assert_eq!(hashed["content_hash"].as_str().unwrap().len(), 64);
        let minted = ok(&mut session, &["mint", "node"], None);
        assert_eq!(minted["kind"], "node");
        let seeded = ok(&mut session, &["seed", "-"], Some(&world.read("seed.yaml")));
        assert_eq!(seeded["result"]["revision"], 0, "{backend:?}");
        let proposed = ok(&mut session, &["propose", "create.yaml"], None);
        assert_eq!(proposed["transaction_id"], GLOBEX.0);
        let validated = session.request(&request(&["validate", GLOBEX.0], None));
        assert!(matches!(
            validated.unwrap().answer(),
            Answer::Outcome(Outcome::Validated(_))
        ));
        let committed = session.request(&request(&["commit", GLOBEX.0], None));
        match committed.unwrap().answer() {
            Answer::Outcome(Outcome::Committed(document)) => {
                assert_eq!(document["result"]["revision"], 1, "{backend:?}")
            }
            other => panic!("{backend:?}: commit: {other:?}"),
        }
        assert_eq!(ok(&mut session, &["head"], None)["revision"], 1);
        assert_eq!(session.id(), pid, "one child throughout");
        assert_eq!(
            session.processes_started(),
            1,
            "{backend:?}: hash, mint, seed, propose, validate, commit and head in one process"
        );
        session.close().unwrap();
    }
}

#[test]
fn every_exit_is_typed_as_an_outcome_a_refusal_a_fault_or_a_usage_error() {
    let world = World::new(Backend::File);
    world.file("initech.yaml", &create("Initech", INITECH));
    world.file("globex-again.yaml", &create("Globex", GLOBEX_AGAIN));
    let mut session = world.seeded();
    let mut answer = |argv: &[&str]| session.request(&request(argv, None)).unwrap().answer();

    answer(&["propose", "create.yaml"]);
    answer(&["propose", "initech.yaml"]);
    assert!(matches!(
        answer(&["validate", GLOBEX.0]),
        Answer::Outcome(Outcome::Validated(_))
    ));
    assert!(matches!(
        answer(&["validate", INITECH.0]),
        Answer::Outcome(Outcome::Validated(_))
    ));
    assert!(matches!(
        answer(&["commit", GLOBEX.0]),
        Answer::Outcome(Outcome::Committed(_))
    ));
    match answer(&["commit", INITECH.0]) {
        Answer::Outcome(Outcome::Stale(document)) => assert_eq!(document["kind"], "Stale"),
        other => panic!("a commit after the head moved is Stale: {other:?}"),
    }
    answer(&["propose", "globex-again.yaml"]);
    match answer(&["validate", GLOBEX_AGAIN.0]) {
        Answer::Outcome(Outcome::Rejected(document)) => {
            assert_eq!(document["issues"][0]["code"], "alias-already-exists")
        }
        other => panic!("a second Globex is Rejected: {other:?}"),
    }
    match answer(&["head"]) {
        Answer::Outcome(Outcome::Other(document)) => assert_eq!(document["revision"], 1),
        other => panic!("head: {other:?}"),
    }
    match answer(&["commit", UNKNOWN_TRANSACTION]) {
        Answer::Refusal(Refusal { code, reason }) => {
            assert_eq!(code, "ekr.kernel.TransactionNotFound");
            assert!(reason.contains(UNKNOWN_TRANSACTION), "{reason}");
            assert!(!reason.ends_with('\n'));
        }
        other => panic!("an unknown transaction is a named refusal: {other:?}"),
    }
    match answer(&["propose", "missing.yaml"]) {
        Answer::Fault(Fault { exit, message }) => {
            assert_eq!(exit, 1);
            assert!(message.contains("missing.yaml"), "{message}");
        }
        other => panic!("an unreadable document is a fault: {other:?}"),
    }
    match answer(&["snapshot", "--at", "zero"]) {
        Answer::Usage(message) => assert!(message.contains("zero"), "{message}"),
        other => panic!("a malformed argv is clap's usage error: {other:?}"),
    }
}

#[test]
fn a_session_killed_mid_request_fails_that_call_naming_the_verb_and_every_later_call_without_a_restart(
) {
    let world = World::new(Backend::File);
    let mut session = world.session();
    ok(&mut session, &["mint", "node"], None);
    let pid = session.id();
    // Stopped, the child cannot answer: the request below is in flight when the kill lands.
    signal("STOP", pid);
    let failed = std::thread::scope(|scope| {
        let call = scope.spawn(|| session.request(&request(&["mint", "edge"], None)));
        std::thread::sleep(Duration::from_millis(300));
        signal("KILL", pid);
        call.join().unwrap()
    });
    let error = failed.unwrap_err();
    assert!(
        matches!(&error, TransportError::Died { verb, stderr_tail, .. }
            if verb == "mint" && stderr_tail.is_empty()),
        "{error:?}"
    );
    let message = error.to_string();
    assert!(message.contains("`mint`"), "names the verb: {message}");
    assert!(message.contains("stderr tail"), "names the tail: {message}");
    assert!(message.contains("SIGKILL"), "names how it ended: {message}");
    assert!(gone(pid));

    for argv in [["head"].as_slice(), &["mint", "node"]] {
        let later = session.request(&request(argv, None)).unwrap_err();
        assert!(
            matches!(&later, TransportError::Latched { verb, cause }
                if verb == argv[0] && cause.contains("SIGKILL")),
            "{later:?}"
        );
        assert!(later.to_string().contains(&format!("`{}`", argv[0])));
    }
    assert_eq!(session.processes_started(), 1, "no restart");
}

#[test]
fn a_session_that_dies_names_the_tail_of_what_it_wrote_to_stderr() {
    let world = World::new(Backend::File);
    let mut seeded = world.seeded_with_broken_host();
    let error = seeded
        .request(&request(&["mint", "node"], None))
        .unwrap_err();
    match &error {
        TransportError::Died {
            verb, stderr_tail, ..
        } => {
            assert_eq!(verb, "mint");
            assert!(stderr_tail.starts_with("ekr: "), "{stderr_tail:?}");
            assert!(stderr_tail.len() <= ekr_sdk::session::STDERR_TAIL_BYTES);
            assert!(error.to_string().contains(stderr_tail.trim_end()));
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        seeded.request(&request(&["hash", "-"], Some("x"))),
        Err(TransportError::Latched { .. })
    ));
    assert_eq!(seeded.processes_started(), 1);
}

impl World {
    /// A session over an existing store whose host document no longer matches: the child writes
    /// its refusal to stderr and exits before answering anything.
    fn seeded_with_broken_host(&self) -> ProcessSession {
        let setup = std::process::Command::new(ekr_path())
            .current_dir(self.path())
            .args([
                "--host",
                "host.json",
                "--store",
                "store",
                "--backend",
                "file",
            ])
            .args(["seed", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .and_then(|mut child| {
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(ekr_text(&["example", "ekr-seed/2"]).as_bytes())?;
                child.wait()
            })
            .unwrap();
        assert!(setup.success());
        self.file("host.json", "{\"format\": \"not a host\"}");
        self.session()
    }
}

#[test]
fn a_request_over_the_line_cap_returns_the_document_of_the_one_shot_verb() {
    let world = World::new(Backend::File);
    let mut session = world.session();
    // Each U+0001 is six bytes of JSON: the line is over the cap while the text is 4.3 MB.
    let payload = "\u{1}".repeat(4_300_000);
    let line = serde_json::to_vec(&json!({"argv": ["hash", "-"], "stdin": payload})).unwrap();
    assert_eq!(ekr_sdk::session::LINE_CAP, 25_231_360);
    assert!(
        line.len() > ekr_sdk::session::LINE_CAP,
        "{} bytes",
        line.len()
    );

    let reply = session
        .request(&request(&["hash", "-"], Some(&payload)))
        .unwrap();
    assert_eq!(reply.exit, 0, "{}", reply.stderr);

    let mut one_shot = std::process::Command::new(ekr_path())
        .args(["hash", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = one_shot.stdin.take().unwrap();
    let writer = std::thread::spawn(move || input.write_all(payload.as_bytes()));
    let output = one_shot.wait_with_output().unwrap();
    writer.join().unwrap().unwrap();
    let expected: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reply.document.as_ref(), Some(&expected));
    assert_eq!(expected["byte_len"], 4_300_000);

    assert_eq!(
        session.processes_started(),
        1,
        "a `-` over the cap is read from a file inside the session"
    );

    // An argv with no `-` leaves the text unread; its line is over the cap only because of it,
    // so it runs as a one-shot process and answers as the session does.
    world.file("payload.txt", "Alice is CEO of Acme.\n");
    let in_session = ok(&mut session, &["hash", "payload.txt"], None);
    let one_shot = ok(
        &mut session,
        &["hash", "payload.txt"],
        Some(&"x".repeat(LINE_CAP_PLUS)),
    );
    assert_eq!(one_shot, in_session);
    assert_eq!(
        session.processes_started(),
        2,
        "the session and one one-shot"
    );
    ok(&mut session, &["mint", "node"], None);
    assert_eq!(session.processes_started(), 2, "the session still serves");
}

/// One byte more than the session's line cap.
const LINE_CAP_PLUS: usize = 25_231_361;

#[test]
fn a_binary_that_never_answers_its_probe_is_killed_and_refused() {
    let directory = tempfile::tempdir().unwrap();
    let hangs = directory.path().join("ekr");
    std::fs::write(&hangs, "#!/bin/sh\nexec sleep 30\n").unwrap();
    std::fs::set_permissions(
        &hangs,
        <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
    )
    .unwrap();
    assert_eq!(ekr_sdk::binary::PROBE_TIMEOUT, Duration::from_secs(5));
    let started = Instant::now();
    let refused = EkrBinary::open_with(&hangs, Version::new(0, 0, 1), Duration::from_millis(300))
        .unwrap_err();
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
    assert!(
        matches!(&refused, BinaryError::TimedOut { what, .. } if what == "--version"),
        "{refused:?}"
    );
    assert!(refused.to_string().contains("--version"), "{refused}");
}

#[test]
fn an_ekr_below_the_minimum_version_is_refused_naming_both_versions() {
    let binary = binary();
    let found = binary.version();
    assert!(found >= ekr_sdk::binary::MINIMUM_VERSION);
    assert_eq!(
        ekr_text(&["--version"]).trim(),
        format!("ekr {found}"),
        "the version is what `ekr --version` prints"
    );
    assert_eq!(binary.path(), ekr_path());

    let minimum = Version::new(found.major + 1, 0, 0);
    let error = EkrBinary::open_with_minimum(ekr_path(), minimum).unwrap_err();
    assert!(
        matches!(&error, BinaryError::TooOld { found: f, minimum: m, .. } if *f == found && *m == minimum),
        "{error:?}"
    );
    let message = error.to_string();
    assert!(message.contains(&found.to_string()), "{message}");
    assert!(message.contains(&minimum.to_string()), "{message}");
    assert_eq!(
        "0.0.14".parse::<Version>().unwrap(),
        ekr_sdk::binary::MINIMUM_VERSION
    );
    assert_eq!("1.2.3-rc.1".parse::<Version>(), Ok(Version::new(1, 2, 3)));
    for text in ["0.0", "1.2.3.4", "1.x.3", "", "1..3"] {
        let refused: Result<Version, VersionError> = text.parse();
        assert!(refused.is_err(), "{text:?}");
    }

    let operations = binary.operations().unwrap();
    assert!(operations.iter().any(|kind| kind == "CreateNode"));
    assert_eq!(
        operations.len(),
        ekr_sdk::document::OperationKind::ALL.len(),
        "{operations:?}"
    );
    binary
        .require_operations(&["CreateNode", "AddAssertion"])
        .unwrap();
    let missing = binary
        .require_operations(&["CreateNode", "NoSuchOperation"])
        .unwrap_err();
    assert!(
        matches!(&missing, BinaryError::MissingOperations { missing, .. } if missing == &["NoSuchOperation"]),
        "{missing:?}"
    );
    assert!(missing.to_string().contains("NoSuchOperation"));
}

/// The environment of a live process, from `/proc/<pid>/environ`.
#[cfg(target_os = "linux")]
fn environment_of(pid: u32) -> BTreeMap<String, String> {
    std::fs::read(format!("/proc/{pid}/environ"))
        .unwrap()
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let entry = String::from_utf8_lossy(entry).into_owned();
            let (name, value) = entry.split_once('=').unwrap_or((&entry, ""));
            (name.to_owned(), value.to_owned())
        })
        .collect()
}

/// The environment of the `ekr` process `pid` is exactly `expected`. A failure names variables
/// and never prints a value it did not expect: the consumer's environment can hold credentials.
#[cfg(target_os = "linux")]
fn assert_environment(pid: u32, expected: &BTreeMap<String, String>) {
    let exe = std::fs::read_link(format!("/proc/{pid}/exe")).unwrap();
    assert_eq!(
        exe.canonicalize().unwrap(),
        ekr_path().canonicalize().unwrap(),
        "process {pid} runs ekr"
    );
    let found = environment_of(pid);
    let names: Vec<&String> = found.keys().collect();
    assert_eq!(
        names,
        expected.keys().collect::<Vec<_>>(),
        "the child's variables, by name"
    );
    for (name, value) in expected {
        assert_eq!(&found[name], value, "{name}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn the_child_environment_holds_only_the_store_variables_or_exactly_the_given_list() {
    let world = World::new(Backend::Sqlite);
    let store = world.store();
    let mut session = world.session();
    let expected: BTreeMap<String, String> = [
        ("EKR_HOST", store.host.to_str().unwrap()),
        ("EKR_STORE", store.store.to_str().unwrap()),
        ("EKR_BACKEND", Backend::Sqlite.as_str()),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect();
    // A reply means the child has exec'd ekr: `spawn` can return while it still runs this image.
    ok(&mut session, &["mint", "node"], None);
    assert_environment(session.id(), &expected);

    let list = vec![
        (OsString::from("EKR_FULL_REPLAY"), OsString::from("0")),
        (OsString::from("SDK_PROBE"), OsString::from("given")),
    ];
    let options = SessionOptions {
        environment: Environment::Exact(list.clone()),
        ..world.options()
    };
    let mut exact = ProcessSession::start(&binary(), world.store(), options).unwrap();
    let expected: BTreeMap<String, String> = list
        .iter()
        .map(|(name, value)| {
            (
                name.to_str().unwrap().to_owned(),
                value.to_str().unwrap().to_owned(),
            )
        })
        .collect();
    ok(&mut exact, &["mint", "node"], None);
    assert_environment(exact.id(), &expected);
    assert_eq!(Environment::default(), Environment::Store);
    assert_eq!(Backend::File.as_str(), "file");
    assert_eq!(Backend::Sqlite.as_str(), "sqlite");
}

/// Run one `#[ignore]`d case of this binary in a child test process with `environment` added.
fn run_child_case(case: &str, environment: &[(&str, &OsString)]) {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            case,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("EKR_SDK_TEST_EKR", ekr_path());
    for (name, value) in environment {
        command.env(name, value);
    }
    let output = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{case}: {stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_session_starts_with_an_empty_path_and_never_searches_it() {
    run_child_case("child_case_empty_path", &[("PATH", &OsString::new())]);
    let directory = ekr_path().parent().unwrap().as_os_str().to_owned();
    run_child_case("child_case_bare_names_on_path", &[("PATH", &directory)]);
}

#[test]
#[ignore = "run by the_session_starts_with_an_empty_path_and_never_searches_it, with PATH empty"]
fn child_case_empty_path() {
    assert_eq!(std::env::var_os("PATH"), Some(OsString::new()));
    let world = World::new(Backend::File);
    let mut session = world.session();
    ok(&mut session, &["mint", "node"], None);
}

#[test]
#[ignore = "run by the_session_starts_with_an_empty_path_and_never_searches_it, ekr on PATH"]
fn child_case_bare_names_on_path() {
    let on_path = Path::new(&std::env::var_os("PATH").unwrap()).join("ekr");
    assert!(on_path.is_file(), "a PATH search would find {on_path:?}");
    assert!(matches!(EkrBinary::open(""), Err(BinaryError::NoPath)));
    let refused = EkrBinary::open("ekr").unwrap_err();
    assert!(
        matches!(&refused, BinaryError::BareName { name } if name == "ekr"),
        "{refused:?}"
    );
    assert!(refused.to_string().contains("PATH"), "{refused}");
}

#[test]
fn a_cancel_from_a_sigterm_handler_ends_the_child_within_one_second_and_latches() {
    let world = World::new(Backend::File);
    let mut session = world.session();
    ok(&mut session, &["mint", "node"], None);
    let pid = session.id();
    let handle: CancelHandle = session.cancel_handle();
    signal_hook::flag::register(signal_hook::consts::SIGTERM, handle.flag()).unwrap();
    signal("STOP", pid);
    let (failed, elapsed) = std::thread::scope(|scope| {
        let call = scope.spawn(|| session.request(&request(&["mint", "edge"], None)));
        std::thread::sleep(Duration::from_millis(300));
        let signalled = Instant::now();
        signal("TERM", std::process::id());
        let failed = call.join().unwrap();
        (failed, signalled.elapsed())
    });
    assert!(handle.is_cancelled());
    assert!(elapsed < Duration::from_secs(1), "{elapsed:?}");
    assert!(gone(pid), "the child has ended");
    let error = failed.unwrap_err();
    assert!(
        matches!(&error, TransportError::Cancelled { verb } if verb == "mint"),
        "{error:?}"
    );
    let later = session.request(&request(&["head"], None)).unwrap_err();
    assert!(matches!(later, TransportError::Latched { .. }), "{later:?}");
    assert_eq!(session.processes_started(), 1);
}

#[test]
fn a_cancel_between_requests_ends_the_child_within_one_second() {
    let world = World::new(Backend::File);
    let mut session = world.session();
    ok(&mut session, &["mint", "node"], None);
    let pid = session.id();
    let handle = session.cancel_handle();
    handle.cancel();
    let cancelled = Instant::now();
    while !gone(pid) && cancelled.elapsed() < Duration::from_secs(1) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        gone(pid),
        "ended {:?} after the cancel",
        cancelled.elapsed()
    );
    let error = session
        .request(&request(&["mint", "node"], None))
        .unwrap_err();
    assert!(
        matches!(error, TransportError::Cancelled { .. }),
        "{error:?}"
    );
    assert!(handle.flag().load(Ordering::SeqCst));
}

#[test]
fn a_request_past_its_timeout_stops_the_child_and_latches() {
    let world = World::new(Backend::File);
    let options = SessionOptions {
        timeout: Duration::from_millis(500),
        ..world.options()
    };
    let mut session = ProcessSession::start(&binary(), world.store(), options).unwrap();
    ok(&mut session, &["mint", "node"], None);
    let pid = session.id();
    signal("STOP", pid);
    let error = session
        .request(&request(&["hash", "-"], Some("x")))
        .unwrap_err();
    assert!(
        matches!(&error, TransportError::TimedOut { verb, .. } if verb == "hash"),
        "{error:?}"
    );
    assert!(gone(pid));
    assert!(matches!(
        session.request(&request(&["mint", "node"], None)),
        Err(TransportError::Latched { .. })
    ));
}

/// What a consumer does, through any transport: hash, seed, one write trio, head. Every request
/// is derived from constants and earlier replies, so a replay sees the same requests.
fn consumer(transport: &mut dyn Transport, seed: &str) -> Vec<Value> {
    let hashed = ok(transport, &["hash", "-"], Some("Alice is CEO of Acme.\n"));
    let minted = ok(transport, &["mint", "node"], None);
    let node = minted["id"].as_str().unwrap().to_owned();
    ok(transport, &["seed", "-"], Some(seed));
    let document = create("Globex", (GLOBEX.0, &node));
    ok(transport, &["propose", "-"], Some(&document));
    ok(transport, &["validate", GLOBEX.0], None);
    let committed = ok(transport, &["commit", GLOBEX.0], None);
    let head = ok(transport, &["head"], None);
    vec![hashed, minted, committed, head]
}

#[test]
fn a_replayed_recording_passes_with_no_ekr_binary() {
    let world = World::new(Backend::File);
    let installed = world.path().join("bin");
    std::fs::create_dir(&installed).unwrap();
    let copy = installed.join("ekr");
    // A link, not a copy: a copy is written by this process, and a test thread that forks while
    // the write is open leaves the new file busy (ETXTBSY) when it is executed.
    std::os::unix::fs::symlink(std::fs::canonicalize(ekr_path()).unwrap(), &copy).unwrap();
    let seed = world.read("seed.yaml");

    let recording_file = world.path().join("recording.json");
    let recorded = {
        let binary = EkrBinary::open(&copy).unwrap();
        let session = ProcessSession::start(&binary, world.store(), world.options()).unwrap();
        let mut recorder = RecordingTransport::record(session);
        let answers = consumer(&mut recorder, &seed);
        assert_eq!(recorder.recording().exchanges.len(), 7);
        let recording: Recording = recorder.into_recording();
        std::fs::write(&recording_file, recording.to_json()).unwrap();
        answers
    };
    assert_eq!(recorded[3]["revision"], 1);
    std::fs::remove_file(&copy).unwrap();
    assert!(!copy.exists(), "no ekr binary is left to run");

    let recording =
        Recording::from_json(&std::fs::read_to_string(&recording_file).unwrap()).unwrap();
    assert_eq!(recording.format, ekr_sdk::transport::RECORDING_FORMAT);
    let first: &Exchange = &recording.exchanges[0];
    assert_eq!(first.request.verb(), "hash");
    let reply: &Reply = &first.reply;
    assert_eq!(reply.exit, 0);

    let mut replay = ReplayTransport::new(recording.clone());
    assert_eq!(consumer(&mut replay, &seed), recorded);
    assert_eq!(replay.remaining(), 0);

    let mut diverging = ReplayTransport::new(recording);
    let error = diverging
        .request(&request(&["hash", "-"], Some("something else")))
        .unwrap_err();
    assert!(
        matches!(&error, TransportError::Replay { verb, .. } if verb == "hash"),
        "{error:?}"
    );
    assert!(matches!(
        Recording::from_json("{\"format\": \"other\", \"exchanges\": []}"),
        Err(RecordingError::Format { found }) if found == "other"
    ));
    assert!(matches!(
        Recording::from_json("[]"),
        Err(RecordingError::Json(_))
    ));
}

#[test]
fn the_sdk_depends_on_no_async_runtime_in_any_feature() {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let output = std::process::Command::new(cargo)
        .arg("tree")
        .arg("--manifest-path")
        .arg(workspace_root().join("Cargo.toml"))
        .args(["--locked", "-p", "ekr-sdk", "--all-features"])
        .args(["--edges", "normal,build", "--prefix", "none"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8(output.stdout).unwrap();
    assert!(tree.starts_with("ekr-sdk v"), "{tree}");
    assert!(tree.lines().any(|line| line.starts_with("ekr-core v")));
    for runtime in ["tokio", "async-std", "smol", "futures"] {
        assert!(
            !tree
                .lines()
                .any(|line| line.starts_with(&format!("{runtime} v"))),
            "{runtime} in:\n{tree}"
        );
    }
}

#[test]
fn viewer_spawn_zero_returns_a_url_whose_head_answers() {
    let world = World::new(Backend::File);
    drop(world.seeded());
    let viewer = Viewer::spawn(&binary(), &world.store(), 0).unwrap();
    let url = viewer.url().to_owned();
    let authority = url
        .strip_prefix("http://")
        .and_then(|rest| rest.strip_suffix('/'))
        .unwrap_or_else(|| panic!("{url}"));
    assert!(authority.starts_with("127.0.0.1:"), "{url}");
    assert!(!authority.ends_with(":0"), "a port was picked: {url}");

    let mut stream = std::net::TcpStream::connect(authority).unwrap();
    write!(
        stream,
        "GET /head HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let body = response.split("\r\n\r\n").nth(1).unwrap();
    let head: Value = serde_json::from_str(body).unwrap();
    assert_eq!(head, json!({"format": "ekr.view-head/1", "head": 0}));
    viewer.stop();

    let absent = World::new(Backend::File);
    let refused = Viewer::spawn(&binary(), &absent.store(), 0).err().unwrap();
    assert!(
        matches!(&refused, ViewerError::NoUrl { stderr_tail, .. } if stderr_tail.contains("store-not-found")),
        "{refused:?}"
    );
}
