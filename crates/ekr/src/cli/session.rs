//! `ekr session`: the JSON verbs served over one opened store, one request per line.
//!
//! The session resolves its configuration and opens the store once, then answers each line of its
//! input with one line of output until end of input. A request is the argv of a one-shot verb,
//! parsed by the same clap definitions and run through the same dispatch as the one-shot verb,
//! against the runtime the session holds instead of one it opens: every verb body in this
//! directory runs unchanged. The runtime reads the store's retained history on every verb, so a
//! commit a request makes is what the next request reads.

use std::io::{BufRead, Write};

use clap::error::ErrorKind;
use clap::Parser;
use ekr_core::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Cli, Command, Configured, Held, Source};
use crate::exit::Failure;

/// A line that is not a request: not JSON, not an object, without `argv`, with a field a request
/// does not have, or with an `argv` that is not a list of strings or a `stdin` that is not text.
const MALFORMED: &str = "session-request-malformed";
/// A request whose `argv` names no verb, or one `ekr` does not have.
const UNKNOWN: &str = "session-verb-unknown";
/// A verb, clap's `help` verb, `--help` or `--version`: what a session does not serve.
const REFUSED: &str = "session-verb-refused";
/// A request that sets a global option: the session's store was fixed when it started.
const OPTION: &str = "session-option-refused";
/// A line longer than [`LINE_LIMIT`]: longer than any request a verb could accept.
const TOO_LARGE: &str = "session-request-too-large";

/// The most bytes of one request line, its newline excluded: 25 231 360. The largest input a verb
/// reads is an `ekr.transaction-document/2` of 8 388 608 bytes, and JSON-escaping text in a string
/// at most triples it (a two-byte UTF-8 character written `\uXXXX` is six bytes; `\n`, `\"`
/// are two), so three times that and 64 KiB for `argv` and the framing hold any request a verb
/// could accept. Only this much of a line is ever held; the rest is read and dropped.
const LINE_LIMIT: usize = 3 * ekr_kernel::DOCUMENT_V2_LIMITS.input_bytes + 65_536;

/// One request line.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    /// The verb and its arguments, as `ekr` takes them after its name.
    argv: Vec<String>,
    /// What `-` reads, for `propose -`, `resolve -` and `hash -`; nothing when absent.
    #[serde(default)]
    stdin: Option<String>,
}

/// One answer line: what the one-shot verb would have exited with and printed.
#[derive(Serialize)]
struct Answer {
    exit: u8,
    /// The verb's JSON document; `null` when it printed nothing.
    stdout: Value,
    /// The verb's stderr text, newline included; empty when it printed nothing there.
    stderr: String,
}

/// Opens the store `cli`'s configuration names, once, then answers each line of `input` with one
/// line on `output`, flushed, until end of input.
///
/// # Errors
///
/// What a store verb reports when its configuration or store does not open — before any line is
/// read — and a fault reading `input` or writing `output`. A request never ends the session.
pub fn serve(
    cli: Cli,
    now: &dyn Fn() -> Timestamp,
    input: &mut dyn BufRead,
    output: &mut dyn Write,
) -> Result<(), Failure> {
    let (configured, _) = Configured::split(cli);
    let store = configured.resolve("session")?;
    let held = Held {
        runtime: store.open()?,
        operator: store.host.context.operator,
    };
    let mut line = Vec::new();
    loop {
        let whole = match next_line(input, &mut line, LINE_LIMIT)
            .map_err(|error| Failure::fault(format!("reading a session request: {error}")))?
        {
            None => return Ok(()),
            Some(whole) => whole,
        };
        let answered = if whole {
            respond(&line, &held, now)
        } else {
            Err(Failure::refused(
                TOO_LARGE,
                format!(
                    "a request line holds at most {LINE_LIMIT} bytes; the rest of the line was \
                     read and dropped"
                ),
            ))
        };
        let answer = match answered {
            Ok(stdout) => Answer {
                exit: 0,
                stdout,
                stderr: String::new(),
            },
            Err(failure) => Answer {
                exit: failure.code(),
                stdout: Value::Null,
                stderr: stderr(&failure),
            },
        };
        let mut text = serde_json::to_string(&answer).map_err(Failure::fault)?;
        text.push('\n');
        output
            .write_all(text.as_bytes())
            .and_then(|()| output.flush())
            .map_err(|error| Failure::fault(format!("writing a session answer: {error}")))?;
    }
}

/// Reads the next line into `line`, its newline dropped, holding at most `limit` bytes of
/// it: `Some(true)` for a whole line, `Some(false)` for one longer than that, whose rest was read
/// and dropped up to its newline, and `None` at the end of input.
pub(super) fn next_line(
    input: &mut dyn BufRead,
    line: &mut Vec<u8>,
    limit: usize,
) -> std::io::Result<Option<bool>> {
    line.clear();
    let (mut any, mut whole) = (false, true);
    loop {
        let buffer = input.fill_buf()?;
        if buffer.is_empty() {
            return Ok(any.then_some(whole));
        }
        any = true;
        let (taken, ended) = match buffer.iter().position(|byte| *byte == b'\n') {
            Some(at) => (at, true),
            None => (buffer.len(), false),
        };
        if whole {
            if line.len() + taken > limit {
                whole = false;
                line.clear();
            } else {
                line.extend_from_slice(&buffer[..taken]);
            }
        }
        input.consume(if ended { taken + 1 } else { taken });
        if ended {
            return Ok(Some(whole));
        }
    }
}

/// The request's verb run against the held runtime, and the document it prints.
fn respond(line: &[u8], held: &Held, now: &dyn Fn() -> Timestamp) -> Result<Value, Failure> {
    let request: Request = serde_json::from_slice(line).map_err(|error| {
        Failure::refused(
            MALFORMED,
            format!("a request is {{\"argv\": [...]}}: {error}"),
        )
    })?;
    let cli = parse(&request.argv)?;
    admit(&cli)?;
    let mut stdin = request.stdin.as_deref().unwrap_or_default().as_bytes();
    match super::dispatch(cli.command, Source::Held(held), now, &mut stdin)? {
        super::Printed::Document(document) => Ok(document),
        super::Printed::Text(_) => {
            Err(Failure::fault("the verb printed text, not a JSON document"))
        }
    }
}

/// `argv` parsed by the one-shot verb's own clap definitions.
fn parse(argv: &[String]) -> Result<Cli, Failure> {
    Cli::try_parse_from(std::iter::once("ekr").chain(argv.iter().map(String::as_str))).map_err(
        |error| match error.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => Failure::refused(
                REFUSED,
                "`help`, `--help` and `--version` print text, not a JSON document; run them \
                 outside the session",
            ),
            ErrorKind::InvalidSubcommand
            | ErrorKind::MissingSubcommand
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => Failure::refused(
                UNKNOWN,
                format!("{argv:?} names no verb of `ekr`; `ekr --help` lists them"),
            ),
            _ => Failure::Usage {
                message: error.render().to_string(),
            },
        },
    )
}

/// Refuses a request that sets a global option, or names a verb the session does not serve.
fn admit(cli: &Cli) -> Result<(), Failure> {
    if cli.host.is_some() || cli.store.is_some() || cli.backend.is_some() || cli.full_replay {
        return Err(Failure::refused(
            OPTION,
            "--host, --store, --backend and --full-replay are the session's own, fixed when it \
             started; a request carries none of them",
        ));
    }
    match cli.command {
        Command::Seed { .. } => Err(verb_refused("seed")),
        Command::View { .. } => Err(verb_refused("view")),
        Command::Session => Err(verb_refused("session")),
        Command::Mcp => Err(verb_refused("mcp")),
        Command::Guide => Err(verb_refused("guide")),
        Command::Operations { .. } => Err(verb_refused("operations")),
        Command::Example { .. } => Err(verb_refused("example")),
        Command::Propose { .. }
        | Command::Validate { .. }
        | Command::Commit { .. }
        | Command::Snapshot { .. }
        | Command::Explain { .. }
        | Command::Resolve { .. }
        | Command::Head
        | Command::Transactions { .. }
        | Command::Ontology { .. }
        | Command::Schema { .. }
        | Command::Mint { .. }
        | Command::Hash { .. } => Ok(()),
    }
}

/// The refusal of a verb a session does not serve: `seed` creates a store, `view` serves until
/// interrupted, `mcp` until its own input ends, a session does not nest, and `guide`,
/// `operations` and `example` print text.
pub(super) fn verb_refused(verb: &str) -> Failure {
    Failure::refused(
        REFUSED,
        format!(
            "a session serves the verbs that print one JSON document from an existing store; run \
             `ekr {verb}` outside the session"
        ),
    )
}

/// What the one-shot binary writes to stderr for `failure`: clap's own message as it renders it,
/// every other failure as one line.
fn stderr(failure: &Failure) -> String {
    match failure {
        Failure::Usage { message } => message.clone(),
        failure => format!("{failure}\n"),
    }
}
