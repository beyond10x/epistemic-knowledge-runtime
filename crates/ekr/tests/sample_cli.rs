//! `story:fact-quality-by-judged-sample`: `ekr sample` prints the `ekr.fact-sample/1` document of
//! one revision (`ekr.views.DrawFactSample`) and `ekr fact-quality` the `ekr.fact-quality/1`
//! report of a judged sample (`ekr.views.ReportFactQuality`), as one-shot verbs and as
//! `ekr session` verbs, on both providers.
//!
//! The store is the example seed: three `CEO_OF` assertions about Alice and Bob, each citing one
//! of two evidence records whose bytes are text. The draw and the Wilson figures themselves are
//! held on richer inputs in `crates/ekr-views/tests/sample.rs` and the views conformance suite.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const PERSON: &str = "00000000-0000-4000-8000-000000000201";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

/// Exit 0 and stdout as text.
fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// One provider in its own directory, seeded from `ekr example ekr-seed/2`.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn seeded(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        world.file("host.json", &text(&["example", "ekr.cli-host/1"]));
        let seed = world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        assert_eq!(world.ok(&["seed", &seed])["result"]["revision"], 0);
        world
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn file(&self, name: &str, contents: &str) -> String {
        let path = self.directory.path().join(name);
        std::fs::write(&path, contents).unwrap();
        path.display().to_string()
    }

    fn command(&self, verb: &[&str]) -> std::process::Command {
        let mut command = ekr();
        command
            .arg("--host")
            .arg(self.directory.path().join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(verb);
        command
    }

    fn run(&self, verb: &[&str]) -> Output {
        self.command(verb).output().unwrap()
    }

    fn ok(&self, verb: &[&str]) -> Value {
        serde_json::from_slice(&self.stdout(verb)).unwrap()
    }

    fn stdout(&self, verb: &[&str]) -> Vec<u8> {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    fn session(&self, requests: &[Value]) -> Vec<Value> {
        let mut child = self
            .command(&["session"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let mut stdin = child.stdin.take().unwrap();
            for request in requests {
                writeln!(stdin, "{request}").unwrap();
            }
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{}", self.backend);
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

/// `ekr fact-quality` with `args`, no store configured, `stdin` on its standard input.
fn fact_quality(args: &[&str], stdin: &str) -> Output {
    let mut child = ekr()
        .arg("fact-quality")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// `sample` with every assertion's `recorded_from`, the seed's wall-clock time, removed.
fn without_recorded_from(mut sample: Value) -> Value {
    for item in sample["items"].as_array_mut().unwrap() {
        let assertion = item["assertion"].as_object_mut().unwrap();
        assert!(assertion.remove("recorded_from").is_some(), "{assertion:?}");
    }
    sample
}

fn ids(sample: &Value) -> Vec<String> {
    sample["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["assertion"]["id"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn sample_draws_one_sample_for_one_seed_size_and_revision_on_both_providers() {
    let mut printed = Vec::new();
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let first = world.stdout(&["sample", "--seed", "42", "--size", "2"]);
        assert_eq!(
            first,
            world.stdout(&["sample", "--seed", "42", "--size", "2", "--revision", "0"]),
            "{backend}: the head is revision 0"
        );
        let sample: Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(
            sample["meta"],
            json!({
                "format": "ekr.fact-sample/1",
                "revision": 0,
                "seed": 42,
                "size": 2,
                "population": 3,
                "drawn": 2,
            }),
            "{backend}"
        );
        for item in sample["items"].as_array().unwrap() {
            assert_eq!(item["subject_kind"], "Node");
            assert_eq!(item["subject_type"], PERSON);
            assert!(["Alice", "Bob"].contains(&item["subject_name"].as_str().unwrap()));
            assert_eq!(item["predicate_name"], "CEO_OF");
            assert_eq!(item["object_name"], "Acme");
            let evidence = item["evidence"].as_array().unwrap();
            assert_eq!(evidence.len(), 1, "{item}");
            assert!(
                evidence[0]["text"]
                    .as_str()
                    .unwrap()
                    .contains("CEO of Acme"),
                "{item}"
            );
        }
        let whole = world.ok(&["sample", "--seed", "42", "--size", "1000"]);
        assert_eq!(ids(&whole)[..2], ids(&sample)[..], "{backend}: a prefix");
        let negative = world.ok(&["sample", "--seed", "-42", "--size", "3"]);
        assert_eq!(negative["meta"]["seed"], -42);
        let none = world.ok(&[
            "sample",
            "--seed",
            "1",
            "--size",
            "3",
            "--type",
            ORGANIZATION,
        ]);
        assert_eq!(
            none["meta"]["population"], 0,
            "{backend}: no fact is about Acme"
        );
        printed.push(without_recorded_from(sample));
    }
    // `ekr seed` stamps each seeded assertion with the host clock, so the two stores differ in
    // `recorded_from` and in nothing the draw reads; with a fixed clock the bytes are equal
    // (`crates/ekr-views/tests/sample.rs`).
    assert_eq!(printed[0], printed[1], "the two providers print one sample");
}

#[test]
fn sample_refuses_a_size_outside_its_bounds_and_a_revision_beyond_the_head_by_name() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        for size in ["0", "1001", "-1"] {
            let refused = world.run(&["sample", "--seed", "1", "--size", size]);
            assert_eq!(refused.status.code(), Some(2), "{backend} size {size}");
            let stderr = String::from_utf8_lossy(&refused.stderr);
            assert!(stderr.contains("ekr.views.LimitExceeded"), "{stderr}");
            assert!(stderr.contains("size"), "{stderr}");
            assert!(refused.stdout.is_empty());
        }
        let beyond = world.run(&["sample", "--seed", "1", "--size", "1", "--revision", "3"]);
        assert_eq!(beyond.status.code(), Some(2), "{backend}");
        assert!(String::from_utf8_lossy(&beyond.stderr).contains("ekr.views.RevisionNotFound"));
    }
}

fn judged(verdicts: &[(&str, &str)]) -> String {
    json!({
        "format": "ekr.fact-judgements/1",
        "sample": {"revision": 0, "seed": 42, "size": 3},
        "judgements": verdicts
            .iter()
            .map(|(assertion, verdict)| json!({"assertion": assertion, "verdict": verdict}))
            .collect::<Vec<_>>(),
    })
    .to_string()
}

#[test]
fn a_drawn_sample_judged_by_the_caller_is_reported_with_its_wilson_interval() {
    let world = World::seeded("sqlite");
    let sample = world.ok(&["sample", "--seed", "42", "--size", "3"]);
    let drawn = ids(&sample);
    let verdicts: Vec<(&str, &str)> = drawn
        .iter()
        .enumerate()
        .map(|(n, id)| (id.as_str(), if n == 0 { "Fail" } else { "Pass" }))
        .collect();
    let document = judged(&verdicts);
    let file = world.file("judged.json", &document);

    // It reads no store: no --host, --store or --backend.
    let report: Value = serde_json::from_str(&text(&["fact-quality", &file])).unwrap();
    assert_eq!(report["meta"]["format"], "ekr.fact-quality/1");
    assert_eq!(report["meta"]["confidence"], 9_500);
    assert_eq!(
        report["meta"]["sample"],
        json!({"revision": 0, "seed": 42, "size": 3})
    );
    assert_eq!(
        (&report["judged"], &report["passed"], &report["failed"]),
        (&json!(3), &json!(2), &json!(1))
    );
    let rate = report["rate"].as_f64().unwrap();
    assert!((rate - 2.0 / 3.0).abs() < 1e-15, "{report}");
    // 2 of 3 at 95 %: (0.2077, 0.9385).
    let (lower, upper) = (
        report["lower"].as_f64().unwrap(),
        report["upper"].as_f64().unwrap(),
    );
    assert!((lower - 0.207_659_600_8).abs() < 1e-9, "{report}");
    assert!((upper - 0.938_508_055_3).abs() < 1e-9, "{report}");

    let from_stdin = fact_quality(&["-", "--confidence", "9900"], &document);
    assert_eq!(from_stdin.status.code(), Some(0));
    let at_99: Value = serde_json::from_slice(&from_stdin.stdout).unwrap();
    assert_eq!(at_99["meta"]["confidence"], 9_900);
    assert!(at_99["lower"].as_f64().unwrap() < lower);
    assert!(at_99["upper"].as_f64().unwrap() > upper);
}

#[test]
fn fact_quality_refuses_by_name_and_a_malformed_document_is_a_fault() {
    let a = "00000000-0000-4000-8000-000000000510";
    let b = "00000000-0000-4000-8000-000000000511";
    let twice = fact_quality(&["-"], &judged(&[(a, "Pass"), (b, "Pass"), (a, "Fail")]));
    assert_eq!(twice.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&twice.stderr);
    assert!(stderr.contains("ekr.views.JudgedTwice"), "{stderr}");
    assert!(stderr.contains(a), "{stderr}");
    assert!(twice.stdout.is_empty());

    for confidence in ["0", "10000"] {
        let refused = fact_quality(&["-", "--confidence", confidence], &judged(&[(a, "Pass")]));
        assert_eq!(refused.status.code(), Some(2), "{confidence}");
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert!(stderr.contains("ekr.views.LimitExceeded"), "{stderr}");
        assert!(stderr.contains("confidence"), "{stderr}");
    }

    for malformed in [
        "{".to_owned(),
        json!({"format": "ekr.fact-judgements/1", "judgements": [{"assertion": a, "verdict": "ok"}]})
            .to_string(),
        json!({"format": "ekr.fact-sample/1", "judgements": []}).to_string(),
    ] {
        let fault = fact_quality(&["-"], &malformed);
        assert_eq!(fault.status.code(), Some(1), "{malformed}");
        let stderr = String::from_utf8_lossy(&fault.stderr);
        assert!(stderr.contains("ekr.fact-judgements/1"), "{stderr}");
    }
}

#[test]
fn a_session_serves_sample_and_fact_quality_as_the_one_shot_verbs_print_them() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let sample = world.ok(&["sample", "--seed", "5", "--size", "2"]);
        let verdicts: Vec<(String, &str)> =
            ids(&sample).into_iter().map(|id| (id, "Pass")).collect();
        let borrowed: Vec<(&str, &str)> = verdicts
            .iter()
            .map(|(id, verdict)| (id.as_str(), *verdict))
            .collect();
        let document = judged(&borrowed);
        let answers = world.session(&[
            json!({"argv": ["sample", "--seed", "5", "--size", "2"]}),
            json!({"argv": ["fact-quality", "-"], "stdin": document}),
            json!({"argv": ["sample", "--seed", "5", "--size", "0"]}),
        ]);
        assert_eq!(answers.len(), 3, "{backend}");
        assert_eq!(answers[0]["exit"], 0, "{backend}: {}", answers[0]);
        assert_eq!(answers[0]["stdout"], sample, "{backend}");
        assert_eq!(answers[1]["exit"], 0, "{backend}: {}", answers[1]);
        let one_shot: Value =
            serde_json::from_slice(&fact_quality(&["-"], &document).stdout).unwrap();
        assert_eq!(answers[1]["stdout"], one_shot, "{backend}");
        assert_eq!(answers[1]["stdout"]["upper"], 1.0, "{backend}: all passed");
        assert_eq!(answers[2]["exit"], 2, "{backend}: {}", answers[2]);
        assert!(answers[2]["stderr"]
            .as_str()
            .unwrap()
            .contains("ekr.views.LimitExceeded"));
    }
}
