//! What this crate writes against what `systems/ekr/domains/integrate.yaml` declares.
//!
//! Every outcome kind, every refusal code and every field name the crate serialises is read off the
//! document, so a name that drifts on either side fails here.

use std::collections::BTreeSet;

use ekr_core::{NodeId, TypeId};
use ekr_integrate::{
    AmbiguousReference, ResolutionOutcome, ResolutionRefusal, ResolutionRefusalCode,
    ResolvedReference, TypedReference,
};
use serde_yaml_ng::Value;

fn domain() -> Value {
    let path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("../../systems/ekr/domains/integrate.yaml");
    let text = std::fs::read_to_string(path).expect("the ESS domain is beside the crates");
    serde_yaml_ng::from_str(&text).expect("the domain is YAML")
}

fn declaration(name: &str) -> Value {
    domain()["types"]
        .as_sequence()
        .expect("the domain declares types")
        .iter()
        .find(|declared| declared["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("the domain declares no {name}"))
        .clone()
}

fn declared_fields(name: &str) -> BTreeSet<String> {
    let fields: BTreeSet<String> = declaration(name)["fields"]
        .as_sequence()
        .unwrap_or_else(|| panic!("{name} declares fields"))
        .iter()
        .map(|field| {
            field["name"]
                .as_str()
                .expect("a field has a name")
                .to_owned()
        })
        .collect();
    assert!(!fields.is_empty(), "the field scan found nothing in {name}");
    fields
}

fn written_fields(value: &serde_json::Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("a struct serialises as an object")
        .keys()
        .cloned()
        .collect()
}

fn sample_reference() -> TypedReference {
    TypedReference {
        type_id: TypeId::mint(),
        aliases: vec!["alias".to_owned()],
    }
}

/// Every refusal code, by an exhaustive match: a variant added to the crate fails to compile here
/// until it is listed, and then fails the comparison until the domain declares it.
fn every_code() -> Vec<ResolutionRefusalCode> {
    let all = [
        ResolutionRefusalCode::ReferenceWithoutIdentity,
        ResolutionRefusalCode::ReferenceTypeHasSubtypes,
    ];
    for code in all {
        match code {
            ResolutionRefusalCode::ReferenceWithoutIdentity
            | ResolutionRefusalCode::ReferenceTypeHasSubtypes => {}
        }
    }
    all.to_vec()
}

/// Every outcome, one per variant, by the same exhaustive match.
fn every_outcome() -> Vec<ResolutionOutcome> {
    let all = vec![
        ResolutionOutcome::Resolved(ResolvedReference {
            node_id: NodeId::mint(),
        }),
        ResolutionOutcome::ProposeNew(sample_reference()),
        ResolutionOutcome::Ambiguous(AmbiguousReference {
            candidates: vec![NodeId::mint(), NodeId::mint()],
        }),
        ResolutionOutcome::Refused(ResolutionRefusal {
            code: ResolutionRefusalCode::ReferenceWithoutIdentity,
            reference: sample_reference(),
        }),
    ];
    for outcome in &all {
        match outcome {
            ResolutionOutcome::Resolved(_)
            | ResolutionOutcome::ProposeNew(_)
            | ResolutionOutcome::Ambiguous(_)
            | ResolutionOutcome::Refused(_) => {}
        }
    }
    all
}

#[test]
fn every_refusal_code_is_the_one_the_domain_declares() {
    let declared: Vec<String> = declaration("ekr.integrate.ResolutionRefusalCode")["variants"]
        .as_sequence()
        .expect("the code declares variants")
        .iter()
        .map(|variant| variant.as_str().expect("a variant is a name").to_owned())
        .collect();
    let written: Vec<String> = every_code()
        .into_iter()
        .map(|code| {
            let text = serde_json::to_value(code).expect("a code serialises");
            let text = text.as_str().expect("a code is a string").to_owned();
            assert_eq!(text, code.code(), "the wire name and code() agree");
            text
        })
        .collect();
    assert_eq!(written, declared);
}

#[test]
fn every_outcome_kind_and_field_is_the_one_the_domain_declares() {
    let union = declaration("ekr.integrate.ResolutionOutcome");
    let tag = union["tag"].as_str().expect("the union declares its tag");
    let variants = union["variants"]
        .as_mapping()
        .expect("the union declares variants");
    let declared: BTreeSet<String> = variants
        .keys()
        .map(|key| key.as_str().expect("a variant is a name").to_owned())
        .collect();

    let mut written = BTreeSet::new();
    for outcome in every_outcome() {
        let mut value = serde_json::to_value(&outcome).expect("an outcome serialises");
        let kind = value
            .as_object_mut()
            .expect("an outcome is an object")
            .remove(tag)
            .unwrap_or_else(|| panic!("an outcome carries its {tag}"));
        let kind = kind.as_str().expect("the tag is a name").to_owned();
        let payload = variants[kind.as_str()]
            .as_str()
            .unwrap_or_else(|| panic!("the domain declares no outcome {kind}"));
        assert_eq!(
            written_fields(&value),
            declared_fields(payload),
            "{kind} carries the fields of {payload}"
        );
        written.insert(kind);
    }
    assert_eq!(written, declared);
}

#[test]
fn a_refusal_carries_the_reference_with_the_fields_the_domain_declares() {
    let refusal = serde_json::to_value(ResolutionRefusal {
        code: ResolutionRefusalCode::ReferenceTypeHasSubtypes,
        reference: sample_reference(),
    })
    .expect("a refusal serialises");
    assert_eq!(
        written_fields(&refusal["reference"]),
        declared_fields("ekr.integrate.TypedReference")
    );
}
