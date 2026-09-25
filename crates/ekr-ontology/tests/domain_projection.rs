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
//! The durable format activation closes that historical gap with a recursive projection.
//! The case below now checks its actual parameter carriers and the matching strict codec.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{Canonical, TypeId};
use ekr_ontology::{ValueKind, ValueType};

/// `systems/ekr/domains/ontology.yaml`, as text.
fn domain_text() -> String {
    let path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("../../systems/ekr/domains/ontology.yaml");
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

/// Replacement for the historical partial-projection assertion: every compound parameter has
/// a typed recursive carrier, and nested parameters survive the current codec and hash.
#[test]
fn the_domain_and_current_codec_retain_all_recursive_compound_parameters() {
    let fields = fields_of("ekr.ontology.PropertyDefinition");
    assert!(fields.contains("value_type"));
    assert_eq!(
        fields_of("ekr.ontology.ValueTypeProjection"),
        BTreeSet::from(
            ["kind", "allowed_types", "variants", "element", "fields"].map(str::to_owned)
        )
    );
    let declarations = declared_fields();
    for (owner, field, ty) in [
        (
            "ekr.ontology.PropertyDefinition",
            "value_type",
            "ekr.ontology.ValueTypeProjection",
        ),
        (
            "ekr.ontology.ValueTypeProjection",
            "allowed_types",
            "Optional<List<ekr.ontology.TypeId>>",
        ),
        (
            "ekr.ontology.ValueTypeProjection",
            "variants",
            "Optional<List<String>>",
        ),
        (
            "ekr.ontology.ValueTypeProjection",
            "element",
            "Optional<ekr.ontology.ValueTypeProjection>",
        ),
        (
            "ekr.ontology.ValueTypeProjection",
            "fields",
            "Optional<Map<String, ekr.ontology.ValueTypeProjection>>",
        ),
    ] {
        assert!(
            declarations.contains(&(owner.into(), field.into(), ty.into())),
            "missing {owner}.{field}: {ty}"
        );
    }
    let original = ValueType::List(Box::new(ValueType::Record(BTreeMap::from([
        (
            "link".into(),
            ValueType::NodeRef {
                allowed_types: BTreeSet::from([TypeId::mint()]),
            },
        ),
        (
            "kind".into(),
            ValueType::Enum {
                variants: BTreeSet::from(["alpha".into(), "beta".into()]),
            },
        ),
    ]))));
    let encoded = serde_json::to_vec(&original).unwrap();
    let decoded: ValueType = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(decoded.canonical_bytes(), original.canonical_bytes());
    assert_ne!(
        ValueType::List(Box::new(ValueType::String)).canonical_bytes(),
        original.canonical_bytes()
    );
}

/// Every declaration of the domain, as `(type name, field name, declared type)`.
fn declared_fields() -> Vec<(String, String, String)> {
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&domain_text()).expect("the ESS domain parses");
    let mut found = Vec::new();
    for declared in ["types", "entities", "views"]
        .into_iter()
        .filter_map(|section| document.get(section))
        .filter_map(serde_yaml_ng::Value::as_sequence)
        .flatten()
    {
        let Some(owner) = declared.get("name").and_then(serde_yaml_ng::Value::as_str) else {
            continue;
        };
        let Some(fields) = declared
            .get("fields")
            .and_then(serde_yaml_ng::Value::as_sequence)
        else {
            continue;
        };
        for field in fields {
            let (Some(name), Some(declared_type)) = (
                field.get("name").and_then(serde_yaml_ng::Value::as_str),
                field.get("type").and_then(serde_yaml_ng::Value::as_str),
            ) else {
                continue;
            };
            found.push((owner.to_owned(), name.to_owned(), declared_type.to_owned()));
        }
    }
    assert!(
        found.len() > 10,
        "the field scan is broken, not the domain: {found:?}"
    );
    found
}

/// Every `.rs` file of this crate's `src/`, as one string.
fn crate_source() -> String {
    let directory = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("src");
    std::fs::read_dir(directory)
        .expect("the crate has a src/")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .map(|path| std::fs::read_to_string(&path).expect("a source file"))
        .collect()
}

/// No field this domain declares as a `Timestamp` is carried here as a bare integer.
///
/// `architecture-decision-record:0004-timestamp-in-ekr-core` made `Timestamp` an `ekr-core`
/// newtype over `i64` milliseconds *because* `ontology.yaml` declares one and `ekr-ontology` does
/// not depend on `ekr-graph`, where the roadmap had put the type: one domain scalar would
/// otherwise have two unrelated Rust representations, which is the drift this file exists to
/// catch.
///
/// The check is over the *class* rather than over `created_at`: the domain is read for every field
/// it declares as a `Timestamp`, and each is required to be a `Timestamp` here. A later domain
/// field cannot arrive as a bare `i64` and wait for a reviewer to notice.
#[test]
fn every_timestamp_the_domain_declares_is_carried_as_a_timestamp() {
    let source = crate_source();
    let mut checked = 0usize;

    for (owner, field, declared_type) in declared_fields() {
        if !declared_type.contains("Timestamp") {
            continue;
        }
        checked += 1;
        let declaration = format!("pub {field}: ");
        let at = source.find(&declaration).unwrap_or_else(|| {
            panic!("{owner}.{field} is declared {declared_type} and this crate declares no field named {field}")
        });
        let carried = source[at + declaration.len()..]
            .lines()
            .next()
            .expect("a field declaration is a line")
            .trim_end_matches(',')
            .trim();
        assert!(
            carried.contains("Timestamp"),
            "{owner}.{field} is declared {declared_type} and this crate carries it as {carried:?}. \
             ADR 0004 makes ekr_core::Timestamp the one representation of that scalar."
        );
    }

    assert!(
        checked > 0,
        "the domain declares no Timestamp field, so this case is checking nothing — \
         ontology.yaml moved and ADR 0004's premise with it"
    );
}

/// The runtime value of a `Timestamp` property is a `Timestamp` too, not only the schema field.
///
/// `ekr.ontology.ValueKind` declares `Timestamp` as one of the kinds a property may have, and the
/// value that inhabits it is `Value::Timestamp`. That variant has no field name, so the case above
/// cannot see it; this one names it directly. Both halves of ADR 0004's adoption are then held:
/// the schema's own `created_at`, and the values the schema describes.
#[test]
fn a_timestamp_value_carries_a_timestamp() {
    let source = crate_source();
    assert!(
        source.contains("Timestamp(Timestamp)"),
        "Value::Timestamp carries a bare integer; ADR 0004 adopts ekr_core::Timestamp there too"
    );
    assert!(
        !source.contains("Timestamp(i64)"),
        "a bare epoch integer survives in this crate"
    );
}

/// The field of `ekr.ontology.ValueTypeProjection` that carries a kind's parameters, or `kind`
/// for a kind that has none.
///
/// An exhaustive `match` with no `_` arm, so a twelfth [`ValueKind`] does not compile until
/// somebody says where its parameters go.
const fn carrier(kind: ValueKind) -> &'static str {
    match kind {
        ValueKind::String
        | ValueKind::Boolean
        | ValueKind::Integer
        | ValueKind::Float
        | ValueKind::Decimal
        | ValueKind::Timestamp
        | ValueKind::Duration => "kind",
        ValueKind::NodeRef => "allowed_types",
        ValueKind::Enum => "variants",
        ValueKind::List => "element",
        ValueKind::Record => "fields",
    }
}

/// The variant names of `pub enum ValueKind` in this crate's source, in declaration order.
///
/// Read from the source rather than listed here, so that the crate's half of the comparison below
/// is not a second hand-written list beside the domain's. Comments and literals are blanked first
/// and the enumeration's braces are matched, so a `}` in a doc comment — `{ name: type }` in a
/// variant's description — neither ends the body early nor turns into a variant (adversary pass 1,
/// wave p1-14).
fn crate_value_kinds() -> Vec<String> {
    let code = code_only(&crate_source());
    let head = "pub enum ValueKind {";
    let open = code
        .find(head)
        .expect("the crate declares `pub enum ValueKind`")
        + head.len()
        - 1;
    let mut depth = 0usize;
    let close = code[open..]
        .char_indices()
        .find(|&(_, c)| {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            depth == 0
        })
        .map(|(at, _)| open + at)
        .expect("the enumeration's braces balance");
    let mut kinds = Vec::new();
    for entry in code[open + 1..close].split(',') {
        let mut entry = entry.trim();
        // Attributes before the variant: `#[…]`, balanced.
        while let Some(rest) = entry.strip_prefix("#[") {
            let mut depth = 1usize;
            let end = rest
                .char_indices()
                .find(|&(_, c)| {
                    match c {
                        '[' => depth += 1,
                        ']' => depth -= 1,
                        _ => {}
                    }
                    depth == 0
                })
                .map(|(at, _)| at)
                .expect("an attribute closes");
            entry = rest[end + 1..].trim_start();
        }
        if entry.is_empty() {
            continue;
        }
        assert!(
            entry.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "ValueKind has a variant this scan cannot read, so the scan is broken: {entry:?}"
        );
        kinds.push(entry.to_owned());
    }
    assert!(
        kinds.len() >= 4,
        "the variant scan is broken, not the crate: {kinds:?}"
    );
    kinds
}

/// `text` with every comment and the contents of every string and character literal replaced by
/// spaces of the same byte length, newlines kept, so a brace or comma inside one is not read as
/// structure and a byte offset into the result is one into `text`.
fn code_only(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let blank = |out: &mut String, c: char| {
        if c == '\n' {
            out.push('\n');
        } else {
            out.extend(std::iter::repeat_n(' ', c.len_utf8()));
        }
    };
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
        if c == '/' && next == Some('/') {
            while at < chars.len() && chars[at] != '\n' {
                blank(&mut out, chars[at]);
                at += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0usize;
            while at < chars.len() {
                if chars[at] == '/' && chars.get(at + 1) == Some(&'*') {
                    depth += 1;
                    blank(&mut out, '/');
                    blank(&mut out, '*');
                    at += 2;
                } else if chars[at] == '*' && chars.get(at + 1) == Some(&'/') {
                    depth -= 1;
                    blank(&mut out, '*');
                    blank(&mut out, '/');
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    blank(&mut out, chars[at]);
                    at += 1;
                }
            }
        } else if c == 'r'
            && (next == Some('"') || next == Some('#'))
            && !chars
                .get(at.wrapping_sub(1))
                .is_some_and(|p| p.is_alphanumeric() || *p == '_')
        {
            // A raw string: `r"…"` or `r#"…"#`, closed by a quote and as many hashes.
            let mut hashes = 0;
            let mut probe = at + 1;
            while chars.get(probe) == Some(&'#') {
                hashes += 1;
                probe += 1;
            }
            if chars.get(probe) != Some(&'"') {
                out.push(c);
                at += 1;
                continue;
            }
            for &k in &chars[at..=probe] {
                out.push(k);
            }
            at = probe + 1;
            while at < chars.len() {
                if chars[at] == '"' && (1..=hashes).all(|h| chars.get(at + h) == Some(&'#')) {
                    out.push('"');
                    out.extend(std::iter::repeat_n('#', hashes));
                    at += 1 + hashes;
                    break;
                }
                blank(&mut out, chars[at]);
                at += 1;
            }
        } else if c == '"' {
            out.push('"');
            at += 1;
            while at < chars.len() && chars[at] != '"' {
                if chars[at] == '\\' {
                    blank(&mut out, '\\');
                    at += 1;
                }
                if at < chars.len() {
                    blank(&mut out, chars[at]);
                    at += 1;
                }
            }
            if at < chars.len() {
                out.push('"');
                at += 1;
            }
        } else if c == '\'' {
            // A character literal is `'x'` or `'\…'`; anything else is a lifetime.
            let close = if next == Some('\\') {
                (at + 2..chars.len().min(at + 12)).find(|&k| chars[k] == '\'')
            } else if chars.get(at + 2) == Some(&'\'') {
                Some(at + 2)
            } else {
                None
            };
            if let Some(close) = close {
                out.push('\'');
                for &k in &chars[at + 1..close] {
                    blank(&mut out, k);
                }
                out.push('\'');
                at = close + 1;
            } else {
                out.push(c);
                at += 1;
            }
        } else {
            out.push(c);
            at += 1;
        }
    }
    assert_eq!(out.len(), text.len(), "blanking preserves byte offsets");
    out
}

/// `ekr.ontology.ValueKind` is exactly the crate's [`ValueKind`], and every kind has its carrier
/// in `ekr.ontology.ValueTypeProjection`.
///
/// Three directions, each of which the case above cannot see because its field list is
/// hand-written:
///
/// * a kind the domain declares and the crate does not, or the reverse, fails the set equality;
/// * a kind whose carrier field the domain stops declaring fails the carrier lookup;
/// * a projection field no kind is carried by — a carrier left behind after its kind went — fails
///   the coverage check, as does two compound kinds sharing one parameter field.
#[test]
fn every_value_kind_the_domain_declares_is_the_crates_and_has_its_carrier() {
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&domain_text()).expect("the ESS domain parses");
    let declared: Vec<String> = document
        .get("types")
        .and_then(serde_yaml_ng::Value::as_sequence)
        .into_iter()
        .flatten()
        .find(|declared| {
            declared.get("name").and_then(serde_yaml_ng::Value::as_str)
                == Some("ekr.ontology.ValueKind")
        })
        .expect("the domain declares ekr.ontology.ValueKind under types:")
        .get("variants")
        .and_then(serde_yaml_ng::Value::as_sequence)
        .expect("ekr.ontology.ValueKind is an enumeration")
        .iter()
        .map(|variant| variant.as_str().expect("a variant is a name").to_owned())
        .collect();

    assert_eq!(
        declared.iter().collect::<BTreeSet<_>>(),
        crate_value_kinds().iter().collect::<BTreeSet<_>>(),
        "ekr.ontology.ValueKind and the crate's ValueKind disagree"
    );

    let projection = fields_of("ekr.ontology.ValueTypeProjection");
    let mut carried = BTreeSet::new();
    let mut parameter_carriers = Vec::new();
    for name in &declared {
        let kind: ValueKind = serde_json::from_value(serde_json::Value::String(name.clone()))
            .unwrap_or_else(|error| panic!("{name} is not a ValueKind: {error}"));
        assert_eq!(
            &kind.to_string(),
            name,
            "{kind:?} displays as its domain name"
        );
        let field = carrier(kind);
        assert!(
            projection.contains(field),
            "{name} is carried by ValueTypeProjection.{field}, which the domain does not declare"
        );
        carried.insert(field.to_owned());
        if field != "kind" {
            parameter_carriers.push(field);
        }
    }
    assert_eq!(
        carried, projection,
        "every field of ekr.ontology.ValueTypeProjection carries some kind"
    );
    assert_eq!(
        parameter_carriers.len(),
        parameter_carriers.iter().collect::<BTreeSet<_>>().len(),
        "no two compound kinds share a parameter field: {parameter_carriers:?}"
    );
}
