//! The extraction document's reader: the acceptance of
//! `story:extraction-document-applies-to-a-store`.
//!
//! The example `ekr example ekr.extraction-document/1` prints is read against the ontology of the
//! store `ekr example ekr-seed/2` seeds, and accepted. A document naming a type neither it nor the
//! store declares, and a fact citing no evidence, are each refused with a named code. Every code,
//! every fact kind and every field the crate reads is the one `systems/ekr/domains/integrate.yaml`
//! declares.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ekr_integrate::{
    ExtractedFact, ExtractionDocument, ExtractionError, ExtractionRefusal, ExtractionRefusalCode,
    EXTRACTION_FORMAT,
};
use ekr_ontology::Ontology;
use serde_yaml_ng::Value;

fn root() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
}

fn read(relative: &str) -> String {
    let path = root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// The document `ekr example ekr.extraction-document/1` prints.
fn example() -> String {
    read("../ekr/src/cli/examples/extraction.yaml")
}

/// The ontology of the store `ekr example ekr-seed/2` seeds.
fn store() -> Ontology {
    let seed: Value = serde_yaml_ng::from_str(&read("../ekr/src/cli/examples/seed.yaml"))
        .expect("the example seed is YAML");
    let section = serde_yaml_ng::to_string(&seed["ontology"]).expect("the section writes");
    Ontology::from_yaml(&section).expect("the example seed's ontology loads")
}

/// `document` with `from` replaced by `to`, once; `from` must be in it.
fn edit(document: &str, from: &str, to: &str) -> String {
    assert!(document.contains(from), "{from:?} is not in {document}");
    document.replacen(from, to, 1)
}

fn refusal(document: &str) -> ExtractionRefusal {
    match ekr_integrate::read_extraction(document, &store()) {
        Err(ExtractionError::Refused(refusal)) => refusal,
        Err(ExtractionError::Document(error)) => {
            panic!("refused as a document, not with a code: {error}\n{document}")
        }
        Ok(_) => panic!("accepted:\n{document}"),
    }
}

#[test]
fn the_example_is_accepted_against_the_example_store() {
    let document =
        ekr_integrate::read_extraction(&example(), &store()).expect("the example is accepted");
    assert_eq!(EXTRACTION_FORMAT, "ekr.extraction-document/1");
    assert!(
        !document.ontology.node_types.is_empty(),
        "it grows the ontology"
    );
    assert!(!document.entities.is_empty(), "it names a thing");
    let kinds: BTreeSet<&str> = document
        .facts
        .iter()
        .map(|fact| match fact {
            ExtractedFact::Property(_) => "Property",
            ExtractedFact::Relation(_) => "Relation",
        })
        .collect();
    assert_eq!(kinds, BTreeSet::from(["Property", "Relation"]));
    assert!(!document.evidence.is_empty());
    for item in &document.evidence {
        assert_eq!(
            item.evidence.content_hash,
            ekr_core::ContentHash::of_bytes(&item.payload),
            "an evidence item's payload hashes to its entry"
        );
    }
}

#[test]
fn a_type_neither_the_document_nor_the_store_declares_is_refused_by_name() {
    let document = edit(
        &example(),
        "    node_type: Organization\n",
        "    node_type: Company\n",
    );
    assert_eq!(
        refusal(&document),
        ExtractionRefusal {
            code: ExtractionRefusalCode::ExtractionTypeUndeclared,
            name: "Company".to_owned(),
        }
    );
    // A relation names an edge type, and a type the document declares itself is not refused.
    let relation = edit(&example(), "  relation: CEO_OF\n", "  relation: FOUNDED\n");
    assert_eq!(
        refusal(&relation).code,
        ExtractionRefusalCode::ExtractionTypeUndeclared
    );
    let declared_here = ekr_integrate::read_extraction(&example(), &store()).unwrap();
    assert!(declared_here
        .ontology
        .node_types
        .iter()
        .all(|declared| store()
            .to_document()
            .node_types
            .iter()
            .all(|held| held.name != declared.name)));
}

#[test]
fn a_property_the_subjects_type_does_not_declare_is_refused_by_name() {
    let document = edit(
        &example(),
        "  property: legal_name\n",
        "  property: ticker\n",
    );
    assert_eq!(
        refusal(&document),
        ExtractionRefusal {
            code: ExtractionRefusalCode::ExtractionPropertyUndeclared,
            name: "Organization.ticker".to_owned(),
        }
    );
}

#[test]
fn a_fact_citing_no_evidence_is_refused_by_name() {
    let example = example();
    let cited = "  evidence:\n  - 00000000-0000-4000-8000-000000000403\n";
    for citing_none in ["  evidence: []\n", ""] {
        let document = edit(&example, cited, citing_none);
        assert_eq!(
            refusal(&document),
            ExtractionRefusal {
                code: ExtractionRefusalCode::FactWithoutEvidence,
                name: "facts[0]".to_owned(),
            },
            "{citing_none:?}"
        );
    }
    let unlisted = edit(
        &example,
        cited,
        "  evidence:\n  - 00000000-0000-4000-8000-000000000499\n",
    );
    assert_eq!(
        refusal(&unlisted),
        ExtractionRefusal {
            code: ExtractionRefusalCode::FactEvidenceUnlisted,
            name: "facts[0]: 00000000-0000-4000-8000-000000000499".to_owned(),
        }
    );
}

#[test]
fn a_document_that_is_not_the_format_is_refused_before_it_is_checked() {
    let example = example();
    for (what, document) in [
        (
            "another format",
            edit(
                &example,
                "format: ekr.extraction-document/1",
                "format: ekr.extraction-document/2",
            ),
        ),
        (
            "an unknown field",
            edit(&example, "entities:\n", "notes: []\nentities:\n"),
        ),
        (
            "a YAML alias",
            edit(
                &edit(
                    &example,
                    "  aliases:\n  - Carol\n",
                    "  aliases: &carol\n  - Carol\n",
                ),
                "  aliases:\n  - Apollo\n",
                "  aliases: *carol\n",
            ),
        ),
        (
            "a fact as a one-key object rather than a tag",
            edit(
                &example,
                "- !Property\n  subject:\n    node_type: Organization\n    aliases:\n    - \
                 Globex\n  property: legal_name\n  value:\n    value_kind: String\n    value: \
                 Globex Corporation\n",
                "- Property:\n    subject:\n      node_type: Organization\n      aliases:\n      \
                 - Globex\n    property: legal_name\n    value:\n      value_kind: String\n      \
                 value: Globex Corporation\n    evidence:\n    - \
                 00000000-0000-4000-8000-000000000403\n- !Property\n  subject:\n    node_type: \
                 Organization\n    aliases:\n    - Globex\n  property: legal_name\n  value:\n    \
                 value_kind: String\n    value: Globex Corporation\n",
            ),
        ),
        (
            "a fact as a one-key object with the tag as its key",
            edit(
                &example,
                "- !Property\n  subject:\n    node_type: Organization\n    aliases:\n    - \
                 Globex\n  property: legal_name\n  value:\n    value_kind: String\n    value: \
                 Globex Corporation\n",
                "- \"!Property\":\n    subject:\n      node_type: Organization\n      aliases:\n      \
                 - Globex\n    property: legal_name\n    value:\n      value_kind: String\n      \
                 value: Globex Corporation\n    evidence:\n    - \
                 00000000-0000-4000-8000-000000000403\n- !Property\n  subject:\n    node_type: \
                 Organization\n    aliases:\n    - Globex\n  property: legal_name\n  value:\n    \
                 value_kind: String\n    value: Globex Corporation\n",
            ),
        ),
        ("two documents", format!("{example}---\n{example}")),
    ] {
        match ExtractionDocument::from_yaml(&document) {
            Err(ExtractionError::Document(_)) => {}
            other => panic!("{what}: {other:?}"),
        }
    }
    let over = "#".repeat(ekr_integrate::EXTRACTION_INPUT_BYTES + 1);
    assert!(matches!(
        ExtractionDocument::from_yaml(&over),
        Err(ExtractionError::Document(_))
    ));
}

// The domain ------------------------------------------------------------------------------------

/// The declaration of `name` in the domain file its prefix names.
fn declaration(name: &str) -> Value {
    let file = if name.starts_with("ekr.kernel.") {
        "kernel"
    } else {
        "integrate"
    };
    let domain: Value =
        serde_yaml_ng::from_str(&read(&format!("../../systems/ekr/domains/{file}.yaml")))
            .expect("the domain is YAML");
    domain["types"]
        .as_sequence()
        .expect("the domain declares types")
        .iter()
        .find(|declared| declared["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("the domain declares no {name}"))
        .clone()
}

fn declared_fields(name: &str) -> BTreeSet<String> {
    declaration(name)["fields"]
        .as_sequence()
        .unwrap_or_else(|| panic!("{name} declares fields"))
        .iter()
        .map(|field| field["name"].as_str().expect("a named field").to_owned())
        .collect()
}

fn keys(value: &Value) -> BTreeSet<String> {
    value
        .as_mapping()
        .expect("a mapping")
        .keys()
        .map(|key| key.as_str().expect("a named key").to_owned())
        .collect()
}

/// Every code by an exhaustive match: a new variant does not compile here until it is listed.
fn every_code() -> Vec<ExtractionRefusalCode> {
    use ExtractionRefusalCode as Code;
    let all = Code::ALL;
    for code in all {
        match code {
            Code::ExtractionDocumentTooLarge
            | Code::ExtractionDocumentTooDeep
            | Code::ExtractionYamlAlias
            | Code::ExtractionDocumentMalformed
            | Code::ExtractionNameDuplicate
            | Code::ExtractionTypeConflict
            | Code::ExtractionTypeUndeclared
            | Code::ExtractionValueTypeEmpty
            | Code::ReferenceWithoutIdentity
            | Code::ReferenceTypeHasSubtypes
            | Code::ExtractionPropertyUndeclared
            | Code::ExtractionValueMismatch
            | Code::ExtractionRelationEnds
            | Code::FactWithoutEvidence
            | Code::FactEvidenceUnlisted
            | Code::DuplicateIdentity
            | Code::EvidencePayloadMismatch => {}
        }
    }
    let distinct: BTreeSet<_> = all.iter().collect();
    assert_eq!(distinct.len(), all.len(), "ALL lists each code once");
    all.to_vec()
}

#[test]
fn every_refusal_code_is_the_one_the_domain_declares() {
    let declared: Vec<String> = declaration("ekr.integrate.ExtractionRefusalCode")["variants"]
        .as_sequence()
        .expect("variants")
        .iter()
        .map(|variant| variant.as_str().expect("a name").to_owned())
        .collect();
    let written: Vec<String> = every_code()
        .into_iter()
        .map(|code| {
            let text = serde_json::to_value(code).unwrap();
            assert_eq!(text.as_str(), Some(code.code()));
            code.code().to_owned()
        })
        .collect();
    assert_eq!(written, declared);
    let refusal = ExtractionRefusal {
        code: ExtractionRefusalCode::FactWithoutEvidence,
        name: "facts[0]".to_owned(),
    };
    assert_eq!(
        keys(&serde_yaml_ng::to_value(&refusal).unwrap()),
        declared_fields("ekr.integrate.ExtractionRefusal")
    );
}

/// The example's own keys, at every level the domain declares a struct for, are the fields the
/// domain gives that struct, and its fact tags are the union's variants: a field the crate reads
/// and the domain does not declare, or the other way round, fails here.
#[test]
fn the_examples_keys_are_the_fields_the_domain_declares() {
    let document: Value = serde_yaml_ng::from_str(&example()).unwrap();
    assert_eq!(
        keys(&document),
        declared_fields("ekr.integrate.ExtractionDocument")
    );
    assert_eq!(
        declaration("ekr.integrate.ExtractionFormat")["variants"][0].as_str(),
        Some(EXTRACTION_FORMAT)
    );
    assert_eq!(
        keys(&document["ontology"]),
        declared_fields("ekr.integrate.OntologySpec")
    );
    let node_type = &document["ontology"]["node_types"][0];
    assert_eq!(
        keys(node_type),
        declared_fields("ekr.integrate.NodeTypeSpec")
    );
    assert_eq!(
        keys(&node_type["properties"][0]),
        declared_fields("ekr.integrate.PropertySpec")
    );
    assert_eq!(
        keys(&document["ontology"]["edge_types"][0]),
        declared_fields("ekr.integrate.EdgeTypeSpec")
    );
    assert_eq!(
        keys(&document["entities"][0]),
        declared_fields("ekr.integrate.ExtractedReference")
    );
    let union = declaration("ekr.integrate.ExtractedFact");
    let variants = union["variants"].as_mapping().unwrap();
    let mut tags = BTreeSet::new();
    for fact in document["facts"].as_sequence().unwrap() {
        let Value::Tagged(tagged) = fact else {
            panic!("a fact is written as a tag: {fact:?}")
        };
        let tag = tagged.tag.to_string();
        let kind = tag.trim_start_matches('!');
        let payload = variants[kind]
            .as_str()
            .unwrap_or_else(|| panic!("the domain declares no fact {kind}"));
        assert_eq!(keys(&tagged.value), declared_fields(payload), "{kind}");
        tags.insert(kind.to_owned());
    }
    let declared: BTreeSet<String> = variants
        .keys()
        .map(|key| key.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(tags, declared);
    assert_eq!(
        keys(&document["evidence"][0]),
        declared_fields("ekr.kernel.EvidenceAdditionProjection")
    );
}

/// Each refusal before decoding carries its own code.
#[test]
fn a_refusal_before_decoding_names_its_code() {
    use ExtractionRefusalCode as Code;
    let example = example();
    let code = |document: &str| match ExtractionDocument::from_yaml(document) {
        Err(ExtractionError::Document(refusal)) => refusal.code,
        other => panic!("{other:?}"),
    };
    let alias = edit(
        &edit(
            &example,
            "  aliases:\n  - Carol\n",
            "  aliases: &carol\n  - Carol\n",
        ),
        "  aliases:\n  - Apollo\n",
        "  aliases: *carol\n",
    );
    assert_eq!(code(&alias), Code::ExtractionYamlAlias);
    let over = "#".repeat(ekr_integrate::EXTRACTION_INPUT_BYTES + 1);
    assert_eq!(code(&over), Code::ExtractionDocumentTooLarge);
    let deep = format!(
        "format: ekr.extraction-document/1\nevidence: []\nfacts: {}{}\n",
        "[".repeat(ekr_integrate::EXTRACTION_DEPTH),
        "]".repeat(ekr_integrate::EXTRACTION_DEPTH)
    );
    assert_eq!(code(&deep), Code::ExtractionDocumentTooDeep);
    let within = format!(
        "format: ekr.extraction-document/1\nevidence: []\nfacts: {}{}\n",
        "[".repeat(ekr_integrate::EXTRACTION_DEPTH - 1),
        "]".repeat(ekr_integrate::EXTRACTION_DEPTH - 1)
    );
    assert_eq!(code(&within), Code::ExtractionDocumentMalformed);
    let twice = edit(&example, "entities:\n", "entities: []\nentities:\n");
    assert_eq!(code(&twice), Code::ExtractionDocumentMalformed);
}

/// The example reads into the types the crate exports, field for field: the format is the one
/// constant, each named thing is an `ExtractedReference`, each fact a `PropertyFact` or a
/// `RelationFact` with its subject, name and cited evidence, and each evidence item the entry and
/// the exact bytes its hash addresses.
#[test]
fn the_example_reads_into_the_exported_fact_and_evidence_types() {
    let document =
        ekr_integrate::read_extraction(&example(), &store()).expect("the example is accepted");
    assert_eq!(document.format, ekr_integrate::ExtractionFormat::V1);
    assert_eq!(
        serde_json::to_value(ekr_integrate::ExtractionFormat::V1).unwrap(),
        serde_json::Value::from(ekr_integrate::EXTRACTION_FORMAT),
        "the one format variant is written as the format constant"
    );
    let named = |node_type: &str, alias: &str| ekr_integrate::ExtractedReference {
        node_type: node_type.to_owned(),
        aliases: vec![alias.to_owned()],
    };
    assert_eq!(
        document.entities,
        [named("Person", "Carol"), named("Project", "Apollo")]
    );

    let [ekr_integrate::ExtractionEvidence { evidence, payload }] = document.evidence.as_slice()
    else {
        panic!("one evidence item: {:?}", document.evidence)
    };
    assert_eq!(
        payload.as_slice(),
        b"Carol, CEO of Globex Corporation, leads project Apollo."
    );
    assert_eq!(
        evidence.content_hash,
        ekr_core::ContentHash::of_bytes(payload)
    );
    let cited = vec![evidence.id];

    assert_eq!(document.facts.len(), 4, "{:?}", document.facts);
    let ExtractedFact::Property(ekr_integrate::PropertyFact {
        subject,
        property,
        value,
        evidence: rests_on,
    }) = &document.facts[0]
    else {
        panic!("the first fact is a property: {:?}", document.facts[0])
    };
    assert_eq!(subject, &named("Organization", "Globex"));
    assert_eq!(property, "legal_name");
    assert_eq!(
        value,
        &ekr_ontology::Value::String("Globex Corporation".to_owned())
    );
    assert_eq!(rests_on, &cited);
    assert_eq!(
        document.facts[1],
        ExtractedFact::Relation(ekr_integrate::RelationFact {
            subject: named("Person", "Carol"),
            relation: "CEO_OF".to_owned(),
            object: named("Organization", "Globex"),
            evidence: cited.clone(),
        })
    );
    assert_eq!(
        document.facts[2],
        ExtractedFact::Relation(ekr_integrate::RelationFact {
            subject: named("Person", "Carol"),
            relation: "LEADS".to_owned(),
            object: named("Project", "Apollo"),
            evidence: cited.clone(),
        })
    );
    let ExtractedFact::Property(ekr_integrate::PropertyFact {
        property, value, ..
    }) = &document.facts[3]
    else {
        panic!("the last fact is a property: {:?}", document.facts[3])
    };
    assert_eq!(property, "status");
    assert_eq!(value, &ekr_ontology::Value::Enum("active".to_owned()));
}
