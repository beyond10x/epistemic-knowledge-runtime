//! Adversary pass, wave sdk-01, unit L (`story:node-gains-an-alias`), through the built binary.
//!
//! * `docs/cli.md` says how many kinds `ekr operations` lists; that number is the binary's.
//! * An `ekr session` that commits an `AddAlias` answers the next `resolve` of the same session
//!   with the node, not with the head it opened on.

use std::io::Write as _;
use std::path::Path;
use std::process::Stdio;

use serde_json::{json, Value};

const PERSON: &str = "00000000-0000-4000-8000-000000000201";
const BOB: &str = "00000000-0000-4000-8000-000000000302";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const TRANSACTION: &str = "00000000-0000-4000-8000-000000000a01";

fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

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

fn page() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the manifest dir");
    let root = Path::new(&manifest)
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf();
    std::fs::read_to_string(root.join("docs/cli.md")).unwrap()
}

const NUMBERS: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];

/// The `### \`ekr operations\`` section says `ekr operations` "lists the <n> operation kinds, one
/// per line"; `<n>` is the number of lines the binary prints.
#[test]
fn the_ekr_operations_section_counts_the_kinds_the_binary_lists() {
    let listed = text(&["operations"]).lines().count();
    let page = page();
    let start = page
        .find("### `ekr operations`")
        .expect("docs/cli.md has an `ekr operations` section");
    let section = &page[start..];
    let section = &section[..section[4..]
        .find("\n### ")
        .map_or(section.len(), |end| end + 4)];
    let prose = section.split_whitespace().collect::<Vec<_>>().join(" ");
    let said = prose
        .split("lists the ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .expect("the section says how many kinds it lists");
    assert_eq!(
        NUMBERS.iter().position(|word| *word == said),
        Some(listed),
        "docs/cli.md says `ekr operations` lists the {said} operation kinds; the binary lists \
         {listed}"
    );
}

/// One `ekr session` over the example seed: resolve Bob's new key, commit an `AddAlias` giving it
/// to Bob, resolve again. The second answer is Bob.
#[test]
fn a_session_that_commits_an_add_alias_resolves_the_alias_in_its_next_request() {
    for backend in ["file", "sqlite"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let file = |name: &str, contents: &str| std::fs::write(path.join(name), contents).unwrap();
        file("host.json", &text(&["example", "ekr.cli-host/1"]));
        file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        file(
            "key.yaml",
            &format!("type_id: {PERSON}\naliases:\n- \"people:7\"\n"),
        );
        file(
            "add.yaml",
            &format!(
                "format: ekr.transaction-document/2\ntransaction:\n  id: {TRANSACTION}\n  \
                 proposer: {OPERATOR}\n  operations:\n  - !AddAlias\n    node: {BOB}\n    \
                 alias: \"people:7\"\n  evidence: []\n"
            ),
        );
        let store = match backend {
            "file" => path.join("store"),
            _ => path.join("state.db"),
        };
        let configured = |command: &mut std::process::Command| {
            command
                .current_dir(path)
                .arg("--host")
                .arg(path.join("host.json"))
                .arg("--store")
                .arg(&store)
                .args(["--backend", backend]);
        };
        let mut seed = ekr();
        configured(&mut seed);
        let seeded = seed.args(["seed", "seed.yaml"]).output().unwrap();
        assert_eq!(
            seeded.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&seeded.stderr)
        );

        let lines = [
            json!({"argv": ["resolve", "key.yaml"]}),
            json!({"argv": ["propose", "add.yaml"]}),
            json!({"argv": ["validate", TRANSACTION]}),
            json!({"argv": ["commit", TRANSACTION]}),
            json!({"argv": ["resolve", "key.yaml"]}),
        ];
        let mut input = String::new();
        for line in &lines {
            input.push_str(&line.to_string());
            input.push('\n');
        }
        let mut session = ekr();
        configured(&mut session);
        let mut child = session
            .arg("session")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let answers: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(answers.len(), lines.len(), "{backend}: {answers:?}");
        for answer in &answers {
            assert_eq!(answer["exit"], 0, "{backend}: {answers:?}");
        }
        let document = |answer: &Value| -> Value {
            match &answer["stdout"] {
                Value::String(text) => serde_json::from_str(text).unwrap(),
                other => other.clone(),
            }
        };
        assert_eq!(document(&answers[0])["kind"], "ProposeNew", "{backend}");
        assert_eq!(document(&answers[3])["kind"], "Committed", "{backend}");
        assert_eq!(
            document(&answers[4]),
            json!({"kind": "Resolved", "node_id": BOB}),
            "{backend}: the session's next resolve sees the committed alias"
        );
    }
}
