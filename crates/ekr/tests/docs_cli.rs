//! `story:cli-user-documentation`: `docs/cli.md` is held to the binary it describes.
//!
//! The page is written for a reader who has only the documentation and a built `ekr`. Every
//! statement that can drift from the binary is read back here:
//!
//! * every fenced block whose body is an `ekr-seed/2`, `ekr.transaction-document/1` or
//!   `ekr.cli-host/1` document is tagged with that format and parses with the real reader;
//! * the worked example's blocks (those carrying `file=`) seed a fresh store and reach the outcome
//!   each block declares (`outcome=`) on both providers, and the page's read-back commands print
//!   what the page says they print;
//! * the verb table equals the verbs `ekr --help` lists, and the configuration table equals its
//!   global options;
//! * the schema evolution example's blocks (those carrying `evolve=`) seed a store under
//!   validation profile v2 from the worked seed, reach their declared outcomes on both providers,
//!   and `ekr ontology --at 0` and `ekr ontology` print the seed's and the evolved schema;
//! * the operation-kind table equals `ekr operations`, applied/schema-change/refused split
//!   included, and every operation kind the page names anywhere is one the binary has;
//! * the value-kind table names every `ValueKind` and no other, and the worked schema declares a
//!   property of each;
//! * every refusal code and refusal name the page tells a reader to look for is one the runtime's
//!   source emits, and every code a schema change can be refused with is a row of the page or
//!   named unreachable with its reason, each row drawn by a transaction this suite writes;
//! * `README.md`'s first run runs as written on both providers, and `README.md` and `AGENTS.md`
//!   point at the page without repeating each other.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Output;

use ekr::host::CliHostConfigurationV1;
use ekr_kernel::{SeedDocument, TransactionDocument};
use ekr_ontology::ValueKind;
use serde_json::Value;

const SEED: &str = "ekr-seed/2";
const TRANSACTION: &str = "ekr.transaction-document/1";
const HOST: &str = "ekr.cli-host/1";
const BACKENDS: [&str; 2] = ["file", "sqlite"];

/// The repository root, from the per-process `CARGO_MANIFEST_DIR` (`AGENTS.md` § The gate).
fn workspace_root() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .parent()
    .and_then(Path::parent)
    .expect("crates/ekr has a workspace root two levels up")
    .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn page() -> String {
    read("docs/cli.md")
}

/// One fenced block of the page: its info-string words, its body and the line it opens on.
struct Block {
    info: Vec<String>,
    body: String,
    line: usize,
}

impl Block {
    fn tagged(&self, format: &str) -> bool {
        self.info.iter().any(|word| word == format)
    }

    fn attribute(&self, key: &str) -> Option<&str> {
        self.info
            .iter()
            .find_map(|word| word.strip_prefix(key)?.strip_prefix('='))
    }
}

/// Every fenced block, fences at column 0 only (the page indents none).
fn blocks(text: &str) -> Vec<Block> {
    let mut found = Vec::new();
    let mut open: Option<Block> = None;
    for (at, line) in text.lines().enumerate() {
        match open.take() {
            None => {
                if let Some(info) = line.strip_prefix("```") {
                    open = Some(Block {
                        info: info.split_whitespace().map(str::to_owned).collect(),
                        body: String::new(),
                        line: at + 1,
                    });
                }
            }
            Some(mut block) => {
                if line == "```" {
                    found.push(block);
                } else {
                    block.body.push_str(line);
                    block.body.push('\n');
                    open = Some(block);
                }
            }
        }
    }
    assert!(open.is_none(), "docs/cli.md ends inside a fenced block");
    found
}

/// The format a block's body declares at its top level, whatever its tag says.
fn declared_format(body: &str) -> Option<&'static str> {
    body.lines().find_map(|line| match line.trim_end() {
        "format: ekr-seed/2" => Some(SEED),
        "format: ekr.transaction-document/1" => Some(TRANSACTION),
        "  \"format\": \"ekr.cli-host/1\"," | "  \"format\": \"ekr.cli-host/1\"" => Some(HOST),
        _ => None,
    })
}

/// The text of one `## ` section (or `### ` subsection) of the page, heading excluded.
fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let level = heading.split(' ').next().expect("a heading has a level");
    let marker = format!("\n{heading}\n");
    let at = text
        .find(&marker)
        .unwrap_or_else(|| panic!("docs/cli.md has no `{heading}` heading"));
    assert!(
        text[at + marker.len()..].find(&marker).is_none(),
        "docs/cli.md repeats `{heading}`"
    );
    let rest = &text[at + marker.len()..];
    let (mut offset, mut fenced) = (0usize, false);
    for line in rest.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
        }
        let hashes = line.chars().take_while(|c| *c == '#').count();
        if !fenced && hashes > 0 && hashes <= level.len() && line[hashes..].starts_with(' ') {
            return &rest[..offset];
        }
        offset += line.len() + 1;
    }
    rest
}

/// The cells of every table row of a section, header and separator rows excluded.
fn rows(text: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut in_table = false;
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            in_table = false;
            continue;
        }
        let cells: Vec<String> = line
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().to_owned())
            .collect();
        if !in_table {
            // The header row.
            in_table = true;
            continue;
        }
        if cells
            .iter()
            .all(|cell| cell.chars().all(|c| c == '-' || c == ':'))
        {
            continue;
        }
        out.push(cells);
    }
    out
}

/// The single code span a cell consists of, or `None`.
fn code(cell: &str) -> Option<&str> {
    cell.strip_prefix('`')?.strip_suffix('`')
}

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
        command.env_remove(var);
    }
    command
}

fn stdout(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

// 1 --------------------------------------------------------------------------------------------

#[test]
fn every_document_block_is_tagged_and_parses_with_its_real_reader() {
    let page = page();
    let mut parsed: BTreeMap<&str, usize> = BTreeMap::new();
    for block in blocks(&page) {
        let declared = declared_format(&block.body);
        let tags: Vec<&str> = [SEED, TRANSACTION, HOST]
            .into_iter()
            .filter(|format| block.tagged(format))
            .collect();
        assert_eq!(
            tags,
            declared.into_iter().collect::<Vec<_>>(),
            "docs/cli.md:{}: a block declaring {declared:?} must be tagged with exactly that \
             format, so that this suite reads it",
            block.line
        );
        let Some(format) = declared else { continue };
        let result = match format {
            SEED => SeedDocument::from_yaml(&block.body)
                .map(drop)
                .map_err(|e| e.to_string()),
            TRANSACTION => TransactionDocument::parse(block.body.as_bytes())
                .map(drop)
                .map_err(|e| e.to_string()),
            _ => CliHostConfigurationV1::from_json(block.body.as_bytes())
                .map(drop)
                .map_err(|e| e.to_string()),
        };
        if let Err(error) = result {
            panic!(
                "docs/cli.md:{}: the real {format} reader refuses the block: {error}",
                block.line
            );
        }
        *parsed.entry(format).or_default() += 1;
    }
    for (format, least) in [(SEED, 1), (TRANSACTION, 3), (HOST, 1)] {
        assert!(
            parsed.get(format).copied().unwrap_or(0) >= least,
            "docs/cli.md carries fewer than {least} {format} blocks: {parsed:?}"
        );
    }
}

// 2 --------------------------------------------------------------------------------------------

/// One provider in its own directory, holding the worked example's files.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn args(&self, verb: &[&str]) -> Vec<String> {
        let at = self.directory.path();
        let store = match self.backend {
            "file" => at.join("store"),
            _ => at.join("library.db"),
        };
        let mut args = vec![
            "--host".to_owned(),
            at.join("host.json").display().to_string(),
            "--store".to_owned(),
            store.display().to_string(),
            "--backend".to_owned(),
            self.backend.to_owned(),
        ];
        args.extend(verb.iter().map(|a| (*a).to_owned()));
        args
    }

    fn run(&self, verb: &[&str]) -> Output {
        ekr()
            .current_dir(self.directory.path())
            .args(self.args(verb))
            .output()
            .unwrap()
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

fn issue_codes(validation: &Value) -> BTreeSet<String> {
    validation["issues"]
        .as_array()
        .map(|issues| {
            issues
                .iter()
                .map(|issue| issue["code"].as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// The page's read-back commands, as the worked example prints them.
const VALID_AT: &str = "ekr snapshot --valid-at 2020-01-01";
const EXPLAIN: &str = "ekr explain 00000000-0000-4000-a000-000000000501";
const WROTE_ASSERTION: &str = "00000000-0000-4000-a000-000000000501";
const BOOK: &str = "00000000-0000-4000-a000-000000000302";

#[test]
fn the_worked_example_seeds_commits_and_reads_back_on_both_providers() {
    let page = page();
    let example: Vec<Block> = blocks(&page)
        .into_iter()
        .filter(|block| block.attribute("file").is_some())
        .collect();
    let of = |format: &str| -> Vec<&Block> {
        example
            .iter()
            .filter(|block| block.tagged(format))
            .collect()
    };
    assert_eq!(of(HOST).len(), 1, "the worked example has one host file");
    assert_eq!(of(SEED).len(), 1, "the worked example has one seed file");
    let transactions = of(TRANSACTION);
    assert!(
        transactions.len() >= 3,
        "the worked example commits, extends and is refused: {} transaction files",
        transactions.len()
    );
    for needle in [VALID_AT, EXPLAIN] {
        assert!(
            page.contains(needle),
            "docs/cli.md does not show `{needle}`"
        );
    }

    for backend in BACKENDS {
        let world = World {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        for block in &example {
            let name = block.attribute("file").unwrap();
            std::fs::write(world.directory.path().join(name), &block.body).unwrap();
        }
        let seed = of(SEED)[0].attribute("file").unwrap();
        // Every payload file of the example hashes, through `ekr hash`, to a key of the seed's
        // `evidence_payloads`, and every key is one of those files: the page's hashes are the
        // binary's, trailing newline included.
        let payloads: BTreeSet<String> = SeedDocument::from_yaml(&of(SEED)[0].body)
            .unwrap()
            .evidence_payloads
            .keys()
            .map(ToString::to_string)
            .collect();
        let hashed: BTreeSet<String> = example
            .iter()
            .filter(|block| ![SEED, TRANSACTION, HOST].iter().any(|f| block.tagged(f)))
            .map(|block| {
                let hash = world.ok(&["hash", block.attribute("file").unwrap()]);
                hash["content_hash"].as_str().unwrap().to_owned()
            })
            .collect();
        assert_eq!(
            hashed, payloads,
            "{backend}: the example's payload files and hashes"
        );
        let seeded = world.ok(&["seed", seed]);
        assert_eq!(seeded["result"]["revision"], 0, "{backend}: {seeded}");

        let mut revision = 0;
        for block in &transactions {
            let file = block.attribute("file").unwrap();
            let outcome = block.attribute("outcome").unwrap_or_else(|| {
                panic!(
                    "docs/cli.md:{}: a worked transaction declares outcome=",
                    block.line
                )
            });
            let proposed = world.ok(&["propose", file]);
            let id = proposed["transaction_id"].as_str().unwrap().to_owned();
            let validated = world.ok(&["validate", &id]);
            if outcome == "Committed" {
                assert_eq!(
                    validated["kind"], "Validated",
                    "{backend} {file}: {}",
                    validated["issues"]
                );
                let committed = world.ok(&["commit", &id]);
                revision += 1;
                assert_eq!(committed["kind"], "Committed", "{backend} {file}");
                assert_eq!(
                    committed["result"]["revision"], revision,
                    "{backend} {file}"
                );
            } else {
                let codes = outcome
                    .strip_prefix("Rejected:")
                    .unwrap_or_else(|| panic!("{file}: outcome {outcome} is not understood"));
                let wanted: BTreeSet<String> = codes.split(',').map(str::to_owned).collect();
                assert_eq!(validated["kind"], "Rejected", "{backend} {file}");
                assert_eq!(issue_codes(&validated), wanted, "{backend} {file}");
                let refused = world.run(&["commit", &id]);
                assert_eq!(refused.status.code(), Some(2), "{backend} {file}");
                assert!(
                    String::from_utf8_lossy(&refused.stderr)
                        .contains("ekr.kernel.TransactionStateConflict"),
                    "{backend} {file}"
                );
            }
        }

        let head = world.ok(&["head"]);
        assert_eq!(head["revision"], revision, "{backend}");

        let words: Vec<&str> = VALID_AT.split_whitespace().skip(1).collect();
        let snapshot = world.ok(&words);
        let matching: Vec<&str> = snapshot["matching_assertions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap())
            .collect();
        assert!(
            matching.contains(&WROTE_ASSERTION),
            "{backend}: {matching:?}"
        );
        let book = &snapshot["graph"]["graph"]["nodes"][BOOK];
        assert_eq!(book["type_state"], "published", "{backend}: {book}");

        let words: Vec<&str> = EXPLAIN.split_whitespace().skip(1).collect();
        let explained = world.ok(&words);
        let kinds: Vec<&str> = explained["links"]
            .as_array()
            .unwrap()
            .iter()
            .map(|link| link["kind"].as_str().unwrap())
            .collect();
        assert_eq!(
            kinds,
            ["Assertion", "Proposal", "Validation", "Commit", "Evidence"],
            "{backend}"
        );
        let evidence = explained["links"].as_array().unwrap().last().unwrap();
        let text = evidence["text"].as_str().unwrap();
        assert!(
            page.contains(text),
            "{backend}: the explained evidence text {text:?} is not the page's payload"
        );
    }
}

// 3 --------------------------------------------------------------------------------------------

/// The verbs under `Commands:` in `ekr --help`, clap's own `help` excluded.
fn help_verbs(help: &str) -> BTreeSet<String> {
    help.lines()
        .skip_while(|line| *line != "Commands:")
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .filter(|verb| *verb != "help")
        .map(str::to_owned)
        .collect()
}

/// The long options under `Options:` in `ekr --help`, clap's own `--help`/`--version` excluded.
fn help_flags(help: &str) -> BTreeSet<String> {
    help.lines()
        .skip_while(|line| *line != "Options:")
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let flag = trimmed
                .strip_prefix("-h, ")
                .or_else(|| trimmed.strip_prefix("-V, "))
                .unwrap_or(trimmed);
            let flag = flag.strip_prefix("--")?;
            (line.len() - trimmed.len() <= 6)
                .then(|| flag.split_whitespace().next().unwrap().to_owned())
        })
        .filter(|flag| flag != "help" && flag != "version")
        .collect()
}

#[test]
fn the_verb_table_equals_the_verbs_ekr_help_lists() {
    let page = page();
    let help = stdout(&["--help"]);
    let binary = help_verbs(&help);
    assert!(binary.len() >= 10, "ekr --help lists {binary:?}");

    let listed: Vec<String> = rows(section(&page, "## Verbs"))
        .iter()
        .filter_map(|cells| code(&cells[0])?.strip_prefix("ekr ").map(str::to_owned))
        .collect();
    let unique: BTreeSet<String> = listed.iter().cloned().collect();
    assert_eq!(
        unique.len(),
        listed.len(),
        "the verb table repeats a verb: {listed:?}"
    );
    assert_eq!(
        unique, binary,
        "the verb table of docs/cli.md and `ekr --help` disagree"
    );
    for verb in &binary {
        let heading = format!("\n### `ekr {verb}`");
        assert!(
            page.contains(&heading),
            "docs/cli.md has no `### \\`ekr {verb}\\`` section"
        );
    }
}

#[test]
fn the_configuration_table_equals_the_global_options_and_names_their_variables() {
    let page = page();
    let flags = help_flags(&stdout(&["--help"]));
    assert_eq!(
        flags,
        ["backend", "host", "store"].map(str::to_owned).into(),
        "ekr --help"
    );
    let table = rows(section(&page, "## Configuration"));
    let listed: BTreeSet<String> = table
        .iter()
        .filter_map(|cells| code(&cells[0])?.strip_prefix("--").map(str::to_owned))
        .collect();
    assert_eq!(
        listed, flags,
        "the configuration table and `ekr --help` disagree"
    );
    for flag in &flags {
        let variable = format!("`EKR_{}`", flag.to_uppercase());
        let row = table
            .iter()
            .find(|cells| cells[0] == format!("`--{flag}`"))
            .unwrap();
        assert!(
            row.iter().any(|cell| cell == &variable),
            "the --{flag} row does not name {variable}: {row:?}"
        );
    }
}

// 4 --------------------------------------------------------------------------------------------

/// `ekr operations`: each kind and what its line says of it — `applied`, `schema change` (applied
/// under validation profile v2 in a schema-only transaction) or `refused` (applied under neither
/// profile).
fn binary_kinds() -> BTreeMap<String, &'static str> {
    stdout(&["operations"])
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let status = match (
                line.contains("[schema change:"),
                line.contains("[not applied:"),
            ) {
                (false, false) => "applied",
                (true, false) => "schema change",
                (false, true) => "refused",
                (true, true) => panic!("`ekr operations` marks a line both ways: {line:?}"),
            };
            (line.split_whitespace().next().unwrap().to_owned(), status)
        })
        .collect()
}

const KIND_PREFIXES: [&str; 10] = [
    "Create",
    "Update",
    "Delete",
    "Add",
    "Retract",
    "Define",
    "Modify",
    "Merge",
    "Invoke",
    "Supersede",
];

#[test]
fn the_operation_kind_table_and_its_applied_split_match_ekr_operations() {
    let page = page();
    let binary = binary_kinds();
    assert_eq!(binary.len(), 12, "ekr operations: {binary:?}");

    let table: BTreeMap<String, &'static str> = rows(section(&page, "### Operation kinds"))
        .iter()
        .filter_map(|cells| {
            let kind = code(&cells[0])?.to_owned();
            let status = match cells[1].as_str() {
                "applied" => "applied",
                "schema change" => "schema change",
                "refused" => "refused",
                other => {
                    panic!("the {kind} row says {other:?}; write applied, schema change or refused")
                }
            };
            Some((kind, status))
        })
        .collect();
    assert_eq!(
        table.values().filter(|s| **s == "schema change").count(),
        3,
        "three kinds change the schema: {table:?}"
    );
    assert_eq!(
        table, binary,
        "the operation-kind table of docs/cli.md and `ekr operations` disagree"
    );

    // Every kind-shaped name the page writes as code or as a YAML tag is one the binary has.
    let mut named = BTreeSet::new();
    for (at, _) in page.match_indices(['`', '!']) {
        let word: String = page[at + 1..]
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        if KIND_PREFIXES.iter().any(|prefix| word.starts_with(prefix))
            && word.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        {
            named.insert(word);
        }
    }
    let unknown: Vec<&String> = named.iter().filter(|w| !binary.contains_key(*w)).collect();
    assert!(
        unknown.is_empty(),
        "docs/cli.md names operation kinds the binary does not have: {unknown:?}"
    );
}

// 5 --------------------------------------------------------------------------------------------

/// Every `ValueKind`. The match has no `_` arm, so a new kind does not compile until it is here,
/// and then until the page's value-kind table names it.
fn every_value_kind() -> BTreeSet<String> {
    const fn listed(kind: ValueKind) -> usize {
        match kind {
            ValueKind::String => 0,
            ValueKind::Boolean => 1,
            ValueKind::Integer => 2,
            ValueKind::Float => 3,
            ValueKind::Decimal => 4,
            ValueKind::Timestamp => 5,
            ValueKind::Duration => 6,
            ValueKind::NodeRef => 7,
            ValueKind::Enum => 8,
            ValueKind::List => 9,
            ValueKind::Record => 10,
        }
    }
    let all = [
        ValueKind::String,
        ValueKind::Boolean,
        ValueKind::Integer,
        ValueKind::Float,
        ValueKind::Decimal,
        ValueKind::Timestamp,
        ValueKind::Duration,
        ValueKind::NodeRef,
        ValueKind::Enum,
        ValueKind::List,
        ValueKind::Record,
    ];
    for (at, kind) in all.iter().enumerate() {
        assert_eq!(listed(*kind), at);
    }
    all.iter().map(ToString::to_string).collect()
}

#[test]
fn the_value_kind_table_names_every_value_kind_and_the_worked_schema_declares_each() {
    let page = page();
    let every = every_value_kind();
    let table: Vec<String> = rows(section(&page, "### Value types"))
        .iter()
        .filter_map(|cells| code(&cells[0]).map(str::to_owned))
        .collect();
    let unique: BTreeSet<String> = table.iter().cloned().collect();
    assert_eq!(
        unique.len(),
        table.len(),
        "the value-kind table repeats a kind"
    );
    assert_eq!(unique, every, "the value-kind table of docs/cli.md");

    let seed = blocks(&page)
        .into_iter()
        .find(|block| block.tagged(SEED) && block.attribute("file").is_some())
        .expect("the worked example's seed");
    let seed = SeedDocument::from_yaml(&seed.body).unwrap();
    let declared: BTreeSet<String> = seed
        .ontology
        .node_types
        .iter()
        .flat_map(|t| t.properties.values())
        .chain(
            seed.ontology
                .edge_types
                .iter()
                .flat_map(|t| t.properties.values()),
        )
        .map(|p| p.value_type.kind().to_string())
        .collect();
    assert_eq!(
        declared, every,
        "the worked schema declares a property of every value kind"
    );
}

// 6 --------------------------------------------------------------------------------------------

/// Every `.rs` file under the given source directories, concatenated.
fn sources(directories: &[&str]) -> String {
    fn walk(at: &Path, into: &mut String) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, into);
            } else if path.extension().is_some_and(|e| e == "rs") {
                into.push_str(&std::fs::read_to_string(&path).unwrap());
            }
        }
    }
    let mut all = String::new();
    for directory in directories {
        walk(&workspace_root().join(directory), &mut all);
    }
    all
}

/// The `where` cell of a refusal the validators raise as an issue rather than a verb refusing.
const ISSUE: &str = "validation issue";

/// One row of the page's refusal table: the refusal's name, where it arises and its exit status.
struct Refusal {
    name: String,
    at: String,
    exit: i32,
}

fn refusal_table(page: &str) -> Vec<Refusal> {
    rows(section(page, "## Common refusals"))
        .into_iter()
        .map(|cells| {
            assert_eq!(
                cells.len(),
                5,
                "refusal | where | exit | means | fix: {cells:?}"
            );
            let name = code(&cells[0])
                .unwrap_or_else(|| panic!("a refusal row starts with a code span: {cells:?}"));
            Refusal {
                name: name.to_owned(),
                at: cells[1].clone(),
                exit: cells[2]
                    .parse()
                    .unwrap_or_else(|_| panic!("{name}: exit {:?} is not a number", cells[2])),
            }
        })
        .collect()
}

/// Whether `source` holds `name` as a whole string literal — `"name"` — or as the literal a
/// refusal with a detail starts with — `"name: …"`. A truncated or extended name matches neither.
fn emits(source: &str, name: &str) -> bool {
    source.contains(&format!("\"{name}\"")) || source.contains(&format!("\"{name}: "))
}

#[test]
fn every_refusal_the_page_names_is_a_whole_name_the_runtime_emits() {
    let page = page();
    let source = sources(&[
        "crates/ekr/src",
        "crates/ekr-kernel/src",
        "crates/ekr-ontology/src",
        "crates/ekr-graph/src",
        "crates/ekr-store/src",
    ]);
    let table = refusal_table(&page);
    assert!(
        table.len() >= 10,
        "the refusal table has {} rows",
        table.len()
    );
    let missing: Vec<&str> = table
        .iter()
        .map(|row| row.name.as_str())
        .filter(|name| !emits(&source, name))
        .collect();
    assert!(
        missing.is_empty(),
        "docs/cli.md names refusals no runtime source emits as a whole literal: {missing:?}"
    );
    for row in &table {
        assert_eq!(
            row.at == ISSUE,
            row.exit == 0,
            "{}: a validation issue exits 0 and nothing else does",
            row.name
        );
    }
    // A row filed as a validation issue names a code a validator raises, as a whole literal in the
    // validator sources: not merely a string somewhere in the runtime. The schema validator raises
    // the ontology's own codes through `code()` (`crates/ekr-kernel/src/validate/schema.rs`), so a
    // code of `EvolveError::CODES` or `Incompatibility::CODES` counts too; which of those the page
    // lists is held by `every_code_a_schema_change_is_refused_with_is_listed_or_unreachable`.
    let validators = sources(&["crates/ekr-kernel/src/validate"]);
    for call in ["error.code()", "found.code()", "reused.code()"] {
        assert!(
            validators.contains(call),
            "the schema validator no longer raises the ontology's codes through `{call}`"
        );
    }
    let not_raised: Vec<&str> = table
        .iter()
        .filter(|row| row.at == ISSUE)
        .map(|row| row.name.as_str())
        .filter(|code| {
            !validators.contains(&format!("\"{code}\"")) && !ontology_codes().contains(code)
        })
        .collect();
    assert!(
        not_raised.is_empty(),
        "docs/cli.md files these as validation issues; no validator raises them: {not_raised:?}"
    );
    // The matcher itself: a prefix of a real code is not a code.
    assert!(emits(&source, "seed-decode") && !emits(&source, "seed-dec"));
    assert!(emits(&source, "wrong-type") && !emits(&source, "wrong-typ"));
}

/// The worked example's files, in one fresh directory, driven on the file provider.
struct Lab {
    directory: tempfile::TempDir,
}

impl Lab {
    fn new(page: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        for block in blocks(page) {
            if let Some(name) = block.attribute("file") {
                std::fs::write(directory.path().join(name), &block.body).unwrap();
            }
        }
        Self { directory }
    }

    fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.directory.path().join(name)).unwrap()
    }

    /// Writes `name` as `from` with the first `old` replaced by `new`.
    fn edit(&self, from: &str, name: &str, old: &str, new: &str) {
        let body = self.read(from);
        assert!(body.contains(old), "{from} does not contain {old:?}");
        std::fs::write(self.directory.path().join(name), body.replacen(old, new, 1)).unwrap();
    }

    fn run(&self, host: &str, verb: &[&str]) -> Output {
        let at = self.directory.path();
        ekr()
            .current_dir(at)
            .arg("--host")
            .arg(at.join(host))
            .arg("--store")
            .arg(at.join("store"))
            .args(["--backend", "file"])
            .args(verb)
            .output()
            .unwrap()
    }

    fn seeded(page: &str) -> Self {
        let lab = Self::new(page);
        let output = lab.run("host.json", &["seed", "seed.yaml"]);
        assert_eq!(output.status.code(), Some(0), "the worked seed seeds");
        lab
    }
}

const TX_WROTE: &str = "00000000-0000-4000-a000-000000000701";
const TX_UNKNOWN: &str = "00000000-0000-4000-a000-000000000799";

/// One refused command: the verb it ran and what the binary returned.
struct Ran {
    verb: String,
    output: Output,
}

fn ran(lab: &Lab, host: &str, verb: &[&str]) -> Ran {
    Ran {
        verb: verb[0].to_owned(),
        output: lab.run(host, verb),
    }
}

/// Runs each way the test knows to reach `name` from the worked example, returning the refused
/// commands, or `None` for a refusal it has no trigger for.
fn trigger(page: &str, name: &str) -> Option<Vec<Ran>> {
    let seed_edit = |old: &str, new: &str| {
        let lab = Lab::new(page);
        lab.edit("seed.yaml", "edited.yaml", old, new);
        vec![ran(&lab, "host.json", &["seed", "edited.yaml"])]
    };
    let outputs = match name {
        "seed-decode" => seed_edit(
            "    abstract_type: true\n",
            "    abstract_type: true\n    colour: red\n",
        ),
        "seed-ontology" => seed_edit("        to: out_of_print", "        to: pulped"),
        "seed-ontology-lineage" => seed_edit("    number: 0\n", "    number: 1\n"),
        "seed-root-lineage" => seed_edit("    revision: 0\n", "    revision: 1\n"),
        "seed-schema-version" => seed_edit(
            "schema_version_id: 00000000-0000-4000-a000-000000000001",
            "schema_version_id: 00000000-0000-4000-a000-000000000009",
        ),
        "seed-initial-lifecycle" => seed_edit("type_state: manuscript", "type_state: null"),
        "seed-attribution-mismatch" => seed_edit(
            "extracted_by: 00000000-0000-4000-a000-000000000011",
            "extracted_by: 00000000-0000-4000-a000-000000000012",
        ),
        "seed-misfiled-entity" => seed_edit(
            "      00000000-0000-4000-a000-000000000301:\n",
            "      00000000-0000-4000-a000-000000000399:\n",
        ),
        "seed-misrooted-entity" => seed_edit(
            "        root_id: 00000000-0000-4000-a000-000000000002",
            "        root_id: 00000000-0000-4000-a000-000000000009",
        ),
        "seed-unsupported-source" => seed_edit(
            "source: !HumanStatement\n          identity: Library catalogue desk",
            "source: !Url https://example.org/catalogue",
        ),
        "seed-evidence-payload-missing" => seed_edit("content_hash: b40f", "content_hash: 0000"),
        "seed-evidence-payload-mismatch" => seed_edit(": [84, 104", ": [85, 104"),
        "ekr.kernel.AlreadySeeded" => {
            let lab = Lab::seeded(page);
            lab.edit("seed.yaml", "other.yaml", "value: 212", "value: 213");
            vec![ran(&lab, "host.json", &["seed", "other.yaml"])]
        }
        "seed-authority-profile" => {
            let lab = Lab::new(page);
            lab.edit(
                "host.json",
                "other.json",
                "\"ruleset\": \"ekr.p1-deterministic/1\"",
                "\"ruleset\": \"ekr.p1-other/1\"",
            );
            vec![
                ran(&lab, "other.json", &["seed", "seed.yaml"]),
                ran(&lab, "other.json", &["head"]),
            ]
        }
        "bootstrap-authority-mismatch" => {
            let lab = Lab::seeded(page);
            lab.edit(
                "host.json",
                "other.json",
                "Catalogue validator",
                "Catalogue checker",
            );
            vec![
                ran(&lab, "other.json", &["seed", "seed.yaml"]),
                ran(&lab, "other.json", &["head"]),
                ran(&lab, "other.json", &["propose", "wrote.yaml"]),
            ]
        }
        "ekr.kernel.ProposalAttribution" => {
            let lab = Lab::seeded(page);
            lab.edit(
                "wrote.yaml",
                "edited.yaml",
                "proposer: 00000000-0000-4000-a000-000000000011",
                "proposer: 00000000-0000-4000-a000-000000000012",
            );
            vec![ran(&lab, "host.json", &["propose", "edited.yaml"])]
        }
        "ekr.kernel.StructurallyInvalid" => {
            let lab = Lab::seeded(page);
            lab.edit(
                "wrote.yaml",
                "edited.yaml",
                "assessment: Proposed",
                "assessment: Accepted",
            );
            vec![ran(&lab, "host.json", &["propose", "edited.yaml"])]
        }
        "ekr.kernel.TransactionNotFound" => {
            let lab = Lab::seeded(page);
            vec![
                ran(&lab, "host.json", &["validate", TX_UNKNOWN]),
                ran(&lab, "host.json", &["commit", TX_UNKNOWN]),
            ]
        }
        "ekr.kernel.TransactionStateConflict" => {
            let lab = Lab::seeded(page);
            let proposed = lab.run("host.json", &["propose", "wrote.yaml"]);
            assert_eq!(proposed.status.code(), Some(0));
            let committed_early = ran(&lab, "host.json", &["commit", TX_WROTE]);
            let validated = lab.run("host.json", &["validate", TX_WROTE]);
            assert_eq!(validated.status.code(), Some(0));
            vec![
                committed_early,
                ran(&lab, "host.json", &["validate", TX_WROTE]),
            ]
        }
        "ekr.kernel.AssertionNotFound" => {
            let lab = Lab::seeded(page);
            vec![ran(
                &lab,
                "host.json",
                &["explain", "00000000-0000-4000-a000-000000000599"],
            )]
        }
        "store-not-found" => {
            // Never seeded: `store` holds nothing, and each verb must leave it that way.
            let lab = Lab::new(page);
            let verbs: [&[&str]; 8] = [
                &["propose", "wrote.yaml"],
                &["validate", TX_WROTE],
                &["commit", TX_WROTE],
                &["snapshot"],
                &["explain", "00000000-0000-4000-a000-000000000599"],
                &["head"],
                &["transactions"],
                &["ontology"],
            ];
            let ran: Vec<Ran> = verbs
                .into_iter()
                .map(|verb| ran(&lab, "host.json", verb))
                .collect();
            assert!(
                !lab.directory.path().join("store").exists(),
                "store-not-found: a refused verb created the store"
            );
            ran
        }
        _ => return None,
    };
    Some(outputs)
}

/// The `where` cell meaning every verb that opens the store.
const ANY_STORE_VERB: &str = "any store verb";

/// Verbs that open the store.
const STORE_VERBS: [&str; 9] = [
    "seed",
    "propose",
    "validate",
    "commit",
    "snapshot",
    "explain",
    "head",
    "transactions",
    "ontology",
];

/// Whether `stderr` is `form` followed by `": "` or the end of the line: the refusal's whole name,
/// not a longer one that starts with it.
fn reads_as(stderr: &str, form: &str) -> bool {
    stderr
        .strip_prefix(form)
        .is_some_and(|rest| rest.starts_with(": ") || rest.starts_with('\n'))
}

/// Every refusal row that is not a validation issue is reached from the worked example through
/// each verb its `where` cell names, exits as the row says, and names itself on stderr in the form
/// the page's introduction to the table gives.
#[test]
fn every_refusal_outside_validation_exits_and_reads_as_the_page_says() {
    let page = page();
    let table = refusal_table(&page);
    let mut untriggered = Vec::new();
    for row in table.iter().filter(|row| row.at != ISSUE) {
        let Some(outputs) = trigger(&page, &row.name) else {
            untriggered.push(row.name.clone());
            continue;
        };
        let verbs: BTreeSet<&str> = outputs.iter().map(|r| r.verb.as_str()).collect();
        if row.at == ANY_STORE_VERB {
            assert!(
                verbs.len() >= 2 && verbs.iter().all(|verb| STORE_VERBS.contains(verb)),
                "{}: `any store verb` is triggered through two store verbs, not {verbs:?}",
                row.name
            );
        } else {
            let named: BTreeSet<&str> = row.at.split(", ").collect();
            assert!(
                named.iter().all(|verb| STORE_VERBS.contains(verb)),
                "{}: where {:?} names a verb that is not a store verb",
                row.name,
                row.at
            );
            assert_eq!(
                verbs, named,
                "{}: the where column and the verbs that refuse it disagree",
                row.name
            );
        }
        for Ran { verb, output } in outputs {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(
                output.status.code(),
                Some(row.exit),
                "{} through {verb}: the page says exit {}; stderr {stderr}",
                row.name,
                row.exit
            );
            let form = match (row.exit, row.name.starts_with("ekr.kernel.")) {
                (2, true) => format!("ekr: {}", row.name),
                (2, false) => format!("ekr: ekr.kernel.InvalidSeed: {}", row.name),
                (1, _) if row.name.starts_with("seed-") => {
                    format!("ekr: opening the provider: invalid seed: {}", row.name)
                }
                _ => format!("ekr: {}", row.name),
            };
            assert!(
                reads_as(&stderr, &form),
                "{} through {verb}: stderr {stderr:?} does not read {form:?} then `: ` or the \
                 end of the line",
                row.name
            );
        }
    }
    assert!(
        untriggered.is_empty(),
        "refusal rows this suite does not run, which the page says it runs: {untriggered:?}"
    );
    // The matcher itself: a refusal whose name only starts with the row's does not read as it.
    assert!(!reads_as(
        "ekr: ekr.kernel.InvalidSeed: seed-ontology-lineage\n",
        "ekr: ekr.kernel.InvalidSeed: seed-ontology"
    ));
    assert!(reads_as(
        "ekr: ekr.kernel.InvalidSeed: seed-ontology: the lifecycle …\n",
        "ekr: ekr.kernel.InvalidSeed: seed-ontology"
    ));
}

// 7 --------------------------------------------------------------------------------------------

/// `README.md` § First run is a console block a reader types as it stands: `export` lines set the
/// configuration, `ekr … > file` writes a file, and every `ekr` line exits 0. Run on both
/// providers, it ends in an `explain` of an accepted assertion.
#[test]
fn the_readme_first_run_runs_as_written_on_both_providers() {
    let readme = read("README.md");
    let block = blocks(section(&format!("\n{readme}"), "## First run"))
        .into_iter()
        .find(|block| block.info.first().is_some_and(|w| w == "console"))
        .expect("README.md § First run has a console block");
    let commands: Vec<&str> = block
        .body
        .lines()
        .map(|line| line.split(" #").next().unwrap().trim())
        .filter(|line| !line.is_empty())
        .collect();
    assert!(
        commands.iter().filter(|c| c.starts_with("ekr ")).count() >= 5,
        "{commands:?}"
    );

    for backend in BACKENDS {
        let directory = tempfile::tempdir().unwrap();
        let mut environment: Vec<(String, String)> = Vec::new();
        let mut last = Value::Null;
        for command in &commands {
            if let Some(assignments) = command.strip_prefix("export ") {
                for assignment in assignments.split_whitespace() {
                    let (name, value) = assignment.split_once('=').unwrap();
                    let value = if name == "EKR_BACKEND" {
                        backend
                    } else {
                        value
                    };
                    environment.push((name.to_owned(), value.to_owned()));
                }
                continue;
            }
            let (command, into) = match command.split_once(" > ") {
                Some((command, file)) => (command, Some(file.trim())),
                None => (*command, None),
            };
            let words: Vec<&str> = command
                .strip_prefix("ekr ")
                .unwrap_or_else(|| panic!("README first run: {command:?} is not an ekr command"))
                .split_whitespace()
                .collect();
            let output = ekr()
                .current_dir(directory.path())
                .envs(environment.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .args(&words)
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(0),
                "{backend}: {command}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            match into {
                Some(file) => std::fs::write(directory.path().join(file), &output.stdout).unwrap(),
                None => last = serde_json::from_slice(&output.stdout).unwrap(),
            }
        }
        let links = last["links"]
            .as_array()
            .expect("the run ends in ekr explain");
        assert!(
            links[0]["assessment"]["Accepted"].is_object(),
            "{backend}: {last}"
        );
    }
}

// 8 --------------------------------------------------------------------------------------------

#[test]
fn readme_and_agents_point_at_the_page_without_duplicating_each_other() {
    let readme = read("README.md");
    let agents = read("AGENTS.md");
    assert!(
        readme.contains("docs/cli.md"),
        "README.md links docs/cli.md"
    );
    for needle in ["docs/cli.md", "ekr guide"] {
        assert!(agents.contains(needle), "AGENTS.md names {needle}");
    }
    let readme_lines: BTreeSet<&str> = readme
        .lines()
        .map(str::trim)
        .filter(|line| line.len() >= 60)
        .collect();
    let shared: Vec<&str> = agents
        .lines()
        .map(str::trim)
        .filter(|line| readme_lines.contains(line))
        .collect();
    assert!(
        shared.is_empty(),
        "README.md and AGENTS.md repeat each other; link instead: {shared:?}"
    );
    for (name, text) in [
        ("docs/cli.md", page()),
        ("README.md", readme),
        ("AGENTS.md", agents),
    ] {
        assert!(
            !text.contains("/home/"),
            "{name} carries an absolute home path"
        );
    }
}

// 9 --------------------------------------------------------------------------------------------

/// The ontology's refusal codes, which the schema validator raises through `code()`.
fn ontology_codes() -> BTreeSet<&'static str> {
    ekr_ontology::EvolveError::CODES
        .into_iter()
        .chain(ekr_ontology::Incompatibility::CODES)
        .collect()
}

/// The structural validator's three codes for the shape of a schema-changing transaction.
const SCHEMA_SHAPE_CODES: [&str; 4] = [
    "schema-version-missing",
    "schema-version-without-schema-change",
    "mixed-schema-transaction",
    "modify-property-without-owner",
];

/// The seed version of the worked example, which the evolution example evolves.
const WORKED_SEED_VERSION: &str = "00000000-0000-4000-a000-000000000001";

/// The page's read-back commands for the evolution example.
const MINT_VERSION: &str = "ekr mint schema-version";
const ONTOLOGY_AT_SEED: &str = "ekr ontology --at 0";
const ONTOLOGY_AT_HEAD: &str = "ekr ontology";

/// The evolution example's files, beside the worked example's, in one fresh directory; its store
/// is seeded from the worked seed under the evolution example's own host.
struct Evolution {
    directory: tempfile::TempDir,
    backend: &'static str,
    host: String,
}

impl Evolution {
    fn new(page: &str, backend: &'static str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let mut host = None;
        for block in blocks(page) {
            let name = block
                .attribute("file")
                .or_else(|| block.attribute("evolve"));
            if let Some(name) = name {
                std::fs::write(directory.path().join(name), &block.body).unwrap();
            }
            if block.tagged(HOST) {
                if let Some(name) = block.attribute("evolve") {
                    host = Some(name.to_owned());
                }
            }
        }
        Self {
            directory,
            backend,
            host: host.expect("the evolution example has a host file"),
        }
    }

    fn run(&self, verb: &[&str]) -> Output {
        let at = self.directory.path();
        let store = match self.backend {
            "file" => at.join("evolving-store"),
            _ => at.join("evolving.db"),
        };
        ekr()
            .current_dir(at)
            .arg("--host")
            .arg(at.join(&self.host))
            .arg("--store")
            .arg(store)
            .args(["--backend", self.backend])
            .args(verb)
            .output()
            .unwrap()
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn seeded(page: &str, backend: &'static str) -> Self {
        let evolution = Self::new(page, backend);
        let seeded = evolution.ok(&["seed", "seed.yaml"]);
        assert_eq!(seeded["result"]["revision"], 0, "{backend}: {seeded}");
        evolution
    }

    /// Writes `body` as `name`, proposes and validates it, and returns the validation record.
    fn validate(&self, name: &str, body: &str) -> Value {
        std::fs::write(self.directory.path().join(name), body).unwrap();
        let proposed = self.ok(&["propose", name]);
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        self.ok(&["validate", &id])
    }
}

fn names_of(ontology: &Value, key: &str) -> BTreeSet<String> {
    ontology[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect()
}

/// `docs/cli.md` § Evolve the schema: its host is the worked example's host with validation
/// profile v2 and nothing else changed; seeded from the worked seed, every transaction block of
/// the section (those carrying `evolve=`) reaches the outcome it declares on both providers; and
/// the page's read-back commands print the seed's schema at revision 0 and the evolved one, with
/// its number and parent, at the head.
#[test]
fn the_schema_evolution_example_commits_and_reads_each_version_back_on_both_providers() {
    let page = page();
    let section_text = section(&page, "## Evolve the schema");
    for needle in [MINT_VERSION, ONTOLOGY_AT_SEED, ONTOLOGY_AT_HEAD] {
        assert!(
            section_text
                .lines()
                .any(|line| line.split(" #").next().unwrap().trim() == needle),
            "§ Evolve the schema does not show {needle:?}"
        );
    }
    let evolving: Vec<Block> = blocks(section_text)
        .into_iter()
        .filter(|block| block.attribute("evolve").is_some())
        .collect();
    let hosts: Vec<&Block> = evolving.iter().filter(|b| b.tagged(HOST)).collect();
    assert_eq!(hosts.len(), 1, "the evolution example has one host file");
    let transactions: Vec<&Block> = evolving.iter().filter(|b| b.tagged(TRANSACTION)).collect();
    assert!(
        transactions.len() >= 3,
        "the evolution example changes the schema, uses it and is refused: {} files",
        transactions.len()
    );
    assert!(
        evolving
            .iter()
            .all(|b| b.tagged(HOST) || b.tagged(TRANSACTION)),
        "an evolve= block is a host or a transaction document"
    );

    // The host is the worked host with the v2 ruleset and application, and admits schema changes.
    let worked = blocks(&page)
        .into_iter()
        .find(|b| b.tagged(HOST) && b.attribute("file").is_some())
        .unwrap();
    let v1 = CliHostConfigurationV1::from_json(worked.body.as_bytes()).unwrap();
    let v2 = CliHostConfigurationV1::from_json(hosts[0].body.as_bytes()).unwrap();
    assert!(!v1.authority.validation_profile.admits_schema_changes());
    assert!(v2.authority.validation_profile.admits_schema_changes());
    let mut expected: Value = serde_json::from_str(&worked.body).unwrap();
    expected["authority"]["validation_profile"]["ruleset"] = "ekr.p2-deterministic/1".into();
    expected["authority"]["validation_profile"]["application"] = "ekr.p2-apply/1".into();
    let written: Value = serde_json::from_str(&hosts[0].body).unwrap();
    assert_eq!(
        written, expected,
        "the evolution host differs from the worked host in more than its profile"
    );

    // The types the committed schema changes define, by name.
    let mut defined = BTreeSet::new();
    for block in &transactions {
        if block.attribute("outcome") != Some("Committed") {
            continue;
        }
        let parsed = TransactionDocument::parse(block.body.as_bytes()).unwrap();
        for operation in &parsed.transaction().operations {
            match operation {
                ekr_kernel::GraphOperation::DefineNodeType(t) => {
                    defined.insert(("node_types", t.name.clone()));
                }
                ekr_kernel::GraphOperation::DefineEdgeType(t) => {
                    defined.insert(("edge_types", t.name.clone()));
                }
                _ => {}
            }
        }
    }
    assert!(!defined.is_empty(), "the evolution example defines no type");

    for backend in BACKENDS {
        let evolution = Evolution::seeded(&page, backend);
        let mut revision = 0;
        let mut versions = vec![WORKED_SEED_VERSION.to_owned()];
        for block in &transactions {
            let file = block.attribute("evolve").unwrap();
            let outcome = block.attribute("outcome").unwrap_or_else(|| {
                panic!(
                    "docs/cli.md:{}: an evolve transaction declares outcome=",
                    block.line
                )
            });
            let proposed = evolution.ok(&["propose", file]);
            let id = proposed["transaction_id"].as_str().unwrap().to_owned();
            let validated = evolution.ok(&["validate", &id]);
            let version = TransactionDocument::parse(block.body.as_bytes())
                .unwrap()
                .transaction()
                .schema_version;
            if outcome == "Committed" {
                assert_eq!(
                    validated["kind"], "Validated",
                    "{backend} {file}: {}",
                    validated["issues"]
                );
                let committed = evolution.ok(&["commit", &id]);
                revision += 1;
                assert_eq!(
                    committed["result"]["revision"], revision,
                    "{backend} {file}"
                );
                if let Some(version) = version {
                    versions.push(version.to_string());
                }
            } else {
                let codes = outcome
                    .strip_prefix("Rejected:")
                    .unwrap_or_else(|| panic!("{file}: outcome {outcome} is not understood"));
                let wanted: BTreeSet<String> = codes.split(',').map(str::to_owned).collect();
                assert_eq!(validated["kind"], "Rejected", "{backend} {file}");
                assert_eq!(issue_codes(&validated), wanted, "{backend} {file}");
                let refused = evolution.run(&["commit", &id]);
                assert_eq!(refused.status.code(), Some(2), "{backend} {file}");
            }
        }
        assert!(versions.len() >= 2, "{backend}: no schema change committed");

        let words: Vec<&str> = ONTOLOGY_AT_SEED.split_whitespace().skip(1).collect();
        let seed = evolution.ok(&words);
        assert_eq!(seed["revision"], 0, "{backend}: {seed}");
        assert_eq!(seed["schema_version"], WORKED_SEED_VERSION, "{backend}");
        assert_eq!(seed["schema_version_number"], 0, "{backend}");
        assert_eq!(seed["schema_version_parent"], Value::Null, "{backend}");
        let head = evolution.ok(&["ontology"]);
        assert_eq!(head["revision"], revision, "{backend}: {head}");
        let number = versions.len() - 1;
        assert_eq!(
            head["schema_version"],
            versions[number].as_str(),
            "{backend}"
        );
        assert_eq!(head["schema_version_number"], number, "{backend}");
        assert_eq!(
            head["schema_version_parent"],
            versions[number - 1].as_str(),
            "{backend}"
        );
        for (key, name) in &defined {
            assert!(
                !names_of(&seed, key).contains(name),
                "{backend}: {name} at revision 0"
            );
            assert!(
                names_of(&head, key).contains(name),
                "{backend}: {name} not at the head"
            );
        }
    }
}

/// Codes of the schema class that no `ekr.transaction-document/1` can reach, each with the reason.
/// The page lists every other code of the class; a new code fails
/// `every_code_a_schema_change_is_refused_with_is_listed_or_unreachable` until it is one or the
/// other.
const UNREACHABLE_SCHEMA_CODES: [(&str, &str); 7] = [
    (
        "empty-schema-change",
        "`ekr propose` refuses a document with no operations (ekr.kernel.StructurallyInvalid), and \
         the schema validator reaches Ontology::evolve only when every operation is a schema change",
    ),
    (
        "schema-version-exhausted",
        "the version number would pass u64::MAX",
    ),
    (
        "type-already-declared",
        "the Structural validator reports a type id that is held or defined twice as \
         identity-already-exists or duplicate-identity, and the schema validator then says nothing",
    ),
    ("type-removed", "no operation removes a type"),
    ("property-removed", "no operation removes a property"),
    (
        "type-declaration-changed",
        "ModifyProperty changes only a type's properties, and a type is defined only once",
    ),
    (
        "not-a-successor",
        "the kernel derives the next version from the prior one",
    ),
];

/// A schema-changing transaction against the worked seed, proposed by the worked operator.
fn schema_tx(id: u32, version: Option<&str>, operations: &str) -> String {
    let mut text = format!(
        "format: ekr.transaction-document/1\ntransaction:\n  id: 00000000-0000-4000-a000-00000000{id:04}\n  \
         proposer: 00000000-0000-4000-a000-000000000011\n  operations:{operations}\n  evidence: []\n"
    );
    if let Some(version) = version {
        text.push_str(&format!("  schema_version: {version}\n"));
    }
    text
}

/// A `ModifyProperty` of `owner` in the worked seed, redeclaring one property.
fn modify(owner: &str, property: &str, name: &str, value_type: &str, rest: &str) -> String {
    format!(
        "\n  - !ModifyProperty\n    owner: 00000000-0000-4000-a000-000000000{owner}\n    property:\n      \
         id: 00000000-0000-4000-a000-000000000{property}\n      name: {name}\n      value_type:\n{value_type}{rest}"
    )
}

const JOURNAL: &str = "\n  - !DefineNodeType\n    id: 00000000-0000-4000-a000-000000000105\n    \
name: Journal\n    parents:\n    - 00000000-0000-4000-a000-000000000101\n    properties: {}\n    \
abstract_type: false\n    lifecycle: null\n    operations: {}";
const PAGES: &str = "\n  - !UpdateProperty\n    node: 00000000-0000-4000-a000-000000000302\n    \
property: 00000000-0000-4000-a000-000000000203\n    values:\n    - value_kind: Integer\n      value: 224";
const VERSION: &str = "00000000-0000-4000-a000-000000000009";

/// A transaction against the worked seed under profile v2 that draws `code`, or `None` for a code
/// this suite has no trigger for.
fn schema_trigger(code: &str) -> Option<String> {
    let string = "        value_kind: String\n";
    let one = "      cardinality: One\n      required: false\n      constraints: []";
    Some(match code {
        "schema-version-missing" => schema_tx(1, None, JOURNAL),
        "schema-version-without-schema-change" => schema_tx(2, Some(VERSION), PAGES),
        "mixed-schema-transaction" => schema_tx(3, Some(VERSION), &format!("{JOURNAL}{PAGES}")),
        "modify-property-without-owner" => schema_tx(
            16,
            Some(VERSION),
            "\n  - !ModifyProperty\n    id: 00000000-0000-4000-a000-000000000215\n    name: nickname\n    \
             value_type:\n      value_kind: String\n    cardinality: One\n    required: false\n    \
             constraints: []",
        ),
        "schema-version-reused" => schema_tx(5, Some(WORKED_SEED_VERSION), JOURNAL),
        "unknown-property-owner" => schema_tx(
            6,
            Some(VERSION),
            &modify("199", "214", "issn", string, &format!("\n{one}")),
        ),
        "incoherent-schema" => schema_tx(
            7,
            Some(VERSION),
            "\n  - !DefineEdgeType\n    id: 00000000-0000-4000-a000-000000000106\n    name: EDITED\n    \
             source_types:\n    - 00000000-0000-4000-a000-000000000103\n    target_types:\n    \
             - 00000000-0000-4000-a000-000000000199\n    cardinality: Many\n    properties: {}\n    \
             inverse: null\n    symmetric: false\n    transitive: false",
        ),
        "schema-change-without-effect" => schema_tx(
            8,
            Some(VERSION),
            &modify(
                "101",
                "201",
                "title",
                string,
                "\n      cardinality: One\n      required: true\n      constraints: []",
            ),
        ),
        "required-property-missing" => schema_tx(
            9,
            Some(VERSION),
            &modify(
                "102",
                "208",
                "translation_of",
                "        value_kind: NodeRef\n        parameters:\n          allowed_types:\n          \
                 - 00000000-0000-4000-a000-000000000102\n",
                "\n      cardinality: One\n      required: true\n      constraints: []",
            ),
        ),
        "cardinality-narrowed" => schema_tx(
            10,
            Some(VERSION),
            &modify("102", "210", "subjects", string, &format!("\n{one}")),
        ),
        "value-kind-not-admitted" => schema_tx(
            11,
            Some(VERSION),
            &modify("102", "203", "page_count", string, &format!("\n{one}")),
        ),
        "value-type-narrowed" => schema_tx(
            12,
            Some(VERSION),
            &modify(
                "102",
                "209",
                "binding",
                "        value_kind: Enum\n        parameters:\n          variants:\n          \
                 - hardcover\n          - ebook\n",
                &format!("\n{one}"),
            ),
        ),
        "constraint-changed" => schema_tx(
            13,
            Some(VERSION),
            &modify(
                "102",
                "202",
                "in_print",
                "        value_kind: Boolean\n",
                "\n      cardinality: One\n      required: false\n      constraints: [reviewed]",
            ),
        ),
        _ => return None,
    })
}

/// Every code a schema change can be refused with — the Structural validator's three for the
/// transaction's shape, and every code of `EvolveError` and `Incompatibility`, which the schema
/// validator raises as they are — is either a validation-issue row of the page's refusal table or
/// one of [`UNREACHABLE_SCHEMA_CODES`], never both. Every listed one is drawn, on a store seeded
/// from the worked seed under the evolution example's host, by a transaction this case writes;
/// and the unreachability of `type-already-declared` is measured, not assumed.
#[test]
fn every_code_a_schema_change_is_refused_with_is_listed_or_unreachable() {
    let page = page();
    let class: BTreeSet<&str> = ontology_codes()
        .into_iter()
        .chain(SCHEMA_SHAPE_CODES)
        .collect();
    let listed: BTreeSet<String> = refusal_table(&page)
        .into_iter()
        .filter(|row| row.at == ISSUE && class.contains(row.name.as_str()))
        .map(|row| row.name)
        .collect();
    let unreachable: BTreeSet<&str> = UNREACHABLE_SCHEMA_CODES.iter().map(|(c, _)| *c).collect();
    let both: Vec<&String> = listed
        .iter()
        .filter(|c| unreachable.contains(c.as_str()))
        .collect();
    assert!(both.is_empty(), "listed and called unreachable: {both:?}");
    let neither: Vec<&&str> = class
        .iter()
        .filter(|c| !listed.contains(**c) && !unreachable.contains(**c))
        .collect();
    assert!(
        neither.is_empty(),
        "codes a schema change can be refused with that docs/cli.md does not list: {neither:?}"
    );
    assert!(
        unreachable.iter().all(|c| class.contains(c)),
        "an unreachable code is not of the class: {unreachable:?}"
    );

    let evolution = Evolution::seeded(&page, "file");
    let mut untriggered = Vec::new();
    for code in &listed {
        let Some(document) = schema_trigger(code) else {
            untriggered.push(code.clone());
            continue;
        };
        let validated = evolution.validate(&format!("{code}.yaml"), &document);
        assert_eq!(validated["kind"], "Rejected", "{code}: {validated}");
        assert!(
            issue_codes(&validated).contains(code.as_str()),
            "{code}: the trigger drew {:?}",
            issue_codes(&validated)
        );
    }
    assert!(
        untriggered.is_empty(),
        "listed schema refusals this suite does not draw: {untriggered:?}"
    );

    let redefined = schema_tx(
        14,
        Some(VERSION),
        &JOURNAL.replace("000000000105", "000000000102"),
    );
    let validated = evolution.validate("redefined.yaml", &redefined);
    let codes = issue_codes(&validated);
    assert!(
        codes.contains("identity-already-exists") && !codes.contains("type-already-declared"),
        "a type defined over a held one: {codes:?}"
    );
    // `empty-schema-change`: a versioned document with no operations never reaches validation.
    std::fs::write(
        evolution.directory.path().join("empty.yaml"),
        schema_tx(15, Some(VERSION), " []"),
    )
    .unwrap();
    let empty = evolution.run(&["propose", "empty.yaml"]);
    assert_eq!(empty.status.code(), Some(2), "{empty:?}");
    assert!(
        String::from_utf8_lossy(&empty.stderr).starts_with("ekr: ekr.kernel.StructurallyInvalid"),
        "{empty:?}"
    );
}

// Correction round 1 (p5-01-cli) ---------------------------------------------------------------

/// Phrases that tell a reader only the P1 validation profile is accepted. The binary accepts two
/// profiles (v1 and v2), so no text an agent or a person reads may say either of these.
const P1_ONLY: [&str; 4] = [
    "the P1 profile",
    "P1 validation profile",
    "is not the P1 one",
    "the deterministic P1 validation profile",
];

/// Every text a reader meets that states which validation profile a host carries or a store
/// accepts — `README.md`, `docs/cli.md`, `ekr guide`, every verb's `--help`, and the three
/// JSON Schemas `ekr schema` prints — names both profiles rather than the P1 one alone. The
/// `seed-authority-profile` row names both accepted pairs in its fix.
#[test]
fn no_text_a_reader_meets_says_only_the_p1_profile_is_accepted() {
    let mut texts: Vec<(String, String)> = vec![
        ("README.md".to_owned(), read("README.md")),
        ("docs/cli.md".to_owned(), page()),
        ("ekr guide".to_owned(), stdout(&["guide"])),
    ];
    for format in [SEED, TRANSACTION, HOST] {
        texts.push((format!("ekr schema {format}"), stdout(&["schema", format])));
    }
    let help = stdout(&["--help"]);
    texts.push(("ekr --help".to_owned(), help.clone()));
    for verb in help
        .lines()
        .skip_while(|line| !line.starts_with("Commands:"))
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .filter_map(|line| line.split_whitespace().next())
        .filter(|verb| *verb != "help")
    {
        texts.push((format!("ekr {verb} --help"), stdout(&[verb, "--help"])));
    }
    let mut found = Vec::new();
    for (name, text) in &texts {
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        for phrase in P1_ONLY {
            if flat.contains(phrase) {
                found.push(format!("{name}: {phrase:?}"));
            }
        }
    }
    assert!(
        found.is_empty(),
        "texts that accept only the P1 profile: {found:?}"
    );

    let host = stdout(&["schema", HOST]);
    let schema: Value = serde_json::from_str(&host).unwrap();
    let authority = schema["properties"]["authority"]["description"]
        .as_str()
        .unwrap();
    for needle in [
        "ekr.p1-deterministic/1",
        "ekr.p2-deterministic/1",
        "schema change",
    ] {
        assert!(
            authority.contains(needle),
            "`ekr schema ekr.cli-host/1` authority description lacks {needle:?}: {authority}"
        );
    }

    let page = page();
    let row = refusal_table(&page)
        .into_iter()
        .map(|row| row.name)
        .find(|name| name == "seed-authority-profile")
        .expect("a seed-authority-profile row");
    let line = page
        .lines()
        .find(|line| line.starts_with(&format!("| `{row}` |")))
        .unwrap();
    for needle in [
        "`ekr.p1-deterministic/1` with `ekr.p1-apply/1`",
        "`ekr.p2-deterministic/1` with `ekr.p2-apply/1`",
    ] {
        assert!(
            line.contains(needle),
            "the {row} row lacks {needle:?}: {line}"
        );
    }
    // The row's example cause, measured: a v2 ruleset with the v1 application is refused.
    let lab = Lab::new(&page);
    lab.edit(
        "host.json",
        "mixed.json",
        "\"ruleset\": \"ekr.p1-deterministic/1\"",
        "\"ruleset\": \"ekr.p2-deterministic/1\"",
    );
    let output = lab.run("mixed.json", &["seed", "seed.yaml"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        reads_as(
            &String::from_utf8_lossy(&output.stderr),
            "ekr: opening the provider: invalid seed: seed-authority-profile"
        ),
        "{output:?}"
    );
}

/// `README.md` is true of the latest release, 0.0.5: its status table is headed by it, lists schema
/// evolution under validation profile v2 as working and links the page's § Evolve the schema, and
/// keeps `MergeEntity` and a v1 store's fixed schema as not in it. Schema evolution is no longer
/// called a later phase or unreleased.
#[test]
fn readme_says_the_schema_evolves_under_profile_v2() {
    let readme = read("README.md");
    let flat = readme.split_whitespace().collect::<Vec<_>>().join(" ");
    for stale in [
        "works in 0.0.2",
        "works in 0.0.3",
        "works in 0.0.4",
        "not yet released",
        "the incubation forest, schema evolution and maintenance",
    ] {
        assert!(!flat.contains(stale), "README.md still says {stale:?}");
    }
    let header = readme
        .lines()
        .find(|line| line.starts_with("| works in "))
        .expect("README.md has a status table");
    assert!(header.starts_with("| works in 0.0.5 |"), "{header}");
    let prefixed = format!("\n{readme}");
    let released = section(&prefixed, "## Status");
    let table: String = released
        .lines()
        .filter(|line| line.starts_with('|'))
        .collect::<Vec<_>>()
        .join(" ");
    for needle in [
        "validation profile v2",
        "docs/cli.md#evolve-the-schema",
        "`MergeEntity`, refused as `unsupported-operation`",
        "profile v1",
    ] {
        assert!(
            table.contains(needle),
            "the 0.0.5 table lacks {needle:?}: {table}"
        );
    }
    // The link lands: the page has that heading.
    section(&page(), "## Evolve the schema");
}

/// The `unsupported-constraint` row's fix is true under both profiles: under v2 a
/// `ModifyProperty` can change a property's `constraints`, so "must be `[]` in the seed" is not
/// the only way out.
#[test]
fn the_unsupported_constraint_fix_names_modify_property_under_profile_v2() {
    let page = page();
    let line = page
        .lines()
        .find(|line| line.starts_with("| `unsupported-constraint` |"))
        .expect("an unsupported-constraint row");
    assert!(
        !line.contains("none in P1: those must be `[]` in the seed"),
        "{line}"
    );
    for needle in ["`ModifyProperty`", "profile v2", "constraint-changed"] {
        assert!(line.contains(needle), "the row lacks {needle:?}: {line}");
    }
    // What the fix says, measured under v2: a type declared with a constrained property and no
    // instances has its constraints cleared by a ModifyProperty, and a node of it then commits.
    let evolution = Evolution::seeded(&page, "file");
    let issn = |constraints: &str| {
        modify(
            "105",
            "214",
            "issn",
            "        value_kind: String\n",
            &format!(
                "\n      cardinality: One\n      required: false\n      constraints: {constraints}"
            ),
        )
    };
    let commit = |name: &str, body: &str| {
        std::fs::write(evolution.directory.path().join(name), body).unwrap();
        let proposed = evolution.ok(&["propose", name]);
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        let validated = evolution.ok(&["validate", &id]);
        assert_eq!(validated["kind"], "Validated", "{name}: {validated}");
        assert_eq!(
            evolution.ok(&["commit", &id])["kind"],
            "Committed",
            "{name}"
        );
    };
    commit(
        "constrained.yaml",
        &schema_tx(
            21,
            Some("00000000-0000-4000-a000-000000000021"),
            &format!("{JOURNAL}{}", issn("[reviewed]")),
        ),
    );
    commit(
        "cleared.yaml",
        &schema_tx(
            22,
            Some("00000000-0000-4000-a000-000000000022"),
            &issn("[]"),
        ),
    );
    let node = "\n  - !CreateNode\n    id: 00000000-0000-4000-a000-000000000304\n    \
                root_id: 00000000-0000-4000-a000-000000000002\n    \
                type_id: 00000000-0000-4000-a000-000000000105\n    canonical_name: Q\n    \
                properties:\n      00000000-0000-4000-a000-000000000201:\n      \
                - value_kind: String\n        value: Q\n      \
                00000000-0000-4000-a000-000000000214:\n      \
                - value_kind: String\n        value: x";
    commit("journal-node.yaml", &schema_tx(23, None, node));
}

// Correction round 2 (p5-01-cli) ---------------------------------------------------------------

/// Every whole kebab-case string literal in the given validator sources: the issue codes they
/// raise. (Every such literal under `crates/ekr-kernel/src/validate` is a code; the partition
/// below fails on one that is neither listed nor classified, which is how a new one is found.)
fn validator_codes(files: &[&str]) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for file in files {
        let source = read(file);
        for piece in source.split('"').skip(1).step_by(2) {
            let kebab = piece.contains('-')
                && piece
                    .split('-')
                    .all(|word| !word.is_empty() && word.chars().all(|c| c.is_ascii_lowercase()));
            if kebab {
                found.insert(piece.to_owned());
            }
        }
    }
    found
}

/// The Structural validator's codes that are not about the shape of a schema change. With
/// [`SCHEMA_SHAPE_CODES`] they partition `validate/structural.rs`: a code added there fails
/// `the_structural_codes_are_partitioned_into_schema_shape_and_the_rest` until it is put in one
/// of the two lists, and one put in `SCHEMA_SHAPE_CODES` must then be listed and drawn by
/// `every_code_a_schema_change_is_refused_with_is_listed_or_unreachable`.
const STRUCTURAL_NOT_SCHEMA_SHAPE: [&str; 7] = [
    "empty-transaction",
    "duplicate-identity",
    "identity-already-exists",
    "evidence-set-mismatch",
    "merge-into-itself",
    "conflicting-write",
    "unsupported-operation",
];

#[test]
fn the_structural_codes_are_partitioned_into_schema_shape_and_the_rest() {
    let structural = validator_codes(&["crates/ekr-kernel/src/validate/structural.rs"]);
    let shape: BTreeSet<String> = SCHEMA_SHAPE_CODES.iter().map(|c| (*c).to_owned()).collect();
    let rest: BTreeSet<String> = STRUCTURAL_NOT_SCHEMA_SHAPE
        .iter()
        .map(|c| (*c).to_owned())
        .collect();
    assert!(shape.is_disjoint(&rest), "a code in both lists");
    let classified: BTreeSet<String> = shape.union(&rest).cloned().collect();
    assert_eq!(
        structural, classified,
        "validate/structural.rs raises codes this suite has not classified as schema shape or \
         not (left), or the lists name codes it no longer raises (right)"
    );
}

/// Validator codes a transaction written through `ekr propose` cannot draw, each with the reason
/// and a measurement in the case below.
const NOT_A_VALIDATION_ISSUE_HERE: [(&str, &str); 2] = [
    (
        "empty-transaction",
        "`ekr propose` refuses a document with no operations as ekr.kernel.StructurallyInvalid",
    ),
    (
        "proposer-is-validator",
        "a host whose operator is its validator is refused when the store is opened, exit 1, \
         as `opening the provider: invalid seed: proposer-is-validator`",
    ),
];

/// Every code any validator raises is a validation-issue row of the refusal table, an ontology
/// code (held by the schema-class case), or one of [`NOT_A_VALIDATION_ISSUE_HERE`] with its reason
/// measured here. `merge-into-itself`, which the table lists, is drawn.
#[test]
fn every_code_a_validator_raises_is_a_row_or_measured_unreachable() {
    let page = page();
    let validate = workspace_root().join("crates/ekr-kernel/src/validate");
    let files: Vec<String> = std::fs::read_dir(&validate)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .map(|path| {
            path.strip_prefix(workspace_root())
                .unwrap()
                .display()
                .to_string()
        })
        .collect();
    let files: Vec<&str> = files.iter().map(String::as_str).collect();
    let codes = validator_codes(&files);
    assert!(codes.len() >= 30, "the code scan is broken: {codes:?}");
    let rows: BTreeSet<String> = refusal_table(&page)
        .into_iter()
        .filter(|row| row.at == ISSUE)
        .map(|row| row.name)
        .collect();
    let excluded: BTreeSet<&str> = NOT_A_VALIDATION_ISSUE_HERE
        .iter()
        .map(|(c, _)| *c)
        .collect();
    let unaccounted: Vec<&String> = codes
        .iter()
        .filter(|c| !rows.contains(*c) && !excluded.contains(c.as_str()))
        .collect();
    assert!(
        unaccounted.is_empty(),
        "validator codes docs/cli.md neither lists nor names unreachable: {unaccounted:?}"
    );
    let listed_and_excluded: Vec<&&str> = excluded.iter().filter(|c| rows.contains(**c)).collect();
    assert!(listed_and_excluded.is_empty(), "{listed_and_excluded:?}");

    // empty-transaction: refused before validation.
    let evolution = Evolution::seeded(&page, "file");
    std::fs::write(
        evolution.directory.path().join("empty.yaml"),
        schema_tx(31, None, " []"),
    )
    .unwrap();
    let empty = evolution.run(&["propose", "empty.yaml"]);
    assert_eq!(empty.status.code(), Some(2), "{empty:?}");
    assert!(
        String::from_utf8_lossy(&empty.stderr).starts_with("ekr: ekr.kernel.StructurallyInvalid")
    );

    // proposer-is-validator: refused when the store is opened.
    let lab = Lab::new(&page);
    lab.edit(
        "host.json",
        "same.json",
        "\"validator\": \"00000000-0000-4000-a000-000000000012\"\n  },",
        "\"validator\": \"00000000-0000-4000-a000-000000000011\"\n  },",
    );
    let same = lab.run("same.json", &["seed", "seed.yaml"]);
    assert_eq!(same.status.code(), Some(1), "{same:?}");
    let stderr = String::from_utf8_lossy(&same.stderr);
    assert!(
        stderr.starts_with("ekr: opening the provider: invalid seed: "),
        "{stderr}"
    );

    // merge-into-itself is drawn.
    let merge = "\n  - !MergeEntity\n    absorbed: 00000000-0000-4000-a000-000000000302\n    \
                 into: 00000000-0000-4000-a000-000000000302";
    let validated = evolution.validate("merge.yaml", &schema_tx(32, None, merge));
    assert!(
        issue_codes(&validated).contains("merge-into-itself"),
        "{validated}"
    );
}
