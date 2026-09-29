//! `story:ekr-session`: `ekr session` serves the existing verbs over one opened store.
//!
//! A session reads one JSON request per line on standard input — `{"argv": [...]}`, the argv of a
//! one-shot verb, with an optional `"stdin"` text that `-` reads — and answers each with one JSON
//! line on standard output: `{"exit": <status>, "stdout": <document or null>, "stderr": <text>}`,
//! what the one-shot verb exits with and prints. Every case drives the built binary on both
//! providers under the example host and seed:
//!
//! * each verb's answer is the one-shot verb's output on the same store state — byte for byte
//!   once `stdout` is printed as the one-shot verb prints it, except the fields a write samples
//!   afresh on every run (its clock and minted event ids), which are measured as the fields two
//!   one-shot runs on byte-identical copies of the store disagree on;
//! * a line that is not a request is refused by name and the next line is still served, and the
//!   page's session refusal table lists exactly the refusals drawn here;
//! * a transaction proposed, validated and committed in the session is seen by the next `head`
//!   and `resolve` of the same session;
//! * `story:session-starts-before-a-store`: a session on a path holding no store serves `mint`,
//!   `hash` and `schema` and answers a store verb `store-not-found`; `ekr session --create` seeds
//!   the store and then answers as the one-shot sequence does, in one process.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const SEEDED_ASSERTION: &str = "00000000-0000-4000-8000-000000000510";
const NODE: &str = "00000000-0000-4000-8000-000000000901";
const TRANSACTION: &str = "00000000-0000-4000-8000-000000000902";
const UNKNOWN_TRANSACTION: &str = "00000000-0000-4000-8000-000000000999";

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

/// Exit 0 and stdout as text.
fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// A `CreateNode` of an Organization known as `Globex`: what the example typed reference names.
fn create_globex() -> String {
    format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {TRANSACTION}\n  proposer: \
         {OPERATOR}\n  operations:\n  - !CreateNode\n    id: {NODE}\n    root_id: {ROOT}\n    \
         type_id: {ORGANIZATION}\n    canonical_name: Globex\n    properties: {{}}\n    \
         aliases: [Globex]\n  evidence: []\n"
    )
}

/// Every file at or below `from`, copied byte for byte to the same place below `to`.
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

/// The example seed with its `evidence_payloads` replaced by `{}`, and the bytes of each payload
/// it held, in document order: what `ekr seed - --evidence <file>...` completes again.
fn example_without_payloads() -> (String, Vec<Vec<u8>>) {
    let example = text(&["example", "ekr-seed/2"]);
    let lines: Vec<&str> = example.lines().collect();
    let start = lines
        .iter()
        .position(|line| *line == "evidence_payloads:")
        .unwrap();
    let payloads: Vec<Vec<u8>> = lines[start + 1..]
        .iter()
        .filter_map(|line| line.trim().split_once(": ["))
        .map(|(_, list)| {
            list.trim_end_matches(']')
                .split(',')
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.trim().parse().unwrap())
                .collect()
        })
        .collect();
    assert_eq!(payloads.len(), 2, "the example seed carries two payloads");
    let mut document = lines[..start].join("\n");
    document.push_str("\nevidence_payloads: {}\n");
    (document, payloads)
}

/// One provider in its own directory under the example host, with the files the requests name:
/// `seed.yaml`, `create.yaml`, `reference.yaml` and `payload.txt`; `stripped.yaml`, the example
/// seed without its payloads, and `evidence-0.txt` and `evidence-1.txt`, those payloads;
/// `other-seed.yaml`, a different valid seed. Every `ekr` process a world starts is counted.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
    spawned: std::cell::Cell<usize>,
}

impl World {
    /// The files, and no store.
    fn absent(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
            spawned: std::cell::Cell::new(0),
        };
        let seed = text(&["example", "ekr-seed/2"]);
        world.file("host.json", &text(&["example", "ekr.cli-host/1"]));
        world.file(
            "other-seed.yaml",
            &seed.replace("canonical_name: Bob", "canonical_name: Robert"),
        );
        world.file("seed.yaml", &seed);
        world.file("reference.yaml", &text(&["example", "typed-reference"]));
        world.file("create.yaml", &create_globex());
        world.file("payload.txt", "Alice is CEO of Acme.\n");
        let (stripped, payloads) = example_without_payloads();
        world.file("stripped.yaml", &stripped);
        for (at, payload) in payloads.iter().enumerate() {
            std::fs::write(
                world.directory.path().join(format!("evidence-{at}.txt")),
                payload,
            )
            .unwrap();
        }
        assert!(!world.store().exists());
        world
    }

    fn seeded(backend: &'static str) -> Self {
        let world = Self::absent(backend);
        let seeded = world.run(&["seed", "seed.yaml"]);
        assert_eq!(
            seeded.status.code(),
            Some(0),
            "{backend}: seed: {}",
            String::from_utf8_lossy(&seeded.stderr)
        );
        world
    }

    /// A byte-identical copy of this world, store included, in a directory of its own.
    fn copy(&self) -> Self {
        let copy = Self {
            directory: tempfile::tempdir().unwrap(),
            backend: self.backend,
            spawned: std::cell::Cell::new(0),
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

    /// The store configuration as flags, then `verb`, run from the world's directory.
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

    /// One one-shot verb configured by flags, with `stdin` as its standard input.
    fn one_shot(&self, verb: &[&str], stdin: &str) -> Output {
        self.spawn(self.command(verb), stdin)
    }

    /// One one-shot verb configured by `EKR_*`, so that its argv is exactly `verb`: clap's usage
    /// line names the global options an argv gives, and a session request gives none.
    fn one_shot_by_environment(&self, verb: &[&str], stdin: &str) -> Output {
        let mut command = ekr();
        command
            .current_dir(self.directory.path())
            .env("EKR_HOST", self.directory.path().join("host.json"))
            .env("EKR_STORE", self.store())
            .env("EKR_BACKEND", self.backend)
            .args(verb);
        self.spawn(command, stdin)
    }

    fn spawn(&self, mut command: std::process::Command, stdin: &str) -> Output {
        self.spawned.set(self.spawned.get() + 1);
        let mut child = command
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

    fn run(&self, verb: &[&str]) -> Output {
        self.one_shot(verb, "")
    }

    /// One `ekr session` over `lines`, configured by flags or by `EKR_*`, and its answers: exit 0,
    /// nothing on stderr, and one JSON line per request line.
    fn session(&self, lines: &[String], by_environment: bool) -> Vec<Value> {
        self.session_as(&["session"], lines, by_environment)
    }

    /// [`World::session`], started as `verb`: `["session"]` or `["session", "--create"]`.
    fn session_as(&self, verb: &[&str], lines: &[String], by_environment: bool) -> Vec<Value> {
        let mut command = if by_environment {
            let mut command = ekr();
            command
                .current_dir(self.directory.path())
                .env("EKR_HOST", self.directory.path().join("host.json"))
                .env("EKR_STORE", self.store())
                .env("EKR_BACKEND", self.backend)
                .args(verb);
            command
        } else {
            self.command(verb)
        };
        self.spawned.set(self.spawned.get() + 1);
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let written: String = lines.iter().map(|line| format!("{line}\n")).collect();
        // Written from another thread: a session answers while it reads, and a full stdout pipe
        // would otherwise stop it reading.
        let writer = std::thread::spawn(move || input.write_all(written.as_bytes()));
        let output = child.wait_with_output().unwrap();
        writer.join().unwrap().unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}: session: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty(), "a session writes answers only");
        let answers: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            answers.len(),
            lines.len(),
            "{}: one answer per request line: {answers:?}",
            self.backend
        );
        answers
    }
}

/// One request line: `argv`, and `stdin` when `-` should read something.
fn request(argv: &[&str], stdin: Option<&str>) -> String {
    match stdin {
        Some(stdin) => json!({"argv": argv, "stdin": stdin}).to_string(),
        None => json!({"argv": argv}).to_string(),
    }
}

/// `stdout` as the one-shot verb prints a document: pretty JSON and a newline; nothing for null.
fn printed(stdout: &Value) -> Vec<u8> {
    if stdout.is_null() {
        return Vec::new();
    }
    let mut text = serde_json::to_string_pretty(stdout).unwrap();
    text.push('\n');
    text.into_bytes()
}

/// The answer's exit status, stdout and stderr are the one-shot run's, byte for byte.
fn assert_answers_as(answer: &Value, one_shot: &Output, what: &str) {
    assert_eq!(
        answer["exit"].as_i64(),
        one_shot.status.code().map(i64::from),
        "{what}: exit; {answer}"
    );
    assert_eq!(
        answer["stderr"].as_str().unwrap(),
        String::from_utf8_lossy(&one_shot.stderr),
        "{what}: stderr"
    );
    assert_eq!(
        String::from_utf8(printed(&answer["stdout"])).unwrap(),
        String::from_utf8_lossy(&one_shot.stdout),
        "{what}: stdout"
    );
}

/// Every leaf of `value` — a scalar, an empty object or an empty list — by its JSON pointer.
fn leaves(value: &Value, at: &str, into: &mut BTreeMap<String, Value>) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (key, item) in map {
                let key = key.replace('~', "~0").replace('/', "~1");
                leaves(item, &format!("{at}/{key}"), into);
            }
        }
        Value::Array(items) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                leaves(item, &format!("{at}/{index}"), into);
            }
        }
        leaf => {
            into.insert(at.to_owned(), leaf.clone());
        }
    }
}

fn leaf_map(value: &Value) -> BTreeMap<String, Value> {
    let mut map = BTreeMap::new();
    leaves(value, "", &mut map);
    map
}

/// The pointers at which two documents disagree, or which only one of them holds.
fn disagreeing(left: &Value, right: &Value) -> BTreeSet<String> {
    let (left, right) = (leaf_map(left), leaf_map(right));
    left.keys()
        .chain(right.keys())
        .filter(|at| left.get(*at) != right.get(*at))
        .cloned()
        .collect()
}

/// `document` with the leaf at each pointer of `volatile` taken from `from`.
fn with_volatile_from(document: &Value, from: &Value, volatile: &BTreeSet<String>) -> Value {
    let mut document = document.clone();
    for at in volatile {
        let (Some(slot), Some(value)) = (document.pointer_mut(at), from.pointer(at)) else {
            panic!("{at} is not in both documents");
        };
        assert_eq!(
            std::mem::discriminant(slot),
            std::mem::discriminant(value),
            "{at}: a volatile field changes type"
        );
        *slot = value.clone();
    }
    document
}

fn one_shot_document(output: &Output, what: &str) -> Value {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{what}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn each_verb_answers_as_the_one_shot_verb_on_the_same_store_state() {
    let nobody = "type_id: 00000000-0000-4000-8000-000000000201\naliases: [nobody]\n";
    // Reads and refusals: one answer on one store state, whoever asks.
    let reads: Vec<(Vec<&str>, Option<&str>)> = vec![
        (vec!["head"], None),
        (vec!["snapshot"], None),
        (
            vec!["snapshot", "--at", "0", "--valid-at", "2020-01-01"],
            None,
        ),
        (vec!["ontology"], None),
        (vec!["ontology", "--at", "0"], None),
        (vec!["resolve", "reference.yaml"], None),
        (vec!["resolve", "-"], Some(nobody)),
        (vec!["transactions"], None),
        (vec!["transactions", "--state", "Committed"], None),
        (vec!["explain", SEEDED_ASSERTION], None),
        (vec!["hash", "payload.txt"], None),
        (vec!["hash", "-"], Some("Bob is CEO of Acme.\n")),
        (vec!["schema", "typed-reference"], None),
        // Named refusals, faults and usage errors, answered as the one-shot verb exits.
        (vec!["commit", UNKNOWN_TRANSACTION], None),
        (vec!["resolve", "reference.yaml", "--at", "9"], None),
        (vec!["ontology", "--at", "9"], None),
        (vec!["validate", "not-a-transaction-id"], None),
        (vec!["snapshot", "--at", "zero"], None),
        (vec!["propose", "missing.yaml"], None),
        (vec!["resolve", "-"], Some("aliases: [nobody]\n")),
        // Arguments clap refuses outside the named session refusals: its own usage message.
        (vec!["--sto", "x", "head"], None),
        (vec!["head", "-V"], None),
    ];
    let writes: [Vec<&str>; 3] = [
        vec!["propose", "create.yaml"],
        vec!["validate", TRANSACTION],
        vec!["commit", TRANSACTION],
    ];
    let after: [Vec<&str>; 6] = [
        vec!["head"],
        vec!["snapshot"],
        vec!["resolve", "reference.yaml"],
        vec!["transactions"],
        vec!["ontology"],
        vec!["validate", TRANSACTION],
    ];
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let (before, twin) = (world.copy(), world.copy());

        let mut lines: Vec<String> = reads
            .iter()
            .map(|(argv, stdin)| request(argv, *stdin))
            .collect();
        lines.push(request(&["mint", "node"], None));
        let first_write = lines.len();
        lines.extend(writes.iter().map(|argv| request(argv, None)));
        let first_after = lines.len();
        lines.extend(after.iter().map(|argv| request(argv, None)));
        let answers = world.session(&lines, false);

        // Every read and refusal on the seeded state, which `before` still holds byte for byte,
        // against the one-shot verb given the same argv.
        for ((argv, stdin), answer) in reads.iter().zip(&answers) {
            let one_shot = before.one_shot_by_environment(argv, stdin.unwrap_or(""));
            assert_answers_as(answer, &one_shot, &format!("{backend} {argv:?}"));
        }
        let refused = reads
            .iter()
            .zip(&answers)
            .filter(|(_, answer)| answer["exit"] != 0)
            .count();
        assert_eq!(refused, 9, "{backend}: the refusal lines are refused");

        // `mint`: the shape, not the id.
        let minted = &answers[first_write - 1];
        let one_shot = one_shot_document(&before.run(&["mint", "node"]), "mint");
        assert_eq!(minted["exit"], 0, "{backend} mint: {minted}");
        assert_eq!(minted["stderr"], "");
        let keys = |value: &Value| -> BTreeSet<String> {
            value.as_object().unwrap().keys().cloned().collect()
        };
        assert_eq!(keys(&minted["stdout"]), keys(&one_shot), "{backend} mint");
        assert_eq!(minted["stdout"]["kind"], one_shot["kind"]);
        assert_ne!(minted["stdout"]["id"], one_shot["id"], "a fresh id");

        // Writes: the one-shot verbs on two copies of the seeded state disagree only where a
        // write samples its clock or mints an event id. The session agrees with them everywhere
        // else, and prints byte for byte what they print once those fields are theirs.
        for (argv, answer) in writes.iter().zip(&answers[first_write..]) {
            let what = format!("{backend} {argv:?}");
            let first = before.run(argv);
            let second = twin.run(argv);
            let first_document = one_shot_document(&first, &what);
            let volatile = disagreeing(&first_document, &one_shot_document(&second, &what));
            // The disagreement is a clock, an event id or a hash over them, and a minority of the
            // document: otherwise this comparison would be comparing nothing.
            assert!(
                volatile
                    .iter()
                    .all(|at| ["_at", "_id", "_hash"].iter().any(|end| at.ends_with(end)))
                    && volatile.len() * 4 < leaf_map(&first_document).len(),
                "{what}: two one-shot runs disagree at {volatile:?}"
            );
            assert_eq!(answer["exit"], 0, "{what}: {answer}");
            assert_eq!(answer["stderr"], "", "{what}");
            let elsewhere: Vec<String> = disagreeing(&answer["stdout"], &first_document)
                .difference(&volatile)
                .cloned()
                .collect();
            assert!(
                elsewhere.is_empty(),
                "{what}: the session disagrees with the one-shot verb at {elsewhere:?}, outside \
                 the fields two one-shot runs disagree on ({volatile:?})"
            );
            let aligned = with_volatile_from(&answer["stdout"], &first_document, &volatile);
            assert_eq!(
                String::from_utf8(printed(&aligned)).unwrap(),
                String::from_utf8_lossy(&first.stdout),
                "{what}: stdout"
            );
        }

        // After the session's commit, the one-shot verbs on the session's own store answer as
        // it did: the commit's exact retry returns its retained receipt byte for byte, every read
        // agrees, and validating the committed transaction again is the same refusal.
        assert_answers_as(
            &answers[first_after - 1],
            &world.run(&["commit", TRANSACTION]),
            &format!("{backend} commit, retried one-shot"),
        );
        for (argv, answer) in after.iter().zip(&answers[first_after..]) {
            assert_answers_as(
                answer,
                &world.run(argv),
                &format!("{backend} {argv:?} after the commit"),
            );
        }
    }
}

/// The most bytes of one request line, newline excluded, as docs/cli.md states it: three times the
/// 8388608-byte document cap and 65536 bytes.
const LINE_LIMIT: usize = 25_231_360;

/// A `hash -` request line of exactly `length` bytes, its payload `a` repeated.
fn hash_request(length: usize) -> String {
    let (open, close) = (r#"{"argv":["hash","-"],"stdin":""#, r#""}"#);
    format!(
        "{open}{}{close}",
        "a".repeat(length - open.len() - close.len())
    )
}

/// One malformed or refused line and the refusal name its answer carries.
fn malformed() -> Vec<(String, &'static str)> {
    let line = |value: Value| value.to_string();
    vec![
        ("not json".to_owned(), "session-request-malformed"),
        (String::new(), "session-request-malformed"),
        ("[\"head\"]".to_owned(), "session-request-malformed"),
        (line(json!({})), "session-request-malformed"),
        (line(json!({"argv": "head"})), "session-request-malformed"),
        (
            line(json!({"argv": ["head", 1]})),
            "session-request-malformed",
        ),
        (
            line(json!({"argv": ["head"], "extra": true})),
            "session-request-malformed",
        ),
        (
            line(json!({"argv": ["hash", "-"], "stdin": [1, 2]})),
            "session-request-malformed",
        ),
        (line(json!({"argv": []})), "session-verb-unknown"),
        (
            line(json!({"argv": ["frobnicate"]})),
            "session-verb-unknown",
        ),
        (
            line(json!({"argv": ["seed", "seed.yaml"]})),
            "session-verb-refused",
        ),
        (line(json!({"argv": ["view"]})), "session-verb-refused"),
        (line(json!({"argv": ["session"]})), "session-verb-refused"),
        (line(json!({"argv": ["mcp"]})), "session-verb-refused"),
        (
            line(json!({"argv": ["migrate", "--to", "elsewhere"]})),
            "session-verb-refused",
        ),
        (line(json!({"argv": ["guide"]})), "session-verb-refused"),
        (
            line(json!({"argv": ["operations"]})),
            "session-verb-refused",
        ),
        (
            line(json!({"argv": ["example", "ekr-seed/2"]})),
            "session-verb-refused",
        ),
        (
            line(json!({"argv": ["head", "--help"]})),
            "session-verb-refused",
        ),
        (line(json!({"argv": ["--version"]})), "session-verb-refused"),
        (line(json!({"argv": ["help"]})), "session-verb-refused"),
        (
            line(json!({"argv": ["help", "head"]})),
            "session-verb-refused",
        ),
        (hash_request(LINE_LIMIT + 1), "session-request-too-large"),
        (
            line(json!({"argv": ["--store", "elsewhere", "head"]})),
            "session-option-refused",
        ),
        (
            line(json!({"argv": ["head", "--backend", "sqlite"]})),
            "session-option-refused",
        ),
        (
            line(json!({"argv": ["head", "--host", "host.json"]})),
            "session-option-refused",
        ),
        (
            line(json!({"argv": ["head", "--full-replay"]})),
            "session-option-refused",
        ),
    ]
}

#[test]
fn a_malformed_line_is_refused_by_name_and_the_next_line_is_still_served() {
    let cases = malformed();
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let head = world.run(&["head"]);
        let mut lines = Vec::new();
        for (line, _) in &cases {
            lines.push(line.clone());
            lines.push(request(&["head"], None));
        }
        let answers = world.session(&lines, true);
        for ((line, name), pair) in cases.iter().zip(answers.chunks(2)) {
            let (refusal, next) = (&pair[0], &pair[1]);
            assert_eq!(refusal["exit"], 2, "{backend} {line:?}: {refusal}");
            assert!(refusal["stdout"].is_null(), "{backend} {line:?}: {refusal}");
            let stderr = refusal["stderr"].as_str().unwrap();
            assert!(
                stderr.starts_with(&format!("ekr: {name}: ")) && stderr.ends_with('\n'),
                "{backend} {line:?}: {stderr:?} is not `ekr: {name}: <reason>`"
            );
            assert_answers_as(next, &head, &format!("{backend} head after {line:?}"));
        }
    }
}

/// The limit is the line's length: a line of exactly [`LINE_LIMIT`] bytes is served whole, one
/// byte more is `session-request-too-large`, and the session serves the line after either.
#[test]
fn a_line_at_the_limit_is_served_and_one_byte_more_is_refused() {
    let world = World::seeded("file");
    let head = world.run(&["head"]);
    let (at, over) = (hash_request(LINE_LIMIT), hash_request(LINE_LIMIT + 1));
    assert_eq!((at.len(), over.len()), (LINE_LIMIT, LINE_LIMIT + 1));
    let payload = at.len() - r#"{"argv":["hash","-"],"stdin":""}"#.len();
    let lines = [at, request(&["head"], None), over, request(&["head"], None)];
    let answers = world.session(&lines, false);
    assert_eq!(answers[0]["exit"], 0, "{}", answers[0]["stderr"]);
    assert_eq!(answers[0]["stdout"]["byte_len"], payload);
    assert_answers_as(&answers[1], &head, "head after the line at the limit");
    assert_eq!(answers[2]["exit"], 2);
    assert!(answers[2]["stdout"].is_null());
    assert!(answers[2]["stderr"]
        .as_str()
        .unwrap()
        .starts_with("ekr: session-request-too-large: "));
    assert_answers_as(&answers[3], &head, "head after the line over the limit");
}

/// The `ekr session` section of `docs/cli.md`, from its heading to the next `###` heading.
fn session_section() -> String {
    let page = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("../../docs/cli.md");
    let page = std::fs::read_to_string(&page)
        .unwrap_or_else(|e| panic!("reading {}: {e}", page.display()));
    let start = page
        .find("\n### `ekr session`\n")
        .expect("docs/cli.md has an `ekr session` section");
    let rest = &page[start + 1..];
    let end = rest[4..].find("\n### ").map_or(rest.len(), |at| at + 4);
    rest[..end].to_owned()
}

#[test]
fn the_page_lists_exactly_the_session_refusals_this_suite_draws() {
    let section = session_section();
    let listed: BTreeSet<String> = section
        .lines()
        .filter_map(|line| line.strip_prefix("| `session-"))
        .map(|rest| format!("session-{}", rest.split('`').next().unwrap()))
        .collect();
    let drawn: BTreeSet<String> = malformed()
        .into_iter()
        .map(|(_, name)| name.to_owned())
        .collect();
    assert_eq!(
        listed, drawn,
        "the session refusal table of docs/cli.md and the refusals this suite draws disagree"
    );
    for needle in [
        "25231360",
        "\"argv\"",
        "\"stdin\"",
        "\"exit\"",
        "\"stdout\"",
        "\"stderr\"",
        "ekr session --create",
        "`store-not-found`, `\"exit\": 1`",
    ] {
        assert!(
            section.contains(needle),
            "the session section names {needle}"
        );
    }
}

/// The head revision of each replay checkpoint the file provider at `store` retains.
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

/// Design § 99.5: when its input ends, a session writes the replay checkpoint of the head it
/// reached if that head is past the retained checkpoint, so that the next process continues from
/// it. One commit is fewer than any commit bound, so without it the retained checkpoint would
/// still be the seed's. Read off the file provider's blobs; the kernel's own case holds both
/// providers (`crates/ekr-kernel/tests/replay_checkpoint.rs`,
/// `a_handle_at_rest_writes_the_checkpoint_of_a_head_past_the_retained_one`).
#[test]
fn a_session_leaves_the_checkpoint_of_its_head_when_its_input_ends() {
    let world = World::seeded("file");
    assert_eq!(retained_checkpoint_revisions(&world.store()), [0]);
    let lines = [
        request(&["propose", "-"], Some(&create_globex())),
        request(&["validate", TRANSACTION], None),
        request(&["commit", TRANSACTION], None),
    ];
    let answers = world.session(&lines, true);
    for (line, answer) in lines.iter().zip(&answers) {
        assert_eq!(answer["exit"], 0, "{line}: {answer}");
    }
    assert_eq!(answers[2]["stdout"]["result"]["revision"], 1);
    assert_eq!(
        retained_checkpoint_revisions(&world.store()),
        [1],
        "the session's head, and only it"
    );
    let head = world.run(&["head"]);
    assert_eq!(head.status.code(), Some(0));
    let head: Value = serde_json::from_slice(&head.stdout).unwrap();
    assert_eq!(head["revision"], 1);
}

#[test]
fn a_commit_in_the_session_is_seen_by_the_next_head_and_resolve() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let lines = [
            request(&["head"], None),
            request(&["resolve", "reference.yaml"], None),
            request(&["propose", "-"], Some(&create_globex())),
            request(&["validate", TRANSACTION], None),
            request(&["commit", TRANSACTION], None),
            request(&["head"], None),
            request(&["resolve", "reference.yaml"], None),
            request(&["resolve", "reference.yaml", "--at", "0"], None),
            request(&["transactions", "--state", "Committed"], None),
        ];
        let answers = world.session(&lines, true);
        for (line, answer) in lines.iter().zip(&answers) {
            assert_eq!(answer["exit"], 0, "{backend} {line}: {answer}");
        }
        let out = |at: usize| &answers[at]["stdout"];
        assert_eq!(out(0)["revision"], 0, "{backend}");
        assert_eq!(out(1)["kind"], "ProposeNew", "{backend}: {}", out(1));
        assert_eq!(out(2)["transaction_id"], TRANSACTION, "{backend}");
        assert_eq!(out(3)["kind"], "Validated", "{backend}: {}", out(3));
        assert_eq!(out(4)["kind"], "Committed", "{backend}: {}", out(4));
        assert_eq!(out(4)["result"]["revision"], 1, "{backend}: {}", out(4));
        assert_eq!(
            out(5)["revision"],
            1,
            "{backend}: the next head is the commit's"
        );
        assert_ne!(out(5)["root"], out(0)["root"], "{backend}");
        assert_eq!(out(6)["kind"], "Resolved", "{backend}: {}", out(6));
        assert_eq!(out(6)["node_id"], NODE, "{backend}");
        assert_eq!(out(7)["kind"], "ProposeNew", "{backend}: {}", out(7));
        assert_eq!(
            out(8).as_array().map(Vec::len),
            Some(1),
            "{backend}: {}",
            out(8)
        );
        // And a fresh process sees what the session committed.
        assert_answers_as(
            &answers[5],
            &world.run(&["head"]),
            &format!("{backend} head after the session"),
        );
    }
}

/// A session whose configuration does not resolve — no host named, or a host file that is not
/// there — answers nothing and exits as a store verb does on the same configuration. A store that
/// does not exist yet is not such a failure: see
/// [`a_session_on_an_absent_store_serves_mint_hash_and_schema_and_answers_store_verbs_as_one_shot`].
#[test]
fn a_session_that_cannot_open_its_store_answers_nothing_and_exits_as_the_verbs_do() {
    let world = World::seeded("file");
    for create in [false, true] {
        let session: &[&str] = if create {
            &["session", "--create"]
        } else {
            &["session"]
        };
        let unconfigured = ekr().args(session).output().unwrap();
        assert_eq!(unconfigured.status.code(), Some(2), "{session:?}");
        assert!(unconfigured.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&unconfigured.stderr)
                .starts_with("ekr: `session` needs --host or EKR_HOST"),
            "{}",
            String::from_utf8_lossy(&unconfigured.stderr)
        );
        let no_host = |verb: &[&str]| {
            ekr()
                .arg("--host")
                .arg(world.directory.path().join("no-host.json"))
                .arg("--store")
                .arg(world.directory.path().join("nothing-here"))
                .args(["--backend", "file"])
                .args(verb)
                .stdin(Stdio::null())
                .output()
                .unwrap()
        };
        let (session_output, head) = (no_host(session), no_host(&["head"]));
        assert_eq!(session_output.status.code(), Some(1), "{session:?}");
        assert!(session_output.stdout.is_empty());
        assert_eq!(head.status.code(), Some(1));
        let session_stderr = String::from_utf8_lossy(&session_output.stderr);
        assert!(
            session_stderr.starts_with("ekr: reading host "),
            "{session_stderr}"
        );
        assert_eq!(session_stderr, String::from_utf8_lossy(&head.stderr));
        assert!(!world.directory.path().join("nothing-here").exists());
    }
}

/// `story:session-starts-before-a-store`: a session started on a path holding no store answers
/// `mint`, `hash` and `schema` as the one-shot verbs do, answers a store verb as the one-shot verb
/// does there — `store-not-found`, exit 1 — and serves the next line; without `--create` it
/// refuses `seed`, and it creates nothing.
#[test]
fn a_session_on_an_absent_store_serves_mint_hash_and_schema_and_answers_store_verbs_as_one_shot() {
    let bob = "Bob is CEO of Acme.\n";
    let lines: Vec<(Vec<&str>, Option<&str>)> = vec![
        (vec!["schema", "typed-reference"], None),
        (vec!["schema", "ekr-seed/2"], None),
        (vec!["hash", "payload.txt"], None),
        (vec!["hash", "-"], Some(bob)),
        (vec!["hash", "evidence-0.txt"], None),
        (vec!["head"], None),
        (vec!["resolve", "reference.yaml"], None),
        (vec!["propose", "create.yaml"], None),
        (vec!["validate", TRANSACTION], None),
        (vec!["snapshot"], None),
        (vec!["schema", "ekr.cli-host/1"], None),
        (vec!["head"], None),
    ];
    for backend in BACKENDS {
        let world = World::absent(backend);
        let mut requests: Vec<String> = lines
            .iter()
            .map(|(argv, stdin)| request(argv, *stdin))
            .collect();
        requests.push(request(&["mint", "node"], None));
        requests.push(request(&["seed", "seed.yaml"], None));
        requests.push(request(&["head"], None));
        let answers = world.session(&requests, true);
        for ((argv, stdin), answer) in lines.iter().zip(&answers) {
            let one_shot = world.one_shot_by_environment(argv, stdin.unwrap_or(""));
            assert_answers_as(answer, &one_shot, &format!("{backend} {argv:?}"));
        }
        let not_found = lines
            .iter()
            .zip(&answers)
            .filter(|(_, answer)| {
                answer["exit"] == 1
                    && answer["stderr"]
                        .as_str()
                        .unwrap()
                        .starts_with("ekr: store-not-found: ")
            })
            .count();
        assert_eq!(
            not_found, 6,
            "{backend}: every store verb is store-not-found"
        );

        let minted = &answers[lines.len()];
        assert_eq!(minted["exit"], 0, "{backend} mint: {minted}");
        assert_eq!(minted["stdout"]["kind"], "node", "{backend} mint: {minted}");
        let seed = &answers[lines.len() + 1];
        assert_eq!(seed["exit"], 2, "{backend} seed: {seed}");
        assert!(
            seed["stderr"]
                .as_str()
                .unwrap()
                .starts_with("ekr: session-verb-refused: "),
            "{backend} seed: {seed}"
        );
        assert_eq!(
            answers[lines.len() + 2],
            answers[5],
            "{backend}: head again"
        );
        assert!(!world.store().exists(), "{backend}: nothing was created");
    }
}

/// The one-shot documents of `a` and the fields two independent one-shot runs disagree on:
/// `a` and `b` ran the same sequence on two worlds that started equal.
fn volatile_between(a: &Output, b: &Output, what: &str) -> (Value, BTreeSet<String>) {
    let (first, second) = (one_shot_document(a, what), one_shot_document(b, what));
    let volatile = disagreeing(&first, &second);
    (first, volatile)
}

/// The session's answer is the one-shot run's `a`: its exit status and stderr byte for byte, and
/// its document byte for byte once the fields `a` and `b` disagree on are `a`'s. Those fields are
/// a clock, a minted id or a hash over them — measured on 2026-09-28 at most 7 of the seed
/// result's 14 leaves — and never more than half the document.
fn assert_answers_as_the_sequence(answer: &Value, a: &Output, b: &Output, what: &str) {
    assert_eq!(
        a.status.code(),
        b.status.code(),
        "{what}: two one-shot runs"
    );
    if a.status.code() != Some(0) {
        assert_answers_as(answer, a, what);
        return;
    }
    let (document, volatile) = volatile_between(a, b, what);
    assert!(
        volatile.len() * 2 <= leaf_map(&document).len(),
        "{what}: two one-shot runs disagree at {volatile:?}, over half the document"
    );
    assert_eq!(answer["exit"], 0, "{what}: {answer}");
    assert_eq!(answer["stderr"], "", "{what}");
    let elsewhere: Vec<String> = disagreeing(&answer["stdout"], &document)
        .difference(&volatile)
        .cloned()
        .collect();
    assert!(
        elsewhere.is_empty(),
        "{what}: the session disagrees with the one-shot verb at {elsewhere:?}, outside the fields \
         two one-shot runs disagree on ({volatile:?})"
    );
    let aligned = with_volatile_from(&answer["stdout"], &document, &volatile);
    assert_eq!(
        String::from_utf8(printed(&aligned)).unwrap(),
        String::from_utf8_lossy(&a.stdout),
        "{what}: stdout"
    );
}

/// `story:session-starts-before-a-store`: `ekr session --create` on an absent store serves `seed`
/// — a document on `stdin` through `-`, payloads through `--evidence` — then holds the new store:
/// `propose`, `validate`, `commit`, `resolve` and the reads after it answer as the same one-shot
/// sequence does, and a second `seed` answers as one-shot `ekr seed` does on an existing store.
#[test]
fn a_create_session_seeds_and_then_answers_as_the_one_shot_sequence() {
    let (stripped, _) = example_without_payloads();
    let sequence: Vec<(Vec<&str>, Option<&str>)> = vec![
        (vec!["head"], None),
        (vec!["hash", "evidence-0.txt"], None),
        (
            vec![
                "seed",
                "-",
                "--evidence",
                "evidence-0.txt",
                "--evidence",
                "evidence-1.txt",
            ],
            Some(&stripped),
        ),
        (vec!["head"], None),
        (vec!["resolve", "reference.yaml"], None),
        (vec!["propose", "create.yaml"], None),
        (vec!["validate", TRANSACTION], None),
        (vec!["commit", TRANSACTION], None),
        (vec!["head"], None),
        (vec!["resolve", "reference.yaml"], None),
        (vec!["snapshot"], None),
        (vec!["transactions"], None),
        (vec!["ontology"], None),
        // A second seed: the same document is its exact retry, another one is refused.
        (
            vec![
                "seed",
                "-",
                "--evidence",
                "evidence-0.txt",
                "--evidence",
                "evidence-1.txt",
            ],
            Some(&stripped),
        ),
        (vec!["seed", "other-seed.yaml"], None),
        (vec!["seed", "stripped.yaml"], None),
        (vec!["head"], None),
    ];
    let requests: Vec<String> = sequence
        .iter()
        .map(|(argv, stdin)| request(argv, *stdin))
        .collect();
    for backend in BACKENDS {
        let (session, a, b) = (
            World::absent(backend),
            World::absent(backend),
            World::absent(backend),
        );
        let answers = session.session_as(&["session", "--create"], &requests, true);
        assert_eq!(answers[0]["exit"], 1, "{backend}: head before the seed");
        assert_eq!(answers[2]["exit"], 0, "{backend}: seed: {}", answers[2]);
        assert_eq!(answers[7]["stdout"]["kind"], "Committed", "{backend}");
        assert_eq!(answers[9]["stdout"]["kind"], "Resolved", "{backend}");
        assert_eq!(answers[13], answers[2], "{backend}: the exact retry");
        for (at, ((argv, stdin), answer)) in sequence.iter().zip(&answers).enumerate() {
            let stdin = stdin.unwrap_or("");
            // A fault names the store's path, which is each world's own.
            let rebased = |from: &World, output: Output| Output {
                stderr: String::from_utf8(output.stderr)
                    .unwrap()
                    .replace(
                        from.directory.path().to_str().unwrap(),
                        session.directory.path().to_str().unwrap(),
                    )
                    .into_bytes(),
                ..output
            };
            let (first, second) = (
                rebased(&a, a.one_shot_by_environment(argv, stdin)),
                rebased(&b, b.one_shot_by_environment(argv, stdin)),
            );
            assert_answers_as_the_sequence(
                answer,
                &first,
                &second,
                &format!("{backend} #{at} {argv:?}"),
            );
        }
        // Another create session on the store this one made: `seed` is one-shot `ekr seed` on an
        // existing store, byte for byte, and every store verb is served.
        let again = [
            request(&["seed", "seed.yaml"], None),
            request(&["seed", "other-seed.yaml"], None),
            request(&["head"], None),
        ];
        let answers = session.session_as(&["session", "--create"], &again, false);
        for (argv, answer) in [
            vec!["seed", "seed.yaml"],
            vec!["seed", "other-seed.yaml"],
            vec!["head"],
        ]
        .iter()
        .zip(&answers)
        {
            assert_answers_as(
                answer,
                &session.run(argv),
                &format!("{backend} {argv:?} on the created store"),
            );
        }
        assert_eq!(answers[2]["stdout"]["revision"], 1, "{backend}");
    }
}

/// A session held open: one request written, its one answer read, then the next.
struct Live {
    child: std::process::Child,
    stdin: Option<std::process::ChildStdin>,
    stdout: std::io::BufReader<std::process::ChildStdout>,
}

impl Live {
    fn start(world: &World, verb: &[&str]) -> Self {
        world.spawned.set(world.spawned.get() + 1);
        let mut child = world
            .command(verb)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let stdout = std::io::BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
        }
    }

    /// The whole answer line, whatever its exit.
    fn answer(&mut self, argv: &[&str], stdin: Option<&str>) -> Value {
        use std::io::BufRead as _;
        let input = self.stdin.as_mut().unwrap();
        input
            .write_all(format!("{}\n", request(argv, stdin)).as_bytes())
            .unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|error| panic!("{argv:?}: {error}: {line:?}"))
    }

    /// The answer's document, which must have exit 0.
    fn ask(&mut self, argv: &[&str], stdin: Option<&str>) -> Value {
        let answer = self.answer(argv, stdin);
        assert_eq!(answer["exit"], 0, "{argv:?}: {answer}");
        answer["stdout"].clone()
    }

    fn close(mut self) {
        drop(self.stdin.take());
        let output = self.child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
    }
}

/// `path` with `suffix` appended to its last component.
fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut text = path.as_os_str().to_owned();
    text.push(suffix);
    PathBuf::from(text)
}

/// Moves what is at `store` to `aside` and `from` into its place, each by rename, as a host
/// promoting a new store does. A SQLite database moves with its `-wal` and `-shm` files.
fn replace(store: &Path, aside: &Path, from: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let at = suffixed(store, suffix);
        if std::fs::symlink_metadata(&at).is_ok() {
            std::fs::rename(&at, suffixed(aside, suffix)).unwrap();
        }
    }
    for suffix in ["", "-wal", "-shm"] {
        let at = suffixed(from, suffix);
        if std::fs::symlink_metadata(&at).is_ok() {
            std::fs::rename(&at, suffixed(store, suffix)).unwrap();
        }
    }
}

/// `task:readers-reopen-a-replaced-store`: a running session whose store is replaced by rename
/// answers the next store verb from the store now at its path. While a transaction the session
/// proposed is neither committed nor rejected, it refuses a replacement by name,
/// `store-replaced-proposals-open`, on every store verb — never dropping the proposal, never
/// answering from the replaced store — and serves again once the store it opened is back. A file
/// that is not a store is `store-replaced`, exit 1, on every later store verb. Verbs that open no
/// store are served throughout.
#[test]
fn a_session_answers_from_a_store_replaced_by_rename_and_keeps_its_open_proposals() {
    const PROPOSED: &str = "00000000-0000-4000-8000-000000000904";
    const PROPOSED_NODE: &str = "00000000-0000-4000-8000-000000000905";
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let original = world.directory.path().join("original");
        let second = world.directory.path().join("second");
        let mut live = Live::start(&world, &["session"]);
        assert_eq!(live.ask(&["head"], None)["revision"], 0, "{backend}");

        let replacement = World::seeded(backend);
        let proposed = one_shot_document(&replacement.run(&["propose", "create.yaml"]), "propose");
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        one_shot_document(&replacement.run(&["validate", &id]), "validate");
        one_shot_document(&replacement.run(&["commit", &id]), "commit");
        replace(&world.store(), &original, &replacement.store());

        let head = live.answer(&["head"], None);
        assert_answers_as(&head, &world.run(&["head"]), &format!("{backend} head"));
        assert_eq!(head["stdout"]["revision"], 1, "{backend}");
        let resolved = live.ask(&["resolve", "reference.yaml"], None);
        assert_eq!(resolved["node_id"], NODE, "{backend}: {resolved}");

        // A proposal of this session is open: the store it lives in is not dropped.
        let initech = create_globex_as(PROPOSED, PROPOSED_NODE).replace("Globex", "Initech");
        live.ask(&["propose", "-"], Some(&initech));
        replace(&world.store(), &second, &original);
        for argv in [
            &["head"][..],
            &["validate", PROPOSED],
            &["commit", PROPOSED],
        ] {
            let refused = live.answer(argv, None);
            assert_eq!(refused["exit"], 2, "{backend} {argv:?}: {refused}");
            assert!(refused["stdout"].is_null(), "{backend}: {refused}");
            let stderr = refused["stderr"].as_str().unwrap();
            assert!(
                stderr.starts_with("ekr: store-replaced-proposals-open: ")
                    && stderr.contains(PROPOSED),
                "{backend} {argv:?}: {stderr}"
            );
        }
        live.ask(&["mint", "node"], None);
        replace(&world.store(), &original, &second);
        assert_eq!(live.ask(&["head"], None)["revision"], 1, "{backend}");
        assert_eq!(live.ask(&["validate", PROPOSED], None)["kind"], "Validated");
        assert_eq!(live.ask(&["commit", PROPOSED], None)["kind"], "Committed");

        // Nothing open: the next replacement is followed.
        replace(&world.store(), &second, &original);
        let head = live.answer(&["head"], None);
        assert_answers_as(&head, &world.run(&["head"]), &format!("{backend} head"));
        assert_eq!(head["stdout"]["revision"], 0, "{backend}");

        // A file that is not a store.
        let junk = world.directory.path().join("junk");
        std::fs::write(&junk, "not a store\n").unwrap();
        replace(&world.store(), &original, &junk);
        for argv in [&["head"][..], &["resolve", "reference.yaml"], &["head"]] {
            let refused = live.answer(argv, None);
            assert_eq!(refused["exit"], 1, "{backend} {argv:?}: {refused}");
            assert!(refused["stdout"].is_null(), "{backend}: {refused}");
            let stderr = refused["stderr"].as_str().unwrap();
            assert!(
                stderr.starts_with("ekr: store-replaced: ")
                    && stderr.contains(&world.store().display().to_string()),
                "{backend} {argv:?}: {stderr}"
            );
        }
        live.ask(&["mint", "node"], None);
        live.close();
    }
}

/// `CreateNode` of `Globex` under a minted transaction and node id.
fn create_globex_as(transaction: &str, node: &str) -> String {
    create_globex()
        .replace(TRANSACTION, transaction)
        .replace(NODE, node)
}

/// The consumer's build of a small store: hash each evidence file for the seed's entries, seed
/// with those files, mint the ids of a first transaction, then propose, validate and commit it.
/// One-shot, every call is a process; in `ekr session --create` the whole build is one. The
/// counts are measured as the processes each world started, and printed.
#[test]
fn a_create_session_builds_a_store_in_one_process_where_one_shot_takes_one_per_call() {
    let seed_argv = [
        "seed",
        "stripped.yaml",
        "--evidence",
        "evidence-0.txt",
        "--evidence",
        "evidence-1.txt",
    ];
    for backend in BACKENDS {
        let (one_shot, session) = (World::absent(backend), World::absent(backend));
        let stripped =
            std::fs::read_to_string(one_shot.directory.path().join("stripped.yaml")).unwrap();

        let ok =
            |argv: &[&str], stdin: &str| one_shot_document(&one_shot.one_shot(argv, stdin), "");
        for file in ["evidence-0.txt", "evidence-1.txt"] {
            let hash = ok(&["hash", file], "");
            assert!(stripped.contains(hash["content_hash"].as_str().unwrap()));
        }
        ok(&seed_argv, "");
        let (transaction, node) = (ok(&["mint", "transaction"], ""), ok(&["mint", "node"], ""));
        let (transaction, node) = (
            transaction["id"].as_str().unwrap().to_owned(),
            node["id"].as_str().unwrap().to_owned(),
        );
        ok(&["propose", "-"], &create_globex_as(&transaction, &node));
        ok(&["validate", &transaction], "");
        ok(&["commit", &transaction], "");
        let built = ok(&["resolve", "reference.yaml"], "");
        assert_eq!(built["node_id"], node.as_str(), "{backend}: {built}");

        let mut live = Live::start(&session, &["session", "--create"]);
        for file in ["evidence-0.txt", "evidence-1.txt"] {
            let hash = live.ask(&["hash", file], None);
            assert!(stripped.contains(hash["content_hash"].as_str().unwrap()));
        }
        live.ask(&seed_argv, None);
        let (transaction, node) = (
            live.ask(&["mint", "transaction"], None),
            live.ask(&["mint", "node"], None),
        );
        let (transaction, node) = (
            transaction["id"].as_str().unwrap().to_owned(),
            node["id"].as_str().unwrap().to_owned(),
        );
        live.ask(
            &["propose", "-"],
            Some(&create_globex_as(&transaction, &node)),
        );
        live.ask(&["validate", &transaction], None);
        live.ask(&["commit", &transaction], None);
        let built = live.ask(&["resolve", "reference.yaml"], None);
        assert_eq!(built["node_id"], node.as_str(), "{backend}: {built}");
        live.close();

        println!(
            "{backend}: building the store took {} one-shot processes and {} session process",
            one_shot.spawned.get(),
            session.spawned.get()
        );
        assert_eq!((one_shot.spawned.get(), session.spawned.get()), (9, 1));
    }
}

/// The in-process seam: `ekr::cli::serve` answers the lines of any reader into any writer, as
/// the binary does, and `ekr::cli::run` of `session` returns the same answers as its result.
#[test]
fn the_library_seam_serves_a_session_in_process() {
    let world = World::seeded("sqlite");
    let argv = |verb: &str| -> Vec<std::ffi::OsString> {
        vec![
            "ekr".into(),
            "--host".into(),
            world.directory.path().join("host.json").into(),
            "--store".into(),
            world.store().into(),
            "--backend".into(),
            "sqlite".into(),
            verb.into(),
        ]
    };
    let lines = format!(
        "{}\nnot json\n{}\n",
        request(&["head"], None),
        request(&["head"], None)
    );
    let clock = || ekr_core::Timestamp::from_millis(0);

    let cli = <ekr::cli::Cli as clap::Parser>::try_parse_from(argv("session")).unwrap();
    let mut output = Vec::new();
    ekr::cli::serve(cli, &clock, &mut lines.as_bytes(), &mut output).unwrap();
    let served = String::from_utf8(output).unwrap();

    let returned = ekr::cli::run(argv("session"), &clock, &mut lines.as_bytes()).unwrap();
    assert_eq!(served, returned);
    let answers: Vec<Value> = served
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(answers.len(), 3);
    assert_answers_as(&answers[0], &world.run(&["head"]), "in-process head");
    assert_eq!(answers[1]["exit"], 2);
    assert_eq!(answers[0], answers[2]);

    // `ekr::cli::Command::Session { create }`: the flag as the library parses it, and an
    // in-process create session on an absent store seeding it.
    let absent = World::absent("sqlite");
    let create = vec![
        std::ffi::OsString::from("ekr"),
        "--host".into(),
        absent.directory.path().join("host.json").into(),
        "--store".into(),
        absent.store().into(),
        "--backend".into(),
        "sqlite".into(),
        "session".into(),
        "--create".into(),
    ];
    let cli = <ekr::cli::Cli as clap::Parser>::try_parse_from(&create).unwrap();
    assert!(matches!(
        cli.command,
        ekr::cli::Command::Session { create: true }
    ));
    let seed = std::fs::read_to_string(absent.directory.path().join("seed.yaml")).unwrap();
    let lines = format!(
        "{}\n{}\n{}\n",
        request(&["head"], None),
        request(&["seed", "-"], Some(&seed)),
        request(&["head"], None)
    );
    let returned = ekr::cli::run(create, &clock, &mut lines.as_bytes()).unwrap();
    let answers: Vec<Value> = returned
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(answers[0]["exit"], 1, "{}", answers[0]);
    assert_eq!(answers[1]["exit"], 0, "{}", answers[1]);
    assert_answers_as(
        &answers[2],
        &absent.run(&["head"]),
        "in-process head after seed",
    );
}
