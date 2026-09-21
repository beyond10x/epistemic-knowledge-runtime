//! The story is the contract; this pins the manifests to it.
//!
//! `story:workspace-crate-skeleton` states, in its "Constraints the scope carries" section, every
//! crate dependency edge and every external dependency each crate declares. Nothing in `task check`
//! compares that document to the manifests the same unit wrote, so these cases do: the expectations
//! are transcribed into the tables below, from the story at revision 9, so that the case answers a
//! question about this tree alone. They are deliberately not read from the planning store at run
//! time — a case whose expected values live in prose another branch may amend goes red in a tree
//! that changed nothing, and no product test may read what the planning store owns. Amending the
//! story amends these tables, in the same change.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/ekr has a workspace root two levels up")
        .to_path_buf()
}

/// The six crates the story's `## Scope` names, in the order the story lists them.
const CRATES: [&str; 6] = [
    "ekr-core",
    "ekr-kernel",
    "ekr-ontology",
    "ekr-graph",
    "ekr-store",
    "ekr",
];

/// The story's "Crate dependency edges, declared now" list, revision 9.
const EDGES: [(&str, &[&str]); 6] = [
    ("ekr-core", &[]),
    (
        "ekr-kernel",
        &["ekr-core", "ekr-ontology", "ekr-graph", "ekr-store"],
    ),
    ("ekr-ontology", &["ekr-core"]),
    ("ekr-graph", &["ekr-core", "ekr-ontology"]),
    ("ekr-store", &["ekr-core", "ekr-graph"]),
    (
        "ekr",
        &[
            "ekr-core",
            "ekr-kernel",
            "ekr-ontology",
            "ekr-graph",
            "ekr-store",
        ],
    ),
];

/// The story's "External dependencies, declared now per crate" list, revision 9, as
/// `(crate, [dependencies], [dev-dependencies])`.
const EXTERNAL: [(&str, &[&str], &[&str]); 6] = [
    (
        "ekr-core",
        &["uuid", "sha2", "hex", "serde", "serde_json", "thiserror"],
        &["proptest"],
    ),
    (
        "ekr-kernel",
        &["serde", "serde_json", "serde_yaml_ng", "thiserror"],
        &["proptest", "trybuild"],
    ),
    (
        "ekr-ontology",
        &["serde", "serde_json", "serde_yaml_ng", "thiserror"],
        &["proptest"],
    ),
    ("ekr-graph", &["serde", "thiserror"], &["trybuild"]),
    (
        "ekr-store",
        &[
            "eventlog-core",
            "eventlog-sqlite",
            "eventlog-file",
            "serde",
            "serde_json",
            "thiserror",
        ],
        &["tempfile"],
    ),
    ("ekr", &["clap", "serde_json"], &["assert_cmd", "tempfile"]),
];

/// The body of a manifest section, from its `[header]` to the next `[` at column zero.
fn manifest_section<'a>(text: &'a str, header: &str) -> Option<&'a str> {
    let at = text.find(header)?;
    let body = &text[at + header.len()..];
    Some(match body.find("\n[") {
        Some(end) => &body[..end],
        None => body,
    })
}

/// Every `[dependencies]` / `[dev-dependencies]` key of a crate manifest, in both the inline form
/// (`serde.workspace = true`) and the table form (`[dependencies.serde]`). A key declared either
/// way is declared; a case that reads only the inline form is blind to half of TOML.
fn manifest_deps(crate_name: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let path = workspace_root().join(format!("crates/{crate_name}/Cargo.toml"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let mut deps = BTreeSet::new();
    let mut dev = BTreeSet::new();
    let mut section = "";
    for line in text.lines() {
        let line = line.trim_end();
        if line.starts_with('[') {
            let header = line.trim_start_matches('[').trim_end_matches(']');
            section = match header {
                "dependencies" => "deps",
                "dev-dependencies" => "dev",
                _ => "",
            };
            // `[dependencies.<name>]` / `[dev-dependencies.<name>]` declare `<name>` themselves.
            for (prefix, target) in [
                ("dependencies.", &mut deps),
                ("dev-dependencies.", &mut dev),
            ] {
                if let Some(name) = header.strip_prefix(prefix) {
                    let name = name.trim().trim_matches('"');
                    if !name.is_empty() {
                        target.insert(name.to_string());
                    }
                }
            }
            continue;
        }
        if section.is_empty() || line.trim().is_empty() {
            continue;
        }
        let key = line
            .split(['.', '='])
            .next()
            .expect("a manifest line has a first field")
            .trim()
            .to_string();
        if key.is_empty() {
            continue;
        }
        match section {
            "deps" => {
                deps.insert(key);
            }
            "dev" => {
                dev.insert(key);
            }
            _ => {}
        }
    }
    (deps, dev)
}

fn set<'a>(items: impl IntoIterator<Item = &'a &'a str>) -> BTreeSet<String> {
    items.into_iter().map(|s| (*s).to_string()).collect()
}

/// The story's "External dependencies, declared now per crate" list, against the manifests.
#[test]
fn external_dependencies_match_the_story() {
    let mut checked = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for (crate_name, deps_part, dev_part) in EXTERNAL {
        assert!(
            CRATES.contains(&crate_name),
            "the story names a crate the scope does not: {crate_name}"
        );
        let want_deps = set(deps_part);
        let want_dev = set(dev_part);

        let (have_deps, have_dev) = manifest_deps(crate_name);
        // Crate-to-crate edges are checked by their own case; ignore them here.
        let have_deps: BTreeSet<String> = have_deps
            .into_iter()
            .filter(|d| !CRATES.contains(&d.as_str()))
            .collect();

        for missing in want_deps.difference(&have_deps) {
            failures.push(format!(
                "crates/{crate_name}/Cargo.toml [dependencies] lacks `{missing}`, which the story declares"
            ));
        }
        for extra in have_deps.difference(&want_deps) {
            failures.push(format!(
                "crates/{crate_name}/Cargo.toml [dependencies] declares `{extra}`, which the story does not"
            ));
        }
        for missing in want_dev.difference(&have_dev) {
            failures.push(format!(
                "crates/{crate_name}/Cargo.toml [dev-dependencies] lacks `{missing}`, which the story declares"
            ));
        }
        for extra in have_dev.difference(&want_dev) {
            failures.push(format!(
                "crates/{crate_name}/Cargo.toml [dev-dependencies] declares `{extra}`, which the story does not"
            ));
        }
        checked += 1;
    }

    assert_eq!(checked, CRATES.len(), "the story lists a bullet per crate");
    assert!(
        failures.is_empty(),
        "the manifests and story:workspace-crate-skeleton disagree:\n  {}",
        failures.join("\n  ")
    );
}

/// The story's "Crate dependency edges, declared now" list, against the manifests.
#[test]
fn crate_dependency_edges_match_the_story() {
    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for (crate_name, edges) in EDGES {
        let want = set(edges);

        let (have_all, _dev) = manifest_deps(crate_name);
        let have: BTreeSet<String> = have_all
            .into_iter()
            .filter(|d| CRATES.contains(&d.as_str()))
            .collect();

        for missing in want.difference(&have) {
            failures.push(format!(
                "crates/{crate_name}/Cargo.toml lacks the edge to `{missing}` the story declares"
            ));
        }
        for extra in have.difference(&want) {
            failures.push(format!(
                "crates/{crate_name}/Cargo.toml declares an edge to `{extra}` the story does not"
            ));
        }
        checked += 1;
    }

    assert_eq!(checked, CRATES.len(), "the story lists a bullet per crate");
    assert!(
        failures.is_empty(),
        "the crate edges and story:workspace-crate-skeleton disagree:\n  {}",
        failures.join("\n  ")
    );
}

/// Both transcribed tables name every crate the scope names, and no other. The markdown parsing
/// these tables replace derived this from the story; a hand-written table has to be held to it.
#[test]
fn the_expectation_tables_cover_every_crate() {
    let scope: BTreeSet<String> = set(CRATES.iter());
    let edges: BTreeSet<String> = EDGES.iter().map(|(c, _)| (*c).to_string()).collect();
    let external: BTreeSet<String> = EXTERNAL.iter().map(|(c, _, _)| (*c).to_string()).collect();
    assert_eq!(
        edges, scope,
        "the crate-edge table and the story's scope name different crates"
    );
    assert_eq!(
        external, scope,
        "the external-dependency table and the story's scope name different crates"
    );
}

/// The acceptance statement names six workspace members; nothing in `task check` asserts they are
/// members, only that whatever is a member compiles.
#[test]
fn the_six_crates_are_workspace_members() {
    let root = workspace_root().join("Cargo.toml");
    let text = std::fs::read_to_string(&root).expect("the workspace manifest is readable");
    let members: BTreeSet<String> = text
        .split_once("members = [")
        .expect("the workspace manifest has a members list")
        .1
        .split_once(']')
        .expect("the members list is closed")
        .0
        .split(',')
        .map(|m| m.trim().trim_matches('"').to_string())
        .filter(|m| !m.is_empty())
        .collect();

    let missing: Vec<&str> = CRATES
        .iter()
        .copied()
        .filter(|c| !members.contains(&format!("crates/{c}")))
        .collect();
    assert!(
        missing.is_empty(),
        "the acceptance names six workspace members; Cargo.toml omits {missing:?}"
    );
}

/// The column of the first `#` on a TOML line that starts a comment — that is, one outside a
/// basic (`"`) or literal (`'`) string. A whole-line comment and a trailing comment are the same
/// defect; a case that only recognises the first lets the second through.
fn comment_column(line: &str) -> Option<usize> {
    let mut basic = false;
    let mut literal = false;
    for (col, c) in line.char_indices() {
        match c {
            '"' if !literal => basic = !basic,
            '\'' if !basic => literal = !literal,
            '#' if !basic && !literal => return Some(col + 1),
            _ => {}
        }
    }
    None
}

/// "Comments in `Cargo.toml` files: none. A manifest carries values, not explanations."
#[test]
fn no_manifest_carries_a_comment() {
    let mut offenders: Vec<String> = Vec::new();
    let mut manifests: Vec<PathBuf> = vec![workspace_root().join("Cargo.toml")];
    manifests.extend(
        CRATES
            .iter()
            .map(|c| workspace_root().join(format!("crates/{c}/Cargo.toml"))),
    );
    for path in manifests {
        let text = std::fs::read_to_string(&path).expect("manifest is readable");
        for (i, line) in text.lines().enumerate() {
            if let Some(col) = comment_column(line) {
                offenders.push(format!("{}:{}:{}", path.display(), i + 1, col));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "manifests carry comments: {offenders:?}"
    );
}

/// "Every crate opts into `[lints] workspace = true`."
///
/// The `workspace = true` has to sit *inside* the `[lints]` table: reading the whole remainder of
/// the manifest finds the `workspace = true` of any dependency declared after an empty `[lints]`,
/// and reports an opt-out as an opt-in.
#[test]
fn every_crate_opts_into_workspace_lints() {
    let mut offenders: Vec<&str> = Vec::new();
    for c in CRATES {
        let path = workspace_root().join(format!("crates/{c}/Cargo.toml"));
        let text = std::fs::read_to_string(&path).expect("manifest is readable");
        let opted_in = manifest_section(&text, "[lints]")
            .is_some_and(|body| body.lines().any(|l| l.trim() == "workspace = true"));
        if !opted_in {
            offenders.push(c);
        }
    }
    assert!(
        offenders.is_empty(),
        "crates without `[lints] workspace = true`: {offenders:?}"
    );
}

/// A dependency's *name* is not its declaration. The story qualifies three of them, and a
/// dependency whose qualifier is dropped still satisfies every case above while declaring
/// something else: `uuid` without `features = ["v7"]` compiles until the first `Uuid::now_v7`, and
/// an eventlog crate without `tag = "0.2.1"` is a different, unpinned dependency.
///
/// `(name, [required substrings of the declaration])`, from the story's constraint list and the
/// unit brief's eventlog pins.
const QUALIFIED: [(&str, &[&str]); 4] = [
    ("uuid", &["version = \"1\"", "features = [\"v7\"]"]),
    (
        "eventlog-core",
        &[
            "git = \"https://github.com/beyond10x/eventlog\"",
            "tag = \"0.2.1\"",
        ],
    ),
    (
        "eventlog-file",
        &[
            "git = \"https://github.com/beyond10x/eventlog\"",
            "tag = \"0.2.1\"",
        ],
    ),
    (
        "eventlog-sqlite",
        &[
            "git = \"https://github.com/beyond10x/eventlog\"",
            "tag = \"0.2.1\"",
        ],
    ),
];

/// The qualifiers the story attaches to a `[workspace.dependencies]` entry, against the manifest.
#[test]
fn workspace_dependencies_carry_the_story_qualifiers() {
    let text = std::fs::read_to_string(workspace_root().join("Cargo.toml"))
        .expect("the workspace manifest is readable");
    let section = manifest_section(&text, "[workspace.dependencies]")
        .expect("the workspace manifest has a [workspace.dependencies] table");

    let mut failures: Vec<String> = Vec::new();
    for (name, required) in QUALIFIED {
        let Some(line) = section
            .lines()
            .map(str::trim)
            .find(|l| l.starts_with(&format!("{name} =")))
        else {
            failures.push(format!(
                "[workspace.dependencies] declares no `{name}`, which the story requires"
            ));
            continue;
        };
        for needle in required {
            if !line.contains(needle) {
                failures.push(format!(
                    "[workspace.dependencies] `{name}` lacks `{needle}`, which the story requires; \
                     it reads `{line}`"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "the workspace dependency qualifiers and story:workspace-crate-skeleton disagree:\n  {}",
        failures.join("\n  ")
    );
}

/// The class the re-pin above is one instance of: no Rust source in this workspace may name the
/// planning store's path. Those records are the CLI's, are amended on branches this tree never
/// sees, and a case that reads one reports a store edit as a code failure. The rule is lexical, so
/// this case holds its own source to it too — hence the needle is assembled, not written out.
///
/// A walk that reaches nothing passes vacuously, so the count of files it actually read is part of
/// the assertion: at least one root source per crate, plus `xtask`'s. The walk starts at the
/// workspace root rather than at `crates/` and `xtask/`, so a source added at a layout the unit did
/// not anticipate — a top-level `tests/`, a crate outside `crates/` — is covered rather than
/// silently exempt.
#[test]
fn no_rust_source_reads_the_planning_store() {
    let needle = [".engineering", "planning"].join("/");
    let skip = [".git", "target"];
    let mut offenders: Vec<String> = Vec::new();
    let mut visited = 0usize;
    let root = workspace_root();
    let mut stack: Vec<PathBuf> = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let entries =
            std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()));
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                if !skip.iter().any(|s| name == *s) {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "rs") {
                visited += 1;
                let text = std::fs::read_to_string(&path).expect("a source file is readable");
                for (i, line) in text.lines().enumerate() {
                    if line.contains(&needle) {
                        offenders.push(format!(
                            "{}:{}",
                            path.strip_prefix(&root).unwrap_or(&path).display(),
                            i + 1
                        ));
                    }
                }
            }
        }
    }
    offenders.sort();
    let floor = CRATES.len() + 1;
    assert!(
        visited >= floor,
        "the walk read {visited} .rs file(s) under the workspace root, fewer than the {floor} \
         root sources the workspace has; a walk that reaches nothing reports clean"
    );
    assert!(
        offenders.is_empty(),
        "Rust sources read the planning store, which the CLI owns and another branch amends: {offenders:?}"
    );
}
