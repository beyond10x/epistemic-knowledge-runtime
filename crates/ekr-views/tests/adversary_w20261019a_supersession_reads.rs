//! Adversary pass 1, wave 2026-10-19a, `story:supersede-node`: the head reads the unit's own
//! `supersession.rs` does not call, over the same `superseded` fixture, on both providers.
//!
//! The fixture's head holds an active assertion about beta, the superseded node
//! (`SUPERSEDED_CLAIM`), as ADR 0017 decision 3 keeps it. That decision also says what a read
//! that reaches such an assertion from elsewhere answers: "a read that reaches it from elsewhere
//! names the retired id with its pointer".

mod support;

use ekr_core::RevisionNumber;
use ekr_views::{ChangesRequest, SampleRequest, SinceKind, SourceText};
use serde_json::Value;

use support::fixtures::{self, Fixture, Provider, BETA_NODE, SUPERSEDED_KEPT};

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn each(mut run: impl FnMut(&str, &ekr_kernel::Runtime)) {
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), provider);
        Fixture::Superseded.build(&runtime);
        run(provider.name(), &runtime);
    }
}

/// Every other head read answers after a supersession, and the ones that list nodes list no
/// beta: the OCEL export, the process map, the quality report, code names, the changes since the
/// seed.
#[test]
fn adv_every_other_head_read_answers_after_a_supersession() {
    each(|provider, runtime| {
        let beta = uuid(BETA_NODE);
        let ocel = ekr_views::export_ocel(runtime, None, &[]).expect("the OCEL export answers");
        let text = String::from_utf8_lossy(&ocel.bytes).into_owned();
        assert!(!text.contains(&beta), "{provider}: OCEL names beta: {text}");
        ekr_views::export_process_map(runtime, None, &[], &[]).expect("the process map answers");
        ekr_views::report_quality(runtime, None).expect("the quality report answers");
        let loaded = ekr_views::load(runtime, None).expect("the head loads");
        let found = ekr_views::code_names(
            &loaded,
            &[SourceText {
                path: "notes.txt".into(),
                text: "beta and beta-alias-a and beta, kept".into(),
            }],
        )
        .expect("code names answer");
        assert!(
            !String::from_utf8_lossy(&found.bytes).contains(&beta),
            "{provider}: code names name beta: {}",
            String::from_utf8_lossy(&found.bytes)
        );
        let head = ekr_views::Index::load(runtime, None).expect("the head indexes");
        head.changes(
            runtime,
            &ChangesRequest::new(SinceKind::Revision, 0, None, None).expect("bounds"),
        )
        .expect("the changes answer");
        let earlier =
            ekr_views::Index::load(runtime, Some(RevisionNumber::new(1))).expect("revision 1");
        earlier
            .changes(
                runtime,
                &ChangesRequest::new(SinceKind::Revision, 0, None, None).expect("bounds"),
            )
            .expect("the changes at revision 1 answer");
    });
}

/// ADR 0017 decision 3: a fact sample at the head reaches the active assertion about beta from
/// the assertion population, not through beta, so it names beta's id with its pointer: the node
/// beta is superseded by.
#[test]
fn adv_a_sampled_fact_about_a_superseded_node_names_the_node_it_is_superseded_by() {
    each(|provider, runtime| {
        let beta = uuid(BETA_NODE);
        let drawn = ekr_views::draw_sample(
            runtime,
            None,
            &SampleRequest::new(7, 100, None).expect("bounds"),
        )
        .expect("the sample answers");
        let document: Value = serde_json::from_slice(&drawn.bytes).expect("JSON");
        let items = document["items"].as_array().expect("items");
        let about_beta: Vec<&Value> = items
            .iter()
            .filter(|item| item["subject"] == Value::String(beta.clone()))
            .collect();
        assert_eq!(
            about_beta.len(),
            1,
            "{provider}: the head's sample holds the assertion about beta: {document}"
        );
        assert!(
            about_beta[0].to_string().contains(&uuid(SUPERSEDED_KEPT)),
            "{provider}: the sampled fact names beta without the node it is superseded by: {}",
            about_beta[0]
        );
    });
}
