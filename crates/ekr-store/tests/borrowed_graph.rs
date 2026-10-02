//! Borrowed checkpoint output preserves the complete owned graph-document wire shape.
use ekr_core::*;
use ekr_graph::*;
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::GraphDocument;
use std::collections::{BTreeMap, BTreeSet};

fn id<T: std::str::FromStr>(n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn graph() -> CanonicalGraph {
    let root = GraphRoot {
        id: id(1),
        space: Space::Canonical,
        schema_version_id: id(2),
        parent: Some(id(3)),
        created_at: Timestamp::from_millis(11),
    };
    let values = vec![
        CanonicalValue::String("text \"with\" escaping\n".into()),
        CanonicalValue::Boolean(true),
        CanonicalValue::Integer(-9),
        CanonicalValue::Decimal("0.125".into()),
        CanonicalValue::Timestamp(Timestamp::from_millis(23)),
        CanonicalValue::Duration(-4),
        CanonicalValue::NodeRef(CanonicalRef::new(id(4))),
        CanonicalValue::Enum("choice".into()),
        CanonicalValue::List(vec![
            CanonicalValue::Integer(2),
            CanonicalValue::Record(BTreeMap::from([(
                "nested".into(),
                CanonicalValue::NodeRef(CanonicalRef::new(id(4))),
            )])),
        ]),
        CanonicalValue::Record(BTreeMap::from([(
            "list".into(),
            CanonicalValue::List(vec![CanonicalValue::Boolean(false)]),
        )])),
    ];
    let mut node = Node::new(id(4), root.id, id(5), "subject");
    node.aliases = vec!["first".into(), "second".into()];
    node.type_state = Some("active".into());
    node.properties.insert(id(6), values.clone());
    let mut edge = Edge::new(
        id(7),
        root.id,
        id(8),
        CanonicalRef::new(node.id),
        CanonicalRef::new(node.id),
    );
    edge.properties.insert(id(6), values.clone());
    let evidence = Evidence {
        id: id(9),
        source: EvidenceSource::Document {
            document_id: "doc".into(),
            section: Some("section".into()),
        },
        content_hash: ContentHash::of_bytes(b"synthetic"),
        extracted_by: id(10),
        observed_at: Timestamp::from_millis(17),
        confidence: Confidence::from_basis_points(8750).unwrap(),
    };
    let mut assertions = BTreeMap::new();
    for (index, value) in values.into_iter().enumerate() {
        let assertion = Assertion {
            id: id(100 + index as u64),
            root_id: root.id,
            subject: if index % 2 == 0 {
                Subject::Node(CanonicalRef::new(node.id))
            } else {
                Subject::Edge(CanonicalRef::new(edge.id))
            },
            predicate: Predicate::Property(id(6)),
            object: Object::Value(value),
            evidence: BTreeSet::from([CanonicalRef::new(evidence.id)]),
            proposed_by: id(10),
            assessment: match index % 5 {
                0 => Assessment::Proposed,
                1 => Assessment::Validating {
                    completed: 2,
                    required: 3,
                },
                2 => Assessment::Accepted {
                    validators: BTreeSet::from([id(10), id(11)]),
                },
                3 => Assessment::Rejected {
                    issues: vec![id(12)],
                },
                _ => Assessment::Disputed {
                    competing_assertions: vec![CanonicalRef::new(id(100))],
                },
            },
            lifecycle: match index % 3 {
                0 => AssertionLifecycle::Active,
                1 => AssertionLifecycle::Retracted {
                    at_revision: RevisionNumber::new(7),
                    reason: RetractionReason::new("withdrawn"),
                },
                _ => AssertionLifecycle::Superseded {
                    by: CanonicalRef::new(id(100)),
                    at_revision: RevisionNumber::new(7),
                    effective_from: Timestamp::from_millis(31),
                },
            },
            valid_time: TemporalRange {
                from: Some(Timestamp::from_millis(13)),
                to: Some(Timestamp::from_millis(31)),
            },
            transaction_time: TransactionTime::new(
                Timestamp::from_millis(19),
                Some(Timestamp::from_millis(37)),
            )
            .unwrap(),
        };
        assertions.insert(assertion.id, assertion);
    }
    CanonicalGraph {
        root,
        revision: RevisionNumber::new(7),
        ontology: Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(root.schema_version_id, Timestamp::EPOCH),
            node_types: vec![],
            edge_types: vec![],
        })
        .unwrap(),
        nodes: BTreeMap::from([(node.id, node)]),
        edges: BTreeMap::from([(edge.id, edge)]),
        assertions,
        evidence: BTreeMap::from([(evidence.id, evidence)]),
        attachments: BTreeMap::new(),
    }
}

#[test]
fn borrowed_graph_matches_owned_bytes_for_every_value_and_retained_record_field() {
    let mut graph = graph();
    for attached in [false, true] {
        if attached {
            graph.attachments.insert(
                id(100),
                BTreeSet::from([AttachedEvidence {
                    evidence: CanonicalRef::new(id(9)),
                    revision: RevisionNumber::new(6),
                }]),
            );
        }
        let owned = GraphDocument::of(&graph);
        let expected = owned.to_bytes().unwrap();
        let mut actual = Vec::new();
        GraphDocument::serialize_graph(&graph, &mut serde_json::Serializer::new(&mut actual))
            .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(GraphDocument::from_bytes(&actual).unwrap(), owned);
        let value: serde_json::Value = serde_json::from_slice(&actual).unwrap();
        assert_eq!(value["graph"].get("attachments").is_some(), attached);
        let mut yaml = Vec::new();
        GraphDocument::serialize_graph(&graph, &mut serde_yaml_ng::Serializer::new(&mut yaml))
            .unwrap();
        assert_eq!(yaml, serde_yaml_ng::to_string(&owned).unwrap().into_bytes());
    }
}

#[test]
fn borrowed_graph_returns_the_serializer_write_error() {
    struct Refuses;
    impl std::io::Write for Refuses {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("expected refusal"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let error = GraphDocument::serialize_graph(&graph(), &mut serde_json::Serializer::new(Refuses))
        .unwrap_err();
    assert!(error.to_string().contains("expected refusal"));
}

#[test]
fn borrowed_graph_preserves_serde_envelope_and_graph_struct_names() {
    use serde::ser::{Impossible, SerializeStruct};
    use serde::{Serialize, Serializer};
    type Error = serde::de::value::Error;
    struct Names<'a>(&'a mut Vec<(&'static str, usize)>);
    fn unexpected<T>() -> Result<T, Error> {
        Err(serde::ser::Error::custom(
            "expected the graph envelope and fields",
        ))
    }
    macro_rules! scalar {
        ($($method:ident($kind:ty)),* $(,)?) => {$ (
            fn $method(self, _: $kind) -> Result<(), Error> { unexpected() }
        )*};
    }
    impl<'a> Serializer for Names<'a> {
        type Ok = ();
        type Error = Error;
        type SerializeSeq = Impossible<(), Error>;
        type SerializeTuple = Impossible<(), Error>;
        type SerializeTupleStruct = Impossible<(), Error>;
        type SerializeTupleVariant = Impossible<(), Error>;
        type SerializeMap = Impossible<(), Error>;
        type SerializeStruct = Self;
        type SerializeStructVariant = Impossible<(), Error>;
        scalar!(
            serialize_bool(bool),
            serialize_i8(i8),
            serialize_i16(i16),
            serialize_i32(i32),
            serialize_i64(i64),
            serialize_u8(u8),
            serialize_u16(u16),
            serialize_u32(u32),
            serialize_u64(u64),
            serialize_f32(f32),
            serialize_f64(f64),
            serialize_char(char),
            serialize_str(&str),
            serialize_bytes(&[u8])
        );
        fn serialize_none(self) -> Result<(), Error> {
            unexpected()
        }
        fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> Result<(), Error> {
            unexpected()
        }
        fn serialize_unit(self) -> Result<(), Error> {
            unexpected()
        }
        fn serialize_unit_struct(self, _: &'static str) -> Result<(), Error> {
            unexpected()
        }
        fn serialize_unit_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
        ) -> Result<(), Error> {
            unexpected()
        }
        fn serialize_newtype_struct<T: ?Sized + Serialize>(
            self,
            _: &'static str,
            _: &T,
        ) -> Result<(), Error> {
            unexpected()
        }
        fn serialize_newtype_variant<T: ?Sized + Serialize>(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: &T,
        ) -> Result<(), Error> {
            unexpected()
        }
        fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Error> {
            unexpected()
        }
        fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Error> {
            unexpected()
        }
        fn serialize_tuple_struct(
            self,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleStruct, Error> {
            unexpected()
        }
        fn serialize_tuple_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeTupleVariant, Error> {
            unexpected()
        }
        fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Error> {
            unexpected()
        }
        fn serialize_struct(self, name: &'static str, fields: usize) -> Result<Self, Error> {
            self.0.push((name, fields));
            Ok(self)
        }
        fn serialize_struct_variant(
            self,
            _: &'static str,
            _: u32,
            _: &'static str,
            _: usize,
        ) -> Result<Self::SerializeStructVariant, Error> {
            unexpected()
        }
    }
    impl SerializeStruct for Names<'_> {
        type Ok = ();
        type Error = Error;
        fn serialize_field<T: ?Sized + Serialize>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Error> {
            if key == "graph" {
                value.serialize(Names(self.0))?;
            }
            Ok(())
        }
        fn end(self) -> Result<(), Error> {
            Ok(())
        }
    }
    let graph = graph();
    let mut owned = Vec::new();
    GraphDocument::of(&graph)
        .serialize(Names(&mut owned))
        .unwrap();
    let mut borrowed = Vec::new();
    GraphDocument::serialize_graph(&graph, Names(&mut borrowed)).unwrap();
    assert_eq!(borrowed, owned);
}
