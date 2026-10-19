//! The `ekr.views` reads a session serves as verbs (`story:sdk-read-helpers`): `overview`,
//! `search`, `describe`, `expand`, `timeline` and `changes`.
//!
//! Each answers the document `ekr view` serves on its route for the same request and revision —
//! `GET /overview`, `/search`, `/node/<id>`, `/expand` (as the one `ekr.graph-slice/1` page `ekr
//! mcp`'s `expand` answers, not NDJSON), `/timeline` and `/changes` — through the same
//! `ekr_views` calls, with the same bounds checked in the same order, and refuses what that route
//! refuses by the same name and message: `ekr.views.LimitExceeded` and `ekr.views.SinceMalformed`
//! before the store is read, then `ekr.views.NotSeeded`, `ekr.views.RevisionNotFound`,
//! `ekr.views.NodeNotFound`, `ekr.views.NodeIsAType` and `ekr.views.NodeTypeNotFound`, each a named
//! refusal (exit 2). A query `ekr view` answers `invalid-query` is an argv clap does not take
//! here, or a search filter `search_filter` refuses: a usage error.
//!
//! `overview`, `expand`, `timeline` and `changes` take `--format`, as their routes take `format`:
//! each answers its route's predecessor format unless the request names the named one
//! (`views.yaml`, "Named formats"); a name the read does not answer is a usage error.
//!
//! These are session verbs only: `ekr` has no one-shot verb of these names. A request is parsed
//! by [`ViewsCli`] when the one-shot verbs' definitions name no such verb.

use std::path::PathBuf;

use clap::error::ErrorKind;
use clap::{Parser, Subcommand, ValueEnum};
use ekr_core::{NodeId, RevisionNumber, TypeId};
use ekr_kernel::Runtime;
use ekr_views::{
    BucketWidth, ChangesError, ChangesRequest, DetailFormat, ExpandRequest, IndexCache,
    LimitExceeded, MatchesFormat, OverviewRequest, ProjectError, QueryError, SearchError,
    SearchRequest, SinceKind, TimelineRequest, UnknownMatchesFormat,
};
use ekr_views::{ChangesFormat, OverviewFormat, SliceFormat, TimelineFormat, UnknownFormat};
use serde_json::Value;

use super::super::view::{
    describe, node_detail, overview_sections, project_refusal, search_filter, since_not_one,
    DetailRefused, OverviewSections, LIMIT_EXCEEDED, NODE_IS_A_TYPE, NODE_NOT_FOUND,
    NODE_SUPERSEDED, NODE_TYPE_NOT_FOUND, SEARCH_LIMIT, SINCE_MALFORMED,
};
use super::super::Backend;
use crate::exit::Failure;

/// A views request: the session's global options, which [`super::admit`] refuses, and the verb.
#[derive(Debug, Parser)]
#[command(name = "ekr")]
pub(super) struct ViewsCli {
    #[arg(long, global = true)]
    host: Option<PathBuf>,
    #[arg(long, global = true)]
    store: Option<PathBuf>,
    #[arg(long, value_enum, global = true)]
    backend: Option<Backend>,
    #[arg(long, global = true)]
    full_replay: bool,
    #[arg(long, global = true)]
    stage: Option<String>,
    #[command(subcommand)]
    verb: Verb,
}

impl ViewsCli {
    /// Whether the request sets a global option, which is the session's own.
    pub(super) const fn sets_an_option(&self) -> bool {
        self.host.is_some()
            || self.store.is_some()
            || self.backend.is_some()
            || self.full_replay
            || self.stage.is_some()
    }

    /// The verb.
    pub(super) const fn verb(&self) -> &Verb {
        &self.verb
    }
}

/// `timeline --bucket`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(super) enum Bucket {
    Day,
    Week,
}

/// The six views verbs, each with the arguments of its `ekr view` route's query.
#[derive(Clone, Debug, Subcommand)]
pub(super) enum Verb {
    /// `GET /overview`: the `ekr.graph-overview/2` document, or `ekr.graph-overview/3` when
    /// `--format` names it.
    Overview {
        /// The format answered: `ekr.graph-overview/2` when absent, or `ekr.graph-overview/3`,
        /// which names each type and says when the revision was committed.
        #[arg(long, value_parser = named_format::<OverviewFormat>)]
        format: Option<OverviewFormat>,
        /// The committed revision to read; the head when absent.
        #[arg(long, value_parser = revision_number)]
        revision: Option<u64>,
        /// How many of the highest-degree nodes to list, 1 to 500; 300 when absent.
        #[arg(long, allow_negative_numbers = true, value_parser = bound)]
        limit: Option<i64>,
        /// The optional sections to answer as well, joined by `,`: `ontology`, `schema`,
        /// `roles`, `timeline`; none when absent or empty.
        #[arg(long, value_parser = overview_sections)]
        sections: Option<OverviewSections>,
    },
    /// `GET /search`: the `ekr.node-matches/1` document, or `ekr.node-matches/2` or
    /// `ekr.node-matches/3` when `--format` names it.
    Search {
        /// The text searched for; empty matches every node. Put `--` before a text that starts
        /// with `-`.
        text: String,
        /// The most matches answered, 1 to 100; 20 when absent.
        #[arg(long, allow_negative_numbers = true, value_parser = bound)]
        limit: Option<i64>,
        /// The format answered: `ekr.node-matches/1` when absent, `ekr.node-matches/2` or
        /// `ekr.node-matches/3`.
        #[arg(long, value_parser = matches_format)]
        format: Option<MatchesFormat>,
        /// `ekr.node-matches/2` and /3 only: the node type matched, by id or name.
        #[arg(long = "type")]
        node_type: Option<String>,
        /// `ekr.node-matches/2` and /3 only: the window's first instant, milliseconds since the
        /// epoch or `YYYY-MM-DD` (that day's first millisecond, UTC).
        #[arg(long, allow_hyphen_values = true)]
        from: Option<String>,
        /// `ekr.node-matches/2` and /3 only: the window's last instant, milliseconds since the
        /// epoch or `YYYY-MM-DD` (that day's last millisecond, UTC).
        #[arg(long, allow_hyphen_values = true)]
        to: Option<String>,
        /// The committed revision to read; the head when absent.
        #[arg(long, value_parser = revision_number)]
        revision: Option<u64>,
    },
    /// `GET /node/<id>`: the `ekr.node-detail/1` document of one node, or with `--format
    /// ekr.node-detail/2` its bounded `ekr.node-detail/2`.
    Describe {
        /// The node's id; one that is no node id names no node (`ekr.views.NodeNotFound`).
        node: String,
        /// The format answered: `ekr.node-detail/1` (the default) or `ekr.node-detail/2`.
        #[arg(long, value_parser = detail_format)]
        format: Option<DetailFormat>,
        /// For `ekr.node-detail/2`, the most values per property and neighbours per relation
        /// group, 1 to 500; 10 when absent.
        #[arg(long, allow_negative_numbers = true, value_parser = bound)]
        limit: Option<i64>,
        /// For `ekr.node-detail/2`, only this edge type's relation groups, by its id or name.
        #[arg(long = "edge-type")]
        edge_type: Option<String>,
        /// For `ekr.node-detail/2`, the first neighbour of each relation group to answer: a
        /// group's `next`; 0 when absent.
        #[arg(long, allow_negative_numbers = true, value_parser = bound)]
        after: Option<i64>,
        /// The committed revision to read; the head when absent.
        #[arg(long, value_parser = revision_number)]
        revision: Option<u64>,
    },
    /// `GET /expand`: one `ekr.graph-slice/1` page, or `ekr.graph-slice/2` when `--format` names
    /// it, as one document.
    Expand {
        /// The format answered: `ekr.graph-slice/1` when absent, or `ekr.graph-slice/2`, whose
        /// nodes and edges carry their `type_name`.
        #[arg(long, value_parser = named_format::<SliceFormat>)]
        format: Option<SliceFormat>,
        /// The node ids to start from; none answers an empty page.
        seeds: Vec<NodeId>,
        /// The hops from the seeds, 0 to 2.
        #[arg(long, allow_negative_numbers = true, value_parser = bound)]
        depth: i64,
        /// The most nodes a page holds, 1 to 2,000.
        #[arg(long, allow_negative_numbers = true, value_parser = bound)]
        limit: i64,
        /// The most edges a page holds, 1 to 5,000; 5,000 when absent.
        #[arg(long, allow_negative_numbers = true, value_parser = bound)]
        edges: Option<i64>,
        /// The cursor the page starts at: an earlier page's `next`; 0 when absent.
        #[arg(long, allow_negative_numbers = true, value_parser = bound)]
        after: Option<i64>,
        /// The committed revision to read; the head when absent.
        #[arg(long, value_parser = revision_number)]
        revision: Option<u64>,
    },
    /// `GET /timeline`: the `ekr.graph-timeline/1` document, or `ekr.graph-timeline/2` when
    /// `--format` names it.
    Timeline {
        /// The format answered: `ekr.graph-timeline/1` when absent, or `ekr.graph-timeline/2`,
        /// which carries `names`.
        #[arg(long, value_parser = named_format::<TimelineFormat>)]
        format: Option<TimelineFormat>,
        /// The row type's id; the first type the document ranks when absent.
        #[arg(long = "type")]
        row_type: Option<TypeId>,
        /// How far an event may be from its subject, 1 to 3.
        #[arg(long, allow_negative_numbers = true, value_parser = bound)]
        hops: i64,
        /// The most rows answered, 1 to 500.
        #[arg(long, allow_negative_numbers = true, value_parser = bound)]
        limit: i64,
        /// The finest bucket.
        #[arg(long, value_enum)]
        bucket: Option<Bucket>,
        /// One node whose row and events to answer.
        #[arg(long)]
        subject: Option<NodeId>,
        /// The committed revision to read; the head when absent.
        #[arg(long, value_parser = revision_number)]
        revision: Option<u64>,
    },
    /// `GET /changes`: the `ekr.graph-changes/1` page of what changed after exactly one since, or
    /// `ekr.graph-changes/2` when `--format` names it.
    #[command(group(clap::ArgGroup::new("since").required(true).multiple(false)))]
    Changes {
        /// The format answered: `ekr.graph-changes/1` when absent, or `ekr.graph-changes/2`,
        /// which carries `names`.
        #[arg(long, value_parser = named_format::<ChangesFormat>)]
        format: Option<ChangesFormat>,
        /// The changes of the revisions after this one.
        #[arg(long, group = "since", allow_negative_numbers = true, value_parser = bound)]
        since_revision: Option<i64>,
        /// The assertion changes valid after this time, in milliseconds since the epoch.
        #[arg(long, group = "since", allow_negative_numbers = true, value_parser = bound)]
        since_valid: Option<i64>,
        /// The changes of the revisions committed after this time, in milliseconds since the epoch.
        #[arg(long, group = "since", allow_negative_numbers = true, value_parser = bound)]
        since_recorded: Option<i64>,
        /// The last committed revision read; the head when absent.
        #[arg(long, value_parser = revision_number)]
        at: Option<u64>,
        /// The most changes a page holds, 1 to 2,000; 500 when absent.
        #[arg(long, allow_negative_numbers = true, value_parser = bound)]
        limit: Option<i64>,
        /// The cursor the page starts at: an earlier page's `next`; 0 when absent.
        #[arg(long, allow_negative_numbers = true, value_parser = bound)]
        after: Option<i64>,
    },
}

/// A bound as `ekr view` reads one: an optional `-` and one or more ASCII digits that fit an
/// `i64`. A `+`, a space or another digit script is not one, as the route refuses it
/// `invalid-query`; the range is the engine's to refuse, as `ekr.views.LimitExceeded`.
fn bound(value: &str) -> Result<i64, String> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("{value:?} is not a decimal integer"));
    }
    value
        .parse()
        .map_err(|_| format!("{value:?} is out of range"))
}

/// `--format`: a format the read answers, by its literal, as `ekr view` reads `format`.
fn named_format<F>(value: &str) -> Result<F, String>
where
    F: std::str::FromStr<Err = UnknownFormat>,
{
    value
        .parse()
        .map_err(|error: UnknownFormat| error.to_string())
}

/// A revision as `ekr view` reads one: one or more ASCII digits that fit a `u64`, no sign.
fn revision_number(value: &str) -> Result<u64, String> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("{value:?} is not a decimal revision number"));
    }
    value
        .parse()
        .map_err(|_| format!("{value:?} is not a revision number"))
}

/// A node-detail format as `ekr view` reads `/node/<id>`'s `format`: its literal.
fn detail_format(value: &str) -> Result<DetailFormat, String> {
    value.parse().map_err(|error| format!("{error}"))
}

/// `argv` as a views request: `None` when it names no views verb, so the one-shot verbs'
/// refusal stands; a refusal for help and version, as for every verb; clap's usage message for
/// anything else it does not take.
pub(super) fn parse(argv: &[String]) -> Option<Result<ViewsCli, Failure>> {
    match ViewsCli::try_parse_from(std::iter::once("ekr").chain(argv.iter().map(String::as_str))) {
        Ok(cli) => Some(Ok(cli)),
        Err(error) => match error.kind() {
            ErrorKind::InvalidSubcommand
            | ErrorKind::MissingSubcommand
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => None,
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => Some(Err(super::help_refused())),
            _ => Some(Err(Failure::Usage {
                message: error.render().to_string(),
            })),
        },
    }
}

/// The document `verb` answers from the runtime the session holds and the indexes it keeps of
/// that store.
pub(super) fn answer(
    verb: &Verb,
    runtime: &Runtime,
    indexes: &mut IndexCache,
) -> Result<Value, Failure> {
    serde_json::from_slice(&read(verb, runtime, indexes)?).map_err(Failure::fault)
}

/// The document's bytes, as `ekr view` renders them for the same query.
fn read(verb: &Verb, runtime: &Runtime, indexes: &mut IndexCache) -> Result<Vec<u8>, Failure> {
    let revision = |at: &Option<u64>| at.map(RevisionNumber::new);
    Ok(match verb {
        Verb::Overview {
            format,
            revision: at,
            limit,
            sections,
        } => {
            let request = OverviewRequest::new(*limit)
                .map_err(limit_exceeded)?
                .with_sections(
                    sections
                        .iter()
                        .flat_map(|sections| sections.0.iter().copied()),
                );
            let index = indexes.index(runtime, revision(at)).map_err(project)?;
            index
                .overview_as(format.unwrap_or(OverviewFormat::V2), &request)
                .map_err(project)?
                .bytes
        }
        Verb::Search {
            text,
            limit,
            format,
            node_type,
            from,
            to,
            revision: at,
        } => {
            let format = format.unwrap_or(MatchesFormat::V1);
            let filter = search_filter(format, node_type.clone(), from.as_deref(), to.as_deref())
                .map_err(|message| Failure::Usage {
                message: format!("error: {message}\n"),
            })?;
            let request = SearchRequest::new(text.clone(), limit.unwrap_or(SEARCH_LIMIT))
                .map_err(limit_exceeded)?;
            let index = indexes.index(runtime, revision(at)).map_err(project)?;
            index
                .search_as(format, &request, &filter)
                .map_err(search)?
                .bytes
        }
        Verb::Describe {
            node,
            format,
            limit,
            edge_type,
            after,
            revision: at,
        } => {
            let detail = node_detail(*format, DetailFormat::V1, *limit, edge_type.clone(), *after)
                .map_err(|message| Failure::Usage {
                    message: format!("error: {message}\n"),
                })?
                .map_err(limit_exceeded)?;
            let index = indexes.index(runtime, revision(at)).map_err(project)?;
            describe(&index, node, &detail).map_err(|refusal| match refusal {
                DetailRefused::NotANodeId(message) => Failure::refused(NODE_NOT_FOUND, message),
                DetailRefused::Query(error) => query(error),
            })?
        }
        Verb::Expand {
            format,
            seeds,
            depth,
            limit,
            edges,
            after,
            revision: at,
        } => {
            let request = ExpandRequest::new(seeds.clone(), *depth, *limit, *edges, *after)
                .map_err(limit_exceeded)?;
            let index = indexes.index(runtime, revision(at)).map_err(project)?;
            index
                .expand_as(format.unwrap_or(SliceFormat::V1), &request)
                .map_err(query)?
                .bytes
        }
        Verb::Timeline {
            format,
            row_type,
            hops,
            limit,
            bucket,
            subject,
            revision: at,
        } => {
            let bucket = bucket.map(|bucket| match bucket {
                Bucket::Day => BucketWidth::Day,
                Bucket::Week => BucketWidth::Week,
            });
            let request = TimelineRequest::new(*row_type, *hops, *limit, bucket, *subject)
                .map_err(limit_exceeded)?;
            let index = indexes.index(runtime, revision(at)).map_err(project)?;
            index
                .timeline_as(format.unwrap_or(TimelineFormat::V1), &request)
                .map_err(project)?
                .bytes
        }
        Verb::Changes {
            format,
            since_revision,
            since_valid,
            since_recorded,
            at,
            limit,
            after,
        } => {
            let (kind, since) = SinceKind::one_of(*since_revision, *since_valid, *since_recorded)
                .map_err(|given| Failure::Usage {
                message: format!("error: {}\n", since_not_one(&given)),
            })?;
            let request = ChangesRequest::new(kind, since, *limit, *after).map_err(changes)?;
            let index = indexes.index(runtime, revision(at)).map_err(project)?;
            index
                .changes_as(runtime, &request, format.unwrap_or(ChangesFormat::V1))
                .map_err(changes)?
                .bytes
        }
    })
}

/// `ekr.views.LimitExceeded`, before the store is read.
fn limit_exceeded(error: LimitExceeded) -> Failure {
    Failure::refused(LIMIT_EXCEEDED, error)
}

/// A revision that could not be loaded: `ekr.views.NotSeeded` or `ekr.views.RevisionNotFound`,
/// else a store that could not be read, a fault.
fn project(error: ProjectError) -> Failure {
    match project_refusal(&error) {
        Some(name) => Failure::refused(name, error),
        None => Failure::unread(error),
    }
}

/// A bounded read that answered nothing.
fn query(error: QueryError) -> Failure {
    match error {
        QueryError::LimitExceeded(error) => limit_exceeded(error),
        error @ QueryError::NodeNotFound { .. } => Failure::refused(NODE_NOT_FOUND, error),
        error @ QueryError::NodeIsAType { .. } => Failure::refused(NODE_IS_A_TYPE, error),
        error @ QueryError::NodeSuperseded { .. } => Failure::refused(NODE_SUPERSEDED, error),
        QueryError::Project(error) => project(error),
    }
}

/// A `search` that answered nothing once its bounds held.
fn search(error: SearchError) -> Failure {
    match error {
        error @ SearchError::NodeTypeNotFound { .. } => {
            Failure::refused(NODE_TYPE_NOT_FOUND, error)
        }
        SearchError::Project(error) => project(error),
    }
}

/// `--format`: a search format's literal.
fn matches_format(text: &str) -> Result<MatchesFormat, String> {
    text.parse()
        .map_err(|error: UnknownMatchesFormat| error.to_string())
}

/// A `changes` that answered nothing.
fn changes(error: ChangesError) -> Failure {
    match error {
        ChangesError::SinceMalformed(error) => Failure::refused(SINCE_MALFORMED, error),
        ChangesError::LimitExceeded(error) => limit_exceeded(error),
        ChangesError::Project(error) => project(error),
    }
}
