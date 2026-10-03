//! Adversary pass 1 on `task:viewer-compact-mode` and the shift+click type solo (`4eb71965`), both in
//! `crates/ekr/src/cli/viewer/index.html`.
//!
//! Every case seeds the `view-page/sounding` fixture through the real binary and serves it with the
//! real `ekr view`. A front server answers `/` with the embedded page and forwards every other
//! request to `ekr view` unchanged, so a case can serve a mutated copy of the page, held in memory
//! only (`EKR_ADVERSARY_MUTANT`), to show that it fails on the defect it names. The browser is
//! driven over the DevTools protocol by a local copy of the `Driven` harness in `view_page.rs`,
//! which is private to that file; the copy adds modifier keys, an init script and window metrics.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{json, Value};

fn manifest_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"))
}

/// The embedded page, or a mutant of it named by `EKR_ADVERSARY_MUTANT`, changed in memory.
fn page() -> String {
    let page = std::fs::read_to_string(manifest_dir().join("src/cli/viewer/index.html")).unwrap();
    let mutate = |from: &str, to: &str| {
        assert!(page.contains(from), "the mutant's anchor is in the page");
        page.replacen(from, to, 1)
    };
    match std::env::var("EKR_ADVERSARY_MUTANT").as_deref() {
        // the address's compact state is read only by `applyState`, after the first draw
        Ok("late-fold") => mutate(
            "  setFolds(...foldsFrom(location.hash));\n  $(\"statusText\")",
            "  $(\"statusText\")",
        ),
        // `compact=left` is never read from the address
        Ok("no-left-read") => mutate("[c === \"1\" || c === \"left\",", "[c === \"1\","),
        // the left tab collapses nothing
        Ok("left-tab-dead") => mutate(
            "$(\"foldLeft\").addEventListener(\"click\", () => fold(true, folded(\"right\")));",
            "",
        ),
        _ => page,
    }
}

// ---- the store and the servers -----------------------------------------------------------------

struct Seeded {
    directory: tempfile::TempDir,
}

impl Seeded {
    fn fixture() -> PathBuf {
        manifest_dir().join("tests/fixtures/view-page/sounding")
    }

    fn new() -> Self {
        let seeded = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        let seed = Self::fixture().join("seed.yaml").display().to_string();
        let output = Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(seeded.args(&["seed", &seed]))
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "seed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        seeded
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            Self::fixture().join("host.json").display().to_string(),
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
        let url = printed["url"].as_str().unwrap();
        let address = url
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned();
        Server { child, address }
    }
}

struct Server {
    child: Child,
    address: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

/// Answers `/` with `page()` and forwards every other request to `ekr view`.
struct Front {
    url: String,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    _server: Server,
    _seeded: Seeded,
}

impl Front {
    fn start(seeded: Seeded) -> Self {
        let server = seeded.serve();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let halt = Arc::clone(&stop);
        let upstream = Arc::new(server.address.clone());
        let body = Arc::new(page().into_bytes());
        let thread = std::thread::spawn(move || {
            for connection in listener.incoming() {
                if halt.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(connection) = connection else { continue };
                let (upstream, body) = (Arc::clone(&upstream), Arc::clone(&body));
                std::thread::spawn(move || front_answer(connection, &upstream, &body));
            }
        });
        Self {
            url,
            stop,
            thread: Some(thread),
            _server: server,
            _seeded: seeded,
        }
    }
}

impl Drop for Front {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let address = self.url.trim_start_matches("http://").trim_end_matches('/');
        TcpStream::connect(address).ok();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}

fn front_answer(mut stream: TcpStream, upstream: &str, page: &[u8]) {
    stream.set_read_timeout(Some(Duration::from_secs(60))).ok();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request = String::new();
    if reader.read_line(&mut request).unwrap_or(0) == 0 {
        return;
    }
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
            break;
        }
    }
    let target = request.split_whitespace().nth(1).unwrap_or("").to_owned();
    if target.split('?').next() == Some("/") {
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
             Cache-Control: no-store\r\nConnection: close\r\n\r\n",
            page.len()
        );
        stream.write_all(head.as_bytes()).ok();
        stream.write_all(page).ok();
        return;
    }
    let Ok(mut up) = TcpStream::connect(upstream) else {
        return;
    };
    write!(
        up,
        "GET {target} HTTP/1.1\r\nHost: {upstream}\r\nConnection: close\r\n\r\n"
    )
    .ok();
    let mut raw = Vec::new();
    up.read_to_end(&mut raw).ok();
    stream.write_all(&raw).ok();
}

// ---- the browser -------------------------------------------------------------------------------

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

fn one_browser() -> std::sync::MutexGuard<'static, ()> {
    static BROWSER: Mutex<()> = Mutex::new(());
    BROWSER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// DevTools may announce its listener before the initial page target exists.
fn page_target(port: u16) -> String {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        let mut list = TcpStream::connect(("127.0.0.1", port)).unwrap();
        list.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        write!(
            list,
            "GET /json/list HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
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
        let targets: Value = serde_json::from_slice(&body).unwrap();
        let target = targets
            .as_array()
            .unwrap()
            .iter()
            .find(|target| target["type"] == "page");
        if let Some(target) = target {
            return target["webSocketDebuggerUrl"].as_str().unwrap().to_owned();
        }
        if std::time::Instant::now() >= deadline {
            panic!("no page target within discovery deadline");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn discovery_waits_for_a_page_after_the_devtools_listener_is_ready() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let address = format!("ws://127.0.0.1:{port}/devtools/page/fixture");
    let expected = address.clone();
    let server = std::thread::spawn(move || {
        for targets in [
            json!([]),
            json!([{"type": "service_worker"}]),
            json!([{"type": "page", "webSocketDebuggerUrl": address}]),
        ] {
            let (mut connection, _) = listener.accept().unwrap();
            let mut request = BufReader::new(connection.try_clone().unwrap());
            let mut line = String::new();
            request.read_line(&mut line).unwrap();
            assert_eq!(line, "GET /json/list HTTP/1.1\r\n");
            loop {
                line.clear();
                assert!(request.read_line(&mut line).unwrap() > 0);
                if line == "\r\n" {
                    break;
                }
            }
            let body = targets.to_string();
            write!(
                connection,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    assert_eq!(page_target(port), expected);
    server.join().unwrap();
}

struct BrowserChild(Child);

impl Drop for BrowserChild {
    fn drop(&mut self) {
        self.0.kill().ok();
        self.0.wait().ok();
    }
}

/// The `Driven` harness of `view_page.rs`, copied (it is private to that file).
struct Driven {
    _child: BrowserChild,
    socket: TcpStream,
    reader: BufReader<TcpStream>,
    next: u64,
    errors: Vec<Value>,
    _profile: tempfile::TempDir,
    _one: std::sync::MutexGuard<'static, ()>,
}

impl Driven {
    /// Starts the browser, runs `init` in every document before its own scripts, and opens `url`.
    fn launch(browser: &Path, url: &str, init: Option<&str>) -> Self {
        let one = one_browser();
        let profile = tempfile::tempdir().unwrap();
        let child = BrowserChild(
            Command::new(browser)
                .args([
                    "--headless",
                    "--use-angle=swiftshader",
                    "--enable-unsafe-swiftshader",
                    "--no-sandbox",
                    "--no-first-run",
                    "--disable-extensions",
                    "--window-size=1600,1000",
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
        let mut child = child;
        let mut lines = BufReader::new(child.0.stderr.take().unwrap());
        let port: u16 = loop {
            let mut line = String::new();
            assert!(
                lines.read_line(&mut line).unwrap() > 0,
                "no DevTools address"
            );
            if let Some(rest) = line
                .trim()
                .strip_prefix("DevTools listening on ws://127.0.0.1:")
            {
                break rest.split('/').next().unwrap().parse().unwrap();
            }
        };
        std::thread::spawn(move || std::io::copy(&mut lines, &mut std::io::sink()).ok());
        let address = page_target(port);
        let path = &address[address.find("/devtools/").unwrap()..];
        let mut socket = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(
            socket,
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: AAAAAAAAAAAAAAAAAAAAAA==\r\n\
             Sec-WebSocket-Version: 13\r\n\r\n"
        )
        .unwrap();
        let mut reader = BufReader::new(socket.try_clone().unwrap());
        let mut status = String::new();
        reader.read_line(&mut status).unwrap();
        assert!(status.contains(" 101 "), "the DevTools socket: {status}");
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
        }
        let mut driven = Self {
            _child: child,
            socket,
            reader,
            next: 0,
            errors: Vec::new(),
            _profile: profile,
            _one: one,
        };
        driven.call("Runtime.enable", json!({}));
        driven.call("Log.enable", json!({}));
        driven.call("Page.enable", json!({}));
        if let Some(source) = init {
            driven.call(
                "Page.addScriptToEvaluateOnNewDocument",
                json!({ "source": source }),
            );
        }
        driven.call("Page.navigate", json!({ "url": url }));
        driven
    }

    fn send(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let mut frame = vec![0x81_u8];
        if bytes.len() < 126 {
            frame.push(0x80 | u8::try_from(bytes.len()).unwrap());
        } else if let Ok(medium) = u16::try_from(bytes.len()) {
            frame.push(0x80 | 126);
            frame.extend(medium.to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend(u64::try_from(bytes.len()).unwrap().to_be_bytes());
        }
        frame.extend([0_u8; 4]);
        frame.extend(bytes);
        self.socket.write_all(&frame).unwrap();
    }

    fn receive(&mut self) -> String {
        let mut message = Vec::new();
        loop {
            let mut head = [0_u8; 2];
            self.reader.read_exact(&mut head).unwrap();
            let (last, opcode) = (head[0] & 0x80 != 0, head[0] & 0x0f);
            let mut length = u64::from(head[1] & 0x7f);
            if length == 126 {
                let mut more = [0_u8; 2];
                self.reader.read_exact(&mut more).unwrap();
                length = u64::from(u16::from_be_bytes(more));
            } else if length == 127 {
                let mut more = [0_u8; 8];
                self.reader.read_exact(&mut more).unwrap();
                length = u64::from_be_bytes(more);
            }
            let mut payload = vec![0_u8; usize::try_from(length).unwrap()];
            self.reader.read_exact(&mut payload).unwrap();
            match opcode {
                0x8 => panic!("the browser closed the DevTools socket"),
                0x9 => {
                    let mut pong = vec![0x8a_u8, 0x80 | u8::try_from(payload.len()).unwrap()];
                    pong.extend([0_u8; 4]);
                    pong.extend(&payload);
                    self.socket.write_all(&pong).unwrap();
                }
                _ => {
                    message.extend(payload);
                    if last {
                        return String::from_utf8(message).unwrap();
                    }
                }
            }
        }
    }

    fn call(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = self.next;
        self.send(&json!({"id": id, "method": method, "params": params}).to_string());
        loop {
            let reply: Value = serde_json::from_str(&self.receive()).unwrap();
            if reply["id"] == id {
                return reply;
            }
            if reply["method"] == "Runtime.exceptionThrown"
                || (reply["method"] == "Runtime.consoleAPICalled"
                    && reply["params"]["type"] == "error")
                || (reply["method"] == "Log.entryAdded"
                    && reply["params"]["entry"]["level"] == "error")
            {
                self.errors.push(reply["params"].clone());
            }
        }
    }

    fn eval(&mut self, expression: &str) -> Value {
        let reply = self.call(
            "Runtime.evaluate",
            json!({"expression": expression, "awaitPromise": true, "returnByValue": true}),
        );
        assert!(
            reply["result"].get("exceptionDetails").is_none(),
            "{expression}: {reply}"
        );
        reply["result"]["result"]["value"].clone()
    }

    fn wait_for(&mut self, expression: &str, seconds: u64) -> bool {
        for _ in 0..seconds * 5 {
            if self.eval(&format!(
                "(() => {{ try {{ return {expression}; }} catch {{ return false; }} }})()"
            )) == Value::Bool(true)
            {
                return true;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        false
    }

    /// One press of a printable key with `modifiers` (1 Alt, 2 Ctrl, 4 Meta, 8 Shift), sent to
    /// whatever holds the focus.
    fn key(&mut self, key: &str, code: &str, modifiers: u8) {
        let text = if modifiers & 0b0110 == 0 { key } else { "" };
        self.call(
            "Input.dispatchKeyEvent",
            json!({"type": "keyDown", "key": key, "code": code, "text": text,
                "unmodifiedText": key, "modifiers": modifiers}),
        );
        self.call(
            "Input.dispatchKeyEvent",
            json!({"type": "keyUp", "key": key, "code": code, "modifiers": modifiers}),
        );
    }

    /// A left click at the centre of the element `selector` names, with `modifiers`.
    fn click(&mut self, selector: &str, modifiers: u8) {
        let at = self.eval(&format!(
            "(r => [r.x + r.width / 2, r.y + r.height / 2])(document.querySelector({selector:?}).getBoundingClientRect())"
        ));
        let (x, y) = (at[0].as_f64().unwrap(), at[1].as_f64().unwrap());
        for kind in ["mousePressed", "mouseReleased"] {
            self.call(
                "Input.dispatchMouseEvent",
                json!({"type": kind, "x": x, "y": y, "button": "left", "buttons": 1,
                    "clickCount": 1, "modifiers": modifiers}),
            );
        }
    }

    fn metrics(&mut self, width: u32, height: u32) {
        self.call(
            "Emulation.setDeviceMetricsOverride",
            json!({"width": width, "height": height, "deviceScaleFactor": 1, "mobile": false}),
        );
    }

    /// Bounded browser-native observations; no additional page script is injected or evaluated.
    fn sidebar_diagnostic(&mut self) -> Value {
        self.call("DOM.enable", json!({}));
        self.call("CSS.enable", json!({}));
        let document = self.call("DOM.getDocument", json!({"depth": 0}));
        let root = &document["result"]["root"]["nodeId"];
        let mut nodes = Vec::new();
        for selector in ["aside.left", "#panel", "#run", "#statusText", "#subline"] {
            let found = self.call(
                "DOM.querySelector",
                json!({"nodeId": root, "selector": selector}),
            );
            let node = &found["result"]["nodeId"];
            let computed = self.call("CSS.getComputedStyleForNode", json!({"nodeId": node}));
            let styles: Vec<_> = computed["result"]["computedStyle"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|property| {
                    matches!(
                        property["name"].as_str(),
                        Some(
                            "display"
                                | "width"
                                | "height"
                                | "font-family"
                                | "font-size"
                                | "line-height"
                                | "overflow-x"
                                | "overflow-y"
                                | "overflow-anchor"
                                | "scrollbar-width"
                                | "scrollbar-gutter"
                        )
                    )
                })
                .cloned()
                .collect();
            nodes.push(json!({
                "selector": selector,
                "style": styles,
                "box": self.call("DOM.getBoxModel", json!({"nodeId": node})),
                "fonts": self.call("CSS.getPlatformFontsForNode", json!({"nodeId": node})),
            }));
        }
        let accessibility = self.call("Accessibility.getFullAXTree", json!({}));
        let focused: Vec<_> = accessibility["result"]["nodes"]
            .as_array().into_iter().flatten()
            .filter(|node| node["properties"].as_array().is_some_and(|properties| {
                properties.iter().any(|property| property["name"] == "focused" && property["value"]["value"] == true)
            }))
            .map(|node| json!({"backendNode": node["backendDOMNodeId"], "role": node["role"]["value"]}))
            .collect();
        json!({
            "browser": self.call("Browser.getVersion", json!({}))["result"],
            "viewport": self.call("Page.getLayoutMetrics", json!({}))["result"],
            "nodes": nodes,
            "focused": focused,
        })
    }

    fn no_errors(&self) {
        assert!(
            self.errors.is_empty(),
            "the page reported errors: {:?}",
            self.errors
        );
    }
}

const SHIFT: u8 = 8;
const CTRL: u8 = 2;

const SETTLED: &str = "!!window.__viewer && !window.__viewer.layoutRunning \
     && !document.querySelector('[data-act=stream-stop]') && !!window.__viewer.renderer";

/// What the reader sees: the widths of the canvas, the stage, each sidebar and strip, the address.
const MEASURE: &str = "(() => { const shown = e => e && getComputedStyle(e).display !== 'none' ? e.getBoundingClientRect().width : 0;
  const c = __viewer.fg ? __viewer.fg.renderer().domElement : document.querySelector('#graph canvas');
  return {canvas: c.getBoundingClientRect().width, stage: document.getElementById('stage').getBoundingClientRect().width,
    left: shown(document.querySelector('aside.left')), right: shown(document.getElementById('panel')),
    leftStrip: shown(document.getElementById('leftStrip')), rightStrip: shown(document.getElementById('rightStrip')),
    rightEdge: document.getElementById('panel').getBoundingClientRect().right,
    hash: decodeURIComponent(location.hash), search: document.getElementById('search').value}; })()";

fn measured_when(driven: &mut Driven, condition: &str, what: &str) -> Value {
    let expression = format!("(m => {condition})({MEASURE})");
    assert!(
        driven.wait_for(&expression, 30),
        "{what}: {}",
        driven.eval(MEASURE)
    );
    driven.eval(MEASURE)
}

/// A page on the sounding store at `hash`, settled in 2D.
fn opened(browser: &Path, hash: &str, init: Option<&str>) -> (Front, Driven) {
    let seeded = Seeded::new();
    let front = Front::start(seeded);
    let mut driven = Driven::launch(browser, &format!("{}{hash}", front.url), init);
    assert!(driven.wait_for(SETTLED, 90), "the page settled");
    (front, driven)
}

fn chip_types(driven: &mut Driven, container: &str) -> Vec<String> {
    driven
        .eval(&format!(
            "[...document.querySelectorAll('#{container} .chip')].map(c => c.dataset.type)"
        ))
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap().to_owned())
        .collect()
}

fn off_chips(driven: &mut Driven, container: &str) -> Vec<String> {
    driven
        .eval(&format!(
            "[...document.querySelectorAll('#{container} .chip.off')].map(c => c.dataset.type)"
        ))
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap().to_owned())
        .collect()
}

macro_rules! need_browser {
    () => {
        match browser() {
            Some(browser) => browser,
            None => {
                eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
                return;
            }
        }
    };
}

// ---- keyboard ----------------------------------------------------------------------------------

/// `c` typed into the search box types a `c` and leaves the sidebars; Ctrl+C and Alt+C toggle
/// nothing; Shift+C (a capital) toggles as `c` does, as Shift+F fits.
#[test]
fn c_in_the_search_box_or_with_ctrl_or_alt_does_not_toggle_compact() {
    let browser = need_browser!();
    let (_front, mut driven) = opened(&browser, "#view=2d", None);
    driven.eval("document.getElementById('search').focus()");
    driven.key("c", "KeyC", 0);
    std::thread::sleep(Duration::from_millis(600));
    let m = driven.eval(MEASURE);
    assert!(
        m["left"].as_f64().unwrap() > 0.0 && m["right"].as_f64().unwrap() > 0.0,
        "typing c in the search box collapsed a sidebar: {m}"
    );
    assert_eq!(m["search"], "c", "{m}");
    driven.eval("document.activeElement.blur(); document.getElementById('search').value = ''");
    driven.key("c", "KeyC", CTRL);
    driven.key("c", "KeyC", 1);
    std::thread::sleep(Duration::from_millis(600));
    let m = driven.eval(MEASURE);
    assert!(
        m["left"].as_f64().unwrap() > 0.0 && !m["hash"].as_str().unwrap().contains("compact"),
        "Ctrl+C or Alt+C toggled compact: {m}"
    );
    driven.key("C", "KeyC", SHIFT);
    measured_when(
        &mut driven,
        "m.left === 0 && m.right === 0 && m.hash.includes('compact=1')",
        "Shift+C toggles as c does",
    );
    driven.no_errors();
}

// ---- the address -------------------------------------------------------------------------------

/// `compact=left` from the address collapses the left sidebar alone, and the left tab writes it.
/// The unit's own cases never read or write `compact=left` and never press the left tab: the mutants
/// `no-left-read` and `left-tab-dead` pass `view_page.rs` and fail here.
#[test]
fn the_address_compact_left_and_the_left_tab_collapse_the_left_sidebar_alone() {
    let browser = need_browser!();
    let (_front, mut driven) = opened(&browser, "#view=2d&compact=left", None);
    let m = measured_when(
        &mut driven,
        "m.left === 0 && m.leftStrip > 0 && m.right > 0 && m.rightStrip === 0 && Math.abs(m.canvas - m.stage) < 1",
        "compact=left in the address collapsed the left sidebar alone",
    );
    assert!(m["hash"].as_str().unwrap().contains("compact=left"), "{m}");
    driven.eval("document.getElementById('leftStrip').click()");
    measured_when(
        &mut driven,
        "m.left > 0 && m.right > 0 && !m.hash.includes('compact')",
        "the left strip restored it",
    );
    driven.click("#foldLeft", 0);
    let m = measured_when(
        &mut driven,
        "m.left === 0 && m.leftStrip > 0 && m.right > 0",
        "the left tab collapsed the left sidebar",
    );
    assert!(m["hash"].as_str().unwrap().contains("compact=left"), "{m}");
    driven.no_errors();
}

/// The address's compact state holds from the page's first read, not only once the graph is
/// drawn: the sidebars are already collapsed when the page asks for its first data. The unit's
/// own reload case passes the `late-fold` mutant (compact applied only by `applyState`, after the
/// first draw); this case fails on it.
#[test]
fn a_compact_address_holds_before_the_page_reads_or_draws_anything() {
    let browser = need_browser!();
    let init = "window.__atFirstRead = null; const own = window.fetch;
      window.fetch = (...a) => { if (window.__atFirstRead === null) window.__atFirstRead = document.body.className; return own(...a); };";
    let (_front, mut driven) = opened(&browser, "#view=2d&compact=1", Some(init));
    let classes = driven.eval("window.__atFirstRead");
    let classes = classes.as_str().unwrap_or_default();
    assert!(
        classes.contains("left-min") && classes.contains("right-min"),
        "at the first read the body's classes were {classes:?}"
    );
    driven.no_errors();
}

/// A `compact` value the page does not write shows both sidebars, and the address it rewrites
/// carries no `compact`; of two `compact` keys the first holds.
#[test]
fn a_malformed_or_repeated_compact_value_is_read_as_the_page_writes_it() {
    let browser = need_browser!();
    for (hash, left, right, written) in [
        ("#view=2d&compact=0", false, false, None),
        ("#view=2d&compact=LEFT", false, false, None),
        ("#view=2d&compact=", false, false, None),
        ("#view=2d&compact=both", false, false, None),
        (
            "#view=2d&compact=right&compact=left",
            false,
            true,
            Some("compact=right"),
        ),
        ("#compact=1&compact=0", true, true, Some("compact=1")),
    ] {
        let (_front, mut driven) = opened(&browser, hash, None);
        let condition = format!(
            "(m.left === 0) === {left} && (m.right === 0) === {right} && Math.abs(m.canvas - m.stage) < 1"
        );
        let m = measured_when(&mut driven, &condition, hash);
        let written_hash = m["hash"].as_str().unwrap();
        match written {
            Some(value) => assert!(written_hash.contains(value), "{hash}: {m}"),
            None => assert!(!written_hash.contains("compact"), "{hash}: {m}"),
        }
        driven.no_errors();
    }
}

/// `compact` beside every other parameter: the focus trail, its hops, the types shown, a node,
/// the 3D view; a reload keeps them all, and back and forward through compact and a type toggle
/// restore each address.
#[test]
fn compact_beside_every_other_parameter_survives_a_reload_and_history() {
    let browser = need_browser!();
    let (front, mut driven) = opened(&browser, "#view=2d", None);
    let node = driven.eval("__viewer.graph.nodes().sort((a, b) => __viewer.graph.degree(b) - __viewer.graph.degree(a))[0]");
    let node = node.as_str().unwrap().to_owned();
    let types = chip_types(&mut driven, "nodeTypes");
    assert!(types.len() >= 2, "{types:?}");
    let shown = types[..types.len() - 1].join(",");
    drop(driven);
    let hash = format!("#view=3d&compact=right&node={node}&types={shown}&focus={node}&hops=2");
    let mut driven = Driven::launch(&browser, &format!("{}{hash}", front.url), None);
    assert!(
        driven.wait_for(
            "!!window.__viewer && !!window.__viewer.fg && !!document.querySelector('#panel h1')",
            90
        ),
        "the 3D view opened"
    );
    let m = measured_when(
        &mut driven,
        "m.left > 0 && m.right === 0 && m.rightStrip > 0 && Math.abs(m.canvas - m.stage) < 1",
        "compact=right beside the rest",
    );
    let written = m["hash"].as_str().unwrap().to_owned();
    for part in [
        "view=3d",
        "compact=right",
        &format!("node={node}"),
        &format!("types={shown}"),
        &format!("focus={node}"),
        "hops=2",
    ] {
        assert!(written.contains(part), "{part} is kept: {written}");
    }
    // a type shown again, then compact, then back twice and forward twice (a chip is clicked while
    // its sidebar is shown: a collapsed sidebar's chips cannot be reached)
    assert!(
        driven.wait_for(
            "!document.querySelector('[data-act=stream-stop]') && !window.__viewer.layoutRunning",
            60
        ),
        "the streams ended"
    );
    let hidden = types.last().unwrap().clone();
    driven.click(&format!("#nodeTypes .chip[data-type='{hidden}']"), 0);
    measured_when(
        &mut driven,
        "!m.hash.includes('types=') && m.hash.includes('compact=right')",
        "every type shown",
    );
    driven.key("c", "KeyC", 0);
    measured_when(
        &mut driven,
        "m.hash.includes('compact=1') && m.left === 0 && m.right === 0",
        "c",
    );
    driven.eval("history.back()");
    measured_when(
        &mut driven,
        "m.hash.includes('compact=right') && !m.hash.includes('types=') && m.left > 0 && m.right === 0 \
         && Math.abs(m.canvas - m.stage) < 1",
        "back: compact=right with every type",
    );
    assert!(off_chips(&mut driven, "nodeTypes").is_empty());
    driven.eval("history.back()");
    measured_when(
        &mut driven,
        &format!(
            "m.hash.includes('compact=right') && m.hash.includes('types={shown}') && m.left > 0 && m.right === 0"
        ),
        "back: compact=right with the type hidden",
    );
    assert!(
        driven.wait_for(
            &format!(
                "document.querySelector(\"#nodeTypes .chip[data-type='{hidden}']\").classList.contains('off')"
            ),
            20
        ),
        "the type is hidden again"
    );
    driven.eval("history.forward()");
    driven.eval("history.forward()");
    measured_when(
        &mut driven,
        "m.hash.includes('compact=1') && !m.hash.includes('types=') && m.left === 0 && m.right === 0 \
         && Math.abs(m.canvas - m.stage) < 1",
        "forward twice: compact with every type",
    );
    assert!(off_chips(&mut driven, "nodeTypes").is_empty());
    driven.no_errors();
}

// ---- layout ------------------------------------------------------------------------------------

/// A sidebar the reader scrolled keeps its place through a collapse and a restore: the left one
/// scrolled to its lower sections, the right one partway down a node's detail.
#[test]
fn a_scrolled_sidebar_keeps_its_scroll_position_through_collapse_and_restore() {
    let browser = need_browser!();
    let (_front, mut driven) = opened(&browser, "#view=2d", None);
    let node = driven.eval("__viewer.graph.nodes().sort((a, b) => __viewer.graph.degree(b) - __viewer.graph.degree(a))[0]");
    driven.eval(&format!(
        "history.pushState(null, '', '#view=2d&node={}'); dispatchEvent(new PopStateEvent('popstate'))",
        node.as_str().unwrap()
    ));
    assert!(
        driven.wait_for("!!document.querySelector('#panel h1')", 30),
        "a node is open"
    );
    // a short window, so both sidebars scroll
    driven.metrics(1600, 260);
    assert!(
        driven.wait_for(
            "(l => l.scrollHeight > l.clientHeight + 120)(document.querySelector('aside.left')) \
             && (r => r.scrollHeight > r.clientHeight + 40)(document.getElementById('panel'))",
            20
        ),
        "both sidebars overflow: {}",
        driven.eval("[document.querySelector('aside.left').scrollHeight, document.querySelector('aside.left').clientHeight, document.getElementById('panel').scrollHeight, document.getElementById('panel').clientHeight]")
    );
    let before = driven.eval(
        "(l => { l.scrollTop = 120; const r = document.getElementById('panel'); r.scrollTop = 40; return [l.scrollTop, r.scrollTop]; })(document.querySelector('aside.left'))",
    );
    assert_eq!(before, json!([120, 40]), "both sidebars scrolled");
    // Keep the normal assertion path free of layout-forcing diagnostic calls.
    let diagnostics = std::env::var_os("EKR_COMPACT_DIAGNOSTICS").is_some();
    let diagnostic_before = diagnostics.then(|| driven.sidebar_diagnostic());
    driven.key("c", "KeyC", 0);
    measured_when(&mut driven, "m.left === 0 && m.right === 0", "collapsed");
    let diagnostic_collapsed = diagnostics.then(|| driven.sidebar_diagnostic());
    driven.eval("document.getElementById('leftStrip').click(); document.getElementById('rightStrip').click()");
    measured_when(&mut driven, "m.left > 0 && m.right > 0", "restored");
    let after = driven.eval(
        "[document.querySelector('aside.left').scrollTop, document.getElementById('panel').scrollTop]",
    );
    if after != before || diagnostics {
        eprintln!(
            "compact sidebar diagnostic: {}",
            json!({
                "before": before, "after": after,
                "before_layout": diagnostic_before,
                "collapsed_layout": diagnostic_collapsed,
                "restored_layout": driven.sidebar_diagnostic(),
            })
        );
    }
    assert_eq!(
        after, before,
        "the sidebars' scroll positions [left, right] before the collapse and after the restore"
    );
    driven.no_errors();
}

/// In a narrow window compact mode gives the graph all but the two strips, and a restore gives
/// the sidebars back without an error.
#[test]
fn in_a_narrow_window_compact_gives_the_graph_all_but_the_strips() {
    let browser = need_browser!();
    let (_front, mut driven) = opened(&browser, "#view=2d", None);
    for width in [800_u32, 400] {
        driven.metrics(width, 700);
        driven.key("c", "KeyC", 0);
        let m = measured_when(
            &mut driven,
            &format!(
                "m.left === 0 && m.right === 0 && Math.abs(m.canvas - m.stage) < 1 && Math.abs(m.canvas - ({width} - m.leftStrip - m.rightStrip)) < 1"
            ),
            &format!("compact at {width} px"),
        );
        eprintln!("{width} px compact: {m}");
        driven.key("c", "KeyC", 0);
        let m = measured_when(
            &mut driven,
            &if width >= 800 {
                "m.left > 0 && m.right > 0 && Math.abs(m.canvas - m.stage) < 1".to_owned()
            } else {
                // 290 + 360 px of sidebars overflow a 400 px window with or without this unit
                "m.left > 0 && m.right > 0".to_owned()
            },
            &format!("restored at {width} px"),
        );
        eprintln!("{width} px restored: {m}");
    }
    driven.no_errors();
}

// ---- shift+click solo --------------------------------------------------------------------------

/// Shift+click on a node type chip shows that type alone and writes it into the address; a second
/// shift+click on it shows every type; a plain click on another type after a solo shows that type
/// as well; a shift+click on a hidden type shows it alone; back restores each address; an edge type
/// chip solos its edge type the same way.
#[test]
fn shift_click_solo_writes_the_address_toggles_back_and_restores_through_history() {
    let browser = need_browser!();
    let (_front, mut driven) = opened(&browser, "#view=2d", None);
    let types = chip_types(&mut driven, "nodeTypes");
    assert!(types.len() >= 3, "{types:?}");
    let (a, b) = (types[0].clone(), types[1].clone());
    let others = |keep: &[&String]| -> Vec<String> {
        types
            .iter()
            .filter(|t| !keep.contains(t))
            .cloned()
            .collect()
    };

    driven.click(&format!("#nodeTypes .chip[data-type='{a}']"), SHIFT);
    assert!(
        driven.wait_for(
            &format!("decodeURIComponent(location.hash).includes('types={a}')"),
            20
        ),
        "the solo is in the address: {}",
        driven.eval("location.hash")
    );
    let mut off = off_chips(&mut driven, "nodeTypes");
    off.sort();
    let mut want = others(&[&a]);
    want.sort();
    assert_eq!(off, want, "only {a} is shown");
    assert_eq!(
        driven.eval("window.getSelection().toString()"),
        "",
        "a shift+click selects no text"
    );

    driven.click(&format!("#nodeTypes .chip[data-type='{b}']"), 0);
    assert!(driven.wait_for(
        "document.querySelectorAll('#nodeTypes .chip.off').length === 1",
        20
    ));
    driven.eval("history.back()");
    assert!(
        driven.wait_for(
            &format!(
                "document.querySelectorAll('#nodeTypes .chip.off').length === {}",
                types.len() - 1
            ),
            20
        ),
        "back restores the solo"
    );
    driven.click(&format!("#nodeTypes .chip[data-type='{a}']"), SHIFT);
    assert!(
        driven.wait_for("!decodeURIComponent(location.hash).includes('types=') && document.querySelectorAll('#nodeTypes .chip.off').length === 0", 20),
        "a second shift+click shows every type: {}",
        driven.eval("location.hash")
    );
    // a hidden type, shift+clicked, is shown alone
    driven.click(&format!("#nodeTypes .chip[data-type='{b}']"), 0);
    assert!(driven.wait_for(&format!("document.querySelector(\"#nodeTypes .chip[data-type='{b}']\").classList.contains('off')"), 20));
    driven.click(&format!("#nodeTypes .chip[data-type='{b}']"), SHIFT);
    let mut off = off_chips(&mut driven, "nodeTypes");
    off.sort();
    let mut want = others(&[&b]);
    want.sort();
    assert_eq!(off, want, "only {b} is shown");
    let drawn_kinds = "JSON.stringify([...new Set(__viewer.graph.nodes().filter(n => !__viewer.renderer.getNodeDisplayData(n)?.hidden).map(n => __viewer.graph.getNodeAttribute(n, 'kind')))])";
    assert!(
        driven.wait_for(&format!("{drawn_kinds} === '[\"{b}\"]'"), 20),
        "only {b} is drawn: {}",
        driven.eval(drawn_kinds)
    );

    // edge types share the gesture
    let edges = chip_types(&mut driven, "edgeTypes");
    if edges.len() >= 2 {
        driven.click(
            &format!("#edgeTypes .chip[data-type='{}']", edges[0]),
            SHIFT,
        );
        let off = off_chips(&mut driven, "edgeTypes");
        assert_eq!(off.len(), edges.len() - 1, "{off:?}");
        driven.click(
            &format!("#edgeTypes .chip[data-type='{}']", edges[0]),
            SHIFT,
        );
        assert!(off_chips(&mut driven, "edgeTypes").is_empty());
    }
    driven.no_errors();
}
