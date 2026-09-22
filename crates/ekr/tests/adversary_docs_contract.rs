//! What the unit changed, against the documents that state it to a reader.
//!
//! `story_contract.rs` holds the manifests to the story's dependency tables, and `msrv_contract.rs`
//! holds the declared `rust-version` to the lockfile. Neither asks whether the prose a human reads
//! before building this workspace still describes it. This unit raised the toolchain floor and
//! added six members; `README.md` states both facts and was not in the story's `## Scope`, so
//! nothing reconciled them. The story's first constraint — each crate's doc comment names the ESS
//! domain it implements — is likewise asserted by no case.

use std::path::{Path, PathBuf};

#[path = "support/workspace_manifest.rs"]
mod workspace_manifest;

fn workspace_root() -> PathBuf {
    std::path::PathBuf::from(
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
    workspace_manifest::members(&read("Cargo.toml"))
        .expect("workspace member grammar must be supported before checking the README")
        .into_iter()
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
    let members: Vec<String> = crate_members()
        .iter()
        .map(|member| {
            read(&format!("{member}/Cargo.toml"))
                .split_once("[package]")
                .expect("a workspace member has a package")
                .1
                .split('[')
                .next()
                .expect("the package section")
                .lines()
                .find_map(|line| line.trim().strip_prefix("name = "))
                .expect("a product package declares its name")
                .trim_matches('"')
                .to_owned()
        })
        .collect();
    let readme = read("README.md");

    status_matches_members(&readme, &members).expect("README Status must match product members");
}

/// Only the explicit product and utility declarations inside Status establish membership.
fn status_matches_members(readme: &str, members: &[String]) -> Result<(), String> {
    let mut sections = readme
        .match_indices("## Status\n")
        .filter(|(at, _)| *at == 0 || readme.as_bytes()[at - 1] == b'\n');
    let (at, heading) = sections.next().ok_or("README has no Status section")?;
    if sections.next().is_some() {
        return Err("README has more than one Status section".to_owned());
    }
    let offset = at + heading.len();
    let status = readme[offset..]
        .split("\n## ")
        .next()
        .expect("a section exists");
    let names_after = |label: &str| -> Result<Vec<String>, String> {
        let (_, after) = status
            .split_once(label)
            .ok_or_else(|| format!("Status has no {label}"))?;
        if after.contains(label) {
            return Err(format!("Status repeats {label}"));
        }
        let paragraph = after.split("\n\n").next().expect("a paragraph exists");
        let pieces: Vec<&str> = paragraph.split('`').collect();
        if pieces.len().is_multiple_of(2) {
            return Err(format!("{label} has an unclosed code name"));
        }
        Ok(pieces
            .iter()
            .skip(1)
            .step_by(2)
            .map(|name| (*name).to_owned())
            .collect())
    };
    let actual = names_after("Product crates:")?;
    let wanted: std::collections::BTreeSet<&str> = members.iter().map(String::as_str).collect();
    let found: std::collections::BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    if wanted.is_empty() || found != wanted || found.len() != actual.len() {
        return Err(format!(
            "Status product crates {actual:?} differ from Cargo packages {members:?}"
        ));
    }
    let utilities = names_after("Repository utility:")?;
    if utilities != ["xtask"] {
        return Err(format!(
            "Status must identify xtask separately as a repository utility: {utilities:?}"
        ));
    }
    Ok(())
}

#[test]
fn status_membership_refuses_absence_omission_and_invented_crates() {
    let members = vec!["ekr".to_owned(), "ekr-core".to_owned()];
    let valid = "## Status\n\nProduct crates: `ekr`, `ekr-core`.\n\nRepository utility: `xtask`.\n\n## More\n";
    assert!(status_matches_members(valid, &members).is_ok());
    for invalid in [
        "# Readme\nNo status.",
        "Not a heading: ## Status\n\nProduct crates: `ekr`, `ekr-core`.\n\nRepository utility: `xtask`.",
        "## Status\n\nProduct crates: `ekr`.\n\nRepository utility: `xtask`.",
        "## Status\n\nProduct crates: `ekr`, `ekr-core`, `ekr-invented`.\n\nRepository utility: `xtask`.",
        "## Status\n\nProduct crates: `ekr`, `ekr-core`, `xtask`.",
        "## Status\n\nProduct crates: `ekr`.\n\nRepository utility: `xtask`.\n\n## Elsewhere\n`ekr-core`",
    ] {
        assert!(status_matches_members(invalid, &members).is_err(), "{invalid}");
    }
}

#[test]
fn adversary_readme_membership_requires_one_unambiguous_product_inventory() {
    let members = vec!["ekr".to_owned(), "ekr-core".to_owned()];
    for status in [
        "## Status\n\nProduct crates: `ekr`, `ekr-core`, `ekr`.\n\nRepository utility: `xtask`.\n",
        "## Status\n\nProduct crates: `ekr`, `ekr-core`.\n\nRepository utility: `xtask`.\n\n## Status\n",
        "## Status\n\nProduct crates: `ekr`, `ekr-core`.\n\nProduct crates: `ekr`.\n\nRepository utility: `xtask`.\n",
        "## Status\n\nProduct crates: `ekr`, `ekr-core`.\n\nRepository utility: `xtask`, `ekr-extra`.\n",
        "## Status\n\nProduct crates: `ekr`, `ekr-core`.\n\n## More\nRepository utility: `xtask`.\n",
        "## Status\n\nProduct crates: `ekr`, `ekr-core.\n\nRepository utility: `xtask`.\n",
    ] {
        assert!(status_matches_members(status, &members).is_err(), "{status}");
    }
    assert!(status_matches_members(
        "## Status\n\nProduct crates: `ekr-core`, `ekr`.\n\nRepository utility: `xtask`.\n",
        &members,
    )
    .is_ok());
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
