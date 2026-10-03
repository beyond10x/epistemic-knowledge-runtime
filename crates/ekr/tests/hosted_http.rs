//! Hosted listener acceptance; synthetic stores and real HTTP connections only.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for key in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(key);
    }
    command
}
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("tests/fixtures/retraction")
        .join(name)
}
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}
impl World {
    fn empty(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        std::fs::copy(
            fixture("host.json"),
            world.directory.path().join("host.json"),
        )
        .unwrap();
        world
    }
    fn seeded(backend: &'static str) -> Self {
        let world = Self::empty(backend);
        world.seed();
        world
    }
    fn path(&self) -> PathBuf {
        self.directory.path().join("store")
    }
    fn create_unseeded(&self) {
        let host = ekr::host::CliHostConfigurationV1::from_json(
            &std::fs::read(self.directory.path().join("host.json")).unwrap(),
        )
        .unwrap();
        let runtime = match self.backend {
            "file" => {
                ekr_kernel::Runtime::file(&self.path(), &host.tenant, host.context, host.authority)
            }
            "sqlite" => ekr_kernel::Runtime::sqlite(
                &self.path(),
                &host.tenant,
                host.context,
                host.authority,
            ),
            _ => unreachable!(),
        }
        .unwrap();
        assert!(runtime.head().unwrap().is_none());
    }
    fn seed(&self) {
        let mut command = self.command(&["seed"]);
        command.arg(fixture("seed.yaml"));
        success(command);
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut command = ekr();
        command
            .arg("--host")
            .arg(self.directory.path().join("host.json"))
            .arg("--store")
            .arg(self.path())
            .args(["--backend", self.backend])
            .args(args);
        command
    }
    fn commit(&self) {
        let mut command = self.command(&["propose"]);
        command.arg(fixture("propose-alice.yaml"));
        success(command);
        success(self.command(&[
            "validate",
            "00000000-0000-4000-8000-000000000601",
            "--against",
            "0",
        ]));
        success(self.command(&["commit", "00000000-0000-4000-8000-000000000601"]));
    }
    fn stdio(&self, messages: &[Value]) -> Vec<Value> {
        let mut process = self
            .command(&["mcp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let mut input = process.stdin.take().unwrap();
            for message in messages {
                writeln!(input, "{message}").unwrap();
            }
        }
        let out = process.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect()
    }
}
fn success(mut command: Command) -> Value {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Restore permissions before TempDir removes its synthetic fixture, including on panic.
struct ReadOnly(Vec<(PathBuf, std::fs::Permissions)>);
impl ReadOnly {
    fn tree(path: &std::path::Path) -> Self {
        fn protect(path: &std::path::Path, held: &mut ReadOnly) {
            let metadata = std::fs::metadata(path).unwrap();
            held.0.push((path.to_owned(), metadata.permissions()));
            if metadata.is_dir() {
                for entry in std::fs::read_dir(path).unwrap() {
                    protect(&entry.unwrap().path(), held);
                }
            }
            let mut permissions = metadata.permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(path, permissions).unwrap();
        }
        let mut held = Self(Vec::new());
        protect(path, &mut held);
        held
    }
}
impl Drop for ReadOnly {
    fn drop(&mut self) {
        for (path, permissions) in &self.0 {
            let _ = std::fs::set_permissions(path, permissions.clone());
        }
    }
}
fn files(path: &std::path::Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut result = std::collections::BTreeMap::new();
    if path.is_dir() {
        for entry in std::fs::read_dir(path).unwrap() {
            result.extend(files(&entry.unwrap().path()));
        }
    } else {
        result.insert(path.to_owned(), std::fs::read(path).unwrap());
    }
    result
}
struct Response {
    status: u16,
    body: Vec<u8>,
}
impl Response {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}
struct Server {
    child: Child,
    authority: String,
}
impl Server {
    fn start(mut command: Command) -> Self {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        if line.is_empty() {
            let mut error = String::new();
            child
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut error)
                .unwrap();
            let status = child.wait().unwrap();
            panic!("server exited {status}: {error}");
        }
        let value: Value = serde_json::from_str(&line).unwrap();
        let authority = value["url"]
            .as_str()
            .unwrap()
            .strip_prefix("http://")
            .unwrap()
            .trim_end_matches('/')
            .to_owned();
        Self { child, authority }
    }
    fn raw(&self, request: &[u8]) -> Response {
        let address = self.authority.replacen("0.0.0.0:", "127.0.0.1:", 1);
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        stream.write_all(request).unwrap();
        let mut bytes = Vec::new();
        if let Err(error) = stream.read_to_end(&mut bytes) {
            assert!(
                error.kind() == std::io::ErrorKind::ConnectionReset && !bytes.is_empty(),
                "{error}"
            );
        }
        let separator = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let headers = std::str::from_utf8(&bytes[..separator]).unwrap();
        let status = headers.split_whitespace().nth(1).unwrap().parse().unwrap();
        let length = headers
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let body = bytes[separator + 4..].to_vec();
        assert_eq!(body.len(), length, "{headers}");
        Response { status, body }
    }
    fn get(&self, path: &str) -> Response {
        self.raw(format!("GET {path} HTTP/1.1\r\nHost: {}\r\n\r\n", self.authority).as_bytes())
    }
    fn post(&self, message: &Value, headers: &str) -> Response {
        let body = message.to_string();
        self.raw(format!("POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: {}\r\n{headers}\r\n{body}",self.authority,body.len()).as_bytes())
    }
    fn mcp(&self, message: &Value) -> Response {
        self.post(message, "MCP-Protocol-Version: 2025-11-25\r\n")
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn initialize() -> Value {
    json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"synthetic-client","version":"1"}}})
}
fn call(name: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":name,"arguments":arguments}})
}

#[test]
fn hosted_viewer_exposes_liveness_and_admitted_seeded_readiness() {
    for backend in ["file", "sqlite"] {
        let world = World::seeded(backend);
        let server = Server::start(world.command(&["view", "--port", "0"]));
        assert!(server.authority.starts_with("127.0.0.1:"));
        for path in ["/healthz", "/readyz"] {
            assert_eq!(server.get(path).status, 200, "{backend} {path}");
        }
    }
}
#[test]
fn hosted_mcp_has_a_separate_explicit_listener_command() {
    let out = ekr().args(["mcp-http", "--help"]).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let help = String::from_utf8(out.stdout).unwrap();
    for flag in ["--bind", "--port", "--allow-host", "--allow-origin"] {
        assert!(help.contains(flag), "missing {flag}: {help}");
    }
}

#[test]
fn http_listener_is_refused_inside_a_session() {
    let world = World::seeded("file");
    let mut session = world
        .command(&["session"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(
        session.stdin.take().unwrap(),
        "{}",
        json!({"argv":["mcp-http"]})
    )
    .unwrap();
    let result = session.wait_with_output().unwrap();
    assert!(result.status.success());
    let response: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(response["exit"], 2);
    assert!(response["stderr"]
        .as_str()
        .unwrap()
        .contains("session-verb-refused"));
}

#[test]
fn both_http_listeners_read_nonwritable_stores_without_changing_files() {
    for backend in ["file", "sqlite"] {
        let world = World::seeded(backend);
        let before = files(world.directory.path());
        let readonly = ReadOnly::tree(world.directory.path());
        for verb in ["view", "mcp-http"] {
            let server = Server::start(world.command(&[verb, "--port", "0"]));
            assert_eq!(server.get("/readyz").status, 200);
            if verb == "view" {
                assert_eq!(server.get("/head").json()["head"], 0);
                assert_eq!(server.get("/").status, 200);
            } else {
                let response = server.mcp(&call("head", json!({}))).json();
                assert_eq!(response["result"]["isError"], false);
                assert_eq!(response["result"]["structuredContent"]["head"], 0);
            }
        }
        assert_eq!(
            files(world.directory.path()),
            before,
            "{backend}: reader changed fixture files"
        );
        drop(readonly);
    }
}
#[test]
fn health_stays_live_before_seed_and_readiness_recovers_after_seed() {
    for verb in ["view", "mcp-http"] {
        for backend in ["file", "sqlite"] {
            let world = World::empty(backend);
            let server = Server::start(world.command(&[verb, "--port", "0"]));
            assert_eq!(server.get("/healthz").status, 200);
            assert_eq!(server.get("/readyz").status, 503);
            assert!(!world.path().exists(), "reader must not create a store");
            world.create_unseeded();
            assert!(world.path().exists());
            assert_eq!(
                server.get("/readyz").status,
                503,
                "an existing unseeded provider is not ready"
            );
            world.seed();
            assert_eq!(server.get("/readyz").status, 200);
            assert_eq!(server.get("/healthz").status, 200);
            std::fs::rename(world.path(), world.directory.path().join("away")).unwrap();
            assert_eq!(server.get("/readyz").status, 503);
            assert_eq!(server.get("/healthz").status, 200);
            std::fs::rename(world.directory.path().join("away"), world.path()).unwrap();
            assert_eq!(server.get("/readyz").status, 200);
            drop(server);
            let restarted = Server::start(world.command(&[verb, "--port", "0"]));
            assert_eq!(restarted.get("/readyz").status, 200);
        }
    }
}

#[test]
fn readiness_recovers_when_sqlite_is_replaced_in_place() {
    for verb in ["view", "mcp-http"] {
        let world = World::seeded("sqlite");
        world.commit();
        let server = Server::start(world.command(&[verb, "--port", "0"]));
        assert_eq!(server.get("/readyz").status, 200);
        let replacement = World::empty("sqlite");
        let mut seed = replacement.command(&["seed"]);
        seed.arg(fixture("seed-different.yaml"));
        success(seed);
        std::fs::copy(replacement.path(), world.path()).unwrap();
        assert_eq!(server.get("/readyz").status, 200);
        assert_eq!(server.get("/healthz").status, 200);
        let head = if verb == "view" {
            server.get("/head").json()
        } else {
            let response = server.mcp(&call("head", json!({}))).json();
            serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap()
        };
        assert_eq!(head["head"], 0);
    }
}

/// Stage the native file provider immediately before the completion transaction commits.
/// Keep its valid log prefix and manifest, with the receipt's already-written blob unreferenced.
fn interrupt_file_completion(world: &World) {
    let path = world.path();
    let log = std::fs::read_to_string(path.join("events.jsonl")).unwrap();
    let mut frames: Vec<&str> = log.split_inclusive('\n').collect();
    let last: Value = serde_json::from_str(frames.pop().unwrap()).unwrap();
    assert_eq!(last["format"], "eventlog-file/1");
    let operations = last["transaction"].as_array().unwrap();
    let blob = operations
        .iter()
        .find(|operation| operation["operation"] == "blob")
        .unwrap();
    let receipt = std::fs::read(
        path.join("blobs")
            .join(blob["object"]["id"].as_str().unwrap()),
    )
    .unwrap();
    let receipt: Value = serde_json::from_slice(&receipt).unwrap();
    assert_eq!(
        receipt["format"], "ekr.migration-finished/2",
        "truncate only completion, never history"
    );
    assert!(operations
        .iter()
        .filter_map(|operation| operation.get("event"))
        .all(|event| event["stream_type"] == "ekr.store.object"));
    let previous: Value = serde_json::from_str(frames.last().unwrap()).unwrap();
    let prefix = frames.concat();
    let manifest_path = path.join("manifest.json");
    let mut manifest: Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    manifest["sequence"] = previous["sequence"].clone();
    manifest["digest"] = previous["digest"].clone();
    manifest["length"] = json!(prefix.len());
    std::fs::write(path.join("events.jsonl"), prefix).unwrap();
    std::fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
}

#[test]
fn readiness_refuses_an_incomplete_copy_even_with_a_seed_and_checkpoint() {
    for verb in ["view", "mcp-http"] {
        let source = World::seeded("file");
        source.commit();
        let world = World::empty("file");
        let mut migrate = source.command(&["migrate", "--to"]);
        migrate.arg(world.path());
        success(migrate);
        interrupt_file_completion(&world);
        let refused = world.command(&["head"]).output().unwrap();
        assert!(!refused.status.success());
        assert!(
            String::from_utf8_lossy(&refused.stderr).contains("migrate-incomplete"),
            "{}",
            String::from_utf8_lossy(&refused.stderr)
        );
        let before = files(world.directory.path());
        let server = Server::start(world.command(&[verb, "--port", "0"]));
        assert_eq!(server.get("/healthz").status, 200);
        assert_eq!(server.get("/readyz").status, 503);
        assert_eq!(server.get("/healthz").status, 200);
        assert_eq!(files(world.directory.path()), before);
    }
}
#[test]
fn http_and_stdio_mcp_return_identical_nine_tool_documents() {
    for backend in ["file", "sqlite"] {
        let world = World::seeded(backend);
        world.commit();
        let before = success(world.command(&["snapshot"]));
        let server = Server::start(world.command(&["mcp-http", "--port", "0"]));
        let messages = vec![
            initialize(),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            call("head", json!({})),
            call("overview", json!({})),
            call("search", json!({"text":"Alice"})),
            call(
                "describe_node",
                json!({"node":"00000000-0000-4000-8000-000000000301"}),
            ),
            call(
                "expand",
                json!({"seeds":["00000000-0000-4000-8000-000000000301"],"depth":1,"limit":20}),
            ),
            call("timeline", json!({"hops":1,"limit":20})),
            call("changes_since", json!({"since_revision":0})),
            call(
                "explain",
                json!({"assertion":"00000000-0000-4000-8000-000000000501"}),
            ),
            call("resolve", json!({"aliases":["Alice"]})),
            call("commit", json!({})),
        ];
        let expected = world.stdio(&messages);
        assert_eq!(expected.len(), messages.len());
        assert_eq!(expected[1]["result"]["tools"].as_array().unwrap().len(), 9);
        for (request, expected) in messages.iter().zip(expected) {
            let response = server.mcp(request);
            assert_eq!(response.status, 200);
            assert_eq!(response.json(), expected, "{backend} {request}");
        }
        assert_eq!(
            success(world.command(&["snapshot"])),
            before,
            "no canonical changes from HTTP tools"
        );
    }
}
#[test]
fn mcp_http_protocol_statuses_versions_and_origins() {
    let world = World::seeded("file");
    let server = Server::start(world.command(&[
        "mcp-http",
        "--port",
        "0",
        "--allow-origin",
        "https://reader.example.invalid",
    ]));
    assert_eq!(server.post(&initialize(), "").status, 200);
    let initialized = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
    let response = server.mcp(&initialized);
    assert_eq!(response.status, 202);
    assert!(response.body.is_empty());
    let response = server.mcp(&json!({"jsonrpc":"2.0","id":5,"result":{}}));
    assert_eq!(response.status, 202);
    assert!(response.body.is_empty());
    let list = json!({"jsonrpc":"2.0","id":4,"method":"tools/list"});
    assert_eq!(server.post(&list, "").status, 400);
    for version in ["bogus", "2025-03-26"] {
        assert_eq!(
            server
                .post(&list, &format!("MCP-Protocol-Version: {version}\r\n"))
                .status,
            400
        );
    }
    assert_eq!(
        server
            .post(&list, "MCP-Protocol-Version: 2025-06-18\r\n")
            .status,
        200
    );
    assert_eq!(
        server
            .post(
                &list,
                "MCP-Protocol-Version: 2025-11-25\r\nOrigin: https://reader.example.invalid\r\n"
            )
            .status,
        200
    );
    assert_eq!(
        server
            .post(
                &list,
                "MCP-Protocol-Version: 2025-11-25\r\nOrigin: https://attacker.example.invalid\r\n"
            )
            .status,
        403
    );
    assert_eq!(server.post(&list,"MCP-Protocol-Version: 2025-11-25\r\nOrigin: https://reader.example.invalid\r\nOrigin: https://reader.example.invalid\r\n").status,403);
    for method in ["GET", "DELETE"] {
        assert_eq!(
            server
                .raw(
                    format!(
                        "{method} /mcp HTTP/1.1\r\nHost: {}\r\n\r\n",
                        server.authority
                    )
                    .as_bytes()
                )
                .status,
            405
        );
    }
}

#[test]
fn unavailable_store_does_not_override_transport_refusals() {
    let world = World::empty("file");
    let server = Server::start(world.command(&["mcp-http", "--port", "0"]));
    for (method, path, expected) in [
        ("GET", "/mcp", 405),
        ("DELETE", "/mcp", 405),
        ("GET", "/unknown", 404),
        ("POST", "/readyz", 405),
    ] {
        let response = server.raw(
            format!(
                "{method} {path} HTTP/1.1\r\nHost: {}\r\n\r\n",
                server.authority
            )
            .as_bytes(),
        );
        assert_eq!(response.status, expected, "{method} {path}");
    }
    let list = json!({"jsonrpc":"2.0","id":4,"method":"tools/list"});
    assert_eq!(
        server.post(&list, "MCP-Protocol-Version: bogus\r\n").status,
        400
    );
    assert_eq!(server.get("/readyz").status, 503);
    assert_eq!(server.get("/healthz").status, 200);
    assert!(!world.path().exists());
}
#[test]
fn mcp_http_rejects_ambiguous_framing_and_unapproved_hosts() {
    let world = World::seeded("file");
    let server = Server::start(world.command(&["mcp-http", "--port", "0"]));
    for framing in [
        "Content-Length: 0\r\nContent-Length: 0\r\n",
        "Content-Length: 1, 1\r\n",
        "Transfer-Encoding: chunked\r\n",
        "Content-Length: +0\r\n",
    ] {
        assert_eq!(
            server
                .raw(
                    format!(
                        "POST /mcp HTTP/1.1\r\nHost: {}\r\n{framing}\r\n",
                        server.authority
                    )
                    .as_bytes()
                )
                .status,
            400,
            "{framing}"
        );
    }
    assert_eq!(
        server
            .raw(
                format!(
                    "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Length: 99999999\r\n\r\n",
                    server.authority
                )
                .as_bytes()
            )
            .status,
        413
    );
    for host in [
        "Host: attacker.example.invalid\r\n".to_owned(),
        format!(
            "Host: {}\r\nHost: {}\r\n",
            server.authority, server.authority
        ),
        format!(
            "Host: attacker.example.invalid\r\nX-Forwarded-Host: {}\r\n",
            server.authority
        ),
    ] {
        assert_eq!(
            server
                .raw(format!("GET /healthz HTTP/1.1\r\n{host}\r\n").as_bytes())
                .status,
            421
        );
    }
    let oversized = "a".repeat(17 * 1024);
    assert_eq!(
        server
            .raw(
                format!(
                    "GET /healthz HTTP/1.1\r\nHost: {}\r\nX-Large: {oversized}\r\n\r\n",
                    server.authority
                )
                .as_bytes()
            )
            .status,
        400
    );
    assert_eq!(server.get("/healthz").status, 200);
}
#[test]
fn external_binding_requires_and_enforces_explicit_authorities() {
    for verb in ["view", "mcp-http"] {
        let world = World::seeded("file");
        let refused = world
            .command(&[verb, "--bind", "0.0.0.0"])
            .output()
            .unwrap();
        assert!(!refused.status.success());
        let server = Server::start(world.command(&[
            verb,
            "--bind",
            "0.0.0.0",
            "--port",
            "0",
            "--allow-host",
            "reader.example.invalid",
            "--allow-host",
            "localhost:8800",
        ]));
        assert_eq!(
            server
                .raw(b"GET /healthz HTTP/1.1\r\nHost: reader.example.invalid\r\n\r\n")
                .status,
            200
        );
        assert_eq!(server.raw(b"GET /healthz HTTP/1.1\r\nHost: attacker.example.invalid\r\nX-Forwarded-Host: reader.example.invalid\r\n\r\n").status,421);
        assert_eq!(
            server
                .raw(b"GET /readyz HTTP/1.1\r\nHost: localhost:8800\r\n\r\n")
                .status,
            200
        );
        assert_eq!(server.raw(b"GET /healthz HTTP/1.1\r\nHost: reader.example.invalid\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n").status,if verb == "view" {413} else {400});
    }
}
#[test]
fn http_readers_observe_external_commits_and_keep_historical_documents() {
    for backend in ["file", "sqlite"] {
        let world = World::seeded(backend);
        let viewer = Server::start(world.command(&["view", "--port", "0"]));
        let mcp = Server::start(world.command(&["mcp-http", "--port", "0"]));
        let original = viewer.get("/projection?revision=0").json();
        let old = mcp.mcp(&call("overview", json!({"revision":0}))).json();
        assert_eq!(viewer.get("/head").json()["head"], 0);
        world.commit();
        assert_eq!(viewer.get("/head").json()["head"], 1);
        assert_eq!(
            mcp.mcp(&call("head", json!({}))).json()["result"]["structuredContent"]["head"],
            1
        );
        assert_eq!(viewer.get("/projection?revision=0").json(), original);
        assert_eq!(
            mcp.mcp(&call("overview", json!({"revision":0}))).json(),
            old
        );
    }
}
