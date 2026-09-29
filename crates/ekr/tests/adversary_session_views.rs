//! Adversary pass on `story:sdk-read-helpers`: the session's `ekr.views` verbs against the
//! `ekr view` routes they claim to equal, and against what `ekr session --help` tells a caller.
//!
//! Each case drives the built `ekr` binary: a seeded file store, one `ekr session` fed request
//! lines, and one `ekr view --port 0` over the same store.

#![cfg(unix)]

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde_json::{json, Value};

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// A seeded file store under the example host.
struct Seeded {
    directory: tempfile::TempDir,
}

impl Seeded {
    fn new() -> Self {
        let seeded = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        let path = seeded.directory.path();
        std::fs::write(path.join("host.json"), text(&["example", "ekr.cli-host/1"])).unwrap();
        std::fs::write(path.join("seed.yaml"), text(&["example", "ekr-seed/2"])).unwrap();
        let output = seeded.command(&["seed", "seed.yaml"]).output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "seed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        seeded
    }

    fn store(&self) -> PathBuf {
        self.directory.path().join("store")
    }

    fn command(&self, verb: &[&str]) -> std::process::Command {
        let path: &Path = self.directory.path();
        let mut command = ekr();
        command
            .current_dir(path)
            .arg("--host")
            .arg(path.join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", "file"])
            .args(verb);
        command
    }

    /// One `ekr session`, fed `argvs` one request line each: its answers.
    fn session(&self, argvs: &[&[&str]]) -> Vec<Value> {
        let mut child = self
            .command(&["session"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        for argv in argvs {
            writeln!(stdin, "{}", json!({ "argv": argv })).unwrap();
        }
        drop(stdin);
        let output = child.wait_with_output().unwrap();
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

/// A running `ekr view --port 0`, killed when dropped.
struct View {
    child: std::process::Child,
    address: String,
}

impl View {
    fn start(seeded: &Seeded) -> Self {
        let mut child = seeded
            .command(&["view", "--port", "0"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut line = String::new();
        std::io::BufRead::read_line(
            &mut std::io::BufReader::new(child.stdout.take().unwrap()),
            &mut line,
        )
        .unwrap();
        let printed: Value = serde_json::from_str(&line).unwrap();
        let address = printed["url"]
            .as_str()
            .unwrap()
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned();
        Self { child, address }
    }

    /// `GET <target>`: the status and the (unchunked) body.
    fn get(&self, target: &str) -> (u16, String) {
        let mut stream = std::net::TcpStream::connect(&self.address).unwrap();
        write!(
            stream,
            "GET {target} HTTP/1.0\r\nHost: {}\r\n\r\n",
            self.address
        )
        .unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        let raw = String::from_utf8_lossy(&raw).into_owned();
        let (head, body) = raw.split_once("\r\n\r\n").unwrap();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, body.to_owned())
    }
}

impl Drop for View {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

/// `docs/cli.md` § session views: "A query `ekr view` answers `invalid-query` […] is an argv
/// these verbs do not take: clap's usage message, `"exit": 2`." `ekr view` reads a bound as
/// "an optional `-` and one or more ASCII digits" and a revision as ASCII digits only
/// (`crates/ekr/src/cli/view.rs` `Query::integer`, `revision`), so `limit=+5` and `revision=+0`
/// are 400 `invalid-query`. The session's clap `i64`/`u64` parsers take a leading `+`, and the
/// session answers a document for the query the route refuses.
#[test]
#[ignore = "defect: session views verbs accept a `+`-signed bound or revision that the `ekr view` route refuses as invalid-query"]
fn a_signed_bound_the_view_route_refuses_is_not_answered_by_the_session() {
    let seeded = Seeded::new();
    let view = View::start(&seeded);
    let cases: [(&[&str], &str); 3] = [
        (&["overview", "--limit", "+5"], "/overview?limit=%2B5"),
        (&["overview", "--revision", "+0"], "/overview?revision=%2B0"),
        (
            &["search", "--limit", "+2", "--", "A"],
            "/search?q=A&limit=%2B2",
        ),
    ];
    let argvs: Vec<&[&str]> = cases.iter().map(|(argv, _)| *argv).collect();
    let answers = seeded.session(&argvs);
    for ((argv, target), answer) in cases.iter().zip(&answers) {
        let (status, body) = view.get(target);
        assert_eq!(status, 400, "GET {target}: {body}");
        assert!(body.contains("invalid-query"), "GET {target}: {body}");
        assert_eq!(
            answer["exit"],
            2,
            "{argv:?}: `ekr view` answers {target} 400 invalid-query, the session answered {}",
            answer.to_string().chars().take(160).collect::<String>()
        );
    }
}

/// `ekr session --help` is what a caller reads of the session's surface on the binary. It lists
/// the verbs the session refuses by name, but none of the six `ekr.views` verbs it now serves,
/// and says a request's verb is taken "as `ekr` takes them" — while `ekr` has no verb of those
/// names (`docs/cli.md` § session views says so itself).
#[test]
#[ignore = "defect: `ekr session --help` does not name the six ekr.views verbs the session serves"]
fn session_help_names_the_views_verbs_the_session_serves() {
    let help = text(&["session", "--help"]);
    let missing: Vec<&str> = [
        "overview", "search", "describe", "expand", "timeline", "changes",
    ]
    .into_iter()
    .filter(|verb| !help.contains(&format!("`{verb}`")))
    .collect();
    assert!(
        missing.is_empty(),
        "`ekr session --help` does not name {missing:?}"
    );
}
