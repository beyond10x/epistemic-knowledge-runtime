//! `story:store-reading-code-names-no-contents`: ekr.code-names/1, the store's names found as
//! literals in a consumer's source files, held against `systems/ekr/domains/views.yaml`
//! (`ekr.views.FindCodeNames`): what a literal is, which names are found, which are exempt — the
//! runtime's own vocabulary and the store's ids — and the document's determinism.

mod support;

use std::collections::BTreeSet;

use ekr_core::RevisionNumber;
use ekr_views::{Literal, ProjectError, SourceText};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use support::fixtures::{self, Fixture, Provider};

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn source(path: &str, text: &str) -> SourceText {
    SourceText {
        path: path.to_owned(),
        text: text.to_owned(),
    }
}

/// `(line, column, text)` of every literal `text` holds.
fn literals(text: &str) -> Vec<(u64, u64, String)> {
    let found: Vec<Literal> = ekr_views::literals(text);
    found
        .into_iter()
        .map(|literal| (literal.line, literal.column, literal.text))
        .collect()
}

fn triple(line: u64, column: u64, text: &str) -> (u64, u64, String) {
    (line, column, text.to_owned())
}

/// Every field name, enum variant, union tag and union variant name the ESS domains declare.
fn declared_vocabulary() -> BTreeSet<String> {
    use serde_yaml_ng::Value as Yaml;
    fn names_of(list: &Yaml, into: &mut BTreeSet<String>) {
        for entry in list.as_sequence().into_iter().flatten() {
            if let Some(name) = entry["name"].as_str() {
                into.insert(name.to_owned());
            }
        }
    }
    let directory = support::workspace_root().join("systems/ekr/domains");
    let mut files: Vec<_> = std::fs::read_dir(&directory)
        .expect("the domains directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "yaml")
        })
        .collect();
    files.sort();
    assert!(files.len() >= 7, "{files:?}");
    let mut vocabulary = BTreeSet::new();
    for file in files {
        let domain: Yaml =
            serde_yaml_ng::from_str(&std::fs::read_to_string(&file).expect("a domain file"))
                .expect("a YAML domain");
        for declared in domain["types"].as_sequence().into_iter().flatten() {
            names_of(&declared["fields"], &mut vocabulary);
            if let Some(tag) = declared["tag"].as_str() {
                vocabulary.insert(tag.to_owned());
            }
            match &declared["variants"] {
                Yaml::Sequence(variants) => vocabulary.extend(
                    variants
                        .iter()
                        .map(|variant| variant.as_str().expect("a variant name").to_owned()),
                ),
                Yaml::Mapping(variants) => vocabulary.extend(
                    variants
                        .keys()
                        .map(|variant| variant.as_str().expect("a variant name").to_owned()),
                ),
                _ => {}
            }
        }
        for errors_or_events in ["errors", "events"] {
            for declared in domain[errors_or_events].as_sequence().into_iter().flatten() {
                names_of(&declared["fields"], &mut vocabulary);
            }
        }
        for command in domain["commands"].as_sequence().into_iter().flatten() {
            names_of(&command["input"], &mut vocabulary);
            names_of(&command["response"], &mut vocabulary);
        }
    }
    vocabulary
}

#[test]
fn the_runtime_vocabulary_is_every_name_the_ess_domains_declare() {
    let listed: Vec<&str> = ekr_views::RUNTIME_VOCABULARY.to_vec();
    let mut sorted = listed.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(listed, sorted, "RUNTIME_VOCABULARY ascends with no repeat");
    let listed: BTreeSet<String> = listed.into_iter().map(str::to_owned).collect();
    let declared = declared_vocabulary();
    let missing: Vec<&String> = declared.difference(&listed).collect();
    let extra: Vec<&String> = listed.difference(&declared).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "RUNTIME_VOCABULARY is not what systems/ekr/domains declares; missing {missing:?}, \
         not declared {extra:?}"
    );
    for word in [
        "name",
        "aliases",
        "String",
        "Accepted",
        "CreateNode",
        "ekr.graph-projection/1",
        "ekr.code-names/1",
        "kind",
    ] {
        assert!(listed.contains(word), "{word}");
    }
}

#[test]
fn a_literal_is_the_text_between_two_quotes_of_one_character_on_one_line() {
    // Each of the three quote characters, and the text between, raw.
    assert_eq!(
        literals("a(\"one\", 'two', `three`)"),
        vec![
            triple(1, 3, "one"),
            triple(1, 10, "two"),
            triple(1, 17, "three")
        ]
    );
    // Paired left to right, scanning on after the closing quote.
    assert_eq!(
        literals("\"a\" + \"b\""),
        vec![triple(1, 1, "a"), triple(1, 7, "b")]
    );
    // A backslash escapes the next character, and no escape is decoded.
    assert_eq!(
        literals(r#"x = "say \"hi\"" + "\\";"#),
        vec![triple(1, 5, r#"say \"hi\""#), triple(1, 20, r"\\")]
    );
    // A quote with no partner on its line opens nothing; no literal spans a line.
    assert_eq!(
        literals("\"open\nclose\" \"shut\""),
        vec![triple(2, 6, " ")]
    );
    // The characters are scanned independently: a literal may lie inside another.
    assert_eq!(
        literals("// don't read \"Name\" here"),
        vec![triple(1, 15, "Name")]
    );
    assert_eq!(
        literals("'a \"b\" c'"),
        vec![triple(1, 1, "a \"b\" c"), triple(1, 4, "b")]
    );
    // Identifiers are no literal; an empty literal is one.
    assert_eq!(literals("let Name = Name;"), Vec::new());
    assert_eq!(literals("f(\"\")"), vec![triple(1, 3, "")]);
    // Columns count Unicode scalar values, lines count `\n` only.
    assert_eq!(
        literals("é \"x\"\r\n\"y\""),
        vec![triple(1, 3, "x"), triple(2, 1, "y")]
    );
}

/// The `edge-assertion` store: node type `Subject` with property `label`, edge type `links` with
/// property `weight`, nodes `alpha` and `beta`, each with aliases `<name>-alias-b` and
/// `<name>-alias-a`.
fn built(provider: Provider) -> (tempfile::TempDir, ekr_kernel::Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    Fixture::EdgeAssertion.build(&runtime);
    (work, runtime)
}

fn document(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("JSON")
}

/// One name of each kind on lines 1 to 4. `links` (line 2) and `weight` (line 3) are store names
/// that are also the runtime's vocabulary, so they are exempt, not found.
const PLANTED: &str = "\
const kind = \"Subject\";
const relation = 'links';
read(node, \"label\", `weight`);
find(\"alpha\") || find('beta-alias-a');
// A bare word is no literal: Subject alpha links
const generic = [\"name\", \"aliases\", \"store\", \"String\"];
const ids = [\"00000000-0000-4000-8000-000000000110\", \"00000000-0000-4000-8000-000000000100\"];
const near = [\"subject\", \"alpha \", \"Alpha\", \"alpha-alias\"];
";

#[test]
fn every_planted_name_is_found_with_its_file_and_line_and_no_id_or_vocabulary_is() {
    for provider in [Provider::File, Provider::Sqlite] {
        let (_work, runtime) = built(provider);
        let events = runtime.published_events().expect("the provider log").len();
        let answer = ekr_views::find_code_names(
            &runtime,
            None,
            &[
                source("src/reader.ts", PLANTED),
                source("src/clean.ts", "const kind = node.type_id;\n"),
            ],
        )
        .expect("an answer");
        assert_eq!(
            runtime.published_events().expect("the provider log").len(),
            events,
            "the read wrote nothing"
        );
        let value = document(&answer.bytes);
        let subject = uuid(0x100);
        let finding = |line: u64, column: u64, literal: &str, names: Value| {
            json!({
                "file": "src/reader.ts",
                "line": line,
                "column": column,
                "literal": literal,
                "names": names,
            })
        };
        let node_name = |kind: &str, node: u64| {
            json!([{
                "kind": kind,
                "id": uuid(node),
                "type_id": subject,
                "type_name": "Subject",
            }])
        };
        let expected = json!({
            "meta": {
                "format": "ekr.code-names/1",
                "revision": 0,
                "files": 2,
                "literals": 16,
                "exempt": 2,
                "findings": 4,
            },
            "findings": [
                finding(1, 14, "Subject", json!([{"kind": "NodeType", "id": subject}])),
                finding(3, 12, "label", json!([{"kind": "Property", "id": uuid(0x101)}])),
                finding(4, 6, "alpha", node_name("CanonicalName", 0x110)),
                finding(4, 23, "beta-alias-a", node_name("Alias", 0x111)),
            ],
        });
        assert_eq!(value, expected, "{provider:?}");
        assert_eq!(value["meta"]["format"], ekr_views::CODE_NAMES_FORMAT);
        let summary = &answer.summary;
        assert_eq!(
            (
                summary.revision,
                summary.files,
                summary.literals,
                summary.exempt,
                summary.findings
            ),
            (0, 2, 16, 2, 4)
        );
        assert_eq!(
            (
                summary.node_types,
                summary.edge_types,
                summary.properties,
                summary.canonical_names,
                summary.aliases
            ),
            (1, 0, 1, 1, 1)
        );
        assert_eq!(summary.first_file.as_deref(), Some("src/reader.ts"));
        assert_eq!(summary.first_line, Some(1));
        assert_eq!(
            summary.code_names_hash,
            hex::encode(Sha256::digest(&answer.bytes))
        );
    }
}

#[test]
fn the_answer_is_byte_identical_whatever_the_order_the_sources_came_in() {
    let (_work, runtime) = built(Provider::File);
    let one = source("b.ts", "f(\"alpha\")\n");
    let two = source("a.ts", "g('Subject')\n");
    let forward =
        ekr_views::find_code_names(&runtime, None, &[one.clone(), two.clone()]).expect("an answer");
    let backward =
        ekr_views::find_code_names(&runtime, None, &[two.clone(), one.clone(), two.clone()])
            .expect("an answer");
    assert_eq!(forward, backward);
    // The pure half answers the same from the loaded revision, on every call.
    let loaded = ekr_views::load(&runtime, None).expect("the head loads");
    for _ in 0..2 {
        assert_eq!(
            ekr_views::code_names(&loaded, &[one.clone(), two.clone()]).expect("an answer"),
            forward
        );
    }
    let value = document(&forward.bytes);
    assert_eq!(value["meta"]["files"], 2);
    assert_eq!(value["findings"][0]["file"], "a.ts");
    assert_eq!(value["findings"][1]["file"], "b.ts");
    let text = String::from_utf8(forward.bytes).expect("UTF-8");
    assert!(
        text.starts_with("{\"meta\":{\"format\":\"ekr.code-names/1\",\"revision\":0,"),
        "{text}"
    );
    assert!(!text.contains(char::is_whitespace), "{text}");
}

#[test]
fn no_source_answers_no_finding_and_an_unknown_revision_is_refused() {
    let (_work, runtime) = built(Provider::File);
    let answer =
        ekr_views::find_code_names(&runtime, Some(RevisionNumber::new(0)), &[]).expect("an answer");
    assert_eq!(
        document(&answer.bytes),
        json!({
            "meta": {
                "format": "ekr.code-names/1",
                "revision": 0,
                "files": 0,
                "literals": 0,
                "exempt": 0,
                "findings": 0,
            },
            "findings": [],
        })
    );
    assert_eq!(answer.summary.first_file, None);
    assert_eq!(answer.summary.first_line, None);
    assert!(matches!(
        ekr_views::find_code_names(&runtime, Some(RevisionNumber::new(9)), &[]),
        Err(ProjectError::RevisionNotFound { requested, head })
            if requested == RevisionNumber::new(9) && head == RevisionNumber::new(0)
    ));
    let work = tempfile::tempdir().expect("work directory");
    let unseeded = fixtures::open(work.path(), Provider::File);
    assert!(matches!(
        ekr_views::find_code_names(&unseeded, None, &[]),
        Err(ProjectError::NotSeeded { requested: None })
    ));
}
