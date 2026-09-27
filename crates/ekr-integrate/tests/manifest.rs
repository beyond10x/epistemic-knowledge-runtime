//! The resolver matches identities byte for byte and nothing else (design § 45, amendment A10), so
//! the crate declares no string-similarity dependency, read off its own manifest.

use std::collections::BTreeSet;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
}

/// The dependency names of every `[*dependencies]` table of the crate's manifest.
fn declared_dependencies() -> BTreeSet<String> {
    let text = std::fs::read_to_string(manifest_dir().join("Cargo.toml"))
        .expect("the crate has a manifest");
    let mut names = BTreeSet::new();
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line.trim_matches(['[', ']']).ends_with("dependencies");
            continue;
        }
        if inside && !line.is_empty() && !line.starts_with('#') {
            let key = line.split(['=', '.']).next().expect("a key").trim();
            names.insert(key.to_owned());
        }
    }
    assert!(
        names.contains("ekr-graph"),
        "the manifest scan is broken, not the manifest"
    );
    names
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
        "serde",
        "proptest",
        "serde_json",
        "serde_yaml_ng",
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
