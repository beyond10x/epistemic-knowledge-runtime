//! One child `ekr session` over stdio (`story:sdk-session-transport`).
//!
//! [`ProcessSession`] starts `ekr session --create` from the [`EkrBinary`] the consumer chose, with
//! a cleared environment, and sends each request as one JSON line, reading one reply line back
//! (`docs/cli.md` § `ekr session`). A request whose line would exceed [`LINE_CAP`] reads its `-`
//! text from a private temporary file inside the session, or runs as its own one-shot `ekr`
//! process when its argv reads no `-`. Any failure — the child ending, a timeout, a line that is
//! not a reply, a cancel — stops the child and latches the session: that call and every later one
//! fail, and the session is never restarted.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{BufRead as _, BufReader, Read, Write as _};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::binary::{spawn, spawn_within, EkrBinary, SpawnRefused};
use crate::reply::Reply;
use crate::transport::{Request, Transport, TransportError};

/// The longest request line `ekr session` serves, newline excluded: three times the 8388608-byte
/// `ekr.transaction-document/2` cap plus 65536 bytes (`session-request-too-large` above it).
pub const LINE_CAP: usize = 25_231_360;

/// How many bytes of the child's stderr a failure reports.
pub const STDERR_TAIL_BYTES: usize = 4096;

/// How often a waiting call looks at its deadline and at the cancel flag.
const TICK: Duration = Duration::from_millis(20);

/// The storage provider, `--backend`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Backend {
    /// `file`: a directory.
    File,
    /// `sqlite`: a database file.
    Sqlite,
}

impl Backend {
    /// The name `--backend` and `EKR_BACKEND` take.
    pub fn as_str(self) -> &'static str {
        match self {
            Backend::File => "file",
            Backend::Sqlite => "sqlite",
        }
    }
}

/// The store a session serves: `--host`, `--store` and `--backend`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreConfig {
    /// The host document, an `ekr.cli-host/1` file.
    pub host: PathBuf,
    /// Where the data lives: a directory for `file`, a database file for `sqlite`.
    pub store: PathBuf,
    /// The provider.
    pub backend: Backend,
}

/// The environment of every `ekr` process a session starts. It never inherits the consumer's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Environment {
    /// Exactly `EKR_HOST`, `EKR_STORE` and `EKR_BACKEND`, set to the session's [`StoreConfig`].
    #[default]
    Store,
    /// Exactly these variables and no other. The store travels as `--host`, `--store` and
    /// `--backend` either way.
    Exact(Vec<(OsString, OsString)>),
}

/// How a session runs its processes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionOptions {
    /// The child's environment; [`Environment::Store`] by default.
    pub environment: Environment,
    /// How long one request may take, writing it and reading its reply; 300 s by default.
    pub timeout: Duration,
    /// The child's working directory, which relative paths in a request are read from; the
    /// consumer's own when `None`.
    pub current_dir: Option<PathBuf>,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            environment: Environment::Store,
            timeout: Duration::from_secs(300),
            current_dir: None,
        }
    }
}

/// Cancels a [`ProcessSession`] from anywhere, including a signal handler.
///
/// [`CancelHandle::cancel`] only stores `true` in an atomic flag, which is async-signal-safe; a
/// watcher thread of the session sees the flag within 20 ms and kills the child. The flag itself
/// is [`CancelHandle::flag`], so `signal_hook::flag::register(SIGTERM, handle.flag())` cancels on
/// `SIGTERM` with no handler code of the consumer's own.
#[derive(Clone, Debug)]
pub struct CancelHandle {
    flag: Arc<AtomicBool>,
}

impl CancelHandle {
    /// Stop the session's child and latch the session. Idempotent.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Whether the session has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// The flag [`CancelHandle::cancel`] sets: storing `true` in it cancels.
    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.flag)
    }
}

/// What the reader and writer threads report.
enum Event {
    /// One line from the child's stdout, newline excluded.
    Line(Vec<u8>),
    /// The child's stdout ended.
    Closed,
    /// Writing a request to the child's stdin failed.
    WriteFailed,
}

/// The last `limit` bytes a process wrote to stderr, collected by a thread of its own.
#[derive(Debug)]
pub(crate) struct StderrTail {
    bytes: Arc<Mutex<VecDeque<u8>>>,
    reader: JoinHandle<()>,
}

impl StderrTail {
    pub(crate) fn collect(mut stderr: impl Read + Send + 'static, limit: usize) -> Self {
        let bytes = Arc::new(Mutex::new(VecDeque::new()));
        let collected = Arc::clone(&bytes);
        let reader = std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            while let Ok(read @ 1..) = stderr.read(&mut chunk) {
                let mut tail = collected.lock().unwrap_or_else(|e| e.into_inner());
                tail.extend(&chunk[..read]);
                let over = tail.len().saturating_sub(limit);
                tail.drain(..over);
            }
        });
        Self { bytes, reader }
    }

    /// The tail as text, after waiting up to one second for the process's stderr to close.
    pub(crate) fn settled(&self) -> String {
        let waited = Instant::now();
        while !self.reader.is_finished() && waited.elapsed() < Duration::from_secs(1) {
            std::thread::sleep(Duration::from_millis(5));
        }
        let tail = self.bytes.lock().unwrap_or_else(|e| e.into_inner());
        String::from_utf8_lossy(&tail.iter().copied().collect::<Vec<u8>>()).into_owned()
    }
}

/// The last [`STDERR_TAIL_BYTES`] of `text`, cut at a character boundary.
fn tail(text: &str) -> String {
    let mut start = text.len().saturating_sub(STDERR_TAIL_BYTES);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].to_owned()
}

/// The child and the flags its watcher thread reads.
struct Shared {
    child: Mutex<Child>,
    cancel: Arc<AtomicBool>,
    closing: AtomicBool,
}

impl Shared {
    fn kill(&self) -> Option<ExitStatus> {
        let mut child = self.child.lock().unwrap_or_else(|e| e.into_inner());
        let _ = child.kill();
        child.wait().ok()
    }
}

/// A request line on the wire.
#[derive(Serialize)]
struct WireRequest<'a> {
    argv: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    stdin: Option<&'a str>,
}

/// A reply line on the wire.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireReply {
    exit: i32,
    stdout: Value,
    stderr: String,
}

/// A child `ekr session --create`, driven one blocking request at a time.
pub struct ProcessSession {
    binary: EkrBinary,
    store: StoreConfig,
    options: SessionOptions,
    shared: Arc<Shared>,
    pid: u32,
    requests: Option<Sender<Vec<u8>>>,
    events: Receiver<Event>,
    stderr: StderrTail,
    latched: Option<String>,
    started: usize,
}

impl std::fmt::Debug for ProcessSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessSession")
            .field("binary", &self.binary)
            .field("store", &self.store)
            .field("pid", &self.pid)
            .field("latched", &self.latched)
            .field("started", &self.started)
            .finish_non_exhaustive()
    }
}

impl ProcessSession {
    /// Start `ekr session --create` over `store`, from `binary`.
    ///
    /// Nothing is read from the child here: a child that cannot open its store exits, and the
    /// first request reports that, with what it wrote to stderr.
    pub fn start(
        binary: &EkrBinary,
        store: StoreConfig,
        options: SessionOptions,
    ) -> Result<Self, TransportError> {
        let mut command = command(binary, &store, &options);
        command
            .args(["session", "--create"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = spawn(&mut command).map_err(|source| TransportError::Io {
            verb: "session".to_owned(),
            what: format!("starting {}", binary.path().display()),
            source,
        })?;
        let pid = child.id();
        let stdin = child.stdin.take().expect("stdin is piped");
        let stdout = child.stdout.take().expect("stdout is piped");
        let stderr = StderrTail::collect(
            child.stderr.take().expect("stderr is piped"),
            STDERR_TAIL_BYTES,
        );

        let (events_to, events) = mpsc::channel();
        let (requests, requests_from) = mpsc::channel::<Vec<u8>>();
        let writer_events = events_to.clone();
        std::thread::spawn(move || write_requests(stdin, &requests_from, &writer_events));
        std::thread::spawn(move || read_replies(stdout, &events_to));

        let cancel = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Shared {
            child: Mutex::new(child),
            cancel,
            closing: AtomicBool::new(false),
        });
        let watched = Arc::clone(&shared);
        std::thread::spawn(move || watch(&watched));

        Ok(Self {
            binary: binary.clone(),
            store,
            options,
            shared,
            pid,
            requests: Some(requests),
            events,
            stderr,
            latched: None,
            started: 1,
        })
    }

    /// A handle that cancels this session from any thread or a signal handler.
    pub fn cancel_handle(&self) -> CancelHandle {
        CancelHandle {
            flag: Arc::clone(&self.shared.cancel),
        }
    }

    /// The child session's process id.
    pub fn id(&self) -> u32 {
        self.pid
    }

    /// How many `ekr` processes this session has started: the session itself and one per
    /// request over [`LINE_CAP`].
    pub fn processes_started(&self) -> usize {
        self.started
    }

    /// End the session: close its input and wait, up to the request timeout, for it to exit —
    /// it writes the store's replay checkpoint first. A child still running then is killed.
    pub fn close(mut self) -> Result<ExitStatus, TransportError> {
        self.shutdown().ok_or_else(|| TransportError::Io {
            verb: "session".to_owned(),
            what: "waiting for the session to exit".to_owned(),
            source: std::io::Error::other("the child's status could not be read"),
        })
    }

    fn shutdown(&mut self) -> Option<ExitStatus> {
        self.shared.closing.store(true, Ordering::SeqCst);
        self.requests = None;
        let deadline = Instant::now() + self.options.timeout;
        loop {
            let status = {
                let mut child = self.shared.child.lock().unwrap_or_else(|e| e.into_inner());
                child.try_wait()
            };
            match status {
                Ok(Some(status)) => return Some(status),
                // The watcher has stopped; a cancel while closing is honoured here instead.
                Ok(None) if Instant::now() < deadline && !self.cancelled() => {
                    std::thread::sleep(TICK)
                }
                _ => return self.shared.kill(),
            }
        }
    }

    /// Stop the child, latch the session with `error`'s message and return `error`.
    fn fail(&mut self, error: TransportError) -> TransportError {
        self.shared.kill();
        self.latched = Some(error.to_string());
        error
    }

    fn cancelled(&self) -> bool {
        self.shared.cancel.load(Ordering::SeqCst)
    }

    /// The failure of a child that ended: cancelled if it was, otherwise how it ended.
    fn died(&mut self, verb: String) -> TransportError {
        if self.cancelled() {
            return self.fail(TransportError::Cancelled { verb });
        }
        let status = self
            .shared
            .kill()
            .map_or_else(|| "status unknown".to_owned(), |status| status.to_string());
        let stderr_tail = self.stderr.settled();
        self.fail(TransportError::Died {
            verb,
            status,
            stderr_tail,
        })
    }

    /// `request` as its own `ekr <argv>` process, for a line over [`LINE_CAP`].
    fn one_shot(&mut self, request: &Request, verb: String) -> Result<Reply, TransportError> {
        let mut command = command(&self.binary, &self.store, &self.options);
        command
            .args(&request.argv)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let io = |what: &str, source| TransportError::Io {
            verb: verb.clone(),
            what: what.to_owned(),
            source,
        };
        // The timeout and the cancel flag cover waiting for a busy binary as well as the request.
        let deadline = Instant::now() + self.options.timeout;
        let cancel = Arc::clone(&self.shared.cancel);
        let started = spawn_within(&mut command, Some(deadline), &|| {
            cancel.load(Ordering::SeqCst)
        });
        let mut child = match started {
            Ok(child) => child,
            Err(SpawnRefused::Io(source)) => return Err(io("starting a one-shot ekr", source)),
            Err(SpawnRefused::Stopped) => {
                return Err(self.fail(TransportError::Cancelled { verb }));
            }
            Err(SpawnRefused::Expired) => {
                return Err(TransportError::TimedOut {
                    verb,
                    timeout: self.options.timeout,
                    stderr_tail: String::new(),
                });
            }
        };
        self.started += 1;
        let mut stdin = child.stdin.take().expect("stdin is piped");
        let text = request.stdin.clone().unwrap_or_default();
        let writer = std::thread::spawn(move || stdin.write_all(text.as_bytes()));
        let mut stdout = child.stdout.take().expect("stdout is piped");
        let reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).map(|_| bytes)
        });
        // The reply carries the whole of stderr, as the one-shot verb wrote it; errors show its tail.
        let stderr = StderrTail::collect(child.stderr.take().expect("stderr is piped"), usize::MAX);
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {}
                Err(source) => {
                    let _ = child.kill();
                    return Err(io("waiting for a one-shot ekr", source));
                }
            }
            if self.cancelled() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(self.fail(TransportError::Cancelled { verb }));
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(TransportError::TimedOut {
                    verb,
                    timeout: self.options.timeout,
                    stderr_tail: tail(&stderr.settled()),
                });
            }
            std::thread::sleep(TICK);
        };
        let _ = writer.join();
        let stdout = reader
            .join()
            .expect("the stdout reader does not panic")
            .map_err(|source| io("reading a one-shot ekr", source))?;
        let stderr_text = stderr.settled();
        let Some(exit) = status.code() else {
            return Err(TransportError::Died {
                verb,
                status: status.to_string(),
                stderr_tail: tail(&stderr_text),
            });
        };
        let document = if stdout.iter().all(u8::is_ascii_whitespace) {
            None
        } else {
            Some(
                serde_json::from_slice(&stdout).map_err(|error| TransportError::Protocol {
                    verb,
                    detail: format!("a one-shot ekr printed something that is not JSON: {error}"),
                    stderr_tail: tail(&stderr_text),
                })?,
            )
        };
        Ok(Reply {
            exit,
            document,
            stderr: stderr_text,
        })
    }
}

impl Transport for ProcessSession {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        let verb = request.verb().to_owned();
        if let Some(cause) = &self.latched {
            return Err(TransportError::Latched {
                verb,
                cause: cause.clone(),
            });
        }
        if self.cancelled() {
            return Err(self.fail(TransportError::Cancelled { verb }));
        }
        let line = serde_json::to_vec(&WireRequest {
            argv: &request.argv,
            stdin: request.stdin.as_deref(),
        })
        .expect("a request is JSON");
        if line.len() <= LINE_CAP {
            return self.exchange(line, verb);
        }
        // Over the cap, a `-` argument reads the text from a private file instead, inside the
        // session, so a `seed -` leaves the session holding the store it creates.
        let reads_stdin = request.argv.iter().skip(1).any(|argument| argument == "-");
        if let (true, Some(text)) = (reads_stdin, &request.stdin) {
            let file = spill(text).map_err(|source| TransportError::Io {
                verb: verb.clone(),
                what: "writing the request's stdin to a temporary file".to_owned(),
                source,
            })?;
            if let Some(path) = file.path().to_str() {
                let argv: Vec<String> = request
                    .argv
                    .iter()
                    .enumerate()
                    .map(|(at, argument)| match argument.as_str() {
                        "-" if at > 0 => path.to_owned(),
                        _ => argument.clone(),
                    })
                    .collect();
                let line = serde_json::to_vec(&WireRequest {
                    argv: &argv,
                    stdin: None,
                })
                .expect("a request is JSON");
                if line.len() <= LINE_CAP {
                    return self.exchange(line, verb);
                }
            }
        }
        self.one_shot(request, verb)
    }
}

/// `text` in a new temporary file only this user can read (mode 0600), deleted when dropped.
fn spill(text: &str) -> std::io::Result<tempfile::NamedTempFile> {
    let mut file = tempfile::Builder::new()
        .prefix("ekr-sdk-stdin-")
        .tempfile()?;
    file.write_all(text.as_bytes())?;
    file.flush()?;
    Ok(file)
}

impl ProcessSession {
    /// Send one request line to the child and read its reply.
    fn exchange(&mut self, mut line: Vec<u8>, verb: String) -> Result<Reply, TransportError> {
        line.push(b'\n');
        let sent = self
            .requests
            .as_ref()
            .is_some_and(|requests| requests.send(line).is_ok());
        if !sent {
            return Err(self.died(verb));
        }
        let deadline = Instant::now() + self.options.timeout;
        loop {
            let wait = deadline.saturating_duration_since(Instant::now()).min(TICK);
            match self.events.recv_timeout(wait) {
                Ok(Event::Line(bytes)) => {
                    return match serde_json::from_slice::<WireReply>(&bytes) {
                        Ok(reply) => Ok(Reply {
                            exit: reply.exit,
                            document: (!reply.stdout.is_null()).then_some(reply.stdout),
                            stderr: reply.stderr,
                        }),
                        Err(error) => {
                            let stderr_tail = self.stderr.settled();
                            Err(self.fail(TransportError::Protocol {
                                verb,
                                detail: error.to_string(),
                                stderr_tail,
                            }))
                        }
                    };
                }
                Ok(Event::Closed | Event::WriteFailed) | Err(RecvTimeoutError::Disconnected) => {
                    return Err(self.died(verb))
                }
                Err(RecvTimeoutError::Timeout) => {
                    if self.cancelled() {
                        return Err(self.fail(TransportError::Cancelled { verb }));
                    }
                    if Instant::now() >= deadline {
                        self.shared.kill();
                        let stderr_tail = self.stderr.settled();
                        return Err(self.fail(TransportError::TimedOut {
                            verb,
                            timeout: self.options.timeout,
                            stderr_tail,
                        }));
                    }
                }
            }
        }
    }
}

impl Drop for ProcessSession {
    fn drop(&mut self) {
        if self.latched.is_some() || self.cancelled() {
            self.shared.closing.store(true, Ordering::SeqCst);
            self.shared.kill();
        } else {
            self.shutdown();
        }
    }
}

/// `binary --host … --store … --backend …` with a cleared environment: the store's three
/// variables, or exactly the list given.
pub(crate) fn command(
    binary: &EkrBinary,
    store: &StoreConfig,
    options: &SessionOptions,
) -> Command {
    let mut command = Command::new(binary.path());
    command.env_clear();
    match &options.environment {
        Environment::Store => {
            command
                .env("EKR_HOST", &store.host)
                .env("EKR_STORE", &store.store)
                .env("EKR_BACKEND", store.backend.as_str());
        }
        Environment::Exact(variables) => {
            command.envs(variables.iter().map(|(name, value)| (name, value)));
        }
    }
    command
        .arg("--host")
        .arg(&store.host)
        .arg("--store")
        .arg(&store.store)
        .args(["--backend", store.backend.as_str()]);
    if let Some(directory) = &options.current_dir {
        command.current_dir(directory);
    }
    command
}

/// Write each request line to the child, until the session drops its sender; then close stdin.
fn write_requests(mut stdin: ChildStdin, requests: &Receiver<Vec<u8>>, events: &Sender<Event>) {
    for line in requests {
        if stdin.write_all(&line).and_then(|()| stdin.flush()).is_err() {
            let _ = events.send(Event::WriteFailed);
            return;
        }
    }
}

/// Send each line of the child's stdout as an event, then [`Event::Closed`].
fn read_replies(stdout: impl Read, events: &Sender<Event>) {
    let mut stdout = BufReader::new(stdout);
    loop {
        let mut line = Vec::new();
        match stdout.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => {
                let _ = events.send(Event::Closed);
                return;
            }
            Ok(_) => {
                if line.last() == Some(&b'\n') {
                    line.pop();
                }
                if events.send(Event::Line(line)).is_err() {
                    return;
                }
            }
        }
    }
}

/// Kill the child once the cancel flag is set; stop watching once the session closes.
fn watch(shared: &Shared) {
    while !shared.closing.load(Ordering::SeqCst) {
        if shared.cancel.load(Ordering::SeqCst) {
            shared.kill();
            return;
        }
        std::thread::sleep(TICK);
    }
}
