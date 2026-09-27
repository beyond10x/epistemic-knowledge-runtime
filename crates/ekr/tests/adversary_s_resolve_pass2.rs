//! Adversary cases, pass 2, for `story:ekr-resolve-verb` (`ekr resolve`): the correction that put
//! a strict walk of the loader's event tape (`resolve::strict`) in front of the typed decoder and a
//! self-describing list decoder on `TypedReference::aliases`.
//!
//! * `strict` checks the value kinds only when the document is a top-level mapping ("a document
//!   of another shape is left to the decoder"), but serde's derived struct decoder also accepts a
//!   sequence, `[type_id, aliases]`. `docs/cli.md` says the reader refuses what
//!   `ekr schema typed-reference` refuses, and that schema is an object.
//! * Deep nesting, several documents, empty input and unreadable input are faults naming the
//!   document, exit 1, and never a crash.
//! * The strict list decoder must still read the JSON the verb itself prints.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use ekr_integrate::{ResolutionOutcome, TypedReference};
use serde_json::Value;

const PERSON: &str = "00000000-0000-4000-8000-000000000201";

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

struct World {
    directory: tempfile::TempDir,
}

impl World {
    fn seeded() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let world = Self { directory };
        world.file("host.json", &text(&["example", "ekr.cli-host/1"]));
        world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        let output = world.run(&["seed", "seed.yaml"]);
        assert_eq!(output.status.code(), Some(0), "seed");
        world
    }

    fn store(&self) -> PathBuf {
        self.directory.path().join("store")
    }

    fn file(&self, name: &str, contents: &str) -> String {
        std::fs::write(self.directory.path().join(name), contents).unwrap();
        name.to_owned()
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

    fn run(&self, verb: &[&str]) -> Output {
        self.command(verb).output().unwrap()
    }

    fn stdin(&self, bytes: &[u8]) -> Output {
        let mut child = self
            .command(&["resolve", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(bytes).unwrap();
        child.wait_with_output().unwrap()
    }
}

fn projection(document: &str) -> Value {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(document).unwrap();
    serde_json::to_value(value).unwrap()
}

fn assert_fault(output: &Output, file: &str, what: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{what}: expected a fault (exit 1), got {:?}; stdout {}; stderr {stderr}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
    );
    assert!(output.stdout.is_empty(), "{what}");
    assert!(
        stderr.starts_with(&format!("ekr: typed reference {file}: ")),
        "{what}: {stderr}"
    );
}

/// A typed reference written as a YAML sequence, `[type_id, [aliases]]`, is not an object, so
/// `ekr schema typed-reference` refuses it. The reader must refuse it too (a fault naming the
/// document, exit 1), in flow and block style.
#[test]
fn a_typed_reference_written_as_a_sequence_is_refused_as_the_schema_refuses_it() {
    let schema: Value = serde_json::from_str(&text(&["schema", "typed-reference"])).unwrap();
    let validator = jsonschema::draft202012::new(&schema).unwrap();
    let world = World::seeded();
    for (what, document) in [
        ("flow sequence", format!("[{PERSON}, [alice]]\n")),
        ("block sequence", format!("- {PERSON}\n- - alice\n")),
    ] {
        assert!(
            !validator.is_valid(&projection(&document)),
            "{what}: the schema accepts {document}"
        );
        let file = world.file("sequence.yaml", &document);
        let output = world.run(&["resolve", &file]);
        assert_fault(&output, &file, what);
    }
}

/// Deep nesting under the input limit, at the top, under keys and as block mappings: a fault
/// naming the document, never a stack overflow or an abort.
#[test]
fn very_deep_nesting_is_a_fault_not_a_crash() {
    let world = World::seeded();
    let top = format!("{}{}", "[".repeat(200_000), "]".repeat(200_000));
    let deep = format!("{}{}", "[".repeat(5_000), "]".repeat(5_000));
    for (what, document) in [
        ("deep top-level sequence", format!("{top}\n")),
        (
            "deep value under an unknown key",
            format!("type_id: {PERSON}\naliases: [alice]\nextra: {deep}\n"),
        ),
        (
            "deep value under aliases",
            format!("type_id: {PERSON}\naliases: {deep}\n"),
        ),
        (
            "deep block mappings",
            (0..1_000).fold(String::new(), |mut out, depth| {
                out.push_str(&" ".repeat(depth));
                out.push_str("k:\n");
                out
            }),
        ),
    ] {
        let file = world.file("deep.yaml", &document);
        let output = world.run(&["resolve", &file]);
        assert_fault(&output, &file, what);
    }
}

/// A stream of several documents is refused, whichever document holds the problem.
#[test]
fn several_documents_are_a_fault() {
    let world = World::seeded();
    let one = format!("type_id: {PERSON}\naliases: [alice]\n");
    for (what, document) in [
        ("two valid documents", format!("{one}---\n{one}")),
        (
            "an alias in the second document",
            format!("{one}---\n- &a x\n- *a\n"),
        ),
        (
            "a number in the second document",
            format!("{one}---\ntype_id: {PERSON}\naliases: [123]\n"),
        ),
        ("an empty first document", format!("---\n---\n{one}")),
    ] {
        let file = world.file("many.yaml", &document);
        let output = world.run(&["resolve", &file]);
        assert_fault(&output, &file, what);
    }
}

/// Empty input, a comment only, and bytes that are not UTF-8 on stdin are faults, exit 1.
#[test]
fn empty_and_unreadable_input_are_faults() {
    let world = World::seeded();
    for (what, document) in [("empty", ""), ("a comment only", "# nothing\n")] {
        let file = world.file("empty.yaml", document);
        assert_fault(&world.run(&["resolve", &file]), &file, what);
    }
    let mut bytes = format!("type_id: {PERSON}\naliases: [\"").into_bytes();
    bytes.extend_from_slice(&[0xff, 0xfe]);
    bytes.extend_from_slice(b"\"]\n");
    assert_fault(&world.stdin(&bytes), "-", "not UTF-8 on stdin");
    let missing = world.run(&["resolve", "missing.yaml"]);
    assert_eq!(missing.status.code(), Some(1), "missing file");
    assert!(String::from_utf8_lossy(&missing.stderr).contains("missing.yaml"));
}

/// `LIMIT` bounds the bytes read so that the work done on them is bounded. 60 000 nested flow
/// sequences under a key (120 KB, an eighth of `LIMIT`) must be refused about as fast as the
/// same nesting at the top level, which the reader refuses in about a second for 400 KB in a
/// debug build. Measured here: the loader's scan is quadratic in the nesting depth when the
/// nesting sits inside a block mapping, and `strict` loads the document a second time before
/// the decoder loads it again.
#[test]
fn deep_nesting_under_a_key_is_refused_in_bounded_time() {
    let world = World::seeded();
    let deep = format!("{}{}", "[".repeat(60_000), "]".repeat(60_000));
    let document = format!("type_id: {PERSON}\naliases: [alice]\nextra: {deep}\n");
    let file = world.file("slow.yaml", &document);
    let started = std::time::Instant::now();
    let output = world.run(&["resolve", &file]);
    let took = started.elapsed();
    assert_fault(&output, &file, "deep value under an unknown key");
    assert!(
        took < std::time::Duration::from_secs(5),
        "a {}-byte document took {took:?} to refuse",
        document.len()
    );
}

/// Forms the schema accepts are still accepted: a JSON document, a flow mapping, an anchor with
/// no alias, and a long alias under the limit.
#[test]
fn json_flow_and_anchored_forms_the_schema_accepts_still_resolve() {
    let schema: Value = serde_json::from_str(&text(&["schema", "typed-reference"])).unwrap();
    let validator = jsonschema::draft202012::new(&schema).unwrap();
    let world = World::seeded();
    let long = "y".repeat(900_000);
    for (what, document) in [
        (
            "JSON",
            format!("{{\"type_id\": \"{PERSON}\", \"aliases\": [\"alice\"]}}"),
        ),
        (
            "anchored mapping",
            format!("&m {{type_id: {PERSON}, aliases: &l [alice]}}\n"),
        ),
        (
            "long alias",
            format!("type_id: {PERSON}\naliases: [{long}]\n"),
        ),
    ] {
        assert!(validator.is_valid(&projection(&document)), "{what}");
        let file = world.file("ok.yaml", &document);
        let output = world.run(&["resolve", &file]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{what}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// The JSON `ekr resolve` prints decodes back through `ekr-integrate`'s own types, whose
/// `aliases` decoder is now strict: the correction must not break the verb's own output.
#[test]
fn the_printed_outcome_decodes_back_through_the_strict_aliases_decoder() {
    let world = World::seeded();
    let document = format!("type_id: {PERSON}\naliases: [nobody-by-this-name, \"a\\u00e9\"]\n");
    let file = world.file("new.yaml", &document);
    let output = world.run(&["resolve", &file]);
    assert_eq!(output.status.code(), Some(0));
    let printed = String::from_utf8(output.stdout).unwrap();
    let outcome: ResolutionOutcome = serde_json::from_str(&printed).unwrap();
    assert!(
        matches!(outcome, ResolutionOutcome::ProposeNew(_)),
        "{printed}"
    );
    let json: TypedReference = serde_json::from_str(&format!(
        "{{\"type_id\":\"{PERSON}\",\"aliases\":[\"a\\\"b\"]}}"
    ))
    .unwrap();
    assert_eq!(json.aliases, vec!["a\"b".to_owned()]);
    let value: TypedReference = serde_json::from_value(serde_json::json!({
        "type_id": PERSON, "aliases": ["x"]
    }))
    .unwrap();
    assert_eq!(value.aliases, vec!["x".to_owned()]);
}
