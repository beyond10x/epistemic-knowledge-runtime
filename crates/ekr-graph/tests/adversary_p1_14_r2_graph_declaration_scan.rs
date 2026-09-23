//! Adversary, unit p1-14-bindings, pass 2.
//!
//! `domain_projection.rs`'s older line scans (`declarations_with_fields`, `fields_of`,
//! `variants_of`) see a declaration only when its mapping opens with `- name: ekr.…`. The unit's
//! header says so and leaves it. The consequence: a `graph.yaml` declaration written
//! `- kind: struct` then `name: …` is the same YAML, is invisible to
//! `every_declaration_of_the_domain_is_carried_field_for_field`, and needs no carrier to stay green.
//!
//! The case below holds the assumption those scans rest on, reading the document as
//! `serde_yaml_ng` parses it. Paths resolve through the runtime `CARGO_MANIFEST_DIR`, so it can be
//! pointed at a mirror of the tree.

use serde_yaml_ng::Value as Yaml;

/// Every declaration under `types:`, `entities:` or `views:` that carries `fields:` or `variants:`
/// opens its mapping with `name:`, which is the only shape the line scans read.
#[test]
fn every_graph_declaration_the_line_scans_must_see_opens_with_its_name() {
    let path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("../../systems/ekr/domains/graph.yaml");
    let text = std::fs::read_to_string(path).expect("the ESS domain is beside the crates");
    let document: Yaml = serde_yaml_ng::from_str(&text).expect("the ESS domain parses");
    let heads: Vec<&str> = text.lines().map(str::trim).collect();

    let mut checked = 0usize;
    for section in ["types", "entities", "views"] {
        for declared in document
            .get(section)
            .and_then(Yaml::as_sequence)
            .into_iter()
            .flatten()
        {
            if declared.get("fields").is_none() && declared.get("variants").is_none() {
                continue;
            }
            let name = declared
                .get("name")
                .and_then(Yaml::as_str)
                .unwrap_or_else(|| panic!("a {section} entry has no name: {declared:?}"));
            assert!(
                heads.contains(&format!("- name: {name}").as_str()),
                "{name} does not open its mapping with `name:`, so the line scans of \
                 domain_projection.rs never see it and it needs no carrier"
            );
            checked += 1;
        }
    }
    assert!(checked >= 20, "the declaration scan is broken: {checked}");
}
