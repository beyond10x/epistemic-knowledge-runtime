//! Adversarial cases, pass 2, for `story:ontology-types-and-values`.
//!
//! Round 1 replaced a merge over the ancestor *set* with a breadth-first walk, and added a new
//! refusal — `OntologyError::AmbiguousProperty` — for two declarations of one property at the same
//! breadth-first distance. These cases are about the gap between *distance* and *specificity*.
//!
//! In a type hierarchy the two are not the same relation. `P` is a strict descendant of `G`, so
//! `P`'s declaration of a property is a redeclaration of `G`'s and is the more specific of the
//! two, *whatever* breadth-first distances a third type happens to see them at. When the two
//! relations disagree, `resolve_properties` follows distance: once by refusing a document that the
//! hierarchy resolves on its own, and once by silently preferring the declaration that was
//! redeclared.
//!
//! Every id is parsed from a fixed canonical UUID rather than minted, so that the sort order of a
//! fixture is chosen and not drawn.

use std::collections::BTreeMap;

use ekr_core::{PropertyId, Timestamp, TypeId};
use ekr_ontology::{
    NodeType, Ontology, OntologyDocument, OntologyError, PropertyDefinition, SchemaVersion, Value,
    ValueKind, ValueType,
};

/// A `TypeId` from a fixed canonical UUID.
fn type_id(tail: &str) -> TypeId {
    format!("018f2a00-0000-7000-8000-0000000001{tail}")
        .parse()
        .expect("a canonical uuid")
}

/// The one property id every case here redeclares.
fn property_id() -> PropertyId {
    "018f2a00-0000-7000-8000-0000000001e0"
        .parse()
        .expect("a canonical uuid")
}

/// The seed version every document here carries.
fn seed() -> SchemaVersion {
    SchemaVersion::seed(
        "018f2a00-0000-7000-8000-0000000001f0"
            .parse()
            .expect("a canonical uuid"),
        Timestamp::EPOCH,
    )
}

/// A node type with the given parents, optionally declaring the one property.
fn node(id: TypeId, name: &str, parents: &[TypeId], declares: Option<ValueType>) -> NodeType {
    let mut declared = NodeType::new(id, name);
    for parent in parents {
        declared.parents.insert(*parent);
    }
    if let Some(value_type) = declares {
        let property = property_id();
        declared
            .properties
            .insert(property, PropertyDefinition::new(property, "p", value_type));
    }
    declared
}

/// Every permutation of three fixed ids, so that "which id sorts first" is enumerated rather than
/// drawn — both cases below must hold for all six.
fn orderings() -> Vec<[TypeId; 3]> {
    let [a, b, c] = [type_id("a0"), type_id("b0"), type_id("c0")];
    vec![
        [a, b, c],
        [a, c, b],
        [b, a, c],
        [b, c, a],
        [c, a, b],
        [c, b, a],
    ]
}

/// A parent that specialises another parent is not an ambiguous declaration.
///
/// `grand` declares `p` as a `String`; `parent` has `grand` as its parent and redeclares `p` as an
/// `Integer`; `child` names **both** `parent` and `grand` as parents. That is the ordinary
/// redundant-parent diamond — a type naming the base it already reaches through a refinement of
/// the base — and design § 11.1 permits it, because `NodeType::parents` is a set with no rule
/// against it.
///
/// Both declarations sit at breadth-first distance 1 from `child`, so `resolve_properties` calls
/// the document ambiguous and `Ontology::load` refuses it. The refusal says "nothing but an id
/// order could choose between them", and that sentence is untrue of this document: `parent` is a
/// strict descendant of `grand`, so `parent`'s declaration is the redeclaration and the hierarchy
/// the document itself declares chooses between them without reference to any id.
///
/// The document is well formed, `Ontology::load` must accept it, and `child` must resolve `p` to
/// `parent`'s `Integer`.
#[test]
fn a_parent_that_specialises_another_parent_is_not_an_ambiguous_declaration() {
    let property = property_id();

    for [grand, parent, child] in orderings() {
        let document = OntologyDocument {
            version: seed(),
            node_types: vec![
                node(grand, "Grand", &[], Some(ValueType::String)),
                node(parent, "Parent", &[grand], Some(ValueType::Integer)),
                node(child, "Child", &[parent, grand], None),
            ],
            edge_types: Vec::new(),
        };

        let loaded = Ontology::load(document).unwrap_or_else(|refused| {
            panic!(
                "a type naming both a base and a refinement of that base is a well-formed \
                 document, and the hierarchy chooses between the two declarations without any id \
                 order: {parent} is a strict descendant of {grand}. Refused with: {refused}"
            )
        });

        assert_eq!(
            loaded
                .properties_of(child)
                .get(&property)
                .map(|found| &found.value_type),
            Some(&ValueType::Integer),
            "{child} must resolve p to the redeclaration in {parent}, not to {grand}'s"
        );
    }
}

/// A redeclaration is never overridden by the declaration it redeclares.
///
/// `grand` declares `p` as a `String`. `mid` has `grand` as its parent and redeclares `p` as an
/// `Integer`. `parent` has `mid` as its parent and declares nothing. `child` names `grand` and
/// `parent` as its parents.
///
/// Breadth-first from `child`, `grand` is at distance 1 and `mid` is at distance 2, so
/// `resolve_properties` takes `grand`'s `String` and never looks at `mid`'s redeclaration — no
/// refusal, no diagnostic, just the wrong declaration. But `mid` is a strict descendant of `grand`
/// and an ancestor of `child`, so `mid`'s declaration is the specialisation of exactly the thing
/// that won.
///
/// The effect is visible through the checker and not only through `properties_of`: `child` accepts
/// a `String` for `p` and refuses an `Integer`, which is the opposite of what the nearest
/// redeclaring ancestor on its own chain declares.
#[test]
fn a_redeclaration_is_never_overridden_by_the_declaration_it_redeclares() {
    let property = property_id();

    for [grand, mid, parent] in orderings() {
        let child = type_id("d0");
        let document = OntologyDocument {
            version: seed(),
            node_types: vec![
                node(grand, "Grand", &[], Some(ValueType::String)),
                node(mid, "Mid", &[grand], Some(ValueType::Integer)),
                node(parent, "Parent", &[mid], None),
                node(child, "Child", &[grand, parent], None),
            ],
            edge_types: Vec::new(),
        };

        let loaded = Ontology::load(document).expect("the hierarchy closes and declares no cycle");

        assert!(
            loaded.conforms_to(child, mid),
            "{child} reaches {mid} through {parent}, so it conforms to it"
        );
        assert_eq!(
            loaded
                .properties_of(child)
                .get(&property)
                .map(|found| &found.value_type),
            Some(&ValueType::Integer),
            "{mid} redeclares p and is a strict descendant of {grand}, so {mid}'s declaration is \
             the more specific one and must win for {child}, whatever breadth-first distance each \
             sits at"
        );

        let nodes: BTreeMap<ekr_core::NodeId, TypeId> = BTreeMap::new();
        let integer: BTreeMap<_, _> = [(property, vec![Value::Integer(1)])].into_iter().collect();
        assert!(
            loaded.check_node(child, &integer, &nodes).is_ok(),
            "{child} inherits {mid}'s Integer declaration and must accept an Integer"
        );
    }
}

/// The reason a malformed document is refused is still decided by id order when the document
/// carries two faults at once.
///
/// `inheritance_and_declaration_coherence.rs` asserts the rule by name —
/// `the_reason_a_malformed_hierarchy_is_refused_never_depends_on_id_order` — and every document it
/// enumerates carries exactly one fault. This one carries two, both true of it: `looper` is its
/// own parent, and `child` names a parent the document does not declare.
///
/// `check_declarations` walks `node_types` in `TypeId` order and returns at the first fault it
/// meets, so the reason a reader is given is `CyclicParents` when `looper` sorts first and
/// `UnknownType` when `child` does. Two readers of the same document are sent after two different
/// defects, which is the thing round 1's finding 2 was about.
#[test]
fn the_reason_a_two_fault_document_is_refused_does_not_depend_on_id_order() {
    let undeclared = type_id("f9");
    let low = type_id("10");
    let high = type_id("20");

    let mut reported = Vec::new();
    for (looper, child) in [(low, high), (high, low)] {
        let document = OntologyDocument {
            version: seed(),
            node_types: vec![
                node(looper, "Looper", &[looper], None),
                node(child, "Child", &[looper, undeclared], None),
            ],
            edge_types: Vec::new(),
        };
        let refused = Ontology::load(document).expect_err("the document carries two faults");
        reported.push(match refused {
            OntologyError::CyclicParents { .. } => "CyclicParents",
            OntologyError::UnknownType { .. } => "UnknownType",
            other => panic!("neither fault was reported: {other}"),
        });
    }

    assert_eq!(
        reported[0], reported[1],
        "one document, two true faults, and the reason it is refused changed when the ids were \
         swapped: {reported:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// The class findings 1 and 2 are instances of.
// ---------------------------------------------------------------------------------------------

/// What one shape expects of the subject type's resolved declaration.
enum Expect {
    /// The property resolves, to this declared type.
    Resolves(ValueType),
    /// The document does not choose, and is refused at load.
    Ambiguous,
}

/// A hierarchy, written by index so that the ids can be permuted underneath it.
struct Shape {
    what: &'static str,
    /// Per type: its parents, by index.
    parents: &'static [&'static [usize]],
    /// Per type: the declared type of `p`, if it declares it.
    declares: &'static [Option<ValueKind>],
    /// The index whose property set is resolved.
    subject: usize,
    expect: Expect,
}

/// Every permutation of `count` of the fixed ids, so that "which id sorts first" is enumerated
/// rather than drawn. Both findings were invisible under some orderings and fatal under others.
fn permutations(count: usize) -> Vec<Vec<TypeId>> {
    let pool: Vec<TypeId> = ["20", "21", "22", "23", "24", "25"]
        .into_iter()
        .take(count)
        .map(type_id)
        .collect();
    let mut out = Vec::new();
    permute(&mut pool.clone(), 0, &mut out);
    out
}

fn permute(ids: &mut Vec<TypeId>, at: usize, out: &mut Vec<Vec<TypeId>>) {
    if at + 1 >= ids.len() {
        out.push(ids.clone());
        return;
    }
    for next in at..ids.len() {
        ids.swap(at, next);
        permute(ids, at + 1, out);
        ids.swap(at, next);
    }
}

/// A scalar `ValueType` for a kind, so that a shape's table stays readable.
fn declared_as(kind: ValueKind) -> ValueType {
    match kind {
        ValueKind::String => ValueType::String,
        ValueKind::Integer => ValueType::Integer,
        ValueKind::Boolean => ValueType::Boolean,
        other => panic!("{other} is not used by these shapes"),
    }
}

/// **The class of findings 1 and 2: a declaration is chosen by the specialisation order the
/// document declares, and by nothing else.**
///
/// Both findings are one defect seen twice — `resolve_properties` compared declarations by
/// breadth-first *distance*, and the question is which declaration is most *specific*. The two
/// agree on a chain, which is why a chain-only suite missed it, and they disagree the moment a
/// type reaches one ancestor by two routes of different length.
///
/// Distance is not in this table at all. Each shape says which declaration must win and why in
/// terms of the hierarchy, and every shape is run under every permutation of its ids — so neither
/// a distance rule nor an id-order rule can pass it, whatever fixture a later change is tuned to.
#[test]
fn a_declaration_is_chosen_by_the_specialisation_order_and_by_nothing_else() {
    let property = property_id();

    let shapes = [
        Shape {
            what: "a chain: the descendant's redeclaration wins over the one it redeclares",
            parents: &[&[], &[0], &[1]],
            declares: &[Some(ValueKind::String), Some(ValueKind::Integer), None],
            subject: 2,
            expect: Expect::Resolves(ValueType::Integer),
        },
        Shape {
            what: "a type's own declaration wins over every ancestor's",
            parents: &[&[], &[0], &[1]],
            declares: &[
                Some(ValueKind::String),
                Some(ValueKind::Integer),
                Some(ValueKind::Boolean),
            ],
            subject: 2,
            expect: Expect::Resolves(ValueType::Boolean),
        },
        Shape {
            what: "a redundant parent: naming both a base and a refinement of it is well formed, \
                   and the refinement wins — the two sit at one distance and are not ambiguous",
            parents: &[&[], &[0], &[1, 0]],
            declares: &[Some(ValueKind::String), Some(ValueKind::Integer), None],
            subject: 2,
            expect: Expect::Resolves(ValueType::Integer),
        },
        Shape {
            what: "the long branch: the redeclaration is farther away than the thing it \
                   redeclares, and still wins",
            parents: &[&[], &[0], &[1], &[0, 2]],
            declares: &[
                Some(ValueKind::String),
                Some(ValueKind::Integer),
                None,
                None,
            ],
            subject: 3,
            expect: Expect::Resolves(ValueType::Integer),
        },
        Shape {
            what: "a longer branch still: distance 1 against distance 3, and specificity decides",
            parents: &[&[], &[0], &[1], &[2], &[0, 3]],
            declares: &[
                Some(ValueKind::String),
                None,
                None,
                Some(ValueKind::Integer),
                None,
            ],
            subject: 4,
            expect: Expect::Resolves(ValueType::Integer),
        },
        Shape {
            what: "three declarations on one chain: the most derived wins, not the middle one",
            parents: &[&[], &[0], &[1], &[2]],
            declares: &[
                Some(ValueKind::String),
                Some(ValueKind::Integer),
                Some(ValueKind::Boolean),
                None,
            ],
            subject: 3,
            expect: Expect::Resolves(ValueType::Boolean),
        },
        Shape {
            what: "two unrelated declarations: neither is an ancestor of the other, so the \
                   document really does fail to choose",
            parents: &[&[], &[], &[0, 1]],
            declares: &[Some(ValueKind::String), Some(ValueKind::Integer), None],
            subject: 2,
            expect: Expect::Ambiguous,
        },
        Shape {
            what: "two unrelated declarations that agree: nothing to choose between, so nothing \
                   is refused",
            parents: &[&[], &[], &[0, 1]],
            declares: &[Some(ValueKind::String), Some(ValueKind::String), None],
            subject: 2,
            expect: Expect::Resolves(ValueType::String),
        },
        Shape {
            what: "an unrelated pair under a nearer redeclaration: the subject's own branch \
                   settles it and there is no ambiguity to report",
            parents: &[&[], &[], &[0, 1]],
            declares: &[
                Some(ValueKind::String),
                Some(ValueKind::Integer),
                Some(ValueKind::Boolean),
            ],
            subject: 2,
            expect: Expect::Resolves(ValueType::Boolean),
        },
    ];

    for shape in shapes {
        let count = shape.parents.len();
        assert_eq!(shape.declares.len(), count, "{}", shape.what);

        for ids in permutations(count) {
            let node_types: Vec<NodeType> = (0..count)
                .map(|at| {
                    let parents: Vec<TypeId> = shape.parents[at].iter().map(|p| ids[*p]).collect();
                    node(ids[at], "T", &parents, shape.declares[at].map(declared_as))
                })
                .collect();
            let document = OntologyDocument {
                version: seed(),
                node_types,
                edge_types: Vec::new(),
            };
            let subject = ids[shape.subject];

            match &shape.expect {
                Expect::Resolves(expected) => {
                    let loaded = Ontology::load(document).unwrap_or_else(|refused| {
                        panic!(
                            "{}: the document is well formed, and was refused with: {refused}",
                            shape.what
                        )
                    });
                    assert_eq!(
                        loaded
                            .properties_of(subject)
                            .get(&property)
                            .map(|found| &found.value_type),
                        Some(expected),
                        "{}: ids were {ids:?}",
                        shape.what
                    );
                }
                Expect::Ambiguous => {
                    assert_eq!(
                        Ontology::load(document).err(),
                        Some(OntologyError::AmbiguousProperty {
                            type_id: subject,
                            property
                        }),
                        "{}: ids were {ids:?}",
                        shape.what
                    );
                }
            }
        }
    }
}

/// `AmbiguousProperty` means what its name says, and the refusal's own words are true of the
/// document that provoked it.
///
/// The round-1 refusal said "nothing but an id order could choose between them" about a document
/// in which the hierarchy chose perfectly well. A refusal that misstates the defect is the class
/// of round-1 finding 2 wearing different clothes, so the words are asserted and not only the
/// variant.
#[test]
fn the_ambiguity_refusal_only_fires_when_neither_declaring_type_is_an_ancestor_of_the_other() {
    let property = property_id();
    let [left, right, child] = [type_id("30"), type_id("31"), type_id("32")];

    let refused = Ontology::load(OntologyDocument {
        version: seed(),
        node_types: vec![
            node(left, "Left", &[], Some(ValueType::String)),
            node(right, "Right", &[], Some(ValueType::Integer)),
            node(child, "Child", &[left, right], None),
        ],
        edge_types: Vec::new(),
    })
    .expect_err("neither parent is an ancestor of the other");
    assert_eq!(
        refused,
        OntologyError::AmbiguousProperty {
            type_id: child,
            property
        }
    );

    let words = refused.to_string();
    assert!(
        words.contains(&child.to_string()) && words.contains(&property.to_string()),
        "the refusal names the type and the property: {words}"
    );
    assert!(
        !words.contains("distance"),
        "the refusal must not explain itself by a distance it no longer uses: {words}"
    );

    // And the one-edge change that makes the same document well formed: relate the two.
    let loaded = Ontology::load(OntologyDocument {
        version: seed(),
        node_types: vec![
            node(left, "Left", &[], Some(ValueType::String)),
            node(right, "Right", &[left], Some(ValueType::Integer)),
            node(child, "Child", &[left, right], None),
        ],
        edge_types: Vec::new(),
    })
    .expect("once Right specialises Left, the hierarchy chooses");
    assert_eq!(
        loaded
            .properties_of(child)
            .get(&property)
            .map(|found| &found.value_type),
        Some(&ValueType::Integer)
    );
}

/// The other place this crate asks a hierarchy to choose, held to the same rule.
///
/// The class is "an order over declarations that is not the order the document declares", and
/// `resolve_properties` was one of exactly two sites that ask a hierarchy a question. The other is
/// the checker's `NodeRef` arm, which asks whether a node's type may stand where an allowed type
/// is required — and it has used `conforms_to` since the first commit, never a distance. It is
/// asserted here so that the enumeration of the class is in the suite rather than only in a
/// report.
#[test]
fn the_checker_resolves_allowed_types_by_the_same_order() {
    let [base, refinement, unrelated] = [type_id("40"), type_id("41"), type_id("42")];
    let subject = type_id("43");
    let property = property_id();

    let mut subject_type = node(subject, "Subject", &[], None);
    subject_type.properties.insert(
        property,
        PropertyDefinition::new(
            property,
            "p",
            ValueType::NodeRef {
                allowed_types: [base].into_iter().collect(),
            },
        ),
    );

    let loaded = Ontology::load(OntologyDocument {
        version: seed(),
        node_types: vec![
            node(base, "Base", &[], None),
            node(refinement, "Refinement", &[base], None),
            node(unrelated, "Unrelated", &[], None),
            subject_type,
        ],
        edge_types: Vec::new(),
    })
    .expect("the hierarchy closes");

    let refined_node = ekr_core::NodeId::mint();
    let unrelated_node = ekr_core::NodeId::mint();
    let nodes: BTreeMap<ekr_core::NodeId, TypeId> =
        [(refined_node, refinement), (unrelated_node, unrelated)]
            .into_iter()
            .collect();

    let check = |node_id| {
        loaded.check_node(
            subject,
            &[(property, vec![Value::NodeRef(node_id)])]
                .into_iter()
                .collect(),
            &nodes,
        )
    };
    assert!(
        check(refined_node).is_ok(),
        "a Refinement is a Base, however many edges away"
    );
    assert!(
        check(unrelated_node).is_err(),
        "an Unrelated is not a Base at any distance"
    );
}

/// A broken hierarchy is reported as a broken hierarchy, even when a property set resolved against
/// it would be reported as ambiguous first.
///
/// This pins the ordering the specificity fix rests on. `resolve_properties` asks the hierarchy
/// which of two declarations is the more specific, so its answer is only meaningful once the
/// hierarchy closes — and `AmbiguousProperty` says "neither declaring type specialises the other",
/// which is a claim *about* the hierarchy. A document whose hierarchy is broken cannot support
/// that claim, so every hierarchy fault is reported before any resolution fault, and document
/// order decides only within each pass.
///
/// Without that split the reason would be decided by which fault the document happened to declare
/// first, and a reader of a cyclic document would be sent after an ambiguity instead — a false
/// reason of exactly the kind round-1 finding 2 was about.
///
/// The document below carries both faults at once, with the *ambiguity first* in declaration
/// order, so a single-pass walk reports the ambiguity and this case fails.
#[test]
fn a_hierarchy_fault_is_reported_before_a_property_resolution_fault() {
    let property = property_id();
    let [left, right, child] = [type_id("50"), type_id("51"), type_id("52")];
    let looper = type_id("53");

    let mut child_type = NodeType::new(child, "Child");
    child_type.parents.insert(left);
    child_type.parents.insert(right);
    let declaring = |id: TypeId, name: &str, value_type: ValueType| {
        let mut declared = NodeType::new(id, name);
        declared
            .properties
            .insert(property, PropertyDefinition::new(property, "p", value_type));
        declared
    };
    let mut self_looping = NodeType::new(looper, "Looper");
    self_looping.parents.insert(looper);

    let refused = Ontology::load(OntologyDocument {
        version: seed(),
        node_types: vec![
            // The ambiguity is declared first, so only the pass split can put the cycle ahead
            // of it.
            child_type,
            declaring(left, "Left", ValueType::String),
            declaring(right, "Right", ValueType::Integer),
            self_looping,
        ],
        edge_types: Vec::new(),
    })
    .expect_err("the document carries a cycle and an unresolvable property set");

    assert_eq!(
        refused,
        OntologyError::CyclicParents { type_id: looper },
        "a property set resolved against a hierarchy that does not close cannot be the reason a \
         document is refused"
    );
}
