//! Adversary pass on wave correct-07 unit G: `task:divergence-is-a-typed-store-error`.
//!
//! The unit replaced "the failure's text is the provider's divergence message" with "after any
//! failure, the held runtime's head is refused as diverged" as the session's reopen guard. The two
//! differ for a request that failed for its own reason, without reading the store, while the store
//! happens to be diverged. Acceptance: "the session, MCP and view hosts reopen on it, with every
//! printed text unchanged".
//!
//! Every case drives the built binary: `ekr session` held open across requests, with the store's
//! files replaced inside its directory (its device and inode kept) between two requests.

use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Output, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use serde_json::{json, Value};

const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const NODE_A: &str = "00000000-0000-4000-8000-00000000c901";
const TX_A: &str = "00000000-0000-4000-8000-00000000c902";
const NODE_B: &str = "00000000-0000-4000-8000-00000000d901";
const TX_B: &str = "00000000-0000-4000-8000-00000000d902";

const WITHIN: Duration = Duration::from_secs(120);

fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn create(tx: &str, node: &str, name: &str) -> String {
    format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {tx}\n  proposer: \
         {OPERATOR}\n  operations:\n  - !CreateNode\n    id: {node}\n    root_id: {ROOT}\n    \
         type_id: {ORGANIZATION}\n    canonical_name: {name}\n    properties: {{}}\n    \
         aliases: [{name}]\n  evidence: []\n"
    )
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

/// `rsync -a --delete from/ store/`: the root directory, and so its device and inode, stay.
fn replace_the_files_inside(store: &Path, from: &Path) {
    for entry in std::fs::read_dir(store).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path).unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
        }
    }
    copy_tree(from, store);
}

struct World {
    directory: tempfile::TempDir,
}

impl World {
    fn seeded() -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        world.file("host.json", &text(&["example", "ekr.cli-host/1"]));
        world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        world.file("a.yaml", &create(TX_A, NODE_A, "Alphaco"));
        world.file("b.yaml", &create(TX_B, NODE_B, "Betaco"));
        world.ok(&["seed", "seed.yaml"]);
        world
    }

    fn store(&self) -> PathBuf {
        self.directory.path().join("store")
    }

    fn file(&self, name: &str, contents: &str) {
        std::fs::write(self.directory.path().join(name), contents).unwrap();
    }

    fn command(&self, verb: &[&str]) -> std::process::Command {
        let mut command = ekr();
        command
            .current_dir(self.directory.path())
            .arg("--host")
            .arg(self.directory.path().join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", "file"])
            .args(verb);
        command
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.command(verb).stdin(Stdio::null()).output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{verb:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn commit(&self, file: &str, tx: &str) {
        self.ok(&["propose", file]);
        assert_eq!(self.ok(&["validate", tx])["kind"], "Validated");
        assert_eq!(self.ok(&["commit", tx])["kind"], "Committed");
    }
}

struct Lines {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
}

impl Lines {
    fn spawn(mut command: std::process::Command) -> Self {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if send.send(line).is_err() {
                    return;
                }
            }
        });
        Self {
            child,
            stdin: Some(stdin),
            lines,
        }
    }

    fn ask(&mut self, argv: &[&str]) -> Value {
        let input = self.stdin.as_mut().unwrap();
        input
            .write_all(format!("{}\n", json!({ "argv": argv })).as_bytes())
            .unwrap();
        input.flush().unwrap();
        let line = self
            .lines
            .recv_timeout(WITHIN)
            .unwrap_or_else(|error| panic!("{argv:?}: no answer: {error}"));
        serde_json::from_str(&line).unwrap_or_else(|error| panic!("{error}: {line}"))
    }

    fn ok(&mut self, argv: &[&str]) -> Value {
        let answer = self.ask(argv);
        assert_eq!(answer["exit"], 0, "{argv:?}: {answer}");
        answer["stdout"].clone()
    }

    fn close(mut self) -> Output {
        drop(self.stdin.take());
        self.child.wait_with_output().unwrap()
    }
}

/// A request that fails for its own reason before it reads the store — `propose` of a document
/// that is not there fails in `input::open`, before the held runtime is touched — answers the
/// same while the session holds an open proposal, whether or not the store at the path has been
/// replaced under the same inode since. At the base (`30703729`) the reopen guard was the
/// failure's own text, which here is a document read failure, so the original answer was printed.
/// The unit asks the held runtime's head after *any* failure; that head is refused as diverged,
/// so the session goes to `follow(.., true)`, which refuses with `store-replaced-proposals-open`
/// and the document failure is never printed.
#[test]
#[ignore = "adversary x7-g: a non-store failure in a diverged session with an open proposal prints store-replaced-proposals-open instead of its own failure"]
fn adversary_x7_g_a_failure_that_reads_no_store_prints_its_own_text_in_a_diverged_session() {
    let world = World::seeded();
    let mut session = Lines::spawn(world.command(&["session"]));
    assert_eq!(session.ok(&["head"])["revision"], 0);
    let proposed = session.ok(&["propose", "a.yaml"]);
    assert_eq!(proposed["transaction_id"], TX_A, "{proposed}");

    let before = session.ask(&["propose", "missing.yaml"]);
    assert_ne!(
        before["exit"], 0,
        "precondition: the document is missing: {before}"
    );

    let replacement = World::seeded();
    replacement.commit("b.yaml", TX_B);
    replace_the_files_inside(&world.store(), &replacement.store());

    let after = session.ask(&["propose", "missing.yaml"]);
    session.close();
    assert_eq!(
        after, before,
        "the same request, failing for the same reason before any store read, printed \
         something else once the store had diverged"
    );
}

/// GUARD (green). Without an open proposal the same request, rerun after the reopen, prints the
/// same failure: the reopen the unit adds on a non-store failure is invisible there.
#[test]
fn adversary_x7_g_without_a_proposal_a_non_store_failure_is_unchanged_by_the_reopen() {
    let world = World::seeded();
    let mut session = Lines::spawn(world.command(&["session"]));
    assert_eq!(session.ok(&["head"])["revision"], 0);
    let before = session.ask(&["propose", "missing.yaml"]);
    assert_ne!(
        before["exit"], 0,
        "precondition: the document is missing: {before}"
    );

    let replacement = World::seeded();
    replacement.commit("b.yaml", TX_B);
    replace_the_files_inside(&world.store(), &replacement.store());

    let after = session.ask(&["propose", "missing.yaml"]);
    assert_eq!(after, before);
    assert_eq!(
        session.ok(&["head"])["revision"],
        1,
        "the store now at the path"
    );
    session.close();
}
