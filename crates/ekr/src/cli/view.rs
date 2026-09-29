//! `ekr view`: a read-only viewer for an existing store, served on 127.0.0.1 only
//! (`story:ekr-view-server`).
//!
//! The server is a [`TcpListener`] bound to 127.0.0.1, with `httparse` reading request heads; it
//! is blocking, and no async runtime exists anywhere in the process. An accept thread hands each
//! connection to a short-lived thread of its own, at most 64 in flight; one more is answered 503
//! `busy` at once and closed, unread. The connection's deadline is fixed at accept: its whole head
//! must arrive within 5 s of it, every read waits only for what is left, and a head not complete
//! by then is 400. The thread reads at most 16 KiB of request head and parses it; it never reads a
//! body. It sends what it parsed over a channel to the one thread that opened the [`Runtime`],
//! gets the answer back — a whole response, or the [`SlicePage`] of an `/expand` — writes it with
//! `Connection: close` (each write waiting at most 5 s), and closes. So every store call runs on
//! that one thread, outside any Tokio context, as
//! `architecture-decision-record:0006-ekr-store-bridges-the-async-port` requires. A stream is
//! written by its own connection's thread, never by the store thread, so a slow reader of one
//! holds its own place and nobody else's; the whole stream is written within [`STREAM_TIMEOUT`]
//! or abandoned, and a client that closes the connection ends it at the next write.
//!
//! How long one connection holds one of the 64 places: a head not complete 5 s after accept is
//! refused; the wait for the store thread, which answers one request at a time, has no bound of
//! its own; a whole answer's writes each wait at most 5 s, so a client that reads nothing frees
//! the place 5 s after the first write it does not take; and a stream holds its place for up to
//! 65 s — the 5 s head deadline plus the 60 s [`STREAM_TIMEOUT`] — beyond that wait for the store
//! thread, since a client reading a byte every few seconds keeps every write inside its 5 s.
//!
//! Before any store call a request is refused unless it carries exactly one `Host` header naming
//! this server, `127.0.0.1:<port>` or `localhost:<port>` — on port 80 also `127.0.0.1` or
//! `localhost` alone, as a browser sends it — (421 otherwise, so a page reached through DNS
//! rebinding is served nothing), and unless it announces no body: any `Content-Length` above zero
//! or any `Transfer-Encoding` is 413, answered without reading the body.
//!
//! Nothing here proposes, validates, commits or seeds: the only store calls are
//! [`Runtime::head`], [`IndexCache::index`] (which loads through [`ekr_views::load`]),
//! [`ekr_views::Index::changes`] (which reads [`Runtime::head`], [`Runtime::transactions`] and
//! [`Runtime::replay`]), [`Runtime::snapshot`] and [`Runtime::content`]. A local read-only page is
//! not an outward write (design § 82, § 83).
//!
//! A revision is loaded once: the first request of a revision, whichever path, loads and indexes
//! it through one [`IndexCache`] of [`IndexCache::DEFAULT_CAPACITY`] revisions, keyed by the
//! revision, and every endpoint of that revision answers from that index. `/projection` and
//! `/roles` render from the index's loaded revision and also keep their rendered answers
//! ([`Cache`]). A committed revision never changes and no answer names the head
//! (`task:historical-projection-carries-the-head`), so a commit empties neither; the head is read
//! on every request, so a request naming no revision reads the newest. At most [`CACHE_LIMIT`]
//! revisions are kept in [`Cache`]; the one used longest ago goes first.
//!
//! | request | answer |
//! |---|---|
//! | `GET /` | the embedded viewer page, `text/html; charset=utf-8` |
//! | `GET /alt` | the embedded earlier viewer page, kept while the new one is accepted, `text/html; charset=utf-8` |
//! | `GET /head` | `{"format":"ekr.view-head/1","head":N}`, the store's newest committed revision read at the request, `application/json`; no document names it, and both pages read it here. Any query is 400 `invalid-query`, an unseeded store 404 `ekr.views.NotSeeded` |
//! | `GET /projection` | the `ekr.graph-projection/1` bytes `ekr-views` renders at the head, `application/json` |
//! | `GET /projection?revision=N` | the same at revision `N`; an absent revision is 404 `ekr.views.RevisionNotFound` |
//! | `GET /roles[?revision=N]` | the `ekr.view-roles/1` node-type roles of that revision (the head when absent), derived by [`super::view_roles`] from the same loaded revision the projection renders, `application/json`; refused as `/projection` refuses |
//! | `GET /evidence/<evidence id>` | that evidence's retained bytes, `text/plain; charset=utf-8` when they are UTF-8, else `application/octet-stream`; 404 for an unknown id or bytes not retained |
//! | `GET /overview[?revision=N&limit=L]` | [`ekr_views::Index::overview`]'s `ekr.graph-overview/1` bytes, `application/json` |
//! | `GET /expand?seeds=<id>,<id>&depth=D&limit=L[&edges=E][&after=A][&revision=N]` | [`ekr_views::Index::page`]'s records, streamed by [`write_stream`] as `application/x-ndjson`: chunked to an HTTP/1.1 request; to an HTTP/1.0 request, which may not be sent `Transfer-Encoding` (RFC 9112 § 6.1), unframed and ended by the close. `seeds=` is the empty set, answered with an empty page |
//! | `GET /node/<node id>[?revision=N]` | [`ekr_views::Index::describe`]'s `ekr.node-detail/1` bytes, `application/json` |
//! | `GET /search?q=<text>[&limit=L][&revision=N]` | [`ekr_views::Index::search`]'s `ekr.node-matches/1` bytes (`L` is [`SEARCH_LIMIT`] when absent), `application/json` |
//! | `GET /timeline?[type=<id>&]hops=H&limit=L[&bucket=B][&subject=<id>][&revision=N]` | [`ekr_views::Index::timeline`]'s `ekr.graph-timeline/1` bytes: one row per subject of the row type with its events within `H` hops (1 to 3; at most `L` rows, 1 to 500; `B` the finest bucket, `day` or `week`), or the named subject's row and events, `application/json` |
//! | `GET /changes?since_revision=N\|since_valid=T\|since_recorded=T[&at=N][&limit=L][&after=A]` | [`ekr_views::Index::changes`]'s `ekr.graph-changes/1` bytes: the changes after the revision `N`, the valid time `T` or the transaction time `T`, up to revision `at` (the head when absent), at most `L` (1 to 2,000, 500 when absent) from cursor `A`, `application/json`; exactly one of the three since names, else 400 `invalid-query` |
//!
//! Those six read their query with [`Query`]: `name=value` pairs, each name one the path takes
//! and at most once, each value percent-decoded; anything else is 400 `invalid-query`. Then, in
//! the order views.yaml gives: a revision since below 0 is 400 `ekr.views.SinceMalformed` and a
//! bound out of range 400 `ekr.views.LimitExceeded`, before any store call; an unseeded store 404
//! `ekr.views.NotSeeded`; an absent revision 404 `ekr.views.RevisionNotFound`; an unknown node or
//! seed — or a `/node/<id>` whose id is no node id — 404 `ekr.views.NodeNotFound`. Each is a
//! whole JSON refusal, decided before the first byte of an answer.
//! | any other method on those paths | 405, with `Allow: GET` |
//! | any other path | 404 |
//! | a `GET` of those paths that announces a body | 413 |
//! | any request whose `Host` is not this server's | 421 |
//! | a head that does not parse, or is not complete within 16 KiB or 5 s of accept | 400 |
//! | a connection while 64 are in flight | 503 `busy`, unread |
//!
//! Every response carries `X-Content-Type-Options: nosniff`, `Cache-Control: no-store` and
//! `Connection: close`, and no cookie or CORS header. The page also carries its
//! Content-Security-Policy as a header, with `frame-ancestors 'none'` added. Record text is untrusted evidence (A14) and
//! is never served as HTML: the only HTML is the page, which is embedded in the binary at build
//! time and writes what it fetches through `textContent`.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ekr_core::{EvidenceId, NodeId, RevisionNumber, TypeId};
use ekr_kernel::Runtime;
use ekr_views::{
    BucketWidth, ChangesError, ChangesRequest, ExpandRequest, IndexCache, LimitExceeded,
    OverviewRequest, ProjectError, QueryError, SearchRequest, SinceKind, SliceEdge, SliceMeta,
    SliceNode, SlicePage, SliceRecord, TimelineRequest,
};
use serde::Serialize;

use crate::exit::Failure;

/// The viewer page, embedded at build time; nothing is read from disk at run time.
const PAGE: &str = include_str!("viewer/index.html");
/// The earlier viewer page, served at `/alt` under the same policy.
const ALT_PAGE: &str = include_str!("viewer/alt.html");
/// The most revisions whose answers are kept in memory at once.
const CACHE_LIMIT: usize = 8;
/// The page's Content-Security-Policy as its `<meta>` carries it; the header adds
/// [`FRAME_POLICY`], which a `<meta>` policy cannot express.
const PAGE_POLICY: &str = "default-src 'none'; script-src 'unsafe-inline' https://cdn.jsdelivr.net/npm/graphology@0.26.0/dist/graphology.umd.min.js https://cdn.jsdelivr.net/npm/graphology-library@0.8.0/dist/graphology-library.min.js https://cdn.jsdelivr.net/npm/sigma@3.0.3/dist/sigma.min.js https://cdn.jsdelivr.net/npm/3d-force-graph@1.80.0/dist/3d-force-graph.min.js; worker-src blob:; style-src 'unsafe-inline'; connect-src 'self'";
/// Refuses framing, so another page cannot overlay the viewer.
const FRAME_POLICY: &str = "; frame-ancestors 'none'";

const HTML: &str = "text/html; charset=utf-8";
const JSON: &str = "application/json";
const TEXT: &str = "text/plain; charset=utf-8";
const BYTES: &str = "application/octet-stream";
const NDJSON: &str = "application/x-ndjson";

/// The most request-head bytes a connection reads.
const HEAD_LIMIT: usize = 16 * 1024;
/// The most header lines a request head may carry.
const HEADER_LIMIT: usize = 64;
/// How long after accept a connection has to send its whole head, and how long a write waits.
const TIMEOUT: Duration = Duration::from_secs(5);
/// How long a whole `/expand` stream may take to write before it is abandoned.
const STREAM_TIMEOUT: Duration = Duration::from_secs(60);
/// The most connections in flight at once; one more is answered 503 without being read.
const IN_FLIGHT_LIMIT: usize = 64;
/// A stream writes a `progress` line, and ends its chunk, after every this many records.
const PROGRESS_EVERY: usize = 256;
/// The `/search` limit when the query names none.
pub(super) const SEARCH_LIMIT: i64 = 20;

/// One parsed request and where its answer goes: from a connection thread to the store thread.
type Job = (Asked, Sender<Answered>);

/// What the store thread hands back to a connection: a whole response, or the page an
/// `/expand` streams, which the connection writes itself.
#[derive(Debug)]
enum Answered {
    Whole(Reply),
    Stream(Box<SlicePage>),
}

/// Everything the store thread keeps between requests.
#[derive(Debug)]
struct Memory {
    /// The index every endpoint of a revision answers from.
    indexes: IndexCache,
    /// `/projection` and `/roles`, rendered.
    rendered: Cache,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            indexes: IndexCache::new(IndexCache::DEFAULT_CAPACITY),
            rendered: Cache::default(),
        }
    }
}

/// Binds 127.0.0.1 on `port` (0 picks a free one), prints `{"url": …}` as one JSON line on
/// stdout, and answers requests until the process is interrupted.
///
/// # Errors
///
/// The address does not bind, stdout cannot be written, or the accept loop stops.
pub(super) fn run(runtime: &Runtime, port: u16) -> Result<String, Failure> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|error| Failure::fault(format!("binding 127.0.0.1:{port}: {error}")))?;
    let address = listener
        .local_addr()
        .map_err(|error| Failure::fault(format!("reading the bound address: {error}")))?;
    let line = serde_json::json!({ "url": format!("http://{address}/") });
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{line}")
        .and_then(|()| stdout.flush())
        .map_err(|error| Failure::fault(format!("writing the URL: {error}")))?;
    drop(stdout);

    let (jobs, store_thread) = channel::<Job>();
    std::thread::Builder::new()
        .name("ekr-view-accept".to_owned())
        .spawn(move || accept(&listener, &jobs))
        .map_err(|error| Failure::fault(format!("starting the accept thread: {error}")))?;
    let port = address.port();
    let mut memory = Memory::default();
    for (asked, reply_to) in store_thread {
        // A connection that timed out meanwhile is its own business.
        let _ = reply_to.send(answer(runtime, &mut memory, port, &asked));
    }
    Err(Failure::fault("the accept loop stopped"))
}

/// One connection in flight: counted while it lives, released when its thread ends, however it
/// ends.
struct InFlight(Arc<AtomicUsize>);

impl InFlight {
    /// Counts one more connection, or `None` when [`IN_FLIGHT_LIMIT`] are already in flight.
    fn admit(count: &Arc<AtomicUsize>) -> Option<Self> {
        let admitted = count
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |now| {
                (now < IN_FLIGHT_LIMIT).then_some(now + 1)
            })
            .is_ok();
        admitted.then(|| Self(Arc::clone(count)))
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Hands every accepted connection to a short-lived thread of its own, at most
/// [`IN_FLIGHT_LIMIT`] at a time; one over the cap is answered 503 at once and closed, unread.
fn accept(listener: &TcpListener, jobs: &Sender<Job>) {
    let in_flight = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let deadline = Instant::now() + TIMEOUT;
                let Some(counted) = InFlight::admit(&in_flight) else {
                    let busy = Reply::text(
                        503,
                        format!("busy: {IN_FLIGHT_LIMIT} connections are in flight"),
                    );
                    // A client that does not take 200 bytes within the second is its own business.
                    let _ = stream
                        .set_write_timeout(Some(Duration::from_secs(1)))
                        .and_then(|()| stream.write_all(&busy.into_bytes()))
                        .and_then(|()| stream.shutdown(Shutdown::Write));
                    continue;
                };
                let jobs = jobs.clone();
                // A thread that cannot start drops the connection and the count with it.
                let _ = std::thread::Builder::new()
                    .name("ekr-view-connection".to_owned())
                    .spawn(move || {
                        let _counted = counted;
                        connection(stream, deadline, &jobs);
                    });
            }
            // Out of descriptors or a connection reset before accept: take the next one.
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
}

/// One connection: read and parse the head by `deadline`, get the answer from the store thread,
/// write it — a stream from this thread, with the store thread already free — close. The body,
/// if any, is never read.
fn connection(stream: TcpStream, deadline: Instant, jobs: &Sender<Job>) {
    if stream.set_write_timeout(Some(TIMEOUT)).is_err() {
        return;
    }
    let mut stream = stream;
    let head = {
        let mut reader = &stream;
        read_head(&mut reader, || {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(late());
            }
            stream
                .set_read_timeout(Some(left))
                .map_err(|error| format!("setting the read timeout: {error}"))
        })
    };
    let mut framing = Framing::Chunked;
    let answered = match head {
        Ok(asked) => {
            framing = asked.framing;
            let (reply_to, reply) = channel();
            if jobs.send((asked, reply_to)).is_err() {
                return;
            }
            match reply.recv() {
                Ok(answered) => answered,
                Err(_) => return,
            }
        }
        Err(message) => Answered::Whole(Reply::text(400, format!("bad-request: {message}"))),
    };
    // A client that went away is its own business: the first failed write ends the answer.
    let _ = match answered {
        Answered::Whole(reply) => stream
            .write_all(&reply.into_bytes())
            .and_then(|()| stream.flush()),
        Answered::Stream(page) => write_stream(
            &mut Deadlined {
                stream: &stream,
                deadline: Instant::now() + STREAM_TIMEOUT,
            },
            &page,
            framing,
        ),
    };
    let _ = stream.shutdown(Shutdown::Write);
}

/// A connection written to within a deadline: each write waits at most [`TIMEOUT`] and never past
/// `deadline`, and once it has passed every write fails.
struct Deadlined<'a> {
    stream: &'a TcpStream,
    deadline: Instant,
}

impl Write for Deadlined<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let left = self.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        self.stream.set_write_timeout(Some(left.min(TIMEOUT)))?;
        let mut stream = self.stream;
        stream.write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut stream = self.stream;
        stream.flush()
    }
}

/// One NDJSON line of a stream: `{"kind": <kind>, …the fields of body}`.
#[derive(Serialize)]
struct Line<'a, T: Serialize> {
    kind: &'static str,
    #[serde(flatten)]
    body: &'a T,
}

#[derive(Serialize)]
struct Progress {
    sent: usize,
}

#[derive(Serialize)]
struct End {
    next: Option<u64>,
    remaining: u64,
}

/// How a stream's body is delimited, chosen by the version the request names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Framing {
    /// HTTP/1.1 chunks, ended by the last, empty chunk: the answer to an HTTP/1.1 request.
    Chunked,
    /// The bare lines, ended by closing the connection: the answer to an HTTP/1.0 request, which
    /// may not be sent `Transfer-Encoding` (RFC 9112 § 6.1).
    Close,
}

/// Writes `page` as the `/expand` answer: the response head, then the NDJSON in pieces, each
/// flushed as it is written — the meta line alone, then the records with a `progress` line
/// ending the piece after every [`PROGRESS_EVERY`], then the rest and the `end` line. Under
/// [`Framing::Chunked`] each piece is one HTTP/1.1 chunk and the last, empty chunk follows; under
/// [`Framing::Close`] the pieces are bare and the caller's close ends the body. Stops at the
/// first write that fails.
fn write_stream(out: &mut impl Write, page: &SlicePage, framing: Framing) -> std::io::Result<()> {
    write_lines(
        out,
        framing,
        page.meta(),
        page.records(),
        page.next(),
        page.remaining(),
    )
}

/// [`write_stream`] over a page's parts.
fn write_lines(
    out: &mut impl Write,
    framing: Framing,
    meta: &SliceMeta,
    records: &[SliceRecord],
    next: Option<u64>,
    remaining: u64,
) -> std::io::Result<()> {
    out.write_all(stream_head(framing).as_bytes())?;
    let mut chunk = Vec::with_capacity(64 * 1024);
    line(&mut chunk, "meta", meta)?;
    send_chunk(out, &mut chunk, framing)?;
    for (at, record) in records.iter().enumerate() {
        match record {
            SliceRecord::Node(node) => line::<SliceNode>(&mut chunk, "node", node)?,
            SliceRecord::Edge(edge) => line::<SliceEdge>(&mut chunk, "edge", edge)?,
        }
        let sent = at + 1;
        if sent % PROGRESS_EVERY == 0 {
            line(&mut chunk, "progress", &Progress { sent })?;
            send_chunk(out, &mut chunk, framing)?;
        }
    }
    line(&mut chunk, "end", &End { next, remaining })?;
    send_chunk(out, &mut chunk, framing)?;
    if framing == Framing::Chunked {
        out.write_all(b"0\r\n\r\n")?;
    }
    out.flush()
}

/// The head of a streamed answer: the headers every answer carries, no `Content-Length`, and
/// `Transfer-Encoding: chunked` only under [`Framing::Chunked`].
fn stream_head(framing: Framing) -> String {
    let encoding = match framing {
        Framing::Chunked => "Transfer-Encoding: chunked\r\n",
        Framing::Close => "",
    };
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {NDJSON}\r\n{encoding}\
         X-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n"
    )
}

/// Appends `{"kind": kind, …body}` and a newline to `chunk`.
fn line<T: Serialize>(chunk: &mut Vec<u8>, kind: &'static str, body: &T) -> std::io::Result<()> {
    serde_json::to_writer(&mut *chunk, &Line { kind, body }).map_err(std::io::Error::other)?;
    chunk.push(b'\n');
    Ok(())
}

/// Writes `chunk` — as one HTTP/1.1 chunk under [`Framing::Chunked`], bare under
/// [`Framing::Close`] — and flushes it, then empties it.
fn send_chunk(out: &mut impl Write, chunk: &mut Vec<u8>, framing: Framing) -> std::io::Result<()> {
    if chunk.is_empty() {
        return Ok(());
    }
    if framing == Framing::Chunked {
        out.write_all(format!("{:x}\r\n", chunk.len()).as_bytes())?;
        chunk.extend_from_slice(b"\r\n");
    }
    out.write_all(chunk)?;
    chunk.clear();
    out.flush()
}

/// Why a head that did not arrive in time is refused.
fn late() -> String {
    format!(
        "the request head was not complete within {} s of the connection",
        TIMEOUT.as_secs()
    )
}

/// Reads one request head, at most [`HEAD_LIMIT`] bytes of it, and what [`answer`] needs of it.
/// `before_read` runs before every read: it gives the read what is left of the connection's
/// deadline as its timeout, or refuses once the deadline has passed.
fn read_head(
    stream: &mut impl Read,
    mut before_read: impl FnMut() -> Result<(), String>,
) -> Result<Asked, String> {
    let mut head = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 4096];
    loop {
        before_read()?;
        let room = (HEAD_LIMIT - head.len()).min(chunk.len());
        let read = match stream.read(&mut chunk[..room]) {
            Ok(0) => return Err("the connection closed before the request head ended".to_owned()),
            Ok(read) => read,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Err(late())
            }
            Err(error) => return Err(format!("reading the request head: {error}")),
        };
        head.extend_from_slice(&chunk[..read]);
        if let Some(asked) = parse_head(&head)? {
            return Ok(asked);
        }
        if head.len() >= HEAD_LIMIT {
            return Err(format!("the request head is over {HEAD_LIMIT} bytes"));
        }
    }
}

/// Parses `head`: `None` while it is incomplete.
fn parse_head(head: &[u8]) -> Result<Option<Asked>, String> {
    let mut headers = [httparse::EMPTY_HEADER; HEADER_LIMIT];
    let mut request = httparse::Request::new(&mut headers);
    match request.parse(head) {
        Ok(httparse::Status::Partial) => Ok(None),
        Err(error) => Err(format!("the request head does not parse: {error}")),
        Ok(httparse::Status::Complete(_)) => {
            let named = |name: &str| {
                request
                    .headers
                    .iter()
                    .filter(|header| header.name.eq_ignore_ascii_case(name))
                    .map(|header| header.value)
                    .collect::<Vec<&[u8]>>()
            };
            let announces_body = named("Content-Length")
                .iter()
                .any(|value| value.trim_ascii() != b"0")
                || !named("Transfer-Encoding").is_empty();
            // A Host that is not UTF-8 names no server, and keeps its place so it still counts.
            let hosts = named("Host")
                .iter()
                .map(|value| String::from_utf8_lossy(value).into_owned())
                .collect();
            // httparse admits only HTTP/1.0 (version 0) and HTTP/1.1 (version 1).
            let framing = if request.version == Some(1) {
                Framing::Chunked
            } else {
                Framing::Close
            };
            Ok(Some(Asked {
                method: request.method.unwrap_or_default().to_owned(),
                target: request.path.unwrap_or_default().to_owned(),
                hosts,
                announces_body,
                framing,
            }))
        }
    }
}

/// One answer, before it becomes response bytes.
#[derive(Debug, PartialEq, Eq)]
struct Reply {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Reply {
    fn ok(content_type: &'static str, body: Vec<u8>) -> Self {
        Self {
            status: 200,
            content_type,
            body,
        }
    }

    fn text(status: u16, message: impl Into<String>) -> Self {
        let mut body = message.into().into_bytes();
        body.push(b'\n');
        Self {
            status,
            content_type: TEXT,
            body,
        }
    }

    /// A named refusal as JSON: `{"refusal": <name>, "message": <reason>}`.
    fn refusal(status: u16, name: &str, message: impl std::fmt::Display) -> Self {
        let body = serde_json::json!({ "refusal": name, "message": message.to_string() });
        Self {
            status,
            content_type: JSON,
            body: body.to_string().into_bytes(),
        }
    }

    /// The whole HTTP/1.1 response: status line, headers, body.
    fn into_bytes(self) -> Vec<u8> {
        let reason = match self.status {
            200 => "OK",
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            413 => "Content Too Large",
            503 => "Service Unavailable",
            421 => "Misdirected Request",
            _ => "Internal Server Error",
        };
        let allow = if self.status == 405 {
            "Allow: GET\r\n"
        } else {
            ""
        };
        let policy = if self.content_type == HTML {
            format!("Content-Security-Policy: {PAGE_POLICY}{FRAME_POLICY}\r\n")
        } else {
            String::new()
        };
        let mut bytes = format!(
            "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
             X-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\n{allow}{policy}\
             Connection: close\r\n\r\n",
            self.status,
            self.content_type,
            self.body.len()
        )
        .into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

/// The paths this server knows.
enum Route<'a> {
    Page,
    AltPage,
    Head,
    Projection,
    Roles,
    Evidence(&'a str),
    Overview,
    Expand,
    Node(&'a str),
    Search,
    Timeline,
    Changes,
}

fn route(path: &str) -> Option<Route<'_>> {
    let named = |prefix: &str| {
        path.strip_prefix(prefix)
            .filter(|id| !id.is_empty() && !id.contains('/'))
    };
    match path {
        "/" => Some(Route::Page),
        "/alt" => Some(Route::AltPage),
        "/head" => Some(Route::Head),
        "/projection" => Some(Route::Projection),
        "/roles" => Some(Route::Roles),
        "/overview" => Some(Route::Overview),
        "/expand" => Some(Route::Expand),
        "/search" => Some(Route::Search),
        "/timeline" => Some(Route::Timeline),
        "/changes" => Some(Route::Changes),
        _ => named("/evidence/")
            .map(Route::Evidence)
            .or_else(|| named("/node/").map(Route::Node)),
    }
}

/// What [`answer`] reads of one request: nothing of its body but whether it announces one.
#[derive(Debug, PartialEq, Eq)]
struct Asked {
    method: String,
    /// The request target: path and query.
    target: String,
    /// Every `Host` header value the request carries.
    hosts: Vec<String>,
    /// A `Content-Length` other than zero or any `Transfer-Encoding`.
    announces_body: bool,
    /// How a stream answering this request is delimited: chunked for HTTP/1.1, ended by the close
    /// for HTTP/1.0.
    framing: Framing,
}

/// Whether `hosts` is exactly one `Host`, naming this server's own loopback authority. A page
/// reached through DNS rebinding sends its own name, and is served nothing. On port 80 a browser
/// leaves the port out, so there `127.0.0.1` and `localhost` alone are the server's own too.
fn own_host(hosts: &[String], port: u16) -> bool {
    let [host] = hosts else {
        return false;
    };
    let (name, given) = match host.rsplit_once(':') {
        Some((name, given)) => (name, Some(given)),
        None => (host.as_str(), None),
    };
    let names_port = match given {
        Some(given) => given == port.to_string(),
        None => port == 80,
    };
    matches!(name, "127.0.0.1" | "localhost") && names_port
}

/// Answers one request, and touches the store only for a `GET` of a known path that names this
/// server as its `Host` and announces no body: another `Host` (or none) is 421, an unknown path
/// 404 whatever the method, a known path 405 for any method but `GET`, and a body 413.
fn answer(runtime: &Runtime, memory: &mut Memory, port: u16, asked: &Asked) -> Answered {
    if !own_host(&asked.hosts, port) {
        return Answered::Whole(Reply::text(
            421,
            format!("misdirected-request: Host must be 127.0.0.1:{port} or localhost:{port}"),
        ));
    }
    let target = asked.target.as_str();
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let Some(route) = route(path) else {
        return Answered::Whole(Reply::text(404, format!("not-found: {path}")));
    };
    if asked.method != "GET" {
        return Answered::Whole(Reply::text(
            405,
            format!("method-not-allowed: {} {path}; only GET", asked.method),
        ));
    }
    if asked.announces_body {
        return Answered::Whole(Reply::text(
            413,
            "request-body-refused: a GET carries no body",
        ));
    }
    let reply = match route {
        Route::Page => Reply::ok(HTML, PAGE.as_bytes().to_vec()),
        Route::AltPage => Reply::ok(HTML, ALT_PAGE.as_bytes().to_vec()),
        Route::Head => head(runtime, query),
        Route::Projection => rendered(runtime, memory, query, "projection", |r| &r.projection),
        Route::Roles => rendered(runtime, memory, query, "roles", |r| &r.roles),
        Route::Evidence(id) => evidence(runtime, id),
        Route::Overview => overview(runtime, &mut memory.indexes, query).unwrap_or_else(|r| r),
        Route::Node(id) => node(runtime, &mut memory.indexes, id, query).unwrap_or_else(|r| r),
        Route::Search => search(runtime, &mut memory.indexes, query).unwrap_or_else(|r| r),
        Route::Timeline => timeline(runtime, &mut memory.indexes, query).unwrap_or_else(|r| r),
        Route::Changes => changes(runtime, &mut memory.indexes, query).unwrap_or_else(|r| r),
        Route::Expand => {
            return match expand(runtime, &mut memory.indexes, query) {
                Ok(page) => Answered::Stream(Box::new(page)),
                Err(refused) => Answered::Whole(refused),
            }
        }
    };
    Answered::Whole(reply)
}

/// The query of `/overview`, `/expand`, `/node/<id>` and `/search`: `name=value` pairs joined by
/// `&`, each name one the path takes and given at most once, each value percent-decoded to UTF-8
/// with `+` read as a space. An empty query has no pairs; an empty pair, a pair without `=`, an
/// unknown or repeated name, a `%` not followed by two hex digits, or bytes that are not UTF-8
/// are refused.
#[derive(Debug, PartialEq, Eq)]
struct Query {
    pairs: Vec<(&'static str, String)>,
}

impl Query {
    fn parse(query: &str, known: &[&'static str]) -> Result<Self, String> {
        let mut pairs: Vec<(&'static str, String)> = Vec::new();
        if query.is_empty() {
            return Ok(Self { pairs });
        }
        for pair in query.split('&') {
            let Some((name, value)) = pair.split_once('=') else {
                return Err(format!("the query pair {pair:?} is not `name=value`"));
            };
            let name = decode(name)?;
            let Some(known) = known.iter().find(|known| **known == name) else {
                return Err(format!(
                    "the query names {name:?}; this path takes {}",
                    known.join(", ")
                ));
            };
            if pairs.iter().any(|(held, _)| held == known) {
                return Err(format!("the query names {name:?} twice"));
            }
            pairs.push((known, decode(value)?));
        }
        Ok(Self { pairs })
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(held, _)| *held == name)
            .map(|(_, value)| value.as_str())
    }

    fn required(&self, name: &str) -> Result<&str, String> {
        self.get(name)
            .ok_or_else(|| format!("the query names no {name}"))
    }

    /// `name` as a decimal integer: an optional `-` and one or more ASCII digits that fit an
    /// `i64`. The range is the engine's to refuse, as `ekr.views.LimitExceeded`.
    fn integer(&self, name: &str) -> Result<Option<i64>, String> {
        let Some(value) = self.get(name) else {
            return Ok(None);
        };
        let digits = value.strip_prefix('-').unwrap_or(value);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(format!("{name} {value:?} is not a decimal integer"));
        }
        value
            .parse()
            .map(Some)
            .map_err(|_| format!("{name} {value:?} is out of range"))
    }

    /// `revision` as ASCII decimal digits that fit a `u64`, as `/projection` reads it.
    fn revision(&self) -> Result<Option<RevisionNumber>, String> {
        let Some(value) = self.get("revision") else {
            return Ok(None);
        };
        revision(&format!("revision={value}"))
    }
}

/// Percent-decodes `text` to UTF-8, reading `+` as a space.
fn decode(text: &str) -> Result<String, String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.as_bytes();
    while let Some((first, tail)) = rest.split_first() {
        match first {
            b'%' => {
                let hex = tail
                    .get(..2)
                    .and_then(|hex| std::str::from_utf8(hex).ok())
                    .filter(|hex| hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
                    .ok_or_else(|| format!("{text:?} has a `%` not followed by two hex digits"))?;
                bytes.push(u8::from_str_radix(hex, 16).map_err(|error| error.to_string())?);
                rest = &tail[2..];
            }
            b'+' => {
                bytes.push(b' ');
                rest = tail;
            }
            other => {
                bytes.push(*other);
                rest = tail;
            }
        }
    }
    String::from_utf8(bytes).map_err(|_| format!("{text:?} does not decode to UTF-8"))
}

fn invalid_query(message: impl std::fmt::Display) -> Reply {
    Reply::refusal(400, "invalid-query", message)
}

/// `ekr.views.LimitExceeded`: a bound refused before the store is read. `ekr view` and `ekr mcp`
/// both name it from here.
pub(super) const LIMIT_EXCEEDED: &str = "ekr.views.LimitExceeded";
/// `ekr.views.NodeNotFound`: a node or seed the revision does not hold, or an id that is no node
/// id.
pub(super) const NODE_NOT_FOUND: &str = "ekr.views.NodeNotFound";

/// The `ekr.views` refusal a revision that could not be loaded names — `NotSeeded` or
/// `RevisionNotFound` — or `None` for a store that could not be read, which is a fault.
pub(super) fn project_refusal(error: &ProjectError) -> Option<&'static str> {
    match error {
        ProjectError::RevisionNotFound { .. } => Some("ekr.views.RevisionNotFound"),
        ProjectError::NotSeeded { .. } => Some("ekr.views.NotSeeded"),
        ProjectError::Read(_) | ProjectError::Inconsistent(_) => None,
    }
}

fn limit_exceeded(error: &LimitExceeded) -> Reply {
    Reply::refusal(400, LIMIT_EXCEEDED, error)
}

/// A bounded read that answered nothing: the named 400s and 404s, or a 500 for `what`.
fn query_refused(what: &str, error: QueryError) -> Reply {
    match error {
        QueryError::LimitExceeded(error) => limit_exceeded(&error),
        error @ QueryError::NodeNotFound { .. } => Reply::refusal(404, NODE_NOT_FOUND, error),
        QueryError::Project(error) => refused(what, error),
    }
}

/// `/overview[?revision=N&limit=L]`.
fn overview(runtime: &Runtime, indexes: &mut IndexCache, query: &str) -> Result<Reply, Reply> {
    let query = Query::parse(query, &["revision", "limit"]).map_err(invalid_query)?;
    let at = query.revision().map_err(invalid_query)?;
    let limit = query.integer("limit").map_err(invalid_query)?;
    let request = OverviewRequest::new(limit).map_err(|error| limit_exceeded(&error))?;
    let index = indexes
        .index(runtime, at)
        .map_err(|error| refused("overview", error))?;
    let answer = index
        .overview(&request)
        .map_err(|error| refused("overview", error))?;
    Ok(Reply::ok(JSON, answer.bytes))
}

/// `/node/<id>[?revision=N]`: an id that is not a node id names no node, so it is 404
/// `ekr.views.NodeNotFound` too — refused where views.yaml orders that refusal, after the store
/// is read and found seeded and holding the revision.
fn node(
    runtime: &Runtime,
    indexes: &mut IndexCache,
    id: &str,
    query: &str,
) -> Result<Reply, Reply> {
    let query = Query::parse(query, &["revision"]).map_err(invalid_query)?;
    let at = query.revision().map_err(invalid_query)?;
    let index = indexes
        .index(runtime, at)
        .map_err(|error| refused("node", error))?;
    let Ok(node) = id.parse::<NodeId>() else {
        return Err(Reply::refusal(404, NODE_NOT_FOUND, not_a_node_id(id)));
    };
    let answer = index
        .describe(node)
        .map_err(|error| query_refused("node", error))?;
    Ok(Reply::ok(JSON, answer.bytes))
}

/// Why an id that is no node id names no node: [`NODE_NOT_FOUND`]'s message for it.
pub(super) fn not_a_node_id(id: &str) -> String {
    format!("{id:?} is not a node id")
}

/// `/search?q=<text>[&limit=L][&revision=N]`.
fn search(runtime: &Runtime, indexes: &mut IndexCache, query: &str) -> Result<Reply, Reply> {
    let query = Query::parse(query, &["q", "limit", "revision"]).map_err(invalid_query)?;
    let text = query.required("q").map_err(invalid_query)?.to_owned();
    let limit = query.integer("limit").map_err(invalid_query)?;
    let at = query.revision().map_err(invalid_query)?;
    let request = SearchRequest::new(text, limit.unwrap_or(SEARCH_LIMIT))
        .map_err(|error| limit_exceeded(&error))?;
    let index = indexes
        .index(runtime, at)
        .map_err(|error| refused("search", error))?;
    let answer = index
        .search(&request)
        .map_err(|error| refused("search", error))?;
    Ok(Reply::ok(JSON, answer.bytes))
}

/// `/timeline?[type=<id>&]hops=H&limit=L[&bucket=day|week][&subject=<id>][&revision=N]`: the
/// `ekr.graph-timeline/1` document. `hops` and `limit` are required; a `type` or `subject` that is
/// not an id, or a `bucket` other than `day` or `week`, is `invalid-query`.
fn timeline(runtime: &Runtime, indexes: &mut IndexCache, query: &str) -> Result<Reply, Reply> {
    let query = Query::parse(
        query,
        &["type", "hops", "limit", "bucket", "subject", "revision"],
    )
    .map_err(invalid_query)?;
    let row_type = query
        .get("type")
        .map(|text| {
            text.parse::<TypeId>()
                .map_err(|_| invalid_query(format!("the type {text:?} is not a type id")))
        })
        .transpose()?;
    let subject = query
        .get("subject")
        .map(|text| {
            text.parse::<NodeId>()
                .map_err(|_| invalid_query(format!("the subject {text:?} is not a node id")))
        })
        .transpose()?;
    let bucket = match query.get("bucket") {
        None => None,
        Some("day") => Some(BucketWidth::Day),
        Some("week") => Some(BucketWidth::Week),
        Some(other) => {
            return Err(invalid_query(format!(
                "the bucket {other:?} is not `day` or `week`"
            )))
        }
    };
    let required = |name: &str| {
        query
            .integer(name)
            .and_then(|value| value.ok_or_else(|| format!("the query names no {name}")))
            .map_err(invalid_query)
    };
    let hops = required("hops")?;
    let limit = required("limit")?;
    let at = query.revision().map_err(invalid_query)?;
    let request = TimelineRequest::new(row_type, hops, limit, bucket, subject)
        .map_err(|error| limit_exceeded(&error))?;
    let index = indexes
        .index(runtime, at)
        .map_err(|error| refused("timeline", error))?;
    let answer = index
        .timeline(&request)
        .map_err(|error| refused("timeline", error))?;
    Ok(Reply::ok(JSON, answer.bytes))
}

/// `ekr.views.SinceMalformed`: a revision since below 0. `ekr view` and `ekr mcp` both name it
/// from here.
pub(super) const SINCE_MALFORMED: &str = "ekr.views.SinceMalformed";

/// Why a `since` given as `since_revision`, `since_valid` and `since_recorded` is not exactly one
/// of them, from the names [`SinceKind::one_of`] answers: `ekr view`'s `invalid-query` message and
/// `ekr mcp`'s invalid-params message.
pub(super) fn since_not_one(given: &[&str]) -> String {
    if given.is_empty() {
        "no since is given; give exactly one of since_revision, since_valid and since_recorded"
            .to_owned()
    } else {
        format!(
            "{} are given; give exactly one of since_revision, since_valid and since_recorded",
            given.join(" and ")
        )
    }
}

/// A `ChangesSince` that answered nothing: 400 `ekr.views.SinceMalformed` or
/// `ekr.views.LimitExceeded`, or the 404s and 500 a revision that could not be loaded answers.
fn changes_refused(error: ChangesError) -> Reply {
    match error {
        ChangesError::SinceMalformed(error) => Reply::refusal(400, SINCE_MALFORMED, error),
        ChangesError::LimitExceeded(error) => limit_exceeded(&error),
        ChangesError::Project(error) => refused("changes", error),
    }
}

/// `/changes?since_revision=N|since_valid=T|since_recorded=T[&at=N][&limit=L][&after=A]`: the
/// `ekr.graph-changes/1` page. Exactly one of the three since names is required (`invalid-query`
/// otherwise); `at` is a revision as `revision` is elsewhere, the head when absent.
fn changes(runtime: &Runtime, indexes: &mut IndexCache, query: &str) -> Result<Reply, Reply> {
    let query = Query::parse(
        query,
        &[
            "since_revision",
            "since_valid",
            "since_recorded",
            "at",
            "limit",
            "after",
        ],
    )
    .map_err(invalid_query)?;
    let integer = |name: &str| query.integer(name).map_err(invalid_query);
    let (kind, since) = SinceKind::one_of(
        integer("since_revision")?,
        integer("since_valid")?,
        integer("since_recorded")?,
    )
    .map_err(|given| invalid_query(since_not_one(&given)))?;
    let limit = integer("limit")?;
    let after = integer("after")?;
    let at = query
        .get("at")
        .map(|value| revision(&format!("revision={value}")).map_err(invalid_query))
        .transpose()?
        .flatten();
    let request = ChangesRequest::new(kind, since, limit, after).map_err(changes_refused)?;
    let index = indexes
        .index(runtime, at)
        .map_err(|error| refused("changes", error))?;
    let answer = index.changes(runtime, &request).map_err(changes_refused)?;
    Ok(Reply::ok(JSON, answer.bytes))
}

/// `/expand?seeds=<id>,<id>&depth=D&limit=L[&edges=E][&after=A][&revision=N]`: the page the
/// connection streams, or the refusal it answers instead. `seeds=` is the empty set, which
/// views.yaml answers with an empty page (node_total 0); an empty id among others is refused.
fn expand(runtime: &Runtime, indexes: &mut IndexCache, query: &str) -> Result<SlicePage, Reply> {
    let query = Query::parse(
        query,
        &["seeds", "depth", "limit", "edges", "after", "revision"],
    )
    .map_err(invalid_query)?;
    let listed = query.required("seeds").map_err(invalid_query)?;
    let seeds = if listed.is_empty() {
        Vec::new()
    } else {
        listed
            .split(',')
            .map(|seed| {
                seed.parse::<NodeId>()
                    .map_err(|_| invalid_query(format!("the seed {seed:?} is not a node id")))
            })
            .collect::<Result<Vec<NodeId>, Reply>>()?
    };
    let required = |name: &str| {
        query
            .integer(name)
            .and_then(|value| value.ok_or_else(|| format!("the query names no {name}")))
            .map_err(invalid_query)
    };
    let depth = required("depth")?;
    let limit = required("limit")?;
    let edges = query.integer("edges").map_err(invalid_query)?;
    let after = query.integer("after").map_err(invalid_query)?;
    let at = query.revision().map_err(invalid_query)?;
    let request = ExpandRequest::new(seeds, depth, limit, edges, after)
        .map_err(|error| limit_exceeded(&error))?;
    let index = indexes
        .index(runtime, at)
        .map_err(|error| refused("expand", error))?;
    index
        .page(&request)
        .map_err(|error| query_refused("expand", error))
}

/// The revision a `/projection` or `/roles` query names: none for an empty query, else the query
/// is exactly `revision=N` with `N` one or more ASCII digits that fit a `u64`. Anything else —
/// an empty pair, a second pair, a sign, a space, another digit script — is refused.
fn revision(query: &str) -> Result<Option<RevisionNumber>, String> {
    if query.is_empty() {
        return Ok(None);
    }
    let Some(value) = query.strip_prefix("revision=") else {
        return Err(format!("the query {query:?} is not `revision=N`"));
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!(
            "the query {query:?} is not `revision=N` with N a decimal revision number"
        ));
    }
    let number = value
        .parse::<u64>()
        .map_err(|_| format!("revision {value:?} is not a revision number"))?;
    Ok(Some(RevisionNumber::new(number)))
}

/// Both answers of one loaded revision: the `ekr.graph-projection/1` bytes and the
/// `ekr.view-roles/1` body, rendered from the same [`ekr_views::load`].
#[derive(Debug, PartialEq, Eq)]
struct Answers {
    projection: Vec<u8>,
    roles: Vec<u8>,
}

/// The answers of the revisions loaded so far, the one used most recently last. A committed
/// revision never changes and neither answer names the head, so a new head keeps them.
#[derive(Debug, Default)]
struct Cache {
    entries: Vec<(RevisionNumber, Answers)>,
}

impl Cache {
    /// The answers of `revision`: from memory, else from `load`, which is kept when it succeeds
    /// and not when it fails. Beyond [`CACHE_LIMIT`] entries the one used longest ago goes.
    fn get_or_load<E>(
        &mut self,
        revision: RevisionNumber,
        load: impl FnOnce() -> Result<Answers, E>,
    ) -> Result<&Answers, E> {
        if let Some(at) = self.entries.iter().position(|(held, _)| *held == revision) {
            let entry = self.entries.remove(at);
            self.entries.push(entry);
        } else {
            let answers = load()?;
            if self.entries.len() >= CACHE_LIMIT {
                self.entries.remove(0);
            }
            self.entries.push((revision, answers));
        }
        Ok(&self
            .entries
            .last()
            .expect("an entry was just placed last")
            .1)
    }
}

/// The format literal of the `/head` body.
const HEAD_FORMAT: &str = "ekr.view-head/1";

/// `/head`: the store's newest committed revision, read on every request, as
/// `{"format":"ekr.view-head/1","head":N}`. No `ekr.views` document names the head, so this is
/// where a page learns which revisions it may offer. It takes no query (400 `invalid-query`); an
/// unseeded store is 404 `ekr.views.NotSeeded`.
fn head(runtime: &Runtime, query: &str) -> Reply {
    if !query.is_empty() {
        return invalid_query(format!("/head takes no query; it was given {query:?}"));
    }
    match runtime.head() {
        Ok(Some(root)) => {
            let body = serde_json::json!({ "format": HEAD_FORMAT, "head": root.revision.get() });
            Reply::ok(JSON, body.to_string().into_bytes())
        }
        Ok(None) => refused("head", ProjectError::NotSeeded { requested: None }),
        Err(error) => Reply::text(500, format!("head: reading the head: {error}")),
    }
}

/// `/projection` or `/roles` (`what`) of the revision the query names, the head when it names
/// none: `pick` chooses which of the revision's [`Answers`]. The head is read on every request;
/// the answers are rendered only when the cache does not hold them, from the index every other
/// endpoint of the revision reads. Refused as before: 400 for a query that is not `revision=N`,
/// 404 `ekr.views.NotSeeded` or `ekr.views.RevisionNotFound`.
fn rendered(
    runtime: &Runtime,
    memory: &mut Memory,
    query: &str,
    what: &str,
    pick: impl Fn(&Answers) -> &Vec<u8>,
) -> Reply {
    let at = match revision(query) {
        Ok(at) => at,
        Err(message) => return Reply::refusal(400, "invalid-query", message),
    };
    let head = match runtime.head() {
        Ok(Some(root)) => root.revision,
        Ok(None) => return refused(what, ProjectError::NotSeeded { requested: at }),
        Err(error) => return Reply::text(500, format!("{what}: reading the head: {error}")),
    };
    let wanted = at.unwrap_or(head);
    if wanted > head {
        return refused(
            what,
            ProjectError::RevisionNotFound {
                requested: wanted,
                head,
            },
        );
    }
    let Memory { indexes, rendered } = memory;
    match rendered.get_or_load(wanted, || load_answers(runtime, indexes, wanted)) {
        Ok(answers) => Reply::ok(JSON, pick(answers).clone()),
        Err(error) => refused(what, error),
    }
}

/// Renders both answers of revision `at` from its one index, loading it only when `indexes` does
/// not hold it.
fn load_answers(
    runtime: &Runtime,
    indexes: &mut IndexCache,
    at: RevisionNumber,
) -> Result<Answers, ProjectError> {
    let index = indexes.index(runtime, Some(at))?;
    let loaded = index.loaded();
    let projection = ekr_views::render(loaded)?.bytes;
    let roles = super::view_roles::document(&loaded.graph);
    Ok(Answers { projection, roles })
}

/// A revision that could not be loaded or rendered: the named 404s, or a 500 for `what`.
fn refused(what: &str, error: ProjectError) -> Reply {
    match project_refusal(&error) {
        Some(name) => Reply::refusal(404, name, error),
        None => Reply::text(500, format!("{what}: {error}")),
    }
}

/// The retained bytes of the evidence the head holds under `id`, never as HTML.
fn evidence(runtime: &Runtime, id: &str) -> Reply {
    let not_found = || Reply::text(404, format!("evidence-not-found: {id}"));
    let Ok(id) = id.parse::<EvidenceId>() else {
        return not_found();
    };
    let graph = match runtime.snapshot() {
        Ok(graph) => graph,
        Err(error) => return Reply::text(500, format!("reading the head: {error}")),
    };
    let Some(item) = graph.evidence.get(&id) else {
        return not_found();
    };
    match runtime.content(&item.content_hash) {
        Ok(Some(bytes)) => Reply::ok(content_type(&bytes), bytes),
        Ok(None) => Reply::text(
            404,
            format!(
                "evidence-not-retained: {id} payload {} is not retained",
                item.content_hash
            ),
        ),
        Err(error) => Reply::text(500, format!("reading evidence {id}: {error}")),
    }
}

/// `text/plain; charset=utf-8` for UTF-8 bytes, `application/octet-stream` otherwise.
fn content_type(bytes: &[u8]) -> &'static str {
    if std::str::from_utf8(bytes).is_ok() {
        TEXT
    } else {
        BYTES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader with no deadline, for heads read from memory.
    #[allow(clippy::unnecessary_wraps)]
    fn no_deadline() -> Result<(), String> {
        Ok(())
    }

    /// The header policy is the page's own `<meta>` policy plus `frame-ancestors 'none'`, so the
    /// two cannot drift apart.
    #[test]
    fn the_page_header_policy_is_the_meta_policy_and_refuses_framing() {
        for (name, page) in [("/", PAGE), ("/alt", ALT_PAGE)] {
            assert!(
                page.contains(&format!("content=\"{PAGE_POLICY}\"")),
                "the <meta> policy of {name} is not PAGE_POLICY"
            );
        }
        let head = String::from_utf8(Reply::ok(HTML, Vec::new()).into_bytes())
            .expect("a reply head is text");
        assert!(
            head.contains(&format!(
                "Content-Security-Policy: {PAGE_POLICY}; frame-ancestors 'none'\r\n"
            )),
            "{head}"
        );
    }

    #[test]
    fn a_head_is_refused_once_its_deadline_has_passed_whatever_the_bytes_so_far() {
        let text = b"GET / HTTP/1.1\r\nHost: 127.0.0.1:9\r\n\r\n";
        let mut reads = 0;
        let refused = read_head(&mut &text[..], || {
            reads += 1;
            Err(late())
        });
        assert_eq!(refused, Err(late()));
        assert_eq!(reads, 1, "the deadline is checked before the first read");
        // A dribbled head is read one byte per call; the deadline passes after the fifth.
        let mut left = 5;
        let refused = read_head(&mut Dribble(&text[..]), || {
            if left == 0 {
                return Err(late());
            }
            left -= 1;
            Ok(())
        });
        assert_eq!(refused, Err(late()));
    }

    /// Hands out one byte per read.
    struct Dribble<'a>(&'a [u8]);

    impl Read for Dribble<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let Some((first, rest)) = self.0.split_first() else {
                return Ok(0);
            };
            buf[0] = *first;
            self.0 = rest;
            Ok(1)
        }
    }

    #[test]
    fn a_timed_out_read_is_the_late_refusal() {
        struct Stalled;
        impl Read for Stalled {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::WouldBlock.into())
            }
        }
        assert_eq!(read_head(&mut Stalled, no_deadline), Err(late()));
    }

    #[test]
    fn in_flight_is_capped_and_released_on_drop() {
        let count = Arc::new(AtomicUsize::new(0));
        let held: Vec<InFlight> = (0..IN_FLIGHT_LIMIT)
            .map(|_| InFlight::admit(&count).unwrap())
            .collect();
        assert!(InFlight::admit(&count).is_none());
        assert_eq!(count.load(Ordering::Acquire), IN_FLIGHT_LIMIT);
        drop(held);
        assert_eq!(count.load(Ordering::Acquire), 0);
        assert!(InFlight::admit(&count).is_some());
    }

    #[test]
    fn a_busy_reply_is_a_503_like_every_other() {
        let bytes = Reply::text(503, "busy").into_bytes();
        let head = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
        assert!(
            head.starts_with("http/1.1 503 service unavailable\r\n"),
            "{head}"
        );
        assert!(
            head.contains("\r\nx-content-type-options: nosniff\r\n"),
            "{head}"
        );
        assert!(head.contains("\r\ncache-control: no-store\r\n"), "{head}");
        assert!(head.contains("\r\nconnection: close\r\n"), "{head}");
    }

    #[test]
    fn on_port_80_a_host_without_a_port_is_its_own() {
        let one = |host: &str| vec![host.to_owned()];
        for host in ["127.0.0.1", "localhost", "127.0.0.1:80", "localhost:80"] {
            assert!(own_host(&one(host), 80), "{host}");
        }
        for host in ["127.0.0.1", "localhost"] {
            assert!(!own_host(&one(host), 8080), "{host} on 8080");
        }
        for host in [
            "127.0.0.1:8080",
            "rebound.example",
            "LOCALHOST",
            "127.0.0.1:",
        ] {
            assert!(!own_host(&one(host), 80), "{host}");
        }
    }

    #[test]
    fn utf8_bytes_are_text_and_any_other_bytes_are_an_octet_stream() {
        assert_eq!(content_type(b"Alice is chief executive"), TEXT);
        assert_eq!(content_type("é ü".as_bytes()), TEXT);
        assert_eq!(content_type(b""), TEXT);
        assert_eq!(content_type(&[0xff, 0xfe, 0x00]), BYTES);
        assert_eq!(content_type(b"<script>\xc3</script>"), BYTES);
    }

    #[test]
    fn a_projection_query_is_nothing_or_exactly_one_revision() {
        assert_eq!(revision(""), Ok(None));
        assert_eq!(revision("revision=3"), Ok(Some(RevisionNumber::new(3))));
        for bad in [
            "revision=",
            "revision=-1",
            "revision=x",
            "at=1",
            "revision=1&revision=2",
            "revision",
            "&",
            "revision=1&",
            "&revision=1",
            "revision=+0",
            "revision= 1",
            "revision=1 ",
            "revision=٣",
        ] {
            assert!(revision(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn only_one_host_naming_this_server_is_its_own() {
        let owned = |hosts: &[&str]| {
            hosts
                .iter()
                .map(|&host| host.to_owned())
                .collect::<Vec<_>>()
        };
        assert!(own_host(&owned(&["127.0.0.1:8080"]), 8080));
        assert!(own_host(&owned(&["localhost:8080"]), 8080));
        for hosts in [
            &[][..],
            &["127.0.0.1:8081"],
            &["127.0.0.1"],
            &["localhost"],
            &["LOCALHOST:8080"],
            &["rebound.example:8080"],
            &["127.0.0.1:8080 "],
            &["[::1]:8080"],
            &["127.0.0.1:8080", "127.0.0.1:8080"],
            &["127.0.0.1:8080", "rebound.example"],
        ] {
            assert!(!own_host(&owned(hosts), 8080), "{hosts:?}");
        }
    }

    #[test]
    fn only_the_twelve_routes_exist() {
        assert!(matches!(route("/"), Some(Route::Page)));
        assert!(matches!(route("/alt"), Some(Route::AltPage)));
        assert!(matches!(route("/head"), Some(Route::Head)));
        assert!(matches!(route("/projection"), Some(Route::Projection)));
        assert!(matches!(route("/roles"), Some(Route::Roles)));
        assert!(matches!(
            route("/evidence/abc"),
            Some(Route::Evidence("abc"))
        ));
        assert!(matches!(route("/overview"), Some(Route::Overview)));
        assert!(matches!(route("/expand"), Some(Route::Expand)));
        assert!(matches!(route("/search"), Some(Route::Search)));
        assert!(matches!(route("/node/abc"), Some(Route::Node("abc"))));
        assert!(matches!(route("/timeline"), Some(Route::Timeline)));
        assert!(matches!(route("/changes"), Some(Route::Changes)));
        for path in [
            "",
            "/changes/",
            "/change",
            "/index.html",
            "/projection/",
            "/roles/",
            "/evidence/",
            "/evidence/a/b",
            "/evidence",
            "/alt/",
            "/alt.html",
            "/head/",
            "/heads",
            "/overview/",
            "/expand/",
            "/search/",
            "/timeline/",
            "/node",
            "/node/",
            "/node/a/b",
            "/nodes/abc",
        ] {
            assert!(route(path).is_none(), "{path}");
        }
    }

    #[test]
    fn a_query_is_known_names_once_each_with_percent_decoded_values() {
        let known = &["q", "limit", "revision"];
        assert_eq!(Query::parse("", known), Ok(Query { pairs: Vec::new() }));
        let query = Query::parse("q=a%20b+c%2B%C3%A9&limit=-3&revision=%32", known).unwrap();
        assert_eq!(query.get("q"), Some("a b c+é"));
        assert_eq!(query.integer("limit"), Ok(Some(-3)));
        assert_eq!(query.revision(), Ok(Some(RevisionNumber::new(2))));
        assert_eq!(query.integer("absent"), Ok(None));
        assert_eq!(Query::parse("q=", known).unwrap().get("q"), Some(""));
        for bad in [
            "&", "q=a&", "&q=a", "q", "q=a&q=b", "other=1", "Q=a", "q=%", "q=%4", "q=%4g", "q=%ff",
            "q=%C3",
        ] {
            assert!(Query::parse(bad, known).is_err(), "{bad}");
        }
        for bad in [
            "1.5",
            "",
            "-",
            "+1",
            " 1",
            "1e3",
            "٣",
            "18446744073709551616",
        ] {
            let query = Query {
                pairs: vec![("limit", bad.to_owned()), ("revision", bad.to_owned())],
            };
            assert!(query.integer("limit").is_err(), "limit {bad:?}");
            assert!(query.revision().is_err(), "revision {bad:?}");
        }
        let query = Query {
            pairs: vec![("revision", "1&revision=2".to_owned())],
        };
        assert!(query.revision().is_err(), "a decoded `&` is not a digit");
    }

    fn id<T: std::str::FromStr>(n: u64) -> T
    where
        T::Err: std::fmt::Debug,
    {
        format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
    }

    fn meta() -> SliceMeta {
        SliceMeta {
            format: ekr_views::SLICE_FORMAT,
            revision: 1,
            seeds: vec![id(1)],
            depth: 1,
            after: 0,
            node_total: 600,
            edge_total: 0,
        }
    }

    fn records(count: u64) -> Vec<SliceRecord> {
        (0..count)
            .map(|n| {
                SliceRecord::Node(SliceNode {
                    id: id(n + 1),
                    type_id: id(9),
                    name: format!("n{n}"),
                    degree: u64::MAX,
                    distance: 0,
                })
            })
            .collect()
    }

    /// Each chunk's data from a whole chunked body, checking its framing.
    fn chunks(mut body: &[u8]) -> Vec<&[u8]> {
        let mut out = Vec::new();
        loop {
            let end = body.windows(2).position(|w| w == b"\r\n").unwrap();
            let size =
                usize::from_str_radix(std::str::from_utf8(&body[..end]).unwrap(), 16).unwrap();
            body = &body[end + 2..];
            if size == 0 {
                assert_eq!(body, b"\r\n");
                return out;
            }
            out.push(&body[..size]);
            assert_eq!(&body[size..size + 2], b"\r\n");
            body = &body[size + 2..];
        }
    }

    #[test]
    fn a_stream_is_the_meta_chunk_then_a_chunk_per_256_records_then_the_end() {
        for (count, next) in [(0_u64, None), (256, Some(256)), (600, Some(600))] {
            let mut out = Vec::new();
            write_lines(
                &mut out,
                Framing::Chunked,
                &meta(),
                &records(count),
                next,
                7,
            )
            .unwrap();
            let end = out.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
            assert_eq!(
                std::str::from_utf8(&out[..end + 4]).unwrap(),
                stream_head(Framing::Chunked),
                "{count}"
            );
            let head = stream_head(Framing::Chunked).to_ascii_lowercase();
            for line in [
                "content-type: application/x-ndjson",
                "transfer-encoding: chunked",
                "x-content-type-options: nosniff",
                "cache-control: no-store",
                "connection: close",
            ] {
                assert!(head.lines().any(|l| l == line), "{line}");
            }
            assert!(!head.contains("content-length"));
            let chunks = chunks(&out[end + 4..]);
            let full = usize::try_from(count).unwrap() / PROGRESS_EVERY;
            assert_eq!(chunks.len(), 2 + full, "{count}");
            assert_eq!(
                chunks[0],
                format!(
                    "{{\"kind\":\"meta\",{}\n",
                    &serde_json::to_string(&meta()).unwrap()[1..]
                )
                .as_bytes()
            );
            let body = chunks.concat();
            let lines: Vec<&str> = body
                .split_inclusive(|byte| *byte == b'\n')
                .map(|line| std::str::from_utf8(line).unwrap())
                .collect();
            assert!(lines.iter().all(|line| line.ends_with('\n')));
            assert_eq!(
                lines.len(),
                1 + usize::try_from(count).unwrap() + full + 1,
                "{count}"
            );
            if count > 0 {
                assert!(
                    lines[1].starts_with("{\"kind\":\"node\",\"id\":"),
                    "{}",
                    lines[1]
                );
            }
            if count >= 256 {
                assert_eq!(lines[257], "{\"kind\":\"progress\",\"sent\":256}\n");
                assert!(std::str::from_utf8(chunks[1])
                    .unwrap()
                    .ends_with("{\"kind\":\"progress\",\"sent\":256}\n"));
                assert!(lines[1].contains("\"degree\":18446744073709551615"));
            }
            let expected_end = match next {
                Some(next) => format!("{{\"kind\":\"end\",\"next\":{next},\"remaining\":7}}\n"),
                None => "{\"kind\":\"end\",\"next\":null,\"remaining\":7}\n".to_owned(),
            };
            assert_eq!(*lines.last().unwrap(), expected_end);
        }
    }

    /// Under [`Framing::Close`] a stream is the same head without `Transfer-Encoding`, then the
    /// same lines the chunked stream carries, bare, with no last chunk: the close ends them.
    #[test]
    fn an_unframed_stream_is_the_chunked_streams_lines_bare_and_unterminated() {
        for count in [0_u64, 256, 600] {
            let mut chunked = Vec::new();
            write_lines(
                &mut chunked,
                Framing::Chunked,
                &meta(),
                &records(count),
                None,
                7,
            )
            .unwrap();
            let mut bare = Vec::new();
            write_lines(&mut bare, Framing::Close, &meta(), &records(count), None, 7).unwrap();
            let end = bare.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
            let head = std::str::from_utf8(&bare[..end + 4]).unwrap();
            assert_eq!(head, stream_head(Framing::Close));
            assert_eq!(
                head.replace("\r\n", "\n"),
                stream_head(Framing::Chunked)
                    .replace("Transfer-Encoding: chunked\r\n", "")
                    .replace("\r\n", "\n"),
                "only Transfer-Encoding differs"
            );
            assert!(!head.to_ascii_lowercase().contains("transfer-encoding"));
            assert!(!head.to_ascii_lowercase().contains("content-length"));
            let chunked_end = chunked.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
            assert_eq!(
                &bare[end + 4..],
                &chunks(&chunked[chunked_end + 4..]).concat()[..],
                "{count}"
            );
        }
    }

    /// Takes `room` bytes, then fails every write, and counts the writes it was asked for.
    struct Closing {
        room: usize,
        refused: usize,
    }

    impl Write for Closing {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.room == 0 {
                self.refused += 1;
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
            let taken = bytes.len().min(self.room);
            self.room -= taken;
            Ok(taken)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_stream_stops_at_the_first_write_that_fails() {
        let mut closing = Closing {
            room: 300,
            refused: 0,
        };
        let written = write_lines(
            &mut closing,
            Framing::Chunked,
            &meta(),
            &records(2000),
            None,
            0,
        );
        assert_eq!(
            written.map_err(|error| error.kind()),
            Err(std::io::ErrorKind::BrokenPipe)
        );
        assert_eq!(closing.refused, 1, "nothing is written after the failure");
    }

    #[test]
    fn a_stream_past_its_deadline_writes_nothing() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let mut late = Deadlined {
            stream: &client,
            deadline: Instant::now(),
        };
        assert_eq!(
            late.write(b"x").map_err(|error| error.kind()),
            Err(std::io::ErrorKind::TimedOut)
        );
    }

    fn answers(tag: u8) -> Answers {
        Answers {
            projection: vec![tag],
            roles: vec![tag, tag],
        }
    }

    #[test]
    fn a_revision_is_loaded_once_and_served_from_memory_after() {
        let mut cache = Cache::default();
        let mut loads = 0;
        for _ in 0..3 {
            let got = cache
                .get_or_load(RevisionNumber::new(2), || {
                    loads += 1;
                    Ok::<_, ()>(answers(2))
                })
                .unwrap();
            assert_eq!(got, &answers(2));
        }
        assert_eq!(
            loads, 1,
            "the second and third request are answered from memory"
        );
    }

    #[test]
    fn a_failed_load_is_not_kept_and_a_kept_one_is_not_loaded_again() {
        let mut cache = Cache::default();
        let at = RevisionNumber::new(1);
        assert_eq!(
            cache.get_or_load(at, || Err::<Answers, _>("refused")),
            Err("refused")
        );
        assert!(cache.entries.is_empty(), "a refusal is not cached");
        cache.get_or_load(at, || Ok::<_, ()>(answers(1))).unwrap();
        let got = cache
            .get_or_load(at, || Err::<Answers, _>("loaded again"))
            .unwrap();
        assert_eq!(got, &answers(1));
        assert_eq!(cache.entries.len(), 1);
    }

    #[test]
    fn beyond_the_limit_the_revision_used_longest_ago_goes() {
        let mut cache = Cache::default();
        let tag = |n: usize| u8::try_from(n).unwrap();
        for n in 0..CACHE_LIMIT {
            cache
                .get_or_load(RevisionNumber::new(u64::try_from(n).unwrap()), || {
                    Ok::<_, ()>(answers(tag(n)))
                })
                .unwrap();
        }
        // Revision 0 is used again, so revision 1 is now the one used longest ago.
        cache
            .get_or_load(RevisionNumber::new(0), || Err::<Answers, _>(()))
            .unwrap();
        cache
            .get_or_load(RevisionNumber::new(99), || Ok::<_, ()>(answers(99)))
            .unwrap();
        let held: Vec<u64> = cache.entries.iter().map(|(n, _)| n.get()).collect();
        assert_eq!(held.len(), CACHE_LIMIT);
        assert!(
            held.contains(&0) && !held.contains(&1) && held.contains(&99),
            "{held:?}"
        );
    }

    #[test]
    fn every_reply_is_nosniff_no_store_and_close_with_no_cookie_or_cors_header() {
        for reply in [
            Reply::ok(HTML, b"<p>".to_vec()),
            Reply::text(400, "bad-request"),
            Reply::text(404, "not-found"),
            Reply::text(405, "method-not-allowed"),
            Reply::text(413, "request-body-refused"),
            Reply::text(421, "misdirected-request"),
            Reply::refusal(404, "ekr.views.RevisionNotFound", "absent"),
        ] {
            let status = reply.status;
            let body = reply.body.clone();
            let bytes = reply.into_bytes();
            let end = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
            let head = std::str::from_utf8(&bytes[..end])
                .unwrap()
                .to_ascii_lowercase();
            assert!(head.starts_with(&format!("http/1.1 {status} ")), "{head}");
            for line in [
                "x-content-type-options: nosniff",
                "cache-control: no-store",
                "connection: close",
                &format!("content-length: {}", body.len()),
            ] {
                assert!(
                    head.lines().any(|l| l == line),
                    "{status}: {line} in {head}"
                );
            }
            assert_eq!(head.contains("\r\nallow: get"), status == 405, "{head}");
            assert!(!head.contains("set-cookie") && !head.contains("access-control-"));
            assert_eq!(&bytes[end + 4..], &body[..]);
        }
    }

    #[test]
    fn a_head_is_read_to_its_end_and_never_past_it() {
        let text = b"GET /projection?revision=0 HTTP/1.1\r\nHost: 127.0.0.1:9\r\n\r\nBODY";
        let asked = read_head(&mut &text[..], no_deadline).unwrap();
        assert_eq!(
            asked,
            Asked {
                method: "GET".to_owned(),
                target: "/projection?revision=0".to_owned(),
                hosts: vec!["127.0.0.1:9".to_owned()],
                announces_body: false,
                framing: Framing::Chunked,
            }
        );
        let older = b"GET /expand HTTP/1.0\r\nHost: 127.0.0.1:9\r\n\r\n";
        assert_eq!(
            read_head(&mut &older[..], no_deadline).unwrap().framing,
            Framing::Close,
            "an HTTP/1.0 request is never answered chunked"
        );
        let announcing = [
            "Content-Length: 1",
            "content-length: 18446744073709551615",
            "Content-Length: x",
            "Transfer-Encoding: chunked",
        ];
        for header in announcing {
            let text = format!("GET / HTTP/1.1\r\nHost: h\r\n{header}\r\n\r\n");
            assert!(
                read_head(&mut text.as_bytes(), no_deadline)
                    .unwrap()
                    .announces_body,
                "{header}"
            );
        }
        let zero = "GET / HTTP/1.1\r\nHost: h\r\nContent-Length: 0\r\n\r\n";
        assert!(
            !read_head(&mut zero.as_bytes(), no_deadline)
                .unwrap()
                .announces_body
        );
    }

    #[test]
    fn a_malformed_short_or_oversized_head_is_refused() {
        let oversized = format!(
            "GET / HTTP/1.1\r\nHost: h\r\nX-Pad: {}\r\n\r\n",
            "a".repeat(HEAD_LIMIT)
        );
        let many = format!(
            "GET / HTTP/1.1\r\n{}\r\n",
            "X-Many: 1\r\n".repeat(HEADER_LIMIT + 1)
        );
        for text in [
            "GET / HTTP/1.1\r\nHost: h\r\na header line without a colon\r\n\r\n",
            "GET / HTTP/1.1\r\nHost: h\r\n",
            "",
            "\u{0}\u{1}\u{2}",
            &oversized,
            &many,
        ] {
            assert!(
                read_head(&mut text.as_bytes(), no_deadline).is_err(),
                "{text:?}"
            );
        }
    }

    /// The named refusal `answer` gives `target` on port 9, or `None` when it answers otherwise.
    fn refusal_name(runtime: &Runtime, memory: &mut Memory, target: &str) -> Option<String> {
        let head = format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1:9\r\n\r\n");
        let asked = read_head(&mut head.as_bytes(), no_deadline).unwrap();
        let Answered::Whole(reply) = answer(runtime, memory, 9, &asked) else {
            return None;
        };
        let body: serde_json::Value = serde_json::from_slice(&reply.body).ok()?;
        body["refusal"].as_str().map(str::to_owned)
    }

    /// views.yaml, "Bounds and the order of refusals": a broken bound answers LimitExceeded before
    /// the store is read, then an unseeded store answers NotSeeded — before an absent revision and
    /// before a node the revision does not hold, whether or not that node's text is a node id.
    #[test]
    fn every_bounded_read_on_an_unseeded_store_is_not_seeded_after_its_bounds() {
        let host = crate::host::CliHostConfigurationV1::from_json(
            &std::fs::read(
                std::path::PathBuf::from(
                    std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"),
                )
                .join("tests/fixtures/retraction/host.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let runtime = Runtime::file(
            &directory.path().join("store"),
            &host.tenant,
            host.context,
            host.authority,
        )
        .unwrap();
        assert_eq!(runtime.head().unwrap(), None, "the store is unseeded");
        let node = "00000000-0000-4000-8000-000000000399";
        let mut memory = Memory::default();
        for (target, name) in [
            (
                "/overview?limit=0&revision=9".to_owned(),
                "ekr.views.LimitExceeded",
            ),
            ("/overview?revision=9".to_owned(), "ekr.views.NotSeeded"),
            (
                format!("/expand?seeds={node}&depth=3&limit=10&revision=9"),
                "ekr.views.LimitExceeded",
            ),
            (
                format!("/expand?seeds={node}&depth=1&limit=10&revision=9"),
                "ekr.views.NotSeeded",
            ),
            (
                format!("/expand?seeds={node}&depth=1&limit=10"),
                "ekr.views.NotSeeded",
            ),
            (
                "/expand?seeds=&depth=1&limit=10".to_owned(),
                "ekr.views.NotSeeded",
            ),
            (format!("/node/{node}"), "ekr.views.NotSeeded"),
            (format!("/node/{node}?revision=9"), "ekr.views.NotSeeded"),
            ("/node/not-an-id".to_owned(), "ekr.views.NotSeeded"),
            (
                "/node/not-an-id?revision=9".to_owned(),
                "ekr.views.NotSeeded",
            ),
            (
                "/search?q=a&limit=0&revision=9".to_owned(),
                "ekr.views.LimitExceeded",
            ),
            ("/search?q=a&revision=9".to_owned(), "ekr.views.NotSeeded"),
            (
                "/timeline?hops=0&limit=10&revision=9".to_owned(),
                "ekr.views.LimitExceeded",
            ),
            (
                format!("/timeline?subject={node}&hops=1&limit=10&revision=9"),
                "ekr.views.NotSeeded",
            ),
            (
                "/changes?since_revision=-1&limit=0&at=9".to_owned(),
                "ekr.views.SinceMalformed",
            ),
            (
                "/changes?since_revision=0&limit=0&at=9".to_owned(),
                "ekr.views.LimitExceeded",
            ),
            (
                "/changes?since_valid=0&after=-1".to_owned(),
                "ekr.views.LimitExceeded",
            ),
            (
                "/changes?since_revision=9&at=9".to_owned(),
                "ekr.views.NotSeeded",
            ),
            (
                "/changes?since_recorded=0".to_owned(),
                "ekr.views.NotSeeded",
            ),
            ("/changes".to_owned(), "invalid-query"),
            (
                "/changes?since_revision=0&since_valid=0".to_owned(),
                "invalid-query",
            ),
            ("/changes?since_revision=0&at=x".to_owned(), "invalid-query"),
            ("/projection?revision=9".to_owned(), "ekr.views.NotSeeded"),
            ("/roles".to_owned(), "ekr.views.NotSeeded"),
            ("/head".to_owned(), "ekr.views.NotSeeded"),
            ("/head?revision=0".to_owned(), "invalid-query"),
        ] {
            assert_eq!(
                refusal_name(&runtime, &mut memory, &target).as_deref(),
                Some(name),
                "GET {target}"
            );
        }
    }
}
