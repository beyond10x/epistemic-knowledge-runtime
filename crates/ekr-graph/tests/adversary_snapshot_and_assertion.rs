//! Adversarial cases against `GraphSnapshot` and `Assertion`, wave p1-04 pass 1, **re-aimed after
//! correction round 1.**
//!
//! Each case was written to fail on a property the crate's own documentation claimed. Each has
//! been kept and pointed at the statement that is now true, the way wave p1-03 re-aimed its two —
//! a case deleted once it is green is a case that stops guarding the thing it found.
//!
//! | original finding | what the case holds now |
//! |---|---|
//! | `active()` is `valid_at(i64::MAX)` | no public read answers "valid time with no known end" |
//! | the `AssertionStatus` conjunct is unreachable | `is_current()` is exactly its three conjuncts |
//! | `recorded_from` is required and the doctest omits it | the omission is not representable |
//! | `is_open` reads only the upper bound | there is no lower bound to read |
//!
//! Correction round 2 widened the first of them from one module to all of `src/`, and removed
//! `TemporalRange::is_open` outright — a method whose own doc explained why nothing should call it
//! is surface, not a warning.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, EvidenceId, GraphRootId, IssueId, NodeId, RevisionNumber,
    SchemaVersionId, Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, CanonicalRef, GraphRoot,
    GraphSnapshot, Object, Predicate, RetractionReason, Space, Subject, TemporalRange,
    TransactionTime,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};

/// 2024-01-01T00:00:00Z.
const TENURE_BEGAN: Timestamp = Timestamp::from_millis(1_704_067_200_000);
/// 2026-03-12T00:00:00Z — the day design § 65 hands the chair over.
const HANDOVER: Timestamp = Timestamp::from_millis(1_773_273_600_000);
/// 2027-01-01T00:00:00Z — an instant after the handover, to stand in for "now".
const AFTER_HANDOVER: Timestamp = Timestamp::from_millis(1_798_761_600_000);

/// Every `.rs` file of this crate's `src/`, as `(file name, text)`.
fn crate_modules() -> Vec<(String, String)> {
    let directory = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
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
    assert!(found.len() >= 9, "the module scan is broken: {found:?}");
    found
}

fn empty_ontology(schema: SchemaVersionId) -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(schema, TENURE_BEGAN),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("an empty ontology coheres")
}

/// One active assertion with the valid time and assessment given, and an open transaction time.
fn assertion(valid_time: TemporalRange, assessment: Assessment) -> Assertion {
    assertion_with(valid_time, assessment, AssertionLifecycle::Active)
}

/// One assertion with the valid time, assessment and lifecycle given, and an open transaction time.
fn assertion_with(
    valid_time: TemporalRange,
    assessment: Assessment,
    lifecycle: AssertionLifecycle,
) -> Assertion {
    let id = AssertionId::mint();
    Assertion {
        id,
        root_id: GraphRootId::mint(),
        subject: Subject::Node(CanonicalRef::new(NodeId::mint())),
        predicate: Predicate::Relation(TypeId::mint()),
        object: Object::Node(CanonicalRef::new(NodeId::mint())),
        evidence: BTreeSet::from([CanonicalRef::new(EvidenceId::mint())]),
        proposed_by: AgentId::mint(),
        assessment,
        lifecycle,
        valid_time,
        transaction_time: TransactionTime::since(TENURE_BEGAN),
    }
}

fn accepted() -> Assessment {
    Assessment::Accepted {
        validators: [AgentId::mint()].into_iter().collect(),
    }
}

/// A canonical graph holding exactly the assertions given.
fn graph_of(assertions: Vec<Assertion>) -> CanonicalGraph {
    let schema = SchemaVersionId::mint();
    let root_id = GraphRootId::mint();
    CanonicalGraph {
        root: GraphRoot {
            id: root_id,
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

/// Every assessment the domain declares, one value each.
fn every_assessment() -> Vec<Assessment> {
    vec![
        Assessment::Proposed,
        Assessment::Validating {
            completed: 1,
            required: 2,
        },
        accepted(),
        Assessment::Rejected {
            issues: vec![IssueId::mint()],
        },
        Assessment::Disputed {
            competing_assertions: vec![AssertionId::mint()],
        },
    ]
}

/// Every lifecycle the domain declares, one value each.
fn every_lifecycle() -> Vec<AssertionLifecycle> {
    vec![
        AssertionLifecycle::Active,
        AssertionLifecycle::Superseded {
            by: AssertionId::mint(),
            at_revision: RevisionNumber::new(5),
            effective_from: HANDOVER,
        },
        AssertionLifecycle::Retracted {
            at_revision: RevisionNumber::new(4),
            reason: RetractionReason::new("the evidence was another Acme"),
        },
    ]
}

/// No public read answers "valid time with no known end".
///
/// **The finding.** `snapshot.rs` called `active()` "the current-world query: the assertions
/// canonical state holds true **now**", and it was `is_current() && valid_time.to.is_none()`.
/// `valid_at(t)` is `is_current() && from.is_none_or(|f| f <= t) && to.is_none_or(|to| t < to)`;
/// at `t = i64::MAX` the lower bound is satisfied by every `from` and the upper by no `to` at all,
/// so the two filters are one filter. Measured, over every shape of valid time crossed with every
/// validation state: the same answer. The "current world" was the world at the end of representable
/// time, which is why `active()` answered Bob — not because now is after the handover, but because
/// nobody had closed his tenure.
///
/// **The resolution, which is not a rename.** `active()` is deleted. The runtime has no clock, so
/// a clock-free current-world read cannot exist, and `valid_at(t)` carries both roles: the
/// caller's now gives the current world, any other instant gives a historical one.
///
/// **What the case holds now.** That no such read comes back **anywhere in this crate**. It reads
/// `src/` the way `tests/domain_projection.rs` reads the domain, and asserts three things: the
/// public reads on `GraphSnapshot` are exactly the four that remain, `TemporalRange` no longer
/// exposes an `is_open` for a read to be built on, and no module filters on valid time having no
/// known end.
///
/// # What it does not hold, and this is the second time in this wave a guard's doc overclaimed
///
/// Removing `TemporalRange::is_open` did not make the filter unrepresentable. `valid_time.to`
/// is a public field and `to.is_none()` rebuilds it in one line, here or in any crate above this
/// one. This case is a text search over `crates/ekr-graph/src/`, and that is the whole of its
/// reach. The cross-crate owner is now
/// `crates/ekr/tests/temporal_reads.rs::no_product_source_rebuilds_the_open_ended_valid_time_filter`,
/// which recursively scans product source in every workspace crate. Neither textual tripwire
/// proves that all semantically equivalent expressions are impossible.
///
/// Widened after adversary pass 2, which found it reading one module of ten while the capability
/// it bans stayed exported.
#[test]
fn active_is_not_merely_valid_at_the_end_of_representable_time() {
    let modules = crate_modules();
    let snapshot_source = modules
        .iter()
        .find(|(name, _)| name == "snapshot.rs")
        .map(|(_, text)| text.clone())
        .expect("the crate has a snapshot module");

    let public: Vec<String> = snapshot_source
        .lines()
        .map(str::trim_start)
        .filter_map(|line| {
            line.strip_prefix("pub const fn ")
                .or_else(|| line.strip_prefix("pub fn "))
        })
        .map(|rest| {
            rest.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .collect();

    assert_eq!(
        public,
        vec!["of", "revision", "graph", "valid_at"],
        "a snapshot read was added or removed. `valid_at` is the only read P1 has: the runtime \
         cannot supply \"now\", so any read that does not take an instant from its caller is \
         answering a question it cannot know the answer to — which is what `active()` did."
    );

    // Every module, not just the one the read used to live in. `CanonicalGraph` could grow a
    // `standing()` and `Assertion` an `is_open_ended()` just as easily.
    for (module, text) in &modules {
        for filter in ["valid_time.is_open", "valid_time.to.is_none"] {
            assert!(
                !text.contains(filter),
                "src/{module} filters on valid time having no known end ({filter}). That is not \
                 \"still true\": a fixed-term appointment that runs until next year is true today \
                 and fails it, and an announced successor whose tenure begins next year is not \
                 true today and passes it. It is the filter `active()` was, under a new name."
            );
        }
    }

    // And the capability itself is gone from the valid-time type, so rebuilding it takes a
    // deliberate line rather than a method call. `TransactionTime::is_open` stays: `is_current()`
    // calls it, and it asks a different question.
    let temporal_range = modules
        .iter()
        .map(|(_, text)| text.as_str())
        .find_map(|text| text.split_once("impl TemporalRange {"))
        .expect("the crate declares TemporalRange")
        .1;
    let temporal_range =
        &temporal_range[..temporal_range.find("\n}").expect("the impl block closes")];
    assert!(
        !temporal_range.contains("fn is_open"),
        "TemporalRange::is_open is back. It existed only to be warned about — its own doc \
         explained why no read should be built on it — and P1 ships no public surface whose only \
         users are the cases written about it."
    );

    // And the property that made the two reads indistinguishable is still true of the one that is
    // left, which is why the deletion — rather than a second read — was the answer.
    let end_of_time = Timestamp::from_millis(i64::MAX);
    let shapes = [
        TemporalRange::UNBOUNDED,
        TemporalRange::since(TENURE_BEGAN),
        TemporalRange::since(AFTER_HANDOVER),
        TemporalRange::new(None, Some(HANDOVER)).expect("not inverted"),
        TemporalRange::new(Some(TENURE_BEGAN), Some(HANDOVER)).expect("not inverted"),
        // A fixed-term appointment whose end is known in advance and has not arrived: true now.
        TemporalRange::new(Some(TENURE_BEGAN), Some(AFTER_HANDOVER)).expect("not inverted"),
        TemporalRange::new(Some(HANDOVER), None).expect("not inverted"),
        TemporalRange::new(Some(end_of_time), None).expect("not inverted"),
    ];

    // Amendment 88: supersession closes the former interval at its boundary, so a superseded
    // record always has a finite `to`. Crossing it with an open-ended shape would build a state
    // the writer cannot produce, and that state is the only one on which the two reads below
    // differ.
    let mut population = Vec::new();
    for shape in shapes {
        for assessment in every_assessment() {
            for lifecycle in every_lifecycle() {
                if matches!(lifecycle, AssertionLifecycle::Superseded { .. }) && shape.to.is_none()
                {
                    continue;
                }
                population.push(assertion_with(shape, assessment.clone(), lifecycle));
            }
        }
    }
    let count = population.len();
    let graph = graph_of(population);
    let snapshot = GraphSnapshot::of(&graph);
    assert_eq!(graph.assertions.len(), count, "the fixture lost a record");

    let open_ended: BTreeSet<AssertionId> = graph
        .assertions
        .values()
        .filter(|a| a.is_current() && a.valid_time.to.is_none())
        .map(|a| a.id)
        .collect();
    let at_end_of_time: BTreeSet<AssertionId> = snapshot
        .valid_at(end_of_time)
        .iter()
        .map(|a| a.id)
        .collect();
    let now: BTreeSet<AssertionId> = snapshot
        .valid_at(AFTER_HANDOVER)
        .iter()
        .map(|a| a.id)
        .collect();

    assert!(!open_ended.is_empty(), "the fixture is vacuous");
    assert_eq!(
        open_ended, at_end_of_time,
        "the old active() filter and valid_at(i64::MAX) are one read over {count} records; if \
         they have come apart, the finding needs re-measuring rather than this assertion loosening"
    );
    assert_ne!(
        now, at_end_of_time,
        "and an instant a caller might actually pass is a different answer from the end of time, \
         which is the whole reason the read takes one"
    );
}

/// `is_current()` is exactly `is_accepted() && transaction_time.is_open()`, and nothing more.
///
/// Was `the_assertion_status_clause_of_is_current_does_work`, which asked for one assertion on
/// which a third conjunct — `matches!(self.status(), AssertionStatus::Active)` — changed the
/// answer. There is none, and there cannot be: `status()` is *derived* from `validation`, is
/// non-`Active` only for `Retracted` and `Superseded`, and `is_accepted()` is true only for
/// `Accepted`. The sets are disjoint by construction, so the conjunct was unreachable — a clause
/// that reads as a check and is not one, in the method every read depends on.
///
/// The conjunct was deleted. Amendment 88 then made lifecycle a stored field independent of
/// assessment, so an accepted record can be retracted or superseded and the lifecycle conjunct is
/// reachable again. This case pins the three conditions, over every assessment crossed with every
/// lifecycle and both transaction-time shapes, so a fourth arriving without an input that reaches
/// it turns red where it is written.
#[test]
fn is_current_is_exactly_acceptance_an_active_lifecycle_and_an_open_transaction_time() {
    let mut checked = 0usize;
    let mut current = 0usize;

    for assessment in every_assessment() {
        for lifecycle in every_lifecycle() {
            for transaction_time in [
                TransactionTime::since(TENURE_BEGAN),
                TransactionTime::new(TENURE_BEGAN, Some(HANDOVER)).expect("not inverted"),
            ] {
                let record = Assertion {
                    transaction_time,
                    ..assertion_with(
                        TemporalRange::UNBOUNDED,
                        assessment.clone(),
                        lifecycle.clone(),
                    )
                };
                let three_conjuncts = record.assessment.is_accepted()
                    && matches!(record.lifecycle, AssertionLifecycle::Active)
                    && record.transaction_time.is_open();
                assert_eq!(
                    record.is_current(),
                    three_conjuncts,
                    "is_current() disagrees with `is_accepted() && Active && \
                     transaction_time.is_open()` for {} / {} with recorded_to = {:?}. Either a \
                     conjunct was added — in which case it needs a case of its own — or one was \
                     dropped.",
                    assessment.name(),
                    lifecycle.name(),
                    record.transaction_time.recorded_to
                );
                checked += 1;
                current += usize::from(record.is_current());
            }
        }
    }

    assert_eq!(checked, 30, "the cross product lost a case");
    assert_eq!(
        current, 1,
        "exactly one of the thirty is current: Accepted, Active, with an open transaction time. If \
         more are, the filter stopped filtering; if none is, the case proves nothing."
    );
}

/// `ekr.graph.Assertion.recorded_from` is declared `Timestamp`, and the crate now carries it that
/// way.
///
/// `systems/ekr/domains/graph.yaml:256-259`:
///
/// ```yaml
///       - name: recorded_from
///         type: Timestamp
///       - name: recorded_to
///         type: Optional<Timestamp>
/// ```
///
/// **The finding.** Transaction time was a `TemporalRange`, whose two bounds are both `Option`, so
/// a record with no `recorded_from` was representable — and the crate's own published example
/// built one, `transaction_time: TemporalRange::UNBOUNDED` in the doctest `cargo test` runs.
/// `is_current()` then reported as the runtime's current belief a record it had no record of ever
/// forming.
///
/// **The resolution.** Transaction time has its own type, [`TransactionTime`], whose
/// `recorded_from` is a `Timestamp` and not an `Option`. Valid time keeps `TemporalRange`, because
/// `valid_from` and `valid_to` really are both optional. The case now reads the domain's own
/// declaration and holds the crate to it, rather than asserting a fact about one hand-built value:
/// if `graph.yaml` ever makes `recorded_from` optional, this goes red and asks for the type to
/// follow.
#[test]
fn the_domain_requires_a_recorded_from_and_the_crate_cannot_omit_one() {
    let domain = std::fs::read_to_string(
        std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("Cargo supplies the runtime manifest directory"),
        )
        .join("../../systems/ekr/domains/graph.yaml"),
    )
    .expect("the ESS domain is beside the crates");

    let declared = |field: &str| -> String {
        let at = domain
            .find(&format!("- name: {field}"))
            .unwrap_or_else(|| panic!("the domain declares no field {field}"));
        domain[at..]
            .lines()
            .nth(1)
            .and_then(|line| line.trim().strip_prefix("type: "))
            .unwrap_or_else(|| panic!("{field} has no declared type"))
            .to_owned()
    };

    assert_eq!(declared("recorded_from"), "Timestamp");
    assert_eq!(declared("recorded_to"), "Optional<Timestamp>");

    // The record `crates/ekr-graph/src/lib.rs` publishes, field for field. It no longer compiles
    // without a `recorded_from`, which is the whole of the fix: the state is gone, not filtered.
    let held: Assertion = Assertion {
        id: AssertionId::mint(),
        root_id: GraphRootId::mint(),
        subject: Subject::Node(CanonicalRef::new(NodeId::mint())),
        predicate: Predicate::Relation(TypeId::mint()),
        object: Object::Node(CanonicalRef::new(NodeId::mint())),
        evidence: BTreeSet::new(),
        proposed_by: AgentId::mint(),
        assessment: Assessment::Accepted {
            validators: BTreeSet::new(),
        },
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::new(None, Some(HANDOVER)).expect("not inverted"),
        transaction_time: TransactionTime::since(TENURE_BEGAN),
    };

    // The projection back to the entity is total, which is what the domain says it is.
    assert_eq!(held.transaction_time.recorded_from, TENURE_BEGAN);
    assert_eq!(held.transaction_time.recorded_to, None);
    assert!(held.is_current());
}

/// A belief the runtime never recorded forming is unrepresentable, not filtered.
///
/// Was `a_record_with_no_transaction_time_at_all_is_not_a_current_belief`, which built one with
/// `TemporalRange::UNBOUNDED` and asked the read not to answer with it. Design § 14.2 makes
/// transaction time "when the system believed or stored the assertion", and `is_open` looked only
/// at `to`, so a record whose transaction time never *started* was "still open" and was reported
/// as current.
///
/// The fix is not a filter. [`TransactionTime`] has no constructor that omits the start and no
/// `Option` to leave empty, so the state the old case described cannot be built to be excluded.
/// What is left to check is that `is_open` still means what `is_current` reads it as, which after
/// the change is a statement about the upper bound alone — correctly, because there is no lower
/// bound to forget.
#[test]
fn a_record_with_no_transaction_time_at_all_is_not_a_current_belief() {
    let source = std::fs::read_to_string(
        std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("Cargo supplies the runtime manifest directory"),
        )
        .join("src/assertion.rs"),
    )
    .expect("the crate's own source");
    let transaction_time = source
        .split_once("pub struct TransactionTime {")
        .expect("the crate declares TransactionTime")
        .1
        .split_once('}')
        .expect("the declaration closes")
        .0;

    assert!(
        transaction_time.contains("pub recorded_from: Timestamp,"),
        "TransactionTime.recorded_from must be a Timestamp and not an Option: a belief with no \
         beginning is the state this case exists to make unrepresentable. Declared as: \
         {transaction_time}"
    );
    assert!(
        transaction_time.contains("pub recorded_to: Option<Timestamp>,"),
        "and recorded_to must stay optional, because a belief the runtime still holds has no end"
    );

    // Both shapes the type admits, and the only two, against the meaning `is_current` reads.
    let still_held = TransactionTime::since(TENURE_BEGAN);
    let withdrawn = TransactionTime::new(TENURE_BEGAN, Some(HANDOVER)).expect("not inverted");

    assert!(still_held.is_open());
    assert!(!withdrawn.is_open());
    assert_eq!(still_held.recorded_from, TENURE_BEGAN);
    assert_eq!(withdrawn.recorded_to, Some(HANDOVER));

    // Withdrawn before it was formed is not a third shape: the constructor refuses it.
    assert_eq!(
        TransactionTime::new(HANDOVER, Some(TENURE_BEGAN)),
        None,
        "a belief withdrawn before it was formed is held at no instant, including its own \
         recorded_from, and nothing in P1 would report it"
    );

    let record = Assertion {
        transaction_time: still_held,
        ..assertion(TemporalRange::UNBOUNDED, accepted())
    };
    let id = record.id;
    let graph = graph_of(vec![record]);
    let snapshot = GraphSnapshot::of(&graph);
    assert!(
        snapshot.valid_at(AFTER_HANDOVER).iter().any(|a| a.id == id),
        "a record the runtime began holding and has not withdrawn is a current belief"
    );
}
