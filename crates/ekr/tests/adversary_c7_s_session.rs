//! Adversary pass on wave correct-07 unit S: `task:sqlite-store-replaced-in-place`.
//!
//! Acceptance: "No live handle answers from a SQLite file that is no longer at its path", and the
//! decision: "the session, MCP and view hosts reopen on it". The unit's session case asks `head`
//! and `overview`. These ask every other store-reading verb of `ekr session`, each as the first
//! request after a SQLite database was copied over the session's store file (`cp`, device and
//! inode kept), and compare the answer with the one `ekr` gives for the database copied there.

use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use serde_json::{json, Value};

const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const NODE_A: &str = "00000000-0000-4000-8000-00000000c7a1";
const TX_A: &str = "00000000-0000-4000-8000-00000000c7a2";
const NODE_B: &str = "00000000-0000-4000-8000-00000000c7b1";
const TX_B: &str = "00000000-0000-4000-8000-00000000c7b2";
const NODE_C: &str = "00000000-0000-4000-8000-00000000c7c1";
const TX_C: &str = "00000000-0000-4000-8000-00000000c7c2";

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

fn identity(path: &Path) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt as _;
    let metadata = std::fs::metadata(path).unwrap();
    (metadata.dev(), metadata.ino())
}

/// A SQLite store seeded and holding one committed transaction, `tx` creating `node`.
struct World {
    directory: tempfile::TempDir,
}

impl World {
    fn committed(tx: &str, node: &str, name: &str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        world.file("host.json", &text(&["example", "ekr.cli-host/1"]));
        world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        world.file("tx.yaml", &create(tx, node, name));
        world.file("c.yaml", &create(TX_C, NODE_C, "Gammaco"));
        world.ok(&["seed", "seed.yaml"]);
        world.ok(&["propose", "tx.yaml"]);
        assert_eq!(world.ok(&["validate", tx])["kind"], "Validated");
        assert_eq!(world.ok(&["commit", tx])["kind"], "Committed");
        world
    }

    fn store(&self) -> PathBuf {
        self.directory.path().join("store.db")
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
            .args(["--backend", "sqlite"])
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
            .stderr(Stdio::null())
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

    fn close(mut self) {
        drop(self.stdin.take());
        let _ = self.child.wait();
    }
}

/// GUARD (green). Every store-reading verb of `ekr session`, asked first after a SQLite database was copied over
/// the session's store file, answers what `ekr` answers for that database.
#[test]
fn adversary_c7_s_every_session_read_verb_answers_from_a_sqlite_database_copied_over_its_file() {
    let verbs: [&[&str]; 7] = [
        &["head"],
        &["snapshot"],
        &["transactions"],
        &["rejections"],
        &["ontology"],
        &["quality"],
        &["ocel"],
    ];
    let mut wrong = Vec::new();
    for verb in verbs {
        let world = World::committed(TX_A, NODE_A, "Alphaco");
        let mut session = Lines::spawn(world.command(&["session"]));
        let before = session.ask(verb);
        assert_eq!(before["exit"], 0, "{verb:?}: {before}");

        let replacement = World::committed(TX_B, NODE_B, "Betaco");
        let expected = replacement.ok(verb);
        let held = identity(&world.store());
        std::fs::copy(replacement.store(), world.store()).unwrap();
        assert_eq!(
            identity(&world.store()),
            held,
            "precondition: copied in place"
        );

        let after = session.ask(verb);
        session.close();
        if after["exit"] != 0 || after["stdout"] != expected {
            wrong.push(format!(
                "{verb:?}: the session answered {after}; `ekr` answers {expected} for the \
                 database at the path"
            ));
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// GUARD (green). A write asked first after the copy lands in the database now at the path.
#[test]
fn adversary_c7_s_a_session_proposal_after_a_sqlite_database_was_copied_over_its_file_lands_there()
{
    let world = World::committed(TX_A, NODE_A, "Alphaco");
    let mut session = Lines::spawn(world.command(&["session"]));
    assert_eq!(session.ask(&["head"])["exit"], 0);
    let replacement = World::committed(TX_B, NODE_B, "Betaco");
    std::fs::copy(replacement.store(), world.store()).unwrap();

    let proposed = session.ask(&["propose", "c.yaml"]);
    session.close();
    assert_eq!(proposed["exit"], 0, "{proposed}");
    let listed = world.ok(&["transactions"]).to_string();
    assert!(
        listed.contains(TX_C) && listed.contains(TX_B) && !listed.contains(TX_A),
        "the database at the path lists {listed}"
    );
}
