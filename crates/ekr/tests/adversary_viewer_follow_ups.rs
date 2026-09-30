//! Adversary pass 1 on `task:viewer-compact-follow-ups` (`4acd71bf`), in
//! `crates/ekr/src/cli/viewer/index.html`: the marked right strip, keyboard type chips and the
//! narrow-window rule.
//!
//! Every case seeds the `view-page/sounding` fixture through the real binary and serves it with the
//! real `ekr view`. A front server answers `/` with the embedded page and forwards every other
//! request to `ekr view` unchanged, so a case can serve a mutated copy of the page, held in memory
//! only (`EKR_ADVERSARY_MUTANT`), to show that it fails on the defect it names. The browser is
//! driven over the DevTools protocol by a local copy of the `Driven` harness of
//! `adversary_viewer_compact.rs`, which adds a window size, extra browser flags and accessibility
//! reads.

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
        // the threshold moved: a 969 px window opens with both sidebars shown
        Ok("narrow-700") => mutate("const NARROW = 970;", "const NARROW = 700;"),
        // the comparison includes the threshold: a 970 px window opens compact
        Ok("narrow-le") => mutate("window.innerWidth < NARROW", "window.innerWidth <= NARROW"),
        // only the node type chips take the keyboard
        Ok("no-edge-keys") => mutate(
            "[[\"nodeTypes\", hiddenNodeTypes], [\"edgeTypes\", hiddenEdgeTypes]]",
            "[[\"nodeTypes\", hiddenNodeTypes]]",
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
    fn start() -> Self {
        let seeded = Seeded::new();
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

/// The `Driven` harness of `view_page.rs`, copied (it is private to that file).
struct Driven {
    child: Child,
    socket: TcpStream,
    reader: BufReader<TcpStream>,
    next: u64,
    errors: Vec<Value>,
    _profile: tempfile::TempDir,
    _one: std::sync::MutexGuard<'static, ()>,
}

impl Driven {
    /// Starts the browser in a window of `size` (width, height) with `flags` besides the usual ones,
    /// and opens `url`.
    fn launch(browser: &Path, url: &str, size: (u32, u32), flags: &[&str]) -> Self {
        let one = one_browser();
        let profile = tempfile::tempdir().unwrap();
        let mut args = vec![
            "--headless".to_owned(),
            "--use-angle=swiftshader".to_owned(),
            "--enable-unsafe-swiftshader".to_owned(),
            "--no-sandbox".to_owned(),
            "--no-first-run".to_owned(),
            "--disable-extensions".to_owned(),
            format!("--window-size={},{}", size.0, size.1),
            "--remote-debugging-port=0".to_owned(),
            format!("--user-data-dir={}", profile.path().display()),
        ];
        args.extend(flags.iter().map(|flag| (*flag).to_owned()));
        args.push("about:blank".to_owned());
        let mut child = Command::new(browser)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(child.stderr.take().unwrap());
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
        let mut list = TcpStream::connect(("127.0.0.1", port)).unwrap();
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
            .find(|target| target["type"] == "page")
            .expect("a page target");
        let address = target["webSocketDebuggerUrl"].as_str().unwrap();
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
            child,
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

    /// One press of `key` (with its `code`) and `modifiers` (2 Ctrl, 8 Shift), sent to whatever
    /// holds the focus.
    fn key(&mut self, key: &str, code: &str, modifiers: u8) {
        let (text, keycode) = match key {
            "Enter" => ("\r", 13),
            " " => (" ", 32),
            "[" => ("[", 219),
            "k" => ("k", 75),
            other => panic!("key: {other:?} is not one this harness sends"),
        };
        let text = if modifiers & 0b0110 == 0 { text } else { "" };
        self.call(
            "Input.dispatchKeyEvent",
            json!({"type": "keyDown", "key": key, "code": code, "text": text, "unmodifiedText": text,
                "windowsVirtualKeyCode": keycode, "modifiers": modifiers}),
        );
        self.call(
            "Input.dispatchKeyEvent",
            json!({"type": "keyUp", "key": key, "code": code, "windowsVirtualKeyCode": keycode,
                "modifiers": modifiers}),
        );
    }

    /// The accessibility tree's role, computed name and pressed state of the element `selector`
    /// names, as a screen reader is told them.
    fn ax(&mut self, selector: &str) -> Value {
        self.call("Accessibility.enable", json!({}));
        let document = self.call("DOM.getDocument", json!({"depth": 0}));
        let root = document["result"]["root"]["nodeId"].clone();
        let found = self.call(
            "DOM.querySelector",
            json!({"nodeId": root, "selector": selector}),
        );
        let node = found["result"]["nodeId"].clone();
        let tree = self.call(
            "Accessibility.getPartialAXTree",
            json!({"nodeId": node, "fetchRelatives": false}),
        );
        let ax = &tree["result"]["nodes"][0];
        let pressed = ax["properties"]
            .as_array()
            .and_then(|all| all.iter().find(|p| p["name"] == "pressed"))
            .map_or(Value::Null, |p| p["value"]["value"].clone());
        json!({"role": ax["role"]["value"], "name": ax["name"]["value"], "pressed": pressed,
            "ignored": ax["ignored"]})
    }

    fn metrics(&mut self, width: u32, height: u32) {
        self.call(
            "Emulation.setDeviceMetricsOverride",
            json!({"width": width, "height": height, "deviceScaleFactor": 1, "mobile": false}),
        );
    }

    fn no_errors(&self) {
        assert!(
            self.errors.is_empty(),
            "the page reported errors: {:?}",
            self.errors
        );
    }
}

impl Drop for Driven {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

const SHIFT: u8 = 8;
const CTRL: u8 = 2;
const WIDE: (u32, u32) = (1600, 1000);

const SETTLED: &str = "!!window.__viewer && !window.__viewer.layoutRunning \
     && !document.querySelector('[data-act=stream-stop]') && !!window.__viewer.renderer";

/// What the reader sees: the widths of the canvas, the stage, each sidebar and strip, the address.
const MEASURE: &str = "(() => { const shown = e => e && getComputedStyle(e).display !== 'none' ? e.getBoundingClientRect().width : 0;
  const c = __viewer.fg ? __viewer.fg.renderer().domElement : document.querySelector('#graph canvas');
  return {canvas: c.getBoundingClientRect().width, stage: document.getElementById('stage').getBoundingClientRect().width,
    left: shown(document.querySelector('aside.left')), right: shown(document.getElementById('panel')),
    leftStrip: shown(document.getElementById('leftStrip')), rightStrip: shown(document.getElementById('rightStrip')),
    hash: decodeURIComponent(location.hash), inner: innerWidth, ratio: devicePixelRatio}; })()";

/// The right strip's mark: whether its dot is shown, its title and its accessible name.
const MARK: &str = "(s => ({dot: getComputedStyle(s.querySelector('.mark')).display !== 'none', marked: s.classList.contains('marked'),
  title: s.title, label: s.getAttribute('aria-label')}))(document.getElementById('rightStrip'))";

/// The busiest node drawn: its id and name.
const BUSIEST: &str =
    "(id => ({id, name: __viewer.graph.getNodeAttribute(id, 'name')}))(__viewer.graph.nodes()
  .sort((a, b) => __viewer.graph.degree(b) - __viewer.graph.degree(a))[0])";

fn measured_when(driven: &mut Driven, condition: &str, what: &str) -> Value {
    let expression = format!("(m => {condition})({MEASURE})");
    assert!(
        driven.wait_for(&expression, 30),
        "{what}: {}",
        driven.eval(MEASURE)
    );
    driven.eval(MEASURE)
}

fn settled(driven: &mut Driven) {
    assert!(driven.wait_for(SETTLED, 90), "the page settled");
}

/// Replaces the address with `hash` as a history entry and applies it, as back and forward do.
fn go(driven: &mut Driven, hash: &str) {
    driven.eval(&format!(
        "history.pushState(null, '', {}); dispatchEvent(new PopStateEvent('popstate'))",
        serde_json::to_string(hash).unwrap()
    ));
}

/// Restores the right sidebar with its strip and collapses it again with its tab: the reader has
/// seen what the panel holds, so the strip carries no mark.
fn seen_and_collapsed(driven: &mut Driven) {
    driven.eval("document.getElementById('rightStrip').click()");
    measured_when(driven, "m.right > 0", "the strip restored the details");
    driven.eval("document.getElementById('foldRight').click()");
    measured_when(
        driven,
        "m.right === 0 && m.rightStrip > 0",
        "the tab collapsed the details",
    );
    std::thread::sleep(Duration::from_millis(300));
    let mark = driven.eval(MARK);
    assert_eq!(mark["dot"], false, "no mark once seen: {mark}");
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

// ---- the keyboard ------------------------------------------------------------------------------

/// A keyboard reader on a type chip keeps the focus when the page draws the chips again for a
/// reason other than the chip's own key: here a node's neighbourhood stream that lands while the
/// chip holds the focus (the page streams on load, on every opened node and on every hop). Every
/// stream batch ends in `merge` → `render(false)`, which rebuilds both chip lists with `put`, so the
/// focused chip leaves the document and the focus falls to `body`; only the chip's own Enter and
/// Space put it back.
#[test]
fn a_focused_type_chip_keeps_the_focus_when_the_page_draws_the_chips_again() {
    let browser = need_browser!();
    let front = Front::start();
    let mut driven = Driven::launch(&browser, &format!("{}#view=2d", front.url), WIDE, &[]);
    settled(&mut driven);
    let types = chip_types(&mut driven, "nodeTypes");
    let first = types[0].clone();
    let focus = format!(
        "document.querySelector(\"#nodeTypes .chip[data-type='{first}']\").focus(); document.activeElement.dataset.type ?? null"
    );
    let held = format!("document.activeElement?.dataset?.type === '{first}'");

    // a neighbourhood stream lands while the chip holds the focus
    assert_eq!(
        driven.eval(&focus),
        first.as_str(),
        "the chip takes the focus"
    );
    driven.eval("window.__chipsBefore = document.querySelector('#nodeTypes .chip')");
    let busiest = driven.eval(BUSIEST);
    go(
        &mut driven,
        &format!("#view=2d&node={}", busiest["id"].as_str().unwrap()),
    );
    assert!(
        driven.wait_for(
            &format!("{SETTLED} && !!document.querySelector('#panel h1') && window.__chipsBefore !== document.querySelector('#nodeTypes .chip')"),
            30
        ),
        "the node's stream landed and the chips were drawn again"
    );
    assert!(
        driven.wait_for(&held, 2),
        "after the stream's batch the focus is on {}",
        driven.eval("document.activeElement.tagName + '#' + document.activeElement.id")
    );
    driven.no_errors();
}

/// An edge type chip takes the keyboard as a node type chip does: Enter hides its type, Space
/// shows it, Shift+Enter shows it alone and Shift+Space every edge type again, the focus staying on
/// the chip; Space scrolls nothing. The unit's own case presses only node type chips: the mutant
/// `no-edge-keys` (the keydown handler bound to `#nodeTypes` alone) passes `view_page.rs` and fails
/// here. The accessibility tree names every chip, tab, strip and arrow in words.
#[test]
fn an_edge_type_chip_takes_the_keyboard_and_every_toggle_has_a_spoken_name() {
    let browser = need_browser!();
    let front = Front::start();
    let mut driven = Driven::launch(&browser, &format!("{}#view=2d", front.url), WIDE, &[]);
    settled(&mut driven);
    let edges = chip_types(&mut driven, "edgeTypes");
    assert!(edges.len() >= 2, "{edges:?}");
    let e = edges[0].clone();
    let chip = format!("#edgeTypes .chip[data-type='{e}']");
    let state = |off: bool| {
        format!(
            "(c => c && c.classList.contains('off') === {off} && c.getAttribute('aria-pressed') === '{}')\
             (document.querySelector(\"{chip}\")) && document.activeElement?.dataset?.type === '{e}'",
            !off
        )
    };
    let offs = "document.querySelectorAll('#edgeTypes .chip.off').length";
    driven.eval(&format!("document.querySelector(\"{chip}\").focus()"));
    driven.key("Enter", "Enter", 0);
    assert!(driven.wait_for(&state(true), 10), "Enter hid the edge type");
    driven.key(" ", "Space", 0);
    assert!(
        driven.wait_for(&format!("{} && {offs} === 0", state(false)), 10),
        "Space showed it again"
    );
    driven.key("Enter", "Enter", SHIFT);
    assert!(
        driven.wait_for(
            &format!("{} && {offs} === {}", state(false), edges.len() - 1),
            10
        ),
        "Shift+Enter showed the edge type alone"
    );
    driven.key(" ", "Space", SHIFT);
    assert!(
        driven.wait_for(&format!("{} && {offs} === 0", state(false)), 10),
        "Shift+Space showed every edge type again"
    );

    // Space on a chip in a scrolled sidebar does not scroll it
    driven.metrics(1600, 300);
    let node_chip = format!(
        "#nodeTypes .chip[data-type='{}']",
        chip_types(&mut driven, "nodeTypes")[0]
    );
    let scrolled = driven.eval(&format!(
        "(l => {{ document.querySelector(\"{node_chip}\").focus(); return [l.scrollHeight > l.clientHeight + 40, l.scrollTop]; }})(document.querySelector('aside.left'))"
    ));
    assert_eq!(scrolled[0], true, "the left sidebar scrolls: {scrolled}");
    driven.key(" ", "Space", 0);
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(
        driven.eval("document.querySelector('aside.left').scrollTop"),
        scrolled[1],
        "Space scrolled the sidebar"
    );
    driven.key(" ", "Space", 0);
    driven.metrics(1600, 1000);

    // what a screen reader is told
    let node_ax = driven.ax(&node_chip);
    let label = driven.eval(&format!(
        "document.querySelector(\"{node_chip}\").children[1].textContent"
    ));
    assert!(
        node_ax["role"] == "button"
            && node_ax["name"]
                .as_str()
                .unwrap_or_default()
                .contains(label.as_str().unwrap())
            && node_ax["pressed"] == "true",
        "a node type chip is a pressed button named by its type {label}: {node_ax}"
    );
    for (selector, name) in [
        ("#foldLeft", "Collapse the controls"),
        ("#foldRight", "Collapse the details"),
        ("#revPrev", "The revision before"),
        ("#revNext", "The revision after"),
    ] {
        let ax = driven.ax(selector);
        assert!(
            ax["role"] == "button" && ax["name"] == name,
            "{selector}: {ax}"
        );
    }
    assert_eq!(driven.ax("#compact")["pressed"], "false");
    driven.eval("document.activeElement.blur(); document.getElementById('compact').click()");
    measured_when(&mut driven, "m.left === 0 && m.right === 0", "compact");
    for (selector, name) in [
        ("#leftStrip", "Show the controls"),
        ("#rightStrip", "Show the details"),
    ] {
        let ax = driven.ax(selector);
        assert!(
            ax["role"] == "button" && ax["name"] == name,
            "{selector}: {ax}"
        );
    }
    assert_eq!(driven.ax("#compact")["pressed"], "true");
    driven.no_errors();
}

// ---- the strip's mark --------------------------------------------------------------------------

/// Every way a new detail reaches the collapsed panel marks the strip with what the panel's heading
/// names: a node from the palette and a path from the address; the accessibility tree reads the
/// name. A detail shown while only the left sidebar is collapsed marks nothing. (The fixture holds
/// one revision, so a revision change is not driven here.)
#[test]
fn every_way_a_detail_reaches_the_collapsed_panel_marks_the_strip() {
    let browser = need_browser!();
    let front = Front::start();
    let mut driven = Driven::launch(
        &browser,
        &format!("{}#view=2d&compact=right", front.url),
        WIDE,
        &[],
    );
    settled(&mut driven);
    let busiest = driven.eval(BUSIEST);
    let (id, name) = (
        busiest["id"].as_str().unwrap().to_owned(),
        busiest["name"].as_str().unwrap().to_owned(),
    );
    let marked_with = |heading: &str| {
        let quoted = serde_json::to_string(heading).unwrap();
        format!(
            "(k => k.dot && k.title.endsWith(': ' + {quoted}) && k.label === 'Show the details: ' + {quoted})({MARK}) \
             && document.querySelector('#panel h1')?.textContent === {quoted}"
        )
    };

    // the palette
    driven.key("k", "KeyK", CTRL);
    assert!(
        driven.wait_for("document.activeElement?.id === 'palQ'", 10),
        "the palette opened"
    );
    driven.eval(&format!(
        "(q => {{ q.value = {}; q.dispatchEvent(new Event('input', {{bubbles: true}})); }})(document.getElementById('palQ'))",
        serde_json::to_string(&name).unwrap()
    ));
    let clicked = driven.eval(&format!(
        "(p => p ? (p.click(), true) : false)([...document.querySelectorAll('#palList .pi')].find(p => p.textContent.includes({})))",
        serde_json::to_string(&name).unwrap()
    ));
    assert_eq!(clicked, true, "the palette lists the node");
    assert!(
        driven.wait_for(&marked_with(&name), 20),
        "the palette's node marks the strip: {}",
        driven.eval(MARK)
    );
    let spoken = driven.ax("#rightStrip");
    assert_eq!(
        spoken["name"],
        format!("Show the details: {name}"),
        "{spoken}"
    );
    seen_and_collapsed(&mut driven);

    // a path from the address
    let ends = driven.eval(&format!(
        "(g => g.neighbors('{id}').map(n => [n, g.getNodeAttribute(n, 'name')])[0])(__viewer.graph)"
    ));
    let other = ends[0].as_str().unwrap().to_owned();
    go(
        &mut driven,
        &format!("#view=2d&compact=right&path={id}~{other}"),
    );
    assert!(
        driven.wait_for(
            &format!(
                "(h => !!h && h.startsWith('Path') && (k => k.dot && k.title.endsWith(': ' + h))({MARK}))(document.querySelector('#panel h1')?.textContent)"
            ),
            30
        ),
        "the path marks the strip: {} / {}",
        driven.eval("document.querySelector('#panel h1')?.textContent ?? null"),
        driven.eval(MARK)
    );

    // only the left sidebar collapsed: a detail is shown, and nothing is marked
    go(&mut driven, &format!("#view=2d&compact=left&node={id}"));
    measured_when(
        &mut driven,
        "m.left === 0 && m.right > 0",
        "compact=left shows the details",
    );
    assert!(
        driven.wait_for(
            &format!(
                "document.querySelector('#panel h1')?.textContent === {}",
                serde_json::to_string(&name).unwrap()
            ),
            20
        ),
        "the node is in the panel"
    );
    std::thread::sleep(Duration::from_millis(300));
    let mark = driven.eval(MARK);
    assert!(
        mark["marked"] == false && mark["label"] == "Show the details",
        "{mark}"
    );
    driven.no_errors();
}

/// A node the reader has already seen, drawn into the collapsed panel again, is not new: the
/// unit's own case asserts "no mark over a node the reader has seen" for the fold tab. Back to an
/// address that names the node the panel already shows re-renders it (`applyState` calls
/// `showNode` for the focused node), and the strip is marked as though a new detail arrived.
#[test]
fn back_to_the_node_already_shown_does_not_mark_the_strip() {
    let browser = need_browser!();
    let front = Front::start();
    let mut driven = Driven::launch(
        &browser,
        &format!("{}#view=2d&compact=right", front.url),
        WIDE,
        &[],
    );
    settled(&mut driven);
    let busiest = driven.eval(BUSIEST);
    let (id, name) = (
        busiest["id"].as_str().unwrap().to_owned(),
        busiest["name"].as_str().unwrap().to_owned(),
    );
    go(&mut driven, &format!("#view=2d&compact=right&node={id}"));
    assert!(
        driven.wait_for(
            &format!(
                "document.querySelector('#panel h1')?.textContent === {} && ({MARK}).dot",
                serde_json::to_string(&name).unwrap()
            ),
            30
        ),
        "the node marks the strip"
    );
    seen_and_collapsed(&mut driven);
    let t = chip_types(&mut driven, "nodeTypes").pop().unwrap();
    driven.eval(&format!(
        "document.querySelector(\"#nodeTypes .chip[data-type='{t}']\").click()"
    ));
    measured_when(
        &mut driven,
        &format!("m.hash.includes('types=') && m.hash.includes('node={id}') && m.hash.includes('compact=right')"),
        "a type hidden",
    );
    std::thread::sleep(Duration::from_millis(300));
    let control = driven.eval(MARK);
    assert_eq!(
        control["dot"], false,
        "a type toggle marks nothing: {control}"
    );
    driven.eval("history.back()");
    measured_when(
        &mut driven,
        &format!("!m.hash.includes('types=') && m.hash.includes('node={id}') && m.hash.includes('compact=right')"),
        "back: every type, the same node",
    );
    assert!(driven.wait_for(SETTLED, 30), "the page settled after back");
    std::thread::sleep(Duration::from_millis(800));
    let mark = driven.eval(MARK);
    assert_eq!(
        mark["dot"], false,
        "back to the node the panel already shows marked the strip: {mark}"
    );
    driven.no_errors();
}

// ---- a narrow window ---------------------------------------------------------------------------

/// The threshold is 970 px exactly: 969 opens compact, 970 and 971 open with both sidebars shown.
/// The unit's own case opens only a 600 px window: the mutants `narrow-700` and `narrow-le` pass
/// `view_page.rs` and fail here. `compact=0` beside every other parameter holds in a 969 px window
/// and keeps every parameter; `compact=0` in a wide window shows both and is not written back; a
/// window of 1200 CSS px at a device pixel ratio of 2 is not narrow.
#[test]
fn the_narrow_rule_holds_below_970_px_only_and_compact_0_keeps_every_parameter() {
    let browser = need_browser!();
    let front = Front::start();
    let (id, shown) = {
        let mut driven = Driven::launch(&browser, &format!("{}#view=2d", front.url), WIDE, &[]);
        settled(&mut driven);
        let id = driven.eval(BUSIEST)["id"].as_str().unwrap().to_owned();
        let types = chip_types(&mut driven, "nodeTypes");
        assert!(types.len() >= 2, "{types:?}");
        (id, types[..types.len() - 1].join(","))
    };
    let full = format!("#view=2d&compact=0&node={id}&types={shown}&focus={id}&hops=2&q=a");
    for (width, flags, hash, both_shown, kept) in [
        (
            969,
            &[][..],
            "#view=2d",
            false,
            vec!["compact=1".to_owned()],
        ),
        (970, &[][..], "#view=2d", true, vec![]),
        (971, &[][..], "#view=2d", true, vec![]),
        (
            969,
            &[][..],
            full.as_str(),
            true,
            vec![
                "compact=0".to_owned(),
                format!("node={id}"),
                format!("types={shown}"),
                format!("focus={id}"),
                "hops=2".to_owned(),
                "q=a".to_owned(),
            ],
        ),
        (1600, &[][..], "#view=2d&compact=0", true, vec![]),
        (
            1200,
            &["--force-device-scale-factor=2"][..],
            "#view=2d",
            true,
            vec![],
        ),
    ] {
        let mut driven = Driven::launch(
            &browser,
            &format!("{}{hash}", front.url),
            (width, 900),
            flags,
        );
        settled(&mut driven);
        let condition = if both_shown {
            "m.left > 0 && m.right > 0"
        } else {
            "m.left === 0 && m.right === 0"
        };
        let m = measured_when(
            &mut driven,
            &format!("m.inner === {width} && {condition}"),
            &format!("{width} px {flags:?} at {hash}"),
        );
        let written = m["hash"].as_str().unwrap();
        for part in &kept {
            assert!(
                written.contains(part.as_str()),
                "{width} px {hash}: {part} is kept: {m}"
            );
        }
        if kept.is_empty() {
            assert!(!written.contains("compact"), "{width} px {hash}: {m}");
        }
        driven.no_errors();
    }
}

/// History entries written in a wide window carry no `compact`, because both sidebars shown was
/// the default there. When the window has narrowed since (a split screen, a rotated tablet), back
/// to such an entry reads the missing `compact` as the narrow default and collapses both sidebars
/// the reader had open: history changes the layout although the reader changed nothing about it.
#[test]
fn back_in_a_narrowed_window_keeps_the_sidebars_the_reader_has_open() {
    let browser = need_browser!();
    let front = Front::start();
    let mut driven = Driven::launch(&browser, &format!("{}#view=2d", front.url), WIDE, &[]);
    settled(&mut driven);
    let t = chip_types(&mut driven, "nodeTypes").pop().unwrap();
    let chip = format!("document.querySelector(\"#nodeTypes .chip[data-type='{t}']\").click()");
    driven.eval(&chip);
    measured_when(&mut driven, "m.hash.includes('types=')", "a type hidden");
    driven.eval(&chip);
    let m = measured_when(
        &mut driven,
        "!m.hash.includes('types=')",
        "the type shown again",
    );
    assert!(!m["hash"].as_str().unwrap().contains("compact"), "{m}");
    driven.metrics(900, 900);
    let m = measured_when(
        &mut driven,
        "m.inner === 900 && m.left > 0 && m.right > 0",
        "narrowed to 900 px, both sidebars still shown",
    );
    eprintln!("narrowed: {m}");
    driven.eval("history.back()");
    assert!(
        driven.wait_for(
            "decodeURIComponent(location.hash).includes('types=') && !!document.querySelector('#nodeTypes .chip.off')",
            20
        ),
        "back restored the hidden type"
    );
    std::thread::sleep(Duration::from_millis(500));
    let m = driven.eval(MEASURE);
    assert!(
        m["left"].as_f64().unwrap() > 0.0 && m["right"].as_f64().unwrap() > 0.0,
        "back collapsed the sidebars the reader had open: {m}"
    );
    driven.no_errors();
}
