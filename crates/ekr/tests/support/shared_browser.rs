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

use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::process::Child;
use std::time::Duration;

use serde_json::{json, Value};

/// The environment variable that names the step's browser, as `127.0.0.1:<port>`.
pub(crate) const VARIABLE: &str = "EKR_TEST_BROWSER";

/// How long one protocol call to the shared browser may take.
const CALL_TIMEOUT: Duration = Duration::from_secs(60);

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
    match shared_address() {
        Some(address) => Browser::Shared(Box::new(Context::open(&address, size))),
        None => Browser::Own(own()),
    }
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
pub(crate) fn wait_in_page(
    mut evaluate: impl FnMut(Value) -> Value,
    expression: &str,
    seconds: u64,
) -> bool {
    let until = std::time::Instant::now() + Duration::from_secs(seconds);
    loop {
        let left = until
            .saturating_duration_since(std::time::Instant::now())
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
        if let Value::Bool(held) = reply["result"]["result"]["value"] {
            return held;
        }
        assert_ne!(
            reply["result"]["exceptionDetails"]["exception"]["className"], "SyntaxError",
            "{expression}: {reply}"
        );
        if std::time::Instant::now() >= until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
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
