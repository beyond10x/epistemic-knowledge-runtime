---
format: aep.planning-md/3
id: review-result:adversary-w20261020b-u1-pass-2
kind: review-result
status: active
title: Adversary pass 2, wave 2026-10-20b U1 (MergeEntity)
relations:
- reviews: story:merge-entity-applies
revision: 1
---
Adversary pass 2 (`aep:adversary`) on wave 2026-10-20b unit U1, `story:merge-entity-applies`, tree
`ekr-w20b-u1` at `5b59a139f`, recorded by the coordinator as it returned below.

Owners: 1 finding, 1 coordinator (N1: the J2 decision named relation assertions only).

Coordinator's decision on N1 (2026-10-21): no change in this wave. Two nodes joined through a NodeRef
property or a property assertion with a `!Node` object merge into a node that references itself;
that is data the proposer wrote about two records of one entity, the merge rewrites it as it rewrites
every other reference, and the lineage keeps the prior values. Recorded as no-op.

---

unit: story:merge-entity-applies, tree ekr-w20b-u1 at 5b59a139f (base 2a843eb60), no tracked or untracked changes left
verdict: nothing found that blocks or needs a change; one note (CONFIRMED) for the coordinator to decide on
cases: executed 2578→2590, red 0
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 13 paths in `<home>/.cache/ekr-w20b-scratch/adversary-2/` (listed in part 6)
needs-coordinator: yes. Should `merge-self-relation` also cover the two nodes joined through a NodeRef property, which a merge now turns into a self-reference? (N1)

**1. Diff stat**

`git --no-pager diff --stat` prints nothing (exit 0), and `git status --short` prints nothing. No tracked file changed. All 12 cases I wrote passed, so I moved their 4 files to scratch, as pass 1 did with its passing probe. Their test binaries are still in `target/`, which I did not touch.

**2. Cases added (all green; each run alone first, before the suite)**

| file (now `<scratch>/adversary-2/*.rs.green`) | cases | what they assert | run alone |
|---|---|---|---|
| `crates/ekr-kernel/tests/adversary2_w20261020b_merge_kernel.rs` | 5 | Refused as `superseded-node-named-in-transaction`: a SupersedeNode by the absorbed node, and a neighbour of the absorbed node superseded in the same transaction. J3 the other way round, WorkItem merged into MergeRequest: a moved Integer is `wrong-type`; the same estimate written as each type declares it is only `merge-property-conflict`; an active assertion with an Integer estimate is `wrong-type`. An assertion object of the wrong type is `wrong-type`. J2: a joining relation retracted in the merging transaction still refuses; one superseded in an earlier revision does not. N1 probe (see part 4). | 4 passed, then 1 passed (EXIT=0) |
| `crates/ekr-kernel/tests/adversary2_w20261020b_merge_checkpoint.rs` | 1 | A fresh open of a store whose checkpoint holds a full lineage begins 0 replays from the seed. Seven forged lineages (missing, or each of the 7 fields emptied) are ignored and answer what a full replay answers. Run under v1 and v3, on both providers. | 1 passed (EXIT=0) |
| `crates/ekr-views/tests/adversary2_w20261020b_merge_changes.rs` | 3 | F1 checked at every revision. A list held in an IndexCache equals a fresh `Index::load`, byte for byte, for 4 kinds of `since` (revision 0, revision 1, transaction time, valid time). Covered: a retraction before two chained merges, a supersession between two merges, and a retraction about an edge that later collapses. | 3 passed (EXIT=0) |
| `crates/ekr/tests/adversary2_w20261020b_merge_cli.rs` | 3 | F2: two stores differing only in `merge.moved_properties` have different roots and `ekr compare` answers `equal: false`. The worked example in `docs/cli.md` matches what step 10 says the snapshot lists (`collapsed {…0602: …0601}`, `assertions` names `…0503`, the claim now names `…0301`). In `ekr mcp`, a server that read `changes_since` before a merge committed by another process answers the same bytes as a fresh server; `describe_node` refuses with `ekr.views.NodeSuperseded` naming Alice; `resolve` answers Alice. | 3 passed (EXIT=0) |

**3. Suite run (after the cases existed)**

- Command: `pinned.sh cargo test --locked --workspace --no-fail-fast`. Log: `<scratch>/adversary-2/suite-after.log`.
- Result: EXIT=0. 2590 passed, 0 failed, 13 ignored, across 411 `test result` lines.
- `ekr-ontology` ran, including `yaml_ingress`.
- All 4 of my targets ran: 3+1+5+3 = 12 cases.
- "Before" = 2590 − 12 = 2578. That is arithmetic, not a second run.

Rule 1 and rule 2 in this run:
- `base_era_v1_replay`: 4 ok.
- `a_bare_merge_keeps_its_retained_verdict`: ok.
- `every_supersession_record_field_reaches_only_the_knowledge_root_and_its_root_is_pinned`: ok.

**4. Findings (cover 5b59a139f)**

| # | file:line | severity | verdict | origin | finding | what reaches it |
|---|---|---|---|---|---|---|
| N1 | `crates/ekr-kernel/src/validate/merge.rs:365` | note | CONFIRMED | introduced | `merge-self-relation` only matches a `!Relation` assertion with `!Node` subject and object. When the two nodes are joined through a NodeRef property instead, the merge commits and the node kept references itself in three ways: `absorbed.related=[into]` moves (`merge.rs:336`), `into.related=[absorbed]` is rewritten, and a property assertion `absorbed related !Node into` is rewritten. Measured on both providers under v1 and v3. The J2 decision and `kernel.yaml:674` speak only of relation assertions, so this is a decision for the coordinator, not a defect against the spec. | Any NodeRef property whose allowed types admit both nodes, e.g. WorkItem `related` in the fixture, and property assertions with a `!Node` object |

**5. Attacked and could not break**

- **F1:** chains of merges, a retraction after a merge, a supersession whose replacement is about another node, a collapsed edge's assertion, and windows that cross the merge (revision, transaction time, valid time). Covered both without a cache and through MCP across processes.
- **F2:** pairs that differ in a single lineage field. The lineage uses only BTree collections, so ordering cannot differ.
- **J2:** both directions, retracted and superseded joining assertions, and retraction in the merging transaction. A relation with a NodeRef value object cannot occur: `types.rs:539` refuses any relation object that is not a `!Node`.
- **J3:** child-type redeclarations in both directions, for moved values, conflicts and assertion objects.
- **Validation vs apply:** filtering of empty values, `UpdateProperty` removal, `Invoke` (only changes `type_state`), and created or deleted edges in the same transaction. Read side by side; no disagreement found.
- **Checkpoints:** a checkpoint holding a merge is admitted at open (0 seed replays), and forged lineages are rejected.
- **Rule 1:** every new path is gated on `!is_bare()`. 0.0.40 had `deny_unknown_fields` on `EntityMerge`, so no retained document can carry `reason` or `evidence`.

**6. Paths written outside the worktree**

All under `<home>/.cache/ekr-w20b-scratch/adversary-2/`:
- the 4 green probes: `adversary2_w20261020b_merge_{kernel,checkpoint,changes,cli}.rs.green`
- the 6 logs: `{kernel-alone,kernel-selfref,checkpoint-alone,views-alone,cli-alone,suite-after}.log`
- `example-seed.yaml`, `sup.txt`, `mer.txt`

No lease was taken.

**7. Findings block**
```findings
[
  {"file": "crates/ekr-kernel/src/validate/merge.rs", "line": 365, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "merge-self-relation covers only Relation assertions, so two nodes joined through a NodeRef property or a property assertion with a Node object merge into a node that references itself, with no refusal."}
]
```
