//! Every YAML entry point is classified, including imports that could hide a new reader.
//! This guard is an inventory, not a proof of control flow; behavior cases hold each input bound.

use std::collections::BTreeMap;
use std::path::Path;

type Inventory = BTreeMap<(String, String), usize>;

// A small lexical scan skips comments and string/character literals: examples in documentation
// and fixture text cannot admit an executable reader. Imports are retained as whole statements.
fn tokens(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut at = 0;
    let mut out = Vec::new();
    while at < bytes.len() {
        if bytes[at..].starts_with(b"//") {
            while at < bytes.len() && bytes[at] != b'\n' {
                at += 1;
            }
        } else if bytes[at..].starts_with(b"/*") {
            at += 2;
            let mut depth = 1;
            while at < bytes.len() && depth > 0 {
                if bytes[at..].starts_with(b"/*") {
                    depth += 1;
                    at += 2;
                } else if bytes[at..].starts_with(b"*/") {
                    depth -= 1;
                    at += 2;
                } else {
                    at += 1;
                }
            }
        } else if bytes[at] == b'r' && {
            let mut end = at + 1;
            while bytes.get(end) == Some(&b'#') {
                end += 1;
            }
            bytes.get(end) == Some(&b'"')
        } {
            let start = at;
            at += 1;
            while bytes.get(at) == Some(&b'#') {
                at += 1;
            }
            let hashes = at - start - 1;
            at += 1;
            while at < bytes.len() {
                if bytes[at] == b'"'
                    && bytes
                        .get(at + 1..at + 1 + hashes)
                        .is_some_and(|tail| tail.iter().all(|b| *b == b'#'))
                {
                    at += 1 + hashes;
                    break;
                }
                at += 1;
            }
        } else if bytes[at] == b'\''
            && (bytes.get(at + 2) == Some(&b'\'')
                || (bytes.get(at + 1) == Some(&b'\\') && bytes.get(at + 3) == Some(&b'\'')))
        {
            at += if bytes.get(at + 1) == Some(&b'\\') {
                4
            } else {
                3
            };
        } else if bytes[at] == b'"' {
            at += 1;
            while at < bytes.len() {
                if bytes[at] == b'\\' {
                    at = (at + 2).min(bytes.len());
                } else if bytes[at] == b'"' {
                    at += 1;
                    break;
                } else {
                    at += 1;
                }
            }
        } else if bytes[at].is_ascii_alphabetic() || bytes[at] == b'_' {
            let start = at;
            at += 1;
            while at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_') {
                at += 1;
            }
            out.push(text[start..at].to_owned());
        } else {
            if !bytes[at].is_ascii_whitespace() {
                out.push(char::from(bytes[at]).to_string());
            }
            at += 1;
        }
    }
    out
}

fn references(path: &str, text: &str, inventory: &mut Inventory) {
    let tokens = tokens(text);
    for (at, token) in tokens.iter().enumerate() {
        if token != "serde_yaml_ng" && token != "Documents" {
            continue;
        }
        let start = tokens[..at]
            .iter()
            .rposition(|t| matches!(t.as_str(), ";" | "{" | "}"))
            .map_or(0, |i| i + 1);
        let import = tokens[start..at]
            .iter()
            .any(|t| t == "use" || t == "extern");
        // `Documents` in a serde_yaml_ng import is already covered by that full import.
        if token == "Documents" && import {
            continue;
        }
        if token == "Documents" && tokens.get(at + 1).map(String::as_str) != Some(":") {
            continue;
        }
        let mut end = at + 1;
        if import {
            while end < tokens.len() && tokens[end] != ";" {
                end += 1;
            }
        } else {
            while tokens.get(end).map(String::as_str) == Some(":")
                && tokens.get(end + 1).map(String::as_str) == Some(":")
            {
                end += 2;
                if tokens
                    .get(end)
                    .is_some_and(|t| t.as_bytes()[0].is_ascii_alphabetic() || t.starts_with('_'))
                {
                    end += 1;
                } else {
                    break;
                }
            }
        }
        let signature = tokens[at..end].join(" ");
        *inventory.entry((path.into(), signature)).or_default() += 1;
    }
}

fn sources(root: &Path, dir: &Path, inventory: &mut Inventory) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(root, &path, inventory);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            references(
                path.strip_prefix(root).unwrap().to_str().unwrap(),
                &std::fs::read_to_string(&path).unwrap(),
                inventory,
            );
        }
    }
}

fn classified() -> Inventory {
    let mut expected = Inventory::new();
    // Each entry below must identify its real caller and the policy before adding a reader.
    let entries: &[(&str, &[(&str, usize)])] = &[
        // Alias-refusing input facade: extraction and SDK extraction callers set byte/depth caps.
        (
            "crates/ekr-core/src/decode.rs",
            &[("serde_yaml_ng : : observation : : Event", 1)],
        ),
        // Kernel seed and transaction callers cap bytes before loading; bounded tape precedes
        // typed deserialization. shape.rs expands transaction aliases under its own budgets.
        (
            "crates/ekr-core/src/decode/yaml.rs",
            &[
                (
                    "serde_yaml_ng : : observation : : { Document , Documents , Event }",
                    1,
                ),
                ("serde_yaml_ng : : Error", 1),
                ("Documents : : from_str_within_depth", 1),
            ],
        ),
        (
            "crates/ekr-kernel/src/yaml.rs",
            &[(
                "serde_yaml_ng : : observation : : { Document , Documents }",
                1,
            )],
        ),
        (
            "crates/ekr-kernel/src/seed.rs",
            &[("serde_yaml_ng : : from_str", 1)],
        ),
        (
            "crates/ekr-kernel/src/document.rs",
            &[
                ("serde_yaml_ng : : Deserializer : : from_str", 1),
                ("serde_yaml_ng : : Error", 1),
            ],
        ),
        (
            "crates/ekr-kernel/src/document/shape.rs",
            &[(
                "serde_yaml_ng : : { observation : : { Document , Event , Tag } , Error , }",
                1,
            )],
        ),
        // Caller input: Ontology::from_yaml observes bytes, depth and expanded alias work first.
        (
            "crates/ekr-ontology/src/schema.rs",
            &[("serde_yaml_ng : : from_str", 1)],
        ),
        (
            "crates/ekr-integrate/src/extraction.rs",
            &[("serde_yaml_ng : : from_str", 1)],
        ),
        // SDK extraction observes caller input before typed decoding or unsupported_source's
        // diagnostic-only Value decode. Transaction limits only decode SDK-generated to_yaml.
        (
            "crates/ekr-sdk/src/document/extraction.rs",
            &[
                ("serde_yaml_ng : : from_str", 2),
                ("serde_yaml_ng : : Value", 1),
                ("serde_yaml_ng : : Value : : Tagged", 1),
            ],
        ),
        (
            "crates/ekr-sdk/src/document/limits.rs",
            &[
                ("serde_yaml_ng : : Value", 1),
                ("serde_yaml_ng : : from_str", 1),
            ],
        ),
        (
            "crates/ekr-sdk/src/document/mod.rs",
            &[("serde_yaml_ng : : Error", 1)],
        ),
        (
            "crates/ekr-sdk/src/document/yaml.rs",
            &[("serde_yaml_ng : : to_string", 1)],
        ),
        // Trusted embedded ESS domains (include_str!), never a caller's YAML document.
        (
            "crates/ekr-views/src/code_names.rs",
            &[
                ("serde_yaml_ng : : Value", 1),
                ("serde_yaml_ng : : from_str", 1),
            ],
        ),
        // read() caps caller bytes and runs nesting() before decode(); the event reader refuses
        // aliases and tags without expansion, then checks the exact typed-reference shape.
        (
            "crates/ekr/src/cli/resolve.rs",
            &[
                (
                    "serde_yaml_ng : : observation : : { Documents , Event , ScalarKind }",
                    1,
                ),
                ("serde_yaml_ng : : Error", 1),
                ("Documents : : from_str", 1),
            ],
        ),
        // These remaining references are inside cfg(test) modules, using locally built fixtures.
        // They are still enumerated so a new product use in the same file cannot hide behind it.
        (
            "crates/ekr-kernel/src/replay.rs",
            &[
                ("serde_yaml_ng : : to_string", 2),
                ("serde_yaml_ng : : Value", 1),
                ("serde_yaml_ng : : from_slice", 1),
                ("serde_yaml_ng : : from_value", 1),
            ],
        ),
        (
            "crates/ekr-kernel/src/checkpoint.rs",
            &[("serde_yaml_ng : : to_string", 1)],
        ),
        (
            "crates/ekr-kernel/src/document/checked.rs",
            &[("serde_yaml_ng : : Deserializer : : from_str", 2)],
        ),
    ];
    for (path, signatures) in entries {
        for (signature, count) in *signatures {
            expected.insert(((*path).into(), (*signature).into()), *count);
        }
    }
    expected
}

#[test]
fn every_production_yaml_reader_has_an_explicit_input_policy() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&manifest).parent().unwrap().parent().unwrap();
    let mut found = Inventory::new();
    for entry in std::fs::read_dir(root.join("crates")).unwrap() {
        let src = entry.unwrap().path().join("src");
        if src.is_dir() {
            sources(root, &src, &mut found);
        }
    }
    // The other workspace member has no YAML reader today; a future tooling reader must not
    // escape classification merely because its source lives outside crates/.
    sources(root, &root.join("xtask/src"), &mut found);
    assert_eq!(
        found,
        classified(),
        "unclassified YAML reference: identify its caller and enforce input bounds before admission"
    );
}

#[test]
fn the_yaml_inventory_rejects_new_readers_and_import_aliases() {
    for input in [
        "fn read(s: &str) { serde_yaml_ng::from_str(s); }",
        "fn read(s: &[u8]) { serde_yaml_ng::from_slice(s); }",
        "fn read(s: &str) { serde_yaml_ng::Deserializer::from_str(s); }",
        "use serde_yaml_ng::from_reader as read;",
        "use serde_yaml_ng::{from_str as read, Value};",
        "use serde_yaml_ng::*;",
        "extern crate serde_yaml_ng as yaml;",
        "fn read(s: &str) { Documents::from_str(s); }",
    ] {
        let mut found = classified();
        references("crates/new-reader/src/lib.rs", input, &mut found);
        assert_ne!(found, classified(), "new reader escaped: {input}");
        let mut found = classified();
        references("xtask/src/reader.rs", input, &mut found);
        assert_ne!(found, classified(), "new tooling reader escaped: {input}");
        let mut found = classified();
        references("crates/ekr-ontology/src/schema.rs", input, &mut found);
        assert_ne!(
            found,
            classified(),
            "an extra reader in an existing module escaped: {input}"
        );
    }
    let mut ignored = Inventory::new();
    references(
        "example",
        "// serde_yaml_ng::from_str(s)\n/* outer /* serde_yaml_ng */ */\nlet s = r###\"serde_yaml_ng::from_str(s)\"###; let t = \"Documents::from_str(s)\";",
        &mut ignored,
    );
    assert!(
        ignored.is_empty(),
        "prose must not classify executable code: {ignored:?}"
    );
}
