//! Adversary pass 1 on `task:read-verbs-open-a-read-only-store`, at the binary.
//!
//! Each case builds a store the way `read_only_store.rs` does — seeded through `ekr seed`, then
//! made read-only with file permissions — and attacks what that file's cases do not reach: a
//! long-lived reader while another process commits, a SQLite database that is read-only in a
//! directory that is not, the private copy's lifetime when `ekr view` is terminated, and single
//! paths of a store made read-only on their own.

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Output, Stdio};

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const TRANSACTION: &str = "00000000-0000-4000-8000-000000000601";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
    .join("tests/fixtures/retraction")
    .join(name)
}

struct World {
    outer: tempfile::TempDir,
    directory: PathBuf,
    backend: &'static str,
}

impl World {
    fn seeded(backend: &'static str) -> Self {
        let outer = tempfile::tempdir().unwrap();
        let directory = outer.path().join("world");
        std::fs::create_dir(&directory).unwrap();
        let world = Self {
            outer,
            directory,
            backend,
        };
        let seed = fixture("seed.yaml").display().to_string();
        let seeded = world.run(&["seed", &seed]);
        assert_eq!(seeded.status.code(), Some(0), "{}", stderr(&seeded));
        world
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.join("store"),
            _ => self.directory.join("state.db"),
        }
    }

    /// A directory of its own to use as the child's `TMPDIR`.
    fn tmpdir(&self) -> PathBuf {
        let tmpdir = self.outer.path().join("tmpdir");
        std::fs::create_dir_all(&tmpdir).unwrap();
        tmpdir
    }

    fn command(&self, verb: &[&str]) -> std::process::Command {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
        for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
            command.env_remove(var);
        }
        command
            .arg("--host")
            .arg(fixture("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(verb);
        command
    }

    fn run(&self, verb: &[&str]) -> Output {
        self.command(verb).stdin(Stdio::null()).output().unwrap()
    }

    fn feed(&self, verb: &[&str], stdin: &str) -> Output {
        let mut child = self
            .command(verb)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }

    fn read_only(&self) -> ReadOnly {
        set_modes(&self.directory, 0o555, 0o444);
        ReadOnly(self.directory.clone())
    }

    /// Another process with write access proposes, validates and commits the fixture's
    /// transaction, and returns the head a one-shot `ekr head` then prints.
    fn commit_elsewhere(&self) -> Value {
        let propose = fixture("propose-alice.yaml").display().to_string();
        for verb in [
            vec!["propose", propose.as_str()],
            vec!["validate", TRANSACTION, "--against", "0"],
            vec!["commit", TRANSACTION],
        ] {
            let output = self.run(&verb);
            assert_eq!(
                output.status.code(),
                Some(0),
                "{} {verb:?}: {}",
                self.backend,
                stderr(&output)
            );
        }
        self.head()
    }

    fn head(&self) -> Value {
        let head = self.run(&["head"]);
        assert_eq!(head.status.code(), Some(0), "{}", stderr(&head));
        serde_json::from_slice(&head.stdout).unwrap()
    }

    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

struct ReadOnly(PathBuf);

impl Drop for ReadOnly {
    fn drop(&mut self) {
        set_modes(&self.0, 0o755, 0o644);
    }
}

fn set_modes(at: &Path, directory: u32, file: u32) {
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(0o755)).unwrap();
    for path in walk(at) {
        let mode = if path.is_dir() { directory } else { file };
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
    }
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(directory)).unwrap();
}

fn walk(at: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(at).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(walk(&path));
        }
        found.push(path);
    }
    found
}

fn chmod(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A long-lived child answering one JSON line per line it is sent.
struct Served {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Served {
    fn start(world: &World, verb: &[&str]) -> Self {
        let mut child = world
            .command(verb)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            output,
        }
    }

    fn send(&mut self, line: &Value) {
        writeln!(self.input, "{line}").unwrap();
        self.input.flush().unwrap();
    }

    fn ask(&mut self, line: &Value) -> Value {
        self.send(line);
        let mut answer = String::new();
        self.output.read_line(&mut answer).unwrap();
        serde_json::from_str(&answer).unwrap_or_else(|_| panic!("not JSON: {answer:?}"))
    }

    fn end(self) {
        drop(self.input);
        let mut child = self.child;
        assert!(child.wait().unwrap().success());
    }
}

fn copies(tmpdir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(tmpdir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("ekr-read-only-"))
        })
        .collect()
}

/// The control for the case below: on a writable store, a session reads a commit another process
/// made after it started (docs/cli.md, `ekr session`: "a transaction this session or another
/// process committed is what the next one reads").
#[test]
fn a_session_on_a_writable_store_reads_another_process_commit() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let before = world.head();
        let mut session = Served::start(&world, &["session"]);
        let first = session.ask(&json!({"argv": ["head"]}));
        assert_eq!(first["stdout"], before, "{backend}: {first}");
        let after = world.commit_elsewhere();
        assert_ne!(after, before, "{backend}: the commit moved the head");
        let second = session.ask(&json!({"argv": ["head"]}));
        assert_eq!(second["stdout"], after, "{backend}: {second}");
        session.end();
    }
}

/// A session started on a store this process may not write holds a private copy (file) or an
/// in-memory image (SQLite) taken when it opened. When the store's owner then commits, the
/// session keeps answering from the copy: the head it prints never moves, where on a writable
/// store it does (the control above). The owner is simulated by restoring write permission
/// between the session's two reads; the session's own identity check sees the same inode.
#[test]
fn a_session_on_a_read_only_store_reads_another_process_commit() {
    let mut stale = Vec::new();
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let before = world.head();
        let guard = world.read_only();
        let mut session = Served::start(&world, &["session"]);
        let first = session.ask(&json!({"argv": ["head"]}));
        assert_eq!(first["stdout"], before, "{backend}: {first}");
        drop(guard);
        let after = world.commit_elsewhere();
        assert_ne!(after, before, "{backend}: the commit moved the head");
        let second = session.ask(&json!({"argv": ["head"]}));
        session.end();
        if second["stdout"] != after {
            stale.push(format!(
                "{backend}: answered {} after the commit to {}",
                second["stdout"]["revision"], after["revision"]
            ));
        }
    }
    assert!(
        stale.is_empty(),
        "the session on a read-only store still answers the head it opened at: {stale:#?}"
    );
}

/// The same for `ekr mcp`, whose page says "Every tool reads the store as it stands when the call
/// is read, so a transaction another process committed is what the next call reads".
#[test]
fn mcp_on_a_read_only_store_reads_another_process_commit() {
    let initialize = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2025-11-25", "capabilities": {},
                   "clientInfo": {"name": "adversary", "version": "0"}}});
    let initialized = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    let head = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "head", "arguments": {}}});
    let mut stale = Vec::new();
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let guard = world.read_only();
        let mut mcp = Served::start(&world, &["mcp"]);
        mcp.ask(&initialize);
        mcp.send(&initialized);
        let first = mcp.ask(&head);
        drop(guard);
        world.commit_elsewhere();
        let second = mcp.ask(&head);
        mcp.end();
        // What a fresh `ekr mcp`, started after the commit, answers to the same call.
        let fresh = world.feed(&["mcp"], &format!("{initialize}\n{initialized}\n{head}\n"));
        let fresh: Value = serde_json::from_str(
            String::from_utf8_lossy(&fresh.stdout)
                .lines()
                .nth(1)
                .unwrap(),
        )
        .unwrap();
        assert_ne!(fresh, first, "{backend}: the commit moved the head");
        if second != fresh {
            stale.push(format!(
                "{backend}: answered {} after the commit, a fresh server {}",
                second["result"]["structuredContent"], fresh["result"]["structuredContent"]
            ));
        }
    }
    assert!(stale.is_empty(), "the long-lived server on a read-only store still answers the head it opened at: {stale:#?}");
}

/// A SQLite database made read-only by its owner (`chmod 444 state.db`, the directory left as it
/// is) opens read-only for a read verb, which must write nothing at the store's path
/// (docs/cli.md, Configuration: "so it writes nothing at the store's path").
#[test]
fn a_read_verb_on_a_read_only_database_in_a_writable_directory_creates_nothing_beside_it() {
    let world = World::seeded("sqlite");
    let before = world.names();
    assert!(
        !before.iter().any(|name| name.ends_with("-wal")),
        "precondition: no WAL after the seed: {before:?}"
    );
    chmod(&world.store(), 0o444);
    let head = world.run(&["head"]);
    let after = world.names();
    chmod(&world.store(), 0o644);
    assert_eq!(head.status.code(), Some(0), "{}", stderr(&head));
    assert_eq!(
        after, before,
        "a read verb created files beside the database"
    );
}

/// The consequence for the owner: once the database is writable again, a verb that writes must
/// work. The `-wal` a reader left behind carries the database's mode at the time it was created
/// (0444 here; another user's ownership in the multi-user case), so the writer is refused
/// `store-read-only` on its own store.
#[test]
fn the_owner_writes_again_after_a_read_on_its_read_only_database() {
    let world = World::seeded("sqlite");
    chmod(&world.store(), 0o444);
    let head = world.run(&["head"]);
    chmod(&world.store(), 0o644);
    assert_eq!(head.status.code(), Some(0), "{}", stderr(&head));
    let propose = fixture("propose-alice.yaml").display().to_string();
    let proposed = world.run(&["propose", &propose]);
    assert_eq!(
        proposed.status.code(),
        Some(0),
        "the owner's propose after a reader: {} (left beside the database: {:?})",
        stderr(&proposed),
        world.names()
    );
}

/// `ekr view` serves until it is interrupted, so interruption is its only way to end. On a
/// read-only file store it reads a private copy in `TMPDIR`; that copy must not outlive it.
#[test]
fn ekr_view_on_a_read_only_file_store_removes_its_copy_when_terminated() {
    let world = World::seeded("file");
    let tmpdir = world.tmpdir();
    let guard = world.read_only();
    let mut child = world
        .command(&["view", "--port", "0"])
        .env("TMPDIR", &tmpdir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut url = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut url)
        .unwrap();
    assert!(url.contains("http://127.0.0.1:"), "{url:?}");
    // Binding precedes lazy store admission. Prove it serves the store before
    // checking that the private copy is held and then cleaned up on termination.
    let authority = url
        .trim()
        .strip_prefix("http://")
        .unwrap()
        .trim_end_matches('/');
    let mut request = std::net::TcpStream::connect(authority).unwrap();
    request
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    write!(
        request,
        "GET /readyz HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    request.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 "), "{response}");
    // TMPDIR is respected: the copy is there while the viewer serves.
    assert_eq!(copies(&tmpdir).len(), 1, "one copy while it serves");
    let killed = std::process::Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(killed.success());
    child.wait().unwrap();
    drop(guard);
    assert_eq!(
        copies(&tmpdir),
        Vec::<PathBuf>::new(),
        "the terminated viewer left its copy of the store"
    );
}

/// The exits that do run destructors: a one-shot read verb and a session ending at EOF leave no
/// copy in `TMPDIR`.
#[test]
fn a_one_shot_read_and_a_session_at_eof_leave_no_copy_in_tmpdir() {
    let world = World::seeded("file");
    let tmpdir = world.tmpdir();
    let guard = world.read_only();
    let head = world
        .command(&["head"])
        .env("TMPDIR", &tmpdir)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(head.status.code(), Some(0), "{}", stderr(&head));
    assert_eq!(
        copies(&tmpdir),
        Vec::<PathBuf>::new(),
        "after a one-shot read"
    );
    let mut session = world
        .command(&["session"])
        .env("TMPDIR", &tmpdir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = session.stdin.take().unwrap();
    writeln!(input, "{}", json!({"argv": ["head"]})).unwrap();
    let mut output = BufReader::new(session.stdout.take().unwrap());
    let mut first = String::new();
    output.read_line(&mut first).unwrap();
    assert_eq!(
        copies(&tmpdir).len(),
        1,
        "one copy while the session holds it"
    );
    drop(input);
    assert!(session.wait().unwrap().success());
    drop(guard);
    assert_eq!(copies(&tmpdir), Vec::<PathBuf>::new(), "after EOF");
}

/// A session that opens its store only when a write request arrives, on a store made read-only
/// meanwhile, refuses `validate` and `commit` by name as it refuses `propose`
/// (`read_only_store.rs` covers only `propose`).
#[test]
fn a_late_session_refuses_validate_and_commit_by_name() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let propose = fixture("propose-alice.yaml").display().to_string();
        let proposed = world.run(&["propose", &propose]);
        assert_eq!(proposed.status.code(), Some(0), "{}", stderr(&proposed));
        let guard = world.read_only();
        for argv in [
            json!(["validate", TRANSACTION, "--against", "0"]),
            json!(["commit", TRANSACTION]),
        ] {
            let output = world.feed(&["session"], &format!("{}\n", json!({"argv": argv})));
            let answer: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(answer["exit"], 2, "{backend} {argv}: {answer}");
            assert!(
                answer["stderr"]
                    .as_str()
                    .is_some_and(|stderr| stderr.starts_with("ekr: store-read-only: ")),
                "{backend} {argv}: {answer}"
            );
        }
        drop(guard);
    }
}

/// A file store whose `events.jsonl` alone is read-only (the directory writable) is a store the
/// writer cannot write: the read verbs answer, and a verb that writes is refused by name.
#[test]
fn a_file_store_with_only_its_events_read_only_is_read_only() {
    let world = World::seeded("file");
    let writable = world.run(&["head"]);
    let events = world.store().join("events.jsonl");
    chmod(&events, 0o444);
    let head = world.run(&["head"]);
    let propose = fixture("propose-alice.yaml").display().to_string();
    let proposed = world.run(&["propose", &propose]);
    chmod(&events, 0o644);
    assert_eq!(head.status.code(), Some(0), "{}", stderr(&head));
    assert_eq!(head.stdout, writable.stdout);
    assert_eq!(proposed.status.code(), Some(2), "{}", stderr(&proposed));
    assert!(
        stderr(&proposed).starts_with("ekr: store-read-only: "),
        "{}",
        stderr(&proposed)
    );
}

/// A file store whose `blobs` directory alone is read-only: `ekr propose`, which stores the
/// document's objects there, is refused by name rather than failing on the write.
#[test]
fn a_file_store_with_only_its_blobs_read_only_refuses_a_write_by_name() {
    let world = World::seeded("file");
    let blobs = world.store().join("blobs");
    assert!(blobs.is_dir(), "precondition: the seed stored objects");
    chmod(&blobs, 0o555);
    let propose = fixture("propose-alice.yaml").display().to_string();
    let proposed = world.run(&["propose", &propose]);
    let head = world.run(&["head"]);
    chmod(&blobs, 0o755);
    assert_eq!(proposed.status.code(), Some(2), "{}", stderr(&proposed));
    assert!(
        stderr(&proposed).starts_with("ekr: store-read-only: "),
        "{}",
        stderr(&proposed)
    );
    assert_eq!(head.status.code(), Some(0), "{}", stderr(&head));
}
