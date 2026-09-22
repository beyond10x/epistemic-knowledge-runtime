//! Repository tasks that are not the product.
//!
//! Every verb here is something the gate or a migration needs and no crate of the runtime should
//! carry. The surface is clap derive, as everywhere in the workspace.

use std::path::Path;

use clap::{Parser, Subcommand};

/// Repository tasks that are not the product.
#[derive(Parser)]
#[command(name = "xtask", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn resolve_workspace(
    manifest: Option<&Path>,
    current: &Path,
) -> Result<std::path::PathBuf, String> {
    let start = manifest.unwrap_or(current);
    if manifest.is_some() && !start.join("Cargo.toml").is_file() {
        return Err(format!(
            "runtime CARGO_MANIFEST_DIR has no Cargo.toml: {}",
            start.display()
        ));
    }
    let start = start
        .canonicalize()
        .map_err(|error| format!("cannot locate {}: {error}", start.display()))?;
    for candidate in start.ancestors() {
        if !candidate.join("Cargo.lock").is_file() {
            continue;
        }
        let manifest = std::fs::read_to_string(candidate.join("Cargo.toml"))
            .map_err(|error| format!("cannot read workspace manifest: {error}"))?;
        if manifest.lines().any(|line| line.trim() == "[workspace]") {
            return Ok(candidate.to_owned());
        }
    }
    Err(format!(
        "no Cargo workspace containing Cargo.lock found from {}",
        start.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::resolve_workspace;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "ekr-xtask-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).expect("unique task-owned fixture directory");
            Self(path)
        }

        fn workspace(&self) -> PathBuf {
            std::fs::write(self.0.join("Cargo.lock"), "# fixture\n").unwrap();
            std::fs::write(
                self.0.join("Cargo.toml"),
                "[workspace]\nmembers = [\"xtask\"]\n",
            )
            .unwrap();
            let manifest = self.0.join("xtask");
            std::fs::create_dir(&manifest).unwrap();
            std::fs::write(manifest.join("Cargo.toml"), "[package]\nname = \"xtask\"\n").unwrap();
            manifest
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).expect("remove this test's isolated fixture");
        }
    }

    #[test]
    fn runtime_manifest_selects_its_own_workspace() {
        let fixture = Fixture::new();
        let manifest = fixture.workspace();
        assert_eq!(
            resolve_workspace(Some(&manifest), Path::new("/")).unwrap(),
            fixture.0
        );
    }

    #[test]
    fn standalone_invocation_walks_from_its_current_directory() {
        let fixture = Fixture::new();
        let nested = fixture.workspace().join("nested");
        std::fs::create_dir(&nested).unwrap();
        assert_eq!(resolve_workspace(None, &nested).unwrap(), fixture.0);
    }

    #[test]
    fn missing_runtime_manifest_and_unrelated_directory_refuse() {
        let fixture = Fixture::new();
        assert!(resolve_workspace(None, &fixture.0).is_err());
        assert!(resolve_workspace(Some(&fixture.0.join("missing")), Path::new("/")).is_err());
    }
}

#[derive(Subcommand)]
enum Command {
    /// Report the workspace root and the members the manifest declares.
    Doctor,
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Doctor => doctor(),
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask doctor: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn doctor() -> Result<(), String> {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR").map(std::path::PathBuf::from);
    let current = std::env::current_dir()
        .map_err(|error| format!("cannot read current directory: {error}"))?;
    let root = resolve_workspace(manifest.as_deref(), &current)?;
    println!("workspace root: {}", root.display());
    let manifest = std::fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("the workspace manifest is unreadable: {error}"))?;
    for line in manifest.lines() {
        let line = line.trim();
        if let Some(member) = line.strip_prefix('"').and_then(|l| l.strip_suffix("\",")) {
            println!("member: {member}");
        }
    }
    Ok(())
}
