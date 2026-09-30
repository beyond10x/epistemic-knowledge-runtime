//! The request and reply seam every SDK call goes through (`story:sdk-session-transport`).
//!
//! A [`Transport`] takes one [`Request`] — an argv as `ekr` takes it after its name, and the text
//! `-` reads — and returns the [`Reply`]. [`crate::session::ProcessSession`] is the transport over
//! a child `ekr session`; [`RecordingTransport`] records any transport's exchanges as a
//! [`Recording`], and [`ReplayTransport`] answers from one with no `ekr` at all.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::reply::Reply;

/// One request: `argv` as `ekr` takes it after its name, and the text a `-` argument reads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// The verb and its arguments: `["snapshot", "--at", "0"]` is `ekr snapshot --at 0`.
    pub argv: Vec<String>,
    /// The text `-` reads, for `seed -`, `propose -`, `resolve -` and `hash -`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
}

impl Request {
    /// A request of `argv`, with nothing for `-` to read.
    pub fn new<I, S>(argv: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            argv: argv.into_iter().map(Into::into).collect(),
            stdin: None,
        }
    }

    /// This request with `stdin` as the text `-` reads.
    pub fn with_stdin(mut self, stdin: impl Into<String>) -> Self {
        self.stdin = Some(stdin.into());
        self
    }

    /// The verb: the first word of `argv`, or the empty string when there is none.
    pub fn verb(&self) -> &str {
        self.argv.first().map(String::as_str).unwrap_or("")
    }
}

/// Sends one request to `ekr` and returns its reply. Blocking: a call returns when the reply is
/// read or the transport has failed.
pub trait Transport {
    /// Answer `request`. A refusal or a fault is a [`Reply`]; an `Err` means no reply was read.
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError>;
}

impl<T: Transport + ?Sized> Transport for &mut T {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        (**self).request(request)
    }
}

impl<T: Transport + ?Sized> Transport for Box<T> {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        (**self).request(request)
    }
}

/// Why a transport returned no reply. Every variant names the verb of the request it failed.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// The `ekr` process ended before it answered.
    #[error(
        "`{verb}`: the ekr process ended before answering ({status}); stderr tail: {}",
        shown(stderr_tail)
    )]
    Died {
        /// The verb of the request in flight.
        verb: String,
        /// How the process ended, as the operating system reports it.
        status: String,
        /// The last bytes the process wrote to stderr.
        stderr_tail: String,
    },
    /// No reply came within the per-request timeout; the process was stopped.
    #[error(
        "`{verb}`: no answer within {timeout:?}, so the ekr process was stopped; stderr tail: {}",
        shown(stderr_tail)
    )]
    TimedOut {
        /// The verb of the request in flight.
        verb: String,
        /// The timeout the request ran out of.
        timeout: Duration,
        /// The last bytes the process wrote to stderr.
        stderr_tail: String,
    },
    /// The session's [`crate::session::CancelHandle`] was triggered; the process was stopped.
    #[error("`{verb}`: the session was cancelled and its ekr process stopped")]
    Cancelled {
        /// The verb of the request in flight, or of the first request after the cancel.
        verb: String,
    },
    /// An earlier request failed the session, and a failed session is never restarted.
    #[error("`{verb}`: the ekr session failed earlier and is not restarted: {cause}")]
    Latched {
        /// The verb of this request.
        verb: String,
        /// The earlier failure, as it was reported.
        cause: String,
    },
    /// The process answered with something that is not a reply.
    #[error("`{verb}`: the ekr process answered with something that is not a reply ({detail}): {answer:?}; stderr tail: {}", shown(stderr_tail))]
    Protocol {
        /// The verb of the request in flight.
        verb: String,
        /// What was wrong with the answer.
        detail: String,
        /// The start of what the process printed, without its final line end: at most
        /// [`ANSWER_BYTES`], cut back to a character boundary and ended with [`ANSWER_CUT`] when
        /// it was longer. Bytes that are not UTF-8 read as U+FFFD.
        answer: String,
        /// The last bytes the process wrote to stderr.
        stderr_tail: String,
    },
    /// A closed session did not exit 0: it exited with another status, or it was still running
    /// when the close timeout ran out and was killed. Returned by
    /// [`crate::session::ProcessSession::close`] only.
    #[error(
        "`session`: the ekr session {} ({status}); stderr tail: {}",
        closed_how(*killed),
        shown(stderr_tail)
    )]
    CloseFailed {
        /// How the process ended, as the operating system reports it: its exit code, or the
        /// signal that ended it.
        status: std::process::ExitStatus,
        /// Whether the SDK killed the process because it had not exited within the timeout.
        killed: bool,
        /// The last bytes the process wrote to stderr, at most
        /// [`crate::session::STDERR_TAIL_BYTES`].
        stderr_tail: String,
    },
    /// The `ekr` process could not be started or spoken to.
    #[error("`{verb}`: {what}: {source}")]
    Io {
        /// The verb of the request, or `session` when starting the session.
        verb: String,
        /// What was being done.
        what: String,
        /// The operating system's error.
        source: std::io::Error,
    },
    /// A replay was asked a request its recording does not hold next.
    #[error("`{verb}`: replay: {detail}")]
    Replay {
        /// The verb of the request asked.
        verb: String,
        /// How the request differs from the recording.
        detail: String,
    },
}

/// How a closed session ended, as [`TransportError::CloseFailed`] says it.
fn closed_how(killed: bool) -> &'static str {
    if killed {
        "was killed when it did not exit within the close timeout"
    } else {
        "exited unsuccessfully when closed"
    }
}

/// The most bytes of an answer [`TransportError::Protocol`] keeps, [`ANSWER_CUT`] included.
pub const ANSWER_BYTES: usize = 400;

/// What ends a [`TransportError::Protocol`] answer that was cut to [`ANSWER_BYTES`].
pub const ANSWER_CUT: &str = " [cut]";

/// The start of `printed` as [`TransportError::Protocol`] keeps it.
pub(crate) fn answer_start(printed: &[u8]) -> String {
    let text = String::from_utf8_lossy(printed);
    let text = text.trim_end_matches(['\n', '\r']);
    if text.len() <= ANSWER_BYTES {
        return text.to_owned();
    }
    let mut end = ANSWER_BYTES - ANSWER_CUT.len();
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{ANSWER_CUT}", &text[..end])
}

/// A stderr tail as an error message shows it.
fn shown(tail: &str) -> String {
    match tail.trim_end() {
        "" => "(empty)".to_owned(),
        tail => format!("{tail:?}"),
    }
}

/// One request and the reply it got.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Exchange {
    /// The request as it was sent.
    pub request: Request,
    /// The reply it got.
    pub reply: Reply,
}

/// The format a [`Recording`] is written in.
pub const RECORDING_FORMAT: &str = "ekr-sdk.recording/1";

/// The exchanges of one transport, in order: what [`RecordingTransport`] writes and
/// [`ReplayTransport`] answers from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recording {
    /// Always [`RECORDING_FORMAT`].
    pub format: String,
    /// Every request that got a reply, in the order it was sent.
    pub exchanges: Vec<Exchange>,
}

/// A recording that does not read.
#[derive(Debug, thiserror::Error)]
pub enum RecordingError {
    /// The text is not a recording's JSON.
    #[error("not a recording: {0}")]
    Json(#[from] serde_json::Error),
    /// The recording is in another format.
    #[error("a recording in format {found:?}; this SDK reads {RECORDING_FORMAT}")]
    Format {
        /// The format the recording names.
        found: String,
    },
}

impl Default for Recording {
    fn default() -> Self {
        Self {
            format: RECORDING_FORMAT.to_owned(),
            exchanges: Vec::new(),
        }
    }
}

impl Recording {
    /// The recording as pretty JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a recording is JSON")
    }

    /// A recording read from [`Recording::to_json`]'s output.
    pub fn from_json(text: &str) -> Result<Self, RecordingError> {
        let recording: Self = serde_json::from_str(text)?;
        if recording.format != RECORDING_FORMAT {
            return Err(RecordingError::Format {
                found: recording.format,
            });
        }
        Ok(recording)
    }
}

/// A transport that passes every request to `inner` and records each request that got a reply.
/// A request that failed is not recorded: its error passes through.
#[derive(Debug)]
pub struct RecordingTransport<T> {
    inner: T,
    recording: Recording,
}

impl<T: Transport> RecordingTransport<T> {
    /// Record what `inner` answers.
    pub fn record(inner: T) -> Self {
        Self {
            inner,
            recording: Recording::default(),
        }
    }

    /// The exchanges recorded so far.
    pub fn recording(&self) -> &Recording {
        &self.recording
    }

    /// The recording, dropping the inner transport.
    pub fn into_recording(self) -> Recording {
        self.recording
    }
}

impl<T: Transport> Transport for RecordingTransport<T> {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        let reply = self.inner.request(request)?;
        self.recording.exchanges.push(Exchange {
            request: request.clone(),
            reply: reply.clone(),
        });
        Ok(reply)
    }
}

/// A transport that answers from a [`Recording`], in order, and starts no process. Each request
/// must equal the next recorded one, `argv` and `stdin` both; any other is refused, as is a
/// request after the last.
#[derive(Clone, Debug)]
pub struct ReplayTransport {
    exchanges: std::vec::IntoIter<Exchange>,
}

impl ReplayTransport {
    /// Replay `recording` from its first exchange.
    pub fn new(recording: Recording) -> Self {
        Self {
            exchanges: recording.exchanges.into_iter(),
        }
    }

    /// How many recorded exchanges have not been asked for yet.
    pub fn remaining(&self) -> usize {
        self.exchanges.len()
    }
}

impl Transport for ReplayTransport {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        let verb = request.verb().to_owned();
        let Some(next) = self.exchanges.as_slice().first() else {
            return Err(TransportError::Replay {
                verb,
                detail: "the recording holds no further request".to_owned(),
            });
        };
        if next.request != *request {
            let field = if next.request.argv == request.argv {
                "stdin"
            } else {
                "argv"
            };
            return Err(TransportError::Replay {
                verb,
                detail: format!(
                    "the request's {field} differs from the next recorded request, {:?}",
                    next.request.argv
                ),
            });
        }
        let exchange = self.exchanges.next().expect("checked above");
        Ok(exchange.reply)
    }
}
