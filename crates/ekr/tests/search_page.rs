//! Search HTML is escaped, bounded and usable without loading the graph application.

#[path = "../src/cli/search_page.rs"]
mod search_page;

use search_page::{render, EvidenceLink, Page, SearchResult, State};

fn page<'a>(query: &'a str, state: State<'a>) -> Page<'a> {
    Page {
        query,
        revision: Some(7),
        requested_revision: None,
        state,
    }
}

#[test]
fn entry_has_an_accessible_get_form_without_scripts_or_external_assets() {
    let html = render(&page("", State::Initial));
    for expected in [
        "<form action=\"/find\" method=\"get\">",
        "<label for=\"query\">",
        "type=\"search\"",
        "name=\"q\"",
        "Search names and aliases",
        "Revision 7",
        "name=\"viewport\"",
    ] {
        assert!(html.contains(expected), "missing {expected}");
    }
    for forbidden in ["<script", "https://", "http://", "@import", "onload="] {
        assert!(!html.contains(forbidden), "unexpected {forbidden}");
    }
    assert!(!html.contains("type=\"hidden\" name=\"revision\""));
}

#[test]
fn historical_results_keep_authoritative_order_and_pin_every_link() {
    let results = vec![
        SearchResult {
            id: "node-first".into(),
            name: "First result".into(),
            alias: Some("Matched alias".into()),
            evidence: vec![EvidenceLink {
                id: "evidence-first".into(),
                label: "Retained source".into(),
            }],
        },
        SearchResult {
            id: "node-second".into(),
            name: "Second result".into(),
            alias: None,
            evidence: vec![],
        },
    ];
    let mut request = page(
        "result",
        State::Results {
            matches: &results,
            total: 2,
        },
    );
    request.requested_revision = Some(7);
    let html = render(&request);
    assert!(html.find("First result").unwrap() < html.find("Second result").unwrap());
    for expected in [
        "name=\"revision\" value=\"7\"",
        "/#revision=7&amp;node=node-first",
        "/evidence/evidence-first?revision=7",
        "Matched alias",
        "Retained source",
        "2 results",
    ] {
        assert!(html.contains(expected), "missing {expected}");
    }
}

#[test]
fn untrusted_text_is_escaped_and_identity_link_parameters_are_encoded() {
    let attack = "\"><script>alert('x')</script>&";
    let results = vec![SearchResult {
        id: "node&injected=1/#".into(),
        name: attack.into(),
        alias: Some(attack.into()),
        evidence: vec![EvidenceLink {
            id: "id/../?&".into(),
            label: attack.into(),
        }],
    }];
    let html = render(&page(
        attack,
        State::Results {
            matches: &results,
            total: 1,
        },
    ));
    assert!(!html.contains("<script>"));
    assert!(!html.contains(attack));
    assert!(html.contains("&quot;&gt;&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;&amp;"));
    assert!(html.contains("node=node%26injected%3D1%2F%23"));
    assert!(html.contains("/evidence/id%2F..%2F%3F%26?revision=7"));
}

#[test]
fn blank_no_match_and_unavailable_states_explain_the_next_action() {
    let initial = render(&page("", State::Initial));
    assert!(initial.contains("Enter a name or alias"));
    let empty = render(&page(
        "missing",
        State::Results {
            matches: &[],
            total: 0,
        },
    ));
    assert!(empty.contains("No matches"));
    assert!(empty.contains("Try part of a name or an alias"));
    let unavailable = render(&Page {
        query: "saved query",
        revision: None,
        requested_revision: Some(12),
        state: State::Unavailable,
    });
    assert!(unavailable.contains("Search is unavailable"));
    assert!(unavailable.contains("saved query"));
    assert!(!unavailable.contains("Revision 0"));
    assert!(!unavailable.contains("Revision 12"));
    assert!(unavailable.contains("name=\"revision\" value=\"12\""));
    assert!(!unavailable.contains("No matches"));
    let missing = render(&page("saved query", State::MissingRevision));
    assert!(missing.contains("This revision is unavailable"));
    assert!(!missing.contains("No matches"));
}

#[test]
fn results_and_visible_untrusted_fields_have_hard_bounds() {
    let huge = "word".repeat(10_000);
    let results: Vec<_> = (0..100)
        .map(|i| SearchResult {
            id: format!("node-{i}"),
            name: huge.clone(),
            alias: Some(huge.clone()),
            evidence: (0..50)
                .map(|n| EvidenceLink {
                    id: format!("evidence-{n}"),
                    label: huge.clone(),
                })
                .collect(),
        })
        .collect();
    let html = render(&page(
        &huge,
        State::Results {
            matches: &results,
            total: 100,
        },
    ));
    assert_eq!(html.matches("<li class=\"result\">").count(), 20);
    assert_eq!(html.matches("class=\"evidence\"").count(), 60);
    assert!(html.contains("Showing 20 of 100 results"));
    assert!(html.len() < 150_000, "unbounded HTML: {} bytes", html.len());
    assert!(!html.contains(&huge));
}

mod live {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpStream;
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    struct World {
        path: PathBuf,
        child: Option<Child>,
        authority: String,
    }
    impl World {
        fn command(&self, args: &[&str]) -> Command {
            let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
            for key in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
                command.env_remove(key);
            }
            command
                .arg("--host")
                .arg(self.path.join("host.json"))
                .arg("--store")
                .arg(self.path.join("store"))
                .args(["--backend", "sqlite"])
                .args(args);
            command
        }
        fn new(seeded: bool) -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "ekr-search-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::SeqCst)
            ));
            std::fs::create_dir(&path).unwrap();
            let fixtures = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("tests/fixtures/retraction");
            std::fs::copy(fixtures.join("host.json"), path.join("host.json")).unwrap();
            let mut world = Self {
                path,
                child: None,
                authority: String::new(),
            };
            if seeded {
                let out = world
                    .command(&["seed"])
                    .arg(fixtures.join("seed.yaml"))
                    .output()
                    .unwrap();
                assert!(
                    out.status.success(),
                    "{}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            let mut child = world
                .command(&["view", "--port", "0"])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let mut line = String::new();
            BufReader::new(child.stdout.take().unwrap())
                .read_line(&mut line)
                .unwrap();
            world.authority = line
                .split('"')
                .nth(3)
                .expect("listener URL")
                .strip_prefix("http://")
                .unwrap()
                .trim_end_matches('/')
                .into();
            world.child = Some(child);
            world
        }
        fn get(&self, path: &str) -> (u16, String, String) {
            let mut stream = TcpStream::connect(&self.authority).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(60)))
                .unwrap();
            write!(
                stream,
                "GET {path} HTTP/1.1\r\nHost: {}\r\n\r\n",
                self.authority
            )
            .unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            let (headers, body) = response.split_once("\r\n\r\n").unwrap();
            (
                headers.split_whitespace().nth(1).unwrap().parse().unwrap(),
                headers.into(),
                body.into(),
            )
        }
        fn commit_assertion(&self) {
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
                let output = self.command(&args).output().unwrap();
                assert!(
                    output.status.success(),
                    "{args:?}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
    impl Drop for World {
        fn drop(&mut self) {
            if let Some(child) = &mut self.child {
                let _ = child.kill();
                let _ = child.wait();
            }
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn search_entry_works_without_graph_libraries_and_keeps_response_protections() {
        let world = World::new(true);
        let (status, headers, body) = world.get("/find");
        assert_eq!(status, 200, "{body}");
        assert!(headers.contains("Content-Type: text/html; charset=utf-8"));
        for expected in [
            "Cache-Control: no-store",
            "X-Content-Type-Options: nosniff",
            "frame-ancestors 'none'",
            "script-src 'none'",
        ] {
            assert!(headers.contains(expected), "{headers}");
        }
        assert!(body.contains("<form action=\"/find\" method=\"get\">"));
        assert!(body.contains("Revision 0"));
        assert!(!body.contains("<script"));
        let (status, _, body) = world.get("/find?q=Alice");
        assert_eq!(status, 200);
        assert!(body.contains("Alice"));
        assert!(body.contains("/#revision=0&amp;node=00000000-0000-4000-8000-000000000301"));
    }

    #[test]
    fn unavailable_search_keeps_query_in_a_useful_html_page() {
        let world = World::new(false);
        let (status, headers, body) = world.get("/find?q=Saved+query&revision=9");
        assert_eq!(status, 503);
        assert!(headers.contains("Content-Type: text/html; charset=utf-8"));
        assert!(body.contains("Search is unavailable"));
        assert!(body.contains("Saved query"));
        assert!(body.contains("name=\"revision\" value=\"9\""));
        assert!(!world.path.join("store").exists());
    }

    #[test]
    fn historical_search_keeps_its_results_and_retained_evidence_pinned_after_a_commit() {
        let world = World::new(true);
        let before = world.get("/find?q=Alice&revision=0");
        assert_eq!(before.0, 200);
        assert!(!before.2.contains("class=\"evidence\""));
        world.commit_assertion();
        assert_eq!(world.get("/find?q=Alice&revision=0").2, before.2);
        let (status, _, current) = world.get("/find?q=Alice");
        assert_eq!(status, 200);
        assert!(current.contains("Revision 1"));
        assert!(current.contains("/evidence/00000000-0000-4000-8000-000000000401?revision=1"));
        assert!(current.contains("/#revision=1&amp;node=00000000-0000-4000-8000-000000000301"));
        let (status, headers, bytes) =
            world.get("/evidence/00000000-0000-4000-8000-000000000401?revision=0");
        assert_eq!(status, 200);
        assert!(headers.contains("Content-Type: text/plain; charset=utf-8"));
        assert_eq!(bytes, "Alice is CEO of Acme.");
        assert_eq!(
            world
                .get("/evidence/00000000-0000-4000-8000-000000000401?revision=99")
                .0,
            404
        );
        assert_eq!(
            world
                .get("/evidence/00000000-0000-4000-8000-000000000499?revision=0")
                .0,
            404
        );
        for path in ["/find?revision=99", "/find?q=Alice&revision=99"] {
            let (status, _, body) = world.get(path);
            assert_eq!(status, 404);
            assert!(body.contains("This revision is unavailable"));
        }
    }

    #[test]
    fn live_search_escapes_query_refuses_bad_bounds_and_matches_json_ranking() {
        let world = World::new(true);
        let (status, _, body) = world.get("/find?q=%22%3E%3Cscript%3E%26%27");
        assert_eq!(status, 200);
        assert!(body.contains("&quot;&gt;&lt;script&gt;&amp;&#39;"));
        assert!(!body.contains("<script>"));
        assert!(body.contains("No matches"));
        let (_, _, json) = world.get("/search?q=a&limit=20");
        let json: serde_json::Value = serde_json::from_str(&json).unwrap();
        let (_, _, html) = world.get("/find?q=a");
        let mut previous = None;
        for item in json["matches"].as_array().unwrap() {
            let position = html
                .find(&format!("node={}", item["id"].as_str().unwrap()))
                .unwrap();
            assert!(previous.is_none_or(|before| before < position));
            previous = Some(position);
        }
        assert!(previous.is_some());
        for path in [
            "/find?q=a&q=b",
            "/find?revision=-1",
            "/find?limit=999",
            "/find?q=%FF",
            "/evidence/00000000-0000-4000-8000-000000000401?revision=-1",
        ] {
            assert_eq!(world.get(path).0, 400, "{path}");
        }
        assert_eq!(world.get(&format!("/find?q={}", "x".repeat(2049))).0, 400);
    }
}
