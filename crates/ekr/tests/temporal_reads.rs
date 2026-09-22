//! Workspace policy tripwire against rebuilding the deleted open-ended valid-time query.
//!
//! Recursively reads product src/ only. Negative test witnesses remain legal, as do transaction
//! time reads. This does not prove that every semantically equivalent expression is impossible.

#[path = "support/rust_source.rs"]
mod rust_source;

#[path = "support/workspace_manifest.rs"]
mod workspace_manifest;

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .parent()
    .and_then(Path::parent)
    .expect("crates/ekr has a workspace root")
    .to_owned()
}

fn collect(directory: &Path, into: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("source directory is readable") {
        let path = entry.expect("a source entry").path();
        if path.is_dir() {
            collect(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

fn source_paths(root: &Path) -> Vec<PathBuf> {
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("workspace manifest");
    let members = workspace_manifest::members(&manifest)
        .expect("workspace member grammar must be supported before scanning sources");
    let mut paths = Vec::new();
    for member in members
        .iter()
        .filter(|member| member.starts_with("crates/"))
    {
        collect(&root.join(member).join("src"), &mut paths);
    }
    paths.sort();
    assert!(
        !paths.is_empty(),
        "the product source scan must be nonempty"
    );
    paths
}

fn forbidden_reads(source: &str) -> bool {
    let tokens = rust_source::tokens(source);
    tokens
        .windows(3)
        .any(|part| part == ["valid_time", ".", "is_open"])
        || tokens
            .windows(5)
            .any(|part| part == ["valid_time", ".", "to", ".", "is_none"])
}

fn violations(root: &Path) -> Vec<PathBuf> {
    source_paths(root)
        .into_iter()
        .filter(|path| {
            forbidden_reads(&std::fs::read_to_string(path).expect("product source is readable"))
        })
        .collect()
}

fn compile_time_manifest_path(source: &str) -> bool {
    rust_source::tokens(source).windows(4).any(|tokens| {
        tokens[0] == "env"
            && tokens[1] == "!"
            && matches!(tokens[2], "(" | "{" | "[")
            && manifest_argument_or_unsupported(tokens[3])
    })
}

fn manifest_argument_or_unsupported(literal: &str) -> bool {
    if let Some(raw) = literal.strip_prefix('r') {
        if let Some(opening) = raw.find('"') {
            if raw[..opening].chars().all(|ch| ch == '#') {
                let content = &raw[opening + 1..raw.len() - opening - 1];
                return content == "CARGO_MANIFEST_DIR";
            }
        }
    }
    // JSON decodes the ordinary string subset without an added dependency. Rust-only escapes
    // or a nonliteral argument are unsupported, so fail closed rather than lose the guard.
    serde_json::from_str::<String>(literal).map_or(true, |name| name == "CARGO_MANIFEST_DIR")
}

#[test]
fn macro_argument_spellings_cannot_hide_compile_time_manifest_authority() {
    for source in [
        r##"std::env! { r#"CARGO_MANIFEST_DIR"# }"##,
        r#"env!["CARGO_\u{004d}ANIFEST_DIR"]"#,
        r#"env!("CARGO_\x4dANIFEST_DIR")"#,
        r#"env /* comment */ ! { "CARGO_MANIFEST_DIR" }"#,
    ] {
        assert!(compile_time_manifest_path(source), "{source}");
    }
    for source in [
        r#"env!("CARGO_PKG_NAME")"#,
        r##"env! { r#"CARGO_PKG_NAME"# }"##,
        r#"// env!{"CARGO_MANIFEST_DIR"}"#,
        r##"let text = r#"env!{"CARGO_MANIFEST_DIR"}"#;"##,
    ] {
        assert!(!compile_time_manifest_path(source), "{source}");
    }
}

#[test]
fn no_product_source_rebuilds_the_open_ended_valid_time_filter() {
    let root = workspace_root();
    assert!(
        violations(&root).is_empty(),
        "forbidden valid-time reads: {:?}",
        violations(&root)
    );
}

#[test]
fn both_forbidden_spellings_are_code_not_text() {
    for source in [
        "record.valid_time.is_open()",
        "record . valid_time . to . is_none ()",
        "record.valid_time /* a comment */ . to\n.is_none()",
    ] {
        assert!(forbidden_reads(source), "{source}");
    }
    for source in [
        "record.transaction_time.is_open()",
        "record.valid_time.contains(at)",
        "// record.valid_time.is_open()",
        r#"let text = "record.valid_time.to.is_none()";"#,
        r##"let text = br#"record.valid_time.is_open()"#;"##,
        "record.other_valid_time.is_open()",
    ] {
        assert!(!forbidden_reads(source), "{source}");
    }
}

#[test]
fn the_same_policy_reaches_graph_kernel_store_and_nested_modules() {
    let directory = tempfile::TempDir::new().expect("isolated policy fixture");
    std::fs::write(directory.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/ekr-graph\", \"crates/ekr-kernel\", \"crates/ekr-store\"]\n")
        .expect("fixture manifest");
    let cases = [
        "crates/ekr-graph/src/lib.rs",
        "crates/ekr-kernel/src/lib.rs",
        "crates/ekr-store/src/lib.rs",
        "crates/ekr-store/src/nested/reader.rs",
    ];
    for path in cases {
        let path = directory.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "// a clean product module\n").unwrap();
    }
    for path in cases {
        let full = directory.path().join(path);
        std::fs::write(&full, "fn query() { record.valid_time.to.is_none(); }\n").unwrap();
        assert_eq!(
            violations(directory.path()),
            std::slice::from_ref(&full),
            "{path}"
        );
        std::fs::write(full, "// restored\n").unwrap();
    }
    assert!(violations(directory.path()).is_empty());
}

#[test]
fn executable_source_location_macros_cannot_return() {
    let root = workspace_root();
    let mut sources = source_paths(&root);
    for crate_path in std::fs::read_dir(root.join("crates")).unwrap() {
        let tests = crate_path.unwrap().path().join("tests");
        if tests.is_dir() {
            collect(&tests, &mut sources);
        }
    }
    collect(&root.join("xtask/src"), &mut sources);
    let mut offenders = Vec::new();
    for path in sources {
        let source = std::fs::read_to_string(&path).unwrap();
        if compile_time_manifest_path(&source) {
            offenders.push(path);
        }
    }
    assert!(
        offenders.is_empty(),
        "compile-time checkout readers: {offenders:?}"
    );
}

#[test]
fn adversary_cargo_member_comment_cannot_hide_a_product_source() {
    let directory = tempfile::TempDir::new().expect("isolated workspace fixture");
    std::fs::write(
        directory.path().join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\n\
         \"crates/ekr-graph\", # graph comes before kernel\n\
         \"crates/ekr-kernel\",\n]\n",
    )
    .unwrap();
    for member in ["ekr-graph", "ekr-kernel"] {
        let crate_path = directory.path().join("crates").join(member);
        std::fs::create_dir_all(crate_path.join("src")).unwrap();
        std::fs::write(
            crate_path.join("Cargo.toml"),
            format!("[package]\nname = \"{member}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"),
        )
        .unwrap();
        std::fs::write(crate_path.join("src/lib.rs"), "// clean source\n").unwrap();
    }
    let forbidden = directory.path().join("crates/ekr-kernel/src/lib.rs");
    std::fs::write(
        &forbidden,
        "pub struct Time { pub to: Option<u8> }\n\
         pub fn query(valid_time: &Time) -> bool { valid_time.to.is_none() }\n",
    )
    .unwrap();
    let cargo = std::process::Command::new("cargo")
        .args([
            "metadata",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ])
        .current_dir(directory.path())
        .output()
        .expect("Cargo metadata validates the synthetic workspace");
    assert!(
        cargo.status.success(),
        "{}",
        String::from_utf8_lossy(&cargo.stderr)
    );
    let metadata = String::from_utf8(cargo.stdout).unwrap();
    assert!(metadata.contains("\"name\":\"ekr-graph\""));
    assert!(metadata.contains("\"name\":\"ekr-kernel\""));
    assert_eq!(violations(directory.path()), vec![forbidden]);
}

#[test]
fn adversary_manifest_macro_guard_rejects_all_rust_delimiters() {
    let directory = tempfile::TempDir::new().expect("isolated macro fixture");
    std::fs::write(
        directory.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/ekr\"]\n",
    )
    .unwrap();
    let crate_path = directory.path().join("crates/ekr");
    std::fs::create_dir_all(crate_path.join("src")).unwrap();
    std::fs::create_dir_all(crate_path.join("tests")).unwrap();
    std::fs::create_dir_all(directory.path().join("xtask/src")).unwrap();
    std::fs::write(crate_path.join("src/main.rs"), "fn main() {}\n").unwrap();
    let probe = crate_path.join("tests/reader.rs");
    for invocation in [
        "env!(\"CARGO_MANIFEST_DIR\")",
        "env!{\"CARGO_MANIFEST_DIR\"}",
        "env![\"CARGO_MANIFEST_DIR\"]",
    ] {
        std::fs::write(&probe, format!("const ROOT: &str = {invocation};\n")).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "executable_source_location_macros_cannot_return",
                "--nocapture",
            ])
            .env("CARGO_MANIFEST_DIR", &crate_path)
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "the actual guard accepted {invocation}:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("1 failed"),
            "refusal must execute the named guard: {}",
            String::from_utf8_lossy(&output.stdout),
        );
    }
}

#[test]
fn rereview_member_inventory_reads_literal_hash_paths_and_refuses_partial_globs() {
    let directory = tempfile::TempDir::new().unwrap();
    let manifest = "[workspace] # root\nresolver = '2'\nmembers = [\n\
        'crates/with#hash', # not part of the quoted name\n\
        \"crates/with space\" # no trailing comma\n]\n\
        [workspace.package]\nversion = '0.0.0'\n";
    std::fs::write(directory.path().join("Cargo.toml"), manifest).unwrap();
    for (path, package) in [
        ("crates/with#hash", "hash_fixture"),
        ("crates/with space", "space_fixture"),
    ] {
        let root = directory.path().join(path);
        std::fs::create_dir_all(root.join("src/nested")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            format!("[package]\nname = '{package}'\nversion = '0.0.0'\nedition = '2021'\n"),
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub mod nested;\n").unwrap();
        std::fs::write(root.join("src/nested/mod.rs"), "// clean\n").unwrap();
    }
    let forbidden = directory.path().join("crates/with#hash/src/nested/mod.rs");
    std::fs::write(
        &forbidden,
        "pub struct Time { pub to: Option<u8> }\n\
         pub fn query(valid_time: &Time) -> bool { valid_time.to.is_none() }\n",
    )
    .unwrap();
    let metadata = std::process::Command::new("cargo")
        .args([
            "metadata",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(
        metadata.status.success(),
        "{}",
        String::from_utf8_lossy(&metadata.stderr)
    );
    assert_eq!(violations(directory.path()), vec![forbidden]);
    for unsupported in [
        "[workspace]\nmembers=['crates/with#hash', 'crates/*']\n",
        "[workspace]\nmembers=['crates/with#hash', '../outside']\n",
        "[workspace]\nmembers=['crates/with#hash']\nmembers=['crates/with space']\n",
    ] {
        std::fs::write(directory.path().join("Cargo.toml"), unsupported).unwrap();
        assert!(
            std::panic::catch_unwind(|| source_paths(directory.path())).is_err(),
            "unsupported inventory must refuse, not report a partial scan: {unsupported}"
        );
    }
}
