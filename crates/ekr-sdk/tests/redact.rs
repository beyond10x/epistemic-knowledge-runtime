//! `story:credential-redaction-before-model-input`, through the library: a consumer redacts with
//! `ekr_sdk::redaction`, which is `ekr-core`'s function and links no kernel, store or graph.

use ekr_sdk::redaction::{redact_for_model, CredentialShape};

/// The SDK's `redact_for_model` is `ekr_core::redaction`'s: one synthetic credential, built from
/// pieces so no credential-shaped literal sits in this file, is replaced and recorded by shape and
/// span.
#[test]
fn a_consumer_redacts_through_the_sdk() {
    // The same function, not a copy: this assignment compiles only if the SDK's item is
    // `ekr_core`'s.
    let same: fn(&str) -> ekr_core::redaction::ModelInput = ekr_sdk::redaction::redact_for_model;

    let secret = ["AK", "IA", "FIXTURE0", "FIXTURE1"].concat();
    let text = format!("printed {secret} once");

    let projection = redact_for_model(&text);

    assert_eq!(projection.text, "printed [REDACTED:AwsAccessKeyId] once");
    assert_eq!(projection.redactions.len(), 1);
    assert_eq!(
        projection.redactions[0].shape,
        CredentialShape::AwsAccessKeyId
    );
    assert_eq!(&text[projection.redactions[0].span.range()], secret);
    assert_eq!(projection, same(&text));

    let clean = "nothing to hide";
    assert_eq!(redact_for_model(clean).text.as_bytes(), clean.as_bytes());
}

/// `epic:consumer-sdk`: redaction brings none of `ekr-graph`, `ekr-kernel`, `ekr-store` or
/// `ekr-observe` into a consumer's build. `story_contract.rs` and `document_drift.rs` read the
/// SDK's own manifest; this reads the resolved dependency tree, where a transitive edge would
/// show.
#[test]
fn redaction_links_no_graph_kernel_or_store() {
    let manifest = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"),
    )
    .join("Cargo.toml");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = std::process::Command::new(cargo)
        .args(["tree", "--offline", "--locked", "--edges", "normal,build"])
        .args(["--prefix", "none", "--format", "{p}", "--manifest-path"])
        .arg(&manifest)
        .args(["--package", "ekr-sdk"])
        .output()
        .expect("cargo tree runs");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8(output.stdout).expect("cargo tree prints UTF-8");
    let packages: Vec<&str> = tree
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    assert!(
        packages.contains(&"ekr-core"),
        "the tree does not list ekr-core, so it is not the tree this case is about:\n{tree}"
    );
    for unlinked in ["ekr-graph", "ekr-kernel", "ekr-store", "ekr-observe"] {
        assert!(
            !packages.contains(&unlinked),
            "{unlinked} is in the SDK's linked tree:\n{tree}"
        );
    }
}
