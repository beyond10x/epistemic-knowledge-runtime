//! `Assertion` is content-addressable, and a `Float` cannot be inside one:
//! `architecture-decision-record:0005-float-is-not-canonical`.
//!
//! The acceptance of `task:canonical-value-in-the-graph` is two claims, and they are checked here
//! separately because they fail separately:
//!
//! * **total.** [`Assertion`] implements `Canonical` with no fallible path and no unreachable arm,
//!   which is only possible because the value it carries cannot be a float.
//! * **unrepresentable, not refused.** A value carrying a `Float` at any depth cannot inhabit an
//!   assertion, a node property or an edge property — the refusal happens once, at the conversion,
//!   and names the path to the offending value.
//!
//! The sum-type cases are the other half of `crates/ekr-core/src/canonical.rs` rule 5: a variant
//! tag is hand-written, so nothing derives the numbering from the declaration and the two can come
//! apart silently. `tests/revision_events.rs` pins `RevisionEvent`'s numbering; the scan below
//! covers **every** sum type in this crate that writes a tag, so the next one cannot arrive
//! unpinned.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::canonical::Canonical;
use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, IssueId, NodeId,
    ObservationId, PropertyId, RevisionNumber, SupportId, Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalRef, CanonicalValue, Confidence, Edge,
    Evidence, EvidenceSource, InadmissibleValue, Node, Object, Observation, ObservationContent,
    Predicate, RetractionReason, Subject, Support, TemporalRange, TransactionTime,
};
use ekr_ontology::Value;

/// 2024-01-01T00:00:00Z.
const RECORDED: Timestamp = Timestamp::from_millis(1_704_067_200_000);
/// 2026-03-12T00:00:00Z.
const HANDOVER: Timestamp = Timestamp::from_millis(1_773_273_600_000);

/// A record, built from pairs, so a case reads as the value it is about.
fn record(fields: &[(&str, Value)]) -> Value {
    Value::Record(
        fields
            .iter()
            .map(|(name, value)| ((*name).to_owned(), value.clone()))
            .collect::<BTreeMap<String, Value>>(),
    )
}

/// One value of every admissible kind, and two compounds of them.
fn admissible() -> Vec<Value> {
    let node = NodeId::mint();
    vec![
        Value::String("acme".to_owned()),
        Value::Boolean(false),
        Value::Integer(-7),
        Value::Decimal("0.1".to_owned()),
        Value::Timestamp(HANDOVER),
        Value::Duration(90_000),
        Value::NodeRef(node),
        Value::Enum("decided".to_owned()),
        Value::List(vec![Value::Integer(1), Value::String("two".to_owned())]),
        record(&[
            ("head", Value::NodeRef(node)),
            ("since", Value::Timestamp(RECORDED)),
        ]),
        Value::List(Vec::new()),
        Value::Record(BTreeMap::new()),
    ]
}

/// A value carrying a float, and the path the refusal must name.
fn inadmissible() -> Vec<(Value, &'static str)> {
    vec![
        (Value::Float(2.5), "value"),
        (Value::Float(f64::NAN), "value"),
        (
            Value::List(vec![Value::Integer(1), Value::Float(2.5)]),
            "value[1]",
        ),
        (record(&[("reading", Value::Float(2.5))]), "value.reading"),
        (
            record(&[(
                "samples",
                Value::List(vec![record(&[("mean", Value::Float(0.5))])]),
            )]),
            "value.samples[0].mean",
        ),
    ]
}

#[test]
fn an_admissible_value_round_trips_through_the_newtype() {
    for value in admissible() {
        let held = CanonicalValue::try_from(value.clone())
            .unwrap_or_else(|e| panic!("{:?} is admissible: {e}", value.kind()));
        assert_eq!(
            Value::from(held.clone()),
            value,
            "the value that came out is not the one that went in"
        );
        assert_eq!(
            held.canonical_bytes(),
            held.canonical_bytes(),
            "an encoding is a function of the value"
        );
    }
}

#[test]
fn two_values_that_differ_do_not_encode_alike() {
    let mut seen: BTreeMap<Vec<u8>, String> = BTreeMap::new();
    for value in admissible() {
        let held = CanonicalValue::try_from(value.clone()).expect("admissible");
        let described = format!("{value:?}");
        if let Some(other) = seen.insert(held.canonical_bytes(), described.clone()) {
            panic!("{described} and {other} share an encoding");
        }
    }
    assert_eq!(seen.len(), admissible().len(), "a value lost its encoding");

    // The two compound kinds are the collision rule 2 of `ekr_core::canonical` is about: a list of
    // one string and a string must not agree.
    let text = CanonicalValue::try_from(Value::String("a".to_owned())).expect("admissible");
    let listed = CanonicalValue::try_from(Value::List(vec![Value::String("a".to_owned())]))
        .expect("admissible");
    assert_ne!(text.canonical_bytes(), listed.canonical_bytes());
}

#[test]
fn a_float_at_any_depth_cannot_inhabit_a_canonical_value_and_the_refusal_names_it() {
    for (value, path) in inadmissible() {
        let refusal: InadmissibleValue = CanonicalValue::try_from(value.clone())
            .expect_err("a float is not admissible in canonical state");
        assert_eq!(
            refusal.path().to_string(),
            path,
            "the refusal of {value:?} does not name the offending value"
        );
        assert!(
            refusal.to_string().contains(path),
            "the message a reader sees does not carry the path: {refusal}"
        );
    }
}

/// The two crates answer the same question, so they answer it the same way.
///
/// `ekr-ontology` states the rule over a `Value`, which is what a validator above this crate asks;
/// the conversion here refuses as it converts. Two walks that can disagree are a defect, so the
/// agreement is a case rather than a comment.
#[test]
fn the_conversion_refuses_exactly_where_the_ontology_says_it_must() {
    let every: Vec<Value> = admissible()
        .into_iter()
        .chain(inadmissible().into_iter().map(|(value, _)| value))
        .collect();

    for value in every {
        let by_conversion = CanonicalValue::try_from(value.clone())
            .err()
            .map(|refusal| refusal.path().to_string());
        let by_ontology = value
            .inadmissible_in_canonical_state()
            .map(|path| path.to_string());
        assert_eq!(
            by_conversion, by_ontology,
            "the crates disagree about {value:?}"
        );
    }
}

#[test]
fn a_node_and_an_edge_property_carry_only_an_admissible_value() {
    let property = PropertyId::mint();
    let held = CanonicalValue::try_from(Value::Decimal("0.1".to_owned())).expect("admissible");

    let mut node = Node::new(NodeId::mint(), GraphRootId::mint(), TypeId::mint(), "Acme");
    node.properties.insert(property, vec![held.clone()]);
    assert_eq!(node.properties.get(&property), Some(&vec![held.clone()]));

    let mut edge = Edge::new(
        EdgeId::mint(),
        GraphRootId::mint(),
        TypeId::mint(),
        CanonicalRef::new(NodeId::mint()),
        CanonicalRef::new(NodeId::mint()),
    );
    edge.properties.insert(property, vec![held.clone()]);
    assert_eq!(edge.properties.get(&property), Some(&vec![held]));

    // The only way in is the conversion, and it is where the float stops. Both maps are keyed to
    // `CanonicalValue`, so there is no second door for a caller who skips it.
    assert!(CanonicalValue::try_from(Value::Float(0.1)).is_err());
}

/// One assertion, with every field set, built from ids the caller supplies so that two calls with
/// the same ids produce two structurally equal records.
fn assertion(ids: [u128; 6]) -> Assertion {
    Assertion {
        id: id(ids[0]),
        root_id: id(ids[1]),
        subject: Subject::Node(CanonicalRef::new(id(ids[2]))),
        predicate: Predicate::Property(id(ids[3])),
        object: Object::Value(
            CanonicalValue::try_from(Value::String("Acme".to_owned())).expect("admissible"),
        ),
        evidence: BTreeSet::from([CanonicalRef::new(id(ids[4])), CanonicalRef::new(id(ids[5]))]),
        proposed_by: id(ids[0] ^ 0xff),
        assessment: Assessment::Accepted {
            validators: BTreeSet::new(),
        },
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(RECORDED),
        transaction_time: TransactionTime::since(RECORDED),
    }
}

/// An id of any of the crate's id types, from a number, so that a case can name the same id twice
/// and two different ones apart. Through the text form, which is the only way in from outside
/// `ekr-core` — this crate does not depend on `uuid`.
fn id<T: std::str::FromStr>(bits: u128) -> T
where
    T::Err: std::fmt::Debug,
{
    let hex = format!("{bits:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
    .parse()
    .expect("the one text form of an id")
}

/// The base every mutation below is a single step from.
fn base() -> Assertion {
    assertion([1, 2, 3, 4, 5, 6])
}

/// A field's name, and a change to that field and to nothing else.
type Mutation = (&'static str, fn(&mut Assertion));

/// Every field of `Assertion`, each with **at least one** change to exactly that field — and for
/// every field whose value is a sum type or has parts of its own, one change per part.
///
/// The one-row-per-field version of this table was a hole the adversary walked through: its
/// `validation` row moved `Accepted{..}` to `Rejected{..}`, which the *variant tag* alone
/// separates, so deleting all eight payload-encoding lines from `ValidationState::encode` left the
/// whole suite green — and two `Accepted` assertions with different validator sets would have
/// shared a content address. The same held two fields wider: the `valid_time` row varied only
/// `from` and the `transaction_time` row varied only `recorded_to`, so dropping `TemporalRange.to`
/// and `TransactionTime.recorded_from` cost nothing either.
///
/// **A mutation table is a claim about a class, and a row that varies one field does not test the
/// others.** So a payload is varied *within* its variant here — `Accepted{[a]}` against
/// `Accepted{}`, `Retracted{r,x}` against `Retracted{r,y}` against `Retracted{r2,x}` — and the case
/// below compares every row against every other row rather than each against the base.
///
/// Held to the declaration by `every_field_of_an_assertion_reaches_the_encoding`, which compares
/// the *set* of names here to the fields the type declares.
///
/// Amendment 88 split the one `validation` field into `assessment` and `lifecycle`. Each keeps the
/// rows its states had under `validation`, and `lifecycle` gains one row per `Superseded` field it
/// carries now and did not before.
const MUTATIONS: [Mutation; 32] = [
    ("id", |a| a.id = id::<AssertionId>(99)),
    ("root_id", |a| {
        a.root_id = id::<GraphRootId>(99);
    }),
    // Both arms of the change a sum type can carry: a different variant, and a different payload
    // under the same variant. The first is settled by the tag, the second only by the payload.
    ("subject", |a| {
        a.subject = Subject::Edge(CanonicalRef::new(id::<EdgeId>(3)));
    }),
    ("subject", |a| {
        a.subject = Subject::Node(CanonicalRef::new(id::<NodeId>(97)));
    }),
    ("predicate", |a| {
        a.predicate = Predicate::Relation(id::<TypeId>(4));
    }),
    ("predicate", |a| {
        a.predicate = Predicate::Property(id::<PropertyId>(96));
    }),
    ("object", |a| {
        a.object = Object::Value(
            CanonicalValue::try_from(Value::String("Acme Ltd".to_owned())).expect("admissible"),
        );
    }),
    ("object", |a| {
        a.object = Object::Node(CanonicalRef::new(id::<NodeId>(3)));
    }),
    ("evidence", |a| {
        a.evidence.insert(CanonicalRef::new(id::<EvidenceId>(7)));
    }),
    ("evidence", |a| {
        a.evidence.remove(&CanonicalRef::new(id::<EvidenceId>(5)));
    }),
    ("proposed_by", |a| {
        a.proposed_by = id::<AgentId>(98);
    }),
    // The five assessments, and within each the payload that distinguishes two records in it. The
    // base carries `Accepted { validators: {} }`, so the first row here differs from it by a
    // payload alone.
    ("assessment", |a| {
        a.assessment = Assessment::Accepted {
            validators: BTreeSet::from([id::<AgentId>(21)]),
        };
    }),
    ("assessment", |a| {
        a.assessment = Assessment::Accepted {
            validators: BTreeSet::from([id::<AgentId>(22)]),
        };
    }),
    ("assessment", |a| a.assessment = Assessment::Proposed),
    ("assessment", |a| {
        a.assessment = Assessment::Validating {
            completed: 1,
            required: 2,
        };
    }),
    ("assessment", |a| {
        // The same two numbers the other way round: a swap no field order can hide.
        a.assessment = Assessment::Validating {
            completed: 2,
            required: 1,
        };
    }),
    ("assessment", |a| {
        a.assessment = Assessment::Rejected {
            issues: vec![id::<IssueId>(8)],
        };
    }),
    ("assessment", |a| {
        a.assessment = Assessment::Rejected { issues: Vec::new() };
    }),
    ("assessment", |a| {
        a.assessment = Assessment::Disputed {
            competing_assertions: vec![CanonicalRef::new(id::<AssertionId>(31))],
        };
    }),
    ("assessment", |a| {
        a.assessment = Assessment::Disputed {
            competing_assertions: vec![
                CanonicalRef::new(id::<AssertionId>(31)),
                CanonicalRef::new(id::<AssertionId>(32)),
            ],
        };
    }),
    // The two withdrawals. The base is `Active`, which carries no payload.
    ("lifecycle", |a| {
        a.lifecycle = AssertionLifecycle::Superseded {
            by: CanonicalRef::new(id::<AssertionId>(41)),
            at_revision: RevisionNumber::new(7),
            effective_from: HANDOVER,
        };
    }),
    ("lifecycle", |a| {
        // `by` alone.
        a.lifecycle = AssertionLifecycle::Superseded {
            by: CanonicalRef::new(id::<AssertionId>(42)),
            at_revision: RevisionNumber::new(7),
            effective_from: HANDOVER,
        };
    }),
    ("lifecycle", |a| {
        // `at_revision` alone.
        a.lifecycle = AssertionLifecycle::Superseded {
            by: CanonicalRef::new(id::<AssertionId>(41)),
            at_revision: RevisionNumber::new(8),
            effective_from: HANDOVER,
        };
    }),
    ("lifecycle", |a| {
        // `effective_from` alone.
        a.lifecycle = AssertionLifecycle::Superseded {
            by: CanonicalRef::new(id::<AssertionId>(41)),
            at_revision: RevisionNumber::new(7),
            effective_from: RECORDED,
        };
    }),
    ("lifecycle", |a| {
        a.lifecycle = AssertionLifecycle::Retracted {
            at_revision: RevisionNumber::new(7),
            reason: RetractionReason::new("the evidence was another Acme"),
        };
    }),
    ("lifecycle", |a| {
        // The same revision, a different reason.
        a.lifecycle = AssertionLifecycle::Retracted {
            at_revision: RevisionNumber::new(7),
            reason: RetractionReason::new("the source withdrew it"),
        };
    }),
    ("lifecycle", |a| {
        // The same reason, a different revision.
        a.lifecycle = AssertionLifecycle::Retracted {
            at_revision: RevisionNumber::new(8),
            reason: RetractionReason::new("the evidence was another Acme"),
        };
    }),
    // Both bounds of valid time, separately. The base is `[RECORDED, )`.
    ("valid_time", |a| {
        a.valid_time = TemporalRange::since(HANDOVER);
    }),
    ("valid_time", |a| {
        a.valid_time = TemporalRange::new(Some(RECORDED), Some(HANDOVER)).expect("not inverted");
    }),
    ("valid_time", |a| a.valid_time = TemporalRange::UNBOUNDED),
    // Both fields of transaction time, separately. The base is `[RECORDED, )`.
    ("transaction_time", |a| {
        a.transaction_time = TransactionTime::since(HANDOVER);
    }),
    ("transaction_time", |a| {
        a.transaction_time = TransactionTime::new(RECORDED, Some(HANDOVER)).expect("not inverted");
    }),
];

#[test]
fn two_assertions_equal_in_every_field_hash_equally() {
    let left = base();
    let mut right = assertion([1, 2, 3, 4, 5, 6]);
    // The same evidence set, reached the other way round: a content address is a function of the
    // value, never of the order a collection was filled in.
    right.evidence = BTreeSet::from([
        CanonicalRef::new(id::<EvidenceId>(6)),
        CanonicalRef::new(id::<EvidenceId>(5)),
    ]);

    assert_eq!(left, right, "the fixture builds two equal records");
    assert_eq!(
        left.canonical_bytes(),
        right.canonical_bytes(),
        "two equal assertions do not encode alike"
    );
    assert_eq!(ContentHash::of(&left), ContentHash::of(&right));
}

/// No two of the base and its thirty-two mutants share a content address.
///
/// Pairwise, not each-against-the-base, and that is the point: two records that differ from the
/// base in the same field but not from each other — `Retracted{r,x}` and `Retracted{r,y}` — are
/// exactly the pair a payload dropped from an encoding would collapse, and comparing each only to
/// the base would not look at them.
#[test]
fn no_two_assertions_that_differ_share_a_content_address() {
    let base = base();
    let mut labelled = vec![("the base record".to_owned(), base.canonical_bytes())];

    for (position, (field, mutate)) in MUTATIONS.iter().enumerate() {
        let mut changed = base.clone();
        mutate(&mut changed);
        assert_ne!(
            &changed, &base,
            "the {field} mutation at {position} changed nothing"
        );
        labelled.push((
            format!("the {field} mutation at {position}"),
            changed.canonical_bytes(),
        ));
    }

    no_two_share_an_encoding("assertions", &labelled);
    assert_eq!(labelled.len(), MUTATIONS.len() + 1);
}

/// Every field the type declares has a mutation above, so none can be left out of the encoding.
///
/// It is the field list that is read, not a count: a field added to `Assertion` and forgotten in
/// `Canonical::encode` is invisible to a hash case that does not touch it, and the record would
/// then have two values with one address.
#[test]
fn every_field_of_an_assertion_reaches_the_encoding() {
    let source = std::fs::read_to_string(
        std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("Cargo supplies the runtime manifest directory"),
        )
        .join("src/assertion.rs"),
    )
    .expect("the crate's own source");
    let declared = fields_of(&source, "Assertion");

    let mut covered: Vec<String> = MUTATIONS
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    covered.sort();
    covered.dedup();
    let mut expected = declared.clone();
    expected.sort();

    assert_eq!(
        covered, expected,
        "the fields Assertion declares and the fields MUTATIONS changes have come apart"
    );
    assert_eq!(declared.len(), 11, "the field scan is broken, not the type");
}

/// The names of the `pub` fields of one struct, in declaration order.
fn fields_of(source: &str, type_name: &str) -> Vec<String> {
    let body = declaration_body(source, "pub struct", type_name)
        .unwrap_or_else(|| panic!("the crate declares struct {type_name}"));
    body.lines()
        .skip(1)
        .take_while(|line| *line != "}")
        .filter(|line| line.starts_with("    pub ") && !line.starts_with("     "))
        .filter_map(|line| {
            line.trim()
                .strip_prefix("pub ")?
                .split_once(':')
                .map(|(name, _)| name.to_owned())
        })
        .collect()
}

/// The body of a `pub struct X` or `pub enum X` declaration, from its opening brace to the brace
/// that closes it, or `None` if the source does not declare one.
///
/// Matched on the name and then on the first `{` after it, rather than on the literal text
/// `pub enum X {`. `architecture-decision-record:0005-float-is-not-canonical`, as amended, made
/// `Object`, `Node`, `Edge` and `Assertion` generic, so their heads read `pub enum Object<V = ...>
/// {` — and the earlier scan, keyed to the ungeneric spelling, silently read `Object` as *not an
/// enum* and skipped it. That was caught by the set assertion in the numbering case rather than by
/// this function, which is the shape to keep: a scan states what it found, and something else
/// compares that to what there is.
///
/// A name is matched only where the character after it is not an identifier character, so
/// `NodeType` is not `Node`.
fn declaration_body(source: &str, keyword: &str, type_name: &str) -> Option<String> {
    let head = format!("{keyword} {type_name}");
    let mut from = 0;
    while let Some(offset) = source[from..].find(&head) {
        let at = from + offset;
        from = at + head.len();
        let rest = &source[from..];
        if rest
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let open = from + rest.find('{')?;
        return Some(block_from(source, open));
    }
    None
}

/// Every pair of records that share an encoding, named — **all** of them, not the first.
///
/// A case that panics at the first collision reports "1 red" whether one field broke or eight, so a
/// mutation measurement made with it is not a measurement (adversary pass 2, finding 6). The gate
/// does not change: a red is still a red. What changes is that the failure says how much broke.
fn no_two_share_an_encoding(what: &str, labelled: &[(String, Vec<u8>)]) {
    let collisions = collisions(labelled);
    assert!(
        collisions.is_empty(),
        "{} pair(s) of {what} have one content address between them, so whatever differs between \
         each pair does not reach the encoding: {collisions:#?}",
        collisions.len()
    );
}

/// Every pair sharing an encoding, named. Returned rather than asserted, so a caller running over
/// several types reports all of them in one failure instead of the first type's.
fn collisions(labelled: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut seen: BTreeMap<&Vec<u8>, &str> = BTreeMap::new();
    let mut found: Vec<String> = Vec::new();
    for (label, bytes) in labelled {
        if let Some(other) = seen.insert(bytes, label) {
            found.push(format!("{label} and {other}"));
        }
    }
    found
}

/// The canonical node every node mutation below is a single step from.
fn node() -> Node {
    let mut node = Node::new(id(51), id(52), id(53), "Acme");
    node.aliases = vec!["ACME".to_owned(), "Acme Inc".to_owned()];
    node.properties
        .insert(id::<PropertyId>(54), vec![CanonicalValue::Integer(1)]);
    node
}

/// The canonical edge every edge mutation below is a single step from.
fn edge() -> Edge {
    let mut edge = Edge::new(
        id(61),
        id(62),
        id(63),
        CanonicalRef::new(id(64)),
        CanonicalRef::new(id(65)),
    );
    edge.properties
        .insert(id::<PropertyId>(66), vec![CanonicalValue::Integer(2)]);
    edge
}

/// A field's name, and a change to that field of a node and to nothing else.
type NodeMutation = (&'static str, fn(&mut Node));

/// Every field of `Node`, and for the fields with parts of their own, one change per part.
///
/// `Node` and `Edge` gained encodings because `Root.knowledge_root` is an address over graph
/// state, and graph state is a root's nodes, its edges *and* its assertions — an address over
/// assertions alone is not the graph's. An encoding with no case that varies each of its fields is
/// the hole the assertion table had, so these tables are the same shape and are read by the same
/// pairwise case.
const NODE_MUTATIONS: [NodeMutation; 13] = [
    ("id", |n| n.id = id::<NodeId>(99)),
    ("root_id", |n| n.root_id = id::<GraphRootId>(99)),
    ("type_id", |n| n.type_id = id::<TypeId>(99)),
    ("canonical_name", |n| {
        n.canonical_name = "Acme Ltd".to_owned();
    }),
    ("aliases", |n| n.aliases.push("ACME Corp".to_owned())),
    ("aliases", |n| n.aliases.reverse()),
    ("aliases", |n| n.aliases.clear()),
    ("type_state", |n| n.type_state = Some("open".to_owned())),
    ("type_state", |n| n.type_state = Some("decided".to_owned())),
    ("properties", |n| {
        // The same key, a different value.
        n.properties
            .insert(id::<PropertyId>(54), vec![CanonicalValue::Integer(2)]);
    }),
    ("properties", |n| {
        // The same value, a different key.
        n.properties.clear();
        n.properties
            .insert(id::<PropertyId>(55), vec![CanonicalValue::Integer(1)]);
    }),
    ("properties", |n| {
        // The same value twice: a multi-valued property is not its first value.
        n.properties.insert(
            id::<PropertyId>(54),
            vec![CanonicalValue::Integer(1), CanonicalValue::Integer(1)],
        );
    }),
    ("properties", |n| {
        // One list holding the value: an inner `List` is not the outer values.
        n.properties.insert(
            id::<PropertyId>(54),
            vec![CanonicalValue::try_from(Value::List(vec![Value::Integer(1)])).expect("ok")],
        );
    }),
];

/// A field's name, and a change to that field of an edge and to nothing else.
type EdgeMutation = (&'static str, fn(&mut Edge));

/// Every field of `Edge`, read the same way [`NODE_MUTATIONS`] is.
const EDGE_MUTATIONS: [EdgeMutation; 7] = [
    ("id", |e| e.id = id::<EdgeId>(99)),
    ("root_id", |e| e.root_id = id::<GraphRootId>(99)),
    ("type_id", |e| e.type_id = id::<TypeId>(99)),
    // Separately, and each against the base: an encoding that wrote `source` twice would leave the
    // `target` row equal to the base, and one that dropped `source` would leave the `source` row
    // equal to it.
    ("source", |e| e.source = CanonicalRef::new(id::<NodeId>(99))),
    ("target", |e| e.target = CanonicalRef::new(id::<NodeId>(99))),
    ("properties", |e| {
        e.properties
            .insert(id::<PropertyId>(66), vec![CanonicalValue::Integer(3)]);
    }),
    ("properties", |e| {
        e.properties
            .insert(id::<PropertyId>(67), vec![CanonicalValue::Integer(2)]);
    }),
];

#[test]
fn no_two_nodes_that_differ_share_a_content_address() {
    let base = node();
    let mut labelled = vec![("the base node".to_owned(), base.canonical_bytes())];

    for (position, (field, mutate)) in NODE_MUTATIONS.iter().enumerate() {
        let mut changed = base.clone();
        mutate(&mut changed);
        assert_ne!(
            &changed, &base,
            "the {field} mutation at {position} changed nothing"
        );
        labelled.push((
            format!("the {field} mutation at {position}"),
            changed.canonical_bytes(),
        ));
    }
    no_two_share_an_encoding("nodes", &labelled);
}

#[test]
fn no_two_edges_that_differ_share_a_content_address() {
    let base = edge();
    let mut labelled = vec![("the base edge".to_owned(), base.canonical_bytes())];

    for (position, (field, mutate)) in EDGE_MUTATIONS.iter().enumerate() {
        let mut changed = base.clone();
        mutate(&mut changed);
        assert_ne!(
            &changed, &base,
            "the {field} mutation at {position} changed nothing"
        );
        labelled.push((
            format!("the {field} mutation at {position}"),
            changed.canonical_bytes(),
        ));
    }
    no_two_share_an_encoding("edges", &labelled);
}

/// Two nodes and two edges equal in every field encode equally, insertion order included.
#[test]
fn graph_state_equal_in_every_field_hashes_equally() {
    let mut other = Node::new(id(51), id(52), id(53), "Acme");
    // The aliases in order, the properties into a map built the other way round.
    other.aliases = vec!["ACME".to_owned(), "Acme Inc".to_owned()];
    other
        .properties
        .insert(id::<PropertyId>(54), vec![CanonicalValue::Integer(1)]);
    assert_eq!(other, node());
    assert_eq!(ContentHash::of(&other), ContentHash::of(&node()));

    let mut same_edge = Edge::new(
        id(61),
        id(62),
        id(63),
        CanonicalRef::new(id(64)),
        CanonicalRef::new(id(65)),
    );
    same_edge
        .properties
        .insert(id::<PropertyId>(66), vec![CanonicalValue::Integer(2)]);
    assert_eq!(ContentHash::of(&same_edge), ContentHash::of(&edge()));

    // And a node is not an edge over the same ids: the two are different shapes, and rule 5 of
    // `ekr_core::canonical` leaves that to their field structure.
    assert_ne!(node().canonical_bytes(), edge().canonical_bytes());
}

/// Every field `Node` and `Edge` declare is varied by a mutation above.
///
/// The same guard the assertion table carries, for the same reason: a field added to either type
/// and forgotten in its encoding is invisible to a case that does not touch it, and two nodes
/// differing only in that field would then share an address.
#[test]
fn every_field_of_a_node_and_an_edge_reaches_the_encoding() {
    for (module, type_name, covered, expected_fields) in [
        (
            "/src/node.rs",
            "Node",
            NODE_MUTATIONS
                .iter()
                .map(|(name, _)| (*name).to_owned())
                .collect::<Vec<_>>(),
            7,
        ),
        (
            "/src/edge.rs",
            "Edge",
            EDGE_MUTATIONS
                .iter()
                .map(|(name, _)| (*name).to_owned())
                .collect::<Vec<_>>(),
            6,
        ),
    ] {
        let source = std::fs::read_to_string(format!(
            "{}{module}",
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("Cargo supplies the runtime manifest directory")
        ))
        .expect("the crate's own source");
        let mut declared = fields_of(&source, type_name);
        assert_eq!(
            declared.len(),
            expected_fields,
            "the field scan of {type_name} is broken, not the type: {declared:?}"
        );
        declared.sort();

        let mut covered = covered;
        covered.sort();
        covered.dedup();
        assert_eq!(
            covered, declared,
            "the fields {type_name} declares and the fields its mutation table changes have come \
             apart"
        );
    }
}

/// The canonical evidence, observation and support every mutation below is a single step from.
fn evidence() -> Evidence {
    Evidence {
        id: id(71),
        source: EvidenceSource::Url("https://example.invalid/a".to_owned()),
        content_hash: ContentHash::of_bytes(b"what was read"),
        extracted_by: id(72),
        observed_at: RECORDED,
        confidence: Confidence::from_basis_points(5_000).expect("in range"),
    }
}

fn observation() -> Observation {
    Observation {
        id: id(81),
        source: "the runtime's own vocabulary".to_owned(),
        source_native_id: Some("a-1".to_owned()),
        content: ObservationContent::Document(ContentHash::of_bytes(b"the bytes received")),
        captured_at: RECORDED,
    }
}

fn support() -> Support {
    Support {
        id: id(91),
        assertion_id: id(92),
        evidence_id: id(93),
    }
}

/// A field's name, and a change to that field of a piece of evidence and to nothing else.
type EvidenceMutation = (&'static str, fn(&mut Evidence));
/// The same, for an observation.
type ObservationMutation = (&'static str, fn(&mut Observation));
/// The same, for a support link.
type SupportMutation = (&'static str, fn(&mut Support));

/// Every field of `Evidence`.
///
/// `Root.evidence_root` is the address of evidence state, which is the fourth map `CanonicalGraph`
/// holds — so these three types carry the same obligation `Node` and `Edge` carry, and the same
/// tables.
const EVIDENCE_MUTATIONS: [EvidenceMutation; 7] = [
    ("id", |e| e.id = id::<EvidenceId>(99)),
    ("source", |e| {
        e.source = EvidenceSource::Url("https://example.invalid/b".to_owned());
    }),
    ("source", |e| {
        e.source = EvidenceSource::GraphAssertion(CanonicalRef::new(id::<AssertionId>(73)));
    }),
    ("content_hash", |e| {
        e.content_hash = ContentHash::of_bytes(b"what was read the second time");
    }),
    ("extracted_by", |e| e.extracted_by = id::<AgentId>(99)),
    ("observed_at", |e| e.observed_at = HANDOVER),
    ("confidence", |e| {
        e.confidence = Confidence::CERTAIN;
    }),
];

/// Every field of `Observation`, including the absence of the optional one.
const OBSERVATION_MUTATIONS: [ObservationMutation; 7] = [
    ("id", |o| o.id = id::<ObservationId>(99)),
    ("source", |o| o.source = "another source".to_owned()),
    ("source_native_id", |o| {
        o.source_native_id = Some("a-2".to_owned());
    }),
    ("source_native_id", |o| o.source_native_id = None),
    ("content", |o| {
        // The same kind, other bytes.
        o.content = ObservationContent::Document(ContentHash::of_bytes(b"other bytes"));
    }),
    ("content", |o| {
        // The same bytes, another kind.
        o.content = ObservationContent::ApiResponse(ContentHash::of_bytes(b"the bytes received"));
    }),
    ("captured_at", |o| o.captured_at = HANDOVER),
];

/// Every field of `Support`. All three are ids over the same shape, so position is all that
/// separates them — which is what rule 5 of `ekr_core::canonical` says a struct's fields are for.
const SUPPORT_MUTATIONS: [SupportMutation; 3] = [
    ("id", |s| s.id = id::<SupportId>(99)),
    ("assertion_id", |s| s.assertion_id = id::<AssertionId>(99)),
    ("evidence_id", |s| s.evidence_id = id::<EvidenceId>(99)),
];

#[test]
fn no_two_pieces_of_evidence_that_differ_share_a_content_address() {
    let base = evidence();
    let mut labelled = vec![("the base evidence".to_owned(), base.canonical_bytes())];
    for (position, (field, mutate)) in EVIDENCE_MUTATIONS.iter().enumerate() {
        let mut changed = base.clone();
        mutate(&mut changed);
        assert_ne!(
            &changed, &base,
            "the {field} mutation at {position} changed nothing"
        );
        labelled.push((
            format!("the {field} mutation at {position}"),
            changed.canonical_bytes(),
        ));
    }
    no_two_share_an_encoding("pieces of evidence", &labelled);
}

#[test]
fn no_two_observations_that_differ_share_a_content_address() {
    let base = observation();
    let mut labelled = vec![("the base observation".to_owned(), base.canonical_bytes())];
    for (position, (field, mutate)) in OBSERVATION_MUTATIONS.iter().enumerate() {
        let mut changed = base.clone();
        mutate(&mut changed);
        assert_ne!(
            &changed, &base,
            "the {field} mutation at {position} changed nothing"
        );
        labelled.push((
            format!("the {field} mutation at {position}"),
            changed.canonical_bytes(),
        ));
    }
    no_two_share_an_encoding("observations", &labelled);
}

#[test]
fn no_two_support_links_that_differ_share_a_content_address() {
    let base = support();
    let mut labelled = vec![("the base support".to_owned(), base.canonical_bytes())];
    for (position, (field, mutate)) in SUPPORT_MUTATIONS.iter().enumerate() {
        let mut changed = base;
        mutate(&mut changed);
        assert_ne!(
            changed, base,
            "the {field} mutation at {position} changed nothing"
        );
        labelled.push((
            format!("the {field} mutation at {position}"),
            changed.canonical_bytes(),
        ));
    }
    no_two_share_an_encoding("support links", &labelled);
}

/// Every field `Evidence`, `Observation` and `Support` declare is varied by a mutation above.
///
/// `kind` and `content_hash` on `Observation` are deliberately absent from both the table and the
/// encoding: the domain declares them, the crate answers them from `content`, and
/// `tests/domain_projection.rs` is what holds that projection. Encoding them would put the same
/// bytes in an address twice.
#[test]
fn every_field_of_evidence_state_reaches_the_encoding() {
    let source = std::fs::read_to_string(
        std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("Cargo supplies the runtime manifest directory"),
        )
        .join("src/evidence.rs"),
    )
    .expect("the crate's own source");

    for (type_name, covered, expected_fields) in [
        (
            "Evidence",
            EVIDENCE_MUTATIONS
                .iter()
                .map(|(name, _)| (*name).to_owned())
                .collect::<Vec<_>>(),
            6,
        ),
        (
            "Observation",
            OBSERVATION_MUTATIONS
                .iter()
                .map(|(name, _)| (*name).to_owned())
                .collect::<Vec<_>>(),
            5,
        ),
        (
            "Support",
            SUPPORT_MUTATIONS
                .iter()
                .map(|(name, _)| (*name).to_owned())
                .collect::<Vec<_>>(),
            3,
        ),
    ] {
        let mut declared = fields_of(&source, type_name);
        assert_eq!(
            declared.len(),
            expected_fields,
            "the field scan of {type_name} is broken, not the type: {declared:?}"
        );
        declared.sort();

        let mut covered = covered;
        covered.sort();
        covered.dedup();
        assert_eq!(
            covered, declared,
            "the fields {type_name} declares and the fields its mutation table changes have come \
             apart"
        );
    }
}

/// Every sum type this crate content-addresses, with **at least two values of every variant that
/// carries a payload**, each tagged with the position of the variant it belongs to.
///
/// One value per variant was the hole adversary pass 2 walked through. Distinctness across
/// variants is settled by the *tag*, so six payloads — `Subject::Edge`, `Subject::Type`,
/// `Predicate::Relation`, `Object::Node`, `Object::Type` and `Validating.completed` — could be
/// deleted from their encodings with all 223 cases green.
///
/// **The rule, which is the thing under test rather than the six instances: a variant's payload is
/// pinned only where that variant appears twice, with different payloads.**
/// [`every_variant_of_every_sum_type_opens_with_its_own_marker`] asserts the rule itself over these
/// fixtures, so a variant added with one value turns red without anybody remembering it.
///
/// Where a payload has parts, there is a value per part, differing in that part *alone*:
/// `Validating { 1, 2 }` against `{ 2, 2 }` against `{ 1, 3 }`. A swap — `{1,2}` against `{2,1}` —
/// is what the last round wrote, and a swap catches neither a dropped field nor a reordered one,
/// because both rows stay distinct either way.
///
/// The payloads are otherwise the *same* wherever the shapes allow, so that anything separating two
/// values of different variants is the variant tag and not the fixture.
fn sum_type_fixtures() -> Vec<(&'static str, Fixtures)> {
    let node = id::<NodeId>(11);
    let other_node = id::<NodeId>(12);
    let edge = id::<EdgeId>(11);
    let other_edge = id::<EdgeId>(12);
    let type_id = id::<TypeId>(11);
    let other_type = id::<TypeId>(12);
    let property = id::<PropertyId>(11);
    let other_property = id::<PropertyId>(12);
    let assertion_id = id::<AssertionId>(11);
    let other_assertion = id::<AssertionId>(12);
    let text = "same".to_owned();
    let other_text = "other".to_owned();
    let hash = ContentHash::of_bytes(b"one payload for all of them");
    let other_hash = ContentHash::of_bytes(b"a second payload");

    let subjects = vec![
        (0, Subject::Node(node)),
        (0, Subject::Node(other_node)),
        (1, Subject::Edge(edge)),
        (1, Subject::Edge(other_edge)),
        (2, Subject::Type(type_id)),
        (2, Subject::Type(other_type)),
    ];
    let predicates = vec![
        (0, Predicate::Property(property)),
        (0, Predicate::Property(other_property)),
        (1, Predicate::Relation(type_id)),
        (1, Predicate::Relation(other_type)),
    ];
    let objects = vec![
        (
            0,
            Object::Value(CanonicalValue::try_from(Value::String(text.clone())).expect("ok")),
        ),
        (
            0,
            Object::Value(CanonicalValue::try_from(Value::String(other_text.clone())).expect("ok")),
        ),
        (1, Object::Node(node)),
        (1, Object::Node(other_node)),
        (2, Object::Type(type_id)),
        (2, Object::Type(other_type)),
    ];
    let assessments = vec![
        // The one variant with no payload, and so the one with a single value: there is no second
        // payload for it to differ in, and its encoding is the marker alone.
        (0, Assessment::Proposed),
        (
            1,
            Assessment::Validating {
                completed: 1,
                required: 2,
            },
        ),
        (
            1,
            // `completed` alone.
            Assessment::Validating {
                completed: 2,
                required: 2,
            },
        ),
        (
            1,
            // `required` alone.
            Assessment::Validating {
                completed: 1,
                required: 3,
            },
        ),
        (
            2,
            Assessment::Accepted {
                validators: BTreeSet::from([id::<AgentId>(11)]),
            },
        ),
        (
            2,
            Assessment::Accepted {
                validators: BTreeSet::from([id::<AgentId>(12)]),
            },
        ),
        (
            3,
            Assessment::Rejected {
                issues: vec![id::<IssueId>(11)],
            },
        ),
        (
            3,
            Assessment::Rejected {
                issues: vec![id::<IssueId>(12)],
            },
        ),
        (
            4,
            Assessment::Disputed {
                competing_assertions: vec![assertion_id],
            },
        ),
        (
            4,
            Assessment::Disputed {
                competing_assertions: vec![other_assertion],
            },
        ),
    ];
    let lifecycles = vec![
        // No payload, so a single value.
        (0, AssertionLifecycle::Active),
        (
            1,
            AssertionLifecycle::Retracted {
                at_revision: RevisionNumber::new(1),
                reason: RetractionReason::new(text.clone()),
            },
        ),
        (
            1,
            // `at_revision` alone.
            AssertionLifecycle::Retracted {
                at_revision: RevisionNumber::new(2),
                reason: RetractionReason::new(text.clone()),
            },
        ),
        (
            1,
            // `reason` alone.
            AssertionLifecycle::Retracted {
                at_revision: RevisionNumber::new(1),
                reason: RetractionReason::new(other_text.clone()),
            },
        ),
        (
            2,
            AssertionLifecycle::Superseded {
                by: assertion_id,
                at_revision: RevisionNumber::new(1),
                effective_from: Timestamp::from_millis(1),
            },
        ),
        (
            2,
            // `by` alone.
            AssertionLifecycle::Superseded {
                by: other_assertion,
                at_revision: RevisionNumber::new(1),
                effective_from: Timestamp::from_millis(1),
            },
        ),
        (
            2,
            // `at_revision` alone.
            AssertionLifecycle::Superseded {
                by: assertion_id,
                at_revision: RevisionNumber::new(2),
                effective_from: Timestamp::from_millis(1),
            },
        ),
        (
            2,
            // `effective_from` alone.
            AssertionLifecycle::Superseded {
                by: assertion_id,
                at_revision: RevisionNumber::new(1),
                effective_from: Timestamp::from_millis(2),
            },
        ),
    ];
    let values: Vec<(usize, CanonicalValue)> = vec![
        (0, Value::String(text.clone())),
        (0, Value::String(other_text.clone())),
        (1, Value::Boolean(true)),
        (1, Value::Boolean(false)),
        (2, Value::Integer(1)),
        (2, Value::Integer(2)),
        (3, Value::Decimal(text.clone())),
        (3, Value::Decimal(other_text.clone())),
        (4, Value::Timestamp(Timestamp::from_millis(1))),
        (4, Value::Timestamp(Timestamp::from_millis(2))),
        (5, Value::Duration(1)),
        (5, Value::Duration(2)),
        (6, Value::NodeRef(node)),
        (6, Value::NodeRef(other_node)),
        (7, Value::Enum(text.clone())),
        (7, Value::Enum(other_text.clone())),
        (8, Value::List(vec![Value::String(text.clone())])),
        (8, Value::List(vec![Value::String(other_text.clone())])),
        (
            9,
            Value::Record([(text.clone(), Value::Integer(1))].into_iter().collect()),
        ),
        (
            9,
            Value::Record(
                [(other_text.clone(), Value::Integer(1))]
                    .into_iter()
                    .collect(),
            ),
        ),
    ]
    .into_iter()
    .map(|(position, value)| {
        (
            position,
            CanonicalValue::try_from(value).expect("admissible"),
        )
    })
    .collect();
    let sources = vec![
        (0, EvidenceSource::Url(text.clone())),
        (0, EvidenceSource::Url(other_text.clone())),
        (
            1,
            EvidenceSource::Document {
                document_id: text.clone(),
                section: Some(text.clone()),
            },
        ),
        (
            1,
            // `document_id` alone.
            EvidenceSource::Document {
                document_id: other_text.clone(),
                section: Some(text.clone()),
            },
        ),
        (
            1,
            // `section` alone, including its absence, which is not its emptiness.
            EvidenceSource::Document {
                document_id: text.clone(),
                section: None,
            },
        ),
        (
            2,
            EvidenceSource::DatabaseRecord {
                database: text.clone(),
                table: text.clone(),
                key: text.clone(),
            },
        ),
        (
            2,
            EvidenceSource::DatabaseRecord {
                database: other_text.clone(),
                table: text.clone(),
                key: text.clone(),
            },
        ),
        (
            2,
            EvidenceSource::DatabaseRecord {
                database: text.clone(),
                table: other_text.clone(),
                key: text.clone(),
            },
        ),
        (
            2,
            EvidenceSource::DatabaseRecord {
                database: text.clone(),
                table: text.clone(),
                key: other_text.clone(),
            },
        ),
        (
            3,
            EvidenceSource::GraphAssertion(CanonicalRef::new(assertion_id)),
        ),
        (
            3,
            EvidenceSource::GraphAssertion(CanonicalRef::new(other_assertion)),
        ),
        (4, EvidenceSource::Observation(id::<ObservationId>(11))),
        (4, EvidenceSource::Observation(id::<ObservationId>(12))),
        (
            5,
            EvidenceSource::HumanStatement {
                identity: Some(text.clone()),
            },
        ),
        (5, EvidenceSource::HumanStatement { identity: None }),
    ];
    let contents = vec![
        (0, ObservationContent::Document(hash)),
        (0, ObservationContent::Document(other_hash)),
        (1, ObservationContent::ApiResponse(hash)),
        (1, ObservationContent::ApiResponse(other_hash)),
        (2, ObservationContent::DatabaseRecord(hash)),
        (2, ObservationContent::DatabaseRecord(other_hash)),
        (3, ObservationContent::FeedItem(hash)),
        (3, ObservationContent::FeedItem(other_hash)),
        (4, ObservationContent::GraphFragment(hash)),
        (4, ObservationContent::GraphFragment(other_hash)),
        (5, ObservationContent::MessageBatch(hash)),
        (5, ObservationContent::MessageBatch(other_hash)),
        (6, ObservationContent::GitDiff(hash)),
        (6, ObservationContent::GitDiff(other_hash)),
        (
            7,
            ObservationContent::Blob {
                hash,
                media_type: text.clone(),
                byte_len: 1,
            },
        ),
        (
            7,
            // `hash` alone.
            ObservationContent::Blob {
                hash: other_hash,
                media_type: text.clone(),
                byte_len: 1,
            },
        ),
        (
            7,
            // `media_type` alone.
            ObservationContent::Blob {
                hash,
                media_type: other_text.clone(),
                byte_len: 1,
            },
        ),
        (
            7,
            // `byte_len` alone.
            ObservationContent::Blob {
                hash,
                media_type: text,
                byte_len: 2,
            },
        ),
    ];

    let mut fixtures = vec![
        ("Subject", encodings(subjects)),
        ("Predicate", encodings(predicates)),
        ("Object", encodings(objects)),
        ("Assessment", encodings(assessments)),
        ("AssertionLifecycle", encodings(lifecycles)),
        ("CanonicalValue", encodings(values)),
        ("EvidenceSource", encodings(sources)),
        ("ObservationContent", encodings(contents)),
    ];
    fixtures.extend(legacy_sum_type_fixtures());
    fixtures
}

/// Frozen types keep their own payload-distinguishing fixture roster.
fn legacy_sum_type_fixtures() -> Vec<(&'static str, Fixtures)> {
    use ekr_graph::legacy::Value as CanonicalValue;
    use ekr_graph::legacy::{
        EvidenceSource, Object, Predicate, RetractionReason, Subject, ValidationState, Value,
    };
    let node = id::<NodeId>(11);
    let other_node = id::<NodeId>(12);
    let edge = id::<EdgeId>(11);
    let other_edge = id::<EdgeId>(12);
    let type_id = id::<TypeId>(11);
    let other_type = id::<TypeId>(12);
    let property = id::<PropertyId>(11);
    let other_property = id::<PropertyId>(12);
    let assertion_id = id::<AssertionId>(11);
    let other_assertion = id::<AssertionId>(12);
    let text = "same".to_owned();
    let other_text = "other".to_owned();

    let subjects = vec![
        (0, Subject::Node(node)),
        (0, Subject::Node(other_node)),
        (1, Subject::Edge(edge)),
        (1, Subject::Edge(other_edge)),
        (2, Subject::Type(type_id)),
        (2, Subject::Type(other_type)),
    ];
    let predicates = vec![
        (0, Predicate::Property(property)),
        (0, Predicate::Property(other_property)),
        (1, Predicate::Relation(type_id)),
        (1, Predicate::Relation(other_type)),
    ];
    let objects = vec![
        (0, Object::Value(Value::String(text.clone()))),
        (0, Object::Value(Value::String(other_text.clone()))),
        (1, Object::Node(node)),
        (1, Object::Node(other_node)),
        (2, Object::Type(type_id)),
        (2, Object::Type(other_type)),
    ];
    let validations = vec![
        // The one variant with no payload, and so the one with a single value: there is no second
        // payload for it to differ in, and its encoding is the marker alone.
        (0, ValidationState::Proposed),
        (
            1,
            ValidationState::Validating {
                completed: 1,
                required: 2,
            },
        ),
        (
            1,
            // `completed` alone.
            ValidationState::Validating {
                completed: 2,
                required: 2,
            },
        ),
        (
            1,
            // `required` alone.
            ValidationState::Validating {
                completed: 1,
                required: 3,
            },
        ),
        (
            2,
            ValidationState::Accepted {
                validators: BTreeSet::from([id::<AgentId>(11)]),
            },
        ),
        (
            2,
            ValidationState::Accepted {
                validators: BTreeSet::from([id::<AgentId>(12)]),
            },
        ),
        (
            3,
            ValidationState::Rejected {
                issues: vec![id::<IssueId>(11)],
            },
        ),
        (
            3,
            ValidationState::Rejected {
                issues: vec![id::<IssueId>(12)],
            },
        ),
        (
            4,
            ValidationState::Disputed {
                competing_assertions: vec![assertion_id],
            },
        ),
        (
            4,
            ValidationState::Disputed {
                competing_assertions: vec![other_assertion],
            },
        ),
        (5, ValidationState::Superseded { by: assertion_id }),
        (
            5,
            ValidationState::Superseded {
                by: other_assertion,
            },
        ),
        (
            6,
            ValidationState::Retracted {
                at_revision: RevisionNumber::new(1),
                reason: RetractionReason::new(text.clone()),
            },
        ),
        (
            6,
            // `at_revision` alone.
            ValidationState::Retracted {
                at_revision: RevisionNumber::new(2),
                reason: RetractionReason::new(text.clone()),
            },
        ),
        (
            6,
            // `reason` alone.
            ValidationState::Retracted {
                at_revision: RevisionNumber::new(1),
                reason: RetractionReason::new(other_text.clone()),
            },
        ),
    ];
    let values: Vec<(usize, CanonicalValue)> = vec![
        (0, Value::String(text.clone())),
        (0, Value::String(other_text.clone())),
        (1, Value::Boolean(true)),
        (1, Value::Boolean(false)),
        (2, Value::Integer(1)),
        (2, Value::Integer(2)),
        (3, Value::Decimal(text.clone())),
        (3, Value::Decimal(other_text.clone())),
        (4, Value::Timestamp(Timestamp::from_millis(1))),
        (4, Value::Timestamp(Timestamp::from_millis(2))),
        (5, Value::Duration(1)),
        (5, Value::Duration(2)),
        (6, Value::NodeRef(node)),
        (6, Value::NodeRef(other_node)),
        (7, Value::Enum(text.clone())),
        (7, Value::Enum(other_text.clone())),
        (8, Value::List(vec![Value::String(text.clone())])),
        (8, Value::List(vec![Value::String(other_text.clone())])),
        (
            9,
            Value::Record([(text.clone(), Value::Integer(1))].into_iter().collect()),
        ),
        (
            9,
            Value::Record(
                [(other_text.clone(), Value::Integer(1))]
                    .into_iter()
                    .collect(),
            ),
        ),
    ];
    let sources = vec![
        (0, EvidenceSource::Url(text.clone())),
        (0, EvidenceSource::Url(other_text.clone())),
        (
            1,
            EvidenceSource::Document {
                document_id: text.clone(),
                section: Some(text.clone()),
            },
        ),
        (
            1,
            // `document_id` alone.
            EvidenceSource::Document {
                document_id: other_text.clone(),
                section: Some(text.clone()),
            },
        ),
        (
            1,
            // `section` alone, including its absence, which is not its emptiness.
            EvidenceSource::Document {
                document_id: text.clone(),
                section: None,
            },
        ),
        (
            2,
            EvidenceSource::DatabaseRecord {
                database: text.clone(),
                table: text.clone(),
                key: text.clone(),
            },
        ),
        (
            2,
            EvidenceSource::DatabaseRecord {
                database: other_text.clone(),
                table: text.clone(),
                key: text.clone(),
            },
        ),
        (
            2,
            EvidenceSource::DatabaseRecord {
                database: text.clone(),
                table: other_text.clone(),
                key: text.clone(),
            },
        ),
        (
            2,
            EvidenceSource::DatabaseRecord {
                database: text.clone(),
                table: text.clone(),
                key: other_text.clone(),
            },
        ),
        (3, EvidenceSource::GraphAssertion(assertion_id)),
        (3, EvidenceSource::GraphAssertion(other_assertion)),
        (4, EvidenceSource::Observation(id::<ObservationId>(11))),
        (4, EvidenceSource::Observation(id::<ObservationId>(12))),
        (
            5,
            EvidenceSource::HumanStatement {
                identity: Some(text.clone()),
            },
        ),
        (5, EvidenceSource::HumanStatement { identity: None }),
    ];
    vec![
        ("legacy::Subject", encodings(subjects)),
        ("legacy::Predicate", encodings(predicates)),
        ("legacy::Object", encodings(objects)),
        ("legacy::ValidationState", encodings(validations)),
        ("legacy::Value", encodings(values)),
        ("legacy::EvidenceSource", encodings(sources)),
    ]
}

/// One sum type's fixtures: for each, the position of the variant it belongs to and its bytes.
type Fixtures = Vec<(usize, Vec<u8>)>;

/// Each value's canonical bytes, keeping the variant position it was tagged with.
fn encodings<T: Canonical>(values: Vec<(usize, T)>) -> Fixtures {
    values
        .into_iter()
        .map(|(position, value)| (position, value.canonical_bytes()))
        .collect()
}

/// Every variant opens with its own marker, no two values of one type share an encoding, and every
/// variant that carries a payload is present twice.
///
/// The third of those is the rule adversary pass 2 derived, asserted rather than remembered: a
/// variant's payload is pinned **only** where the variant appears twice with different payloads, so
/// a fixture list that names a payload-carrying variant once is a hole whatever else it checks. A
/// variant carrying no payload encodes as the five-byte marker alone and is the one case that is
/// allowed to appear once — and that is read off the bytes, not off a list kept here.
#[test]
fn every_variant_of_every_sum_type_opens_with_its_own_marker() {
    /// The marker `Encoder::variant` writes: the tag byte, then the index, big-endian.
    fn marker(position: usize) -> Vec<u8> {
        let index = u32::try_from(position).expect("a small enum");
        let mut bytes = vec![0x0f_u8];
        bytes.extend_from_slice(&index.to_be_bytes());
        bytes
    }

    // Collected across every type and reported once. A case that stops at the first type reports
    // one fault for a mutation that broke six, and a measurement made with it is not a
    // measurement (adversary pass 2, finding 6).
    let mut faults: Vec<String> = Vec::new();

    for (type_name, fixtures) in sum_type_fixtures() {
        assert!(!fixtures.is_empty(), "{type_name} has no fixtures");

        for (position, bytes) in &fixtures {
            assert_eq!(
                &bytes[..5],
                &marker(*position)[..],
                "a {type_name} fixture for variant {position} does not open with that variant's \
                 marker; a renumbering moves every address that contains it"
            );
        }

        let labelled: Vec<(String, Vec<u8>)> = fixtures
            .iter()
            .enumerate()
            .map(|(at, (position, bytes))| {
                (
                    format!("{type_name}'s fixture {at} of variant {position}"),
                    bytes.clone(),
                )
            })
            .collect();
        let shared = collisions(&labelled);
        faults.extend(shared.iter().cloned());

        // The rule: a payload is pinned only where its variant appears twice. Asked only where the
        // encodings are distinct — where they are not, the collisions above are the finding, and
        // this would report each of them a second time in other words.
        if !shared.is_empty() {
            continue;
        }
        let highest = fixtures
            .iter()
            .map(|(position, _)| *position)
            .max()
            .expect("a sum type has a variant");
        for position in 0..=highest {
            let of_this_variant: Vec<&Vec<u8>> = fixtures
                .iter()
                .filter(|(at, _)| *at == position)
                .map(|(_, bytes)| bytes)
                .collect();
            assert!(
                !of_this_variant.is_empty(),
                "{type_name} has no fixture for variant {position}"
            );
            let carries_a_payload = of_this_variant.iter().any(|bytes| bytes.len() > 5);
            if carries_a_payload != (of_this_variant.len() > 1) {
                faults.push(format!(
                    "{type_name}'s variant {position} carries a payload and appears {} time(s) \
                     in the fixtures. A payload is pinned only where its variant appears twice \
                     with different payloads: with one value, deleting the line that encodes it \
                     leaves every case green. (A variant with no payload appears once.)",
                    of_this_variant.len()
                ));
            }
        }
    }

    assert!(
        faults.is_empty(),
        "{} fault(s) across the crate's sum types: {faults:#?}",
        faults.len()
    );
}

#[test]
fn the_declaration_order_of_every_sum_type_equals_its_numbering() {
    let modules = crate_modules();
    let pinned_elsewhere = std::fs::read_to_string(
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo crate path"))
            .join("tests/revision_events.rs"),
    )
    .expect("the sibling case that pins the other form");

    let mut checked: Vec<String> = Vec::new();
    for (name, source) in &modules {
        for type_name in canonical_implementors(source) {
            if declaration_body(source, "pub enum", &type_name).is_none() {
                continue; // a struct or a newtype: rule 5 makes it structural, with no tag.
            }
            let body = implementation_body(source, &type_name);
            assert!(
                body.contains("out.variant("),
                "{type_name} is a sum type this crate encodes and writes no variant tag, so two \
                 of its variants carrying one payload shape would share an address ({name})"
            );

            if body.contains("out.variant(self.variant_index())") {
                assert!(
                    pinned_elsewhere.contains(&type_name),
                    "{type_name} numbers its variants through variant_index() and \
                     tests/revision_events.rs does not mention it, so nothing pins the numbering"
                );
                continue;
            }

            let declared = variants_of(source, &type_name);
            let numbered = tagged_arms(&body);
            assert!(
                declared.len() > 1,
                "{type_name} was scanned as {declared:?}: the declaration scan is broken, not the \
                 type — a sum type this crate encodes has at least two variants"
            );
            assert_eq!(
                declared, numbered,
                "{type_name}'s variants are declared in one order and numbered in another. \
                 Nothing derives one from the other: the index is a literal in the encoding. \
                 Changing a number moves every address that contains that variant."
            );
            checked.push(if name == "legacy.rs" {
                format!("legacy::{type_name}")
            } else {
                type_name
            });
        }
    }

    checked.sort();
    assert_eq!(
        checked,
        vec![
            "AssertionLifecycle".to_owned(),
            "Assessment".to_owned(),
            "CanonicalValue".to_owned(),
            "EvidenceSource".to_owned(),
            "Object".to_owned(),
            "ObservationContent".to_owned(),
            "Predicate".to_owned(),
            "Subject".to_owned(),
            "legacy::EvidenceSource".to_owned(),
            "legacy::Object".to_owned(),
            "legacy::Predicate".to_owned(),
            "legacy::Subject".to_owned(),
            "legacy::ValidationState".to_owned(),
            "legacy::Value".to_owned(),
        ],
        "the scan found a different set of literally numbered sum types than the fixtures cover"
    );
    assert_eq!(
        checked.len(),
        sum_type_fixtures().len(),
        "a sum type the scan found has no fixture in this file"
    );
}

/// Every `.rs` file of the crate's `src/`, by file name.
fn crate_modules() -> Vec<(String, String)> {
    let directory =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo crate path"))
            .join("src");
    let mut found: Vec<(String, String)> = std::fs::read_dir(directory)
        .expect("the crate has a src/")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .map(|path| {
            (
                path.file_name()
                    .expect("a file has a name")
                    .to_string_lossy()
                    .into_owned(),
                std::fs::read_to_string(&path).expect("a source file"),
            )
        })
        .collect();
    found.sort();
    assert!(found.len() >= 10, "the module scan is broken: {found:?}");
    found
}

/// Every type this crate implements `Canonical` for, in source order.
///
/// Both spellings: `impl Canonical for Root {` and the bounded generic
/// `impl<V: Canonical> Canonical for Node<V> {`, which is what the amended ADR 0005 makes the three
/// types both spaces hold. A scan that read only the first would report a generic type as having no
/// implementation at all.
fn canonical_implementors(source: &str) -> Vec<String> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("impl") && line.contains(" Canonical for "))
        .filter_map(|line| line.split(" Canonical for ").nth(1))
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect()
        })
        .collect()
}

/// The body of the `Canonical` implementation for one type, whatever bounds it carries.
fn implementation_body(source: &str, type_name: &str) -> String {
    let at = canonical_impl_head(source, type_name)
        .unwrap_or_else(|| panic!("the crate implements Canonical for {type_name}"));
    block_from(source, at)
}

/// The `{` that opens `impl … Canonical for <type_name> …`, whatever bounds the implementation
/// carries.
///
/// **Structural, and deliberately not a list of spellings.** The list was two —
/// `impl Canonical for Root {` and `impl<V: Canonical> Canonical for Node<V> {` — and
/// `architecture-decision-record:0008-canonical-state-references-are-typed` added a third,
/// `impl<V: ValueSpace + Canonical> Canonical for Edge<V> {`, at which point a scan enumerating
/// spellings reported the type as having *no implementation at all*. A rule enumerated by its
/// instances has a next instance; this one reads the shape — a line beginning `impl`, naming
/// `Canonical for` the type at an identifier boundary, and opening a block.
fn canonical_impl_head(source: &str, type_name: &str) -> Option<usize> {
    let needle = format!(" Canonical for {type_name}");
    source.match_indices(&needle).find_map(|(at, _)| {
        let line_start = source[..at].rfind('\n').map_or(0, |n| n + 1);
        if !source[line_start..at].trim_start().starts_with("impl") {
            return None;
        }
        // The next character after the name is what keeps `Node` from matching `NodeDraft`.
        if !source[at + needle.len()..].starts_with(['<', ' ', '{']) {
            return None;
        }
        let line_end = source[at..]
            .find('\n')
            .map_or(source.len(), |offset| at + offset);
        source[at..line_end].rfind('{').map(|offset| at + offset)
    })
}

/// The text from the `{` at `at` through the `}` that matches it.
///
/// **Stated bound:** braces are counted, not parsed, so an unbalanced brace inside a doc comment or
/// a string literal in the scanned region would send it to the wrong `}`. No such brace exists in
/// this crate and the cost of a real parser is not worth an unreachable case; the bound is written
/// here rather than left to be discovered.
fn block_from(source: &str, at: usize) -> String {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    for (offset, byte) in bytes[at..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return source[at..=at + offset].to_owned();
                }
            }
            _ => {}
        }
    }
    panic!("an item's braces do not balance");
}

/// The variant names of one enum, in declaration order.
///
/// All four written forms: `Proposed,` (unit), `Validating {` (struct), `Node(NodeId),` (tuple on
/// one line) and `Node(` (tuple broken across lines). The first version of this read the last
/// three only — the sibling scan in `tests/revision_events.rs` was written against a type whose
/// variants are all struct-form — and answered `[]` for `Subject`, which is the failure a scan
/// makes look like a clean bill of health. The vacuity assertion beside the call is what turned
/// that into a red case rather than a silent pass.
fn variants_of(source: &str, type_name: &str) -> Vec<String> {
    let body = declaration_body(source, "pub enum", type_name)
        .unwrap_or_else(|| panic!("the crate declares enum {type_name}"));
    body.lines()
        .skip(1)
        .take_while(|line| *line != "}")
        .filter(|line| line.starts_with("    ") && !line.starts_with("     "))
        .filter_map(|line| {
            let head = line.trim();
            let name: String = head.chars().take_while(|c| c.is_alphanumeric()).collect();
            let rest = &head[name.len()..];
            (!name.is_empty()
                && name.starts_with(char::is_uppercase)
                && (rest == "," || rest == " {" || rest.starts_with('(')))
            .then_some(name)
        })
        .collect()
}

/// The variants an encoding tags, in the order its arms write them, checked to be `0..n`.
fn tagged_arms(body: &str) -> Vec<String> {
    let mut named: Option<String> = None;
    let mut found: Vec<(u32, String)> = Vec::new();
    for line in body.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("Self::") {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric()).collect();
            // A unit arm is written on one line: `Self::Proposed => out.variant(0),`.
            if let Some((_, tagged)) = rest.split_once("=> out.variant(") {
                let index: u32 = tagged
                    .split(')')
                    .next()
                    .expect("the call is closed")
                    .parse()
                    .expect("the index is a literal");
                found.push((index, name));
                named = None;
            } else {
                named = Some(name);
            }
        } else if let Some(rest) = line.strip_prefix("out.variant(") {
            let index: u32 = rest
                .split(')')
                .next()
                .expect("the call is closed")
                .parse()
                .expect("the index is a literal");
            found.push((index, named.clone().expect("an arm precedes its tag")));
        }
    }
    assert_eq!(
        found.iter().map(|(index, _)| *index).collect::<Vec<_>>(),
        (0..u32::try_from(found.len()).expect("a small enum")).collect::<Vec<_>>(),
        "the variant numbers are not 0..n in the order the arms are written: {found:?}"
    );
    found.into_iter().map(|(_, name)| name).collect()
}

#[test]
fn confidence_refusal_preserves_the_public_error_and_its_bound() {
    let refused: ekr_graph::ConfidenceOutOfRange = Confidence::try_from(10_001).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "10001 is not a confidence: expected basis points between 0 and 10000"
    );
    assert_eq!(Confidence::try_from(10_000).unwrap(), Confidence::CERTAIN);
}

#[test]
fn public_canonical_targets_preserve_deserialized_identity_and_address() {
    // Each kind round-trips the id of its own kind, which is what `CanonicalTarget::Id` now
    // says. `Observation`, `Support` and `GraphRoot` were targets and are not: canonical state
    // keeps no map of them, and `compile_fail/a_canonical_reference_targets_only_what_canonical_state_holds.rs`
    // holds that `CanonicalRef` of any of them is not a type.
    fn round_trip<T: ekr_graph::CanonicalTarget>(id: T::Id) {
        let reference = CanonicalRef::<T>::new(id);
        let text = id.to_string();
        let input = serde::de::value::StrDeserializer::<serde::de::value::Error>::new(&text);
        let restored: CanonicalRef<T> = serde::Deserialize::deserialize(input).unwrap();
        assert_eq!(restored.canonical_bytes(), id.canonical_bytes());
        assert_eq!(ContentHash::of(&restored), ContentHash::of(&reference));
    }
    round_trip::<Node>(NodeId::mint());
    round_trip::<Edge>(EdgeId::mint());
    round_trip::<Assertion>(AssertionId::mint());
    round_trip::<Evidence>(EvidenceId::mint());
}

/// Observe the actual typed refusal passed by the range's TryFrom into serde::Error::custom.
/// A formatted serde_json::Error alone would erase which error supplied the message.
#[derive(Debug)]
struct CapturedRangeError {
    producer: &'static str,
    message: String,
}

impl std::fmt::Display for CapturedRangeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CapturedRangeError {}

impl serde::de::Error for CapturedRangeError {
    fn custom<T: std::fmt::Display>(message: T) -> Self {
        Self {
            producer: std::any::type_name::<T>(),
            message: message.to_string(),
        }
    }
}

struct RangeMillis(i64);

impl<'de> serde::de::IntoDeserializer<'de, CapturedRangeError> for RangeMillis {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self {
        self
    }
}

impl<'de> serde::Deserializer<'de> for RangeMillis {
    type Error = CapturedRangeError;

    fn deserialize_any<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_i64(self.0)
    }

    fn deserialize_option<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_some(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any
    }
}

#[test]
fn range_serde_refusals_keep_the_actual_current_and_frozen_error_type() {
    fn decode<T: serde::de::DeserializeOwned>(from: i64, to: i64) -> Result<T, CapturedRangeError> {
        let fields = [("from", RangeMillis(from)), ("to", RangeMillis(to))];
        T::deserialize(serde::de::value::MapDeserializer::new(fields.into_iter()))
    }
    let current = decode::<TemporalRange>(2, 1).unwrap_err();
    assert_eq!(
        current.producer,
        std::any::type_name::<ekr_graph::InvertedRange>()
    );
    assert_eq!(
        current.message,
        "1 precedes 2: a range whose end precedes its start contains no instant"
    );
    let frozen = decode::<ekr_graph::legacy::TemporalRange>(2, 1).unwrap_err();
    assert_eq!(
        frozen.producer,
        std::any::type_name::<ekr_graph::legacy::InvertedRange>()
    );
    assert_eq!(frozen.message, current.message);
    assert!(decode::<TemporalRange>(1, 2).is_ok());
    assert!(decode::<ekr_graph::legacy::TemporalRange>(1, 2).is_ok());
}
