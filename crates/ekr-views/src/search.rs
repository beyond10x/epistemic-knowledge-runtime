//! `ekr.node-matches/2` ([`Index::search_v2`]): `SearchNodes` answered with every word of the
//! text, a whole match first, the node's type named, and the matches narrowed to node types
//! and a window of time (`views.yaml`, `ekr.views.SearchNodes` and `ekr.views.NodeMatchesV2`).
//!
//! `ekr.node-matches/1` ([`Index::search`]) is served unchanged beside it; [`Index::search_as`]
//! answers the format a [`MatchesFormat`] names. Under /1 the command reads no [`SearchFilter`]:
//! a surface refuses a filter named with /1 as malformed input before it reaches the command.
//!
//! The order of /2 (`views.yaml`, determinism rule (1)): tier (Exact, Folded, Words); a whole
//! match before one that is not; with `from` or `to`, the node's time descending; degree
//! descending; id. Every node of the revision is compared, so the cost is the revision's name
//! bytes, as for /1.
//!
//! `ekr.node-matches/3` ([`Index::search_v3`]) matches the nodes /2 matches and names each
//! match's kind (`views.yaml`, `ekr.views.MatchKind` and `ekr.views.NodeMatchesV3`): whole name,
//! whole alias, prefix, substring, words, each case-exact or case-folded. Its order (determinism
//! rule for /3): kind, whatever the case; a case-exact match before a case-folded one; with
//! `from` or `to`, time descending; degree descending; id.

use std::cmp::Ordering;
use std::str::FromStr;

use ekr_core::{NodeId, RevisionNumber, TypeId};
use serde::Serialize;

use crate::index::Index;
use crate::query::{
    encode, hash, Answer, MatchField, MatchTier, NodesSearched, SearchRequest, MATCHES_FORMAT,
};
use crate::ProjectError;

/// The format literal of [`Index::search_v2`]'s documents.
pub const MATCHES_V2_FORMAT: &str = "ekr.node-matches/2";

/// The format literal of [`Index::search_v3`]'s documents.
pub const MATCHES_V3_FORMAT: &str = "ekr.node-matches/3";

/// `ekr.views.NodeMatchesFormat`: the format a `SearchNodes` request is answered in. The
/// command's default is /1; each surface names its own default (`views.yaml`,
/// `ekr.views.SearchNodes`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MatchesFormat {
    /// `ekr.node-matches/1`, [`Index::search`].
    #[default]
    V1,
    /// `ekr.node-matches/2`, [`Index::search_v2`].
    V2,
    /// `ekr.node-matches/3`, [`Index::search_v3`].
    V3,
}

impl MatchesFormat {
    /// The format's literal.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::V1 => MATCHES_FORMAT,
            Self::V2 => MATCHES_V2_FORMAT,
            Self::V3 => MATCHES_V3_FORMAT,
        }
    }
}

/// A text that names no search format.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "{0:?} is no search format; the formats are ekr.node-matches/1, ekr.node-matches/2 and \
     ekr.node-matches/3"
)]
pub struct UnknownMatchesFormat(pub String);

impl FromStr for MatchesFormat {
    type Err = UnknownMatchesFormat;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            MATCHES_FORMAT => Ok(Self::V1),
            MATCHES_V2_FORMAT => Ok(Self::V2),
            MATCHES_V3_FORMAT => Ok(Self::V3),
            other => Err(UnknownMatchesFormat(other.to_owned())),
        }
    }
}

/// Which bound of a search's window a text is read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowBound {
    /// `from`: a date is read as its first millisecond.
    From,
    /// `to`: a date is read as its last millisecond.
    To,
}

const DAY_MS: i64 = 86_400_000;

impl WindowBound {
    /// The input's name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::From => "from",
            Self::To => "to",
        }
    }

    /// Reads `text` as this bound, in milliseconds since the Unix epoch: the text of
    /// milliseconds (an optional `-` and ASCII digits that fit an `i64`), or a date `YYYY-MM-DD`
    /// of the proleptic Gregorian calendar naming its whole UTC day — as `from` its first
    /// millisecond, as `to` its last (`views.yaml`, `ekr.views.SearchNodes`).
    ///
    /// # Errors
    ///
    /// [`WindowMalformed`] for any other text, which is malformed input that never reaches the
    /// command.
    pub fn read(self, text: &str) -> Result<i64, WindowMalformed> {
        let malformed = || WindowMalformed {
            bound: self,
            text: text.to_owned(),
        };
        let digits = text.strip_prefix('-').unwrap_or(text);
        if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return text.parse().map_err(|_| malformed());
        }
        let day = civil_day(text).ok_or_else(malformed)?;
        let first = day * DAY_MS;
        Ok(match self {
            Self::From => first,
            Self::To => first + DAY_MS - 1,
        })
    }
}

/// The days from 1970-01-01 to the date `YYYY-MM-DD` `text` names, if it names one.
fn civil_day(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let part = &bytes[range];
        if !part.iter().all(u8::is_ascii_digit) {
            return None;
        }
        std::str::from_utf8(part).ok()?.parse::<i64>().ok()
    };
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let length = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=length).contains(&day) {
        return None;
    }
    // Howard Hinnant's days_from_civil, over the proleptic Gregorian calendar.
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let of_era_days = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    Some(era * 146_097 + of_era_days - 719_468)
}

/// A `from` or `to` that is neither milliseconds nor a date.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{} {text:?} is neither milliseconds since the epoch nor a YYYY-MM-DD date", bound.name())]
pub struct WindowMalformed {
    /// The bound the text was given as.
    pub bound: WindowBound,
    /// The text as the request gave it.
    pub text: String,
}

/// What `ekr.node-matches/2` and `/3` narrow their matches to: node types and a window of time
/// (`views.yaml`, `ekr.views.SearchNodes` `type`, `from` and `to`). The default narrows nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchFilter {
    node_type: Option<String>,
    from: Option<i64>,
    to: Option<i64>,
}

impl SearchFilter {
    /// `node_type` is a node type's id or name, resolved against the revision read; `from` and
    /// `to` are inclusive bounds in milliseconds since the epoch ([`WindowBound::read`]).
    #[must_use]
    pub const fn new(node_type: Option<String>, from: Option<i64>, to: Option<i64>) -> Self {
        Self {
            node_type,
            from,
            to,
        }
    }

    /// Whether the filter names nothing: what a /1 request may carry.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.node_type.is_none() && self.from.is_none() && self.to.is_none()
    }

    /// The node type's id or name, as the request gave it.
    #[must_use]
    pub fn node_type(&self) -> Option<&str> {
        self.node_type.as_deref()
    }

    /// The window's first millisecond.
    #[must_use]
    pub const fn from(&self) -> Option<i64> {
        self.from
    }

    /// The window's last millisecond.
    #[must_use]
    pub const fn to(&self) -> Option<i64> {
        self.to
    }

    const fn windowed(&self) -> bool {
        self.from.is_some() || self.to.is_some()
    }
}

/// Why a search answered nothing, once its bounds held.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    /// `ekr.views.NodeTypeNotFound`: the request's `type` is neither the id of a node type of the
    /// revision's ontology nor the name of one.
    #[error("revision {revision} holds no node type with the id or name {node_type:?}")]
    NodeTypeNotFound {
        /// The `type` as the request gave it.
        node_type: String,
        /// The revision that was read.
        revision: RevisionNumber,
    },
    /// `ekr.views.NotSeeded`, `ekr.views.RevisionNotFound`, or a store or revision that could
    /// not be read.
    #[error(transparent)]
    Project(#[from] ProjectError),
}

#[derive(Serialize)]
struct NodeMatchV2<'a> {
    id: NodeId,
    #[serde(rename = "type")]
    type_id: TypeId,
    type_name: &'a str,
    name: &'a str,
    degree: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<i64>,
    tier: MatchTier,
    field: MatchField,
    #[serde(skip_serializing_if = "Option::is_none")]
    alias: Option<&'a str>,
}

#[derive(Serialize)]
struct MatchesMetaV2<'a> {
    format: &'static str,
    revision: u64,
    text: &'a str,
    types: &'a [TypeId],
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<i64>,
    total: u64,
    exact_total: u64,
}

#[derive(Serialize)]
struct NodeMatchesV2<'a> {
    meta: MatchesMetaV2<'a>,
    matches: Vec<NodeMatchV2<'a>>,
}

/// `ekr.views.MatchKind`: how an `ekr.node-matches/3` match compares the text with a field, in
/// the order the kinds rank.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) enum MatchKind {
    WholeName,
    WholeAlias,
    Prefix,
    Substring,
    Words,
}

impl MatchKind {
    /// The kind's name, as the documents write it.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::WholeName => "WholeName",
            Self::WholeAlias => "WholeAlias",
            Self::Prefix => "Prefix",
            Self::Substring => "Substring",
            Self::Words => "Words",
        }
    }
}

#[derive(Serialize)]
struct NodeMatchV3<'a> {
    id: NodeId,
    #[serde(rename = "type")]
    type_id: TypeId,
    type_name: &'a str,
    name: &'a str,
    degree: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<i64>,
    kind: MatchKind,
    folded: bool,
    field: MatchField,
    #[serde(skip_serializing_if = "Option::is_none")]
    alias: Option<&'a str>,
}

#[derive(Serialize)]
struct MatchesMetaV3<'a> {
    format: &'static str,
    revision: u64,
    text: &'a str,
    types: &'a [TypeId],
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<i64>,
    total: u64,
    exact_total: u64,
    whole_total: u64,
}

#[derive(Serialize)]
struct NodeMatchesV3<'a> {
    meta: MatchesMetaV3<'a>,
    matches: Vec<NodeMatchV3<'a>>,
}

/// One node that matched under /3: its kind, whether it matched only after lowercasing, whether
/// its name or an alias holds the text byte for byte, its index, field, alias and time.
struct KindHit<'a> {
    kind: MatchKind,
    folded: bool,
    exact: bool,
    index: u32,
    field: MatchField,
    alias: Option<&'a str>,
    time: Option<i64>,
}

/// Whether `field` passes `kind` for `text`, whose whitespace-separated words are `words`, both
/// sides compared byte for byte. The caller lowercases both sides for a case-folded comparison.
fn passes(kind: MatchKind, field: &str, text: &str, words: &[&str]) -> bool {
    match kind {
        MatchKind::WholeName | MatchKind::WholeAlias => field == text,
        MatchKind::Prefix => field.starts_with(text),
        MatchKind::Substring => field.contains(text),
        MatchKind::Words => words.len() >= 2 && words.iter().all(|word| field.contains(word)),
    }
}

/// One node that matched: its tier, whether it is whole, its index, field, alias and time.
struct Hit<'a> {
    tier: MatchTier,
    whole: bool,
    index: u32,
    field: MatchField,
    alias: Option<&'a str>,
    time: Option<i64>,
}

/// `text`'s maximal runs of alphanumeric characters, sorted: what a Words match is whole by.
fn runs(text: &str) -> Vec<&str> {
    let mut runs: Vec<&str> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|run| !run.is_empty())
        .collect();
    runs.sort_unstable();
    runs
}

impl Index {
    /// `SearchNodes` in the format `format` names: [`Self::search`] for /1, which reads no
    /// filter, [`Self::search_v2`] for /2 and [`Self::search_v3`] for /3.
    ///
    /// # Errors
    ///
    /// What the format's read refuses.
    pub fn search_as(
        &self,
        format: MatchesFormat,
        request: &SearchRequest,
        filter: &SearchFilter,
    ) -> Result<Answer<NodesSearched>, SearchError> {
        match format {
            MatchesFormat::V1 => Ok(self.search(request)?),
            MatchesFormat::V2 => self.search_v2(request, filter),
            MatchesFormat::V3 => self.search_v3(request, filter),
        }
    }

    /// The node types `filter` narrows to, by id: none when it names no type.
    fn filter_types(&self, filter: &SearchFilter) -> Result<Vec<TypeId>, SearchError> {
        match filter.node_type() {
            None => Ok(Vec::new()),
            Some(named) => {
                self.node_types_named(named)
                    .ok_or_else(|| SearchError::NodeTypeNotFound {
                        node_type: named.to_owned(),
                        revision: self.loaded.graph.revision,
                    })
            }
        }
    }

    /// The node types `named` names in this revision: the one whose id it is, else every node
    /// type the ontology gives that name, by id; `None` when it names none.
    fn node_types_named(&self, named: &str) -> Option<Vec<TypeId>> {
        let ontology = &self.loaded.graph.ontology;
        if let Ok(id) = named.parse::<TypeId>() {
            if ontology.node_type(id).is_some() {
                return Some(vec![id]);
            }
        }
        let mut types: Vec<TypeId> = ontology
            .to_document()
            .node_types
            .iter()
            .filter(|declared| declared.name == named)
            .map(|declared| declared.id)
            .collect();
        types.sort_unstable();
        types.dedup();
        (!types.is_empty()).then_some(types)
    }

    /// `SearchNodes` as `ekr.node-matches/2` (`views.yaml`, `ekr.views.NodeMatchesV2`): the first
    /// `limit` nodes of the filter's types, in its window, whose canonical name or an alias
    /// contains the text — byte-exactly (Exact), else after lowercasing both sides (Folded), else,
    /// for a text of two or more whitespace-separated words, every word after lowercasing
    /// (Words) — ordered by tier, a whole match first, with a window newest first, then degree
    /// descending, then id. Every match names its type.
    ///
    /// # Errors
    ///
    /// [`SearchError::NodeTypeNotFound`] for a `type` the revision's ontology holds as no node
    /// type's id or name; [`ProjectError::Inconsistent`] for a node whose type the ontology does
    /// not declare, which a revision the kernel admitted never has, or a document that cannot be
    /// encoded.
    pub fn search_v2(
        &self,
        request: &SearchRequest,
        filter: &SearchFilter,
    ) -> Result<Answer<NodesSearched>, SearchError> {
        let graph = &self.loaded.graph;
        let types = self.filter_types(filter)?;
        let text = request.text();
        let folded = text.to_lowercase();
        let words: Vec<&str> = folded.split_whitespace().collect();
        let every_word = |field: &str| words.iter().all(|word| field.contains(word));
        let text_runs = runs(&folded);
        let windowed = filter.windowed();
        let mut found: Vec<Hit<'_>> = Vec::new();
        for (index, (node, (name, aliases))) in graph.nodes.values().zip(&self.folded).enumerate() {
            if !types.is_empty() && types.binary_search(&node.type_id).is_err() {
                continue;
            }
            let time = self.node_time.get(index).copied().flatten().map(|t| t.0);
            if windowed {
                let Some(at) = time else { continue };
                if filter.from.is_some_and(|from| at < from) || filter.to.is_some_and(|to| at > to)
                {
                    continue;
                }
            }
            let index = u32::try_from(index).unwrap_or(u32::MAX);
            let hit = if node.canonical_name.contains(text) {
                Some((MatchTier::Exact, MatchField::Name, None))
            } else if let Some(alias) = node.aliases.iter().find(|alias| alias.contains(text)) {
                Some((MatchTier::Exact, MatchField::Alias, Some(alias.as_str())))
            } else if name.contains(&folded) {
                Some((MatchTier::Folded, MatchField::Name, None))
            } else if let Some(at) = aliases.iter().position(|alias| alias.contains(&folded)) {
                Some((
                    MatchTier::Folded,
                    MatchField::Alias,
                    Some(node.aliases[at].as_str()),
                ))
            } else if words.len() < 2 {
                None
            } else if every_word(name.as_str()) {
                Some((MatchTier::Words, MatchField::Name, None))
            } else {
                aliases
                    .iter()
                    .position(|alias| every_word(alias.as_str()))
                    .map(|at| {
                        (
                            MatchTier::Words,
                            MatchField::Alias,
                            Some(node.aliases[at].as_str()),
                        )
                    })
            };
            let Some((tier, field, alias)) = hit else {
                continue;
            };
            let whole = match tier {
                MatchTier::Exact => {
                    node.canonical_name == text || node.aliases.iter().any(|alias| alias == text)
                }
                MatchTier::Folded => *name == folded || aliases.contains(&folded),
                MatchTier::Words => {
                    runs(name) == text_runs || aliases.iter().any(|alias| runs(alias) == text_runs)
                }
            };
            found.push(Hit {
                tier,
                whole,
                index,
                field,
                alias,
                time,
            });
        }
        found.sort_unstable_by(|a, b| {
            a.tier
                .cmp(&b.tier)
                .then(b.whole.cmp(&a.whole))
                .then(if windowed {
                    b.time.cmp(&a.time)
                } else {
                    Ordering::Equal
                })
                .then(self.degree[b.index as usize].cmp(&self.degree[a.index as usize]))
                .then(a.index.cmp(&b.index))
        });
        let total = found.len() as u64;
        let exact_total = found
            .iter()
            .filter(|hit| hit.tier == MatchTier::Exact)
            .count() as u64;
        let mut matches: Vec<NodeMatchV2<'_>> = Vec::new();
        for hit in found
            .iter()
            .take(usize::try_from(request.limit()).unwrap_or(usize::MAX))
        {
            let id = self.node_ids[hit.index as usize];
            let node = &graph.nodes[&id];
            let Some(declared) = graph.ontology.node_type(node.type_id) else {
                return Err(ProjectError::Inconsistent(format!(
                    "node {id} is of type {}, which revision {} does not declare",
                    node.type_id, graph.revision
                ))
                .into());
            };
            matches.push(NodeMatchV2 {
                id,
                type_id: node.type_id,
                type_name: &declared.name,
                name: &node.canonical_name,
                degree: self.degree[hit.index as usize],
                time: hit.time,
                tier: hit.tier,
                field: hit.field,
                alias: hit.alias,
            });
        }
        let first_match = matches.first().map(|hit| hit.id);
        let first_tier = matches.first().map(|hit| hit.tier.name());
        let document = NodeMatchesV2 {
            meta: MatchesMetaV2 {
                format: MATCHES_V2_FORMAT,
                revision: graph.revision.get(),
                text,
                types: &types,
                from: filter.from,
                to: filter.to,
                total,
                exact_total,
            },
            matches,
        };
        let bytes = encode(&document)?;
        let summary = NodesSearched {
            revision: document.meta.revision,
            text: text.to_owned(),
            matches: document.matches.len() as u64,
            total,
            exact_total,
            format: MATCHES_V2_FORMAT,
            first_match,
            first_tier,
            matches_hash: hash(&bytes),
            whole_total: None,
            first_kind: None,
            first_folded: None,
        };
        Ok(Answer { bytes, summary })
    }

    /// `SearchNodes` as `ekr.node-matches/3` (`views.yaml`, `ekr.views.NodeMatchesV3`): the
    /// nodes [`Self::search_v2`] matches for the same request, each naming its
    /// `ekr.views.MatchKind` and whether it matched only after lowercasing. A node's match is the
    /// first of WholeName, WholeAlias, Prefix, Substring and Words, each case-exact before
    /// case-folded, that its canonical name or one alias passes; the first `limit` are ordered by
    /// that kind and fold, with a window newest first, then degree descending, then id.
    ///
    /// # Errors
    ///
    /// As [`Self::search_v2`].
    pub fn search_v3(
        &self,
        request: &SearchRequest,
        filter: &SearchFilter,
    ) -> Result<Answer<NodesSearched>, SearchError> {
        const KINDS: [MatchKind; 5] = [
            MatchKind::WholeName,
            MatchKind::WholeAlias,
            MatchKind::Prefix,
            MatchKind::Substring,
            MatchKind::Words,
        ];
        let graph = &self.loaded.graph;
        let types = self.filter_types(filter)?;
        let text = request.text();
        let lowered = text.to_lowercase();
        let words: Vec<&str> = text.split_whitespace().collect();
        let lowered_words: Vec<&str> = lowered.split_whitespace().collect();
        let windowed = filter.windowed();
        let mut found: Vec<KindHit<'_>> = Vec::new();
        for (index, (node, (name, aliases))) in graph.nodes.values().zip(&self.folded).enumerate() {
            if !types.is_empty() && types.binary_search(&node.type_id).is_err() {
                continue;
            }
            let time = self.node_time.get(index).copied().flatten().map(|t| t.0);
            if windowed {
                let Some(at) = time else { continue };
                if filter.from.is_some_and(|from| at < from) || filter.to.is_some_and(|to| at > to)
                {
                    continue;
                }
            }
            let index = u32::try_from(index).unwrap_or(u32::MAX);
            let mut hit = None;
            'kinds: for kind in KINDS {
                for folded in [false, true] {
                    let (needle, needle_words, own_name, own_aliases) = if folded {
                        (lowered.as_str(), &lowered_words, name, aliases)
                    } else {
                        (text, &words, &node.canonical_name, &node.aliases)
                    };
                    if kind != MatchKind::WholeAlias && passes(kind, own_name, needle, needle_words)
                    {
                        hit = Some((kind, folded, MatchField::Name, None));
                        break 'kinds;
                    }
                    if kind == MatchKind::WholeName {
                        continue;
                    }
                    if let Some(at) = own_aliases
                        .iter()
                        .position(|alias| passes(kind, alias, needle, needle_words))
                    {
                        hit = Some((
                            kind,
                            folded,
                            MatchField::Alias,
                            Some(node.aliases[at].as_str()),
                        ));
                        break 'kinds;
                    }
                }
            }
            let Some((kind, folded, field, alias)) = hit else {
                continue;
            };
            let exact = node.canonical_name.contains(text)
                || node.aliases.iter().any(|alias| alias.contains(text));
            found.push(KindHit {
                kind,
                folded,
                exact,
                index,
                field,
                alias,
                time,
            });
        }
        found.sort_unstable_by(|a, b| {
            a.kind
                .cmp(&b.kind)
                .then(a.folded.cmp(&b.folded))
                .then(if windowed {
                    b.time.cmp(&a.time)
                } else {
                    Ordering::Equal
                })
                .then(self.degree[b.index as usize].cmp(&self.degree[a.index as usize]))
                .then(a.index.cmp(&b.index))
        });
        let total = found.len() as u64;
        let exact_total = found.iter().filter(|hit| hit.exact).count() as u64;
        let whole_total = found
            .iter()
            .filter(|hit| matches!(hit.kind, MatchKind::WholeName | MatchKind::WholeAlias))
            .count() as u64;
        let mut matches: Vec<NodeMatchV3<'_>> = Vec::new();
        for hit in found
            .iter()
            .take(usize::try_from(request.limit()).unwrap_or(usize::MAX))
        {
            let id = self.node_ids[hit.index as usize];
            let node = &graph.nodes[&id];
            let Some(declared) = graph.ontology.node_type(node.type_id) else {
                return Err(ProjectError::Inconsistent(format!(
                    "node {id} is of type {}, which revision {} does not declare",
                    node.type_id, graph.revision
                ))
                .into());
            };
            matches.push(NodeMatchV3 {
                id,
                type_id: node.type_id,
                type_name: &declared.name,
                name: &node.canonical_name,
                degree: self.degree[hit.index as usize],
                time: hit.time,
                kind: hit.kind,
                folded: hit.folded,
                field: hit.field,
                alias: hit.alias,
            });
        }
        let first_match = matches.first().map(|hit| hit.id);
        let first_kind = matches.first().map(|hit| hit.kind.name());
        let first_folded = matches.first().map(|hit| hit.folded);
        let document = NodeMatchesV3 {
            meta: MatchesMetaV3 {
                format: MATCHES_V3_FORMAT,
                revision: graph.revision.get(),
                text,
                types: &types,
                from: filter.from,
                to: filter.to,
                total,
                exact_total,
                whole_total,
            },
            matches,
        };
        let bytes = encode(&document)?;
        let summary = NodesSearched {
            revision: document.meta.revision,
            text: text.to_owned(),
            matches: document.matches.len() as u64,
            total,
            exact_total,
            format: MATCHES_V3_FORMAT,
            first_match,
            first_tier: None,
            matches_hash: hash(&bytes),
            whole_total: Some(whole_total),
            first_kind,
            first_folded,
        };
        Ok(Answer { bytes, summary })
    }
}

#[cfg(test)]
mod tests {
    use super::{civil_day, runs, MatchesFormat, WindowBound, DAY_MS};

    /// ekr-mcp-fixes-20261015 #9: a date names its whole UTC day, milliseconds are read as they
    /// are, and nothing else is a bound.
    #[test]
    fn a_window_bound_is_milliseconds_or_a_whole_utc_day() {
        // 2027-01-10T00:00:00Z.
        let tenth = 1_799_539_200_000;
        assert_eq!(WindowBound::From.read("2027-01-10"), Ok(tenth));
        assert_eq!(WindowBound::To.read("2027-01-10"), Ok(tenth + DAY_MS - 1));
        assert_eq!(
            WindowBound::From.read("1799568000000"),
            Ok(1_799_568_000_000)
        );
        assert_eq!(WindowBound::To.read("-5"), Ok(-5));
        assert_eq!(WindowBound::From.read("1970-01-01"), Ok(0));
        assert_eq!(WindowBound::To.read("1969-12-31"), Ok(-1));
        assert_eq!(civil_day("2000-02-29"), Some(11_016));
        assert_eq!(civil_day("2024-03-01"), Some(19_783));
        for text in [
            "",
            "-",
            "2027-1-10",
            "2027-02-29",
            "1900-02-29",
            "2027-13-01",
            "2027-00-10",
            "2027-01-32",
            "2027-01-00",
            "2027/01/10",
            "+12",
            "1.5",
            " 12",
            "99999999999999999999",
            "２０２７-01-10",
        ] {
            assert!(WindowBound::From.read(text).is_err(), "{text:?}");
            assert!(WindowBound::To.read(text).is_err(), "{text:?}");
        }
        assert_eq!(
            WindowBound::To.read("x").unwrap_err().to_string(),
            "to \"x\" is neither milliseconds since the epoch nor a YYYY-MM-DD date"
        );
    }

    #[test]
    fn the_formats_are_named_by_their_literals() {
        for format in [MatchesFormat::V1, MatchesFormat::V2, MatchesFormat::V3] {
            assert_eq!(format.name().parse::<MatchesFormat>(), Ok(format));
        }
        assert_eq!(MatchesFormat::default(), MatchesFormat::V1);
        assert!("ekr.node-matches/4".parse::<MatchesFormat>().is_err());
    }

    /// ekr-mcp-fixes-20261015 #2: a Words match is whole when its alphanumeric runs are the
    /// text's, whatever separates them.
    #[test]
    fn whole_words_compare_alphanumeric_runs() {
        assert_eq!(runs("widget-router"), runs("router widget"));
        assert_ne!(runs("widget-router-2"), runs("widget router"));
        assert_eq!(runs("prod-disk-507"), runs("disk-507 prod"));
        assert!(runs("--").is_empty());
    }
}
