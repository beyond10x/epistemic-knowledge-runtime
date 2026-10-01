//! Adversarial cases against `ekr.extraction-document/1` and its reader
//! (`story:extraction-document-applies-to-a-store`, wave extract-06 unit D).
//!
//! Each case drives `read_extraction` with an edit of the document `ekr example
//! ekr.extraction-document/1` prints, read against the ontology the example seed declares, and
//! asserts what the format's own documents say: the module documentation of
//! `crates/ekr-integrate/src/extraction.rs`, `docs/cli.md` § Extraction documents and § Value
//! types, `systems/ekr/domains/integrate.yaml`, and the SDK path the apply verb is to run on
//! (`crates/ekr-sdk/src/document/ontology.rs`). A case that fails names a disagreement between
//! those documents and the reader.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ekr_integrate::{
    read_extraction, ExtractionDocument, ExtractionError, TypedReference, ValueSpec,
};
use ekr_ontology::{Ontology, ValueType};
use serde_yaml_ng::Value;

fn root() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    let path = root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn example() -> String {
    read("../ekr/src/cli/examples/extraction.yaml")
}

fn store() -> Ontology {
    let seed: Value = serde_yaml_ng::from_str(&read("../ekr/src/cli/examples/seed.yaml"))
        .expect("the example seed is YAML");
    let section = serde_yaml_ng::to_string(&seed["ontology"]).expect("the section writes");
    Ontology::from_yaml(&section).expect("the example seed's ontology loads")
}

fn edit(document: &str, from: &str, to: &str) -> String {
    assert!(document.contains(from), "{from:?} is not in {document}");
    document.replacen(from, to, 1)
}

/// The reader's answer, as text: `accepted` or the refusal.
fn verdict(document: &str) -> String {
    match read_extraction(document, &store()) {
        Ok(_) => "accepted".to_owned(),
        Err(error) => format!("refused: {error}"),
    }
}

fn assert_refused(what: &str, document: &str) {
    let answer = verdict(document);
    assert!(
        answer.starts_with("refused"),
        "{what}: the reader accepts it"
    );
}

/// The example's one evidence item, from `- evidence:` to the end of the file.
fn evidence_item(example: &str) -> String {
    let at = example
        .find("- evidence:\n")
        .expect("the example has an evidence item");
    example[at..].to_owned()
}

// The ontology section, against the SDK path the apply verb is to run on ------------------------

/// `NodeTypeSpec.name` is "unique among node types" (`extraction.rs`, `integrate.yaml`), and
/// `ekr_sdk::document::OntologySpec` refuses a second declaration of one name as
/// `DuplicateName` (`check_unique`). The reader merges the two declarations instead.
#[test]
#[ignore = "finding: the reader accepts a node type, a property or an edge type declared twice"]
fn adv_a_name_the_document_declares_twice_is_refused() {
    let example = example();
    let node_twice = edit(
        &example,
        "  edge_types:\n",
        "  - name: Project\n    properties:\n    - name: budget\n      value:\n        \
         value_kind: Integer\n  edge_types:\n",
    );
    let property_twice = edit(
        &example,
        "      required: false\n  edge_types:\n",
        "      required: false\n    - name: status\n      value:\n        value_kind: String\n  \
         edge_types:\n",
    );
    let edge_twice = edit(
        &example,
        "    properties: []\nentities:\n",
        "    properties: []\n  - name: LEADS\n    source_types: [Project]\n    target_types: \
         [Person]\nentities:\n",
    );
    let mut accepted = Vec::new();
    for (what, document) in [
        ("node type Project twice", node_twice),
        (
            "property Project.status twice, String and Enum",
            property_twice,
        ),
        ("edge type LEADS twice, with opposite ends", edge_twice),
    ] {
        if verdict(&document) == "accepted" {
            accepted.push(what);
        }
    }
    assert!(accepted.is_empty(), "the reader accepts: {accepted:?}");
}

/// A document redeclaring a type the store holds, with other parents, is what the SDK refuses as
/// `OntologyError::Conflict` ("the store declares it with other parents"); `docs/cli.md` says a
/// type the store holds "is kept". The reader instead merges the document's parents into the
/// store's type, and so accepts a property the store's type does not have.
#[test]
#[ignore = "finding: a store type redeclared with another parent lends it the parent's properties"]
fn adv_a_store_type_redeclared_with_another_parent_is_refused() {
    let document = edit(
        &edit(
            &example(),
            "  edge_types:\n",
            "  - name: Organization\n    parents: [Project]\n  edge_types:\n",
        ),
        "  property: legal_name\n  value:\n    value_kind: String\n    value: Globex Corporation\n",
        "  property: status\n  value:\n    value_kind: Enum\n    value: active\n",
    );
    assert_refused(
        "Organization redeclared under Project, then Organization.status",
        &document,
    );
}

// Value types -------------------------------------------------------------------------------------

/// `extraction.rs` says a `ValueSpec` is "written as a value type is (`value_kind`, and
/// `parameters` for the kinds that take one)", and `docs/cli.md` says a property's `value` "is a
/// value type whose `NodeRef` names node types in `parameters`". § Value types writes an `Enum`
/// as `{value_kind: Enum, parameters: {variants: [a, b]}}`, which is what `ValueType` writes.
/// The reader refuses that form and takes `parameters: [a, b]` only.
#[test]
#[ignore = "finding: an Enum value type written as docs/cli.md § Value types writes it is refused"]
fn adv_an_enum_value_type_written_as_a_value_type_is_read() {
    let written = "value_kind: Enum\nparameters:\n  variants: [active, closed]\n";
    let held: ValueType = serde_yaml_ng::from_str(written).expect("a value type reads it");
    assert!(matches!(held, ValueType::Enum { .. }));
    let spec = serde_yaml_ng::from_str::<ValueSpec>(written);
    assert!(
        spec.is_ok(),
        "ValueSpec refuses the value type form: {spec:?}"
    );

    let document = edit(
        &example(),
        "        parameters:\n        - active\n        - closed\n",
        "        parameters:\n          variants: [active, closed]\n",
    );
    assert_eq!(verdict(&document), "accepted");
}

/// `ValueType` refuses an empty `Enum` or `NodeRef` at load ("a type no value inhabits is not a
/// type", `ekr-ontology/src/value.rs`) and a repeated variant at decode (`unique_set`). The
/// reader takes all of them, so a document it accepts declares a type no store loads.
#[test]
#[ignore = "finding: an empty or repeated Enum/NodeRef parameter list is accepted"]
fn adv_an_enum_or_node_ref_no_store_can_declare_is_refused() {
    let example = example();
    let enum_at = "        parameters:\n        - active\n        - closed\n";
    let mut accepted = Vec::new();
    for (what, document) in [
        (
            "Enum with no variant",
            edit(&example, enum_at, "        parameters: []\n"),
        ),
        (
            "Enum with one variant twice",
            edit(&example, enum_at, "        parameters: [active, active]\n"),
        ),
        (
            "NodeRef to no node type",
            edit(
                &example,
                "        value_kind: Enum\n",
                "        value_kind: NodeRef\n",
            )
            .replacen(enum_at, "        parameters: []\n", 1),
        ),
    ] {
        if verdict(&document) == "accepted" {
            accepted.push(what);
        }
    }
    assert!(
        serde_yaml_ng::from_str::<ValueType>(
            "value_kind: Enum\nparameters:\n  variants: [active, active]\n"
        )
        .is_err(),
        "a value type refuses a repeated variant"
    );
    assert!(accepted.is_empty(), "the reader accepts: {accepted:?}");
}

/// A `Record` value type with one field written twice. `ValueSpec::Record` is a plain
/// `BTreeMap`, so the second silently replaces the first; the repository's own decoders refuse a
/// repeated key (`ekr_core::decode::unique_map`, and the YAML schema description: "The readers
/// refuse ... a mapping key written twice").
#[test]
#[ignore = "finding: a Record value type with a field written twice keeps the last and is accepted"]
fn adv_a_record_value_type_with_a_field_written_twice_is_refused() {
    let written = "value_kind: Record\nparameters:\n  amount:\n    value_kind: String\n  \
                   amount:\n    value_kind: Integer\n";
    let read = serde_yaml_ng::from_str::<ValueSpec>(written);
    assert!(
        read.is_err(),
        "the second `amount` replaced the first: {read:?}"
    );
}

// Facts against the types they name --------------------------------------------------------------

/// The reader checks a fact's property name and nothing about its value: a value of another kind
/// than the property's, an `Enum` value no variant names, and a `Float` (which `docs/cli.md`
/// says "no Float value can be committed") are all accepted.
#[test]
#[ignore = "finding: a property fact whose value cannot satisfy the property's type is accepted"]
fn adv_a_property_fact_whose_value_the_property_cannot_hold_is_refused() {
    let example = example();
    let status = "  property: status\n  value:\n    value_kind: Enum\n    value: active\n";
    let mut accepted = Vec::new();
    for (what, document) in [
        (
            "Project.status (Enum) given an Integer",
            edit(
                &example,
                status,
                "  property: status\n  value:\n    value_kind: Integer\n    value: 5\n",
            ),
        ),
        (
            "Project.status (Enum active|closed) given `archived`",
            edit(
                &example,
                status,
                "  property: status\n  value:\n    value_kind: Enum\n    value: archived\n",
            ),
        ),
        (
            "Organization.legal_name (String) given a Float",
            edit(
                &example,
                "    value_kind: String\n    value: Globex Corporation\n",
                "    value_kind: Float\n    value: 1.5\n",
            ),
        ),
    ] {
        if verdict(&document) == "accepted" {
            accepted.push(what);
        }
    }
    assert!(accepted.is_empty(), "the reader accepts: {accepted:?}");
}

/// A relation whose subject is not of a source type of the edge type: `CEO_OF` runs from
/// `Person` to `Organization` in the example store, and a `Project` is its subject here.
#[test]
#[ignore = "finding: a relation between types its edge type does not connect is accepted"]
fn adv_a_relation_between_types_its_edge_type_does_not_connect_is_refused() {
    let document = edit(
        &example(),
        "    node_type: Person\n    aliases:\n    - Carol\n  relation: CEO_OF\n",
        "    node_type: Project\n    aliases:\n    - Apollo\n  relation: CEO_OF\n",
    );
    assert_refused("Project CEO_OF Organization", &document);
}

// Named things ------------------------------------------------------------------------------------

/// `integrate.yaml` says a named thing, once its type maps to an id, "is an
/// `ekr.integrate.TypedReference`, with these aliases, and resolves as one". `TypedReference`
/// refuses an alias YAML reads as null, a boolean or a number ("`List<String>` means strings",
/// `lib.rs`); `ExtractedReference.aliases` is a plain `Vec<String>` and reads each as its text.
#[test]
#[ignore = "finding: an alias YAML reads as null, a boolean or a number is accepted as text"]
fn adv_an_alias_that_is_not_a_string_is_refused_as_a_typed_reference_refuses_it() {
    let example = example();
    let mut accepted = Vec::new();
    for alias in ["~", "null", "true", "1.0", "0x10"] {
        let reference =
            format!("type_id: 00000000-0000-4000-8000-000000000201\naliases: [{alias}]\n");
        assert!(
            serde_yaml_ng::from_str::<TypedReference>(&reference).is_err(),
            "TypedReference refuses alias {alias}"
        );
        let document = edit(
            &example,
            "- node_type: Person\n  aliases:\n  - Carol\n",
            &format!("- node_type: Person\n  aliases:\n  - {alias}\n"),
        );
        if let Ok(read) = read_extraction(&document, &store()) {
            accepted.push(format!("{alias} read as {:?}", read.entities[0].aliases));
        }
    }
    assert!(accepted.is_empty(), "the reader accepts: {accepted:?}");
}

/// A named thing with no alias, or only the empty one, identifies nothing: the resolver it is to
/// resolve through refuses it as `reference-without-identity`. The reader accepts it.
#[test]
#[ignore = "finding: a named thing with no identifying alias is accepted"]
fn adv_a_named_thing_with_no_identifying_alias_is_refused() {
    let example = example();
    let mut accepted = Vec::new();
    for aliases in ["[]", "[\"\"]"] {
        let document = edit(
            &example,
            "- node_type: Person\n  aliases:\n  - Carol\n",
            &format!("- node_type: Person\n  aliases: {aliases}\n"),
        );
        if verdict(&document) == "accepted" {
            accepted.push(aliases);
        }
    }
    assert!(
        accepted.is_empty(),
        "the reader accepts aliases: {accepted:?}"
    );
}

// Evidence ----------------------------------------------------------------------------------------

/// Two evidence items under one id: a fact citing it rests on whichever an applier picks, and the
/// second `AddEvidence` of one id cannot commit. The reader collects ids into a set and accepts.
#[test]
#[ignore = "finding: two evidence items under one id are accepted"]
fn adv_two_evidence_items_under_one_id_are_refused() {
    let example = example();
    let item = evidence_item(&example);
    let other = item.replacen("payload: [67, ", "payload: [68, ", 1);
    let document = format!("{example}{other}");
    let read = read_extraction(&document, &store());
    if let Ok(read) = &read {
        let ids: BTreeSet<_> = read.evidence.iter().map(|item| item.evidence.id).collect();
        assert_eq!(read.evidence.len(), 2);
        assert_eq!(ids.len(), 1);
    }
    assert!(read.is_err(), "the reader accepts two items under one id");
}

/// `docs/cli.md` says each evidence item carries "a payload that hashes to `content_hash`", and
/// the schema says the payload is "the exact bytes its `content_hash` addresses". The reader does
/// not compare them.
#[test]
#[ignore = "finding: an evidence payload that does not hash to its content_hash is accepted"]
fn adv_an_evidence_payload_that_does_not_hash_to_its_entry_is_refused() {
    let document = edit(&example(), "payload: [67, ", "payload: [68, ");
    assert_refused("payload byte 0 changed, content_hash kept", &document);
}

// Decoding: nesting, tags, merge keys, round trip ------------------------------------------------

/// `n` `List` value types nested in the example's `Project.status`, `parameters` written first
/// when `parameters_first`, so that serde buffers each level before it knows the kind.
fn nested_lists(n: usize, parameters_first: bool) -> String {
    let mut spec = "{value_kind: String}".to_owned();
    for _ in 0..n {
        spec = if parameters_first {
            format!("{{parameters: {spec}, value_kind: List}}")
        } else {
            format!("{{value_kind: List, parameters: {spec}}}")
        };
    }
    edit(
        &example(),
        "      value:\n        value_kind: Enum\n        parameters:\n        - active\n        \
         - closed\n",
        &format!("      value: {spec}\n"),
    )
}

/// Nesting up to and past the YAML deserializer's recursion limit (128) finishes, on the thread
/// the test runs on (2 MiB unless `RUST_MIN_STACK` says otherwise), with an answer: no overflow.
#[test]
fn adv_value_types_nested_to_the_recursion_limit_answer_without_overflowing() {
    for parameters_first in [false, true] {
        for n in [100, 119, 120, 121, 122, 200, 10_000] {
            let document = nested_lists(n, parameters_first);
            let answer = ExtractionDocument::from_yaml(&document);
            assert!(
                matches!(answer, Ok(_) | Err(ExtractionError::Document(_))),
                "{n}: {answer:?}"
            );
        }
    }
}

/// A value nested inside a fact past the recursion limit is refused as a document.
#[test]
fn adv_a_value_nested_past_the_limit_is_refused_as_a_document() {
    let mut value = "{value_kind: String, value: x}".to_owned();
    for _ in 0..200 {
        value = format!("{{value_kind: List, value: [{value}]}}");
    }
    let deep_value = edit(
        &example(),
        "  value:\n    value_kind: String\n    value: Globex Corporation\n",
        &format!("  value: {value}\n"),
    );
    assert!(matches!(
        ExtractionDocument::from_yaml(&deep_value),
        Err(ExtractionError::Document(_))
    ));
}

/// A document of `depth` nested flow sequences as its `facts`.
fn flow_nesting(depth: usize) -> String {
    format!(
        "format: ekr.extraction-document/1\nevidence: []\nfacts: {}{}\n",
        "[".repeat(depth),
        "]".repeat(depth)
    )
}

/// The byte cap is the reader's only bound before the YAML loader runs. Nested flow sequences
/// are a few bytes a level, and the loader's time grows with the square of the depth, so a
/// document under the cap holds the reader for far longer than any document of the format needs.
/// Prints the time of each depth it measures, then gives the largest flow nesting the cap admits
/// a budget of 30 s (the example reads in milliseconds).
#[test]
#[ignore = "finding: a flow nesting within the 8 MiB cap is not answered in 30 s (quadratic load)"]
fn adv_the_deepest_flow_nesting_under_the_byte_cap_is_answered_promptly() {
    for depth in [4_096, 8_192, 16_384, 32_768] {
        let document = flow_nesting(depth);
        let started = std::time::Instant::now();
        let answer = ExtractionDocument::from_yaml(&document);
        let reader = started.elapsed();
        let started = std::time::Instant::now();
        let plain = serde_yaml_ng::from_str::<Value>(&document).map(drop);
        let loader = started.elapsed();
        eprintln!(
            "depth {depth:>6} ({:>6} bytes): reader {reader:?} ({}), plain Value decode \
             {loader:?} ({})",
            document.len(),
            if answer.is_ok() {
                "accepted"
            } else {
                "refused"
            },
            if plain.is_ok() { "ok" } else { "error" },
        );
    }
    let prefix = flow_nesting(0).len();
    let depth = (ekr_integrate::EXTRACTION_INPUT_BYTES - prefix) / 2;
    let document = flow_nesting(depth);
    assert!(document.len() <= ekr_integrate::EXTRACTION_INPUT_BYTES);
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let answer = ExtractionDocument::from_yaml(&document).map(drop);
        let _ = sender.send(answer);
    });
    match receiver.recv_timeout(std::time::Duration::from_secs(30)) {
        Ok(answer) => assert!(answer.is_err(), "depth {depth} accepted"),
        Err(_) => panic!(
            "a {}-byte document of {depth} nested flow sequences is not answered in 30 s",
            prefix + 2 * depth
        ),
    }
}

/// A merge key is an unknown field, not a merge: the reader neither expands nor ignores it.
#[test]
fn adv_a_merge_key_is_refused_not_merged() {
    let document = edit(
        &example(),
        "- node_type: Person\n  aliases:\n  - Carol\n",
        "- <<: {node_type: Person}\n  aliases:\n  - Carol\n",
    );
    assert!(matches!(
        ExtractionDocument::from_yaml(&document),
        Err(ExtractionError::Document(_))
    ));
}

/// What the reader reads, written back as YAML and read again, is the same document.
#[test]
fn adv_the_example_survives_a_write_and_a_second_read() {
    let first = read_extraction(&example(), &store()).expect("the example is accepted");
    let written = serde_yaml_ng::to_string(&first).expect("the document writes");
    let second = read_extraction(&written, &store())
        .unwrap_or_else(|e| panic!("the written document is refused: {e}\n{written}"));
    assert_eq!(first, second);
}

// Every site the module documentation names ------------------------------------------------------

/// `extraction-type-undeclared` is documented for a type "named by a parent, an edge type's end,
/// a `NodeRef`, a named thing, a fact's subject or object, or a relation". The unit's own cases
/// reach only a property fact's subject and a relation, so deleting the check at any other site
/// leaves them green. Each site, and a property found on an ancestor, is held here.
#[test]
fn adv_every_site_that_names_a_type_refuses_an_undeclared_one() {
    use ekr_integrate::{ExtractionRefusal, ExtractionRefusalCode};
    let example = example();
    let sites = [
        (
            "a parent",
            edit(&example, "    parents: []\n", "    parents: [Thing]\n"),
            "Thing",
        ),
        (
            "an edge type's source",
            edit(
                &example,
                "    source_types:\n    - Person\n",
                "    source_types:\n    - Human\n",
            ),
            "Human",
        ),
        (
            "an edge type's target",
            edit(
                &example,
                "    target_types:\n    - Project\n",
                "    target_types:\n    - Venture\n",
            ),
            "Venture",
        ),
        (
            "a NodeRef",
            edit(
                &example,
                "        value_kind: Enum\n",
                "        value_kind: NodeRef\n",
            )
            .replacen(
                "        parameters:\n        - active\n        - closed\n",
                "        parameters:\n        - Gadget\n",
                1,
            ),
            "Gadget",
        ),
        (
            "a named thing",
            edit(
                &example,
                "- node_type: Person\n  aliases:\n  - Carol\n",
                "- node_type: Human\n  aliases:\n  - Carol\n",
            ),
            "Human",
        ),
        (
            "a relation's subject",
            edit(
                &example,
                "    node_type: Person\n    aliases:\n    - Carol\n  relation: CEO_OF\n",
                "    node_type: Human\n    aliases:\n    - Carol\n  relation: CEO_OF\n",
            ),
            "Human",
        ),
        (
            "a relation's object",
            edit(
                &example,
                "  object:\n    node_type: Organization\n",
                "  object:\n    node_type: Firm\n",
            ),
            "Firm",
        ),
    ];
    for (site, document, name) in sites {
        match read_extraction(&document, &store()) {
            Err(ExtractionError::Refused(refusal)) => assert_eq!(
                refusal,
                ExtractionRefusal {
                    code: ExtractionRefusalCode::ExtractionTypeUndeclared,
                    name: name.to_owned(),
                },
                "{site}"
            ),
            other => panic!("{site}: {other:?}"),
        }
    }
    let inherited = edit(
        &edit(
            &example,
            "  edge_types:\n",
            "  - name: Subproject\n    parents: [Project]\n  edge_types:\n",
        ),
        "    node_type: Project\n    aliases:\n    - Apollo\n  property: status\n",
        "    node_type: Subproject\n    aliases:\n    - Apollo\n  property: status\n",
    );
    assert_eq!(
        verdict(&inherited),
        "accepted",
        "a property declared on a parent"
    );
}
