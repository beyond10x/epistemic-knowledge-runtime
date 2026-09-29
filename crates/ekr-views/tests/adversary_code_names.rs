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
#[ignore = "defect: a quote of another kind before a quoted store name hides it (views.yaml CodeNamesV1 literal rule)"]
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
#[ignore = "defect (judgement): a store name that is also a runtime vocabulary word is never reported"]
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

/// The literal scan of one line is linear in the line's length. An opening quote with no
/// unescaped partner scans to the end of its line, and the scan for openings does not skip
/// escaped quotes, so a line of `n` escaped quotes costs `n` scans of the line.
#[test]
#[ignore = "defect (note): literals() is quadratic in a line of escaped quotes with no unescaped partner"]
fn scanning_a_long_line_of_escaped_quotes_is_linear() {
    let line = format!("data = '{}';\n", "\\\"".repeat(50_000));
    let started = Instant::now();
    let literals = ekr_views::literals(&line);
    let elapsed = started.elapsed();
    assert_eq!(literals.len(), 1, "only the single-quoted literal");
    assert!(
        elapsed < Duration::from_secs(1),
        "a 100 000-character line took {elapsed:?}"
    );
}

/// Many sources: the answer's cost is not quadratic in the number of files. Measured on the unit's
/// tree (debug build): 5 000 sources 0.19 s, 10 000 0.71 s, 20 000 3.4 s, 40 000 15.3 s — each
/// doubling costs four times, from the path dedup in `code_names`, which compares every source
/// with every one kept before it.
#[test]
#[ignore = "defect (note): code_names() is quadratic in the number of sources (path dedup)"]
fn forty_thousand_sources_answer_in_bounded_time() {
    let (_work, runtime) = built();
    let loaded = ekr_views::load(&runtime, None).expect("the head loads");
    let sources: Vec<SourceText> = (0..40_000)
        .map(|n| {
            source(
                &format!("src/module_{n:06}/reader.ts"),
                "const kind = \"Subject\";\nconst other = 'nothing';\n",
            )
        })
        .collect();
    let started = Instant::now();
    let answer = ekr_views::code_names(&loaded, &sources).expect("an answer");
    let elapsed = started.elapsed();
    assert_eq!(answer.summary.files, 40_000);
    assert_eq!(answer.summary.findings, 40_000);
    assert!(
        elapsed < Duration::from_secs(5),
        "40 000 sources took {elapsed:?}"
    );
}
