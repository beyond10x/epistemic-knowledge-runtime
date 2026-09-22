//! Adversary, unit p1-12-vectors. Seed-input mutations beyond the implementor's list in
//! `current_root_sensitivity.rs`, through the real kernel on both providers: each moves exactly
//! the sub-roots § 91.2 says it feeds, and the revision root.
mod current_fixture;

use std::collections::BTreeSet;

use current_fixture::{anchor, context, id, seed, seeded, SEEDED_AT, STATEMENT};
use ekr_core::{ContentHash, Timestamp};
use ekr_graph::{EvidenceSource, Root};
use ekr_kernel::SeedDocument;
use ekr_ontology::{NodeType, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Sub {
    Ontology,
    Knowledge,
    Evidence,
    Agent,
    Transaction,
}

fn moved(base: &Root, changed: &Root) -> (BTreeSet<Sub>, bool) {
    let pairs = [
        (Sub::Ontology, base.ontology_root, changed.ontology_root),
        (Sub::Knowledge, base.knowledge_root, changed.knowledge_root),
        (Sub::Evidence, base.evidence_root, changed.evidence_root),
        (Sub::Agent, base.agent_root, changed.agent_root),
        (Sub::Transaction, base.transaction, changed.transaction),
    ];
    (
        pairs
            .into_iter()
            .filter(|(_, a, b)| a != b)
            .map(|(which, _, _)| which)
            .collect(),
        ContentHash::of(base) != ContentHash::of(changed),
    )
}

type Change = Box<dyn Fn(&mut SeedDocument)>;

#[test]
fn further_seed_inputs_move_exactly_the_sub_roots_they_feed_on_both_providers() {
    use Sub::{Evidence, Knowledge, Ontology, Transaction};
    let evidence = id::<ekr_core::EvidenceId>(0x13);
    let first = id::<ekr_core::NodeId>(0x10);
    let cases: Vec<(&str, Vec<Sub>, Change)> = vec![
        (
            "graph root created_at",
            vec![Transaction],
            Box::new(|s| s.graph.root.created_at = Timestamp::from_millis(3)),
        ),
        (
            "unused ontology node declaration",
            vec![Ontology, Transaction],
            Box::new(|s| {
                s.ontology
                    .node_types
                    .push(NodeType::new(id(0x7e), "Unused"))
            }),
        ),
        (
            "evidence source identity",
            vec![Evidence, Transaction],
            Box::new(move |s| {
                s.graph.evidence.get_mut(&evidence).unwrap().source =
                    EvidenceSource::HumanStatement { identity: None };
            }),
        ),
        (
            "evidence payload bytes and address",
            vec![Evidence, Transaction],
            Box::new(move |s| {
                let other = b"a different synthetic statement".to_vec();
                let hash = ContentHash::of_bytes(&other);
                s.evidence_payloads.clear();
                s.evidence_payloads.insert(hash, other);
                s.graph.evidence.get_mut(&evidence).unwrap().content_hash = hash;
            }),
        ),
        (
            "node alias",
            vec![Knowledge, Transaction],
            Box::new(move |s| s.graph.nodes.get_mut(&first).unwrap().aliases = vec!["one".into()]),
        ),
        (
            "assertion object and property value together",
            vec![Knowledge, Transaction],
            Box::new(move |s| {
                s.graph.nodes.get_mut(&first).unwrap().properties =
                    [(id(0x06), vec![Value::String("omega".into())])]
                        .into_iter()
                        .collect();
                s.graph.assertions.get_mut(&id(0x14)).unwrap().object =
                    ekr_graph::Object::Value(Value::String("omega".into()));
            }),
        ),
    ];
    assert!(!STATEMENT.is_empty());
    let mut wrong = Vec::new();
    for file in [false, true] {
        let (_d, _r, base) = seeded(seed(), context(), anchor(), file, SEEDED_AT);
        for (name, feeds, change) in &cases {
            let mut document = seed();
            change(&mut document);
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                seeded(document, context(), anchor(), file, SEEDED_AT)
                    .2
                    .result
            }));
            let Ok(root) = outcome else {
                wrong.push(format!("file={file} {name}: seed refused"));
                continue;
            };
            let expected: BTreeSet<Sub> = feeds.iter().copied().collect();
            let observed = moved(&base.result, &root);
            if observed != (expected.clone(), true) {
                wrong.push(format!(
                    "file={file} {name}: expected {expected:?}, observed {observed:?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
