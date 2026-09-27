//! Adversary cases, pass 2, against `story:ekr-view-server` (`ekr view`) after the rewrite to a
//! `std::net` server with `httparse`.
//!
//! Each case drives the real binary over raw TCP: a head dribbled a byte at a time, bytes the
//! server never reads, a commit from another process while the viewer serves, and SIGINT.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const T_ALICE: &str = "00000000-0000-4000-8000-000000000601";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
    .join("tests/fixtures/retraction")
    .join(name)
}

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn new(backend: &'static str) -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        }
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            fixture("host.json").display().to_string(),
            "--store".to_owned(),
            self.store().display().to_string(),
            "--backend".to_owned(),
            self.backend.to_owned(),
        ];
        args.extend(verb.iter().map(|arg| (*arg).to_owned()));
        args
    }

    fn run(&self, verb: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(self.args(verb))
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    fn ok(&self, verb: &[&str]) -> serde_json::Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: stderr {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn seeded(backend: &'static str) -> Self {
        let world = Self::new(backend);
        let seed = fixture("seed.yaml").display().to_string();
        world.ok(&["seed", &seed]);
        world
    }

    fn serve(&self) -> Served {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(self.args(&["view", "--port", "0"]))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let printed: serde_json::Value = serde_json::from_str(&line).unwrap();
        let address = printed["url"]
            .as_str()
            .unwrap()
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned();
        Served { child, address }
    }
}

struct Served {
    child: Child,
    address: String,
}

impl Drop for Served {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

impl Served {
    fn get(&self, path: &str) -> Vec<u8> {
        let mut stream = TcpStream::connect(&self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.address
        )
        .unwrap();
        let mut out = Vec::new();
        stream.read_to_end(&mut out).unwrap();
        out
    }
}

fn status(raw: &[u8]) -> Option<u16> {
    std::str::from_utf8(raw.split(|b| *b == b'\r').next()?)
        .ok()?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

fn body(raw: &[u8]) -> &[u8] {
    let end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("a response head");
    &raw[end + 4..]
}

/// `docs/cli.md` and the module doc: a head "not complete within 16 KiB or 5 seconds, is 400".
/// The 5 s is a timeout per `read`, so a client that sends one byte a second is never refused and
/// holds a thread and a descriptor for as long as it likes (16 KiB at one byte per 4.9 s is
/// about 22 hours).
#[test]
fn a_head_dribbled_a_byte_a_second_is_refused_within_its_5_s() {
    let world = World::seeded("file");
    let served = world.serve();
    let mut stream = TcpStream::connect(&served.address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(900)))
        .unwrap();
    let started = Instant::now();
    let mut answer = Vec::new();
    let mut buf = [0_u8; 4096];
    // Twelve seconds of `X-Slow: aaaa…`, one byte a second, after a well-formed start.
    write!(
        stream,
        "GET / HTTP/1.1\r\nHost: {}\r\nX-Slow: ",
        served.address
    )
    .unwrap();
    while started.elapsed() < Duration::from_secs(12) {
        if stream.write_all(b"a").is_err() {
            break;
        }
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                answer.extend_from_slice(&buf[..n]);
                break;
            }
            Err(_) => {}
        }
    }
    let waited = started.elapsed();
    assert_eq!(
        status(&answer),
        Some(400),
        "after {waited:?} of a head dribbled a byte a second the server had answered {:?}",
        String::from_utf8_lossy(&answer)
    );
    assert!(
        waited < Duration::from_secs(7),
        "the head was refused only after {waited:?}"
    );
}

/// A client that holds as many dribbling connections as the viewer has descriptors must not stop
/// it answering anybody for longer than the bound the coordinator decided: every connection has
/// 5 s from accept to send its head (then 400), and at most 64 are in flight (one more is 503
/// `busy`). The descriptor limit is lowered to 48 here so the case is cheap. Under that bound a
/// well-formed `GET /projection` sent while 60 connections dribble is answered, 200 or 503, within
/// about 6 s of sending, and once the dribblers are refused the viewer answers 200 again.
#[test]
fn dribbling_connections_up_to_the_descriptor_limit_do_not_starve_a_well_formed_client() {
    let world = World::seeded("file");
    let mut args = vec![
        "-c".to_owned(),
        "ulimit -n 48 && exec \"$0\" \"$@\"".to_owned(),
        env!("CARGO_BIN_EXE_ekr").to_owned(),
    ];
    args.extend(world.args(&["view", "--port", "0"]));
    let mut child = Command::new("sh")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let printed: serde_json::Value = serde_json::from_str(&line).unwrap();
    let address = printed["url"]
        .as_str()
        .unwrap()
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_owned();
    let served = Served { child, address };

    let mut flood: Vec<TcpStream> = (0..60)
        .map(|_| {
            let mut stream = TcpStream::connect(&served.address).unwrap();
            write!(stream, "GET / HTTP/1.1\r\nX-Slow: ").unwrap();
            stream
        })
        .collect();
    let address = served.address.clone();
    let honest = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(1));
        let mut stream = TcpStream::connect(&address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(8)))
            .unwrap();
        let sent = Instant::now();
        write!(
            stream,
            "GET /projection HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut out = Vec::new();
        let read = stream.read_to_end(&mut out);
        (
            read.map(|_| ()).map_err(|error| error.kind()),
            status(&out),
            sent.elapsed(),
        )
    });
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        for stream in &mut flood {
            let _ = stream.write_all(b"a");
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    let (read, answered, waited) = honest.join().unwrap();
    assert!(
        read.is_ok() && matches!(answered, Some(200 | 503)) && waited < Duration::from_millis(6500),
        "decided bound (5 s head deadline from accept, 64 connections in flight, 503 busy over \
         the cap): a well-formed GET /projection sent while 60 connections dribbled a head must \
         be answered 200 or 503 within about 6 s; it got {read:?}, status {answered:?}, after \
         {waited:?}"
    );
    drop(flood);
    std::thread::sleep(Duration::from_secs(1));
    let after = served.get("/projection");
    assert_eq!(
        status(&after),
        Some(200),
        "decided bound: once the dribblers are refused a well-formed GET /projection is 200 again"
    );
}

/// A complete request followed by bytes the server never reads (a second, pipelined request):
/// the server answers the first, then closes a socket with unread input, which on Linux sends RST.
/// A client that has not read the answer yet when the RST lands loses it.
#[test]
fn the_answer_to_a_pipelined_request_survives_the_bytes_the_server_never_read() {
    let world = World::seeded("file");
    let served = world.serve();
    let mut stream = TcpStream::connect(&served.address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let one = format!("GET / HTTP/1.1\r\nHost: {}\r\n\r\n", served.address);
    stream
        .write_all(format!("{one}{one}{one}").as_bytes())
        .unwrap();
    // A client that is busy for a moment before it reads, as any real client may be.
    std::thread::sleep(Duration::from_millis(500));
    let mut answer = Vec::new();
    let read = stream.read_to_end(&mut answer);
    assert!(
        read.is_ok(),
        "reading the answer to the first of three pipelined GETs: {read:?}, {} bytes before it",
        answer.len()
    );
    assert_eq!(status(&answer), Some(200));
}

/// The same for a body the server refuses: the 413 the docs promise must reach the client.
#[test]
fn the_413_for_a_body_reaches_a_client_that_sent_the_body() {
    let world = World::seeded("file");
    let served = world.serve();
    let mut stream = TcpStream::connect(&served.address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let payload = "x".repeat(2048);
    write!(
        stream,
        "POST /projection HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\n\r\n{payload}",
        served.address,
        payload.len()
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(500));
    let mut answer = Vec::new();
    let read = stream.read_to_end(&mut answer);
    assert!(
        read.is_ok(),
        "reading the refusal of a request with a 2 KiB body: {read:?}, {} bytes before it",
        answer.len()
    );
    // A POST to a known path is 405 before the body check; either refusal is fine here.
    assert!(
        matches!(status(&answer), Some(405 | 413)),
        "{:?}",
        String::from_utf8_lossy(&answer)
    );
}

/// The viewer is a reader: while it serves, another process can commit, and the viewer's
/// projection at head then shows the new revision. On both providers.
#[test]
fn a_commit_from_another_process_while_the_viewer_serves_lands_and_is_served() {
    for backend in ["file", "sqlite"] {
        let world = World::seeded(backend);
        let propose = fixture("propose-alice.yaml").display().to_string();
        world.ok(&["propose", &propose]);
        world.ok(&["validate", T_ALICE, "--against", "0"]);
        let served = world.serve();
        let before: serde_json::Value =
            serde_json::from_slice(body(&served.get("/projection"))).unwrap();
        let committed = world.run(&["commit", T_ALICE]);
        assert_eq!(
            committed.status.code(),
            Some(0),
            "{backend}: commit while `ekr view` serves: {}",
            String::from_utf8_lossy(&committed.stderr)
        );
        let after_raw = served.get("/projection");
        assert_eq!(status(&after_raw), Some(200), "{backend}");
        let after: serde_json::Value = serde_json::from_slice(body(&after_raw)).unwrap();
        assert_ne!(
            before, after,
            "{backend}: the head projection did not move after revision 1 was committed"
        );
        assert_eq!(after["meta"]["revision"], 1, "{backend}");
        let at_zero: serde_json::Value =
            serde_json::from_slice(body(&served.get("/projection?revision=0"))).unwrap();
        assert_eq!(at_zero["meta"]["revision"], 0, "{backend}");
        assert_eq!(
            at_zero["nodes"], before["nodes"],
            "{backend}: revision 0 is what the head was"
        );
    }
}

fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                out.insert(path.strip_prefix(root).unwrap().to_path_buf(), Vec::new());
                stack.push(path);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                );
            }
        }
    }
    out
}

/// SIGINT, the documented way to stop it ("serves until interrupted"), ends the process promptly,
/// and a file store it served is byte-for-byte what it was, with every file name the same: the
/// viewer writes nothing, not even a lock it leaves behind.
#[test]
fn sigint_ends_the_viewer_and_leaves_the_file_store_byte_identical() {
    let world = World::seeded("file");
    let before = tree(&world.store());
    let mut served = world.serve();
    assert_eq!(status(&served.get("/projection")), Some(200));
    assert_eq!(status(&served.get("/")), Some(200));
    let killed = Command::new("kill")
        .args(["-INT", &served.child.id().to_string()])
        .status()
        .unwrap();
    assert!(killed.success());
    let started = Instant::now();
    let mut exited = None;
    while started.elapsed() < Duration::from_secs(5) {
        if let Some(status) = served.child.try_wait().unwrap() {
            exited = Some(status);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        exited.is_some(),
        "`ekr view` was still running 5 s after SIGINT"
    );
    assert_eq!(
        tree(&world.store()),
        before,
        "the file store changed while `ekr view` served it"
    );
    // And the store takes a write afterwards.
    let propose = fixture("propose-alice.yaml").display().to_string();
    world.ok(&["propose", &propose]);
}
