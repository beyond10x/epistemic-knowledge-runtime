//! The transport of [`super::OneShotReader`]: each request its own `ekr <argv>` process.

use std::io::{Read as _, Write as _};
use std::process::Stdio;
use std::time::{Duration, Instant};

use crate::binary::EkrBinary;
use crate::reply::Reply;
use crate::session::{command, SessionOptions, StderrTail, StoreConfig, STDERR_TAIL_BYTES};
use crate::transport::{Request, Transport, TransportError};

/// How often a waiting read looks at its process.
const TICK: Duration = Duration::from_millis(10);

/// Runs each request as `binary --host … --store … --backend … <argv>`, started as a session's
/// processes are, and returns what it exits with and prints.
#[derive(Debug)]
pub(super) struct OneShot {
    pub(super) binary: EkrBinary,
    pub(super) store: StoreConfig,
    pub(super) options: SessionOptions,
}

/// The last [`STDERR_TAIL_BYTES`] of `text`, cut at a character boundary.
fn tail(text: &str) -> String {
    let mut start = text.len().saturating_sub(STDERR_TAIL_BYTES);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].to_owned()
}

impl Transport for OneShot {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        let verb = request.verb().to_owned();
        let io = |what: &str, source| TransportError::Io {
            verb: verb.clone(),
            what: what.to_owned(),
            source,
        };
        let mut child = command(&self.binary, &self.store, &self.options)
            .args(&request.argv)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| io("starting a one-shot ekr", source))?;
        let mut stdin = child.stdin.take().expect("stdin is piped");
        let text = request.stdin.clone().unwrap_or_default();
        let writer = std::thread::spawn(move || stdin.write_all(text.as_bytes()));
        let mut stdout = child.stdout.take().expect("stdout is piped");
        let reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).map(|_| bytes)
        });
        let stderr = StderrTail::collect(child.stderr.take().expect("stderr is piped"), usize::MAX);
        let deadline = Instant::now() + self.options.timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => std::thread::sleep(TICK),
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(TransportError::TimedOut {
                        verb,
                        timeout: self.options.timeout,
                        stderr_tail: tail(&stderr.settled()),
                    });
                }
                Err(source) => {
                    let _ = child.kill();
                    return Err(io("waiting for a one-shot ekr", source));
                }
            }
        };
        let _ = writer.join();
        let stdout = reader
            .join()
            .expect("the stdout reader does not panic")
            .map_err(|source| io("reading a one-shot ekr", source))?;
        let stderr = stderr.settled();
        let Some(exit) = status.code() else {
            return Err(TransportError::Died {
                verb,
                status: status.to_string(),
                stderr_tail: tail(&stderr),
            });
        };
        let document = if stdout.iter().all(u8::is_ascii_whitespace) {
            None
        } else {
            Some(
                serde_json::from_slice(&stdout).map_err(|error| TransportError::Protocol {
                    verb,
                    detail: format!("a one-shot ekr printed something that is not JSON: {error}"),
                    stderr_tail: tail(&stderr),
                })?,
            )
        };
        Ok(Reply {
            exit,
            document,
            stderr,
        })
    }
}
