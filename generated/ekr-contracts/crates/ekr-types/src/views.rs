// generated from ekr v1
// model digest 890aea90d130e26fabda8485a25a78599aa119258635fdea52179df20555f601
// contract digest 281bbd37905ec8f6f636fc68d1767f3895a88fb29b6dbd4e383ae18d0e714c43
// do not edit: regenerate with `ess synthesize`

//! Views — `ekr.views`.
//!
//! Read-only projections of one committed revision. A view is a deterministic function of a snapshot at a revision, a template version and a query scope; every rendered fact carries the id of the assertion it came from, and what a view renders is StorageClass Cache (design § 82). The full export is the graph projection ekr.graph-projection/1: one JSON document per committed revision, scoped to canonical state as of that revision (§ 47), carrying assertion and evidence ids for the explain chain (§ 62), the schema version lineage (§ 95) and evidence identities without their bytes (§ 86). The format names no domain concept: it is generic over any ontology, and every name a viewer shows is taken from the projection's ontology or graph state. Five bounded reads draw from the same revision without exporting it: ekr.graph-overview/1 (ProjectOverview), ekr.graph-slice/1 (ExpandNeighbourhood), ekr.node-detail/1 (DescribeNode), ekr.node-matches/1 (SearchNodes) and ekr.graph-timeline/1 (ProjectTimeline), each generic over any ontology in the same way. A sixth, ekr.graph-changes/1 (ChangesSince), reads what the revisions up to one revision changed after a revision or a time. ekr.store-quality/1 (ReportStoreQuality) reports how well one revision's assertions are evidenced, its properties constrained and its nodes of one type named apart. ekr.code-names/1 (FindCodeNames) reports which of a revision's names a consumer's source files carry as literals. ekr.ocel/1 (ExportOcel) exports one revision as an OCEL 2.0 object-centric event log. This domain writes nothing; its ten commands read. Two more, DrawFactSample and ReportFactQuality, sample facts for a judge and report the judged sample's pass rate (see the fact-quality paragraph below).
//! Determinism, which every renderer of ekr.graph-projection/1 holds to so that two renders of one revision are byte-identical. (1) Order. By id: every array whose members carry an `id`, and every array of ids, ascending in the lowercase hyphenated text of the UUID — ontology.node_types, ontology.edge_types, ontology.properties (the entries of one id by their first owner, see ekr.views.ProjectedProperty), each property's owners, each type's properties, source_types, target_types and assertions, nodes, edges, evidence, each node's and edge's assertions, each assertion's evidence, assessment.validators (a set) and each schema version's added. By number, ascending: schema.versions and schema.revisions. In the store's own order, because the order is part of the value: each node's aliases, each property's value list in props (an ordered list in which duplicates count), each ListValue's items, and assessment.issues and assessment.competing_assertions (ordered vectors, amendment 88). Map keys (props, a RecordValue's fields) ascend by their text. No other array appears in the format. (2) Key order: every object's keys appear in the order its type below declares its fields, and a union's tag key `kind` comes first. (3) Encoding: UTF-8 JSON with no insignificant whitespace; an Integer or a RevisionNumber is a JSON integer with no sign on zero, no leading zero, no fraction and no exponent; every time is an ekr.views.EpochMillis, a JSON integer of milliseconds since the Unix epoch in UTC, which is how the runtime stores a Timestamp; a Uuid is its lowercase hyphenated text; a Decimal is its exact text as the store holds it. (4) Presence: every required field is always emitted; an Optional field with no value is omitted, never emitted as null, and one with a value is always emitted; an empty list or map is emitted empty, never omitted. No field of this format is an Optional inside a list or a map. (5) No field carries a wall-clock time, a host, a path, the store's head or anything else that differs between two renders of one revision; a revision renders the same bytes before and after any later commit, so a cached or exported render and its content hash stay those of a fresh render of that revision. A host that needs the head reads it elsewhere.
//! Determinism of the five bounded formats, so that two answers to one request against one revision are byte-identical. Rules (2) key order, (3) encoding, (4) presence and (5) above hold for each of them as written for ekr.graph-projection/1, with "render" read as "answer to one request"; a ProjectedOntology, ProjectedAssertion, ProjectedEdge or ProjectedValue inside them is ordered as rule (1) orders it; and ids ascend in the lowercase hyphenated text of the UUID. Rule (1), order, is per format, and in each no array appears that is not named: ekr.graph-overview/1 — ontology as rule (1); schema.versions and schema.revisions by number ascending; each version's added and removed by id, then kind in the order ekr.views.SchemaMemberKind declares its variants; each version's widened by edge type id, a Source end before a Target end, and each widened end's node_types by id; each version's modified by owner id, then property id, and each one's changed in the order ekr.views.PropertyAspect declares its variants; node_types, edge_types and roles.types by type id; timeline.buckets by start ascending, then type id; top by degree descending, then id. ekr.graph-slice/1 — nodes and edges in record-sequence order (ekr.views.GraphSliceV1: distance ascending, degree descending, id ascending, each node followed by the edges it closes by id), which is not id order; meta.seeds and each edge's assertions by id. ekr.node-detail/1 — assertions, edges and neighbours by id; referencing by its assertion's id; node.aliases and each props value list in the store's order; props keys ascending by their text. ekr.node-matches/1 — matches by tier (Exact before Folded), then degree descending, then id; never by a name. ekr.graph-timeline/1 — row_types by weight / nodes descending, compared exactly as rationals, then type id; rows by total descending, then id; each row's cells and strip by start ascending, then type id; events by start ascending, then id; each event's path from the subject outward, which is not id order. ekr.graph-changes/1 — changes by revision ascending, then change kind in the order ekr.views.ChangeKind declares its variants, then id ascending; each change's evidence by id. Its answer is a function of the request with `at` resolved: a request that names no `at` reads the head as it stands at the request, and its answer is byte for byte the answer to the same request naming that revision, which meta.revision carries. meta.revision is the revision read, never a report of the head: the same request naming that revision answers the same bytes after any later commit. A reader that pages keeps `at` fixed to the first page's meta.revision. A refusal is outside that rule: a since_revision past the head is refused as RevisionNotFound naming the head, and after a later commit the same request may answer an empty page. Bounds and the order of refusals, for the six: a bound over the input (depth, hops, limit, edge_limit, after) is checked before the store is read, and a broken one answers LimitExceeded naming the first broken input in the command's declared input order — for ChangesSince a since_revision below 0 answers SinceMalformed before any bound, while a request naming none or two of the three since inputs is malformed input that never reaches the command (the endpoint answers invalid-query, the MCP tool -32602); then an unseeded store answers NotSeeded; then a revision beyond the head answers RevisionNotFound — for ChangesSince `at` first, then since_revision; then a node the revision does not hold answers NodeNotFound.
//! Evidence without retained bytes is unreachable. Evidence enters canonical state in two ways, and both carry its bytes: with the seed, whose envelope retains each payload, and through an ekr.kernel AddEvidence operation, which carries the payload beside the entry and whose commit stores it (decision-blocker:evidence-entry-after-seed, answered by option 1). The kernel's verified read also refuses a store whose evidence bytes are missing, rather than reading it (crates/ekr-kernel/tests/explain.rs, "A payload deleted from the provider refuses the read itself"). So `retained` is true for every evidence record a projection can be rendered from, and `retained: false` is unreachable. The flag is kept so the format does not change if a later decision admits evidence without bytes.
//! The store's quality, ekr.store-quality/1 (ReportStoreQuality), is a function of one revision and nothing else: it counts that revision's canonical state, and no refusal, rejected transaction or open ambiguity, none of which one revision holds. Rules (2) key order, (3) encoding, (4) presence and (5) above hold for it as written for ekr.graph-projection/1; rule (1), order: shared_names by type id, then name ascending by its UTF-8 bytes; each entry's nodes by id. No other array appears in it. A share is basis points: 10000 times the count, divided by the whole it is a share of and rounded down, so 10000 is all of it; a share of a whole of 0 is omitted. Every assertion the kernel admits cites evidence (ekr.kernel's provenance validator, `assertion-without-evidence`), and every cited record's bytes are retained, so with_evidence equals active in every store the kernel wrote; it is counted, not assumed, so a store that broke either rule would show it.
//! Fact quality by a judged sample: ekr.fact-sample/1 (DrawFactSample) and ekr.fact-quality/1 (ReportFactQuality). This domain writes nothing and judges nothing; DrawFactSample reads one revision, and ReportFactQuality reads no store. The draw. The population is the revision's Active assertions, those whose subject is of the requested type when one is named (a node subject's type, an edge subject's type, a Type subject's own id, compared exactly: a subtype is another type). Each is ranked by its draw key, the lowercase hex SHA-256 of the UTF-8 text `ekr.fact-sample/1:<seed>:<assertion id>`, the seed in decimal with a leading `-` when negative and the id lowercase hyphenated; the sample is the `size` lowest-ranked, all of them when the population is smaller. So one (seed, size, type, revision) draws one sample on every provider, a larger size draws a sample the smaller one's is a prefix of, and a later commit leaves the sample of an earlier revision as it was. Rules (2) key order, (3) encoding, (4) presence and (5) above hold for both formats as written for ekr.graph-projection/1; rule (1), order: items by draw key ascending, then assertion id, which is not id order; each item's evidence by id; a ProjectedAssertion inside an item as rule (1) orders it. ekr.fact-quality/1 holds no array. The report. z is computed from the confidence by Wichura's algorithm AS 241 (PPND16), taking the logarithm it needs as 2·atanh((m − 1)/(m + 1)) + e·ln 2 over the binary64 exponent e and mantissa m, summed from a fixed number of terms with only the operations IEEE 754 rounds exactly; rate, lower and upper follow from z by the formula ekr.views.FactQualityV1 states. Every host therefore answers the same binary64 values. A Decimal in ekr.fact-quality/1 is a JSON number: the shortest decimal text that reads back as the same binary64 value.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// AssertionQuality — `ekr.views.AssertionQuality`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionQuality {
    /// `active` — `Integer`.
    pub active: i64,
    /// `with_evidence` — `Integer`.
    pub with_evidence: i64,
    /// `with_item_evidence` — `Integer`.
    pub with_item_evidence: i64,
    /// `with_seed_evidence` — `Integer`.
    pub with_seed_evidence: i64,
    /// `with_evidence_share` — `Optional<ekr.views.BasisPoints>`.
    pub with_evidence_share: Option<BasisPoints>,
    /// `with_item_evidence_share` — `Optional<ekr.views.BasisPoints>`.
    pub with_item_evidence_share: Option<BasisPoints>,
}

/// BasisPoints — `ekr.views.BasisPoints`: a distinct wrapper around `Integer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasisPoints(pub i64);

/// BooleanValue — `ekr.views.BooleanValue`: a distinct wrapper around `Boolean`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BooleanValue(pub bool);

/// ChangeCursor — `ekr.views.ChangeCursor`: a distinct wrapper around `Integer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeCursor(pub i64);

/// ChangeKind — `ekr.views.ChangeKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// `NodeCreated`.
    NodeCreated,
    /// `EdgeCreated`.
    EdgeCreated,
    /// `AssertionAdded`.
    AssertionAdded,
    /// `AssertionSuperseded`.
    AssertionSuperseded,
    /// `AssertionRetracted`.
    AssertionRetracted,
    /// `EvidenceAdded`.
    EvidenceAdded,
}

/// ChangesMeta — `ekr.views.ChangesMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangesMeta {
    /// `format` — `ekr.views.GraphChangesFormatV1`.
    pub format: GraphChangesFormatV1,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `since_revision` — `Optional<ekr.kernel.RevisionNumber>`.
    pub since_revision: Option<crate::kernel::RevisionNumber>,
    /// `since_valid` — `Optional<ekr.views.EpochMillis>`.
    pub since_valid: Option<EpochMillis>,
    /// `since_recorded` — `Optional<ekr.views.EpochMillis>`.
    pub since_recorded: Option<EpochMillis>,
    /// `limit` — `Integer`.
    pub limit: i64,
    /// `after` — `ekr.views.ChangeCursor`.
    pub after: ChangeCursor,
    /// `total` — `Integer`.
    pub total: i64,
}

/// CodeNameFinding — `ekr.views.CodeNameFinding`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeNameFinding {
    /// `file` — `String`.
    pub file: String,
    /// `line` — `Integer`.
    pub line: i64,
    /// `column` — `Integer`.
    pub column: i64,
    /// `literal` — `String`.
    pub literal: String,
    /// `runtime_word` — `Boolean`.
    pub runtime_word: bool,
    /// `names` — `List<ekr.views.CodeNameMatch>`.
    pub names: Vec<CodeNameMatch>,
}

/// CodeNameKind — `ekr.views.CodeNameKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeNameKind {
    /// `NodeType`.
    NodeType,
    /// `EdgeType`.
    EdgeType,
    /// `Property`.
    Property,
    /// `CanonicalName`.
    CanonicalName,
    /// `Alias`.
    Alias,
}

/// CodeNameMatch — `ekr.views.CodeNameMatch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeNameMatch {
    /// `kind` — `ekr.views.CodeNameKind`.
    pub kind: CodeNameKind,
    /// `id` — `Uuid`.
    pub id: crate::primitives::Uuid,
    /// `type_id` — `Optional<ekr.ontology.TypeId>`.
    pub type_id: Option<crate::ontology::TypeId>,
    /// `type_name` — `Optional<String>`.
    pub type_name: Option<String>,
}

/// CodeNameMode — `ekr.views.CodeNameMode`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeNameMode {
    /// `Literals`.
    Literals,
    /// `Words`.
    Words,
}

/// CodeNamesFormatV1 — `ekr.views.CodeNamesFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeNamesFormatV1 {
    /// `EkrCodeNames1`.
    EkrCodeNames1,
}

/// CodeNamesMeta — `ekr.views.CodeNamesMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeNamesMeta {
    /// `format` — `ekr.views.CodeNamesFormatV1`.
    pub format: CodeNamesFormatV1,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `files` — `Integer`.
    pub files: i64,
    /// `literals` — `Integer`.
    pub literals: i64,
    /// `exempt` — `Integer`.
    pub exempt: i64,
    /// `findings` — `Integer`.
    pub findings: i64,
    /// `runtime_word_findings` — `Integer`.
    pub runtime_word_findings: i64,
    /// `mode` — `Optional<ekr.views.CodeNameMode>`.
    pub mode: Option<CodeNameMode>,
}

/// CodeNamesV1 — `ekr.views.CodeNamesV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeNamesV1 {
    /// `meta` — `ekr.views.CodeNamesMeta`.
    pub meta: CodeNamesMeta,
    /// `findings` — `List<ekr.views.CodeNameFinding>`.
    pub findings: Vec<CodeNameFinding>,
}

/// DetailMeta — `ekr.views.DetailMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailMeta {
    /// `format` — `ekr.views.NodeDetailFormatV1`.
    pub format: NodeDetailFormatV1,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
}

/// DetailNode — `ekr.views.DetailNode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailNode {
    /// `id` — `ekr.graph.NodeId`.
    pub id: crate::graph::NodeId,
    /// `name` — `String`.
    pub name: String,
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `aliases` — `List<String>`.
    pub aliases: Vec<String>,
    /// `state` — `Optional<String>`.
    pub state: Option<String>,
    /// `props` — `Map<String, List<ekr.views.ProjectedValue>>`.
    pub props: std::collections::BTreeMap<String, Vec<ProjectedValue>>,
    /// `degree` — `Integer`.
    pub degree: i64,
}

/// EdgeSide — `ekr.views.EdgeSide`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeSide {
    /// `Source`.
    Source,
    /// `Target`.
    Target,
}

/// EpochMillis — `ekr.views.EpochMillis`: a distinct wrapper around `Integer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochMillis(pub i64);

/// FactJudgement — `ekr.views.FactJudgement`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactJudgement {
    /// `assertion` — `ekr.graph.AssertionId`.
    pub assertion: crate::graph::AssertionId,
    /// `verdict` — `ekr.views.FactVerdict`.
    pub verdict: FactVerdict,
}

/// FactJudgementsFormatV1 — `ekr.views.FactJudgementsFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactJudgementsFormatV1 {
    /// `EkrFactJudgements1`.
    EkrFactJudgements1,
}

/// FactJudgementsV1 — `ekr.views.FactJudgementsV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactJudgementsV1 {
    /// `format` — `ekr.views.FactJudgementsFormatV1`.
    pub format: FactJudgementsFormatV1,
    /// `sample` — `Optional<ekr.views.SampleOrigin>`.
    pub sample: Option<SampleOrigin>,
    /// `judgements` — `List<ekr.views.FactJudgement>`.
    pub judgements: Vec<FactJudgement>,
}

/// FactQualityFormatV1 — `ekr.views.FactQualityFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactQualityFormatV1 {
    /// `EkrFactQuality1`.
    EkrFactQuality1,
}

/// FactQualityMeta — `ekr.views.FactQualityMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactQualityMeta {
    /// `format` — `ekr.views.FactQualityFormatV1`.
    pub format: FactQualityFormatV1,
    /// `confidence` — `ekr.views.BasisPoints`.
    pub confidence: BasisPoints,
    /// `z` — `Decimal`.
    pub z: crate::primitives::Decimal,
    /// `sample` — `Optional<ekr.views.SampleOrigin>`.
    pub sample: Option<SampleOrigin>,
}

/// FactQualityV1 — `ekr.views.FactQualityV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactQualityV1 {
    /// `meta` — `ekr.views.FactQualityMeta`.
    pub meta: FactQualityMeta,
    /// `judged` — `Integer`.
    pub judged: i64,
    /// `passed` — `Integer`.
    pub passed: i64,
    /// `failed` — `Integer`.
    pub failed: i64,
    /// `rate` — `Optional<Decimal>`.
    pub rate: Option<crate::primitives::Decimal>,
    /// `lower` — `Decimal`.
    pub lower: crate::primitives::Decimal,
    /// `upper` — `Decimal`.
    pub upper: crate::primitives::Decimal,
}

/// FactSampleFormatV1 — `ekr.views.FactSampleFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactSampleFormatV1 {
    /// `EkrFactSample1`.
    EkrFactSample1,
}

/// FactSampleMeta — `ekr.views.FactSampleMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactSampleMeta {
    /// `format` — `ekr.views.FactSampleFormatV1`.
    pub format: FactSampleFormatV1,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `seed` — `Integer`.
    pub seed: i64,
    /// `size` — `Integer`.
    pub size: i64,
    /// `type` — `Optional<ekr.ontology.TypeId>`.
    pub r#type: Option<crate::ontology::TypeId>,
    /// `population` — `Integer`.
    pub population: i64,
    /// `drawn` — `Integer`.
    pub drawn: i64,
}

/// FactSampleV1 — `ekr.views.FactSampleV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactSampleV1 {
    /// `meta` — `ekr.views.FactSampleMeta`.
    pub meta: FactSampleMeta,
    /// `items` — `List<ekr.views.SampledFact>`.
    pub items: Vec<SampledFact>,
}

/// FactVerdict — `ekr.views.FactVerdict`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactVerdict {
    /// `Pass`.
    Pass,
    /// `Fail`.
    Fail,
}

/// GraphChange — `ekr.views.GraphChange`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphChange {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `recorded_at` — `ekr.views.EpochMillis`.
    pub recorded_at: EpochMillis,
    /// `change` — `ekr.views.ChangeKind`.
    pub change: ChangeKind,
    /// `id` — `Uuid`.
    pub id: crate::primitives::Uuid,
    /// `type` — `Optional<ekr.ontology.TypeId>`.
    pub r#type: Option<crate::ontology::TypeId>,
    /// `name` — `Optional<String>`.
    pub name: Option<String>,
    /// `source` — `Optional<ekr.graph.NodeId>`.
    pub source: Option<crate::graph::NodeId>,
    /// `target` — `Optional<ekr.graph.NodeId>`.
    pub target: Option<crate::graph::NodeId>,
    /// `subject_kind` — `Optional<ekr.graph.SubjectKind>`.
    pub subject_kind: Option<crate::graph::SubjectKind>,
    /// `subject` — `Optional<Uuid>`.
    pub subject: Option<crate::primitives::Uuid>,
    /// `by` — `Optional<ekr.graph.AssertionId>`.
    pub by: Option<crate::graph::AssertionId>,
    /// `valid_time` — `Optional<ekr.views.EpochMillis>`.
    pub valid_time: Option<EpochMillis>,
    /// `locator` — `Optional<String>`.
    pub locator: Option<String>,
    /// `content_hash` — `Optional<ekr.kernel.ContentHash>`.
    pub content_hash: Option<crate::kernel::ContentHash>,
    /// `evidence` — `List<ekr.graph.EvidenceId>`.
    pub evidence: Vec<crate::graph::EvidenceId>,
}

/// GraphChangesFormatV1 — `ekr.views.GraphChangesFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphChangesFormatV1 {
    /// `EkrGraphChanges1`.
    EkrGraphChanges1,
}

/// GraphChangesV1 — `ekr.views.GraphChangesV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphChangesV1 {
    /// `meta` — `ekr.views.ChangesMeta`.
    pub meta: ChangesMeta,
    /// `changes` — `List<ekr.views.GraphChange>`.
    pub changes: Vec<GraphChange>,
    /// `next` — `Optional<ekr.views.ChangeCursor>`.
    pub next: Option<ChangeCursor>,
    /// `remaining` — `Integer`.
    pub remaining: i64,
}

/// GraphOverviewFormatV1 — `ekr.views.GraphOverviewFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphOverviewFormatV1 {
    /// `EkrGraphOverview1`.
    EkrGraphOverview1,
}

/// GraphOverviewV1 — `ekr.views.GraphOverviewV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphOverviewV1 {
    /// `meta` — `ekr.views.OverviewMeta`.
    pub meta: OverviewMeta,
    /// `ontology` — `ekr.views.ProjectedOntology`.
    pub ontology: ProjectedOntology,
    /// `schema` — `ekr.views.OverviewSchema`.
    pub schema: OverviewSchema,
    /// `node_types` — `List<ekr.views.TypeCount>`.
    pub node_types: Vec<TypeCount>,
    /// `edge_types` — `List<ekr.views.TypeCount>`.
    pub edge_types: Vec<TypeCount>,
    /// `roles` — `ekr.views.OverviewRoles`.
    pub roles: OverviewRoles,
    /// `timeline` — `ekr.views.OverviewTimeline`.
    pub timeline: OverviewTimeline,
    /// `top` — `List<ekr.views.NodeSummary>`.
    pub top: Vec<NodeSummary>,
}

/// GraphProjectionFormatV1 — `ekr.views.GraphProjectionFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphProjectionFormatV1 {
    /// `EkrGraphProjection1`.
    EkrGraphProjection1,
}

/// GraphProjectionV1 — `ekr.views.GraphProjectionV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphProjectionV1 {
    /// `meta` — `ekr.views.ProjectionMeta`.
    pub meta: ProjectionMeta,
    /// `ontology` — `ekr.views.ProjectedOntology`.
    pub ontology: ProjectedOntology,
    /// `nodes` — `List<ekr.views.ProjectedNode>`.
    pub nodes: Vec<ProjectedNode>,
    /// `edges` — `List<ekr.views.ProjectedEdge>`.
    pub edges: Vec<ProjectedEdge>,
    /// `evidence` — `List<ekr.views.ProjectedEvidence>`.
    pub evidence: Vec<ProjectedEvidence>,
    /// `schema` — `ekr.views.ProjectedSchema`.
    pub schema: ProjectedSchema,
}

/// GraphSliceFormatV1 — `ekr.views.GraphSliceFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphSliceFormatV1 {
    /// `EkrGraphSlice1`.
    EkrGraphSlice1,
}

/// GraphSliceV1 — `ekr.views.GraphSliceV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphSliceV1 {
    /// `meta` — `ekr.views.SliceMeta`.
    pub meta: SliceMeta,
    /// `nodes` — `List<ekr.views.SliceNode>`.
    pub nodes: Vec<SliceNode>,
    /// `edges` — `List<ekr.views.SliceEdge>`.
    pub edges: Vec<SliceEdge>,
    /// `next` — `Optional<ekr.views.SliceCursor>`.
    pub next: Option<SliceCursor>,
}

/// GraphTimelineFormatV1 — `ekr.views.GraphTimelineFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphTimelineFormatV1 {
    /// `EkrGraphTimeline1`.
    EkrGraphTimeline1,
}

/// GraphTimelineV1 — `ekr.views.GraphTimelineV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphTimelineV1 {
    /// `meta` — `ekr.views.TimelineMeta`.
    pub meta: TimelineMeta,
    /// `row_types` — `List<ekr.views.TimelineRowType>`.
    pub row_types: Vec<TimelineRowType>,
    /// `rows` — `List<ekr.views.TimelineRow>`.
    pub rows: Vec<TimelineRow>,
    /// `strip` — `List<ekr.views.TimelineCell>`.
    pub strip: Vec<TimelineCell>,
    /// `events` — `List<ekr.views.TimelineEvent>`.
    pub events: Vec<TimelineEvent>,
}

/// IntegerValue — `ekr.views.IntegerValue`: a distinct wrapper around `Integer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerValue(pub i64);

/// ListValue — `ekr.views.ListValue`: a distinct wrapper around `List<ekr.views.ProjectedValue>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListValue(pub Vec<ProjectedValue>);

/// MatchField — `ekr.views.MatchField`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchField {
    /// `Name`.
    Name,
    /// `Alias`.
    Alias,
}

/// MatchTier — `ekr.views.MatchTier`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchTier {
    /// `Exact`.
    Exact,
    /// `Folded`.
    Folded,
}

/// MatchesMeta — `ekr.views.MatchesMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchesMeta {
    /// `format` — `ekr.views.NodeMatchesFormatV1`.
    pub format: NodeMatchesFormatV1,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `text` — `String`.
    pub text: String,
    /// `total` — `Integer`.
    pub total: i64,
    /// `exact_total` — `Integer`.
    pub exact_total: i64,
}

/// ModifiedProperty — `ekr.views.ModifiedProperty`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifiedProperty {
    /// `owner` — `ekr.ontology.TypeId`.
    pub owner: crate::ontology::TypeId,
    /// `property` — `ekr.ontology.PropertyId`.
    pub property: crate::ontology::PropertyId,
    /// `name` — `String`.
    pub name: String,
    /// `changed` — `List<ekr.views.PropertyAspect>`.
    pub changed: Vec<PropertyAspect>,
}

/// NodeDetailFormatV1 — `ekr.views.NodeDetailFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeDetailFormatV1 {
    /// `EkrNodeDetail1`.
    EkrNodeDetail1,
}

/// NodeDetailV1 — `ekr.views.NodeDetailV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDetailV1 {
    /// `meta` — `ekr.views.DetailMeta`.
    pub meta: DetailMeta,
    /// `node` — `ekr.views.DetailNode`.
    pub node: DetailNode,
    /// `assertions` — `List<ekr.views.ProjectedAssertion>`.
    pub assertions: Vec<ProjectedAssertion>,
    /// `referencing` — `List<ekr.views.ReferencingAssertion>`.
    pub referencing: Vec<ReferencingAssertion>,
    /// `edges` — `List<ekr.views.ProjectedEdge>`.
    pub edges: Vec<ProjectedEdge>,
    /// `neighbours` — `List<ekr.views.NodeSummary>`.
    pub neighbours: Vec<NodeSummary>,
}

/// NodeMatch — `ekr.views.NodeMatch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeMatch {
    /// `id` — `ekr.graph.NodeId`.
    pub id: crate::graph::NodeId,
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `name` — `String`.
    pub name: String,
    /// `degree` — `Integer`.
    pub degree: i64,
    /// `tier` — `ekr.views.MatchTier`.
    pub tier: MatchTier,
    /// `field` — `ekr.views.MatchField`.
    pub field: MatchField,
    /// `alias` — `Optional<String>`.
    pub alias: Option<String>,
}

/// NodeMatchesFormatV1 — `ekr.views.NodeMatchesFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeMatchesFormatV1 {
    /// `EkrNodeMatches1`.
    EkrNodeMatches1,
}

/// NodeMatchesV1 — `ekr.views.NodeMatchesV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeMatchesV1 {
    /// `meta` — `ekr.views.MatchesMeta`.
    pub meta: MatchesMeta,
    /// `matches` — `List<ekr.views.NodeMatch>`.
    pub matches: Vec<NodeMatch>,
}

/// NodeSummary — `ekr.views.NodeSummary`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSummary {
    /// `id` — `ekr.graph.NodeId`.
    pub id: crate::graph::NodeId,
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `name` — `String`.
    pub name: String,
    /// `degree` — `Integer`.
    pub degree: i64,
}

/// Ocel20Log — `ekr.views.Ocel20Log`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ocel20Log {
    /// `eventTypes` — `List<ekr.views.OcelType>`.
    pub event_types: Vec<OcelType>,
    /// `objectTypes` — `List<ekr.views.OcelType>`.
    pub object_types: Vec<OcelType>,
    /// `events` — `List<ekr.views.OcelEvent>`.
    pub events: Vec<OcelEvent>,
    /// `objects` — `List<ekr.views.OcelObject>`.
    pub objects: Vec<OcelObject>,
}

/// OcelAttributeType — `ekr.views.OcelAttributeType`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcelAttributeType {
    /// `string`.
    String,
    /// `time`.
    Time,
    /// `integer`.
    Integer,
    /// `float`.
    Float,
    /// `boolean`.
    Boolean,
}

/// OcelEvent — `ekr.views.OcelEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelEvent {
    /// `id` — `String`.
    pub id: String,
    /// `type` — `String`.
    pub r#type: String,
    /// `time` — `ekr.views.OcelTime`.
    pub time: OcelTime,
    /// `attributes` — `List<ekr.views.OcelEventAttribute>`.
    pub attributes: Vec<OcelEventAttribute>,
    /// `relationships` — `List<ekr.views.OcelRelationship>`.
    pub relationships: Vec<OcelRelationship>,
}

/// OcelEventAttribute — `ekr.views.OcelEventAttribute`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelEventAttribute {
    /// `name` — `String`.
    pub name: String,
    /// `value` — `String`.
    pub value: String,
}

/// OcelFormatV1 — `ekr.views.OcelFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcelFormatV1 {
    /// `EkrOcel1`.
    EkrOcel1,
}

/// OcelLogV1 — `ekr.views.OcelLogV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelLogV1 {
    /// `meta` — `ekr.views.OcelMeta`.
    pub meta: OcelMeta,
    /// `names` — `ekr.views.OcelNames`.
    pub names: OcelNames,
    /// `ocel` — `ekr.views.Ocel20Log`.
    pub ocel: Ocel20Log,
}

/// OcelMeta — `ekr.views.OcelMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelMeta {
    /// `format` — `ekr.views.OcelFormatV1`.
    pub format: OcelFormatV1,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
}

/// OcelName — `ekr.views.OcelName`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelName {
    /// `id` — `String`.
    pub id: String,
    /// `name` — `String`.
    pub name: String,
}

/// OcelNames — `ekr.views.OcelNames`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelNames {
    /// `node_types` — `List<ekr.views.OcelName>`.
    pub node_types: Vec<OcelName>,
    /// `edge_types` — `List<ekr.views.OcelName>`.
    pub edge_types: Vec<OcelName>,
    /// `properties` — `List<ekr.views.OcelName>`.
    pub properties: Vec<OcelName>,
}

/// OcelObject — `ekr.views.OcelObject`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelObject {
    /// `id` — `String`.
    pub id: String,
    /// `type` — `String`.
    pub r#type: String,
    /// `attributes` — `List<ekr.views.OcelObjectAttribute>`.
    pub attributes: Vec<OcelObjectAttribute>,
    /// `relationships` — `List<ekr.views.OcelRelationship>`.
    pub relationships: Vec<OcelRelationship>,
}

/// OcelObjectAttribute — `ekr.views.OcelObjectAttribute`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelObjectAttribute {
    /// `name` — `String`.
    pub name: String,
    /// `value` — `String`.
    pub value: String,
    /// `time` — `ekr.views.OcelTime`.
    pub time: OcelTime,
}

/// OcelRelationship — `ekr.views.OcelRelationship`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelRelationship {
    /// `objectId` — `String`.
    pub object_id: String,
    /// `qualifier` — `String`.
    pub qualifier: String,
}

/// OcelTime — `ekr.views.OcelTime`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelTime(pub String);

/// OcelType — `ekr.views.OcelType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelType {
    /// `name` — `String`.
    pub name: String,
    /// `attributes` — `List<ekr.views.OcelTypeAttribute>`.
    pub attributes: Vec<OcelTypeAttribute>,
}

/// OcelTypeAttribute — `ekr.views.OcelTypeAttribute`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelTypeAttribute {
    /// `name` — `String`.
    pub name: String,
    /// `type` — `ekr.views.OcelAttributeType`.
    pub r#type: OcelAttributeType,
}

/// OverviewMeta — `ekr.views.OverviewMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewMeta {
    /// `format` — `ekr.views.GraphOverviewFormatV1`.
    pub format: GraphOverviewFormatV1,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `node_count` — `Integer`.
    pub node_count: i64,
    /// `edge_count` — `Integer`.
    pub edge_count: i64,
    /// `assertion_count` — `Integer`.
    pub assertion_count: i64,
    /// `evidence_count` — `Integer`.
    pub evidence_count: i64,
    /// `limit` — `Integer`.
    pub limit: i64,
}

/// OverviewRevision — `ekr.views.OverviewRevision`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewRevision {
    /// `number` — `ekr.kernel.RevisionNumber`.
    pub number: crate::kernel::RevisionNumber,
    /// `committed_at` — `ekr.views.EpochMillis`.
    pub committed_at: EpochMillis,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<crate::kernel::TransactionId>,
    /// `schema_version` — `ekr.ontology.SchemaVersionId`.
    pub schema_version: crate::ontology::SchemaVersionId,
    /// `nodes` — `Integer`.
    pub nodes: i64,
    /// `edges` — `Integer`.
    pub edges: i64,
    /// `assertions` — `Integer`.
    pub assertions: i64,
}

/// OverviewRoles — `ekr.views.OverviewRoles`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewRoles {
    /// `types` — `List<ekr.views.TypeTiming>`.
    pub types: Vec<TypeTiming>,
    /// `observation_type` — `Optional<ekr.ontology.TypeId>`.
    pub observation_type: Option<crate::ontology::TypeId>,
}

/// OverviewSchema — `ekr.views.OverviewSchema`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewSchema {
    /// `versions` — `List<ekr.views.OverviewSchemaVersion>`.
    pub versions: Vec<OverviewSchemaVersion>,
    /// `revisions` — `List<ekr.views.OverviewRevision>`.
    pub revisions: Vec<OverviewRevision>,
    /// `unrecorded_nodes` — `Integer`.
    pub unrecorded_nodes: i64,
    /// `unrecorded_edges` — `Integer`.
    pub unrecorded_edges: i64,
}

/// OverviewSchemaVersion — `ekr.views.OverviewSchemaVersion`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewSchemaVersion {
    /// `number` — `Integer`.
    pub number: i64,
    /// `id` — `ekr.ontology.SchemaVersionId`.
    pub id: crate::ontology::SchemaVersionId,
    /// `parent` — `Optional<ekr.ontology.SchemaVersionId>`.
    pub parent: Option<crate::ontology::SchemaVersionId>,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `added` — `List<ekr.views.SchemaMember>`.
    pub added: Vec<SchemaMember>,
    /// `removed` — `List<ekr.views.SchemaMember>`.
    pub removed: Vec<SchemaMember>,
    /// `widened` — `Optional<List<ekr.views.WidenedEnd>>`.
    pub widened: Option<Vec<WidenedEnd>>,
    /// `modified` — `Optional<List<ekr.views.ModifiedProperty>>`.
    pub modified: Option<Vec<ModifiedProperty>>,
}

/// OverviewTimeline — `ekr.views.OverviewTimeline`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewTimeline {
    /// `bucket_ms` — `Integer`.
    pub bucket_ms: i64,
    /// `first` — `Optional<ekr.views.EpochMillis>`.
    pub first: Option<EpochMillis>,
    /// `last` — `Optional<ekr.views.EpochMillis>`.
    pub last: Option<EpochMillis>,
    /// `undated` — `Integer`.
    pub undated: i64,
    /// `buckets` — `List<ekr.views.TimelineBucket>`.
    pub buckets: Vec<TimelineBucket>,
}

/// ProjectedAssertion — `ekr.views.ProjectedAssertion`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedAssertion {
    /// `id` — `ekr.graph.AssertionId`.
    pub id: crate::graph::AssertionId,
    /// `predicate_kind` — `ekr.graph.PredicateKind`.
    pub predicate_kind: crate::graph::PredicateKind,
    /// `predicate` — `Uuid`.
    pub predicate: crate::primitives::Uuid,
    /// `object_kind` — `ekr.graph.ObjectKind`.
    pub object_kind: crate::graph::ObjectKind,
    /// `object_value` — `Optional<ekr.views.ProjectedValue>`.
    pub object_value: Option<ProjectedValue>,
    /// `object_ref` — `Optional<Uuid>`.
    pub object_ref: Option<crate::primitives::Uuid>,
    /// `assessment` — `ekr.graph.AssessmentProjection`.
    pub assessment: crate::graph::AssessmentProjection,
    /// `lifecycle` — `ekr.views.ProjectedLifecycle`.
    pub lifecycle: ProjectedLifecycle,
    /// `valid_from` — `Optional<ekr.views.EpochMillis>`.
    pub valid_from: Option<EpochMillis>,
    /// `valid_to` — `Optional<ekr.views.EpochMillis>`.
    pub valid_to: Option<EpochMillis>,
    /// `recorded_from` — `ekr.views.EpochMillis`.
    pub recorded_from: EpochMillis,
    /// `recorded_to` — `Optional<ekr.views.EpochMillis>`.
    pub recorded_to: Option<EpochMillis>,
    /// `evidence` — `List<ekr.graph.EvidenceId>`.
    pub evidence: Vec<crate::graph::EvidenceId>,
}

/// ProjectedEdge — `ekr.views.ProjectedEdge`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedEdge {
    /// `id` — `ekr.graph.EdgeId`.
    pub id: crate::graph::EdgeId,
    /// `source` — `ekr.graph.NodeId`.
    pub source: crate::graph::NodeId,
    /// `target` — `ekr.graph.NodeId`.
    pub target: crate::graph::NodeId,
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `props` — `Map<String, List<ekr.views.ProjectedValue>>`.
    pub props: std::collections::BTreeMap<String, Vec<ProjectedValue>>,
    /// `assertions` — `List<ekr.views.ProjectedAssertion>`.
    pub assertions: Vec<ProjectedAssertion>,
}

/// ProjectedEdgeType — `ekr.views.ProjectedEdgeType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedEdgeType {
    /// `id` — `ekr.ontology.TypeId`.
    pub id: crate::ontology::TypeId,
    /// `name` — `String`.
    pub name: String,
    /// `source_types` — `List<ekr.ontology.TypeId>`.
    pub source_types: Vec<crate::ontology::TypeId>,
    /// `target_types` — `List<ekr.ontology.TypeId>`.
    pub target_types: Vec<crate::ontology::TypeId>,
    /// `properties` — `List<ekr.ontology.PropertyId>`.
    pub properties: Vec<crate::ontology::PropertyId>,
    /// `assertions` — `List<ekr.views.ProjectedAssertion>`.
    pub assertions: Vec<ProjectedAssertion>,
}

/// ProjectedEvidence — `ekr.views.ProjectedEvidence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedEvidence {
    /// `id` — `ekr.graph.EvidenceId`.
    pub id: crate::graph::EvidenceId,
    /// `kind` — `ekr.graph.EvidenceKind`.
    pub kind: crate::graph::EvidenceKind,
    /// `locator` — `String`.
    pub locator: String,
    /// `section` — `Optional<String>`.
    pub section: Option<String>,
    /// `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: crate::kernel::ContentHash,
    /// `observed_at` — `ekr.views.EpochMillis`.
    pub observed_at: EpochMillis,
    /// `confidence_bp` — `Integer`.
    pub confidence_bp: i64,
    /// `retained` — `Boolean`.
    pub retained: bool,
}

/// ProjectedLifecycle — `ekr.views.ProjectedLifecycle`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedLifecycle {
    /// `kind` — `ekr.graph.AssertionLifecycleKind`.
    pub kind: crate::graph::AssertionLifecycleKind,
    /// `at_revision` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at_revision: Option<crate::kernel::RevisionNumber>,
    /// `reason` — `Optional<String>`.
    pub reason: Option<String>,
    /// `by` — `Optional<ekr.graph.AssertionId>`.
    pub by: Option<crate::graph::AssertionId>,
    /// `effective_from` — `Optional<ekr.views.EpochMillis>`.
    pub effective_from: Option<EpochMillis>,
}

/// ProjectedNode — `ekr.views.ProjectedNode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedNode {
    /// `id` — `ekr.graph.NodeId`.
    pub id: crate::graph::NodeId,
    /// `name` — `String`.
    pub name: String,
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `aliases` — `List<String>`.
    pub aliases: Vec<String>,
    /// `state` — `Optional<String>`.
    pub state: Option<String>,
    /// `props` — `Map<String, List<ekr.views.ProjectedValue>>`.
    pub props: std::collections::BTreeMap<String, Vec<ProjectedValue>>,
    /// `assertions` — `List<ekr.views.ProjectedAssertion>`.
    pub assertions: Vec<ProjectedAssertion>,
}

/// ProjectedNodeType — `ekr.views.ProjectedNodeType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedNodeType {
    /// `id` — `ekr.ontology.TypeId`.
    pub id: crate::ontology::TypeId,
    /// `name` — `String`.
    pub name: String,
    /// `properties` — `List<ekr.ontology.PropertyId>`.
    pub properties: Vec<crate::ontology::PropertyId>,
    /// `assertions` — `List<ekr.views.ProjectedAssertion>`.
    pub assertions: Vec<ProjectedAssertion>,
}

/// ProjectedOntology — `ekr.views.ProjectedOntology`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedOntology {
    /// `node_types` — `List<ekr.views.ProjectedNodeType>`.
    pub node_types: Vec<ProjectedNodeType>,
    /// `edge_types` — `List<ekr.views.ProjectedEdgeType>`.
    pub edge_types: Vec<ProjectedEdgeType>,
    /// `properties` — `List<ekr.views.ProjectedProperty>`.
    pub properties: Vec<ProjectedProperty>,
}

/// ProjectedProperty — `ekr.views.ProjectedProperty`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedProperty {
    /// `id` — `ekr.ontology.PropertyId`.
    pub id: crate::ontology::PropertyId,
    /// `name` — `String`.
    pub name: String,
    /// `value_kind` — `ekr.ontology.ValueKind`.
    pub value_kind: crate::ontology::ValueKind,
    /// `owners` — `Optional<List<ekr.ontology.TypeId>>`.
    pub owners: Option<Vec<crate::ontology::TypeId>>,
}

/// ProjectedRevision — `ekr.views.ProjectedRevision`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedRevision {
    /// `number` — `ekr.kernel.RevisionNumber`.
    pub number: crate::kernel::RevisionNumber,
    /// `committed_at` — `ekr.views.EpochMillis`.
    pub committed_at: EpochMillis,
    /// `transaction_id` — `Optional<ekr.kernel.TransactionId>`.
    pub transaction_id: Option<crate::kernel::TransactionId>,
    /// `schema_version` — `ekr.ontology.SchemaVersionId`.
    pub schema_version: crate::ontology::SchemaVersionId,
}

/// ProjectedSchema — `ekr.views.ProjectedSchema`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedSchema {
    /// `versions` — `List<ekr.views.ProjectedSchemaVersion>`.
    pub versions: Vec<ProjectedSchemaVersion>,
    /// `revisions` — `List<ekr.views.ProjectedRevision>`.
    pub revisions: Vec<ProjectedRevision>,
}

/// ProjectedSchemaVersion — `ekr.views.ProjectedSchemaVersion`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedSchemaVersion {
    /// `number` — `Integer`.
    pub number: i64,
    /// `id` — `ekr.ontology.SchemaVersionId`.
    pub id: crate::ontology::SchemaVersionId,
    /// `parent` — `Optional<ekr.ontology.SchemaVersionId>`.
    pub parent: Option<crate::ontology::SchemaVersionId>,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `added` — `List<Uuid>`.
    pub added: Vec<crate::primitives::Uuid>,
}

/// ProjectedValue — `ekr.views.ProjectedValue`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectedValue {
    /// Tagged `Boolean` — `ekr.views.BooleanValue`.
    Boolean(BooleanValue),
    /// Tagged `Decimal` — `ekr.views.TextValue`.
    Decimal(TextValue),
    /// Tagged `Duration` — `ekr.views.IntegerValue`.
    Duration(IntegerValue),
    /// Tagged `Enum` — `ekr.views.TextValue`.
    Enum(TextValue),
    /// Tagged `Integer` — `ekr.views.IntegerValue`.
    Integer(IntegerValue),
    /// Tagged `List` — `ekr.views.ListValue`.
    List(ListValue),
    /// Tagged `NodeRef` — `ekr.graph.NodeId`.
    NodeRef(crate::graph::NodeId),
    /// Tagged `Record` — `ekr.views.RecordValue`.
    Record(RecordValue),
    /// Tagged `String` — `ekr.views.TextValue`.
    String(TextValue),
    /// Tagged `Timestamp` — `ekr.views.EpochMillis`.
    Timestamp(EpochMillis),
}

/// ProjectionMeta — `ekr.views.ProjectionMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionMeta {
    /// `format` — `ekr.views.GraphProjectionFormatV1`.
    pub format: GraphProjectionFormatV1,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `node_count` — `Integer`.
    pub node_count: i64,
    /// `edge_count` — `Integer`.
    pub edge_count: i64,
    /// `assertion_count` — `Integer`.
    pub assertion_count: i64,
    /// `evidence_count` — `Integer`.
    pub evidence_count: i64,
}

/// PropertyAspect — `ekr.views.PropertyAspect`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyAspect {
    /// `Declared`.
    Declared,
    /// `Undeclared`.
    Undeclared,
    /// `Name`.
    Name,
    /// `ValueType`.
    ValueType,
    /// `Cardinality`.
    Cardinality,
    /// `Required`.
    Required,
    /// `Constraints`.
    Constraints,
}

/// PropertyQuality — `ekr.views.PropertyQuality`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyQuality {
    /// `declared` — `Integer`.
    pub declared: i64,
    /// `constrained` — `Integer`.
    pub constrained: i64,
    /// `constrained_types` — `Integer`.
    pub constrained_types: i64,
    /// `constrained_share` — `Optional<ekr.views.BasisPoints>`.
    pub constrained_share: Option<BasisPoints>,
}

/// QualityMeta — `ekr.views.QualityMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityMeta {
    /// `format` — `ekr.views.StoreQualityFormatV1`.
    pub format: StoreQualityFormatV1,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
}

/// RecordValue — `ekr.views.RecordValue`: a distinct wrapper around `Map<String, ekr.views.ProjectedValue>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordValue(pub std::collections::BTreeMap<String, ProjectedValue>);

/// ReferencingAssertion — `ekr.views.ReferencingAssertion`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencingAssertion {
    /// `subject_kind` — `ekr.graph.SubjectKind`.
    pub subject_kind: crate::graph::SubjectKind,
    /// `subject` — `Uuid`.
    pub subject: crate::primitives::Uuid,
    /// `assertion` — `ekr.views.ProjectedAssertion`.
    pub assertion: ProjectedAssertion,
}

/// SampleOrigin — `ekr.views.SampleOrigin`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleOrigin {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `seed` — `Integer`.
    pub seed: i64,
    /// `size` — `Integer`.
    pub size: i64,
    /// `type` — `Optional<ekr.ontology.TypeId>`.
    pub r#type: Option<crate::ontology::TypeId>,
}

/// SampledEvidence — `ekr.views.SampledEvidence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampledEvidence {
    /// `id` — `ekr.graph.EvidenceId`.
    pub id: crate::graph::EvidenceId,
    /// `kind` — `ekr.graph.EvidenceKind`.
    pub kind: crate::graph::EvidenceKind,
    /// `locator` — `String`.
    pub locator: String,
    /// `section` — `Optional<String>`.
    pub section: Option<String>,
    /// `content_hash` — `ekr.kernel.ContentHash`.
    pub content_hash: crate::kernel::ContentHash,
    /// `text` — `Optional<String>`.
    pub text: Option<String>,
    /// `base64` — `Optional<String>`.
    pub base64: Option<String>,
}

/// SampledFact — `ekr.views.SampledFact`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampledFact {
    /// `subject_kind` — `ekr.graph.SubjectKind`.
    pub subject_kind: crate::graph::SubjectKind,
    /// `subject` — `Uuid`.
    pub subject: crate::primitives::Uuid,
    /// `subject_type` — `ekr.ontology.TypeId`.
    pub subject_type: crate::ontology::TypeId,
    /// `subject_name` — `Optional<String>`.
    pub subject_name: Option<String>,
    /// `predicate_name` — `Optional<String>`.
    pub predicate_name: Option<String>,
    /// `object_name` — `Optional<String>`.
    pub object_name: Option<String>,
    /// `assertion` — `ekr.views.ProjectedAssertion`.
    pub assertion: ProjectedAssertion,
    /// `evidence` — `List<ekr.views.SampledEvidence>`.
    pub evidence: Vec<SampledEvidence>,
}

/// SchemaMember — `ekr.views.SchemaMember`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaMember {
    /// `id` — `Uuid`.
    pub id: crate::primitives::Uuid,
    /// `kind` — `ekr.views.SchemaMemberKind`.
    pub kind: SchemaMemberKind,
    /// `name` — `String`.
    pub name: String,
}

/// SchemaMemberKind — `ekr.views.SchemaMemberKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaMemberKind {
    /// `NodeType`.
    NodeType,
    /// `EdgeType`.
    EdgeType,
    /// `Property`.
    Property,
}

/// SharedName — `ekr.views.SharedName`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedName {
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `name` — `String`.
    pub name: String,
    /// `nodes` — `List<ekr.graph.NodeId>`.
    pub nodes: Vec<crate::graph::NodeId>,
}

/// SinceKind — `ekr.views.SinceKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinceKind {
    /// `Revision`.
    Revision,
    /// `ValidTime`.
    ValidTime,
    /// `TransactionTime`.
    TransactionTime,
}

/// SliceCursor — `ekr.views.SliceCursor`: a distinct wrapper around `Integer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceCursor(pub i64);

/// SliceEdge — `ekr.views.SliceEdge`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceEdge {
    /// `id` — `ekr.graph.EdgeId`.
    pub id: crate::graph::EdgeId,
    /// `source` — `ekr.graph.NodeId`.
    pub source: crate::graph::NodeId,
    /// `target` — `ekr.graph.NodeId`.
    pub target: crate::graph::NodeId,
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `assertions` — `List<ekr.graph.AssertionId>`.
    pub assertions: Vec<crate::graph::AssertionId>,
}

/// SliceMeta — `ekr.views.SliceMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceMeta {
    /// `format` — `ekr.views.GraphSliceFormatV1`.
    pub format: GraphSliceFormatV1,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `seeds` — `List<ekr.graph.NodeId>`.
    pub seeds: Vec<crate::graph::NodeId>,
    /// `depth` — `Integer`.
    pub depth: i64,
    /// `after` — `ekr.views.SliceCursor`.
    pub after: SliceCursor,
    /// `node_total` — `Integer`.
    pub node_total: i64,
    /// `edge_total` — `Integer`.
    pub edge_total: i64,
}

/// SliceNode — `ekr.views.SliceNode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceNode {
    /// `id` — `ekr.graph.NodeId`.
    pub id: crate::graph::NodeId,
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `name` — `String`.
    pub name: String,
    /// `degree` — `Integer`.
    pub degree: i64,
    /// `distance` — `Integer`.
    pub distance: i64,
}

/// SourceText — `ekr.views.SourceText`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceText {
    /// `path` — `String`.
    pub path: String,
    /// `text` — `String`.
    pub text: String,
}

/// StoreLocation — `ekr.views.StoreLocation`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreLocation(pub String);

/// StoreQualityFormatV1 — `ekr.views.StoreQualityFormatV1`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreQualityFormatV1 {
    /// `EkrStoreQuality1`.
    EkrStoreQuality1,
}

/// StoreQualityV1 — `ekr.views.StoreQualityV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreQualityV1 {
    /// `meta` — `ekr.views.QualityMeta`.
    pub meta: QualityMeta,
    /// `assertions` — `ekr.views.AssertionQuality`.
    pub assertions: AssertionQuality,
    /// `properties` — `ekr.views.PropertyQuality`.
    pub properties: PropertyQuality,
    /// `shared_names` — `List<ekr.views.SharedName>`.
    pub shared_names: Vec<SharedName>,
    /// `sharing_nodes` — `Integer`.
    pub sharing_nodes: i64,
}

/// TextValue — `ekr.views.TextValue`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextValue(pub String);

/// TimelineBucket — `ekr.views.TimelineBucket`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineBucket {
    /// `start` — `ekr.views.EpochMillis`.
    pub start: EpochMillis,
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `assertions` — `Integer`.
    pub assertions: i64,
}

/// TimelineBucketWidth — `ekr.views.TimelineBucketWidth`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineBucketWidth {
    /// `Day`.
    Day,
    /// `Week`.
    Week,
}

/// TimelineCell — `ekr.views.TimelineCell`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineCell {
    /// `start` — `ekr.views.EpochMillis`.
    pub start: EpochMillis,
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `events` — `Integer`.
    pub events: i64,
}

/// TimelineEvent — `ekr.views.TimelineEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEvent {
    /// `id` — `ekr.graph.NodeId`.
    pub id: crate::graph::NodeId,
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `name` — `String`.
    pub name: String,
    /// `start` — `ekr.views.EpochMillis`.
    pub start: EpochMillis,
    /// `end` — `ekr.views.EpochMillis`.
    pub end: EpochMillis,
    /// `distance` — `Integer`.
    pub distance: i64,
    /// `path` — `List<ekr.views.TimelineStep>`.
    pub path: Vec<TimelineStep>,
}

/// TimelineMeta — `ekr.views.TimelineMeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineMeta {
    /// `format` — `ekr.views.GraphTimelineFormatV1`.
    pub format: GraphTimelineFormatV1,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `row_type` — `Optional<ekr.ontology.TypeId>`.
    pub row_type: Option<crate::ontology::TypeId>,
    /// `subject` — `Optional<ekr.graph.NodeId>`.
    pub subject: Option<crate::graph::NodeId>,
    /// `hops` — `Integer`.
    pub hops: i64,
    /// `limit` — `Integer`.
    pub limit: i64,
    /// `bucket_ms` — `Integer`.
    pub bucket_ms: i64,
    /// `first` — `Optional<ekr.views.EpochMillis>`.
    pub first: Option<EpochMillis>,
    /// `last` — `Optional<ekr.views.EpochMillis>`.
    pub last: Option<EpochMillis>,
    /// `subjects` — `Integer`.
    pub subjects: i64,
    /// `active` — `Integer`.
    pub active: i64,
    /// `events` — `Integer`.
    pub events: i64,
}

/// TimelineRow — `ekr.views.TimelineRow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineRow {
    /// `id` — `ekr.graph.NodeId`.
    pub id: crate::graph::NodeId,
    /// `name` — `String`.
    pub name: String,
    /// `total` — `Integer`.
    pub total: i64,
    /// `first` — `Optional<ekr.views.EpochMillis>`.
    pub first: Option<EpochMillis>,
    /// `last` — `Optional<ekr.views.EpochMillis>`.
    pub last: Option<EpochMillis>,
    /// `cells` — `List<ekr.views.TimelineCell>`.
    pub cells: Vec<TimelineCell>,
}

/// TimelineRowType — `ekr.views.TimelineRowType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineRowType {
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `nodes` — `Integer`.
    pub nodes: i64,
    /// `reached` — `Integer`.
    pub reached: i64,
    /// `weight` — `Integer`.
    pub weight: i64,
}

/// TimelineStep — `ekr.views.TimelineStep`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineStep {
    /// `edge` — `ekr.graph.EdgeId`.
    pub edge: crate::graph::EdgeId,
    /// `edge_type` — `ekr.ontology.TypeId`.
    pub edge_type: crate::ontology::TypeId,
    /// `forward` — `Boolean`.
    pub forward: bool,
    /// `node` — `ekr.graph.NodeId`.
    pub node: crate::graph::NodeId,
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `name` — `String`.
    pub name: String,
}

/// TypeCount — `ekr.views.TypeCount`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeCount {
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `count` — `Integer`.
    pub count: i64,
}

/// TypeTiming — `ekr.views.TypeTiming`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeTiming {
    /// `type` — `ekr.ontology.TypeId`.
    pub r#type: crate::ontology::TypeId,
    /// `nodes` — `Integer`.
    pub nodes: i64,
    /// `timestamped` — `Integer`.
    pub timestamped: i64,
    /// `judged` — `Integer`.
    pub judged: i64,
    /// `within_hour` — `Integer`.
    pub within_hour: i64,
    /// `instant` — `Integer`.
    pub instant: i64,
    /// `neighbour_types` — `Integer`.
    pub neighbour_types: i64,
    /// `event` — `Boolean`.
    pub event: bool,
}

/// WidenedEnd — `ekr.views.WidenedEnd`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidenedEnd {
    /// `edge_type` — `ekr.ontology.TypeId`.
    pub edge_type: crate::ontology::TypeId,
    /// `side` — `ekr.views.EdgeSide`.
    pub side: EdgeSide,
    /// `node_types` — `List<ekr.ontology.TypeId>`.
    pub node_types: Vec<crate::ontology::TypeId>,
}

/// List changes since — the input of `ekr.views.ChangesSince`.
///
/// Everything it can result in is [`ChangesSinceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangesSince {
    /// `store` — `ekr.views.StoreLocation`.
    pub store: StoreLocation,
    /// `since_kind` — `ekr.views.SinceKind`.
    pub since_kind: SinceKind,
    /// `since` — `Integer`.
    pub since: i64,
    /// `at` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at: Option<crate::kernel::RevisionNumber>,
    /// `limit` — `Optional<Integer>`.
    pub limit: Option<i64>,
    /// `after` — `Optional<ekr.views.ChangeCursor>`.
    pub after: Option<ChangeCursor>,
}

/// Actual typed response of `ekr.views.ChangesSince`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangesSinceResponse {
    /// `changes` — `ekr.views.GraphChangesV1`.
    pub changes: GraphChangesV1,
}

/// Everything `ekr.views.ChangesSince` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangesSinceOutcome {
    /// `listed` — otherwise.
    ///
    /// One ekr.graph-changes/1 page, byte-identical to every other answer to the same request with `at` resolved.
    Listed {
        /// The `ekr.views.ChangesListed` this outcome publishes.
        changes_listed: ChangesListed,
    },
    /// `since-malformed` — when `(since_kind == Revision and since < 0)`.
    ///
    /// The store was not read; the refusal names the since's kind and its value, a revision number below 0.
    SinceMalformed {
        /// Why it was refused: `ekr.views.SinceMalformed`.
        error: SinceMalformed,
    },
    /// `limit-exceeded` — when `(limit < 1 or limit > 2000 or after < 0)`.
    ///
    /// The store was not read; the refusal names the first broken input of limit and after, its value and its bounds.
    LimitExceeded {
        /// Why it was refused: `ekr.views.LimitExceeded`.
        error: LimitExceeded,
    },
    /// `not-found` — externally decided (the store holds no committed revision with the requested number, as at or as since_revision).
    ///
    /// Nothing was answered.
    NotFound {
        /// Why it was refused: `ekr.views.RevisionNotFound`.
        error: RevisionNotFound,
    },
    /// `not-seeded` — externally decided (the store has never been seeded).
    ///
    /// Nothing was answered; no head is reported, because there is none.
    NotSeeded {
        /// Why it was refused: `ekr.views.NotSeeded`.
        error: NotSeeded,
    },
}

/// Describe a node — the input of `ekr.views.DescribeNode`.
///
/// Everything it can result in is [`DescribeNodeOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescribeNode {
    /// `store` — `ekr.views.StoreLocation`.
    pub store: StoreLocation,
    /// `at` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at: Option<crate::kernel::RevisionNumber>,
    /// `node` — `ekr.graph.NodeId`.
    pub node: crate::graph::NodeId,
}

/// Actual typed response of `ekr.views.DescribeNode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescribeNodeResponse {
    /// `detail` — `ekr.views.NodeDetailV1`.
    pub detail: NodeDetailV1,
}

/// Everything `ekr.views.DescribeNode` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescribeNodeOutcome {
    /// `described` — otherwise.
    ///
    /// One ekr.node-detail/1 document, byte-identical to every other answer to the same request.
    Described {
        /// The `ekr.views.NodeDescribed` this outcome publishes.
        node_described: NodeDescribed,
    },
    /// `node-not-found` — externally decided (the requested revision holds no node with the requested id).
    ///
    /// Nothing was answered; a node created after the requested revision is not found at it.
    NodeNotFound {
        /// Why it was refused: `ekr.views.NodeNotFound`.
        error: NodeNotFound,
    },
    /// `not-found` — externally decided (the store holds no committed revision with the requested number).
    ///
    /// Nothing was answered.
    NotFound {
        /// Why it was refused: `ekr.views.RevisionNotFound`.
        error: RevisionNotFound,
    },
    /// `not-seeded` — externally decided (the store has never been seeded).
    ///
    /// Nothing was answered; no head is reported, because there is none.
    NotSeeded {
        /// Why it was refused: `ekr.views.NotSeeded`.
        error: NotSeeded,
    },
}

/// Draw a fact sample — the input of `ekr.views.DrawFactSample`.
///
/// Everything it can result in is [`DrawFactSampleOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawFactSample {
    /// `store` — `ekr.views.StoreLocation`.
    pub store: StoreLocation,
    /// `at` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at: Option<crate::kernel::RevisionNumber>,
    /// `seed` — `Integer`.
    pub seed: i64,
    /// `size` — `Integer`.
    pub size: i64,
    /// `type` — `Optional<ekr.ontology.TypeId>`.
    pub r#type: Option<crate::ontology::TypeId>,
}

/// Actual typed response of `ekr.views.DrawFactSample`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawFactSampleResponse {
    /// `sample` — `ekr.views.FactSampleV1`.
    pub sample: FactSampleV1,
}

/// Everything `ekr.views.DrawFactSample` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawFactSampleOutcome {
    /// `drawn` — otherwise.
    ///
    /// One ekr.fact-sample/1 document, byte-identical to every other answer to the same request with `at` resolved.
    Drawn {
        /// The `ekr.views.FactSampleDrawn` this outcome publishes.
        fact_sample_drawn: FactSampleDrawn,
    },
    /// `limit-exceeded` — when `(size < 1 or size > 1000)`.
    ///
    /// The store was not read; the refusal names size, its value and its bounds.
    LimitExceeded {
        /// Why it was refused: `ekr.views.LimitExceeded`.
        error: LimitExceeded,
    },
    /// `not-found` — externally decided (the store holds no committed revision with the requested number).
    ///
    /// Nothing was answered.
    NotFound {
        /// Why it was refused: `ekr.views.RevisionNotFound`.
        error: RevisionNotFound,
    },
    /// `not-seeded` — externally decided (the store has never been seeded).
    ///
    /// Nothing was answered; no head is reported, because there is none.
    NotSeeded {
        /// Why it was refused: `ekr.views.NotSeeded`.
        error: NotSeeded,
    },
}

/// Expand a neighbourhood — the input of `ekr.views.ExpandNeighbourhood`.
///
/// Everything it can result in is [`ExpandNeighbourhoodOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandNeighbourhood {
    /// `store` — `ekr.views.StoreLocation`.
    pub store: StoreLocation,
    /// `at` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at: Option<crate::kernel::RevisionNumber>,
    /// `seeds` — `List<ekr.graph.NodeId>`.
    pub seeds: Vec<crate::graph::NodeId>,
    /// `depth` — `Integer`.
    pub depth: i64,
    /// `limit` — `Integer`.
    pub limit: i64,
    /// `edge_limit` — `Optional<Integer>`.
    pub edge_limit: Option<i64>,
    /// `after` — `Optional<ekr.views.SliceCursor>`.
    pub after: Option<SliceCursor>,
}

/// Actual typed response of `ekr.views.ExpandNeighbourhood`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandNeighbourhoodResponse {
    /// `slice` — `ekr.views.GraphSliceV1`.
    pub slice: GraphSliceV1,
}

/// Everything `ekr.views.ExpandNeighbourhood` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpandNeighbourhoodOutcome {
    /// `expanded` — otherwise.
    ///
    /// One ekr.graph-slice/1 page, byte-identical to every other answer to the same request.
    Expanded {
        /// The `ekr.views.NeighbourhoodExpanded` this outcome publishes.
        neighbourhood_expanded: NeighbourhoodExpanded,
    },
    /// `limit-exceeded` — when `(depth < 0 or depth > 2 or limit < 1 or limit > 2000 or edge_limit < 1 or edge_limit > 5000 or after < 0)`.
    ///
    /// The store was not read; the refusal names the first broken input of depth, limit, edge_limit and after, its value and its bounds.
    LimitExceeded {
        /// Why it was refused: `ekr.views.LimitExceeded`.
        error: LimitExceeded,
    },
    /// `node-not-found` — externally decided (a seed names a node the requested revision does not hold).
    ///
    /// Nothing was answered; the refusal names the first such seed in the order the request gave them.
    NodeNotFound {
        /// Why it was refused: `ekr.views.NodeNotFound`.
        error: NodeNotFound,
    },
    /// `not-found` — externally decided (the store holds no committed revision with the requested number).
    ///
    /// Nothing was answered.
    NotFound {
        /// Why it was refused: `ekr.views.RevisionNotFound`.
        error: RevisionNotFound,
    },
    /// `not-seeded` — externally decided (the store has never been seeded).
    ///
    /// Nothing was answered; no head is reported, because there is none.
    NotSeeded {
        /// Why it was refused: `ekr.views.NotSeeded`.
        error: NotSeeded,
    },
}

/// Export an OCEL 2.0 event log — the input of `ekr.views.ExportOcel`.
///
/// Everything it can result in is [`ExportOcelOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOcel {
    /// `store` — `ekr.views.StoreLocation`.
    pub store: StoreLocation,
    /// `at` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at: Option<crate::kernel::RevisionNumber>,
    /// `events` — `List<String>`.
    pub events: Vec<String>,
    /// `event_time` — `Optional<List<String>>`.
    pub event_time: Option<Vec<String>>,
}

/// Actual typed response of `ekr.views.ExportOcel`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOcelResponse {
    /// `log` — `ekr.views.OcelLogV1`.
    pub log: OcelLogV1,
}

/// Everything `ekr.views.ExportOcel` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportOcelOutcome {
    /// `exported` — otherwise.
    ///
    /// One ekr.ocel/1 document of the requested revision, byte-identical to every other answer to the same request with `at` resolved.
    Exported {
        /// The `ekr.views.OcelExported` this outcome publishes.
        ocel_exported: OcelExported,
    },
    /// `not-found` — externally decided (the store holds no committed revision with the requested number).
    ///
    /// Nothing was answered.
    NotFound {
        /// Why it was refused: `ekr.views.RevisionNotFound`.
        error: RevisionNotFound,
    },
    /// `not-seeded` — externally decided (the store has never been seeded).
    ///
    /// Nothing was answered; no head is reported, because there is none.
    NotSeeded {
        /// Why it was refused: `ekr.views.NotSeeded`.
        error: NotSeeded,
    },
    /// `event-type-not-found` — externally decided (the revision's ontology holds no node type with a name the request lists in events).
    ///
    /// Nothing was answered; the refusal names the first such name.
    EventTypeNotFound {
        /// Why it was refused: `ekr.views.EventTypeNotFound`.
        error: EventTypeNotFound,
    },
    /// `event-time-invalid` — externally decided (an event_time selector is malformed, absent, ambiguous or conflicting, or a selected node holds multiple distinct timestamps).
    ///
    /// Nothing was answered; the refusal names the selector, reason and revision.
    EventTimeInvalid {
        /// Why it was refused: `ekr.views.EventTimeInvalid`.
        error: EventTimeInvalid,
    },
}

/// Find store names in code — the input of `ekr.views.FindCodeNames`.
///
/// Everything it can result in is [`FindCodeNamesOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindCodeNames {
    /// `store` — `ekr.views.StoreLocation`.
    pub store: StoreLocation,
    /// `at` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at: Option<crate::kernel::RevisionNumber>,
    /// `sources` — `List<ekr.views.SourceText>`.
    pub sources: Vec<SourceText>,
    /// `mode` — `Optional<ekr.views.CodeNameMode>`.
    pub mode: Option<CodeNameMode>,
}

/// Actual typed response of `ekr.views.FindCodeNames`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindCodeNamesResponse {
    /// `code_names` — `ekr.views.CodeNamesV1`.
    pub code_names: CodeNamesV1,
}

/// Everything `ekr.views.FindCodeNames` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindCodeNamesOutcome {
    /// `found` — otherwise.
    ///
    /// One ekr.code-names/1 document, byte-identical to every other answer to the same request with `at` resolved, however many findings it holds.
    Found {
        /// The `ekr.views.CodeNamesFound` this outcome publishes.
        code_names_found: CodeNamesFound,
    },
    /// `not-found` — externally decided (the store holds no committed revision with the requested number).
    ///
    /// Nothing was answered.
    NotFound {
        /// Why it was refused: `ekr.views.RevisionNotFound`.
        error: RevisionNotFound,
    },
    /// `not-seeded` — externally decided (the store has never been seeded).
    ///
    /// Nothing was answered; no head is reported, because there is none.
    NotSeeded {
        /// Why it was refused: `ekr.views.NotSeeded`.
        error: NotSeeded,
    },
}

/// Project the graph — the input of `ekr.views.ProjectGraph`.
///
/// Everything it can result in is [`ProjectGraphOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGraph {
    /// `store` — `ekr.views.StoreLocation`.
    pub store: StoreLocation,
    /// `at` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at: Option<crate::kernel::RevisionNumber>,
}

/// Actual typed response of `ekr.views.ProjectGraph`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGraphResponse {
    /// `projection` — `ekr.views.GraphProjectionV1`.
    pub projection: GraphProjectionV1,
}

/// Everything `ekr.views.ProjectGraph` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectGraphOutcome {
    /// `projected` — otherwise.
    ///
    /// One ekr.graph-projection/1 document of the requested revision, byte-identical to every other render of that revision.
    Projected {
        /// The `ekr.views.GraphProjected` this outcome publishes.
        graph_projected: GraphProjected,
    },
    /// `not-found` — externally decided (the store holds no committed revision with the requested number).
    ///
    /// Nothing was rendered.
    NotFound {
        /// Why it was refused: `ekr.views.RevisionNotFound`.
        error: RevisionNotFound,
    },
    /// `not-seeded` — externally decided (the store has never been seeded).
    ///
    /// Nothing was rendered; the request is echoed and no head is reported, because there is none.
    NotSeeded {
        /// Why it was refused: `ekr.views.NotSeeded`.
        error: NotSeeded,
    },
}

/// Overview the graph — the input of `ekr.views.ProjectOverview`.
///
/// Everything it can result in is [`ProjectOverviewOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectOverview {
    /// `store` — `ekr.views.StoreLocation`.
    pub store: StoreLocation,
    /// `at` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at: Option<crate::kernel::RevisionNumber>,
    /// `limit` — `Optional<Integer>`.
    pub limit: Option<i64>,
}

/// Actual typed response of `ekr.views.ProjectOverview`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectOverviewResponse {
    /// `overview` — `ekr.views.GraphOverviewV1`.
    pub overview: GraphOverviewV1,
}

/// Everything `ekr.views.ProjectOverview` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectOverviewOutcome {
    /// `overviewed` — otherwise.
    ///
    /// One ekr.graph-overview/1 document of the requested revision, byte-identical to every other answer to the same request.
    Overviewed {
        /// The `ekr.views.GraphOverviewed` this outcome publishes.
        graph_overviewed: GraphOverviewed,
    },
    /// `limit-exceeded` — when `(limit < 1 or limit > 500)`.
    ///
    /// The store was not read; the refusal names `limit`, its value and its bounds 1 and 500.
    LimitExceeded {
        /// Why it was refused: `ekr.views.LimitExceeded`.
        error: LimitExceeded,
    },
    /// `not-found` — externally decided (the store holds no committed revision with the requested number).
    ///
    /// Nothing was answered.
    NotFound {
        /// Why it was refused: `ekr.views.RevisionNotFound`.
        error: RevisionNotFound,
    },
    /// `not-seeded` — externally decided (the store has never been seeded).
    ///
    /// Nothing was answered; no head is reported, because there is none.
    NotSeeded {
        /// Why it was refused: `ekr.views.NotSeeded`.
        error: NotSeeded,
    },
}

/// Timeline the subjects — the input of `ekr.views.ProjectTimeline`.
///
/// Everything it can result in is [`ProjectTimelineOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTimeline {
    /// `store` — `ekr.views.StoreLocation`.
    pub store: StoreLocation,
    /// `at` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at: Option<crate::kernel::RevisionNumber>,
    /// `row_type` — `Optional<ekr.ontology.TypeId>`.
    pub row_type: Option<crate::ontology::TypeId>,
    /// `hops` — `Integer`.
    pub hops: i64,
    /// `limit` — `Integer`.
    pub limit: i64,
    /// `bucket` — `Optional<ekr.views.TimelineBucketWidth>`.
    pub bucket: Option<TimelineBucketWidth>,
    /// `subject` — `Optional<ekr.graph.NodeId>`.
    pub subject: Option<crate::graph::NodeId>,
}

/// Actual typed response of `ekr.views.ProjectTimeline`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTimelineResponse {
    /// `timeline` — `ekr.views.GraphTimelineV1`.
    pub timeline: GraphTimelineV1,
}

/// Everything `ekr.views.ProjectTimeline` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectTimelineOutcome {
    /// `timelined` — otherwise.
    ///
    /// One ekr.graph-timeline/1 document, byte-identical to every other answer to the same request.
    Timelined {
        /// The `ekr.views.SubjectsTimelined` this outcome publishes.
        subjects_timelined: SubjectsTimelined,
    },
    /// `limit-exceeded` — when `(hops < 1 or hops > 3 or limit < 1 or limit > 500)`.
    ///
    /// The store was not read; the refusal names the first broken input of hops and limit, its value and its bounds.
    LimitExceeded {
        /// Why it was refused: `ekr.views.LimitExceeded`.
        error: LimitExceeded,
    },
    /// `not-found` — externally decided (the store holds no committed revision with the requested number).
    ///
    /// Nothing was answered.
    NotFound {
        /// Why it was refused: `ekr.views.RevisionNotFound`.
        error: RevisionNotFound,
    },
    /// `not-seeded` — externally decided (the store has never been seeded).
    ///
    /// Nothing was answered; no head is reported, because there is none.
    NotSeeded {
        /// Why it was refused: `ekr.views.NotSeeded`.
        error: NotSeeded,
    },
}

/// Report fact quality — the input of `ekr.views.ReportFactQuality`.
///
/// Everything it can result in is [`ReportFactQualityOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportFactQuality {
    /// `judgements` — `List<ekr.views.FactJudgement>`.
    pub judgements: Vec<FactJudgement>,
    /// `sample` — `Optional<ekr.views.SampleOrigin>`.
    pub sample: Option<SampleOrigin>,
    /// `confidence` — `Optional<Integer>`.
    pub confidence: Option<i64>,
}

/// Actual typed response of `ekr.views.ReportFactQuality`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportFactQualityResponse {
    /// `quality` — `ekr.views.FactQualityV1`.
    pub quality: FactQualityV1,
}

/// Everything `ekr.views.ReportFactQuality` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportFactQualityOutcome {
    /// `reported` — otherwise.
    ///
    /// One ekr.fact-quality/1 document, byte-identical to every other answer to the same input.
    Reported {
        /// The `ekr.views.FactQualityReported` this outcome publishes.
        fact_quality_reported: FactQualityReported,
    },
    /// `limit-exceeded` — when `(confidence < 1 or confidence > 9999)`.
    ///
    /// Nothing was reported; the refusal names confidence, its value and its bounds.
    LimitExceeded {
        /// Why it was refused: `ekr.views.LimitExceeded`.
        error: LimitExceeded,
    },
    /// `judged-twice` — externally decided (the judgements judge one assertion more than once).
    ///
    /// Nothing was reported; the refusal names the first assertion judged again.
    JudgedTwice {
        /// Why it was refused: `ekr.views.JudgedTwice`.
        error: JudgedTwice,
    },
}

/// Report the store's quality — the input of `ekr.views.ReportStoreQuality`.
///
/// Everything it can result in is [`ReportStoreQualityOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportStoreQuality {
    /// `store` — `ekr.views.StoreLocation`.
    pub store: StoreLocation,
    /// `at` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at: Option<crate::kernel::RevisionNumber>,
}

/// Actual typed response of `ekr.views.ReportStoreQuality`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportStoreQualityResponse {
    /// `quality` — `ekr.views.StoreQualityV1`.
    pub quality: StoreQualityV1,
}

/// Everything `ekr.views.ReportStoreQuality` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportStoreQualityOutcome {
    /// `reported` — otherwise.
    ///
    /// One ekr.store-quality/1 document of the requested revision, byte-identical to every other answer to the same request with `at` resolved.
    Reported {
        /// The `ekr.views.StoreQualityReported` this outcome publishes.
        store_quality_reported: StoreQualityReported,
    },
    /// `not-found` — externally decided (the store holds no committed revision with the requested number).
    ///
    /// Nothing was answered.
    NotFound {
        /// Why it was refused: `ekr.views.RevisionNotFound`.
        error: RevisionNotFound,
    },
    /// `not-seeded` — externally decided (the store has never been seeded).
    ///
    /// Nothing was answered; no head is reported, because there is none.
    NotSeeded {
        /// Why it was refused: `ekr.views.NotSeeded`.
        error: NotSeeded,
    },
}

/// Search nodes — the input of `ekr.views.SearchNodes`.
///
/// Everything it can result in is [`SearchNodesOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchNodes {
    /// `store` — `ekr.views.StoreLocation`.
    pub store: StoreLocation,
    /// `at` — `Optional<ekr.kernel.RevisionNumber>`.
    pub at: Option<crate::kernel::RevisionNumber>,
    /// `text` — `String`.
    pub text: String,
    /// `limit` — `Integer`.
    pub limit: i64,
}

/// Actual typed response of `ekr.views.SearchNodes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchNodesResponse {
    /// `matches` — `ekr.views.NodeMatchesV1`.
    pub matches: NodeMatchesV1,
}

/// Everything `ekr.views.SearchNodes` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchNodesOutcome {
    /// `searched` — otherwise.
    ///
    /// One ekr.node-matches/1 document, byte-identical to every other answer to the same request.
    Searched {
        /// The `ekr.views.NodesSearched` this outcome publishes.
        nodes_searched: NodesSearched,
    },
    /// `limit-exceeded` — when `(limit < 1 or limit > 100)`.
    ///
    /// The store was not read; the refusal names `limit`, its value and its bounds 1 and 100.
    LimitExceeded {
        /// Why it was refused: `ekr.views.LimitExceeded`.
        error: LimitExceeded,
    },
    /// `not-found` — externally decided (the store holds no committed revision with the requested number).
    ///
    /// Nothing was answered.
    NotFound {
        /// Why it was refused: `ekr.views.RevisionNotFound`.
        error: RevisionNotFound,
    },
    /// `not-seeded` — externally decided (the store has never been seeded).
    ///
    /// Nothing was answered; no head is reported, because there is none.
    NotSeeded {
        /// Why it was refused: `ekr.views.NotSeeded`.
        error: NotSeeded,
    },
}

/// ChangesListed — the event `ekr.views.ChangesListed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangesListed {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `after` — `Integer`.
    pub after: i64,
    /// `changes` — `Integer`.
    pub changes: i64,
    /// `total` — `Integer`.
    pub total: i64,
    /// `remaining` — `Integer`.
    pub remaining: i64,
    /// `nodes_created` — `Integer`.
    pub nodes_created: i64,
    /// `edges_created` — `Integer`.
    pub edges_created: i64,
    /// `assertions_added` — `Integer`.
    pub assertions_added: i64,
    /// `assertions_superseded` — `Integer`.
    pub assertions_superseded: i64,
    /// `assertions_retracted` — `Integer`.
    pub assertions_retracted: i64,
    /// `evidence_added` — `Integer`.
    pub evidence_added: i64,
    /// `first_revision` — `Optional<ekr.kernel.RevisionNumber>`.
    pub first_revision: Option<crate::kernel::RevisionNumber>,
    /// `last_revision` — `Optional<ekr.kernel.RevisionNumber>`.
    pub last_revision: Option<crate::kernel::RevisionNumber>,
    /// `changes_hash` — `ekr.kernel.ContentHash`.
    pub changes_hash: crate::kernel::ContentHash,
}

/// CodeNamesFound — the event `ekr.views.CodeNamesFound`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeNamesFound {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `files` — `Integer`.
    pub files: i64,
    /// `literals` — `Integer`.
    pub literals: i64,
    /// `exempt` — `Integer`.
    pub exempt: i64,
    /// `findings` — `Integer`.
    pub findings: i64,
    /// `runtime_word_findings` — `Integer`.
    pub runtime_word_findings: i64,
    /// `node_types` — `Integer`.
    pub node_types: i64,
    /// `edge_types` — `Integer`.
    pub edge_types: i64,
    /// `properties` — `Integer`.
    pub properties: i64,
    /// `canonical_names` — `Integer`.
    pub canonical_names: i64,
    /// `aliases` — `Integer`.
    pub aliases: i64,
    /// `first_file` — `Optional<String>`.
    pub first_file: Option<String>,
    /// `first_line` — `Optional<Integer>`.
    pub first_line: Option<i64>,
    /// `code_names_hash` — `ekr.kernel.ContentHash`.
    pub code_names_hash: crate::kernel::ContentHash,
}

/// FactQualityReported — the event `ekr.views.FactQualityReported`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactQualityReported {
    /// `confidence` — `Integer`.
    pub confidence: i64,
    /// `judged` — `Integer`.
    pub judged: i64,
    /// `passed` — `Integer`.
    pub passed: i64,
    /// `failed` — `Integer`.
    pub failed: i64,
    /// `rate_bp` — `Optional<Integer>`.
    pub rate_bp: Option<i64>,
    /// `lower_bp` — `Integer`.
    pub lower_bp: i64,
    /// `upper_bp` — `Integer`.
    pub upper_bp: i64,
    /// `fact_quality_hash` — `ekr.kernel.ContentHash`.
    pub fact_quality_hash: crate::kernel::ContentHash,
}

/// FactSampleDrawn — the event `ekr.views.FactSampleDrawn`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactSampleDrawn {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `seed` — `Integer`.
    pub seed: i64,
    /// `size` — `Integer`.
    pub size: i64,
    /// `population` — `Integer`.
    pub population: i64,
    /// `drawn` — `Integer`.
    pub drawn: i64,
    /// `evidence` — `Integer`.
    pub evidence: i64,
    /// `first_assertion` — `Optional<ekr.graph.AssertionId>`.
    pub first_assertion: Option<crate::graph::AssertionId>,
    /// `sample_hash` — `ekr.kernel.ContentHash`.
    pub sample_hash: crate::kernel::ContentHash,
}

/// GraphOverviewed — the event `ekr.views.GraphOverviewed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphOverviewed {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `nodes` — `Integer`.
    pub nodes: i64,
    /// `edges` — `Integer`.
    pub edges: i64,
    /// `assertions` — `Integer`.
    pub assertions: i64,
    /// `evidence` — `Integer`.
    pub evidence: i64,
    /// `node_types` — `Integer`.
    pub node_types: i64,
    /// `edge_types` — `Integer`.
    pub edge_types: i64,
    /// `schema_versions` — `Integer`.
    pub schema_versions: i64,
    /// `revisions` — `Integer`.
    pub revisions: i64,
    /// `added` — `Integer`.
    pub added: i64,
    /// `removed` — `Integer`.
    pub removed: i64,
    /// `widened` — `Integer`.
    pub widened: i64,
    /// `modified` — `Integer`.
    pub modified: i64,
    /// `unrecorded_nodes` — `Integer`.
    pub unrecorded_nodes: i64,
    /// `unrecorded_edges` — `Integer`.
    pub unrecorded_edges: i64,
    /// `revision_zero_nodes` — `Integer`.
    pub revision_zero_nodes: i64,
    /// `revision_zero_edges` — `Integer`.
    pub revision_zero_edges: i64,
    /// `revision_zero_assertions` — `Integer`.
    pub revision_zero_assertions: i64,
    /// `event_types` — `Integer`.
    pub event_types: i64,
    /// `observation_type` — `Optional<ekr.ontology.TypeId>`.
    pub observation_type: Option<crate::ontology::TypeId>,
    /// `bucket_ms` — `Integer`.
    pub bucket_ms: i64,
    /// `timeline_buckets` — `Integer`.
    pub timeline_buckets: i64,
    /// `dated_assertions` — `Integer`.
    pub dated_assertions: i64,
    /// `undated_assertions` — `Integer`.
    pub undated_assertions: i64,
    /// `top` — `Integer`.
    pub top: i64,
    /// `overview_hash` — `ekr.kernel.ContentHash`.
    pub overview_hash: crate::kernel::ContentHash,
}

/// GraphProjected — the event `ekr.views.GraphProjected`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphProjected {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `nodes` — `Integer`.
    pub nodes: i64,
    /// `edges` — `Integer`.
    pub edges: i64,
    /// `assertions` — `Integer`.
    pub assertions: i64,
    /// `evidence` — `Integer`.
    pub evidence: i64,
    /// `schema_versions` — `Integer`.
    pub schema_versions: i64,
    /// `revisions` — `Integer`.
    pub revisions: i64,
    /// `transactions` — `Integer`.
    pub transactions: i64,
    /// `node_types` — `Integer`.
    pub node_types: i64,
    /// `edge_types` — `Integer`.
    pub edge_types: i64,
    /// `properties` — `Integer`.
    pub properties: i64,
    /// `edge_assertions` — `Integer`.
    pub edge_assertions: i64,
    /// `retracted_assertions` — `Integer`.
    pub retracted_assertions: i64,
    /// `retained_evidence` — `Integer`.
    pub retained_evidence: i64,
    /// `projection_hash` — `ekr.kernel.ContentHash`.
    pub projection_hash: crate::kernel::ContentHash,
}

/// NeighbourhoodExpanded — the event `ekr.views.NeighbourhoodExpanded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeighbourhoodExpanded {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `seeds` — `Integer`.
    pub seeds: i64,
    /// `depth` — `Integer`.
    pub depth: i64,
    /// `after` — `Integer`.
    pub after: i64,
    /// `nodes` — `Integer`.
    pub nodes: i64,
    /// `edges` — `Integer`.
    pub edges: i64,
    /// `node_total` — `Integer`.
    pub node_total: i64,
    /// `edge_total` — `Integer`.
    pub edge_total: i64,
    /// `remaining` — `Integer`.
    pub remaining: i64,
    /// `slice_hash` — `ekr.kernel.ContentHash`.
    pub slice_hash: crate::kernel::ContentHash,
}

/// NodeDescribed — the event `ekr.views.NodeDescribed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDescribed {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `node` — `ekr.graph.NodeId`.
    pub node: crate::graph::NodeId,
    /// `assertions` — `Integer`.
    pub assertions: i64,
    /// `referencing` — `Integer`.
    pub referencing: i64,
    /// `edges` — `Integer`.
    pub edges: i64,
    /// `neighbours` — `Integer`.
    pub neighbours: i64,
    /// `detail_hash` — `ekr.kernel.ContentHash`.
    pub detail_hash: crate::kernel::ContentHash,
}

/// NodesSearched — the event `ekr.views.NodesSearched`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodesSearched {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `text` — `String`.
    pub text: String,
    /// `matches` — `Integer`.
    pub matches: i64,
    /// `total` — `Integer`.
    pub total: i64,
    /// `exact_total` — `Integer`.
    pub exact_total: i64,
    /// `first_match` — `Optional<ekr.graph.NodeId>`.
    pub first_match: Option<crate::graph::NodeId>,
    /// `matches_hash` — `ekr.kernel.ContentHash`.
    pub matches_hash: crate::kernel::ContentHash,
}

/// OcelExported — the event `ekr.views.OcelExported`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelExported {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `event_types` — `Integer`.
    pub event_types: i64,
    /// `object_types` — `Integer`.
    pub object_types: i64,
    /// `events` — `Integer`.
    pub events: i64,
    /// `objects` — `Integer`.
    pub objects: i64,
    /// `event_object_relationships` — `Integer`.
    pub event_object_relationships: i64,
    /// `object_object_relationships` — `Integer`.
    pub object_object_relationships: i64,
    /// `edges_between_events` — `Integer`.
    pub edges_between_events: i64,
    /// `undated_events` — `Integer`.
    pub undated_events: i64,
    /// `edges_of_undated_events` — `Integer`.
    pub edges_of_undated_events: i64,
    /// `parallel_edges_merged` — `Integer`.
    pub parallel_edges_merged: i64,
    /// `attribute_values_out_of_range` — `Integer`.
    pub attribute_values_out_of_range: i64,
    /// `ocel_hash` — `ekr.kernel.ContentHash`.
    pub ocel_hash: crate::kernel::ContentHash,
}

/// StoreQualityReported — the event `ekr.views.StoreQualityReported`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreQualityReported {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `active_assertions` — `Integer`.
    pub active_assertions: i64,
    /// `with_evidence` — `Integer`.
    pub with_evidence: i64,
    /// `with_item_evidence` — `Integer`.
    pub with_item_evidence: i64,
    /// `with_seed_evidence` — `Integer`.
    pub with_seed_evidence: i64,
    /// `properties` — `Integer`.
    pub properties: i64,
    /// `constrained_properties` — `Integer`.
    pub constrained_properties: i64,
    /// `constrained_types` — `Integer`.
    pub constrained_types: i64,
    /// `shared_names` — `Integer`.
    pub shared_names: i64,
    /// `sharing_nodes` — `Integer`.
    pub sharing_nodes: i64,
    /// `quality_hash` — `ekr.kernel.ContentHash`.
    pub quality_hash: crate::kernel::ContentHash,
}

/// SubjectsTimelined — the event `ekr.views.SubjectsTimelined`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectsTimelined {
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
    /// `row_type` — `Optional<ekr.ontology.TypeId>`.
    pub row_type: Option<crate::ontology::TypeId>,
    /// `hops` — `Integer`.
    pub hops: i64,
    /// `bucket_ms` — `Integer`.
    pub bucket_ms: i64,
    /// `row_types` — `Integer`.
    pub row_types: i64,
    /// `subjects` — `Integer`.
    pub subjects: i64,
    /// `active` — `Integer`.
    pub active: i64,
    /// `rows` — `Integer`.
    pub rows: i64,
    /// `events` — `Integer`.
    pub events: i64,
    /// `row_events` — `Integer`.
    pub row_events: i64,
    /// `cells` — `Integer`.
    pub cells: i64,
    /// `strip` — `Integer`.
    pub strip: i64,
    /// `first_row` — `Optional<ekr.graph.NodeId>`.
    pub first_row: Option<crate::graph::NodeId>,
    /// `listed_events` — `Integer`.
    pub listed_events: i64,
    /// `timeline_hash` — `ekr.kernel.ContentHash`.
    pub timeline_hash: crate::kernel::ContentHash,
}

/// The declared error `ekr.views.EventTimeInvalid`.
///
/// A named event-time selector or its timestamp values are invalid or ambiguous; nothing was answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventTimeInvalid {
    /// `selector` — `String`.
    pub selector: String,
    /// `reason` — `String`.
    pub reason: String,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
}

/// The declared error `ekr.views.EventTypeNotFound`.
///
/// The revision's ontology holds no node type with a name the request lists as an event type; nothing was answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventTypeNotFound {
    /// `name` — `String`.
    pub name: String,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
}

/// The declared error `ekr.views.JudgedTwice`.
///
/// A judged sample judges one assertion more than once; nothing was reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgedTwice {
    /// `assertion` — `ekr.graph.AssertionId`.
    pub assertion: crate::graph::AssertionId,
}

/// The declared error `ekr.views.LimitExceeded`.
///
/// An input is outside its bound; the store was not read and nothing was answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitExceeded {
    /// `parameter` — `String`.
    pub parameter: String,
    /// `requested` — `Integer`.
    pub requested: i64,
    /// `minimum` — `Integer`.
    pub minimum: i64,
    /// `maximum` — `Optional<Integer>`.
    pub maximum: Option<i64>,
}

/// The declared error `ekr.views.NodeNotFound`.
///
/// The revision holds no node with the requested id; nothing was answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeNotFound {
    /// `node` — `ekr.graph.NodeId`.
    pub node: crate::graph::NodeId,
    /// `revision` — `ekr.kernel.RevisionNumber`.
    pub revision: crate::kernel::RevisionNumber,
}

/// The declared error `ekr.views.NotSeeded`.
///
/// The store was never seeded, so it has no revision and no head; nothing was rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotSeeded {
    /// `requested` — `Optional<ekr.kernel.RevisionNumber>`.
    pub requested: Option<crate::kernel::RevisionNumber>,
}

/// The declared error `ekr.views.RevisionNotFound`.
///
/// The store holds no committed revision with the requested number; nothing was rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionNotFound {
    /// `requested` — `ekr.kernel.RevisionNumber`.
    pub requested: crate::kernel::RevisionNumber,
    /// `head` — `ekr.kernel.RevisionNumber`.
    pub head: crate::kernel::RevisionNumber,
}

/// The declared error `ekr.views.SinceMalformed`.
///
/// A ChangesSince request's since is no revision, valid time or transaction time; the store was not read and nothing was answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinceMalformed {
    /// `kind` — `ekr.views.SinceKind`.
    pub kind: SinceKind,
    /// `requested` — `Integer`.
    pub requested: i64,
}

/// What this bounded context owes its implementor, as typed seams.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every trait by refusing in the type system, so the workspace builds —
/// and says exactly what it cannot yet do — before a line is hand-written.
pub mod obligations {
    /// The behaviour `ekr.views.ChangesSince` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.views.ChangesSince` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `listed` otherwise, emits `ekr.views.ChangesListed`; `since-malformed` when `(since_kind == Revision and since < 0)`, error `ekr.views.SinceMalformed`; `limit-exceeded` when `(limit < 1 or limit > 2000 or after < 0)`, error `ekr.views.LimitExceeded`; `not-found` externally decided (the store holds no committed revision with the requested number, as at or as since_revision), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded`.
    pub trait ChangesSinceBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.views.ChangesSince`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn changes_since(&mut self, input: super::ChangesSince) -> Result<super::ChangesSinceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.views.DescribeNode` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.views.DescribeNode` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `described` otherwise, emits `ekr.views.NodeDescribed`; `node-not-found` externally decided (the requested revision holds no node with the requested id), error `ekr.views.NodeNotFound`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded`.
    pub trait DescribeNodeBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.views.DescribeNode`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn describe_node(&mut self, input: super::DescribeNode) -> Result<super::DescribeNodeOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.views.DrawFactSample` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.views.DrawFactSample` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `drawn` otherwise, emits `ekr.views.FactSampleDrawn`; `limit-exceeded` when `(size < 1 or size > 1000)`, error `ekr.views.LimitExceeded`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded`.
    pub trait DrawFactSampleBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.views.DrawFactSample`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn draw_fact_sample(&mut self, input: super::DrawFactSample) -> Result<super::DrawFactSampleOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.views.ExpandNeighbourhood` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.views.ExpandNeighbourhood` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `expanded` otherwise, emits `ekr.views.NeighbourhoodExpanded`; `limit-exceeded` when `(depth < 0 or depth > 2 or limit < 1 or limit > 2000 or edge_limit < 1 or edge_limit > 5000 or after < 0)`, error `ekr.views.LimitExceeded`; `node-not-found` externally decided (a seed names a node the requested revision does not hold), error `ekr.views.NodeNotFound`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded`.
    pub trait ExpandNeighbourhoodBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.views.ExpandNeighbourhood`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn expand_neighbourhood(&mut self, input: super::ExpandNeighbourhood) -> Result<super::ExpandNeighbourhoodOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.views.ExportOcel` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.views.ExportOcel` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `exported` otherwise, emits `ekr.views.OcelExported`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded`; `event-type-not-found` externally decided (the revision's ontology holds no node type with a name the request lists in events), error `ekr.views.EventTypeNotFound`; `event-time-invalid` externally decided (an event_time selector is malformed, absent, ambiguous or conflicting, or a selected node holds multiple distinct timestamps), error `ekr.views.EventTimeInvalid`.
    pub trait ExportOcelBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.views.ExportOcel`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn export_ocel(&mut self, input: super::ExportOcel) -> Result<super::ExportOcelOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.views.FindCodeNames` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.views.FindCodeNames` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `found` otherwise, emits `ekr.views.CodeNamesFound`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded`.
    pub trait FindCodeNamesBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.views.FindCodeNames`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn find_code_names(&mut self, input: super::FindCodeNames) -> Result<super::FindCodeNamesOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.views.ProjectGraph` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.views.ProjectGraph` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `projected` otherwise, emits `ekr.views.GraphProjected`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded`.
    pub trait ProjectGraphBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.views.ProjectGraph`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn project_graph(&mut self, input: super::ProjectGraph) -> Result<super::ProjectGraphOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.views.ProjectOverview` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.views.ProjectOverview` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `overviewed` otherwise, emits `ekr.views.GraphOverviewed`; `limit-exceeded` when `(limit < 1 or limit > 500)`, error `ekr.views.LimitExceeded`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded`.
    pub trait ProjectOverviewBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.views.ProjectOverview`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn project_overview(&mut self, input: super::ProjectOverview) -> Result<super::ProjectOverviewOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.views.ProjectTimeline` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.views.ProjectTimeline` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `timelined` otherwise, emits `ekr.views.SubjectsTimelined`; `limit-exceeded` when `(hops < 1 or hops > 3 or limit < 1 or limit > 500)`, error `ekr.views.LimitExceeded`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded`.
    pub trait ProjectTimelineBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.views.ProjectTimeline`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn project_timeline(&mut self, input: super::ProjectTimeline) -> Result<super::ProjectTimelineOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.views.ReportFactQuality` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.views.ReportFactQuality` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `reported` otherwise, emits `ekr.views.FactQualityReported`; `limit-exceeded` when `(confidence < 1 or confidence > 9999)`, error `ekr.views.LimitExceeded`; `judged-twice` externally decided (the judgements judge one assertion more than once), error `ekr.views.JudgedTwice`.
    pub trait ReportFactQualityBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.views.ReportFactQuality`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn report_fact_quality(&mut self, input: super::ReportFactQuality) -> Result<super::ReportFactQualityOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.views.ReportStoreQuality` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.views.ReportStoreQuality` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `reported` otherwise, emits `ekr.views.StoreQualityReported`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded`.
    pub trait ReportStoreQualityBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.views.ReportStoreQuality`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn report_store_quality(&mut self, input: super::ReportStoreQuality) -> Result<super::ReportStoreQualityOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `ekr.views.SearchNodes` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `ekr.views.SearchNodes` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `searched` otherwise, emits `ekr.views.NodesSearched`; `limit-exceeded` when `(limit < 1 or limit > 100)`, error `ekr.views.LimitExceeded`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded`.
    pub trait SearchNodesBehavior {
        /// Decides and enacts exactly one declared outcome of `ekr.views.SearchNodes`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn search_nodes(&mut self, input: super::SearchNodes) -> Result<super::SearchNodesOutcome, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl ChangesSinceBehavior for Unimplemented {
        fn changes_since(&mut self, _input: super::ChangesSince) -> Result<super::ChangesSinceOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.views.ChangesSince" })
        }
    }

    impl DescribeNodeBehavior for Unimplemented {
        fn describe_node(&mut self, _input: super::DescribeNode) -> Result<super::DescribeNodeOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.views.DescribeNode" })
        }
    }

    impl DrawFactSampleBehavior for Unimplemented {
        fn draw_fact_sample(&mut self, _input: super::DrawFactSample) -> Result<super::DrawFactSampleOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.views.DrawFactSample" })
        }
    }

    impl ExpandNeighbourhoodBehavior for Unimplemented {
        fn expand_neighbourhood(&mut self, _input: super::ExpandNeighbourhood) -> Result<super::ExpandNeighbourhoodOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.views.ExpandNeighbourhood" })
        }
    }

    impl ExportOcelBehavior for Unimplemented {
        fn export_ocel(&mut self, _input: super::ExportOcel) -> Result<super::ExportOcelOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.views.ExportOcel" })
        }
    }

    impl FindCodeNamesBehavior for Unimplemented {
        fn find_code_names(&mut self, _input: super::FindCodeNames) -> Result<super::FindCodeNamesOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.views.FindCodeNames" })
        }
    }

    impl ProjectGraphBehavior for Unimplemented {
        fn project_graph(&mut self, _input: super::ProjectGraph) -> Result<super::ProjectGraphOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.views.ProjectGraph" })
        }
    }

    impl ProjectOverviewBehavior for Unimplemented {
        fn project_overview(&mut self, _input: super::ProjectOverview) -> Result<super::ProjectOverviewOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.views.ProjectOverview" })
        }
    }

    impl ProjectTimelineBehavior for Unimplemented {
        fn project_timeline(&mut self, _input: super::ProjectTimeline) -> Result<super::ProjectTimelineOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.views.ProjectTimeline" })
        }
    }

    impl ReportFactQualityBehavior for Unimplemented {
        fn report_fact_quality(&mut self, _input: super::ReportFactQuality) -> Result<super::ReportFactQualityOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.views.ReportFactQuality" })
        }
    }

    impl ReportStoreQualityBehavior for Unimplemented {
        fn report_store_quality(&mut self, _input: super::ReportStoreQuality) -> Result<super::ReportStoreQualityOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.views.ReportStoreQuality" })
        }
    }

    impl SearchNodesBehavior for Unimplemented {
        fn search_nodes(&mut self, _input: super::SearchNodes) -> Result<super::SearchNodesOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "ekr.views.SearchNodes" })
        }
    }
}
