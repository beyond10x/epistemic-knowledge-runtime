//! `story:names-inline-in-read-answers` (ekr-mcp-fixes-20261015 #3): `ekr explain --format
//! ekr.explanation/3` and `ekr resolve --format ekr.resolution/1` carry `names`, the name of every
//! type and property id the document holds, read from the ontology of the revision read; without
//! `--format`, and with the earlier format named, each prints the bytes it printed before
//! (`kernel.yaml`, `ekr.kernel.ExplanationV3`; `integrate.yaml`, `ekr.integrate.ResolutionV1`).
//!
//! Every case runs the built binary on both providers, under the example host and seed: Person
//! (…0201), Organization (…0202, with the property legal_name …0801) and the edge type CEO_OF
//! (…0203); the seeded assertion …0510 states that Alice is CEO_OF Acme.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde_json::{json, Map, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const PERSON: &str = "00000000-0000-4000-8000-000000000201";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const CEO_OF: &str = "00000000-0000-4000-8000-000000000203";
const LEGAL_NAME: &str = "00000000-0000-4000-8000-000000000801";
const ACME: &str = "00000000-0000-4000-8000-000000000303";
const SEEDED_ASSERTION: &str = "00000000-0000-4000-8000-000000000510";

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
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

/// One provider in its own directory under the example host, seeded from the example seed.
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
        world.ok(&["seed", "seed.yaml"]);
        world
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

    fn run(&self, verb: &[&str]) -> Output {
        ekr()
            .current_dir(self.directory.path())
            .arg("--host")
            .arg(self.directory.path().join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(verb)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    /// A verb that must exit 0: its stdout, as printed.
    fn ok(&self, verb: &[&str]) -> String {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
}

/// The example ontology's ids, with their names.
fn declared() -> Map<String, Value> {
    [
        (PERSON, "Person"),
        (ORGANIZATION, "Organization"),
        (CEO_OF, "CEO_OF"),
        (LEGAL_NAME, "legal_name"),
    ]
    .into_iter()
    .map(|(id, name)| (id.to_owned(), json!(name)))
    .collect()
}

/// Every string of `value`, as a value or as an object key.
fn strings(value: &Value, into: &mut Vec<String>) {
    match value {
        Value::String(text) => into.push(text.clone()),
        Value::Array(items) => items.iter().for_each(|item| strings(item, into)),
        Value::Object(fields) => {
            for (key, item) in fields {
                into.push(key.clone());
                strings(item, into);
            }
        }
        _ => {}
    }
}

/// `names` is exactly the declared ids `document` holds, each with its declared name, keys in
/// id order.
fn holds_exactly_the_names_of(names: &Value, document: &Value, what: &str) {
    let mut held = Vec::new();
    strings(document, &mut held);
    let declared = declared();
    let expected: Map<String, Value> = declared
        .iter()
        .filter(|(id, _)| held.contains(id))
        .map(|(id, name)| (id.clone(), name.clone()))
        .collect();
    assert_eq!(names, &Value::Object(expected), "{what}");
    let keys: Vec<&String> = names.as_object().unwrap().keys().collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "{what}: keys in id order");
}

#[test]
fn explain_with_format_v3_names_the_ids_it_holds_and_without_it_prints_v2_unchanged() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let plain = world.ok(&["explain", SEEDED_ASSERTION]);
        assert_eq!(
            world.ok(&["explain", SEEDED_ASSERTION, "--format", "ekr.explanation/2"]),
            plain,
            "{backend}: /2 named is the default, byte for byte"
        );
        let v2: Value = serde_json::from_str(&plain).unwrap();
        assert_eq!(v2["format"], "ekr.explanation/2", "{v2}");
        assert!(
            v2.get("names").is_none(),
            "{backend}: /2 has no names: {v2}"
        );

        for documents in [false, true] {
            let mut verb = vec!["explain", SEEDED_ASSERTION];
            if documents {
                verb.push("--documents");
            }
            let before: Value = serde_json::from_str(&world.ok(&verb)).unwrap();
            verb.extend(["--format", "ekr.explanation/3"]);
            let named: Value = serde_json::from_str(&world.ok(&verb)).unwrap();
            assert_eq!(named["format"], "ekr.explanation/3", "{backend}: {named}");
            assert_eq!(
                named["names"][CEO_OF], "CEO_OF",
                "{backend}: the relation the assertion states: {named}"
            );
            holds_exactly_the_names_of(
                &named["names"],
                &before,
                &format!("{backend} documents={documents}"),
            );
            // The seeded assertion cites the one evidence entry its chain lists, so the scope
            // `cited` omits none (ekr-mcp-fixes-20261015 #7).
            assert_eq!(named["scope"], "cited", "{backend}: {named}");
            assert_eq!(named["omitted_evidence"], 0, "{backend}: {named}");
            // Everything else is /2's, but that /3 answers no base64 `payload` beside its `text`.
            let mut rest = named.clone();
            let fields = rest.as_object_mut().unwrap();
            for added in ["names", "scope", "omitted_evidence"] {
                fields.remove(added);
            }
            fields.insert("format".to_owned(), json!("ekr.explanation/2"));
            let mut expected = before.clone();
            for link in expected["links"].as_array_mut().unwrap() {
                if link["kind"] == "Evidence" && link.get("text").is_some() {
                    link.as_object_mut().unwrap().remove("payload");
                }
            }
            assert_eq!(rest, expected, "{backend} documents={documents}");
        }

        for verb in [
            &["explain", SEEDED_ASSERTION, "--format", "ekr.explanation/4"][..],
            &["explain", SEEDED_ASSERTION, "--scope", "all"][..],
            &[
                "explain",
                SEEDED_ASSERTION,
                "--format",
                "ekr.explanation/2",
                "--scope",
                "cited",
            ][..],
        ] {
            let refused = world.run(verb);
            assert_eq!(
                refused.status.code(),
                Some(2),
                "{backend} {verb:?}: a usage error"
            );
            assert!(refused.stdout.is_empty(), "{backend} {verb:?}");
        }
    }
}

#[test]
fn resolve_with_format_v1_names_the_type_its_outcome_holds_and_without_it_prints_the_outcome() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        // Globex is no node of the seed: ProposeNew, which holds the type. Acme is, by its
        // canonical name; whichever outcome it is, its names are the type ids it holds.
        for (reference, proposed) in [
            (json!({"type_id": ORGANIZATION, "aliases": ["Acme"]}), None),
            (
                json!({"type_id": ORGANIZATION, "aliases": ["Globex"]}),
                Some(true),
            ),
        ] {
            world.file("reference.json", &reference.to_string());
            let bare = world.ok(&["resolve", "reference.json"]);
            assert_eq!(
                world.ok(&["resolve", "reference.json", "--format", "outcome"]),
                bare,
                "{backend}: the bare outcome named is the default, byte for byte"
            );
            let outcome: Value = serde_json::from_str(&bare).unwrap();
            let kind = outcome["kind"].as_str().unwrap().to_owned();
            if let Some(proposed) = proposed {
                assert_eq!(kind == "ProposeNew", proposed, "{backend}: {outcome}");
            }
            let names = if kind == "ProposeNew" {
                json!({ORGANIZATION: "Organization"})
            } else {
                json!({})
            };
            let named: Value = serde_json::from_str(&world.ok(&[
                "resolve",
                "reference.json",
                "--format",
                "ekr.resolution/1",
            ]))
            .unwrap();
            assert_eq!(
                named,
                json!({"format": "ekr.resolution/1", "names": names, "outcome": outcome}),
                "{backend}: {reference}"
            );
            if kind == "Resolved" {
                assert_eq!(outcome["node_id"], ACME, "{backend}");
            }
            holds_exactly_the_names_of(&named["names"], &outcome, backend);
        }
        // A refused reference is refused whatever the format.
        world.file(
            "reference.json",
            &json!({"type_id": CEO_OF, "aliases": ["Acme"]}).to_string(),
        );
        for format in [None, Some("ekr.resolution/1")] {
            let mut verb = vec!["resolve", "reference.json"];
            if let Some(format) = format {
                verb.extend(["--format", format]);
            }
            let refused = world.run(&verb);
            assert_eq!(refused.status.code(), Some(2), "{backend} {format:?}");
            assert!(
                String::from_utf8_lossy(&refused.stderr).contains("reference-type-undeclared"),
                "{backend} {format:?}: {}",
                String::from_utf8_lossy(&refused.stderr)
            );
        }
    }
}
