//! Adversary cases for `task:sdk-docs-cover-the-document-builders`: what
//! `crates/ekr-sdk/tests/docs_examples.rs` does not hold the "Documents" section of `docs/sdk.md`
//! to.
//!
//! That guard compares each Rust block with a region it compiles, and each backticked Rust name
//! of the prose with the file's code. It does not read the limits table, a variant's field list,
//! a fence other than a column-0 three-backtick one, or whether an example imports what it uses:
//! the guard's own module-level `use` supplies names the regions do not import. Each check below
//! runs on the page, and on one edited copy of it in memory to show it has teeth.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ekr_sdk::document::{
    DocumentError, DocumentLimit, Lifecycle, NodeDraft, NodeType, OntologyError, OntologySection,
    SchemaVersion, SeedBuilder, Timestamp, Transition,
};

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

/// The "Documents" section of `docs/sdk.md`, heading to the next `## ` heading.
fn section() -> String {
    let path = workspace_root().join("docs/sdk.md");
    let page = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut lines = page.lines().skip_while(|line| *line != "## Documents");
    let heading = lines.next().expect("a `## Documents` section");
    std::iter::once(heading)
        .chain(lines.take_while(|line| !line.starts_with("## ")))
        .map(|line| format!("{line}\n"))
        .collect()
}

/// The text of each column-0 "```rust" block of `section`, in order.
fn rust_blocks(section: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut open: Option<String> = None;
    for line in section.lines() {
        match (&mut open, line) {
            (None, "```rust") => open = Some(String::new()),
            (Some(_), "```") => blocks.push(open.take().expect("open")),
            (Some(text), _) => {
                text.push_str(line);
                text.push('\n');
            }
            (None, _) => {}
        }
    }
    blocks
}

// ----- the limits table -----

/// Every row of the table under "### The ten limits": the variant, the name and the first number
/// of the bound column, commas removed.
fn limit_rows(section: &str) -> Vec<(String, String, usize)> {
    section
        .lines()
        .skip_while(|line| *line != "### The ten limits")
        .skip_while(|line| !line.starts_with("| `DocumentLimit`"))
        .skip(2)
        .take_while(|line| line.starts_with('|'))
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            let number: String = cells[2]
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == ',')
                .filter(char::is_ascii_digit)
                .collect();
            (
                cells[0].trim_matches('`').to_owned(),
                cells[1].trim_matches('`').to_owned(),
                number.parse().unwrap_or(0),
            )
        })
        .collect()
}

fn check_limits_table(section: &str) -> Result<(), String> {
    let rows = limit_rows(section);
    let mut seen = BTreeSet::new();
    for (variant, name, bound) in &rows {
        let limit = DocumentLimit::ALL
            .into_iter()
            .find(|limit| format!("{limit:?}") == *variant)
            .ok_or_else(|| format!("row `{variant}` is no DocumentLimit"))?;
        if !seen.insert(limit) {
            return Err(format!("row `{variant}` twice"));
        }
        if limit.name() != name {
            return Err(format!(
                "`{variant}` is named `{}`, the page says `{name}`",
                limit.name()
            ));
        }
        if limit.bound() != *bound {
            return Err(format!(
                "`{variant}` is bounded at {}, the page says {bound}",
                limit.bound()
            ));
        }
    }
    if seen.len() != DocumentLimit::ALL.len() {
        return Err(format!("the table has {} of the ten limits", seen.len()));
    }
    Ok(())
}

#[test]
fn the_limits_table_names_each_limit_and_bound_the_sdk_holds() {
    check_limits_table(&section()).unwrap();
}

#[test]
fn the_limits_check_refuses_a_table_with_a_wrong_bound() {
    let edited = section().replace("| 10,000 operations |", "| 1,000 operations |");
    assert_ne!(edited, section(), "the edit applies");
    assert!(check_limits_table(&edited)
        .unwrap_err()
        .contains("Operations"));
}

// ----- imports -----

/// `text` with `//` comments and string literals emptied.
fn strip(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
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
                out.push_str("\"\"");
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            c => out.push(c),
        }
    }
    out
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Every identifier of `code` that starts a path (not after `::` or `.`) and is capitalised.
fn path_heads(code: &str) -> BTreeSet<String> {
    let mut heads = BTreeSet::new();
    let mut at = 0;
    while at < code.len() {
        let c = code[at..].chars().next().expect("in bounds");
        if (c.is_ascii_alphabetic() || c == '_')
            && !code[..at].chars().next_back().is_some_and(is_ident)
        {
            let end = code[at..]
                .find(|c: char| !is_ident(c))
                .map_or(code.len(), |n| at + n);
            let word = &code[at..end];
            let before = code[..at].trim_end();
            if c.is_ascii_uppercase() && !before.ends_with("::") && !before.ends_with('.') {
                heads.insert(word.to_owned());
            }
            at = end;
        } else {
            at += c.len_utf8();
        }
    }
    heads
}

/// Every name the `use` statements of `code` bring in, and `code` without them.
fn uses(code: &str) -> (BTreeSet<String>, String) {
    let mut imported = BTreeSet::new();
    let mut rest = String::new();
    let mut statement: Option<String> = None;
    for line in code.lines() {
        if statement.is_none() && line.trim_start().starts_with("use ") {
            statement = Some(String::new());
        }
        match &mut statement {
            Some(text) => {
                text.push_str(line);
                if line.contains(';') {
                    imported.extend(
                        text.split(|c: char| !is_ident(c))
                            .filter(|word| !word.is_empty())
                            .map(str::to_owned),
                    );
                    statement = None;
                }
                rest.push('\n');
            }
            None => {
                rest.push_str(line);
                rest.push('\n');
            }
        }
    }
    (imported, rest)
}

/// Each block, read as a program continuing the blocks before it, imports every capitalised
/// path head it uses, apart from the prelude's.
fn check_imports(section: &str) -> Result<(), String> {
    let prelude: BTreeSet<&str> = [
        "Some", "None", "Ok", "Err", "String", "Vec", "Box", "Option",
    ]
    .into_iter()
    .collect();
    let mut imported = BTreeSet::new();
    for (number, block) in rust_blocks(section).iter().enumerate() {
        let (brought, rest) = uses(&strip(block));
        imported.extend(brought);
        let missing: Vec<String> = path_heads(&rest)
            .into_iter()
            .filter(|head| !imported.contains(head) && !prelude.contains(head.as_str()))
            .collect();
        if !missing.is_empty() {
            return Err(format!(
                "Rust block {} uses {missing:?}, which no block up to it imports",
                number + 1
            ));
        }
    }
    Ok(())
}

#[test]
fn every_type_a_block_uses_is_imported_by_that_block_or_an_earlier_one() {
    check_imports(&section()).unwrap();
}

#[test]
fn the_import_check_refuses_a_block_that_leans_on_the_guard_s_own_imports() {
    let edited = section().replace(
        "payload_hash, Assertion, Confidence,",
        "payload_hash, Confidence,",
    );
    assert_ne!(edited, section(), "the edit applies");
    assert!(check_imports(&edited)
        .unwrap_err()
        .contains("\"Assertion\""));
}

// ----- field lists -----

/// The fields of every struct variant the section spells out, held by the compiler: each pattern
/// below names every field and no `..`, so a field added, removed or renamed fails to compile.
fn variant_fields() -> BTreeMap<&'static str, &'static [&'static str]> {
    fn exhaustive(refusal: &OntologyError, error: &DocumentError) {
        match refusal {
            OntologyError::SchemaFixed { missing: _ } => {}
            OntologyError::Conflict {
                kind: _,
                name: _,
                reason: _,
            } => {}
            OntologyError::UnknownName { kind: _, name: _ } => {}
            OntologyError::DuplicateName { kind: _, name: _ } => {}
            OntologyError::Read(_) => {}
        }
        if let DocumentError::Limit {
            limit: _,
            bound: _,
            value: _,
        } = error
        {}
    }
    let _ = exhaustive;
    BTreeMap::from([
        ("SchemaFixed", &["missing"][..]),
        ("Conflict", &["kind", "name", "reason"][..]),
        ("UnknownName", &["kind", "name"][..]),
        ("DuplicateName", &["kind", "name"][..]),
        ("Limit", &["limit", "bound", "value"][..]),
    ])
}

/// Every backticked `Name { a, b }` of the prose is a variant of [`variant_fields`] with exactly
/// those fields.
fn check_fields(section: &str) -> Result<usize, String> {
    let known = variant_fields();
    let mut checked = 0;
    for span in section.split('`').skip(1).step_by(2) {
        let Some((head, body)) = span.split_once(" { ") else {
            continue;
        };
        let Some(body) = body.strip_suffix(" }") else {
            continue;
        };
        let variant = head.rsplit("::").next().unwrap_or(head);
        let fields: Vec<&str> = body.split(", ").collect();
        match known.get(variant) {
            Some(expected) if *expected == fields.as_slice() => checked += 1,
            Some(expected) => {
                return Err(format!("`{span}`: {variant} has the fields {expected:?}"));
            }
            None => return Err(format!("`{span}`: no struct variant {variant} is known")),
        }
    }
    Ok(checked)
}

#[test]
fn every_field_list_the_prose_spells_out_is_the_variant_s() {
    assert_eq!(check_fields(&section()), Ok(5));
}

#[test]
fn the_field_check_refuses_a_wrong_field_list() {
    let edited = section().replace(
        "`Conflict { kind, name, reason }`",
        "`Conflict { kind, reason }`",
    );
    assert_ne!(edited, section(), "the edit applies");
    assert!(check_fields(&edited).unwrap_err().contains("Conflict"));
}

// ----- fences -----

/// Every fence line of the section is one the guard reads: at column 0, three backticks.
fn check_fences(section: &str) -> Result<(), String> {
    for line in section.lines() {
        let trimmed = line.trim_start();
        let fence = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        if fence && (!line.starts_with("```") || line.starts_with("````")) {
            return Err(format!("a fence docs_examples.rs does not read: {line:?}"));
        }
    }
    Ok(())
}

#[test]
fn every_fence_of_the_section_is_one_the_guard_reads() {
    check_fences(&section()).unwrap();
}

#[test]
fn the_fence_check_refuses_a_tilde_block() {
    let edited = section().replacen(
        "### The ten limits\n",
        "### The ten limits\n\n~~~rust\nlet _ = ekr_sdk::document::NoSuchThing::new();\n~~~\n",
        1,
    );
    assert!(check_fences(&edited).is_err());
}

// ----- a sentence of the section -----

/// `docs/sdk.md` § "The ontology by name and the seed": "`SeedBuilder::node` adds a `NodeDraft`
/// in its type's `initial` lifecycle state (`None` for a type without a lifecycle)". It adds it in
/// the state its caller passes, whatever the type's lifecycle.
#[test]
#[ignore = "defect: docs/sdk.md says SeedBuilder::node adds a node in its type's initial lifecycle \
            state; it stores whatever type_state its caller passes"]
fn seed_builder_node_puts_a_node_in_its_type_s_initial_state() {
    let mut manuscript = NodeType::new("Manuscript");
    manuscript.lifecycle = Some(Lifecycle::new(
        "Draft",
        ["Draft", "Published"],
        [Transition::new("Draft", "Published")],
    ));
    let section = OntologySection {
        version: SchemaVersion::seed(Timestamp::EPOCH),
        node_types: vec![manuscript.clone()],
        edge_types: Vec::new(),
    };
    let seed = SeedBuilder::new(section, Timestamp::EPOCH);
    let draft = NodeDraft::new(seed.root_id(), manuscript.id, "A manuscript");
    let id = draft.id;
    let document = seed.node(draft, None).build();
    assert_eq!(
        document.graph.graph.nodes[&id].type_state.as_deref(),
        Some("Draft"),
        "the node's state is its type's `initial`"
    );
}
