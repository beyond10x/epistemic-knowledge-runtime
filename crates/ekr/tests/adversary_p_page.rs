//! Adversary cases for `story:data-free-graph-viewer` (unit P, the page half).
//!
//! Each case seeds a store derived from the unit's own `tests/fixtures/view-page/sounding`
//! fixture through the real binary, serves it with `ekr view --port 0`, and reads the DOM a
//! headless Chromium builds from the embedded page. Without a local headless Chromium the
//! browser cases say so and return.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::Value;

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

/// The DOM after `url` loads, from a browser held to two cores at the lowest priority.
fn rendered(browser: &Path, url: &str) -> String {
    let profile = tempfile::tempdir().unwrap();
    let output = Command::new("taskset")
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
    use std::io::{Read, Write};
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
