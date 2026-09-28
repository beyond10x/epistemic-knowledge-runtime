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
//!   and `resolve` of the same session.

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

/// One provider in its own directory under the example host, seeded from the example seed, with
/// the files the requests name: `create.yaml`, `reference.yaml` and `payload.txt`.
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
        world.file("reference.yaml", &text(&["example", "typed-reference"]));
        world.file("create.yaml", &create_globex());
        world.file("payload.txt", "Alice is CEO of Acme.\n");
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

    /// One one-shot verb, with `stdin` as its standard input.
    fn one_shot(&self, verb: &[&str], stdin: &str) -> Output {
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

    fn run(&self, verb: &[&str]) -> Output {
        self.one_shot(verb, "")
    }

    /// One `ekr session` over `lines`, configured by flags or by `EKR_*`, and its answers: exit 0,
    /// nothing on stderr, and one JSON line per request line.
    fn session(&self, lines: &[String], by_environment: bool) -> Vec<Value> {
        let mut command = if by_environment {
            let mut command = ekr();
            command
                .current_dir(self.directory.path())
                .env("EKR_HOST", self.directory.path().join("host.json"))
                .env("EKR_STORE", self.store())
                .env("EKR_BACKEND", self.backend)
                .arg("session");
            command
        } else {
            self.command(&["session"])
        };
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

        // Every read and refusal on the seeded state, which `before` still holds byte for byte.
        for ((argv, stdin), answer) in reads.iter().zip(&answers) {
            let one_shot = before.one_shot(argv, stdin.unwrap_or(""));
            assert_answers_as(answer, &one_shot, &format!("{backend} {argv:?}"));
        }
        let refused = reads
            .iter()
            .zip(&answers)
            .filter(|(_, answer)| answer["exit"] != 0)
            .count();
        assert_eq!(refused, 7, "{backend}: the refusal lines are refused");

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
        "\"argv\"",
        "\"stdin\"",
        "\"exit\"",
        "\"stdout\"",
        "\"stderr\"",
    ] {
        assert!(
            section.contains(needle),
            "the session section names {needle}"
        );
    }
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

#[test]
fn a_session_that_cannot_open_its_store_answers_nothing_and_exits_as_the_verbs_do() {
    let world = World::seeded("file");
    let unconfigured = ekr().arg("session").output().unwrap();
    assert_eq!(unconfigured.status.code(), Some(2));
    assert!(unconfigured.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&unconfigured.stderr)
            .starts_with("ekr: `session` needs --host or EKR_HOST"),
        "{}",
        String::from_utf8_lossy(&unconfigured.stderr)
    );
    let missing = ekr()
        .arg("--host")
        .arg(world.directory.path().join("host.json"))
        .arg("--store")
        .arg(world.directory.path().join("nothing-here"))
        .args(["--backend", "file", "session"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert!(missing.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&missing.stderr).starts_with("ekr: store-not-found: "),
        "{}",
        String::from_utf8_lossy(&missing.stderr)
    );
    assert!(!world.directory.path().join("nothing-here").exists());
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
}
