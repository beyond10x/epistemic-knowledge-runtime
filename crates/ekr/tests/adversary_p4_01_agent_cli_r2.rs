//! Adversary pass 2 on `story:agent-discoverable-cli`: the sections the correction added to
//! `ekr guide` (OUTPUT, ASSESSMENT, CONFIGURATION), read as the specification they claim to be
//! and driven against the binary that prints them.
//!
//! Every case uses only strings the binary printed: `ekr example`, `ekr operations <Kind>`,
//! `ekr mint`, `ekr guide`.

use std::path::PathBuf;
use std::process::Output;

use serde_json::Value;

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
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

fn minted(kind: &str) -> String {
    let out: Value = serde_json::from_str(&text(&["mint", kind])).unwrap();
    out["id"].as_str().unwrap().to_owned()
}

fn host() -> Value {
    serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap()
}

/// The example operation of `ekr operations <Kind>`: every line after the `Example` heading.
fn example_operation(kind: &str) -> String {
    let out = text(&["operations", kind]);
    let at = out
        .lines()
        .position(|line| line.starts_with("Example"))
        .unwrap_or_else(|| panic!("{kind}: no Example section"));
    let mut example = out.lines().skip(at + 1).collect::<Vec<_>>().join("\n");
    example.push('\n');
    example
}

/// A transaction document around printed operations, citing `evidence`.
fn document(operations: &[String], evidence: &[&str]) -> (String, String) {
    let id = minted("transaction");
    let operator = host()["context"]["operator"].as_str().unwrap().to_owned();
    let mut text = format!(
        "format: ekr.transaction-document/1\ntransaction:\n  id: {id}\n  proposer: {operator}\n  operations:\n"
    );
    for operation in operations {
        for line in operation.lines() {
            text.push_str("  ");
            text.push_str(line);
            text.push('\n');
        }
    }
    text.push_str(&format!("  evidence: [{}]\n", evidence.join(", ")));
    (id, text)
}

/// The printed AddAssertion example with a freshly minted id; (id, operation, cited evidence).
fn fresh_assertion() -> (String, String, String) {
    let example = example_operation("AddAssertion");
    let printed_id = example
        .lines()
        .find_map(|l| l.trim().strip_prefix("id: "))
        .unwrap()
        .to_owned();
    let evidence = example
        .lines()
        .skip_while(|l| l.trim() != "evidence:")
        .nth(1)
        .and_then(|l| l.trim().strip_prefix("- "))
        .unwrap()
        .to_owned();
    let id = minted("assertion");
    (id.clone(), example.replace(&printed_id, &id), evidence)
}

/// A file store seeded from `ekr example ekr-seed/2` under `ekr example ekr.cli-host/1`.
struct World {
    directory: tempfile::TempDir,
    host: PathBuf,
}

impl World {
    fn seeded() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let host = directory.path().join("host.json");
        std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
        let world = Self { directory, host };
        let seed = world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        world.ok(&["seed", &seed]);
        world
    }

    fn store(&self) -> PathBuf {
        self.directory.path().join("store")
    }

    fn file(&self, name: &str, contents: &str) -> String {
        let path = self.directory.path().join(name);
        std::fs::write(&path, contents).unwrap();
        path.display().to_string()
    }

    fn run(&self, verb: &[&str]) -> Output {
        ekr()
            .arg("--host")
            .arg(&self.host)
            .arg("--store")
            .arg(self.store())
            .args(["--backend", "file"])
            .args(verb)
            .output()
            .unwrap()
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{verb:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

/// Standard padded base64 (RFC 4648 § 4), independent of the implementation's encoder.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        out.push(ALPHABET[usize::from(b[0] >> 2)] as char);
        out.push(ALPHABET[usize::from(((b[0] & 3) << 4) | (b[1] >> 4))] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[usize::from(((b[1] & 15) << 2) | (b[2] >> 6))] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[usize::from(b[2] & 63)] as char
        } else {
            '='
        });
    }
    out
}

fn strings(value: &Value, into: &mut Vec<String>) {
    match value {
        Value::String(s) => into.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|i| strings(i, into)),
        Value::Object(map) => map.values().for_each(|i| strings(i, into)),
        _ => {}
    }
}

/// `ekr guide`, OUTPUT, says "the seed's evidence payloads" print as one base64 string. The
/// printed seed says evidence 0401's payload is "Alice is CEO of Acme." So some verb an agent can
/// run on a store seeded from it prints that payload, base64-encoded.
///
/// Measured: no verb prints any evidence payload. `snapshot` and `explain` carry only
/// `content_hash`; the `evidence_payloads` branch of `render` (`src/cli/mod.rs`) is reached by
/// no verb's result. After the fix, either a verb prints the payload (the read exists:
/// `Runtime::content`) and this case holds it, or OUTPUT stops promising it and this case is
/// replaced by one that holds the new sentence.
#[test]
fn the_evidence_payloads_the_guide_says_print_as_base64_are_printed_by_some_verb() {
    let guide = text(&["guide"])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        guide.contains("the seed's evidence payloads"),
        "precondition: {guide}"
    );
    let seed = text(&["example", "ekr-seed/2"]);
    let payload = "Alice is CEO of Acme.";
    assert!(
        seed.contains(&format!("\"{payload}\" (evidence 0401)")),
        "precondition: {seed}"
    );
    let world = World::seeded();
    let snapshot = world.ok(&["snapshot"]);
    let assertion = snapshot["graph"]["graph"]["assertions"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, a)| {
            a["evidence"][0]
                .as_str()
                .is_some_and(|e| e.ends_with("0401"))
        })
        .map(|(id, _)| id.clone())
        .unwrap();
    let mut printed = Vec::new();
    for verb in [
        &["snapshot"][..],
        &["snapshot", "--valid-at", "2026-01-01"],
        &["explain", &assertion],
        &["transactions"],
        &["head"],
        &["ontology"],
    ] {
        strings(&world.ok(verb), &mut printed);
    }
    let encoded = base64(payload.as_bytes());
    assert!(
        printed.iter().any(|s| s == &encoded),
        "no verb prints evidence 0401's payload as base64 {encoded:?}, though `ekr guide` OUTPUT \
         says the seed's evidence payloads print that way"
    );
}

/// `ekr guide`, ASSESSMENT, as corrected: acceptance is judged at commit, and "An assertion added
/// and retracted in the same transaction validates and commits, and reads back Accepted and
/// Retracted." This case holds that sentence to the binary.
#[test]
fn an_assertion_retracted_in_the_transaction_that_adds_it_commits_as_the_guide_says() {
    let guide = text(&["guide"])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        guide.contains("Acceptance is judged at commit")
            && guide.contains(
                "An assertion added and retracted in the same transaction validates and \
                 commits, and reads back Accepted and Retracted."
            ),
        "precondition: {guide}"
    );
    let world = World::seeded();
    let (id, add, evidence) = fresh_assertion();
    let retract = example_operation("RetractAssertion");
    let printed = retract
        .lines()
        .find_map(|l| l.trim().strip_prefix("assertion: "))
        .unwrap()
        .to_owned();
    let retract = retract.replace(&printed, &id);
    let (transaction, doc) = document(&[add, retract], &[&evidence]);
    let path = world.file("add-and-retract.yaml", &doc);
    world.ok(&["propose", &path]);
    let validated = world.ok(&["validate", &transaction]);
    assert_eq!(validated["kind"], "Validated", "{validated}");
    let committed = world.ok(&["commit", &transaction]);
    assert_eq!(committed["kind"], "Committed", "{committed}");
    let held = &world.ok(&["snapshot"])["graph"]["graph"]["assertions"][&id];
    assert!(held["assessment"]["Accepted"].is_object(), "{held}");
    assert!(held["lifecycle"]["Retracted"].is_object(), "{held}");
}

/// `ekr guide`, ASSESSMENT, as corrected: an assertion stating its own verdict is not a refusal
/// (EXIT CODES: exit 2, nothing recorded): "propose records it (exit 0) and validation rejects it
/// with the issue code assertion-states-its-own-verdict". This case holds that sentence.
#[test]
fn an_assertion_that_states_its_own_verdict_is_recorded_then_rejected_as_the_guide_says() {
    let guide = text(&["guide"])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        guide.contains(
            "propose records it (exit 0) and validation rejects it with the issue code \
             assertion-states-its-own-verdict"
        ) && !guide.contains("verdict is refused"),
        "precondition: {guide}"
    );
    let validator = host()["context"]["validator"].as_str().unwrap().to_owned();
    let world = World::seeded();
    let (_, add, evidence) = fresh_assertion();
    let add = add.replace(
        "  assessment: Proposed",
        &format!("  assessment: !Accepted\n    validators:\n    - {validator}"),
    );
    let (transaction, doc) = document(&[add], &[&evidence]);
    let path = world.file("self-accepted.yaml", &doc);
    let output = world.run(&["propose", &path]);
    let recorded = world.ok(&["transactions"]);
    let recorded =
        recorded.as_array().unwrap().iter().any(|t| {
            t["id"] == transaction.as_str() || t["transaction_id"] == transaction.as_str()
        });
    assert!(
        output.status.code() == Some(0) && recorded,
        "propose exited {:?} and recorded={recorded}; the guide says propose records it: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let validated = world.ok(&["validate", &transaction]);
    assert_eq!(validated["kind"], "Rejected", "{validated}");
    assert!(
        validated
            .to_string()
            .contains("assertion-states-its-own-verdict"),
        "{validated}"
    );
}

/// `ekr guide`, CONFIGURATION: "--backend <file | sqlite> or EKR_BACKEND". One setting, one set
/// of values: a value the flag refuses is refused through the variable too, and the reverse.
///
/// Measured: `--backend FILE` is a usage error (exit 2); `EKR_BACKEND=FILE` is accepted (exit
/// 0), because the variable is parsed with `ValueEnum::from_str(_, true)` (ignore case) and the
/// flag by clap's case-sensitive `value_enum`.
#[test]
fn the_backend_flag_and_its_variable_accept_the_same_values() {
    let world = World::seeded();
    let flag = ekr()
        .arg("--host")
        .arg(&world.host)
        .arg("--store")
        .arg(world.store())
        .args(["--backend", "FILE", "head"])
        .output()
        .unwrap();
    let var = ekr()
        .env("EKR_BACKEND", "FILE")
        .arg("--host")
        .arg(&world.host)
        .arg("--store")
        .arg(world.store())
        .arg("head")
        .output()
        .unwrap();
    assert_eq!(
        flag.status.code(),
        var.status.code(),
        "--backend FILE exited {:?}, EKR_BACKEND=FILE exited {:?}",
        flag.status.code(),
        var.status.code()
    );
}

/// `ekr guide`, CONFIGURATION: "A flag wins over its variable". No existing case sets a flag and
/// the variable of the same setting together, so swapping the precedence in `flag_or_var` or the
/// backend match (`src/cli/mod.rs`) leaves the suite green. This case would catch that mutant: each
/// variable names a store that is not the flag's, and the flag's store is the one read.
#[test]
fn a_flag_wins_over_its_variable_for_every_setting() {
    let world = World::seeded();
    let other = tempfile::tempdir().unwrap();
    let decoy_host = other.path().join("host.json");
    std::fs::write(&decoy_host, "not a host document").unwrap();
    let decoy_store = other.path().join("store.db");
    let output = ekr()
        .env("EKR_HOST", &decoy_host)
        .env("EKR_STORE", &decoy_store)
        .env("EKR_BACKEND", "sqlite")
        .arg("--host")
        .arg(&world.host)
        .arg("--store")
        .arg(world.store())
        .args(["--backend", "file", "head"])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let head: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(head["revision"], 0, "{head}");
    assert!(!decoy_store.exists(), "the variable's store was opened");
}
