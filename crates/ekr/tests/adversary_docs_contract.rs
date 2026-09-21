//! What the unit changed, against the documents that state it to a reader.
//!
//! `story_contract.rs` holds the manifests to the story's dependency tables, and `msrv_contract.rs`
//! holds the declared `rust-version` to the lockfile. Neither asks whether the prose a human reads
//! before building this workspace still describes it. This unit raised the toolchain floor and
//! added six members; `README.md` states both facts and was not in the story's `## Scope`, so
//! nothing reconciled them. The story's first constraint — each crate's doc comment names the ESS
//! domain it implements — is likewise asserted by no case.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/ekr has a workspace root two levels up")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// `1.85`, `1.88.0` -> `(1, 85)`, `(1, 88)`.
fn version(v: &str) -> (u64, u64) {
    let mut it = v.split('.');
    let major = it.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = it.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor)
}

/// `[workspace.package] rust-version`, as the root manifest declares it.
fn declared_rust_version() -> String {
    read("Cargo.toml")
        .lines()
        .find_map(|l| l.trim().strip_prefix("rust-version = "))
        .map(|v| v.trim().trim_matches('"').to_string())
        .expect("[workspace.package] declares rust-version")
}

/// The workspace members this unit added, as paths under `crates/`.
fn crate_members() -> Vec<String> {
    read("Cargo.toml")
        .split_once("members = [")
        .expect("the workspace manifest has a members list")
        .1
        .split_once(']')
        .expect("the members list is closed")
        .0
        .split(',')
        .map(|m| m.trim().trim_matches('"').to_string())
        .filter(|m| m.starts_with("crates/"))
        .collect()
}

/// `README.md` states the minimum toolchain to a reader who is about to install one. It is the only
/// place in this repository that states it in prose, and `cargo` refuses the build below
/// `[workspace.package] rust-version`, so the two have to agree.
#[test]
fn the_readme_states_the_workspace_rust_version() {
    let declared = declared_rust_version();
    let readme = read("README.md");

    let stated: Vec<(usize, String)> = readme
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let rest = line.trim_start().strip_prefix("Rust ")?;
            let token: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            (!token.is_empty()).then(|| (i + 1, token))
        })
        .collect();

    assert!(
        !stated.is_empty(),
        "README.md states no minimum Rust version, while Cargo.toml declares {declared}"
    );

    let disagreements: Vec<String> = stated
        .iter()
        .filter(|(_, v)| version(v) != version(&declared))
        .map(|(line, v)| {
            format!("README.md:{line} says Rust {v}, Cargo.toml declares rust-version = {declared}")
        })
        .collect();

    assert!(
        disagreements.is_empty(),
        "the README's toolchain floor and the workspace's disagree; a reader who installs the \
         README's version gets a hard cargo refusal:\n  {}",
        disagreements.join("\n  ")
    );
}

/// `README.md` § Status describes the workspace's contents. This unit changed those contents.
#[test]
fn the_readme_status_matches_the_workspace_members() {
    let members = crate_members();
    let readme = read("README.md");

    let stale = [
        "holds `xtask` only",
        "no runtime crate exists yet",
        "The workspace holds `xtask` only",
    ];

    let offenders: Vec<String> = readme
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            stale
                .iter()
                .find(|needle| line.contains(*needle))
                .map(|needle| format!("README.md:{}: {needle}", i + 1))
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "the workspace now holds {} crate member(s) ({}), and README.md still tells the reader it \
         holds none:\n  {}",
        members.len(),
        members.join(", "),
        offenders.join("\n  ")
    );
}

/// The crates in the workspace, and the root source file of each.
const CRATE_SOURCES: [(&str, &str); 6] = [
    ("ekr-core", "src/lib.rs"),
    ("ekr-kernel", "src/lib.rs"),
    ("ekr-ontology", "src/lib.rs"),
    ("ekr-graph", "src/lib.rs"),
    ("ekr-store", "src/lib.rs"),
    ("ekr", "src/main.rs"),
];

/// The leading `//!` block of a source file, joined into one line.
fn crate_doc(crate_name: &str, source: &str) -> String {
    read(&format!("crates/{crate_name}/{source}"))
        .lines()
        .take_while(|l| l.trim_start().starts_with("//!") || l.trim().is_empty())
        .map(|l| l.trim_start().trim_start_matches("//!").trim().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The ESS file the `ekr` binary names instead of a domain: it implements none, it composes all
/// four, and `systems/ekr/components.yaml` is where that composition is written down.
const COMPONENTS: &str = "systems/ekr/components.yaml";

/// story:workspace-crate-skeleton, "Constraints the scope carries", first bullet, as amended by the
/// coordinator in correction round 2: "each library crate names the domain it implements
/// (`systems/ekr/domains/<domain>.yaml`; `ekr-core` and `ekr-kernel` both name `kernel`); `ekr`
/// names `systems/ekr/components.yaml`". `missing_docs` under clippy makes the presence of a doc
/// comment part of the exit code; nothing makes what it names part of it. Either way the file the
/// comment names has to exist, or the comment points a reader at nothing.
#[test]
fn every_crate_doc_comment_names_an_existing_ess_domain_file() {
    let marker = "systems/ekr/domains/";
    let mut failures: Vec<String> = Vec::new();

    for (crate_name, source) in CRATE_SOURCES {
        let doc = crate_doc(crate_name, source);

        // The binary composes the domains rather than implementing one.
        if source == "src/main.rs" {
            if !doc.contains(COMPONENTS) {
                failures.push(format!(
                    "crates/{crate_name}/{source} names no {COMPONENTS}; the binary implements no \
                     single domain, so the story requires it to name the composition"
                ));
            } else if !workspace_root().join(COMPONENTS).exists() {
                failures.push(format!(
                    "crates/{crate_name}/{source} names {COMPONENTS}, which does not exist"
                ));
            }
            if doc.contains(marker) {
                failures.push(format!(
                    "crates/{crate_name}/{source} names a {marker}<domain>.yaml; the binary \
                     implements no single domain and must name {COMPONENTS} instead"
                ));
            }
            continue;
        }

        let named: Vec<String> = doc
            .match_indices(marker)
            .map(|(at, _)| {
                let tail = &doc[at + marker.len()..];
                let name: String = tail
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                name
            })
            .filter(|n| !n.is_empty())
            .collect();

        if named.is_empty() {
            failures.push(format!(
                "crates/{crate_name}/{source} names no {marker}<domain>.yaml, which the story's \
                 first constraint requires of each crate"
            ));
            continue;
        }
        for domain in named {
            let path = workspace_root().join(format!("{marker}{domain}.yaml"));
            if !path.exists() {
                failures.push(format!(
                    "crates/{crate_name}/{source} names {marker}{domain}.yaml, which does not exist"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "crate doc comments and the ESS domains under systems/ekr disagree:\n  {}",
        failures.join("\n  ")
    );
}
