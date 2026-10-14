//! Adversary, story:store-reading-code-names-no-contents, pass 1.
//!
//! The story's purpose is that a consumer's store-reading code names none of the store's
//! contents; its acceptance is that every planted name is reported with file and line. These
//! cases drive `ekr.views.FindCodeNames` with source lines a consumer plausibly writes, and the
//! literal scanner with input sizes a source tree plausibly holds.
//!
//! The store is the `edge-assertion` fixture: node type `Subject` with property `label`, edge type
//! `links` with property `weight`, nodes `alpha` and `beta` with aliases.

mod support;

use std::time::{Duration, Instant};

use ekr_views::SourceText;
use serde_json::Value;

use support::fixtures::{self, Fixture, Provider};

fn source(path: &str, text: &str) -> SourceText {
    SourceText {
        path: path.to_owned(),
        text: text.to_owned(),
    }
}

fn built() -> (tempfile::TempDir, ekr_kernel::Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::File);
    Fixture::EdgeAssertion.build(&runtime);
    (work, runtime)
}

/// `(line, literal)` of every finding for `text` as the one source `src/reader.ts`.
fn found(runtime: &ekr_kernel::Runtime, text: &str) -> Vec<(u64, String)> {
    let answer = ekr_views::find_code_names(runtime, None, &[source("src/reader.ts", text)])
        .expect("an answer");
    let value: Value = serde_json::from_slice(&answer.bytes).expect("JSON");
    value["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .map(|finding| {
            (
                finding["line"].as_u64().expect("line"),
                finding["literal"].as_str().expect("literal").to_owned(),
            )
        })
        .collect()
}

/// A quote character of one kind inside a literal of another kind is paired with the next quote of
/// its own kind on the line, so the literal after it is swallowed. Rust's `'"'` char literal, a
/// JavaScript `split('"')`, and an apostrophe in a double-quoted message before a single-quoted
/// name all hide a store name that stands quoted on its own.
#[test]
fn a_quote_of_another_kind_earlier_on_the_line_does_not_hide_a_quoted_store_name() {
    let (_work, runtime) = built();
    let text = "\
if c == '\"' { return find(\"Subject\"); }
const parts = line.split('\"'); const kind = \"Subject\";
raise KeyError(\"can't find\", 'Subject')
";
    assert_eq!(
        found(&runtime, text),
        vec![
            (1, "Subject".to_owned()),
            (2, "Subject".to_owned()),
            (3, "Subject".to_owned()),
        ]
    );
}

/// The fixture's edge type `links` and property `weight` are ontology-specific names: code that
/// quotes them only works on a store that declares them. Both are exempt because they are also
/// words somewhere in the runtime's ESS domains, so the check passes code that is not generic.
#[test]
fn an_edge_type_and_a_property_the_consumer_hard_codes_are_reported_even_if_vocabulary() {
    let (_work, runtime) = built();
    let text = "\
const related = edges.filter((edge) => edge.type_name === 'links');
const weight = edge.properties[\"weight\"];
";
    assert_eq!(
        found(&runtime, text),
        vec![(1, "links".to_owned()), (2, "weight".to_owned())]
    );
}

/// What the rule does report, and must keep reporting: a Rust raw string, JSON inside a
/// single-quoted string, a name after a closed literal on the same line.
#[test]
fn a_raw_string_and_a_name_inside_single_quoted_json_are_reported() {
    let (_work, runtime) = built();
    let text = "\
let kind = r#\"Subject\"#;
const q = '{\"type\": \"Subject\"}';
log(\"it is\", \"alpha\"); // don't
";
    assert_eq!(
        found(&runtime, text),
        vec![
            (1, "Subject".to_owned()),
            (2, "Subject".to_owned()),
            (3, "alpha".to_owned()),
        ]
    );
}

/// The fastest of three runs of `work`. Each comparison below sets two such measurements, taken in
/// one process moments apart, against each other: a ratio, not a wall-clock bound, so load moves
/// both sides together.
fn fastest(mut work: impl FnMut()) -> Duration {
    (0..3)
        .map(|_| {
            let started = Instant::now();
            work();
            started.elapsed()
        })
        .min()
        .expect("three runs")
}

/// Linear work grows four times when its input does; quadratic work grows sixteen times. Eight
/// lies between them with a factor of two to spare on either side.
const QUADRUPLED_INPUT_CEILING: f64 = 8.0;

fn grows_linearly(name: &str, small: Duration, large: Duration) {
    let ratio = large.as_secs_f64() / small.as_secs_f64().max(1e-6);
    assert!(
        ratio < QUADRUPLED_INPUT_CEILING,
        "{name}: four times the input took {ratio:.1} times as long ({small:?} -> {large:?})"
    );
}

/// The literal scan of one line is linear in the line's length. An opening quote with no
/// unescaped partner scans to the end of its line, and the scan for openings does not skip
/// escaped quotes, so a line of `n` escaped quotes costs `n` scans of the line.
#[test]
fn scanning_a_long_line_of_escaped_quotes_is_linear() {
    let line = |quotes: usize| format!("data = '{}';\n", "\\\"".repeat(quotes));
    let (short, long) = (line(12_500), line(50_000));
    assert_eq!(
        ekr_views::literals(&long).len(),
        1,
        "only the single-quoted literal"
    );
    let small = fastest(|| assert_eq!(ekr_views::literals(&short).len(), 1));
    let large = fastest(|| assert_eq!(ekr_views::literals(&long).len(), 1));
    grows_linearly("a line of escaped quotes", small, large);
}

/// Many sources: the answer's cost is not quadratic in the number of files. Measured on the unit's
/// tree (debug build), before the fix: 5 000 sources 0.19 s, 10 000 0.71 s, 20 000 3.4 s, 40 000
/// 15.3 s — each doubling costs four times, from the path dedup in `code_names`, which compared
/// every source with every one kept before it.
#[test]
fn many_sources_answer_in_time_linear_in_their_number() {
    let (_work, runtime) = built();
    let loaded = ekr_views::load(&runtime, None).expect("the head loads");
    let sources = |count: usize| -> Vec<SourceText> {
        (0..count)
            .map(|n| {
                source(
                    &format!("src/module_{n:06}/reader.ts"),
                    "const kind = \"Subject\";\nconst other = 'nothing';\n",
                )
            })
            .collect()
    };
    let (few, many) = (sources(10_000), sources(40_000));
    let answer = ekr_views::code_names(&loaded, &many).expect("an answer");
    assert_eq!(answer.summary.files, 40_000);
    assert_eq!(answer.summary.findings, 40_000);
    let small = fastest(|| {
        ekr_views::code_names(&loaded, &few).expect("an answer");
    });
    let large = fastest(|| {
        ekr_views::code_names(&loaded, &many).expect("an answer");
    });
    grows_linearly("code names over many sources", small, large);
}
