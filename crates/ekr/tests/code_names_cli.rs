//! `story:store-reading-code-names-no-contents`: `ekr code-names <file>...` reports every store
//! name a consumer's source files carry as a literal, with its file and line, driven through the
//! real binary on both providers.
//!
//! The store is `tests/fixtures/code-names/store`: node types Volume (properties leaf_count and
//! shelfmark) and Folio (property `name`), edge types BOUND_IN and CITES, and five nodes. Codex
//! Aurum carries the alias `Folio`, which is also a type's name; Ledger of Tides the alias Tidal
//! Register; Folio Alpha an alias that is Codex Aurum's id; Folio Beta the alias `Accepted`. The
//! sources under `tests/fixtures/code-names/sources` plant one name of each kind in `reader.ts`,
//! quote the property `name` and the alias `Accepted`, which are also runtime words and so are
//! reported flagged `runtime_word`, and Folio Alpha's alias, a store id and so exempt, and name the
//! rest as bare words; `generic.js` is store-reading code that names only runtime words.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];

fn manifest_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
}

fn fixture(name: &str) -> PathBuf {
    manifest_dir().join("tests/fixtures/code-names").join(name)
}

fn sources() -> (String, String) {
    (
        fixture("sources/reader.ts").display().to_string(),
        fixture("sources/generic.js").display().to_string(),
    )
}

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

/// The fixture store seeded on one provider in a directory of its own.
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
        let seed = fixture("store/seed.yaml").display().to_string();
        let output = world.run(&["seed", &seed]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        world
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            fixture("store/host.json").display().to_string(),
            "--store".to_owned(),
            self.store().display().to_string(),
            "--backend".to_owned(),
            self.backend.to_owned(),
        ];
        args.extend(verb.iter().map(|arg| (*arg).to_owned()));
        args
    }

    fn run(&self, verb: &[&str]) -> Output {
        ekr()
            .args(self.args(verb))
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
        assert!(output.stderr.is_empty());
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

/// Every file at or below `at`, by path, with its bytes.
fn tree_bytes(at: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(at: &Path, into: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, into);
            } else {
                into.insert(path.clone(), std::fs::read(&path).unwrap());
            }
        }
    }
    let mut found = BTreeMap::new();
    walk(at, &mut found);
    found
}

const VOLUME: &str = "00000000-0000-4000-8000-00000000b201";
const FOLIO: &str = "00000000-0000-4000-8000-00000000b202";
const NAME_PROPERTY: &str = "00000000-0000-4000-8000-00000000b803";

/// The `ekr.code-names/1` document the fixture sources answer at revision 0: `reader.ts` alone,
/// or with `generic.js`, whose path sorts first.
fn expected(reader: &str, generic: Option<&str>) -> Value {
    let finding = |file: &str, line: u64, column: u64, literal: &str, word: bool, names: Value| {
        json!({
            "file": file,
            "line": line,
            "column": column,
            "literal": literal,
            "runtime_word": word,
            "names": names,
        })
    };
    let node = |kind: &str, node: &str, type_id: &str, type_name: &str| {
        json!({
            "kind": kind,
            "id": format!("00000000-0000-4000-8000-00000000{node}"),
            "type_id": type_id,
            "type_name": type_name,
        })
    };
    let name_property = json!([{"kind": "Property", "id": NAME_PROPERTY}]);
    let mut findings = Vec::new();
    if let Some(generic) = generic {
        // `node["name"]`: the property `name`, which is also a runtime word.
        findings.push(finding(generic, 2, 47, "name", true, name_property.clone()));
    }
    findings.extend([
        finding(
            reader,
            2,
            16,
            "Volume",
            false,
            json!([{"kind": "NodeType", "id": VOLUME}]),
        ),
        finding(
            reader,
            3,
            15,
            "BOUND_IN",
            false,
            json!([{
                "kind": "EdgeType",
                "id": "00000000-0000-4000-8000-00000000b211",
            }]),
        ),
        finding(
            reader,
            4,
            12,
            "shelfmark",
            false,
            json!([{
                "kind": "Property",
                "id": "00000000-0000-4000-8000-00000000b802",
            }]),
        ),
        finding(
            reader,
            5,
            6,
            "Codex Aurum",
            false,
            json!([node("CanonicalName", "b301", VOLUME, "Volume"),]),
        ),
        finding(
            reader,
            6,
            6,
            "Tidal Register",
            false,
            json!([node("Alias", "b302", VOLUME, "Volume"),]),
        ),
        finding(
            reader,
            7,
            16,
            "Folio",
            false,
            json!([
                {"kind": "NodeType", "id": FOLIO},
                node("Alias", "b301", VOLUME, "Volume"),
            ]),
        ),
        // Line 8: `name` and Folio Beta's alias `Accepted` are store names and runtime words;
        // `aliases` is no store name, and Folio Alpha's alias is Codex Aurum's id: exempt.
        finding(reader, 8, 18, "name", true, name_property),
        finding(
            reader,
            8,
            37,
            "Accepted",
            true,
            json!([node("Alias", "b312", FOLIO, "Folio"),]),
        ),
    ]);
    let files = if generic.is_some() { 2 } else { 1 };
    let words = if generic.is_some() { 3 } else { 2 };
    json!({
        "meta": {
            "format": "ekr.code-names/1",
            "revision": 0,
            "files": files,
            // reader.ts: 12, one of them the text between the apostrophes of "consumer's" and
            // "store's" on line 1; generic.js: 2.
            "literals": if generic.is_some() { 14 } else { 12 },
            "exempt": 1,
            "findings": findings.len(),
            "runtime_word_findings": words,
        },
        "findings": findings,
    })
}

#[test]
fn ekr_code_names_reports_every_planted_name_with_its_file_and_line_on_both_providers() {
    let (reader, generic) = sources();
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let found = world.ok(&["code-names", &reader, &generic]);
        assert_eq!(found, expected(&reader, Some(&generic)), "{backend}");
        // The same files in another order, one named twice, answer the same document.
        assert_eq!(
            world.ok(&["code-names", &generic, &reader, &generic]),
            found,
            "{backend}"
        );
        // A revision named with --at answers as the head does; findings exit 0, never 1.
        assert_eq!(
            world.ok(&["code-names", "--at", "0", &reader]),
            expected(&reader, None),
            "{backend}"
        );
        // Code that names only runtime words has only flagged findings.
        let generic_only = world.ok(&["code-names", &generic]);
        assert_eq!(generic_only["meta"]["findings"], 1, "{backend}");
        assert_eq!(
            generic_only["meta"]["runtime_word_findings"], 1,
            "{backend}"
        );
        assert_eq!(generic_only["meta"]["exempt"], 0, "{backend}");
        assert_eq!(
            generic_only["findings"][0]["runtime_word"], true,
            "{backend}"
        );
    }
}

#[test]
fn ekr_code_names_writes_nothing_to_the_store_or_the_sources() {
    let (reader, generic) = sources();
    let sources_before = tree_bytes(&fixture("sources"));
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let before = tree_bytes(world.directory.path());
        world.ok(&["code-names", &reader, &generic]);
        assert_eq!(
            world
                .run(&["code-names", "--at", "7", &reader])
                .status
                .code(),
            Some(2)
        );
        assert!(
            tree_bytes(world.directory.path()) == before,
            "{backend}: ekr code-names changed bytes under the store's directory"
        );
    }
    assert!(
        tree_bytes(&fixture("sources")) == sources_before,
        "ekr code-names changed the sources"
    );
}

#[test]
fn ekr_code_names_refuses_what_it_cannot_read() {
    let world = World::seeded("file");
    // No file is a usage error.
    let none = world.run(&["code-names"]);
    assert_eq!(none.status.code(), Some(2));
    assert!(none.stdout.is_empty());
    // A revision the store does not hold is the views refusal, by name.
    let (reader, _) = sources();
    let beyond = world.run(&["code-names", "--at", "7", &reader]);
    assert_eq!(beyond.status.code(), Some(2));
    assert!(beyond.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&beyond.stderr).contains("ekr.views.RevisionNotFound"),
        "{}",
        String::from_utf8_lossy(&beyond.stderr)
    );
    // A file that is missing, or is not UTF-8 text, is a fault naming it: exit 1.
    let missing = world.directory.path().join("missing.ts");
    let binary = world.directory.path().join("binary.ts");
    std::fs::write(&binary, [0xff, 0xfe, b'"', b'x', b'"']).unwrap();
    for path in [&missing, &binary] {
        let path = path.display().to_string();
        let output = world.run(&["code-names", &reader, &path]);
        assert_eq!(output.status.code(), Some(1), "{path}");
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(&path),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    // An unseeded store is refused by name.
    let empty = World {
        directory: tempfile::tempdir().unwrap(),
        backend: "file",
    };
    let unseeded = empty.run(&["code-names", &reader]);
    assert_ne!(unseeded.status.code(), Some(0));
    assert!(unseeded.stdout.is_empty());
}

#[test]
fn ekr_session_serves_code_names_as_the_one_shot_verb_answers() {
    let (reader, generic) = sources();
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let one_shot = world.ok(&["code-names", &reader, &generic]);
        let mut child = ekr()
            .args(world.args(&["session"]))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let request = json!({"argv": ["code-names", reader, generic]});
        child
            .stdin
            .take()
            .unwrap()
            .write_all(format!("{request}\n").as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{backend}");
        let answer: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(answer["exit"], 0, "{backend}: {answer}");
        assert_eq!(answer["stdout"], one_shot, "{backend}");
    }
}
