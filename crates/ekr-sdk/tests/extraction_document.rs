//! `story:extraction-verb-shares-the-sdk-path`: the SDK's mirror of `ekr.extraction-document/1`
//! reads the document `ekr example ekr.extraction-document/1` prints, and what it writes passes
//! `ekr schema ekr.extraction-document/1` and reads back as itself.
//!
//! The verbs run in process through `ekr::cli::run`, a dev-dependency.

use ekr_sdk::document::{
    ExtractedFact, ExtractedReference, ExtractionDocument, ExtractionFormat, ValueSpec,
    EXTRACTION_FORMAT,
};
use serde_json::Value as Json;

fn ekr(args: &[&str]) -> String {
    let mut input: &[u8] = b"";
    ekr::cli::run(
        std::iter::once("ekr").chain(args.iter().copied()),
        &|| ekr_sdk::document::Timestamp::from_millis(0),
        &mut input,
    )
    .unwrap_or_else(|failure| panic!("ekr {args:?}: {failure}"))
}

/// `ekr schema ekr.extraction-document/1` accepts `yaml` read as YAML and written as JSON.
fn schema_accepts(yaml: &str) {
    let schema: Json = serde_json::from_str(&ekr(&["schema", EXTRACTION_FORMAT])).unwrap();
    let validator = jsonschema::draft202012::new(&schema).unwrap();
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
    let instance = serde_json::to_value(value).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect();
    assert!(errors.is_empty(), "{errors:#?}\n{yaml}");
}

#[test]
fn the_mirror_reads_the_printed_example_and_writes_what_the_schema_accepts() {
    let example = ekr(&["example", EXTRACTION_FORMAT]);
    let document = ExtractionDocument::from_yaml(&example).unwrap();
    assert_eq!(document.format, ExtractionFormat::V1);
    assert_eq!(
        document.ontology.node_types[0].properties[0].value,
        ValueSpec::Enum(vec!["active".to_owned(), "closed".to_owned()])
    );
    assert_eq!(document.facts.len(), 4);
    assert!(matches!(
        &document.facts[1],
        ExtractedFact::Relation(fact)
            if fact.subject == ExtractedReference::new("Person", ["Carol"]) && fact.relation == "CEO_OF"
    ));
    assert_eq!(document.facts[0].evidence().len(), 1);
    assert_eq!(document.evidence.len(), 1);

    let written = document.to_yaml().unwrap();
    assert!(
        written.contains("!Property") && written.contains("!Relation"),
        "{written}"
    );
    assert!(written.contains("variants:"), "{written}");
    schema_accepts(&written);
    assert_eq!(ExtractionDocument::from_yaml(&written).unwrap(), document);
}

#[test]
fn a_node_ref_written_by_the_mirror_names_its_types_in_parameters() {
    let mut document = ExtractionDocument::new();
    document.ontology = ekr_sdk::document::OntologySpec::new().with_node_type(
        ekr_sdk::document::NodeTypeSpec::new("Ticket").with_property(
            ekr_sdk::document::PropertySpec::new(
                "owner",
                ValueSpec::List(Box::new(ValueSpec::NodeRef(vec!["Person".to_owned()]))),
            ),
        ),
    );
    let written = document.to_yaml().unwrap();
    assert!(written.contains("allowed_types:"), "{written}");
    schema_accepts(&written);
    assert_eq!(ExtractionDocument::from_yaml(&written).unwrap(), document);
    assert_eq!(ExtractionDocument::default(), ExtractionDocument::new());
}
