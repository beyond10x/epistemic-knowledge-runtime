//! Adversary pass 1 on unit D `p5-01-cli` (story:schema-evolution-transactions, part D).
//!
//! The unit's goal is that an agent holding only the binary and `docs/cli.md` can evolve a
//! schema, and the first thing such an agent does is move from the example host (profile v1) to a
//! profile v2 host. Every text it can read on the way must say v2 is accepted. These cases hold
//! two of those texts to what the binary does with a v2 host.

use std::process::Output;

use serde_json::Value;

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
        command.env_remove(var);
    }
    command
}

fn stdout(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// `ekr example ekr.cli-host/1` with validation profile v2, as `ekr guide` says to write it.
fn v2_host() -> String {
    stdout(&["example", "ekr.cli-host/1"])
        .replace("\"ekr.p1-deterministic/1\"", "\"ekr.p2-deterministic/1\"")
        .replace("\"ekr.p1-apply/1\"", "\"ekr.p2-apply/1\"")
}

/// Seeds the example seed under the v2 host on `backend`; the store verbs accept the v2 profile.
fn seed_under_v2(backend: &str) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let host = directory.path().join("host.json");
    std::fs::write(&host, v2_host()).unwrap();
    let seed = directory.path().join("seed.yaml");
    std::fs::write(&seed, stdout(&["example", "ekr-seed/2"])).unwrap();
    let store = match backend {
        "file" => directory.path().join("store"),
        _ => directory.path().join("state.db"),
    };
    ekr()
        .arg("--host")
        .arg(&host)
        .arg("--store")
        .arg(&store)
        .args(["--backend", backend])
        .arg("seed")
        .arg(&seed)
        .output()
        .unwrap()
}

fn page() -> String {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::fs::read_to_string(std::path::Path::new(&root).join("../../docs/cli.md")).unwrap()
}

/// `docs/cli.md`'s refusal table row for `seed-authority-profile` names the cause. The binary
/// accepts profile v2 (this case seeds under it), so a cause of "not the P1 profile" tells the
/// agent that has just written a v2 host, and got the code for some other slip, that v2 itself is
/// what was refused.
#[test]
fn the_seed_authority_profile_row_does_not_say_only_the_p1_profile_is_accepted() {
    for backend in ["file", "sqlite"] {
        let seeded = seed_under_v2(backend);
        assert_eq!(seeded.status.code(), Some(0), "{backend}: {seeded:?}");
    }
    let page = page();
    let row = page
        .lines()
        .find(|line| line.starts_with("| `seed-authority-profile` |"))
        .expect("docs/cli.md has a seed-authority-profile row");
    assert!(
        !row.contains("not the P1 profile"),
        "docs/cli.md: the seed-authority-profile row says the cause is a profile that is \
         \"not the P1 profile\", and the binary accepts profile v2: {row}"
    );
}

/// `ekr schema ekr.cli-host/1` is the workflow's way to check a host document before use
/// (`ekr guide`, step 5). Its `authority` description names the profile the host carries; the
/// same schema admits `ekr.p2-deterministic/1`, and the binary seeds under it.
#[test]
fn the_host_schema_does_not_describe_the_authority_as_the_p1_profile_only() {
    let schema: Value = serde_json::from_str(&stdout(&["schema", "ekr.cli-host/1"])).unwrap();
    assert!(
        schema.to_string().contains("ekr.p2-deterministic/1"),
        "precondition: the host schema admits profile v2"
    );
    let seeded = seed_under_v2("file");
    assert_eq!(seeded.status.code(), Some(0), "{seeded:?}");
    let description = schema["properties"]["authority"]["description"]
        .as_str()
        .expect("the host schema describes authority");
    assert!(
        !description.contains("the P1 validation profile"),
        "`ekr schema ekr.cli-host/1` describes authority as carrying \"the P1 validation \
         profile\", and the binary accepts profile v2: {description}"
    );
}
