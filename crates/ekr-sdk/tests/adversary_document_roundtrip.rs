//! Adversary cases for `story:sdk-typed-documents`: what the SDK builds must be what the kernel
//! reads, field for field, and what a builder returns `Ok` for must be a document the kernel reads.

use std::collections::BTreeMap;

use ekr::exit::Failure;
use ekr_sdk::document::{
    payload_hash, AgentId, Assertion, AssertionId, Cardinality, Confidence, EdgeTypeSpec,
    EvidenceAddition, EvidenceId, EvidenceSource, GraphRootId, Invocation, Lifecycle, NodeDraft,
    NodeId, NodeType, NodeTypeSpec, Object, Ontology, OntologyError, OntologySpec,
    OperationDefinition, Predicate, PropertyDefinition, PropertyId, PropertySpec, Retraction,
    Subject, Supersession, TemporalRange, Timestamp, TransactionBuilder, Transition, TypeId,
    ValidationProfile, Value, ValueSpec, ValueType,
};

/// Every text the round trips carry: YAML 1.1 and 1.2 look-alikes, indicators, whitespace at the
/// ends, line breaks of every kind, control characters and text outside the BMP.
fn awkward_strings() -> Vec<String> {
    let mut texts: Vec<String> = [
        "",
        " ",
        "yes",
        "no",
        "Yes",
        "NO",
        "on",
        "Off",
        "y",
        "n",
        "~",
        "null",
        "Null",
        "NULL",
        "true",
        "False",
        "1e3",
        "1E3",
        "0x1F",
        "0o17",
        "017",
        "+1",
        "-0",
        ".5",
        "1_000",
        "-.inf",
        ".nan",
        ".NaN",
        "12:30",
        "1:2:3",
        "a: b",
        "key:",
        "#hash",
        "a #b",
        "- dash",
        "-",
        "? q",
        "?",
        ":",
        "!Node x",
        "!",
        "&anchor",
        "*alias",
        "%dir",
        "@at",
        "`bt",
        "|",
        "> fold",
        "'single",
        "\"double",
        "[x]",
        "{x}",
        "a,b",
        " leading",
        "trailing ",
        "  both  ",
        "line1\nline2",
        "trailing\n",
        "two\n\n",
        "\n",
        "\nlead",
        "  indented\nline",
        "line \nspace",
        "a\r\nb",
        "a\rb",
        "tab\there",
        "\t",
        "nul\0byte",
        "bell\u{7}",
        "del\u{7f}",
        "nel\u{85}x",
        "ls\u{2028}x",
        "ps\u{2029}x",
        "\u{feff}bom",
        "emoji \u{1F600}",
        "e\u{301}",
        "rtl \u{202e}x",
        "nbsp\u{a0}x",
        "\u{200b}",
        "\u{fffd}",
        "\u{10ffff}",
        "---",
        "...",
        "--- x",
        "=",
        "<<",
        "2026-09-29",
        "2026-09-29T15:13:55Z",
        "0",
        "00",
        "1.0",
        "24.90",
        "-24.90",
        "0.1e-5",
        "\\",
        "\\n",
        "a\\\"b",
    ]
    .iter()
    .map(|text| (*text).to_owned())
    .collect();
    texts.push("k".repeat(200));
    texts.push(format!("{}\n{}", "x".repeat(130), " y"));
    texts
}

/// The kernel's reader on an SDK transaction document, which must accept it and read back exactly
/// the transaction the SDK built.
fn same_transaction_after_the_kernel_reads_it(document: &ekr_sdk::document::TransactionDocument) {
    let yaml = document.to_yaml().expect("an SDK document writes");
    let read = ekr_kernel::TransactionDocument::parse(yaml.as_bytes())
        .unwrap_or_else(|e| panic!("the kernel refuses an SDK document: {e}\n{yaml}"));
    let built = serde_yaml_ng::to_value(&document.transaction).expect("the SDK model serializes");
    let kernel = serde_yaml_ng::to_value(read.transaction()).expect("the kernel model serializes");
    assert_eq!(
        kernel,
        built,
        "the kernel reads another transaction than the SDK built:\n{yaml}\nkernel read:\n{}",
        serde_yaml_ng::to_string(read.transaction()).unwrap()
    );
}

#[test]
fn every_awkward_text_the_sdk_writes_is_the_text_the_kernel_reads() {
    let operator = AgentId::mint();
    let root = GraphRootId::mint();
    for text in awkward_strings() {
        let property = PropertyId::mint();
        let record: BTreeMap<String, Value> = [
            (text.clone(), Value::String(text.clone())),
            ("plain".to_owned(), Value::Enum(text.clone())),
        ]
        .into();
        let node = NodeDraft::new(root, TypeId::mint(), text.clone())
            .with_alias(text.clone())
            .with_property(property, Value::String(text.clone()))
            .with_property(property, Value::Decimal(text.clone()))
            .with_property(property, Value::Enum(text.clone()))
            .with_property(property, Value::Record(record.clone()))
            .with_property(property, Value::List(vec![Value::String(text.clone())]));
        let evidence = EvidenceAddition::new(
            EvidenceSource::HumanStatement {
                identity: Some(text.clone()),
            },
            operator,
            Timestamp::EPOCH,
            Confidence::CERTAIN,
            text.as_bytes().to_vec(),
        );
        let claim = Assertion::new(
            root,
            Subject::Node(node.id),
            Predicate::Property(property),
            Object::Value(Value::Record(record)),
            operator,
        )
        .citing(evidence.evidence.id);
        let data = TransactionBuilder::new(operator)
            .push(node.into())
            .push(evidence.into())
            .push(claim.into())
            .push(Retraction::new(AssertionId::mint(), text.clone()).into())
            .push(
                Invocation::new(NodeId::mint(), text.clone())
                    .with_argument(text.clone(), Value::String(text.clone()))
                    .into(),
            )
            .build()
            .unwrap();
        same_transaction_after_the_kernel_reads_it(&data);

        let schema = TransactionBuilder::new(operator)
            .push(
                NodeType::new(text.clone())
                    .with_property(PropertyDefinition::new(
                        text.clone(),
                        ValueType::Record(
                            [(
                                text.clone(),
                                ValueType::Enum {
                                    variants: [text.clone(), "other".to_owned()].into(),
                                },
                            )]
                            .into(),
                        ),
                    ))
                    .with_lifecycle(Lifecycle::new(
                        text.clone(),
                        [text.clone(), "done".to_owned()],
                        [Transition::new(text.clone(), "done")],
                    ))
                    .with_operation(
                        OperationDefinition::new(text.clone())
                            .with_argument(text.clone(), ValueType::String)
                            .with_transition(Transition::new(text.clone(), "done")),
                    )
                    .into(),
            )
            .build()
            .unwrap();
        same_transaction_after_the_kernel_reads_it(&schema);
    }
}

#[test]
fn numbers_and_instants_at_their_bounds_are_the_numbers_the_kernel_reads() {
    let operator = AgentId::mint();
    let root = GraphRootId::mint();
    let property = PropertyId::mint();
    let mut node = NodeDraft::new(root, TypeId::mint(), "bounds");
    for whole in [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX] {
        node = node
            .with_property(property, Value::Integer(whole))
            .with_property(property, Value::Duration(whole))
            .with_property(property, Value::Timestamp(Timestamp::from_millis(whole)));
    }
    for float in [
        0.0,
        1.5,
        -2.25,
        1e20,
        1e-7,
        1e300,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        5e-324,
        123_456_789_012_345_680.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        node = node.with_property(property, Value::Float(float));
    }
    let evidence = EvidenceAddition::new(
        EvidenceSource::HumanStatement { identity: None },
        operator,
        Timestamp::from_millis(i64::MIN),
        Confidence::new(0).unwrap(),
        Vec::new(),
    );
    let claim = Assertion::new(
        root,
        Subject::Node(node.id),
        Predicate::Property(property),
        Object::Value(Value::Timestamp(Timestamp::from_millis(i64::MAX))),
        operator,
    )
    .citing(evidence.evidence.id)
    .with_valid_time(
        TemporalRange::new(
            Some(Timestamp::from_millis(i64::MIN)),
            Some(Timestamp::from_millis(i64::MAX)),
        )
        .unwrap(),
    );
    let document = TransactionBuilder::new(operator)
        .push(node.into())
        .push(evidence.into())
        .push(claim.into())
        .push(
            Supersession::new(
                AssertionId::mint(),
                AssertionId::mint(),
                Timestamp::from_millis(i64::MIN),
            )
            .into(),
        )
        .build()
        .unwrap();
    same_transaction_after_the_kernel_reads_it(&document);
}

/// What the kernel's reader says of `document`, which the builder returned `Ok` for.
fn kernel_verdict(document: &ekr_sdk::document::TransactionDocument) -> Result<(), String> {
    let yaml = document
        .to_yaml()
        .map_err(|e| format!("the SDK writer refused: {e}"))?;
    ekr_kernel::TransactionDocument::parse(yaml.as_bytes())
        .map(|_| ())
        .map_err(|e| format!("{e} ({} bytes of YAML)", yaml.len()))
}

#[test]
fn a_document_the_builder_returns_ok_for_is_one_the_kernel_reads() {
    let operator = AgentId::mint();
    let root = GraphRootId::mint();
    let deletes = |count: usize| {
        (0..count).fold(TransactionBuilder::new(operator), |builder, _| {
            builder.push(ekr_sdk::document::Operation::DeleteEdge(
                ekr_sdk::document::EdgeId::mint(),
            ))
        })
    };
    let evidence = |bytes: usize| {
        EvidenceAddition::new(
            EvidenceSource::human("desk"),
            operator,
            Timestamp::EPOCH,
            Confidence::CERTAIN,
            vec![b'x'; bytes],
        )
    };
    let nested = (0..16).fold(Value::Integer(1), |inner, _| Value::List(vec![inner]));

    // The frozen `/2` bounds themselves are admitted: these must read.
    let at_the_caps = [
        ("10,000 operations", deletes(10_000)),
        (
            "a payload of 16,384 bytes",
            TransactionBuilder::new(operator).push(evidence(16_384).into()),
        ),
    ];
    for (case, builder) in at_the_caps {
        let document = builder.build().expect("builds");
        kernel_verdict(&document).unwrap_or_else(|e| panic!("{case}: {e}"));
    }

    let past_the_caps = [
        ("10,001 operations", deletes(10_001)),
        (
            "a payload of 16,385 bytes (EvidenceAddition says at most 16,384)",
            TransactionBuilder::new(operator).push(evidence(16_385).into()),
        ),
        (
            "60 payloads of 16,384 bytes",
            (0..60).fold(TransactionBuilder::new(operator), |builder, _| {
                builder.push(evidence(16_384).into())
            }),
        ),
        (
            "a value nested 16 lists deep",
            TransactionBuilder::new(operator).push(
                NodeDraft::new(root, TypeId::mint(), "deep")
                    .with_property(PropertyId::mint(), nested)
                    .into(),
            ),
        ),
        (
            "a canonical name of 65,537 bytes",
            TransactionBuilder::new(operator)
                .push(NodeDraft::new(root, TypeId::mint(), "n".repeat(65_537)).into()),
        ),
    ];
    let mut refused = Vec::new();
    for (case, builder) in past_the_caps {
        match builder.build() {
            Err(_) => {}
            Ok(document) => {
                if let Err(refusal) = kernel_verdict(&document) {
                    refused.push(format!(
                        "{case}: build() returned Ok, the kernel says {refusal}"
                    ));
                }
            }
        }
    }
    assert!(
        refused.is_empty(),
        "the builder returns Ok for documents the kernel refuses:\n{}",
        refused.join("\n")
    );
}

// ----- the ontology by name -----

fn map_of(spec: &OntologySpec) -> Ontology {
    spec.seed(Timestamp::EPOCH).expect("the spec seeds").1
}

/// Publication and its subtype Book, neither declaring a property.
fn hierarchy() -> OntologySpec {
    OntologySpec::new()
        .with_node_type(NodeTypeSpec::new("Publication"))
        .with_node_type(NodeTypeSpec::new("Book").with_parent("Publication"))
}

#[test]
fn ensure_gives_one_spec_one_ontology_whatever_order_it_lists_its_types_in() {
    let held = map_of(&hierarchy());
    let isbn = || PropertySpec::new("isbn", ValueSpec::String);
    let parent_first = OntologySpec::new()
        .with_node_type(NodeTypeSpec::new("Publication").with_property(isbn()))
        .with_node_type(
            NodeTypeSpec::new("Book")
                .with_parent("Publication")
                .with_property(isbn()),
        );
    let child_first = OntologySpec::new()
        .with_node_type(
            NodeTypeSpec::new("Book")
                .with_parent("Publication")
                .with_property(isbn()),
        )
        .with_node_type(NodeTypeSpec::new("Publication").with_property(isbn()));

    let shape = |spec: &OntologySpec| {
        let change = held.ensure(spec, ValidationProfile::V2).unwrap();
        let map = change.ontology();
        (
            change.operations().len(),
            map.property("Book", "isbn") == map.property("Publication", "isbn"),
        )
    };
    let (a, b) = (shape(&parent_first), shape(&child_first));
    assert_eq!(
        a, b,
        "one spec, two ontologies: listed parent first it emits {} operation(s) and Book.isbn is \
         Publication.isbn is {}; listed child first it emits {} and it is {}",
        a.0, a.1, b.0, b.1
    );
}

#[test]
fn ensure_refuses_a_new_type_whose_ancestor_declares_its_property_differently() {
    let held = map_of(
        &OntologySpec::new().with_node_type(
            NodeTypeSpec::new("Publication")
                .with_property(PropertySpec::new("title", ValueSpec::String)),
        ),
    );
    let differently = PropertySpec::new("title", ValueSpec::Integer);

    let spec = OntologySpec::new()
        .with_node_type(
            NodeTypeSpec::new("Publication")
                .with_property(PropertySpec::new("title", ValueSpec::String)),
        )
        .with_node_type(
            NodeTypeSpec::new("Book")
                .with_parent("Publication")
                .with_property(differently),
        );
    let result = held.ensure(&spec, ValidationProfile::V2);
    assert!(
        matches!(&result, Err(OntologyError::Conflict { name, .. }) if name == "Book.title"),
        "ensure documents Conflict for `a property an ancestor declares differently`; for a new \
         type it returns {:?}",
        result.map(|change| change.operations().to_vec())
    );
}

#[test]
fn ensure_treats_ends_the_store_already_widens_as_met() {
    let wide = OntologySpec::new()
        .with_node_type(NodeTypeSpec::new("Author"))
        .with_node_type(NodeTypeSpec::new("Book"))
        .with_node_type(NodeTypeSpec::new("Journal"))
        .with_edge_type(
            EdgeTypeSpec::new("WROTE", ["Author"], ["Book", "Journal"])
                .with_cardinality(Cardinality::Many),
        );
    let held = map_of(&wide);
    let narrow = OntologySpec::new()
        .with_node_type(NodeTypeSpec::new("Author"))
        .with_node_type(NodeTypeSpec::new("Book"))
        .with_edge_type(
            EdgeTypeSpec::new("WROTE", ["Author"], ["Book"]).with_cardinality(Cardinality::Many),
        );
    // Schema changes only add, so a spec states a minimum: WROTE reaching Journal as well as Book
    // meets a spec that asks for Book (coordinator's decision, wave sdk-01).
    let result = held.ensure(&narrow, ValidationProfile::V2);
    assert!(
        matches!(&result, Ok(change) if change.operations().is_empty()),
        "a spec the store's wider WROTE already meets needs no operation and no refusal: {:?}",
        result.map(|change| change.operations().to_vec())
    );
}

// ----- the verbs -----

fn ekr(args: &[&str], stdin: &[u8]) -> Result<String, Failure> {
    let mut input = stdin;
    ekr::cli::run(
        std::iter::once("ekr").chain(args.iter().copied()),
        &|| Timestamp::from_millis(1_790_000_000_000),
        &mut input,
    )
}

#[test]
fn a_local_hash_equals_ekr_hash_for_bytes_that_are_not_text() {
    for bytes in [
        &[0xff_u8, 0xfe, 0x00, 0x80][..],
        &[0x00][..],
        &b"\r"[..],
        &b"\r\n"[..],
        &b"\n"[..],
        &[0xef, 0xbb, 0xbf, b'a'][..],
        &[0xc3][..],
    ] {
        let out = ekr(&["hash", "-"], bytes).unwrap_or_else(|f| panic!("{bytes:?}: {f}"));
        let printed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            printed["content_hash"].as_str(),
            Some(payload_hash(bytes).to_hex().as_str()),
            "{bytes:?}"
        );
    }
}

#[test]
fn ids_minted_on_many_threads_at_once_are_distinct_uuid_v7() {
    let handles: Vec<_> = (0..16)
        .map(|_| {
            std::thread::spawn(|| {
                (0..20_000)
                    .map(|_| EvidenceId::mint().to_string())
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let mut all = std::collections::BTreeSet::new();
    for handle in handles {
        for id in handle.join().unwrap() {
            assert_eq!(id.len(), 36, "{id}");
            assert_eq!(&id[14..15], "7", "{id} is not version 7");
            assert_eq!(id, id.to_lowercase());
            assert!(all.insert(id.clone()), "{id} minted twice");
        }
    }
    assert_eq!(all.len(), 16 * 20_000);
}

// ----- a store -----

struct Store {
    _scratch: tempfile::TempDir,
    host: String,
    store: String,
    operator: AgentId,
}

impl Store {
    fn v2() -> Self {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let (operator, validator) = (AgentId::mint(), AgentId::mint());
        let agent = |id: AgentId, name: &str, capabilities: &[&str]| serde_json::json!({"id": id.to_string(), "name": name, "capabilities": capabilities});
        let host = serde_json::json!({
            "format": "ekr.cli-host/1",
            "tenant": "library",
            "context": {"operator": operator.to_string(), "validator": validator.to_string()},
            "authority": {
                "format": "ekr.authority-state/1",
                "agents": {
                    operator.to_string(): agent(operator, "operator", &["propose", "read"]),
                    validator.to_string(): agent(validator, "validator", &["validate"]),
                },
                "validation_profile": {
                    "format": "ekr.p1-validation-profile/1",
                    "ruleset": "ekr.p2-deterministic/1",
                    "checks": ["Structural", "Reference", "Type", "Cardinality",
                               "OntologyConstraint", "Provenance", "Authorization"],
                    "validator": validator.to_string(),
                    "proposer_separation": "distinct-authenticated-actor/1",
                    "provenance": "retained-admissible-evidence/1",
                    "application": "ekr.p2-apply/1"
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

    fn run(&self, args: &[&str], stdin: &[u8]) -> String {
        let mut argv = vec![
            "--host",
            &self.host,
            "--store",
            &self.store,
            "--backend",
            "file",
        ];
        argv.extend_from_slice(args);
        ekr(&argv, stdin).unwrap_or_else(|failure| panic!("ekr {args:?}: {failure}"))
    }

    fn json(&self, args: &[&str], stdin: &[u8]) -> serde_json::Value {
        serde_json::from_str(&self.run(args, stdin)).unwrap()
    }

    /// Propose and validate; the validation's kind, and commit when it validated.
    fn apply(&self, document: &ekr_sdk::document::TransactionDocument) -> serde_json::Value {
        let yaml = document.to_yaml().unwrap();
        let proposed = self.json(&["propose", "-"], yaml.as_bytes());
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        let validated = self.json(&["validate", &id], b"");
        if validated["kind"] == "Validated" {
            let committed = self.json(&["commit", &id], b"");
            assert_eq!(committed["kind"], "Committed", "{committed:#}");
        }
        validated
    }
}

fn catalogue(title: ValueSpec) -> OntologySpec {
    OntologySpec::new()
        .with_node_type(NodeTypeSpec::new("Author"))
        .with_node_type(NodeTypeSpec::new("Book").with_property(PropertySpec::new("title", title)))
        .with_edge_type(
            EdgeTypeSpec::new("WROTE", ["Author"], ["Book"]).with_cardinality(Cardinality::Many),
        )
}

#[test]
fn ontology_read_at_every_schema_version_is_the_map_of_that_version() {
    let store = Store::v2();
    let (seed, at_seed) = catalogue(ValueSpec::String).seed(Timestamp::EPOCH).unwrap();
    store.run(&["seed", "-"], seed.build().to_yaml().unwrap().as_bytes());

    let grown = catalogue(ValueSpec::String)
        .with_node_type(NodeTypeSpec::new("Journal"))
        .with_edge_type(
            EdgeTypeSpec::new("WROTE", ["Author"], ["Book", "Journal"])
                .with_cardinality(Cardinality::Many),
        );
    let grown = OntologySpec {
        edge_types: grown.edge_types[1..].to_vec(),
        ..grown
    };
    let first = Ontology::read(&store.run(&["ontology"], b""))
        .unwrap()
        .ensure(&grown, ValidationProfile::V2)
        .unwrap();
    let first_doc = first.transaction(store.operator).unwrap().unwrap();
    assert_eq!(store.apply(&first_doc)["kind"], "Validated");
    let second = Ontology::read(&store.run(&["ontology"], b""))
        .unwrap()
        .ensure(
            &OntologySpec::new().with_node_type(
                NodeTypeSpec::new("Journal")
                    .with_property(PropertySpec::new("issn", ValueSpec::String)),
            ),
            ValidationProfile::V2,
        )
        .unwrap();
    let second_doc = second.transaction(store.operator).unwrap().unwrap();
    assert_eq!(store.apply(&second_doc)["kind"], "Validated");

    let at =
        |revision: &str| Ontology::read(&store.run(&["ontology", "--at", revision], b"")).unwrap();
    let (zero, one, two) = (at("0"), at("1"), at("2"));
    assert_eq!(zero.schema_version(), at_seed.schema_version());
    assert_eq!(one.schema_version(), first_doc.transaction.schema_version);
    assert_eq!(two.schema_version(), second_doc.transaction.schema_version);
    assert_eq!(zero.node_type("Book"), at_seed.node_type("Book"));
    assert_eq!(zero.node_type("Journal"), None);
    assert_eq!(
        one.node_type("Journal"),
        first.ontology().node_type("Journal")
    );
    assert_eq!(one.property("Journal", "issn"), None);
    assert_eq!(
        two.property("Journal", "issn"),
        second.ontology().property("Journal", "issn")
    );
    assert!(zero
        .ensure(&catalogue(ValueSpec::String), ValidationProfile::V2)
        .unwrap()
        .is_empty());
    assert!(two
        .ensure(&grown, ValidationProfile::V2)
        .unwrap()
        .is_empty());
}

#[test]
fn ensure_redeclaring_a_held_property_under_another_value_kind_is_refused_by_the_kernel() {
    let store = Store::v2();
    let (seed, names) = catalogue(ValueSpec::String).seed(Timestamp::EPOCH).unwrap();
    let root = seed.root_id();
    let book = NodeDraft::new(root, names.node_type("Book").unwrap(), "Lichens").with_property(
        names.property("Book", "title").unwrap(),
        Value::String("Lichens".into()),
    );
    store.run(
        &["seed", "-"],
        seed.node(book, None).build().to_yaml().unwrap().as_bytes(),
    );
    let change = Ontology::read(&store.run(&["ontology"], b""))
        .unwrap()
        .ensure(&catalogue(ValueSpec::Integer), ValidationProfile::V2)
        .unwrap();
    assert_eq!(change.operations().len(), 1, "{:?}", change.operations());
    let verdict = store.apply(&change.transaction(store.operator).unwrap().unwrap());
    assert_eq!(verdict["kind"], "Rejected", "{verdict:#}");
}

#[test]
fn a_seed_the_sdk_writes_is_the_seed_the_kernel_reads_and_every_awkward_alias_resolves() {
    let store = Store::v2();
    let operator = store.operator;
    let (builder, names) = catalogue(ValueSpec::String).seed(Timestamp::EPOCH).unwrap();
    let root = builder.root_id();
    let book_type = names.node_type("Book").unwrap();
    let title = names.property("Book", "title").unwrap();
    let texts = awkward_strings();
    let mut builder = builder;
    let mut books = Vec::new();
    for text in &texts {
        let book = NodeDraft::new(root, book_type, text.clone())
            .with_alias(format!("alias:{text}"))
            .with_property(title, Value::String(text.clone()));
        books.push((text.clone(), book.id));
        let evidence = EvidenceAddition::new(
            EvidenceSource::HumanStatement {
                identity: Some(text.clone()),
            },
            operator,
            Timestamp::EPOCH,
            Confidence::CERTAIN,
            text.as_bytes().to_vec(),
        );
        let claim = Assertion::new(
            root,
            Subject::Node(book.id),
            Predicate::Property(title),
            Object::Value(Value::String(text.clone())),
            operator,
        )
        .citing(evidence.evidence.id);
        builder = builder
            .node(book, Some(text.clone()))
            .evidence(evidence)
            .assertion(claim);
    }
    let seed = builder.build();
    let yaml = seed.to_yaml().unwrap();
    let read = ekr_kernel::SeedDocument::from_yaml(&yaml)
        .unwrap_or_else(|e| panic!("the kernel refuses an SDK seed: {e}"));
    assert_eq!(
        serde_yaml_ng::to_value(&read).unwrap(),
        serde_yaml_ng::to_value(&seed).unwrap(),
        "the kernel reads another seed than the SDK built"
    );

    // The same nodes, without a lifecycle state their type does not declare, on a store.
    let mut on_store = seed.clone();
    for node in on_store.graph.graph.nodes.values_mut() {
        node.type_state = None;
    }
    store.run(&["seed", "-"], on_store.to_yaml().unwrap().as_bytes());
    for (text, id) in books {
        let reference =
            ekr_sdk::document::TypedReference::new(book_type, [format!("alias:{text}")]);
        let resolved = store.json(&["resolve", "-"], reference.to_yaml().unwrap().as_bytes());
        assert_eq!(
            (resolved["kind"].as_str(), resolved["node_id"].as_str()),
            (Some("Resolved"), Some(id.to_string().as_str())),
            "alias:{text:?}: {resolved}"
        );
    }
}
