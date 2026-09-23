//! Adversary, unit p1-14-bindings, pass 2.
//!
//! `domain_projection.rs::every_type_and_entity_the_domain_declares_names_a_rust_carrier` compares
//! each `store.yaml` declaration with its Rust carrier by **member name**, the Rust identifier as
//! written. Two drifts leave it green:
//!
//! * a field's declared `type:` changes on one side only — `byte_len: Integer` becomes `String`
//!   in the domain while the carrier keeps `u64`;
//! * a `#[serde(rename …)]` (or `rename_all`, `alias`, `flatten`, …) on a carrier, which moves the
//!   name the field is *written under* away from the domain's while the Rust identifier the case
//!   reads stays the same.
//!
//! The two cases below close those. Like the unit's case they resolve every path through the
//! runtime `CARGO_MANIFEST_DIR`, so they can be pointed at a mirror of the tree.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_yaml_ng::Value as Yaml;

fn manifest_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
}

fn domain() -> Yaml {
    let text = std::fs::read_to_string(manifest_dir().join("../../systems/ekr/domains/store.yaml"))
        .expect("the ESS domain is beside the crates");
    serde_yaml_ng::from_str(&text).expect("the ESS domain parses")
}

/// Every current module of `src/`, excluding the frozen legacy codec, as the unit's case does.
fn crate_sources() -> Vec<String> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(manifest_dir().join("src"))
        .expect("the crate has a src/")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.extension().is_some_and(|e| e == "rs")
                && path.file_name().is_some_and(|file| file != "legacy.rs")
        })
        .collect();
    paths.sort();
    assert!(paths.len() >= 5, "the module scan is broken: {paths:?}");
    paths
        .into_iter()
        .map(|path| std::fs::read_to_string(path).expect("a source file"))
        .collect()
}

/// The struct carriers the unit's `BINDINGS` names as `Carrier::Whole`, by declaration.
const STRUCT_CARRIERS: &[(&str, &str)] = &[
    ("ekr.store.PublicationCommandKey", "PublicationCommandKey"),
    ("ekr.store.PublicationObject", "PublicationObject"),
    ("ekr.store.Publication", "Publication"),
    ("ekr.store.NativeExpected", "NativeExpected"),
    ("ekr.store.NativeStreamId", "NativeStreamId"),
    ("ekr.store.NativeNewEvent", "NativeNewEvent"),
    ("ekr.store.NativeStreamAppend", "NativeStreamAppend"),
    ("ekr.store.NativeClaim", "NativeClaim"),
    ("ekr.store.NativeCommandMeta", "NativeCommandMeta"),
    ("ekr.store.NativeBlobWrite", "NativeBlobWrite"),
    (
        "ekr.store.NativePublicationRequest",
        "NativePublicationRequest",
    ),
    (
        "ekr.store.PublicationPreparationV1",
        "PublicationPreparationV1",
    ),
    ("ekr.store.StoredObject", "StoredObject"),
];

/// The enum carriers, whose variant names are what the domain declares.
const ENUM_CARRIERS: &[&str] = &[
    "StorageClass",
    "PublicationCommandKind",
    "NativeExpectedKind",
    "Appended",
];

/// Fields whose Rust type is deliberately not the domain's spelling of it, and why.
const TYPE_EXCEPTIONS: &[(&str, &str, &str)] = &[
    // `ekr_graph::RevisionEvent::FORMAT` is `ekr.revision-event/2`: the V2 carrier.
    ("ekr.store.Publication", "event", "RevisionEvent"),
    // `ess/1` keys every map by `String`; the key is the hash's text.
    (
        "ekr.store.Publication",
        "objects",
        "Map<ContentHash, PublicationObject>",
    ),
    // The one-variant format enumeration is carried as the string constant it names.
    ("ekr.store.PublicationPreparationV1", "format", "String"),
];

/// The lines of `pub struct name { … }` or `pub enum name { … }`, with the attribute lines directly
/// above the item, as `(preamble, body)`.
fn item(name: &str) -> (Vec<String>, Vec<String>) {
    for source in crate_sources() {
        let lines: Vec<&str> = source.lines().collect();
        let Some(head) = lines.iter().position(|line| {
            let line = line.trim();
            ["pub struct ", "pub enum "].iter().any(|keyword| {
                line.strip_prefix(keyword)
                    .and_then(|rest| rest.strip_prefix(name))
                    .is_some_and(|rest| rest.trim_start().starts_with('{'))
            })
        }) else {
            continue;
        };
        let preamble = lines[..head]
            .iter()
            .rev()
            .map(|line| line.trim())
            .take_while(|line| line.starts_with("#["))
            .map(str::to_owned)
            .collect();
        let body = lines[head + 1..]
            .iter()
            .map(|line| line.trim())
            .take_while(|line| *line != "}")
            .map(str::to_owned)
            .collect();
        return (preamble, body);
    }
    panic!("the crate declares no `pub struct {name} {{` or `pub enum {name} {{`");
}

/// `(field, type)` for every `pub field: Type,` line of a struct body.
fn rust_fields(body: &[String]) -> BTreeMap<String, String> {
    body.iter()
        .filter_map(|line| line.strip_prefix("pub "))
        .filter_map(|line| line.split_once(": "))
        .map(|(field, ty)| (field.to_owned(), ty.trim_end_matches(',').to_owned()))
        .collect()
}

/// The top-level comma-separated arguments of a generic's `<…>` contents.
fn arguments(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (at, c) in text.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                out.push(text[start..at].trim());
                start = at + 1;
            }
            _ => {}
        }
    }
    out.push(text[start..].trim());
    out
}

/// A Rust field type in the domain's vocabulary.
fn rust_as_domain(ty: &str) -> String {
    let ty = ty.trim();
    let generic = |outer: &str| {
        ty.strip_prefix(outer)
            .and_then(|rest| rest.strip_prefix('<'))
            .and_then(|rest| rest.strip_suffix('>'))
    };
    if let Some(inner) = generic("Option") {
        return format!("Optional<{}>", rust_as_domain(inner));
    }
    if ty == "Vec<u8>" {
        return "Bytes".to_owned();
    }
    if let Some(inner) = generic("Vec") {
        return format!("List<{}>", rust_as_domain(inner));
    }
    if let Some(inner) = generic("BTreeMap") {
        let parts = arguments(inner);
        return format!(
            "Map<{}, {}>",
            rust_as_domain(parts[0]),
            rust_as_domain(parts[1])
        );
    }
    if matches!(
        ty,
        "u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize"
    ) {
        return "Integer".to_owned();
    }
    ty.rsplit("::").next().unwrap_or(ty).to_owned()
}

/// A domain field type with its qualified names reduced to their last segment.
fn domain_bare(ty: &str) -> String {
    let ty = ty.trim();
    for outer in ["Optional", "List", "Map"] {
        if let Some(inner) = ty
            .strip_prefix(outer)
            .and_then(|rest| rest.strip_prefix('<'))
            .and_then(|rest| rest.strip_suffix('>'))
        {
            let parts: Vec<String> = arguments(inner).into_iter().map(domain_bare).collect();
            return format!("{outer}<{}>", parts.join(", "));
        }
    }
    ty.rsplit('.').next().unwrap_or(ty).to_owned()
}

/// `(field, type)` for every field of a declaration, with an entity's identity among them.
fn domain_fields(declaration: &str) -> BTreeMap<String, String> {
    let document = domain();
    let declared = ["types", "entities"]
        .into_iter()
        .filter_map(|section| document.get(section))
        .filter_map(Yaml::as_sequence)
        .flatten()
        .find(|declared| declared.get("name").and_then(Yaml::as_str) == Some(declaration))
        .unwrap_or_else(|| panic!("the domain declares no {declaration}"));
    let field = |value: &Yaml| {
        (
            value
                .get("name")
                .and_then(Yaml::as_str)
                .expect("a field has a name")
                .to_owned(),
            value
                .get("type")
                .and_then(Yaml::as_str)
                .expect("a field has a type")
                .to_owned(),
        )
    };
    let mut fields: BTreeMap<String, String> = declared
        .get("fields")
        .and_then(Yaml::as_sequence)
        .into_iter()
        .flatten()
        .map(field)
        .collect();
    if let Some(identity) = declared.get("identity") {
        let (name, ty) = field(identity);
        fields.insert(name, ty);
    }
    fields
}

/// Every field of every struct carrier has the type the domain declares for it, in the domain's
/// vocabulary, save the three the exception table names and says why.
#[test]
fn every_store_field_is_carried_with_the_type_the_domain_declares() {
    let mut compared = 0usize;
    for (declaration, carrier) in STRUCT_CARRIERS {
        let declared = domain_fields(declaration);
        let carried = rust_fields(&item(carrier).1);
        for (field, domain_type) in &declared {
            let rust_type = carried
                .get(field)
                .unwrap_or_else(|| panic!("{carrier} has no `pub {field}:` line"));
            let exception = TYPE_EXCEPTIONS
                .iter()
                .find(|(owner, name, _)| owner == declaration && name == field);
            let want = match exception {
                Some((_, _, carried_as)) => (*carried_as).to_owned(),
                None => domain_bare(domain_type),
            };
            assert_eq!(
                rust_as_domain(rust_type),
                want,
                "{declaration}.{field} is declared `{domain_type}` and {carrier} carries `{rust_type}`"
            );
            compared += 1;
        }
    }
    assert!(compared >= 40, "the field scan is broken: {compared}");
}

/// The serde entries that change the name, shape or presence of a member on the wire.
const WIRE_CHANGING: &[&str] = &[
    "rename",
    "rename_all",
    "rename_all_fields",
    "alias",
    "flatten",
    "skip",
    "skip_serializing",
    "skip_serializing_if",
    "with",
    "serialize_with",
    "tag",
    "untagged",
    "content",
    "transparent",
    "into",
    "from",
    "try_from",
    "remote",
];

/// The top-level entry keys of every `#[serde(…)]` in `lines`.
fn serde_keys(lines: &[String]) -> Vec<String> {
    let mut keys = Vec::new();
    for line in lines {
        let Some(inner) = line
            .strip_prefix("#[serde(")
            .and_then(|rest| rest.strip_suffix(")]"))
        else {
            continue;
        };
        let mut depth = 0usize;
        let mut in_string = false;
        let mut entry = String::new();
        for c in inner.chars().chain(std::iter::once(',')) {
            match c {
                '"' => in_string = !in_string,
                '(' if !in_string => depth += 1,
                ')' if !in_string => depth -= 1,
                ',' if !in_string && depth == 0 => {
                    let key: String = entry
                        .trim()
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    keys.push(key);
                    entry.clear();
                    continue;
                }
                _ => {}
            }
            entry.push(c);
        }
    }
    keys
}

/// No carrier the unit binds writes a member under a name other than the Rust identifier the
/// unit's case compares with the domain.
#[test]
fn every_store_carrier_is_written_under_the_member_names_the_binding_compares() {
    let carriers: Vec<&str> = STRUCT_CARRIERS
        .iter()
        .map(|(_, carrier)| *carrier)
        .chain(ENUM_CARRIERS.iter().copied())
        .collect();
    for carrier in carriers {
        let (preamble, body) = item(carrier);
        for key in serde_keys(&preamble).into_iter().chain(serde_keys(&body)) {
            assert!(
                !WIRE_CHANGING.contains(&key.as_str()),
                "{carrier} carries `#[serde({key} …)]`, so the name it is written under is not \
                 the identifier the binding case compares with store.yaml"
            );
        }
    }
}
