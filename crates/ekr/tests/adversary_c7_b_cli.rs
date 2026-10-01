//! Adversarial CLI cases for wave correct-07 unit B (`task:seed-document-bounds-alias-expansion`):
//! `ekr seed` reads a document path no further than the byte cap and one byte, and a document of
//! exactly the cap is read whole and seeded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ekr_kernel::SEED_LIMITS;

fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
        command.env_remove(var);
    }
    command
}

fn stdout(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

struct Lab {
    directory: tempfile::TempDir,
    host: PathBuf,
}

impl Lab {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let host = directory.path().join("host.json");
        std::fs::write(&host, stdout(&["example", "ekr.cli-host/1"])).unwrap();
        Self { directory, host }
    }

    fn store(&self) -> PathBuf {
        self.directory.path().join("store")
    }

    fn seed(&self, document: &Path) -> Command {
        let mut command = ekr();
        command
            .arg("--host")
            .arg(&self.host)
            .arg("--store")
            .arg(self.store())
            .args(["--backend", "file", "seed"])
            .arg(document);
        command
    }
}

/// The example seed, padded with a comment to exactly `total` bytes.
fn padded(total: usize) -> Vec<u8> {
    let mut seed = stdout(&["example", "ekr-seed/2"]).into_bytes();
    seed.push(b'#');
    seed.resize(total - 1, b' ');
    seed.push(b'\n');
    assert_eq!(seed.len(), total);
    seed
}

/// A seed path that is a FIFO is read no further than the cap and one byte, like stdin: offered
/// the cap and another 16 MiB, `ekr seed` takes at most the cap, one byte and a pipe's buffer.
#[test]
fn a_seed_path_is_read_no_further_than_the_cap() {
    let lab = Lab::new();
    let cap = SEED_LIMITS.input_bytes;
    let fifo = lab.directory.path().join("seed.fifo");
    let made = Command::new("mkfifo").arg(&fifo).status().unwrap();
    assert!(made.success());
    let mut bytes = padded(cap);
    bytes.resize(cap + 16 * 1024 * 1024, b' ');
    let writer_path = fifo.clone();
    let writer = std::thread::spawn(move || {
        let Ok(mut pipe) = std::fs::OpenOptions::new().write(true).open(&writer_path) else {
            return 0;
        };
        let mut written = 0usize;
        let mut rest = bytes.as_slice();
        while !rest.is_empty() {
            match pipe.write(&rest[..rest.len().min(64 * 1024)]) {
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
    let output = lab
        .seed(&fifo)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    // Unblocks a writer still waiting to open the FIFO if `ekr seed` never opened it: a
    // non-blocking read end, opened and dropped (O_NONBLOCK is 0o4000 on Linux).
    {
        use std::os::unix::fs::OpenOptionsExt;
        let _ = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(0o4000)
            .open(&fifo);
    }
    let written = writer.join().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.starts_with("ekr: ekr.kernel.InvalidSeed: seed-too-large: "),
        "{stderr}"
    );
    assert!(!lab.store().exists(), "a refused seed created a store");
    let slack = 1024 * 1024 + 64 * 1024;
    assert!(
        written <= cap + 1 + slack,
        "ekr seed took {written} bytes from its document path; the cap is {cap}"
    );
}

/// A document of exactly the cap, from stdin, is read whole and seeds: the cap is inclusive at the
/// reader as well as in the kernel.
#[test]
fn a_seed_of_exactly_the_cap_from_stdin_seeds() {
    let lab = Lab::new();
    let cap = SEED_LIMITS.input_bytes;
    let bytes = padded(cap);
    let mut child = lab
        .seed(Path::new("-"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&bytes);
    });
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(lab.store().exists());
}
