//! Every explicitly exported function, constant and type must have a code use in the tests.
//!
//! This is a lexical policy tripwire, not behavioral coverage or Rust name resolution. Comments,
//! opaque literals, restricted visibility and partial identifiers cannot satisfy it. Functions
//! and constants still require path/method use; types additionally allow constructors, type
//! annotations, references, generic arguments, bounds and trait implementations. Imports alone
//! do not exercise a type. Macro-expanded declarations and aliases require their separate tests.

#[path = "support/rust_source.rs"]
mod rust_source;

use std::path::{Path, PathBuf};

/// The workspace root: this crate's manifest directory is `<root>/crates/ekr`.
fn workspace_root() -> PathBuf {
    std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .ancestors()
    .nth(2)
    .expect("crates/ekr sits two levels below the workspace root")
    .to_path_buf()
}

/// Every `.rs` file at or below a directory, read. Recurses, so a later module directory does not
/// become a hole in the scan.
fn rust_sources(directory: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    if !directory.is_dir() {
        return found;
    }
    let entries = std::fs::read_dir(directory)
        .unwrap_or_else(|e| panic!("reading {}: {e}", directory.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            found.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
            found.push((path, text));
        }
    }
    found
}

/// Comments are absent and literals remain opaque tokens.
fn without_comments(text: &str) -> String {
    rust_source::tokens(text).join(" ")
}

/// Functions/constants retain the original path-or-method criterion.
fn is_used(code: &str, name: &str) -> bool {
    function_used(&rust_source::tokens(code), name)
}

fn function_used(tokens: &[&str], name: &str) -> bool {
    tokens
        .windows(2)
        .any(|pair| matches!(pair[0], "." | "::") && pair[1] == name)
}

fn is_type_used(code: &str, name: &str) -> bool {
    type_used(&rust_source::tokens(code), name)
}

fn type_used(tokens: &[&str], name: &str) -> bool {
    let mut in_import = false;
    for (at, token) in tokens.iter().enumerate() {
        if *token == "use" {
            in_import = true;
        }
        if *token == ";" {
            in_import = false;
        }
        if in_import {
            continue;
        }
        if *token != name {
            continue;
        }
        let mut start = at;
        while start >= 2 && tokens[start - 1] == "::" {
            start -= 2;
        }
        let before = start.checked_sub(1).map(|index| tokens[index]);
        if matches!(
            before,
            Some("struct" | "enum" | "trait" | "type" | "fn" | "use")
        ) {
            continue;
        }
        let after = tokens.get(at + 1).copied();
        if matches!(after, Some("::" | "{" | "(" | "."))
            || matches!(
                before,
                Some("&" | ":" | "->" | "<" | "," | "+" | "impl" | "dyn" | "as" | "[" | "(")
            )
            || after_impl_generics(tokens, start)
        {
            return true;
        }
        // A reference may carry a lifetime and mutability before its type.
        let mut prefix = start;
        if prefix > 0 && tokens[prefix - 1] == "mut" {
            prefix -= 1;
        }
        if prefix >= 2 && tokens[prefix - 2] == "'" {
            prefix -= 2;
        }
        if prefix > 0 && tokens[prefix - 1] == "&" {
            return true;
        }
    }
    false
}

/// Recognize an impl's optional generic parameter list without treating every `>` as a type use.
fn after_impl_generics(tokens: &[&str], start: usize) -> bool {
    if start == 0 || tokens[start - 1] != ">" {
        return false;
    }
    let mut depth = 0usize;
    for at in (0..start).rev() {
        match tokens[at] {
            ">" => depth += 1,
            "<" => {
                depth -= 1;
                if depth == 0 {
                    return at > 0 && tokens[at - 1] == "impl";
                }
            }
            _ => {}
        }
    }
    false
}

/// Explicit, unrestricted pub declarations. The bool distinguishes types from functions/constants.
fn declared_public_items(source: &str) -> Vec<(&str, bool)> {
    let tokens = rust_source::tokens(source);
    let hidden_modules = hidden_inline_modules(&tokens);
    let mut found = Vec::new();
    for at in 0..tokens.len() {
        if tokens[at] != "pub" {
            continue;
        }
        let mut kind = at + 1;
        while matches!(tokens.get(kind), Some(&"async" | &"unsafe")) {
            kind += 1;
        }
        if tokens.get(kind) == Some(&"const") && tokens.get(kind + 1) == Some(&"fn") {
            kind += 1;
        }
        let is_type = matches!(
            tokens.get(kind),
            Some(&"struct" | &"enum" | &"trait" | &"type")
        );
        if is_type
            && hidden_modules
                .iter()
                .any(|(start, end)| *start < at && at < *end)
        {
            continue;
        }
        if is_type || matches!(tokens.get(kind), Some(&"fn" | &"const")) {
            if let Some(name) = tokens.get(kind + 1).filter(|name| {
                name.chars()
                    .next()
                    .is_some_and(|character| character.is_alphabetic() || character == '_')
            }) {
                found.push((*name, is_type));
            }
        }
    }
    found
}

/// A pub item in a restricted inline module does not become an exported type.
/// External module files are conservatively scanned: re-exports can expose their declarations.
fn hidden_inline_modules(tokens: &[&str]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    for at in 0..tokens.len().saturating_sub(2) {
        if tokens[at] != "mod" || tokens[at + 2] != "{" || (at > 0 && tokens[at - 1] == "pub") {
            continue;
        }
        let start = at + 2;
        let mut depth = 1usize;
        for (end, token) in tokens.iter().enumerate().skip(start + 1) {
            match *token {
                "{" => depth += 1,
                "}" => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                ranges.push((start, end));
                break;
            }
        }
    }
    ranges
}

fn declared_public_name(source: &str) -> Option<&str> {
    declared_public_items(source).first().map(|(name, _)| *name)
}

/// Every crate directory under `crates/`, sorted, so the report names them in a stable order.
fn crate_directories() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(workspace_root().join("crates"))
        .expect("reading crates/")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.join("Cargo.toml").is_file())
        .collect();
    found.sort();
    found
}

#[test]
fn no_public_item_in_any_crate_is_untested() {
    let crates = crate_directories();
    assert!(
        crates.len() >= 6,
        "the crate scan is broken, not the workspace: {crates:?}"
    );

    // The suite is every crate's tests pooled: `ekr` depends on all five, so a case anywhere may
    // legitimately exercise an item declared anywhere.
    let read: Vec<(PathBuf, String)> = crates
        .iter()
        .flat_map(|directory| rust_sources(&directory.join("tests")))
        .collect();
    let raw: String = read.iter().map(|(_, text)| text.as_str()).collect();
    let suite: String = read
        .iter()
        .map(|(_, text)| without_comments(text))
        .collect();
    let suite_tokens = rust_source::tokens(&suite);

    // A check that silently stops stripping is a check that is back to counting prose, so the
    // stripper is measured rather than trusted. The marker is assembled from two pieces here and
    // written whole in the comment two lines below, so this line is not itself a match.
    let marker = concat!("workspace-stripper", "-sentinel");
    // workspace-stripper-sentinel — the one occurrence, and it is inside a comment.
    assert!(raw.contains(marker), "this file is not in the scan");
    assert!(
        !suite.contains(marker),
        "the comment stripper did nothing: a marker written only in a comment survived it"
    );

    let mut untested: Vec<String> = Vec::new();
    let mut declared_total = 0usize;
    for directory in &crates {
        let name = directory
            .file_name()
            .expect("a crate directory has a name")
            .to_string_lossy()
            .into_owned();
        let mut declared: Vec<(String, bool)> = rust_sources(&directory.join("src"))
            .iter()
            .flat_map(|(_, text)| declared_public_items(text))
            .map(|(name, is_type)| (name.to_owned(), is_type))
            .collect();
        declared.sort();
        declared.dedup();
        declared_total += declared.len();
        untested.extend(
            declared
                .into_iter()
                .filter(|(item, is_type)| {
                    if *is_type {
                        !type_used(&suite_tokens, item)
                    } else {
                        !function_used(&suite_tokens, item)
                    }
                })
                .map(|(item, _)| format!("{name}::{item}")),
        );
    }

    assert!(
        declared_total > 20,
        "the declaration scan is broken, not the suite: {declared_total} items across {} crates",
        crates.len()
    );
    assert!(
        untested.is_empty(),
        "public items no case uses outside a comment: {untested:?}"
    );
}

#[test]
fn exported_type_declarations_are_not_invisible() {
    for declaration in [
        "pub struct UnusedType;",
        "pub enum UnusedType { First }",
        "pub trait UnusedType {}",
        "pub type UnusedType = ();",
    ] {
        assert_eq!(declared_public_name(declaration), Some("UnusedType"));
    }
    for declaration in [
        "pub(crate) struct UnusedType;",
        "pub(super) enum UnusedType { First }",
        "pub(in crate::inner) trait UnusedType {}",
    ] {
        assert_eq!(declared_public_name(declaration), None);
    }
}

#[test]
fn strings_and_comments_do_not_exercise_public_functions() {
    for source in [
        "// Thing::unexercised()",
        "/* outer /* Thing::unexercised() */ still comment */",
        r#"let text = "Thing::unexercised()";"#,
        r##"let text = r#"Thing::unexercised()"#;"##,
        r#"let bytes = b"Thing::unexercised()";"#,
        r##"let bytes = br#"Thing::unexercised()"#;"##,
    ] {
        assert!(
            !is_used(&without_comments(source), "unexercised"),
            "{source}"
        );
    }
}

#[test]
fn actual_type_positions_count_and_partial_or_declaration_names_do_not() {
    for source in [
        "Widget::new()",
        "fn f(value: &Widget) {}",
        "let value: Widget = make();",
        "Widget { value: 1 }",
        "fn f() -> Widget {}",
        "Some(Widget(1))",
        "Vec<Widget>",
        "impl Widget for Adapter {}",
        "fn f<'a>(v: &'a mut Widget) {}",
        "let v: namespace::Widget = make();",
        "fn f<T: Widget>() {}",
        "fn f<T: Other + Widget>() {}",
        "Widget.method()",
    ] {
        assert!(is_type_used(source, "Widget"), "{source}");
    }
    for source in [
        "LongerWidget::new()",
        "WidgetSuffix::new()",
        "Widgeté::new()",
        "use namespace::Widget;",
        "pub struct Widget {}",
        "enum Widget { One }",
        "use namespace::{Other, Widget};",
        "use Widget::{First, Second};",
        "/* Widget::new() */",
        r#"let text = "Widget::new()";"#,
        "fn Widget() {}",
        "let widget = 1;",
    ] {
        assert!(!is_type_used(source, "Widget"), "{source}");
    }
    assert!(!is_used("Object::call_suffix()", "call"));
    assert!(!is_used("Object::callé()", "call"));
    assert!(is_used("Object :: call()", "call"));
    assert!(is_used("object . call()", "call"));
}

#[test]
fn declarations_ignore_comments_literals_and_support_multiline_visibility() {
    assert_eq!(
        declared_public_items("// pub struct Ghost;\n/* pub trait Ghost {} */"),
        []
    );
    assert_eq!(
        declared_public_items(r#"let text = "pub enum Ghost {}";"#),
        []
    );
    assert_eq!(
        declared_public_items("pub\nstruct\nVisible;"),
        vec![("Visible", true)]
    );
    assert_eq!(
        declared_public_items("pub const VALUE: u32 = 1; pub const fn value() {}"),
        vec![("VALUE", false), ("value", false)]
    );
    assert_eq!(
        declared_public_items(
            "pub(crate) mod hidden { pub trait Seal {} pub mod nested { pub struct Hidden; } }"
        ),
        []
    );
    assert_eq!(
        declared_public_items("pub mod visible { pub struct Visible; }"),
        vec![("Visible", true)]
    );
}

#[test]
fn adversary_generic_trait_implementation_is_an_actual_type_use() {
    assert!(is_type_used("impl<T> Widget for Adapter<T> {}", "Widget"));
    assert!(is_type_used(
        "impl<T> api::Widget for Adapter<T> {}",
        "Widget"
    ));
}

#[test]
fn adversary_tuple_and_array_elements_are_actual_type_uses() {
    for code in [
        "fn consume(_: [Widget; 2]) {}",
        "fn consume(_: (Widget, u8)) {}",
        "fn consume(_: &[Widget]) {}",
    ] {
        assert!(is_type_used(code, "Widget"), "{code}");
    }
}

#[test]
fn adversary_opaque_literals_keep_comment_and_identifier_boundaries() {
    for source in [
        r####"const TEXT: &str = r###"/* Widget::new() */"###;"####,
        r#"let bytes = b"\"Widget::new()\"";"#,
        r#"let text = c"/* Widget::new() */";"#,
        r#"let text = "\"Widget::new()\""; /* outside */"#,
        "/* before /* Widget::new() */ after */",
        "pub struct WidgetSuffix; WidgetSuffix::new();",
        "use api::{Widget, other::Thing};",
    ] {
        assert!(!is_type_used(source, "Widget"), "{source}");
        assert!(
            !is_used(source, "new") || source.contains("WidgetSuffix"),
            "{source}"
        );
    }
    assert!(is_type_used("let value: api::r#Widget = make();", "Widget"));
    assert!(is_used("value /* separator */ . r#call()", "call"));
}

#[test]
fn rereview_generic_impl_recognition_balances_nested_types_without_comparison_mentions() {
    for code in [
        "impl<T: Fn() -> Vec<(u8, u8)>> api::Widget for Adapter<T> {}",
        "impl<'a, T: Iterator<Item = &'a str>> api::Widget for Adapter<T> {}",
        "impl<const N: usize> Widget for [u8; N] {}",
        "fn consume(_: &[api::Widget; 2]) {}",
        "fn consume(_: (api::Widget, u8)) {}",
    ] {
        assert!(is_type_used(code, "Widget"), "{code}");
    }
    for code in [
        "let compared = left > Widget;",
        "let compared = Vec::<u8>::new().len() > Widget;",
        "use api::{outer::Widget};",
        "impl<T: Fn() -> Vec<T>> api::WidgetSuffix for Adapter<T> {}",
        "/* impl<T> Widget for Adapter<T> {} */",
        r#"let code = "impl<T> Widget for Adapter<T> {}";"#,
    ] {
        assert!(!is_type_used(code, "Widget"), "{code}");
    }
}
