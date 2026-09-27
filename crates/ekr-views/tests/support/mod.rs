//! Test support shared by the `ekr-views` integration tests: fixture stores and the conformance
//! target. Each test binary uses a different part of it.
#![allow(dead_code)]

pub mod fixtures;
pub mod suite;
pub mod target;

use std::path::PathBuf;

/// The workspace root, resolved per process: this crate is `<root>/crates/ekr-views`.
pub fn workspace_root() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .ancestors()
    .nth(2)
    .expect("crates/ekr-views sits two levels below the workspace root")
    .to_path_buf()
}

/// The text of a file under the workspace root.
pub fn read(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}
