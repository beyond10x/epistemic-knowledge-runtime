//! `[workspace.package] rust-version` is a promise every crate in the workspace inherits.
//!
//! The gate runs on whatever toolchain the machine has, so no step in `task check` ever compiles
//! this workspace at its declared minimum. This case asks the resolved graph the question the gate
//! does not: is there a package in `Cargo.lock` whose own `rust-version` is above the one every
//! crate here claims to support?

use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .parent()
    .and_then(std::path::Path::parent)
    .expect("crates/ekr has a workspace root two levels up")
    .to_path_buf()
}

/// `1.85`, `1.88.0` -> `(1, 85)`, `(1, 88)`.
fn msrv(v: &str) -> (u64, u64) {
    let mut it = v.split('.');
    let major = it.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = it.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor)
}

#[test]
fn no_resolved_dependency_needs_more_than_the_declared_rust_version() {
    let root = workspace_root();

    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("workspace manifest");
    let declared = manifest
        .lines()
        .find_map(|l| l.trim().strip_prefix("rust-version = "))
        .map(|v| v.trim().trim_matches('"').to_string())
        .expect("[workspace.package] declares rust-version");
    let floor = msrv(&declared);

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let out = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--offline",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .output()
        .expect("cargo metadata runs");
    assert!(
        out.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let meta: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("cargo metadata emits json");
    let packages = meta["packages"].as_array().expect("metadata has packages");

    let mut over: Vec<String> = Vec::new();
    for p in packages {
        let Some(rv) = p["rust_version"].as_str() else {
            continue;
        };
        if msrv(rv) > floor {
            over.push(format!(
                "{} {} needs rust {rv}",
                p["name"].as_str().unwrap_or("?"),
                p["version"].as_str().unwrap_or("?")
            ));
        }
    }
    over.sort();

    assert!(
        over.is_empty(),
        "the workspace declares rust-version = \"{declared}\", but the lockfile this unit wrote \
         resolves {} package(s) that need more:\n  {}",
        over.len(),
        over.join("\n  ")
    );
}
