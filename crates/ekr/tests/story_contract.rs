//! The story is the contract; this pins the manifests to it.
//!
//! `story:workspace-crate-skeleton` states, in its "Constraints the scope carries" section, every
//! crate dependency edge and every external dependency each crate declares. Nothing in `task check`
//! compares that document to the manifests the same unit wrote, so these cases do: the expectations
//! are transcribed into the tables below, from the story at revision 19, so that the case answers a
//! question about this tree alone. They are deliberately not read from the planning store at run
//! time — a case whose expected values live in prose another branch may amend goes red in a tree
//! that changed nothing, and no product test may read what the planning store owns. Amending the
//! story amends these tables, in the same change.
//!
//! # `ekr-kernel`'s `tempfile`, added by ADR 0007 in wave p1-06
//!
//! `ekr-kernel` gained the commit path, and a commit path is exercised over a lineage on disk:
//! `ekr-store`'s fold is `pub(crate)`, so a store this crate could hold in memory could not answer
//! `head` or `fold` and a case over one would assert nothing. `tempfile` is already this
//! workspace's temporary directory — `ekr-store` and `ekr` both declare it — and it is a
//! dev-dependency, so nothing the runtime ships gains an edge.
//!
//! # `ekr`'s edge to `ekr-store`, removed by ADR 0007 in wave p1-06
//!
//! `architecture-decision-record:0007-the-commit-path-is-the-kernels` drops it, and the table below
//! moves in the same change because this file's own rule says it must. The reason is the invariant:
//! `RevisionLog::append` is a public port that takes a bare event, so a binary that declares
//! `ekr-store` reaches a writer to canonical state without passing a `ValidatedTransaction` through
//! anything. After the ADR, `ekr-kernel` is the only crate in the workspace that declares the store,
//! and [`crate_dependency_edges_match_the_story`] is what holds that — it is not a type, and the
//! ADR says so.
//!
//! # `ekr-store`, amended by ADR 0006 in wave p1-05
//!
//! `architecture-decision-record:0006-ekr-store-bridges-the-async-port` widened `ekr-store`'s
//! declarations after the skeleton story was written: `eventlog-core`'s `EventStore` is async and
//! `eventlog-sqlite` needs a tokio runtime context, so the store owns a current-thread runtime and
//! bridges to a synchronous surface. `tokio` is that runtime; `time` is `CommandMeta.occurred_at`,
//! which the envelope requires and no consumer can build without naming the crate; `ekr-ontology`
//! is `CanonicalGraph.ontology`, which `RevisionLog::fold` returns. The skeleton story's own table
//! is therefore out of date and is amended in the planning store alongside this change; this file
//! is the tree's copy of it, and the two move together.

mod authority_scan;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The workspace root, located at **run time**.
///
/// `std::env::var` and not `env!`. `AGENTS.md` § The gate, added in `c4e436e` — the commit this
/// branch forked from — forbids a test locating this repository with the compile-time macro,
/// because it is baked into the binary: with the shared `CARGO_TARGET_DIR` the same document
/// mandates, a binary compiled in one checkout keeps reading that checkout whatever tree later
/// runs it, and *a guard reading the wrong tree reports clean*.
///
/// This file is the first of the twenty sites the ban names to be fixed, ahead of
/// `task:guards-read-source-through-a-compile-time-path`, because
/// [`only_the_kernel_implements_the_commit_authority`] and
/// [`crate_dependency_edges_match_the_story`] are what `AGENTS.md` invariant 1 says holds its
/// second half — so this one guard cannot wait for that wave. The other nineteen still do.
fn workspace_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time");
    Path::new(&manifest)
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

/// The story's "Crate dependency edges, declared now" list, revision 19.
const EDGES: [(&str, &[&str]); 6] = [
    ("ekr-core", &[]),
    (
        "ekr-kernel",
        &["ekr-core", "ekr-ontology", "ekr-graph", "ekr-store"],
    ),
    ("ekr-ontology", &["ekr-core"]),
    ("ekr-graph", &["ekr-core", "ekr-ontology"]),
    ("ekr-store", &["ekr-core", "ekr-graph", "ekr-ontology"]),
    (
        "ekr",
        &["ekr-core", "ekr-kernel", "ekr-ontology", "ekr-graph"],
    ),
];

/// The story's "External dependencies, declared now per crate" list, revision 19, as
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
        &[
            "eventlog-core",
            "eventlog-file",
            "eventlog-sqlite",
            "proptest",
            "tempfile",
            "time",
            "tokio",
            "trybuild",
        ],
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
            "time",
            "tokio",
        ],
        &["tempfile"],
    ),
    (
        "ekr",
        &[
            "clap",
            "ess-conformance",
            "ess-primitives",
            "serde",
            "serde_json",
            "time",
        ],
        &["assert_cmd", "tempfile"],
    ),
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

/// The scan's matcher, held to both spellings and to the boundary that separates them from a
/// different trait.
///
/// The scan that carries half of invariant 1 is only as good as this predicate, and a predicate
/// exercised only by the tree it happens to walk is one that passes for whatever that tree
/// contains today. Each sample is assembled from the same parts the matcher assembles, so that no
/// line of this file is itself an implementation head — the guard walks this file too.
#[test]
fn the_commit_authority_matcher_reads_a_head_by_shape_and_not_by_spelling() {
    let name = authority_scan::trait_name();

    for accepted in [
        format!("impl {name} for Validations {{"),
        format!("    impl {name} for Validations {{"),
        format!("impl ekr_store::{name} for Attesting {{"),
        format!("impl crate::{name} for Attesting {{"),
        format!("impl<S> {name} for Shared<S> {{"),
    ] {
        assert!(
            authority_scan::implements_commit_authority(&accepted),
            "a legal spelling of the head the guard must see: {accepted}"
        );
    }

    for refused in [
        // Not an implementation of this trait.
        format!("impl My{name} for Validations {{"),
        format!("impl store{name} for Validations {{"),
        // Not an implementation at all.
        format!("pub trait {name} {{"),
        format!("use ekr_store::{name};"),
        format!("/// impl {name} for Validations"),
        format!("    // impl {name} for Validations"),
    ] {
        assert!(
            !authority_scan::implements_commit_authority(&refused),
            "not a head of this trait, and a guard that counted it would report a second authority \
             that is not there: {refused}"
        );
    }
}

/// The other half of AGENTS.md invariant 1, which is not a type and is read here.
///
/// `architecture-decision-record:0007-the-commit-path-is-the-kernels`. `ekr-store`'s fold commits
/// only what a `CommitAuthority` stands behind, and that trait is declared *below* `ekr-kernel`, so
/// Rust cannot say "only that other crate implements it". What holds is this: **the only
/// implementation in any crate's `src/` is `ekr-kernel`'s**, beside the edge case above, which says
/// the only crate that can reach `ekr-store` at all is `ekr-kernel`. Together they are the
/// mechanism the ADR names, stated as two cases rather than as a sentence.
///
/// The walk and the rule are [`authority_scan`], shared with
/// `adversary_p1_06_authority_guard.rs`, which runs the same code over a tree it plants a second
/// implementation into. A guard and a case that reads it must not be two bodies of code: the case
/// that used to read this one re-derived its needle instead, and so could go green for a scan this
/// file does not run.
#[test]
fn only_the_kernel_implements_the_commit_authority() {
    let root = workspace_root();
    let (found, visited) = authority_scan::implementations(&root);

    let floor = CRATES.len() + 1;
    assert!(
        visited >= floor,
        "the walk read {visited} .rs file(s), fewer than the {floor} root sources the workspace \
         has; a walk that reaches nothing reports clean"
    );
    assert!(
        found.iter().any(|at| at.contains("/tests/")),
        "no test in the workspace implements the commit authority, so nothing exercises the rule \
         this case is about and the scan may be reading for a name that has moved: {found:?}"
    );
    assert!(
        authority_scan::second_authorities(&found).is_empty(),
        "AGENTS.md invariant 1 and ADR 0007: the only implementation of ekr-store's \
         CommitAuthority in a crate's src/ is ekr-kernel's, and a second one is a second thing \
         the fold will commit for: {:?}",
        authority_scan::second_authorities(&found)
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
/// an eventlog crate without the verified immutable revision is a different dependency.
///
/// `(name, [required substrings of the declaration])`, from the story's constraint list and the
/// unit brief's eventlog pins.
const QUALIFIED: [(&str, &[&str]); 4] = [
    ("uuid", &["version = \"1\"", "features = [\"v7\"]"]),
    (
        "eventlog-core",
        &[
            "git = \"https://github.com/beyond10x/eventlog\"",
            "rev = \"28e578568846fc860e44a5f7c76b7e807abddc12\"",
        ],
    ),
    (
        "eventlog-file",
        &[
            "git = \"https://github.com/beyond10x/eventlog\"",
            "rev = \"28e578568846fc860e44a5f7c76b7e807abddc12\"",
        ],
    ),
    (
        "eventlog-sqlite",
        &[
            "git = \"https://github.com/beyond10x/eventlog\"",
            "rev = \"28e578568846fc860e44a5f7c76b7e807abddc12\"",
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

#[test]
fn seed_admission_is_kernel_owned_and_the_commit_api_lends_no_writer() {
    let root = workspace_root();
    let commit = std::fs::read_to_string(root.join("crates/ekr-kernel/src/commit.rs")).unwrap();
    for writer in ["pub fn store(", "pub const fn store(", "pub fn store_mut("] {
        assert!(
            !commit.contains(writer),
            "Commit lends its unvalidated writer: {writer}"
        );
    }
    let snapshot = std::fs::read_to_string(root.join("crates/ekr-store/src/snapshot.rs")).unwrap();
    assert!(
        !snapshot.contains("fn into_canonical("),
        "store regained unchecked canonical construction"
    );
    // Amendment 94 and ADR 0009 replaced `admit_seed` with one publication path: `initialize`
    // accepts only a Seeded occurrence at version zero and delegates to `publish`, and `publish`
    // replays the candidate history — the pending occurrence included — through the injected
    // kernel authority before it builds any native append.
    let store = std::fs::read_to_string(root.join("crates/ekr-store/src/eventlog.rs")).unwrap();
    let initialize = &store[store
        .find("fn initialize(&self, publication: &Publication)")
        .expect("the store implements Initialize")..];
    let initialize = &initialize[..initialize.find("\n    }\n").expect("initialize closes")];
    assert!(
        initialize.contains("RevisionPayload::Seeded")
            && initialize.contains("self.publish(publication)"),
        "initialize no longer restricts itself to a Seeded occurrence published through publish"
    );
    let publish = &store[store
        .find("fn publish(&self, publication: &Publication)")
        .expect("the store implements publish")..];
    let staged = publish
        .find("history.occurrences.push(")
        .expect("publish stages the candidate occurrence");
    let admitted = publish[staged..]
        .find(".replay(&history, self.ontology.as_ref(), None)?")
        .map(|at| staged + at)
        .expect("publish replays the staged candidate through the kernel authority");
    let written = publish
        .find("let mut appends")
        .expect("publish builds the native append");
    assert!(
        staged < admitted && admitted < written,
        "publish writes before the kernel authority admits the staged candidate"
    );
    assert!(
        store.contains("self.authority.as_deref().ok_or(StoreError::NoSeedAuthority)"),
        "a store without an injected authority no longer refuses"
    );
    // The type-level `ValidatedSeed` capability was replaced in `edf4799` by crate-private seed
    // admission: only the kernel turns a seed document into the admitted graph a Seeded
    // publication carries.
    let seed = std::fs::read_to_string(root.join("crates/ekr-kernel/src/seed.rs")).unwrap();
    assert!(seed.contains("pub(crate) fn admitted_graph("));
    assert!(!seed.contains("pub fn admitted_graph("));
}
