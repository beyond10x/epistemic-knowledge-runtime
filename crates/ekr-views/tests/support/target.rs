//! The ESS conformance target over `ekr_views`' eight reads, on one native provider:
//! `ekr_views::project` for `ProjectGraph`, `ekr_views::report_quality` for
//! `ReportStoreQuality`, and an [`ekr_views::Index`] of the requested revision for
//! `ProjectOverview`, `ExpandNeighbourhood`, `DescribeNode`, `SearchNodes`, `ProjectTimeline` and
//! `ChangesSince`.
//!
//! * **Isolation.** Every scenario gets a fresh directory below the caller's work directory, and
//!   every store a scenario names is a fresh provider root inside it.
//! * **Stores.** A `store` input names a [`Fixture`]; the first command naming it in a scenario
//!   builds it through the real kernel handlers, and every later command reads the same store.
//! * **Bounds.** A bounded read's request is built from its input before the store is touched,
//!   so a broken bound is answered by the request type's own refusal and the store is not read.
//! * **External outcomes.** A control establishes the state its branch declares and never selects
//!   the reported outcome: `not-found` builds the named store as the seed alone, so the requested
//!   revision does not exist; `not-seeded` opens the named store's provider without seeding it;
//!   and `node-not-found` builds it as `schema-evolution` — revision 1 exists, and
//!   [`fixtures::DESCRIBED`], the node the generated scenarios name, does not. An expansion that
//!   names no seed is given that node as its one seed, which is what makes a seed unknown; the
//!   generated `ExpandNeighbourhood` scenarios send `seeds: []`. The reads then answer whatever
//!   they answer.
//! * **Observations.** A command reports the event built from the summary the read returned, and
//!   every event the provider log gained while it ran, by its logged name — so a read that wrote
//!   anything is caught by the suite's `expect_no_event` steps.
//! * **Responses.** None: no admitted views scenario observes a command response. A document
//!   reaches the suite through its event's hash and counts.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::PathBuf;

use ekr_core::{NodeId, RevisionNumber, TypeId};
use ekr_kernel::Runtime;
use ekr_views::{
    BucketWidth, ChangesError, ChangesListed, ChangesRequest, ExpandRequest, GraphOverviewed,
    GraphProjected, Index, LimitExceeded, NeighbourhoodExpanded, NodeDescribed, NodesSearched,
    OverviewRequest, ProjectError, QueryError, SearchRequest, SinceKind, StoreQualityReported,
    SubjectsTimelined, TimelineRequest,
};
use ess_conformance::target::{
    ConformanceTarget, DeclaredErrorValue, EventObservationRequest, ExternalOutcomeControl,
    ImplementationIdentity, ObservedEvent, RedeliveryRequest, ScenarioContext,
    SemanticCommandRequest, SemanticCommandResult, SemanticViewRequest, SemanticViewResult,
    TargetError,
};
use ess_primitives::facts::Number;
use ess_primitives::node::Node;

use super::fixtures::{self, Fixture, Provider};

const PROJECT_GRAPH: &str = "ekr.views.ProjectGraph";
const PROJECT_OVERVIEW: &str = "ekr.views.ProjectOverview";
const EXPAND_NEIGHBOURHOOD: &str = "ekr.views.ExpandNeighbourhood";
const DESCRIBE_NODE: &str = "ekr.views.DescribeNode";
const SEARCH_NODES: &str = "ekr.views.SearchNodes";
const PROJECT_TIMELINE: &str = "ekr.views.ProjectTimeline";
const CHANGES_SINCE: &str = "ekr.views.ChangesSince";
const REPORT_STORE_QUALITY: &str = "ekr.views.ReportStoreQuality";
const COMMANDS: [&str; 8] = [
    PROJECT_GRAPH,
    PROJECT_OVERVIEW,
    EXPAND_NEIGHBOURHOOD,
    DESCRIBE_NODE,
    SEARCH_NODES,
    PROJECT_TIMELINE,
    CHANGES_SINCE,
    REPORT_STORE_QUALITY,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    RevisionAbsent,
    Unseeded,
    NodeAbsent,
}

struct Scenario {
    directory: PathBuf,
    control: Option<Control>,
    stores: BTreeMap<String, Runtime>,
    events: Vec<ObservedEvent>,
}

/// One projection the target returned: which store, which revision, and its summary.
#[derive(Clone, Debug)]
pub struct Answered {
    pub store: String,
    pub summary: GraphProjected,
}

pub struct ViewsTarget {
    provider: Provider,
    work: PathBuf,
    opened: Cell<u64>,
    scenario: RefCell<Option<Scenario>>,
    answered: RefCell<Vec<Answered>>,
}

/// One command's input, read and — for the bounded reads — bounded.
enum Read {
    Graph,
    Overview(Result<OverviewRequest, LimitExceeded>),
    Expand(Result<ExpandRequest, LimitExceeded>),
    Describe(NodeId),
    Search(Result<SearchRequest, LimitExceeded>),
    Timeline(Result<TimelineRequest, LimitExceeded>),
    Changes(Result<ChangesRequest, ChangesError>),
    Quality,
}

fn unavailable(operation: &str, detail: impl std::fmt::Display) -> TargetError {
    TargetError::unavailable(operation, detail.to_string())
}

fn integer(value: u64) -> Result<Node, TargetError> {
    i64::try_from(value)
        .map(|exact| Node::Number(Number::from(exact)))
        .map_err(|_| unavailable("projecting an integer", format!("{value} exceeds i64")))
}

fn signed(value: i64) -> Node {
    Node::Number(Number::from(value))
}

fn observed(name: &str, fields: Vec<(&str, Node)>) -> Result<ObservedEvent, TargetError> {
    let name = name
        .parse()
        .map_err(|e| unavailable(&format!("naming {name}"), e))?;
    Ok(fields
        .into_iter()
        .fold(ObservedEvent::new(name), |event, (field, value)| {
            event.with(field, value)
        }))
}

fn counts(fields: &[(&'static str, u64)]) -> Result<Vec<(&'static str, Node)>, TargetError> {
    fields
        .iter()
        .map(|(field, value)| integer(*value).map(|node| (*field, node)))
        .collect()
}

fn graph_projected(summary: &GraphProjected) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("revision", summary.revision),
        ("nodes", summary.nodes),
        ("edges", summary.edges),
        ("assertions", summary.assertions),
        ("evidence", summary.evidence),
        ("schema_versions", summary.schema_versions),
        ("revisions", summary.revisions),
        ("transactions", summary.transactions),
        ("node_types", summary.node_types),
        ("edge_types", summary.edge_types),
        ("properties", summary.properties),
        ("edge_assertions", summary.edge_assertions),
        ("retracted_assertions", summary.retracted_assertions),
        ("retained_evidence", summary.retained_evidence),
    ])?;
    fields.push((
        "projection_hash",
        Node::Text(summary.projection_hash.clone()),
    ));
    observed("ekr.views.GraphProjected", fields)
}

fn graph_overviewed(summary: &GraphOverviewed) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("revision", summary.revision),
        ("nodes", summary.nodes),
        ("edges", summary.edges),
        ("assertions", summary.assertions),
        ("evidence", summary.evidence),
        ("node_types", summary.node_types),
        ("edge_types", summary.edge_types),
        ("schema_versions", summary.schema_versions),
        ("revisions", summary.revisions),
        ("added", summary.added),
        ("removed", summary.removed),
        ("unrecorded_nodes", summary.unrecorded_nodes),
        ("unrecorded_edges", summary.unrecorded_edges),
        ("revision_zero_nodes", summary.revision_zero_nodes),
        ("revision_zero_edges", summary.revision_zero_edges),
        ("revision_zero_assertions", summary.revision_zero_assertions),
        ("event_types", summary.event_types),
        ("bucket_ms", summary.bucket_ms),
        ("timeline_buckets", summary.timeline_buckets),
        ("dated_assertions", summary.dated_assertions),
        ("undated_assertions", summary.undated_assertions),
        ("top", summary.top),
    ])?;
    if let Some(observation) = summary.observation_type {
        fields.push(("observation_type", Node::Text(observation.to_string())));
    }
    fields.push(("overview_hash", Node::Text(summary.overview_hash.clone())));
    observed("ekr.views.GraphOverviewed", fields)
}

fn neighbourhood_expanded(summary: &NeighbourhoodExpanded) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("revision", summary.revision),
        ("seeds", summary.seeds),
        ("depth", summary.depth),
        ("after", summary.after),
        ("nodes", summary.nodes),
        ("edges", summary.edges),
        ("node_total", summary.node_total),
        ("edge_total", summary.edge_total),
        ("remaining", summary.remaining),
    ])?;
    fields.push(("slice_hash", Node::Text(summary.slice_hash.clone())));
    observed("ekr.views.NeighbourhoodExpanded", fields)
}

fn node_described(summary: &NodeDescribed) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("revision", summary.revision),
        ("assertions", summary.assertions),
        ("referencing", summary.referencing),
        ("edges", summary.edges),
        ("neighbours", summary.neighbours),
    ])?;
    fields.push(("node", Node::Text(summary.node.to_string())));
    fields.push(("detail_hash", Node::Text(summary.detail_hash.clone())));
    observed("ekr.views.NodeDescribed", fields)
}

fn nodes_searched(summary: &NodesSearched) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("revision", summary.revision),
        ("matches", summary.matches),
        ("total", summary.total),
        ("exact_total", summary.exact_total),
    ])?;
    fields.push(("text", Node::Text(summary.text.clone())));
    if let Some(first) = summary.first_match {
        fields.push(("first_match", Node::Text(first.to_string())));
    }
    fields.push(("matches_hash", Node::Text(summary.matches_hash.clone())));
    observed("ekr.views.NodesSearched", fields)
}

fn subjects_timelined(summary: &SubjectsTimelined) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("revision", summary.revision),
        ("hops", summary.hops),
        ("bucket_ms", summary.bucket_ms),
        ("row_types", summary.row_types),
        ("subjects", summary.subjects),
        ("active", summary.active),
        ("rows", summary.rows),
        ("events", summary.events),
        ("row_events", summary.row_events),
        ("cells", summary.cells),
        ("strip", summary.strip),
        ("listed_events", summary.listed_events),
    ])?;
    if let Some(row_type) = summary.row_type {
        fields.push(("row_type", Node::Text(row_type.to_string())));
    }
    if let Some(first) = summary.first_row {
        fields.push(("first_row", Node::Text(first.to_string())));
    }
    fields.push(("timeline_hash", Node::Text(summary.timeline_hash.clone())));
    observed("ekr.views.SubjectsTimelined", fields)
}

fn changes_listed(summary: &ChangesListed) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("revision", summary.revision),
        ("after", summary.after),
        ("changes", summary.changes),
        ("total", summary.total),
        ("remaining", summary.remaining),
        ("nodes_created", summary.nodes_created),
        ("edges_created", summary.edges_created),
        ("assertions_added", summary.assertions_added),
        ("assertions_superseded", summary.assertions_superseded),
        ("assertions_retracted", summary.assertions_retracted),
    ])?;
    if let Some(first) = summary.first_revision {
        fields.push(("first_revision", integer(first)?));
    }
    if let Some(last) = summary.last_revision {
        fields.push(("last_revision", integer(last)?));
    }
    fields.push(("changes_hash", Node::Text(summary.changes_hash.clone())));
    observed("ekr.views.ChangesListed", fields)
}

fn store_quality_reported(summary: &StoreQualityReported) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("revision", summary.revision),
        ("active_assertions", summary.active_assertions),
        ("with_evidence", summary.with_evidence),
        ("with_item_evidence", summary.with_item_evidence),
        ("properties", summary.properties),
        ("constrained_properties", summary.constrained_properties),
        ("shared_names", summary.shared_names),
        ("sharing_nodes", summary.sharing_nodes),
    ])?;
    fields.push(("quality_hash", Node::Text(summary.quality_hash.clone())));
    observed("ekr.views.StoreQualityReported", fields)
}

/// `ChangesSince`'s input, bounded: its since kind by name, its since, and its page.
fn changes_request(
    request: &SemanticCommandRequest,
) -> Result<Result<ChangesRequest, ChangesError>, TargetError> {
    let kind = match text(request, "since_kind")?.as_str() {
        "Revision" => SinceKind::Revision,
        "ValidTime" => SinceKind::ValidTime,
        "TransactionTime" => SinceKind::TransactionTime,
        other => {
            return Err(unavailable(
                "reading `since_kind`",
                format!("{other} is no since kind"),
            ))
        }
    };
    Ok(ChangesRequest::new(
        kind,
        required_integer(request, "since")?,
        optional_integer(request, "limit")?,
        optional_integer(request, "after")?,
    ))
}

/// A `ChangesSince` that answered nothing, as the outcome and error it names.
fn changes_refused(
    command: &str,
    refusal: ChangesError,
) -> Result<SemanticCommandResult, TargetError> {
    match refusal {
        ChangesError::SinceMalformed(malformed) => {
            Ok(
                SemanticCommandResult::took(outcome_ref(command, "since-malformed")?).with_error(
                    error("ekr.views.SinceMalformed")?
                        .with("kind", Node::Text(malformed.kind.name().to_owned()))
                        .with("requested", signed(malformed.requested)),
                ),
            )
        }
        ChangesError::LimitExceeded(refusal) => limit_exceeded(command, &refusal),
        ChangesError::Project(error) => refused(command, QueryError::Project(error)),
    }
}

/// An optional text input: `None` when absent or null.
fn optional_text(
    request: &SemanticCommandRequest,
    field: &str,
) -> Result<Option<String>, TargetError> {
    match request.input.get(field) {
        None | Some(Node::Null) => Ok(None),
        Some(Node::Text(text)) => Ok(Some(text.clone())),
        Some(other) => Err(unavailable(
            &format!("reading `{field}`"),
            format!("{} is not a text", other.type_name()),
        )),
    }
}

/// `ProjectTimeline`'s input, bounded.
fn timeline_request(
    request: &SemanticCommandRequest,
) -> Result<Result<TimelineRequest, LimitExceeded>, TargetError> {
    let row_type = optional_text(request, "row_type")?
        .map(|text| {
            text.parse::<TypeId>()
                .map_err(|e| unavailable("reading `row_type`", format!("{text}: {e}")))
        })
        .transpose()?;
    let bucket = match optional_text(request, "bucket")?.as_deref() {
        None => None,
        Some("Day") => Some(BucketWidth::Day),
        Some("Week") => Some(BucketWidth::Week),
        Some(other) => {
            return Err(unavailable(
                "reading `bucket`",
                format!("{other} is no bucket width"),
            ))
        }
    };
    let subject = optional_text(request, "subject")?
        .map(|text| node_id(&text, "subject"))
        .transpose()?;
    Ok(TimelineRequest::new(
        row_type,
        required_integer(request, "hops")?,
        required_integer(request, "limit")?,
        bucket,
        subject,
    ))
}

fn outcome_ref(
    command: &str,
    outcome: &str,
) -> Result<ess_conformance::scenario::OutcomeRef, TargetError> {
    serde_json::from_value(serde_json::json!({ "command": command, "outcome": outcome }))
        .map_err(|e| unavailable("naming a declared outcome", format!("{outcome}: {e}")))
}

fn error(name: &str) -> Result<DeclaredErrorValue, TargetError> {
    name.parse()
        .map(DeclaredErrorValue::new)
        .map_err(|e| unavailable("naming a declared error", format!("{name}: {e}")))
}

/// An optional integer input: `None` when absent or null.
fn optional_integer(
    request: &SemanticCommandRequest,
    field: &str,
) -> Result<Option<i64>, TargetError> {
    match request.input.get(field) {
        None | Some(Node::Null) => Ok(None),
        Some(Node::Number(number)) => number.as_i64().map(Some).ok_or_else(|| {
            unavailable(
                &format!("reading `{field}`"),
                format!("{number} is not an integer"),
            )
        }),
        Some(other) => Err(unavailable(
            &format!("reading `{field}`"),
            format!("{} is not an integer", other.type_name()),
        )),
    }
}

fn required_integer(request: &SemanticCommandRequest, field: &str) -> Result<i64, TargetError> {
    optional_integer(request, field)?
        .ok_or_else(|| unavailable(&format!("reading `{field}`"), "the input is absent"))
}

fn text(request: &SemanticCommandRequest, field: &str) -> Result<String, TargetError> {
    match request.input.get(field) {
        Some(Node::Text(text)) => Ok(text.clone()),
        _ => Err(unavailable(&format!("reading `{field}`"), "not a text")),
    }
}

fn node_id(text: &str, field: &str) -> Result<NodeId, TargetError> {
    text.parse()
        .map_err(|e| unavailable(&format!("reading `{field}`"), format!("{text}: {e}")))
}

fn revision_input(request: &SemanticCommandRequest) -> Result<Option<RevisionNumber>, TargetError> {
    optional_integer(request, "at")?
        .map(|at| {
            u64::try_from(at)
                .map(RevisionNumber::new)
                .map_err(|_| unavailable("reading `at`", format!("{at} is not a revision")))
        })
        .transpose()
}

/// Reads `request`'s command input; a bounded read's request is built, and so bounded, here.
fn read(request: &SemanticCommandRequest, command: &str) -> Result<Read, TargetError> {
    Ok(match command {
        PROJECT_OVERVIEW => {
            Read::Overview(OverviewRequest::new(optional_integer(request, "limit")?))
        }
        EXPAND_NEIGHBOURHOOD => {
            let seeds = match request.input.get("seeds") {
                Some(Node::Seq(seeds)) => seeds
                    .iter()
                    .map(|seed| match seed {
                        Node::Text(text) => node_id(text, "seeds"),
                        _ => Err(unavailable("reading `seeds`", "a seed is not a text")),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err(unavailable("reading `seeds`", "not a list")),
            };
            Read::Expand(ExpandRequest::new(
                seeds,
                required_integer(request, "depth")?,
                required_integer(request, "limit")?,
                optional_integer(request, "edge_limit")?,
                optional_integer(request, "after")?,
            ))
        }
        DESCRIBE_NODE => Read::Describe(node_id(&text(request, "node")?, "node")?),
        SEARCH_NODES => Read::Search(SearchRequest::new(
            text(request, "text")?,
            required_integer(request, "limit")?,
        )),
        PROJECT_TIMELINE => Read::Timeline(timeline_request(request)?),
        CHANGES_SINCE => Read::Changes(changes_request(request)?),
        REPORT_STORE_QUALITY => Read::Quality,
        _ => Read::Graph,
    })
}

fn limit_exceeded(
    command: &str,
    refusal: &LimitExceeded,
) -> Result<SemanticCommandResult, TargetError> {
    let mut value = error("ekr.views.LimitExceeded")?
        .with("parameter", Node::Text(refusal.parameter.to_owned()))
        .with("requested", signed(refusal.requested))
        .with("minimum", signed(refusal.minimum));
    if let Some(maximum) = refusal.maximum {
        value = value.with("maximum", signed(maximum));
    }
    Ok(SemanticCommandResult::took(outcome_ref(command, "limit-exceeded")?).with_error(value))
}

fn refused(command: &str, refusal: QueryError) -> Result<SemanticCommandResult, TargetError> {
    match refusal {
        QueryError::LimitExceeded(refusal) => limit_exceeded(command, &refusal),
        QueryError::NodeNotFound { node, revision } => Ok(SemanticCommandResult::took(
            outcome_ref(command, "node-not-found")?,
        )
        .with_error(
            error("ekr.views.NodeNotFound")?
                .with("node", Node::Text(node.to_string()))
                .with("revision", integer(revision.get())?),
        )),
        QueryError::Project(ProjectError::RevisionNotFound { requested, head }) => Ok(
            SemanticCommandResult::took(outcome_ref(command, "not-found")?).with_error(
                error("ekr.views.RevisionNotFound")?
                    .with("requested", integer(requested.get())?)
                    .with("head", integer(head.get())?),
            ),
        ),
        QueryError::Project(ProjectError::NotSeeded { requested }) => {
            let mut refusal = error("ekr.views.NotSeeded")?;
            if let Some(requested) = requested {
                refusal = refusal.with("requested", integer(requested.get())?);
            }
            Ok(
                SemanticCommandResult::took(outcome_ref(command, "not-seeded")?)
                    .with_error(refusal),
            )
        }
        QueryError::Project(other) => Err(unavailable(&format!("answering `{command}`"), other)),
    }
}

/// Answers one read against `runtime`, or refuses it as the read refused.
fn answer(
    runtime: &Runtime,
    command: &str,
    at: Option<RevisionNumber>,
    read: Read,
) -> Result<(SemanticCommandResult, Option<GraphProjected>), TargetError> {
    let took =
        |outcome: &str, event: ObservedEvent| -> Result<SemanticCommandResult, TargetError> {
            Ok(SemanticCommandResult::took(outcome_ref(command, outcome)?).emitting(event))
        };
    if let Read::Graph = read {
        return match ekr_views::project(runtime, at) {
            Ok(rendered) => Ok((
                took("projected", graph_projected(&rendered.summary)?)?,
                Some(rendered.summary),
            )),
            Err(error) => Ok((refused(command, QueryError::Project(error))?, None)),
        };
    }
    if let Read::Quality = read {
        return match ekr_views::report_quality(runtime, at) {
            Ok(answer) => Ok((
                took("reported", store_quality_reported(&answer.summary)?)?,
                None,
            )),
            Err(error) => Ok((refused(command, QueryError::Project(error))?, None)),
        };
    }
    if let Read::Changes(request) = read {
        // The since and the bounds first: a broken one is refused before the store is read.
        let listed = request.and_then(|request| {
            let index = Index::load(runtime, at)?;
            index.changes(runtime, &request)
        });
        return match listed {
            Ok(answer) => Ok((took("listed", changes_listed(&answer.summary)?)?, None)),
            Err(refusal) => Ok((changes_refused(command, refusal)?, None)),
        };
    }
    let result = (|| -> Result<Result<SemanticCommandResult, TargetError>, QueryError> {
        // The bound first: a broken one is refused before the store is read.
        match read {
            Read::Graph | Read::Changes(_) | Read::Quality => unreachable!("answered above"),
            Read::Overview(request) => {
                let request = request?;
                let index = Index::load(runtime, at)?;
                let answer = index.overview(&request)?;
                Ok(graph_overviewed(&answer.summary).and_then(|e| took("overviewed", e)))
            }
            Read::Expand(request) => {
                let request = request?;
                let index = Index::load(runtime, at)?;
                let answer = index.expand(&request)?;
                Ok(neighbourhood_expanded(&answer.summary).and_then(|e| took("expanded", e)))
            }
            Read::Describe(node) => {
                let index = Index::load(runtime, at)?;
                let answer = index.describe(node)?;
                Ok(node_described(&answer.summary).and_then(|e| took("described", e)))
            }
            Read::Search(request) => {
                let request = request?;
                let index = Index::load(runtime, at)?;
                let answer = index.search(&request)?;
                Ok(nodes_searched(&answer.summary).and_then(|e| took("searched", e)))
            }
            Read::Timeline(request) => {
                let request = request?;
                let index = Index::load(runtime, at)?;
                let answer = index.timeline(&request)?;
                Ok(subjects_timelined(&answer.summary).and_then(|e| took("timelined", e)))
            }
        }
    })();
    match result {
        Ok(answered) => Ok((answered?, None)),
        Err(refusal) => Ok((refused(command, refusal)?, None)),
    }
}

impl ViewsTarget {
    pub fn new(provider: Provider, work: PathBuf) -> Self {
        Self {
            provider,
            work,
            opened: Cell::new(0),
            scenario: RefCell::new(None),
            answered: RefCell::new(Vec::new()),
        }
    }

    /// Every projection this target returned, in order.
    pub fn answered(&self) -> Vec<Answered> {
        self.answered.borrow().clone()
    }
}

impl ConformanceTarget for ViewsTarget {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            format!("ekr-views ({} provider)", self.provider.name()),
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn begin_scenario(&self, _scenario: &ScenarioContext) -> Result<(), TargetError> {
        let number = self.opened.get() + 1;
        self.opened.set(number);
        let directory = self.work.join(format!("scenario-{number:04}"));
        if directory.exists() {
            return Err(unavailable(
                "opening an isolated scenario",
                format!("{} already exists", directory.display()),
            ));
        }
        std::fs::create_dir_all(&directory)
            .map_err(|e| unavailable("opening an isolated scenario", e))?;
        *self.scenario.borrow_mut() = Some(Scenario {
            directory,
            control: None,
            stores: BTreeMap::new(),
            events: Vec::new(),
        });
        Ok(())
    }

    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let command = request.command.to_string();
        let Some(command) = COMMANDS.iter().copied().find(|known| *known == command) else {
            return Err(TargetError::unsupported(
                format!("executing `{}`", request.command),
                "the views target answers the ekr.views commands only",
            ));
        };
        let store = match request.input.get("store") {
            Some(Node::Text(store)) => store.clone(),
            _ => return Err(unavailable("reading `store`", "not a store location")),
        };
        let at = revision_input(&request)?;
        let mut read = read(&request, command)?;
        let mut guard = self.scenario.borrow_mut();
        let scenario = guard
            .as_mut()
            .ok_or_else(|| unavailable("executing a command", "no scenario is open"))?;
        let control = scenario.control.take();
        if control.is_some() && scenario.stores.contains_key(&store) {
            return Err(unavailable(
                "establishing an external outcome",
                format!("the store `{store}` is already built in this scenario"),
            ));
        }
        if control == Some(Control::NodeAbsent) {
            if let Read::Expand(Ok(expansion)) = &read {
                if expansion.seeds().is_empty() {
                    read = Read::Expand(ExpandRequest::new(
                        vec![fixtures::id(fixtures::DESCRIBED)],
                        i64::try_from(expansion.depth()).unwrap_or(i64::MAX),
                        i64::try_from(expansion.limit()).unwrap_or(i64::MAX),
                        Some(i64::try_from(expansion.edge_limit()).unwrap_or(i64::MAX)),
                        Some(i64::try_from(expansion.after()).unwrap_or(i64::MAX)),
                    ));
                }
            }
        }
        if !scenario.stores.contains_key(&store) {
            let fixture = Fixture::named(&store).ok_or_else(|| {
                TargetError::unsupported(
                    format!("opening the store `{store}`"),
                    "no fixture of that name",
                )
            })?;
            let root = scenario.directory.join(&store);
            let runtime = fixtures::open(&root, self.provider);
            match control {
                Some(Control::Unseeded) => {}
                Some(Control::RevisionAbsent) => Fixture::SeedOnly.build(&runtime),
                Some(Control::NodeAbsent) => Fixture::SchemaEvolution.build(&runtime),
                None => fixture.build(&runtime),
            }
            scenario.stores.insert(store.clone(), runtime);
        }
        let runtime = &scenario.stores[&store];
        let before = runtime
            .published_events()
            .map_err(|e| unavailable("reading the provider log", e))?
            .len();
        let (mut result, projected) = answer(runtime, command, at, read)?;
        let gained = runtime
            .published_events()
            .map_err(|e| unavailable("reading the provider log", e))?;
        if let Some(summary) = projected {
            self.answered.borrow_mut().push(Answered {
                store: store.clone(),
                summary,
            });
        }
        for written in gained.iter().skip(before) {
            let name = written
                .name
                .parse()
                .map_err(|e| unavailable("naming a logged event", e))?;
            result = result.emitting(ObservedEvent::new(name).at(written.position));
        }
        scenario.events.extend(result.direct_events.iter().cloned());
        Ok(result)
    }

    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        Err(TargetError::unsupported(
            format!("reading `{}`", request.view),
            "ekr.views declares no view",
        ))
    }

    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        let guard = self.scenario.borrow();
        let scenario = guard
            .as_ref()
            .ok_or_else(|| unavailable("observing events", "no scenario is open"))?;
        Ok(scenario
            .events
            .iter()
            .filter(|event| event.event == request.event)
            .cloned()
            .collect())
    }

    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), TargetError> {
        let command = request.force.command.to_string();
        let outcome = request.force.outcome.to_string();
        let control = match (command.as_str(), outcome.as_str()) {
            (command, "not-found") if COMMANDS.contains(&command) => Control::RevisionAbsent,
            (command, "not-seeded") if COMMANDS.contains(&command) => Control::Unseeded,
            (EXPAND_NEIGHBOURHOOD | DESCRIBE_NODE, "node-not-found") => Control::NodeAbsent,
            _ => {
                return Err(TargetError::unsupported(
                    format!("establishing `{}`", request.force),
                    "the views target knows no precondition for that branch",
                ))
            }
        };
        let mut guard = self.scenario.borrow_mut();
        let scenario = guard
            .as_mut()
            .ok_or_else(|| unavailable("configuring an external outcome", "no scenario is open"))?;
        scenario.control = Some(control);
        Ok(())
    }

    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        Err(TargetError::unsupported(
            format!("redelivering `{}`", request.event),
            "ekr.views declares no binding",
        ))
    }

    fn end_scenario(&self, _scenario: &ScenarioContext) -> Result<(), TargetError> {
        self.scenario.borrow_mut().take();
        Ok(())
    }
}
