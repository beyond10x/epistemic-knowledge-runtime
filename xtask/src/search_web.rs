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
    let home = home.canonicalize().map_err(|e| e.to_string())?;
    let flags = format!(
        "--remap-path-prefix={}=/ekr-search-web\x1f--remap-path-prefix={}=/cargo",
        copy.display(),
        home.display()
    );
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
