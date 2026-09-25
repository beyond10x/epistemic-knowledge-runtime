//! Adversary pass 2 on `story:pin-ess-0-32` (wave p1-15).
//!
//! The version guard is a go-task `preconditions:` entry. `task` 3.53.1 does not evaluate
//! preconditions under `--force` (`-f`), a documented flag of the runner the story names, so
//! `task --force spec-check` runs any `ess` it finds. Nothing in this repository passes `--force`
//! today (`task check` in `.github/workflows/correctness.yml` does not); the case records what the
//! guard does not cover.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

/// The workspace root: this crate is `<root>/crates/ekr`.
fn workspace_root() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the manifest directory"),
    )
    .ancestors()
    .nth(2)
    .expect("crates/ekr sits two levels below the workspace root")
    .to_path_buf()
}

/// Today's state, held on purpose (coordinator decision, wave p1-15, `story:pin-ess-0-32`):
/// `task --force spec-check` skips the guard and runs a stand-in `ess 0.29.0`. That is accepted
/// only while no caller passes `--force`, which the second case holds. When the guard moves where
/// `--force` cannot skip it, invert this assertion.
#[test]
fn task_force_skips_the_ess_guard_which_is_accepted_only_while_no_caller_forces() {
    let scratch = tempfile::TempDir::new().expect("scratch");
    let bin = scratch.path().join("bin");
    std::fs::create_dir_all(&bin).expect("bin directory");
    let log = scratch.path().join("ran.log");
    let ess = bin.join("ess");
    std::fs::write(
        &ess,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'ess 0.29.0'; exit 0; fi\necho \"$*\" >> '{}'\n",
            log.display()
        ),
    )
    .expect("stand-in ess");
    std::fs::set_permissions(&ess, std::fs::Permissions::from_mode(0o755)).expect("executable");
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("task")
        .args(["--force", "spec-check"])
        .current_dir(workspace_root())
        .env("PATH", path)
        .output()
        .expect("`task` is on PATH, as CI installs it");
    let ran = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(
        out.status.success() && !ran.is_empty(),
        "`task --force spec-check` no longer runs ess 0.29.0 (exit {:?}): the guard now holds under \
         --force; invert this case (story:pin-ess-0-32)",
        out.status.code()
    );
}

/// No task runner invocation in the repository passes `--force` or `-f`, so the precondition guard
/// is evaluated on every path that runs `spec-check` or `conform-check`.
#[test]
fn no_caller_runs_task_with_force() {
    let root = workspace_root();
    let mut offenders = Vec::new();
    let mut files = vec![root.join("Taskfile.yml")];
    if let Ok(entries) = std::fs::read_dir(root.join(".github/workflows")) {
        files.extend(entries.filter_map(Result::ok).map(|e| e.path()));
    }
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        for line in text.lines() {
            let forced = line.contains("task ")
                && line
                    .split_whitespace()
                    .any(|word| word == "--force" || word == "-f");
            if forced {
                offenders.push(format!("{}: {}", file.display(), line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a caller runs task with --force, which skips the ess guard (story:pin-ess-0-32): {offenders:?}"
    );
}
