//! `story:extraction-verb-shares-the-sdk-path`: `ekr apply-extraction` runs the SDK's apply
//! routine in process, and writes what that routine writes over a child `ekr session`.
//!
//! Every case drives the built binary, and the SDK case a real child session of it. Stores are
//! compared with their ids ignored: a node by its type, name and aliases, an assertion by what it
//! says of whom, an evidence entry whole and by the bytes `ekr explain --documents` returns.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Output;

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{
    AgentId, ExtractedFact, ExtractedReference, ExtractionDocument, ExtractionRefusal,
    PropertyFact, RelationFact, ValueSpec,
};
use ekr_sdk::extraction::{
    apply, AmbiguousExtraction, ApplyError, CommittedExtraction, ExtractionIssue, ExtractionReport,
    RejectedExtraction,
};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";

/// A document for the example seed's own types: no ontology section, so a store under the example
/// host, whose profile fixes the schema, takes it. Carol is new; Globex is new; both facts cite
/// item 0403.
const KNOWN_TYPES: &str = "format: ekr.extraction-document/1
entities:
- node_type: Person
  aliases: [Carol]
facts:
- !Property
  subject: {node_type: Organization, aliases: [Globex]}
  property: legal_name
  value: {value_kind: String, value: Globex Corporation}
  evidence: [00000000-0000-4000-8000-000000000403]
- !Relation
  subject: {node_type: Person, aliases: [Carol]}
  relation: CEO_OF
  object: {node_type: Organization, aliases: [Globex]}
  evidence: [00000000-0000-4000-8000-000000000403]
evidence:
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

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ekr"))
}

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(binary());
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
        "{args:?}: stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// One provider in its own directory: the example host, v2 when asked, and the example seed.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn new(backend: &'static str, schema_evolving: bool) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        let mut host = text(&["example", "ekr.cli-host/1"]);
        if schema_evolving {
            host = host
                .replace("\"ekr.p1-deterministic/1\"", "\"ekr.p2-deterministic/1\"")
                .replace("\"ekr.p1-apply/1\"", "\"ekr.p2-apply/1\"");
        }
        std::fs::write(world.host(), host).unwrap();
        std::fs::write(world.file("seed.yaml"), text(&["example", "ekr-seed/2"])).unwrap();
        let seed = world.file("seed.yaml");
        world.store_json(&["seed", seed.to_str().unwrap()]);
        world
    }

    fn file(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn host(&self) -> PathBuf {
        self.file("host.json")
    }

    fn store(&self) -> PathBuf {
        self.file("store")
    }

    fn config(&self) -> StoreConfig {
        StoreConfig {
            host: self.host(),
            store: self.store(),
            backend: match self.backend {
                "file" => Backend::File,
                _ => Backend::Sqlite,
            },
        }
    }

    fn command(&self, args: &[&str]) -> std::process::Command {
        let mut command = ekr();
        command
            .arg("--host")
            .arg(self.host())
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(args);
        command
    }

    fn store_output(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    fn store_json(&self, args: &[&str]) -> Value {
        let output = self.store_output(args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?}: stderr {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// `ekr apply-extraction` of `document`, which must exit 0: the report.
    fn apply_verb(&self, document: &str) -> Value {
        let path = self.file("extraction.yaml");
        std::fs::write(&path, document).unwrap();
        self.store_json(&["apply-extraction", path.to_str().unwrap()])
    }

    /// The SDK's routine over a child `ekr session` of the same binary: the report as JSON.
    fn apply_sdk(&self, document: &str) -> Value {
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

    /// The store with its ids ignored: nodes by type, name and aliases; active assertions by what
    /// they say of whom, each with the bytes of the evidence it cites; evidence entries whole.
    fn content(&self) -> Value {
        let snapshot = self.store_json(&["snapshot"]);
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
        // Types and properties a schema change added are minted by whoever applied it.
        let ontology = self.store_json(&["ontology"]);
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
            .iter()
            .map(|(id, assertion)| {
                let mut said = assertion.clone();
                let fields = said.as_object_mut().unwrap();
                fields.remove("id");
                fields.remove("transaction_time");
                fields.remove("assessment");
                let explained = self.store_json(&["explain", id, "--documents"]);
                let bytes: Vec<Value> = explained["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|link| link["kind"] == "Evidence")
                    .map(|link| link["payload"].clone())
                    .collect();
                fields.insert("evidence_bytes".to_owned(), Value::Array(bytes));
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

/// `value` with every node, type and property id replaced by its name, in values and keys.
fn renamed(value: &Value, names: &BTreeMap<String, String>) -> Value {
    match value {
        Value::String(text) => names
            .get(text)
            .map_or_else(|| value.clone(), |name| Value::String(name.clone())),
        Value::Array(items) => Value::Array(items.iter().map(|v| renamed(v, names)).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, v)| {
                    let key = names.get(key).unwrap_or(key).clone();
                    (key, renamed(v, names))
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

fn example_document() -> String {
    text(&["example", "ekr.extraction-document/1"])
}

/// Acceptance 1: one fixture, through the verb and through the SDK over a child session, gives
/// equal stores — the example document, which grows the schema, on a v2 store of each provider.
#[test]
fn the_verb_and_the_sdk_over_a_child_session_write_equal_stores() {
    let document = example_document();
    for backend in BACKENDS {
        let (verb, sdk) = (World::new(backend, true), World::new(backend, true));
        let by_verb = verb.apply_verb(&document);
        let by_sdk = sdk.apply_sdk(&document);
        for report in [&by_verb, &by_sdk] {
            assert_eq!(
                report["rejected"],
                serde_json::json!([]),
                "{backend}: {report}"
            );
            assert_eq!(
                report["ambiguous"],
                serde_json::json!([]),
                "{backend}: {report}"
            );
            // The schema change, the new nodes, the facts.
            assert_eq!(
                report["committed"].as_array().unwrap().len(),
                3,
                "{backend}: {report}"
            );
        }
        let (left, right) = (verb.content(), sdk.content());
        assert_eq!(left, right, "{backend}: the two stores differ");
        // Four facts, each one active assertion citing 0403's bytes.
        let cited = left["assertions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|assertion| assertion["evidence"].to_string().contains("000000000403"))
            .count();
        assert_eq!(cited, 4, "{backend}: {left}");
        // Carol, Apollo and Globex were created, each named by its alias and carrying it.
        for name in ["Carol", "Apollo", "Globex"] {
            let created = left["nodes"].as_array().unwrap().iter().any(|node| {
                node["canonical_name"] == name && node["aliases"] == serde_json::json!([name])
            });
            assert!(created, "{backend}: {name} in {}", left["nodes"]);
        }
        let ontology = verb.store_json(&["ontology"]).to_string();
        assert!(ontology.contains("\"Project\"") && ontology.contains("\"LEADS\""));
    }
}

/// Acceptance 3: the verb starts no process — with an empty `PATH` it applies a fixture.
#[test]
fn the_verb_applies_a_fixture_with_an_empty_path() {
    for backend in BACKENDS {
        let world = World::new(backend, false);
        let path = world.file("extraction.yaml");
        std::fs::write(&path, KNOWN_TYPES).unwrap();
        let output = world
            .command(&["apply-extraction", path.to_str().unwrap()])
            .env_clear()
            .env("PATH", "")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["rejected"], serde_json::json!([]), "{report}");
        assert_eq!(report["committed"].as_array().unwrap().len(), 2, "{report}");
        assert_eq!(world.store_json(&["head"])["revision"], 2);
    }
}

/// A document the engine's reader refuses is refused by its code, exit 2, and nothing is written.
#[test]
fn a_document_the_reader_refuses_is_refused_by_name_and_writes_nothing() {
    for backend in BACKENDS {
        let world = World::new(backend, false);
        let path = world.file("extraction.yaml");
        std::fs::write(
            &path,
            KNOWN_TYPES.replace("node_type: Person", "node_type: Starship"),
        )
        .unwrap();
        let output = world.store_output(&["apply-extraction", path.to_str().unwrap()]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{backend}: {stderr}");
        assert!(
            stderr.starts_with("ekr: extraction-type-undeclared: "),
            "{backend}: {stderr}"
        );
        assert!(output.stdout.is_empty());
        assert_eq!(world.store_json(&["head"])["revision"], 0);
        assert_eq!(world.store_json(&["transactions"]), serde_json::json!([]));
    }
}

/// A session refuses the verb: it is a run of the verbs a session already serves.
#[test]
fn a_session_refuses_the_verb() {
    let world = World::new("file", false);
    let mut command = world.command(&["session"]);
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    let mut child = command.spawn().unwrap();
    {
        use std::io::Write as _;
        let mut stdin = child.stdin.take().unwrap();
        writeln!(
            stdin,
            "{{\"argv\": [\"apply-extraction\", \"-\"], \"stdin\": \"\"}}"
        )
        .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    let answer: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(answer["exit"], 2, "{answer}");
    assert!(
        answer["stderr"]
            .as_str()
            .unwrap()
            .starts_with("ekr: session-verb-refused: "),
        "{answer}"
    );
}

/// The SDK's mirror reads the document `ekr example` prints in its value-type form, and what it
/// writes back the engine's reader reads as the same document.
#[test]
fn the_sdk_mirror_reads_the_example_and_the_engine_reads_what_it_writes() {
    let example = example_document();
    let mirrored = ExtractionDocument::from_yaml(&example).unwrap();
    let project = mirrored
        .ontology
        .node_types
        .iter()
        .find(|node| node.name == "Project")
        .expect("the example declares Project");
    assert_eq!(
        project.properties[0].value,
        ValueSpec::Enum(vec!["active".to_owned(), "closed".to_owned()])
    );
    let written = mirrored.to_yaml().unwrap();
    assert_eq!(ExtractionDocument::from_yaml(&written).unwrap(), mirrored);
    assert_eq!(
        ekr_integrate::ExtractionDocument::from_yaml(&written).unwrap(),
        ekr_integrate::ExtractionDocument::from_yaml(&example).unwrap(),
        "{written}"
    );
}

/// A `NodeRef` names its node types in `parameters: {allowed_types: [...]}`, as the engine reads
/// it; the tuple form the SDK's own type has is not the document's.
#[test]
fn the_sdk_mirror_reads_a_node_ref_as_the_document_writes_it() {
    let document = |parameters: &str| {
        format!(
            "format: ekr.extraction-document/1\nontology:\n  node_types:\n  - name: Ticket\n    \
             properties:\n    - name: owner\n      value: {{value_kind: NodeRef, parameters: \
             {parameters}}}\nfacts: []\nevidence: []\n"
        )
    };
    let read = ExtractionDocument::from_yaml(&document("{allowed_types: [Person]}")).unwrap();
    assert_eq!(
        read.ontology.node_types[0].properties[0].value,
        ValueSpec::NodeRef(vec!["Person".to_owned()])
    );
    let written = read.to_yaml().unwrap();
    assert!(written.contains("allowed_types:"), "{written}");
    assert!(ekr_integrate::ExtractionDocument::from_yaml(&written).is_ok());
    assert!(ExtractionDocument::from_yaml(&document("[Person]")).is_err());
}

/// The fields `systems/ekr/domains/integrate.yaml` declares for `name`.
fn declared_fields(name: &str) -> std::collections::BTreeSet<String> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest).join("../../systems/ekr/domains/integrate.yaml");
    let domain: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let declared = domain["types"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|declared| declared["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("integrate.yaml declares no {name}"));
    declared["fields"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|field| field["name"].as_str().unwrap().to_owned())
        .collect()
}

#[track_caller]
fn keys(value: &Value) -> std::collections::BTreeSet<String> {
    let Some(fields) = value.as_object() else {
        panic!("not an object: {value}")
    };
    fields.keys().cloned().collect()
}

/// The report carries exactly the fields the domain declares for `ekr.integrate.ExtractionReport`
/// and each of its rows: a committed transaction, an ambiguous named thing — whose fact is not
/// applied — and a part validation rejected, with its transaction and issues.
#[test]
fn the_report_carries_the_fields_the_domain_declares() {
    let world = World::new("file", false);
    // Dana and Dee are two Persons, so a reference known by both names is ambiguous.
    let two = KNOWN_TYPES.replace(
        "entities:\n- node_type: Person\n  aliases: [Carol]\n",
        "entities:\n- node_type: Person\n  aliases: [Carol]\n- node_type: Person\n  aliases: \
         [Dana]\n- node_type: Person\n  aliases: [Dee]\n",
    );
    let first = world.apply_verb(&two);
    assert_eq!(first["rejected"], serde_json::json!([]), "{first}");
    let report = world.apply_verb(&KNOWN_TYPES.replace("aliases: [Carol]", "aliases: [Dana, Dee]"));
    assert_eq!(
        keys(&report),
        declared_fields("ekr.integrate.ExtractionReport")
    );
    assert_eq!(
        keys(&first["committed"][0]),
        declared_fields("ekr.integrate.CommittedExtraction"),
        "{first}"
    );
    let row = &report["ambiguous"][0];
    assert_eq!(
        keys(row),
        declared_fields("ekr.integrate.AmbiguousExtraction"),
        "{report}"
    );
    assert_eq!(row["candidates"].as_array().unwrap().len(), 2, "{report}");
    assert_eq!(
        row["reference"]["aliases"],
        serde_json::json!(["Dana", "Dee"])
    );
    assert_eq!(report["ambiguous"].as_array().unwrap().len(), 1, "{report}");
    // The relation about Dana or Dee was not applied; the property about Globex is held: the first
    // document asserted it with the same evidence, so nothing is committed.
    assert_eq!(report["rejected"], serde_json::json!([]), "{report}");
    assert_eq!(report["committed"], serde_json::json!([]), "{report}");
    assert_eq!(report["held"], serde_json::json!(["facts[0]"]), "{report}");
    assert_eq!(report["stopped"], Value::Null, "{report}");

    // The example grows the schema, which the example host's profile fixes: the schema change is
    // rejected by validation, and nothing after it is tried.
    let fixed = World::new("sqlite", false);
    let report = fixed.apply_verb(&example_document());
    assert_eq!(report["committed"], serde_json::json!([]), "{report}");
    let rejected = &report["rejected"][0];
    assert_eq!(
        keys(rejected),
        declared_fields("ekr.integrate.RejectedExtraction"),
        "{report}"
    );
    assert_eq!(rejected["item"], "ontology", "{report}");
    assert!(rejected["transaction_id"].is_string(), "{report}");
    assert_eq!(
        keys(&rejected["issues"][0]),
        declared_fields("ekr.integrate.ExtractionIssue"),
        "{report}"
    );
    assert_eq!(fixed.store_json(&["head"])["revision"], 0);
}

/// The SDK's routine over a child session, read through its own types: the mirror's format and
/// facts, a committed transaction, an ambiguous named thing, a rejected schema change with its
/// issues, a document the mirror does not read, and a name no declaration gives, which stops the
/// routine as `ApplyError::Undeclared` before it writes anything.
#[test]
fn the_sdk_routine_reports_through_its_own_types() {
    assert_eq!(
        ekr_sdk::document::EXTRACTION_FORMAT,
        ekr_integrate::EXTRACTION_FORMAT
    );
    let unread: ExtractionRefusal = ExtractionDocument::from_yaml(
        "format: ekr.extraction-document/0\nfacts: []\nevidence: []\n",
    )
    .unwrap_err();
    assert_eq!(unread.code, "extraction-document-malformed", "{unread}");

    let world = World::new("file", false);
    let binary = EkrBinary::open(binary()).unwrap();
    let mut session =
        ProcessSession::start(&binary, world.config(), SessionOptions::default()).unwrap();
    let operator = OPERATOR.parse::<AgentId>().unwrap();

    // Dana and Dee are two Persons, so a reference known by both names is ambiguous.
    let two = KNOWN_TYPES.replace(
        "entities:\n- node_type: Person\n  aliases: [Carol]\n",
        "entities:\n- node_type: Person\n  aliases: [Carol]\n- node_type: Person\n  aliases: \
         [Dana]\n- node_type: Person\n  aliases: [Dee]\n",
    );
    let first: ExtractionReport = apply(
        &mut session,
        &ExtractionDocument::from_yaml(&two).unwrap(),
        operator,
    )
    .unwrap();
    assert!(first.rejected.is_empty(), "{first:?}");

    let document = ExtractionDocument::from_yaml(
        &KNOWN_TYPES.replace("aliases: [Carol]", "aliases: [Dana, Dee]"),
    )
    .unwrap();
    let property: PropertyFact = match &document.facts[0] {
        ExtractedFact::Property(fact) => fact.clone(),
        other => panic!("facts[0] is a property: {other:?}"),
    };
    assert_eq!(
        property.subject,
        ExtractedReference::new("Organization", ["Globex"])
    );
    let relation: RelationFact = match &document.facts[1] {
        ExtractedFact::Relation(fact) => fact.clone(),
        other => panic!("facts[1] is a relation: {other:?}"),
    };
    assert_eq!(
        relation.subject,
        ExtractedReference::new("Person", ["Dana", "Dee"])
    );

    let committed: &CommittedExtraction = &first.committed[0];
    assert!(committed.revision > 0, "{first:?}");
    let report: ExtractionReport = apply(&mut session, &document, operator).unwrap();
    assert!(report.rejected.is_empty(), "{report:?}");
    // The property about Globex is the one the first document asserted: held, not asserted again.
    assert!(report.committed.is_empty(), "{report:?}");
    assert_eq!(report.held, ["facts[0]"], "{report:?}");
    assert_eq!(report.stopped, None);
    assert_eq!(report.ambiguous.len(), 1, "{report:?}");
    let ambiguous: &AmbiguousExtraction = &report.ambiguous[0];
    assert_eq!(ambiguous.reference, relation.subject);
    assert_eq!(ambiguous.candidates.len(), 2, "{report:?}");

    // No ontology section, and a type the store does not declare: nothing is written.
    session.close().unwrap();
    let head = world.store_json(&["head"]);
    let mut session =
        ProcessSession::start(&binary, world.config(), SessionOptions::default()).unwrap();
    let robot = ExtractionDocument::from_yaml(&KNOWN_TYPES.replace(
        "- node_type: Person\n  aliases: [Carol]\n",
        "- node_type: Robot\n  aliases: [Carol]\n",
    ))
    .unwrap();
    match apply(&mut session, &robot, operator) {
        Err(ApplyError::Undeclared { kind, name }) => {
            assert_eq!((kind, name.as_str()), ("node type", "Robot"));
        }
        other => panic!("an undeclared type stops the routine: {other:?}"),
    }
    session.close().unwrap();
    assert_eq!(world.store_json(&["head"]), head);

    // The example grows the schema, which the example host's profile fixes.
    let fixed = World::new("file", false);
    let mut session =
        ProcessSession::start(&binary, fixed.config(), SessionOptions::default()).unwrap();
    let example = ExtractionDocument::from_yaml(&example_document()).unwrap();
    let report: ExtractionReport = apply(&mut session, &example, operator).unwrap();
    session.close().unwrap();
    assert!(report.committed.is_empty(), "{report:?}");
    let rejected: &RejectedExtraction = &report.rejected[0];
    assert_eq!(rejected.item, "ontology");
    assert!(rejected.transaction_id.is_some(), "{report:?}");
    assert_eq!(rejected.refusal, None);
    let issue: &ExtractionIssue = &rejected.issues[0];
    assert!(
        !issue.code.is_empty() && !issue.validator.is_empty(),
        "{report:?}"
    );
}

/// The SDK's mirror refuses what the engine's reader refuses while decoding, and two evidence
/// items under one id or a payload that does not hash to its entry, each by the reader's code;
/// its limits are the reader's.
#[test]
fn the_mirror_refuses_by_the_readers_codes() {
    assert_eq!(
        ekr_sdk::document::EXTRACTION_INPUT_BYTES,
        ekr_integrate::EXTRACTION_INPUT_BYTES
    );
    assert_eq!(
        ekr_sdk::document::EXTRACTION_DEPTH,
        ekr_integrate::EXTRACTION_DEPTH
    );
    let mut deep = "{value_kind: String, value: x}".to_owned();
    for _ in 0..20 {
        deep = format!("{{value_kind: List, value: [{deep}]}}");
    }
    let second = KNOWN_TYPES
        .split_once("evidence:\n- evidence:")
        .map(|(_, item)| format!("- evidence:{item}"))
        .unwrap()
        .replace("identity: Quarterly report", "identity: Another report");
    let cases = [
        (
            "format: ekr.extraction-document/1\nentities:\n- node_type: Person\n  aliases: [&n \
             Carol]\n- node_type: Person\n  aliases: [*n]\nfacts: []\nevidence: []\n"
                .to_owned(),
            "extraction-yaml-alias",
        ),
        (
            "format: ekr.extraction-document/1\nentities:\n- node_type: Person\n  aliases: \
             [2024]\nfacts: []\nevidence: []\n"
                .to_owned(),
            "extraction-document-malformed",
        ),
        (
            format!(
                "format: ekr.extraction-document/1\nentities:\n- node_type: Person\n  aliases: \
                 [{}]\nfacts: []\nevidence: []\n",
                "C".repeat(ekr_integrate::EXTRACTION_INPUT_BYTES)
            ),
            "extraction-document-too-large",
        ),
        (
            format!(
                "format: ekr.extraction-document/1\nfacts:\n- !Property\n  subject: {{node_type: \
                 Person, aliases: [Carol]}}\n  property: tags\n  value: {deep}\n  evidence: \
                 [00000000-0000-4000-8000-000000000403]\nevidence: []\n"
            ),
            "extraction-document-too-deep",
        ),
        (format!("{KNOWN_TYPES}{second}"), "duplicate-identity"),
        (
            KNOWN_TYPES.replace("payload: [67,", "payload: [68,"),
            "evidence-payload-mismatch",
        ),
    ];
    let world = World::new("file", false);
    for (document, code) in cases {
        let reader = match ekr_integrate::ExtractionDocument::from_yaml(&document) {
            Err(error) => error.refusal().code.code().to_owned(),
            // A check of the document alone, which the reader makes after the store's: read off
            // the verb's refusal over a store the document's types are declared in.
            Ok(_) => {
                let path = world.file("refused.yaml");
                std::fs::write(&path, &document).unwrap();
                let output = world.store_output(&["apply-extraction", path.to_str().unwrap()]);
                assert_eq!(output.status.code(), Some(2), "{code}");
                let stderr = String::from_utf8(output.stderr).unwrap();
                stderr
                    .strip_prefix("ekr: ")
                    .and_then(|rest| rest.split_once(':'))
                    .map(|(name, _)| name.to_owned())
                    .unwrap_or(stderr)
            }
        };
        assert_eq!(reader, code, "the reader");
        let mirror = ExtractionDocument::from_yaml(&document).map(|_| ());
        assert_eq!(
            mirror.map_err(|refusal| refusal.code),
            Err(code.to_owned()),
            "the mirror"
        );
    }
}

/// A child session that dies after its first commit: every later request is refused as a
/// process that ended before answering.
struct DiesAfterFirstCommit {
    session: ProcessSession,
    committed: bool,
}

impl ekr_sdk::transport::Transport for DiesAfterFirstCommit {
    fn request(
        &mut self,
        request: &ekr_sdk::transport::Request,
    ) -> Result<ekr_sdk::reply::Reply, ekr_sdk::transport::TransportError> {
        if self.committed {
            return Err(ekr_sdk::transport::TransportError::Died {
                verb: request.verb().to_owned(),
                status: "cut by the test".to_owned(),
                stderr_tail: String::new(),
            });
        }
        let reply = self.session.request(request)?;
        self.committed = request.verb() == "commit" && reply.exit == 0;
        Ok(reply)
    }
}

/// Once something has committed, the routine does not end in an error: a request that then gets
/// no answer stops it, and the report lists what committed and says why it stopped. Before
/// anything committed, the same failure is the error it is, with nothing written.
#[test]
fn a_failure_after_a_commit_is_reported_with_what_committed() {
    let world = World::new("file", true);
    let binary = EkrBinary::open(binary()).unwrap();
    let document = ExtractionDocument::from_yaml(&example_document()).unwrap();
    let operator = OPERATOR.parse::<AgentId>().unwrap();
    let mut cut = DiesAfterFirstCommit {
        session: ProcessSession::start(&binary, world.config(), SessionOptions::default())
            .unwrap(),
        committed: false,
    };
    let report = apply(&mut cut, &document, operator).unwrap();
    // The schema change committed; resolving the first named thing found the session gone.
    assert_eq!(report.committed.len(), 1, "{report:?}");
    let stopped = report.stopped.as_deref().unwrap_or_default();
    assert!(stopped.contains("cut by the test"), "{report:?}");
    cut.session.close().unwrap();
    assert_eq!(world.store_json(&["head"])["revision"], 1);

    let fresh = World::new("file", true);
    let mut dead = DiesAfterFirstCommit {
        session: ProcessSession::start(&binary, fresh.config(), SessionOptions::default())
            .unwrap(),
        committed: true,
    };
    assert!(matches!(
        apply(&mut dead, &document, operator),
        Err(ApplyError::Read(_))
    ));
    dead.session.close().unwrap();
    assert_eq!(fresh.store_json(&["head"])["revision"], 0);
}
