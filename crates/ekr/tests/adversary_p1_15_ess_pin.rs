//! Adversary pass 1 on `story:pin-ess-0-32` (wave p1-15).
//!
//! `conformance.rs::spec_and_conform_checks_refuse_an_ess_that_is_not_the_pinned_release` reads
//! every `- sh:` line between a task's header and the next task and runs it under `/bin/sh`. It
//! never asks the task runner. A guard filed under a key go-task does not know (`precondition:`)
//! is ignored by `task` 3.53.1 — the task runs any `ess` — and that case stays green. These cases
//! drive `task` itself, the program `task check` and CI run, with a stand-in `ess` first on PATH.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
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

/// A stand-in `ess` in `dir/bin` that answers `--version` with `reported` and records every
/// other invocation in `dir/ran.log`, exiting zero.
fn stand_in_ess(dir: &Path, reported: &str) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("bin directory");
    let ess = bin.join("ess");
    let log = dir.join("ran.log");
    std::fs::write(
        &ess,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '{reported}'; exit 0; fi\necho \"$*\" >> '{}'\n",
            log.display()
        ),
    )
    .expect("stand-in ess");
    std::fs::set_permissions(&ess, std::fs::Permissions::from_mode(0o755)).expect("executable");
    bin
}

/// Runs `task <name>` from the workspace root with `bin` first on PATH; returns (success, ran).
fn run_task(name: &str, reported: &str) -> (bool, String, String) {
    let scratch = tempfile::TempDir::new().expect("scratch");
    let bin = stand_in_ess(scratch.path(), reported);
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("task")
        .arg(name)
        // A mutant Taskfile can be run from a copy: `EKR_ADVERSARY_TASK_DIR=<dir with Taskfile.yml>`.
        .current_dir(
            std::env::var_os("EKR_ADVERSARY_TASK_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(workspace_root),
        )
        .env("PATH", path)
        .output()
        .expect("`task` is on PATH, as CI installs it");
    let ran = std::fs::read_to_string(scratch.path().join("ran.log")).unwrap_or_default();
    (
        out.status.success(),
        ran,
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// `task spec-check` and `task conform-check` refuse an `ess` that reports another release, and
/// never invoke it past `--version`.
#[test]
fn task_refuses_an_ess_that_is_not_the_pinned_release() {
    for task in ["spec-check", "conform-check"] {
        for other in ["ess 0.29.0", "ess 0.32.1", "ess 0.32.0-rc.1", "ess 0.32.0 "] {
            let (ok, ran, stderr) = run_task(task, other);
            assert!(!ok, "`task {task}` admitted `{other}`; it ran: {ran:?}");
            assert!(
                ran.is_empty(),
                "`task {task}` invoked `{other}` past --version: {ran:?}"
            );
            assert!(
                stderr.contains("precondition not met"),
                "`task {task}` refused `{other}` for another reason: {stderr}"
            );
        }
    }
}

/// `task spec-check` admits the pinned release and runs it.
#[test]
fn task_spec_check_admits_the_pinned_release() {
    let (ok, ran, stderr) = run_task("spec-check", "ess 0.32.0");
    assert!(ok, "`task spec-check` refused ess 0.32.0: {stderr}");
    assert_eq!(ran, "specify validate --path systems/ekr\n");
}
