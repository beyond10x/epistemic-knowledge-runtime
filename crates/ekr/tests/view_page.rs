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
use ekr_core::{NodeId, RevisionNumber, TypeId};
use ekr_kernel::Runtime;
use ekr_views::{
    BucketWidth, ExpandRequest, Index, OverviewRequest, QueryError, SearchRequest, SlicePage,
    SliceRecord, TimelineRequest,
};
use serde_json::Value;

/// The seven addresses the page may read, and no other: the head, the overview, the streamed
/// expansion, one node's detail, the search, the subjects' timeline and evidence by id (the
/// `ekr view` data contract).
const PAGE_ADDRESSES: [&str; 7] = [
    "/head",
    "/overview",
    "/expand",
    "/node/",
    "/search",
    "/timeline",
    "/evidence/",
];

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

/// The embedded page, held to the data-free and no-markup rules.
fn pages() -> [(&'static str, String); 1] {
    [("index.html", page())]
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
        let (status, _) = server.get("/alt");
        assert_eq!(
            status,
            404,
            "{}: GET /alt, the retired earlier page",
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
    /// A change made to every `/overview` answer before it is served, if any.
    edit: Option<fn(&mut Value)>,
}

struct Double {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Seeded {
    fn double(&self, delay: Duration) -> Double {
        self.edited_double(delay, None)
    }

    /// [`Seeded::double`], serving every `/overview` answer as `edit` changes it.
    fn edited_double(&self, delay: Duration, edit: Option<fn(&mut Value)>) -> Double {
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
            edit,
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
    if path == "/head" {
        let body =
            serde_json::json!({"format": "ekr.view-head/1", "head": engine.head}).to_string();
        return reply(&mut stream, 200, "application/json", body.as_bytes());
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
                    let mut bytes = index.overview(&request).unwrap().bytes;
                    if let Some(edit) = engine.edit {
                        let mut document: Value = serde_json::from_slice(&bytes).unwrap();
                        edit(&mut document);
                        bytes = serde_json::to_vec(&document).unwrap();
                    }
                    reply(&mut stream, 200, "application/json", &bytes);
                }
                Err(limit) => refusal(&mut stream, 400, "LimitExceeded", &limit.to_string()),
            }
        }
        "/timeline" => {
            let row_type = query.get("type").map(|text| text.parse::<TypeId>());
            let subject = query.get("subject").map(|text| text.parse::<NodeId>());
            let bucket = match query.get("bucket").map(String::as_str) {
                None => Ok(None),
                Some("day") => Ok(Some(BucketWidth::Day)),
                Some("week") => Ok(Some(BucketWidth::Week)),
                Some(_) => Err(()),
            };
            let (Ok(row_type), Ok(subject), Ok(bucket), Ok(Some(hops)), Ok(Some(limit))) = (
                row_type.transpose(),
                subject.transpose(),
                bucket,
                number("hops"),
                number("limit"),
            ) else {
                return refusal(&mut stream, 400, "invalid-query", "timeline");
            };
            match TimelineRequest::new(row_type, hops, limit, bucket, subject) {
                Ok(request) => {
                    let bytes = index.timeline(&request).unwrap().bytes;
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
                || target == "/head"
                || [
                    "/overview",
                    "/expand?",
                    "/node/",
                    "/search?",
                    "/timeline?",
                    "/evidence/",
                ]
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
        let head = projection["meta"]["revision"].as_u64().unwrap(); // the head projection is of the head
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

/// The schema history's card of version `number` in a dumped DOM: from its id to the next card's,
/// or to the page's script, whose text a dumped DOM also holds.
fn version_card(dom: &str, number: u64) -> &str {
    dom.split(&format!("id=\"ver-{number}\""))
        .nth(1)
        .and_then(|rest| rest.split("id=\"ver-").next())
        .and_then(|card| card.split("<script").next())
        .unwrap_or_default()
}

/// `task:lineage-shows-widened-ends-and-modified-properties`: the schema history lists each
/// version's widened edge ends and modified properties, and calls a version empty only when the
/// overview lists nothing for it. `tests/fixtures/view-lineage/volumes` is the `archive` stand-in
/// with two more versions: v2 widens `CITES`' source to take a `Volume` as well as a `Folio`, and
/// v3 makes `shelfmark` on `Volume` `Many`. Neither adds a type or a property. Served again with
/// v2's `widened` taken out of every `/overview` — exactly what a version whose ontology is its
/// parent's answers — v2 is the one card the page calls empty.
#[test]
fn the_schema_history_lists_widened_ends_and_modified_properties_and_calls_only_an_unchanged_version_empty(
) {
    const EMPTY: &str = "changes nothing against its parent";
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let seeded = Seeded::new(&manifest_dir().join("tests/fixtures/view-lineage/volumes"));
    let double = seeded.double(Duration::ZERO);
    let dom = rendered(&browser, &format!("{}#schema=1", double.url));
    let widened = version_card(&dom, 2);
    assert!(
        widened.contains("widens 1 edge end")
            && widened.contains("Widened edge ends · 1")
            && widened.contains("CITES")
            && widened.contains("source + Volume"),
        "v2 lists the end it widened: {widened}"
    );
    let modified = version_card(&dom, 3);
    assert!(
        modified.contains("modifies 1 property declaration")
            && modified.contains("Modified properties · 1")
            && modified.contains("shelfmark")
            && modified.contains("cardinality"),
        "v3 lists the property it modified: {modified}"
    );
    for number in 0..=3 {
        let card = version_card(&dom, number);
        assert!(!card.is_empty(), "v{number} has a card: {dom}");
        assert!(
            !card.contains(EMPTY) && !card.contains("adds no types or properties"),
            "v{number} changed its ontology and is not called empty: {card}"
        );
    }
    drop(double);

    let unchanged = seeded.edited_double(
        Duration::ZERO,
        Some(|overview: &mut Value| {
            for version in overview["schema"]["versions"].as_array_mut().unwrap() {
                if version["number"] == 2 {
                    version.as_object_mut().unwrap().remove("widened");
                }
            }
        }),
    );
    let dom = rendered(&browser, &format!("{}#schema=1", unchanged.url));
    for number in 0..=3 {
        let card = version_card(&dom, number);
        assert_eq!(
            card.contains(EMPTY),
            number == 2,
            "v{number}: only the version the overview lists nothing for is called empty: {card}"
        );
    }
    assert!(version_card(&dom, 2).contains("changes nothing"));
}

/// `task:timeline-rows-are-subjects`: the timeline's rows are subjects — one per node of the first
/// row type `/timeline` lists that has an event, each under its own name — read from `/timeline`
/// and never from `/projection`; a subject's swimlanes are read from `/timeline` naming it.
#[test]
fn the_timeline_rows_are_the_subjects_the_timeline_address_answers() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    // No fixture store has a type whose facts cluster in valid time, so one is generated whose
    // second type is timed by an epoch-ms property: four subjects with four, three, two and one
    // events.
    let generated = tempfile::tempdir().unwrap();
    timed_fixture(generated.path());
    let mut stores = fixture_stores();
    stores.push(generated.path().to_owned());
    let mut with_rows = 0;
    for fixture in stores {
        let what = fixture.display().to_string();
        let seeded = Seeded::new(&fixture);
        let head = seeded.projection()["meta"]["revision"].as_u64().unwrap(); // the head projection is of the head
        let index = double_index(&seeded, head);
        let request = TimelineRequest::new(None, 2, 500, None, None).unwrap();
        let answer: Value =
            serde_json::from_slice(&index.timeline(&request).unwrap().bytes).unwrap();
        let rows = answer["rows"].as_array().unwrap();
        let overview: Value = serde_json::from_slice(
            &index
                .overview(&OverviewRequest::new(None).unwrap())
                .unwrap()
                .bytes,
        )
        .unwrap();
        let events = overview["roles"]["types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|timing| timing["event"] == true);
        let double = seeded.double(Duration::ZERO);
        let dom = rendered(&browser, &format!("{}#mode=timeline", double.url));
        let requests = double.requests();
        reads_no_projection(&requests, &what);
        if !events {
            // No type is an event type: there is nothing to put on a time axis, and nothing to read.
            assert!(
                dom.contains("nothing to put on a time axis"),
                "{what}: {dom}"
            );
            assert!(
                !requests
                    .iter()
                    .any(|target| target.starts_with("/timeline")),
                "{what}: {requests:?}"
            );
            continue;
        }
        assert!(
            requests
                .iter()
                .any(|target| target.starts_with("/timeline?")),
            "{what}: the rows are read from /timeline: {requests:?}"
        );
        assert_eq!(
            dom.matches("class=\"tlrow\"").count(),
            rows.len(),
            "{what}: one row per subject with an event: {dom}"
        );
        let mut placed = Vec::new();
        for row in rows {
            let name = escaped(row["name"].as_str().unwrap());
            let at = dom.find(&format!("<span class=\"nm\">{name}</span>"));
            assert!(at.is_some(), "{what}: the row of {name}: {dom}");
            placed.extend(at);
        }
        assert!(
            placed.windows(2).all(|pair| pair[0] < pair[1]),
            "{what}: the rows stand in the answer's order, the most active first"
        );
        let Some(first) = rows.first() else { continue };
        with_rows += 1;
        let id = first["id"].as_str().unwrap();
        let dom = rendered(
            &browser,
            &format!("{}#mode=timeline&subject={id}", double.url),
        );
        assert!(
            dom.contains("class=\"tllane\""),
            "{what}: {id} has swimlanes: {dom}"
        );
        let requests = double.requests();
        reads_no_projection(&requests, &what);
        assert!(
            requests
                .iter()
                .any(|target| target.starts_with("/timeline?") && target.contains(id)),
            "{what}: the swimlanes are read from /timeline naming {id}: {requests:?}"
        );
    }
    assert!(
        with_rows > 0,
        "no fixture store has a subject with an event"
    );
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
        let head = projection["meta"]["revision"].as_u64().unwrap(); // the head projection is of the head
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

/// Writes a store fixture into `directory` whose timeline has rows: `type-1` subjects
/// `entity-1` to `entity-4`, and `type-2` nodes `entity-101` to `entity-110`, each carrying one
/// epoch-ms value of its Integer property a day apart, so `type-2` is an event type. Event `n`
/// points at subject 1 for `n` up to 4, 2 up to 7, 3 up to 9 and 4 for 10.
fn timed_fixture(directory: &Path) {
    use std::fmt::Write as _;
    let (version, root) = (generated_id(0, 1), generated_id(0, 2));
    let (subject_type, event_type, link) =
        (generated_id(1, 1), generated_id(1, 2), generated_id(2, 1));
    let at = generated_id(7, 1);
    let mut yaml = format!(
        "format: ekr-seed/2\nontology:\n  version:\n    id: {version}\n    number: 0\n    parent: null\n    created_at: 0\n  node_types:\n  - id: {subject_type}\n    name: type-1\n    parents: []\n    properties: {{}}\n    abstract_type: false\n    lifecycle: null\n    operations: {{}}\n  - id: {event_type}\n    name: type-2\n    parents: []\n    properties:\n      {at}:\n        id: {at}\n        name: value-1\n        value_type:\n          value_kind: Integer\n        cardinality: One\n        required: false\n        constraints: []\n    abstract_type: false\n    lifecycle: null\n    operations: {{}}\n  edge_types:\n  - id: {link}\n    name: link-1\n    source_types:\n    - {subject_type}\n    - {event_type}\n    target_types:\n    - {subject_type}\n    - {event_type}\n    cardinality: Many\n    properties: {{}}\n    inverse: null\n    symmetric: false\n    transitive: false\ngraph:\n  format: ekr.graph-document/2\n  graph:\n    root:\n      id: {root}\n      space: Canonical\n      schema_version_id: {version}\n      parent: null\n      created_at: 0\n    revision: 0\n    nodes:\n"
    );
    for n in 1..=4 {
        let id = generated_id(3, n);
        let _ = write!(
            yaml,
            "      {id}:\n        id: {id}\n        root_id: {root}\n        type_id: {subject_type}\n        canonical_name: entity-{n}\n        aliases: []\n        type_state: null\n        properties: {{}}\n"
        );
    }
    for n in 1..=10_usize {
        let id = generated_id(3, 100 + n);
        let time = 1_700_000_000_000_i64 + i64::try_from(n).unwrap() * 86_400_000;
        let _ = write!(
            yaml,
            "      {id}:\n        id: {id}\n        root_id: {root}\n        type_id: {event_type}\n        canonical_name: entity-{}\n        aliases: []\n        type_state: null\n        properties:\n          {at}:\n          - value_kind: Integer\n            value: {time}\n",
            100 + n
        );
    }
    yaml.push_str("    edges:\n");
    for n in 1..=10_usize {
        let id = generated_id(4, n);
        let subject = match n {
            1..=4 => 1,
            5..=7 => 2,
            8 | 9 => 3,
            _ => 4,
        };
        let _ = write!(
            yaml,
            "      {id}:\n        id: {id}\n        root_id: {root}\n        type_id: {link}\n        source: {}\n        target: {}\n        properties: {{}}\n",
            generated_id(3, 100 + n),
            generated_id(3, subject)
        );
    }
    yaml.push_str("    assertions: {}\n    evidence: {}\nevidence_payloads: {}\n");
    std::fs::write(directory.join("seed.yaml"), yaml).unwrap();
    std::fs::copy(
        manifest_dir().join("tests/fixtures/view-page/sounding/host.json"),
        directory.join("host.json"),
    )
    .unwrap();
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

/// The first page of `seed`'s neighbourhood of `depth` hops, as the page asks for it.
fn expanded_page(index: &Index, seed: &str, depth: i64) -> SlicePage {
    let request = ExpandRequest::new(vec![seed.parse().unwrap()], depth, 500, None, None).unwrap();
    index.page(&request).unwrap()
}

/// `task:hops-slider-streams-the-neighbourhood`: a hops depth of 2 on a focus whose 2-hop
/// neighbourhood the page has not loaded streams that neighbourhood with one `/expand` of depth 2,
/// and the focus then holds exactly it. The address carries the slider's value, and the slider and
/// the address set it through the same function, so an address stands in for the slider here: a
/// headless browser under `--dump-dom` cannot move one.
#[test]
fn a_hops_depth_on_a_focus_streams_the_neighbourhood_it_names_once() {
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
    let projection = seeded.projection();
    let chosen = projection["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap())
        .find(|id| {
            let near = page_nodes(&expanded_page(&index, id, 1));
            let far = page_nodes(&expanded_page(&index, id, 2));
            !top.contains(*id)
                && near.len() > 1
                && far.iter().any(|n| !top.contains(n) && !near.contains(n))
        })
        .unwrap()
        .to_owned();
    let two = expanded_page(&index, &chosen, 2);
    assert!(
        two.next().is_none(),
        "{chosen}: its 2-hop neighbourhood is one page"
    );
    let neighbourhood = page_nodes(&two);
    let mut expected = top.clone();
    expected.extend(page_nodes(&expanded_page(&index, &chosen, 1)));
    expected.extend(neighbourhood.iter().cloned());

    let double = seeded.double(Duration::ZERO);
    let dom = rendered(
        &browser,
        &format!("{}#node={chosen}&focus={chosen}&hops=2", double.url),
    );
    let requests = double.requests();
    let focused: Vec<&String> = requests
        .iter()
        .filter(|target| {
            target.starts_with("/expand?") && target.contains(&format!("seeds={chosen}&"))
        })
        .collect();
    let deep = focused
        .iter()
        .filter(|target| target.contains("&depth=2&"))
        .count();
    assert_eq!(deep, 1, "one expansion of depth 2: {focused:?}");
    assert!(
        dom.contains(&format!(
            "<span class=\"n\">{} nodes</span>",
            grouped(neighbourhood.len() as u64)
        )),
        "the focus holds the {} nodes of the 2-hop neighbourhood: {dom}",
        neighbourhood.len()
    );
    assert!(
        dom.contains(&drawn(expected.len(), nodes as u64)),
        "{} nodes drawn: {dom}",
        expected.len()
    );
    reads_no_projection(&requests, "hops");
}

/// What the crumb bar says with no neighbourhood focus: how to get one, and with it the hops slider.
const HOPS_HINT: &str =
    "Double-click a node, or choose Neighbourhood in its panel, for a hops slider";

/// `task:hops-slider-streams-the-neighbourhood`: with nothing focused, the page says how to reach
/// the depth control.
#[test]
fn with_nothing_focused_the_crumb_bar_says_how_to_reach_the_hops_slider() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let seeded = Seeded::new(&manifest_dir().join("tests/fixtures/view-page/sounding"));
    let double = seeded.double(Duration::ZERO);
    let dom = rendered(&browser, &double.url);
    let crumbs = dom
        .split("id=\"crumbs\"")
        .nth(1)
        .and_then(|rest| rest.split("</div>").next())
        .expect("the page has a crumb bar");
    assert!(crumbs.contains(HOPS_HINT), "the crumb bar: {crumbs}");
}

/// `story:viewer-3d-draws-in-batches`: the 3D view starts on a generated store and draws the
/// overview without an error. Its draw calls and frame times are measured in a real-time browser
/// with a GPU, not here.
#[test]
fn the_3d_view_draws_a_generated_store_without_an_error() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let (_, _, nodes, _, _) = GENERATED;
    let fixture = tempfile::tempdir().unwrap();
    generated_fixture(fixture.path(), GENERATED, false);
    let seeded = Seeded::new(fixture.path());
    let top = top_ids(&double_index(&seeded, 0));
    let double = seeded.double(Duration::ZERO);
    let dom = rendered(&browser, &format!("{}#view=3d", double.url));
    assert!(dom.contains("<div id=\"error\"></div>"), "no error: {dom}");
    assert!(
        dom.contains(&drawn(top.len(), nodes as u64)),
        "the overview's {} top nodes: {dom}",
        top.len()
    );
    let stage = dom.split("id=\"graph3d\"").nth(1).unwrap_or_default();
    assert!(
        stage
            .split('>')
            .next()
            .unwrap_or_default()
            .contains("display: block")
            && stage.contains("<canvas"),
        "the 3D view is shown and draws on a canvas: {stage}"
    );
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

/// The text of the element `id` in a dumped DOM, up to its first closing `</div>`.
fn element<'a>(dom: &'a str, id: &str) -> &'a str {
    dom.split(&format!("id=\"{id}\""))
        .nth(1)
        .and_then(|rest| rest.split("</div>").next())
        .unwrap_or_default()
}

/// `task:hops-slider-streams-the-neighbourhood`: the slider runs from 1 to 3 hops, and a depth
/// names a neighbourhood to stream. `/expand` answers a depth of at most 2 (`ExpandRequest::new`
/// refuses 3 with `LimitExceeded`, as `ekr view` does: "depth is 3, and it must be at least 0 and
/// at most 2"). A focus at 3 hops must not end in an expansion the server refuses.
#[test]
fn a_focus_at_three_hops_streams_nothing_the_server_refuses() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let seeded = Seeded::new(&manifest_dir().join("tests/fixtures/view-page/sounding"));
    let projection = seeded.projection();
    let id = busiest(&projection)["id"].as_str().unwrap().to_owned();
    assert!(
        ExpandRequest::new(vec![id.parse().unwrap()], 3, 500, None, None).is_err(),
        "the read contract refuses a depth of 3"
    );
    let double = seeded.double(Duration::ZERO);
    let dom = rendered(
        &browser,
        &format!("{}#node={id}&focus={id}&hops=3", double.url),
    );
    let refused: Vec<String> = double
        .requests()
        .into_iter()
        .filter(|target| target.starts_with("/expand?") && target.contains("&depth=3&"))
        .collect();
    let stream = element(&dom, "stream");
    assert!(
        !stream.contains("the expansion failed"),
        "the stream line after a focus at 3 hops: {stream}; the refused reads: {refused:?}"
    );
    assert!(refused.is_empty(), "reads of depth 3: {refused:?}");
}

// ---- a page driven over the DevTools protocol ------------------------------------------------------
//
// `--dump-dom` loads one address and cannot press, drag or move a slider. `Driven` starts the same
// headless browser with a DevTools port, speaks the protocol over one WebSocket (text frames only)
// and evaluates expressions in the page, so a case can act as a reader does.

struct Driven {
    child: Child,
    socket: TcpStream,
    reader: BufReader<TcpStream>,
    next: u64,
    /// Every exception the page threw, every `console.error` it wrote and every error the browser logged
    /// for it (a failed load, a refused script), as the protocol reported them.
    errors: Vec<Value>,
    _profile: tempfile::TempDir,
    _one: std::sync::MutexGuard<'static, ()>,
}

impl Driven {
    fn launch(browser: &Path, url: &str) -> Self {
        Self::launch_sized(browser, url, (1600, 1000))
    }

    /// [`Driven::launch`] in a window of `size` (width, height) pixels, from the first load.
    fn launch_sized(browser: &Path, url: &str, size: (u32, u32)) -> Self {
        let one = one_browser();
        let profile = tempfile::tempdir().unwrap();
        let mut child = Command::new(browser)
            .args([
                "--headless",
                "--use-angle=swiftshader",
                "--enable-unsafe-swiftshader",
                "--no-sandbox",
                "--no-first-run",
                "--disable-extensions",
                &format!("--window-size={},{}", size.0, size.1),
                "--remote-debugging-port=0",
                &format!("--user-data-dir={}", profile.path().display()),
                "about:blank",
            ])
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
        let mut list = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(
            list,
            "GET /json/list HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        // the DevTools server keeps the connection open: the body is read by its length
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
        let page = targets
            .as_array()
            .unwrap()
            .iter()
            .find(|target| target["type"] == "page")
            .expect("a page target");
        let address = page["webSocketDebuggerUrl"].as_str().unwrap();
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
        driven.call("Runtime.enable", serde_json::json!({}));
        driven.call("Log.enable", serde_json::json!({}));
        driven.call("Page.navigate", serde_json::json!({ "url": url }));
        driven
    }

    /// One masked text frame, its mask all zeros so the payload goes as it stands.
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

    /// The next whole message: frames joined up to the final one, pings answered.
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
        self.send(&serde_json::json!({"id": id, "method": method, "params": params}).to_string());
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

    /// The value of `expression` in the page, awaited.
    fn eval(&mut self, expression: &str) -> Value {
        let reply = self.call(
            "Runtime.evaluate",
            serde_json::json!({"expression": expression, "awaitPromise": true, "returnByValue": true}),
        );
        assert!(
            reply["result"].get("exceptionDetails").is_none(),
            "{expression}: {reply}"
        );
        reply["result"]["result"]["value"].clone()
    }

    /// Waits up to `seconds` for `expression` to be `true`.
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

    fn mouse(&mut self, kind: &str, x: f64, y: f64, pressed: bool) {
        let button = if kind == "mouseMoved" && !pressed {
            "none"
        } else {
            "left"
        };
        self.call(
            "Input.dispatchMouseEvent",
            serde_json::json!({"type": kind, "x": x, "y": y, "button": button,
                "buttons": u8::from(pressed), "clickCount": u8::from(kind != "mouseMoved")}),
        );
    }

    /// One press of a printable key, as a reader types it outside any field.
    fn key(&mut self, key: &str) {
        for kind in ["keyDown", "keyUp"] {
            self.call(
                "Input.dispatchKeyEvent",
                serde_json::json!({"type": kind, "key": key, "text": key, "unmodifiedText": key}),
            );
        }
    }

    /// One press of Enter or Space with `modifiers` (8 is Shift), sent to whatever holds the focus.
    fn press(&mut self, key: &str, modifiers: u8) {
        let (code, keycode, text) = match key {
            "Enter" => ("Enter", 13, "\r"),
            " " => ("Space", 32, " "),
            other => panic!("press: {other:?} is neither Enter nor Space"),
        };
        self.call(
            "Input.dispatchKeyEvent",
            serde_json::json!({"type": "keyDown", "key": key, "code": code, "text": text,
                "unmodifiedText": text, "windowsVirtualKeyCode": keycode, "modifiers": modifiers}),
        );
        self.call(
            "Input.dispatchKeyEvent",
            serde_json::json!({"type": "keyUp", "key": key, "code": code,
                "windowsVirtualKeyCode": keycode, "modifiers": modifiers}),
        );
    }
}

impl Drop for Driven {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

/// `story:viewer-3d-draws-in-batches`, acceptance: at rest the 3D view draws at most 20 calls per
/// frame, and none while idle. `the_3d_view_draws_a_generated_store_without_an_error` asserts the
/// error line, the 2D count and a canvas, which a page that adds no batch to the scene also passes.
/// This case reads what one frame draws: every shown node in the node batches, every shown edge in
/// the line batch, in at least two calls and at most 20; and no frame while the view is idle.
#[test]
fn the_3d_view_draws_every_shown_node_and_edge_in_a_few_calls_and_none_at_rest() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let seeded = Seeded::new(&manifest_dir().join("tests/fixtures/view-page/sounding"));
    let double = seeded.double(Duration::ZERO);
    let mut driven = Driven::launch(&browser, &format!("{}#view=3d", double.url));
    assert!(
        driven.wait_for(&format!("{SETTLED} && !!window.__viewer.fg"), 90),
        "the 3D view settled"
    );
    std::thread::sleep(Duration::from_secs(1));
    let frame = driven.eval(
        "(() => { const fg = __viewer.fg, g = __viewer.graph, gl = fg.renderer().getContext();
          const iso = document.getElementById('showIsolated').checked;
          const superseded = document.getElementById('showSuperseded').checked;
          let nodes = 0; g.forEachNode((id, a) => { if (iso || a.deg !== 0) nodes++; });
          let edges = 0; g.forEachEdge((id, a) => {
            if (superseded || a.lifecycle == null || a.lifecycle === 'Active' || a.lifecycle === 'unasserted') edges++; });
          window.__calls = 0;
          for (const f of ['drawArrays', 'drawElements', 'drawArraysInstanced', 'drawElementsInstanced']) {
            const own = gl[f].bind(gl); gl[f] = (...a) => { window.__calls++; return own(...a); }; }
          fg.renderer().render(fg.scene(), fg.camera());
          const drawn = fg.scene().children.filter(o => o.visible);
          const glyphs = drawn.filter(o => o.isInstancedMesh && o.geometry.type === 'SphereGeometry' && o.geometry.parameters.radius >= 1);
          const lines = drawn.filter(o => o.isLineSegments);
          return {calls: window.__calls, nodes, edges,
            drawnNodes: glyphs.reduce((s, o) => s + o.count, 0),
            drawnEdges: lines.reduce((s, o) => s + o.geometry.drawRange.count / 2, 0)}; })()",
    );
    assert_eq!(
        frame["drawnNodes"], frame["nodes"],
        "every shown node drawn: {frame}"
    );
    assert_eq!(
        frame["drawnEdges"], frame["edges"],
        "every shown edge drawn: {frame}"
    );
    let calls = frame["calls"].as_u64().unwrap();
    assert!(
        (2..=20).contains(&calls),
        "draw calls in one frame: {frame}"
    );
    let idle = driven.eval(
        "new Promise(done => { const at = window.__calls; setTimeout(() => done(window.__calls - at), 1000); })",
    );
    assert_eq!(idle, 0, "draw calls in one idle second");
}

/// The page has drawn its first view and nothing is streaming or laying out.
const SETTLED: &str = "!!window.__viewer && !window.__viewer.layoutRunning \
     && !document.querySelector('[data-act=stream-stop]')";

/// The `/expand` reads of `seed` at `depth` the stand-in has answered.
fn expansions(double: &Double, seed: &str, depth: u8) -> Vec<String> {
    double
        .requests()
        .into_iter()
        .filter(|target| {
            target.starts_with(&format!("/expand?seeds={seed}&"))
                && target.contains(&format!("&depth={depth}&"))
        })
        .collect()
}

/// Moves the hops slider through `values`, each as a reader lets go of it there.
fn slide_hops(driven: &mut Driven, values: &[u8]) {
    let values: Vec<String> = values.iter().map(u8::to_string).collect();
    driven.eval(&format!(
        "(() => {{ const r = document.getElementById('hopsR'); for (const v of [{}]) {{ r.value = v; \
         r.dispatchEvent(new Event('input', {{bubbles: true}})); \
         r.dispatchEvent(new Event('change', {{bubbles: true}})); }} }})()",
        values.join(",")
    ));
}

/// `task:hops-slider-streams-the-neighbourhood`, acceptance: "moving the slider to 2 issues one
/// `/expand` with `depth=2`". A reader who drags past 2 to 3 and back lets go at 3 and then at 2;
/// the page notes depth 3 as streamed before its read is answered (and refused, see
/// `a_focus_at_three_hops_streams_nothing_the_server_refuses`), so 2 is never streamed.
#[test]
fn moving_the_slider_to_three_and_back_to_two_streams_two_hops() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let seeded = Seeded::new(&manifest_dir().join("tests/fixtures/view-page/sounding"));
    let projection = seeded.projection();
    let id = busiest(&projection)["id"].as_str().unwrap().to_owned();
    let double = seeded.double(Duration::ZERO);
    let mut driven = Driven::launch(
        &browser,
        &format!("{}#node={id}&focus={id}&hops=1", double.url),
    );
    assert!(
        driven.wait_for(
            &format!("{SETTLED} && !!document.getElementById('hopsR')"),
            60
        ),
        "the page settled on a focus"
    );
    slide_hops(&mut driven, &[3, 2]);
    std::thread::sleep(Duration::from_secs(2));
    assert!(driven.wait_for(SETTLED, 60), "the page settled");
    let two = expansions(&double, &id, 2);
    assert_eq!(
        two.len(),
        1,
        "one read of the 2-hop neighbourhood: {two:?}; every read: {:?}",
        double.requests()
    );
}

/// `task:hops-slider-streams-the-neighbourhood`: the page's own note says "a depth already
/// streamed, or one inside it, is not streamed again", and the CHANGELOG says `/expand` is
/// streamed "once per focus and depth". "Expand 2 hops" streams the 2-hop neighbourhood, and the
/// slider set to 2 on the same focus streams it a second time.
#[test]
fn a_neighbourhood_expanded_two_hops_is_not_streamed_again_by_the_slider() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let seeded = Seeded::new(&manifest_dir().join("tests/fixtures/view-page/sounding"));
    let projection = seeded.projection();
    let id = busiest(&projection)["id"].as_str().unwrap().to_owned();
    let double = seeded.double(Duration::ZERO);
    let mut driven = Driven::launch(&browser, &format!("{}#node={id}", double.url));
    assert!(
        driven.wait_for(
            &format!("{SETTLED} && !!document.querySelector('[data-act=expand]')"),
            60
        ),
        "the page settled on the node"
    );
    driven.eval("document.querySelector('[data-act=expand]').click()");
    assert!(
        driven.wait_for(SETTLED, 60) && expansions(&double, &id, 2).len() == 1,
        "Expand 2 hops streamed: {:?}",
        double.requests()
    );
    driven.eval("document.querySelector('[data-act=scope]').click()");
    assert!(
        driven.wait_for("!!document.getElementById('hopsR')", 30),
        "a slider"
    );
    slide_hops(&mut driven, &[2]);
    std::thread::sleep(Duration::from_secs(2));
    assert!(driven.wait_for(SETTLED, 60), "the page settled");
    let two = expansions(&double, &id, 2);
    assert_eq!(two.len(), 1, "the 2-hop neighbourhood read once: {two:?}");
}

/// `story:viewer-3d-draws-in-batches`: the CHANGELOG says node drag works "as before". Before, the
/// drag was 3d-force-graph's, which reheats the layout while a node is held, so its neighbours
/// follow it. Measured with the base page (0ab728e9) and this one, served over the `sounding` store
/// and dragged the same way in headless Brave on SwiftShader: the dragged node moved 199 and 189
/// units, its neighbour 256 and 0.
#[test]
fn a_node_dragged_in_3d_pulls_its_neighbours_along_as_before() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let seeded = Seeded::new(&manifest_dir().join("tests/fixtures/view-page/sounding"));
    let double = seeded.double(Duration::ZERO);
    let mut driven = Driven::launch(&browser, &format!("{}#view=3d", double.url));
    assert!(
        driven.wait_for(&format!("{SETTLED} && !!window.__viewer.fg"), 90),
        "the 3D view settled"
    );
    std::thread::sleep(Duration::from_millis(500));
    let chosen = driven.eval(
        "(() => { const fg = __viewer.fg, g = __viewer.graph, box = document.getElementById('graph3d').getBoundingClientRect();
          const drawn = new Map(fg.graphData().nodes.filter(n => n.x != null).map(n => [n.id, n]));
          for (const n of [...drawn.values()].sort((a, b) => g.degree(b.id) - g.degree(a.id))) {
            const near = g.neighbors(n.id).find(o => o !== n.id && drawn.has(o));
            const p = fg.graph2ScreenCoords(n.x, n.y, n.z);
            if (near && p.x > 200 && p.y > 200 && p.x < box.width - 200 && p.y < box.height - 200)
              return {id: n.id, near, x: box.left + p.x, y: box.top + p.y}; }
          return null; })()",
    );
    assert!(
        chosen.is_object(),
        "a node on screen with a drawn neighbour"
    );
    let at = |driven: &mut Driven, id: &Value| -> Vec<f64> {
        driven
            .eval(&format!(
                "(() => {{ const n = __viewer.fg.graphData().nodes.find(n => n.id === {id}); return [n.x, n.y, n.z]; }})()"
            ))
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    };
    let apart = |a: &[f64], b: &[f64]| {
        a.iter()
            .zip(b)
            .map(|(p, q)| (p - q).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    let (node, near) = (chosen["id"].clone(), chosen["near"].clone());
    let (x, y) = (chosen["x"].as_f64().unwrap(), chosen["y"].as_f64().unwrap());
    let (node_before, near_before) = (at(&mut driven, &node), at(&mut driven, &near));
    driven.mouse("mouseMoved", x, y, false);
    driven.mouse("mousePressed", x, y, true);
    for step in 1..=20 {
        driven.mouse(
            "mouseMoved",
            x + 8.0 * f64::from(step),
            y + 4.0 * f64::from(step),
            true,
        );
        std::thread::sleep(Duration::from_millis(40));
    }
    std::thread::sleep(Duration::from_millis(600));
    let (node_held, near_held) = (at(&mut driven, &node), at(&mut driven, &near));
    driven.mouse("mouseReleased", x + 160.0, y + 80.0, false);
    let dragged = apart(&node_before, &node_held);
    let pulled = apart(&near_before, &near_held);
    assert!(
        dragged > 10.0,
        "the node was dragged: it moved {dragged:.1}"
    );
    assert!(
        pulled > 1.0,
        "the dragged node moved {dragged:.1} and its neighbour {pulled:.1}"
    );
}

// ---- compact mode: both sidebars collapse to a strip at their edge --------------------------------

/// `task:viewer-compact-mode`: the page has a compact toggle, a strip per sidebar that restores it
/// and a control that collapses each sidebar on its own; its help names the key that toggles both,
/// and the compact state is written into the address and read back from it.
#[test]
fn the_page_has_a_compact_toggle_a_strip_and_a_collapse_control_per_sidebar() {
    let page = page();
    for id in [
        "compact",
        "leftStrip",
        "rightStrip",
        "foldLeft",
        "foldRight",
    ] {
        assert!(page.contains(&format!("id=\"{id}\"")), "an element {id}");
    }
    assert!(page.contains("C compact"), "the help names the key");
    assert!(
        page.contains("p.set(\"compact\""),
        "the address carries the compact state"
    );
    assert!(
        page.contains(".get(\"compact\")"),
        "the compact state is read from the address"
    );
}

/// The widths the reader sees, the address, the panel's heading and the camera: the 3D camera's
/// position, or the 2D camera's centre and zoom.
const MEASURE: &str = "(() => { const shown = e => e && getComputedStyle(e).display !== 'none' ? e.getBoundingClientRect().width : 0;
  const c = __viewer.fg ? __viewer.fg.renderer().domElement : document.querySelector('#graph canvas');
  const cam = __viewer.fg ? (p => [p.x, p.y, p.z])(__viewer.fg.camera().position) : (s => [s.x, s.y, s.ratio])(__viewer.camera.getState());
  return {canvas: c.getBoundingClientRect().width, stage: document.getElementById('stage').getBoundingClientRect().width,
    left: shown(document.querySelector('aside.left')), right: shown(document.getElementById('panel')),
    leftStrip: shown(document.getElementById('leftStrip')), rightStrip: shown(document.getElementById('rightStrip')),
    hash: location.hash, panel: document.querySelector('#panel h1')?.textContent ?? null, camera: cam}; })()";

/// Waits for `MEASURE` to hold `condition` (an expression over `m`), and returns it.
fn measured_when(driven: &mut Driven, condition: &str, what: &str) -> Value {
    let expression = format!("(m => {condition})({MEASURE})");
    assert!(
        driven.wait_for(&expression, 30),
        "{what}: {}",
        driven.eval(MEASURE)
    );
    driven.eval(MEASURE)
}

fn width(m: &Value, key: &str) -> f64 {
    m[key].as_f64().unwrap()
}

fn same_camera(a: &Value, b: &Value) -> bool {
    let (a, b) = (
        a["camera"].as_array().unwrap(),
        b["camera"].as_array().unwrap(),
    );
    a.iter()
        .zip(b)
        .all(|(p, q)| (p.as_f64().unwrap() - q.as_f64().unwrap()).abs() < 1e-6)
}

/// The 3D view's draw calls, counted from here on in `window.__calls`.
const COUNT_DRAWS: &str =
    "(() => { const gl = __viewer.fg.renderer().getContext(); window.__calls = 0;
  for (const f of ['drawArrays', 'drawElements', 'drawArraysInstanced', 'drawElementsInstanced']) {
    const own = gl[f].bind(gl); gl[f] = (...a) => { window.__calls++; return own(...a); }; } })()";

fn draws_within(driven: &mut Driven, millis: u32) -> u64 {
    driven
        .eval(&format!(
            "new Promise(done => {{ const at = window.__calls; setTimeout(() => done(window.__calls - at), {millis}); }})"
        ))
        .as_u64()
        .unwrap()
}

/// `task:viewer-compact-mode`, acceptance, in the view `dim` names, with a node `chosen` or none:
/// the page opens with both sidebars shown; the key `c` collapses both to their strips, and the
/// canvas grows by what they gave up, keeping the camera and the chosen node; each strip restores
/// its own sidebar; one sidebar collapses on its own; the address carries the state through a
/// reload and back through history; and the page reports no error. In 3D with nothing chosen the
/// loop draws for each resize and rests again.
fn compact_mode_in(dim: &str, chosen: bool) {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let seeded = Seeded::new(&manifest_dir().join("tests/fixtures/view-page/sounding"));
    let projection = seeded.projection();
    let id = busiest(&projection)["id"].as_str().unwrap().to_owned();
    let double = seeded.double(Duration::ZERO);
    let address = if chosen {
        format!("{}#node={id}&view={dim}", double.url)
    } else {
        format!("{}#view={dim}", double.url)
    };
    let mut driven = Driven::launch(&browser, &address);
    // with a node chosen, particles run along its edges and the 3D loop never rests
    let rests = dim == "3d" && !chosen;
    let drawn = if dim == "3d" {
        "!!window.__viewer.fg"
    } else {
        "!!window.__viewer.renderer && !window.__viewer.camera.isAnimated()"
    };
    assert!(
        driven.wait_for(
            &format!("{SETTLED} && {drawn} && !!document.querySelector('#panel h1')"),
            90
        ),
        "the {dim} view settled on the node"
    );
    // the camera's flight to the chosen node has ended
    std::thread::sleep(Duration::from_millis(1500));
    let open = driven.eval(MEASURE);
    assert!(
        width(&open, "left") > 0.0
            && width(&open, "right") > 0.0
            && width(&open, "leftStrip") == 0.0
            && width(&open, "rightStrip") == 0.0,
        "not compact by default: {open}"
    );
    assert!(
        !open["hash"].as_str().unwrap().contains("compact"),
        "{open}"
    );
    assert!(
        (width(&open, "canvas") - width(&open, "stage")).abs() < 1.0,
        "{open}"
    );
    if rests {
        driven.eval(COUNT_DRAWS);
        assert_eq!(draws_within(&mut driven, 1000), 0, "the 3D loop rests");
    }

    // the key collapses both
    driven.key("c");
    if rests {
        let woke = draws_within(&mut driven, 400);
        assert!(woke > 0, "the 3D loop drew the resize: {woke} calls");
        eprintln!("3d: {woke} draw calls in the 400 ms after the key");
        std::thread::sleep(Duration::from_millis(1000));
        assert_eq!(
            draws_within(&mut driven, 1000),
            0,
            "the 3D loop rests after the resize"
        );
    }
    let compact = measured_when(
        &mut driven,
        &format!(
            "m.left === 0 && m.right === 0 && m.leftStrip > 0 && m.rightStrip > 0 \
             && Math.abs(m.canvas - m.stage) < 1 && m.canvas > {}",
            width(&open, "canvas")
        ),
        "both sidebars collapsed to their strips and the canvas took their width",
    );
    let freed = width(&open, "left") + width(&open, "right")
        - width(&compact, "leftStrip")
        - width(&compact, "rightStrip");
    assert!(
        (width(&compact, "canvas") - width(&open, "canvas") - freed).abs() < 1.0,
        "the canvas grew by {freed}: {open} → {compact}"
    );
    eprintln!(
        "{dim}: canvas {} → {} (sidebars {} + {}, strips {} + {})",
        width(&open, "canvas"),
        width(&compact, "canvas"),
        width(&open, "left"),
        width(&open, "right"),
        width(&compact, "leftStrip"),
        width(&compact, "rightStrip")
    );
    let hash = compact["hash"].as_str().unwrap();
    assert!(
        hash.contains("compact=1") && (!chosen || hash.contains(&format!("node={id}"))),
        "{compact}"
    );
    assert_eq!(
        compact["panel"], open["panel"],
        "the chosen node stays open"
    );
    assert!(
        same_camera(&open, &compact),
        "the camera: {open} → {compact}"
    );

    // each strip restores its own sidebar
    let at = if rests {
        driven.eval("window.__calls").as_u64().unwrap()
    } else {
        0
    };
    driven.eval("document.getElementById('leftStrip').click()");
    let left = measured_when(
        &mut driven,
        "m.left > 0 && m.leftStrip === 0 && m.right === 0 && m.rightStrip > 0 && Math.abs(m.canvas - m.stage) < 1",
        "the left strip restored the left sidebar alone",
    );
    assert!(
        left["hash"].as_str().unwrap().contains("compact=right"),
        "{left}"
    );
    driven.eval("document.getElementById('rightStrip').click()");
    let restored = measured_when(
        &mut driven,
        "m.left > 0 && m.right > 0 && m.leftStrip === 0 && m.rightStrip === 0 && Math.abs(m.canvas - m.stage) < 1",
        "the right strip restored the right sidebar",
    );
    assert!(
        (width(&restored, "canvas") - width(&open, "canvas")).abs() < 1.0,
        "the canvas returned to its width: {open} → {restored}"
    );
    assert!(
        !restored["hash"].as_str().unwrap().contains("compact"),
        "{restored}"
    );
    assert!(
        same_camera(&open, &restored),
        "the camera: {open} → {restored}"
    );
    if rests {
        let woke = driven.eval("window.__calls").as_u64().unwrap() - at;
        assert!(woke > 0, "the 3D loop drew the restores: {woke} calls");
        eprintln!("3d: {woke} draw calls for the two restores");
        std::thread::sleep(Duration::from_millis(1000));
        assert_eq!(
            draws_within(&mut driven, 1000),
            0,
            "the 3D loop rests after the restores"
        );
    }

    // one sidebar collapses on its own, and a reload keeps it collapsed
    driven.eval("document.getElementById('foldRight').click()");
    let right = measured_when(
        &mut driven,
        "m.left > 0 && m.right === 0 && m.rightStrip > 0 && Math.abs(m.canvas - m.stage) < 1",
        "the right sidebar collapsed on its own",
    );
    assert!(
        right["hash"].as_str().unwrap().contains("compact=right"),
        "{right}"
    );
    driven.eval("window.__before = true");
    driven.call("Page.reload", serde_json::json!({}));
    assert!(
        driven.wait_for(&format!("!window.__before && {SETTLED} && {drawn}"), 90),
        "the page reloaded"
    );
    let reloaded = measured_when(
        &mut driven,
        "m.left > 0 && m.right === 0 && m.rightStrip > 0 && Math.abs(m.canvas - m.stage) < 1",
        "the reload restored the collapsed right sidebar from the address",
    );
    assert!(
        (width(&reloaded, "canvas") - width(&right, "canvas")).abs() < 1.0,
        "{right} → {reloaded}"
    );

    // the toggle collapses both, and going back restores the address before it
    driven.eval("document.getElementById('compact').click()");
    measured_when(
        &mut driven,
        "m.left === 0 && m.right === 0 && m.hash.includes('compact=1')",
        "the toggle collapsed both",
    );
    driven.eval("history.back()");
    measured_when(
        &mut driven,
        "m.left > 0 && m.right === 0 && m.hash.includes('compact=right') && Math.abs(m.canvas - m.stage) < 1",
        "going back restored the address before the toggle",
    );
    assert!(
        driven.errors.is_empty(),
        "the page reported errors: {:?}",
        driven.errors
    );
}

#[test]
fn compact_mode_collapses_both_sidebars_and_restores_them_in_2d() {
    compact_mode_in("2d", true);
}

#[test]
fn compact_mode_collapses_both_sidebars_and_restores_them_in_3d() {
    compact_mode_in("3d", true);
}

#[test]
fn compact_mode_wakes_the_resting_3d_loop_for_the_resize_and_it_rests_again() {
    compact_mode_in("3d", false);
}

// ---- compact mode's follow-ups: a hidden selection, the keyboard, a narrow window ------------------

/// The right strip's mark: the width of its dot as shown, its title and its accessible name.
const MARK: &str = "(s => ({dot: (d => d && getComputedStyle(d).display !== 'none' ? d.getBoundingClientRect().width : 0)(s.querySelector('.mark')),
  title: s.title, label: s.getAttribute('aria-label')}))(document.getElementById('rightStrip'))";

/// `task:viewer-compact-follow-ups`, acceptance 1: with the details sidebar collapsed, a node the
/// reader chooses (here by its search hit, in the left sidebar) marks the right strip with a dot and
/// a title and accessible name naming the node, and leaves the sidebar collapsed; a click on the
/// strip restores the sidebar showing that node and clears the mark. Collapsing the sidebar again
/// over what the reader has seen marks nothing.
#[test]
fn a_node_chosen_behind_the_collapsed_details_marks_their_strip_and_the_strip_shows_it() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let seeded = Seeded::new(&manifest_dir().join("tests/fixtures/view-page/sounding"));
    let projection = seeded.projection();
    let node = busiest(&projection);
    let (id, name) = (
        node["id"].as_str().unwrap().to_owned(),
        node["name"].as_str().unwrap().to_owned(),
    );
    let double = seeded.double(Duration::ZERO);
    let mut driven = Driven::launch(&browser, &format!("{}#view=2d&compact=right", double.url));
    assert!(
        driven.wait_for(&format!("{SETTLED} && !!window.__viewer.renderer"), 90),
        "the page settled"
    );
    measured_when(
        &mut driven,
        "m.left > 0 && m.right === 0 && m.rightStrip > 0",
        "the details sidebar is collapsed",
    );
    let before = driven.eval(MARK);
    assert_eq!(before["dot"], 0, "no mark before a choice: {before}");

    let quoted = serde_json::to_string(&name).unwrap();
    driven.eval(&format!(
        "(s => {{ s.value = {quoted}; s.dispatchEvent(new Event('input', {{bubbles: true}})); }})(document.getElementById('search'))"
    ));
    let hit = format!("#hits .hit[data-node='{id}']");
    assert!(
        driven.wait_for(&format!("!!document.querySelector(\"{hit}\")"), 30),
        "the search lists the node"
    );
    driven.eval(&format!("document.querySelector(\"{hit}\").click()"));
    assert!(
        driven.wait_for(
            &format!("document.querySelector('#panel h1')?.textContent === {quoted}"),
            30
        ),
        "the collapsed panel holds the node"
    );
    let quoted_name = quoted.clone();
    assert!(
        driven.wait_for(
            &format!(
                "(k => k.dot > 0 && k.title.includes({quoted_name}) && (k.label || '').includes({quoted_name}))({MARK})"
            ),
            20
        ),
        "the strip is marked with the node: {}",
        driven.eval(MARK)
    );
    let m = driven.eval(MEASURE);
    assert!(
        width(&m, "right") == 0.0 && width(&m, "rightStrip") > 0.0,
        "the sidebar is not forced open: {m}"
    );
    let hash = m["hash"].as_str().unwrap();
    assert!(
        hash.contains(&format!("node={id}")) && hash.contains("compact=right"),
        "{m}"
    );

    driven.eval("document.getElementById('rightStrip').click()");
    let restored = measured_when(
        &mut driven,
        "m.right > 0 && m.rightStrip === 0 && Math.abs(m.canvas - m.stage) < 1",
        "the strip restored the details sidebar",
    );
    assert_eq!(restored["panel"], name, "it shows the node: {restored}");
    driven.eval("document.getElementById('foldRight').click()");
    measured_when(
        &mut driven,
        "m.right === 0 && m.rightStrip > 0",
        "the tab collapsed the details sidebar again",
    );
    let after = driven.eval(MARK);
    assert_eq!(
        after["dot"], 0,
        "no mark over a node the reader has seen: {after}"
    );
    assert_eq!(after["title"], before["title"], "{before} → {after}");
    assert!(
        driven.errors.is_empty(),
        "the page reported errors: {:?}",
        driven.errors
    );
}

/// Every type chip's role, tab index and `aria-pressed`, and whether it is drawn as hidden.
const CHIPS: &str = "[...document.querySelectorAll('#nodeTypes .chip, #edgeTypes .chip')].map(c =>
  ({type: c.dataset.type, role: c.getAttribute('role'), tab: c.tabIndex, pressed: c.getAttribute('aria-pressed'), off: c.classList.contains('off')}))";

/// `task:viewer-compact-follow-ups`, acceptance 2: a type chip is a button the reader focuses and
/// presses from the keyboard. Enter or Space hides or shows its type; Shift+Enter or Shift+Space
/// shows only that type, and pressed again every type. `aria-pressed` says whether the type is
/// shown, the address follows and the focus stays on the chip. The fold tabs and the strips are
/// named in words, not by their arrow, and the Compact button says whether it is pressed.
#[test]
fn a_type_chip_hides_and_solos_its_type_from_the_keyboard_and_the_toggles_are_named() {
    const SHIFT: u8 = 8;
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let seeded = Seeded::new(&manifest_dir().join("tests/fixtures/view-page/sounding"));
    let double = seeded.double(Duration::ZERO);
    let mut driven = Driven::launch(&browser, &format!("{}#view=2d", double.url));
    assert!(
        driven.wait_for(
            &format!("{SETTLED} && !!window.__viewer.renderer && !!document.querySelector('#nodeTypes .chip')"),
            90
        ),
        "the page settled"
    );
    let chips = driven.eval(CHIPS);
    let chips = chips.as_array().unwrap();
    assert!(!chips.is_empty());
    for chip in chips {
        assert!(
            chip["role"] == "button" && chip["tab"] == 0 && chip["pressed"] == "true",
            "a shown chip is a focusable button, pressed: {chip}"
        );
    }
    let names = driven.eval(
        "['foldLeft', 'foldRight', 'leftStrip', 'rightStrip'].map(id => document.getElementById(id).getAttribute('aria-label'))",
    );
    for name in names.as_array().unwrap() {
        let name = name.as_str().unwrap_or_default();
        assert!(
            name.chars().filter(|c| c.is_alphabetic()).count() >= 4 && !name.contains(['‹', '›']),
            "a tab or strip named in words: {names}"
        );
    }
    // every button on the page: a glyph for text is no name, so one carries a label; a title names only
    // a button with no text at all
    let unnamed = driven.eval(
        "[...document.querySelectorAll('button')].filter(b => !/\\p{L}/u.test(b.getAttribute('aria-label') ?? (b.textContent.trim() ? b.textContent : b.title)))
           .map(b => b.id || b.textContent)",
    );
    assert_eq!(unnamed, serde_json::json!([]), "buttons named by a glyph");
    assert_eq!(
        driven.eval("document.getElementById('compact').getAttribute('aria-pressed')"),
        "false"
    );

    let types: Vec<String> = driven
        .eval("[...document.querySelectorAll('#nodeTypes .chip')].map(c => c.dataset.type)")
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap().to_owned())
        .collect();
    assert!(types.len() >= 2, "{types:?}");
    let (a, b) = (&types[0], &types[1]);
    let focus = |driven: &mut Driven, t: &str| {
        let held = driven.eval(&format!(
            "document.querySelector(\"#nodeTypes .chip[data-type='{t}']\").focus(); document.activeElement.dataset.type ?? null"
        ));
        assert_eq!(held, t, "the chip takes the focus");
    };
    let state = |t: &str, off: bool, pressed: &str| {
        format!(
            "(c => c && c.classList.contains('off') === {off} && c.getAttribute('aria-pressed') === '{pressed}')\
             (document.querySelector(\"#nodeTypes .chip[data-type='{t}']\")) && document.activeElement?.dataset.type === '{t}'"
        )
    };
    let hash = "decodeURIComponent(location.hash)";

    focus(&mut driven, a);
    driven.press("Enter", 0);
    assert!(
        driven.wait_for(
            &format!("{} && {hash}.includes('types=')", state(a, true, "false")),
            20
        ),
        "Enter hid {a}: {}",
        driven.eval(CHIPS)
    );
    driven.press(" ", 0);
    assert!(
        driven.wait_for(
            &format!(
                "{} && !{hash}.includes('types=') && !document.querySelector('#nodeTypes .chip.off')",
                state(a, false, "true")
            ),
            20
        ),
        "Space showed {a} again: {}",
        driven.eval(CHIPS)
    );

    focus(&mut driven, b);
    driven.press("Enter", SHIFT);
    let alone = format!(
        "document.querySelectorAll('#nodeTypes .chip.off').length === {} && document.querySelectorAll('#nodeTypes .chip[aria-pressed=false]').length === {}",
        types.len() - 1,
        types.len() - 1
    );
    assert!(
        driven.wait_for(
            &format!(
                "{} && {alone} && {hash}.includes('types={b}')",
                state(b, false, "true")
            ),
            20
        ),
        "Shift+Enter showed only {b}: {}",
        driven.eval(CHIPS)
    );
    driven.press(" ", SHIFT);
    assert!(
        driven.wait_for(
            &format!(
                "{} && !{hash}.includes('types=') && !document.querySelector('#nodeTypes .chip.off')",
                state(b, false, "true")
            ),
            20
        ),
        "Shift+Space showed every type again: {}",
        driven.eval(CHIPS)
    );

    driven.eval("document.activeElement.blur()");
    driven.key("c");
    assert!(
        driven.wait_for(
            "document.getElementById('compact').getAttribute('aria-pressed') === 'true'",
            20
        ),
        "the Compact button is pressed while both sidebars are collapsed"
    );
    driven.key("c");
    assert!(
        driven.wait_for(
            "document.getElementById('compact').getAttribute('aria-pressed') === 'false'",
            20
        ),
        "and not once they are shown"
    );
    assert!(
        driven.errors.is_empty(),
        "the page reported errors: {:?}",
        driven.errors
    );
}

/// `task:viewer-compact-follow-ups`, acceptance 3: a window narrower than the two sidebars and a
/// usable graph (290 + 360 + 320 = 970 px) opens compact, so a 600 px window draws a graph of all
/// but the two strips; the address carries `compact=1`. Shown again, the sidebars write `compact=0`,
/// which a reload keeps: the address says otherwise.
#[test]
fn a_narrow_window_opens_compact_and_the_graph_keeps_its_width() {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return;
    };
    let seeded = Seeded::new(&manifest_dir().join("tests/fixtures/view-page/sounding"));
    let double = seeded.double(Duration::ZERO);
    let mut driven = Driven::launch_sized(&browser, &format!("{}#view=2d", double.url), (600, 900));
    assert!(
        driven.wait_for(&format!("{SETTLED} && !!window.__viewer.renderer"), 90),
        "the page settled"
    );
    let m = measured_when(
        &mut driven,
        "m.left === 0 && m.right === 0 && m.leftStrip > 0 && m.rightStrip > 0 && m.canvas > 0 \
         && Math.abs(m.canvas - m.stage) < 1 && m.hash.includes('compact=1')",
        "a 600 px window opened compact",
    );
    eprintln!("600 px: {m}");
    assert!(
        (width(&m, "canvas") + width(&m, "leftStrip") + width(&m, "rightStrip") - 600.0).abs()
            < 1.0,
        "the graph takes all but the strips: {m}"
    );

    driven.key("c");
    measured_when(
        &mut driven,
        "m.left > 0 && m.right > 0 && m.hash.includes('compact=0')",
        "shown again, the sidebars write compact=0",
    );
    driven.eval("window.__before = true");
    driven.call("Page.reload", serde_json::json!({}));
    assert!(
        driven.wait_for(&format!("!window.__before && {SETTLED}"), 90),
        "the page reloaded"
    );
    measured_when(
        &mut driven,
        "m.left > 0 && m.right > 0 && m.hash.includes('compact=0')",
        "compact=0 in the address shows both in a narrow window",
    );
    assert!(
        driven.errors.is_empty(),
        "the page reported errors: {:?}",
        driven.errors
    );
}
