//! What this crate says about `systems/ekr/domains/graph.yaml`, checked against the document.
//!
//! `ekr-ontology` has a case of this shape, and it exists because two review passes each found a
//! different false sentence in the gap between the ESS document and the crate that claims to
//! implement it: `ess specify validate` checks the document against itself and the rest of the
//! suite checks the crate against itself, so nothing reads the two together. `ekr-graph` projects
//! more of a domain than any crate before it, so it gets the same case.
//!
//! `ekr-graph` declares no YAML parser — its dependencies are `ekr-core`, `ekr-ontology`, `serde`
//! and `thiserror`, fixed by `story:workspace-crate-skeleton` — so the document is read as text.
//! The scan is held to its own catch: a parse that stops finding declarations fails loudly rather
//! than passing vacuously.

/// `systems/ekr/domains/graph.yaml`, as text.
fn domain_text() -> String {
    let path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("../../systems/ekr/domains/graph.yaml");
    std::fs::read_to_string(path).expect("the ESS domain is beside the crates")
}

/// The variants one `kind: enum` type of the domain declares, in document order.
fn variants_of(type_name: &str) -> Vec<String> {
    let text = domain_text();
    let mut lines = text
        .lines()
        .skip_while(|line| line.trim_start() != format!("- name: {type_name}"));
    assert!(
        lines.next().is_some(),
        "the domain declares no type named {type_name}"
    );

    let mut variants = Vec::new();
    let mut inside = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("- name: ") {
            break;
        }
        if trimmed == "variants:" {
            inside = true;
            continue;
        }
        if inside {
            if let Some(variant) = trimmed.strip_prefix("- ") {
                variants.push(variant.to_owned());
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                break;
            }
        }
    }
    assert!(
        !variants.is_empty(),
        "the variant scan is broken, not the domain: {type_name}"
    );
    variants
}

/// Every declaration of the domain that carries a `fields:` block, by name, in document order.
///
/// Derived rather than listed. The first version of the entity guard below carried a hand-written
/// list of five, and the two it left out — `ekr.graph.Assertion` and `ekr.graph.Evidence` — were
/// exactly the two whose projection diverges, so the case was titled for the whole domain and
/// checked the easy half of it. A list a person maintains is the defect; the omission is only its
/// symptom.
fn declarations_with_fields() -> Vec<String> {
    let text = domain_text();
    let mut found = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        // Only a qualified name heads a declaration; `- name: kind` inside a `fields:` block is a
        // field, and reading it as a declaration is how the scan would silently drift.
        if let Some(name) = trimmed.strip_prefix("- name: ekr.") {
            current = Some(format!("ekr.{name}"));
        } else if trimmed == "fields:" {
            if let Some(name) = current.take() {
                found.push(name);
            }
        }
    }
    assert!(
        found.len() >= 8,
        "the declaration scan is broken, not the domain: {found:?}"
    );
    found
}

/// The field names one declaration of the domain carries, in document order.
fn fields_of(name: &str) -> Vec<String> {
    let text = domain_text();
    let mut lines = text
        .lines()
        .skip_while(|line| line.trim_start() != format!("- name: {name}"));
    assert!(lines.next().is_some(), "the domain declares no {name}");

    let mut fields = Vec::new();
    let mut inside = false;
    for line in lines {
        let trimmed = line.trim();
        // The next declaration ends this one. Checked before the field pattern, because a
        // declaration head *is* a `- name:` line and reading it as a field is how the scan drifts:
        // `ekr.graph.TypedValue` sits under `types:` and is followed straight by `entities:`, with
        // no `relations:` or `lifecycle:` in between to stop at.
        if trimmed.starts_with("- name: ekr.")
            || matches!(
                trimmed,
                "relations:" | "lifecycle:" | "invariants:" | "views:" | "naming:" | "entities:"
            )
        {
            if inside {
                break;
            }
            continue;
        }
        if trimmed == "fields:" {
            inside = true;
            continue;
        }
        if inside {
            if let Some(field) = trimmed.strip_prefix("- name: ") {
                fields.push(field.to_owned());
            }
        }
    }
    assert!(!fields.is_empty(), "the field scan is broken: {name}");
    fields
}

/// Every enumeration the domain declares, and the Rust type that carries it.
///
/// Transcribed here rather than derived, for the reason `crates/ekr/tests/story_contract.rs`
/// transcribes the story's dependency tables: a case whose expectation is read from the thing it
/// is checking asserts nothing. Adding a variant to `graph.yaml` without adding it here, or here
/// without the crate, turns this red.
const ENUMERATIONS: &[(&str, &str)] = &[
    ("ekr.graph.Space", "Space"),
    ("ekr.graph.AssessmentKind", "Assessment"),
    ("ekr.graph.AssertionLifecycleKind", "AssertionLifecycle"),
    ("ekr.graph.CanonicalValueKind", "CanonicalValue"),
    ("ekr.graph.SubjectKind", "Subject"),
    ("ekr.graph.PredicateKind", "Predicate"),
    ("ekr.graph.ObjectKind", "Object"),
    ("ekr.graph.EvidenceKind", "EvidenceKind"),
];

#[test]
fn every_enumeration_the_domain_declares_is_carried_variant_for_variant() {
    for &(declared, rust_type) in ENUMERATIONS {
        let source = type_region(rust_type);
        for variant in variants_of(declared) {
            assert!(
                source.lines().any(|line| {
                    let line = line.trim();
                    line == format!("{variant},")
                        || line.starts_with(&format!("{variant}("))
                        || line.starts_with(&format!("{variant} {{"))
                }),
                "{declared} declares the variant {variant}, and {rust_type} does not name it"
            );
        }
    }

    // `ObservationKind` is the one whose variants the crate spells twice — once as the flat kind
    // the domain declares and once as the content form design § 15 gives — so it is checked apart
    // from the list above rather than by a substring that would match either.
    let source = type_region("ObservationKind");
    for variant in variants_of("ekr.graph.ObservationKind") {
        assert!(
            source.contains(&format!("{variant},")) || source.contains(&format!("{variant} {{")),
            "ekr.graph.ObservationKind declares {variant} and ObservationKind does not"
        );
    }
}

/// The old quotation test required missing assessment payloads. The current contract
/// carries the complete assessment separately from lifecycle; exercise those real carriers.
#[test]
fn assessment_and_lifecycle_retain_independent_payloads() {
    use ekr_core::{
        AgentId, AssertionId, Canonical, GraphRootId, IssueId, RevisionNumber, Timestamp, TypeId,
    };
    use ekr_graph::{
        Assertion, AssertionLifecycle, Assessment, Object, Predicate, RetractionReason, Subject,
        TemporalRange, TransactionTime,
    };
    use std::collections::BTreeSet;

    assert_eq!(
        fields_of("ekr.graph.AssessmentProjection"),
        [
            "kind",
            "completed",
            "required",
            "validators",
            "issues",
            "competing_assertions"
        ]
    );
    assert_eq!(
        fields_of("ekr.graph.AssertionLifecycleProjection"),
        ["kind", "at_revision", "reason", "by", "effective_from"]
    );
    let assessment = [
        Assessment::Proposed,
        Assessment::Validating {
            completed: 2,
            required: 7,
        },
        Assessment::Accepted {
            validators: BTreeSet::from([AgentId::mint()]),
        },
        Assessment::Rejected {
            issues: vec![IssueId::mint()],
        },
        Assessment::Disputed {
            competing_assertions: vec![AssertionId::mint()],
        },
    ];
    assert_eq!(
        assessment.iter().map(Assessment::name).collect::<Vec<_>>(),
        variants_of("ekr.graph.AssessmentKind")
    );
    let lifecycle = [
        AssertionLifecycle::Active,
        AssertionLifecycle::Retracted {
            at_revision: RevisionNumber::new(2),
            reason: RetractionReason::new("supplied evidence withdrawn"),
        },
        AssertionLifecycle::Superseded {
            by: AssertionId::mint(),
            at_revision: RevisionNumber::new(3),
            effective_from: Timestamp::from_millis(100),
        },
    ];
    assert_eq!(
        lifecycle
            .iter()
            .map(AssertionLifecycle::name)
            .collect::<Vec<_>>(),
        variants_of("ekr.graph.AssertionLifecycleKind")
    );
    let mut assertion: Assertion = Assertion {
        id: AssertionId::mint(),
        root_id: GraphRootId::mint(),
        subject: Subject::Type(TypeId::mint()),
        predicate: Predicate::Relation(TypeId::mint()),
        object: Object::Type(TypeId::mint()),
        evidence: BTreeSet::new(),
        proposed_by: AgentId::mint(),
        assessment: assessment[2].clone(),
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::new(None, None).unwrap(),
        transaction_time: TransactionTime::new(Timestamp::EPOCH, None).unwrap(),
    };
    // This exercises the actual Assertion encoder under every withdrawal state.
    // Changing retained attribution must change its address even after withdrawal.
    for withdrawal in &lifecycle {
        assertion.lifecycle = withdrawal.clone();
        assertion.assessment = assessment[2].clone();
        let with_original_validator = assertion.canonical_bytes();
        assertion.assessment = Assessment::Accepted {
            validators: BTreeSet::from([AgentId::mint()]),
        };
        assert_ne!(
            assertion.canonical_bytes(),
            with_original_validator,
            "the assertion address lost retained attribution under {}",
            withdrawal.name()
        );
    }
    let different_reason = AssertionLifecycle::Retracted {
        at_revision: RevisionNumber::new(2),
        reason: RetractionReason::new("different reason"),
    };
    assert_ne!(
        different_reason.canonical_bytes(),
        lifecycle[1].canonical_bytes()
    );
}

/// Name lookups are scoped to the actual current type; a field on another type cannot cover it.
#[test]
fn current_type_regions_cannot_borrow_legacy_or_unrelated_fields() {
    let node = type_region("Node");
    assert!(node.contains("pub canonical_name:"));
    assert!(!node.contains("pub parent:"));
    assert!(type_region("GraphRoot").contains("pub parent:"));
    assert!(!crate_modules().iter().any(|(name, _)| name == "legacy.rs"));
    assert!(!type_region("Assessment").contains("Retracted {"));
}

/// Current `.rs` modules in this crate's `src/`, excluding the frozen legacy codec.
fn crate_modules() -> Vec<(String, String)> {
    let directory = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("src");
    let mut found: Vec<(String, String)> = std::fs::read_dir(directory)
        .expect("the crate has a src/")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.extension().is_some_and(|e| e == "rs")
                && path.file_name().is_some_and(|name| name != "legacy.rs")
        })
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
    assert!(found.len() >= 9, "the module scan is broken: {found:?}");
    found
}

/// The source text belonging to one Rust type: its `struct`/`enum` declaration and every `impl`
/// block on it, and nothing else.
///
/// Searching concatenated modules let `GraphRoot.parent` and `Root.parent` answer for a
/// hypothetical `ekr.graph.Node.parent`. The guard now looks only inside the declared carrier.
///
/// Braces are matched rather than searched for, because a nested type in a field position (
/// `properties: BTreeMap<PropertyId, Value>`) and a nested block in a method body both put a `}`
/// before the one that closes the item.
fn type_region(type_name: &str) -> String {
    let mut region = String::new();

    let (type_name, modules) = if let Some(name) = type_name.strip_prefix("store::") {
        let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("../ekr-store/src/snapshot.rs");
        (
            name,
            vec![("snapshot.rs".into(), std::fs::read_to_string(path).unwrap())],
        )
    } else {
        (type_name, crate_modules())
    };
    for (_, text) in modules {
        let mut at = 0;
        for line in text.lines() {
            if opens_item(line, type_name) {
                let open = at + line.find('{').expect("an item head opens a block");
                region.push_str(&block_at(&text, open));
            }
            at += line.len() + 1;
        }
    }

    assert!(
        !region.is_empty(),
        "no declaration or impl block found for `{type_name}`; PROJECTIONS names a type this \
         crate does not have"
    );
    region
}

/// Whether `line` opens the declaration of `type_name` or an implementation on it.
///
/// A match on the head rather than on three literal strings, because
/// `architecture-decision-record:0005-float-is-not-canonical`, as amended, made `Node`, `Edge`,
/// `Object` and `Assertion` generic over the value they carry: their heads read
/// `pub struct Node<V = CanonicalValue> {` and `impl<V> Node<V> {`, and the literal form matched
/// neither — the scanner found no region at all for `Node` and said so, which is why this arrived
/// as a red case rather than as a guard quietly covering nothing.
///
/// Trait implementations are resolved through their `for` type. This binds a
/// canonical-byte projection to that carrier's encoder, without reading a
/// similarly named method on another type. Private store envelope declarations
/// are read only from the explicitly named current store codec.
///
/// **Stated bounds**, neither reachable in this crate today and both written down rather than
/// worked around:
///
/// * the head has to be on one line, which `cargo fmt` makes true for every item here;
/// * an item head carrying a `where` clause is missed, because what follows the name is then
///   `where …` rather than `{`. The miss is quiet — `type_region` asserts only that the region it
///   built is non-empty, so a *second* item of a type that already has one would vanish from the
///   region without a word.
fn opens_item(line: &str, type_name: &str) -> bool {
    let line = line.trim();
    let rest = if let Some(rest) = line.strip_prefix("pub struct ") {
        rest
    } else if let Some(rest) = line.strip_prefix("struct ") {
        rest
    } else if let Some(rest) = line.strip_prefix("enum ") {
        rest
    } else if let Some(rest) = line.strip_prefix("pub enum ") {
        rest
    } else if let Some(rest) = line.strip_prefix("impl") {
        let rest = after_generics(rest).trim_start();
        rest.split_once(" for ").map_or(rest, |(_, name)| name)
    } else {
        return false;
    };

    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name != type_name {
        return false;
    }
    after_generics(&rest[name.len()..])
        .trim_start()
        .starts_with('{')
}

/// The text after a balanced `<…>` at the start of `text`, or `text` unchanged when it has none.
fn after_generics(text: &str) -> &str {
    if !text.starts_with('<') {
        return text;
    }
    let mut depth = 0usize;
    for (at, character) in text.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return &text[at + character.len_utf8()..];
                }
            }
            _ => {}
        }
    }
    text
}

/// The text from the `{` at `open` through its matching `}`.
fn block_at(text: &str, open: usize) -> String {
    let bytes = text.as_bytes();
    assert_eq!(bytes[open], b'{', "block_at was not handed a brace");
    let mut depth = 0usize;
    for (offset, byte) in bytes[open..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return text[open..=open + offset].to_owned();
                }
            }
            _ => {}
        }
    }
    panic!("an item's braces do not balance");
}

/// Current typed carriers for every declaration with fields. These are semantic
/// carrier checks, not proof that the CLI's ESS transport projections have executed.
/// The graph envelope belongs to the store codec; inspect that precise current owner.
const PROJECTIONS: &[(&str, &[&str])] = &[
    ("ekr.graph.AssessmentProjection", &["Assessment"]),
    (
        "ekr.graph.AssertionLifecycleProjection",
        &["AssertionLifecycle"],
    ),
    ("ekr.graph.TypedValue", &["CanonicalValue"]),
    ("ekr.graph.RevisionRoot", &["Root"]),
    ("ekr.graph.GraphRootRecord", &["GraphRoot"]),
    ("ekr.graph.NodeRecord", &["Node"]),
    ("ekr.graph.EdgeRecord", &["Edge"]),
    ("ekr.graph.SubjectProjection", &["Subject"]),
    ("ekr.graph.PredicateProjection", &["Predicate"]),
    ("ekr.graph.ObjectProjection", &["Object"]),
    ("ekr.graph.TemporalRange", &["TemporalRange"]),
    ("ekr.graph.TransactionTime", &["TransactionTime"]),
    ("ekr.graph.AssertionRecord", &["Assertion"]),
    ("ekr.graph.EvidenceSourceProjection", &["EvidenceSource"]),
    ("ekr.graph.EvidenceRecord", &["Evidence"]),
    ("ekr.graph.GraphDocumentBodyProjection", &["CanonicalGraph"]),
    (
        "ekr.graph.GraphDocumentV2Projection",
        &["store::GraphEnvelope"],
    ),
    ("ekr.graph.GraphRoot", &["GraphRoot"]),
    ("ekr.graph.Node", &["Node"]),
    ("ekr.graph.Edge", &["Edge"]),
    (
        "ekr.graph.Assertion",
        &["Assertion", "TemporalRange", "TransactionTime"],
    ),
    ("ekr.graph.Support", &["Support"]),
    (
        "ekr.graph.Evidence",
        &["Evidence", "EvidenceSource", "Confidence"],
    ),
    (
        "ekr.graph.Observation",
        &["Observation", "ObservationContent"],
    ),
    ("ekr.graph.Assertions", &["Assertion", "TransactionTime"]),
];

/// Renamed/derived fields bind explicit tokens in their actual carrier instead of
/// exempting a field from verification. Enum kinds are also checked against each
/// declared arm above. No entry can match a field on an unrelated type.
const FUSIONS: &[(&str, &str, &str, &[&str])] = &[
    (
        "ekr.graph.AssessmentProjection",
        "kind",
        "Assessment",
        &["fn name("],
    ),
    (
        "ekr.graph.AssertionLifecycleProjection",
        "kind",
        "AssertionLifecycle",
        &["fn name("],
    ),
    (
        "ekr.graph.TypedValue",
        "kind",
        "CanonicalValue",
        &["String(String)", "Record(BTreeMap"],
    ),
    (
        "ekr.graph.TypedValue",
        "canonical_bytes",
        "CanonicalValue",
        &["fn encode("],
    ),
    (
        "ekr.graph.SubjectProjection",
        "kind",
        "Subject",
        &["Node(R)", "Edge(E)", "Type(TypeId)"],
    ),
    (
        "ekr.graph.SubjectProjection",
        "id",
        "Subject",
        &["Node(R)", "Edge(E)", "Type(TypeId)"],
    ),
    (
        "ekr.graph.PredicateProjection",
        "kind",
        "Predicate",
        &["Property(PropertyId)", "Relation(TypeId)"],
    ),
    (
        "ekr.graph.PredicateProjection",
        "id",
        "Predicate",
        &["Property(PropertyId)", "Relation(TypeId)"],
    ),
    (
        "ekr.graph.ObjectProjection",
        "kind",
        "Object",
        &["Value(V)", "Node(R)", "Type(TypeId)"],
    ),
    (
        "ekr.graph.ObjectProjection",
        "value",
        "Object",
        &["Value(V)"],
    ),
    (
        "ekr.graph.ObjectProjection",
        "reference",
        "Object",
        &["Node(R)", "Type(TypeId)"],
    ),
    (
        "ekr.graph.EvidenceSourceProjection",
        "kind",
        "EvidenceSource",
        &["fn kind("],
    ),
    (
        "ekr.graph.EvidenceSourceProjection",
        "url",
        "EvidenceSource",
        &["Url(String)"],
    ),
    (
        "ekr.graph.EvidenceSourceProjection",
        "assertion",
        "EvidenceSource",
        &["GraphAssertion(AssertionId)"],
    ),
    (
        "ekr.graph.EvidenceSourceProjection",
        "observation",
        "EvidenceSource",
        &["Observation(ObservationId)"],
    ),
    (
        "ekr.graph.EvidenceRecord",
        "confidence_bp",
        "Confidence",
        &["fn basis_points("],
    ),
    (
        "ekr.graph.Assertion",
        "subject_kind",
        "Subject",
        &["Node(R)", "Edge(E)", "Type(TypeId)"],
    ),
    (
        "ekr.graph.Assertion",
        "predicate_kind",
        "Predicate",
        &["Property(PropertyId)", "Relation(TypeId)"],
    ),
    (
        "ekr.graph.Assertion",
        "object_kind",
        "Object",
        &["Value(V)", "Node(R)", "Type(TypeId)"],
    ),
    (
        "ekr.graph.Assertion",
        "object_value",
        "Object",
        &["Value(V)"],
    ),
    (
        "ekr.graph.Assertion",
        "object_ref",
        "Object",
        &["Node(R)", "Type(TypeId)"],
    ),
    (
        "ekr.graph.Assertion",
        "valid_from",
        "TemporalRange",
        &["pub from: Option<Timestamp>"],
    ),
    (
        "ekr.graph.Assertion",
        "valid_to",
        "TemporalRange",
        &["pub to: Option<Timestamp>"],
    ),
    (
        "ekr.graph.Evidence",
        "confidence_bp",
        "Confidence",
        &["fn basis_points("],
    ),
    (
        "ekr.graph.Evidence",
        "observation_id",
        "EvidenceSource",
        &["fn observation("],
    ),
    (
        "ekr.graph.Assertions",
        "assertion_id",
        "Assertion",
        &["pub id: AssertionId"],
    ),
];

#[test]
fn every_declaration_of_the_domain_is_carried_field_for_field() {
    let declarations = declarations_with_fields();
    let expected: std::collections::BTreeSet<_> =
        PROJECTIONS.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        declarations
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        expected,
        "every declaration needs a current, explicit carrier"
    );
    for (declaration, types) in PROJECTIONS {
        let region: String = types.iter().map(|name| type_region(name)).collect();
        for field in fields_of(declaration) {
            if let Some((_, _, owner, tokens)) = FUSIONS
                .iter()
                .find(|(declared, name, _, _)| declared == declaration && *name == field)
            {
                let carrier = type_region(owner);
                assert!(!tokens.is_empty());
                for token in *tokens {
                    assert!(
                        carrier.contains(token),
                        "{declaration}.{field} lost {owner}'s {token}"
                    );
                }
            } else {
                assert!(
                    region.lines().any(|line| {
                        let line = line.trim();
                        line.strip_prefix("pub ")
                            .unwrap_or(line)
                            .starts_with(&format!("{field}:"))
                    }) || region.contains(&format!("fn {field}(")),
                    "{declaration}.{field} has no field/method on {types:?}"
                );
            }
        }
    }
    for (owner, field, _, _) in FUSIONS {
        assert!(declarations.iter().any(|declared| declared == owner));
        assert!(
            fields_of(owner).iter().any(|declared| declared == field),
            "stale renamed-field mapping {owner}.{field}"
        );
    }
}
