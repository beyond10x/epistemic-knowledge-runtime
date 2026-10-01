//! Adversary pass on `story:fact-quality-by-judged-sample`, CLI lanes: `ekr fact-quality` one-shot
//! and as an `ekr session` verb.
//!
//! * The numbers the CLI prints are the binary64 values the library computed. `views.yaml` says
//!   "Every host therefore answers the same binary64 values. A Decimal in ekr.fact-quality/1 is a
//!   JSON number: the shortest decimal text that reads back as the same binary64 value", and
//!   `docs/cli.md` says "every host prints the same numbers". Both lanes print
//!   `serde_json::from_slice::<Value>` of the library's bytes (`crates/ekr/src/cli/sample.rs`).
//!   Under `cargo test -p ekr` this case is green whatever the CLI does: the `jsonschema`
//!   dev-dependency turns on `serde_json/float_roundtrip` for the binary under test, which a
//!   `cargo build` or `cargo install` of `ekr` does not have. The same parse without that feature
//!   is held red in `crates/ekr-views/tests/adversary_x6_q.rs`.
//! * An empty judged sample, and an empty input.

use std::io::Write;
use std::process::{Output, Stdio};

use ekr_core::AssertionId;
use ekr_views::{report_fact_quality, FactJudgements, Judgement, Verdict};
use serde_json::{json, Value};

fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

fn with_stdin(args: &[&str], stdin: &str) -> Output {
    let mut child = ekr()
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

fn assertion(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

/// Two of three passed, as the documented example judges them.
fn two_of_three() -> (String, FactJudgements) {
    let verdicts = [(0x510, "Pass"), (0x511, "Pass"), (0x512, "Fail")];
    let document = json!({
        "format": "ekr.fact-judgements/1",
        "judgements": verdicts
            .iter()
            .map(|(n, verdict)| json!({"assertion": assertion(*n), "verdict": verdict}))
            .collect::<Vec<_>>(),
    })
    .to_string();
    let judged = FactJudgements {
        sample: None,
        judgements: verdicts
            .iter()
            .map(|(n, verdict)| Judgement {
                assertion: assertion(*n).parse::<AssertionId>().unwrap(),
                verdict: if *verdict == "Pass" {
                    Verdict::Pass
                } else {
                    Verdict::Fail
                },
            })
            .collect(),
    };
    (document, judged)
}

/// The JSON number after `"key":` (compact) or `"key": ` (pretty), read with the standard
/// library's correctly rounded parser.
fn number(text: &str, key: &str) -> f64 {
    let marker = format!("\"{key}\":");
    let start = text
        .find(&marker)
        .unwrap_or_else(|| panic!("{key} in {text}"))
        + marker.len();
    let rest = text[start..].trim_start();
    let end = rest.find([',', '}', '\n']).expect("a terminated number");
    rest[..end].trim().parse().unwrap()
}

#[test]
fn fact_quality_prints_the_binary64_values_the_library_computed_one_shot_and_in_a_session() {
    let (document, judged) = two_of_three();
    // 15, 75 and 81 basis points: z is one the CLI's parse moves to a neighbouring binary64.
    let confidences = [15_i64, 75, 81, 9_500];
    let mut differ = Vec::new();
    for confidence in confidences {
        let library = report_fact_quality(&judged, Some(confidence)).unwrap();
        let library = std::str::from_utf8(&library.bytes).unwrap().to_owned();
        let confidence_text = confidence.to_string();
        let one_shot = with_stdin(
            &["fact-quality", "-", "--confidence", &confidence_text],
            &document,
        );
        assert_eq!(
            one_shot.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&one_shot.stderr)
        );
        let printed = String::from_utf8(one_shot.stdout).unwrap();
        for key in ["z", "lower", "upper", "rate"] {
            let (computed, shown) = (number(&library, key), number(&printed, key));
            if computed.to_bits() != shown.to_bits() {
                differ.push(format!(
                    "one-shot at {confidence}: {key} computed {computed:?}, printed {shown:?}"
                ));
            }
        }
    }

    let work = tempfile::tempdir().unwrap();
    let host = ekr().args(["example", "ekr.cli-host/1"]).output().unwrap();
    assert_eq!(host.status.code(), Some(0));
    std::fs::write(work.path().join("host.json"), host.stdout).unwrap();
    let mut session = ekr()
        .arg("--host")
        .arg(work.path().join("host.json"))
        .arg("--store")
        .arg(work.path().join("absent"))
        .args(["--backend", "file", "session"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = session.stdin.take().unwrap();
        for confidence in confidences {
            let request = json!({
                "argv": ["fact-quality", "-", "--confidence", confidence.to_string()],
                "stdin": document,
            });
            writeln!(stdin, "{request}").unwrap();
        }
    }
    let output = session.wait_with_output().unwrap();
    let lines: Vec<String> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        lines.len(),
        confidences.len(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (confidence, line) in confidences.iter().zip(&lines) {
        let answer: Value = serde_json::from_str(line).unwrap();
        assert_eq!(answer["exit"], 0, "{line}");
        let library = report_fact_quality(&judged, Some(*confidence)).unwrap();
        let library = std::str::from_utf8(&library.bytes).unwrap().to_owned();
        for key in ["z", "lower", "upper", "rate"] {
            let (computed, shown) = (number(&library, key), number(line, key));
            if computed.to_bits() != shown.to_bits() {
                differ.push(format!(
                    "session at {confidence}: {key} computed {computed:?}, printed {shown:?}"
                ));
            }
        }
    }
    assert!(differ.is_empty(), "{differ:#?}");
}

#[test]
fn an_empty_judged_sample_reports_zero_judged_and_an_empty_input_is_a_fault() {
    let none = with_stdin(
        &["fact-quality", "-"],
        r#"{"format":"ekr.fact-judgements/1","judgements":[]}"#,
    );
    assert_eq!(
        none.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&none.stderr)
    );
    let report: Value = serde_json::from_slice(&none.stdout).unwrap();
    assert_eq!(
        (&report["judged"], &report["passed"], &report["failed"]),
        (&json!(0), &json!(0), &json!(0))
    );
    for omitted in ["rate", "lower", "upper"] {
        assert!(report.get(omitted).is_none(), "{report}");
    }

    let empty = with_stdin(&["fact-quality", "-"], "");
    assert_eq!(empty.status.code(), Some(1));
    assert!(empty.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&empty.stderr);
    assert!(stderr.contains("ekr.fact-judgements/1"), "{stderr}");
}
