//! `task:one-event-type-rule`: `/roles`, the timeline and `ekr ocel` agree on the event types of
//! every views fixture, at every revision it holds.
//!
//! One rule decides which node types are events (`views.yaml`, `ekr.views.TypeTiming` `event`),
//! and [`Index::event_types`] is it. Each consumer is read through its own answer, not through
//! that function:
//!
//! * `/roles`: the `event` entries of [`Index::view_roles_document`] (`ekr.view-roles/1`);
//! * the timeline: the overview's `roles.types[].event`, which the page's timeline reads, and
//!   the types of every event `ekr.graph-timeline/1` lists for any subject, which must be among
//!   them;
//! * `ekr ocel`: the `ocel.eventTypes` names of [`ekr_views::ocel`] with no type named.

mod support;

use std::collections::BTreeSet;

use ekr_core::{NodeId, RevisionNumber};
use ekr_views::{Index, OverviewRequest, TimelineRequest};
use serde_json::Value;

use support::fixtures::{self, Fixture, Provider};

/// Every fixture store, each once. The `match` in [`every_fixture_is_listed`] fails to compile
/// when a fixture is added and not listed here.
const FIXTURES: [Fixture; 14] = [
    Fixture::SeedOnly,
    Fixture::SeededEvidence,
    Fixture::EdgeAssertion,
    Fixture::RetractedAssertion,
    Fixture::SchemaEvolution,
    Fixture::Evolved,
    Fixture::Hub,
    Fixture::Timeline,
    Fixture::Growth,
    Fixture::Subjects,
    Fixture::Changes,
    Fixture::Quality,
    Fixture::Ocel,
    Fixture::SchemaChanges,
];

#[test]
fn every_fixture_is_listed() {
    for fixture in FIXTURES {
        match fixture {
            Fixture::SeedOnly
            | Fixture::SeededEvidence
            | Fixture::EdgeAssertion
            | Fixture::RetractedAssertion
            | Fixture::SchemaEvolution
            | Fixture::Evolved
            | Fixture::Hub
            | Fixture::Timeline
            | Fixture::Growth
            | Fixture::Subjects
            | Fixture::Changes
            | Fixture::Quality
            | Fixture::Ocel
            | Fixture::SchemaChanges => {}
        }
    }
    let distinct: BTreeSet<String> = FIXTURES.iter().map(|f| format!("{f:?}")).collect();
    assert_eq!(distinct.len(), FIXTURES.len(), "each fixture once");
}

fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("a JSON document")
}

fn strings(values: impl IntoIterator<Item = Value>) -> BTreeSet<String> {
    values
        .into_iter()
        .map(|value| value.as_str().expect("a type id").to_owned())
        .collect()
}

/// The `event` entries of `/roles`.
fn roles_events(index: &Index) -> BTreeSet<String> {
    let roles = json(&index.view_roles_document());
    strings(
        roles["node_types"]
            .as_array()
            .expect("node_types")
            .iter()
            .filter(|entry| entry["role"] == "event")
            .map(|entry| entry["type_id"].clone()),
    )
}

/// The overview's event types, which the timeline reads.
fn overview_events(index: &Index) -> BTreeSet<String> {
    let request = OverviewRequest::new(None).expect("the default limit");
    let overview = json(&index.overview(&request).expect("the overview").bytes);
    strings(
        overview["roles"]["types"]
            .as_array()
            .expect("roles.types")
            .iter()
            .filter(|timing| timing["event"] == true)
            .map(|timing| timing["type"].clone()),
    )
}

/// The types of every event the timeline lists for any subject, within three hops.
fn timeline_events(index: &Index, nodes: &[NodeId]) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for node in nodes {
        let request = TimelineRequest::new(None, 3, 500, None, Some(*node)).expect("bounds");
        let answer = json(&index.timeline(&request).expect("the timeline").bytes);
        for event in answer["events"].as_array().expect("events") {
            found.insert(event["type"].as_str().expect("a type").to_owned());
        }
        for row in answer["rows"].as_array().expect("rows") {
            for cell in row["cells"].as_array().expect("cells") {
                found.insert(cell["type"].as_str().expect("a type").to_owned());
            }
        }
    }
    found
}

/// The event types of `ekr ocel` by the rule.
fn ocel_events(index: &Index) -> BTreeSet<String> {
    let exported = json(&ekr_views::ocel(index, &[]).expect("the export").bytes);
    strings(
        exported["ocel"]["eventTypes"]
            .as_array()
            .expect("eventTypes")
            .iter()
            .map(|event_type| event_type["name"].clone()),
    )
}

#[test]
fn roles_the_timeline_and_ocel_agree_on_the_event_types_of_every_fixture() {
    let mut disagreements = Vec::new();
    let mut with_events = 0;
    for fixture in FIXTURES {
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), Provider::File);
        fixture.build(&runtime);
        let head = runtime.head().expect("the head").expect("seeded").revision;
        for revision in 0..=head.get() {
            let index =
                Index::load(&runtime, Some(RevisionNumber::new(revision))).expect("the revision");
            let rule: BTreeSet<String> = index
                .event_types()
                .iter()
                .map(ToString::to_string)
                .collect();
            let nodes: Vec<NodeId> = index.loaded().graph.nodes.keys().copied().collect();
            let roles = roles_events(&index);
            let overview = overview_events(&index);
            let ocel = ocel_events(&index);
            let timeline = timeline_events(&index, &nodes);
            with_events += usize::from(!rule.is_empty());
            if roles != rule || overview != rule || ocel != rule || !timeline.is_subset(&rule) {
                disagreements.push(format!(
                    "{fixture:?} at revision {revision}: rule {rule:?}, /roles {roles:?}, \
                     overview {overview:?}, ocel {ocel:?}, timeline {timeline:?}"
                ));
            }
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} disagreements:\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
    assert!(with_events > 0, "some fixture revision has an event type");
}
