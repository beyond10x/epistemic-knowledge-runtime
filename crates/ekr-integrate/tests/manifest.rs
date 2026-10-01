//! The resolver matches identities byte for byte and nothing else (design § 45, amendment A10), so
//! the crate declares no string-similarity dependency, directly or through anything it depends on.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
}

/// The package name of every dependency the crate's own manifest declares.
fn declared_dependencies() -> BTreeSet<String> {
    let names = dependencies_of(&manifest_dir().join("Cargo.toml"));
    assert!(
        names.contains("ekr-graph"),
        "the manifest reader is broken, not the manifest"
    );
    names
}

/// `cargo metadata` for `manifest`, as JSON. Cargo reads its own manifest format, so every spelling
/// it accepts — table form, a `package =` rename, a `[target.…]` table, dev and build dependencies
/// — is read the way Cargo reads it, with no TOML parser of this crate's own.
fn metadata(manifest: &Path, resolve: bool) -> serde_json::Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let mut command = Command::new(cargo);
    command
        .args([
            "metadata",
            "--format-version",
            "1",
            "--offline",
            "--manifest-path",
        ])
        .arg(manifest);
    if resolve {
        command.arg("--locked");
    } else {
        command.arg("--no-deps");
    }
    let output = command.output().expect("cargo runs");
    assert!(
        output.status.success(),
        "cargo metadata refused {}: {}",
        manifest.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata writes JSON")
}

/// The package name of every dependency, of every kind, that the manifest at `manifest` declares.
fn dependencies_of(manifest: &Path) -> BTreeSet<String> {
    let metadata = metadata(manifest, false);
    let manifest = manifest.canonicalize().expect("the manifest exists");
    let package = metadata["packages"]
        .as_array()
        .expect("metadata lists packages")
        .iter()
        .find(|package| {
            package["manifest_path"]
                .as_str()
                .and_then(|path| Path::new(path).canonicalize().ok())
                == Some(manifest.clone())
        })
        .expect("metadata lists the manifest's own package");
    package["dependencies"]
        .as_array()
        .expect("a package lists its dependencies")
        .iter()
        .map(|dependency| {
            dependency["name"]
                .as_str()
                .expect("a dependency names its package")
                .to_owned()
        })
        .collect()
}

/// The reader is held to every spelling Cargo accepts, not only the one this crate happens to use:
/// a table-form entry, a renamed entry, a target-specific table, dev and build dependencies.
#[test]
fn the_reader_sees_every_form_a_dependency_can_be_declared_in() {
    let probe = tempfile::tempdir().expect("a probe directory");
    std::fs::create_dir(probe.path().join("src")).expect("a probe source directory");
    std::fs::write(probe.path().join("src/lib.rs"), "").expect("a probe library");
    std::fs::write(
        probe.path().join("Cargo.toml"),
        concat!(
            "[package]\nname = \"probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n",
            "[workspace]\n\n",
            "[dependencies.strsim]\nversion = \"0.11\"\n\n",
            "[dependencies]\nmatcher = { package = \"fuzzy-matcher\", version = \"0.3\" }\n\n",
            "[target.'cfg(unix)'.dependencies]\nlevenshtein = \"1\"\n\n",
            "[dev-dependencies.eddie]\nversion = \"0.4\"\n\n",
            "[build-dependencies]\ntriple_accel = \"0.4\"\n",
        ),
    )
    .expect("a probe manifest");
    let found = dependencies_of(&probe.path().join("Cargo.toml"));
    let expected: BTreeSet<String> = [
        "strsim",
        "fuzzy-matcher",
        "levenshtein",
        "eddie",
        "triple_accel",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(found, expected);
}

/// Crates whose purpose is approximate string comparison.
const SIMILARITY: &[&str] = &[
    "strsim",
    "levenshtein",
    "edit-distance",
    "distance",
    "eddie",
    "triple_accel",
    "textdistance",
    "jaro_winkler",
    "fuzzy-matcher",
    "fuzzy_matcher",
    "sublime_fuzzy",
    "nucleo",
    "nucleo-matcher",
    "rapidfuzz",
    "ngrammatic",
];

#[test]
fn the_crate_declares_no_string_similarity_dependency() {
    let declared = declared_dependencies();
    let allowed: BTreeSet<String> = [
        "ekr-core",
        "ekr-graph",
        "ekr-ontology",
        // Optional, behind the `schema` feature: derives the extraction document's JSON Schema.
        "schemars",
        "serde",
        "proptest",
        "serde_json",
        "serde_yaml_ng",
        "tempfile",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(
        declared, allowed,
        "every dependency of the resolver is reviewed here; a new one is added to this list only \
         after checking it compares nothing approximately"
    );
    for name in SIMILARITY {
        assert!(!declared.contains(*name), "{name} is a similarity crate");
    }
}

/// The allow-list above reviews direct dependencies; this walks everything they pull in, of every
/// kind, as Cargo resolved it against the workspace's `Cargo.lock`.
#[test]
fn nothing_the_crate_depends_on_transitively_is_a_similarity_crate() {
    let metadata = metadata(&manifest_dir().join("Cargo.toml"), true);
    let name_of = |id: &str| -> String {
        metadata["packages"]
            .as_array()
            .expect("metadata lists packages")
            .iter()
            .find(|package| package["id"].as_str() == Some(id))
            .and_then(|package| package["name"].as_str())
            .expect("every resolved id is a listed package")
            .to_owned()
    };
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .expect("metadata resolves the graph");
    let start = nodes
        .iter()
        .filter_map(|node| node["id"].as_str())
        .find(|id| name_of(id) == "ekr-integrate")
        .expect("the resolve graph holds this crate")
        .to_owned();

    let mut seen = BTreeSet::from([start.clone()]);
    let mut pending = vec![start];
    while let Some(id) = pending.pop() {
        let node = nodes
            .iter()
            .find(|node| node["id"].as_str() == Some(id.as_str()))
            .expect("every reached id is a resolve node");
        for next in node["dependencies"]
            .as_array()
            .expect("a node lists its dependencies")
        {
            let next = next.as_str().expect("a dependency is an id").to_owned();
            if seen.insert(next.clone()) {
                pending.push(next);
            }
        }
    }
    let reached: BTreeSet<String> = seen.iter().map(|id| name_of(id)).collect();
    assert!(
        reached.contains("ekr-graph") && reached.contains("serde"),
        "the walk is broken, not the graph: {reached:?}"
    );
    for name in SIMILARITY {
        assert!(
            !reached.contains(*name),
            "{name} is reachable from ekr-integrate"
        );
    }
}
