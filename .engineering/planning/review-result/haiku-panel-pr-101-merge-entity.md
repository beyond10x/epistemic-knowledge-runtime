---
format: aep.planning-md/3
id: review-result:haiku-panel-pr-101-merge-entity
kind: review-result
status: active
title: 'Haiku panel, 10 read-only reviewers, on PR #101 (MergeEntity, release 0.0.41) before its merge'
relations:
- reviews: story:merge-entity-applies
revision: 1
---
Owners: 26 findings, not attributed: the panel reviewed PR #101 before its merge, whose code is
the wave 2026-10-20b implementor's and whose story, brief and ADR are its coordinator's; no
reviewer split them.

Recorded by the coordinator. The table is the coordinator's triage of the 10 reports. The
findings block holds every reviewer's findings unchanged, in slice order, serialised as one JSON
list. No reviewer wrote its report file (the harness refused sub-agent report files); the
coordinator saved each final answer under <home>/.cache/ekr-w20b-scratch/haiku/report-NN.md.

10 read-only reviewers (general-purpose, model haiku), one slice and one focus each (`slices.md`:
5 correctness, 2 security, 2 performance, 1 efficiency), on `efe536535..7a7e885be`. 9 returned
findings, 1 none (slice 1, apply and rewrite). 26 findings: 3 blocker, 12 warning, 11 note.

## Fixed before the merge (wave/20261020b)

| # | where | what | sev | source | coordinator check |
|---|---|---|---|---|---|
| H1 | `crates/ekr-kernel/src/validate/merge.rs:203` | an edge the merge's transaction creates and deletes stayed in the merge check's near edges: the absorbed edge collapsed onto it, its type's cardinality went unchecked, and a merge committed two out-edges of a cardinality-One type; with other properties it over-refused | blocker | 2.1, 3.2 | confirmed: the new case `a_merge_does_not_collapse_onto_an_edge_its_transaction_deletes` was Validated on 7a7e885be (expected a rejection), refused `edge-cardinality` after |
| H2 | `crates/ekr-kernel/src/validate/merge.rs:137` | every merge of a transaction was checked before the more-than-one-merge refusal: k merges in one document cost k passes over the head, k kept checks and up to k issues per edge in the stored rejection | blocker | 6.1, 6.2, 6.3, 8.1, 8.2, 10.1 | confirmed: two merges, one across sibling types, gave `superseded-node-named-in-transaction` and `merge-types-unrelated`; now the first alone |
| H3 | `crates/ekr/src/cli/agent.rs:181` | the guide's evidence rule left out MergeEntity, so an agent following it is refused `evidence-set-mismatch` | warning | 5.1 | confirmed against `validate/structural.rs:975-977` |
| H4 | `crates/ekr/src/cli/agent.rs:212` | RESOLVE said every edge naming the absorbed node is rewritten; the joining edge and collapsed duplicates are retired | warning | 5.2 | confirmed against `merge.rs:198-202` |
| H5 | `crates/ekr-kernel/src/validate/merge.rs:7` | module doc said no active relation may relate the kept node to itself; design § 111.3 and the check name the joining pair only | note | 2.2 | doc narrowed to the design (pass-2 N1 decision stands) |

0.0.41 is not released, so no retained rejection records a verdict H1 or H2 changes.

## Deferred to `story:merge-entity-review-follow-ups`

| # | where | what | sev | source |
|---|---|---|---|---|
| H6 | `validate/cardinality.rs:112` | a CreateEdge beside a merge is counted against pre-merge edges: refused when the merge retires the joining edge or collapses a duplicate (over-refusal only) | warning | 3.1 |
| H7 | `validate/structural.rs:1146` | `reason: null` and `evidence: null` decode to the bare shape and get the P1 refusal text, not `merge-without-reason` (nothing commits) | warning | 4.1 |
| H8 | `crates/ekr-kernel/src/merge.rs:381` | records repointed from the absorbed node keep no prior `by` in the lineage; a split reads it from the log | warning | 7.1 |
| H9 | `crates/ekr-views/src/changes.rs:425` | `subject_at` scans every record per superseded or retracted assertion change (cached per revision) | warning | 9.1 |
| H10 | `validate/merge.rs:227`, `:317`, `merge.rs:250`, `:212`, `validate/merge.rs:689` | per-merge passes over the head without an index, repeated at commit; `properties_of` per holder; `fates` d·D | warning/note | 8.3–8.7, 10.2, 10.3 |
| H11 | `validate/merge.rs:845`, `reference.rs:220`, `seed.rs:1076` | helpers duplicated from `types.rs`, `cardinality.rs`, the SupersedeNode evidence loop, and the narrow/widen property loops | note | 10.4–10.6 |
| H12 | `crates/ekr-sdk/src/resolve.rs:223` | one cache retain per committed merge | note | 9.2 |

H6 and H7 change a verdict once 0.0.41 has shipped, so they join the rules-era question of
`story:supersession-verdict-fixes-replay-safe`.

```findings
[
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 215,
  "category": "correctness",
  "severity": "blocker",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "A merge's near-edge list keeps a CreateEdge the same transaction deletes, so an absorbed edge collapses onto a phantom, the cardinality check for its type is skipped, and a merge commits two out-edges of a cardinality-One type on the kept node."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 369,
  "category": "docs",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The self-relation check tests only the absorbed/into pair, so an active relation from the absorbed node to itself commits as a relation of the kept node to itself, contrary to the module doc at lines 7-8 (the design table names only the joining pair)."
 },
 {
  "file": "crates/ekr-kernel/src/validate/cardinality.rs",
  "line": 112,
  "category": "correctness",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "A CreateEdge of B->Z of cardinality One is refused edge-cardinality when a same-tx MergeEntity(A->B) retires the head joining edge B->A, though the commit leaves one edge out of B: the CreateEdge count reads pre-merge candidate.outgoing_counts() and still counts B->A (collapses, e.g. X->A onto X->B, are counted twice the same way)."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 203,
  "category": "correctness",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "A CreateEdge cancelled by a DeleteEdge in the same tx stays in the merge check's near set (no deleted filter on the created-edge arm), so a valid MergeEntity whose absorbed edge would rewrite onto it with different properties is refused merge-duplicate-edge-conflict naming the cancelled edge, while apply commits it."
 },
 {
  "file": "crates/ekr-kernel/src/validate/structural.rs",
  "line": 1146,
  "category": "boundary",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "An explicit null (blank YAML value) for both reason and evidence decodes to None, so is_bare() treats the merge as the P1 bare shape and refuses it as unsupported-operation 'MergeEntity is not supported in P1' instead of merge-without-reason / merge-without-evidence."
 },
 {
  "file": "crates/ekr/src/cli/agent.rs",
  "line": 181,
  "category": "contract-drift",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The ekr guide's evidence rule omits MergeEntity from the operations transaction.evidence must cover, but validate/structural.rs:975-977 counts a non-bare merge's evidence as cited, so an agent following the guide gets refused with evidence-set-mismatch."
 },
 {
  "file": "crates/ekr/src/cli/agent.rs",
  "line": 212,
  "category": "docs",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The RESOLVE guide says every edge naming the absorbed node is rewritten rather than retired, but merge.rs:200-202 retires the joining edge and collapsed duplicates, so an agent expects a joining edge to survive as into-to-into."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 137,
  "category": "security",
  "severity": "blocker",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "checks() builds and keeps one MergeCheck per non-bare MergeEntity before the one-merge refusal, so a transaction of about 4e4 identical merges retains k copies of the absorbed node's edges, references and assertions (and the kept node's out-edges of one type) until validation ends, and replay re-runs the same verdict (replay.rs:1194)."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 604,
  "category": "security",
  "severity": "blocker",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "types() emits per-check issues for every merge (lines 662, 687, 701, 717) and the rejection record persists each one (commands.rs:409-425), so k identical merges into a parent type with E refused edge ends store k*E issue records."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 191,
  "category": "security",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "check_one scans the transaction's operations (191, 282) and the whole head (228-236, 317, 330, 349) once per merge before the refusal, so k merges cost k*(ops + head size) of CPU for one request."
 },
 {
  "file": "crates/ekr-kernel/src/merge.rs",
  "line": 381,
  "category": "correctness",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "A merge of A into B repoints every earlier record whose by is A to B (merge.rs:380-384) and records no prior by in MergeLineage (supersession.rs:197-234), so after two records named A and B respectively, both name B and a split reverting the merge cannot restore X1.by=A without changing X2; head reads are correct today."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 142,
  "category": "performance",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "checks() runs check_one for every merge before same_transaction refuses a second merge, so k merges in one V2 document (up to 10,000 ops) cost about k·E edge visits plus k·m op scans before the refusal."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 326,
  "category": "performance",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Each MergeCheck clones every value a merge rewrites and nothing dedupes identical merges, so k repeated merges over R rewritten values hold k·R cloned values during one validation."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 227,
  "category": "performance",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Each merge transaction scans every edge in the graph (near set, plus once more in same_transaction at line 536) with no per-graph index, which is O(E) per merge on top of the existing candidate copy."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 317,
  "category": "performance",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Each merge validation reads every held node property value, every edge property value and every assertion outside any kept index, so cost is O(N·P + E·P + A) per merge transaction."
 },
 {
  "file": "crates/ekr-kernel/src/merge.rs",
  "line": 250,
  "category": "efficiency",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "apply recomputes the near-edge set and fates and walks all node, edge and assertion values again after validation has already read them, so each committed merge reads the graph about twice."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 689,
  "category": "performance",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "ontology.properties_of is resolved per NodeRef-holding property (and per node-subject assertion at line 785), rebuilding the ancestor closure each time, so a hub node referenced by R properties costs R resolutions instead of one per type."
 },
 {
  "file": "crates/ekr-kernel/src/merge.rs",
  "line": 212,
  "category": "performance",
  "severity": "note",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "fates compares each absorbed edge against every held duplicate under the same (type, source, target) key, so d absorbed edges against D parallel into-edges cost d·D bag compares; the step to confirm is whether a Many-type CreateEdge may repeat a held key."
 },
 {
  "file": "crates/ekr-views/src/changes.rs",
  "line": 425,
  "category": "performance",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "subject_at scans every supersession record of the read revision (changes.rs:427-434) for each SupersedeAssertion or RetractAssertion op (changes.rs:547, 560, loop at 513), so deriving a change list costs A*S (A assertion supersede/retract ops, S node supersession records), also on stores that never merged, against the module's claim (changes.rs:22-25) that cost tracks the chosen revisions, not the whole history."
 },
 {
  "file": "crates/ekr-sdk/src/resolve.rs",
  "line": 223,
  "category": "performance",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "observe() runs a full BTreeMap retain over the resolver cache for each committed MergeEntity (k merges times C cached keys per batch), the same shape as the existing SupersedeNode arm at resolve.rs:215; one retain against a set of absorbed ids would do."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 440,
  "category": "efficiency",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "A v2 transaction of k MergeEntity ops (up to 10,000) runs check_one for every merge, each rescanning the head's nodes, edges and assertions, although same_transaction refuses any transaction with more than one merge: cost k times one merge's pass, all wasted."
 },
 {
  "file": "crates/ekr-kernel/src/merge.rs",
  "line": 213,
  "category": "efficiency",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "fates scans a linear list of the node kept's parallel duplicates for each absorbed edge, so A absorbed against D parallel kept edges costs A*D bag comparisons per call, twice per merge; a keyed lookup gives the same answer in O(log n)."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 228,
  "category": "efficiency",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "Three full passes over the head's edges per merge validation (lines 228, 330, 537) and two in commit (merge.rs 250, 299) where one pass could collect the near edges, the NodeRef-carrying edges and the at-absorbed map."
 },
 {
  "file": "crates/ekr-kernel/src/validate/merge.rs",
  "line": 845,
  "category": "efficiency",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "values_admitted, cardinality() and allows re-implement types.rs check and refuse_reason, cardinality.rs count_property and the types.rs endpoint test, so validation and commit can drift apart; the module doc says they answer in one place."
 },
 {
  "file": "crates/ekr-kernel/src/validate/reference.rs",
  "line": 220,
  "category": "efficiency",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "The MergeEntity evidence loop repeats the SupersedeNode evidence loop at line 344 with the same holds_evidence test and message shape; one helper would serve both."
 },
 {
  "file": "crates/ekr-kernel/src/seed.rs",
  "line": 1076,
  "category": "efficiency",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "narrow_lineage repeats the property narrowing loop of narrow_node and narrow_edge, and widen_lineage in crates/ekr-store/src/snapshot.rs:441 repeats the property widening of widen_node and widen_edge; one helper per direction would serve all four."
 }
]
```
