//! The ESS conformance target over `ekr_views`' twelve commands, on one native provider:
//! `ekr_views::project` for `ProjectGraph`, `ekr_views::report_quality` for `ReportStoreQuality`,
//! `ekr_views::export_ocel` for `ExportOcel`, `ekr_views::find_code_names` for `FindCodeNames`, whose
//! `sources` input is the list of `{path, text}` it answers for, and an [`ekr_views::Index`] of the
//! requested revision for `ProjectOverview`, `ExpandNeighbourhood`, `DescribeNode`, `SearchNodes`,
//! `ProjectTimeline` and `ChangesSince`; and the fact-quality pair, `ekr_views::draw_sample` for
//! `DrawFactSample` and `ekr_views::report_fact_quality` for `ReportFactQuality`, which reads no
//! store and so names none: its `judgements` and `sample` are read as the `ekr.fact-judgements/1`
//! document a host reads them from.
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
//!   `node-not-found` and `event-type-not-found` build it as `schema-evolution` — revision 1
//!   exists, and [`fixtures::DESCRIBED`], the node the generated scenarios name, does not, nor
//!   does a node type named [`fixtures::UNDECLARED_TYPE_NAME`]. An expansion that names no seed is
//!   given that node as its one seed, and an export that names no event type that name as its
//!   one, which is what makes a seed or a name unknown; the generated `ExpandNeighbourhood` and
//!   `ExportOcel` scenarios send `seeds: []` and `events: []`. `judged-twice` judges the first
//!   judgement's assertion a second time, or, where the request judges none (the generated
//!   scenario sends `judgements: []`), judges one fixture assertion twice. The reads then answer
//!   whatever they answer.
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
    BucketWidth, ChangesError, ChangesListed, ChangesRequest, CodeNamesFound, ExpandRequest,
    FactJudgements, FactQualityError, FactQualityReported, FactSampleDrawn, GraphOverviewed,
    GraphProjected, Index, Judgement, LimitExceeded, NeighbourhoodExpanded, NodeDescribed,
    NodesSearched, OcelError, OcelExported, OverviewRequest, ProjectError, QueryError,
    SampleRequest, SearchRequest, SinceKind, SourceText, StoreQualityReported, SubjectsTimelined,
    TimelineRequest, Verdict,
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
const FIND_CODE_NAMES: &str = "ekr.views.FindCodeNames";
const REPORT_STORE_QUALITY: &str = "ekr.views.ReportStoreQuality";
const EXPORT_OCEL: &str = "ekr.views.ExportOcel";
const DRAW_FACT_SAMPLE: &str = "ekr.views.DrawFactSample";
const REPORT_FACT_QUALITY: &str = "ekr.views.ReportFactQuality";
const COMMANDS: [&str; 12] = [
    PROJECT_GRAPH,
    PROJECT_OVERVIEW,
    EXPAND_NEIGHBOURHOOD,
    DESCRIBE_NODE,
    SEARCH_NODES,
    PROJECT_TIMELINE,
    CHANGES_SINCE,
    FIND_CODE_NAMES,
    REPORT_STORE_QUALITY,
    EXPORT_OCEL,
    DRAW_FACT_SAMPLE,
    REPORT_FACT_QUALITY,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    RevisionAbsent,
    Unseeded,
    NodeAbsent,
    EventTypeAbsent,
    EventTimeInvalid,
    JudgedTwice,
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
    CodeNames(Vec<SourceText>, ekr_views::CodeNameMode),
    Quality,
    Ocel {
        events: Vec<String>,
        times: Vec<String>,
    },
    Sample(Result<SampleRequest, LimitExceeded>),
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
        ("widened", summary.widened),
        ("modified", summary.modified),
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
        ("evidence_added", summary.evidence_added),
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

fn code_names_found(summary: &CodeNamesFound) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("revision", summary.revision),
        ("files", summary.files),
        ("literals", summary.literals),
        ("exempt", summary.exempt),
        ("findings", summary.findings),
        ("runtime_word_findings", summary.runtime_word_findings),
        ("node_types", summary.node_types),
        ("edge_types", summary.edge_types),
        ("properties", summary.properties),
        ("canonical_names", summary.canonical_names),
        ("aliases", summary.aliases),
    ])?;
    if let Some(file) = &summary.first_file {
        fields.push(("first_file", Node::Text(file.clone())));
    }
    if let Some(line) = summary.first_line {
        fields.push(("first_line", integer(line)?));
    }
    fields.push((
        "code_names_hash",
        Node::Text(summary.code_names_hash.clone()),
    ));
    observed("ekr.views.CodeNamesFound", fields)
}

/// `FindCodeNames`' `sources`: a list of `{path, text}`.
fn sources(request: &SemanticCommandRequest) -> Result<Vec<SourceText>, TargetError> {
    let Some(Node::Seq(sources)) = request.input.get("sources") else {
        return Err(unavailable("reading `sources`", "not a list"));
    };
    sources
        .iter()
        .map(|source| {
            let field = |name: &str| match source {
                Node::Map(fields) => match fields.get(name) {
                    Some(Node::Text(text)) => Ok(text.clone()),
                    _ => Err(unavailable(
                        "reading `sources`",
                        format!("a source's `{name}` is not a text"),
                    )),
                },
                _ => Err(unavailable("reading `sources`", "a source is not a map")),
            };
            Ok(SourceText {
                path: field("path")?,
                text: field("text")?,
            })
        })
        .collect()
}

/// `ExportOcel`'s `events`: the node type names, in request order.
fn event_names(request: &SemanticCommandRequest) -> Result<Vec<String>, TargetError> {
    let Some(Node::Seq(names)) = request.input.get("events") else {
        return Err(unavailable("reading `events`", "not a list"));
    };
    names
        .iter()
        .map(|name| match name {
            Node::Text(text) => Ok(text.clone()),
            _ => Err(unavailable("reading `events`", "a name is not a text")),
        })
        .collect()
}

fn event_times(request: &SemanticCommandRequest) -> Result<Vec<String>, TargetError> {
    match request.input.get("event_time") {
        None | Some(Node::Null) => Ok(Vec::new()),
        Some(Node::Seq(names)) => names
            .iter()
            .map(|name| match name {
                Node::Text(text) => Ok(text.clone()),
                _ => Err(unavailable("reading event_time", "selector is not text")),
            })
            .collect(),
        _ => Err(unavailable("reading event_time", "not a list")),
    }
}

fn store_quality_reported(summary: &StoreQualityReported) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("revision", summary.revision),
        ("active_assertions", summary.active_assertions),
        ("with_evidence", summary.with_evidence),
        ("with_item_evidence", summary.with_item_evidence),
        ("with_seed_evidence", summary.with_seed_evidence),
        ("properties", summary.properties),
        ("constrained_properties", summary.constrained_properties),
        ("constrained_types", summary.constrained_types),
        ("shared_names", summary.shared_names),
        ("sharing_nodes", summary.sharing_nodes),
    ])?;
    fields.push(("quality_hash", Node::Text(summary.quality_hash.clone())));
    observed("ekr.views.StoreQualityReported", fields)
}

fn ocel_exported(summary: &OcelExported) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("revision", summary.revision),
        ("event_types", summary.event_types),
        ("object_types", summary.object_types),
        ("events", summary.events),
        ("objects", summary.objects),
        (
            "event_object_relationships",
            summary.event_object_relationships,
        ),
        (
            "object_object_relationships",
            summary.object_object_relationships,
        ),
        ("edges_between_events", summary.edges_between_events),
        ("undated_events", summary.undated_events),
        ("edges_of_undated_events", summary.edges_of_undated_events),
        ("parallel_edges_merged", summary.parallel_edges_merged),
        (
            "attribute_values_out_of_range",
            summary.attribute_values_out_of_range,
        ),
    ])?;
    fields.push(("ocel_hash", Node::Text(summary.ocel_hash.clone())));
    observed("ekr.views.OcelExported", fields)
}

fn fact_sample_drawn(summary: &FactSampleDrawn) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("revision", summary.revision),
        ("size", summary.size),
        ("population", summary.population),
        ("drawn", summary.drawn),
        ("evidence", summary.evidence),
    ])?;
    fields.push(("seed", signed(summary.seed)));
    if let Some(first) = summary.first_assertion {
        fields.push(("first_assertion", Node::Text(first.to_string())));
    }
    fields.push(("sample_hash", Node::Text(summary.sample_hash.clone())));
    observed("ekr.views.FactSampleDrawn", fields)
}

fn fact_quality_reported(summary: &FactQualityReported) -> Result<ObservedEvent, TargetError> {
    let mut fields = counts(&[
        ("confidence", summary.confidence),
        ("judged", summary.judged),
        ("passed", summary.passed),
        ("failed", summary.failed),
    ])?;
    for (field, value) in [
        ("rate_bp", summary.rate_bp),
        ("lower_bp", summary.lower_bp),
        ("upper_bp", summary.upper_bp),
    ] {
        if let Some(value) = value {
            fields.push((field, integer(value)?));
        }
    }
    fields.push((
        "fact_quality_hash",
        Node::Text(summary.fact_quality_hash.clone()),
    ));
    observed("ekr.views.FactQualityReported", fields)
}

/// `DrawFactSample`'s input, bounded: its seed, size and optional type id.
fn sample_request(
    request: &SemanticCommandRequest,
) -> Result<Result<SampleRequest, LimitExceeded>, TargetError> {
    let type_id = optional_text(request, "type")?
        .map(|text| {
            text.parse::<TypeId>()
                .map_err(|e| unavailable("reading `type`", format!("{text}: {e}")))
        })
        .transpose()?;
    Ok(SampleRequest::new(
        required_integer(request, "seed")?,
        required_integer(request, "size")?,
        type_id,
    ))
}

/// `ReportFactQuality`'s `judgements` and `sample`, read through the `ekr.fact-judgements/1`
/// document they are the fields of, so the target reads them as a host does.
fn judged_sample(request: &SemanticCommandRequest) -> Result<FactJudgements, TargetError> {
    let json = |node: &Node| -> Result<serde_json::Value, TargetError> {
        serde_json::to_value(node).map_err(|e| unavailable("reading the judged sample", e))
    };
    let judgements = match request.input.get("judgements") {
        Some(node @ Node::Seq(_)) => json(node)?,
        _ => return Err(unavailable("reading `judgements`", "not a list")),
    };
    let mut document = serde_json::json!({
        "format": ekr_views::JUDGEMENTS_FORMAT,
        "judgements": judgements,
    });
    match request.input.get("sample") {
        None | Some(Node::Null) => {}
        Some(node) => {
            // A generated scenario writes its integers as JSON numbers with a fraction.
            let mut sample = json(node)?;
            if let Some(fields) = sample.as_object_mut() {
                for key in ["revision", "seed", "size"] {
                    if let Some(number) = fields.get(key).and_then(serde_json::Value::as_f64) {
                        fields.insert(key.to_owned(), serde_json::json!(number as i64));
                    }
                }
            }
            document["sample"] = sample;
        }
    }
    FactJudgements::from_json(document.to_string().as_bytes())
        .map_err(|e| unavailable("reading the judged sample", e))
}

/// Answers `ReportFactQuality`, which reads no store.
fn report(
    command: &str,
    judged: &FactJudgements,
    confidence: Option<i64>,
) -> Result<SemanticCommandResult, TargetError> {
    match ekr_views::report_fact_quality(judged, confidence) {
        Ok(answer) => Ok(
            SemanticCommandResult::took(outcome_ref(command, "reported")?)
                .emitting(fact_quality_reported(&answer.summary)?),
        ),
        Err(FactQualityError::LimitExceeded(refusal)) => limit_exceeded(command, &refusal),
        Err(FactQualityError::JudgedTwice { assertion }) => Ok(SemanticCommandResult::took(
            outcome_ref(command, "judged-twice")?,
        )
        .with_error(
            error("ekr.views.JudgedTwice")?.with("assertion", Node::Text(assertion.to_string())),
        )),
    }
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
        FIND_CODE_NAMES => Read::CodeNames(
            sources(request)?,
            match request.input.get("mode") {
                None | Some(Node::Null) => ekr_views::CodeNameMode::Literals,
                Some(Node::Text(mode)) if mode == "Literals" => ekr_views::CodeNameMode::Literals,
                Some(Node::Text(mode)) if mode == "Words" => ekr_views::CodeNameMode::Words,
                _ => return Err(unavailable("reading `mode`", "unknown occurrence mode")),
            },
        ),
        REPORT_STORE_QUALITY => Read::Quality,
        EXPORT_OCEL => Read::Ocel {
            events: event_names(request)?,
            times: event_times(request)?,
        },
        DRAW_FACT_SAMPLE => Read::Sample(sample_request(request)?),
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
    if let Read::Sample(request) = read {
        // The size first: a broken one is refused before the store is read.
        let request = match request {
            Ok(request) => request,
            Err(refusal) => return Ok((limit_exceeded(command, &refusal)?, None)),
        };
        return match ekr_views::draw_sample(runtime, at, &request) {
            Ok(answer) => Ok((took("drawn", fact_sample_drawn(&answer.summary)?)?, None)),
            Err(error) => Ok((refused(command, QueryError::Project(error))?, None)),
        };
    }
    if let Read::Ocel { events, times } = read {
        return match ekr_views::export_ocel_with_event_time(runtime, at, &events, &times) {
            Ok(answer) => Ok((took("exported", ocel_exported(&answer.summary)?)?, None)),
            Err(OcelError::EventTypeNotFound { name, revision }) => Ok((
                SemanticCommandResult::took(outcome_ref(command, "event-type-not-found")?)
                    .with_error(
                        error("ekr.views.EventTypeNotFound")?
                            .with("name", Node::Text(name))
                            .with("revision", integer(revision.get())?),
                    ),
                None,
            )),
            Err(OcelError::Project(error)) => {
                Ok((refused(command, QueryError::Project(error))?, None))
            }
            Err(OcelError::EventTimeInvalid {
                selector,
                reason,
                revision,
            }) => Ok((
                SemanticCommandResult::took(outcome_ref(command, "event-time-invalid")?)
                    .with_error(
                        error("ekr.views.EventTimeInvalid")?
                            .with("selector", Node::Text(selector))
                            .with("reason", Node::Text(reason))
                            .with("revision", integer(revision.get())?),
                    ),
                None,
            )),
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
    if let Read::CodeNames(sources, mode) = read {
        return match ekr_views::find_code_names_with_mode(runtime, at, &sources, mode) {
            Ok(answer) => Ok((took("found", code_names_found(&answer.summary)?)?, None)),
            Err(error) => Ok((refused(command, QueryError::Project(error))?, None)),
        };
    }
    let result = (|| -> Result<Result<SemanticCommandResult, TargetError>, QueryError> {
        // The bound first: a broken one is refused before the store is read.
        match read {
            Read::Graph
            | Read::Changes(_)
            | Read::CodeNames(_, _)
            | Read::Quality
            | Read::Ocel { .. }
            | Read::Sample(_) => unreachable!("answered above"),
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
        if command == REPORT_FACT_QUALITY {
            let mut judged = judged_sample(&request)?;
            let confidence = optional_integer(&request, "confidence")?;
            let mut guard = self.scenario.borrow_mut();
            let scenario = guard
                .as_mut()
                .ok_or_else(|| unavailable("executing a command", "no scenario is open"))?;
            if scenario.control.take() == Some(Control::JudgedTwice) {
                // The judged-twice branch's state: one assertion judged a second time.
                let again = judged.judgements.first().cloned().unwrap_or(Judgement {
                    assertion: fixtures::id(fixtures::Q_ASSERTIONS + 1),
                    verdict: Verdict::Pass,
                });
                if judged.judgements.is_empty() {
                    judged.judgements.push(again.clone());
                }
                judged.judgements.push(again);
            }
            let result = report(command, &judged, confidence)?;
            scenario.events.extend(result.direct_events.iter().cloned());
            return Ok(result);
        }
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
        if control == Some(Control::EventTypeAbsent) {
            if let Read::Ocel { events, .. } = &read {
                if events.is_empty() {
                    read = Read::Ocel {
                        events: vec![fixtures::UNDECLARED_TYPE_NAME.to_owned()],
                        times: Vec::new(),
                    };
                }
            }
        }
        if control == Some(Control::EventTimeInvalid) {
            read = Read::Ocel {
                events: Vec::new(),
                times: vec!["undeclared.at".to_owned()],
            };
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
                Some(
                    Control::NodeAbsent | Control::EventTypeAbsent | Control::EventTimeInvalid,
                ) => {
                    Fixture::SchemaEvolution.build(&runtime);
                }
                // Judged-twice is ReportFactQuality's, which names no store: no store's state.
                None | Some(Control::JudgedTwice) => fixture.build(&runtime),
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
            (EXPORT_OCEL, "event-type-not-found") => Control::EventTypeAbsent,
            (EXPORT_OCEL, "event-time-invalid") => Control::EventTimeInvalid,
            (REPORT_FACT_QUALITY, "judged-twice") => Control::JudgedTwice,
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
