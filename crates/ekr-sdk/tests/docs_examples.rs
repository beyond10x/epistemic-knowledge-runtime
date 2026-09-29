//! `task:sdk-docs-cover-the-document-builders`: the "Documents" section of `docs/sdk.md` shows
//! code that compiles and runs, and names only what the public API has.
//!
//! Each Rust block of that section is, byte for byte, one of the regions below marked
//! `// docs/sdk.md, Documents: example <n>` … `// end of example`, after the region's indentation
//! is removed. The compiler builds those regions as part of this file and the tests run them: the
//! first three in order against a real `ekr session` on a new file store, built from this checkout
//! the way `session.rs` builds it, the fourth without a process.
//! [`the_section_s_rust_blocks_are_the_regions_this_file_compiles_and_runs`] holds the page to
//! the regions, so an example edited on the page and not here, or here and not on the page, fails.
//!
//! [`every_api_name_the_section_uses_is_compiled_here`] holds the section's prose the same way:
//! every name in backticks that has the shape of a Rust path (`TransactionBuilder`,
//! `Ontology::ensure`, `ekr_sdk::document`) or of a call (`payload_hash()`) must resolve in the
//! SDK's public API: it must occur in the example regions or in [`names_the_prose_uses`], which
//! the compiler builds, as a path whose head this file imports from `ekr_sdk` ([`unresolved`]).
//! A name only this file defines, such as a test helper, does not resolve
//! ([`the_name_check_refuses_a_name_only_this_file_defines`]). The prose names that no example
//! uses are spelled out in [`names_the_prose_uses`], which the same case also runs.
//!
//! No name an example region uses is imported at module level, so each region compiles on its own
//! `use` lines and those of the regions before it in the same function.

use std::collections::BTreeSet;
use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// No name an example region uses is imported here, so each region compiles on its own `use`
// lines and those of the regions before it.
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{AgentId, OntologyError, TRANSACTION_LIMITS};
use ekr_sdk::reply::{Answer, Outcome};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use serde_json::Value as Json;

// ----- the page and this file -----

/// The workspace root, found at run time from the directory holding `Cargo.lock`: never
/// `env!("CARGO_MANIFEST_DIR")`, which a binary built in another checkout would carry
/// (`AGENTS.md` § The gate).
fn workspace_root() -> PathBuf {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    manifest
        .ancestors()
        .find(|directory| directory.join("Cargo.lock").is_file())
        .expect("a workspace root above the crate")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The "Documents" section of `docs/sdk.md`: from its heading to the next `## ` heading.
fn section() -> String {
    let page = read("docs/sdk.md");
    let mut lines = page.lines().skip_while(|line| *line != "## Documents");
    let heading = lines
        .next()
        .expect("docs/sdk.md has a `## Documents` section");
    std::iter::once(heading)
        .chain(lines.take_while(|line| !line.starts_with("## ")))
        .map(|line| format!("{line}\n"))
        .collect()
}

/// The contents of every fenced block of `section`, with its info string, in order.
fn fenced(section: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut open: Option<(String, String)> = None;
    for line in section.lines() {
        match (&mut open, line.strip_prefix("```")) {
            (None, Some(info)) => open = Some((info.trim().to_owned(), String::new())),
            (Some(_), Some("")) => blocks.push(open.take().expect("open")),
            (Some((_, text)), _) => {
                text.push_str(line);
                text.push('\n');
            }
            (None, None) => {}
        }
    }
    assert!(open.is_none(), "an unclosed fence in the section");
    blocks
}

/// The section's text outside fenced blocks.
fn prose(section: &str) -> String {
    let mut inside = false;
    let mut text = String::new();
    for line in section.lines() {
        if line.starts_with("```") {
            inside = !inside;
        } else if !inside {
            text.push_str(line);
            text.push('\n');
        }
    }
    text
}

/// This file.
fn this_file() -> String {
    read("crates/ekr-sdk/tests/docs_examples.rs")
}

/// Every marked example region of this file, in order, its common indentation removed.
fn regions(source: &str) -> Vec<String> {
    let mut regions = Vec::new();
    let mut open: Option<Vec<&str>> = None;
    for line in source.lines() {
        let marker = line.trim();
        if let Some(number) = marker.strip_prefix("// docs/sdk.md, Documents: example ") {
            assert!(open.is_none(), "example {number} opens inside another");
            assert_eq!(
                number,
                (regions.len() + 1).to_string(),
                "examples are numbered in order"
            );
            open = Some(Vec::new());
        } else if marker == "// end of example" {
            let lines = open.take().expect("an end marker closes an open example");
            let indent = lines
                .iter()
                .filter(|line| !line.trim().is_empty())
                .map(|line| line.len() - line.trim_start().len())
                .min()
                .unwrap_or(0);
            regions.push(
                lines
                    .iter()
                    .map(|line| format!("{}\n", line.get(indent..).unwrap_or("")))
                    .collect(),
            );
        } else if let Some(lines) = &mut open {
            lines.push(line);
        }
    }
    assert!(open.is_none(), "an example region is not closed");
    regions
}

/// `source` with every line comment, string literal and character literal emptied: the text the
/// compiler reads as code. A `'` that starts no character literal is a lifetime's, and is kept.
fn code(source: &str) -> String {
    let mut code = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                let mut ahead = chars.clone();
                let literal = match ahead.next() {
                    Some('\\') => ahead.next().is_some() && ahead.next() == Some('\''),
                    Some(_) => ahead.next() == Some('\''),
                    None => false,
                };
                if literal {
                    chars = ahead;
                    code.push_str("' '");
                } else {
                    code.push('\'');
                }
            }
            '"' => {
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => {
                            chars.next();
                        }
                        '"' => break,
                        _ => {}
                    }
                }
                code.push_str("\"\"");
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        code.push('\n');
                        break;
                    }
                }
            }
            c => code.push(c),
        }
    }
    code
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Whether `name` occurs in `text` as a whole path: nothing that continues an identifier before
/// or after it. A `::` before it is allowed, so `Limit` is named by `DocumentError::Limit`.
fn occurs(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + name.len()..].chars().next();
        !before.is_some_and(is_ident) && (name.ends_with('(') || !after.is_some_and(is_ident))
    })
}

/// Every name in backticks in `prose` that has the shape of a Rust path or of a call, as it must
/// occur in code: `Ontology::ensure`, `DocumentError`, `ekr_sdk::document`, or `to_yaml(` for
/// `to_yaml()`. A span holding several (`SchemaFixed { missing }`, `Vec<Operation>`) gives each
/// capitalised one. Anything else in backticks (a verb, a limit's name, a format, a tag) is not
/// a Rust name and is skipped.
fn api_names(prose: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for span in prose.split('`').skip(1).step_by(2) {
        let first = span.chars().next().unwrap_or(' ');
        let path = |text: &str| text.chars().all(|c| is_ident(c) || c == ':');
        if first.is_ascii_uppercase() {
            names.extend(
                span.split(|c: char| !(is_ident(c) || c == ':'))
                    .filter(|piece| piece.starts_with(|c: char| c.is_ascii_uppercase()))
                    .map(str::to_owned),
            );
        } else if first.is_ascii_lowercase() {
            let (head, call) = match span.split_once('(') {
                Some((head, _)) => (head, true),
                None => (span, false),
            };
            if path(head) && head.contains("::") {
                names.insert(head.to_owned());
            } else if path(head) && call {
                names.insert(format!("{head}("));
            }
        }
    }
    names
}

#[test]
fn the_section_s_rust_blocks_are_the_regions_this_file_compiles_and_runs() {
    let section = section();
    let blocks = fenced(&section);
    let rust: Vec<&String> = blocks
        .iter()
        .filter(|(info, _)| info == "rust")
        .map(|(_, text)| text)
        .collect();
    let other: Vec<&String> = blocks
        .iter()
        .filter(|(info, _)| info != "rust")
        .map(|(info, _)| info)
        .collect();
    assert!(
        other.is_empty(),
        "every block of the section is Rust: {other:?}"
    );
    let regions = regions(&this_file());
    assert_eq!(regions.len(), 4, "this file marks four examples");
    assert_eq!(
        rust.len(),
        regions.len(),
        "the section has one Rust block per example region of this file"
    );
    for (number, (block, region)) in rust.iter().zip(&regions).enumerate() {
        assert_eq!(
            *block,
            region,
            "docs/sdk.md, Documents, Rust block {} differs from example {}",
            number + 1,
            number + 1
        );
    }
}

#[test]
fn the_name_check_refuses_a_name_only_this_file_defines() {
    let prose = format!(
        "{}\n`World::new`, `World`, `ekr_path()` and `names_the_prose_uses()`.\n",
        prose(&section())
    );
    let refused = unresolved(&api_names(&prose), &this_file());
    assert_eq!(
        refused,
        ["World", "World::new", "ekr_path(", "names_the_prose_uses("],
        "a name this test file defines is not the public API"
    );
}

/// Every identifier a `use ekr_sdk::…;` statement of `code` holds: the SDK's modules and the
/// public items this file imports from them. A name defined in this file is never among them.
fn sdk_imports(code: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for (at, _) in code.match_indices("use ekr_sdk::") {
        if code[..at].chars().next_back().is_some_and(is_ident) {
            continue;
        }
        let statement = &code[at..];
        let end = statement.find(';').expect("a use statement ends with `;`");
        names.extend(
            statement[..end]
                .split(|c: char| !is_ident(c))
                .filter(|word| !word.is_empty() && *word != "use")
                .map(str::to_owned),
        );
    }
    names
}

/// Every path of `code` from its head: `a::b::c` where nothing that continues a path or a method
/// call comes before `a`. A path stops at `::{` or `::<`.
fn paths(code: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut at = 0;
    while at < code.len() {
        let c = code[at..].chars().next().expect("in bounds");
        let before = code[..at].chars().next_back();
        if !(c.is_ascii_alphabetic() || c == '_') || before.is_some_and(is_ident) {
            at += c.len_utf8();
            continue;
        }
        let mut end = at;
        loop {
            end = code[end..]
                .find(|c: char| !is_ident(c))
                .map_or(code.len(), |n| end + n);
            let next = code[end..].strip_prefix("::");
            match next.and_then(|rest| rest.chars().next()) {
                Some(c) if c.is_ascii_alphabetic() || c == '_' => end += 2,
                _ => break,
            }
        }
        let head = code[..at].trim_end();
        if !head.ends_with("::") && !head.ends_with('.') {
            found.insert(code[at..end].to_owned());
        }
        at = end;
    }
    found
}

/// The code the section's names resolve against: the example regions and
/// [`names_the_prose_uses`], its signature included. Neither defines anything; each only uses
/// what the SDK exports.
fn resolving_code(source: &str) -> String {
    let witness: String = source
        .lines()
        .skip_while(|line| !line.starts_with("fn names_the_prose_uses("))
        .take_while(|line| *line != "}")
        .map(|line| format!("{line}\n"))
        .collect();
    assert!(!witness.is_empty(), "this file has `names_the_prose_uses`");
    code(&(regions(source).concat() + &witness))
}

/// The names of `names` that do not resolve in the SDK's public API. A path resolves when a path
/// of [`resolving_code`] whose head is a name [`sdk_imports`] finds is that path, extends it, or,
/// for one capitalised word, holds it as a segment (`Limit` in `DocumentError::Limit`). A call
/// `f(` resolves as an SDK function called there, or a method called there on an SDK value. The
/// prelude's `None`, `Some`, `Ok` and `Err` resolve.
fn unresolved(names: &BTreeSet<String>, source: &str) -> Vec<String> {
    let sdk = sdk_imports(&code(source));
    let scope = resolving_code(source);
    let rooted: Vec<String> = paths(&scope)
        .into_iter()
        .filter(|path| sdk.contains(path.split("::").next().unwrap_or_default()))
        .collect();
    let resolves = |name: &String| {
        if ["None", "Some", "Ok", "Err"].contains(&name.as_str()) {
            return true;
        }
        if let Some(function) = name.strip_suffix('(') {
            return (sdk.contains(function) && occurs(&scope, name))
                || scope.contains(&format!(".{name}"));
        }
        rooted.iter().any(|path| {
            path == name
                || path.starts_with(&format!("{name}::"))
                || (!name.contains("::") && path.split("::").any(|segment| segment == name))
        })
    };
    names
        .iter()
        .filter(|name| !resolves(name))
        .cloned()
        .collect()
}

#[test]
fn every_api_name_the_section_uses_is_compiled_here() -> Result<(), Box<dyn Error>> {
    use ekr_sdk::document::{DocumentError, Ontology, OntologySpec, ValidationProfile};

    let names = api_names(&prose(&section()));
    for required in [
        "TransactionBuilder",
        "OntologySpec",
        "Ontology::ensure",
        "DocumentError::Limit",
    ] {
        assert!(
            names.contains(required),
            "the section names `{required}`: {names:?}"
        );
    }
    let missing = unresolved(&names, &this_file());
    assert!(
        missing.is_empty(),
        "the section names what no code here compiles: {missing:?}"
    );

    let operator = AgentId::mint();
    let spec = OntologySpec::new();
    let change = Ontology::default().ensure(&spec, ValidationProfile::V1)?;
    names_the_prose_uses(
        &spec,
        &change,
        &DocumentError::EmptyTransaction,
        &OntologyError::SchemaFixed { missing: 1 },
        &Outcome::Stale(Json::Null),
        operator,
    )
}

/// Every public name the section's prose uses and no example does, in code the compiler holds
/// to the public API. It defines nothing and uses only what the SDK exports, which
/// [`unresolved`] relies on.
fn names_the_prose_uses(
    spec: &ekr_sdk::document::OntologySpec,
    change: &ekr_sdk::document::SchemaChange,
    error: &ekr_sdk::document::DocumentError,
    refusal: &OntologyError,
    outcome: &Outcome,
    operator: AgentId,
) -> Result<(), Box<dyn Error>> {
    use ekr_sdk::document::{
        Assertion, Confidence, DocumentError, DocumentLimit, EdgeDraft, EdgeId, Evidence,
        EvidenceAddition, EvidenceSource, NodeDraft, NodeId, Object, Ontology, OntologySpec,
        Operation, Predicate, PropertyId, SchemaChange, SchemaVersionId, SeedBuilder, SeedDocument,
        Subject, Timestamp, TransactionBuilder, TransactionDocument, TransactionId, TypeId,
        TypedReference, ValidationProfile,
    };
    use ekr_sdk::session::ProcessSession;

    let _ = std::any::type_name::<ProcessSession>();
    let (seed, held): (SeedBuilder, Ontology) = OntologySpec::seed(spec, Timestamp::EPOCH)?;
    let root = SeedBuilder::root_id(&seed);
    let node = NodeDraft::with_id(NodeDraft::new(root, TypeId::mint(), "n"), NodeId::mint());
    let edge = EdgeDraft::new(root, TypeId::mint(), node.id, node.id);
    let source = EvidenceSource::human("h");
    let entry = Evidence::for_payload(
        b"",
        source.clone(),
        operator,
        Timestamp::EPOCH,
        Confidence::CERTAIN,
    );
    let evidence = EvidenceAddition::new(
        source,
        operator,
        Timestamp::EPOCH,
        Confidence::CERTAIN,
        vec![],
    );
    let claim = Assertion::new(
        root,
        Subject::Node(node.id),
        Predicate::Property(PropertyId::mint()),
        Object::Node(node.id),
        operator,
    )
    .citing(entry.id);
    let seed = SeedBuilder::node(seed, node, None);
    let seed = SeedBuilder::edge(seed, edge);
    let seed = SeedBuilder::assertion(seed, claim);
    let seed = SeedBuilder::evidence(seed, evidence);
    let _: SeedDocument = SeedBuilder::build(seed);
    let reference = TypedReference::new(TypeId::mint(), ["a"]);
    let _ = TypedReference::to_yaml(&reference)?;
    let _ = SeedDocument::to_yaml(&SeedBuilder::build(
        OntologySpec::seed(spec, Timestamp::EPOCH)?.0,
    ))?;

    let _ = Ontology::read("{}").is_err();
    let _ = Ontology::node_type(&held, "n");
    let _ = Ontology::edge_type(&held, "n");
    let _ = Ontology::property(&held, "n", "p");
    let _ = Ontology::edge_property(&held, "n", "p");
    let _ = Ontology::ensure(&held, spec, ValidationProfile::V3)?;
    let _ = [
        ValidationProfile::V1,
        ValidationProfile::V2,
        ValidationProfile::V3,
    ];
    let _ = SchemaChange::is_empty(change);
    let _ = SchemaChange::operations(change);
    let _ = SchemaChange::ontology(change);
    let _ = SchemaChange::transaction(change, operator)?;

    let _ = TransactionBuilder::with_schema_version(
        TransactionBuilder::new(operator).with_id(TransactionId::mint()),
        SchemaVersionId::mint(),
    );
    let push = TransactionBuilder::push(
        TransactionBuilder::new(operator),
        Operation::DeleteEdge(EdgeId::mint()),
    );
    let document: TransactionDocument = TransactionBuilder::build(push)?;
    TransactionDocument::check_limits(&document)?;
    let _ = TransactionDocument::to_yaml(&document)?;
    let _ = (
        TRANSACTION_LIMITS,
        DocumentLimit::ALL.map(DocumentLimit::name),
        DocumentLimit::ALL.map(DocumentLimit::bound),
    );
    let _ = [
        DocumentLimit::InputBytes,
        DocumentLimit::Operations,
        DocumentLimit::Evidence,
        DocumentLimit::Depth,
        DocumentLimit::Nodes,
        DocumentLimit::MappingEntries,
        DocumentLimit::SequenceElements,
        DocumentLimit::StringBytes,
        DocumentLimit::KeyBytes,
        DocumentLimit::TotalStringBytes,
    ];
    let _ = matches!(
        error,
        DocumentError::EmptyTransaction
            | DocumentError::MixedSchemaTransaction
            | DocumentError::Limit { .. }
            | DocumentError::Yaml(_)
    );
    let _ = matches!(
        refusal,
        OntologyError::SchemaFixed { .. }
            | OntologyError::Conflict { .. }
            | OntologyError::UnknownName { .. }
            | OntologyError::DuplicateName { .. }
            | OntologyError::Read(_)
    );
    let _ = matches!(outcome, Outcome::Stale(_));
    Ok(())
}

// ----- a store and a session -----

/// The `ekr` binary of this checkout, built once per test process through cargo, as `session.rs`
/// builds it: another package's binary has no `CARGO_BIN_EXE_*`.
fn ekr_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
        let output = std::process::Command::new(cargo)
            .arg("build")
            .arg("--manifest-path")
            .arg(workspace_root().join("Cargo.toml"))
            .args(["--locked", "-p", "ekr", "--bin", "ekr"])
            .arg("--message-format=json-render-diagnostics")
            .stderr(std::process::Stdio::inherit())
            .output()
            .expect("running cargo build -p ekr");
        assert!(output.status.success(), "cargo build -p ekr failed");
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<Json>(line).ok())
            .filter(|message| {
                message["reason"] == "compiler-artifact" && message["target"]["name"] == "ekr"
            })
            .find_map(|message| message["executable"].as_str().map(PathBuf::from))
            .expect("cargo named the ekr executable it built")
    })
}

/// A scratch directory holding a host under validation profile v2, and a store path on the file
/// provider that holds no store yet.
struct World {
    _directory: tempfile::TempDir,
    store: StoreConfig,
    operator: AgentId,
}

impl World {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("a scratch directory");
        let operator = AgentId::mint();
        let validator = AgentId::mint();
        let agent = |id: AgentId, name: &str, capabilities: &[&str]| serde_json::json!({"id": id.to_string(), "name": name, "capabilities": capabilities});
        let host = serde_json::json!({
            "format": "ekr.cli-host/1",
            "tenant": "library",
            "context": {"operator": operator.to_string(), "validator": validator.to_string()},
            "authority": {
                "format": "ekr.authority-state/1",
                "agents": {
                    operator.to_string(): agent(operator, "Catalogue operator", &["propose", "read"]),
                    validator.to_string(): agent(validator, "Catalogue validator", &["validate"]),
                },
                "validation_profile": {
                    "format": "ekr.p1-validation-profile/1",
                    "ruleset": "ekr.p2-deterministic/1",
                    "checks": ["Structural", "Reference", "Type", "Cardinality",
                               "OntologyConstraint", "Provenance", "Authorization"],
                    "validator": validator.to_string(),
                    "proposer_separation": "distinct-authenticated-actor/1",
                    "provenance": "retained-admissible-evidence/1",
                    "application": "ekr.p2-apply/1"
                }
            }
        });
        let host_path = directory.path().join("host.json");
        std::fs::write(&host_path, host.to_string()).expect("writing the host");
        Self {
            store: StoreConfig {
                host: host_path,
                store: directory.path().join("store"),
                backend: Backend::File,
            },
            _directory: directory,
            operator,
        }
    }
}

fn committed(reply: &ekr_sdk::reply::Reply) -> bool {
    matches!(reply.answer(), Answer::Outcome(Outcome::Committed(_)))
}

// ----- the examples -----

#[test]
fn examples_one_to_three_seed_build_ensure_and_commit_on_a_real_store() -> Result<(), Box<dyn Error>>
{
    let world = World::new();
    let operator = world.operator;
    let binary = EkrBinary::open(ekr_path())?;
    let mut session =
        ProcessSession::start(&binary, world.store.clone(), SessionOptions::default())?;

    // docs/sdk.md, Documents: example 1
    use ekr_sdk::document::{
        Cardinality, EdgeTypeSpec, NodeDraft, NodeTypeSpec, OntologySpec, PropertySpec, Timestamp,
        ValueSpec,
    };
    use ekr_sdk::transport::{Request, Transport};

    let spec = OntologySpec::new()
        .with_node_type(NodeTypeSpec::new("Author"))
        .with_node_type(
            NodeTypeSpec::new("Book")
                .with_property(PropertySpec::new("title", ValueSpec::String).as_required()),
        )
        .with_edge_type(
            EdgeTypeSpec::new("WROTE", ["Author"], ["Book"]).with_cardinality(Cardinality::Many),
        );
    let (seed, names) = spec.seed(Timestamp::from_millis(1_790_000_000_000))?;
    let root = seed.root_id();
    let author_type = names.node_type("Author").ok_or("no Author type")?;
    let author =
        NodeDraft::new(root, author_type, "The Field Naturalist").with_alias("field-naturalist");
    let seed = seed.node(author, None).build();
    let seeded = session.request(&Request::new(["seed", "-"]).with_stdin(seed.to_yaml()?))?;
    // end of example
    assert_eq!(seeded.exit, 0, "{seeded:?}");
    assert_eq!(
        seeded.document.as_ref().map(|d| &d["result"]["revision"]),
        Some(&Json::from(0)),
        "{seeded:?}"
    );

    // docs/sdk.md, Documents: example 2
    use ekr_sdk::document::{
        payload_hash, Assertion, Confidence, EvidenceAddition, EvidenceSource, Object, Predicate,
        Subject, TransactionBuilder, Value,
    };

    let book_type = names.node_type("Book").ok_or("no Book type")?;
    let title = names.property("Book", "title").ok_or("no Book.title")?;
    let name = Value::String("A Field Guide to Lichens".into());
    let book = NodeDraft::new(root, book_type, "A Field Guide to Lichens")
        .with_property(title, name.clone())
        .with_alias("lichen-guide");
    let statement = b"The title page reads: A Field Guide to Lichens.\n".to_vec();
    let hash = payload_hash(&statement); // what `ekr hash` prints for the same bytes
    let evidence = EvidenceAddition::new(
        EvidenceSource::human("Catalogue desk"),
        operator,
        Timestamp::from_millis(1_790_000_000_000),
        Confidence::CERTAIN,
        statement,
    );
    let claim = Assertion::new(
        root,
        Subject::Node(book.id),
        Predicate::Property(title),
        Object::Value(name),
        operator,
    )
    .citing(evidence.evidence.id);
    let document = TransactionBuilder::new(operator)
        .push(book.into())
        .push(evidence.into())
        .push(claim.into())
        .build()?;

    let id = document.transaction.id.to_string();
    let proposed =
        session.request(&Request::new(["propose", "-"]).with_stdin(document.to_yaml()?))?;
    let validated = session.request(&Request::new(["validate", id.as_str()]))?;
    let committed_book = session.request(&Request::new(["commit", id.as_str()]))?;
    // end of example
    let hashed = session.request(
        &Request::new(["hash", "-"])
            .with_stdin("The title page reads: A Field Guide to Lichens.\n"),
    )?;
    assert_eq!(
        hashed.document.as_ref().map(|d| d["content_hash"].clone()),
        Some(Json::from(hash.to_string())),
        "{hashed:?}"
    );
    assert_eq!(proposed.exit, 0, "{proposed:?}");
    assert!(
        matches!(validated.answer(), Answer::Outcome(Outcome::Validated(_))),
        "{validated:?}"
    );
    assert!(committed(&committed_book), "{committed_book:?}");
    assert_eq!(document.transaction.evidence.len(), 1);

    // docs/sdk.md, Documents: example 3
    use ekr_sdk::batch::Batcher;
    use ekr_sdk::document::{Ontology, ValidationProfile};

    let printed = session.request(&Request::new(["ontology"]))?;
    let held = Ontology::read(&printed.document.unwrap_or_default().to_string())?;
    let grown = spec.with_node_type(
        NodeTypeSpec::new("Journal").with_property(PropertySpec::new("issn", ValueSpec::String)),
    );
    let change = held.ensure(&grown, ValidationProfile::V2)?;
    let report = Batcher::new(operator).commit(&mut session, &[change.operations().to_vec()])?;
    let names = change.ontology();
    let journal_type = names.node_type("Journal").ok_or("no Journal type")?;
    // end of example
    assert_eq!(change.operations().len(), 1, "{:?}", change.operations());
    assert_eq!(report.committed.len(), 1, "{report:?}");
    assert!(
        report.rejected.is_empty() && report.refused.is_none(),
        "{report:?}"
    );
    let reread = session.request(&Request::new(["ontology"]))?;
    let evolved = Ontology::read(&reread.document.unwrap_or_default().to_string())?;
    assert_eq!(evolved.node_type("Journal"), Some(journal_type));
    assert!(evolved.ensure(&grown, ValidationProfile::V1)?.is_empty());
    assert!(matches!(
        held.ensure(&grown, ValidationProfile::V1),
        Err(OntologyError::SchemaFixed { missing: 1 })
    ));
    session.close()?;
    Ok(())
}

#[test]
fn example_four_is_refused_by_the_limit_ekr_propose_names() {
    let operator = AgentId::mint();

    // docs/sdk.md, Documents: example 4
    use ekr_sdk::document::{
        AliasAddition, DocumentError, DocumentLimit, NodeId, TransactionBuilder,
    };

    let node = NodeId::mint();
    let too_many = (0..10_001).fold(TransactionBuilder::new(operator), |builder, n| {
        builder.push(AliasAddition::new(node, format!("alias-{n}")).into())
    });
    let refused = too_many.build().unwrap_err();
    assert!(matches!(
        refused,
        DocumentError::Limit {
            limit: DocumentLimit::Operations,
            bound: 10_000,
            value: 10_001
        }
    ));
    assert_eq!(
        refused.to_string(),
        "transaction document limit: operations (at most 10000): this document has 10001"
    );
    // end of example
    assert_eq!(DocumentLimit::Operations.name(), "operations");
    assert_eq!(
        DocumentLimit::Operations.bound(),
        TRANSACTION_LIMITS.operations
    );
}
