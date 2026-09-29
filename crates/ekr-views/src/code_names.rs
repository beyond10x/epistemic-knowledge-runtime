//! `ekr.code-names/1` (`ekr.views.FindCodeNames`): which of a revision's names a consumer's
//! source files carry as literals, so the consumer can hold its store-reading code generic over
//! any ontology.
//!
//! [`literals`] finds the literals of one text, [`code_names`] answers from a loaded revision, and
//! [`find_code_names`] loads the revision first. The rules — what a literal is, which names count,
//! which are exempt, and the document's order — are `systems/ekr/domains/views.yaml`'s, at
//! `ekr.views.CodeNamesV1`. Nothing here reads a file: the host reads each source and passes its
//! text as a [`SourceText`].

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use serde::Serialize;

use crate::query::{encode, hash, Answer};
use crate::{load, LoadedRevision, ProjectError};

/// The format literal every answer carries in `meta.format`.
pub const CODE_NAMES_FORMAT: &str = "ekr.code-names/1";

/// The runtime's own vocabulary, ascending: every field name, enum variant, union tag and union
/// variant name the ESS domains under `systems/ekr/domains` declare. A store name equal to one of
/// these is exempt: a reader of the runtime's documents names it whatever the ontology.
/// `tests/code_names.rs` holds this list to those files, entry for entry.
pub const RUNTIME_VOCABULARY: &[&str] = &[
    "Accepted",
    "Active",
    "AddAlias",
    "AddAssertion",
    "AddEvidence",
    "Alias",
    "AlreadyRecorded",
    "Ambiguous",
    "Any",
    "ApiResponse",
    "Assertion",
    "AssertionAdded",
    "AssertionRetracted",
    "AssertionSuperseded",
    "Attempt",
    "Authorization",
    "Blob",
    "Boolean",
    "Bootstrap",
    "Cache",
    "Canonical",
    "CanonicalName",
    "Cardinality",
    "Commit",
    "Committed",
    "Complete",
    "Conflict",
    "CreateEdge",
    "CreateNode",
    "DatabaseRecord",
    "Day",
    "Decimal",
    "DefineEdgeType",
    "DefineNodeType",
    "DeleteEdge",
    "Disputed",
    "Document",
    "Duration",
    "Edge",
    "EdgeCreated",
    "EdgeType",
    "Enum",
    "Ephemeral",
    "Evidence",
    "Exact",
    "Failed",
    "FeedItem",
    "Float",
    "Folded",
    "GitDiff",
    "GraphAssertion",
    "GraphFragment",
    "HumanStatement",
    "Incubating",
    "Integer",
    "Invoke",
    "Lifecycle",
    "List",
    "Many",
    "MergeEntity",
    "MessageBatch",
    "ModifyProperty",
    "Name",
    "NoStream",
    "Node",
    "NodeCreated",
    "NodeRef",
    "NodeType",
    "Observation",
    "One",
    "OntologyConstraint",
    "Partial",
    "Property",
    "Proposal",
    "Propose",
    "ProposeNew",
    "Proposed",
    "Provenance",
    "Record",
    "Reference",
    "Refused",
    "Rejected",
    "Relation",
    "Resolved",
    "RetractAssertion",
    "Retracted",
    "Revision",
    "RevisionCommitted",
    "Seed",
    "Seeded",
    "Stale",
    "String",
    "Structural",
    "SupersedeAssertion",
    "Superseded",
    "Timestamp",
    "TransactionProposed",
    "TransactionRejected",
    "TransactionStale",
    "TransactionTime",
    "TransactionValidated",
    "Transient",
    "Type",
    "UnknownCommit",
    "UpdateProperty",
    "Url",
    "ValidTime",
    "Validate",
    "Validated",
    "Validating",
    "Validation",
    "Value",
    "Week",
    "WidenEdgeType",
    "Written",
    "absorbed",
    "abstract_type",
    "active",
    "actor",
    "added",
    "after",
    "against",
    "agent_root",
    "agents",
    "alias",
    "aliases",
    "allowed_types",
    "appends",
    "application",
    "arguments",
    "assertion",
    "assertion_count",
    "assertion_id",
    "assertions",
    "assertions_added",
    "assertions_retracted",
    "assertions_superseded",
    "assessment",
    "at",
    "at_revision",
    "attempt_number",
    "authority",
    "authority_root",
    "basis",
    "binding",
    "blobs",
    "bucket",
    "bucket_ms",
    "buckets",
    "by",
    "byte_len",
    "bytes",
    "candidates",
    "canonical_bytes",
    "canonical_name",
    "canonical_names",
    "canonical_operations_hash",
    "canonical_transaction_hash",
    "capabilities",
    "cardinality",
    "cardinality-narrowed",
    "carried_objects",
    "causation_depth",
    "causation_id",
    "cells",
    "change",
    "changes",
    "changes_hash",
    "checked_through",
    "checkpoint_hash",
    "checks",
    "claim",
    "code",
    "code_names",
    "code_names_hash",
    "column",
    "command_key",
    "committed_at",
    "committer",
    "competing_assertions",
    "completed",
    "confidence_bp",
    "constraint-changed",
    "constraints",
    "content_hash",
    "context",
    "count",
    "covered",
    "created",
    "created_at",
    "current",
    "data",
    "database",
    "dated_assertions",
    "decision",
    "degree",
    "depth",
    "destination_record_hash",
    "destination_seed_hash",
    "detail",
    "detail_hash",
    "digest",
    "distance",
    "document_bytes",
    "document_hash",
    "document_id",
    "edge",
    "edge-endpoint-removed",
    "edge_assertions",
    "edge_count",
    "edge_limit",
    "edge_total",
    "edge_type",
    "edge_types",
    "edges",
    "edges_created",
    "effective_from",
    "ekr.cli-host/1",
    "ekr.code-names/1",
    "ekr.graph-changes/1",
    "ekr.graph-overview/1",
    "ekr.graph-projection/1",
    "ekr.graph-slice/1",
    "ekr.graph-timeline/1",
    "ekr.node-detail/1",
    "ekr.node-matches/1",
    "ekr.publication-preparation/1",
    "ekr.publication-preparation/2",
    "ekr.publication-preparation/3",
    "ekr.transaction-document/1",
    "ekr.transaction-document/2",
    "element",
    "emits",
    "empty-schema-change",
    "end",
    "event",
    "event_id",
    "event_types",
    "events",
    "evidence",
    "evidence_count",
    "evidence_hash",
    "evidence_payloads",
    "evidence_root",
    "exact_total",
    "exempt",
    "expected",
    "expected_basis",
    "expected_version",
    "extracted_by",
    "field",
    "fields",
    "file",
    "files",
    "findings",
    "first",
    "first_file",
    "first_line",
    "first_match",
    "first_revision",
    "first_row",
    "format",
    "forward",
    "from",
    "graph",
    "graph_root_id",
    "head",
    "hops",
    "id",
    "idempotency_key",
    "identity",
    "incoherent-schema",
    "initial",
    "input",
    "input_hash",
    "instant",
    "into",
    "inverse",
    "issues",
    "judged",
    "key",
    "kind",
    "knowledge_root",
    "last",
    "last_revision",
    "legacy_objects",
    "lifecycle",
    "limit",
    "line",
    "links",
    "listed_events",
    "literal",
    "literals",
    "locator",
    "map_hash",
    "matches",
    "matches_hash",
    "matching_assertions",
    "maximum",
    "message",
    "meta",
    "minimum",
    "name",
    "names",
    "native_fingerprint",
    "native_request",
    "neighbour_types",
    "neighbours",
    "next",
    "node",
    "node_count",
    "node_id",
    "node_total",
    "node_types",
    "nodes",
    "nodes_created",
    "not-a-successor",
    "number",
    "object",
    "object_kind",
    "object_ref",
    "object_value",
    "objects",
    "observation",
    "observation_id",
    "observation_type",
    "observed_at",
    "observed_event_id",
    "observed_record_hash",
    "observed_revision_id",
    "observed_root",
    "observed_root_hash",
    "occurred_at_offset_seconds",
    "occurred_at_unix_nanos",
    "occurrences",
    "ontology",
    "ontology_root",
    "operation",
    "operation_count",
    "operations",
    "operations_hash",
    "operator",
    "overview",
    "overview_hash",
    "owner",
    "parameter",
    "parent",
    "parents",
    "path",
    "payload",
    "preconditions",
    "predecessor_event_id",
    "predecessor_record_hash",
    "predicate",
    "predicate_kind",
    "preparation_hash",
    "previous_attempt_hash",
    "previous_event_id",
    "previous_record_hash",
    "previous_revision_id",
    "previous_root",
    "previous_root_hash",
    "projection",
    "projection_hash",
    "properties",
    "property",
    "property-removed",
    "proposal",
    "proposal_record_hash",
    "proposed_by",
    "proposed_event_id",
    "proposer",
    "proposer_separation",
    "props",
    "provenance",
    "reached",
    "reason",
    "receipt",
    "record_hash",
    "recorded_at",
    "recorded_from",
    "recorded_to",
    "reference",
    "reference-type-has-subtypes",
    "reference-type-undeclared",
    "reference-without-identity",
    "referencing",
    "rejected_at",
    "remaining",
    "removed",
    "request_hash",
    "request_id",
    "requested",
    "requested_basis",
    "required",
    "required-property-missing",
    "result",
    "result_hash",
    "retained",
    "retained_evidence",
    "retracted_assertions",
    "revision",
    "revision_id",
    "revision_zero_assertions",
    "revision_zero_edges",
    "revision_zero_nodes",
    "revisions",
    "roles",
    "root",
    "root_id",
    "row_events",
    "row_type",
    "row_types",
    "rows",
    "ruleset",
    "schema",
    "schema-change-without-effect",
    "schema-version-exhausted",
    "schema-version-reused",
    "schema_version",
    "schema_version_id",
    "schema_versions",
    "scope",
    "section",
    "seed_document",
    "seed_hash",
    "seeds",
    "since",
    "since_kind",
    "since_recorded",
    "since_revision",
    "since_valid",
    "slice",
    "slice_hash",
    "source",
    "source_native_id",
    "source_record_hash",
    "source_seed_hash",
    "source_types",
    "sources",
    "space",
    "stale_at",
    "start",
    "state",
    "states",
    "status",
    "storage_class",
    "store",
    "stored_at",
    "stream",
    "stream_id",
    "stream_type",
    "strip",
    "subject",
    "subject_kind",
    "subjects",
    "submitted_at",
    "submitter",
    "symmetric",
    "table",
    "target",
    "target_types",
    "tenant",
    "text",
    "tier",
    "timeline",
    "timeline_buckets",
    "timeline_hash",
    "timestamped",
    "to",
    "top",
    "total",
    "trace_id",
    "transaction",
    "transaction_document",
    "transaction_hash",
    "transaction_id",
    "transaction_time",
    "transactions",
    "transition",
    "transitions",
    "transitive",
    "type",
    "type-already-declared",
    "type-declaration-changed",
    "type-removed",
    "type_id",
    "type_name",
    "type_state",
    "types",
    "undated",
    "undated_assertions",
    "unknown-edge-type",
    "unknown-endpoint-type",
    "unknown-property-owner",
    "unrecorded_edges",
    "unrecorded_nodes",
    "url",
    "valid_at",
    "valid_from",
    "valid_time",
    "valid_to",
    "validated_against",
    "validated_at",
    "validation",
    "validation_hash",
    "validation_profile",
    "validation_profile_hash",
    "validation_record_hash",
    "validator",
    "validators",
    "value",
    "value-kind-not-admitted",
    "value-type-narrowed",
    "value_kind",
    "value_type",
    "values",
    "variants",
    "version",
    "versions",
    "weight",
    "within_hour",
];

/// One source file as the host read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceText {
    /// The path exactly as the caller gave it, never resolved.
    pub path: String,
    /// The file's whole text.
    pub text: String,
}

/// One literal of a text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Literal {
    /// The 1-based line, lines split at `\n`.
    pub line: u64,
    /// The 1-based position of the opening quote on its line, in Unicode scalar values.
    pub column: u64,
    /// The raw text between the quotes; no escape is decoded.
    pub text: String,
}

/// Every literal of `text`, ordered by line then column: for each of `"`, `'` and a backtick
/// separately, the text between an opening quote and the next unescaped quote of the same
/// character on the same line, paired left to right. A backslash escapes the character after it;
/// a quote with no partner on its line opens nothing.
#[must_use]
pub fn literals(text: &str) -> Vec<Literal> {
    let mut found = Vec::new();
    for (index, line) in text.split('\n').enumerate() {
        let chars: Vec<char> = line.chars().collect();
        for quote in ['"', '\'', '`'] {
            let mut at = 0;
            while at < chars.len() {
                if chars[at] != quote {
                    at += 1;
                    continue;
                }
                match closing(&chars, at + 1, quote) {
                    Some(end) => {
                        found.push(Literal {
                            line: index as u64 + 1,
                            column: at as u64 + 1,
                            text: chars[at + 1..end].iter().collect(),
                        });
                        at = end + 1;
                    }
                    None => at += 1,
                }
            }
        }
    }
    found.sort_by_key(|literal| (literal.line, literal.column));
    found
}

/// The index of the first `quote` at or after `from` that no backslash escapes.
fn closing(chars: &[char], from: usize, quote: char) -> Option<usize> {
    let mut at = from;
    while at < chars.len() {
        match chars[at] {
            '\\' => at += 2,
            found if found == quote => return Some(at),
            _ => at += 1,
        }
    }
    None
}

/// `ekr.views.CodeNamesFound`: what one answer returned, every field a function of its bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CodeNamesFound {
    /// `meta.revision`.
    pub revision: u64,
    /// `meta.files`.
    pub files: u64,
    /// `meta.literals`.
    pub literals: u64,
    /// `meta.exempt`.
    pub exempt: u64,
    /// `meta.findings`.
    pub findings: u64,
    /// Names of kind `NodeType` over every finding.
    pub node_types: u64,
    /// Names of kind `EdgeType` over every finding.
    pub edge_types: u64,
    /// Names of kind `Property` over every finding.
    pub properties: u64,
    /// Names of kind `CanonicalName` over every finding.
    pub canonical_names: u64,
    /// Names of kind `Alias` over every finding.
    pub aliases: u64,
    /// The first finding's file, if there is one.
    pub first_file: Option<String>,
    /// The first finding's line, if there is one.
    pub first_line: Option<u64>,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub code_names_hash: String,
}

/// `ekr.views.CodeNameKind`, ordered as it declares its variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
enum Kind {
    NodeType,
    EdgeType,
    Property,
    CanonicalName,
    Alias,
}

/// `ekr.views.CodeNameMatch`, ordered by kind then id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct Match {
    kind: Kind,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    type_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    type_name: Option<String>,
}

#[derive(Serialize)]
struct Meta {
    format: &'static str,
    revision: u64,
    files: u64,
    literals: u64,
    exempt: u64,
    findings: u64,
}

#[derive(Serialize)]
struct Finding<'a> {
    file: &'a str,
    line: u64,
    column: u64,
    literal: String,
    names: &'a BTreeSet<Match>,
}

#[derive(Serialize)]
struct Document<'a> {
    meta: Meta,
    findings: Vec<Finding<'a>>,
}

/// Every name `loaded` holds, with what it names, and the text of every id it holds.
fn names(loaded: &LoadedRevision) -> (BTreeMap<String, BTreeSet<Match>>, BTreeSet<String>) {
    let graph = &loaded.graph;
    let mut names: BTreeMap<String, BTreeSet<Match>> = BTreeMap::new();
    let mut name = |text: &str, found: Match| {
        if !text.trim().is_empty() {
            names.entry(text.to_owned()).or_default().insert(found);
        }
    };
    let plain = |kind: Kind, id: String| Match {
        kind,
        id,
        type_id: None,
        type_name: None,
    };
    let ontology = graph.ontology.to_document();
    let mut type_names = BTreeMap::new();
    for declared in &ontology.node_types {
        type_names.insert(declared.id, declared.name.clone());
        name(
            &declared.name,
            plain(Kind::NodeType, declared.id.to_string()),
        );
        for property in declared.properties.values() {
            name(
                &property.name,
                plain(Kind::Property, property.id.to_string()),
            );
        }
    }
    for declared in &ontology.edge_types {
        name(
            &declared.name,
            plain(Kind::EdgeType, declared.id.to_string()),
        );
        for property in declared.properties.values() {
            name(
                &property.name,
                plain(Kind::Property, property.id.to_string()),
            );
        }
    }
    for node in graph.nodes.values() {
        let named = |kind: Kind| Match {
            kind,
            id: node.id.to_string(),
            type_id: Some(node.type_id.to_string()),
            type_name: type_names.get(&node.type_id).cloned(),
        };
        name(&node.canonical_name, named(Kind::CanonicalName));
        for alias in &node.aliases {
            name(alias, named(Kind::Alias));
        }
    }

    let mut ids = BTreeSet::new();
    ids.insert(graph.root.id.to_string());
    for (version, (_, schema)) in &loaded.schemas {
        ids.insert(version.to_string());
        let schema = schema.to_document();
        for declared in &schema.node_types {
            ids.insert(declared.id.to_string());
            ids.extend(declared.properties.keys().map(ToString::to_string));
        }
        for declared in &schema.edge_types {
            ids.insert(declared.id.to_string());
            ids.extend(declared.properties.keys().map(ToString::to_string));
        }
    }
    for entry in &loaded.revisions {
        ids.insert(entry.schema_version.to_string());
        ids.extend(entry.transaction_id.map(|id| id.to_string()));
    }
    for declared in &ontology.node_types {
        ids.insert(declared.id.to_string());
        ids.extend(declared.properties.keys().map(ToString::to_string));
    }
    for declared in &ontology.edge_types {
        ids.insert(declared.id.to_string());
        ids.extend(declared.properties.keys().map(ToString::to_string));
    }
    ids.extend(graph.nodes.keys().map(ToString::to_string));
    ids.extend(graph.edges.keys().map(ToString::to_string));
    for (id, assertion) in &graph.assertions {
        ids.insert(id.to_string());
        ids.insert(assertion.proposed_by.to_string());
    }
    for (id, evidence) in &graph.evidence {
        ids.insert(id.to_string());
        ids.insert(evidence.extracted_by.to_string());
    }
    (names, ids)
}

/// Whether the store name `name` is exempt: the runtime's vocabulary or the text of a store id.
fn exempt(name: &str, ids: &BTreeSet<String>) -> bool {
    RUNTIME_VOCABULARY.binary_search(&name).is_ok() || ids.contains(&name.to_lowercase())
}

/// Answers `ekr.views.FindCodeNames` from `loaded`: the `ekr.code-names/1` document of
/// `sources` against its names, and its counts.
///
/// Pure: the same revision and the same sources answer the same bytes, whatever order the
/// sources came in. A path given twice is read once, with the text it was first given.
///
/// # Errors
///
/// [`ProjectError::Inconsistent`] when the document does not encode.
pub fn code_names(
    loaded: &LoadedRevision,
    sources: &[SourceText],
) -> Result<Answer<CodeNamesFound>, ProjectError> {
    let (names, ids) = names(loaded);
    let mut read: Vec<&SourceText> = Vec::with_capacity(sources.len());
    for source in sources {
        if !read.iter().any(|earlier| earlier.path == source.path) {
            read.push(source);
        }
    }
    read.sort_by(|a, b| a.path.cmp(&b.path));
    let (mut literal_count, mut exempt_count) = (0_u64, 0_u64);
    let mut findings = Vec::new();
    for source in &read {
        for literal in literals(&source.text) {
            literal_count += 1;
            let Some((name, matched)) = names.get_key_value(&literal.text) else {
                continue;
            };
            if exempt(name, &ids) {
                exempt_count += 1;
                continue;
            }
            findings.push(Finding {
                file: &source.path,
                line: literal.line,
                column: literal.column,
                literal: literal.text,
                names: matched,
            });
        }
    }
    let document = Document {
        meta: Meta {
            format: CODE_NAMES_FORMAT,
            revision: loaded.graph.revision.get(),
            files: read.len() as u64,
            literals: literal_count,
            exempt: exempt_count,
            findings: findings.len() as u64,
        },
        findings,
    };
    let bytes = encode(&document)?;
    let count = |kind: Kind| {
        document
            .findings
            .iter()
            .flat_map(|finding| finding.names.iter())
            .filter(|found| found.kind == kind)
            .count() as u64
    };
    let summary = CodeNamesFound {
        revision: document.meta.revision,
        files: document.meta.files,
        literals: document.meta.literals,
        exempt: document.meta.exempt,
        findings: document.meta.findings,
        node_types: count(Kind::NodeType),
        edge_types: count(Kind::EdgeType),
        properties: count(Kind::Property),
        canonical_names: count(Kind::CanonicalName),
        aliases: count(Kind::Alias),
        first_file: document
            .findings
            .first()
            .map(|finding| finding.file.to_owned()),
        first_line: document.findings.first().map(|finding| finding.line),
        code_names_hash: hash(&bytes),
    };
    Ok(Answer { bytes, summary })
}

/// Loads revision `at` of `runtime`'s store (the head when `None`) and answers
/// `ekr.views.FindCodeNames` for `sources`: [`load`] then [`code_names`]. Reads only.
///
/// # Errors
///
/// Whatever [`load`] or [`code_names`] refuses: [`ProjectError::NotSeeded`] and
/// [`ProjectError::RevisionNotFound`] among them.
pub fn find_code_names(
    runtime: &Runtime,
    at: Option<RevisionNumber>,
    sources: &[SourceText],
) -> Result<Answer<CodeNamesFound>, ProjectError> {
    code_names(&load(runtime, at)?, sources)
}
