//! Adversary, wave read-01 unit M, pass 1: `ekr mcp`'s JSON-RPC framing.
//!
//! * JSON-RPC 2.0 § 5: the response `id` "MUST be the same as the value of the id member in the
//!   Request Object". A number id is echoed from `serde_json::Value`, which holds an integer beyond
//!   the 64-bit range as an `f64`.
//! * The server answers `protocolVersion: "2025-03-26"` when a client asks for it; that revision's
//!   transport section says implementations "MUST support receiving JSON-RPC batches".

use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use serde_json::{json, Value};

fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

struct World {
    directory: tempfile::TempDir,
}

impl World {
    fn seeded() -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        let example = |kind: &str| {
            let output = ekr().args(["example", kind]).output().unwrap();
            assert_eq!(output.status.code(), Some(0));
            String::from_utf8(output.stdout).unwrap()
        };
        std::fs::write(world.path("host.json"), example("ekr.cli-host/1")).unwrap();
        std::fs::write(world.path("seed.yaml"), example("ekr-seed/2")).unwrap();
        let seeded: Output = world
            .command(&["seed", "seed.yaml"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(seeded.status.code(), Some(0));
        world
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn command(&self, verb: &[&str]) -> std::process::Command {
        let mut command = ekr();
        command
            .current_dir(self.directory.path())
            .arg("--host")
            .arg(self.path("host.json"))
            .arg("--store")
            .arg(self.path("store"))
            .args(["--backend", "file"])
            .args(verb);
        command
    }

    /// `ekr mcp` fed `lines` as written, then end of input: the lines it wrote, raw.
    fn mcp(&self, lines: &[&str]) -> Vec<String> {
        let mut child = self
            .command(&["mcp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        for line in lines {
            input.write_all(line.as_bytes()).unwrap();
            input.write_all(b"\n").unwrap();
        }
        drop(input);
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0));
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

/// An integer id beyond the 64-bit range is echoed as the client wrote it, not rounded to a
/// float: a client matching responses to requests by id finds its answer.
#[test]
fn adversary_mcp_an_integer_id_beyond_64_bits_is_echoed_unchanged() {
    let world = World::seeded();
    let answers = world.mcp(&[r#"{"jsonrpc":"2.0","id":99999999999999999999,"method":"ping"}"#]);
    assert_eq!(answers.len(), 1, "{answers:?}");
    let answered: Value = serde_json::from_str(&answers[0]).unwrap();
    assert_eq!(answered["result"], json!({}), "{}", answers[0]);
    assert!(
        answers[0].contains(r#""id":99999999999999999999"#),
        "the id came back changed: {}",
        answers[0]
    );
}

/// The server refuses batches, so it does not agree to a revision whose transport requires
/// receiving them: a client asking for `2025-03-26` (or `2024-11-05`) is answered with
/// `2025-11-25`, and a batch is still `-32600`. Changed by the correction round from the pass-1
/// case, which expected batches under `2025-03-26`, to the coordinator's decision: the server
/// stops offering the revisions that require batches.
#[test]
fn adversary_mcp_a_revision_requiring_batches_is_not_agreed_and_a_batch_is_refused() {
    let world = World::seeded();
    let answers = world.mcp(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{}}}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"[{"jsonrpc":"2.0","id":2,"method":"ping"},{"jsonrpc":"2.0","id":3,"method":"ping"}]"#,
    ]);
    assert_eq!(answers.len(), 3, "{answers:?}");
    for answer in &answers[..2] {
        let initialized: Value = serde_json::from_str(answer).unwrap();
        assert_eq!(initialized["result"]["protocolVersion"], "2025-11-25");
    }
    let batch: Value = serde_json::from_str(&answers[2]).unwrap();
    assert_eq!(batch["error"]["code"], json!(-32600), "{batch}");
    assert_eq!(batch["id"], Value::Null, "{batch}");
}
