//! Starting `ekr view` for a store (`story:sdk-session-transport`).
//!
//! [`Viewer::spawn`] starts `ekr view --port <port>` from the consumer's [`EkrBinary`], with the
//! environment a session gets by default, and reads the one `{"url": …}` line the viewer prints
//! before it serves (`docs/cli.md` § `ekr view`). With EKR 0.0.28 or newer, it adds
//! `--require-ready` so the URL still proves initial store admission; older binaries already
//! open the store before announcing. The viewer serves until it is stopped or dropped.

use std::io::{BufRead as _, BufReader};
use std::process::{Child, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde::Deserialize;

use crate::binary::{spawn, EkrBinary, Version};
use crate::session::{command, SessionOptions, StderrTail, StoreConfig, STDERR_TAIL_BYTES};

/// How long a viewer may take to print its URL: it opens the store first.
const URL_TIMEOUT: Duration = Duration::from_secs(120);

/// Why a viewer did not start.
#[derive(Debug, thiserror::Error)]
pub enum ViewerError {
    /// `ekr view` could not be started.
    #[error("starting ekr view: {0}")]
    Spawn(#[from] std::io::Error),
    /// `ekr view` did not print its URL line; it was stopped.
    #[error("ekr view printed no URL ({detail}); stderr tail: {stderr_tail:?}")]
    NoUrl {
        /// What came instead: the end of its output, a line that is not the URL, or nothing in time.
        detail: String,
        /// The last bytes it wrote to stderr.
        stderr_tail: String,
    },
}

/// A running `ekr view` on 127.0.0.1.
#[derive(Debug)]
pub struct Viewer {
    child: Child,
    url: String,
}

/// The line `ekr view` prints first.
#[derive(Deserialize)]
struct UrlLine {
    url: String,
}

impl Viewer {
    /// Start `ekr view --port <port>` over `store`; `0` lets the viewer pick a free port.
    /// EKR 0.0.28 and newer also receive `--require-ready`, requiring a seeded complete store
    /// before the URL is announced. Earlier versions receive their original arguments.
    pub fn spawn(binary: &EkrBinary, store: &StoreConfig, port: u16) -> Result<Self, ViewerError> {
        let mut command = command(binary, store, &SessionOptions::default());
        command
            .args(["view", "--port", &port.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if binary.version() >= Version::new(0, 0, 28) {
            command.arg("--require-ready");
        }
        let mut child = spawn(&mut command)?;
        let stderr = StderrTail::collect(
            child.stderr.take().expect("stderr is piped"),
            STDERR_TAIL_BYTES,
        );
        let stdout = child.stdout.take().expect("stdout is piped");
        let (first, line) = mpsc::channel();
        std::thread::spawn(move || {
            let mut stdout = BufReader::new(stdout);
            let mut url = String::new();
            let _ = first.send(stdout.read_line(&mut url).map(|_| url));
            // Drain whatever follows, so the viewer never blocks on a full pipe.
            let _ = std::io::copy(&mut stdout, &mut std::io::sink());
        });
        let detail = match line.recv_timeout(URL_TIMEOUT) {
            Ok(Ok(line)) => match serde_json::from_str::<UrlLine>(&line) {
                Ok(UrlLine { url }) => return Ok(Self { child, url }),
                Err(_) if line.is_empty() => "its output ended".to_owned(),
                Err(error) => format!("the line {line:?}: {error}"),
            },
            Ok(Err(error)) => format!("reading its output: {error}"),
            Err(_) => format!("nothing within {URL_TIMEOUT:?}"),
        };
        let _ = child.kill();
        let _ = child.wait();
        Err(ViewerError::NoUrl {
            detail,
            stderr_tail: stderr.settled(),
        })
    }

    /// The URL the viewer printed, `http://127.0.0.1:<port>/`.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Stop the viewer.
    pub fn stop(self) {}
}

impl Drop for Viewer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
