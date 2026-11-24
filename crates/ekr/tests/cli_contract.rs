//! `story:ekr-cli-contract-fixes`: the CLI's schema read and the output formats its bootstrap
//! verbs print, held to the binary.
//!
//! * `ekr ontology` prints each node type's lifecycle and named operations, with their arguments
//!   and transitions, and each edge type's `inverse`, `symmetric` and `transitive`: everything
//!   the seed declared, on both providers (review of 0.0.42, finding 33).
//! * `README.md` and `docs/cli.md` name the format each bootstrap verb prints, as the binary
//!   prints it, so a client picks the right decoder (finding 44).

use std::path::PathBuf;
use std::process::Command;

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];

/// Every format `ekr example` prints.
const EXAMPLE_FORMATS: [&str; 7] = [
    "ekr.transaction-document/2",
    "ekr.transaction-document/1",
    "schema-change",
    "ekr-seed/2",
    "ekr.cli-host/1",
    "typed-reference",
    "ekr.extraction-document/1",
];

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

/// Exit 0 and stdout as text.
fn stdout(command: &mut Command) -> String {
    let output = command.output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{command:?}: stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn text(args: &[&str]) -> String {
    stdout(ekr().args(args))
}

/// The workspace root, read per process (AGENTS.md: never `env!("CARGO_MANIFEST_DIR")`).
fn read(relative: &str) -> String {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    std::fs::read_to_string(root.join(relative)).unwrap()
}

/// The example seed with a lifecycle on `Person` and an operation that moves it: the example seed
/// itself declares `Organization.record_review(note)` and no lifecycle.
fn seed_with_a_lifecycle() -> String {
    let seed = text(&["example", "ekr-seed/2"]);
    let person = "    lifecycle: null\n    operations: {}\n";
    assert!(seed.contains(person), "the example seed's Person changed");
    let seed = seed.replacen(
        person,
        "    lifecycle:\n      initial: employed\n      states: [employed, retired]\n      \
         transitions:\n      - {from: employed, to: retired}\n    operations:\n      retire:\n        \
         name: retire\n        arguments:\n          on:\n            value_kind: Timestamp\n        \
         preconditions: []\n        transition: {from: employed, to: retired}\n        emits: []\n",
        1,
    );
    // Alice and Bob, the seed's two people, come first and start in the initial state.
    seed.replacen("type_state: null", "type_state: employed", 2)
}

/// The YAML seed's ontology section, as JSON.
fn declared(seed: &str) -> Value {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(seed).unwrap();
    serde_json::to_value(&yaml["ontology"]).unwrap()
}

fn by_name<'a>(types: &'a Value, name: &str) -> &'a Value {
    types
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == name)
        .unwrap_or_else(|| panic!("{name} in {types}"))
}

/// Finding 33: the seed's `Organization.record_review(note)`, a lifecycle and the operation that
/// moves it, and every edge type's flags are in `ekr ontology`, as declared, on both providers.
#[test]
fn the_ontology_read_prints_lifecycles_operations_and_edge_flags_as_declared() {
    let seed = seed_with_a_lifecycle();
    let declared = declared(&seed);
    for backend in BACKENDS {
        let directory = tempfile::tempdir().unwrap();
        let host = directory.path().join("host.json");
        let store = directory.path().join("store");
        let seed_path = directory.path().join("seed.yaml");
        std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
        std::fs::write(&seed_path, &seed).unwrap();
        let run = |args: &[&str]| -> Value {
            let mut command = ekr();
            command
                .arg("--host")
                .arg(&host)
                .arg("--store")
                .arg(&store)
                .args(["--backend", backend])
                .args(args);
            serde_json::from_str(&stdout(&mut command)).unwrap()
        };
        run(&["seed", seed_path.to_str().unwrap()]);
        let printed = run(&["ontology"]);

        let organization = by_name(&printed["node_types"], "Organization");
        assert_eq!(
            organization["operations"],
            json!({"record_review": {
                "name": "record_review",
                "arguments": {"note": {"value_kind": "String"}},
                "preconditions": [],
                "transition": null,
                "emits": [],
            }}),
            "{backend}: {organization}"
        );
        assert_eq!(organization["lifecycle"], Value::Null, "{backend}");
        let person = by_name(&printed["node_types"], "Person");
        assert_eq!(
            person["lifecycle"],
            json!({
                "initial": "employed",
                "states": ["employed", "retired"],
                "transitions": [{"from": "employed", "to": "retired"}],
            }),
            "{backend}: {person}"
        );

        for node_type in declared["node_types"].as_array().unwrap() {
            let name = node_type["name"].as_str().unwrap();
            let read = by_name(&printed["node_types"], name);
            for key in ["lifecycle", "operations"] {
                assert_eq!(read[key], node_type[key], "{backend}: {name}.{key}");
            }
        }
        for edge_type in declared["edge_types"].as_array().unwrap() {
            let name = edge_type["name"].as_str().unwrap();
            let read = by_name(&printed["edge_types"], name);
            for key in ["symmetric", "transitive"] {
                assert_eq!(read[key], edge_type[key], "{backend}: {name}.{key}");
            }
            assert_eq!(
                read["inverse"].get("id").unwrap_or(&Value::Null),
                &edge_type["inverse"],
                "{backend}: {name}.inverse"
            );
        }
    }
}

/// How a client decodes what a verb printed.
fn decoder(printed: &str) -> &'static str {
    if serde_json::from_str::<Value>(printed).is_ok() {
        "JSON"
    } else if serde_yaml_ng::from_str::<serde_yaml_ng::Value>(printed)
        .is_ok_and(|value| value.is_mapping())
    {
        "YAML"
    } else {
        "text"
    }
}

/// Finding 44: `ekr guide` and `ekr operations` print text and `ekr example` prints each format
/// in its own syntax; `README.md`'s introduction and `docs/cli.md` § Exit codes and output say so,
/// and the introduction no longer says every verb prints JSON.
#[test]
fn the_readme_and_the_page_name_the_format_each_bootstrap_verb_prints() {
    for (verb, expected) in [
        (vec!["guide"], "text"),
        (vec!["operations"], "text"),
        (vec!["operations", "AddAssertion"], "text"),
        (vec!["schema", "ekr-seed/2"], "JSON"),
        (vec!["mint", "node"], "JSON"),
    ] {
        assert_eq!(decoder(&text(&verb)), expected, "ekr {verb:?}");
    }
    let mut json_examples = Vec::new();
    for format in EXAMPLE_FORMATS {
        match decoder(&text(&["example", format])) {
            "JSON" => json_examples.push(format!("`{format}`")),
            "YAML" => {}
            other => panic!("ekr example {format} prints {other}"),
        }
    }
    let examples = format!(
        "JSON for {}, YAML for every other",
        json_examples.join(", ")
    );

    let readme = read("README.md");
    let introduction = readme.split("\n## ").next().unwrap().replace('\n', " ");
    assert!(
        !introduction.contains("every verb prints JSON"),
        "README.md's introduction says every verb prints JSON"
    );
    for claim in [
        "`ekr guide` and `ekr operations` print text",
        &examples,
        "`ekr schema` and `ekr mint` print JSON",
    ] {
        assert!(
            introduction.contains(claim),
            "README.md's introduction does not say {claim:?}"
        );
    }

    let page = read("docs/cli.md");
    let output = page
        .split_once("\n## Exit codes and output\n")
        .unwrap()
        .1
        .split_once("\n## ")
        .unwrap()
        .0
        .replace('\n', " ");
    for claim in ["`guide` and `operations` print text", &examples] {
        assert!(
            output.contains(claim),
            "docs/cli.md § Exit codes and output does not say {claim:?}"
        );
    }
}
