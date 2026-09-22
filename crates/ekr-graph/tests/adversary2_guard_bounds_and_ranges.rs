//! Adversarial cases, wave p1-04 **pass 2**.
//!
//! Pass 1 attacked the types. This pass attacks the guards correction 1 built over them, and the
//! new surface it added. A guard is a claim about a whole class, and a claim about a class is
//! exactly the kind of thing that can be true of the examples in front of it and false in general.
//!
//! **Re-aimed by correction round 2**, which is where the findings were answered:
//!
//! | finding | what the case holds now |
//! |---|---|
//! | the field guard matches any type's field | it matches inside the projecting type, and still catches a colliding name |
//! | the open-ended-read ban reads one module | it reads every module, and `TemporalRange::is_open` is gone |
//! | an inverted range is representable | both constructors refuse it, and the empty interval is still admitted |
//!
//! Each keeps its original name. The mutation probes stay: a guard that is asserted to work rather
//! than shown to work is the thing this file exists to disbelieve.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, EvidenceId, GraphRootId, NodeId, RevisionNumber, SchemaVersionId,
    Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, CanonicalGraph, CanonicalRef, GraphRoot, GraphSnapshot, Object, Predicate, Space,
    Subject, TemporalRange, TransactionTime, ValidationState,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};

/// 2024-01-01T00:00:00Z.
const TENURE_BEGAN: Timestamp = Timestamp::from_millis(1_704_067_200_000);
/// 2026-03-12T00:00:00Z.
const HANDOVER: Timestamp = Timestamp::from_millis(1_773_273_600_000);

// ---------------------------------------------------------------------------------------------
// The extended projection guard, and the bound its own doc comment claims.
// ---------------------------------------------------------------------------------------------

/// `systems/ekr/domains/graph.yaml`, as text.
fn domain_text() -> String {
    std::fs::read_to_string(
        std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("Cargo supplies the runtime manifest directory"),
        )
        .join("../../systems/ekr/domains/graph.yaml"),
    )
    .expect("the ESS domain is beside the crates")
}

/// Every `.rs` file of `ekr-graph`'s `src/`, as one string.
fn crate_source() -> String {
    let directory = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("src");
    std::fs::read_dir(directory)
        .expect("the crate has a src/")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .map(|path| std::fs::read_to_string(&path).expect("a source file"))
        .collect()
}

/// The field names one declaration carries, read out of `text`.
///
/// Transcribed from `tests/domain_projection.rs::fields_of` so that this case measures **that**
/// scanner rather than a stricter one of my own. The only change is the source of the text, so
/// that a mutated document can be put through the same rule.
fn fields_of_in(text: &str, name: &str) -> Vec<String> {
    let mut lines = text
        .lines()
        .skip_while(|line| line.trim_start() != format!("- name: {name}"));
    assert!(lines.next().is_some(), "the document declares no {name}");

    let mut fields = Vec::new();
    let mut inside = false;
    for line in lines {
        let trimmed = line.trim();
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

/// The guard's rule as it was: a substring search over the whole crate, with no tie between a
/// declaration and the Rust type that projects it. Kept so that the fix can be shown to be one.
fn the_old_rule(field: &str) -> bool {
    let source = crate_source();
    source.contains(&format!("pub {field}:")) || source.contains(&format!("fn {field}("))
}

/// The guard's rule as it is, transcribed from
/// `domain_projection.rs::every_declaration_of_the_domain_is_carried_field_for_field`: the same
/// two patterns, searched inside the region belonging to the Rust types that project the
/// declaration.
fn the_current_rule(types: &[&str], field: &str) -> bool {
    let region: String = types.iter().map(|name| type_region(name)).collect();
    region.contains(&format!("pub {field}:")) || region.contains(&format!("fn {field}("))
}

/// A Rust type's own text: its declaration and every `impl` block on it. Transcribed from
/// `domain_projection.rs::type_region` so this case measures that scanner rather than a stricter
/// one of its own.
fn type_region(type_name: &str) -> String {
    let directory = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("src");
    let mut region = String::new();
    for entry in std::fs::read_dir(directory).expect("the crate has a src/") {
        let path = entry.expect("a directory entry").path();
        if !path.extension().is_some_and(|e| e == "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a source file");
        let mut at = 0;
        for line in text.lines() {
            if opens_item(line, type_name) {
                let open = at + line.find('{').expect("an item head opens a block");
                let bytes = text.as_bytes();
                let mut depth = 0usize;
                for (offset, byte) in bytes[open..].iter().enumerate() {
                    match byte {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                region.push_str(&text[open..=open + offset]);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            at += line.len() + 1;
        }
    }
    assert!(!region.is_empty(), "no region found for `{type_name}`");
    region
}

/// Whether `line` opens the declaration of `type_name` or an inherent `impl` block on it.
///
/// A match on the head rather than on three literal strings, because
/// `architecture-decision-record:0005-float-is-not-canonical`, as amended, made `Node`, `Edge`,
/// `Object` and `Assertion` generic over the value they carry: their heads read
/// `pub struct Node<V = CanonicalValue> {` and `impl<V> Node<V> {`, and the literal form matched
/// neither — the scanner found no region at all for `Node` and said so, which is why this arrived
/// as a red case rather than as a guard quietly covering nothing.
///
/// A *trait* impl is deliberately not an item head: `impl<V: Canonical> Canonical for Node<V> {`
/// names `Canonical`, not `Node`. That is the behaviour before this change as well — the region is
/// the fields and inherent methods the domain projection is checked against.
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
    } else if let Some(rest) = line.strip_prefix("pub enum ") {
        rest
    } else if let Some(rest) = line.strip_prefix("impl") {
        after_generics(rest).trim_start()
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

/// Add one field to one declaration of the document, in memory.
///
/// The document itself is the coordinator's and is not touched: `systems/` is read, mutated in a
/// `String`, and thrown away. This is the probe the charter permits — the mutation lives inside
/// the case, not in the tree.
fn with_extra_field(text: &str, declaration: &str, field: &str, declared_type: &str) -> String {
    // The newline matters: `ekr.graph.Node` is a prefix of `ekr.graph.NodeId`, which is declared
    // first, and matching the prefix puts the new field in somebody else's declaration.
    let at = text
        .find(&format!("- name: {declaration}\n"))
        .expect("the declaration is in the document");
    let fields_at = at
        + text[at..]
            .find("    fields:\n")
            .expect("the declaration has a fields block")
        + "    fields:\n".len();
    let mut mutated = String::with_capacity(text.len() + 64);
    mutated.push_str(&text[..fields_at]);
    mutated.push_str(&format!(
        "      - name: {field}\n        type: {declared_type}\n"
    ));
    mutated.push_str(&text[fields_at..]);
    mutated
}

/// The guard catches a new field on an existing declaration, whatever it happens to be called.
///
/// `crates/ekr-graph/tests/domain_projection.rs:358-360` states the bound in its own words:
///
/// > The declarations are read from the document rather than listed, so a new entity cannot be
/// > omitted, and a new *field* on an existing one cannot be either: it is carried, or it is an
/// > exception with a reason, or this is red.
///
/// The rule it enforces is `source.contains("pub {field}:") || source.contains("fn {field}(")`
/// over **the whole crate's `src/` concatenated**, with no tie between a declaration and the Rust
/// type that projects it. So the guard answers "carried" whenever *some* type in the crate happens
/// to have a field or method of that name — and the crate has about thirty field names for eight
/// entities, so the names collide.
///
/// **Answered by correction round 2.** Each declaration is now bound to the Rust types that
/// project it by a `PROJECTIONS` table, itself checked against the document, and each field is
/// looked for inside those types' declarations and `impl` blocks rather than anywhere in the
/// crate.
///
/// The case keeps both probes and adds the comparison that shows the fix is one: under the old
/// rule `parent` was reported carried, under the current rule it is not, and the control that
/// nothing in the crate uses is caught under both. If the rule ever widens back, the first
/// assertion below goes red.
#[test]
fn the_field_guard_catches_a_new_field_whatever_it_is_called() {
    let document = domain_text();
    // The types `PROJECTIONS` binds `ekr.graph.Node` to, transcribed.
    let node = ["Node"];

    // A control: the guard is real, and a name the crate does not use is caught.
    let unheard_of = with_extra_field(
        &document,
        "ekr.graph.Node",
        "knowledge_state",
        "ekr.graph.KnowledgeState",
    );
    let unheard_of_fields = fields_of_in(&unheard_of, "ekr.graph.Node");
    assert!(
        unheard_of_fields.contains(&"knowledge_state".to_owned()),
        "the mutation did not land: {unheard_of_fields:?}"
    );
    assert!(
        !the_current_rule(&node, "knowledge_state"),
        "the control is broken: the crate already carries a `knowledge_state`, so this case cannot \
         tell a working guard from a broken one"
    );

    // The bound: a new field whose name the crate already uses somewhere else.
    let colliding = with_extra_field(
        &document,
        "ekr.graph.Node",
        "parent",
        "Optional<ekr.graph.NodeId>",
    );
    let colliding_fields = fields_of_in(&colliding, "ekr.graph.Node");
    assert!(
        colliding_fields.contains(&"parent".to_owned()),
        "the mutation did not land: {colliding_fields:?}"
    );
    // `ekr.graph.Node` has no `parent` in the real document, and `ekr_graph::Node` has no such
    // field — so a faithful guard must report it missing.
    assert!(
        !fields_of_in(&document, "ekr.graph.Node").contains(&"parent".to_owned()),
        "the real document already declares ekr.graph.Node.parent; pick another name"
    );

    // The defect, still measurable: the old rule reports it carried.
    assert!(
        the_old_rule("parent"),
        "the finding no longer reproduces — `pub parent:` has left the crate, so this case is \
         measuring nothing and needs a different colliding name"
    );

    // The fix: bound to `ekr.graph.Node`'s own Rust type, it is not.
    assert!(
        !the_current_rule(&node, "parent"),
        "every_declaration_of_the_domain_is_carried_field_for_field would report a new \
         ekr.graph.Node.parent as carried. Its rule must be a search inside the region of the Rust \
         types PROJECTIONS binds to the declaration, not a substring search over the whole crate — \
         `pub parent:` on GraphRoot and on Root must not answer for a field on Node, and the eight \
         declarations share about thirty field names."
    );

    // And the two rules disagree exactly where the finding said they would, which is what makes
    // the change a fix rather than a coincidence.
    assert_ne!(
        the_old_rule("parent"),
        the_current_rule(&node, "parent"),
        "the guard's rule is still the crate-wide one"
    );
}

// ---------------------------------------------------------------------------------------------
// The new bitemporal surface, at its boundaries.
// ---------------------------------------------------------------------------------------------

fn empty_ontology(schema: SchemaVersionId) -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(schema, TENURE_BEGAN),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("an empty ontology coheres")
}

fn graph_of(assertions: Vec<Assertion>) -> CanonicalGraph {
    let schema = SchemaVersionId::mint();
    CanonicalGraph {
        root: GraphRoot {
            id: GraphRootId::mint(),
            space: Space::Canonical,
            schema_version_id: schema,
            parent: None,
            created_at: TENURE_BEGAN,
        },
        revision: RevisionNumber::new(7),
        ontology: empty_ontology(schema),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: assertions.into_iter().map(|a| (a.id, a)).collect(),
        evidence: BTreeMap::new(),
    }
}

fn accepted_assertion(valid_time: TemporalRange, transaction_time: TransactionTime) -> Assertion {
    let id = AssertionId::mint();
    Assertion {
        id,
        root_id: GraphRootId::mint(),
        subject: Subject::Node(CanonicalRef::new(NodeId::mint())),
        predicate: Predicate::Relation(TypeId::mint()),
        object: Object::Node(CanonicalRef::new(NodeId::mint())),
        evidence: BTreeSet::from([EvidenceId::mint()]),
        proposed_by: AgentId::mint(),
        validation: ValidationState::Accepted {
            validators: [AgentId::mint()].into_iter().collect(),
        },
        valid_time,
        transaction_time,
    }
}

/// A range whose end precedes its start is refused at construction.
///
/// **The finding.** `TransactionTime::new` and `TemporalRange::new` were `const fn`s taking their
/// two bounds independently and accepting `to < from`, so an inverted interval was an ordinary
/// value of each type. What it produced was a record no instant reaches: `held_at` was false at
/// every `Timestamp` including its own `recorded_from`, and an assertion carrying an inverted
/// valid time sat in canonical state and was returned by `valid_at` at no `t` at all — a silent
/// hole rather than a refusal, with no validator in P1 to catch it.
///
/// **The resolution.** The constructors own it. Design § 20's validator list does not include
/// temporal ordering and neither does AGENTS.md invariant 7, so deferring it left it owned by
/// nobody; `Confidence::from_basis_points` was already the crate's pattern for refusing at
/// construction, and both constructors now follow it. `held_at` is gone with the read it served.
///
/// The case is aimed at the refusal, and at the boundary the refusal must **not** cross: `[t, t)`
/// is the empty half-open interval a fact recorded and corrected inside one millisecond produces,
/// and refusing it would be a claim about clock resolution that nothing in P1 supports.
#[test]
fn an_inverted_range_is_refused_or_describes_some_instant() {
    let probes = [
        Timestamp::from_millis(i64::MIN),
        Timestamp::EPOCH,
        TENURE_BEGAN,
        Timestamp::from_millis(TENURE_BEGAN.millis() - 1),
        HANDOVER,
        Timestamp::from_millis(HANDOVER.millis() + 1),
        Timestamp::from_millis(i64::MAX),
    ];

    // Refused, on both dimensions.
    assert_eq!(
        TransactionTime::new(HANDOVER, Some(TENURE_BEGAN)),
        None,
        "a belief withdrawn before it was formed was accepted"
    );
    assert_eq!(
        TemporalRange::new(Some(HANDOVER), Some(TENURE_BEGAN)),
        None,
        "a valid time whose end precedes its start was accepted"
    );
    assert_eq!(
        TemporalRange::new(
            Some(Timestamp::from_millis(i64::MAX)),
            Some(Timestamp::from_millis(i64::MIN))
        ),
        None,
        "the widest inversion representable was accepted"
    );

    // Admitted, and it has to be: the empty interval, and every half-bounded shape.
    let empty = TemporalRange::new(Some(HANDOVER), Some(HANDOVER))
        .expect("[t, t) is the empty interval, not an inverted one");
    assert!(
        !probes.iter().any(|at| empty.contains(*at)),
        "the empty interval contains no instant, which is what makes it empty"
    );
    let withdrawn_at_once = TransactionTime::new(HANDOVER, Some(HANDOVER))
        .expect("a belief formed and corrected inside one millisecond is ordinary");
    assert!(!withdrawn_at_once.is_open());
    assert!(TemporalRange::new(None, Some(HANDOVER)).is_some());
    assert!(TemporalRange::new(Some(HANDOVER), None).is_some());
    assert!(TemporalRange::new(None, None).is_some());
    assert!(TransactionTime::new(HANDOVER, None).is_some());

    // The consequence the finding was really about: no accepted, still-believed assertion can be
    // put into canonical state and reached by no read, because the range it would need is not a
    // value. Every range that *is* a value and is not empty answers somewhere.
    for range in [
        TemporalRange::UNBOUNDED,
        TemporalRange::since(TENURE_BEGAN),
        TemporalRange::new(None, Some(HANDOVER)).expect("not inverted"),
        TemporalRange::new(Some(TENURE_BEGAN), Some(HANDOVER)).expect("not inverted"),
    ] {
        let record = accepted_assertion(range, TransactionTime::since(TENURE_BEGAN));
        let id = record.id;
        let graph = graph_of(vec![record]);
        let snapshot = GraphSnapshot::of(&graph);
        assert_eq!(graph.assertions.len(), 1, "the fixture lost the record");
        assert!(
            probes
                .iter()
                .any(|at| snapshot.valid_at(*at).iter().any(|a| a.id == id)),
            "an accepted, still-believed assertion carrying {range:?} is held in canonical state \
             and is returned by valid_at at no instant at all"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The re-aimed pass-1 guard, and the file it does not look in.
// ---------------------------------------------------------------------------------------------

/// The "no such read comes back" guard covers every file such a read could live in.
///
/// **The finding.** Pass 1 found that `active()` was `valid_at(i64::MAX)` under a name claiming
/// "now". Correction 1 deleted `active()` and re-aimed the case to
/// `tests/adversary_snapshot_and_assertion.rs::active_is_not_merely_valid_at_the_end_of_representable_time`,
/// which read `src/snapshot.rs` — one module of ten — while `TemporalRange::is_open` stayed public
/// and exported with a doc comment explaining that no read should be built on it. The caller was
/// removed; the capability was not.
///
/// **The resolution, in two parts.** `TemporalRange::is_open` is deleted — a method whose only
/// users were a vacuity assertion and the cases written about it is surface, not a warning — and
/// the guard scans every module of `src/` for the filter in both the spellings that remain.
/// `TransactionTime::is_open` stays, because `is_current()` calls it.
///
/// **What none of this reaches, and the guard now says so.** Deleting the method did not make the
/// filter unrepresentable: `valid_time.to` is a public field and `to.is_none()` rebuilds it in one
/// line, here or in `ekr-kernel`, which is the next story. A text search over one crate's `src/`
/// is exactly as far as this goes.
#[test]
fn the_guard_against_a_returning_open_ended_read_covers_the_whole_crate() {
    let guard = std::fs::read_to_string(
        std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("Cargo supplies the runtime manifest directory"),
        )
        .join("tests/adversary_snapshot_and_assertion.rs"),
    )
    .expect("the pass-1 case is still in the tree");

    let mut modules: Vec<String> = std::fs::read_dir(
        std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("Cargo supplies the runtime manifest directory"),
        )
        .join("src"),
    )
    .expect("the crate has a src/")
    .map(|entry| entry.expect("a directory entry").path())
    .filter(|path| path.extension().is_some_and(|e| e == "rs"))
    .map(|path| {
        path.file_name()
            .expect("a file has a name")
            .to_string_lossy()
            .into_owned()
    })
    .collect();
    modules.sort();
    assert!(modules.len() >= 9, "the module scan is broken: {modules:?}");

    // The capability the guard existed to keep unused is gone from the valid-time type. Checked
    // inside `TemporalRange`'s own impl block, because `TransactionTime::is_open` has the same
    // signature and stays.
    assert!(
        !the_current_rule(&["TemporalRange"], "is_open"),
        "TemporalRange::is_open is back: the filter `active()` was is one method call away again"
    );
    assert!(
        the_current_rule(&["TransactionTime"], "is_open"),
        "TransactionTime::is_open is what `is_current()` calls; if it has gone, this case is \
         measuring the wrong type"
    );

    // The guard reads every module rather than naming one. Checked by outcome — the rule applied
    // here, over all of them — rather than by looking for module names in the guard's text: the
    // guard now enumerates the directory, so there are no file names in it to look for, and a rule
    // that is satisfied by a directory scan is the one worth having.
    assert!(
        guard.contains("crate_modules()"),
        "the pass-1 guard no longer enumerates src/; it was reading one module of ten when this \
         case was written"
    );
    assert!(
        !guard.contains("/src/snapshot.rs"),
        "the pass-1 guard is back to naming a single module"
    );

    for module in &modules {
        let text = std::fs::read_to_string(
            std::path::PathBuf::from(
                std::env::var("CARGO_MANIFEST_DIR")
                    .expect("Cargo supplies the runtime manifest directory"),
            )
            .join("src")
            .join(module),
        )
        .expect("a source file");
        for filter in ["valid_time.is_open", "valid_time.to.is_none"] {
            assert!(
                !text.contains(filter),
                "src/{module} contains {filter}: the filter `active()` was, rebuilt in a module \
                 the original guard did not read. {} modules exist and it read one.",
                modules.len()
            );
        }
    }
}

/// The item scanner exists byte-identically in two files, and this is what says so.
///
/// `type_region` above is transcribed from `domain_projection.rs::type_region` so that this file
/// measures *that* scanner rather than a stricter one of its own — which only holds while the two
/// are the same text. Nothing asserted it, and the helper has now been rewritten twice: once for
/// the generic heads `architecture-decision-record:0005-float-is-not-canonical` introduced, once
/// for the bounds in its doc. A copy that drifts turns this file into a case about itself.
#[test]
fn the_item_scanner_is_the_same_text_in_both_files() {
    /// From the head of `opens_item`'s doc comment through the end of `after_generics`.
    fn scanner(file: &str) -> String {
        let source = std::fs::read_to_string(format!(
            "{}/tests/{file}",
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("Cargo supplies the runtime manifest directory")
        ))
        .unwrap_or_else(|e| panic!("reading {file}: {e}"));
        let start = source
            .find("/// Whether `line` opens the declaration of `type_name`")
            .unwrap_or_else(|| panic!("{file} carries the item scanner"));
        let last = source[start..]
            .find("fn after_generics(text: &str) -> &str {")
            .unwrap_or_else(|| panic!("{file} carries after_generics beside it"))
            + start;
        let end = source[last..]
            .find("\n}\n")
            .unwrap_or_else(|| panic!("after_generics closes in {file}"))
            + last
            + 3;
        source[start..end].to_owned()
    }

    let here = scanner("adversary2_guard_bounds_and_ranges.rs");
    let there = scanner("domain_projection.rs");
    assert!(
        here.len() > 500,
        "the extraction is broken, not the files: {} bytes",
        here.len()
    );
    assert_eq!(
        here, there,
        "the two copies of the item scanner have drifted, so this file no longer measures the \
         guard in domain_projection.rs — it measures a different scanner that happens to live \
         beside it"
    );
}
