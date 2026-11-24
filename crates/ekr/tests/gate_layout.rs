//! `story:ekr-review-ci-budget`: the layout the gate's test step relies on (AGENTS.md, "The gate").
//!
//! - Every test file of a crate is a module of the crate's one integration-test binary
//!   (`tests/main.rs`); with `autotests = false` a file left out of it would never run.
//! - `scripts/classify-changes.sh` holds the one path list that lets a planning or prose pull
//!   request pass without building: both sides of it, and every document a test or the product
//!   reads is code to it.
//! - The runner settings: no debug info in test builds, a test over 60 s killed, the step run
//!   through `scripts/test-step.sh`, and the workflow that runs the classifier.
//! - No `[patch]` in the root manifest, and the gate step that builds `ekr-sdk` from a workspace
//!   of its own (`story:sdk-standalone-build`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .ancestors()
    .nth(2)
    .expect("crates/ekr sits two levels below the workspace root")
    .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The stems of the `.rs` files directly under `directory`, sorted.
fn rust_stems(directory: &Path) -> BTreeSet<String> {
    std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .map(|path| path.file_stem().unwrap().to_str().unwrap().to_owned())
        .collect()
}

/// The names of the `[[test]]` targets a manifest declares, in its own text.
fn declared_tests(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_test = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_test = line == "[[test]]";
            continue;
        }
        if in_test {
            if let Some(name) = line.strip_prefix("name = ") {
                names.push(name.trim_matches('"').to_owned());
            }
        }
    }
    names
}

#[test]
fn every_test_file_runs_in_its_crates_one_test_binary() {
    let root = workspace_root();
    let mut crates = 0;
    let mut modules = 0;
    for entry in std::fs::read_dir(root.join("crates")).unwrap() {
        let crate_dir = entry.unwrap().path();
        let tests = crate_dir.join("tests");
        if !tests.is_dir() || rust_stems(&tests).is_empty() {
            continue;
        }
        crates += 1;
        let name = crate_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let manifest = read(&crate_dir.join("Cargo.toml"));
        assert!(
            manifest
                .lines()
                .any(|line| line.trim() == "autotests = false"),
            "{name}: Cargo.toml sets autotests = false, so no test file is a binary of its own"
        );
        assert!(
            manifest.contains("[[test]]\nname = \"main\"\npath = \"tests/main.rs\""),
            "{name}: Cargo.toml declares the one test binary `main` at tests/main.rs"
        );
        let other_targets: BTreeSet<String> = declared_tests(&manifest)
            .into_iter()
            .filter(|target| target != "main")
            .collect();
        let main = read(&tests.join("main.rs"));
        let listed: BTreeSet<String> = main
            .lines()
            .filter_map(|line| line.strip_prefix("mod "))
            .map(|rest| rest.trim_end_matches(';').to_owned())
            .collect();
        for stem in rust_stems(&tests) {
            if stem == "main" || other_targets.contains(&stem) {
                continue;
            }
            assert!(
                main.contains(&format!("#[path = \"{stem}.rs\"]\nmod {stem};\n")),
                "{name}: tests/{stem}.rs is not a module of tests/main.rs and would never run; \
                 add `#[path = \"{stem}.rs\"]` and `mod {stem};` there"
            );
            modules += 1;
        }
        for module in &listed {
            assert!(
                tests.join(format!("{module}.rs")).is_file(),
                "{name}: tests/main.rs names a module tests/{module}.rs that does not exist"
            );
        }
    }
    assert!(crates >= 10, "the crates with tests were found: {crates}");
    assert!(modules >= 380, "the test files were found: {modules}");
}

/// The classifier's answer for `paths`, one per line on its standard input.
fn classify(paths: &[&str]) -> String {
    let mut child = Command::new("sh")
        .arg(workspace_root().join("scripts/classify-changes.sh"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("sh runs scripts/classify-changes.sh");
    {
        use std::io::Write as _;
        let mut stdin = child.stdin.take().unwrap();
        for path in paths {
            writeln!(stdin, "{path}").unwrap();
        }
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "the classifier failed on {paths:?}"
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[test]
fn the_change_classifier_holds_both_sides_of_its_path_list() {
    for (paths, class) in [
        (vec![".engineering/story/a.md"], "planning-only"),
        (
            vec![
                ".engineering/reviews/a.md",
                "CHANGELOG.md",
                "docs/guide.md",
                "docs/handoff/b.md",
            ],
            "planning-only",
        ),
        (
            vec!["website/index.html", "docs/guide.md"],
            "docs-site-only",
        ),
        (vec!["docs/guide.md", "crates/ekr/src/main.rs"], "code"),
        (vec![".engineering/a.md", "Taskfile.yml"], "code"),
        (vec!["AGENTS.md"], "code"),
        (vec!["README.md", "docs/guide.md"], "code"),
        (vec!["docs/cli.md"], "code"),
        (
            vec!["crates/ekr-store/tests/fixtures/legacy/README.md"],
            "code",
        ),
        (vec![".github/workflows/correctness.yml"], "code"),
        (vec!["scripts/classify-changes.sh"], "code"),
        (vec![], "code"),
    ] {
        assert_eq!(classify(&paths), class, "{paths:?}");
    }
}

/// Every `.md` file in the root and under `docs/`, relative to the root.
fn documents(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![root.join("docs")];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                found.push(path);
            }
        }
    }
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() && path.extension().is_some_and(|extension| extension == "md") {
            found.push(path);
        }
    }
    let mut relative: Vec<String> = found
        .iter()
        .map(|path| {
            path.strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    relative.sort();
    relative
}

/// Every Rust source under `directory`, recursively.
fn sources(directory: &Path, into: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

#[test]
fn every_document_a_test_or_the_product_reads_is_code_to_the_classifier() {
    let root = workspace_root();
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(root.join("crates")).unwrap() {
        sources(&entry.unwrap().path(), &mut paths);
    }
    sources(&root.join("xtask/src"), &mut paths);
    // This file names documents only as classifier inputs.
    paths.retain(|path| !path.ends_with("crates/ekr/tests/gate_layout.rs"));
    let text: Vec<String> = paths.iter().map(|path| read(path)).collect();
    let mut read_somewhere = 0;
    for document in documents(&root) {
        let quoted = format!("\"{document}\"");
        let joined = format!("/{document}\"");
        let read = text
            .iter()
            .any(|source| source.contains(&quoted) || source.contains(&joined));
        if read {
            read_somewhere += 1;
            assert_eq!(
                classify(&[&document]),
                "code",
                "{document} is read by a test or the product, so a change to it must run the \
                 tests: list it as code in scripts/classify-changes.sh"
            );
        }
    }
    assert!(
        read_somewhere >= 5,
        "the documents the tests read were found: {read_somewhere}"
    );
}

#[test]
fn the_correctness_job_classifies_first_and_keeps_its_required_name() {
    let workflow = read(&workspace_root().join(".github/workflows/correctness.yml"));
    assert!(workflow.contains("name: Repository correctness"));
    assert!(workflow.contains("sh scripts/classify-changes.sh --diff \"origin/$BASE_REF\""));
    for (step, condition) in [
        (
            "Run all repository checks",
            "steps.classify.outputs.class == 'code'",
        ),
        (
            "Check the documentation site",
            "steps.classify.outputs.class == 'docs-site-only'",
        ),
        (
            "Validate the planning store",
            "steps.classify.outputs.class != 'code'",
        ),
    ] {
        let at = workflow
            .find(&format!("- name: {step}"))
            .unwrap_or_else(|| panic!("correctness.yml has the step {step:?}"));
        let next = workflow[at..]
            .lines()
            .nth(1)
            .expect("the step has a line after its name");
        assert_eq!(next.trim(), format!("if: {condition}"), "{step}");
    }
}

#[test]
fn test_builds_carry_no_debug_info_and_the_runner_holds_the_budgets() {
    let root = workspace_root();
    let config = read(&root.join(".cargo/config.toml"));
    assert!(
        config.contains("[profile.dev]\ndebug = false\n"),
        ".cargo/config.toml turns debug info off for dev and test builds; a manifest setting \
         loses to the host's ~/.cargo/config.toml"
    );
    let manifest = read(&root.join("Cargo.toml"));
    assert!(
        manifest.contains("[profile.test]\nopt-level = 2\n"),
        "test builds compile the workspace at opt-level 2: the checkpoint, conformance and view \
         cases are bound by the CPU, and ran past 60 s at opt-level 0"
    );
    let nextest = read(&root.join(".config/nextest.toml"));
    assert!(
        nextest.contains("slow-timeout = { period = \"60s\", terminate-after = 1 }"),
        "nextest kills a test over 60 s"
    );
    assert!(
        nextest.contains("[profile.default.junit]"),
        "nextest writes the report the step reads"
    );
    let taskfile = read(&root.join("Taskfile.yml"));
    assert!(
        taskfile.contains("  test:\n") && taskfile.contains("      - sh scripts/test-step.sh\n"),
        "task test runs scripts/test-step.sh"
    );
    let step = read(&root.join("scripts/test-step.sh"));
    assert!(
        step.contains("budget=120\n"),
        "the step names the 120 s budget"
    );
    assert!(step.contains("cargo nextest run --workspace --locked --no-fail-fast"));
    assert!(step.contains("cargo test --workspace --locked --doc"));
}

/// `story:ekr-test-step-budget`: a trybuild case compiles crates inside the test (23-38 s each on
/// CI runs 45260269611 and 45297797215), so `.config/nextest.toml`'s `default-filter` leaves every
/// one out of the test step and `task compile-fail-check`, part of `task check`, runs them. A new
/// trybuild case that the filter does not name fails here.
#[test]
fn every_compile_fail_case_runs_outside_the_test_step() {
    let root = workspace_root();
    let nextest = read(&root.join(".config/nextest.toml"));
    let filter = nextest
        .split("default-filter = ")
        .nth(1)
        .expect(".config/nextest.toml has a default filter");
    // put together here, so that this file's own text does not count as a case
    let needle = format!("{}::TestCases::new", "trybuild");
    let mut cases = Vec::new();
    for entry in std::fs::read_dir(root.join("crates")).unwrap() {
        let crate_dir = entry.unwrap().path();
        let tests = crate_dir.join("tests");
        if !tests.is_dir() {
            continue;
        }
        let name = crate_dir.file_name().unwrap().to_str().unwrap().to_owned();
        for stem in rust_stems(&tests) {
            let source = read(&tests.join(format!("{stem}.rs")));
            for (at, _) in source.match_indices(&needle) {
                let before = &source[..at];
                let case = before
                    .rfind("\nfn ")
                    .and_then(|start| before[start + 4..].split('(').next())
                    .unwrap_or_else(|| panic!("{stem}.rs: a trybuild case outside a function"));
                cases.push(format!("(binary_id({name}::main) & test(={stem}::{case}))"));
            }
        }
    }
    assert!(cases.len() >= 4, "the trybuild cases were found: {cases:?}");
    for case in &cases {
        assert!(
            filter.contains(case.as_str()),
            ".config/nextest.toml's default-filter leaves out {case}"
        );
    }
    let taskfile = read(&root.join("Taskfile.yml"));
    assert!(
        taskfile.contains("      - task: compile-fail-check\n"),
        "task check runs task compile-fail-check"
    );
    assert!(
        taskfile.contains(
            "      - cargo nextest run --workspace --locked --ignore-default-filter -E 'not default()'\n"
        ),
        "task compile-fail-check runs what the default filter leaves out"
    );
}

/// `story:sdk-standalone-build`: Cargo applies only the root workspace's `[patch]`, so a crate
/// this workspace selects through one is missing for an application that depends on `ekr-sdk`
/// by release tag. The gate builds the SDK from a workspace of its own, outside the test step.
#[test]
fn the_sdk_needs_no_workspace_patch_and_the_gate_builds_it_standalone() {
    let root = workspace_root();
    let manifest = read(&root.join("Cargo.toml"));
    assert!(
        !manifest
            .lines()
            .any(|line| line.trim_start().starts_with("[patch")),
        "the root manifest selects no crate through [patch]: an application building ekr-sdk \
         does not get it"
    );
    assert!(
        manifest.contains(
            "serde_yaml_ng = { package = \"ekr-serde-yaml-ng\", version = \"0.10.0\", path = \
             \"vendor/serde_yaml_ng\" }\n"
        ),
        "the YAML crate with the observation facade is a path dependency, which travels with a tag"
    );
    let taskfile = read(&root.join("Taskfile.yml"));
    assert!(
        taskfile.contains("      - task: sdk-standalone-check\n")
            && taskfile.contains("      - sh scripts/sdk-standalone-check.sh\n"),
        "task check runs scripts/sdk-standalone-check.sh"
    );
    let script = read(&root.join("scripts/sdk-standalone-check.sh"));
    assert!(
        script.contains("cargo check --offline --manifest-path \"$consumer/Cargo.toml\"")
            && script.contains("'[workspace]'"),
        "the check builds a consumer workspace of its own, offline"
    );
}
