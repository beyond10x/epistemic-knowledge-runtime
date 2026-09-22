//! Adversary, unit p1-12-vectors. § 89 compatibility table and § 91.2, read as the specification
//! of the retained `ekr-seed-envelope/2` the unit pins only by hash and key order:
//!
//! * SeedEnvelope/2 has exactly format, input, context, authority and committed_at;
//! * input is the complete Seed/2: format, ontology, graph, evidence_payloads;
//! * "A seed's graph field holds that complete envelope, not its inner graph":
//!   `{format: "ekr.graph-document/2", graph: <graph fields>}`.
mod current_fixture;

use current_fixture::{anchor, context, seed, seeded, SEEDED_AT};

fn keys(value: &serde_json::Value) -> Vec<String> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("not an object: {value}"))
        .keys()
        .cloned()
        .collect()
}

#[test]
fn the_retained_seed_envelope_nests_the_documented_layers_on_both_providers() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let (_directory, runtime, result) = seeded(seed(), context(), anchor(), file, SEEDED_AT);
        let bytes = runtime.content(&result.seed_hash).unwrap().unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let mut outer = keys(&envelope);
        outer.sort();
        if outer != ["authority", "committed_at", "context", "format", "input"] {
            wrong.push(format!("file={file} envelope keys {outer:?}"));
        }
        let input = &envelope["input"];
        let mut inner = keys(input);
        inner.sort();
        if inner != ["evidence_payloads", "format", "graph", "ontology"] {
            wrong.push(format!("file={file} input keys {inner:?}"));
        }
        let graph = &input["graph"];
        if graph["format"] != "ekr.graph-document/2" {
            wrong.push(format!(
                "file={file} input.graph.format {}",
                graph["format"]
            ));
        }
        let mut layers = keys(graph);
        layers.sort();
        if layers != ["format", "graph"] {
            wrong.push(format!("file={file} input.graph keys {layers:?}"));
        }
        let mut context_keys = keys(&envelope["context"]);
        context_keys.sort();
        if context_keys != ["operator", "validator"] {
            wrong.push(format!("file={file} context keys {context_keys:?}"));
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
