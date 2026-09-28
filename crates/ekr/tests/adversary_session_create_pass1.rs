//! Adversary pass 1 on `story:session-starts-before-a-store` (wave read-02, unit S).
//!
//! `ekr session` now starts on a path that holds no store, and `ekr session --create` serves
//! `seed` and then holds the store it created. Every case drives the built binary on both
//! providers, with sessions held open across requests so that another process can act between
//! two requests of the same session, and compares with what the one-shot verbs do on the same
//! store state:
//!
//! * a refused seed in a `--create` session leaves on disk exactly what the refused one-shot seed
//!   leaves, on an absent path, an empty directory and an empty file;
//! * two `--create` sessions seeding two documents into one absent path at once: one seed wins,
//!   and both sessions then serve the winner's store;
//! * a session started without `--create`, before another process seeds, serves that store;
//! * a `--create` session that seeded sees another process's commit and answers a stale commit as
//!   the one-shot commit does;
//! * a `--create` session holds the store its seed created (an open descriptor into the store);
//! * a `--create` session started on a provisioned store whose seed never landed seeds it and
//!   then serves it;
//! * `--full-replay` with `--create`.

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Output, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use ekr::host::CliHostConfigurationV1;
use ekr_kernel::Runtime;
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

/// The example seed with `evidence_payloads: {}`: the kernel refuses it
/// (`seed-evidence-payload-missing`) before anything is created.
fn seed_without_payloads(seed: &str) -> String {
    let lines: Vec<&str> = seed.lines().collect();
    let start = lines
        .iter()
        .position(|line| *line == "evidence_payloads:")
        .expect("the example seed has evidence_payloads");
    let mut document = lines[..start].join("\n");
    document.push_str("\nevidence_payloads: {}\n");
    document
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

/// Every path below `root`, relative, with `d` for a directory or the file's length.
fn listing(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, at: &Path, into: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                into.insert(name, "d".to_owned());
                walk(root, &path, into);
            } else {
                into.insert(name, meta.len().to_string());
            }
        }
    }
    let mut into = BTreeMap::new();
    walk(root, root, &mut into);
    into
}

/// What the path looks like before anything runs.
#[derive(Clone, Copy, Debug)]
enum Before {
    Absent,
    EmptyDirectory,
    EmptyFile,
}

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn absent(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        let seed = text(&["example", "ekr-seed/2"]);
        world.file("host.json", &text(&["example", "ekr.cli-host/1"]));
        world.file("seed.yaml", &seed);
        world.file(
            "other-seed.yaml",
            &seed.replace("canonical_name: Bob", "canonical_name: Robert"),
        );
        world.file("refused-seed.yaml", &seed_without_payloads(&seed));
        world.file("a.yaml", &create(TX_A, NODE_A, "Alphaco"));
        world.file("b.yaml", &create(TX_B, NODE_B, "Betaco"));
        world.file(
            "ref-b.yaml",
            &format!("type_id: {ORGANIZATION}\naliases: [Betaco]\n"),
        );
        assert!(!world.store().exists());
        world
    }

    fn with(backend: &'static str, before: Before) -> Self {
        let world = Self::absent(backend);
        match before {
            Before::Absent => {}
            Before::EmptyDirectory => std::fs::create_dir(world.store()).unwrap(),
            Before::EmptyFile => std::fs::write(world.store(), b"").unwrap(),
        }
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

    fn host(&self) -> CliHostConfigurationV1 {
        CliHostConfigurationV1::from_json(
            &std::fs::read(self.directory.path().join("host.json")).unwrap(),
        )
        .unwrap()
    }

    fn command(&self, global: &[&str], verb: &[&str]) -> std::process::Command {
        let mut command = ekr();
        command
            .current_dir(self.directory.path())
            .arg("--host")
            .arg(self.directory.path().join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(global)
            .args(verb);
        command
    }

    fn run(&self, verb: &[&str]) -> Output {
        self.command(&[], verb)
            .stdin(Stdio::null())
            .output()
            .unwrap()
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

    /// Everything below the world's directory, the store included.
    fn listing(&self) -> BTreeMap<String, String> {
        listing(self.directory.path())
    }

    fn live(&self, verb: &[&str]) -> Live {
        self.live_with(&[], verb)
    }

    fn live_with(&self, global: &[&str], verb: &[&str]) -> Live {
        let mut child = self
            .command(global, verb)
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

    fn send(&mut self, argv: &[&str], stdin: Option<&str>) {
        let line = match stdin {
            Some(stdin) => json!({"argv": argv, "stdin": stdin}),
            None => json!({"argv": argv}),
        };
        let input = self.stdin.as_mut().unwrap();
        input.write_all(format!("{line}\n").as_bytes()).unwrap();
        input.flush().unwrap();
    }

    fn ask(&mut self, argv: &[&str]) -> Value {
        self.send(argv, None);
        self.answer(&format!("{argv:?}"))
    }

    /// The paths this process holds a descriptor on, below `under` (Linux `/proc/<pid>/fd`).
    fn descriptors_under(&self, under: &Path) -> Vec<PathBuf> {
        let under = std::fs::canonicalize(under.parent().unwrap())
            .unwrap()
            .join(under.file_name().unwrap());
        let mut held = Vec::new();
        let Ok(entries) = std::fs::read_dir(format!("/proc/{}/fd", self.child.id())) else {
            return held;
        };
        for entry in entries.flatten() {
            if let Ok(target) = std::fs::read_link(entry.path()) {
                if target.starts_with(&under) {
                    held.push(target);
                }
            }
        }
        held
    }

    fn close(mut self) -> Output {
        drop(self.stdin.take());
        self.child.wait_with_output().unwrap()
    }
}

/// `stderr` of `output`, with `from`'s directory written as `to`'s: a fault names the store path.
fn rebased(output: &Output, from: &World, to: &World) -> String {
    String::from_utf8_lossy(&output.stderr).replace(
        from.directory.path().to_str().unwrap(),
        to.directory.path().to_str().unwrap(),
    )
}

/// docs/cli.md § `ekr session`: a session on a path holding no store "creates nothing there by
/// starting", and a seed in a `--create` session goes "through one-shot's own path". A refused
/// seed, a store verb before and after it, and a refused seed in a session without `--create`,
/// leave on disk exactly what the same one-shot verbs leave, on each path a store verb takes for
/// "no store": nothing, an empty directory, an empty file. Every answer is the one-shot verb's.
#[test]
fn adversary_session_create_refused_seed_leaves_what_the_one_shot_seed_leaves() {
    let requests: [&[&str]; 5] = [
        &["head"],
        &["seed", "refused-seed.yaml"],
        &["seed", "seed.yaml", "--evidence", "missing.txt"],
        &["propose", "a.yaml"],
        &["head"],
    ];
    for backend in BACKENDS {
        for before in [Before::Absent, Before::EmptyDirectory, Before::EmptyFile] {
            let what = format!("{backend} {before:?}");
            let (in_session, one_shot) =
                (World::with(backend, before), World::with(backend, before));
            let start = one_shot.listing();

            let mut session = in_session.live(&["session", "--create"]);
            let mut answers = Vec::new();
            for argv in requests {
                answers.push(session.ask(argv));
            }
            let closed = session.close();
            assert_eq!(closed.status.code(), Some(0), "{what}: session exit");

            for (argv, answer) in requests.iter().zip(&answers) {
                let output = one_shot.run(argv);
                assert_eq!(
                    answer["exit"].as_i64(),
                    output.status.code().map(i64::from),
                    "{what} {argv:?}: session {answer}; one-shot {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(
                    answer["stderr"].as_str().unwrap(),
                    rebased(&output, &one_shot, &in_session),
                    "{what} {argv:?}: stderr"
                );
            }
            assert_ne!(
                answers[1]["exit"], 0,
                "{what}: the refused seed: {}",
                answers[1]
            );
            assert_eq!(
                in_session.listing(),
                one_shot.listing(),
                "{what}: the session left something the one-shot verbs did not"
            );
            assert_eq!(
                one_shot.listing(),
                start,
                "{what}: a refused one-shot seed created something"
            );

            // Without --create: every store verb and the refused seed leave the path as it was.
            let plain = World::with(backend, before);
            let start = plain.listing();
            let mut session = plain.live(&["session"]);
            for argv in [
                &["head"][..],
                &["seed", "seed.yaml"],
                &["snapshot"],
                &["resolve", "ref-b.yaml"],
                &["propose", "a.yaml"],
                &["validate", TX_A],
                &["commit", TX_A],
                &["transactions"],
                &["ontology"],
                &["explain", TX_A],
            ] {
                let answer = session.ask(argv);
                assert_ne!(answer["exit"], 0, "{what} {argv:?}: {answer}");
            }
            assert_eq!(session.close().status.code(), Some(0), "{what}");
            assert_eq!(plain.listing(), start, "{what}: a session without --create");
        }
    }
}

/// Two `--create` sessions started on one absent path, each sending a seed of a different
/// document at the same moment: one seed publishes, the other is refused, and then both sessions
/// serve the one store there is — each `head` is the one-shot `head`, a commit through the losing
/// session is seen by the winning one, and the one-shot verbs see it too.
#[test]
fn adversary_session_create_two_sessions_seeding_two_documents_at_once_serve_one_store() {
    for backend in BACKENDS {
        for round in 0..4 {
            let what = format!("{backend} round {round}");
            let world = World::absent(backend);
            let mut first = world.live(&["session", "--create"]);
            let mut second = world.live(&["session", "--create"]);
            first.send(&["seed", "seed.yaml"], None);
            second.send(&["seed", "other-seed.yaml"], None);
            let (one, two) = (first.answer("first seed"), second.answer("second seed"));
            let won: Vec<bool> = [&one, &two].iter().map(|a| a["exit"] == 0).collect();
            assert_eq!(
                won.iter().filter(|w| **w).count(),
                1,
                "{what}: exactly one seed publishes: {one} / {two}"
            );
            let (mut winner, mut loser, lost) = if won[0] {
                (first, second, two)
            } else {
                (second, first, one)
            };
            println!("{what}: the losing seed answered {lost}");

            let head = world.ok(&["head"]);
            for (name, session) in [("winner", &mut winner), ("loser", &mut loser)] {
                let answer = session.ask(&["head"]);
                assert_eq!(answer["exit"], 0, "{what} {name}: {answer}");
                assert_eq!(answer["stdout"], head, "{what} {name}: head");
            }
            for argv in [
                &["propose", "b.yaml"][..],
                &["validate", TX_B],
                &["commit", TX_B],
            ] {
                let answer = loser.ask(argv);
                assert_eq!(answer["exit"], 0, "{what} loser {argv:?}: {answer}");
            }
            let resolved = winner.ask(&["resolve", "ref-b.yaml"]);
            assert_eq!(
                resolved["stdout"]["node_id"], NODE_B,
                "{what}: the winner sees the loser's commit: {resolved}"
            );
            assert_eq!(world.ok(&["head"])["revision"], 1, "{what}");
            assert_eq!(winner.close().status.code(), Some(0), "{what}");
            assert_eq!(loser.close().status.code(), Some(0), "{what}");
        }
    }
}

/// A session started without `--create` before any store exists, on a path another process then
/// seeds and commits to: every later answer is the one-shot verb's on the same store, and the
/// session's own commit is seen by the one-shot verbs.
#[test]
fn adversary_session_create_plain_session_serves_the_store_another_process_seeded() {
    for backend in BACKENDS {
        let world = World::absent(backend);
        let mut session = world.live(&["session"]);
        let before = session.ask(&["head"]);
        assert_eq!(before["exit"], 1, "{backend}: {before}");

        world.ok(&["seed", "seed.yaml"]);
        let head = session.ask(&["head"]);
        assert_eq!(
            head["stdout"],
            world.ok(&["head"]),
            "{backend}: after the seed"
        );
        let snapshot = session.ask(&["snapshot"]);
        assert_eq!(snapshot["stdout"], world.ok(&["snapshot"]), "{backend}");

        world.ok(&["propose", "b.yaml"]);
        world.ok(&["validate", TX_B]);
        world.ok(&["commit", TX_B]);
        let head = session.ask(&["head"]);
        assert_eq!(head["stdout"], world.ok(&["head"]), "{backend}: after B");

        for argv in [
            &["propose", "a.yaml"][..],
            &["validate", TX_A],
            &["commit", TX_A],
        ] {
            let answer = session.ask(argv);
            assert_eq!(answer["exit"], 0, "{backend} {argv:?}: {answer}");
        }
        assert_eq!(world.ok(&["head"])["revision"], 2, "{backend}");
        let seed = session.ask(&["seed", "seed.yaml"]);
        assert_eq!(seed["exit"], 2, "{backend}: seed without --create: {seed}");
        assert_eq!(session.close().status.code(), Some(0), "{backend}");
    }
}

/// A `--create` session that seeded holds the store; another process then commits B while the
/// session's A is validated at the seed revision. The session sees B, and its commit of A answers
/// what the one-shot commit answers on a byte-identical copy of the store at that moment.
#[test]
fn adversary_session_create_held_store_sees_another_commit_and_answers_a_stale_commit_as_one_shot()
{
    for backend in BACKENDS {
        let world = World::absent(backend);
        let mut session = world.live(&["session", "--create"]);
        let seeded = session.ask(&["seed", "seed.yaml"]);
        assert_eq!(seeded["exit"], 0, "{backend}: {seeded}");
        for argv in [&["propose", "a.yaml"][..], &["validate", TX_A]] {
            let answer = session.ask(argv);
            assert_eq!(answer["exit"], 0, "{backend} {argv:?}: {answer}");
        }
        world.ok(&["propose", "b.yaml"]);
        world.ok(&["validate", TX_B]);
        world.ok(&["commit", TX_B]);

        let head = session.ask(&["head"]);
        assert_eq!(
            head["stdout"],
            world.ok(&["head"]),
            "{backend}: head after B"
        );
        let resolved = session.ask(&["resolve", "ref-b.yaml"]);
        assert_eq!(
            resolved["stdout"]["node_id"], NODE_B,
            "{backend}: {resolved}"
        );

        let twin = world.copy();
        let one_shot = twin.run(&["commit", TX_A]);
        let answer = session.ask(&["commit", TX_A]);
        assert_eq!(
            answer["exit"].as_i64(),
            one_shot.status.code().map(i64::from),
            "{backend}: session {answer}; one-shot {}",
            String::from_utf8_lossy(&one_shot.stderr)
        );
        if one_shot.status.code() == Some(0) {
            let document: Value = serde_json::from_slice(&one_shot.stdout).unwrap();
            assert_eq!(answer["stdout"]["kind"], document["kind"], "{backend}");
        }
        assert_eq!(
            session.ask(&["head"])["stdout"],
            world.ok(&["head"]),
            "{backend}: head after the session's commit"
        );
        assert_eq!(session.close().status.code(), Some(0), "{backend}");
    }
}

/// docs/cli.md § `ekr session`: "The seed that creates the store leaves the session holding it,
/// opened once as a session opens an existing store when it starts". A session holding a store
/// keeps a descriptor on it between requests; the one a plain session opens on an existing store
/// at start is the reference. The seed's answer is identical whether the session then holds the
/// store or reopens it per request, so no answer can tell the two apart: only the descriptor can.
#[test]
fn adversary_session_create_holds_the_store_its_seed_created() {
    for backend in BACKENDS {
        let reference = World::absent(backend);
        reference.ok(&["seed", "seed.yaml"]);
        let mut plain = reference.live(&["session"]);
        plain.ask(&["head"]);
        let at_start = plain.descriptors_under(&reference.store());
        plain.close();
        if at_start.is_empty() {
            println!("{backend}: a session holds no descriptor on an existing store; no probe");
            continue;
        }

        let world = World::absent(backend);
        let mut session = world.live(&["session", "--create"]);
        let before = session.descriptors_under(&world.store());
        let seeded = session.ask(&["seed", "seed.yaml"]);
        assert_eq!(seeded["exit"], 0, "{backend}: {seeded}");
        let after = session.descriptors_under(&world.store());

        // Calibration: a session that does not hold its store (started before another process
        // seeded, without --create) keeps no descriptor between requests.
        let unheld = World::absent(backend);
        let mut opener = unheld.live(&["session"]);
        opener.ask(&["head"]);
        unheld.ok(&["seed", "seed.yaml"]);
        opener.ask(&["head"]);
        let per_request = opener.descriptors_under(&unheld.store());
        opener.close();
        println!(
            "{backend}: held at start {at_start:?}; --create before seed {before:?}, after \
             {after:?}; per-request session {per_request:?}"
        );
        assert!(
            before.is_empty(),
            "{backend}: nothing to hold before the seed"
        );
        assert!(
            !after.is_empty(),
            "{backend}: the --create session holds no descriptor on the store it seeded"
        );
        assert_eq!(session.close().status.code(), Some(0), "{backend}");
    }
}

/// A store whose provider was provisioned but whose seed never landed (a seed killed between the
/// provider's create and its publication): `open_existing` accepts it, so a `--create` session
/// started there holds it from the start. Its seed then answers as the one-shot seed does on a
/// copy, and every later answer is the one-shot verb's on the seeded store.
#[test]
fn adversary_session_create_seeds_a_provisioned_store_it_already_holds() {
    for backend in BACKENDS {
        let world = World::absent(backend);
        let host = world.host();
        drop(
            match backend {
                "file" => Runtime::file(&world.store(), &host.tenant, host.context, host.authority),
                _ => Runtime::sqlite(&world.store(), &host.tenant, host.context, host.authority),
            }
            .unwrap(),
        );
        let twin = world.copy();
        let one_shot_head = twin.run(&["head"]);
        let one_shot_seed = twin.run(&["seed", "seed.yaml"]);

        let mut session = world.live(&["session", "--create"]);
        let head = session.ask(&["head"]);
        assert_eq!(
            head["exit"].as_i64(),
            one_shot_head.status.code().map(i64::from),
            "{backend}: head before the seed: {head}"
        );
        let seeded = session.ask(&["seed", "seed.yaml"]);
        assert_eq!(
            seeded["exit"].as_i64(),
            one_shot_seed.status.code().map(i64::from),
            "{backend}: seed: {seeded}; one-shot {}",
            String::from_utf8_lossy(&one_shot_seed.stderr)
        );
        if seeded["exit"] != 0 {
            assert_eq!(session.close().status.code(), Some(0), "{backend}");
            continue;
        }
        let head = session.ask(&["head"]);
        assert_eq!(head["exit"], 0, "{backend}: head after the seed: {head}");
        assert_eq!(
            head["stdout"],
            world.ok(&["head"]),
            "{backend}: head after the seed"
        );
        for argv in [
            &["propose", "a.yaml"][..],
            &["validate", TX_A],
            &["commit", TX_A],
        ] {
            let answer = session.ask(argv);
            assert_eq!(answer["exit"], 0, "{backend} {argv:?}: {answer}");
        }
        assert_eq!(
            session.ask(&["snapshot"])["stdout"],
            world.ok(&["snapshot"]),
            "{backend}: snapshot"
        );
        assert_eq!(session.close().status.code(), Some(0), "{backend}");
    }
}

/// `ekr --full-replay session --create`: the seed, a write sequence and the reads after it answer
/// as the one-shot verbs do under --full-replay; a request that sets --full-replay is still
/// `session-option-refused`, and a seed request that names another --store creates nothing there.
#[test]
fn adversary_session_create_with_full_replay_and_refused_options() {
    for backend in BACKENDS {
        let world = World::absent(backend);
        let elsewhere = world.directory.path().join("elsewhere");
        let elsewhere_text = elsewhere.to_str().unwrap().to_owned();
        let mut session = world.live_with(&["--full-replay"], &["session", "--create"]);
        let refused = session.ask(&["--store", &elsewhere_text, "seed", "seed.yaml"]);
        assert_eq!(refused["exit"], 2, "{backend}: {refused}");
        assert!(
            refused["stderr"]
                .as_str()
                .unwrap()
                .starts_with("ekr: session-option-refused: "),
            "{backend}: {refused}"
        );
        assert!(!elsewhere.exists(), "{backend}: created the other --store");
        assert!(
            !world.store().exists(),
            "{backend}: a refused request created the store"
        );
        let refused = session.ask(&["--full-replay", "head"]);
        assert_eq!(refused["exit"], 2, "{backend}: {refused}");

        let seeded = session.ask(&["seed", "seed.yaml"]);
        assert_eq!(seeded["exit"], 0, "{backend}: {seeded}");
        for argv in [
            &["propose", "a.yaml"][..],
            &["validate", TX_A],
            &["commit", TX_A],
        ] {
            let answer = session.ask(argv);
            assert_eq!(answer["exit"], 0, "{backend} {argv:?}: {answer}");
        }
        let full = world
            .command(&["--full-replay"], &["head"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let full: Value = serde_json::from_slice(&full.stdout).unwrap();
        assert_eq!(session.ask(&["head"])["stdout"], full, "{backend}: head");
        assert_eq!(session.close().status.code(), Some(0), "{backend}");
    }
}
