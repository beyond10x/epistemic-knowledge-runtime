//! Rebuild the checked browser artifact from its independent, locked Rust source.
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn run(root: &Path, check: bool) -> Result<(), String> {
    let toolchain =
        std::fs::read_to_string(root.join("rust-toolchain.toml")).map_err(|e| e.to_string())?;
    let toolchain = toolchain
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("channel = \"")
                .and_then(|s| s.strip_suffix('"'))
        })
        .ok_or("pinned toolchain missing")?;
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        root.join(target)
    };
    let work = target.join("search-web-proof");
    let source = root.join("crates/ekr-search-web");
    let first = rebuild(&source, &work.join("first"), toolchain)?;
    let second = rebuild(&source, &work.join("second"), toolchain)?;
    if first != second {
        return Err("browser WASM differs across independent source/build paths".into());
    }
    if [
        b"/home/".as_slice(),
        b"/Users/".as_slice(),
        b"C:\\Users\\".as_slice(),
    ]
    .iter()
    .any(|path| first.windows(path.len()).any(|bytes| bytes == *path))
    {
        return Err("browser WASM contains a personal build path".into());
    }
    let artifact = root.join("crates/ekr/assets/search.wasm");
    if check {
        let actual =
            std::fs::read(&artifact).map_err(|e| format!("{}: {e}", artifact.display()))?;
        if first != actual {
            return Err("checked browser WASM is stale; run cargo xtask search-web".into());
        }
    } else {
        std::fs::write(&artifact, &first).map_err(|e| e.to_string())?;
    }
    println!(
        "search-web: two independent locked builds agree ({} bytes); artifact {}",
        first.len(),
        if check { "verified" } else { "updated" }
    );
    Ok(())
}
fn copy_source(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(from).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            copy_source(&entry.path(), &to.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name())).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
/// The rustc flags, `\x1f`-separated, that map the source copy to `/ekr-search-web` and Cargo's
/// home to `/cargo`, each path both as given and with its symlinks resolved: rustc sees the form
/// Cargo hands it, which for a `CARGO_HOME` that is a symlink is the link, not its target.
fn remap_flags(copy: &Path, home: &Path) -> Result<String, String> {
    let mut flags = Vec::new();
    for (path, to) in [(copy, "/ekr-search-web"), (home, "/cargo")] {
        let resolved = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        flags.push(format!("--remap-path-prefix={}={to}", path.display()));
        if resolved != path {
            flags.push(format!("--remap-path-prefix={}={to}", resolved.display()));
        }
    }
    Ok(flags.join("\x1f"))
}
fn rebuild(source: &Path, work: &Path, toolchain: &str) -> Result<Vec<u8>, String> {
    let copy = work.join("source");
    // This is a generated source copy, never the caller's checkout or build cache.
    if copy.exists() {
        std::fs::remove_dir_all(&copy).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&copy).map_err(|e| e.to_string())?;
    copy_source(&source.join("src"), &copy.join("src"))?;
    for file in ["Cargo.toml", "Cargo.lock"] {
        std::fs::copy(source.join(file), copy.join(file)).map_err(|e| e.to_string())?;
    }
    let home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .ok_or("Cargo home absent")?;
    let flags = remap_flags(&copy, &home)?;
    let status = Command::new("rustup")
        .args([
            "run",
            toolchain,
            "cargo",
            "build",
            "--locked",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
            "--manifest-path",
        ])
        .arg(copy.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(work.join("target"))
        .env("CARGO_ENCODED_RUSTFLAGS", flags)
        .env_remove("RUSTFLAGS")
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_BUILD_JOBS", "2")
        .env("RUSTC_WRAPPER", "")
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("locked browser build failed: {status}"));
    }
    std::fs::read(work.join("target/wasm32-unknown-unknown/release/ekr_search_web.wasm"))
        .map_err(|e| e.to_string())
}

#[cfg(all(test, unix))]
mod tests {
    use super::remap_flags;

    /// A host whose `CARGO_HOME` is a symlink to another directory: Cargo hands rustc the registry
    /// sources under the path as given, so that path is remapped, and so is the one it resolves to.
    #[test]
    fn a_symlinked_cargo_home_is_remapped_as_given_and_resolved() {
        let root = std::env::temp_dir().join(format!("ekr-xtask-remap-{}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        let (real, link, copy) = (root.join("real"), root.join("link"), root.join("copy"));
        for directory in [&real, &copy] {
            std::fs::create_dir_all(directory).unwrap();
        }
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let flags = remap_flags(&copy, &link);
        std::fs::remove_dir_all(&root).unwrap();
        let flags: Vec<String> = flags.unwrap().split('\x1f').map(str::to_owned).collect();
        let real = real.canonicalize().unwrap_or(real);
        for wanted in [
            format!("--remap-path-prefix={}=/ekr-search-web", copy.display()),
            format!("--remap-path-prefix={}=/cargo", link.display()),
            format!("--remap-path-prefix={}=/cargo", real.display()),
        ] {
            assert!(flags.contains(&wanted), "{wanted} missing from {flags:?}");
        }
    }
}
