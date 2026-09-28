//! `story:data-free-graph-viewer`: the page `ekr view` embeds names nothing a store holds.
//!
//! Every fixture store is a directory holding `host.json`, `seed.yaml` and, optionally,
//! `commit-<n>.yaml` transactions that are proposed, validated and committed in name order. The
//! stores live under `tests/fixtures/view/` (the role-derivation stores) and
//! `tests/fixtures/view-page/` (two stand-ins with different ontologies); each directory that
//! exists must hold at least two. Each store is seeded through the real binary into a temporary
//! directory, and the names it holds are read off its own `ekr.graph-projection/1` at the head:
//! every node type, edge type and property name, every node's canonical name, aliases and state,
//! and every text value. The page must contain none of them as a word, in any case.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use ekr::host::CliHostConfigurationV1;
use ekr_core::{NodeId, RevisionNumber};
use ekr_kernel::Runtime;
use ekr_views::{
    ExpandRequest, Index, OverviewRequest, QueryError, SearchRequest, SlicePage, SliceRecord,
};
use serde_json::Value;

/// The five addresses the page may read, and no other: the overview, the streamed expansion, one
/// node's detail, the search and evidence by id (the `ekr view` data contract).
const PAGE_ADDRESSES: [&str; 5] = ["/overview", "/expand", "/node/", "/search", "/evidence/"];

/// The three addresses the earlier page at `/alt` reads, and no other.
const ALT_ADDRESSES: [&str; 3] = ["/projection", "/roles", "/evidence/"];

/// How many nodes the page draws before it asks first.
const RENDER_BUDGET: &str = "20000";

/// The only scripts the page loads from elsewhere, each pinned to one version.
const LIBRARIES: [&str; 4] = [
    "https://cdn.jsdelivr.net/npm/graphology@0.26.0/dist/graphology.umd.min.js",
    "https://cdn.jsdelivr.net/npm/graphology-library@0.8.0/dist/graphology-library.min.js",
    "https://cdn.jsdelivr.net/npm/sigma@3.0.3/dist/sigma.min.js",
    "https://cdn.jsdelivr.net/npm/3d-force-graph@1.80.0/dist/3d-force-graph.min.js",
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
}

fn page() -> String {
    std::fs::read_to_string(manifest_dir().join("src/cli/viewer/index.html")).unwrap()
}

/// The earlier page `ekr view` keeps at `/alt`.
fn alt_page() -> String {
    std::fs::read_to_string(manifest_dir().join("src/cli/viewer/alt.html")).unwrap()
}

/// Both embedded pages, each held to the same data-free and no-markup rules.
fn pages() -> [(&'static str, String); 2] {
    [("index.html", page()), ("alt.html", alt_page())]
}

/// Every store directory under `tests/fixtures/<collection>`, or `None` when the collection is
/// absent.
fn stores_in(collection: &str) -> Option<Vec<PathBuf>> {
    let root = manifest_dir().join("tests/fixtures").join(collection);
    if !root.is_dir() {
        return None;
    }
    let mut stores: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("seed.yaml").is_file())
        .collect();
    stores.sort();
    Some(stores)
}

/// Every fixture store: the stand-ins always, the role-derivation stores once they exist.
fn fixture_stores() -> Vec<PathBuf> {
    let stand_ins = stores_in("view-page").expect("tests/fixtures/view-page exists");
    assert!(
        stand_ins.len() >= 2,
        "tests/fixtures/view-page holds {} stores; the story needs two with different ontologies",
        stand_ins.len()
    );
    let mut all = stand_ins;
    if let Some(stores) = stores_in("view") {
        assert!(
            stores.len() >= 2,
            "tests/fixtures/view holds {} stores; the story needs two with different ontologies",
            stores.len()
        );
        all.extend(stores);
    }
    all
}

/// One fixture store, seeded and committed into a directory of its own.
struct Seeded {
    fixture: PathBuf,
    directory: tempfile::TempDir,
}

impl Seeded {
    fn new(fixture: &Path) -> Self {
        let seeded = Self {
            fixture: fixture.to_owned(),
            directory: tempfile::tempdir().unwrap(),
        };
        let seed = fixture.join("seed.yaml").display().to_string();
        seeded.ok(&["seed", &seed]);
        let mut commits: Vec<PathBuf> = std::fs::read_dir(fixture)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("commit-") && name.ends_with(".yaml"))
            })
            .collect();
        commits.sort();
        for (at, commit) in commits.iter().enumerate() {
            let proposed = seeded.ok(&["propose", &commit.display().to_string()]);
            let id = proposed["transaction_id"].as_str().unwrap().to_owned();
            let against = at.to_string();
            let validated = seeded.ok(&["validate", &id, "--against", &against]);
            assert_eq!(validated["kind"], "Validated", "{}", commit.display());
            seeded.ok(&["commit", &id]);
        }
        seeded
    }

    /// The fixture's own `host.json`, else the host the role-derivation stores are seeded under
    /// (`tests/fixtures/retraction/host.json`, as `view_roles.rs` seeds them).
    fn host(&self) -> PathBuf {
        let own = self.fixture.join("host.json");
        if own.is_file() {
            own
        } else {
            manifest_dir().join("tests/fixtures/retraction/host.json")
        }
    }

    fn store(&self) -> PathBuf {
        self.directory.path().join("store")
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            self.host().display().to_string(),
            "--store".to_owned(),
            self.store().display().to_string(),
            "--backend".to_owned(),
            "file".to_owned(),
        ];
        args.extend(verb.iter().map(|arg| (*arg).to_owned()));
        args
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(self.args(verb))
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: stderr {}",
            self.fixture.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn runtime(&self) -> Runtime {
        let host = CliHostConfigurationV1::from_json(&std::fs::read(self.host()).unwrap()).unwrap();
        Runtime::file_existing(&self.store(), &host.tenant, host.context, host.authority).unwrap()
    }

    /// The head projection's bytes, as `ekr-views` renders them.
    fn projection_bytes(&self) -> Vec<u8> {
        ekr_views::project(&self.runtime(), None).unwrap().bytes
    }

    fn projection(&self) -> Value {
        serde_json::from_slice(&self.projection_bytes()).unwrap()
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
        let printed: Value = serde_json::from_str(&line)
            .unwrap_or_else(|error| panic!("`ekr view` printed {line:?}: {error}"));
        let url = printed["url"].as_str().unwrap().to_owned();
        Server { child, url }
    }
}

struct Server {
    child: Child,
    url: String,
}

impl Server {
    fn address(&self) -> String {
        self.url
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned()
    }

    /// `GET path`: the status and the body.
    fn get(&self, path: &str) -> (u16, Vec<u8>) {
        let address = self.address();
        let mut stream = TcpStream::connect(&address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
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
}

impl Drop for Server {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

/// Every name a projection carries: type, edge-type and property names from `ontology`, each
/// node's canonical name, aliases and state, and every text or enum value, at any depth.
fn names(projection: &Value) -> Vec<String> {
    fn values(value: &Value, into: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                if matches!(
                    map.get("kind").and_then(Value::as_str),
                    Some("String" | "Enum")
                ) {
                    if let Some(text) = map.get("value").and_then(Value::as_str) {
                        into.push(text.to_owned());
                    }
                }
                map.values().for_each(|inner| values(inner, into));
            }
            Value::Array(items) => items.iter().for_each(|inner| values(inner, into)),
            _ => {}
        }
    }
    let texts = |list: &Value, field: &str| -> Vec<String> {
        list.as_array()
            .unwrap()
            .iter()
            .filter_map(|item| item[field].as_str().map(str::to_owned))
            .collect()
    };
    let ontology = &projection["ontology"];
    let mut all = Vec::new();
    all.extend(texts(&ontology["node_types"], "name"));
    all.extend(texts(&ontology["edge_types"], "name"));
    all.extend(texts(&ontology["properties"], "name"));
    all.extend(texts(&projection["nodes"], "name"));
    all.extend(texts(&projection["nodes"], "state"));
    for node in projection["nodes"].as_array().unwrap() {
        for alias in node["aliases"].as_array().unwrap() {
            all.push(alias.as_str().unwrap().to_owned());
        }
    }
    values(&projection["nodes"], &mut all);
    values(&projection["edges"], &mut all);
    all.retain(|name| !name.trim().is_empty());
    all.sort();
    all.dedup();
    all
}

/// Whether `name` occurs in `text` as a word — not inside a longer run of letters, digits or
/// `_` — ignoring case.
fn occurs_as_word(text: &str, name: &str) -> bool {
    let text = text.to_lowercase();
    let name = name.to_lowercase();
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    text.match_indices(&name).any(|(at, found)| {
        !word(text[..at].chars().next_back()) && !word(text[at + found.len()..].chars().next())
    })
}

#[test]
fn the_embedded_page_names_nothing_any_fixture_store_holds() {
    let pages = pages();
    let mut checked = 0;
    for fixture in fixture_stores() {
        let projection = Seeded::new(&fixture).projection();
        let ontology = &projection["ontology"];
        for part in ["node_types", "edge_types", "properties"] {
            assert!(
                !ontology[part].as_array().unwrap().is_empty(),
                "{}: its ontology declares no {part}, so the check would prove nothing",
                fixture.display()
            );
        }
        assert!(!projection["nodes"].as_array().unwrap().is_empty());
        let names = names(&projection);
        for (file, page) in &pages {
            let named: Vec<&String> = names
                .iter()
                .filter(|name| occurs_as_word(page, name))
                .collect();
            assert!(
                named.is_empty(),
                "the embedded {file} names {named:?}, held by {}",
                fixture.display()
            );
        }
        checked += names.len();
    }
    assert!(checked > 0);
}

#[test]
fn the_fixture_stores_have_different_ontologies() {
    let ontologies: Vec<(PathBuf, Vec<String>)> = fixture_stores()
        .into_iter()
        .map(|fixture| {
            let projection = Seeded::new(&fixture).projection();
            let mut declared: Vec<String> = ["node_types", "edge_types", "properties"]
                .iter()
                .flat_map(|part| {
                    projection["ontology"][part]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|item| item["name"].as_str().unwrap().to_owned())
                        .collect::<Vec<_>>()
                })
                .collect();
            declared.sort();
            (fixture, declared)
        })
        .collect();
    for (at, (one, first)) in ontologies.iter().enumerate() {
        for (other, second) in &ontologies[at + 1..] {
            let shared: Vec<&String> = first.iter().filter(|name| second.contains(name)).collect();
            assert!(
                shared.is_empty(),
                "{} and {} share names {shared:?}",
                one.display(),
                other.display()
            );
        }
    }
}

#[test]
fn the_word_check_finds_a_name_in_any_case_and_only_as_a_word() {
    assert!(occurs_as_word("a Sample here", "sample"));
    assert!(occurs_as_word("x SAMPLED_BY y", "sampled_by"));
    assert!(occurs_as_word("\"Kestrel-7\"", "Kestrel-7"));
    assert!(!occurs_as_word("resampled", "sample"));
    assert!(!occurs_as_word("samples", "sample"));
    assert!(!occurs_as_word("", "sample"));
}

#[test]
fn the_embedded_page_writes_no_markup_and_evaluates_nothing() {
    for (file, page) in pages() {
        for forbidden in [
            "innerHTML",
            "outerHTML",
            "insertAdjacentHTML",
            "document.write",
            "new Function",
            "Function(",
            "setTimeout(\"",
            "setInterval(\"",
        ] {
            assert!(!page.contains(forbidden), "{file} uses {forbidden}");
        }
        assert!(!occurs_as_word(&page, "eval"), "{file} uses eval");
        assert!(
            page.contains("textContent"),
            "{file} writes what it fetches as text"
        );
    }
}

/// The page's string literals that begin with `/`.
fn absolute_literals(page: &str) -> Vec<String> {
    let mut found = Vec::new();
    for quote in ['"', '\''] {
        let mut rest = page;
        while let Some(start) = rest.find(quote) {
            let after = &rest[start + 1..];
            let Some(end) = after.find(quote) else { break };
            let literal = &after[..end];
            if literal.starts_with('/') && !literal.contains(' ') && !literal.contains('\n') {
                found.push(literal.to_owned());
            }
            rest = &after[end + 1..];
        }
    }
    found.sort();
    found.dedup();
    found
}

/// The page with its pinned library tags and its policy line taken out, after checking that each
/// tag is there exactly once, pinned, with a sha384 integrity and no credentials, and that the
/// policy allows exactly those scripts, blob workers and reads from this server.
fn without_pinned_libraries(page: &str) -> String {
    let mut rest = page.to_owned();
    for url in LIBRARIES {
        let start = format!("<script src=\"{url}\" integrity=\"sha384-");
        assert_eq!(
            rest.matches(&start).count(),
            1,
            "the page loads {url} once, with a sha384 integrity"
        );
        let at = rest.find(&start).unwrap();
        let end = at + rest[at..].find("</script>").unwrap() + "</script>".len();
        let tag = rest[at..end].to_owned();
        let digest = tag[start.len()..].split('"').next().unwrap();
        assert_eq!(
            digest.len(),
            64,
            "{url}: a sha384 digest is 64 base64 characters"
        );
        assert!(
            digest
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/'),
            "{url}: {digest}"
        );
        assert!(
            tag.ends_with("\" crossorigin=\"anonymous\" referrerpolicy=\"no-referrer\"></script>"),
            "{tag}"
        );
        rest.replace_range(at..end, "");
    }
    let line = rest
        .lines()
        .find(|line| line.contains("Content-Security-Policy"))
        .expect("the page carries a Content-Security-Policy")
        .to_owned();
    let policy = line
        .split("content=\"")
        .nth(1)
        .and_then(|content| content.split('"').next())
        .unwrap();
    let directives: Vec<(String, Vec<String>)> = policy
        .split(';')
        .map(str::trim)
        .filter(|directive| !directive.is_empty())
        .map(|directive| {
            let mut words = directive.split_whitespace().map(str::to_owned);
            (words.next().unwrap(), words.collect())
        })
        .collect();
    let sources = |name: &str| -> Vec<String> {
        directives
            .iter()
            .find(|(directive, _)| directive == name)
            .map(|(_, sources)| sources.clone())
            .unwrap_or_else(|| panic!("the policy has no {name}: {policy}"))
    };
    let mut scripts = sources("script-src");
    scripts.sort();
    let mut expected: Vec<String> = LIBRARIES.iter().map(|url| (*url).to_owned()).collect();
    expected.push("'unsafe-inline'".to_owned());
    expected.sort();
    assert_eq!(
        scripts, expected,
        "script-src is exactly the pinned libraries"
    );
    assert_eq!(sources("default-src"), ["'none'"], "{policy}");
    assert_eq!(sources("connect-src"), ["'self'"], "{policy}");
    assert_eq!(sources("worker-src"), ["blob:"], "{policy}");
    let names: Vec<&str> = directives.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "default-src",
            "script-src",
            "worker-src",
            "style-src",
            "connect-src"
        ],
        "{policy}"
    );
    assert!(!policy.contains("unsafe-eval"), "{policy}");
    rest.replace(&line, "")
}

#[test]
fn the_embedded_page_loads_only_the_pinned_libraries_by_integrity() {
    for (file, page) in pages() {
        let rest = without_pinned_libraries(&page);
        assert!(
            !rest.contains("<script src"),
            "{file}: no other script is loaded"
        );
        assert!(
            !rest.contains("cdn.jsdelivr.net"),
            "{file}: no other library address"
        );
    }
}

#[test]
fn each_embedded_page_reads_only_its_own_addresses() {
    reads_only(
        "index.html",
        &without_pinned_libraries(&page()),
        &PAGE_ADDRESSES,
    );
    reads_only(
        "alt.html",
        &without_pinned_libraries(&alt_page()),
        &ALT_ADDRESSES,
    );
}

/// The page reads an expansion as it arrives, can close it, and names its render budget.
#[test]
fn the_page_streams_an_expansion_and_keeps_a_render_budget() {
    let page = page();
    for needle in ["body.getReader()", ".cancel(", "x-ndjson", RENDER_BUDGET] {
        assert!(page.contains(needle), "index.html carries no {needle:?}");
    }
}

fn reads_only(file: &str, page: &str, addresses: &[&str]) {
    let literals = absolute_literals(page);
    let others: Vec<&String> = literals
        .iter()
        .filter(|literal| !addresses.contains(&literal.as_str()))
        .collect();
    assert!(
        others.is_empty(),
        "{file} names other addresses: {others:?}"
    );
    for address in addresses {
        assert!(
            literals.iter().any(|literal| literal == *address),
            "{file} does not read {address}: {literals:?}"
        );
    }
    assert_eq!(
        page.matches("fetch(").count(),
        1,
        "{file}: every read goes through one fetch"
    );
    for forbidden in [
        "http:",
        "https:",
        "XMLHttpRequest",
        "WebSocket",
        "EventSource",
        "sendBeacon",
        "import(",
        "<link",
        " src=",
        " href=",
        "<iframe",
        "<img",
    ] {
        assert!(!page.contains(forbidden), "{file} carries {forbidden:?}");
    }
}

#[test]
fn ekr_view_serves_each_fixture_store_the_page_its_projection_and_roles_or_none() {
    let page = page();
    let alt = alt_page();
    for fixture in fixture_stores() {
        let seeded = Seeded::new(&fixture);
        let expected = seeded.projection_bytes();
        let server = seeded.serve();
        let (status, body) = server.get("/");
        assert_eq!(status, 200, "{}", fixture.display());
        assert_eq!(
            body,
            page.as_bytes(),
            "{}: the embedded page",
            fixture.display()
        );
        let (status, body) = server.get("/alt");
        assert_eq!(status, 200, "{}: GET /alt", fixture.display());
        assert_eq!(
            body,
            alt.as_bytes(),
            "{}: the earlier page at /alt",
            fixture.display()
        );
        let (status, body) = server.get("/projection");
        assert_eq!(status, 200, "{}", fixture.display());
        assert_eq!(body, expected, "{}: its own projection", fixture.display());
        let (status, body) = server.get("/roles");
        match status {
            404 => {}
            200 => {
                let roles: Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(roles["format"], "ekr.view-roles/1", "{}", fixture.display());
            }
            other => panic!("{}: GET /roles answered {other}", fixture.display()),
        }
    }
}

/// A local headless Chromium, or `None`: `EKR_VIEW_BROWSER`, else the newest Playwright
/// headless shell under the home directory's cache.
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

/// One browser at a time in this binary: each is held to two cores at the lowest priority, and several
/// at once on a loaded machine starve each other past any useful time. A case that panicked while
/// holding it leaves it poisoned, which the next case takes over.
fn one_browser() -> std::sync::MutexGuard<'static, ()> {
    static BROWSER: Mutex<()> = Mutex::new(());
    BROWSER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The DOM the page has built after `url` loads and its reads settle.
fn rendered(browser: &Path, url: &str) -> String {
    let _one = one_browser();
    let profile = tempfile::tempdir().unwrap();
    // Two cores at the lowest priority: the machine is shared.
    let output = Command::new("taskset")
        .args(["-c", "0-1", "nice", "-n", "19"])
        .arg(browser)
        .args([
            "--headless",
            "--use-angle=swiftshader",
            "--enable-unsafe-swiftshader",
            "--no-sandbox",
            "--no-first-run",
            "--disable-extensions",
            &format!("--user-data-dir={}", profile.path().display()),
            "--virtual-time-budget=8000",
            "--dump-dom",
            url,
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success(), "the browser failed on {url}");
    String::from_utf8(output.stdout).unwrap()
}

/// Text as `--dump-dom` writes it inside an element.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ---- a stand-in for the streamed endpoints ---------------------------------------------------------
//
// Until the server unit lands, `ekr view` serves `/projection` and `/roles` only. `Double` is a
// fixture server that answers the data contract's paths — `/overview`, `/expand` (NDJSON, chunked),
// `/node/<id>`, `/search`, `/evidence/<id>` — from the `ekr-views` engine directly: one `Index`
// per revision of the seeded store, built with `Index::load`. It serves the embedded page at `/`
// and records every request target, so a case can say what the page read and what it did not.

/// Everything the stand-in answers from.
struct Engine {
    head: u64,
    indexes: BTreeMap<u64, Index>,
    /// Retained evidence bytes by evidence id.
    evidence: HashMap<String, Vec<u8>>,
    page: Vec<u8>,
    /// A pause after each flushed chunk of an expansion, so one can be seen in progress.
    delay: Duration,
}

struct Double {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Seeded {
    fn double(&self, delay: Duration) -> Double {
        let runtime = self.runtime();
        let head = runtime.head().unwrap().unwrap().revision.get();
        let mut indexes = BTreeMap::new();
        let mut evidence = HashMap::new();
        for at in 0..=head {
            let index = Index::load(&runtime, Some(RevisionNumber::new(at))).unwrap();
            for (id, item) in &index.loaded().graph.evidence {
                if let Some(bytes) = runtime.content(&item.content_hash).unwrap() {
                    evidence.insert(id.to_string(), bytes);
                }
            }
            indexes.insert(at, index);
        }
        Double::start(Engine {
            head,
            indexes,
            evidence,
            page: page().into_bytes(),
            delay,
        })
    }
}

impl Double {
    fn start(engine: Engine) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (log, halt) = (Arc::clone(&requests), Arc::clone(&stop));
        let thread = std::thread::spawn(move || {
            let engine = Arc::new(engine);
            for connection in listener.incoming() {
                if halt.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(connection) = connection else { continue };
                let (engine, log) = (Arc::clone(&engine), Arc::clone(&log));
                std::thread::spawn(move || answer(connection, &engine, &log));
            }
        });
        Self {
            url,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    /// Every request target the page sent so far, in arrival order, without the browser's own.
    fn requests(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|target| !target.starts_with("/favicon"))
            .cloned()
            .collect()
    }
}

impl Drop for Double {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let address = self.url.trim_start_matches("http://").trim_end_matches('/');
        TcpStream::connect(address).ok();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}

/// `%XX` decoded; anything else as it stands.
fn percent_decoded(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let hex = std::str::from_utf8(bytes.get(at + 1..at + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn query_of(query: &str) -> Option<HashMap<String, String>> {
    let mut values = HashMap::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=')?;
        if values
            .insert(percent_decoded(key)?, percent_decoded(value)?)
            .is_some()
        {
            return None;
        }
    }
    Some(values)
}

fn reply(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         X-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).ok();
    stream.write_all(body).ok();
}

fn refusal(stream: &mut TcpStream, status: u16, name: &str, detail: &str) {
    let body = serde_json::json!({"refusal": name, "detail": detail}).to_string();
    reply(stream, status, "application/json", body.as_bytes());
}

fn answer(mut stream: TcpStream, engine: &Engine, log: &Mutex<Vec<String>>) {
    stream.set_read_timeout(Some(Duration::from_secs(20))).ok();
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
    log.lock().unwrap().push(target.clone());
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    let Some(query) = query_of(query) else {
        return refusal(&mut stream, 400, "invalid-query", query);
    };
    if path == "/" {
        return reply(&mut stream, 200, "text/html; charset=utf-8", &engine.page);
    }
    if let Some(id) = path.strip_prefix("/evidence/") {
        return match engine.evidence.get(id) {
            Some(bytes) => reply(&mut stream, 200, "text/plain; charset=utf-8", bytes),
            None => reply(
                &mut stream,
                404,
                "text/plain; charset=utf-8",
                b"evidence-not-found",
            ),
        };
    }
    let revision = match query.get("revision").map(|text| text.parse::<u64>()) {
        None => engine.head,
        Some(Ok(at)) => at,
        Some(Err(_)) => return refusal(&mut stream, 400, "invalid-query", "revision"),
    };
    let Some(index) = engine.indexes.get(&revision) else {
        return refusal(&mut stream, 404, "RevisionNotFound", &revision.to_string());
    };
    let number = |key: &str| -> Result<Option<i64>, ()> {
        query
            .get(key)
            .map(|text| text.parse::<i64>().map_err(|_| ()))
            .transpose()
    };
    let queried = |error: QueryError, stream: &mut TcpStream| match error {
        QueryError::LimitExceeded(limit) => {
            refusal(stream, 400, "LimitExceeded", &limit.to_string());
        }
        QueryError::NodeNotFound { node, .. } => {
            refusal(stream, 404, "NodeNotFound", &node.to_string());
        }
        QueryError::Project(error) => refusal(stream, 500, "Read", &error.to_string()),
    };
    match path {
        "/overview" => {
            let Ok(limit) = number("limit") else {
                return refusal(&mut stream, 400, "invalid-query", "limit");
            };
            match OverviewRequest::new(limit) {
                Ok(request) => {
                    let bytes = index.overview(&request).unwrap().bytes;
                    reply(&mut stream, 200, "application/json", &bytes);
                }
                Err(limit) => refusal(&mut stream, 400, "LimitExceeded", &limit.to_string()),
            }
        }
        "/search" => {
            let (Some(text), Ok(limit)) = (query.get("q"), number("limit")) else {
                return refusal(&mut stream, 400, "invalid-query", "q");
            };
            match SearchRequest::new(text.clone(), limit.unwrap_or(20)) {
                Ok(request) => {
                    let bytes = index.search(&request).unwrap().bytes;
                    reply(&mut stream, 200, "application/json", &bytes);
                }
                Err(limit) => refusal(&mut stream, 400, "LimitExceeded", &limit.to_string()),
            }
        }
        "/expand" => {
            let seeds: Option<Vec<NodeId>> = query.get("seeds").and_then(|seeds| {
                seeds
                    .split(',')
                    .map(|seed| seed.parse::<NodeId>().ok())
                    .collect()
            });
            let (Some(seeds), Ok(Some(depth)), Ok(Some(limit)), Ok(edges), Ok(after)) = (
                seeds,
                number("depth"),
                number("limit"),
                number("edges"),
                number("after"),
            ) else {
                return refusal(&mut stream, 400, "invalid-query", "expand");
            };
            let page = ExpandRequest::new(seeds, depth, limit, edges, after)
                .map_err(QueryError::from)
                .and_then(|request| index.page(&request));
            match page {
                Ok(page) => stream_slice(&mut stream, &page, engine.delay),
                Err(error) => queried(error, &mut stream),
            }
        }
        _ => match path.strip_prefix("/node/").map(str::parse::<NodeId>) {
            Some(Ok(node)) => match index.describe(node) {
                Ok(answer) => reply(&mut stream, 200, "application/json", &answer.bytes),
                Err(error) => queried(error, &mut stream),
            },
            Some(Err(_)) => refusal(&mut stream, 400, "invalid-query", path),
            None => reply(&mut stream, 404, "text/plain; charset=utf-8", b"not-found"),
        },
    }
}

/// `{"kind":<kind>, …fields of value}` on one line.
fn record_line(kind: &str, value: &impl serde::Serialize) -> String {
    let fields = serde_json::to_string(value).unwrap();
    let rest = fields.strip_prefix('{').unwrap();
    if rest == "}" {
        format!("{{\"kind\":\"{kind}\"}}\n")
    } else {
        format!("{{\"kind\":\"{kind}\",{rest}\n")
    }
}

/// The contract's stream: meta, the records in order with a progress line after every 256, and an
/// end line; one chunk per 256 records.
fn stream_slice(stream: &mut TcpStream, page: &SlicePage, delay: Duration) {
    let head = "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\n\
                Transfer-Encoding: chunked\r\nX-Content-Type-Options: nosniff\r\n\
                Cache-Control: no-store\r\nConnection: close\r\n\r\n";
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    let chunk = |stream: &mut TcpStream, text: &str| -> bool {
        let framed = format!("{:x}\r\n{text}\r\n", text.len());
        stream.write_all(framed.as_bytes()).is_ok() && stream.flush().is_ok()
    };
    let mut text = record_line("meta", page.meta());
    for (at, record) in page.records().iter().enumerate() {
        text.push_str(&match record {
            SliceRecord::Node(node) => record_line("node", node),
            SliceRecord::Edge(edge) => record_line("edge", edge),
        });
        if (at + 1).is_multiple_of(256) {
            text.push_str(&format!("{{\"kind\":\"progress\",\"sent\":{}}}\n", at + 1));
            if !chunk(stream, &text) {
                return;
            }
            text.clear();
            std::thread::sleep(delay);
        }
    }
    let next = page
        .next()
        .map_or_else(|| "null".to_owned(), |next| next.to_string());
    text.push_str(&format!(
        "{{\"kind\":\"end\",\"next\":{next},\"remaining\":{}}}\n",
        page.remaining()
    ));
    if chunk(stream, &text) {
        stream.write_all(b"0\r\n\r\n").ok();
    }
}

/// The ids of what the overview draws first: its `top`.
fn top_ids(index: &Index) -> BTreeSet<String> {
    let overview: Value = serde_json::from_slice(
        &index
            .overview(&OverviewRequest::new(None).unwrap())
            .unwrap()
            .bytes,
    )
    .unwrap();
    overview["top"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_owned())
        .collect()
}

/// The node ids of the first page of `seed`'s neighbourhood as the page asks for it on a click.
fn clicked_page(index: &Index, seed: &str) -> SlicePage {
    let request = ExpandRequest::new(vec![seed.parse().unwrap()], 1, 500, None, None).unwrap();
    index.page(&request).unwrap()
}

fn page_nodes(page: &SlicePage) -> BTreeSet<String> {
    page.records()
        .iter()
        .filter_map(|record| match record {
            SliceRecord::Node(node) => Some(node.id.to_string()),
            SliceRecord::Edge(_) => None,
        })
        .collect()
}

/// The page's count of what it has drawn, as the left panel states it.
fn drawn(nodes: usize, total: u64) -> String {
    format!(
        "{} of {} nodes drawn",
        grouped(nodes as u64),
        grouped(total)
    )
}

/// `n` with its thousands separated by commas, as the page writes a count.
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

fn reads_no_projection(requests: &[String], what: &str) {
    assert!(
        requests.first().is_some_and(|first| first == "/"),
        "{what}: {requests:?}"
    );
    assert!(
        requests
            .get(1)
            .is_some_and(|second| second.starts_with("/overview")),
        "{what}: the first read is the overview: {requests:?}"
    );
    for target in requests {
        assert!(
            target == "/"
                || ["/overview", "/expand?", "/node/", "/search?", "/evidence/"]
                    .iter()
                    .any(|address| target.starts_with(address)),
            "{what}: the page read {target}: {requests:?}"
        );
    }
}

#[test]
fn the_page_draws_each_store_from_its_overview_and_never_reads_the_projection() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    for fixture in fixture_stores() {
        let seeded = Seeded::new(&fixture);
        let projection = seeded.projection();
        let double = seeded.double(Duration::ZERO);
        let dom = rendered(&browser, &double.url);
        let head = projection["meta"]["head"].as_u64().unwrap();
        assert!(
            dom.contains(&format!("revision {head} of {head}")),
            "{}: the status names the revision: {dom}",
            fixture.display()
        );
        // Every node type the overview counts a node of is a filter chip, named as `ontology`
        // names it.
        let nodes = projection["nodes"].as_array().unwrap();
        for node_type in projection["ontology"]["node_types"].as_array().unwrap() {
            if !nodes.iter().any(|node| node["type"] == node_type["id"]) {
                continue;
            }
            let name = escaped(node_type["name"].as_str().unwrap());
            assert!(dom.contains(&name), "{}: type {name}", fixture.display());
        }
        let top = top_ids(&double_index(&seeded, head));
        let total = projection["meta"]["node_count"].as_u64().unwrap();
        assert!(
            dom.contains(&drawn(top.len(), total)),
            "{}: the overview's {} top nodes are drawn: {dom}",
            fixture.display(),
            top.len()
        );
        reads_no_projection(&double.requests(), &fixture.display().to_string());
    }
}

/// The index `double` answers revision `at` from, built again for the case's own expectations.
fn double_index(seeded: &Seeded, at: u64) -> Index {
    Index::load(&seeded.runtime(), Some(RevisionNumber::new(at))).unwrap()
}

/// The node with the most assertions of `projection`, which has at least one.
fn busiest(projection: &Value) -> &Value {
    projection["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .max_by_key(|node| node["assertions"].as_array().unwrap().len())
        .unwrap()
}

#[test]
fn a_chosen_node_streams_its_neighbourhood_and_shows_its_detail_from_the_node_read() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    for fixture in fixture_stores() {
        let seeded = Seeded::new(&fixture);
        let projection = seeded.projection();
        let head = projection["meta"]["head"].as_u64().unwrap();
        let chosen = busiest(&projection);
        let claims = chosen["assertions"].as_array().unwrap();
        assert!(!claims.is_empty(), "{}", fixture.display());
        let id = chosen["id"].as_str().unwrap();
        let double = seeded.double(Duration::ZERO);
        let dom = rendered(&browser, &format!("{}#node={id}", double.url));

        // The detail: its name, every assertion by id as a card with its assessment, lifecycle,
        // valid and recorded time, and each property by the name `ontology` gives it.
        let name = escaped(chosen["name"].as_str().unwrap());
        assert!(
            dom.contains(&name),
            "{}: {name} is shown",
            fixture.display()
        );
        for claim in claims {
            let claim_id = claim["id"].as_str().unwrap();
            assert!(
                dom.contains(&format!("<th>{claim_id}</th>")),
                "{}: an assertion card for {claim_id}: {dom}",
                fixture.display()
            );
            for field in [
                claim["assessment"]["kind"].as_str().unwrap(),
                claim["lifecycle"]["kind"].as_str().unwrap(),
            ] {
                assert!(dom.contains(field), "{}: {field}", fixture.display());
            }
        }
        for field in ["assessment", "lifecycle", "valid", "recorded", "evidence"] {
            assert!(
                dom.contains(&format!("<td>{field}</td>")),
                "{}: a card row {field}",
                fixture.display()
            );
        }
        for property in chosen["props"].as_object().unwrap().keys() {
            let name = projection["ontology"]["properties"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["id"] == property.as_str())
                .unwrap()["name"]
                .as_str()
                .unwrap();
            assert!(dom.contains(name), "{}: property {name}", fixture.display());
        }

        // The neighbourhood streamed in: the top nodes and the first page of its 1-hop slice.
        let index = double_index(&seeded, head);
        let mut expected = top_ids(&index);
        expected.extend(page_nodes(&clicked_page(&index, id)));
        let total = projection["meta"]["node_count"].as_u64().unwrap();
        assert!(
            dom.contains(&drawn(expected.len(), total)),
            "{}: {} nodes drawn after the expansion: {dom}",
            fixture.display(),
            expected.len()
        );
        let requests = double.requests();
        reads_no_projection(&requests, &fixture.display().to_string());
        assert!(
            requests
                .iter()
                .any(|target| target.starts_with(&format!("/node/{id}"))),
            "{}: the detail is read from /node/{id}: {requests:?}",
            fixture.display()
        );
        assert!(
            requests.iter().any(|target| target.starts_with("/expand?")
                && target.contains(&format!("seeds={id}"))
                && target.contains("depth=1")),
            "{}: the neighbourhood is streamed from /expand: {requests:?}",
            fixture.display()
        );
    }
}

#[test]
fn evidence_is_read_only_by_an_id_a_shown_assertion_cites_and_shown_as_text() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    for fixture in fixture_stores() {
        let seeded = Seeded::new(&fixture);
        let projection = seeded.projection();
        let chosen = busiest(&projection);
        let id = chosen["id"].as_str().unwrap();
        let evidence = chosen["assertions"][0]["evidence"][0].as_str().unwrap();
        let double = seeded.double(Duration::ZERO);
        let text = escaped(&String::from_utf8(double_evidence(&seeded, evidence)).unwrap());
        let dom = rendered(
            &browser,
            &format!("{}#node={id}&evidence={evidence}", double.url),
        );
        assert!(
            dom.contains(&text),
            "{}: the evidence {evidence} is shown as text {text:?}",
            fixture.display()
        );
        let requests = double.requests();
        assert!(
            requests.contains(&format!("/evidence/{evidence}")),
            "{}: {requests:?}",
            fixture.display()
        );

        // The same id with no node shown is no id the page knows: it is never read.
        let double = seeded.double(Duration::ZERO);
        let dom = rendered(&browser, &format!("{}#evidence={evidence}", double.url));
        assert!(dom.contains(evidence), "{}: {dom}", fixture.display());
        let requests = double.requests();
        assert!(
            !requests
                .iter()
                .any(|target| target.starts_with("/evidence/")),
            "{}: an id no shown assertion cites was read: {requests:?}",
            fixture.display()
        );
    }
}

/// The retained bytes of `evidence`, as the store holds them.
fn double_evidence(seeded: &Seeded, evidence: &str) -> Vec<u8> {
    let runtime = seeded.runtime();
    let index = Index::load(&runtime, None).unwrap();
    let item = index
        .loaded()
        .graph
        .evidence
        .iter()
        .find(|(id, _)| id.to_string() == evidence)
        .unwrap()
        .1;
    runtime.content(&item.content_hash).unwrap().unwrap()
}

#[test]
fn a_search_in_the_address_asks_the_server_and_lists_its_matches() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    for fixture in fixture_stores() {
        let seeded = Seeded::new(&fixture);
        let projection = seeded.projection();
        let chosen = busiest(&projection);
        let name = chosen["name"].as_str().unwrap();
        let double = seeded.double(Duration::ZERO);
        let query: String = name
            .bytes()
            .map(|byte| {
                if byte.is_ascii_alphanumeric() {
                    char::from(byte).to_string()
                } else {
                    format!("%{byte:02X}")
                }
            })
            .collect();
        let dom = rendered(&browser, &format!("{}#q={query}", double.url));
        assert!(
            dom.contains(&format!("data-node=\"{}\"", chosen["id"].as_str().unwrap())),
            "{}: {name} is a hit: {dom}",
            fixture.display()
        );
        let requests = double.requests();
        assert!(
            requests
                .iter()
                .any(|target| target.starts_with("/search?q=")),
            "{}: {requests:?}",
            fixture.display()
        );
        reads_no_projection(&requests, &fixture.display().to_string());
    }
}

/// How many node types, edge types, nodes, edges and relation assertions the generated store holds.
const GENERATED: (usize, usize, usize, usize, usize) = (6, 3, 5000, 10000, 400);

/// An id of the generated store: `space` tells kinds of object apart, `n` numbers them.
fn generated_id(space: u16, n: usize) -> String {
    format!("00000000-0000-4000-{:04x}-{n:012x}", 0x8100 + space)
}

/// Writes a store fixture of `shape` (node types, edge types, nodes, edges, relation assertions) into
/// `directory`, from structure only: types are `type-<n>`, nodes `entity-<n>`, edge types `link-<n>`,
/// every edge type joins every node type, and the edges are a fixed pseudo-random draw — or, for a
/// `star`, every edge joins node 1 to another.
fn generated_fixture(directory: &Path, shape: (usize, usize, usize, usize, usize), star: bool) {
    use std::fmt::Write as _;
    let (types, edge_types, nodes, edges, relations) = shape;
    let evidence = directory.join("payload");
    std::fs::write(&evidence, b"generated evidence").unwrap();
    let hashed = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(["hash", &evidence.display().to_string()])
        .output()
        .unwrap();
    let hashed: Value = serde_json::from_slice(&hashed.stdout).unwrap();
    let hash = hashed["content_hash"].as_str().unwrap();
    let payload = hashed["payload_yaml"].as_str().unwrap();
    let version = generated_id(0, 1);
    let root = generated_id(0, 2);
    let operator = "00000000-0000-4000-8000-000000000101";
    let evidence_id = generated_id(5, 1);
    let type_ids: Vec<String> = (1..=types).map(|n| generated_id(1, n)).collect();
    let listed = |ids: &[String]| -> String {
        ids.iter().fold(String::new(), |mut out, id| {
            let _ = write!(out, "\n    - {id}");
            out
        })
    };
    let mut yaml = format!(
        "format: ekr-seed/2\nontology:\n  version:\n    id: {version}\n    number: 0\n    parent: null\n    created_at: 0\n  node_types:\n"
    );
    for (n, id) in type_ids.iter().enumerate() {
        let _ = write!(
            yaml,
            "  - id: {id}\n    name: type-{}\n    parents: []\n    properties: {{}}\n    abstract_type: false\n    lifecycle: null\n    operations: {{}}\n",
            n + 1
        );
    }
    yaml.push_str("  edge_types:\n");
    for n in 1..=edge_types {
        let _ = write!(
            yaml,
            "  - id: {}\n    name: link-{n}\n    source_types:{}\n    target_types:{}\n    cardinality: Many\n    properties: {{}}\n    inverse: null\n    symmetric: false\n    transitive: false\n",
            generated_id(2, n),
            listed(&type_ids),
            listed(&type_ids)
        );
    }
    let _ = write!(
        yaml,
        "graph:\n  format: ekr.graph-document/2\n  graph:\n    root:\n      id: {root}\n      space: Canonical\n      schema_version_id: {version}\n      parent: null\n      created_at: 0\n    revision: 0\n    nodes:\n"
    );
    for n in 1..=nodes {
        let id = generated_id(3, n);
        let _ = write!(
            yaml,
            "      {id}:\n        id: {id}\n        root_id: {root}\n        type_id: {}\n        canonical_name: entity-{n}\n        aliases: []\n        type_state: null\n        properties: {{}}\n",
            type_ids[n % types]
        );
    }
    // A fixed linear congruential draw, so every run holds the same graph.
    let mut draw = 0x2545_f491_u64;
    let mut next = |bound: usize| -> usize {
        draw = draw
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(draw >> 33).unwrap() % bound
    };
    yaml.push_str("    edges:\n");
    for n in 1..=edges {
        let id = generated_id(4, n);
        let source = if star { 1 } else { 1 + next(nodes) };
        let target = if star {
            1 + n
        } else {
            1 + (source + next(nodes - 1)) % nodes
        };
        let _ = write!(
            yaml,
            "      {id}:\n        id: {id}\n        root_id: {root}\n        type_id: {}\n        source: {}\n        target: {}\n        properties: {{}}\n",
            generated_id(2, 1 + n % edge_types),
            generated_id(3, source),
            generated_id(3, target)
        );
    }
    yaml.push_str("    assertions:\n");
    for n in 1..=relations {
        let id = generated_id(6, n);
        let subject = 1 + next(nodes);
        let object = 1 + (subject + next(nodes - 1)) % nodes;
        let from = 1_700_000_000_000_i64 + i64::try_from(n).unwrap() * 86_400_000;
        let _ = write!(
            yaml,
            "      {id}:\n        id: {id}\n        root_id: {root}\n        subject: !Node {}\n        predicate: !Relation {}\n        object: !Node {}\n        evidence:\n        - {evidence_id}\n        proposed_by: {operator}\n        assessment: Proposed\n        lifecycle: Active\n        valid_time:\n          from: {from}\n          to: null\n        transaction_time:\n          recorded_from: 0\n          recorded_to: null\n",
            generated_id(3, subject),
            generated_id(2, 1 + n % edge_types),
            generated_id(3, object)
        );
    }
    let _ = write!(
        yaml,
        "    evidence:\n      {evidence_id}:\n        id: {evidence_id}\n        source: !HumanStatement\n          identity: generated\n        content_hash: {hash}\n        extracted_by: {operator}\n        observed_at: 1700000000000\n        confidence: 10000\nevidence_payloads:\n  {hash}: {payload}\n"
    );
    std::fs::write(directory.join("seed.yaml"), yaml).unwrap();
    std::fs::copy(
        manifest_dir().join("tests/fixtures/view-page/sounding/host.json"),
        directory.join("host.json"),
    )
    .unwrap();
}

#[test]
fn a_generated_store_of_five_thousand_nodes_and_ten_thousand_edges_is_served_by_name_free_page() {
    let (_, _, nodes, edges, relations) = GENERATED;
    let fixture = tempfile::tempdir().unwrap();
    generated_fixture(fixture.path(), GENERATED, false);
    // A copy for a manual measurement in a real browser, when one is asked for.
    if let Ok(keep) = std::env::var("EKR_VIEW_GENERATED_DIR") {
        std::fs::create_dir_all(&keep).unwrap();
        for file in ["seed.yaml", "host.json"] {
            std::fs::copy(fixture.path().join(file), Path::new(&keep).join(file)).unwrap();
        }
    }
    let seeded = Seeded::new(fixture.path());
    let projection = seeded.projection();
    assert_eq!(projection["meta"]["node_count"], nodes);
    assert_eq!(projection["meta"]["edge_count"], edges);
    let server = seeded.serve();
    let (status, body) = server.get("/projection");
    assert_eq!(status, 200);
    assert_eq!(body, seeded.projection_bytes());
    let names = names(&projection);
    let page = page();
    assert!(
        names.iter().all(|name| !occurs_as_word(&page, name)),
        "the page names a generated name"
    );
    let relation_count: usize = projection["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| {
            node["assertions"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|claim| claim["predicate_kind"] == "Relation")
                .count()
        })
        .sum();
    assert_eq!(relation_count, relations);
    // The drawing half is measured in a real-time browser, not here: a headless browser under
    // virtual time draws the 3D view as fast as the machine allows for the whole budget.
}

#[test]
fn a_node_outside_the_overview_of_a_generated_store_is_streamed_in_when_chosen() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let (_, _, nodes, _, _) = GENERATED;
    let fixture = tempfile::tempdir().unwrap();
    generated_fixture(fixture.path(), GENERATED, false);
    let seeded = Seeded::new(fixture.path());
    let index = double_index(&seeded, 0);
    let top = top_ids(&index);
    assert!(top.len() < nodes, "the overview draws a part of the store");
    let projection = seeded.projection();
    let linked: BTreeSet<&str> = projection["edges"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|edge| {
            [
                edge["source"].as_str().unwrap(),
                edge["target"].as_str().unwrap(),
            ]
        })
        .collect();
    let chosen = projection["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| {
            let id = node["id"].as_str().unwrap();
            !top.contains(id) && linked.contains(id)
        })
        .unwrap();
    let id = chosen["id"].as_str().unwrap();
    let mut expected = top.clone();
    expected.extend(page_nodes(&clicked_page(&index, id)));
    assert!(
        expected.len() > top.len() + 1,
        "{id} brings neighbours the overview lacks"
    );

    let double = seeded.double(Duration::ZERO);
    let dom = rendered(&browser, &format!("{}#node={id}", double.url));
    let name = escaped(chosen["name"].as_str().unwrap());
    assert!(dom.contains(&name), "{name} is shown: {dom}");
    assert!(
        dom.contains(&drawn(expected.len(), nodes as u64)),
        "{} nodes drawn: {dom}",
        expected.len()
    );
    reads_no_projection(&double.requests(), "generated");
}

#[test]
fn a_neighbourhood_beyond_one_page_offers_to_load_more() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let fixture = tempfile::tempdir().unwrap();
    generated_fixture(fixture.path(), STAR, true);
    let seeded = Seeded::new(fixture.path());
    let hub = generated_id(3, 1);
    let page = clicked_page(&double_index(&seeded, 0), &hub);
    let remaining = page.remaining();
    assert!(
        page.next().is_some() && (1..1000).contains(&remaining),
        "the hub's neighbourhood is more than one page: {remaining}"
    );
    let double = seeded.double(Duration::ZERO);
    let dom = rendered(&browser, &format!("{}#node={hub}", double.url));
    assert!(
        dom.contains(&format!("load {remaining} more")),
        "the page offers the {remaining} records after the first page: {dom}"
    );
    let requests = double.requests();
    assert!(
        requests.iter().any(|target| target.starts_with("/expand?")
            && target.contains(&format!("seeds={hub}"))
            && target.contains("limit=500")),
        "{requests:?}"
    );
    reads_no_projection(&requests, "star");
}

/// A store of one hub joined to 800 others, and one relation assertion.
const STAR: (usize, usize, usize, usize, usize) = (2, 1, 801, 800, 1);

/// A screenshot of `url` into `file`, after `millis` of virtual time — or of real time when `real`: a stream
/// still arriving holds virtual time still, so an expansion is seen in progress only in real time.
fn shoot(browser: &Path, url: &str, file: &Path, millis: u32, real: bool) {
    let _one = one_browser();
    let profile = tempfile::tempdir().unwrap();
    let output = Command::new(browser)
        .args([
            "--headless",
            "--use-angle=swiftshader",
            "--enable-unsafe-swiftshader",
            "--no-sandbox",
            "--no-first-run",
            "--disable-extensions",
            "--hide-scrollbars",
            "--window-size=1600,1000",
            &format!("--user-data-dir={}", profile.path().display()),
            &if real {
                format!("--timeout={millis}")
            } else {
                format!("--virtual-time-budget={millis}")
            },
            &format!("--screenshot={}", file.display()),
            url,
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success(), "the browser failed on {url}");
    assert!(file.is_file(), "no screenshot at {}", file.display());
}

/// Not a check: writes the operator's screenshots — the overview, an expansion in progress, 3D, the
/// timeline and the sidebar — into the directory `EKR_VIEW_SCREENSHOTS` names. Run by hand with
/// `--ignored`.
#[test]
#[ignore = "writes screenshots into EKR_VIEW_SCREENSHOTS; run by hand"]
fn screenshots_for_the_operator() {
    let browser = browser().expect("a headless Chromium");
    let into = PathBuf::from(std::env::var("EKR_VIEW_SCREENSHOTS").expect("EKR_VIEW_SCREENSHOTS"));
    std::fs::create_dir_all(&into).unwrap();

    let generated = tempfile::tempdir().unwrap();
    generated_fixture(generated.path(), GENERATED, false);
    let seeded = Seeded::new(generated.path());
    let double = seeded.double(Duration::ZERO);
    shoot(
        &browser,
        &double.url,
        &into.join("overview.png"),
        12000,
        false,
    );
    shoot(
        &browser,
        &format!("{}#view=3d", double.url),
        &into.join("3d.png"),
        12000,
        false,
    );
    shoot(
        &browser,
        &format!("{}#mode=timeline", double.url),
        &into.join("timeline.png"),
        8000,
        false,
    );

    let sounding = Seeded::new(&manifest_dir().join("tests/fixtures/view-page/sounding"));
    let projection = sounding.projection();
    let id = busiest(&projection)["id"].as_str().unwrap().to_owned();
    let double = sounding.double(Duration::ZERO);
    shoot(
        &browser,
        &format!("{}#node={id}", double.url),
        &into.join("sidebar.png"),
        8000,
        false,
    );

    let star = tempfile::tempdir().unwrap();
    generated_fixture(star.path(), STAR, true);
    let seeded = Seeded::new(star.path());
    let double = seeded.double(Duration::from_secs(4));
    let hub = generated_id(3, 1);
    shoot(
        &browser,
        &format!("{}#node={hub}", double.url),
        &into.join("expanding.png"),
        3000,
        true,
    );
}
