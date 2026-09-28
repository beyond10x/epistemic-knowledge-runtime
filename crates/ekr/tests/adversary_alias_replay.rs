//! Adversary, `task:proposed-node-carries-reference-aliases`, pass 1.
//!
//! A node committed with aliases — empty, repeated, and a scalar YAML would read as a number —
//! reads back identically from the replay checkpoint and from a full replay from the seed, under
//! both `/1` and `/2`, on both providers, and both reads agree on the head's root hashes.

use std::path::PathBuf;

use serde_json::Value;

const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";

fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn minted(kind: &str) -> String {
    let minted: Value = serde_json::from_str(&text(&["mint", kind])).unwrap();
    minted["id"].as_str().unwrap().to_owned()
}

#[test]
fn aliases_survive_a_full_replay_and_the_checkpoint_on_both_formats_and_providers() {
    let host: Value = serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap();
    let operator = host["context"]["operator"].as_str().unwrap().to_owned();
    for backend in ["file", "sqlite"] {
        for format in ["ekr.transaction-document/1", "ekr.transaction-document/2"] {
            let directory = tempfile::tempdir().unwrap();
            let store: PathBuf = match backend {
                "file" => directory.path().join("store"),
                _ => directory.path().join("state.db"),
            };
            let write = |name: &str, contents: &str| {
                let path = directory.path().join(name);
                std::fs::write(&path, contents).unwrap();
                path.display().to_string()
            };
            write("host.json", &text(&["example", "ekr.cli-host/1"]));
            let run = |verb: &[&str], full: bool| -> Value {
                let output = ekr()
                    .arg("--host")
                    .arg(directory.path().join("host.json"))
                    .arg("--store")
                    .arg(&store)
                    .args(["--backend", backend])
                    .env("EKR_FULL_REPLAY", if full { "1" } else { "0" })
                    .args(verb)
                    .output()
                    .unwrap();
                assert_eq!(
                    output.status.code(),
                    Some(0),
                    "{backend} {format} {verb:?} full={full}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                serde_json::from_slice(&output.stdout).unwrap()
            };
            let seed = write("seed.yaml", &text(&["example", "ekr-seed/2"]));
            run(&["seed", &seed], false);
            let node = minted("node");
            let document = format!(
                "format: {format}\ntransaction:\n  id: {}\n  proposer: {operator}\n  \
                 operations:\n  - !CreateNode\n    id: {node}\n    root_id: {ROOT}\n    \
                 type_id: {ORGANIZATION}\n    canonical_name: Globex\n    properties: {{}}\n    \
                 aliases: ['', b, a, b, 42]\n  evidence: []\n",
                minted("transaction")
            );
            let path = write("create.yaml", &document);
            let id = run(&["propose", &path], false)["transaction_id"]
                .as_str()
                .unwrap()
                .to_owned();
            assert_eq!(run(&["validate", &id], false)["kind"], "Validated");
            assert_eq!(run(&["commit", &id], false)["kind"], "Committed");
            let expected = serde_json::json!(["", "b", "a", "b", "42"]);
            let checkpoint = run(&["snapshot"], false);
            let full = run(&["snapshot"], true);
            assert_eq!(
                checkpoint["graph"]["graph"]["nodes"][&node]["aliases"], expected,
                "{backend} {format}"
            );
            assert_eq!(checkpoint, full, "{backend} {format}");
            assert_eq!(
                run(&["head"], false),
                run(&["head"], true),
                "{backend} {format}"
            );
        }
    }
}
