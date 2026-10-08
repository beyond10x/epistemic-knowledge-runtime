//! `task:read-verbs-open-a-read-only-store`: on a store whose files and directories this process
//! may read but not write, the read verbs answer on both providers with the bytes they print on a
//! writable one, and a write verb is refused by name, `store-read-only`, exit 2, before it opens
//! anything. The store's bytes are the same afterwards.
//!
//! The store is made read-only with file permissions: every file `0444` and every directory `0555`,
//! the store's own directory and — for SQLite, whose journal files live beside the database — the
//! directory holding it. [`ReadOnly`] restores `0755`/`0644` when it drops, so a failed assertion
//! still leaves a tree the temporary directory can remove.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const ALICE: &str = "00000000-0000-4000-8000-000000000301";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
    .join("tests/fixtures/retraction")
    .join(name)
}

fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

/// A seeded store in a directory of its own, and a source file for `ekr code-names` beside it.
struct World {
    /// Holds the store's directory; never made read-only itself, so it can always be removed.
    _outer: tempfile::TempDir,
    /// The directory the store lives in: made read-only with the store.
    directory: PathBuf,
    backend: &'static str,
}

impl World {
    fn seeded(backend: &'static str) -> Self {
        let world = Self::unseeded(backend);
        world.seed();
        world
    }

    fn unseeded(backend: &'static str) -> Self {
        let outer = tempfile::tempdir().unwrap();
        let directory = outer.path().join("world");
        std::fs::create_dir(&directory).unwrap();
        let world = Self {
            _outer: outer,
            directory,
            backend,
        };
        std::fs::write(
            world.directory.join("reader.ts"),
            "const kind = \"Person\";\nconst who = 'Alice';\n",
        )
        .unwrap();
        world
    }

    fn seed(&self) {
        let seeded = self.run(&["seed", &fixture("seed.yaml").display().to_string()]);
        assert_eq!(
            seeded.status.code(),
            Some(0),
            "{}: {}",
            self.backend,
            String::from_utf8_lossy(&seeded.stderr)
        );
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.join("store"),
            _ => self.directory.join("state.db"),
        }
    }

    fn source(&self) -> String {
        self.directory.join("reader.ts").display().to_string()
    }

    fn command(&self, verb: &[&str]) -> std::process::Command {
        let mut command = ekr();
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

    /// Runs `verb` with `stdin` as its standard input.
    fn feed(&self, verb: &[&str], stdin: &str) -> Output {
        use std::io::Write as _;
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

    /// Every file under the world's directory and its bytes.
    fn bytes(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut all = BTreeMap::new();
        for path in walk(&self.directory) {
            if path.is_file() {
                all.insert(path.clone(), std::fs::read(&path).unwrap());
            }
        }
        all
    }

    /// Makes the world's directory and everything under it read-only until the guard drops.
    fn read_only(&self) -> ReadOnly {
        set_modes(&self.directory, 0o555, 0o444);
        ReadOnly(self.directory.clone())
    }
}

/// Restores write permission on drop.
struct ReadOnly(PathBuf);

impl Drop for ReadOnly {
    fn drop(&mut self) {
        set_modes(&self.0, 0o755, 0o644);
    }
}

fn set_modes(at: &Path, directory: u32, file: u32) {
    // The directory first when opening it up, so its entries can be reached; last when closing.
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(0o755)).unwrap();
    for path in walk(at) {
        let mode = if path.is_dir() { directory } else { file };
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
    }
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(directory)).unwrap();
}

/// Every path under `at`, children before their directory.
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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The read verbs of the acceptance, each as it is run.
fn read_verbs(world: &World) -> Vec<Vec<String>> {
    [
        vec!["head".to_owned()],
        vec!["ontology".to_owned()],
        vec!["snapshot".to_owned()],
        vec!["code-names".to_owned(), world.source()],
    ]
    .into()
}

/// The guard itself: a write under it fails, so a green case below is not a tree that stayed
/// writable.
#[test]
fn the_read_only_tree_refuses_a_write() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let guard = world.read_only();
        assert!(std::fs::write(world.directory.join("probe"), b"x").is_err());
        assert!(std::fs::OpenOptions::new()
            .append(true)
            .open(world.directory.join("reader.ts"))
            .is_err());
        drop(guard);
        std::fs::write(world.directory.join("probe"), b"x").unwrap();
    }
}

/// Acceptance 1: `ekr head`, `ekr ontology`, `ekr snapshot` and `ekr code-names` answer on a
/// read-only store on both providers, with the same bytes as on a writable one, and leave every
/// byte under the store's directory as it was.
#[test]
fn read_verbs_answer_on_a_read_only_store_with_the_writable_bytes_on_both_providers() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let mut writable = Vec::new();
        for verb in read_verbs(&world) {
            let verb: Vec<&str> = verb.iter().map(String::as_str).collect();
            let output = world.run(&verb);
            assert_eq!(
                output.status.code(),
                Some(0),
                "{backend} {verb:?}: {}",
                stderr(&output)
            );
            writable.push(output.stdout);
        }
        let before = world.bytes();
        let guard = world.read_only();
        for (verb, writable) in read_verbs(&world).into_iter().zip(&writable) {
            let verb: Vec<&str> = verb.iter().map(String::as_str).collect();
            let output = world.run(&verb);
            assert_eq!(
                output.status.code(),
                Some(0),
                "{backend} {verb:?} on a read-only store: {}",
                stderr(&output)
            );
            assert!(
                output.stdout == *writable,
                "{backend} {verb:?}: read-only answer\n{}\nwritable answer\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(writable)
            );
            assert_eq!(stderr(&output), "", "{backend} {verb:?}");
        }
        drop(guard);
        assert!(
            world.bytes() == before,
            "{backend}: a read verb changed the store's bytes"
        );
    }
}

/// Acceptance 2: `ekr propose` on a read-only store is refused by name, exit 2, and prints no
/// result; so are the other verbs that write, `validate`, `commit`, `seed` and `apply-extraction`.
/// Nothing changes.
#[test]
fn write_verbs_on_a_read_only_store_are_refused_by_name_with_exit_2() {
    let transaction = "00000000-0000-4000-8000-000000000601";
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let before = world.bytes();
        let guard = world.read_only();
        let propose = fixture("propose-alice.yaml").display().to_string();
        let seed = fixture("seed.yaml").display().to_string();
        let verbs: [&[&str]; 5] = [
            &["propose", &propose],
            &["validate", transaction, "--against", "0"],
            &["commit", transaction],
            &["seed", &seed],
            &["apply-extraction", &propose],
        ];
        for verb in verbs {
            let output = world.run(verb);
            let stderr = stderr(&output);
            assert_eq!(
                output.status.code(),
                Some(2),
                "{backend} {verb:?}: {stderr}"
            );
            assert!(
                stderr.starts_with("ekr: store-read-only: "),
                "{backend} {verb:?}: {stderr:?}"
            );
            assert!(
                stderr.contains(&world.store().display().to_string()),
                "{backend} {verb:?}: the refusal names the store: {stderr:?}"
            );
            assert!(output.stdout.is_empty(), "{backend} {verb:?}");
        }
        drop(guard);
        assert!(
            world.bytes() == before,
            "{backend}: a refused write changed the store"
        );
    }
}

/// The page's verb table says which verbs write the store and which only read it. On a read-only
/// store every verb it says `writes` is refused `store-read-only`, and every one it says `reads`
/// exits, prints and reports exactly as on a writable store — a refusal such as
/// `ekr.kernel.AssertionNotFound` included. `ekr view`, which serves until interrupted, opens
/// through the same reader as `ekr mcp` (`session.rs`, `Held`); `session`, `mcp` and `migrate`
/// have cases or columns of their own. The long-running HTTP commands have real read-only
/// process cases in `hosted_http.rs`.
#[test]
fn every_verb_the_page_says_writes_is_refused_and_every_one_that_reads_answers_as_writable() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let page = std::fs::read_to_string(Path::new(&manifest).join("../../docs/cli.md")).unwrap();
    let table = page
        .split("\n## Verbs\n")
        .nth(1)
        .and_then(|rest| rest.split("\n## ").next())
        .expect("docs/cli.md has a ## Verbs section");
    let mut writes = Vec::new();
    let mut reads = Vec::new();
    for row in table.lines().filter(|line| line.starts_with("| `ekr ")) {
        let cells: Vec<&str> = row.split(" | ").collect();
        let verb = cells[0]
            .trim_start_matches("| `ekr ")
            .trim_end_matches('`')
            .to_owned();
        match cells[1] {
            "writes" => writes.push(verb),
            "reads" => reads.push(verb),
            _ => {}
        }
    }
    assert_eq!(
        writes,
        ["seed", "propose", "validate", "commit", "apply-extraction"],
        "{table}"
    );
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let source = world.source();
        let argv = |verb: &str| -> Option<Vec<String>> {
            let args: &[&str] = match verb {
                "explain" => &["explain", ALICE],
                "resolve" => &["resolve", "-"],
                "code-names" => &["code-names", &source],
                "sample" => &["sample", "--seed", "1", "--size", "3"],
                "view" | "mcp-http" => return None,
                verb => &[verb],
            };
            Some(args.iter().map(|arg| (*arg).to_owned()).collect())
        };
        let reference = "type_id: 00000000-0000-4000-8000-000000000201\naliases: [Alice]\n";
        let mut writable = Vec::new();
        for verb in &reads {
            let Some(args) = argv(verb) else { continue };
            let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
            let output = world.feed(&borrowed, reference);
            writable.push((args, output));
        }
        let guard = world.read_only();
        for (args, writable) in &writable {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let output = world.feed(&args, reference);
            assert_eq!(
                (output.status.code(), &output.stdout, stderr(&output)),
                (writable.status.code(), &writable.stdout, stderr(writable)),
                "{backend} {args:?}"
            );
        }
        drop(guard);
        assert!(
            writable.len() >= 10,
            "{backend}: {} read verbs ran",
            writable.len()
        );
    }
}

/// `ekr migrate` only reads `--store`: a read-only store migrates to a writable `--to`, whose head
/// is the store's.
#[test]
fn migrate_reads_a_read_only_store_into_a_writable_one() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let head = world.run(&["head"]);
        assert_eq!(head.status.code(), Some(0), "{backend}: {}", stderr(&head));
        let to = world._outer.path().join(match backend {
            "file" => "migrated",
            _ => "migrated.db",
        });
        let to = to.display().to_string();
        let guard = world.read_only();
        let migrated = world.run(&["migrate", "--to", &to]);
        drop(guard);
        assert_eq!(
            migrated.status.code(),
            Some(0),
            "{backend}: {}",
            stderr(&migrated)
        );
        let moved = ekr()
            .arg("--host")
            .arg(fixture("host.json"))
            .args(["--store", &to, "--backend", backend, "head"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(
            moved.status.code(),
            Some(0),
            "{backend}: {}",
            stderr(&moved)
        );
        let (head, moved): (Value, Value) = (
            serde_json::from_slice(&head.stdout).unwrap(),
            serde_json::from_slice(&moved.stdout).unwrap(),
        );
        assert_eq!(moved["revision"], head["revision"], "{backend}");
    }
}

/// A session started before its store exists holds none. When another process has seeded the
/// store and it is read-only by the time a write request arrives, the request opens it as the
/// session's reads do — read-only — and the write the store then refuses is still answered
/// `store-read-only`, exit 2, not a fault.
#[test]
fn a_session_that_opens_a_read_only_store_late_refuses_its_write_by_name() {
    use std::io::{BufRead as _, Write as _};
    for backend in BACKENDS {
        let world = World::unseeded(backend);
        let mut child = world
            .command(&["session"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
        writeln!(input, "{}", json!({"argv": ["head"]})).unwrap();
        let mut first = String::new();
        output.read_line(&mut first).unwrap();
        let first: Value = serde_json::from_str(&first).unwrap();
        assert!(
            first["stderr"]
                .as_str()
                .is_some_and(|stderr| stderr.starts_with("ekr: store-not-found: ")),
            "{backend}: the session started without a store: {first}"
        );
        world.seed();
        let guard = world.read_only();
        let propose = fixture("propose-alice.yaml").display().to_string();
        writeln!(input, "{}", json!({"argv": ["propose", propose]})).unwrap();
        writeln!(input, "{}", json!({"argv": ["head"]})).unwrap();
        drop(input);
        let answers: Vec<Value> = output
            .lines()
            .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
            .collect();
        let status = child.wait().unwrap();
        drop(guard);
        assert!(status.success(), "{backend}");
        assert_eq!(answers.len(), 2, "{backend}: {answers:?}");
        assert_eq!(answers[0]["exit"], 2, "{backend}: {}", answers[0]);
        assert!(
            answers[0]["stderr"]
                .as_str()
                .is_some_and(|stderr| stderr.starts_with("ekr: store-read-only: ")),
            "{backend}: {}",
            answers[0]
        );
        assert_eq!(answers[1]["exit"], 0, "{backend}: {}", answers[1]);
    }
}

/// `ekr session` on a read-only store starts, answers a read as the one-shot verb does, and
/// answers a write request `store-read-only`, exit 2, and keeps serving.
#[test]
fn a_session_on_a_read_only_store_reads_and_refuses_a_write_by_name() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let head = world.run(&["head"]);
        assert_eq!(head.status.code(), Some(0), "{backend}: {}", stderr(&head));
        let head: Value = serde_json::from_slice(&head.stdout).unwrap();
        let propose = fixture("propose-alice.yaml").display().to_string();
        let lines = [
            json!({"argv": ["head"]}),
            json!({"argv": ["propose", propose]}),
            json!({"argv": ["resolve", "-"],
                   "stdin": "type_id: 00000000-0000-4000-8000-000000000201\naliases: [Alice]\n"}),
        ]
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<String>();
        let guard = world.read_only();
        let output = world.feed(&["session"], &lines);
        drop(guard);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{backend}: {}",
            stderr(&output)
        );
        let answers: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(answers.len(), 3, "{backend}: {answers:?}");
        assert_eq!(answers[0]["exit"], 0, "{backend}: {}", answers[0]);
        assert_eq!(answers[0]["stdout"], head, "{backend}");
        assert_eq!(answers[1]["exit"], 2, "{backend}: {}", answers[1]);
        assert!(
            answers[1]["stderr"]
                .as_str()
                .is_some_and(|stderr| stderr.starts_with("ekr: store-read-only: ")),
            "{backend}: {}",
            answers[1]
        );
        assert_eq!(answers[2]["exit"], 0, "{backend}: {}", answers[2]);
    }
}

/// `ekr mcp` on a read-only store serves its read tools with the bytes it serves on a writable one.
#[test]
fn mcp_on_a_read_only_store_answers_as_on_a_writable_one() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let lines = [
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                   "params": {"protocolVersion": "2025-11-25", "capabilities": {},
                              "clientInfo": {"name": "read-only", "version": "0"}}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                   "params": {"name": "head", "arguments": {}}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                   "params": {"name": "describe_node", "arguments": {"node": ALICE}}}),
        ]
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<String>();
        let writable = world.feed(&["mcp"], &lines);
        assert_eq!(
            writable.status.code(),
            Some(0),
            "{backend}: {}",
            stderr(&writable)
        );
        let guard = world.read_only();
        let output = world.feed(&["mcp"], &lines);
        drop(guard);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{backend}: {}",
            stderr(&output)
        );
        let answers: Vec<Value> = String::from_utf8(output.stdout.clone())
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(answers.len(), 3, "{backend}: {answers:?}");
        assert_eq!(
            answers[1]["result"]["isError"], false,
            "{backend}: {}",
            answers[1]
        );
        assert_eq!(
            answers[2]["result"]["isError"], false,
            "{backend}: {}",
            answers[2]
        );
        assert!(
            output.stdout == writable.stdout,
            "{backend}: read-only\n{}\nwritable\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&writable.stdout)
        );
    }
}
