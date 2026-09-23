//! The agent-facing text of the binary: the guide, the operation catalogue and the example
//! documents. Static text only; `crates/ekr/tests/agent_cli.rs` holds every example to the real
//! `ekr.transaction-document/1`, `ekr-seed/2` and `ekr.cli-host/1` readers.

use std::fmt::Write as _;

use clap::ValueEnum;
use ekr_kernel::GraphOperation;

/// The workflow, printed by `ekr guide`.
pub(super) const GUIDE: &str = "\
ekr — the Epistemic Knowledge Runtime from the command line

Agents propose; they never mutate. Every change is a transaction document that is proposed,
validated against a committed revision and then committed. Nothing reaches canonical state
any other way.

ROLES (from the host document, `ekr example ekr.cli-host/1`)
  operator   context.operator — proposes and commits. The document's `proposer` and every
             assertion's `proposed_by` must be this id, or propose refuses.
  validator  context.validator — the profile validator `ekr validate` runs as.

CONFIGURATION (every store verb)
  --host <ekr.cli-host/1 JSON>   or EKR_HOST
  --store <dir | sqlite file>    or EKR_STORE
  --backend <file | sqlite>      or EKR_BACKEND
  A flag wins over its variable; an empty variable counts as unset. guide, operations,
  example and mint need none of them and ignore the variables.

WORKFLOW
  1. ekr example ekr.cli-host/1 > host.json       a host document to start from
  2. ekr example ekr-seed/2 > seed.yaml           a seed: ontology, graph, evidence
     ekr seed seed.yaml                            revision 0
  3. ekr ontology                                  node types, edge types, properties: name and id
     ekr snapshot                                  nodes, edges, assertions, evidence at the head
  4. ekr mint node | edge | assertion | transaction | ...
                                                   fresh ids for everything you create
  5. ekr operations                                the operation kinds, one line each
     ekr operations <Kind>                         its fields and an example operation
     ekr example ekr.transaction-document/1        a complete transaction document
  6. ekr propose doc.yaml                          -> transaction_id (state Proposed)
  7. ekr validate <transaction_id>                 against the head (`ekr head`), or --against N
  8. ekr commit <transaction_id>                   -> a new revision
  9. ekr head                                      the head revision number and root
     ekr transactions [--state <State>]            retained transactions, id, state, proposer
     ekr snapshot [--at N] [--valid-at YYYY-MM-DD]  read the result back
     ekr explain <assertion_id>                    why an assertion is what it is

WHAT IS TRUE NOW
  ekr snapshot --valid-at <ms | YYYY-MM-DD> lists the assertions believed at that instant in
  `matching_assertions` (a date is midnight UTC). A plain `ekr snapshot` leaves
  matching_assertions null and returns every assertion, including retracted and superseded ones.

RELATIONS: CreateEdge, an assertion, or both
  A claim that a relation holds is an AddAssertion with `predicate: !Relation <edge type id>`,
  subject and object `!Node`. It carries evidence and a valid time, and it is what
  `snapshot --valid-at`, explain, RetractAssertion and SupersedeAssertion act on. The assertion
  alone is enough to state the claim.
  CreateEdge is the structural record of the relation: it has no evidence and no valid time, is
  held to the edge type's endpoint types and cardinality, and is removed with DeleteEdge. Add one
  when a reader of the graph's edges should see the relation too.

ASSESSMENT: Proposed -> Accepted
  Write every assertion with `assessment: Proposed`. An assertion written with any other
  assessment states its own verdict: propose records it (exit 0) and validation rejects it with
  the issue code assertion-states-its-own-verdict.
  Acceptance is judged at commit: committing a validated transaction makes each assertion it
  adds Accepted, with the validator's id, and seeding does the same for the seed's own
  assertions. RetractAssertion and SupersedeAssertion need an assertion that is accepted and
  active as of that commit, so one added earlier in the same transaction counts. An assertion
  added and retracted in the same transaction validates and commits, and reads back Accepted
  and Retracted.

NOT APPLIED IN P1
  DefineNodeType, DefineEdgeType, ModifyProperty and MergeEntity are not applied in P1: they
  parse, but validation rejects every proposal of them with the issue code
  unsupported-operation. The ontology is the seed's. `ekr operations` marks them.

WHERE VALUES COME FROM
  new ids          ekr mint <kind>; ids are never derived from names
  existing ids     ekr ontology (type ids), ekr snapshot (root, node, edge, assertion,
                   evidence ids)
  revision numbers ekr head; a commit prints its revision
  times            milliseconds since the Unix epoch (valid_time.from, effective_from);
                   transaction_time.recorded_from is written as 0 and set by the kernel
  evidence         a transaction's `evidence` list is exactly the evidence its assertions cite,
                   and each must already be retained (seeded)

EXIT CODES
  exit 0  a declared outcome, JSON on stdout. A validation that rejects and a commit that finds
          the head moved (Stale) are outcomes too: read `kind`.
  exit 1  a fault: provider, verification, unreadable input, host configuration, not seeded.
  exit 2  a named refusal (its ekr.kernel.* name on stderr, nothing recorded) or a usage error.

OUTPUT
  guide, operations and example print text; every other verb prints one JSON document.
  A proposal record's document_bytes (in the results of propose, commit and explain) prints as
  one standard padded base64 string (RFC 4648), not as a number array.
  To read the seed's evidence payloads, explain an assertion that cites them: `ekr explain` adds
  two fields to each Evidence link, `payload`, the evidence's retained bytes as one base64 string, and `text`, the
  same bytes as a string when they are valid UTF-8 (absent otherwise). No other verb prints a
  payload.
";

/// One `ekr.kernel.OperationKind`: a `GraphOperation` variant, by its YAML tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "verbatim")]
pub enum OperationKind {
    /// `!CreateNode`.
    CreateNode,
    /// `!UpdateProperty`.
    UpdateProperty,
    /// `!CreateEdge`.
    CreateEdge,
    /// `!DeleteEdge`.
    DeleteEdge,
    /// `!AddAssertion`.
    AddAssertion,
    /// `!RetractAssertion`.
    RetractAssertion,
    /// `!DefineNodeType`.
    DefineNodeType,
    /// `!DefineEdgeType`.
    DefineEdgeType,
    /// `!ModifyProperty`.
    ModifyProperty,
    /// `!MergeEntity`.
    MergeEntity,
    /// `!Invoke`.
    Invoke,
    /// `!SupersedeAssertion`.
    SupersedeAssertion,
}

impl OperationKind {
    /// The kind of a parsed operation. No `_` arm: a new `GraphOperation` variant does not
    /// compile until it has a kind here, and then until that kind has text below.
    #[must_use]
    pub const fn of(operation: &GraphOperation) -> Self {
        match operation {
            GraphOperation::CreateNode(_) => Self::CreateNode,
            GraphOperation::UpdateProperty(_) => Self::UpdateProperty,
            GraphOperation::CreateEdge(_) => Self::CreateEdge,
            GraphOperation::DeleteEdge(_) => Self::DeleteEdge,
            GraphOperation::AddAssertion(_) => Self::AddAssertion,
            GraphOperation::RetractAssertion(_) => Self::RetractAssertion,
            GraphOperation::DefineNodeType(_) => Self::DefineNodeType,
            GraphOperation::DefineEdgeType(_) => Self::DefineEdgeType,
            GraphOperation::ModifyProperty(_) => Self::ModifyProperty,
            GraphOperation::MergeEntity(_) => Self::MergeEntity,
            GraphOperation::Invoke { .. } => Self::Invoke,
            GraphOperation::SupersedeAssertion(_) => Self::SupersedeAssertion,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::CreateNode => "CreateNode",
            Self::UpdateProperty => "UpdateProperty",
            Self::CreateEdge => "CreateEdge",
            Self::DeleteEdge => "DeleteEdge",
            Self::AddAssertion => "AddAssertion",
            Self::RetractAssertion => "RetractAssertion",
            Self::DefineNodeType => "DefineNodeType",
            Self::DefineEdgeType => "DefineEdgeType",
            Self::ModifyProperty => "ModifyProperty",
            Self::MergeEntity => "MergeEntity",
            Self::Invoke => "Invoke",
            Self::SupersedeAssertion => "SupersedeAssertion",
        }
    }

    /// Whether the P1 kernel applies this kind. The four it does not are rejected at validation
    /// with `unsupported-operation` (`crates/ekr-kernel/src/validate/structural.rs`), and
    /// `apply.rs` refuses them the same way. No `_` arm: a new kind states its own answer.
    const fn applied_in_p1(self) -> bool {
        match self {
            Self::CreateNode
            | Self::UpdateProperty
            | Self::CreateEdge
            | Self::DeleteEdge
            | Self::AddAssertion
            | Self::RetractAssertion
            | Self::Invoke
            | Self::SupersedeAssertion => true,
            Self::DefineNodeType
            | Self::DefineEdgeType
            | Self::ModifyProperty
            | Self::MergeEntity => false,
        }
    }

    /// `(summary, fields, example operation)`. The example is one entry of
    /// `transaction.operations`, using the ids of `ekr example ekr-seed/2`.
    #[allow(clippy::too_many_lines)]
    const fn text(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::CreateNode => (
                "create a node of a declared node type",
                "  id              NodeId       new: ekr mint node
  root_id         GraphRootId  the graph root: ekr snapshot graph.graph.root.id
  type_id         TypeId       a node type: ekr ontology node_types[].id
  canonical_name  String       the name a reader sees (a property, not an identity)
  properties      map PropertyId -> [Value]   {} when none",
                "- !CreateNode
  id: 00000000-0000-4000-8000-000000000304
  root_id: 00000000-0000-4000-8000-000000000002
  type_id: 00000000-0000-4000-8000-000000000202
  canonical_name: Globex
  properties: {}",
            ),
            Self::UpdateProperty => (
                "set the values of one property of one node",
                "  node      NodeId      an existing node
  property  PropertyId  a property its type declares: ekr ontology
  values    [Value]     the values afterwards; [] clears it. A Value is
                        {value_kind: String|Boolean|Integer|Float|Decimal|Timestamp|Duration|
                         NodeRef|Enum|List|Record, value: ...}",
                "- !UpdateProperty
  node: 00000000-0000-4000-8000-000000000303
  property: 00000000-0000-4000-8000-000000000801
  values:
  - value_kind: String
    value: Acme Holdings Ltd",
            ),
            Self::CreateEdge => (
                "create an edge of a declared edge type between two nodes",
                "  id          EdgeId       new: ekr mint edge
  root_id     GraphRootId  the graph root
  type_id     TypeId       an edge type: ekr ontology edge_types[].id
  source      NodeId       a node of one of the type's source_types
  target      NodeId       a node of one of the type's target_types
  properties  map PropertyId -> [Value]   {} when none",
                "- !CreateEdge
  id: 00000000-0000-4000-8000-000000000701
  root_id: 00000000-0000-4000-8000-000000000002
  type_id: 00000000-0000-4000-8000-000000000203
  source: 00000000-0000-4000-8000-000000000302
  target: 00000000-0000-4000-8000-000000000303
  properties: {}",
            ),
            Self::DeleteEdge => (
                "remove an edge (a claim that it held is retracted, not deleted)",
                "  (the value)  EdgeId  an existing edge",
                "- !DeleteEdge 00000000-0000-4000-8000-000000000700",
            ),
            Self::AddAssertion => (
                "add an assertion, citing retained evidence",
                "  id                AssertionId  new: ekr mint assertion
  root_id           GraphRootId  the graph root
  subject           !Node NodeId | !Edge EdgeId | !Type TypeId
  predicate         !Relation TypeId (an edge type) | !Property PropertyId
  object            !Node NodeId | !Type TypeId | !Value Value
  evidence          [EvidenceId] retained evidence: ekr snapshot graph.graph.evidence;
                                 also listed in transaction.evidence
  proposed_by       AgentId      the host operator
  assessment        Proposed
  lifecycle         Active
  valid_time        {from: ms | null, to: ms | null}
  transaction_time  {recorded_from: 0, recorded_to: null}",
                "- !AddAssertion
  id: 00000000-0000-4000-8000-000000000501
  root_id: 00000000-0000-4000-8000-000000000002
  subject: !Node 00000000-0000-4000-8000-000000000301
  predicate: !Relation 00000000-0000-4000-8000-000000000203
  object: !Node 00000000-0000-4000-8000-000000000303
  evidence:
  - 00000000-0000-4000-8000-000000000401
  proposed_by: 00000000-0000-4000-8000-000000000101
  assessment: Proposed
  lifecycle: Active
  valid_time:
    from: 1577836800000
    to: null
  transaction_time:
    recorded_from: 0
    recorded_to: null",
            ),
            Self::RetractAssertion => (
                "withdraw an assertion with a reason; it is kept, not erased",
                "  assertion  AssertionId  an existing assertion
  reason     String       why",
                "- !RetractAssertion
  assertion: 00000000-0000-4000-8000-000000000512
  reason: recorded in error; Bob became CEO on 2026-03-12",
            ),
            Self::DefineNodeType => (
                "declare a node type",
                "  id             TypeId      new: ekr mint type
  name           String
  parents        [TypeId]    types it specialises
  properties     map PropertyId -> PropertyDefinition (see ModifyProperty)
  abstract_type  bool
  lifecycle      null | {initial, states, transitions}
  operations     map String -> {name, ...}",
                "- !DefineNodeType
  id: 00000000-0000-4000-8000-000000000204
  name: Project
  parents: []
  properties: {}
  abstract_type: false
  lifecycle: null
  operations: {}",
            ),
            Self::DefineEdgeType => (
                "declare an edge type",
                "  id            TypeId    new: ekr mint type
  name          String
  source_types  [TypeId]  node types an edge may start at (not empty)
  target_types  [TypeId]  node types an edge may end at (not empty)
  cardinality   One | Many   edges of this type per source
  properties    map PropertyId -> PropertyDefinition
  inverse       null | TypeId
  symmetric     bool
  transitive    bool",
                "- !DefineEdgeType
  id: 00000000-0000-4000-8000-000000000205
  name: WORKS_FOR
  source_types:
  - 00000000-0000-4000-8000-000000000201
  target_types:
  - 00000000-0000-4000-8000-000000000202
  cardinality: Many
  properties: {}
  inverse: null
  symmetric: false
  transitive: false",
            ),
            Self::ModifyProperty => (
                "redeclare a property definition",
                "  id           PropertyId  new: ekr mint property, or an existing one
  name         String
  value_type   {value_kind: String|Boolean|Integer|Float|Decimal|Timestamp|Duration}
               | {value_kind: NodeRef, parameters: {allowed_types: [TypeId]}}
               | {value_kind: Enum, parameters: {variants: [String]}}
               | {value_kind: List, parameters: <value_type>}
               | {value_kind: Record, parameters: {field: <value_type>}}
  cardinality  One | Many
  required     bool
  constraints  [String]    opaque; writes they apply to are refused",
                "- !ModifyProperty
  id: 00000000-0000-4000-8000-000000000801
  name: legal_name
  value_type:
    value_kind: String
  cardinality: One
  required: false
  constraints: []",
            ),
            Self::MergeEntity => (
                "hold two nodes to be one; `into` keeps its id",
                "  absorbed  NodeId  the node that stops being its own entity
  into      NodeId  the node that remains",
                "- !MergeEntity
  absorbed: 00000000-0000-4000-8000-000000000302
  into: 00000000-0000-4000-8000-000000000301",
            ),
            Self::Invoke => (
                "invoke a named operation the node's type declares",
                "  node       NodeId  the node it acts on
  operation  String  an operation name its type declares
  arguments  map String -> Value",
                "- !Invoke
  node: 00000000-0000-4000-8000-000000000303
  operation: record_review
  arguments:
    note:
      value_kind: String
      value: annual review done",
            ),
            Self::SupersedeAssertion => (
                "replace an accepted assertion from a valid-time instant on",
                "  assertion       AssertionId  the earlier assertion
  by              AssertionId  its replacement, possibly added in the same transaction
  effective_from  ms           the replacement's valid_time.from",
                "- !SupersedeAssertion
  assertion: 00000000-0000-4000-8000-000000000510
  by: 00000000-0000-4000-8000-000000000511
  effective_from: 1773273600000",
            ),
        }
    }
}

/// `ekr operations`: every kind, one line each.
pub(super) fn operation_list() -> String {
    let mut out = String::new();
    for kind in OperationKind::value_variants() {
        let (summary, _, _) = kind.text();
        let _ = write!(out, "{:<20}{summary}", kind.name());
        if !kind.applied_in_p1() {
            out.push_str(" [not applied in P1: validation rejects it as unsupported-operation]");
        }
        out.push('\n');
    }
    out
}

/// What a page says about a kind the P1 kernel does not apply.
const NOT_APPLIED: &str =
    "This kind is not applied in P1: validation rejects every proposal of this kind with \
the issue code unsupported-operation. The example shows the shape only and is not accepted today.";

/// `ekr operations <Kind>`: summary, fields and one example operation, last.
pub(super) fn operation(kind: OperationKind) -> String {
    let (summary, fields, example) = kind.text();
    let (status, heading) = if kind.applied_in_p1() {
        (
            String::new(),
            "Example (ids from ekr example ekr-seed/2; validates against a store seeded from it):",
        )
    } else {
        (
            format!("{NOT_APPLIED}\n\n"),
            "Example (ids from ekr example ekr-seed/2; not accepted today):",
        )
    };
    format!(
        "{name} — {summary}\n\n{status}Fields:\n{fields}\n\n\
         Put it in transaction.operations of an ekr.transaction-document/1 \
         (ekr example ekr.transaction-document/1).\n\
         {heading}\n{example}\n",
        name = kind.name()
    )
}

/// The three document formats an agent writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ExampleFormat {
    /// A proposal for `ekr propose`.
    #[value(name = "ekr.transaction-document/1", alias = "transaction")]
    TransactionDocument,
    /// A seed for `ekr seed`.
    #[value(name = "ekr-seed/2", alias = "seed")]
    Seed,
    /// The trusted host document for `--host`.
    #[value(name = "ekr.cli-host/1", alias = "host")]
    Host,
}

/// `ekr example <format>`: a complete document of that format.
pub(super) const fn example(format: ExampleFormat) -> &'static str {
    match format {
        ExampleFormat::TransactionDocument => include_str!("examples/transaction.yaml"),
        ExampleFormat::Seed => include_str!("examples/seed.yaml"),
        ExampleFormat::Host => include_str!("examples/host.json"),
    }
}

/// The id kinds `ekr mint` mints.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum IdKind {
    /// `NodeId`.
    Node,
    /// `EdgeId`.
    Edge,
    /// `AssertionId`.
    Assertion,
    /// `TransactionId`.
    Transaction,
    /// `EvidenceId`.
    Evidence,
    /// `TypeId`, for a node or edge type.
    Type,
    /// `PropertyId`.
    Property,
    /// `AgentId`.
    Agent,
    /// `GraphRootId`.
    GraphRoot,
    /// `SchemaVersionId`.
    SchemaVersion,
}

/// `ekr mint <kind>`'s result.
#[derive(serde::Serialize)]
pub(super) struct Minted {
    kind: &'static str,
    id: String,
}

/// A fresh id of `kind`, minted as that kind's own type.
pub(super) fn mint(kind: IdKind) -> Minted {
    use ekr_core::{
        AgentId, AssertionId, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId, SchemaVersionId,
        TransactionId, TypeId,
    };
    let (kind, id) = match kind {
        IdKind::Node => ("node", NodeId::mint().to_string()),
        IdKind::Edge => ("edge", EdgeId::mint().to_string()),
        IdKind::Assertion => ("assertion", AssertionId::mint().to_string()),
        IdKind::Transaction => ("transaction", TransactionId::mint().to_string()),
        IdKind::Evidence => ("evidence", EvidenceId::mint().to_string()),
        IdKind::Type => ("type", TypeId::mint().to_string()),
        IdKind::Property => ("property", PropertyId::mint().to_string()),
        IdKind::Agent => ("agent", AgentId::mint().to_string()),
        IdKind::GraphRoot => ("graph-root", GraphRootId::mint().to_string()),
        IdKind::SchemaVersion => ("schema-version", SchemaVersionId::mint().to_string()),
    };
    Minted { kind, id }
}
