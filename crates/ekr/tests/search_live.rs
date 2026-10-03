//! Actual browser interactions through CDP. No evaluated browser source is used.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tungstenite::{Message, WebSocket};

struct World {
    dir: tempfile::TempDir,
    child: Child,
    origin: String,
}
impl World {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let fixture = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("tests/fixtures/retraction");
        std::fs::copy(fixture.join("host.json"), dir.path().join("host.json")).unwrap();
        let mut seed = Self::command(&dir);
        let out = seed
            .arg("seed")
            .arg(fixture.join("seed.yaml"))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let mut child = Self::command(&dir)
            .args(["view", "--port", "0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let announcement: Value = serde_json::from_str(&line).unwrap();
        let origin = announcement["url"]
            .as_str()
            .unwrap()
            .trim_end_matches('/')
            .to_owned();
        Self { dir, child, origin }
    }
    fn command(dir: &tempfile::TempDir) -> Command {
        // Also exercise an independently installed binary through the identical browser seam.
        let binary = std::env::var_os("EKR_SEARCH_TEST_BINARY")
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_ekr").into());
        let mut cmd = Command::new(binary);
        for key in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
            cmd.env_remove(key);
        }
        cmd.arg("--host")
            .arg(dir.path().join("host.json"))
            .arg("--store")
            .arg(dir.path().join("store"))
            .args(["--backend", "sqlite"]);
        cmd
    }
    fn get(&self, path: &str) -> (String, String) {
        let mut stream = TcpStream::connect(self.origin.trim_start_matches("http://")).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: {}\r\n\r\n",
            self.origin.trim_start_matches("http://")
        )
        .unwrap();
        let mut bytes = String::new();
        stream.read_to_string(&mut bytes).unwrap();
        let (head, body) = bytes.split_once("\r\n\r\n").unwrap();
        (head.to_owned(), body.to_owned())
    }
}
impl Drop for World {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Browser {
    _child: Running,
    _dir: tempfile::TempDir,
    _serial: std::sync::MutexGuard<'static, ()>,
    socket: WebSocket<TcpStream>,
    next: u64,
    events: Vec<Value>,
}
impl Browser {
    fn new() -> Option<Self> {
        static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let binary = std::env::var_os("EKR_VIEW_BROWSER")
            .map(PathBuf::from)
            .or_else(|| {
                let cache = PathBuf::from(std::env::var_os("HOME")?).join(".cache/ms-playwright");
                let mut paths: Vec<_> = std::fs::read_dir(cache)
                    .ok()?
                    .filter_map(Result::ok)
                    .map(|e| {
                        e.path()
                            .join("chrome-headless-shell-linux64/chrome-headless-shell")
                    })
                    .filter(|p| p.is_file())
                    .collect();
                paths.sort();
                paths.pop()
            });
        let Some(binary) = binary else {
            assert!(
                std::env::var_os("EKR_REQUIRE_BROWSER").is_none(),
                "required Chromium browser absent; set EKR_VIEW_BROWSER"
            );
            eprintln!("SKIP: Chromium absent; set EKR_VIEW_BROWSER and EKR_REQUIRE_BROWSER=1 for acceptance");
            return None;
        };
        let dir = tempfile::tempdir().unwrap();
        let child = Running(
            Command::new("taskset")
                .args(["-c", "0-1", "nice", "-n", "19"])
                .arg(binary)
                .args([
                    "--headless",
                    "--no-sandbox",
                    "--no-first-run",
                    "--disable-gpu",
                    "--remote-debugging-port=0",
                ])
                .arg(format!("--user-data-dir={}", dir.path().display()))
                .arg("about:blank")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let start = Instant::now();
        let port = loop {
            if let Ok(text) = std::fs::read_to_string(dir.path().join("DevToolsActivePort")) {
                break text.lines().next().unwrap().to_owned();
            }
            assert!(
                start.elapsed() < Duration::from_secs(60),
                "Chromium did not announce CDP"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        write!(
            stream,
            "GET /json/list HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        let mut reader = BufReader::new(stream);
        let mut length = None;
        loop {
            let mut line = String::new();
            assert!(
                reader.read_line(&mut line).unwrap() > 0,
                "CDP discovery closed before headers"
            );
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = Some(value.trim().parse::<usize>().unwrap());
            }
        }
        let mut body = vec![0; length.unwrap()];
        reader.read_exact(&mut body).unwrap();
        let list: Value = serde_json::from_slice(&body).unwrap();
        let url = list[0]["webSocketDebuggerUrl"].as_str().unwrap();
        let tcp = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        tcp.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
        let (socket, _) = tungstenite::client(url, tcp).unwrap();
        let mut browser = Self {
            _child: child,
            _dir: dir,
            _serial: serial,
            socket,
            next: 0,
            events: vec![],
        };
        browser.call("Page.enable", json!({}));
        Some(browser)
    }
    fn call(&mut self, method: &str, params: Value) -> Value {
        let value = self.raw_call(method, params);
        assert!(value.get("error").is_none(), "{method}: {value}");
        value["result"].clone()
    }
    fn raw_call(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = self.next;
        self.socket
            .send(Message::Text(
                json!({"id":id,"method":method,"params":params})
                    .to_string()
                    .into(),
            ))
            .unwrap();
        loop {
            let message = self.socket.read().unwrap();
            if let Message::Text(text) = message {
                let value: Value = serde_json::from_str(&text).unwrap();
                if value["id"] == id {
                    return value;
                }
                self.events.push(value);
            }
        }
    }
    fn node(&mut self, selector: &str) -> u64 {
        let root = self.call("DOM.getDocument", json!({}))["root"]["nodeId"]
            .as_u64()
            .unwrap();
        self.call(
            "DOM.querySelector",
            json!({"nodeId":root,"selector":selector}),
        )["nodeId"]
            .as_u64()
            .unwrap()
    }
    fn html(&mut self) -> String {
        let node = self.node("html");
        self.call("DOM.getOuterHTML", json!({"nodeId":node}))["outerHTML"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    fn until(&mut self, expected: &str) -> String {
        let start = Instant::now();
        loop {
            let html = self.html();
            if html.contains(expected) {
                return html;
            }
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "DOM never contained {expected}: {html}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn navigate(&mut self, url: &str) {
        self.events.retain(|e| e["method"] != "Page.loadEventFired");
        self.call("Page.navigate", json!({"url":url}));
        let start = Instant::now();
        loop {
            self.call("Browser.getVersion", json!({}));
            if self
                .events
                .iter()
                .any(|e| e["method"] == "Page.loadEventFired")
            {
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "page load event absent"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        self.until("id=\"query\"");
    }
    fn type_text(&mut self, value: &str) {
        self.call("Input.insertText", json!({"text":value}));
    }
    fn focus(&mut self) {
        let node = self.node("#query");
        self.call("DOM.focus", json!({"nodeId":node}));
    }
    fn key(&mut self, key: &str, code: u32, modifiers: u32) {
        self.call("Input.dispatchKeyEvent", json!({"type":"keyDown","key":key,"windowsVirtualKeyCode":code,"modifiers":modifiers,"text":if key=="Enter" { "\r" } else { "" }}));
        self.call(
            "Input.dispatchKeyEvent",
            json!({"type":"keyUp","key":key,"windowsVirtualKeyCode":code,"modifiers":modifiers}),
        );
    }
    fn query_value(&mut self) -> String {
        let node = self.node("#query");
        let ax = self.call("Accessibility.getPartialAXTree", json!({"nodeId":node}));
        let input = &ax["nodes"][0];
        assert!(
            input["properties"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["name"] == "focused" && p["value"]["value"] == true),
            "input lost focus: {input}"
        );
        input["value"]["value"].as_str().unwrap_or("").to_owned()
    }
}

#[test]
fn live_typing_preserves_focus_and_caret_without_navigation() {
    let Some(mut browser) = Browser::new() else {
        return;
    };
    let world = World::new();
    browser.navigate(&format!("{}/find?revision=0", world.origin));
    browser.focus();
    browser.type_text("Aice");
    browser.until("No matches");
    browser.key("ArrowLeft", 37, 0);
    browser.key("ArrowLeft", 37, 0);
    browser.key("ArrowLeft", 37, 0);
    browser.type_text("l");
    let html = browser.until("class=\"result\"");
    assert_eq!(browser.query_value(), "Alice");
    assert!(html.contains("/#revision=0&amp;node="));
    assert!(html.contains("Historical view"));
    let frame = browser.call("Page.getFrameTree", json!({}));
    assert!(frame["frameTree"]["frame"]["url"]
        .as_str()
        .unwrap()
        .ends_with("/find?revision=0"));
    browser.type_text("x");
    assert_eq!(
        browser.query_value(),
        "Alxice",
        "caret must stay where typing left it"
    );
}

impl Browser {
    fn paused(&mut self, query: &str) -> Value {
        let start = Instant::now();
        loop {
            self.call("Browser.getVersion", json!({}));
            if let Some(at) = self.events.iter().position(|e| {
                e["method"] == "Fetch.requestPaused"
                    && e["params"]["request"]["url"]
                        .as_str()
                        .is_some_and(|url| url.contains(query))
            }) {
                return self.events.remove(at)["params"].clone();
            }
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "no intercepted request for {query}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn fulfill(&mut self, request: &Value, status: u16, html: &str) {
        use base64::Engine as _;
        self.call("Fetch.fulfillRequest",json!({"requestId":request["requestId"],"responseCode":status,"responseHeaders":[{"name":"Content-Type","value":"text/html; charset=utf-8"}],"body":base64::engine::general_purpose::STANDARD.encode(html)}));
    }
    fn replace_query(&mut self, value: &str) {
        self.focus();
        self.key("a", 65, 2);
        self.type_text(value);
    }
}

#[test]
fn superseded_query_clear_failure_and_enter_are_honest() {
    let Some(mut browser) = Browser::new() else {
        return;
    };
    let world = World::new();
    browser.navigate(&format!("{}/find", world.origin));
    browser.until("data-live-search=\"ready\"");
    browser.call("Network.enable", json!({}));
    browser.call(
        "Fetch.enable",
        json!({"patterns":[{"urlPattern":"*/find?q=*","requestStage":"Request"}]}),
    );
    browser.focus();
    browser.type_text("Alice");
    let old = browser.paused("q=Alice");
    browser.replace_query("Nobody");
    let new = browser.paused("q=Nobody");
    let (_, nobody) = world.get("/find?q=Nobody");
    browser.fulfill(&new, 200, &nobody);
    browser.until("No matches");
    // Deliberately release the older server answer after the current one.
    let (_, alice) = world.get("/find?q=Alice");
    use base64::Engine as _;
    let late=browser.raw_call("Fetch.fulfillRequest",json!({"requestId":old["requestId"],"responseCode":200,"body":base64::engine::general_purpose::STANDARD.encode(&alice)}));
    if late.get("error").is_some() {
        assert_eq!(
            late["error"]["message"], "Invalid InterceptionId.",
            "late response failed for an unexpected reason: {late}"
        );
    }
    browser.call("Browser.getVersion", json!({}));
    assert!(!browser.html().contains("class=\"result\""));
    assert_eq!(browser.query_value(), "Nobody");
    assert!(
        browser
            .events
            .iter()
            .any(|e| e["method"] == "Network.loadingFailed" && e["params"]["canceled"] == true),
        "superseded fetch was not canceled"
    );
    browser.replace_query("Alice");
    let request = browser.paused("q=Alice");
    browser.fulfill(&request, 200, &alice);
    browser.until("class=\"result\"");
    browser.key("a", 65, 2);
    browser.key("Backspace", 8, 0);
    let cleared = browser.until("Start with a name.");
    assert!(!cleared.contains("class=\"result\""));
    assert_eq!(browser.query_value(), "");
    browser.type_text("Failure");
    let request = browser.paused("q=Failure");
    browser.fulfill(&request, 503, "unavailable");
    let failed = browser.until("Search is unavailable");
    assert!(!failed.contains("class=\"result\""));
    assert_eq!(browser.query_value(), "Failure");
    // Enter starts one immediate read, preserving the ordinary form semantics.
    browser.replace_query("Alice");
    browser.key("Enter", 13, 0);
    let request = browser.paused("q=Alice");
    browser.fulfill(&request, 200, &alice);
    browser.until("class=\"result\"");
}

#[test]
fn composition_suppresses_partial_queries_and_no_script_get_still_works() {
    let Some(mut browser) = Browser::new() else {
        return;
    };
    let world = World::new();
    browser.navigate(&format!("{}/find", world.origin));
    browser.until("data-live-search=\"ready\"");
    browser.focus();
    browser.call(
        "Fetch.enable",
        json!({"patterns":[{"urlPattern":"*/find?q=*","requestStage":"Request"}]}),
    );
    browser.call(
        "Input.imeSetComposition",
        json!({"text":"Ali","selectionStart":3,"selectionEnd":3}),
    );
    browser.until("data-search-state=\"composing\"");
    // Advance virtual timer time, not a performance deadline, past any debounce.
    browser.call(
        "Emulation.setVirtualTimePolicy",
        json!({"policy":"advance","budget":1000}),
    );
    let start = Instant::now();
    loop {
        browser.call("Browser.getVersion", json!({}));
        if browser
            .events
            .iter()
            .any(|e| e["method"] == "Emulation.virtualTimeBudgetExpired")
        {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "virtual-time barrier absent"
        );
    }
    assert!(
        !browser
            .events
            .iter()
            .any(|e| e["method"] == "Fetch.requestPaused"),
        "partial composition issued a read"
    );
    browser.type_text("Alice");
    browser.call(
        "Emulation.setVirtualTimePolicy",
        json!({"policy":"advance","budget":1000}),
    );
    let request = browser.paused("q=Alice");
    let (_, alice) = world.get("/find?q=Alice");
    browser.fulfill(&request, 200, &alice);
    browser.until("class=\"result\"");
    browser.call("Fetch.disable", json!({}));
    browser.call(
        "Emulation.setVirtualTimePolicy",
        json!({"policy":"advance"}),
    );
    browser.call(
        "Emulation.setScriptExecutionDisabled",
        json!({"value":true}),
    );
    browser.navigate(&format!("{}/find?revision=0", world.origin));
    browser.focus();
    browser.type_text("Alice");
    assert_eq!(browser.query_value(), "Alice");
    browser.key("Enter", 13, 0);
    let html = browser.until("class=\"result\"");
    assert!(html.contains("Historical view"));
    assert!(html.contains("/#revision=0&amp;node="));
    let frame = browser.call("Page.getFrameTree", json!({}));
    let url = frame["frameTree"]["frame"]["url"].as_str().unwrap();
    assert!(
        url.contains("q=Alice") && url.contains("revision=0"),
        "GET fallback URL: {url}"
    );
}

#[test]
fn embedded_assets_work_without_store_and_keep_http_admission() {
    let world = World::new();
    // The viewer has not admitted the store; remove it before requesting static assets.
    std::fs::remove_file(world.dir.path().join("store")).unwrap();
    for (path, mime) in [
        ("/assets/search.js", "text/javascript; charset=utf-8"),
        ("/assets/search_bg.wasm", "application/wasm"),
    ] {
        for (method, host, extra, expected) in [
            ("GET", world.origin.trim_start_matches("http://"), "", 200),
            ("POST", world.origin.trim_start_matches("http://"), "", 405),
            ("GET", "wrong.example.invalid", "", 421),
            (
                "GET",
                world.origin.trim_start_matches("http://"),
                "Content-Length: 1\r\n",
                413,
            ),
        ] {
            let mut stream =
                TcpStream::connect(world.origin.trim_start_matches("http://")).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(60)))
                .unwrap();
            write!(
                stream,
                "{method} {path} HTTP/1.1\r\nHost: {host}\r\n{extra}\r\n"
            )
            .unwrap();
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).unwrap();
            let end = bytes.windows(4).position(|b| b == b"\r\n\r\n").unwrap();
            let headers = std::str::from_utf8(&bytes[..end]).unwrap();
            assert_eq!(
                headers
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .parse::<u16>()
                    .unwrap(),
                expected
            );
            if expected == 200 {
                assert!(headers.contains(mime));
                assert!(headers.contains("nosniff"));
                assert!(headers.contains("no-store"));
                let body = &bytes[end + 4..];
                if path.ends_with(".wasm") {
                    assert!(body.starts_with(b"\0asm"));
                } else {
                    assert!(std::str::from_utf8(body)
                        .unwrap()
                        .contains("WebAssembly.instantiateStreaming"));
                }
            }
        }
    }
    assert!(!world.dir.path().join("store").exists());
}

#[test]
fn adversary_clearing_an_inflight_read_keeps_the_empty_state_after_late_failure() {
    let Some(mut browser) = Browser::new() else {
        return;
    };
    let world = World::new();
    browser.navigate(&format!("{}/find", world.origin));
    browser.until("data-live-search=\"ready\"");
    browser.call("Network.enable", json!({}));
    browser.call(
        "Fetch.enable",
        json!({"patterns":[{"urlPattern":"*/find?q=*","requestStage":"Request"}]}),
    );
    browser.focus();
    browser.type_text("Alice");
    let pending = browser.paused("q=Alice");
    browser.key("a", 65, 2);
    browser.key("Backspace", 8, 0);
    browser.until("data-search-state=\"empty\"");
    let late = browser.raw_call(
        "Fetch.fulfillRequest",
        json!({"requestId":pending["requestId"],"responseCode":503}),
    );
    if late.get("error").is_some() {
        assert_eq!(late["error"]["message"], "Invalid InterceptionId.");
    }
    browser.call("Browser.getVersion", json!({}));
    let html = browser.html();
    assert!(html.contains("data-search-state=\"empty\""));
    assert!(html.contains("Start with a name."));
    assert!(!html.contains("Search is unavailable"));
    assert!(!html.contains("class=\"result\""));
    assert_eq!(browser.query_value(), "");
    assert!(browser.events.iter().any(|event| {
        event["method"] == "Network.loadingFailed" && event["params"]["canceled"] == true
    }));
    // A later successful query must still render: an empty/error state cannot disable the adapter.
    browser.type_text("Alice");
    let next = browser.paused("q=Alice");
    let (_, alice) = world.get("/find?q=Alice");
    browser.fulfill(&next, 200, &alice);
    browser.until("class=\"result\"");
    assert_eq!(browser.query_value(), "Alice");
}

#[test]
fn adversary_reserved_characters_round_trip_as_one_query_without_navigation() {
    let Some(mut browser) = Browser::new() else {
        return;
    };
    let world = World::new();
    let entry = format!("{}/find", world.origin);
    browser.navigate(&entry);
    browser.until("data-live-search=\"ready\"");
    browser.call(
        "Fetch.enable",
        json!({"patterns":[{"urlPattern":"*/find?q=*","requestStage":"Request"}]}),
    );
    browser.focus();
    let query = "Å<&+?#%'\"";
    browser.type_text(query);
    let encoded = "%C3%85%3C%26%2B%3F%23%25%27%22";
    let pending = browser.paused(encoded);
    assert_eq!(pending["request"]["url"], format!("{entry}?q={encoded}"));
    let (headers, body) = world.get(&format!("/find?q={encoded}"));
    assert!(headers.starts_with("HTTP/1.1 200"));
    browser.fulfill(&pending, 200, &body);
    let html = browser.until("No matches");
    assert_eq!(browser.query_value(), query);
    assert_eq!(html.matches("<script").count(), 1);
    assert!(!html.contains("class=\"result\""));
    let frame = browser.call("Page.getFrameTree", json!({}));
    assert_eq!(frame["frameTree"]["frame"]["url"], entry);
}

#[test]
fn browser_result_and_evidence_links_stay_at_the_selected_revision() {
    let Some(mut browser) = Browser::new() else {
        return;
    };
    let world = World::new();
    browser.navigate(&format!("{}/find?revision=0", world.origin));
    browser.until("data-live-search=\"ready\"");
    browser.focus();
    browser.type_text("Alice");
    let before = browser.until("class=\"result\"");
    assert!(!before.contains("class=\"evidence\""));
    let proposal = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("tests/fixtures/retraction/propose-alice.yaml");
    for args in [
        vec!["propose", proposal.to_str().unwrap()],
        vec![
            "validate",
            "00000000-0000-4000-8000-000000000601",
            "--against",
            "0",
        ],
        vec!["commit", "00000000-0000-4000-8000-000000000601"],
    ] {
        let result = World::command(&world.dir).args(args).output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    browser.replace_query("Ali");
    let historical = browser.until("class=\"result\"");
    assert!(historical.contains("/#revision=0&amp;node="));
    assert!(!historical.contains("class=\"evidence\""));
    browser.navigate(&format!("{}/find", world.origin));
    browser.until("data-live-search=\"ready\"");
    browser.focus();
    browser.type_text("Alice");
    let current = browser.until("class=\"evidence\"");
    assert!(current.contains("/#revision=1&amp;node="));
    assert!(current.contains("/evidence/00000000-0000-4000-8000-000000000401?revision=1"));
    browser.navigate(&format!("{}/find?revision=99", world.origin));
    browser.until("data-live-search=\"ready\"");
    browser.focus();
    browser.type_text("Alice");
    let missing = browser.until("data-search-state=\"ready\"");
    assert!(missing.contains("This revision is unavailable"));
    assert!(!missing.contains("class=\"result\""));
    assert_eq!(browser.query_value(), "Alice");
}
