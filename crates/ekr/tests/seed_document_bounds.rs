//! `ekr seed` holds its document to the kernel's seed limits before it is decoded: a byte cap it
//! reads no further than, a nesting depth the loader stops at, and an alias expansion limit. Each
//! refusal is named, exits 2 and creates no store.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use ekr_kernel::{SeedLimits, SEED_LIMITS};

const BACKENDS: [&str; 2] = ["file", "sqlite"];

/// The limits this suite drives `ekr seed` past.
const LIMITS: SeedLimits = SEED_LIMITS;

fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
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

/// A directory holding the example host and a store path nothing has created.
struct Lab {
    directory: tempfile::TempDir,
    host: PathBuf,
}

impl Lab {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let host = directory.path().join("host.json");
        std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
        Self { directory, host }
    }

    fn store(&self, backend: &str) -> PathBuf {
        self.directory.path().join(match backend {
            "file" => "store",
            _ => "state.db",
        })
    }

    fn seed(&self, backend: &str, document: &Path) -> Command {
        let mut command = ekr();
        command
            .arg("--host")
            .arg(&self.host)
            .arg("--store")
            .arg(self.store(backend))
            .args(["--backend", backend, "seed"])
            .arg(document);
        command
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.directory.path().join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }
}

/// Asserts `output` is the named seed refusal `code`, exit 2, and that no store was created.
fn refused(lab: &Lab, backend: &str, output: &Output, code: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{backend}: {stderr}");
    assert!(
        stderr.starts_with(&format!("ekr: ekr.kernel.InvalidSeed: {code}: ")),
        "{backend}: {stderr}"
    );
    assert!(
        !lab.store(backend).exists(),
        "{backend}: a refused seed created a store"
    );
}

/// Asserts: a seed whose aliases expand past the limit is refused as `seed-alias-expansion` on both
/// providers, before any store is created.
#[test]
fn a_seed_alias_bomb_is_refused_by_name() {
    let lab = Lab::new();
    let mut bomb = text(&["example", "ekr-seed/2"]);
    bomb.push_str("laughs:\n  l0: &l0 [ha, ha]\n");
    for level in 1..=40 {
        let previous = level - 1;
        bomb.push_str(&format!(
            "  l{level}: &l{level} [*l{previous}, *l{previous}]\n"
        ));
    }
    let document = lab.write("bomb.yaml", &bomb);
    for backend in BACKENDS {
        let output = lab.seed(backend, &document).output().unwrap();
        refused(&lab, backend, &output, "seed-alias-expansion");
    }
}

/// Asserts: a seed nested past the depth is refused as `seed-too-deep` on both providers.
#[test]
fn a_seed_nested_past_the_depth_is_refused_by_name() {
    let lab = Lab::new();
    let open = LIMITS.depth;
    let deep = format!(
        "{}deep: {}{}\n",
        text(&["example", "ekr-seed/2"]),
        "[".repeat(open),
        "]".repeat(open)
    );
    let document = lab.write("deep.yaml", &deep);
    for backend in BACKENDS {
        let output = lab.seed(backend, &document).output().unwrap();
        refused(&lab, backend, &output, "seed-too-deep");
    }
}

/// Asserts: a seed over the byte cap read from stdin is refused as `seed-too-large`, and `ekr seed`
/// reads no more than the cap and one byte of it: offered the example seed padded to the cap and
/// another 16 MiB, it accepts at most the cap, one byte and what a pipe buffers before it exits.
#[test]
fn a_seed_over_the_size_cap_is_refused_without_reading_past_it() {
    let lab = Lab::new();
    let cap = LIMITS.input_bytes;
    let extra = 16 * 1024 * 1024;
    let mut seed = text(&["example", "ekr-seed/2"]).into_bytes();
    seed.push(b'#');
    seed.resize(cap + extra, b' ');
    let mut child = lab
        .seed("file", Path::new("-"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        // Counts the bytes the pipe took until `ekr seed` closed it.
        let mut written = 0usize;
        let mut rest = seed.as_slice();
        while !rest.is_empty() {
            match stdin.write(&rest[..rest.len().min(64 * 1024)]) {
                Ok(0) => break,
                Ok(count) => {
                    written += count;
                    rest = &rest[count..];
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        written
    });
    let output = child.wait_with_output().unwrap();
    let written = writer.join().unwrap();
    refused(&lab, "file", &output, "seed-too-large");
    // A Linux pipe buffers at most 1 MiB unless resized, and nothing here resizes it.
    let slack = 1024 * 1024 + 64 * 1024;
    assert!(
        written <= cap + 1 + slack,
        "ekr seed accepted {written} bytes of stdin; the cap is {cap}"
    );
}
