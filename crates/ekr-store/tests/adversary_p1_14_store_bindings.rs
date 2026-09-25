//! Adversary, unit p1-14-bindings, pass 1.
//!
//! `domain_projection.rs::every_type_and_entity_the_domain_declares_names_a_rust_carrier` binds
//! each `store.yaml` declaration to a Rust type by *name*: the crate must declare a `pub struct` or
//! `pub enum` spelled like the carrier, and nothing else is compared. Two drifts leave it green:
//!
//! * a field or variant added to, or removed from, one side of a bound pair — the declaration and
//!   its carrier disagree member for member, and both still exist;
//! * a declaration whose first key is not `name` (`- kind: struct` then `name: ...`), which is the
//!   same YAML and which its line scanner (`strip_prefix("  - name: ")`) never sees.
//!
//! The cases below read the same two files and compare members, and read a declaration's name
//! wherever the key sits in its mapping. Like the unit's case they resolve paths through the
//! runtime `CARGO_MANIFEST_DIR`, so they can be pointed at a copy of the tree.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
}

fn domain_text() -> String {
    std::fs::read_to_string(manifest_dir().join("../../systems/ekr/domains/store.yaml"))
        .expect("the ESS domain is beside the crates")
}

/// Every current module of `src/`, excluding the frozen legacy codec, as the unit's case does.
fn crate_sources() -> Vec<String> {
    let mut sources: Vec<(PathBuf, String)> = std::fs::read_dir(manifest_dir().join("src"))
        .expect("the crate has a src/")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.extension().is_some_and(|e| e == "rs")
                && path.file_name().is_some_and(|file| file != "legacy.rs")
        })
        .map(|path| {
            let text = std::fs::read_to_string(&path).expect("a source file");
            (path, text)
        })
        .collect();
    sources.sort();
    assert!(sources.len() >= 5, "the module scan is broken");
    sources.into_iter().map(|(_, text)| text).collect()
}

/// One list item under a top-level `types:` or `entities:` key.
#[derive(Debug, Default)]
struct Declaration {
    name: Option<String>,
    kind: Option<String>,
    identity: Option<String>,
    members: Vec<String>,
}

/// Every item under the top-level `types:` and `entities:` keys, whichever key each item's mapping
/// opens with.
fn declarations(text: &str) -> Vec<Declaration> {
    let mut found: Vec<Declaration> = Vec::new();
    let mut inside = false;
    // Which block key of the current item the deeper lines belong to.
    let mut block = String::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') {
            inside = matches!(line.trim_end(), "types:" | "entities:");
            continue;
        }
        if !inside {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let body = line.trim();
        let key_line = if indent == 2 && body.starts_with("- ") {
            found.push(Declaration::default());
            block.clear();
            Some(body.trim_start_matches("- ").trim())
        } else if indent == 4 {
            Some(body)
        } else {
            None
        };
        let current = found.last_mut().expect("an item precedes its keys");
        if let Some(key_line) = key_line {
            let (key, value) = key_line.split_once(':').unwrap_or((key_line, ""));
            let value = value.trim();
            match key.trim() {
                "name" => current.name = Some(value.to_owned()),
                "kind" => current.kind = Some(value.to_owned()),
                "variants" if value.starts_with('[') => {
                    current.members.extend(
                        value
                            .trim_matches(|c| c == '[' || c == ']')
                            .split(',')
                            .map(|v| v.trim().to_owned())
                            .filter(|v| !v.is_empty()),
                    );
                }
                other => block = other.to_owned(),
            }
            continue;
        }
        // A line deeper than the item's keys, inside `block`.
        match block.as_str() {
            "fields" if indent == 6 => {
                if let Some(field) = body.strip_prefix("- name:") {
                    current.members.push(field.trim().to_owned());
                }
            }
            "variants" if indent == 6 => {
                if let Some(variant) = body.strip_prefix("- ") {
                    current.members.push(variant.trim().to_owned());
                }
            }
            "identity" if indent == 6 => {
                if let Some(name) = body.strip_prefix("name:") {
                    current.identity = Some(name.trim().to_owned());
                }
            }
            _ => {}
        }
    }
    assert!(
        found.len() >= 10,
        "the declaration scan is broken, not the domain: {found:?}"
    );
    found
}

/// The member names of `pub struct name` (fields) or `pub enum name` (variants), read from the
/// item's own braces.
fn rust_members(name: &str) -> Option<Vec<String>> {
    for source in crate_sources() {
        let mut lines = source.lines();
        let Some(_) = lines.by_ref().find(|line| {
            let line = line.trim();
            (line.starts_with(&format!("pub struct {name} "))
                || line.starts_with(&format!("pub enum {name} ")))
                && line.ends_with('{')
        }) else {
            continue;
        };
        let mut depth = 1usize;
        let mut members = Vec::new();
        for line in lines {
            let line = line.trim();
            if line.starts_with("//") || line.is_empty() {
                continue;
            }
            if depth == 1 && !line.starts_with('#') && !line.starts_with('}') {
                let head = line.strip_prefix("pub ").unwrap_or(line);
                let ident: String = head
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !ident.is_empty() {
                    members.push(ident);
                }
            }
            depth += line.matches('{').count();
            depth -= line.matches('}').count().min(depth);
            if depth == 0 {
                return Some(members);
            }
        }
    }
    None
}

/// Each declaration and the one Rust type that carries it member for member, or `None` for a
/// declaration the crate carries across more than one type (checked by the unit's case by name).
const CARRIERS: &[(&str, Option<&str>)] = &[
    ("ekr.store.StorageClass", Some("StorageClass")),
    (
        "ekr.store.PublicationCommandKind",
        Some("PublicationCommandKind"),
    ),
    (
        "ekr.store.PublicationCommandKey",
        Some("PublicationCommandKey"),
    ),
    ("ekr.store.PublicationObject", Some("PublicationObject")),
    ("ekr.store.Publication", Some("Publication")),
    ("ekr.store.NativeExpectedKind", Some("NativeExpectedKind")),
    ("ekr.store.NativeExpected", Some("NativeExpected")),
    ("ekr.store.NativeStreamId", Some("NativeStreamId")),
    ("ekr.store.NativeNewEvent", Some("NativeNewEvent")),
    ("ekr.store.NativeStreamAppend", Some("NativeStreamAppend")),
    ("ekr.store.NativeClaim", Some("NativeClaim")),
    ("ekr.store.NativeCommandMeta", Some("NativeCommandMeta")),
    ("ekr.store.NativeBlobWrite", Some("NativeBlobWrite")),
    (
        "ekr.store.NativePublicationRequest",
        Some("NativePublicationRequest"),
    ),
    ("ekr.store.PublicationPreparationFormatV1", None),
    (
        "ekr.store.PublicationPreparationV1",
        Some("PublicationPreparationV1"),
    ),
    ("ekr.store.PublicationResolution", None),
    ("ekr.store.StoredObject", Some("StoredObject")),
];

/// A declaration is seen whether its mapping opens with `name:` or with any other key.
///
/// `- kind: struct` followed by `name: ekr.store.Snapshot` is the same YAML as the order the file
/// uses today; the unit's scanner reads only lines beginning `  - name: `, so such a declaration
/// is invisible to it and a type nothing implements passes.
#[test]
fn every_store_declaration_names_a_carrier_whatever_key_its_mapping_opens_with() {
    let declared: Vec<Declaration> = declarations(&domain_text());
    let unnamed: Vec<&Declaration> = declared.iter().filter(|d| d.name.is_none()).collect();
    assert!(unnamed.is_empty(), "declarations with no name: {unnamed:?}");
    let declared: BTreeSet<String> = declared.into_iter().filter_map(|d| d.name).collect();
    let bound: BTreeSet<String> = CARRIERS.iter().map(|(d, _)| (*d).to_owned()).collect();
    assert_eq!(
        declared, bound,
        "store.yaml declarations and the carrier table disagree"
    );
}

/// Every bound declaration and its carrier agree member for member: a struct's fields (with an
/// entity's identity field) and an enum's variants.
///
/// The unit's case only asks that a type of the carrier's name exists, so a field added to
/// `ekr.store.NativeClaim` in the domain, or dropped from `NativeClaim` in the crate, is green there.
#[test]
fn every_store_declaration_agrees_member_for_member_with_its_carrier() {
    let declared: BTreeMap<String, Declaration> = declarations(&domain_text())
        .into_iter()
        .filter_map(|d| d.name.clone().map(|name| (name, d)))
        .collect();
    let mut compared = 0usize;
    for (declaration, carrier) in CARRIERS {
        let Some(carrier) = carrier else { continue };
        let found = declared
            .get(*declaration)
            .unwrap_or_else(|| panic!("store.yaml declares {declaration}"));
        let mut domain: BTreeSet<String> = found.members.iter().cloned().collect();
        domain.extend(found.identity.clone());
        let rust: BTreeSet<String> = rust_members(carrier)
            .unwrap_or_else(|| panic!("the crate declares {carrier} with a braced body"))
            .into_iter()
            .collect();
        assert_eq!(
            domain, rust,
            "{declaration} ({:?}) and {carrier} disagree member for member",
            found.kind
        );
        compared += 1;
    }
    assert!(compared >= 15, "the comparison is vacuous: {compared}");
}
