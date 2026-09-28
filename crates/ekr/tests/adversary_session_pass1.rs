//! Adversary pass 1 on `story:ekr-session`: a live session interleaved with one-shot processes on
//! the same store, and the memory one request line costs.
//!
//! Every case drives the built binary. A session is held open across requests: each request is
//! written and its answer read before the next is written, as a host that calls `ekr session` in a
//! loop does, so another process can act between two requests of the same session.

use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Output, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const NODE_A: &str = "00000000-0000-4000-8000-00000000a901";
const TX_A: &str = "00000000-0000-4000-8000-00000000a902";
const NODE_B: &str = "00000000-0000-4000-8000-00000000b901";
const TX_B: &str = "00000000-0000-4000-8000-00000000b902";

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

/// A `CreateNode` of an Organization named `name`, in transaction `tx`.
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

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn seeded(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        world.file("host.json", &text(&["example", "ekr.cli-host/1"]));
        world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        world.file("a.yaml", &create(TX_A, NODE_A, "Alphaco"));
        world.file("b.yaml", &create(TX_B, NODE_B, "Betaco"));
        world.file(
            "ref-b.yaml",
            &format!("type_id: {ORGANIZATION}\naliases: [Betaco]\n"),
        );
        let seeded = world.run(&["seed", "seed.yaml"]);
        assert_eq!(seeded.status.code(), Some(0), "{backend}: seed");
        world
    }

    fn copy(&self) -> Self {
        let copy = Self {
            directory: tempfile::tempdir().unwrap(),
            backend: self.backend,
        };
        copy_tree(self.directory.path(), copy.directory.path());
        copy
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
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
            .args(["--backend", self.backend])
            .args(verb);
        command
    }

    fn run(&self, verb: &[&str]) -> Output {
        self.command(verb).stdin(Stdio::null()).output().unwrap()
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn live(&self) -> Live {
        let mut child = self
            .command(&["session"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, answers) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if send.send(line.unwrap()).is_err() {
                    return;
                }
            }
        });
        Live {
            child,
            stdin: Some(stdin),
            answers,
        }
    }
}

/// A session held open: one request written, its one answer read, then the next.
struct Live {
    child: Child,
    stdin: Option<ChildStdin>,
    answers: Receiver<String>,
}

impl Live {
    fn answer(&self, what: &str) -> Value {
        let line = self
            .answers
            .recv_timeout(Duration::from_secs(120))
            .unwrap_or_else(|error| panic!("{what}: no answer line: {error}"));
        serde_json::from_str(&line).unwrap_or_else(|error| panic!("{what}: {error}: {line}"))
    }

    fn ask(&mut self, argv: &[&str], stdin: Option<&str>) -> Value {
        let line = match stdin {
            Some(stdin) => json!({"argv": argv, "stdin": stdin}),
            None => json!({"argv": argv}),
        };
        let input = self.stdin.as_mut().unwrap();
        input.write_all(format!("{line}\n").as_bytes()).unwrap();
        input.flush().unwrap();
        self.answer(&format!("{argv:?}"))
    }

    /// Peak resident set of the session process so far, in bytes (Linux `VmHWM`).
    fn peak(&self) -> u64 {
        let status = std::fs::read_to_string(format!("/proc/{}/status", self.child.id())).unwrap();
        let line = status
            .lines()
            .find(|line| line.starts_with("VmHWM:"))
            .expect("VmHWM in /proc/<pid>/status");
        let kib: u64 = line.split_whitespace().nth(1).unwrap().parse().unwrap();
        kib * 1024
    }

    fn close(mut self) -> Output {
        drop(self.stdin.take());
        self.child.wait_with_output().unwrap()
    }
}

/// docs/cli.md § `ekr session`: "Every verb runs against the store as it stands when the request
/// is read, so a transaction a request commits is what the next `head`, `snapshot` or `resolve`
/// reads, and so is one another process committed."
#[test]
fn adversary_session_sees_a_commit_another_process_made_while_it_was_open() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let mut session = world.live();
        let before = session.ask(&["head"], None);
        assert_eq!(before["stdout"]["revision"], 0, "{backend}: {before}");
        let unknown = session.ask(&["resolve", "ref-b.yaml"], None);
        assert_eq!(
            unknown["stdout"]["kind"], "ProposeNew",
            "{backend}: {unknown}"
        );

        // Another process commits B while the session is open.
        world.ok(&["propose", "b.yaml"]);
        world.ok(&["validate", TX_B]);
        let committed = world.ok(&["commit", TX_B]);
        assert_eq!(committed["kind"], "Committed", "{backend}: {committed}");

        let head = session.ask(&["head"], None);
        assert_eq!(
            head["stdout"]["revision"], 1,
            "{backend}: the session's head after another process's commit: {head}"
        );
        let resolved = session.ask(&["resolve", "ref-b.yaml"], None);
        assert_eq!(
            resolved["stdout"]["kind"], "Resolved",
            "{backend}: {resolved}"
        );
        assert_eq!(
            resolved["stdout"]["node_id"], NODE_B,
            "{backend}: {resolved}"
        );
        let listed = session.ask(&["transactions", "--state", "Committed"], None);
        assert_eq!(
            listed["stdout"].as_array().map(Vec::len),
            Some(1),
            "{backend}: {listed}"
        );
        let closed = session.close();
        assert_eq!(closed.status.code(), Some(0), "{backend}");
    }
}

/// A transaction the session validated against revision 0, committed by the session after another
/// process moved the head to 1, gets what the same sequence of one-shot verbs gets.
#[test]
fn adversary_session_commit_on_a_stale_validation_answers_as_the_one_shot_commit() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let twin = world.copy();

        // One-shot on the twin: A validated at 0, B committed, then A committed.
        twin.ok(&["propose", "a.yaml"]);
        twin.ok(&["validate", TX_A]);
        twin.ok(&["propose", "b.yaml"]);
        twin.ok(&["validate", TX_B]);
        twin.ok(&["commit", TX_B]);
        let one_shot = twin.run(&["commit", TX_A]);

        // The same with A in a session and B from another process.
        let mut session = world.live();
        let proposed = session.ask(&["propose", "a.yaml"], None);
        assert_eq!(proposed["exit"], 0, "{backend}: {proposed}");
        let validated = session.ask(&["validate", TX_A], None);
        assert_eq!(
            validated["stdout"]["kind"], "Validated",
            "{backend}: {validated}"
        );
        world.ok(&["propose", "b.yaml"]);
        world.ok(&["validate", TX_B]);
        world.ok(&["commit", TX_B]);
        let answer = session.ask(&["commit", TX_A], None);

        assert_eq!(
            answer["exit"].as_i64(),
            one_shot.status.code().map(i64::from),
            "{backend}: exit; session {answer}; one-shot {}",
            String::from_utf8_lossy(&one_shot.stderr)
        );
        assert_eq!(
            answer["stderr"].as_str().unwrap(),
            String::from_utf8_lossy(&one_shot.stderr),
            "{backend}: stderr"
        );
        if one_shot.status.code() == Some(0) {
            let document: Value = serde_json::from_slice(&one_shot.stdout).unwrap();
            assert_eq!(
                answer["stdout"]["kind"], document["kind"],
                "{backend}: kind; session {answer}; one-shot {document}"
            );
        }
        // And both stores now agree on the head revision.
        let head = session.ask(&["head"], None);
        assert_eq!(
            head["stdout"]["revision"],
            twin.ok(&["head"])["revision"],
            "{backend}: {head}"
        );
        session.close();
    }
}

/// The one-shot `ekr propose -` reads at most the document limit (8 MiB, `DOCUMENT_V2_LIMITS`) from
/// its standard input and refuses the rest unread. The same bytes sent as one session request are
/// buffered whole — the line by `read_until`, then again as the decoded `stdin` string — before
/// anything looks at their size, so one request line costs the session twice its length in memory
/// with no bound. A 128 MiB line is streamed here in 1 MiB chunks; the session's peak resident set
/// must not grow by the line's length to answer it.
#[test]
fn adversary_session_one_request_line_does_not_cost_its_whole_length_in_memory() {
    const MIB: usize = 1024 * 1024;
    const LINE: usize = 128 * MIB;
    let world = World::seeded("file");
    let mut session = world.live();
    let warm = session.ask(&["head"], None);
    assert_eq!(warm["exit"], 0, "{warm}");
    let before = session.peak();

    let mut input = session.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        input
            .write_all(br#"{"argv":["propose","-"],"stdin":""#)
            .unwrap();
        let chunk = vec![b'a'; MIB];
        let mut written = 0;
        while written < LINE {
            input.write_all(&chunk).unwrap();
            written += MIB;
        }
        input.write_all(b"\"}\n").unwrap();
        input.flush().unwrap();
        input
    });
    let answer = session.answer("a 128 MiB propose request");
    session.stdin = Some(writer.join().unwrap());
    let grown = session.peak().saturating_sub(before);

    assert_eq!(
        answer["exit"], 2,
        "an over-limit document is refused: {answer}"
    );
    let next = session.ask(&["head"], None);
    assert_eq!(next["exit"], 0, "the session still serves: {next}");
    session.close();
    assert!(
        grown < LINE as u64,
        "the session's peak resident set grew by {} MiB to answer one {} MiB request line; the \
         one-shot verb reads at most 8 MiB of the same input",
        grown / MIB as u64,
        LINE / MIB
    );
}
