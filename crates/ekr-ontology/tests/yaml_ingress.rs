//! Every YAML entry point is classified, including imports that could hide a new reader.
//! This guard is an inventory, not a proof of control flow; behavior cases hold each input bound.

use std::collections::{BTreeMap, BTreeSet};
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
    shared_references(path, &tokens, inventory);
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

/// Flatten ordinary Rust use trees, including groups, self, aliases and glob imports. This is
/// lexical bookkeeping, not name/type resolution: ambiguous local aliases are conservatively
/// inventoried and every new import of a shared module itself needs classification.
fn use_paths(tokens: &[String], prefix: &[String], out: &mut Vec<(Vec<String>, String)>) {
    let mut at = 0;
    while at < tokens.len() {
        let mut path = prefix.to_vec();
        while at < tokens.len() && !matches!(tokens[at].as_str(), "{" | "}" | "," | "as") {
            if tokens[at] != ":" && tokens[at] != "self" {
                path.push(tokens[at].clone());
            }
            at += 1;
        }
        if tokens.get(at).map(String::as_str) == Some("{") {
            let begin = at + 1;
            let mut depth = 1;
            at += 1;
            while at < tokens.len() && depth > 0 {
                match tokens[at].as_str() {
                    "{" => depth += 1,
                    "}" => depth -= 1,
                    _ => {}
                }
                at += 1;
            }
            use_paths(&tokens[begin..at - 1], &path, out);
        } else {
            let mut name = path.last().cloned().unwrap_or_default();
            if tokens.get(at).map(String::as_str) == Some("as") {
                name = tokens.get(at + 1).cloned().unwrap_or_default();
                at += 2;
            }
            if !name.is_empty() {
                out.push((path, name));
            }
        }
        if tokens.get(at).map(String::as_str) == Some(",") {
            at += 1;
        } else {
            break;
        }
    }
}

type Aliases = BTreeMap<String, BTreeSet<Vec<String>>>;

fn expand_alias(path: &[String], aliases: &Aliases) -> BTreeSet<Vec<String>> {
    let mut pending = vec![(path.to_vec(), 0)];
    let mut found = BTreeSet::new();
    // Keep every possible local binding: an unrelated import in a different lexical scope
    // cannot conceal a YAML alias. This intentionally errs toward classification, without
    // claiming compiler-equivalent scope resolution. Finite depth also bounds cyclic imports.
    while let Some((path, depth)) = pending.pop() {
        if !found.insert(path.clone()) || depth >= aliases.len() {
            continue;
        }
        if let Some(prefixes) = path.first().and_then(|name| aliases.get(name)) {
            for prefix in prefixes {
                pending.push((
                    prefix.iter().chain(&path[1..]).cloned().collect(),
                    depth + 1,
                ));
            }
        }
    }
    found
}

fn shared_module(path: &[String]) -> bool {
    !path.iter().any(|part| part == "serde_yaml_ng")
        && (path.iter().any(|part| part == "yaml")
            || (path.iter().any(|part| part == "decode")
                && path
                    .last()
                    .is_some_and(|part| part == "observe_yaml" || part == "*")))
}

fn shared_references(file: &str, tokens: &[String], inventory: &mut Inventory) {
    let mut imports = Vec::new();
    let mut imported = vec![false; tokens.len()];
    let mut aliases = Aliases::new();
    if file.ends_with("ekr-core/src/decode/yaml.rs") {
        aliases.insert(
            "load".into(),
            BTreeSet::from([vec![
                "crate".into(),
                "decode".into(),
                "yaml".into(),
                "load".into(),
            ]]),
        );
    }
    for (start, _) in tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| *token == "use")
    {
        let end = start
            + tokens[start..]
                .iter()
                .position(|token| token == ";")
                .unwrap_or(tokens.len() - start);
        imported[start..end].fill(true);
        let mut paths = Vec::new();
        use_paths(&tokens[start + 1..end], &[], &mut paths);
        for (path, name) in &paths {
            if name != "*" {
                aliases
                    .entry(name.clone())
                    .or_default()
                    .insert(path.clone());
            }
        }
        imports.push((start, end, paths));
    }
    // Globs expose the module's reader names; resolving the parent first also covers an aliased
    // module followed by `use tape::*`. No unrelated methods named `load` are inventoried.
    for (_, _, paths) in &imports {
        for (path, name) in paths {
            if name == "*" {
                for parent in expand_alias(&path[..path.len() - 1], &aliases) {
                    let names: &[&str] = match parent.last().map(String::as_str) {
                        Some("yaml") => &["load", "next"],
                        Some("decode") => &["yaml", "observe_yaml"],
                        Some("ekr_core") => &["decode"],
                        _ => &[],
                    };
                    for name in names {
                        let mut value = parent.clone();
                        value.push((*name).into());
                        aliases.entry((*name).into()).or_default().insert(value);
                    }
                }
            }
        }
    }
    for (start, end, paths) in &imports {
        if paths.iter().any(|(path, _)| {
            expand_alias(path, &aliases)
                .iter()
                .any(|path| shared_module(path))
        }) {
            *inventory
                .entry((
                    file.into(),
                    format!("shared import {}", tokens[start + 1..*end].join(" ")),
                ))
                .or_default() += 1;
        }
    }
    let mut at = 0;
    while at < tokens.len() {
        if imported[at]
            || !(tokens[at].as_bytes()[0].is_ascii_alphabetic() || tokens[at].starts_with('_'))
        {
            at += 1;
            continue;
        }
        let start = at;
        let mut path = vec![tokens[at].clone()];
        at += 1;
        while tokens.get(at).map(String::as_str) == Some(":")
            && tokens.get(at + 1).map(String::as_str) == Some(":")
        {
            let Some(next) = tokens.get(at + 2).filter(|token| {
                token.as_bytes()[0].is_ascii_alphabetic() || token.starts_with('_')
            }) else {
                break;
            };
            path.push(next.clone());
            at += 3;
        }
        let reader = expand_alias(&path, &aliases).iter().any(|expanded| {
            shared_module(expanded)
                && expanded
                    .last()
                    .is_some_and(|name| matches!(name.as_str(), "load" | "next" | "observe_yaml"))
        });
        if reader && tokens.get(start.wrapping_sub(1)).map(String::as_str) != Some("fn") {
            *inventory
                .entry((
                    file.into(),
                    format!("shared reader {}", tokens[start..at].join(" ")),
                ))
                .or_default() += 1;
        }
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
            &[
                ("serde_yaml_ng : : observation : : Event", 1),
                ("shared reader yaml : : load", 1),
            ],
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
                // The module's test-only walk uses its own load function with tiny budgets.
                ("shared reader load", 1),
            ],
        ),
        (
            "crates/ekr-kernel/src/yaml.rs",
            &[(
                "serde_yaml_ng : : observation : : { Document , Documents }",
                1,
            ), (
                "shared import ekr_core : : decode : : yaml : : { expand , load , Expansion , Past , Tally }",
                1,
            )],
        ),
        (
            "crates/ekr-kernel/src/seed.rs",
            &[
                ("serde_yaml_ng : : from_str", 1),
                ("shared import crate : : yaml : : { self , Expansion , Past , Tally }", 1),
                ("shared reader yaml : : load", 1),
                ("shared reader yaml : : next", 1),
                // Test-only event telemetry around the same bounded reader.
                ("shared import crate : : yaml : : loaded", 1),
            ],
        ),
        (
            "crates/ekr-kernel/src/document.rs",
            &[
                ("serde_yaml_ng : : Deserializer : : from_str", 1),
                ("serde_yaml_ng : : Error", 1),
                // Test-only depth-cut event telemetry.
                ("shared import crate : : yaml : : loaded", 1),
            ],
        ),
        (
            "crates/ekr-kernel/src/document/shape.rs",
            &[(
                "serde_yaml_ng : : { observation : : { Document , Event , Tag } , Error , }",
                1,
            ),
                ("shared reader crate : : yaml : : load", 1),
                ("shared reader crate : : yaml : : next", 2),
            ],
        ),
        // Caller input: Ontology::from_yaml observes bytes, depth and expanded alias work first.
        (
            "crates/ekr-ontology/src/schema.rs",
            &[
                ("serde_yaml_ng : : from_str", 1),
                ("shared import ekr_core : : decode : : yaml : : { self , Past , Tally }", 1),
                ("shared reader yaml : : load", 1),
            ],
        ),
        (
            "crates/ekr-integrate/src/extraction.rs",
            &[
                ("serde_yaml_ng : : from_str", 1),
                ("shared reader ekr_core : : decode : : observe_yaml", 1),
            ],
        ),
        // SDK extraction observes caller input before typed decoding or unsupported_source's
        // diagnostic-only Value decode. Transaction limits only decode SDK-generated to_yaml.
        (
            "crates/ekr-sdk/src/document/extraction.rs",
            &[
                ("serde_yaml_ng : : from_str", 2),
                ("serde_yaml_ng : : Value", 1),
                ("serde_yaml_ng : : Value : : Tagged", 1),
                ("shared import ekr_core : : decode : : { observe_yaml , YamlRefusal }", 1),
                ("shared reader observe_yaml", 1),
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
            &[
                ("serde_yaml_ng : : Error", 1),
                // Writer reexport only; a module import is classified even when it only writes.
                ("shared import yaml : : to_yaml", 1),
            ],
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
            "crates/ekr/src/cli/view.rs",
            &[("serde_yaml_ng : : to_string", 1)],
        ),
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

#[test]
fn adversary_input08_the_new_shared_loader_is_an_ingress_too() {
    // This is an ordinary invocation of the public loader introduced by this unit, not a
    // disguised dependency. The caller supplies its own depth and receives the loaded tape.
    let input = format!("{}x{}", "[".repeat(65), "]".repeat(65));
    let mut loaded = ekr_core::decode::yaml::load(&input, usize::MAX).unwrap();
    let document = loaded.next_document().unwrap();
    document.check().unwrap();
    assert_eq!(document.event_count(), 131);

    for source in [
        "fn read(s: &str) { let mut d = ekr_core::decode::yaml::load(s, usize::MAX).unwrap(); d.next_document(); }",
        "use ekr_core::decode::yaml; fn read(s: &str) { yaml::load(s, usize::MAX).unwrap().next_document(); }",
    ] {
        let mut found = classified();
        references("crates/new-reader/src/lib.rs", source, &mut found);
        assert_ne!(found, classified(), "new unclassified shared-loader ingress escaped: {source}");
    }
}

#[test]
fn shared_yaml_calls_remain_counted_after_their_import_is_classified() {
    let mut missed = Vec::new();
    for (imports, call) in [
        ("", "ekr_core::decode::yaml::load(s, limit)"),
        ("", "crate::yaml::load(s, limit)"),
        ("", "crate::yaml::next(&mut documents)"),
        ("", "ekr_core::decode::observe_yaml(s, bytes, depth)"),
        ("use ekr_core::decode::yaml;", "yaml::load(s, limit)"),
        (
            "use ekr_core::decode::yaml as tape;",
            "tape::load(s, limit)",
        ),
        ("use ekr_core::decode::yaml::load;", "load(s, limit)"),
        (
            "use ekr_core::decode::yaml::load as read;",
            "read(s, limit)",
        ),
        (
            "use ekr_core::decode::yaml::load as _read;",
            "_read(s, limit)",
        ),
        (
            "use ekr_core::decode as bounded;",
            "bounded::yaml::load(s, limit)",
        ),
        (
            "use ekr_core as core;",
            "core::decode::yaml::load(s, limit)",
        ),
        (
            "use ekr_core::{decode::{yaml::{self as tape, load as read}}};",
            "read(s, limit)",
        ),
        (
            "use ekr_core::{decode::{yaml::{self as tape, load as read}}};",
            "tape::load(s, limit)",
        ),
        (
            "pub use ekr_core::decode::yaml::load as read;",
            "read(s, limit)",
        ),
        (
            "pub(crate) use ekr_core::decode::yaml::{self as tape, load as read};",
            "tape::load(s, limit)",
        ),
        ("use ekr_core::decode::yaml::*;", "load(s, limit)"),
        ("use ekr_core::decode::*;", "observe_yaml(s, bytes, depth)"),
        (
            "use ekr_core::decode::{observe_yaml as observe};",
            "observe(s, bytes, depth)",
        ),
        (
            "use ekr_core::decode::yaml as tape; use tape::load as read;",
            "read(s, limit)",
        ),
        (
            "use ekr_core::decode::yaml::load as read; mod other { use crate::unrelated as read; }",
            "read(s, limit)",
        ),
    ] {
        let mut admitted = Inventory::new();
        references("crates/new-reader/src/lib.rs", imports, &mut admitted);
        let mut with_call = Inventory::new();
        references(
            "crates/new-reader/src/lib.rs",
            &format!("{imports} fn read_input() {{ {call}; }}"),
            &mut with_call,
        );
        if admitted == with_call {
            missed.push(format!("{imports} {call}"));
        }
    }
    assert!(
        missed.is_empty(),
        "shared reader calls escaped:\n{}",
        missed.join("\n")
    );
}

#[test]
fn adversary_input08_pass2_another_call_counts_after_an_admitted_call() {
    let mut missed = Vec::new();
    for (imports, call) in [
        ("", "::ekr_core::decode::yaml::load(s, limit)"),
        (
            "use ekr_core::{decode::{yaml::{self as _tape, load as _read}}};",
            "_read(s, limit)",
        ),
        (
            "use ekr_core::{decode::{yaml::{self as _tape, load as _read}}};",
            "_tape::load(s, limit)",
        ),
        (
            "use ekr_core::decode as d; use d::yaml as y; use y::load as read;",
            "read(s, limit)",
        ),
        (
            "use y::load as read; use d::yaml as y; use ekr_core::decode as d;",
            "read(s, limit)",
        ),
        (
            "use ekr_core::*; use decode::*; use yaml::*;",
            "load(s, limit)",
        ),
        (
            "use yaml::*; use decode::*; use ekr_core::*;",
            "load(s, limit)",
        ),
        (
            "use ekr_core::decode as d; use d::{observe_yaml as _observe};",
            "_observe(s, bytes, depth)",
        ),
        (
            "use crate::yaml::{self as _tape, next as advance};",
            "advance(&mut documents)",
        ),
        (
            "use crate::yaml::{self as _tape, next as advance};",
            "_tape::next(&mut documents)",
        ),
    ] {
        let mut once = Inventory::new();
        references(
            "crates/new-reader/src/lib.rs",
            &format!("{imports} fn first() {{ {call}; }}"),
            &mut once,
        );
        let mut twice = Inventory::new();
        references(
            "crates/new-reader/src/lib.rs",
            &format!("{imports} fn first() {{ {call}; }} fn second() {{ {call}; }}"),
            &mut twice,
        );
        if twice.values().sum::<usize>() != once.values().sum::<usize>() + 1 {
            missed.push(format!("{imports} {call}"));
        }
    }
    assert!(
        missed.is_empty(),
        "extra calls escaped:\n{}",
        missed.join("\n")
    );
}

#[test]
fn adversary_input08_pass2_facade_prose_does_not_change_real_reader_counts() {
    let imports = "use ekr_core::decode::yaml::load as _read;";
    let live = format!("{imports} fn first() {{ _read(s, limit); }}");
    let mut once = Inventory::new();
    references("crates/new-reader/src/lib.rs", &live, &mut once);
    assert_eq!(
        once.get(&(
            "crates/new-reader/src/lib.rs".into(),
            "shared reader _read".into()
        )),
        Some(&1),
    );
    let prose = r####"
        // _read(s, limit); ekr_core::decode::yaml::load(s, limit);
        /* outer /* _read(s, limit); */ _read(s, limit); */
        const EXAMPLE: &str = r###"_read(s, limit); use ekr_core::decode::yaml;"###;
        const OTHER: &str = "_read(s, limit);";
    "####;
    let mut with_prose = Inventory::new();
    references(
        "crates/new-reader/src/lib.rs",
        &format!("{live}\n{prose}"),
        &mut with_prose,
    );
    assert_eq!(with_prose, once);
}
