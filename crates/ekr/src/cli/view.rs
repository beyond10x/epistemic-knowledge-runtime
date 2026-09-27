//! `ekr view`: a read-only viewer for an existing store, served on 127.0.0.1 only
//! (`story:ekr-view-server`).
//!
//! The server is `tiny_http`, which is blocking: one thread accepts a request, answers it and
//! takes the next, and no async runtime exists anywhere in the process. Every store call below
//! therefore runs on the thread that opened the [`Runtime`], outside any Tokio context, as
//! `architecture-decision-record:0006-ekr-store-bridges-the-async-port` requires.
//!
//! Nothing here proposes, validates, commits or seeds: the only store calls are
//! [`ekr_views::project`], [`Runtime::snapshot`] and [`Runtime::content`]. A local read-only page
//! is not an outward write (design § 82, § 83).
//!
//! | method and path | answer |
//! |---|---|
//! | `GET /` | the embedded viewer page, `text/html; charset=utf-8` |
//! | `GET /projection` | the `ekr.graph-projection/1` bytes `ekr-views` renders at the head, `application/json` |
//! | `GET /projection?revision=N` | the same at revision `N`; an absent revision is 404 `ekr.views.RevisionNotFound` |
//! | `GET /evidence/<evidence id>` | that evidence's retained bytes, `text/plain; charset=utf-8` when they are UTF-8, else `application/octet-stream`; 404 for an unknown id or bytes not retained |
//! | any other method on those paths | 405 |
//! | any other path | 404 |
//!
//! Every response carries `X-Content-Type-Options: nosniff` and no cookie or CORS header. Record
//! text is untrusted evidence (A14) and is never served as HTML: the only HTML is the page, which
//! is embedded in the binary at build time and writes what it fetches through `textContent`.

use std::io::Write;

use ekr_core::{EvidenceId, RevisionNumber};
use ekr_kernel::Runtime;
use ekr_views::ProjectError;
use tiny_http::{Header, Method, Response, Server};

use crate::exit::Failure;

/// The viewer page, embedded at build time; nothing is read from disk at run time.
const PAGE: &str = include_str!("viewer/index.html");

const HTML: &str = "text/html; charset=utf-8";
const JSON: &str = "application/json";
const TEXT: &str = "text/plain; charset=utf-8";
const BYTES: &str = "application/octet-stream";

/// Binds 127.0.0.1 on `port` (0 picks a free one), prints `{"url": …}` as one JSON line on
/// stdout, and answers requests until the process is interrupted.
///
/// # Errors
///
/// The address does not bind, or stdout cannot be written.
pub(super) fn run(runtime: &Runtime, port: u16) -> Result<String, Failure> {
    let server = Server::http(("127.0.0.1", port))
        .map_err(|error| Failure::fault(format!("binding 127.0.0.1:{port}: {error}")))?;
    let address = server
        .server_addr()
        .to_ip()
        .ok_or_else(|| Failure::fault("the server is not listening on an IP address"))?;
    let line = serde_json::json!({ "url": format!("http://{address}/") });
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{line}")
        .and_then(|()| stdout.flush())
        .map_err(|error| Failure::fault(format!("writing the URL: {error}")))?;
    drop(stdout);
    for request in server.incoming_requests() {
        let reply = answer(runtime, request.method(), request.url());
        // A client that went away is its own business; the server takes the next request.
        let _ = request.respond(reply.into_response());
    }
    Ok(String::new())
}

/// One answer, before it becomes a `tiny_http` response.
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

    fn into_response(self) -> Response<std::io::Cursor<Vec<u8>>> {
        let mut response = Response::from_data(self.body)
            .with_status_code(self.status)
            .with_header(header("Content-Type", self.content_type))
            .with_header(header("X-Content-Type-Options", "nosniff"))
            .with_header(header("Cache-Control", "no-store"));
        if self.status == 405 {
            response = response.with_header(header("Allow", "GET"));
        }
        response
    }
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes())
        .unwrap_or_else(|()| unreachable!("{name}: {value} is a valid header"))
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

/// Answers one request: an unknown path is 404 whatever the method, a known path is 405 for any
/// method but `GET`.
fn answer(runtime: &Runtime, method: &Method, url: &str) -> Reply {
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let Some(route) = route(path) else {
        return Reply::text(404, format!("not-found: {path}"));
    };
    if *method != Method::Get {
        return Reply::text(
            405,
            format!("method-not-allowed: {method} {path}; only GET"),
        );
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
    fn every_reply_is_nosniff_and_carries_no_cookie_or_cors_header() {
        for reply in [
            Reply::ok(HTML, b"<p>".to_vec()),
            Reply::text(404, "not-found"),
            Reply::text(405, "method-not-allowed"),
            Reply::refusal(404, "ekr.views.RevisionNotFound", "absent"),
        ] {
            let response = reply.into_response();
            let headers: Vec<(String, String)> = response
                .headers()
                .iter()
                .map(|h| {
                    (
                        h.field.as_str().as_str().to_ascii_lowercase(),
                        h.value.as_str().to_owned(),
                    )
                })
                .collect();
            assert!(headers.contains(&("x-content-type-options".to_owned(), "nosniff".to_owned())));
            assert!(headers
                .iter()
                .all(|(name, _)| name != "set-cookie" && !name.starts_with("access-control-")));
        }
    }
}
