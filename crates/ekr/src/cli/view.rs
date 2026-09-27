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
//! gets the response bytes back, writes them with `Connection: close` (each write waiting at most
//! 5 s), and closes. So every store call runs on that one thread, outside any Tokio context, as
//! `architecture-decision-record:0006-ekr-store-bridges-the-async-port` requires, and a client
//! that stalls holds one of the 64 places for at most 10 s.
//!
//! Before any store call a request is refused unless it carries exactly one `Host` header naming
//! this server, `127.0.0.1:<port>` or `localhost:<port>` — on port 80 also `127.0.0.1` or
//! `localhost` alone, as a browser sends it — (421 otherwise, so a page reached through DNS
//! rebinding is served nothing), and unless it announces no body: any `Content-Length` above zero
//! or any `Transfer-Encoding` is 413, answered without reading the body.
//!
//! Nothing here proposes, validates, commits or seeds: the only store calls are
//! [`ekr_views::project`], [`Runtime::snapshot`] and [`Runtime::content`]. A local read-only page
//! is not an outward write (design § 82, § 83).
//!
//! | request | answer |
//! |---|---|
//! | `GET /` | the embedded viewer page, `text/html; charset=utf-8` |
//! | `GET /projection` | the `ekr.graph-projection/1` bytes `ekr-views` renders at the head, `application/json` |
//! | `GET /projection?revision=N` | the same at revision `N`; an absent revision is 404 `ekr.views.RevisionNotFound` |
//! | `GET /evidence/<evidence id>` | that evidence's retained bytes, `text/plain; charset=utf-8` when they are UTF-8, else `application/octet-stream`; 404 for an unknown id or bytes not retained |
//! | any other method on those paths | 405, with `Allow: GET` |
//! | any other path | 404 |
//! | a `GET` of those paths that announces a body | 413 |
//! | any request whose `Host` is not this server's | 421 |
//! | a head that does not parse, or is not complete within 16 KiB or 5 s of accept | 400 |
//! | a connection while 64 are in flight | 503 `busy`, unread |
//!
//! Every response carries `X-Content-Type-Options: nosniff`, `Cache-Control: no-store` and
//! `Connection: close`, and no cookie or CORS header. Record text is untrusted evidence (A14) and
//! is never served as HTML: the only HTML is the page, which is embedded in the binary at build
//! time and writes what it fetches through `textContent`.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ekr_core::{EvidenceId, RevisionNumber};
use ekr_kernel::Runtime;
use ekr_views::ProjectError;

use crate::exit::Failure;

/// The viewer page, embedded at build time; nothing is read from disk at run time.
const PAGE: &str = include_str!("viewer/index.html");

const HTML: &str = "text/html; charset=utf-8";
const JSON: &str = "application/json";
const TEXT: &str = "text/plain; charset=utf-8";
const BYTES: &str = "application/octet-stream";

/// The most request-head bytes a connection reads.
const HEAD_LIMIT: usize = 16 * 1024;
/// The most header lines a request head may carry.
const HEADER_LIMIT: usize = 64;
/// How long after accept a connection has to send its whole head, and how long a write waits.
const TIMEOUT: Duration = Duration::from_secs(5);
/// The most connections in flight at once; one more is answered 503 without being read.
const IN_FLIGHT_LIMIT: usize = 64;

/// One parsed request and where its answer goes: from a connection thread to the store thread.
type Job = (Asked, Sender<Vec<u8>>);

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
    for (asked, reply_to) in store_thread {
        // A connection that timed out meanwhile is its own business.
        let _ = reply_to.send(answer(runtime, port, &asked).into_bytes());
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
/// write it, close. The body, if any, is never read.
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
    let bytes = match head {
        Ok(asked) => {
            let (reply_to, reply) = channel();
            if jobs.send((asked, reply_to)).is_err() {
                return;
            }
            match reply.recv() {
                Ok(bytes) => bytes,
                Err(_) => return,
            }
        }
        Err(message) => Reply::text(400, format!("bad-request: {message}")).into_bytes(),
    };
    // A client that went away is its own business.
    let _ = stream.write_all(&bytes).and_then(|()| stream.flush());
    let _ = stream.shutdown(Shutdown::Write);
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
            Ok(Some(Asked {
                method: request.method.unwrap_or_default().to_owned(),
                target: request.path.unwrap_or_default().to_owned(),
                hosts,
                announces_body,
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
        let mut bytes = format!(
            "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
             X-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\n{allow}\
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
    Projection,
    Evidence(&'a str),
}

fn route(path: &str) -> Option<Route<'_>> {
    match path {
        "/" => Some(Route::Page),
        "/projection" => Some(Route::Projection),
        _ => path
            .strip_prefix("/evidence/")
            .filter(|id| !id.is_empty() && !id.contains('/'))
            .map(Route::Evidence),
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
fn answer(runtime: &Runtime, port: u16, asked: &Asked) -> Reply {
    if !own_host(&asked.hosts, port) {
        return Reply::text(
            421,
            format!("misdirected-request: Host must be 127.0.0.1:{port} or localhost:{port}"),
        );
    }
    let target = asked.target.as_str();
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let Some(route) = route(path) else {
        return Reply::text(404, format!("not-found: {path}"));
    };
    if asked.method != "GET" {
        return Reply::text(
            405,
            format!("method-not-allowed: {} {path}; only GET", asked.method),
        );
    }
    if asked.announces_body {
        return Reply::text(413, "request-body-refused: a GET carries no body");
    }
    match route {
        Route::Page => Reply::ok(HTML, PAGE.as_bytes().to_vec()),
        Route::Projection => projection(runtime, query),
        Route::Evidence(id) => evidence(runtime, id),
    }
}

/// The revision a projection query names: none, or exactly `revision=N`.
fn revision(query: &str) -> Result<Option<RevisionNumber>, String> {
    let mut revision = None;
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        match pair.split_once('=') {
            Some(("revision", value)) if revision.is_none() => {
                let number = value
                    .parse::<u64>()
                    .map_err(|_| format!("revision {value:?} is not a revision number"))?;
                revision = Some(RevisionNumber::new(number));
            }
            _ => return Err(format!("the query {query:?} is not `revision=N`")),
        }
    }
    Ok(revision)
}

fn projection(runtime: &Runtime, query: &str) -> Reply {
    let at = match revision(query) {
        Ok(at) => at,
        Err(message) => return Reply::refusal(400, "invalid-query", message),
    };
    match ekr_views::project(runtime, at) {
        Ok(rendered) => Reply::ok(JSON, rendered.bytes),
        Err(error @ ProjectError::RevisionNotFound { .. }) => {
            Reply::refusal(404, "ekr.views.RevisionNotFound", error)
        }
        Err(error @ ProjectError::NotSeeded { .. }) => {
            Reply::refusal(404, "ekr.views.NotSeeded", error)
        }
        Err(error) => Reply::text(500, format!("projection: {error}")),
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
    fn only_the_three_routes_exist() {
        assert!(matches!(route("/"), Some(Route::Page)));
        assert!(matches!(route("/projection"), Some(Route::Projection)));
        assert!(matches!(
            route("/evidence/abc"),
            Some(Route::Evidence("abc"))
        ));
        for path in [
            "",
            "/index.html",
            "/projection/",
            "/evidence/",
            "/evidence/a/b",
            "/evidence",
        ] {
            assert!(route(path).is_none(), "{path}");
        }
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
            }
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
}
