//! Adversarial cases holding `ekr schema ekr.extraction-document/1` to
//! `ekr_integrate::read_extraction` (`story:extraction-document-applies-to-a-store`, wave
//! extract-06 unit D), on edits of `ekr example ekr.extraction-document/1` that
//! `crates/ekr/tests/schema_cli.rs` does not try: tags spelled other ways, and names the
//! document repeats. Each document is validated the way that file validates one, as its JSON
//! projection, and read against the example seed's ontology.

use std::process::Command;

use serde_json::Value;

const EXTRACTION: &str = "ekr.extraction-document/1";

fn text(args: &[&str]) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
        command.env_remove(var);
    }
    let output = command.args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn store() -> ekr_ontology::Ontology {
    let seed = ekr_kernel::SeedDocument::from_yaml(&text(&["example", "ekr-seed/2"])).unwrap();
    ekr_ontology::Ontology::load(seed.ontology).unwrap()
}

fn edit(document: &str, from: &str, to: &str) -> String {
    assert!(document.contains(from), "{from:?} is not in the example");
    document.replacen(from, to, 1)
}

/// `(schema accepts, reader accepts)` for `document`.
fn verdicts(
    validator: &jsonschema::Validator,
    store: &ekr_ontology::Ontology,
    document: &str,
) -> (bool, bool) {
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(document).unwrap_or_else(|e| panic!("not YAML ({e})"));
    let instance = serde_json::to_value(yaml).unwrap();
    (
        validator.is_valid(&instance),
        ekr_integrate::read_extraction(document, store).is_ok(),
    )
}

/// The schema and the reader give one answer for each document. Where they part, the schema's
/// description names the gap (its YAML tag and reader paragraphs), or it is a finding.
#[test]
fn adv_the_schema_and_the_reader_agree_on_tags_and_repeats() {
    let schema: Value = serde_json::from_str(&text(&["schema", EXTRACTION])).unwrap();
    let description = schema["description"].as_str().unwrap().to_owned();
    let validator = jsonschema::draft202012::new(&schema).unwrap();
    let store = store();
    let example = text(&["example", EXTRACTION]);
    let cases = [
        (
            "the format written as a tag",
            edit(
                &example,
                "format: ekr.extraction-document/1\n",
                "format: !ekr.extraction-document/1 ~\n",
            ),
        ),
        (
            "a fact tag written verbatim",
            edit(&example, "- !Property\n", "- !<!Property>\n"),
        ),
        (
            "a named thing's node type written with a tag",
            edit(
                &example,
                "- node_type: Person\n",
                "- node_type: !Kind Person\n",
            ),
        ),
        (
            "one alias twice",
            edit(
                &example,
                "  aliases:\n  - Carol\n",
                "  aliases:\n  - Carol\n  - Carol\n",
            ),
        ),
        (
            "a node type declared twice",
            edit(
                &example,
                "  edge_types:\n",
                "  - name: Project\n  edge_types:\n",
            ),
        ),
        (
            "an alias YAML reads as null",
            edit(&example, "  aliases:\n  - Carol\n", "  aliases:\n  - ~\n"),
        ),
    ];
    let mut parted = Vec::new();
    for (what, document) in cases {
        let (schema_accepts, reader_accepts) = verdicts(&validator, &store, &document);
        eprintln!("{what}: schema {schema_accepts}, reader {reader_accepts}");
        if schema_accepts != reader_accepts {
            parted.push(format!(
                "{what}: schema {}, reader {}",
                if schema_accepts { "accepts" } else { "refuses" },
                if reader_accepts { "accepts" } else { "refuses" },
            ));
        }
    }
    // A tag on a scalar that names no variant is a gap the description states.
    assert!(description.contains("a tag on a scalar"), "{description}");
    parted.retain(|line| !line.starts_with("a named thing's node type written with a tag"));
    // A name declared twice in a list of declarations is not a repeated item JSON Schema can
    // see (`uniqueItems` compares whole declarations); the reader refuses it by code, and the
    // description says so.
    assert!(
        description.contains("a name declared twice")
            && description.contains("extraction-name-duplicate"),
        "{description}"
    );
    parted.retain(|line| line != "a node type declared twice: schema accepts, reader refuses");
    assert!(parted.is_empty(), "{parted:#?}");
}
