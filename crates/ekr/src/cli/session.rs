//! `ekr session`: the JSON verbs served over one opened store, one request per line.
//!
//! The session resolves its configuration and opens the store once, then answers each line of its
//! input with one line of output until end of input. A request is the argv of a one-shot verb,
//! parsed by the same clap definitions and run through the same dispatch as the one-shot verb,
//! against the runtime the session holds instead of one it opens: every verb body in this
//! directory runs unchanged. The runtime reads the store's retained history on every verb, so a
//! commit a request makes is what the next request reads. At the end of input the session
//! writes the replay checkpoint of the newest head it reached when that head is past the
//! retained checkpoint (design § 99.5).
//!
//! A session also serves the `ekr.views` reads as verbs of its own — `overview`, `search`,
//! `describe`, `expand`, `timeline` and `changes` ([`views`]) — each answering the document
//! `ekr view` serves for the same query and revision, from indexes it keeps of the store it holds
//! as `ekr view` keeps them. `ekr` has no one-shot verb of these names.
//!
//! On a path that holds no store the session starts without one. Until it holds one, a store
//! verb opens the store as the one-shot verb does — `store-not-found` where there is none — and
//! the verbs that open no store answer as always. Started with `--create`, it also serves `seed`
//! through the one-shot verb's own path; the seed that creates the store leaves the session
//! holding it, opened once as at the start. A views verb that finds a store at the path —
//! another process created it — leaves the session holding it the same way.
//!
//! **A store replaced at its path.** A long-running reader — this session, `ekr mcp`, `ekr view` —
//! compares, before each request that reads the store, the identity of what is at its configured
//! path ([`Identity`]: device and inode of the file store's root directory or of the SQLite
//! database file) with the one it opened. That is one `stat` of the path and no store read. When
//! they differ, the store was replaced (by a rename, say): the reader drops the runtime it holds,
//! closing the replaced store before it opens another, then opens the store now at the path and
//! answers from it, and a reader that keeps indexes empties them. If that open fails it answers
//! [`STORE_REPLACED`], naming the path, and never the replaced store's data; the next request
//! tries again. A file store replaced under the same device and inode — deleted and created again,
//! or its files replaced inside it — passes that check; the held runtime then refuses its read as a
//! diverged history ([`reopens`]), and the reader opens the store at the path once and answers
//! the request again from it. So does a SQLite database copied over the file in place, which the
//! held runtime refuses as `store-replaced` (`PersistenceError::Replaced`). A session holding a
//! transaction it proposed that is neither committed nor rejected — read from the held runtime,
//! so one another process committed does not count — refuses the replacement instead, as
//! [`PROPOSALS_OPEN`], on every store verb until the store it opened is back at the path.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::io::{BufRead, Write};
use std::path::Path;

use clap::error::ErrorKind;
use clap::Parser;
use ekr_core::{Timestamp, TransactionId};
use ekr_kernel::Runtime;
use ekr_views::IndexCache;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Cli, Command, Configured, Session, Source, Store};
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
/// The store at a long-running reader's path is not the one it opened, and what is there now does
/// not open. `ekr session` answers it as a fault (exit 1), `ekr mcp` as a tool refusal and
/// `ekr view` as 503.
pub(super) const STORE_REPLACED: &str = "store-replaced";
/// The store at a session's path was replaced while a transaction the session proposed is neither
/// committed nor rejected: the session keeps the store that holds it.
const PROPOSALS_OPEN: &str = "store-replaced-proposals-open";

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
    stdout: Stdout,
    /// The verb's stderr text, newline included; empty when it printed nothing there.
    stderr: String,
}

/// An answer's `stdout`: a document, `null` included, or the exact bytes a verb printed raw
/// (`Printed::Raw`), embedded as they are.
#[derive(Serialize)]
#[serde(untagged)]
enum Stdout {
    Document(Value),
    Raw(Box<serde_json::value::RawValue>),
}

/// Opens the store `cli`'s configuration names, once — or, where the path holds no store, starts
/// without one — then answers each line of `input` with one line on `output`, flushed, until end
/// of input. `ekr session --create` also serves `seed`.
///
/// # Errors
///
/// What a store verb reports when its configuration or an existing store does not open — before
/// any line is read — and a fault reading `input` or writing `output`. A request never ends the
/// session.
pub fn serve(
    cli: Cli,
    now: &dyn Fn() -> Timestamp,
    input: &mut dyn BufRead,
    output: &mut dyn Write,
) -> Result<(), Failure> {
    let (configured, command) = Configured::split(cli);
    let create = matches!(command, Command::Session { create: true });
    let store = configured.resolve("session")?;
    let opened = identity(&store.store);
    let mut session = Session {
        runtime: store.open_if_any()?,
        store,
        create,
    };
    let mut watch = Watch {
        held: session.runtime.is_some(),
        opened,
        proposed: BTreeSet::new(),
        indexes: IndexCache::new(IndexCache::DEFAULT_CAPACITY),
    };
    let mut line = Vec::new();
    loop {
        let whole = match next_line(input, &mut line, LINE_LIMIT)
            .map_err(|error| Failure::fault(format!("reading a session request: {error}")))?
        {
            None => {
                // At the end of input the session leaves the checkpoint of the head it reached,
                // when that head is past the retained one (design § 99.5) — into the store it
                // opened only, never into one that replaced it at the path.
                if let Some(runtime) = &session.runtime {
                    if identity(&session.store.store) == watch.opened {
                        runtime.retain_checkpoint_at_rest();
                    }
                }
                return Ok(());
            }
            Some(whole) => whole,
        };
        let answered = if whole {
            respond(&line, &mut session, &mut watch, now)
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
                stdout: Stdout::Document(Value::Null),
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

/// What a session knows of the store it holds beyond its runtime: whether it has held one, the
/// identity of the store it opened, the transactions it proposed that are neither committed
/// nor rejected, and the `ekr.views` indexes of the revisions its views verbs read from it,
/// loaded once per revision as `ekr view` keeps them and dropped when it opens another store.
struct Watch {
    held: bool,
    opened: Option<Identity>,
    proposed: BTreeSet<TransactionId>,
    indexes: IndexCache,
}

/// What a request does to the session's open proposals once it has answered.
enum Tracked {
    Proposes,
    Validates(TransactionId),
    Commits(TransactionId),
    Nothing,
}

/// The request's verb run against the session's store, and the document it prints. A seed that
/// succeeds in a session holding no store has created it: the session opens and holds it. A store
/// verb first follows a store replaced at the session's path ([`follow`]).
fn respond(
    line: &[u8],
    session: &mut Session,
    watch: &mut Watch,
    now: &dyn Fn() -> Timestamp,
) -> Result<Stdout, Failure> {
    let request: Request = serde_json::from_slice(line).map_err(|error| {
        Failure::refused(
            MALFORMED,
            format!("a request is {{\"argv\": [...]}}: {error}"),
        )
    })?;
    answer(&request, session, watch, now)
}

/// One request, read from its line, answered as [`respond`] answers that line.
fn answer(
    request: &Request,
    session: &mut Session,
    watch: &mut Watch,
    now: &dyn Fn() -> Timestamp,
) -> Result<Stdout, Failure> {
    let cli = match parse(&request.argv) {
        Ok(cli) => cli,
        // Not a one-shot verb: one of the `ekr.views` reads, or no verb at all.
        Err(unknown) if unknown.name() == Some(UNKNOWN) => {
            return match views::parse(&request.argv) {
                Some(views) => read_views(&views?, session, watch).map(Stdout::Document),
                None => Err(unknown),
            };
        }
        Err(failure) => return Err(failure),
    };
    admit(&cli, session.create)?;
    let seeds = matches!(cli.command, Command::Seed { .. });
    let tracked = match &cli.command {
        Command::Propose { .. } => Tracked::Proposes,
        Command::Validate { transaction_id, .. } => Tracked::Validates(*transaction_id),
        Command::Commit { transaction_id } => Tracked::Commits(*transaction_id),
        _ => Tracked::Nothing,
    };
    let reads_store = !matches!(
        cli.command,
        Command::Mint { .. } | Command::Hash { .. } | Command::Schema { .. }
    );
    if reads_store {
        follow(session, watch, false)?;
    }
    // A session opens its store as a reader does; on a store this process may not write it holds
    // it read-only, and each request whose verb writes is refused by name before it runs.
    if cli.command.access() == super::Access::Write {
        if let Some(runtime) = &session.runtime {
            if runtime.is_read_only() {
                return Err(super::read_only(&format!(
                    "the session holds the store at {} read-only",
                    session.store.store.display()
                )));
            }
        }
    }
    let mut stdin = request.stdin.as_deref().unwrap_or_default().as_bytes();
    let printed = match super::dispatch(cli.command, Source::Session(session), now, &mut stdin) {
        // The held runtime's history diverged from the store at the path, or its SQLite database
        // was replaced in place: a store replaced under the same device and inode. Reopened
        // once, the request is run again there.
        Err(failure) if reads_store && session.runtime.is_some() && failure.reopens() => {
            follow(session, watch, true)?;
            let mut stdin = request.stdin.as_deref().unwrap_or_default().as_bytes();
            super::dispatch(
                parse(&request.argv)?.command,
                Source::Session(session),
                now,
                &mut stdin,
            )?
        }
        printed => printed?,
    };
    if seeds && session.runtime.is_none() {
        // Where this open fails, the seed's answer still stands and the session stays without a
        // store: each store verb then opens it as the one-shot verb does and says why it cannot.
        let opened = identity(&session.store.store);
        if let Ok(Some(runtime)) = session.store.open_if_any() {
            session.runtime = Some(runtime);
            watch.held = true;
            watch.opened = opened;
        }
    }
    let document = match printed {
        super::Printed::Document(document) => document,
        // Only `fact-quality` prints raw, and it proposes, validates and commits nothing.
        super::Printed::Raw(document) => return Ok(Stdout::Raw(document)),
        super::Printed::Text(_) => {
            return Err(Failure::fault("the verb printed text, not a JSON document"))
        }
    };
    match tracked {
        Tracked::Proposes => {
            if let Some(id) = document["transaction_id"]
                .as_str()
                .and_then(|id| id.parse().ok())
            {
                watch.proposed.insert(id);
            }
        }
        Tracked::Validates(id) if document["kind"] == "Rejected" => {
            watch.proposed.remove(&id);
        }
        Tracked::Commits(id) if document["kind"] == "Committed" => {
            watch.proposed.remove(&id);
        }
        Tracked::Validates(_) | Tracked::Commits(_) | Tracked::Nothing => {}
    }
    Ok(Stdout::Document(document))
}

/// A views verb ([`views`]) against the session's store: refused as [`admit`] refuses a global
/// option, then answered as a store verb is — after [`follow`], and once more from the store at
/// the path when the held runtime's history diverged from it.
///
/// A session holding no store opens the one at its path as a one-shot verb does —
/// `store-not-found` where there is none — and, where another process has created one since the
/// session started, holds it from then on, opened once, as after its own seed: so its indexes
/// are loaded once per revision there too.
fn read_views(
    views: &views::ViewsCli,
    session: &mut Session,
    watch: &mut Watch,
) -> Result<Value, Failure> {
    if views.sets_an_option() {
        return Err(option_refused());
    }
    if session.runtime.is_none() {
        let opened = identity(&session.store.store);
        session.runtime = Some(session.store.open()?);
        watch.held = true;
        watch.opened = opened;
        watch.indexes = IndexCache::new(IndexCache::DEFAULT_CAPACITY);
    }
    follow(session, watch, false)?;
    let answer = |session: &Session, indexes: &mut IndexCache| match &session.runtime {
        Some(runtime) => views::answer(views.verb(), runtime, indexes),
        None => Err(Failure::fault("the session holds no store")),
    };
    match answer(session, &mut watch.indexes) {
        Err(failure) if failure.reopens() => {
            follow(session, watch, true)?;
            answer(session, &mut watch.indexes)
        }
        answered => answered,
    }
}

/// Before a store verb: when the store at the session's path is not the one it opened — or,
/// `diverged`, the held runtime found its history diverged or its database replaced under the
/// same identity ([`Failure::reopens`]) — reopens
/// there, unless a transaction the session proposed is open ([`PROPOSALS_OPEN`]). A reopen that
/// fails is [`STORE_REPLACED`], a fault as `store-not-found` is, and leaves the session holding no
/// runtime, so no later request is answered from the replaced store and each tries again.
///
/// While the store is the one opened and the session tracks proposals, the held runtime's
/// transactions are read ([`settle`]), so one another process committed or rejected is not
/// open; a file store replaced by a rename can no longer be read through the held runtime, so
/// this is the read that can tell. Without tracked proposals nothing is read.
fn follow(session: &mut Session, watch: &mut Watch, diverged: bool) -> Result<(), Failure> {
    if !watch.held {
        return Ok(());
    }
    let now = check(&session.store.store);
    if !diverged && now == watch.opened {
        if let Some(runtime) = &session.runtime {
            // A store held read-only is a copy: once the files at the path change, it is taken
            // again, as a store replaced there is.
            if !runtime.source_changed() {
                settle(runtime, &mut watch.proposed);
                return Ok(());
            }
        }
    }
    if let Some(runtime) = &session.runtime {
        settle(runtime, &mut watch.proposed);
    }
    if session.runtime.is_some() && !watch.proposed.is_empty() {
        let open: Vec<String> = watch.proposed.iter().map(ToString::to_string).collect();
        return Err(Failure::refused(
            PROPOSALS_OPEN,
            format!(
                "the store at {} is not the one this session opened, and the session's \
                 transactions {} are neither committed nor rejected; they stay in the store it \
                 opened. Put that store back at the path, or end the session and start one on \
                 the store now there",
                session.store.store.display(),
                open.join(", ")
            ),
        ));
    }
    session.runtime = None;
    watch.indexes = IndexCache::new(IndexCache::DEFAULT_CAPACITY);
    let runtime = reopen(&session.store)
        .map_err(|replaced| Failure::fault(format!("{STORE_REPLACED}: {}", replaced.message)))?;
    session.runtime = Some(runtime);
    watch.opened = now;
    Ok(())
}

/// Drops from `proposed` every transaction the held runtime reads as committed or rejected — by
/// this session or by another process. Reads nothing when `proposed` is empty. Where the held
/// runtime cannot read its transactions, each stays open.
fn settle(runtime: &Runtime, proposed: &mut BTreeSet<TransactionId>) {
    if proposed.is_empty() {
        return;
    }
    count(|work| work.settles += 1);
    if let Ok(transactions) = runtime.transactions() {
        proposed.retain(|id| {
            transactions
                .get(id)
                .is_none_or(|record| record.committed.is_none() && record.rejection.is_none())
        });
    }
}

/// The identity of a store: the device and inode of what its path names, symbolic links
/// followed — the file store's root directory or the SQLite database file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Identity {
    device: u64,
    inode: u64,
}

/// The identity of what is at `path`, or `None` where nothing is.
pub(super) fn identity(path: &Path) -> Option<Identity> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(of(&metadata))
}

#[cfg(unix)]
fn of(metadata: &std::fs::Metadata) -> Identity {
    use std::os::unix::fs::MetadataExt as _;
    Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

/// Without inodes, a replacement is told by its creation time.
#[cfg(not(unix))]
fn of(metadata: &std::fs::Metadata) -> Identity {
    let created = metadata
        .created()
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |at| u64::try_from(at.as_nanos()).unwrap_or(u64::MAX));
    Identity {
        device: 0,
        inode: created,
    }
}

/// What long-running readers did on this thread to follow a replaced store.
///
/// Test instrumentation, as `ekr_store::read_work` is: it lets a case show that a request without
/// a replacement costs the check — one `stat` — and no store open or read, and that a session
/// reads its transactions only while it tracks proposals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct ReaderWork {
    /// Identity checks of a store path before a request.
    pub(super) checks: u64,
    /// Stores opened again because the one at the path was not the one opened.
    pub(super) reopens: u64,
    /// Reads of a session's transactions to settle the proposals it tracks.
    pub(super) settles: u64,
}

thread_local! {
    static WORK: Cell<ReaderWork> = const { Cell::new(ReaderWork {
        checks: 0,
        reopens: 0,
        settles: 0,
    }) };
}

/// The work counted on this thread since the last call, which starts the count again.
#[cfg(test)]
pub(super) fn reader_work() -> ReaderWork {
    WORK.with(|work| work.replace(ReaderWork::default()))
}

fn count(add: impl FnOnce(&mut ReaderWork)) {
    WORK.with(|work| {
        let mut now = work.get();
        add(&mut now);
        work.set(now);
    });
}

/// [`identity`], counted as a check.
fn check(path: &Path) -> Option<Identity> {
    count(|work| work.checks += 1);
    identity(path)
}

/// Why a reader answered nothing from its store: the store at its path is not the one it opened
/// and does not open. `message` names the path and why.
#[derive(Debug)]
pub(super) struct Replaced {
    pub(super) message: String,
}

/// Opens the store now at `store`'s path, counted as a reopen.
fn reopen(store: &Store) -> Result<Runtime, Replaced> {
    count(|work| work.reopens += 1);
    store.open().map_err(|failure| {
        let why = match failure {
            Failure::Refused { name, message } => format!("{name}: {message}"),
            Failure::Fault { message, .. } | Failure::Usage { message } => message,
        };
        Replaced {
            message: format!(
                "the store at {} is not the one this process opened, and what is there now does \
                 not open: {why}",
                store.store.display()
            ),
        }
    })
}

/// A long-running reader's store: its configuration, the runtime of the store it opened — none
/// after a reopen failed — and that store's identity.
pub(super) struct Held {
    store: Store,
    runtime: Option<Runtime>,
    opened: Option<Identity>,
}

/// The runtime a request reads: the one held, or one opened just now at the path.
pub(super) enum Checked<'a> {
    Same(&'a Runtime),
    /// The store was replaced and this is the new one's: whatever was kept of the old is stale.
    Reopened(&'a Runtime),
}

impl Held {
    /// Opens the existing store `store` names, as a store verb does.
    ///
    /// # Errors
    ///
    /// What [`Store::open`] reports.
    pub(super) fn open(store: Store) -> Result<Self, Failure> {
        let opened = identity(&store.store);
        let runtime = store.open()?;
        Ok(Self {
            store,
            runtime: Some(runtime),
            opened,
        })
    }

    /// The runtime of the store at the configured path: the one held while the path still names
    /// it, else — the held one dropped first — the store now there, opened.
    ///
    /// # Errors
    ///
    /// [`Replaced`] when that store does not open; the next call tries again.
    pub(super) fn current(&mut self) -> Result<Checked<'_>, Replaced> {
        let now = check(&self.store.store);
        // A store held read-only is a copy: once the files at the path change, it is opened again.
        let same = now == self.opened
            && self
                .runtime
                .as_ref()
                .is_some_and(|runtime| !runtime.source_changed());
        if !same {
            self.runtime = None;
            self.runtime = Some(reopen(&self.store)?);
            self.opened = now;
        }
        match (&self.runtime, same) {
            (Some(runtime), true) => Ok(Checked::Same(runtime)),
            (Some(runtime), false) => Ok(Checked::Reopened(runtime)),
            (None, _) => Err(Replaced {
                message: format!("no store is held for {}", self.store.store.display()),
            }),
        }
    }

    /// Drops the held runtime, so the next [`Held::current`] opens the store at the path: for a
    /// held history that diverged from the store there, or a SQLite database replaced in place
    /// ([`reopens`]), which a store replaced
    /// under the same device and inode — deleted and created again, or its files replaced inside
    /// it — leaves the identity check blind to.
    pub(super) fn forget(&mut self) {
        self.runtime = None;
    }

    /// [`reopens`] for the held runtime; `false` while none is held.
    pub(super) fn reopens(&self) -> bool {
        self.runtime.as_ref().is_some_and(reopens)
    }
}

/// Whether `runtime`'s store refuses the history it observed as diverged from the store at its
/// path, or refuses to answer because its SQLite database there was replaced in place:
/// `ekr_store`'s typed [`ekr_kernel::PersistenceError::Diverged`], which the store maps from its
/// provider, or [`ekr_kernel::PersistenceError::Replaced`]. `ekr mcp` and `ekr view` ask it, by
/// one head read, after a read through `runtime` answered a fault; a session reads its failure's
/// own [`Failure::reopens`]. No reader tells either by a message's text.
pub(super) fn reopens(runtime: &Runtime) -> bool {
    matches!(
        runtime.head(),
        Err(ekr_kernel::PersistenceError::Diverged(_) | ekr_kernel::PersistenceError::Replaced(_))
    )
}

/// `argv` parsed by the one-shot verb's own clap definitions.
fn parse(argv: &[String]) -> Result<Cli, Failure> {
    Cli::try_parse_from(std::iter::once("ekr").chain(argv.iter().map(String::as_str))).map_err(
        |error| match error.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => help_refused(),
            ErrorKind::InvalidSubcommand
            | ErrorKind::MissingSubcommand
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => Failure::refused(
                UNKNOWN,
                format!(
                    "{argv:?} names no verb of `ekr`, which `ekr --help` lists, and none of the \
                     session's ekr.views reads `overview`, `search`, `describe`, `expand`, \
                     `timeline` and `changes`"
                ),
            ),
            _ => Failure::Usage {
                message: error.render().to_string(),
            },
        },
    )
}

/// The refusal of `help`, `--help` and `--version`, which print text.
fn help_refused() -> Failure {
    Failure::refused(
        REFUSED,
        "`help`, `--help` and `--version` print text, not a JSON document; run them outside the \
         session",
    )
}

/// The refusal of a request that sets a global option.
fn option_refused() -> Failure {
    Failure::refused(
        OPTION,
        "--host, --store, --backend and --full-replay are the session's own, fixed when it \
         started; a request carries none of them",
    )
}

/// Refuses a request that sets a global option, or names a verb the session does not serve:
/// `seed` is served only by a session started with `--create`.
fn admit(cli: &Cli, create: bool) -> Result<(), Failure> {
    if cli.host.is_some() || cli.store.is_some() || cli.backend.is_some() || cli.full_replay {
        return Err(option_refused());
    }
    match cli.command {
        Command::Seed { .. } if create => Ok(()),
        Command::Seed { .. } => Err(verb_refused("seed")),
        Command::View { .. } => Err(verb_refused("view")),
        Command::Session { .. } => Err(verb_refused("session")),
        Command::Mcp => Err(verb_refused("mcp")),
        Command::Migrate { .. } => Err(verb_refused("migrate")),
        Command::Guide => Err(verb_refused("guide")),
        Command::Operations { .. } => Err(verb_refused("operations")),
        Command::Example { .. } => Err(verb_refused("example")),
        Command::ApplyExtraction { .. } => Err(verb_refused("apply-extraction")),
        Command::Propose { .. }
        | Command::Validate { .. }
        | Command::Commit { .. }
        | Command::Snapshot { .. }
        | Command::Explain { .. }
        | Command::Resolve { .. }
        | Command::Head
        | Command::Transactions { .. }
        | Command::Rejections { .. }
        | Command::Ontology { .. }
        | Command::CodeNames { .. }
        | Command::Quality { .. }
        | Command::Ocel { .. }
        | Command::Sample { .. }
        | Command::FactQuality { .. }
        | Command::Schema { .. }
        | Command::Mint { .. }
        | Command::Hash { .. } => Ok(()),
    }
}

/// The refusal of a verb a session does not serve: `seed` creates a store, which only a session
/// started with `--create` does, `view` serves until interrupted, `mcp` until its own input ends,
/// a session does not nest, `migrate` writes a second store, `apply-extraction` is a run of the
/// verbs a session serves, and `guide`, `operations` and `example` print text.
pub(super) fn verb_refused(verb: &str) -> Failure {
    let instead = if verb == "seed" {
        "run `ekr seed` outside the session, or start the session with `ekr session --create`"
            .to_owned()
    } else {
        format!("run `ekr {verb}` outside the session")
    };
    Failure::refused(
        REFUSED,
        format!("a session serves the verbs that print one JSON document; {instead}"),
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

/// The transport `ekr apply-extraction` runs the SDK's apply routine over: each request answered
/// as a session answers its line ([`answer`]), against the runtime the verb opened, in this
/// process. No child process is started and nothing is serialised to a line; what each request
/// is refused or answered with is what a child `ekr session` answers it with. Not exported.
pub(super) struct InProcess<'a> {
    session: Session,
    watch: Watch,
    now: &'a dyn Fn() -> Timestamp,
}

impl<'a> InProcess<'a> {
    /// A session over `runtime`, opened from `store`, answering with `now` as its clock.
    pub(super) fn new(store: Store, runtime: Runtime, now: &'a dyn Fn() -> Timestamp) -> Self {
        let opened = identity(&store.store);
        Self {
            session: Session {
                store,
                runtime: Some(runtime),
                create: false,
            },
            watch: Watch {
                held: true,
                opened,
                proposed: BTreeSet::new(),
                indexes: IndexCache::new(IndexCache::DEFAULT_CAPACITY),
            },
            now,
        }
    }

    /// Ends the session as a child session ends at the end of its input: the replay checkpoint of
    /// the head it reached, into the store it opened only.
    pub(super) fn close(self) {
        if let Some(runtime) = &self.session.runtime {
            if identity(&self.session.store.store) == self.watch.opened {
                runtime.retain_checkpoint_at_rest();
            }
        }
    }
}

impl ekr_sdk::transport::Transport for InProcess<'_> {
    fn request(
        &mut self,
        request: &ekr_sdk::transport::Request,
    ) -> Result<ekr_sdk::reply::Reply, ekr_sdk::transport::TransportError> {
        let request = Request {
            argv: request.argv.clone(),
            stdin: request.stdin.clone(),
        };
        let reply = |exit, document, stderr| ekr_sdk::reply::Reply {
            exit,
            document,
            stderr,
        };
        Ok(
            match answer(&request, &mut self.session, &mut self.watch, self.now) {
                Ok(Stdout::Document(Value::Null)) => reply(0, None, String::new()),
                Ok(Stdout::Document(document)) => reply(0, Some(document), String::new()),
                Ok(Stdout::Raw(document)) => {
                    reply(0, serde_json::from_str(document.get()).ok(), String::new())
                }
                Err(failure) => reply(i32::from(failure.code()), None, stderr(&failure)),
            },
        )
    }
}

mod views;

#[cfg(test)]
pub(super) mod fixture;

#[cfg(test)]
mod tests;
