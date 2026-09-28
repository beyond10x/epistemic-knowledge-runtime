//! Adversary cases, pass 2, for `story:data-free-graph-viewer` (unit P, the page half).
//!
//! The correction keeps every integer a double cannot hold as its source text. A valid-time bound
//! is an `i64` of milliseconds (`ekr-core` `Timestamp`, "roughly ±292 million years"), so a bound
//! outside ±2^53 now reaches the page as a string, and the valid-time code reads only numbers.
//! Each case seeds the unit's `sounding` fixture with one relation assertion moved into deep time
//! (a closed interval, both ends beyond 2^53 ms before the epoch), serves it with
//! `ekr view --port 0`, and reads the DOM a headless Chromium builds. The cases hold the earlier page, which
//! `ekr view` keeps at `/alt`: its valid-time strip is the one they read.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::Value;

const DEEP_FROM: &str = "-200000000000000000";
const DEEP_TO: &str = "-100000000000000000";
const DEEP_CLAIM: &str = "00000000-0000-4000-8000-00000000a510";
const SUBJECT: &str = "00000000-0000-4000-8000-00000000a301";

fn manifest_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
}

fn sounding() -> PathBuf {
    manifest_dir().join("tests/fixtures/view-page/sounding")
}

/// The sounding seed with assertion a510 valid from `DEEP_FROM` to `DEEP_TO`.
fn deep_time_seed() -> String {
    let seed = std::fs::read_to_string(sounding().join("seed.yaml")).unwrap();
    let from = "          from: 1735689600000\n          to: 1777593600000\n";
    assert_eq!(
        seed.matches(from).count(),
        1,
        "the fixture holds a510's valid time once"
    );
    seed.replace(
        from,
        &format!("          from: {DEEP_FROM}\n          to: {DEEP_TO}\n"),
    )
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

/// The markup of the assertion card whose heading row names `claim`.
fn card<'a>(dom: &'a str, claim: &str) -> &'a str {
    let heading = format!("<th>{claim}</th>");
    let at = dom
        .find(&heading)
        .unwrap_or_else(|| panic!("no assertion card for {claim}: {dom}"));
    let start = dom[..at].rfind("<table").unwrap();
    let end = at + dom[at..].find("</table>").unwrap();
    &dom[start..end]
}

fn served() -> Option<(PathBuf, Store)> {
    let Some(browser) = browser() else {
        eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
        return None;
    };
    Some((browser, Store::seeded(&deep_time_seed())))
}

/// The story shows each assertion's valid time. A closed interval whose end lies beyond 2^53 ms
/// must be shown with that end, not as the open end "…".
#[test]
fn a_valid_time_end_beyond_two_to_the_fifty_three_is_shown_and_not_as_open() {
    let Some((browser, store)) = served() else {
        return;
    };
    let server = store.serve();
    let dom = rendered(&browser, &format!("{}alt?node={SUBJECT}", server.url));
    let shown = card(&dom, DEEP_CLAIM);
    assert!(
        shown.contains(&format!("{DEEP_FROM} → {DEEP_TO}")),
        "a510 is valid {DEEP_FROM} → {DEEP_TO}; its card shows: {shown}"
    );
}

/// With a valid time chosen, an assertion whose valid interval ended before it is drawn outside.
/// a510 ended at -10^17 ms; at 2026-05-01 it is not valid.
#[test]
fn an_assertion_that_ended_beyond_two_to_the_fifty_three_before_the_epoch_is_outside_now() {
    let Some((browser, store)) = served() else {
        return;
    };
    let server = store.serve();
    let dom = rendered(
        &browser,
        &format!("{}alt?node={SUBJECT}&valid=1777593600000", server.url),
    );
    assert!(
        dom.contains("id=\"valid-at\" class=\"muted\">2026-05-01T00:00:00Z<"),
        "the valid time 2026-05-01 is chosen: {dom}"
    );
    let shown = card(&dom, DEEP_CLAIM);
    assert!(
        shown.starts_with("<table class=\"claim outside\""),
        "a510 ended at {DEEP_TO} ms and is shown as valid at 2026-05-01: {shown}"
    );
}

/// The valid-time strip "spans every valid-time bound the revision's assertions carry". A valid
/// time inside a510's interval is one the strip can reach, and the URL keeps it.
#[test]
fn the_valid_time_strip_spans_a_bound_beyond_two_to_the_fifty_three() {
    let Some((browser, store)) = served() else {
        return;
    };
    let server = store.serve();
    let dom = rendered(
        &browser,
        &format!("{}alt?node={SUBJECT}&valid=-150000000000000000", server.url),
    );
    assert!(
        dom.contains("id=\"valid-at\" class=\"muted\">-150000000000000000<"),
        "a valid time inside a510's interval was moved off it; the strip shows: {}",
        dom.split("id=\"valid-at\"")
            .nth(1)
            .and_then(|rest| rest.split('<').next())
            .unwrap_or("nothing")
    );
}
