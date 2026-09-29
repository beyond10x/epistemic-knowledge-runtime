//! `story:sdk-typed-documents`: every document the SDK builds is read by the real readers, passes
//! the schema `ekr schema <format>` prints, and does what it says on a real store.
//!
//! The readers are the kernel's (`ekr_kernel::TransactionDocument::parse`,
//! `ekr_kernel::SeedDocument::from_yaml`), and the verbs are `ekr`'s own, run in process through
//! `ekr::cli::run` against a file store: `ekr operations`, `ekr schema`, `ekr hash`, `ekr mint`,
//! `ekr seed`, `ekr propose`, `ekr validate`, `ekr commit`, `ekr ontology` and `ekr resolve`.
//! Both are dev-dependencies: the SDK itself links neither, which the last case holds.
//!
//! The drift the file exists for is a kind, a field or a format the engine grows that the SDK does
//! not: a new `GraphOperation` variant fails [`kernel_kind`] at compile time, a kind `ekr
//! operations` or `ekr schema` lists without an SDK builder fails
//! [`every_operation_kind_ekr_lists_has_a_builder`], and a field the readers or the schema start to
//! require fails the round trips below.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ekr::exit::Failure;
use ekr_sdk::document as sdk;
use ekr_sdk::document::{
    payload_hash, to_yaml, AgentId, Assertion, AssertionId, AssertionLifecycle, Assessment,
    Cardinality, Confidence, DocumentError, EdgeDraft, EdgeId, EdgeType, EdgeTypeSpec,
    EdgeWidening, EntityMerge, Evidence, EvidenceAddition, EvidenceId, EvidenceSource, GraphRoot,
    GraphRootId, GraphSection, Invocation, Lifecycle, NodeDraft, NodeId, NodeType, NodeTypeSpec,
    Object, Ontology, OntologyError, OntologySection, OntologySpec, Operation, OperationDefinition,
    OperationKind, Predicate, PropertyDefinition, PropertyId, PropertyModification,
    PropertyMutation, PropertySpec, Retraction, SchemaChange, SchemaVersion, SchemaVersionId,
    SeedBuilder, SeedDocument, SeedGraph, SeedNode, Space, Subject, Supersession, TemporalRange,
    Timestamp, Transaction, TransactionBuilder, TransactionDocument, TransactionId,
    TransactionTime, Transition, TypeId, TypedReference, ValidationProfile, Value, ValueSpec,
    ValueType,
};
use serde_json::Value as Json;

/// The clock every in-process verb reads.
const NOW: i64 = 1_790_000_000_000;

/// One `ekr` verb, in process, with `stdin` as what `-` reads.
fn ekr(args: &[&str], stdin: &[u8]) -> Result<String, Failure> {
    let mut input = stdin;
    ekr::cli::run(
        std::iter::once("ekr").chain(args.iter().copied()),
        &|| Timestamp::from_millis(NOW),
        &mut input,
    )
}

/// A verb that must succeed, and its stdout.
fn ok(args: &[&str], stdin: &[u8]) -> String {
    ekr(args, stdin).unwrap_or_else(|failure| panic!("ekr {args:?}: {failure}"))
}

/// A verb that must succeed with one JSON document.
fn json(args: &[&str], stdin: &[u8]) -> Json {
    let out = ok(args, stdin);
    serde_json::from_str(&out).unwrap_or_else(|e| panic!("ekr {args:?}: not JSON ({e}): {out}"))
}

/// The instance a YAML format's schema validates: the document read as YAML and written as JSON.
fn projection(yaml: &str) -> Json {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(yaml).unwrap_or_else(|e| panic!("not YAML ({e}): {yaml}"));
    serde_json::to_value(value).expect("a YAML value has a JSON form")
}

/// `ekr schema <format>` accepts `yaml`'s projection, naming every error if it does not.
fn schema_accepts(format: &str, yaml: &str) {
    let schema = json(&["schema", format], b"");
    let validator = jsonschema::draft202012::new(&schema)
        .unwrap_or_else(|e| panic!("ekr schema {format} does not compile: {e}"));
    let instance = projection(yaml);
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "ekr schema {format} refuses an SDK document: {errors:#?}\n{yaml}"
    );
}

/// The kind of a parsed kernel operation. No `_` arm: a `GraphOperation` variant the SDK has not
/// seen does not compile here.
fn kernel_kind(operation: &ekr_kernel::GraphOperation) -> &'static str {
    use ekr_kernel::GraphOperation as G;
    match operation {
        G::CreateNode(_) => "CreateNode",
        G::UpdateProperty(_) => "UpdateProperty",
        G::CreateEdge(_) => "CreateEdge",
        G::DeleteEdge(_) => "DeleteEdge",
        G::AddAssertion(_) => "AddAssertion",
        G::RetractAssertion(_) => "RetractAssertion",
        G::DefineNodeType(_) => "DefineNodeType",
        G::DefineEdgeType(_) => "DefineEdgeType",
        G::ModifyProperty(_) => "ModifyProperty",
        G::MergeEntity(_) => "MergeEntity",
        G::Invoke { .. } => "Invoke",
        G::SupersedeAssertion(_) => "SupersedeAssertion",
        G::AddEvidence(_) => "AddEvidence",
        G::WidenEdgeType(_) => "WidenEdgeType",
    }
}

/// The kernel's reader on `yaml`, which must accept it.
fn kernel_reads(yaml: &str) -> ekr_kernel::TransactionDocument {
    ekr_kernel::TransactionDocument::parse(yaml.as_bytes())
        .unwrap_or_else(|e| panic!("the kernel refuses an SDK transaction document: {e}\n{yaml}"))
}

/// One operation of `kind`, built through the SDK. No `_` arm: a kind the SDK lists has a builder.
fn example(kind: OperationKind, operator: AgentId) -> Operation {
    let root = GraphRootId::mint();
    let type_id = TypeId::mint();
    let at = Timestamp::from_millis(1_554_076_800_000);
    match kind {
        OperationKind::CreateNode => NodeDraft::new(root, type_id, "A Field Guide to Lichens")
            .with_property(PropertyId::mint(), Value::String("A Field Guide".into()))
            .with_property(PropertyId::mint(), Value::Decimal("24.90".into()))
            .with_alias("field-guide")
            .into(),
        OperationKind::UpdateProperty => PropertyMutation::new(
            NodeId::mint(),
            PropertyId::mint(),
            vec![Value::Integer(224)],
        )
        .into(),
        OperationKind::CreateEdge => EdgeDraft::new(root, type_id, NodeId::mint(), NodeId::mint())
            .with_property(PropertyId::mint(), Value::Enum("sole".into()))
            .into(),
        OperationKind::DeleteEdge => Operation::DeleteEdge(EdgeId::mint()),
        OperationKind::AddAssertion => Assertion::new(
            root,
            Subject::Node(NodeId::mint()),
            Predicate::Property(PropertyId::mint()),
            Object::Value(Value::Record(
                [
                    ("height".to_owned(), Value::Integer(210)),
                    (
                        "tags".to_owned(),
                        Value::List(vec![Value::String("true".into())]),
                    ),
                ]
                .into(),
            )),
            operator,
        )
        .citing(EvidenceId::mint())
        .with_valid_time(TemporalRange::since(at))
        .into(),
        OperationKind::RetractAssertion => {
            Retraction::new(AssertionId::mint(), "the count was wrong").into()
        }
        OperationKind::DefineNodeType => NodeType::new("Shelf")
            .with_parent(TypeId::mint())
            .with_property(PropertyDefinition::new(
                "position",
                ValueType::Record(
                    [
                        ("row".to_owned(), ValueType::Integer),
                        (
                            "labels".to_owned(),
                            ValueType::List(Box::new(ValueType::String)),
                        ),
                    ]
                    .into(),
                ),
            ))
            .with_lifecycle(Lifecycle::new(
                "open",
                ["open", "closed"],
                [Transition::new("open", "closed")],
            ))
            .with_operation(
                OperationDefinition::new("close")
                    .with_argument("reason", ValueType::String)
                    .with_transition(Transition::new("open", "closed")),
            )
            .into(),
        OperationKind::DefineEdgeType => EdgeType::new("HOLDS", [type_id], [TypeId::mint()])
            .with_cardinality(Cardinality::Many)
            .with_property(PropertyDefinition::new(
                "since",
                ValueType::NodeRef {
                    allowed_types: [type_id].into(),
                },
            ))
            .into(),
        OperationKind::ModifyProperty => PropertyModification::new(
            type_id,
            PropertyDefinition::new(
                "binding",
                ValueType::Enum {
                    variants: ["hardcover".to_owned(), "paperback".to_owned()].into(),
                },
            )
            .with_cardinality(Cardinality::Many)
            .as_required(),
        )
        .into(),
        OperationKind::MergeEntity => EntityMerge::new(NodeId::mint(), NodeId::mint()).into(),
        OperationKind::Invoke => Invocation::new(NodeId::mint(), "publish")
            .with_argument("imprint", Value::String("Riverside Nature Press".into()))
            .into(),
        OperationKind::SupersedeAssertion => {
            Supersession::new(AssertionId::mint(), AssertionId::mint(), at).into()
        }
        OperationKind::AddEvidence => EvidenceAddition::new(
            EvidenceSource::human("Library catalogue desk"),
            operator,
            at,
            Confidence::new(9_000).expect("9000 basis points is a confidence"),
            b"Bob is CEO of Acme.\n".to_vec(),
        )
        .into(),
        OperationKind::WidenEdgeType => {
            EdgeWidening::new(type_id, [TypeId::mint()], [TypeId::mint(), TypeId::mint()]).into()
        }
    }
}

/// The kinds `ekr operations` lists, by the first word of each line.
fn listed_by_ekr_operations() -> BTreeSet<String> {
    ok(&["operations"], b"")
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

/// The kinds `ekr schema ekr.transaction-document/2` admits, by their `!Kind` tag.
fn listed_by_ekr_schema() -> BTreeSet<String> {
    let schema = json(&["schema", sdk::TRANSACTION_FORMAT], b"");
    let branches = schema["$defs"]["GraphOperation"]["oneOf"]
        .as_array()
        .expect("the transaction schema defines GraphOperation as oneOf");
    let kinds: BTreeSet<String> = branches
        .iter()
        .filter_map(|branch| branch["properties"].as_object())
        .flat_map(|properties| properties.keys())
        .filter_map(|key| key.strip_prefix('!'))
        .map(str::to_owned)
        .collect();
    assert!(
        kinds.len() >= 14,
        "the schema scan is broken, not the SDK: {kinds:?}"
    );
    kinds
}

fn sdk_kinds() -> BTreeSet<String> {
    OperationKind::ALL
        .iter()
        .map(|kind| kind.name().to_owned())
        .collect()
}

#[test]
fn every_operation_kind_ekr_lists_has_a_builder() {
    let sdk = sdk_kinds();
    for (source, listed) in [
        ("ekr operations", listed_by_ekr_operations()),
        ("ekr schema", listed_by_ekr_schema()),
    ] {
        let without_builder: Vec<&String> = listed.difference(&sdk).collect();
        assert!(
            without_builder.is_empty(),
            "{source} lists operation kinds the SDK has no builder for: {without_builder:?}"
        );
        let unknown: Vec<&String> = sdk.difference(&listed).collect();
        assert!(
            unknown.is_empty(),
            "the SDK builds operation kinds {source} does not list: {unknown:?}"
        );
    }
    for kind in OperationKind::ALL {
        assert_eq!(example(kind, AgentId::mint()).kind(), kind);
    }
}

#[test]
fn every_builder_output_is_read_by_the_kernel_and_passes_ekr_schema() {
    let operator = AgentId::mint();
    for kind in OperationKind::ALL {
        let document = TransactionBuilder::new(operator)
            .push(example(kind, operator))
            .build()
            .unwrap_or_else(|e| panic!("{kind:?}: {e}"));
        let yaml = document.to_yaml().expect("an SDK document writes");
        assert!(
            yaml.contains(&format!("- !{}", kind.name())),
            "{kind:?} is written as a tag:\n{yaml}"
        );
        assert!(yaml.ends_with('\n'));
        let read = kernel_reads(&yaml);
        let operations = &read.transaction().operations;
        assert_eq!(operations.len(), 1);
        assert_eq!(kernel_kind(&operations[0]), kind.name());
        assert_eq!(read.format().name(), sdk::TRANSACTION_FORMAT);
        assert_eq!(
            read.transaction().schema_version.is_some(),
            kind.is_schema_change(),
            "{kind:?}: a schema change names the version it produces, nothing else does"
        );
        assert_eq!(
            read.transaction().id.to_string(),
            document.transaction.id.to_string()
        );
        schema_accepts(sdk::TRANSACTION_FORMAT, &yaml);
    }
}

#[test]
fn the_transaction_builder_derives_the_evidence_manifest_and_refuses_what_every_profile_refuses() {
    let operator = AgentId::mint();
    let cited = EvidenceId::mint();
    let assertion = Assertion::new(
        GraphRootId::mint(),
        Subject::Node(NodeId::mint()),
        Predicate::Relation(TypeId::mint()),
        Object::Node(NodeId::mint()),
        operator,
    )
    .citing(cited)
    .with_id(AssertionId::mint());
    let id = TransactionId::mint();
    let document: TransactionDocument = TransactionBuilder::new(operator)
        .with_id(id)
        .push(assertion.into())
        .push(Operation::DeleteEdge(EdgeId::mint()))
        .build()
        .expect("a data transaction builds");
    let transaction: &Transaction = &document.transaction;
    assert_eq!(transaction.id, id);
    assert_eq!(transaction.evidence, [cited].into());
    assert_eq!(transaction.schema_version, None);
    assert_eq!(document.format, sdk::TRANSACTION_FORMAT);
    assert!(!Operation::DeleteEdge(EdgeId::mint()).is_schema_change());

    let empty = TransactionBuilder::new(operator).build();
    assert!(
        matches!(empty, Err(DocumentError::EmptyTransaction)),
        "{empty:?}"
    );
    let mixed = TransactionBuilder::new(operator)
        .push(example(OperationKind::DefineNodeType, operator))
        .push(example(OperationKind::DeleteEdge, operator))
        .build();
    assert!(
        matches!(mixed, Err(DocumentError::MixedSchemaTransaction)),
        "{mixed:?}"
    );
}

#[test]
fn the_seed_builder_files_every_record_under_its_id_inside_the_graph_document_envelope() {
    let operator = AgentId::mint();
    let version = SchemaVersion::seed(Timestamp::EPOCH);
    assert_eq!((version.number, version.parent), (0, None));
    let author = NodeType::new("Author");
    let section = OntologySection {
        version: version.clone(),
        node_types: vec![author.clone()],
        edge_types: Vec::new(),
    };
    let builder = SeedBuilder::new(section, Timestamp::from_millis(7));
    let root = builder.root_id();
    let node_id = NodeId::mint();
    let node = NodeDraft::new(root, author.id, "Ada")
        .with_id(node_id)
        .with_alias("ada");
    let edge = EdgeDraft::new(root, TypeId::mint(), node_id, node_id).with_id(EdgeId::mint());
    let certain = Confidence::new(10_000).expect("10000 basis points is a confidence");
    assert_eq!(certain, Confidence::CERTAIN);
    assert_eq!(certain.basis_points(), 10_000);
    assert_eq!(Confidence::new(10_001), None);
    let evidence = EvidenceAddition::new(
        EvidenceSource::human("desk"),
        operator,
        Timestamp::EPOCH,
        certain,
        b"Ada\n".to_vec(),
    );
    let claim = Assertion::new(
        root,
        Subject::Type(author.id),
        Predicate::Property(PropertyId::mint()),
        Object::Type(author.id),
        operator,
    )
    .citing(evidence.evidence.id);
    assert_eq!(claim.assessment, Assessment::Proposed);
    assert_eq!(claim.lifecycle, AssertionLifecycle::Active);
    assert_eq!(claim.transaction_time, TransactionTime::UNRECORDED);
    assert_eq!(claim.valid_time, TemporalRange::UNBOUNDED);
    assert_eq!(
        TemporalRange::new(
            Some(Timestamp::from_millis(10)),
            Some(Timestamp::from_millis(5))
        ),
        None
    );
    let seed = builder
        .node(node, Some("draft".into()))
        .edge(edge.clone())
        .evidence(evidence.clone())
        .assertion(claim.clone())
        .build();

    let ontology: &OntologySection = &seed.ontology;
    assert_eq!(ontology.version, version);
    let graph: &GraphSection = &seed.graph;
    assert_eq!(graph.format, sdk::GRAPH_FORMAT);
    let inner: &SeedGraph = &graph.graph;
    let root_record: &GraphRoot = &inner.root;
    assert_eq!(root_record.id, root);
    assert_eq!(root_record.space, Space::Canonical);
    assert_eq!(root_record.schema_version_id, version.id);
    assert_eq!(root_record.parent, None);
    assert_eq!(root_record.created_at, Timestamp::from_millis(7));
    let seeded: &SeedNode = &inner.nodes[&node_id];
    assert_eq!(seeded.aliases, ["ada"]);
    assert_eq!(seeded.type_state.as_deref(), Some("draft"));
    assert_eq!(inner.edges[&edge.id], edge);
    assert_eq!(inner.assertions[&claim.id], claim);
    assert_eq!(inner.evidence[&evidence.evidence.id], evidence.evidence);
    assert_eq!(
        seed.evidence_payloads[&evidence.evidence.content_hash],
        b"Ada\n".to_vec()
    );
    let yaml = seed.to_yaml().unwrap();
    ekr_kernel::SeedDocument::from_yaml(&yaml)
        .unwrap_or_else(|e| panic!("the kernel refuses an SDK seed: {e}\n{yaml}"));
    schema_accepts(sdk::SEED_FORMAT, &yaml);

    let version_id = SchemaVersionId::mint();
    let change = TransactionBuilder::new(operator)
        .with_schema_version(version_id)
        .push(NodeType::new("Shelf").as_abstract().into())
        .build()
        .unwrap();
    assert_eq!(change.transaction.schema_version, Some(version_id));
    let read = kernel_reads(&change.to_yaml().unwrap());
    assert_eq!(
        read.transaction().schema_version.map(|id| id.to_string()),
        Some(version_id.to_string())
    );
}

#[test]
fn the_readers_refuse_the_one_key_json_form_of_a_tag_so_the_sdk_writes_tags() {
    let operator = AgentId::mint();
    let document = TransactionBuilder::new(operator)
        .push(example(OperationKind::AddAssertion, operator))
        .build()
        .unwrap();
    let yaml = document.to_yaml().unwrap();
    assert!(yaml.contains("!AddAssertion"), "{yaml}");
    assert!(yaml.contains("!Node "), "{yaml}");
    assert!(!yaml.contains("\"!"), "{yaml}");
    kernel_reads(&yaml);

    let json_form = serde_json::to_string_pretty(&projection(&yaml)).unwrap();
    assert!(json_form.contains("\"!AddAssertion\""), "{json_form}");
    assert!(
        ekr_kernel::TransactionDocument::parse(json_form.as_bytes()).is_err(),
        "the kernel reads the one-key JSON form of a tag: the SDK's writer would be unnecessary"
    );

    let (seed, _) = small_spec().seed(Timestamp::EPOCH).unwrap();
    let seed = seed
        .evidence(EvidenceAddition::new(
            EvidenceSource::human("desk"),
            operator,
            Timestamp::EPOCH,
            Confidence::CERTAIN,
            b"a statement\n".to_vec(),
        ))
        .build();
    let seed_yaml = to_yaml(&seed).unwrap();
    assert!(seed_yaml.contains("!HumanStatement"), "{seed_yaml}");
    ekr_kernel::SeedDocument::from_yaml(&seed_yaml)
        .unwrap_or_else(|e| panic!("the kernel refuses an SDK seed: {e}\n{seed_yaml}"));
    let seed_json = serde_json::to_string(&projection(&seed_yaml)).unwrap();
    assert!(
        ekr_kernel::SeedDocument::from_yaml(&seed_json).is_err(),
        "the seed reader reads the one-key JSON form of a tag"
    );
}

#[test]
fn a_local_hash_equals_ekr_hash_for_the_same_bytes_trailing_newline_included() {
    let mut seen = BTreeSet::new();
    for bytes in [
        &b"The Field Naturalist wrote A Field Guide to Lichens.\n"[..],
        &b"The Field Naturalist wrote A Field Guide to Lichens."[..],
        &b""[..],
        "caf\u{e9}\r\n".as_bytes(),
    ] {
        let printed = json(&["hash", "-"], bytes);
        let local = payload_hash(bytes);
        assert_eq!(
            printed["content_hash"].as_str(),
            Some(local.to_hex().as_str()),
            "{bytes:?}"
        );
        assert!(seen.insert(local.to_hex()), "two payloads, one hash");
    }
    assert_eq!(
        payload_hash(b"wrote.txt\n"),
        Evidence::for_payload(
            b"wrote.txt\n",
            EvidenceSource::human("desk"),
            AgentId::mint(),
            Timestamp::EPOCH,
            Confidence::CERTAIN,
        )
        .content_hash
    );
}

/// The workspace root, read at run time: the invoking checkout, never the compiling one.
fn workspace_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    Path::new(&manifest)
        .ancestors()
        .nth(2)
        .expect("crates/ekr-sdk sits two levels below the workspace root")
        .to_path_buf()
}

#[test]
fn a_local_mint_is_the_function_ekr_mint_runs() {
    let printed = json(&["mint", "node"], b"");
    let from_ekr: NodeId = printed["id"].as_str().unwrap().parse().unwrap();
    let local = NodeId::mint();
    assert_ne!(local, from_ekr);
    assert_eq!(from_ekr.to_uuid().get_version_num(), 7);
    assert_eq!(local.to_uuid().get_version_num(), 7);
    // AGENTS.md says `ekr mint` runs `Id::mint()`: hold the sentence to the verb's source.
    let agent = std::fs::read_to_string(workspace_root().join("crates/ekr/src/cli/agent.rs"))
        .expect("the mint verb's source");
    for call in [
        "NodeId::mint()",
        "EdgeId::mint()",
        "AssertionId::mint()",
        "TransactionId::mint()",
        "EvidenceId::mint()",
        "TypeId::mint()",
        "PropertyId::mint()",
        "AgentId::mint()",
        "GraphRootId::mint()",
        "SchemaVersionId::mint()",
    ] {
        assert!(agent.contains(call), "`ekr mint` no longer calls {call}");
    }
    let agents = std::fs::read_to_string(workspace_root().join("AGENTS.md")).unwrap();
    assert!(
        agents.contains("`Id::mint()`"),
        "AGENTS.md names ekr-core's `Id::mint()` beside `ekr mint`"
    );
}

#[test]
fn the_sdk_links_no_kernel_store_or_graph_and_no_tokio() {
    let path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    let manifest: toml::Table = text.parse().expect("the SDK's manifest is TOML");
    let normal = manifest["dependencies"]
        .as_table()
        .expect("the SDK declares dependencies");
    let workspace_crates: Vec<&String> = normal
        .keys()
        .filter(|name| name.starts_with("ekr"))
        .collect();
    assert_eq!(
        workspace_crates,
        ["ekr-core"],
        "the SDK depends on ekr-core alone among this workspace's crates"
    );
    for forbidden in ["ekr-kernel", "ekr-store", "ekr-graph"] {
        assert!(
            !normal.contains_key(forbidden),
            "{forbidden} is a normal dependency"
        );
    }
    let mut tables: Vec<(&str, &toml::Table)> = Vec::new();
    for section in ["dependencies", "build-dependencies"] {
        if let Some(table) = manifest.get(section).and_then(toml::Value::as_table) {
            tables.push((section, table));
        }
    }
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for target in targets.values().filter_map(toml::Value::as_table) {
            for section in ["dependencies", "build-dependencies"] {
                if let Some(table) = target.get(section).and_then(toml::Value::as_table) {
                    tables.push((section, table));
                }
            }
        }
    }
    for (section, table) in tables {
        assert!(!table.contains_key("tokio"), "tokio in {section}");
        for forbidden in ["ekr-kernel", "ekr-store", "ekr-graph"] {
            assert!(!table.contains_key(forbidden), "{forbidden} in {section}");
        }
    }
    let features = manifest
        .get("features")
        .map(ToString::to_string)
        .unwrap_or_default();
    assert!(
        !features.contains("tokio"),
        "a feature names tokio: {features}"
    );
}

// ----- a store -----

/// A file store under a host of one validation profile, in a scratch directory.
struct Store {
    _scratch: tempfile::TempDir,
    host: String,
    store: String,
    operator: AgentId,
}

impl Store {
    fn new(profile: ValidationProfile) -> Self {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let operator = AgentId::mint();
        let validator = AgentId::mint();
        let (ruleset, application) = match profile {
            ValidationProfile::V1 => ("ekr.p1-deterministic/1", "ekr.p1-apply/1"),
            ValidationProfile::V2 => ("ekr.p2-deterministic/1", "ekr.p2-apply/1"),
            ValidationProfile::V3 => ("ekr.p3-deterministic/1", "ekr.p2-apply/1"),
        };
        let agent = |id: AgentId, name: &str, capabilities: &[&str]| serde_json::json!({"id": id.to_string(), "name": name, "capabilities": capabilities});
        let host = serde_json::json!({
            "format": "ekr.cli-host/1",
            "tenant": "library",
            "context": {"operator": operator.to_string(), "validator": validator.to_string()},
            "authority": {
                "format": "ekr.authority-state/1",
                "agents": {
                    operator.to_string(): agent(operator, "Catalogue operator", &["propose", "read"]),
                    validator.to_string(): agent(validator, "Catalogue validator", &["validate"]),
                },
                "validation_profile": {
                    "format": "ekr.p1-validation-profile/1",
                    "ruleset": ruleset,
                    "checks": ["Structural", "Reference", "Type", "Cardinality",
                               "OntologyConstraint", "Provenance", "Authorization"],
                    "validator": validator.to_string(),
                    "proposer_separation": "distinct-authenticated-actor/1",
                    "provenance": "retained-admissible-evidence/1",
                    "application": application
                }
            }
        });
        let host_path = scratch.path().join("host.json");
        std::fs::write(&host_path, host.to_string()).unwrap();
        Self {
            host: host_path.to_str().unwrap().to_owned(),
            store: scratch.path().join("store").to_str().unwrap().to_owned(),
            _scratch: scratch,
            operator,
        }
    }

    fn run(&self, args: &[&str], stdin: &[u8]) -> Result<String, Failure> {
        let mut argv = vec![
            "--host",
            self.host.as_str(),
            "--store",
            self.store.as_str(),
            "--backend",
            "file",
        ];
        argv.extend_from_slice(args);
        ekr(&argv, stdin)
    }

    fn json(&self, args: &[&str], stdin: &[u8]) -> Json {
        let out = self
            .run(args, stdin)
            .unwrap_or_else(|failure| panic!("ekr {args:?}: {failure}"));
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("ekr {args:?}: not JSON ({e}): {out}"))
    }

    fn seed(&self, seed: &SeedDocument) {
        let yaml = seed.to_yaml().expect("an SDK seed writes");
        ekr_kernel::SeedDocument::from_yaml(&yaml)
            .unwrap_or_else(|e| panic!("the kernel refuses an SDK seed: {e}\n{yaml}"));
        schema_accepts(sdk::SEED_FORMAT, &yaml);
        let seeded = self.json(&["seed", "-"], yaml.as_bytes());
        assert_eq!(seeded["result"]["revision"], 0, "{seeded}");
    }

    /// Propose, validate and commit one SDK document; the commit's outcome.
    fn commit(&self, document: &TransactionDocument) -> Json {
        let yaml = document.to_yaml().expect("an SDK document writes");
        kernel_reads(&yaml);
        schema_accepts(sdk::TRANSACTION_FORMAT, &yaml);
        let proposed = self.json(&["propose", "-"], yaml.as_bytes());
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        assert_eq!(id, document.transaction.id.to_string());
        let validated = self.json(&["validate", &id], b"");
        assert_eq!(validated["kind"], "Validated", "{validated:#}\n{yaml}");
        let committed = self.json(&["commit", &id], b"");
        assert_eq!(committed["kind"], "Committed", "{committed:#}");
        committed
    }

    fn ontology(&self) -> Ontology {
        let printed = self
            .run(&["ontology"], b"")
            .unwrap_or_else(|failure| panic!("ekr ontology: {failure}"));
        Ontology::read(&printed).unwrap_or_else(|e| panic!("{e}: {printed}"))
    }
}

// ----- the ontology by name -----

fn small_spec() -> OntologySpec {
    OntologySpec::new()
        .with_node_type(NodeTypeSpec::new("Author"))
        .with_node_type(
            NodeTypeSpec::new("Book")
                .with_property(PropertySpec::new("title", ValueSpec::String).as_required()),
        )
        .with_edge_type(
            EdgeTypeSpec::new("WROTE", ["Author"], ["Book"]).with_cardinality(Cardinality::Many),
        )
}

/// `small_spec` grown by every kind of schema change: a node type, an edge type, a property on a
/// type the store has, and a widened edge end.
fn grown_spec() -> OntologySpec {
    grown_spec_with(Cardinality::One)
}

/// [`grown_spec`] with `isbn` of the cardinality given.
fn grown_spec_with(isbn: Cardinality) -> OntologySpec {
    OntologySpec::new()
        .with_node_type(NodeTypeSpec::new("Author"))
        .with_node_type(
            NodeTypeSpec::new("Book")
                .with_property(PropertySpec::new("title", ValueSpec::String).as_required())
                .with_property(PropertySpec::new("isbn", ValueSpec::String).with_cardinality(isbn)),
        )
        .with_node_type(
            NodeTypeSpec::new("Journal")
                .with_property(PropertySpec::new("issn", ValueSpec::String))
                .with_property(PropertySpec::new(
                    "successor_of",
                    ValueSpec::NodeRef(vec!["Journal".into()]),
                )),
        )
        .with_edge_type(
            EdgeTypeSpec::new("WROTE", ["Author"], ["Book", "Journal"])
                .with_cardinality(Cardinality::Many),
        )
        .with_edge_type(
            EdgeTypeSpec::new("EDITED", ["Author"], ["Journal"])
                .with_cardinality(Cardinality::Many)
                .with_property(PropertySpec::new(
                    "role",
                    ValueSpec::Enum(vec!["chief".into(), "guest".into()]),
                )),
        )
}

fn kinds(change: &SchemaChange) -> BTreeSet<OperationKind> {
    change.operations().iter().map(Operation::kind).collect()
}

#[test]
fn ensure_against_a_store_whose_ontology_matches_emits_no_operation_and_otherwise_evolves_it() {
    let store = Store::new(ValidationProfile::V2);
    let (seed, names) = small_spec().seed(Timestamp::EPOCH).unwrap();
    store.seed(&seed.build());

    let read = store.ontology();
    assert_eq!(read.node_type("Book"), names.node_type("Book"));
    assert_eq!(read.edge_type("WROTE"), names.edge_type("WROTE"));
    assert_eq!(
        read.property("Book", "title"),
        names.property("Book", "title")
    );
    assert!(read.property("Book", "title").is_some());
    assert_eq!(read.schema_version(), names.schema_version());
    for profile in [
        ValidationProfile::V1,
        ValidationProfile::V2,
        ValidationProfile::V3,
    ] {
        let change = read.ensure(&small_spec(), profile).unwrap();
        assert!(change.is_empty(), "{profile:?}: {:?}", change.operations());
        assert!(change.transaction(store.operator).unwrap().is_none());
    }

    let fixed = read.ensure(&grown_spec(), ValidationProfile::V1);
    assert!(
        matches!(fixed, Err(OntologyError::SchemaFixed { .. })),
        "{fixed:?}"
    );
    assert!(!ValidationProfile::V1.admits_schema_changes());
    assert!(ValidationProfile::V3.admits_schema_changes());

    let change = read.ensure(&grown_spec(), ValidationProfile::V2).unwrap();
    assert_eq!(
        kinds(&change),
        [
            OperationKind::DefineNodeType,
            OperationKind::DefineEdgeType,
            OperationKind::ModifyProperty,
            OperationKind::WidenEdgeType
        ]
        .into(),
        "{:?}",
        change.operations()
    );
    let journal = change
        .ontology()
        .node_type("Journal")
        .expect("Journal is minted");
    let document = change
        .transaction(store.operator)
        .unwrap()
        .expect("a change writes one transaction");
    let version = document
        .transaction
        .schema_version
        .expect("it names its version");
    store.commit(&document);

    let evolved = store.ontology();
    assert_eq!(evolved.node_type("Journal"), Some(journal));
    assert_eq!(evolved.schema_version(), Some(version));
    assert_eq!(
        evolved.property("Book", "isbn"),
        change.ontology().property("Book", "isbn")
    );
    assert_eq!(
        evolved.edge_type("EDITED"),
        change.ontology().edge_type("EDITED")
    );
    let again = evolved
        .ensure(&grown_spec(), ValidationProfile::V2)
        .unwrap();
    assert!(again.is_empty(), "{:?}", again.operations());

    // A declaration that differs is redeclared under the id the store has.
    let isbn = evolved.property("Book", "isbn").unwrap();
    let many = grown_spec_with(Cardinality::Many);
    let redeclared = evolved.ensure(&many, ValidationProfile::V2).unwrap();
    match redeclared.operations() {
        [Operation::ModifyProperty(modification)] => {
            assert_eq!(modification.property.id, isbn);
            assert_eq!(Some(modification.owner), evolved.node_type("Book"));
        }
        other => panic!("one redeclaration: {other:?}"),
    }
    store.commit(&redeclared.transaction(store.operator).unwrap().unwrap());
    assert!(store
        .ontology()
        .ensure(&many, ValidationProfile::V2)
        .unwrap()
        .is_empty());
}

#[test]
fn ensure_refuses_what_no_schema_change_can_make_true_and_names_it() {
    let (_, read) = small_spec().seed(Timestamp::EPOCH).unwrap();
    let unknown = read.ensure(
        &small_spec().with_edge_type(EdgeTypeSpec::new("SHELVED", ["Book"], ["Nowhere"])),
        ValidationProfile::V2,
    );
    assert!(
        matches!(&unknown, Err(OntologyError::UnknownName { name, .. }) if name == "Nowhere"),
        "{unknown:?}"
    );
    let now_abstract =
        OntologySpec::new().with_node_type(NodeTypeSpec::new("Author").as_abstract());
    let conflict = read.ensure(&now_abstract, ValidationProfile::V2);
    assert!(
        matches!(&conflict, Err(OntologyError::Conflict { name, .. }) if name == "Author"),
        "{conflict:?}"
    );
    let twice = OntologySpec::new()
        .with_node_type(NodeTypeSpec::new("Author"))
        .with_node_type(NodeTypeSpec::new("Author"));
    assert!(matches!(
        twice.seed(Timestamp::EPOCH),
        Err(OntologyError::DuplicateName { .. })
    ));
    assert!(matches!(
        Ontology::read("{\"node_types\": 3}"),
        Err(OntologyError::Read(_))
    ));
}

// ----- data documents on a real store -----

#[test]
fn seed_and_data_documents_the_sdk_builds_seed_and_commit_on_a_store() {
    let store = Store::new(ValidationProfile::V3);
    let operator = store.operator;
    let spec = OntologySpec::new()
        .with_node_type(
            NodeTypeSpec::new("Publication")
                .as_abstract()
                .with_property(PropertySpec::new("title", ValueSpec::String).as_required()),
        )
        .with_node_type(
            NodeTypeSpec::new("Book")
                .with_parent("Publication")
                .with_property(PropertySpec::new("in_print", ValueSpec::Boolean))
                .with_property(PropertySpec::new("page_count", ValueSpec::Integer))
                .with_property(PropertySpec::new("list_price", ValueSpec::Decimal))
                .with_property(PropertySpec::new("first_published", ValueSpec::Timestamp))
                .with_property(PropertySpec::new("reading_time_ms", ValueSpec::Duration))
                .with_property(PropertySpec::new(
                    "translation_of",
                    ValueSpec::NodeRef(vec!["Book".into()]),
                ))
                .with_property(PropertySpec::new(
                    "binding",
                    ValueSpec::Enum(vec!["hardcover".into(), "paperback".into()]),
                ))
                .with_property(
                    PropertySpec::new("subjects", ValueSpec::String)
                        .with_cardinality(Cardinality::Many),
                )
                .with_property(PropertySpec::new(
                    "chapter_titles",
                    ValueSpec::List(Box::new(ValueSpec::String)),
                ))
                .with_property(PropertySpec::new(
                    "dimensions_mm",
                    ValueSpec::Record(
                        [
                            ("height".to_owned(), ValueSpec::Integer),
                            ("width".to_owned(), ValueSpec::Integer),
                        ]
                        .into(),
                    ),
                ))
                .with_property(PropertySpec::new("weight_grams", ValueSpec::Float)),
        )
        .with_node_type(NodeTypeSpec::new("Author"))
        .with_edge_type(
            EdgeTypeSpec::new("WROTE", ["Author"], ["Book"])
                .with_cardinality(Cardinality::Many)
                .with_property(PropertySpec::new(
                    "contribution",
                    ValueSpec::Enum(vec!["sole".into(), "joint".into()]),
                )),
        );
    let (builder, names): (SeedBuilder, Ontology) = spec.seed(Timestamp::EPOCH).unwrap();
    let root = builder.root_id();
    let book_type = names.node_type("Book").unwrap();
    let author_type = names.node_type("Author").unwrap();
    let wrote = names.edge_type("WROTE").unwrap();
    let property = |name: &str| {
        names
            .property("Book", name)
            .unwrap_or_else(|| panic!("Book has {name}"))
    };
    assert_eq!(
        names.property("Book", "title"),
        names.property("Publication", "title")
    );
    let observed = Timestamp::from_millis(1_788_220_800_000);
    let wrote_it = EvidenceAddition::new(
        EvidenceSource::human("Library catalogue desk"),
        operator,
        observed,
        Confidence::CERTAIN,
        b"The Field Naturalist wrote A Field Guide to Lichens.\n".to_vec(),
    );
    let author =
        NodeDraft::new(root, author_type, "The Field Naturalist").with_alias("field-naturalist");
    let book = NodeDraft::new(root, book_type, "A Field Guide to Lichens")
        .with_property(
            property("title"),
            Value::String("A Field Guide to Lichens".into()),
        )
        .with_property(property("in_print"), Value::Boolean(true))
        .with_property(property("page_count"), Value::Integer(212))
        .with_property(property("list_price"), Value::Decimal("24.90".into()))
        .with_property(
            property("first_published"),
            Value::Timestamp(Timestamp::from_millis(1_554_076_800_000)),
        )
        .with_property(property("reading_time_ms"), Value::Duration(21_600_000))
        .with_property(property("binding"), Value::Enum("paperback".into()))
        .with_property(property("subjects"), Value::String("lichens".into()))
        .with_property(property("subjects"), Value::String("true".into()))
        .with_property(
            property("chapter_titles"),
            Value::List(vec![Value::String("Reading a thallus".into())]),
        )
        .with_property(
            property("dimensions_mm"),
            Value::Record(
                [
                    ("height".to_owned(), Value::Integer(210)),
                    ("width".to_owned(), Value::Integer(148)),
                ]
                .into(),
            ),
        );
    let seeded_claim = Assertion::new(
        root,
        Subject::Node(author.id),
        Predicate::Relation(wrote),
        Object::Node(book.id),
        operator,
    )
    .citing(wrote_it.evidence.id)
    .with_valid_time(TemporalRange::since(Timestamp::from_millis(
        1_554_076_800_000,
    )));
    let seeded_edge = EdgeDraft::new(root, wrote, author.id, book.id).with_property(
        names.edge_property("WROTE", "contribution").unwrap(),
        Value::Enum("sole".into()),
    );
    let (author_id, book_id, claim_id) = (author.id, book.id, seeded_claim.id);
    let seed: SeedDocument = builder
        .node(author, None)
        .node(book, None)
        .edge(seeded_edge)
        .evidence(wrote_it)
        .assertion(seeded_claim)
        .build();
    assert_eq!(seed.format, sdk::SEED_FORMAT);
    assert_eq!(seed.evidence_payloads.len(), 1);
    store.seed(&seed);
    assert!(store
        .ontology()
        .ensure(&spec, ValidationProfile::V3)
        .unwrap()
        .is_empty());

    // The typed reference the SDK writes is what `ekr resolve` reads.
    let reference = TypedReference::new(author_type, ["field-naturalist"]);
    let reference_yaml = reference.to_yaml().unwrap();
    schema_accepts("typed-reference", &reference_yaml);
    let resolved = store.json(&["resolve", "-"], reference_yaml.as_bytes());
    assert_eq!(resolved["kind"], "Resolved", "{resolved}");
    assert_eq!(resolved["node_id"], author_id.to_string());
    let unknown = TypedReference::new(author_type, ["moss-collector"])
        .to_yaml()
        .unwrap();
    assert_eq!(
        store.json(&["resolve", "-"], unknown.as_bytes())["kind"],
        "ProposeNew"
    );

    // Every applied data kind but `Invoke`, whose type needs a lifecycle, across two commits.
    let published = EvidenceAddition::new(
        EvidenceSource::human("Library catalogue desk"),
        operator,
        observed,
        Confidence::new(9_000).unwrap(),
        b"A Field Guide to Lichens was first published on 2019-04-01.\n".to_vec(),
    );
    let translation = NodeDraft::new(root, book_type, "A Field Guide to Lichens (translation)")
        .with_property(
            property("title"),
            Value::String("A Field Guide (translation)".into()),
        )
        .with_property(property("translation_of"), Value::NodeRef(book_id))
        .with_alias("field-guide-translation");
    let edge = EdgeDraft::new(root, wrote, author_id, translation.id);
    let claim = Assertion::new(
        root,
        Subject::Node(book_id),
        Predicate::Property(property("first_published")),
        Object::Value(Value::Timestamp(Timestamp::from_millis(1_554_076_800_000))),
        operator,
    )
    .citing(published.evidence.id);
    let (edge_id, claim_id_2, published_id) = (edge.id, claim.id, published.evidence.id);
    store.commit(
        &TransactionBuilder::new(operator)
            .push(translation.into())
            .push(edge.into())
            .push(published.into())
            .push(claim.into())
            .push(
                PropertyMutation::new(book_id, property("page_count"), vec![Value::Integer(224)])
                    .into(),
            )
            .build()
            .unwrap(),
    );

    let effective = Timestamp::from_millis(1_600_000_000_000);
    let replacement = Assertion::new(
        root,
        Subject::Node(book_id),
        Predicate::Property(property("first_published")),
        Object::Value(Value::Timestamp(Timestamp::from_millis(1_554_163_200_000))),
        operator,
    )
    .citing(published_id)
    .with_valid_time(TemporalRange::new(Some(effective), None).unwrap());
    let replacement_id = replacement.id;
    let committed = store.commit(
        &TransactionBuilder::new(operator)
            .push(replacement.into())
            .push(Supersession::new(claim_id_2, replacement_id, effective).into())
            .push(Retraction::new(claim_id, "the seeded claim was checked and withdrawn").into())
            .push(Operation::DeleteEdge(edge_id))
            .build()
            .unwrap(),
    );
    assert_eq!(committed["result"]["revision"], 2, "{committed}");
    let snapshot = store.json(&["snapshot"], b"");
    let assertions = &snapshot["graph"]["graph"]["assertions"];
    assert!(
        assertions[replacement_id.to_string()].is_object(),
        "{snapshot:#}"
    );
    assert!(snapshot["graph"]["graph"]["edges"][edge_id.to_string()].is_null());
}
