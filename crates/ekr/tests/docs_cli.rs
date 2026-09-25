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
//! * the operation-kind table equals `ekr operations`, applied/refused split included, and every
//!   operation kind the page names anywhere is one the binary has;
//! * the value-kind table names every `ValueKind` and no other, and the worked schema declares a
//!   property of each;
//! * every refusal code and refusal name the page tells a reader to look for is one the runtime's
//!   source emits;
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

/// `ekr operations`: each kind and whether the P1 kernel applies it.
fn binary_kinds() -> BTreeMap<String, bool> {
    stdout(&["operations"])
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            (
                line.split_whitespace().next().unwrap().to_owned(),
                !line.contains("not applied in P1"),
            )
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

    let table: BTreeMap<String, bool> = rows(section(&page, "### Operation kinds"))
        .iter()
        .filter_map(|cells| {
            let kind = code(&cells[0])?.to_owned();
            let applied = match cells[1].as_str() {
                "applied" => true,
                "refused" => false,
                other => panic!("the {kind} row says {other:?}; write applied or refused"),
            };
            Some((kind, applied))
        })
        .collect();
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

#[test]
fn every_refusal_the_page_names_is_one_the_runtime_emits() {
    let page = page();
    let source = sources(&[
        "crates/ekr/src",
        "crates/ekr-kernel/src",
        "crates/ekr-ontology/src",
        "crates/ekr-graph/src",
    ]);
    let table = rows(section(&page, "## Common refusals"));
    assert!(
        table.len() >= 10,
        "the refusal table has {} rows",
        table.len()
    );
    let mut missing = Vec::new();
    for cells in &table {
        let Some(name) = code(&cells[0]) else {
            panic!("a refusal row starts with a code span: {cells:?}");
        };
        if !source.contains(&format!("\"{name}")) {
            missing.push(name.to_owned());
        }
    }
    assert!(
        missing.is_empty(),
        "docs/cli.md names refusals no runtime source emits: {missing:?}"
    );
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
