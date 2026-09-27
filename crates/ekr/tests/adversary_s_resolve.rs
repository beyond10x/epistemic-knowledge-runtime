//! Adversary cases for `story:ekr-resolve-verb` (`ekr resolve`).
//!
//! * The reader bounds a typed-reference document to `LIMIT` bytes read, but a YAML anchor
//!   repeated by aliases decodes to many times that: the bound is on bytes, not on what decodes.
//! * `ekr schema typed-reference` says it "validates [the document] read as YAML and written as
//!   JSON", so a document the reader decodes and the schema refuses (or the reverse) is a
//!   disagreement between the schema and the reader.
//! * The over-limit refusal has no case in the unit's suite: a document over the limit whose
//!   truncated prefix is itself a typed reference must still be refused.

use std::path::PathBuf;
use std::process::Output;

use ekr_integrate::TypedReference;
use serde_json::Value;

const PERSON: &str = "00000000-0000-4000-8000-000000000201";
/// `crates/ekr/src/cli/resolve.rs` `LIMIT`.
const LIMIT: usize = 1 << 20;

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

/// A file-provider store seeded from the example seed, in its own directory.
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

    fn run(&self, verb: &[&str]) -> Output {
        ekr()
            .current_dir(self.directory.path())
            .arg("--host")
            .arg(self.directory.path().join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", "file"])
            .args(verb)
            .output()
            .unwrap()
    }
}

/// A 100 000-byte anchored alias repeated 20 times: about 100 KB read, about 2 MB decoded, twice
/// `LIMIT`. At the 1 MiB input the reader admits, the same shape decodes to tens of gigabytes.
/// The document must be refused as a fault naming it (exit 1), as an over-limit document is.
#[test]
fn an_anchor_repeated_by_aliases_does_not_decode_past_the_input_limit() {
    let world = World::seeded();
    let long = "x".repeat(100_000);
    let mut document = format!("type_id: {PERSON}\naliases:\n- &a {long}\n");
    for _ in 0..20 {
        document.push_str("- *a\n");
    }
    assert!(document.len() < LIMIT);
    let file = world.file("bomb.yaml", &document);
    let output = world.run(&["resolve", &file]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a {}-byte document decoding to {} bytes of aliases was resolved (stdout {} bytes): {stderr}",
        document.len(),
        long.len() * 21,
        output.stdout.len()
    );
    assert!(
        stderr.starts_with("ekr: typed reference bomb.yaml: "),
        "{stderr}"
    );
}

/// For each document, the reader's answer (`serde_yaml_ng::from_str::<TypedReference>`, as
/// `resolve::read` calls it) equals the schema's on the document's JSON projection, which is what
/// the schema's own description says it validates.
#[test]
fn the_schema_and_the_reader_agree_on_scalar_aliases() {
    let schema: Value = serde_json::from_str(&text(&["schema", "typed-reference"])).unwrap();
    let validator = jsonschema::draft202012::new(&schema).unwrap();
    let mut disagreements = Vec::new();
    for (what, aliases) in [
        ("a leading-zero numeric alias", "- 007\n"),
        ("a numeric alias", "- 123\n"),
        ("a null alias", "- ~\n"),
        ("a tagged alias", "- !Name acme\n"),
        ("aliases written as null", ""),
    ] {
        let document = format!("type_id: {PERSON}\naliases:\n{aliases}");
        let decoded = serde_yaml_ng::from_str::<TypedReference>(&document);
        let projection: serde_yaml_ng::Value = serde_yaml_ng::from_str(&document).unwrap();
        let instance = serde_json::to_value(projection).unwrap();
        let valid = validator.is_valid(&instance);
        if decoded.is_ok() != valid {
            disagreements.push(format!(
                "{what}: reader {} ({:?}), schema {} on {instance}",
                if decoded.is_ok() {
                    "accepts"
                } else {
                    "refuses"
                },
                decoded.map(|reference| reference.aliases),
                if valid { "accepts" } else { "refuses" },
            ));
        }
    }
    assert!(
        disagreements.is_empty(),
        "the typed-reference schema and its reader disagree:\n{}",
        disagreements.join("\n")
    );
}

/// A document one byte over `LIMIT` whose first `LIMIT + 1` bytes are a typed reference by
/// themselves: without the size check the reader would resolve the truncated prefix. Refused,
/// exit 1, naming the limit.
#[test]
fn a_document_over_the_limit_is_refused_even_when_its_prefix_decodes() {
    let world = World::seeded();
    let head = format!("type_id: {PERSON}\naliases:\n- ");
    let mut document = head.clone();
    document.push_str(&"a".repeat(LIMIT + 1 - head.len()));
    document.push_str("tail\n");
    let prefix = &document[..=LIMIT];
    assert!(serde_yaml_ng::from_str::<TypedReference>(prefix).is_ok());
    let file = world.file("long.yaml", &document);
    let output = world.run(&["resolve", &file]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(
        stderr.starts_with(&format!(
            "ekr: typed reference long.yaml: over {LIMIT} bytes"
        )),
        "{stderr}"
    );
}
