//! Adversary cases for `story:data-free-graph-viewer` (unit P, the page half).
//!
//! Each case seeds a store derived from the unit's own `tests/fixtures/view-page/sounding`
//! fixture through the real binary, serves it with `ekr view --port 0`, and reads the DOM a
//! headless Chromium builds from the embedded page. The browser is driven over the DevTools
//! protocol, which serves the page's graph libraries from `tests/fixtures/viewer-libraries/`
//! (`support/viewer_libraries.rs`). Without a local headless Chromium the browser cases say so and
//! return.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

#[path = "support/viewer_libraries.rs"]
mod viewer_libraries;

fn manifest_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
}

fn sounding() -> PathBuf {
    manifest_dir().join("tests/fixtures/view-page/sounding")
}

fn page() -> String {
    std::fs::read_to_string(manifest_dir().join("src/cli/viewer/index.html")).unwrap()
}

/// The sounding fixture's seed with store text replaced: markup in a node name, a type name, an
/// edge-type name, a property name and a text value, and one Integer property value above 2^53.
fn hostile_seed() -> String {
    let seed = std::fs::read_to_string(sounding().join("seed.yaml")).unwrap();
    let replacements = [
        (
            "canonical_name: Kestrel-7",
            "canonical_name: '<i id=\"adv-node\">Kestrel</i>'",
        ),
        ("name: Probe\n", "name: '<b id=\"adv-type\">Probe</b>'\n"),
        (
            "name: SAMPLED_BY\n",
            "name: '<s id=\"adv-edge\">SAMPLED</s>'\n",
        ),
        (
            "name: depth_meters\n",
            "name: '<em id=\"adv-prop\">depth</em>'\n",
        ),
        ("value: NB-1", "value: '<u id=\"adv-value\">NB</u>'"),
        ("value: 55\n", "value: 9007199254740993\n"),
    ];
    let mut out = seed;
    for (from, to) in replacements {
        assert_eq!(
            out.matches(from).count(),
            1,
            "the fixture holds {from:?} once"
        );
        out = out.replace(from, to);
    }
    out
}

struct Store {
    directory: tempfile::TempDir,
}

impl Store {
    fn seeded(seed: &str) -> Self {
        let store = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        let path = store.directory.path().join("seed.yaml");
        std::fs::write(&path, seed).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(store.args(&["seed", &path.display().to_string()]))
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "seed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        store
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            sounding().join("host.json").display().to_string(),
            "--store".to_owned(),
            self.directory.path().join("store").display().to_string(),
            "--backend".to_owned(),
            "file".to_owned(),
        ];
        args.extend(verb.iter().map(|arg| (*arg).to_owned()));
        args
    }

    fn serve(&self) -> Server {
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
        let printed: Value = serde_json::from_str(&line).unwrap();
        let url = printed["url"].as_str().unwrap().to_owned();
        Server { child, url }
    }
}

struct Server {
    child: Child,
    url: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

fn browser() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("EKR_VIEW_BROWSER") {
        return Some(PathBuf::from(path)).filter(|path| path.is_file());
    }
    let cache = PathBuf::from(std::env::var("HOME").ok()?).join(".cache/ms-playwright");
    let mut shells: Vec<PathBuf> = std::fs::read_dir(cache)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("chromium_headless_shell-"))
        })
        .map(|path| path.join("chrome-headless-shell-linux64/chrome-headless-shell"))
        .filter(|path| path.is_file())
        .collect();
    shells.sort();
    shells.pop()
}

/// One browser at a time in this binary: each is held to two cores at the lowest priority.
fn one_browser() -> std::sync::MutexGuard<'static, ()> {
    static BROWSER: Mutex<()> = Mutex::new(());
    BROWSER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// How much virtual time a page gets to load and settle before its DOM is read.
const SETTLE_BUDGET: u32 = 8000;
/// How many runs [`rendered`] gets before a stalled browser fails the case.
const RENDER_ATTEMPTS: usize = 3;
/// How long the page's virtual clock may stand still, with no request in flight, before the run is
/// taken for stalled.
const STALL_WINDOW: Duration = Duration::from_secs(30);
/// The longest one run may take while its clock keeps moving.
const SETTLE_CEILING: Duration = Duration::from_secs(600);

/// The DOM after `url` loads and [`SETTLE_BUDGET`] ms of virtual time pass, as
/// `--virtual-time-budget` gives, from a browser held to two cores at the lowest priority and
/// driven over the DevTools protocol, so that the page's libraries are served from the fixtures. A
/// run whose virtual clock stalls is tried again, up to [`RENDER_ATTEMPTS`] runs.
fn rendered(browser: &Path, url: &str) -> String {
    for _ in 0..RENDER_ATTEMPTS {
        match rendered_once(browser, url) {
            Ok(dom) => return dom,
            Err(stalled) => eprintln!("the browser stalled on {url}: {stalled}"),
        }
    }
    panic!("the browser stalled {RENDER_ATTEMPTS} times on {url}");
}

/// A browser, killed when this is dropped.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        self.0.kill().ok();
        self.0.wait().ok();
    }
}

fn rendered_once(browser: &Path, url: &str) -> Result<String, String> {
    let _one = one_browser();
    let profile = tempfile::tempdir().unwrap();
    // `taskset` and `nice` exec the browser, so the child is the browser itself
    let mut child = Running(
        Command::new("taskset")
            .args(["-c", "0-1", "nice", "-n", "19"])
            .arg(browser)
            .args([
                "--headless",
                "--use-angle=swiftshader",
                "--enable-unsafe-swiftshader",
                "--no-sandbox",
                "--no-first-run",
                "--disable-component-update",
                "--disable-background-networking",
                "--disable-extensions",
                viewer_libraries::UNRESOLVABLE,
                "--remote-debugging-port=0",
                &format!("--user-data-dir={}", profile.path().display()),
                "about:blank",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut lines = BufReader::new(child.0.stderr.take().unwrap());
    let port: u16 = loop {
        let mut line = String::new();
        assert!(
            lines.read_line(&mut line).unwrap() > 0,
            "the browser printed no DevTools address"
        );
        if let Some(rest) = line
            .trim()
            .strip_prefix("DevTools listening on ws://127.0.0.1:")
        {
            break rest.split('/').next().unwrap().parse().unwrap();
        }
    };
    std::thread::spawn(move || std::io::copy(&mut lines, &mut std::io::sink()).ok());
    let tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
    let (socket, _) = tungstenite::client(page_socket(port), tcp).unwrap();
    let mut cdp = Cdp {
        socket,
        next: 0,
        budget_expired: false,
        in_flight: BTreeSet::new(),
        log: Vec::new(),
    };
    cdp.call("Runtime.enable", json!({}));
    cdp.call("Log.enable", json!({}));
    cdp.call("Network.enable", json!({}));
    viewer_libraries::serve(|method, params| cdp.call(method, params));
    cdp.call("Page.navigate", json!({ "url": url }));
    // the policy `--virtual-time-budget` sets, once the page is there: the clock waits while a
    // request is in flight
    cdp.call(
        "Emulation.setVirtualTimePolicy",
        json!({"policy": "pauseIfNetworkFetchesPending", "budget": SETTLE_BUDGET}),
    );
    let start = Instant::now();
    let (mut clock, mut moved) = (Value::Null, Instant::now());
    while !cdp.budget_expired {
        // `performance.now()` reads virtual time: a slow run on a loaded machine keeps it moving
        let now = cdp.eval("performance.now()");
        if now != clock || !cdp.in_flight.is_empty() {
            (clock, moved) = (now, Instant::now());
        }
        if moved.elapsed() >= STALL_WINDOW || start.elapsed() >= SETTLE_CEILING {
            return Err(format!(
                "the page's virtual clock stood at {clock} ms of {SETTLE_BUDGET}, {:?} into the run; its log:\n{}",
                start.elapsed(),
                cdp.log.join("\n")
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Ok(cdp
        .eval("document.documentElement.outerHTML")
        .as_str()
        .unwrap_or_default()
        .to_owned())
}

/// The address of the DevTools socket of the browser's page target, once it lists one.
fn page_socket(port: u16) -> String {
    let start = Instant::now();
    loop {
        let mut list = TcpStream::connect(("127.0.0.1", port)).unwrap();
        list.set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        write!(
            list,
            "GET /json/list HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        // the DevTools server may keep the connection open: the body is read by its length
        let mut listed = BufReader::new(list);
        let mut length = 0;
        loop {
            let mut line = String::new();
            listed.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().unwrap();
                }
            }
        }
        let mut body = vec![0_u8; length];
        listed.read_exact(&mut body).unwrap();
        let targets: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        if let Some(address) = targets
            .as_array()
            .into_iter()
            .flatten()
            .find(|target| target["type"] == "page")
            .and_then(|target| target["webSocketDebuggerUrl"].as_str())
        {
            return address.to_owned();
        }
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "the browser listed no page target: {targets}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// One DevTools protocol connection to a page.
struct Cdp {
    socket: tungstenite::WebSocket<TcpStream>,
    next: u64,
    /// Whether the virtual-time budget has run out.
    budget_expired: bool,
    /// The page's requests sent and not yet finished or failed, by request id.
    in_flight: BTreeSet<String>,
    /// The page's requests, library answers, log entries and exceptions, one line each.
    log: Vec<String>,
}

impl Cdp {
    /// The result of `method`; the events before its reply are read, a paused library request
    /// among them answered from the fixtures.
    fn call(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = self.next;
        self.send(&json!({"id": id, "method": method, "params": params}));
        loop {
            let tungstenite::Message::Text(text) = self.socket.read().unwrap() else {
                continue;
            };
            let value: Value = serde_json::from_str(&text).unwrap();
            if value["id"] == id {
                assert!(value.get("error").is_none(), "{method}: {value}");
                return value["result"].clone();
            }
            let params = &value["params"];
            let text = |value: &Value| value.as_str().unwrap_or_default().to_owned();
            match value["method"].as_str() {
                Some("Fetch.requestPaused") => {
                    if let Some((answer, answered)) = viewer_libraries::answer(params) {
                        self.log.push(format!(
                            "library {answer} {}",
                            text(&params["request"]["url"])
                        ));
                        self.next += 1;
                        self.send(&json!({"id": self.next, "method": answer, "params": answered}));
                    }
                }
                Some("Emulation.virtualTimeBudgetExpired") => self.budget_expired = true,
                Some("Network.requestWillBeSent") => {
                    self.log
                        .push(format!("request {}", text(&params["request"]["url"])));
                    self.in_flight.insert(text(&params["requestId"]));
                }
                Some("Network.loadingFinished" | "Network.loadingFailed") => {
                    self.in_flight.remove(&text(&params["requestId"]));
                }
                Some("Log.entryAdded") => self.log.push(format!(
                    "log.{}: {} {}",
                    text(&params["entry"]["level"]),
                    text(&params["entry"]["text"]),
                    text(&params["entry"]["url"])
                )),
                Some("Runtime.exceptionThrown") => {
                    self.log
                        .push(format!("exception: {}", params["exceptionDetails"]));
                }
                _ if value.get("error").is_some() => {
                    self.log.push(format!("protocol error: {value}"));
                }
                _ => {}
            }
        }
    }

    fn send(&mut self, command: &Value) {
        self.socket
            .send(tungstenite::Message::Text(command.to_string().into()))
            .unwrap();
    }

    /// The value `expression` evaluates to in the page.
    fn eval(&mut self, expression: &str) -> Value {
        self.call(
            "Runtime.evaluate",
            json!({"expression": expression, "returnByValue": true}),
        )["result"]["value"]
            .clone()
    }
}

const MARKERS: [(&str, &str); 5] = [
    ("i", "adv-node"),
    ("b", "adv-type"),
    ("s", "adv-edge"),
    ("em", "adv-prop"),
    ("u", "adv-value"),
];

/// Store text holding markup is shown as text in the index, a node's detail and the schema view,
/// and never becomes an element.
#[test]
fn markup_in_store_text_is_shown_as_text_and_never_becomes_an_element() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let store = Store::seeded(&hostile_seed());
    let server = store.serve();
    let mut shown = Vec::new();
    for query in [
        "",
        "#node=00000000-0000-4000-8000-00000000a301",
        "#node=00000000-0000-4000-8000-00000000a311",
        "#node=00000000-0000-4000-8000-00000000a321",
        "#schema=1",
        "#view=3d",
    ] {
        let dom = rendered(&browser, &format!("{}{query}", server.url));
        for (tag, marker) in MARKERS {
            assert!(
                !dom.contains(&format!("<{tag} id=\"{marker}\"")),
                "{query}: store text became an element {marker}: {dom}"
            );
            if dom.contains(&format!("&lt;{tag} id=\"{marker}\"&gt;")) {
                shown.push(marker);
            }
        }
    }
    for (_, marker) in MARKERS {
        assert!(
            shown.contains(&marker),
            "{marker} was never shown as text, so the check proved nothing"
        );
    }
}

/// `ekr.graph-projection/1` carries an Integer as a JSON integer (views.yaml § ProjectedValue),
/// and the store holds i64. A node's property table must show the value the store holds, not the
/// nearest double.
#[test]
fn an_integer_above_two_to_the_fifty_three_is_shown_as_the_store_holds_it() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let store = Store::seeded(&hostile_seed());
    let server = store.serve();
    let (status, body) = get(&server, "/projection");
    assert_eq!(status, 200);
    assert!(
        String::from_utf8_lossy(&body).contains("9007199254740993"),
        "the projection carries the exact integer"
    );
    let dom = rendered(
        &browser,
        &format!("{}#node=00000000-0000-4000-8000-00000000a312", server.url),
    );
    assert!(
        dom.contains("adv-prop"),
        "the node's property table is shown: {dom}"
    );
    assert!(
        dom.contains("9007199254740993"),
        "the page shows {} instead of the stored 9007199254740993",
        if dom.contains("9007199254740992") {
            "9007199254740992"
        } else {
            "neither"
        }
    );
}

/// The page reads only `/projection`, `/roles` and `/evidence/<id>`. An `evidence` URL parameter
/// of `..` must not make it read any other address: `/evidence/..` resolves to `/`.
#[test]
fn an_evidence_parameter_of_dot_dot_reads_no_address_but_the_three() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let seed = std::fs::read_to_string(sounding().join("seed.yaml")).unwrap();
    let store = Store::seeded(&seed);
    let server = store.serve();
    let page_bytes = page().len();
    let dom = rendered(&browser, &format!("{}#evidence=..", server.url));
    assert!(
        dom.contains("evidence"),
        "the evidence panel is shown: {dom}"
    );
    assert!(
        !dom.contains(&format!("{page_bytes} bytes that are not text")),
        "#evidence=.. made the page read `/` (the page itself, {page_bytes} bytes) as evidence"
    );
}

fn get(server: &Server, path: &str) -> (u16, Vec<u8>) {
    let address = server
        .url
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_owned();
    let mut stream = std::net::TcpStream::connect(&address).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let end = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&raw[..end]).into_owned();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, raw[end + 4..].to_vec())
}
