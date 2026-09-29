//! The `ekr.views` reads a session serves as verbs (`story:sdk-read-helpers`): `overview`,
//! `search`, `describe`, `expand`, `timeline` and `changes`.
//!
//! Each answers the document `ekr view` serves on its route for the same request and revision —
//! `GET /overview`, `/search`, `/node/<id>`, `/expand` (as the one `ekr.graph-slice/1` page `ekr
//! mcp`'s `expand` answers, not NDJSON), `/timeline` and `/changes` — through the same
//! `ekr_views` calls, with the same bounds checked in the same order, and refuses what that route
//! refuses by the same name and message: `ekr.views.LimitExceeded` and `ekr.views.SinceMalformed`
//! before the store is read, then `ekr.views.NotSeeded`, `ekr.views.RevisionNotFound` and
//! `ekr.views.NodeNotFound`, each a named refusal (exit 2). A query `ekr view` answers
//! `invalid-query` is an argv clap does not take here: a usage error.
//!
//! These are session verbs only: `ekr` has no one-shot verb of these names. A request is parsed
//! by [`ViewsCli`] when the one-shot verbs' definitions name no such verb.

use std::path::PathBuf;

use clap::error::ErrorKind;
use clap::{Parser, Subcommand, ValueEnum};
use ekr_core::{NodeId, RevisionNumber, TypeId};
use ekr_kernel::Runtime;
use ekr_views::{
    BucketWidth, ChangesError, ChangesRequest, ExpandRequest, IndexCache, LimitExceeded,
    OverviewRequest, ProjectError, QueryError, SearchRequest, SinceKind, TimelineRequest,
};
use serde_json::Value;

use super::super::view::{
    not_a_node_id, project_refusal, since_not_one, LIMIT_EXCEEDED, NODE_NOT_FOUND, SEARCH_LIMIT,
    SINCE_MALFORMED,
};
use super::super::{Backend, Session};
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
    #[command(subcommand)]
    verb: Verb,
}

impl ViewsCli {
    /// Whether the request sets a global option, which is the session's own.
    pub(super) const fn sets_an_option(&self) -> bool {
        self.host.is_some() || self.store.is_some() || self.backend.is_some() || self.full_replay
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
    /// `GET /overview`: the `ekr.graph-overview/1` document.
    Overview {
        /// The committed revision to read; the head when absent.
        #[arg(long)]
        revision: Option<u64>,
        /// How many of the highest-degree nodes to list, 1 to 500; 300 when absent.
        #[arg(long, allow_negative_numbers = true)]
        limit: Option<i64>,
    },
    /// `GET /search`: the `ekr.node-matches/1` document.
    Search {
        /// The text searched for; empty matches every node. Put `--` before a text that starts
        /// with `-`.
        text: String,
        /// The most matches answered, 1 to 100; 20 when absent.
        #[arg(long, allow_negative_numbers = true)]
        limit: Option<i64>,
        /// The committed revision to read; the head when absent.
        #[arg(long)]
        revision: Option<u64>,
    },
    /// `GET /node/<id>`: the `ekr.node-detail/1` document of one node.
    Describe {
        /// The node's id; one that is no node id names no node (`ekr.views.NodeNotFound`).
        node: String,
        /// The committed revision to read; the head when absent.
        #[arg(long)]
        revision: Option<u64>,
    },
    /// `GET /expand`: one `ekr.graph-slice/1` page, as one document.
    Expand {
        /// The node ids to start from; none answers an empty page.
        seeds: Vec<NodeId>,
        /// The hops from the seeds, 0 to 2.
        #[arg(long, allow_negative_numbers = true)]
        depth: i64,
        /// The most nodes a page holds, 1 to 2,000.
        #[arg(long, allow_negative_numbers = true)]
        limit: i64,
        /// The most edges a page holds, 1 to 5,000; 5,000 when absent.
        #[arg(long, allow_negative_numbers = true)]
        edges: Option<i64>,
        /// The cursor the page starts at: an earlier page's `next`; 0 when absent.
        #[arg(long, allow_negative_numbers = true)]
        after: Option<i64>,
        /// The committed revision to read; the head when absent.
        #[arg(long)]
        revision: Option<u64>,
    },
    /// `GET /timeline`: the `ekr.graph-timeline/1` document.
    Timeline {
        /// The row type's id; the first type the document ranks when absent.
        #[arg(long = "type")]
        row_type: Option<TypeId>,
        /// How far an event may be from its subject, 1 to 3.
        #[arg(long, allow_negative_numbers = true)]
        hops: i64,
        /// The most rows answered, 1 to 500.
        #[arg(long, allow_negative_numbers = true)]
        limit: i64,
        /// The finest bucket.
        #[arg(long, value_enum)]
        bucket: Option<Bucket>,
        /// One node whose row and events to answer.
        #[arg(long)]
        subject: Option<NodeId>,
        /// The committed revision to read; the head when absent.
        #[arg(long)]
        revision: Option<u64>,
    },
    /// `GET /changes`: the `ekr.graph-changes/1` page of what changed after exactly one since.
    #[command(group(clap::ArgGroup::new("since").required(true).multiple(false)))]
    Changes {
        /// The changes of the revisions after this one.
        #[arg(long, group = "since", allow_negative_numbers = true)]
        since_revision: Option<i64>,
        /// The assertion changes valid after this time, in milliseconds since the epoch.
        #[arg(long, group = "since", allow_negative_numbers = true)]
        since_valid: Option<i64>,
        /// The changes of the revisions committed after this time, in milliseconds since the epoch.
        #[arg(long, group = "since", allow_negative_numbers = true)]
        since_recorded: Option<i64>,
        /// The last committed revision read; the head when absent.
        #[arg(long)]
        at: Option<u64>,
        /// The most changes a page holds, 1 to 2,000; 500 when absent.
        #[arg(long, allow_negative_numbers = true)]
        limit: Option<i64>,
        /// The cursor the page starts at: an earlier page's `next`; 0 when absent.
        #[arg(long, allow_negative_numbers = true)]
        after: Option<i64>,
    },
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

/// The document `verb` answers from the session's store: through the runtime it holds and the
/// indexes it keeps of that store, or — holding none — through the store opened as a one-shot
/// verb opens it, `store-not-found` where there is none.
pub(super) fn answer(
    verb: &Verb,
    session: &Session,
    indexes: &mut IndexCache,
) -> Result<Value, Failure> {
    let bytes = match &session.runtime {
        Some(runtime) => read(verb, runtime, indexes)?,
        None => read(verb, &session.store.open()?, &mut IndexCache::new(1))?,
    };
    serde_json::from_slice(&bytes).map_err(Failure::fault)
}

/// The document's bytes, as `ekr view` renders them for the same query.
fn read(verb: &Verb, runtime: &Runtime, indexes: &mut IndexCache) -> Result<Vec<u8>, Failure> {
    let revision = |at: &Option<u64>| at.map(RevisionNumber::new);
    Ok(match verb {
        Verb::Overview {
            revision: at,
            limit,
        } => {
            let request = OverviewRequest::new(*limit).map_err(limit_exceeded)?;
            let index = indexes.index(runtime, revision(at)).map_err(project)?;
            index.overview(&request).map_err(project)?.bytes
        }
        Verb::Search {
            text,
            limit,
            revision: at,
        } => {
            let request = SearchRequest::new(text.clone(), limit.unwrap_or(SEARCH_LIMIT))
                .map_err(limit_exceeded)?;
            let index = indexes.index(runtime, revision(at)).map_err(project)?;
            index.search(&request).map_err(project)?.bytes
        }
        Verb::Describe { node, revision: at } => {
            let index = indexes.index(runtime, revision(at)).map_err(project)?;
            let Ok(id) = node.parse::<NodeId>() else {
                return Err(Failure::refused(NODE_NOT_FOUND, not_a_node_id(node)));
            };
            index.describe(id).map_err(query)?.bytes
        }
        Verb::Expand {
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
            index.expand(&request).map_err(query)?.bytes
        }
        Verb::Timeline {
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
            index.timeline(&request).map_err(project)?.bytes
        }
        Verb::Changes {
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
            index.changes(runtime, &request).map_err(changes)?.bytes
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
        None => Failure::fault(error),
    }
}

/// A bounded read that answered nothing.
fn query(error: QueryError) -> Failure {
    match error {
        QueryError::LimitExceeded(error) => limit_exceeded(error),
        error @ QueryError::NodeNotFound { .. } => Failure::refused(NODE_NOT_FOUND, error),
        QueryError::Project(error) => project(error),
    }
}

/// A `changes` that answered nothing.
fn changes(error: ChangesError) -> Failure {
    match error {
        ChangesError::SinceMalformed(error) => Failure::refused(SINCE_MALFORMED, error),
        ChangesError::LimitExceeded(error) => limit_exceeded(error),
        ChangesError::Project(error) => project(error),
    }
}
