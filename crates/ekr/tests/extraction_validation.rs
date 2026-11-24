//! `story:ekr-extraction-validation-fixes`: extraction and strict validation accept what the
//! schema allows and refuse what it forbids, the same way in the SDK and the CLI (review of
//! 0.0.42, findings 5-11 and 14).
//!
//! Every case drives the built binary on one provider: the verb, and the SDK's routine over a
//! child `ekr session` of it. Each finding has one case per provider.

use std::path::PathBuf;
use std::process::Output;

use ekr_sdk::batch::{BatchReport, Batcher};
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{
    AgentId, Assertion, EvidenceId, ExtractionDocument, GraphRootId, NodeDraft, NodeId,
    NodeSupersession, Object, Operation, Predicate, PropertyId, Subject, Supersession,
    TemporalRange, Timestamp, TypeId, Value as Written,
};
use ekr_sdk::extraction::{ApplyError, ApplyOptions};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use serde_json::Value;

const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
/// The example seed's graph root, its Organization Acme, `Organization.legal_name` and the
/// evidence item 0401 it retains.
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const ACME: &str = "00000000-0000-4000-8000-000000000303";
const LEGAL_NAME: &str = "00000000-0000-4000-8000-000000000801";
const RETAINED: &str = "00000000-0000-4000-8000-000000000401";

/// Types this file adds to the example seed. `Leaf` inherits `code` from `Beta`, one generation
/// up, as a String, and from `Gamma`, two up through `Alpha`, as an Integer; `Alpha` sorts before
/// `Beta`, so a depth-first walk meets `Gamma` first. `Holder` and `Description` both declare
/// `code`, of different types, and only `Description` declares `summary`.
const EXTRA_TYPES: &str = "  - id: 00000000-0000-4000-8000-000000000211
    name: Gamma
    parents: []
    properties:
      00000000-0000-4000-8000-000000000811:
        id: 00000000-0000-4000-8000-000000000811
        name: code
        value_type: {value_kind: Integer}
        cardinality: One
        required: false
        constraints: []
    abstract_type: false
    lifecycle: null
    operations: {}
  - id: 00000000-0000-4000-8000-000000000212
    name: Alpha
    parents: [00000000-0000-4000-8000-000000000211]
    properties: {}
    abstract_type: false
    lifecycle: null
    operations: {}
  - id: 00000000-0000-4000-8000-000000000213
    name: Beta
    parents: []
    properties:
      00000000-0000-4000-8000-000000000812:
        id: 00000000-0000-4000-8000-000000000812
        name: code
        value_type: {value_kind: String}
        cardinality: One
        required: false
        constraints: []
    abstract_type: false
    lifecycle: null
    operations: {}
  - id: 00000000-0000-4000-8000-000000000214
    name: Leaf
    parents: [00000000-0000-4000-8000-000000000212, 00000000-0000-4000-8000-000000000213]
    properties: {}
    abstract_type: false
    lifecycle: null
    operations: {}
  - id: 00000000-0000-4000-8000-000000000215
    name: Holder
    parents: []
    properties:
      00000000-0000-4000-8000-000000000813:
        id: 00000000-0000-4000-8000-000000000813
        name: code
        value_type: {value_kind: Integer}
        cardinality: One
        required: false
        constraints: []
    abstract_type: false
    lifecycle: null
    operations: {}
  - id: 00000000-0000-4000-8000-000000000216
    name: Description
    parents: []
    properties:
      00000000-0000-4000-8000-000000000814:
        id: 00000000-0000-4000-8000-000000000814
        name: code
        value_type: {value_kind: String}
        cardinality: One
        required: false
        constraints: []
      00000000-0000-4000-8000-000000000815:
        id: 00000000-0000-4000-8000-000000000815
        name: summary
        value_type: {value_kind: String}
        cardinality: One
        required: false
        constraints: []
    abstract_type: false
    lifecycle: null
    operations: {}
";

/// A second node type named `Person`, beside the example seed's.
const SECOND_PERSON: &str = "  - id: 00000000-0000-4000-8000-000000000217
    name: Person
    parents: []
    properties: {}
    abstract_type: false
    lifecycle: null
    operations: {}
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
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// One provider in its own directory: the example host under validation profile v2, and the
/// example seed with `types` added to its node types.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn new(backend: &'static str, types: &str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        let host = text(&["example", "ekr.cli-host/1"])
            .replace("\"ekr.p1-deterministic/1\"", "\"ekr.p2-deterministic/1\"")
            .replace("\"ekr.p1-apply/1\"", "\"ekr.p2-apply/1\"");
        std::fs::write(world.file("host.json"), host).unwrap();
        let seed = text(&["example", "ekr-seed/2"]);
        let at = "  edge_types:\n";
        assert!(seed.contains(at), "the example seed lists its edge types");
        std::fs::write(
            world.file("seed.yaml"),
            seed.replacen(at, &format!("{types}{at}"), 1),
        )
        .unwrap();
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
            backend: match self.backend {
                "file" => Backend::File,
                _ => Backend::Sqlite,
            },
        }
    }

    fn output(&self, args: &[&str]) -> Output {
        ekr()
            .arg("--host")
            .arg(self.file("host.json"))
            .arg("--store")
            .arg(self.file("store"))
            .args(["--backend", self.backend])
            .args(args)
            .output()
            .unwrap()
    }

    fn json(&self, args: &[&str]) -> Value {
        let output = self.output(args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}: {args:?}: stderr {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// `ekr apply-extraction` of `document`, `--strict` when `strict`: its output, whatever its
    /// exit.
    fn verb(&self, document: &str, strict: bool) -> Output {
        let path = self.file("extraction.yaml");
        std::fs::write(&path, document).unwrap();
        let path = path.to_str().unwrap();
        if strict {
            self.output(&["apply-extraction", "--strict", path])
        } else {
            self.output(&["apply-extraction", path])
        }
    }

    /// The verb's report, which must exit 0.
    fn verb_report(&self, document: &str, strict: bool) -> Value {
        let output = self.verb(document, strict);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}: stderr {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// The SDK's routine over a child session, on the document as `ExtractionDocument::from_yaml`
    /// reads it, as a consumer calls it.
    fn sdk(&self, document: &str, strict: bool) -> Result<Value, ApplyError> {
        let binary = EkrBinary::open(binary()).unwrap();
        let mut session =
            ProcessSession::start(&binary, self.config(), SessionOptions::default()).unwrap();
        let document = ExtractionDocument::from_yaml(document).unwrap();
        let options = if strict {
            ApplyOptions::default().strict()
        } else {
            ApplyOptions::default()
        };
        let report = ekr_sdk::extraction::apply_with(
            &mut session,
            &document,
            OPERATOR.parse::<AgentId>().unwrap(),
            &options,
        );
        session.close().unwrap();
        report.map(|report| serde_json::to_value(report).unwrap())
    }

    /// One transaction of `groups` through the SDK's batcher, over a child session.
    fn commit(&self, groups: &[Vec<Operation>]) -> BatchReport {
        let binary = EkrBinary::open(binary()).unwrap();
        let mut session =
            ProcessSession::start(&binary, self.config(), SessionOptions::default()).unwrap();
        let report = Batcher::new(OPERATOR.parse().unwrap())
            .commit(&mut session, groups)
            .unwrap();
        session.close().unwrap();
        report
    }

    fn head(&self) -> u64 {
        self.json(&["head"])["revision"].as_u64().unwrap()
    }

    fn graph(&self) -> Value {
        self.json(&["snapshot"])["graph"]["graph"].clone()
    }

    /// The node holding `alias`: its id and the node as the snapshot prints it.
    fn node(&self, alias: &str) -> Option<(String, Value)> {
        self.graph()["nodes"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(_, node)| {
                node["aliases"]
                    .as_array()
                    .is_some_and(|aliases| aliases.iter().any(|held| held == alias))
            })
            .map(|(id, node)| (id.clone(), node.clone()))
    }

    /// The objects of the active assertions of `property` about the node holding `alias`.
    fn active(&self, alias: &str, property: &str) -> Vec<Value> {
        let Some((node, _)) = self.node(alias) else {
            return Vec::new();
        };
        self.graph()["assertions"]
            .as_object()
            .unwrap()
            .values()
            .filter(|assertion| {
                assertion["lifecycle"] == "Active"
                    && assertion["subject"]["Node"] == node.as_str()
                    && assertion["predicate"]["Property"] == property
            })
            .map(|assertion| assertion["object"]["Value"].clone())
            .collect()
    }
}

/// The evidence item `00000000-0000-4000-8000-000000000<suffix>` of a document, whose bytes are
/// `payload`.
fn evidence_item(suffix: &str, payload: &[u8]) -> String {
    let bytes: Vec<String> = payload.iter().map(u8::to_string).collect();
    format!(
        "- evidence:
    id: 00000000-0000-4000-8000-000000000{suffix}
    source: !HumanStatement
      identity: Source {suffix}
    content_hash: {}
    extracted_by: {OPERATOR}
    observed_at: 1775000000000
    confidence: 9000
  payload: [{}]
",
        ekr_sdk::document::payload_hash(payload),
        bytes.join(", ")
    )
}

/// A document of `ontology` (YAML of its section, or empty), `entities`, one `!Property` fact
/// about the `node_type` known as `aliases`, of `property` and `value`, citing item 0430, and
/// that item, the same in every such document.
fn property_document(
    ontology: &str,
    entities: &str,
    (node_type, aliases): (&str, &str),
    property: &str,
    value: &str,
) -> String {
    format!(
        "format: ekr.extraction-document/1
{ontology}entities: [{entities}]
facts:
- !Property
  subject: {{node_type: {node_type}, aliases: [{aliases}]}}
  property: {property}
  value: {value}
  evidence: [00000000-0000-4000-8000-000000000430]
evidence:
{}",
        evidence_item("430", b"The source of every property row.")
    )
}

/// The code an `ApplyError::Refused` carries.
fn refused_code(error: &ApplyError) -> &str {
    match error {
        ApplyError::Refused(refusal) => &refusal.code,
        other => panic!("not a refusal by code: {other}"),
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

// Finding 5 -------------------------------------------------------------------------------------

/// A property two ancestors declare under one name is the nearer one's for the reader and the
/// writer alike: a String value of the nearby declaration commits through the verb and the SDK,
/// and an Integer value of the more distant one is refused before anything is written.
fn inherited_property(backend: &'static str) {
    let world = World::new(backend, EXTRA_TYPES);
    let near = "{value_kind: String, value: near}";
    let report = world.verb_report(
        &property_document("", "", ("Leaf", "leaf-1"), "code", near),
        true,
    );
    assert_eq!(
        report["rejected"],
        serde_json::json!([]),
        "{backend}: {report}"
    );
    assert_eq!(
        world.active("leaf-1", "00000000-0000-4000-8000-000000000812"),
        vec![serde_json::json!({"value_kind": "String", "value": "near"})],
        "{backend}: the nearby String declaration"
    );
    let report = world
        .sdk(
            &property_document("", "", ("Leaf", "leaf-2"), "code", near),
            true,
        )
        .unwrap_or_else(|error| panic!("{backend}: {error}"));
    assert_eq!(
        report["rejected"],
        serde_json::json!([]),
        "{backend}: {report}"
    );

    let head = world.head();
    let distant = property_document(
        "",
        "",
        ("Leaf", "leaf-3"),
        "code",
        "{value_kind: Integer, value: 7}",
    );
    let output = world.verb(&distant, true);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{backend}: {}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("extraction-value-mismatch"),
        "{backend}: {}",
        stderr(&output)
    );
    let error = world.sdk(&distant, true).unwrap_err();
    assert_eq!(
        refused_code(&error),
        "extraction-value-mismatch",
        "{backend}"
    );
    assert_eq!(world.head(), head, "{backend}: nothing was written");
}

#[test]
fn an_inherited_property_is_the_nearest_ancestors_for_reader_and_writer_file() {
    inherited_property("file");
}

#[test]
fn an_inherited_property_is_the_nearest_ancestors_for_reader_and_writer_sqlite() {
    inherited_property("sqlite");
}

// Finding 6 -------------------------------------------------------------------------------------

/// A Holder known as `old-holder` superseded by a Description known as `kept-description`.
fn superseded_holder(world: &World) {
    let root: GraphRootId = ROOT.parse().unwrap();
    let holder: TypeId = "00000000-0000-4000-8000-000000000215".parse().unwrap();
    let description: TypeId = "00000000-0000-4000-8000-000000000216".parse().unwrap();
    let mut old = NodeDraft::new(root, holder, "old-holder");
    old.aliases = vec!["old-holder".to_owned()];
    let mut kept = NodeDraft::new(root, description, "kept-description");
    kept.aliases = vec!["kept-description".to_owned()];
    let (old_id, kept_id): (NodeId, NodeId) = (old.id, kept.id);
    let report = world.commit(&[vec![
        Operation::CreateNode(old),
        Operation::CreateNode(kept),
    ]]);
    assert_eq!(report.committed.len(), 1, "{report:?}");
    let evidence: EvidenceId = RETAINED.parse().unwrap();
    let report = world.commit(&[vec![Operation::SupersedeNode(NodeSupersession::new(
        old_id,
        kept_id,
        "the holder was a description",
        [evidence],
    ))]]);
    assert_eq!(report.committed.len(), 1, "{report:?}");
}

/// A fact on one alias of a superseded node, joined to its old alias by an entity rather than on
/// the fact itself, is checked against the kept node's type: a property only that type declares
/// is applied to the kept node, and a value only the superseded type's property holds is refused
/// before anything is written, through the verb and the SDK.
fn alias_group_typing(backend: &'static str) {
    let world = World::new(backend, EXTRA_TYPES);
    superseded_holder(&world);

    let indirect = property_document(
        "",
        "{node_type: Holder, aliases: [old-holder, linked]}",
        ("Holder", "linked"),
        "summary",
        "{value_kind: String, value: kept summary}",
    );
    let report = world.verb_report(&indirect, false);
    assert_eq!(
        report["rejected"],
        serde_json::json!([]),
        "{backend}: {report}"
    );
    assert_eq!(
        world.active("kept-description", "00000000-0000-4000-8000-000000000815"),
        vec![serde_json::json!({"value_kind": "String", "value": "kept summary"})],
        "{backend}: {report}"
    );

    let head = world.head();
    let wrong = property_document(
        "",
        "{node_type: Holder, aliases: [unrelated]}, {node_type: Holder, aliases: [old-holder, \
         linked-2]}",
        ("Holder", "linked-2"),
        "code",
        "{value_kind: Integer, value: 5}",
    );
    let output = world.verb(&wrong, true);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{backend}: {}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("extraction-value-mismatch"),
        "{backend}: {}",
        stderr(&output)
    );
    let error = world.sdk(&wrong, true).unwrap_err();
    assert_eq!(
        refused_code(&error),
        "extraction-value-mismatch",
        "{backend}"
    );
    assert_eq!(world.head(), head, "{backend}: nothing was written");
    assert!(world.node("unrelated").is_none(), "{backend}");
}

#[test]
fn a_fact_is_typed_by_the_alias_group_it_resolves_with_file() {
    alias_group_typing("file");
}

#[test]
fn a_fact_is_typed_by_the_alias_group_it_resolves_with_sqlite() {
    alias_group_typing("sqlite");
}

// Finding 7 -------------------------------------------------------------------------------------

/// A document with a new Organization and one bad fact about it, under `--strict`, leaves the
/// store as it was through the SDK as through the verb: a value of the wrong type, and a fact
/// citing no evidence.
fn sdk_strict(backend: &'static str) {
    let world = World::new(backend, "");
    let mismatched = property_document(
        "",
        "{node_type: Organization, aliases: [Newco]}",
        ("Organization", "Newco"),
        "legal_name",
        "{value_kind: Integer, value: 7}",
    );
    let unsupported = mismatched
        .replace(
            "{value_kind: Integer, value: 7}",
            "{value_kind: String, value: Newco Ltd}",
        )
        .replace(
            "  evidence: [00000000-0000-4000-8000-000000000430]\n",
            "  evidence: []\n",
        );
    for (document, code) in [
        (&mismatched, "extraction-value-mismatch"),
        (&unsupported, "fact-without-evidence"),
    ] {
        let output = world.verb(document, true);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{backend}: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains(code),
            "{backend}: {}",
            stderr(&output)
        );
        let error = world.sdk(document, true).unwrap_err();
        assert_eq!(refused_code(&error), code, "{backend}");
        assert_eq!(world.head(), 0, "{backend}: {code}: nothing was written");
    }
}

#[test]
fn the_sdk_refuses_a_strict_document_before_any_write_as_the_verb_does_file() {
    sdk_strict("file");
}

#[test]
fn the_sdk_refuses_a_strict_document_before_any_write_as_the_verb_does_sqlite() {
    sdk_strict("sqlite");
}

// Finding 8 -------------------------------------------------------------------------------------

/// A store with two node types named Person applies a document that names only Organization, and
/// refuses one that names Person, at that reference, before anything is written.
fn shared_type_name(backend: &'static str) {
    let world = World::new(backend, SECOND_PERSON);
    let organization = |alias: &str| {
        property_document(
            "",
            "",
            ("Organization", alias),
            "legal_name",
            "{value_kind: String, value: Initech}",
        )
    };
    let report = world.verb_report(&organization("Initech"), true);
    assert_eq!(
        report["rejected"],
        serde_json::json!([]),
        "{backend}: {report}"
    );
    let report = world
        .sdk(&organization("Initrode"), true)
        .unwrap_or_else(|error| panic!("{backend}: {error}"));
    assert_eq!(
        report["rejected"],
        serde_json::json!([]),
        "{backend}: {report}"
    );
    let head = world.head();
    assert!(head > 0, "{backend}");

    let person = organization("Hooli").replace(
        "entities: []",
        "entities: [{node_type: Person, aliases: [Gavin]}]",
    );
    let output = world.verb(&person, true);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{backend}: {}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("extraction-name-duplicate"),
        "{backend}: {}",
        stderr(&output)
    );
    let error = world.sdk(&person, true).unwrap_err();
    assert_eq!(
        refused_code(&error),
        "extraction-name-duplicate",
        "{backend}"
    );
    assert_eq!(world.head(), head, "{backend}: nothing more was written");
}

#[test]
fn an_unused_duplicate_type_name_does_not_stop_ingestion_file() {
    shared_type_name("file");
}

#[test]
fn an_unused_duplicate_type_name_does_not_stop_ingestion_sqlite() {
    shared_type_name("sqlite");
}

// Finding 9 -------------------------------------------------------------------------------------

/// A type the document declares with a required `label`, and a fact giving one: the node is
/// created carrying the value and the fact is asserted, through the verb and the SDK.
fn required_value(backend: &'static str) {
    let world = World::new(backend, "");
    let ontology = "ontology:
  node_types:
  - name: Badge
    properties:
    - {name: label, value: {value_kind: String}, required: true}
";
    for (alias, sdk) in [("badge-1", false), ("badge-2", true)] {
        let document = property_document(
            ontology,
            "",
            ("Badge", alias),
            "label",
            "{value_kind: String, value: provided}",
        );
        let report = if sdk {
            world
                .sdk(&document, false)
                .unwrap_or_else(|error| panic!("{backend}: {error}"))
        } else {
            world.verb_report(&document, false)
        };
        assert_eq!(
            report["rejected"],
            serde_json::json!([]),
            "{backend}: {report}"
        );
        let (_, node) = world
            .node(alias)
            .unwrap_or_else(|| panic!("{backend}: no node {alias}: {report}"));
        let values: Vec<&Value> = node["properties"].as_object().unwrap().values().collect();
        assert_eq!(
            values,
            vec![&serde_json::json!([{"value_kind": "String", "value": "provided"}])],
            "{backend}: {node}"
        );
        let asserted = world.graph()["assertions"]
            .as_object()
            .unwrap()
            .values()
            .filter(|assertion| {
                assertion["object"]["Value"]["value"] == "provided"
                    && assertion["lifecycle"] == "Active"
            })
            .count();
        assert_eq!(asserted, if sdk { 2 } else { 1 }, "{backend}: {report}");
    }
}

#[test]
fn a_required_value_the_document_supplies_initialises_the_new_node_file() {
    required_value("file");
}

#[test]
fn a_required_value_the_document_supplies_initialises_the_new_node_sqlite() {
    required_value("sqlite");
}

// Finding 10 ------------------------------------------------------------------------------------

/// A producer's declaration of `Widget.colour` as `Enum [green]`, applied again after another
/// widened it to `[blue, green]`, still applies its green row, and leaves the wider declaration.
fn widened_enum(backend: &'static str) {
    let world = World::new(backend, "");
    let declared = |variants: &str| {
        format!(
            "ontology:
  node_types:
  - name: Widget
    properties:
    - {{name: colour, value: {{value_kind: Enum, parameters: {{variants: [{variants}]}}}}}}
"
        )
    };
    let row = |variants: &str, alias: &str, colour: &str| {
        property_document(
            &declared(variants),
            "",
            ("Widget", alias),
            "colour",
            &format!("{{value_kind: Enum, value: {colour}}}"),
        )
    };
    for (variants, alias, colour) in [
        ("green", "w-1", "green"),
        ("green, blue", "w-2", "blue"),
        ("green", "w-3", "green"),
    ] {
        let report = world.verb_report(&row(variants, alias, colour), true);
        assert_eq!(
            report["rejected"],
            serde_json::json!([]),
            "{backend}: {variants}: {report}"
        );
    }
    let report = world
        .sdk(&row("green", "w-4", "green"), true)
        .unwrap_or_else(|error| panic!("{backend}: {error}"));
    assert_eq!(
        report["rejected"],
        serde_json::json!([]),
        "{backend}: {report}"
    );
    let ontology = world.json(&["ontology"]);
    let widget = ontology["node_types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["name"] == "Widget")
        .unwrap();
    let colour = widget["properties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|property| property["name"] == "colour")
        .unwrap();
    assert!(
        colour.to_string().contains("blue"),
        "{backend}: the store keeps the wider declaration: {colour}"
    );
}

#[test]
fn an_older_declaration_still_applies_after_the_store_widened_it_file() {
    widened_enum("file");
}

#[test]
fn an_older_declaration_still_applies_after_the_store_widened_it_sqlite() {
    widened_enum("sqlite");
}

// Finding 11 ------------------------------------------------------------------------------------

/// An Acme legal name valid from `from` milliseconds, citing the retained item 0401.
fn legal_name(value: &str, from: i64) -> Assertion {
    Assertion::new(
        ROOT.parse().unwrap(),
        Subject::Node(ACME.parse().unwrap()),
        Predicate::Property(LEGAL_NAME.parse::<PropertyId>().unwrap()),
        Object::Value(Written::String(value.to_owned())),
        OPERATOR.parse().unwrap(),
    )
    .citing(RETAINED.parse().unwrap())
    .with_valid_time(TemporalRange::since(Timestamp::from_millis(from)))
}

/// Three legal names at 1000, 2000 and 3000 ms, committed; their ids.
fn three_names(world: &World, prefix: &str) -> [Assertion; 3] {
    let names = [1000, 2000, 3000].map(|from| legal_name(&format!("{prefix} {from}"), from));
    let report = world.commit(&[names.iter().cloned().map(Operation::from).collect()]);
    assert_eq!(report.committed.len(), 1, "{report:?}");
    names
}

fn supersede(old: &Assertion, by: &Assertion, from: i64) -> Operation {
    Supersession::new(old.id, by.id, Timestamp::from_millis(from)).into()
}

/// A replacement an earlier transaction already superseded is refused as `invalid-supersession`;
/// a replacement the same transaction supersedes in turn, a chain, commits.
fn closed_replacement(backend: &'static str) {
    let world = World::new(backend, "");
    let [first, second, third] = three_names(&world, "closed");
    let report = world.commit(&[vec![supersede(&second, &third, 3000)]]);
    assert_eq!(report.committed.len(), 1, "{backend}: {report:?}");
    let head = world.head();
    let report = world.commit(&[vec![supersede(&first, &second, 2000)]]);
    assert!(report.committed.is_empty(), "{backend}: {report:?}");
    assert!(
        format!("{report:?}").contains("invalid-supersession"),
        "{backend}: {report:?}"
    );
    assert_eq!(world.head(), head, "{backend}");

    let [first, second, third] = three_names(&world, "chained");
    let report = world.commit(&[vec![
        supersede(&second, &third, 3000),
        supersede(&first, &second, 2000),
    ]]);
    assert_eq!(report.committed.len(), 1, "{backend}: {report:?}");
}

#[test]
fn an_already_superseded_replacement_is_refused_and_a_chain_in_one_transaction_commits_file() {
    closed_replacement("file");
}

#[test]
fn an_already_superseded_replacement_is_refused_and_a_chain_in_one_transaction_commits_sqlite() {
    closed_replacement("sqlite");
}

// Finding 14 ------------------------------------------------------------------------------------

/// A document whose evidence item is one byte past the `sequence_elements` limit of the
/// `!AddEvidence` that would add it is refused before anything is written, with or without
/// `--strict` and through the SDK; at the limit, the same document commits.
fn oversized_evidence(backend: &'static str) {
    let world = World::new(backend, "");
    let document = |bytes: usize| {
        format!(
            "format: ekr.extraction-document/1
facts:
- !Property
  subject: {{node_type: Organization, aliases: [Acme]}}
  property: legal_name
  value: {{value_kind: String, value: Acme-Example}}
  evidence: [00000000-0000-4000-8000-000000000430]
evidence:
{}",
            evidence_item("430", &vec![b'x'; bytes])
        )
    };
    let over = document(16_385);
    for strict in [true, false] {
        let output = world.verb(&over, strict);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{backend}: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains("sequence_elements"),
            "{backend}: {}",
            stderr(&output)
        );
    }
    let error = world.sdk(&over, false).unwrap_err();
    assert!(
        format!("{error:?}").starts_with("Limit(")
            && error.to_string().contains("sequence_elements"),
        "{backend}: {error:?}"
    );
    assert_eq!(world.head(), 0, "{backend}: nothing was written");
    assert!(world.node("Acme").is_none(), "{backend}");

    let report = world.verb_report(&document(16_384), true);
    assert_eq!(
        report["rejected"],
        serde_json::json!([]),
        "{backend}: {report}"
    );
    assert!(world.head() > 0, "{backend}");
    assert!(world.node("Acme").is_some(), "{backend}");
}

#[test]
fn an_evidence_item_past_the_transaction_limit_is_refused_before_any_write_file() {
    oversized_evidence("file");
}

#[test]
fn an_evidence_item_past_the_transaction_limit_is_refused_before_any_write_sqlite() {
    oversized_evidence("sqlite");
}
