//! Exact generated artifact inventory and bytes, verified independently of Cargo caches.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratorPin {
    version: String,
    source_revision: String,
    binary_sha256: String,
    release: Option<ReleaseArtifact>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseArtifact {
    tag: String,
    archive: String,
    archive_sha256: String,
}

fn files(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    fn visit(
        root: &Path,
        path: &Path,
        found: &mut BTreeMap<PathBuf, Vec<u8>>,
    ) -> Result<(), String> {
        for entry in
            std::fs::read_dir(path).map_err(|e| format!("reading {}: {e}", path.display()))?
        {
            let path = entry.map_err(|e| e.to_string())?.path();
            let kind = std::fs::symlink_metadata(&path)
                .map_err(|e| e.to_string())?
                .file_type();
            // ESS's output ownership journal contains inode/path/random-anchor data, not
            // generated source. Exclude this exact operational directory only; unknown
            // entries still fail so the exclusion cannot conceal extra generated files.
            if path == root.join(".ess-output") && kind.is_dir() {
                for entry in std::fs::read_dir(&path).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    if entry.file_name() != "state.json"
                        || !entry.file_type().map_err(|e| e.to_string())?.is_file()
                    {
                        return Err(format!(
                            "unexpected ESS operational entry: {}",
                            entry.path().display()
                        ));
                    }
                }
                continue;
            }

            if kind.is_dir() {
                visit(root, &path, found)?;
            } else if kind.is_file() {
                found.insert(
                    path.strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_owned(),
                    std::fs::read(&path).map_err(|e| e.to_string())?,
                );
            } else {
                return Err(format!(
                    "generated artifact is not an ordinary file: {}",
                    path.display()
                ));
            }
        }
        Ok(())
    }
    if !std::fs::symlink_metadata(root)
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        return Err(format!(
            "generated artifact root is not an ordinary directory: {}",
            root.display()
        ));
    }
    let mut found = BTreeMap::new();

    visit(root, root, &mut found)?;
    if found.is_empty() {
        return Err(format!(
            "generated artifact tree is empty: {}",
            root.display()
        ));
    }
    Ok(found)
}

pub(crate) fn compare_trees(expected: &Path, actual: &Path) -> Result<(), String> {
    let expected = files(expected)?;
    let actual = files(actual)?;
    let mut changes = Vec::new();
    for (path, bytes) in &expected {
        match actual.get(path) {
            None => changes.push(format!("missing {}", path.display())),
            Some(other) if other != bytes => changes.push(format!("changed {}", path.display())),
            Some(_) => {}
        }
    }
    for path in actual.keys().filter(|path| !expected.contains_key(*path)) {
        changes.push(format!("extra {}", path.display()));
    }
    if changes.is_empty() {
        Ok(())
    } else {
        Err(format!("generated contract drift:\n{}", changes.join("\n")))
    }
}

fn executable(path: &Path) -> Result<PathBuf, String> {
    if path.components().count() > 1 || path.is_absolute() {
        return path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()));
    }
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .map(|directory| directory.join(path))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| format!("{} is absent from PATH", path.display()))?
        .canonicalize()
        .map_err(|e| e.to_string())
}

fn sha256(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    let mut chunk = [0u8; 65536];
    loop {
        let count = file.read(&mut chunk).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        digest.update(&chunk[..count]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn run(command: &mut Command) -> Result<(), String> {
    let output = command.output().map_err(|e| format!("{command:?}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{command:?} exited {}\n{}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

pub(crate) fn check(root: &Path, ess: &Path, development_candidate: bool) -> Result<(), String> {
    let pin: GeneratorPin = serde_json::from_slice(
        &std::fs::read(root.join("generated/ess-generator.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let is_hex = |text: &str, len: usize| {
        text.len() == len
            && text
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    };
    if !is_hex(&pin.source_revision, 40) || !is_hex(&pin.binary_sha256, 64) {
        return Err("generator pin has no exact source revision or executable SHA-256".into());
    }
    match &pin.release {
        None if !development_candidate => return Err("generator is a development candidate; adoption requires a verified published release pin".into()),
        None => eprintln!("development candidate: {}; this check does not establish release acceptance", pin.source_revision),
        Some(release) if release.tag != pin.version || !is_hex(&release.archive_sha256, 64) || release.archive.is_empty() => return Err("generator release pin is incomplete or disagrees with its version".into()),
        Some(_) => {}
    }
    let ess = executable(ess)?;
    if sha256(&ess)? != pin.binary_sha256 {
        return Err("ESS executable SHA-256 differs from the generator pin".into());
    }
    let version = Command::new(&ess)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;
    if !version.status.success()
        || String::from_utf8_lossy(&version.stdout).trim() != format!("ess {}", pin.version)
    {
        return Err(format!("generator needs exactly ess {}", pin.version));
    }
    let target = root.join("target");
    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let temporary = tempfile::Builder::new()
        .prefix("contract-regeneration-")
        .tempdir_in(&target)
        .map_err(|e| e.to_string())?;
    let semantic = temporary.path().join("ekr-contracts");
    let data = temporary.path().join("ekr-contract-data");
    run(Command::new(&ess)
        .current_dir(root)
        .args([
            "generate",
            "synthesize",
            "--path",
            "systems/ekr",
            "--target",
            "rust",
            "--layout",
            "workspace",
            "--out",
        ])
        .arg(&semantic))?;
    run(Command::new(&ess)
        .current_dir(root)
        .args([
            "generate",
            "types",
            "--path",
            "systems/ekr",
            "--target",
            "rust",
            "--all-types",
            "--package",
            "ekr-contract-data",
            "--out",
        ])
        .arg(&data))?;
    compare_trees(&semantic, &root.join("generated/ekr-contracts"))?;
    compare_trees(&data, &root.join("generated/ekr-contract-data"))?;
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    run(Command::new(cargo)
        .current_dir(root)
        .args(["check", "--offline", "--manifest-path"])
        .arg(semantic.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(target.join("generated-contract-check")))?;
    println!(
        "generated contracts: both complete artifact trees match; synthesized workspace compiles"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::compare_trees;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "ekr-contract-artifacts-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            for name in ["expected", "actual"] {
                std::fs::create_dir_all(root.join(name).join("src")).unwrap();
                std::fs::write(root.join(name).join("Cargo.toml"), b"[package]\n").unwrap();
                std::fs::write(
                    root.join(name).join("src/lib.rs"),
                    b"pub struct Contract;\n",
                )
                .unwrap();
            }
            Self(root)
        }
        fn expected(&self) -> PathBuf {
            self.0.join("expected")
        }
        fn actual(&self) -> PathBuf {
            self.0.join("actual")
        }
        fn compare(&self) -> Result<(), String> {
            compare_trees(&self.expected(), &self.actual())
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn refused(result: Result<(), String>, name: &Path, category: &str) {
        let message = result.expect_err("artifact drift must fail");
        assert!(
            message.contains(&name.to_string_lossy().to_string()),
            "{message}"
        );
        assert!(message.contains(category), "{message}");
    }
    #[test]
    fn identical_artifacts_pass() {
        assert!(Fixture::new().compare().is_ok());
    }
    #[test]
    fn changed_generated_source_fails() {
        let fixture = Fixture::new();
        std::fs::write(
            fixture.actual().join("src/lib.rs"),
            b"pub struct Different;\n",
        )
        .unwrap();
        refused(fixture.compare(), Path::new("src/lib.rs"), "changed");
    }
    #[test]
    fn missing_generated_source_fails() {
        let fixture = Fixture::new();
        std::fs::remove_file(fixture.actual().join("src/lib.rs")).unwrap();
        refused(fixture.compare(), Path::new("src/lib.rs"), "missing");
    }
    #[test]
    fn extra_generated_source_fails() {
        let fixture = Fixture::new();
        std::fs::write(
            fixture.actual().join("src/extra.rs"),
            b"pub struct Shadow;\n",
        )
        .unwrap();
        refused(fixture.compare(), Path::new("src/extra.rs"), "extra");
    }
    #[test]
    fn extra_manifest_fails() {
        let fixture = Fixture::new();
        std::fs::create_dir_all(fixture.actual().join("other")).unwrap();
        std::fs::write(fixture.actual().join("other/Cargo.toml"), b"[package]\n").unwrap();
        refused(fixture.compare(), Path::new("other/Cargo.toml"), "extra");
    }
    #[test]
    fn only_the_known_local_ownership_journal_is_excluded() {
        let fixture = Fixture::new();
        let journal = fixture.actual().join(".ess-output");
        std::fs::create_dir(&journal).unwrap();
        std::fs::write(journal.join("state.json"), b"machine-local ownership state").unwrap();
        assert!(fixture.compare().is_ok());
        std::fs::write(journal.join("hidden.rs"), b"unexpected source").unwrap();
        assert!(fixture.compare().unwrap_err().contains("hidden.rs"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_hide_artifacts_or_ownership_state() {
        let fixture = Fixture::new();
        std::os::unix::fs::symlink(
            fixture.expected().join("src/lib.rs"),
            fixture.actual().join("extra.rs"),
        )
        .unwrap();
        assert!(fixture
            .compare()
            .unwrap_err()
            .contains("not an ordinary file"));
        std::fs::remove_file(fixture.actual().join("extra.rs")).unwrap();
        let journal = fixture.actual().join(".ess-output");
        std::fs::create_dir(&journal).unwrap();
        std::os::unix::fs::symlink(
            fixture.expected().join("src/lib.rs"),
            journal.join("state.json"),
        )
        .unwrap();
        assert!(fixture
            .compare()
            .unwrap_err()
            .contains("unexpected ESS operational entry"));
    }
}
