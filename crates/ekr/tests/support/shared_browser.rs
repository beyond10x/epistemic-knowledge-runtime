//! One Chromium per test run, shared by every browser case (`story:browser-tests-one-chromium`).
//!
//! The test step (`scripts/test-step.sh`) starts one headless Chromium before the tests, with the
//! flags every case's own launch carries, and names its DevTools address in [`VARIABLE`]. A case
//! then opens its page in a browser context of its own in that browser
//! (`Target.createBrowserContext`): no cookies, storage, cache or history are shared with another
//! case, so nextest runs the cases at once. The context is disposed when the case drops its
//! [`Browser`], and by the browser itself when the case's process ends first (`disposeOnDetach`).
//!
//! With [`VARIABLE`] unset (`cargo test -p ekr` or `cargo nextest run` without the step) a case
//! starts a browser of its own, as it did before.

// Six test files include this file and each uses a part of it.
#![allow(dead_code)]

use std::ffi::OsStr;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

use serde_json::{json, Value};

/// The environment variable that names the step's browser, as `127.0.0.1:<port>`.
pub(crate) const VARIABLE: &str = "EKR_TEST_BROWSER";

/// How long a case waits for the browser to answer one protocol call, or to send the next message
/// its harness reads: a call that gets no answer in this time fails the case, naming the call.
pub(crate) const REPLY_BOUND: Duration = Duration::from_secs(20);

/// How long one protocol call to the shared browser may take.
const CALL_TIMEOUT: Duration = REPLY_BOUND;

/// How long a browser case may wait in all, from its browser's launch or its first wait: under
/// the test runner's 60 s kill (`.config/nextest.toml`), so that a case whose page never gets
/// where it waits for fails naming that, rather than being killed with nothing said
/// (`story:ekr-browser-case-hang`).
pub(crate) const CASE_BOUND: Duration = Duration::from_secs(45);

thread_local! {
    /// When the case on this thread launched its browser or first waited; each case runs on a
    /// thread of its own.
    static CASE_START: std::cell::Cell<Option<std::time::Instant>> =
        const { std::cell::Cell::new(None) };
}

/// Fails the case whose harness read nothing from the browser's DevTools socket within
/// [`REPLY_BOUND`] while it waited for `awaiting` (a call's method, or the rest of a message).
pub(crate) fn unanswered(awaiting: &str, error: &std::io::Error) -> ! {
    panic!(
        "{awaiting}: the browser's DevTools socket sent nothing within {REPLY_BOUND:?} ({error})"
    )
}

/// The instant by which the case on this thread must be done waiting: [`CASE_BOUND`] after its
/// browser's launch or its first wait.
pub(crate) fn case_deadline() -> std::time::Instant {
    CASE_START.with(|start| {
        let first = start.get().unwrap_or_else(std::time::Instant::now);
        start.set(Some(first));
        first + CASE_BOUND
    })
}

/// A case's browser: a process of its own, or a context in the step's browser.
pub(crate) enum Browser {
    Own(Child),
    Shared(Box<Context>),
}

/// A case's browser context and page in the step's browser, over a browser-level connection.
pub(crate) struct Context {
    port: u16,
    socket: tungstenite::WebSocket<TcpStream>,
    next: u64,
    context: String,
    target: String,
}

/// The case's browser: a page of `size` (width, height; the browser's default when `None`) in a
/// fresh context of the step's browser when [`VARIABLE`] names one, else the process `own` starts.
/// `own` is the case's launch as it stands; it runs only when there is no shared browser.
pub(crate) fn launch(size: Option<(u32, u32)>, own: impl FnOnce() -> Child) -> Browser {
    case_deadline();
    match shared_address() {
        Some(address) => Browser::Shared(Box::new(Context::open(&address, size))),
        None => Browser::Own(own()),
    }
}

/// The variable that requires a browser of every browser case, as the forge's `CI` does.
pub(crate) const REQUIRE: &str = "EKR_REQUIRE_BROWSER";

/// The headless Chromium a case found, passed through; `None`, with which the case skips, only
/// where no browser is required. On the forge (`CI` set) or with [`REQUIRE`] set, a case that found
/// none fails here rather than passing without starting one (finding 39 of the EKR 0.0.42 review).
pub(crate) fn required(found: Option<PathBuf>) -> Option<PathBuf> {
    required_under(
        found,
        std::env::var_os("CI").as_deref(),
        std::env::var_os(REQUIRE).as_deref(),
    )
}

/// [`required`] with the values of `CI` and [`REQUIRE`] given; a value that is empty, `0` or
/// `false` does not require.
pub(crate) fn required_under(
    found: Option<PathBuf>,
    ci: Option<&OsStr>,
    require: Option<&OsStr>,
) -> Option<PathBuf> {
    let set = |value: Option<&OsStr>| {
        value.is_some_and(|value| !value.is_empty() && value != "0" && value != "false")
    };
    assert!(
        found.is_some() || !(set(ci) || set(require)),
        "no headless Chromium, and one is required here (CI or {REQUIRE} is set): set \
         EKR_VIEW_BROWSER to one"
    );
    found
}

/// The shared browser's address, when the step started one.
pub(crate) fn shared_address() -> Option<String> {
    std::env::var(VARIABLE)
        .ok()
        .filter(|address| !address.trim().is_empty())
}

impl Browser {
    /// The browser's DevTools port, and the address of this case's page socket when the browser
    /// is shared. An own browser is read for the port it announces on its standard error (which
    /// its launch pipes); its page is found by the caller, as before.
    pub(crate) fn devtools(&mut self) -> (u16, Option<String>) {
        match self {
            Browser::Own(child) => {
                let mut lines = BufReader::new(
                    child
                        .stderr
                        .take()
                        .expect("the browser's standard error is piped"),
                );
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
                (port, None)
            }
            Browser::Shared(context) => (
                context.port,
                Some(format!(
                    "ws://127.0.0.1:{}/devtools/page/{}",
                    context.port, context.target
                )),
            ),
        }
    }

    /// The DevTools port and this case's page socket address, when the browser is shared; `None`
    /// for a browser of its own, which the caller reads as before.
    pub(crate) fn shared_page(&self) -> Option<(u16, String)> {
        match self {
            Browser::Own(_) => None,
            Browser::Shared(context) => Some((
                context.port,
                format!(
                    "ws://127.0.0.1:{}/devtools/page/{}",
                    context.port, context.target
                ),
            )),
        }
    }

    /// Ends the browser: kills an own one, disposes a shared one's context. Dropping does it too.
    pub(crate) fn stop(&mut self) {
        match self {
            Browser::Own(child) => {
                child.kill().ok();
                child.wait().ok();
            }
            Browser::Shared(context) => context.close(),
        }
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        self.stop();
        drove();
    }
}

/// The environment variable that names the file in which the test step counts the browser cases
/// that drove a browser: each such case's process appends its id and its thread's name.
pub(crate) const CASES: &str = "EKR_TEST_BROWSER_CASES";

/// Notes in [`CASES`] that this case drove a browser. A case that skipped for want of one never
/// holds a [`Browser`], so the step tells a run whose browser cases all skipped from one that ran.
fn drove() {
    use std::io::Write;
    let Some(path) = std::env::var_os(CASES).filter(|path| !path.is_empty()) else {
        return;
    };
    let line = format!(
        "{} {}\n",
        std::process::id(),
        std::thread::current().name().unwrap_or("-")
    );
    // one short write to a file opened for appending: lines of cases that run at once stay whole
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        file.write_all(line.as_bytes()).ok();
    }
}

impl Context {
    fn open(address: &str, size: Option<(u32, u32)>) -> Self {
        let port: u16 = address
            .rsplit(':')
            .next()
            .and_then(|port| port.trim().parse().ok())
            .unwrap_or_else(|| panic!("{VARIABLE}={address}: not <host>:<port>"));
        let version = http_get(port, "/json/version");
        let url = version["webSocketDebuggerUrl"]
            .as_str()
            .unwrap_or_else(|| panic!("{VARIABLE}={address}: no browser socket in {version}"))
            .to_owned();
        let tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
        tcp.set_read_timeout(Some(CALL_TIMEOUT)).unwrap();
        let (socket, _) = tungstenite::client(url, tcp).unwrap();
        let mut opened = Self {
            port,
            socket,
            next: 0,
            context: String::new(),
            target: String::new(),
        };
        opened.context = opened.call(
            None,
            "Target.createBrowserContext",
            json!({"disposeOnDetach": true}),
        )["browserContextId"]
            .as_str()
            .expect("a browser context id")
            .to_owned();
        let mut params = json!({
            "url": "about:blank",
            "browserContextId": opened.context,
            "newWindow": true,
        });
        if let Some((width, height)) = size {
            params["width"] = json!(width);
            params["height"] = json!(height);
        }
        opened.target = opened.call(None, "Target.createTarget", params)["targetId"]
            .as_str()
            .expect("a target id")
            .to_owned();
        // A page of a browser with several windows is not always the focused one; a case's own
        // browser had one window, which had the focus. This session stays attached while the case
        // holds the context, so the page keeps behaving as focused.
        let session = opened.call(
            None,
            "Target.attachToTarget",
            json!({"targetId": opened.target, "flatten": true}),
        )["sessionId"]
            .as_str()
            .expect("a session id")
            .to_owned();
        opened.call(
            Some(&session),
            "Emulation.setFocusEmulationEnabled",
            json!({"enabled": true}),
        );
        opened
    }

    /// One protocol call; events and other answers read meanwhile are passed over.
    fn call(&mut self, session: Option<&str>, method: &str, params: Value) -> Value {
        self.try_call(session, method, params)
            .unwrap_or_else(|error| panic!("{method} to the shared browser: {error}"))
    }

    /// [`Context::call`], its failure returned rather than raised.
    fn try_call(
        &mut self,
        session: Option<&str>,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        self.next += 1;
        let id = self.next;
        let mut message = json!({"id": id, "method": method, "params": params});
        if let Some(session) = session {
            message["sessionId"] = json!(session);
        }
        self.socket
            .send(tungstenite::Message::Text(message.to_string().into()))
            .map_err(|error| error.to_string())?;
        loop {
            let tungstenite::Message::Text(text) =
                self.socket.read().map_err(|error| error.to_string())?
            else {
                continue;
            };
            let answer: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
            if answer["id"] == id {
                if let Some(error) = answer.get("error") {
                    return Err(error.to_string());
                }
                return Ok(answer["result"].clone());
            }
        }
    }

    /// Disposes the case's context; it never panics, since it runs on drop, and a case that
    /// failed may drop it while unwinding. A browser already gone has nothing to dispose.
    fn close(&mut self) {
        if self.context.is_empty() {
            return;
        }
        let context = std::mem::take(&mut self.context);
        self.try_call(
            None,
            "Target.disposeBrowserContext",
            json!({ "browserContextId": context }),
        )
        .ok();
        self.socket.close(None).ok();
    }
}

/// Waits up to `seconds` for `expression` to be `true` in a case's page, and says whether it was.
///
/// The page checks `expression` itself every 50 ms and answers as soon as it holds, in one
/// `Runtime.evaluate` that `evaluate` sends (with `params`) and whose reply it returns. A case
/// that asked every 200 ms saw a page that had settled up to 200 ms late, and asked it five times
/// a second over the protocol. A page that navigates while it checks loses the context the check
/// ran in; the check starts again in the new one.
///
/// The page is asked for at most [`WAIT_SLICE`] at a time, so a page whose own clock stops
/// leaves the reply waited for no longer than a protocol call's bound. No wait runs past the
/// case's [`case_deadline`]; one cut short by it says so on the case's error output.
pub(crate) fn wait_in_page(
    mut evaluate: impl FnMut(Value) -> Value,
    expression: &str,
    seconds: u64,
) -> bool {
    let asked = std::time::Instant::now() + Duration::from_secs(seconds);
    let until = asked.min(case_deadline());
    let held = wait_until(&mut evaluate, expression, until);
    if !held && until < asked {
        eprintln!(
            "waiting for {expression}: the case's bound of {CASE_BOUND:?} ended the wait before \
             its {seconds} s"
        );
    }
    held
}

/// The longest one in-page check of [`wait_in_page`] runs before it answers.
const WAIT_SLICE: Duration = Duration::from_secs(5);

fn wait_until(
    evaluate: &mut impl FnMut(Value) -> Value,
    expression: &str,
    until: std::time::Instant,
) -> bool {
    loop {
        let left = until
            .saturating_duration_since(std::time::Instant::now())
            .min(WAIT_SLICE)
            .as_millis();
        let reply = evaluate(json!({
            "expression": format!(
                "new Promise(done => {{ const until = performance.now() + {left}; \
                 const check = () => {{ let held = false; \
                 try {{ held = ({expression}) === true; }} catch {{}} \
                 if (held || performance.now() >= until) done(held); \
                 else setTimeout(check, 50); }}; check(); }})"
            ),
            "awaitPromise": true,
            "returnByValue": true,
        }));
        let answered = match reply["result"]["result"]["value"] {
            Value::Bool(true) => return true,
            Value::Bool(false) => true,
            _ => false,
        };
        assert_ne!(
            reply["result"]["exceptionDetails"]["exception"]["className"], "SyntaxError",
            "{expression}: {reply}"
        );
        if std::time::Instant::now() >= until {
            return false;
        }
        if !answered {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// The JSON body `GET path` answers on the DevTools port.
fn http_get(port: u16, path: &str) -> Value {
    use std::io::{Read, Write};
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .unwrap_or_else(|error| panic!("{VARIABLE}: the shared browser on port {port}: {error}"));
    stream.set_read_timeout(Some(CALL_TIMEOUT)).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    // the DevTools server can keep the connection open: the body is read by its length
    let mut reader = BufReader::new(stream);
    let mut length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
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
    reader.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap_or(Value::Null)
}

/// A script a harness gives every page it runs under virtual time, before the page loads
/// (`Page.addScriptToEvaluateOnNewDocument`): a classic Web Worker's `terminate()` takes effect
/// once the worker has started, not before.
///
/// Headless Chromium holds a page's virtual clock while a worker the page created starts, and a
/// worker terminated before it has started now and then never lets go: the clock stands for good
/// with nothing in flight (`story:ekr-browser-case-hang`). The page's 2D layout spawns its
/// ForceAtlas2 worker each time a layout starts, and a streamed expansion stops and starts the
/// layout batch by batch, so a worker is often terminated at the virtual instant it was made.
/// Measured with 4 to 8 cases at a time on 4 CPUs: 1 to 5 % of the runs of
/// `adversary_page_stream_1::store_text_...` stood still, each at the instant a worker was made
/// and terminated, and the whole page made no frame; with `terminate()` a no-op, or with this
/// guard, none of 880 runs did. A worker's first statement here posts a mark the guard takes in;
/// until it has, a `terminate()` is held.
pub(crate) const WORKER_GUARD: &str = r#"(() => {
  const Native = window.Worker;
  if (!Native) return;
  const revoke = URL.revokeObjectURL.bind(URL), loading = new Map(), MARK = "ekr-worker-started";
  // a script address a starting worker still has to load is revoked once every such worker has
  URL.revokeObjectURL = url => { if (loading.has(url)) loading.get(url).revoked = true; else revoke(url); };
  const Guarded = function Worker(url, options) {
    if (options && options.type === "module") return new Native(url, options);
    const target = new URL(url, location.href).href;
    const boot = URL.createObjectURL(new Blob(
      ["postMessage(" + JSON.stringify(MARK) + "); importScripts(" + JSON.stringify(target) + ");"],
      {type: "text/javascript"}));
    const entry = loading.get(target) || {starting: 0, revoked: false};
    entry.starting += 1;
    loading.set(target, entry);
    const worker = new Native(boot, options), end = Native.prototype.terminate.bind(worker);
    let started = false, ending = false;
    const start = () => {
      if (started) return;
      started = true;
      revoke(boot);
      entry.starting -= 1;
      if (!entry.starting) { loading.delete(target); if (entry.revoked) revoke(target); }
      if (ending) end();
    };
    worker.addEventListener("message", event => {
      if (!started && event.data === MARK) { event.stopImmediatePropagation(); start(); }
    });
    worker.addEventListener("error", start);
    worker.terminate = () => { if (started) end(); else ending = true; };
    return worker;
  };
  Guarded.prototype = Native.prototype;
  window.Worker = Guarded;
})();"#;
