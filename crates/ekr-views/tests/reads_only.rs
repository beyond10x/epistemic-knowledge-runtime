//! `story:graph-projection-renderer`: reads only. No propose, validate, commit or seed path — and
//! no provider opening, which may create a store — is reachable from this crate's source, read
//! off the invoking checkout at run time.

use std::path::PathBuf;

fn sources() -> Vec<(PathBuf, String)> {
    let root = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("src");
    let mut pending = vec![root];
    let mut found = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("a source directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let text = std::fs::read_to_string(&path).expect("source text");
                found.push((path, text));
            }
        }
    }
    found
}

#[test]
fn no_write_path_of_the_kernel_is_named_in_this_crates_source() {
    let sources = sources();
    assert!(!sources.is_empty(), "no source was read");
    let forbidden = [
        ".propose(",
        ".propose_reader(",
        ".validate(",
        ".commit(",
        ".seed(",
        "admit_seed",
        "set_full_replay",
        "Runtime::file",
        "Runtime::sqlite",
        "Pipeline",
        "ValidatedTransaction",
        "GraphTransaction",
        "pub use ekr_kernel",
    ];
    for (path, text) in &sources {
        for needle in forbidden {
            assert!(
                !text.contains(needle),
                "{} names `{needle}`",
                path.display()
            );
        }
    }
}
