//! Adversarial pass on `story:extraction-verb-shares-the-sdk-path` (wave extract-07, unit V).
//!
//! `ekr apply-extraction` and the SDK's `extraction::apply` over a child `ekr session` are held to
//! the same store for documents the engine's reader accepts; the SDK's mirror of
//! `ekr.extraction-document/1` is held to the reader's decode refusals; and the routine is held to
//! what a second run of one document and a reordering of its named things should leave.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Output;

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{AgentId, ExtractionDocument};
use ekr_sdk::extraction::apply;
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use serde_json::Value;

const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";

/// Item 0403's entry and bytes, as the unit's own fixture writes them.
const EVIDENCE: &str = "evidence:
- evidence:
    id: 00000000-0000-4000-8000-000000000403
    source: !HumanStatement
      identity: Quarterly report
    content_hash: 7afeb9c852d895a2cdf8d7717a49354badd8b8dc79049925137b1f95f4651af5
    extracted_by: 00000000-0000-4000-8000-000000000101
    observed_at: 1773273600000
    confidence: 10000
  payload: [67, 97, 114, 111, 108, 44, 32, 67, 69, 79, 32, 111, 102, 32, 71, 108, 111, 98, 101, 120, 32, 67, 111, 114, 112, 111, 114, 97, 116, 105, 111, 110, 44, 32, 108, 101, 97, 100, 115, 32, 112, 114, 111, 106, 101, 99, 116, 32, 65, 112, 111, 108, 108, 111, 46]
";

/// The unit's own fixture: Carol and Globex are new, both facts cite item 0403.
fn known_types() -> String {
    format!(
        "format: ekr.extraction-document/1
entities:
- node_type: Person
  aliases: [Carol]
facts:
- !Property
  subject: {{node_type: Organization, aliases: [Globex]}}
  property: legal_name
  value: {{value_kind: String, value: Globex Corporation}}
  evidence: [00000000-0000-4000-8000-000000000403]
- !Relation
  subject: {{node_type: Person, aliases: [Carol]}}
  relation: CEO_OF
  object: {{node_type: Organization, aliases: [Globex]}}
  evidence: [00000000-0000-4000-8000-000000000403]
{EVIDENCE}"
    )
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ekr"))
}

fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(binary());
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

struct World {
    directory: tempfile::TempDir,
}

impl World {
    /// The example host (v2, so a schema change is admitted) and the example seed, File provider.
    fn new() -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        let host = text(&["example", "ekr.cli-host/1"])
            .replace("\"ekr.p1-deterministic/1\"", "\"ekr.p2-deterministic/1\"")
            .replace("\"ekr.p1-apply/1\"", "\"ekr.p2-apply/1\"");
        std::fs::write(world.file("host.json"), host).unwrap();
        std::fs::write(world.file("seed.yaml"), text(&["example", "ekr-seed/2"])).unwrap();
        let seed = world.file("seed.yaml");
        world.json(&["seed", seed.to_str().unwrap()]);
        world
    }

    fn file(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn config(&self) -> StoreConfig {
        StoreConfig {
            host: self.file("host.json"),
            store: self.file("store"),
            backend: Backend::File,
        }
    }

    fn output(&self, args: &[&str]) -> Output {
        ekr()
            .arg("--host")
            .arg(self.file("host.json"))
            .arg("--store")
            .arg(self.file("store"))
            .args(["--backend", "file"])
            .args(args)
            .output()
            .unwrap()
    }

    fn json(&self, args: &[&str]) -> Value {
        let output = self.output(args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn verb(&self, document: &str) -> Value {
        let path = self.file("extraction.yaml");
        std::fs::write(&path, document).unwrap();
        self.json(&["apply-extraction", path.to_str().unwrap()])
    }

    fn sdk(&self, document: &str) -> Value {
        let binary = EkrBinary::open(binary()).unwrap();
        let mut session =
            ProcessSession::start(&binary, self.config(), SessionOptions::default()).unwrap();
        let document = ExtractionDocument::from_yaml(document).unwrap();
        let report = apply(
            &mut session,
            &document,
            OPERATOR.parse::<AgentId>().unwrap(),
        )
        .unwrap_or_else(|error| panic!("the SDK's apply failed: {error}"));
        session.close().unwrap();
        serde_json::to_value(report).unwrap()
    }

    /// The graph with node, type and property ids replaced by names; assertion and transaction
    /// ids and times dropped.
    fn content(&self) -> Value {
        let snapshot = self.json(&["snapshot"]);
        let graph = &snapshot["graph"]["graph"];
        let mut names: BTreeMap<String, String> = graph["nodes"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(id, node)| {
                (
                    id.clone(),
                    format!("node {}", node["canonical_name"].as_str().unwrap()),
                )
            })
            .collect();
        let ontology = self.json(&["ontology"]);
        for list in ["node_types", "edge_types"] {
            for declared in ontology[list].as_array().unwrap() {
                let name = declared["name"].as_str().unwrap();
                names.insert(
                    declared["id"].as_str().unwrap().to_owned(),
                    format!("type {name}"),
                );
                for property in declared["properties"].as_array().unwrap() {
                    names.insert(
                        property["id"].as_str().unwrap().to_owned(),
                        format!("property {name}.{}", property["name"].as_str().unwrap()),
                    );
                }
            }
        }
        let mut nodes: Vec<Value> = graph["nodes"]
            .as_object()
            .unwrap()
            .values()
            .map(|node| {
                let mut node = node.clone();
                node.as_object_mut().unwrap().remove("id");
                renamed(&node, &names)
            })
            .collect();
        nodes.sort_by_key(ToString::to_string);
        let mut assertions: Vec<Value> = graph["assertions"]
            .as_object()
            .unwrap()
            .values()
            .map(|assertion| {
                let mut said = assertion.clone();
                let fields = said.as_object_mut().unwrap();
                fields.remove("id");
                fields.remove("transaction_time");
                fields.remove("assessment");
                renamed(&said, &names)
            })
            .collect();
        assertions.sort_by_key(ToString::to_string);
        serde_json::json!({
            "nodes": nodes,
            "assertions": assertions,
            "evidence": graph["evidence"],
        })
    }
}

fn renamed(value: &Value, names: &BTreeMap<String, String>) -> Value {
    match value {
        Value::String(text) => names
            .get(text)
            .map_or_else(|| value.clone(), |name| Value::String(name.clone())),
        Value::Array(items) => Value::Array(items.iter().map(|v| renamed(v, names)).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, v)| (names.get(key).unwrap_or(key).clone(), renamed(v, names)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// A report with its ids dropped: how many commits, each rejected item with its issue codes and
/// refusal, each ambiguous reference with its candidate count.
fn shape(report: &Value) -> Value {
    serde_json::json!({
        "committed": report["committed"].as_array().unwrap().len(),
        "rejected": report["rejected"].as_array().unwrap().iter().map(|row| serde_json::json!({
            "item": row["item"],
            "codes": row["issues"].as_array().unwrap().iter().map(|i| i["code"].clone()).collect::<Vec<_>>(),
            "refusal": row["refusal"],
        })).collect::<Vec<_>>(),
        "ambiguous": report["ambiguous"].as_array().unwrap().iter().map(|row| serde_json::json!({
            "reference": row["reference"],
            "candidates": row["candidates"].as_array().unwrap().len(),
        })).collect::<Vec<_>>(),
    })
}

/// `setup` through the verb on two stores, then `document` through the verb on one and the SDK
/// over a child session on the other: the reports and the stores agree.
fn agree(setup: Option<&str>, document: &str) -> (Value, Value) {
    let (by_verb, by_sdk) = (World::new(), World::new());
    if let Some(setup) = setup {
        by_verb.verb(setup);
        by_sdk.verb(setup);
    }
    let (verb, sdk) = (by_verb.verb(document), by_sdk.sdk(document));
    assert_eq!(shape(&verb), shape(&sdk), "reports differ\n{verb}\n{sdk}");
    let (left, right) = (by_verb.content(), by_sdk.content());
    assert_eq!(left, right, "stores differ");
    (verb, left)
}

/// Dana and Dee are two Persons; a thing known by both is ambiguous.
#[test]
fn adv_agree_on_an_ambiguous_reference() {
    let setup = known_types().replace(
        "entities:\n- node_type: Person\n  aliases: [Carol]\n",
        "entities:\n- node_type: Person\n  aliases: [Dana]\n- node_type: Person\n  aliases: [Dee]\n",
    );
    let (report, _) = agree(
        Some(&setup),
        &known_types().replace("aliases: [Carol]", "aliases: [Dana, Dee]"),
    );
    assert_eq!(report["ambiguous"].as_array().unwrap().len(), 1, "{report}");
}

/// Aliases shared across named things, in an order that flushes the resolver part-way.
#[test]
fn adv_agree_on_aliases_shared_across_named_things() {
    let document = known_types().replace(
        "entities:\n- node_type: Person\n  aliases: [Carol]\n",
        "entities:\n- node_type: Person\n  aliases: [Carol]\n- node_type: Person\n  aliases: \
         [Carol, CJ]\n- node_type: Person\n  aliases: [CJ]\n- node_type: Organization\n  \
         aliases: [Globex, Globex Corp]\n",
    );
    agree(None, &document);
}

/// One evidence item cited by several facts, and one fact citing it twice.
#[test]
fn adv_agree_on_evidence_reused_and_cited_twice() {
    let document = known_types().replacen(
        "evidence: [00000000-0000-4000-8000-000000000403]",
        "evidence: [00000000-0000-4000-8000-000000000403, 00000000-0000-4000-8000-000000000403]",
        1,
    );
    let (report, content) = agree(None, &document);
    assert_eq!(report["rejected"], serde_json::json!([]), "{report}");
    assert_eq!(
        content["evidence"].as_object().unwrap().len(),
        3,
        "{content}"
    );
    let globex = content["assertions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|assertion| assertion["predicate"]["Property"] == "property Organization.legal_name")
        .unwrap_or_else(|| panic!("{content}"));
    assert_eq!(
        globex["evidence"],
        serde_json::json!(["00000000-0000-4000-8000-000000000403"]),
        "{globex}"
    );
}

/// A document with nothing in it commits nothing on either path.
#[test]
fn adv_agree_on_an_empty_document() {
    let (report, _) = agree(
        None,
        "format: ekr.extraction-document/1\nfacts: []\nevidence: []\n",
    );
    assert_eq!(
        report,
        serde_json::json!({
            "committed": [], "rejected": [], "ambiguous": [], "held": [], "stopped": null
        })
    );
}

/// A document the kernel refuses half of: the schema change commits, one fact names a node no
/// graph holds and is rejected alone, the facts around it commit.
#[test]
fn adv_agree_on_a_document_that_fails_halfway() {
    let document = format!(
        "format: ekr.extraction-document/1
ontology:
  node_types:
  - name: Project
    properties:
    - name: owner
      value: {{value_kind: NodeRef, parameters: {{allowed_types: [Person]}}}}
facts:
- !Property
  subject: {{node_type: Organization, aliases: [Globex]}}
  property: legal_name
  value: {{value_kind: String, value: Globex Corporation}}
  evidence: [00000000-0000-4000-8000-000000000403]
- !Property
  subject: {{node_type: Project, aliases: [Apollo]}}
  property: owner
  value: {{value_kind: NodeRef, value: 00000000-0000-4000-8000-0000000009ff}}
  evidence: [00000000-0000-4000-8000-000000000403]
- !Property
  subject: {{node_type: Organization, aliases: [Globex]}}
  property: legal_name
  value: {{value_kind: String, value: Globex Inc}}
  evidence: [00000000-0000-4000-8000-000000000403]
{EVIDENCE}"
    );
    let (report, _) = agree(None, &document);
    let rejected = report["rejected"].as_array().unwrap();
    assert_eq!(rejected.len(), 1, "{report}");
    assert_eq!(rejected[0]["item"], "facts[1]", "{report}");
}

/// Applying one document a second time adds nothing: every named thing resolves to the node the
/// first run created and every evidence item is already held, so every fact is one the store
/// already asserts with the same evidence.
#[test]
fn adv_a_second_run_of_one_document_adds_no_assertion() {
    let world = World::new();
    world.verb(&known_types());
    let once = world.content();
    let again = world.verb(&known_types());
    assert_eq!(
        world.content()["assertions"],
        once["assertions"],
        "the second run committed {}",
        again["committed"]
    );
}

/// The same named things in another order leave the same store: a thing known as Carol and CJ is
/// one Person whichever of [Carol], [Carol, CJ] and [CJ] the document lists first.
#[test]
fn adv_the_order_of_named_things_does_not_change_the_nodes() {
    let persons = |entities: &str| {
        let world = World::new();
        world.verb(&format!(
            "format: ekr.extraction-document/1\nentities:\n{entities}facts: []\nevidence: []\n"
        ));
        let mut names: Vec<String> = world.content()["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| node["canonical_name"].as_str().unwrap().to_owned())
            .collect();
        names.sort();
        names
    };
    let first = persons(
        "- node_type: Person\n  aliases: [Carol, CJ]\n- node_type: Person\n  aliases: [Carol]\n- \
         node_type: Person\n  aliases: [CJ]\n",
    );
    let second = persons(
        "- node_type: Person\n  aliases: [Carol]\n- node_type: Person\n  aliases: [Carol, CJ]\n- \
         node_type: Person\n  aliases: [CJ]\n",
    );
    assert_eq!(first, second);
}

/// A document the reader accepts — it declares a subtype of a type the store holds, and names a
/// thing of the held type — is either applied with a report (exit 0) or refused before anything
/// is written (exit 2). It is not a fault after the schema change committed.
#[test]
fn adv_a_document_the_reader_accepts_is_reported_not_faulted_halfway() {
    let world = World::new();
    let document = known_types().replace(
        "entities:",
        "ontology:\n  node_types:\n  - name: Employee\n    parents: [Person]\nentities:",
    );
    let path = world.file("extraction.yaml");
    std::fs::write(&path, &document).unwrap();
    let output = world.output(&["apply-extraction", path.to_str().unwrap()]);
    let head = world.json(&["head"])["revision"].clone();
    let code = output.status.code();
    assert!(
        code == Some(0) || (code == Some(2) && head == 0),
        "exit {code:?}, head {head}, stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The SDK's mirror refuses every document the engine's reader refuses while decoding — the
/// shape, which the mirror says it checks.
fn mirror_refuses(document: &str, code: &str) {
    let reader = ekr_integrate::ExtractionDocument::from_yaml(document)
        .expect_err("the reader refuses the document");
    assert_eq!(reader.refusal().code.code(), code, "{reader}");
    let mirror = ExtractionDocument::from_yaml(document);
    assert!(
        mirror.is_err(),
        "the reader refuses as {code}, the mirror reads it, entities: {:?}",
        mirror.map(|document| document.entities.len())
    );
}

#[test]
fn adv_mirror_refuses_a_yaml_alias() {
    mirror_refuses(
        "format: ekr.extraction-document/1\nentities:\n- node_type: Person\n  aliases: [&name \
         Carol]\n- node_type: Person\n  aliases: [*name]\nfacts: []\nevidence: []\n",
        "extraction-yaml-alias",
    );
}

#[test]
fn adv_mirror_refuses_an_alias_that_is_not_a_string() {
    mirror_refuses(
        "format: ekr.extraction-document/1\nentities:\n- node_type: Person\n  aliases: \
         [2024]\nfacts: []\nevidence: []\n",
        "extraction-document-malformed",
    );
}

#[test]
fn adv_mirror_refuses_a_document_over_the_cap() {
    let long = "C".repeat(ekr_integrate::EXTRACTION_INPUT_BYTES);
    mirror_refuses(
        &format!(
            "format: ekr.extraction-document/1\nentities:\n- node_type: Person\n  aliases: \
             [{long}]\nfacts: []\nevidence: []\n"
        ),
        "extraction-document-too-large",
    );
}

#[test]
fn adv_mirror_refuses_a_document_past_the_depth_bound() {
    // A List of Lists: each level is a mapping and a sequence.
    let mut value = "{value_kind: String, value: x}".to_owned();
    for _ in 0..20 {
        value = format!("{{value_kind: List, value: [{value}]}}");
    }
    mirror_refuses(
        &format!(
            "format: ekr.extraction-document/1\nfacts:\n- !Property\n  subject: {{node_type: \
             Person, aliases: [Carol]}}\n  property: tags\n  value: {value}\n  evidence: \
             [00000000-0000-4000-8000-000000000403]\nevidence: []\n"
        ),
        "extraction-document-too-deep",
    );
}

#[test]
fn adv_mirror_refuses_a_mapping_key_written_twice() {
    mirror_refuses(
        "format: ekr.extraction-document/1\nentities:\n- node_type: Person\n  node_type: \
         Organization\n  aliases: [Carol]\nfacts: []\nevidence: []\n",
        "extraction-document-malformed",
    );
}

/// Over a child session, a fact citing no evidence — which the reader refuses as
/// `fact-without-evidence` and the verb therefore never writes — is not committed either: the
/// module says the kernel's validators refuse what the store cannot take, and every canonical
/// assertion carries evidence.
#[test]
fn adv_sdk_over_a_session_commits_no_fact_without_evidence() {
    let document = known_types().replacen(
        "evidence: [00000000-0000-4000-8000-000000000403]",
        "evidence: []",
        1,
    );
    let world = World::new();
    let before = world.content();
    let report = world.sdk(&document);
    let after = world.content();
    let unevidenced: Vec<&Value> = after["assertions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|assertion| assertion["evidence"] == serde_json::json!([]))
        .collect();
    assert!(
        unevidenced.is_empty(),
        "committed with no evidence: {unevidenced:?}\nreport {report}\nbefore {}",
        before["assertions"]
    );
}

/// Over a child session, two evidence items under one id — `duplicate-identity` to the reader —
/// are not silently resolved to whichever the document lists last. Decided (wave correct-07):
/// the SDK's mirror refuses the document by the reader's code, and a document built with two
/// items under one id by hand is refused by the routine before it writes anything.
#[test]
fn adv_sdk_over_a_session_does_not_pick_one_of_two_items_under_one_id() {
    let second = "- evidence:
    id: 00000000-0000-4000-8000-000000000403
    source: !HumanStatement
      identity: Another report
    content_hash: 7afeb9c852d895a2cdf8d7717a49354badd8b8dc79049925137b1f95f4651af5
    extracted_by: 00000000-0000-4000-8000-000000000101
    observed_at: 1773273600001
    confidence: 5000
  payload: [67, 97, 114, 111, 108, 44, 32, 67, 69, 79, 32, 111, 102, 32, 71, 108, 111, 98, 101, 120, 32, 67, 111, 114, 112, 111, 114, 97, 116, 105, 111, 110, 44, 32, 108, 101, 97, 100, 115, 32, 112, 114, 111, 106, 101, 99, 116, 32, 65, 112, 111, 108, 108, 111, 46]
";
    let document = format!("{}{second}", known_types());
    let reader = ekr_integrate::ExtractionDocument::from_yaml(&document).unwrap();
    assert_eq!(reader.evidence.len(), 2);
    let refused = ExtractionDocument::from_yaml(&document).unwrap_err();
    assert_eq!(refused.code, "duplicate-identity", "{refused}");

    let mut built = ExtractionDocument::from_yaml(&known_types()).unwrap();
    let twin = built.evidence[0].clone();
    built.evidence.push(twin);
    let world = World::new();
    let before = world.content();
    let binary = EkrBinary::open(binary()).unwrap();
    let mut session =
        ProcessSession::start(&binary, world.config(), SessionOptions::default()).unwrap();
    match apply(&mut session, &built, OPERATOR.parse::<AgentId>().unwrap()) {
        Err(ekr_sdk::extraction::ApplyError::Refused(refusal)) => {
            assert_eq!(refusal.code, "duplicate-identity", "{refusal}");
        }
        other => panic!("two items under one id are refused before any write: {other:?}"),
    }
    session.close().unwrap();
    let after = world.content();
    assert_eq!(after, before);
    assert!(after["evidence"]["00000000-0000-4000-8000-000000000403"].is_null());
}
