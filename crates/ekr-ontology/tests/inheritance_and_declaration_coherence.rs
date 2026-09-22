//! Adversarial cases for `story:ontology-types-and-values`.
//!
//! Every id here is parsed from a fixed canonical UUID rather than minted, because three of these
//! four cases are about an answer that depends on the order `TypeId` sorts in — and a minted
//! UUIDv7 makes that order an accident of when the test happened to run.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{NodeId, PropertyId, Timestamp, TypeId};
use ekr_ontology::{
    EdgeType, Lifecycle, NodeType, Ontology, OntologyDocument, OntologyError, OperationDefinition,
    PropertyDefinition, SchemaVersion, Value, ValueKind, ValueType,
};

/// A `TypeId` from a fixed canonical UUID, so that the sort order of a fixture is chosen and not
/// drawn.
fn type_id(tail: &str) -> TypeId {
    format!("018f2a00-0000-7000-8000-0000000000{tail}")
        .parse()
        .expect("a canonical uuid")
}

/// The one property id every hierarchy case redeclares.
fn property_id() -> PropertyId {
    "018f2a00-0000-7000-8000-0000000000e0"
        .parse()
        .expect("a canonical uuid")
}

/// The seed version every document here carries.
fn seed() -> SchemaVersion {
    SchemaVersion::seed(
        "018f2a00-0000-7000-8000-0000000000f0"
            .parse()
            .expect("a canonical uuid"),
        Timestamp::EPOCH,
    )
}

/// `grand` (declares the property as `String`) <- `parent` (redeclares it as `Integer`) <- `child`,
/// with the three ids given rather than minted.
fn three_level(grand: TypeId, parent: TypeId, child: TypeId) -> Ontology {
    let property = property_id();

    let mut grand_type = NodeType::new(grand, "Grand");
    grand_type.properties.insert(
        property,
        PropertyDefinition::new(property, "p", ValueType::String),
    );

    let mut parent_type = NodeType::new(parent, "Parent");
    parent_type.parents.insert(grand);
    parent_type.properties.insert(
        property,
        PropertyDefinition::new(property, "p", ValueType::Integer),
    );

    let mut child_type = NodeType::new(child, "Child");
    child_type.parents.insert(parent);

    Ontology::load(OntologyDocument {
        version: seed(),
        node_types: vec![grand_type, parent_type, child_type],
        edge_types: Vec::new(),
    })
    .expect("a three-level hierarchy loads")
}

/// A property redeclared by a nearer ancestor must win over the more distant one that first
/// declared it, and the answer must not depend on which `TypeId` happened to sort higher.
///
/// `Ontology::properties_of` walks `ancestors()` — a `BTreeSet<TypeId>` — and inserts each
/// ancestor's declarations in that set's order, so the *last id in sort order* wins among the
/// ancestors rather than the nearest one in the hierarchy. Two structurally identical ontologies
/// that differ only in the ids assigned to `grand` and `parent` therefore give opposite answers
/// about the same value.
#[test]
fn a_redeclared_property_is_resolved_by_the_hierarchy_and_not_by_type_id_order() {
    let property = property_id();
    let bag: BTreeMap<PropertyId, Vec<Value>> =
        [(property, vec![Value::Integer(1)])].into_iter().collect();
    let nodes: BTreeMap<NodeId, TypeId> = BTreeMap::new();

    // Arrangement A: the grandparent's id sorts above the parent's.
    let child_a = type_id("a0");
    let grand_high = three_level(type_id("c0"), type_id("b0"), child_a);
    let a = grand_high.check_node(child_a, &bag, &nodes);

    // Arrangement B: the same hierarchy, with the grandparent's id sorting below the parent's.
    let child_b = type_id("c1");
    let grand_low = three_level(type_id("a1"), type_id("b1"), child_b);
    let b = grand_low.check_node(child_b, &bag, &nodes);

    assert_eq!(
        a.as_ref().err(),
        b.as_ref().err(),
        "two hierarchies that differ only in which TypeId sorts higher must decide the same value \
         the same way; A gave {a:?} and B gave {b:?}"
    );
    assert_eq!(
        a.err(),
        None,
        "the nearer ancestor redeclared p as Integer, so an Integer satisfies it"
    );
}

/// A parent that is declared and whose own parent is not is an unknown type, not a cycle.
///
/// `check_declarations` checks a type's *direct* parents against the registry and then calls
/// `ancestors()`, which returns `None` both for a cycle and for an ancestor it cannot look up. A
/// grandparent that is not declared therefore surfaces as `CyclicParents` whenever the child's id
/// sorts before the parent's — the same malformed document reports two different errors depending
/// on the ids it happened to be written with.
#[test]
fn an_undeclared_grandparent_is_refused_as_an_unknown_type_and_not_as_a_cycle() {
    let child = type_id("a2");
    let parent = type_id("b2");
    let grand = type_id("c2");

    let mut child_type = NodeType::new(child, "Child");
    child_type.parents.insert(parent);
    let mut parent_type = NodeType::new(parent, "Parent");
    parent_type.parents.insert(grand);

    let refused = Ontology::load(OntologyDocument {
        version: seed(),
        node_types: vec![child_type, parent_type],
        edge_types: Vec::new(),
    })
    .expect_err("a grandparent the document does not declare is refused");

    assert_eq!(
        refused,
        OntologyError::UnknownType { type_id: grand },
        "the document declares no cycle; {grand} is simply absent"
    );
}

/// An operation argument declaring a type no value inhabits is refused at load.
///
/// `lib.rs` says an ontology is only ever held through `Ontology::load`, "so a declaration that
/// would leave the checker unable to answer — a `NodeRef` allowed to point at nothing, ... — is
/// refused before any value is checked against it". `check_properties` is called on a node type's
/// `properties` and an edge type's `properties` and on nothing else, so the same declaration
/// inside `OperationDefinition::arguments` (amendment 87) survives load intact.
#[test]
fn an_operation_argument_no_value_inhabits_is_refused_at_load() {
    let subject = type_id("a3");
    let mut subject_type = NodeType::new(subject, "Subject");
    subject_type.lifecycle = Some(Lifecycle {
        initial: "open".to_owned(),
        states: ["open".to_owned()].into_iter().collect(),
        transitions: BTreeSet::new(),
    });

    let mut attach = OperationDefinition::new("attach");
    attach.arguments.insert(
        "target".to_owned(),
        ValueType::NodeRef {
            allowed_types: BTreeSet::new(),
        },
    );
    subject_type.operations.insert("attach".to_owned(), attach);

    let refused = Ontology::load(OntologyDocument {
        version: seed(),
        node_types: vec![subject_type],
        edge_types: Vec::new(),
    })
    .expect_err("an argument allowed to point at nothing leaves the checker unable to answer");
    assert!(
        matches!(
            refused,
            OntologyError::EmptyValueType {
                kind: ValueKind::NodeRef,
                ..
            }
        ),
        "{refused:?}"
    );
}

/// No doc comment in this crate attributes a name to the ESS domain that the domain does not
/// declare.
///
/// This is the adversary's round-1 case, re-aimed. It was written as
/// `the_ess_domain_names_the_serde_content_key_value_rs_cites_it_for`, asserting that
/// `systems/ekr/domains/ontology.yaml` declares the `parameters` key `value.rs` attributed to it.
/// It does not. The activated domain now carries recursive parameters using its own typed
/// projection; that does not make this crate's tagged `parameters` key an ESS declaration.
#[test]
fn value_rs_does_not_attribute_its_serde_shape_to_the_ess_domain() {
    let declared = declared_names_of_the_domain();

    for carrier in [
        "value_type",
        "kind",
        "allowed_types",
        "variants",
        "element",
        "fields",
    ] {
        assert!(
            declared.contains(carrier),
            "the recursive ontology projection declares {carrier}: {declared:?}"
        );
    }
    assert!(
        !declared.contains("parameters"),
        "the domain declares no `parameters` key, and `value.rs` must not say it does"
    );

    // And the crate does not claim otherwise.
    let value_rs = read_source("value.rs");
    assert!(
        !value_rs.contains("is the naming `systems/ekr/domains/ontology.yaml` gives"),
        "value.rs still attributes its own serde shape to the ESS domain"
    );
    assert!(
        value_rs.contains("ValueTypeProjection")
            && value_rs.contains("PropertyDefinition.value_type"),
        "value.rs must name the recursive projection the domain actually carries"
    );
}

/// The class finding 4 is an instance of: **every name this crate attributes to an ESS domain is a
/// name that domain declares.**
///
/// The bound is stated rather than implied. This enumerates the names the crate's prose claims the
/// `ekr.ontology` domain declares, and asks the document. It does not parse English, so it cannot
/// catch a *new* false attribution written in a new sentence; what it catches is any of these
/// names disappearing from the document while the prose still cites it, which is the direction the
/// drift ran in round 1.
#[test]
fn every_domain_name_this_crate_cites_is_declared_by_the_domain() {
    let declared = declared_names_of_the_domain();
    let cited = [
        "value_type",
        "kind",
        "allowed_types",
        "variants",
        "element",
        "fields",
        "cardinality",
        "required",
        "constraints",
        "source_types",
        "target_types",
        "parents",
        "abstract_type",
        "inverse",
        "symmetric",
        "transitive",
        "arguments",
        "preconditions",
        "transition",
        "emits",
        "number",
        "parent",
        "created_at",
        "ekr.ontology.ValueKind",
        "ekr.ontology.ValueTypeProjection",
        "ekr.ontology.Cardinality",
        "ekr.ontology.Transition",
        "ekr.ontology.OperationDefinition",
        "ekr.ontology.SchemaVersion",
        "ekr.ontology.NodeType",
        "ekr.ontology.EdgeType",
        "ekr.ontology.PropertyDefinition",
    ];

    let absent: Vec<&str> = cited
        .into_iter()
        .filter(|name| !declared.contains(*name))
        .collect();
    assert!(
        absent.is_empty(),
        "this crate's doc comments cite these as declarations of \
         systems/ekr/domains/ontology.yaml, and the document declares no such name: {absent:?}"
    );
}

/// Every name `systems/ekr/domains/ontology.yaml` declares for a type, field or variant.
fn declared_names_of_the_domain() -> BTreeSet<String> {
    let path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("../../systems/ekr/domains/ontology.yaml");
    let text = std::fs::read_to_string(path).expect("the ESS domain is beside the crates");
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&text).expect("the ESS domain parses");
    let mut declared: BTreeSet<String> = BTreeSet::new();
    collect_declared_names(&document, &mut declared);
    declared
}

/// One of this crate's own source files, read as text.
fn read_source(file: &str) -> String {
    let path = format!(
        "{}/src/{file}",
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {path}: {e}"))
}

/// Every `name:` the document declares under a `fields:` or `variants:` list, at any depth.
fn collect_declared_names(node: &serde_yaml_ng::Value, out: &mut BTreeSet<String>) {
    match node {
        serde_yaml_ng::Value::Mapping(mapping) => {
            for (key, value) in mapping {
                if key.as_str() == Some("name") {
                    if let Some(name) = value.as_str() {
                        out.insert(name.to_owned());
                    }
                }
                if key.as_str() == Some("variants") {
                    if let Some(variants) = value.as_sequence() {
                        for variant in variants {
                            if let Some(name) = variant.as_str() {
                                out.insert(name.to_owned());
                            }
                        }
                    }
                }
                collect_declared_names(value, out);
            }
        }
        serde_yaml_ng::Value::Sequence(items) => {
            for item in items {
                collect_declared_names(item, out);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------------
// The classes the four findings are instances of.
//
// A finding arrives as one `file:line` and one reproduction. Answering exactly that leaves the
// rest of its class for the next pass, so each block below states the class as a rule and
// enumerates it.
// ---------------------------------------------------------------------------------------------

/// The six orderings of three fixed ids, so that "which id sorts first" is enumerated and not
/// drawn. Finding 1 and finding 2 were both *conditional* on an ordering; a case that fixes one
/// ordering can only catch half of either.
fn orderings() -> Vec<[TypeId; 3]> {
    let ids = [type_id("d0"), type_id("d1"), type_id("d2")];
    vec![
        [ids[0], ids[1], ids[2]],
        [ids[0], ids[2], ids[1]],
        [ids[1], ids[0], ids[2]],
        [ids[1], ids[2], ids[0]],
        [ids[2], ids[0], ids[1]],
        [ids[2], ids[1], ids[0]],
    ]
}

/// **The class of finding 1: no answer this crate gives about a type may depend on the order its
/// ids were minted in.**
///
/// The finding is one hierarchy and one pair of ids. The rule is quantified over every assignment
/// of ids to structural positions, so a fix that merely reversed the merge order — and would have
/// made the reported instance green — dies here on the ordering the report did not use.
#[test]
fn property_resolution_is_a_function_of_the_hierarchy_and_never_of_id_order() {
    let property = property_id();
    let bag: BTreeMap<PropertyId, Vec<Value>> =
        [(property, vec![Value::Integer(1)])].into_iter().collect();
    let nodes: BTreeMap<NodeId, TypeId> = BTreeMap::new();

    for [grand, parent, child] in orderings() {
        let ontology = three_level(grand, parent, child);

        assert_eq!(
            ontology
                .properties_of(child)
                .get(&property)
                .map(|found| &found.value_type),
            Some(&ValueType::Integer),
            "the nearer ancestor redeclared p as Integer; ids were grand={grand} parent={parent}"
        );
        assert_eq!(
            ontology.check_node(child, &bag, &nodes).err(),
            None,
            "an Integer satisfies the nearest declaration; ids were grand={grand} parent={parent}"
        );
        assert_eq!(
            ontology
                .properties_of(grand)
                .get(&property)
                .map(|found| &found.value_type),
            Some(&ValueType::String),
            "the grandparent keeps its own declaration"
        );
        assert_eq!(
            ontology
                .properties_of(parent)
                .get(&property)
                .map(|found| &found.value_type),
            Some(&ValueType::Integer),
            "the parent keeps its own declaration"
        );
    }
}

/// The same rule one level deeper: a redeclaration four levels up loses to every nearer one, and
/// a type's own declaration beats every ancestor's.
#[test]
fn the_nearest_declaration_wins_at_every_depth() {
    let property = property_id();
    let [a, b, c] = [type_id("e0"), type_id("e1"), type_id("e2")];
    let d = type_id("e3");

    let declaring = |id: TypeId, name: &str, parent: Option<TypeId>, value_type: ValueType| {
        let mut declared = NodeType::new(id, name);
        if let Some(parent) = parent {
            declared.parents.insert(parent);
        }
        declared
            .properties
            .insert(property, PropertyDefinition::new(property, "p", value_type));
        declared
    };

    // a(String) <- b(Integer) <- c(Boolean) <- d(no declaration of its own)
    let mut leaf = NodeType::new(d, "D");
    leaf.parents.insert(c);
    let ontology = Ontology::load(OntologyDocument {
        version: seed(),
        node_types: vec![
            declaring(a, "A", None, ValueType::String),
            declaring(b, "B", Some(a), ValueType::Integer),
            declaring(c, "C", Some(b), ValueType::Boolean),
            leaf,
        ],
        edge_types: Vec::new(),
    })
    .expect("a four-level hierarchy loads");

    for (type_id, expected) in [
        (a, ValueType::String),
        (b, ValueType::Integer),
        (c, ValueType::Boolean),
        (d, ValueType::Boolean),
    ] {
        assert_eq!(
            ontology
                .properties_of(type_id)
                .get(&property)
                .map(|found| &found.value_type),
            Some(&expected),
            "{type_id} resolves p to its nearest declaration"
        );
    }
}

/// The one case "nearest wins" cannot decide: two ancestors the same distance away declaring one
/// property differently. There is no nearer one, so picking either is picking by id order again —
/// the document is ambiguous and is refused rather than resolved.
#[test]
fn two_ancestors_at_one_distance_declaring_a_property_differently_are_refused_at_load() {
    let property = property_id();

    for [left, right, child] in orderings() {
        let mut left_type = NodeType::new(left, "Left");
        left_type.properties.insert(
            property,
            PropertyDefinition::new(property, "p", ValueType::String),
        );
        let mut right_type = NodeType::new(right, "Right");
        right_type.properties.insert(
            property,
            PropertyDefinition::new(property, "p", ValueType::Integer),
        );
        let mut child_type = NodeType::new(child, "Child");
        child_type.parents.insert(left);
        child_type.parents.insert(right);

        let refused = Ontology::load(OntologyDocument {
            version: seed(),
            node_types: vec![left_type, right_type, child_type],
            edge_types: Vec::new(),
        })
        .expect_err("two parents declare p differently and neither is nearer");
        assert_eq!(
            refused,
            OntologyError::AmbiguousProperty {
                type_id: child,
                property,
            },
            "the refusal names the type and the property it cannot resolve, whatever the ids \
             sort like"
        );
        assert!(
            refused.to_string().contains(&property.to_string())
                && refused.to_string().contains(&child.to_string()),
            "the refusal reaches a reader: {refused}"
        );
    }
}

/// And the boundary of that refusal: two ancestors that declare the *same* thing are not
/// ambiguous, because there is nothing to choose between. Refusing this would make multiple
/// inheritance unusable rather than safe.
///
/// Note what this fixture is and is not. It is two *distinct* unrelated types declaring equal
/// definitions — not one type reached by two paths, which the ancestor closure holds once and
/// which never presents a choice at all. A comment on the old resolver described the second and
/// sat above code that only ever saw the first; pass 2 of review caught it, and the wording here
/// is the same correction on the case that exercises it.
#[test]
fn two_ancestors_declaring_one_property_identically_are_not_ambiguous() {
    let property = property_id();
    let [left, right, child] = [type_id("f0"), type_id("f1"), type_id("f2")];

    let declaring = |id: TypeId, name: &str| {
        let mut declared = NodeType::new(id, name);
        declared.properties.insert(
            property,
            PropertyDefinition::new(property, "p", ValueType::String),
        );
        declared
    };
    let mut child_type = NodeType::new(child, "Child");
    child_type.parents.insert(left);
    child_type.parents.insert(right);

    let ontology = Ontology::load(OntologyDocument {
        version: seed(),
        node_types: vec![
            declaring(left, "Left"),
            declaring(right, "Right"),
            child_type,
        ],
        edge_types: Vec::new(),
    })
    .expect("two unrelated types saying the same thing leave nothing to choose between");

    assert_eq!(
        ontology
            .properties_of(child)
            .get(&property)
            .map(|found| &found.value_type),
        Some(&ValueType::String)
    );
}

/// **The class of round-1 finding 2, as far as this case actually proves it: a document carrying
/// exactly one hierarchy fault is refused for that fault, whatever order its ids sort in.**
///
/// The instance was an undeclared grandparent reported as a cycle "whenever the child's id sorts
/// before the parent's" — a *false* reason. The rule is quantified over all six orderings, so the
/// half the report did not exercise cannot stay wrong.
///
/// The name says "one fault" because that is what the documents below carry, and an earlier
/// version of this case was named for the general rule while enumerating only single-fault
/// documents — asserting in the place a later reader looks a rule the crate did not hold. Pass 2
/// of review caught that, and `hierarchy_specificity.rs` carries the two-fault case.
///
/// Where the general rule now stands: across types, `Ontology::load` reports faults in the order
/// the *document* declares its types, so two readers of one document are sent after the same
/// defect. Within one type it is not general — two faulty properties on one type are still
/// reported in `PropertyId` order — and reporting every fault rather than the first is a change to
/// the checker's return contract, which is design § 20's question and not this story's.
#[test]
fn a_single_fault_hierarchy_is_refused_for_that_fault_whatever_the_id_order() {
    for [child, parent, grand] in orderings() {
        let mut child_type = NodeType::new(child, "Child");
        child_type.parents.insert(parent);
        let mut parent_type = NodeType::new(parent, "Parent");
        parent_type.parents.insert(grand);

        let refused = Ontology::load(OntologyDocument {
            version: seed(),
            node_types: vec![child_type, parent_type],
            edge_types: Vec::new(),
        })
        .expect_err("the grandparent is not declared");
        assert_eq!(
            refused,
            OntologyError::UnknownType { type_id: grand },
            "an absent ancestor is absent, not a cycle; ids were child={child} parent={parent}"
        );
    }

    // The other half of the distinction: a document that really does declare a cycle still says
    // so, at every depth and in every ordering.
    for [a, b, c] in orderings() {
        let mut first = NodeType::new(a, "A");
        first.parents.insert(b);
        let mut second = NodeType::new(b, "B");
        second.parents.insert(c);
        let mut third = NodeType::new(c, "C");
        third.parents.insert(a);

        let refused = Ontology::load(OntologyDocument {
            version: seed(),
            node_types: vec![first, second, third],
            edge_types: Vec::new(),
        })
        .expect_err("a three-type cycle is a cycle");
        assert!(
            matches!(refused, OntologyError::CyclicParents { .. }),
            "a declared cycle is reported as one: {refused:?}"
        );
    }
}

/// **The class of finding 3: every position in a document where a `ValueType` can be declared is
/// walked for inhabitability, at every depth.**
///
/// The instance was `OperationDefinition::arguments`. The positions are enumerable from the type
/// graph of `OntologyDocument`, and there are exactly three; all three are here, each at the top
/// of a declaration and each nested inside a compound.
#[test]
fn every_position_a_value_type_is_declared_in_is_walked_for_inhabitability() {
    let subject = type_id("b8");
    let other = type_id("b9");
    let edge = type_id("ba");
    let property = property_id();

    let uninhabitable = [
        ValueType::NodeRef {
            allowed_types: BTreeSet::new(),
        },
        ValueType::Enum {
            variants: BTreeSet::new(),
        },
        ValueType::List(Box::new(ValueType::NodeRef {
            allowed_types: BTreeSet::new(),
        })),
        ValueType::Record(
            [(
                "f".to_owned(),
                ValueType::Enum {
                    variants: BTreeSet::new(),
                },
            )]
            .into_iter()
            .collect(),
        ),
    ];

    for declared_type in uninhabitable {
        // Position 1 of 3: a node type's property.
        let mut node_property = NodeType::new(subject, "Subject");
        node_property.properties.insert(
            property,
            PropertyDefinition::new(property, "p", declared_type.clone()),
        );
        assert!(
            Ontology::load(OntologyDocument {
                version: seed(),
                node_types: vec![node_property, NodeType::new(other, "Other")],
                edge_types: Vec::new(),
            })
            .is_err(),
            "a node type's property declaring {declared_type:?} must be refused"
        );

        // Position 2 of 3: an edge type's property.
        let mut edge_type = EdgeType::new(edge, "e");
        edge_type.source_types.insert(subject);
        edge_type.target_types.insert(subject);
        edge_type.properties.insert(
            property,
            PropertyDefinition::new(property, "p", declared_type.clone()),
        );
        assert!(
            Ontology::load(OntologyDocument {
                version: seed(),
                node_types: vec![NodeType::new(subject, "Subject")],
                edge_types: vec![edge_type],
            })
            .is_err(),
            "an edge type's property declaring {declared_type:?} must be refused"
        );

        // Position 3 of 3: an operation's argument (amendment 87).
        let mut operation = OperationDefinition::new("attach");
        operation
            .arguments
            .insert("target".to_owned(), declared_type.clone());
        let mut with_operation = NodeType::new(subject, "Subject");
        with_operation
            .operations
            .insert("attach".to_owned(), operation);
        assert!(
            Ontology::load(OntologyDocument {
                version: seed(),
                node_types: vec![with_operation],
                edge_types: Vec::new(),
            })
            .is_err(),
            "an operation argument declaring {declared_type:?} must be refused"
        );
    }
}

/// An operation argument naming a type the document does not declare is refused for the same
/// reason a property's `allowed_types` is: the walk is one walk, so it carries every refusal.
#[test]
fn an_operation_argument_naming_an_undeclared_type_is_refused_at_load() {
    let subject = type_id("bb");
    let undeclared = type_id("bc");

    let mut operation = OperationDefinition::new("attach");
    operation.arguments.insert(
        "target".to_owned(),
        ValueType::NodeRef {
            allowed_types: [undeclared].into_iter().collect(),
        },
    );
    let mut subject_type = NodeType::new(subject, "Subject");
    subject_type
        .operations
        .insert("attach".to_owned(), operation);

    assert_eq!(
        Ontology::load(OntologyDocument {
            version: seed(),
            node_types: vec![subject_type],
            edge_types: Vec::new(),
        })
        .err(),
        Some(OntologyError::UnknownType {
            type_id: undeclared
        })
    );
}

/// The machine-checkable half of finding 3's class: the three positions are three because the
/// crate declares exactly two `ValueType`-typed fields, and a hand-kept enumeration of positions
/// is the defect that produced the finding in the first place.
///
/// What it catches: a *new* field of type `ValueType`, or holding one, added anywhere in the crate
/// without the inhabitability walk being extended to it. What it does not catch: a new container
/// of `PropertyDefinition` or `OperationDefinition` — that is stated rather than implied, and it
/// is the remaining hand-kept part.
#[test]
fn the_crate_declares_no_value_type_field_the_walk_does_not_reach() {
    // `value.rs` is excluded: it defines `ValueType` itself, and its compound variants are what
    // `reachable` recurses through rather than separate declaration sites.
    let mut found: BTreeSet<String> = BTreeSet::new();
    for file in [
        "types.rs",
        "lifecycle.rs",
        "schema.rs",
        "check.rs",
        "lib.rs",
    ] {
        for line in read_source(file).lines() {
            let line = line.trim();
            if line.starts_with("///") || line.starts_with("//") {
                continue;
            }
            let Some(field) = line.strip_prefix("pub ") else {
                continue;
            };
            let Some((name, declared)) = field.split_once(": ") else {
                continue;
            };
            // A field, not a function: `pub fn new(id: PropertyId, ...)` splits on the same ": "
            // and named itself "fn new(id" the first time this ran.
            if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                continue;
            }
            if declared.contains("ValueType") {
                found.insert(format!("{file}::{name}"));
            }
        }
    }

    let walked: BTreeSet<String> = ["types.rs::value_type", "lifecycle.rs::arguments"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        found, walked,
        "every field holding a ValueType must be reached by the inhabitability walk in \
         Ontology::load. A field here that is not in the walked set is a declaration that loads \
         uninhabitable; extend check_value_type's callers and add it to this set."
    );
}

#[test]
fn the_public_node_type_index_distinguishes_present_and_absent_nodes() {
    let present = NodeId::mint();
    let absent = NodeId::mint();
    let kind = type_id("01");
    let nodes = BTreeMap::from([(present, kind)]);
    let index: &dyn ekr_ontology::NodeTypes = &nodes;
    assert_eq!(index.type_of(present), Some(kind));
    assert_eq!(index.type_of(absent), None);
}
