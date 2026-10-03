//! Bounded blocking HTTP for hosted readers. Store work stays on the calling thread.
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::exit::Failure;

pub(super) const HEAD_LIMIT: usize = 16 * 1024;
pub(super) const BODY_LIMIT: usize = 6 * 1024 * 1024 + 65_536;
pub(super) const CONNECTION_LIMIT: usize = 64;
pub(super) const QUEUE_LIMIT: usize = 16;
pub(super) const READ_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const WAIT_TIMEOUT: Duration = Duration::from_secs(30);
pub(super) const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

/// Exact admitted HTTP authorities; forwarded headers never contribute.
#[derive(Clone, Debug)]
pub(super) struct Authorities(Vec<String>);
impl Authorities {
    pub(super) fn loopback(port: u16) -> Self {
        let mut hosts = vec![format!("localhost:{port}"), format!("127.0.0.1:{port}")];
        if port == 80 {
            hosts.extend(["localhost".into(), "127.0.0.1".into()]);
        }
        Self(hosts)
    }
    pub(super) fn accepts(&self, hosts: &[String]) -> bool {
        matches!(hosts, [host] if self.0.contains(host))
    }
}
fn authority(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 320
        || !value.is_ascii()
        || value
            .bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace() || b"/@?#\\%".contains(&b))
    {
        return false;
    }
    if let Some(bracketed) = value.strip_prefix('[') {
        let Some((address, tail)) = bracketed.split_once(']') else {
            return false;
        };
        return address.parse::<std::net::Ipv6Addr>().is_ok()
            && (tail.is_empty() || tail.strip_prefix(':').is_some_and(valid_port));
    }
    let (host, port) = value
        .split_once(':')
        .map_or((value, None), |(h, p)| (h, Some(p)));
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        && port.is_none_or(valid_port)
}
fn valid_port(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|b| b.is_ascii_digit())
        && value.parse::<u16>().is_ok_and(|p| p != 0)
}

pub(super) fn bind(
    ip: IpAddr,
    port: u16,
    hosts: Vec<String>,
) -> Result<(TcpListener, Authorities), Failure> {
    if (!ip.is_loopback() && hosts.is_empty()) || hosts.iter().any(|host| !authority(host)) {
        return Err(Failure::fault(
            "external listeners require explicit valid --allow-host authorities",
        ));
    }
    let listener = TcpListener::bind(SocketAddr::new(ip, port))
        .map_err(|_| Failure::fault("binding HTTP listener failed"))?;
    let bound = listener
        .local_addr()
        .map_err(|_| Failure::fault("reading HTTP listener address failed"))?;
    let policy = if hosts.is_empty() {
        let mut policy = Authorities::loopback(bound.port());
        if ip.is_ipv6() {
            policy.0.push(format!("[{}]:{}", ip, bound.port()));
            if bound.port() == 80 {
                policy.0.push(format!("[{ip}]"));
            }
        }
        policy
    } else {
        Authorities(hosts)
    };
    Ok((listener, policy))
}
pub(super) fn announce(listener: &TcpListener) -> Result<(), Failure> {
    let address = listener
        .local_addr()
        .map_err(|_| Failure::fault("reading HTTP listener address failed"))?;
    let mut out = std::io::stdout().lock();
    writeln!(
        out,
        "{}",
        serde_json::json!({"url":format!("http://{address}/")})
    )
    .and_then(|()| out.flush())
    .map_err(|_| Failure::fault("writing HTTP listener URL failed"))
}

#[derive(Debug)]
pub(super) struct Request {
    pub(super) method: String,
    pub(super) target: String,
    pub(super) hosts: Vec<String>,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}
impl Request {
    pub(super) fn header(&self, name: &str) -> Result<Option<&str>, ()> {
        let mut values = self
            .headers
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str());
        let first = values.next();
        if values.next().is_some() {
            Err(())
        } else {
            Ok(first)
        }
    }
}
#[derive(Debug)]
pub(super) struct Reply {
    pub(super) status: u16,
    pub(super) body: Vec<u8>,
    pub(super) allow: Option<&'static str>,
}
impl Reply {
    pub(super) fn json(status: u16, body: Vec<u8>) -> Self {
        Self {
            status,
            body,
            allow: None,
        }
    }
    pub(super) fn refusal(status: u16, reason: &'static str) -> Self {
        Self::json(
            status,
            serde_json::json!({"error":reason}).to_string().into_bytes(),
        )
    }
    pub(super) fn method(allow: &'static str) -> Self {
        Self {
            status: 405,
            body: Vec::new(),
            allow: Some(allow),
        }
    }
    fn write(self, stream: &mut TcpStream) -> std::io::Result<()> {
        let reason = match self.status {
            200 => "OK",
            202 => "Accepted",
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            406 => "Not Acceptable",
            413 => "Content Too Large",
            415 => "Unsupported Media Type",
            421 => "Misdirected Request",
            503 => "Service Unavailable",
            _ => "Internal Server Error",
        };
        let allow = self
            .allow
            .map_or(String::new(), |methods| format!("Allow: {methods}\r\n"));
        let head = format!("HTTP/1.1 {} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\n{allow}Connection: close\r\n\r\n",self.status,self.body.len());
        let mut writer = Deadlined {
            stream,
            deadline: Instant::now() + WRITE_TIMEOUT,
        };
        writer.write_all(head.as_bytes())?;
        writer.write_all(&self.body)?;
        writer.flush()
    }
}
/// A total write deadline, not a deadline restarted by each partial write.
pub(super) struct Deadlined<'a> {
    pub(super) stream: &'a TcpStream,
    pub(super) deadline: Instant,
}
impl Write for Deadlined<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let left = self.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        self.stream
            .set_write_timeout(Some(left.min(WRITE_TIMEOUT)))?;
        (&*self.stream).write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        (&*self.stream).flush()
    }
}

struct Head {
    request: Request,
    consumed: usize,
    length: usize,
}
fn parse(bytes: &[u8]) -> Result<Option<Head>, Reply> {
    let malformed = || Reply::refusal(400, "malformed HTTP request");
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut request = httparse::Request::new(&mut headers);
    let consumed = match request.parse(bytes).map_err(|_| malformed())? {
        httparse::Status::Partial => return Ok(None),
        httparse::Status::Complete(n) => n,
    };
    if consumed > HEAD_LIMIT {
        return Err(malformed());
    }
    let target = request.path.ok_or_else(malformed)?;
    if !target.starts_with('/')
        || target.starts_with("//")
        || target.contains('#')
        || target.bytes().any(|b| b.is_ascii_control())
    {
        return Err(malformed());
    }
    let headers = request
        .headers
        .iter()
        .map(|h| {
            Ok((
                h.name.to_owned(),
                std::str::from_utf8(h.value)
                    .map_err(|_| malformed())?
                    .trim()
                    .to_owned(),
            ))
        })
        .collect::<Result<Vec<_>, Reply>>()?;
    let hosts = headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("host"))
        .map(|(_, v)| v.clone())
        .collect();
    let request = Request {
        method: request.method.ok_or_else(malformed)?.into(),
        target: target.into(),
        hosts,
        headers,
        body: Vec::new(),
    };
    if request
        .header("transfer-encoding")
        .map_err(|()| malformed())?
        .is_some()
    {
        return Err(malformed());
    }
    let length = match request.header("content-length").map_err(|()| malformed())? {
        None => 0,
        Some(value) if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) => {
            value.parse::<usize>().map_err(|_| malformed())?
        }
        Some(_) => return Err(malformed()),
    };
    if length > BODY_LIMIT {
        return Err(Reply::refusal(413, "request body exceeds limit"));
    }
    Ok(Some(Head {
        request,
        consumed,
        length,
    }))
}
fn read(stream: &TcpStream, deadline: Instant) -> Result<Request, Reply> {
    let mut bytes = Vec::with_capacity(1024);
    let mut chunk = [0; 4096];
    let mut input = stream;
    let mut receive = |buffer: &mut [u8]| -> Result<usize, Reply> {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(Reply::refusal(400, "request read deadline exceeded"));
        }
        stream
            .set_read_timeout(Some(left))
            .map_err(|_| Reply::refusal(400, "request read failed"))?;
        input
            .read(buffer)
            .map_err(|_| Reply::refusal(400, "request read failed"))
    };
    let mut head = loop {
        if let Some(head) = parse(&bytes)? {
            break head;
        }
        if bytes.len() == HEAD_LIMIT {
            return Err(Reply::refusal(400, "request header exceeds limit"));
        }
        let length = chunk.len().min(HEAD_LIMIT - bytes.len());
        let got = receive(&mut chunk[..length])?;
        if got == 0 {
            return Err(Reply::refusal(400, "incomplete request header"));
        }
        bytes.extend_from_slice(&chunk[..got]);
    };
    let buffered = &bytes[head.consumed..];
    if buffered.len() > head.length {
        return Err(Reply::refusal(400, "unexpected bytes after request"));
    }
    head.request.body.extend_from_slice(buffered);
    while head.request.body.len() < head.length {
        let length = chunk.len().min(head.length - head.request.body.len());
        let got = receive(&mut chunk[..length])?;
        if got == 0 {
            return Err(Reply::refusal(400, "incomplete request body"));
        }
        head.request.body.extend_from_slice(&chunk[..got]);
    }
    Ok(head.request)
}

/// Queue deadline travels with a job so a departed waiter cannot create stale store work.
pub(super) struct Job<T, R> {
    pub(super) request: T,
    pub(super) reply: SyncSender<R>,
    pub(super) deadline: Instant,
}
type Queue<T, R> = (SyncSender<Job<T, R>>, Receiver<Job<T, R>>);

pub(super) fn queue<T, R>() -> Queue<T, R> {
    sync_channel(QUEUE_LIMIT)
}
pub(super) fn alive<T, R>(job: &Job<T, R>, now: Instant) -> bool {
    now < job.deadline
}
struct Count(Arc<AtomicUsize>);
impl Drop for Count {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Run a stateless MCP listener. The handler is invoked only on this thread.
pub(super) fn serve(
    listener: TcpListener,
    authorities: Authorities,
    origins: Vec<String>,
    mut handler: impl FnMut(Request) -> Reply,
) -> Result<String, Failure> {
    if origins.iter().any(|origin| {
        !origin
            .strip_prefix("https://")
            .or_else(|| origin.strip_prefix("http://"))
            .is_some_and(authority)
    }) {
        return Err(Failure::fault(
            "--allow-origin must be an exact HTTP or HTTPS origin",
        ));
    }
    announce(&listener)?;
    let (jobs, requests) = queue();
    std::thread::Builder::new()
        .name("ekr-http-accept".into())
        .spawn(move || {
            let active = Arc::new(AtomicUsize::new(0));
            for accepted in listener.incoming() {
                let Ok(mut stream) = accepted else {
                    continue;
                };
                if active
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                        (n < CONNECTION_LIMIT).then_some(n + 1)
                    })
                    .is_err()
                {
                    let _ = Reply::refusal(503, "busy").write(&mut stream);
                    continue;
                }
                let counted = Count(active.clone());
                let jobs = jobs.clone();
                let authorities = authorities.clone();
                let origins = origins.clone();
                let started = Instant::now();
                let _ = std::thread::Builder::new()
                    .name("ekr-http-connection".into())
                    .spawn(move || {
                        let _counted = counted;
                        connection(stream, started, &jobs, &authorities, &origins);
                    });
            }
        })
        .map_err(|_| Failure::fault("starting HTTP accept thread failed"))?;
    for job in requests {
        if alive(&job, Instant::now()) {
            let answer = handler(job.request);
            let _ = job.reply.try_send(answer);
        }
    }
    Err(Failure::fault("HTTP accept loop stopped"))
}

fn connection(
    mut stream: TcpStream,
    started: Instant,
    jobs: &SyncSender<Job<Request, Reply>>,
    authorities: &Authorities,
    origins: &[String],
) {
    let result = (|| {
        let request = read(&stream, started + READ_TIMEOUT)?;
        if !authorities.accepts(&request.hosts) {
            return Err(Reply::refusal(421, "unapproved Host"));
        }
        match request.header("origin") {
            Ok(None) => {}
            Ok(Some(origin)) if origins.iter().any(|v| v == origin) => {}
            _ => return Err(Reply::refusal(403, "unapproved Origin")),
        }
        if request.target == "/healthz" {
            return Ok(if request.method != "GET" {
                Reply::method("GET")
            } else if !request.body.is_empty() {
                Reply::refusal(400, "health request has a body")
            } else {
                Reply::json(200, b"{\"healthy\":true}".to_vec())
            });
        }
        let deadline = started + READ_TIMEOUT + WAIT_TIMEOUT;
        let (reply, receiver) = sync_channel(1);
        jobs.try_send(Job {
            request,
            reply,
            deadline,
        })
        .map_err(|_| Reply::refusal(503, "request queue is full"))?;
        receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|_| Reply::refusal(503, "request deadline exceeded"))
    })();
    let _ = result.unwrap_or_else(|reply| reply).write(&mut stream);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framing_rejects_ambiguous_lengths_and_transfer_encoding() {
        for headers in [
            "Content-Length: 0\r\nContent-Length: 0\r\n",
            "Content-Length: 1, 1\r\n",
            "Transfer-Encoding: chunked\r\n",
            "Content-Length: +1\r\n",
            "Content-Length: -1\r\n",
        ] {
            let request = format!("POST /mcp HTTP/1.1\r\nHost: localhost\r\n{headers}\r\n");
            assert!(parse(request.as_bytes()).is_err(), "{headers}");
        }
    }
    #[test]
    fn body_bytes_after_the_header_are_preserved() {
        let bytes = b"POST /mcp HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n\r\n{}";
        let head = parse(bytes).unwrap().unwrap();
        assert_eq!(head.length, 2);
        assert_eq!(head.request.target, "/mcp");
        assert_eq!(&bytes[head.consumed..], b"{}");
    }
    #[test]
    fn timed_out_jobs_cannot_grow_the_bounded_queue() {
        let (send, receive) = queue::<(), ()>();
        let now = Instant::now();
        for _ in 0..QUEUE_LIMIT {
            let (reply, _) = sync_channel(1);
            send.try_send(Job {
                request: (),
                reply,
                deadline: now,
            })
            .unwrap();
        }
        let (reply, _) = sync_channel(1);
        assert!(send
            .try_send(Job {
                request: (),
                reply,
                deadline: now
            })
            .is_err());
        let mut executed = 0;
        for job in receive.try_iter() {
            if alive(&job, now) {
                executed += 1;
            }
        }
        assert_eq!(executed, 0);
        let (reply, _) = sync_channel(1);
        assert!(send
            .try_send(Job {
                request: (),
                reply,
                deadline: now + WAIT_TIMEOUT
            })
            .is_ok());
    }

    #[test]
    fn health_bypasses_a_saturated_store_queue() {
        let (jobs, pending) = queue();
        for _ in 0..QUEUE_LIMIT {
            let (reply, _) = sync_channel(1);
            jobs.try_send(Job {
                request: parse(b"GET /readyz HTTP/1.1\r\nHost: localhost\r\n\r\n")
                    .unwrap()
                    .unwrap()
                    .request,
                reply,
                deadline: Instant::now() + WAIT_TIMEOUT,
            })
            .unwrap();
        }
        for (path, status) in [("/healthz", 200), ("/readyz", 503)] {
            let listener = TcpListener::bind("127.0.0.1:0").expect("loopback test listener");
            let address = listener.local_addr().unwrap();
            let jobs = jobs.clone();
            let serving = std::thread::spawn(move || {
                let (stream, _) = listener.accept().unwrap();
                connection(
                    stream,
                    Instant::now(),
                    &jobs,
                    &Authorities::loopback(address.port()),
                    &[],
                );
            });
            let mut client = TcpStream::connect(address).unwrap();
            client
                .set_read_timeout(Some(Duration::from_secs(60)))
                .unwrap();
            write!(client, "GET {path} HTTP/1.1\r\nHost: {address}\r\n\r\n").unwrap();
            let mut answer = String::new();
            client.read_to_string(&mut answer).unwrap();
            serving.join().unwrap();
            assert!(
                answer.starts_with(&format!("HTTP/1.1 {status} ")),
                "{answer}"
            );
        }
        assert_eq!(pending.try_iter().count(), QUEUE_LIMIT);
    }
}
