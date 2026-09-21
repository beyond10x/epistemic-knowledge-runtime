//! What `crates/ekr-ontology/src/value.rs` says about the ESS domain, checked against the domain.
//!
//! Nothing else compares the two halves: `ess specify validate` checks the document against
//! itself, and the rest of the suite checks the crate against itself. Two review passes found a
//! false sentence in that gap, each time a different one, so the gap has a case in it now.
//!
//! * **Pass 1** found `value.rs` attributing a `parameters` key to
//!   `systems/ekr/domains/ontology.yaml` that the document declares nowhere. The correction did
//!   not remove the attribution; it replaced it with a *quotation*.
//! * **Pass 2** — this file as it was written — found the quoted sentence false in turn.
//!   `ontology.yaml`'s comment claims `PropertyDefinition` carries the parameters of all four
//!   compound kinds, naming them; it carries `NodeRef`'s in `ref_allowed_types` and `Enum`'s in
//!   `enum_variants`, and has no field for a `List`'s element type or a `Record`'s fields.
//!
//! The consequence is not cosmetic: a `List` or `Record` property type is constructible here and
//! `Ontology::load` accepts it, so a store persisting a `PropertyDefinition` in P3 meets a type
//! with nowhere to go, and the one sentence that should have warned it said the opposite.
//!
//! Both halves are now corrected rather than reconciled — the document's comment by the
//! coordinator in the closing commit, the crate's sentence by this unit — and the underlying gap
//! is filed as `task:ess-domain-carries-compound-value-types`. The case below pins the true
//! statement from both sides, so that neither half can drift again without the other noticing.

use std::collections::BTreeSet;

/// `systems/ekr/domains/ontology.yaml`, as text.
fn domain_text() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../systems/ekr/domains/ontology.yaml"
    );
    std::fs::read_to_string(path).expect("the ESS domain is beside the crates")
}

/// The field names one named type of the domain declares.
fn fields_of(type_name: &str) -> BTreeSet<String> {
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&domain_text()).expect("the ESS domain parses");
    // A declaration may sit under `types:`, `entities:` or `views:` — the document puts
    // `ekr.ontology.PropertyDefinition` under `entities:`.
    let declared = ["types", "entities", "views"]
        .into_iter()
        .filter_map(|section| document.get(section))
        .filter_map(serde_yaml_ng::Value::as_sequence)
        .flatten()
        .find(|declared| {
            declared.get("name").and_then(serde_yaml_ng::Value::as_str) == Some(type_name)
        })
        .unwrap_or_else(|| panic!("the domain declares {type_name}"));
    declared
        .get("fields")
        .and_then(serde_yaml_ng::Value::as_sequence)
        .map(|fields| {
            fields
                .iter()
                .filter_map(|field| field.get("name").and_then(serde_yaml_ng::Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The domain carries the parameters of `NodeRef` and `Enum`, and of no other compound kind — and
/// `value.rs` says exactly that.
///
/// This is the adversary's pass-2 case, re-aimed. It was written as
/// `every_compound_kind_the_domain_says_property_definition_carries_it_actually_carries`, asserting
/// that `ekr.ontology.PropertyDefinition` carries the parameters of all four compound kinds,
/// because the document's own comment claimed it did and `value.rs` quoted the claim. The finding
/// was right and the claim was false, on both sides: the document carries two of the four.
///
/// The coordinator's decision is that both halves are corrected rather than reconciled — the
/// document's comment, in the closing commit, to name the two it carries and to say a `List` or
/// `Record` property has no domain representation yet; and the sentence in `value.rs`, here. The
/// gap itself is real and is filed as `task:ess-domain-carries-compound-value-types`; it is not
/// closed by this unit and is not papered over by it either.
///
/// So the case now pins the true statement from both sides. It goes red if the crate stops saying
/// the projection is partial, and it goes red if someone "fixes" the document by adding a carrier
/// for `List` or `Record` without telling the crate — which is the drift that would otherwise
/// leave `value.rs` describing a gap that had been closed.
#[test]
fn the_domain_carries_node_ref_and_enum_parameters_and_no_other_compound_kind() {
    let fields = fields_of("ekr.ontology.PropertyDefinition");
    let carried = |needle: &str| fields.iter().any(|field| field.contains(needle));

    // The two the projection covers. `value_kind` is the discriminant they hang off.
    assert!(
        carried("value_kind"),
        "the projection needs its discriminant: {fields:?}"
    );
    for (kind, needle) in [("NodeRef", "allowed_types"), ("Enum", "variants")] {
        assert!(
            carried(needle),
            "ekr.ontology.PropertyDefinition carries a {kind}'s parameters in a field naming \
             {needle:?}, and declares {fields:?}"
        );
    }

    // The two it does not. If this stops holding, the domain grew a carrier and `value.rs` is
    // describing a gap that is no longer there.
    for (kind, needle) in [("List", "element"), ("Record", "field")] {
        assert!(
            !carried(needle),
            "the domain has grown a carrier for a {kind}'s parameters (a field naming {needle:?} \
             in {fields:?}). That closes the gap value.rs describes and \
             task:ess-domain-carries-compound-value-types carries — update both, then this case."
        );
    }

    // And the crate says so, rather than claiming a projection that covers everything.
    let value_rs = {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/value.rs");
        std::fs::read_to_string(path).expect("the crate's own source")
    };
    assert!(
        value_rs.contains("task:ess-domain-carries-compound-value-types"),
        "value.rs must name the task that carries the List/Record gap"
    );
    assert!(
        value_rs.contains("The projection is partial"),
        "value.rs must say the projection is partial rather than justify itself by a claim to \
         cover all four compound kinds"
    );
    assert!(
        !value_rs.contains("are carried by PropertyDefinition below"),
        "value.rs must not quote the document's four-kind claim as its justification"
    );
}
