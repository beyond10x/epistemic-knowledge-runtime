//! Process-level static guidance boundaries. May be compiled with std-only rustc --test.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn executable() -> PathBuf {
    std::env::var_os("PROBE_EKR")
        .map(PathBuf::from)
        .or_else(|| option_env!("CARGO_BIN_EXE_ekr").map(PathBuf::from))
        .expect("PROBE_EKR or Cargo binary")
}
struct Server {
    path: PathBuf,
    child: Option<Child>,
    authority: String,
}
impl Server {
    fn fixture() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "agent-guidance-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&path).unwrap();
        Self {
            path,
            child: None,
            authority: String::new(),
        }
    }
    fn command(&self) -> Command {
        let fixture = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("tests/fixtures/retraction/host.json");
        let mut command = Command::new(executable());
        for key in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
            command.env_remove(key);
        }
        command
            .arg("--host")
            .arg(fixture)
            .arg("--store")
            .arg(self.path.join("store"))
            .args(["--backend", "sqlite", "view", "--port", "0"]);
        command
    }
    fn start(mut self, options: &[&str]) -> Self {
        let mut child = self
            .command()
            .args(options)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut line = String::new();
        let stdout = child.stdout.take().unwrap();
        self.child = Some(child);
        BufReader::new(stdout).read_line(&mut line).unwrap();
        self.authority = line
            .split('"')
            .nth(3)
            .expect("listener announcement")
            .strip_prefix("http://")
            .unwrap()
            .trim_end_matches('/')
            .into();
        self
    }
    fn request(&self, method: &str, path: &str, headers: &str) -> (u16, String, String) {
        let mut socket = TcpStream::connect(&self.authority).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        write!(socket, "{method} {path} HTTP/1.1\r\n{headers}\r\n").unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        (
            head.split_whitespace().nth(1).unwrap().parse().unwrap(),
            head.into(),
            body.into(),
        )
    }
    fn get(&self, path: &str) -> (u16, String, String) {
        self.request("GET", path, &format!("Host: {}\r\n", self.authority))
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn corrupt_store_never_changes_static_guidance_or_fabricates_readiness() {
    let fixture = Server::fixture();
    let corrupt = b"synthetic invalid SQLite store: no knowledge records";
    std::fs::write(fixture.path.join("store"), corrupt).unwrap();
    let server = fixture.start(&[]);
    let before = server.get("/agent-guide.md");
    assert_eq!(before.0, 200);
    assert!(before.2.contains("MCP endpoint is not configured"));
    assert!(before.2.contains("`head`"));
    assert_eq!(server.get("/readyz").0, 503);
    assert_eq!(server.get("/agent-guide.md"), before);
    let (status, headers, discovery) = server.get("/llms.txt");
    assert_eq!(status, 200);
    assert!(discovery.contains("[Agent connection guide](/agent-guide.md)"));
    assert!(headers.contains("Cache-Control: no-store"));
    assert!(headers.contains("X-Content-Type-Options: nosniff"));
    assert!(headers.contains("Content-Type: text/markdown; charset=utf-8"));
    assert_eq!(std::fs::read(server.path.join("store")).unwrap(), corrupt);
    let (status, _, search) = server.get("/find?q=Example&revision=9");
    assert_eq!(status, 503);
    assert!(search.contains("Connect an agent"));
}

#[test]
fn static_routes_reject_duplicate_authority_framing_and_queries_before_store_access() {
    let server = Server::fixture().start(&["--mcp-url", "https://agent.example.invalid/mcp"]);
    let host = format!("Host: {}\r\n", server.authority);
    for route in ["/agent-guide.md", "/llms.txt"] {
        for (method, target, headers, expected) in [
            ("GET", route.to_owned(), String::new(), 421),
            ("GET", route.to_owned(), format!("{host}{host}"), 421),
            (
                "GET",
                route.to_owned(),
                "Host: unrelated.example.invalid\r\n".into(),
                421,
            ),
            ("HEAD", route.to_owned(), host.clone(), 405),
            ("DELETE", route.to_owned(), host.clone(), 405),
            (
                "GET",
                route.to_owned(),
                format!("{host}Content-Length: 0\r\nContent-Length: 0\r\n"),
                413,
            ),
            (
                "GET",
                route.to_owned(),
                format!("{host}Transfer-Encoding: chunked\r\n"),
                413,
            ),
            ("GET", format!("{route}?"), host.clone(), 400),
            (
                "GET",
                format!("{route}?q=Example&revision=1"),
                host.clone(),
                400,
            ),
        ] {
            let (status, _, body) = server.request(method, &target, &headers);
            assert_eq!(status, expected, "{method} {target}: {body}");
            assert!(!body.contains("agent.example.invalid"));
        }
        let (status, _, body) =
            server.request("GET", route, &format!("{host}Content-Length: 0\r\n"));
        assert_eq!(status, 200);
        assert!(body.contains("https://agent.example.invalid/mcp"));
    }
    assert!(!server.path.join("store").exists());
}

#[test]
fn maximum_url_is_preserved_and_one_extra_byte_is_refused() {
    let prefix = "https://agent.example.invalid/";
    let endpoint = format!("{prefix}{}", "a".repeat(4096 - prefix.len()));
    let guide = "https://docs.example.invalid/a(b)[c]?x=%3Ctag%3E&y='q'";
    let server = Server::fixture().start(&["--mcp-url", &endpoint, "--agent-guide-url", guide]);
    let (status, _, body) = server.get("/agent-guide.md");
    assert_eq!(status, 200);
    assert!(body.contains(&format!("```text\n{endpoint}\n```")));
    let (status, _, discovery) = server.get("/llms.txt");
    assert_eq!(status, 200);
    assert!(discovery.contains(&format!("[MCP endpoint]({endpoint})")));
    assert!(discovery.contains("a%28b%29[c]?x=%3Ctag%3E&y='q'"));
    let (_, _, page) = server.get("/find");
    assert!(page.contains("a(b)[c]?x=%3Ctag%3E&amp;y=&#39;q&#39;"));
    for flag in ["--mcp-url", "--agent-guide-url"] {
        let output = server
            .command()
            .args([flag, &format!("{endpoint}a")])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn explicit_require_ready_still_refuses_before_url_despite_static_guides() {
    let fixture = Server::fixture();
    let output = fixture
        .command()
        .args([
            "--require-ready",
            "--mcp-url",
            "https://agent.example.invalid/mcp",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!fixture.path.join("store").exists());
    let server = fixture.start(&["--mcp-url", "https://agent.example.invalid/mcp"]);
    assert_eq!(server.get("/readyz").0, 503);
    assert_eq!(server.get("/agent-guide.md").0, 200);
}
