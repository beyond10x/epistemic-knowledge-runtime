---
format: aep.planning-md/3
id: review-result:haiku-panel-0-0-39-supersede-node
kind: review-result
status: active
title: Haiku panel, 20 read-only reviewers, on release 0.0.39 (SupersedeNode)
relations:
- reviews: story:supersede-node
- reviews: story:supersede-node-review-fixes
revision: 1
---
Owners: 54 findings, not attributed: the panel reviewed the merged 0.0.39 change, whose code is
the wave 2026-10-19a implementor's and whose story, brief and ADR are its coordinator's; no
reviewer split them.

Recorded by the coordinator. The table is the coordinator's merge of the 20 reports. The
findings block holds every reviewer's findings unchanged, in slice order, serialised as one
JSON list. The store refused slice 2's YAML, slice 9's verdict PLAUSIBLE and slices 10 and 11's
severity error (the last two offered by the coordinator's brief, not store values); each of
those reviewers restated its own block, choosing its own store verdict or severity. The
reports' prose was condensed by the coordinator and is kept under
<home>/.cache/ekr-w19a-u1-adversary/haiku/report-NN.md.


20 read-only reviewers (general-purpose, model haiku), one slice each (`slices.md`), on
`2dd094077..4d43b5017`. 18 returned findings, 2 none (slices 6, 14). 54 findings raw (1
pre-existing, 17.5, left out); merged and grouped into the 31 rows below. "Coordinator check" = the coordinator read the cited code itself.

## Defects (fix in a follow-up story)

| # | where | what | sev | source | coordinator check |
|---|---|---|---|---|---|
| D1 | `crates/ekr-views/src/index.rs:228` | describe_node/1 `referencing` of a live node lists assertions whose subject is a superseded node or retired edge, and counts them | error | 11.1, 12.1 | confirmed: push runs for every claim regardless of the skip arms at :209-210 |
| D2 | `crates/ekr-sdk/src/resolve.rs:209` | `Resolver::observe` ignores a committed SupersedeNode; own revisions never clear the cache (:243-250), so `resolve_with` keeps answering the superseded id | warning | 7.1 | confirmed |
| D3 | `crates/ekr-views/src/index.rs:556` | timeline dating: a Relation assertion whose subject is superseded still dates its object node; overview_timeline skips it | warning | 13.1 | confirmed: `subject` None, object still marked |
| D4 | `crates/ekr/src/cli/explain.rs:434`, `document.rs:367`, `sample.rs:361` | an `Object::Value(NodeRef)` naming a superseded node gets no pointer (ADR 0017 d3 vs views.yaml:339 narrowing) | warning | 15.1 | not checked |
| D5 | `crates/ekr-views/src/detail.rs:604` | describe_node/2 (MCP default) omits relations to a superseded node with no pointer; design §110.5 says both formats carry it | warning | 12.2, 16.2 | not checked; fix code or the sentence |
| D6 | `crates/ekr-sdk/src/extraction.rs:396`, `:936` | cross-type superseded key: pre-check passes on the key's type, a new object node is committed, then the kernel refuses the edge (orphan node); `name_supersessions` attaches its note without checking the cause | warning | 10.1, 10.3 | not checked |
| D7 | `crates/ekr-kernel/src/validate/supersession.rs:218`, `:140` | a SupersedeNode refused by Reference still enters the retiring set, so other ops get a second, false refusal; `X by X` on absent X gets three | warning | 1.1, 1.2 | not checked |
| D8 | `reference.rs:425`, `:283`, `:458`; `structural.rs:246`, `:294` | refusal advice names a `by` the same transaction retires, an edge of `by` that does not exist, or a superseded holder with "use that node" | warning | 2.1–2.3, 8.1–8.2, 17.3 | not checked |
| D9 | `crates/ekr-graph/src/canonical.rs:484` (`retired_with`) | linear scan over all records, no reverse index; called per assertion on a retired edge in every validation (reference.rs:331), per new edge (structural.rs:836, :920), per sampled assertion (sample.rs:216) | warning | 19.1–19.3 | not checked |
| D10 | `crates/ekr-kernel/src/validate/structural.rs:912` | CreateNode with a retired edge's id accepted under v1/v2 (kernel.yaml:741 says every profile) | warning | 17.2 | read :900-925: confirmed, node arm checks only nodes and superseded |

## Tests that would not catch a regression

| # | where | surviving mutant or gap | source |
|---|---|---|---|
| T1 | `structural.rs:415` | drop `.chain(superseded)` in the standalone alias scan | 20.1 |
| T2 | `supersession.rs:92`, `:120` | drop `draft.target` or the AddAlias arm from `named()` | 20.2 |
| T3 | `reference.rs:133` | drop the later CreateEdge target check | 20.3 |
| T4 | `reference.rs:272` | check against the candidate instead of state | 20.4 |
| T5 | `ekr-graph/src/supersession.rs:80` | drop or reorder a field in `SupersededNode::encode`; no non-empty record root pinned | 20.5 |
| T6 | `replay.rs:2247`, `:2396`; `commit_applies_once.rs:231`; `adversary_validate_once.rs:727` | cost counters never run with a supersession | 19.5–19.8 |
| T7 | `ekr-store/tests/borrowed_graph.rs:134`; `ekr/tests/store_comparison.rs:283`; `ekr-views/tests/supersession.rs:6` | borrowed vs owned bytes with records; seventh compare category; referencing leak | 5.1, 15.3, 11.2 |

## Documents that disagree with the code

| # | where | what | source |
|---|---|---|---|
| R1 | design §110.2, `graph.yaml:511`, `ekr-graph/src/supersession.rs:47` | "every edge at either end": an edge between two nodes superseded together is in the source's record only (code and an adversary test pin that) | 3.1, 17.1 |
| R2 | ADR 0017 d4 | "`by` must be a node the head holds": a `by` created in the same transaction is accepted and pinned | 20.6 |
| R3 | design §110.6 | "two generated node-superseded outcomes": there are three | 16.1 |
| R4 | `docs/cli.md:2621` | 404 claimed for MCP and session reads; only HTTP is 404 | 16.3 |
| R5 | `graph.yaml:522` | entity declares no evidence and no retired-edge relation | 17.4 |
| R6 | `structural.rs:200`, `:694`; `canonical.rs:505` | doc comments no longer true (alias "Ambiguous", HeldIdentities "adds nothing else", resolve None) | 8.3, 6, 4.1 |
| R7 | overview vs projection | assertion totals differ for one revision; views.yaml gives both readings | 11.3, 18.3 |

## Design questions (no defect against the ADR as written)

| # | question | source |
|---|---|---|
| Q1 | no check that `node` and `by` are related; any proposer with any retained evidence can hide any node (ADR 0017 allows unrelated types on purpose) | 18.1 |
| Q2 | the NodeSuperseded change row omits the reason and lists no retired edges; a later repoint emits no row | 18.2, 18.4 |
| Q3 | `ekr.transaction-document/1` accepts post-/1 operations (pre-existing for AddAlias etc.) | 7.3 |
| Q4 | cross-type: facts typed by the old key's type are refused against that type (10.2, rated error by the reviewer; the coordinator reads it as the document's own typing) | 10.2 |
| Q5 | a kept node whose type later gains a subtype is reachable through an old key but refused through its own | 9.1 (PLAUSIBLE) |
| Q6 | `SupersededNode` Deserialize accepts records the kernel forbids; only the root compare refuses them | 4.2 |
| Q7 | duplicate evidence ids in `!SupersedeNode` are dropped silently, unlike assertion evidence | 7.2 |

```findings
[
 {
  "file": "crates/ekr-kernel/src/validate/supersession.rs",
  "line": 218,
  "category": "correctness",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a SupersedeNode refused by Reference (node absent or already superseded, or by already superseded) still puts that node in the same-transaction retiring set, so each other op naming it gets a second refusal that says a supersession of the transaction retires it, which it does not."
 },
 {
  "file": "crates/ekr-kernel/src/validate/supersession.rs",
  "line": 140,
  "category": "correctness",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "SupersedeNode{X by X} on an absent X gets three refusals, two of which state the same absence (supersede-unknown-node and supersede-by-unknown-node)."
 },
 {
  "file": "crates/ekr-kernel/src/validate/reference.rs",
  "line": 425,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "In one tx with SupersedeNode{node: B, by: C} and an operation naming A (already superseded by B), node() and edge() (452-461) answer node-superseded naming B, which this tx retires; after commit A's record names C (apply.rs:367), so the advice names a node the head will not hold."
 },
 {
  "file": "crates/ekr-kernel/src/validate/reference.rs",
  "line": 283,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "In one tx with SupersedeNode{X by B} and SupersedeNode{K by L}, where B is already superseded by K, supersede-by-superseded-node tells the caller to supersede X by K, which this tx retires; the right by is L."
 },
 {
  "file": "crates/ekr-kernel/src/validate/reference.rs",
  "line": 458,
  "category": "docs",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The edge advice \"name an edge of node {by} instead\" presumes an edge of `by` exists, but ADR 0017 decision 2 retires edges rather than moving them, so a caller who follows it gets unresolved-edge unless it creates the edge first."
 },
 {
  "file": "crates/ekr-kernel/src/apply.rs",
  "line": 350,
  "category": "contract-drift",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "A pair of nodes superseded in one transaction retires the edge between them into the source's record only (apply.rs:350-355, 382), so the target's record omits an edge its documented contract (supersession.rs:47, design 110.2) says it must hold; no read loses the edge; pinned by adversary_w20261019a_supersede_node.rs:443."
 },
 {
  "file": "crates/ekr-graph/src/canonical.rs",
  "line": 505,
  "category": "contract-drift",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "CanonicalGraph::resolve answers None for a node held only in a supersession record, while superseded_by answers its pointer for the same id, against the doc's \"None if this state does not hold it\"; no production caller reaches it."
 },
 {
  "file": "crates/ekr-graph/src/supersession.rs",
  "line": 34,
  "category": "boundary",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "the derived Deserialize accepts an empty reason, empty evidence, by equal to node, a key differing from node.id and a foreign root_id, which the docs and kernel forbid; a hand-built by==node record lists the superseded node under its own alias in AliasIndex::of; checkpoint loads refuse such records only at the knowledge-root compare."
 },
 {
  "file": "crates/ekr-store/tests/borrowed_graph.rs",
  "line": 134,
  "category": "test-coverage",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The byte-equality test for serialize_graph against GraphDocument::of builds its graph with superseded empty (line 134), so the new superseded branch of BorrowedGraphFields is never compared byte-for-byte with the owned writer; no current divergence was found."
 },
 {
  "file": "crates/ekr-sdk/src/resolve.rs",
  "line": 209,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Resolver::observe matches Operation with a `_ => {}` arm, so a committed SupersedeNode leaves cached alias→superseded-node answers in place, and resolve_with returns the superseded node from cache without asking the store."
 },
 {
  "file": "crates/ekr-kernel/src/transaction.rs",
  "line": 248,
  "category": "boundary",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "NodeSupersession.evidence is a plain BTreeSet, so a duplicated evidence id in !SupersedeNode is silently dropped, where Assertion.evidence (ekr-graph assertion.rs:640, unique_set) refuses it."
 },
 {
  "file": "crates/ekr-kernel/src/document.rs",
  "line": 417,
  "category": "boundary",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The ekr.transaction-document/1 decoder and its JSON schema share the operation type with /2, so /1 documents now accept !SupersedeNode (as they accept AddAlias, AttachEvidence, CiteEvidence); whether /1 should exclude post-/1 operations is an open design question."
 },
 {
  "file": "crates/ekr-kernel/src/validate/structural.rs",
  "line": 294,
  "category": "docs",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "AddAlias of a superseded node's alias onto its kept node is refused as alias-already-exists naming the superseded node as already holding it, without saying it is superseded or naming the kept node, and with no next step, though the alias already resolves to the kept node"
 },
 {
  "file": "crates/ekr-kernel/src/validate/structural.rs",
  "line": 246,
  "category": "docs",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a CreateNode whose alias a node superseded in the same transaction holds is refused with advice to use that node, which the same transaction retires, and the working order (create without the alias, then supersede) is not stated"
 },
 {
  "file": "crates/ekr-kernel/src/validate/structural.rs",
  "line": 200,
  "category": "docs",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "the aliases doc says a duplicate alias makes every reference Ambiguous, which is false when AddAlias targets the superseded node's kept node (resolve dedupes to it), so that refusal rests on design 110.3 and the doc does not cite it"
 },
 {
  "file": "crates/ekr-integrate/src/lib.rs",
  "line": 228,
  "category": "contract-drift",
  "severity": "note",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "A T-typed reference answers the kept node N of type U through a superseded alias, while U-typed references to N are refused as reference-type-has-subtypes once U gains a declared child; the design does not say which should hold. Unverified step: the kernel's DefineNodeType admission of a child of a type that already has instances (evolve.rs read, kernel path not traced)."
 },
 {
  "file": "crates/ekr-sdk/src/extraction.rs",
  "line": 396,
  "category": "correctness",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a relation whose subject or object is a key superseded across types passes the pre-check on the key's declared type, resolve() creates and commits the new object node, then the kernel refuses the edge to the kept node's type, leaving that node behind although the pre-check's comment says the relation is refused before anything is written."
 },
 {
  "file": "crates/ekr-sdk/src/extraction.rs",
  "line": 396,
  "category": "correctness",
  "severity": "blocker",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "the relation and property checks use the key's declared type rather than the kept node's, so a fact valid for the node the key resolves to (Person Bob) is refused extraction-relation-ends or extraction-property-undeclared, in both the SDK and the CLI reader (crates/ekr-integrate/src/extraction.rs:707, :730)."
 },
 {
  "file": "crates/ekr-sdk/src/extraction.rs",
  "line": 936,
  "category": "correctness",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "name_supersessions appends the supersession note to any refused fact whose subject or object matches a cross-type superseded record without checking the refusal's cause, so a relation refused for its object Person[alice] is reported as caused by a supersession it does not depend on."
 },
 {
  "file": "crates/ekr-views/src/index.rs",
  "line": 228,
  "category": "boundary",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The referencing index keeps assertions whose subject is a superseded node or a retired edge, so DescribeNode v1 of a live node lists the superseded node's id or retired edge's id in `referencing` and counts it, against the rule that a superseded node is in no read."
 },
 {
  "file": "crates/ekr-views/tests/supersession.rs",
  "line": 6,
  "category": "test-coverage",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "No test covers a live node's referencing list when a superseded subject points at it, so the leak in index.rs:228 passes the suite."
 },
 {
  "file": "crates/ekr-views/src/index.rs",
  "line": 455,
  "category": "contract-drift",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The same head revision reports one assertion total in the overview revisions (all assertions) and another in the projection meta.assertion_count and overview timeline (superseded-subject assertions dropped), and views.yaml gives both readings."
 },
 {
  "file": "crates/ekr-views/src/query.rs",
  "line": 1428,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "describe/1 `referencing` names a superseded subject node or retired edge by id with no pointer to its `by` (index.rs:228 builds referencing without a subject check), against ADR 0017 decision 3"
 },
 {
  "file": "crates/ekr-views/src/detail.rs",
  "line": 604,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "describe/2, the MCP default, drops every relation assertion to or from a superseded node with no pointer, while describe/1 lists it with object_superseded_by, against design 110.5 \"both node-detail formats\""
 },
 {
  "file": "crates/ekr-views/src/index.rs",
  "line": 556,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "roles() dates a held object node from a Relation assertion whose subject is a superseded node or a retired edge, while overview_timeline skips that assertion, so a timeline row's start can come from a fact the revision lists under nothing."
 },
 {
  "file": "crates/ekr/src/cli/explain.rs",
  "line": 434,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "pointers() writes no object_superseded_by for Object::Value(NodeRef) naming a superseded node (also document.rs:367, sample.rs:361), so an assertion whose NodeRef-typed value names a retired node gives no pointer to its replacement; ADR 0017 decision 3 says any read reaching it names the pointer, while kernel.yaml:1102 and views.yaml:339 narrow it to a !Node object."
 },
 {
  "file": "crates/ekr-kernel/src/explain.rs",
  "line": 407,
  "category": "contract-drift",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The library ExplanationLink::Assertion carries no subject_superseded_by or object_superseded_by, which only the CLI rendering (cli/explain.rs:292-299) adds, so a library consumer of the explanation misses the pointers."
 },
 {
  "file": "crates/ekr/tests/store_comparison.rs",
  "line": 283,
  "category": "test-coverage",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "well_formed asserts exactly the six CATEGORIES names, so any fixture that supersedes fails on the conditional seventh superseded category, and no test covers a one-sided superseded comparison."
 },
 {
  "file": "docs/epistemic-knowledge-runtime-design.md",
  "line": 6501,
  "category": "docs",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "§110.6 says \"the two generated node-superseded outcomes\" but views.yaml:3354, :3453 and :3643 generate three (ExpandNeighbourhood, DescribeNode, ProjectTimeline), listed at views-suite.json:324, :334, :358."
 },
 {
  "file": "docs/epistemic-knowledge-runtime-design.md",
  "line": 6476,
  "category": "docs",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "§110.5 says object_superseded_by is on \"both node-detail formats\", but ekr.node-detail/2 (detail.rs:431-478) renders no ProjectedAssertion and no pointer; only /1 (query.rs:901, :1427) carries it."
 },
 {
  "file": "docs/cli.md",
  "line": 2621,
  "category": "docs",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "the refusal is said to be 404 for the MCP and session reads too, but only HTTP returns 404 (view.rs:1329); MCP returns an isError tool refusal (mcp.rs:328) and the session a named Failure (session/views.rs:447)."
 },
 {
  "file": "crates/ekr-kernel/src/apply.rs",
  "line": 362,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "A transaction superseding X by K and Y by L with an X–Y edge retires the edge into X's record only, so Y's record lacks an edge that graph.yaml:511-512 and design §110.2 say it holds."
 },
 {
  "file": "crates/ekr-kernel/src/validate/structural.rs",
  "line": 912,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "A CreateNode whose id equals a retired edge's id is accepted under the v1 and v2 structural validators, though kernel.yaml:741 and design §110.3 say identity-already-exists under every profile; only v3 refuses it."
 },
 {
  "file": "crates/ekr-kernel/src/validate/structural.rs",
  "line": 246,
  "category": "correctness",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "A CreateNode alias held only by a superseded node is refused naming that node with \"use that node\", but its reads answer node-superseded naming the kept node."
 },
 {
  "file": "systems/ekr/domains/graph.yaml",
  "line": 522,
  "category": "docs",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Entity ekr.graph.SupersededNode declares no evidence and no retired-edge relation, though its prose at graph.yaml:510-512 says the record holds both."
 },
 {
  "file": "crates/ekr-graph/src/assertion.rs",
  "line": 279,
  "category": "docs",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "pre-existing",
  "message": "The comment cites graph.yaml:256-259 for recorded_from/recorded_to, which are SubjectKind/Uuid lines in both releases; the declarations are at graph.yaml:289-292."
 },
 {
  "file": "crates/ekr-kernel/src/validate/supersession.rs",
  "line": 135,
  "category": "security",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "No check that node and by are duplicates (type, alias or relation), and the evidence need not bear on either node, so any proposer can supersede any held node with any retained evidence and hide it from search, overview, describe and expand; the single changes row is the only trace."
 },
 {
  "file": "crates/ekr-views/src/changes.rs",
  "line": 551,
  "category": "security",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The NodeSuperseded change omits the reason and emits no rows for the edges retired with the node or for assertions about it, so neighbour reads lose those edges with no change row explaining them."
 },
 {
  "file": "crates/ekr-views/src/index.rs",
  "line": 209,
  "category": "contract-drift",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Assertions about a superseded node are omitted from the document and index projections while query.rs:1147 overview assertion_count still counts them, so the two counts diverge."
 },
 {
  "file": "crates/ekr-kernel/src/apply.rs",
  "line": 367,
  "category": "contract-drift",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "A later supersession repoints the head by of earlier retired records without a change row for those records, so describe_node of an already retired id silently names a different kept node."
 },
 {
  "file": "crates/ekr-kernel/src/validate/reference.rs",
  "line": 331,
  "category": "performance",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "After k supersessions retiring m edges that keep assertions, every validation calls Known::retired per such edge, a linear scan over all records (canonical.rs:484), so one validation costs about k*k*m*log m."
 },
 {
  "file": "crates/ekr-kernel/src/validate/structural.rs",
  "line": 920,
  "category": "performance",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Once a supersession exists, each CreateEdge with a new id scans all records via retired_with in check (:920) and once_held (:836), so a transaction with c new edges costs O(c*S*log e)."
 },
 {
  "file": "crates/ekr-views/src/sample.rs",
  "line": 216,
  "category": "performance",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "A type-filtered sample read calls subject_type for every Active assertion (:313); retired-edge subjects fall to a find_map over all records, so one read costs O(A + R*S)."
 },
 {
  "file": "crates/ekr-kernel/src/apply.rs",
  "line": 367,
  "category": "performance",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Each SupersedeNode commit runs an O(S) repoint loop and an O(E) edge scan, O(k*S) cumulative; dominated by the full head clone at apply.rs:316."
 },
 {
  "file": "crates/ekr-kernel/src/replay.rs",
  "line": 2247,
  "category": "test-coverage",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The edge-index sharing count test has no SupersedeNode commit, so changing the keep-list at replay.rs:185 to rebuild on every supersession passes all tests."
 },
 {
  "file": "crates/ekr-kernel/src/replay.rs",
  "line": 2396,
  "category": "test-coverage",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The warm alias-visit count test has no supersession head, so a regression in AliasCache::advance (validate/aliases.rs:97) that rebuilds after SupersedeNode stays green."
 },
 {
  "file": "crates/ekr-kernel/tests/commit_applies_once.rs",
  "line": 231,
  "category": "test-coverage",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The once-per-commit apply and hash counts are not extended to a SupersedeNode commit."
 },
 {
  "file": "crates/ekr-kernel/tests/adversary_validate_once.rs",
  "line": 727,
  "category": "test-coverage",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Per-validation candidate and edge-index counts are not run on a supersession-bearing head, so the new retired_with scans have no counter to assert against."
 },
 {
  "file": "crates/ekr-kernel/src/validate/structural.rs",
  "line": 415,
  "category": "test-coverage",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Dropping `.chain(superseded)` from the standalone alias scan (public Pipeline::validate path) lets a CreateNode of a superseded node's type and alias pass; no test runs the standalone path over a supersession."
 },
 {
  "file": "crates/ekr-kernel/src/validate/supersession.rs",
  "line": 92,
  "category": "test-coverage",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Dropping `draft.target` from named() (or the AddAlias arm at :120) lets a same-transaction CreateEdge into, or AddAlias of, the node being superseded commit; no test covers either within its own transaction."
 },
 {
  "file": "crates/ekr-kernel/src/validate/reference.rs",
  "line": 133,
  "category": "test-coverage",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "No test names a superseded node as a later CreateEdge target, so removing known.node(tx, draft.target) survives."
 },
 {
  "file": "crates/ekr-kernel/src/validate/reference.rs",
  "line": 272,
  "category": "test-coverage",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Switching this check from known.state.nodes to the candidate accepts a SupersedeNode of a node the same transaction creates; no test supersedes a same-transaction node."
 },
 {
  "file": "crates/ekr-graph/src/supersession.rs",
  "line": 80,
  "category": "test-coverage",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Dropping or reordering a field in SupersededNode::encode leaves every test green; the only root check is assert_ne! at supersede_node.rs:394, and no test pins a non-empty record's root."
 },
 {
  "file": ".engineering/planning/architecture-decision-record/0017-a-node-is-superseded-by-another.md",
  "line": 80,
  "category": "contract-drift",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "ADR decision 4 says `by` must be a node the head holds, but reference.rs:293 accepts a `by` the same transaction creates, pinned by adversary_w20261019a_supersede_node.rs:443-500."
 }
]
```
