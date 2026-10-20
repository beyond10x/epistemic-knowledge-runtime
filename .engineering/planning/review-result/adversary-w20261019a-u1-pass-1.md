---
format: aep.planning-md/3
id: review-result:adversary-w20261019a-u1-pass-1
kind: review-result
status: active
title: Adversary pass 1, wave 2026-10-19a U1 (SupersedeNode)
relations:
- reviews: story:supersede-node
revision: 1
---
Adversary pass 1 (`aep:adversary`) on wave 2026-10-19a unit U1, `story:supersede-node`, tree
`ekr-w19a-u1` at `7f0deb561`, recorded by the coordinator as it returned below.

Owners: 5 findings, 3 implementor (F1, F2, F3), 2 coordinator (F4: the story narrowed ADR 0017
decision 3 to "answers without failing"; J1: neither the ADR nor the brief said what becomes of a
NodeRef property value naming a superseded node).

---

unit: story:supersede-node, U1, tree ekr-w19a-u1 at head 7f0deb561 plus three untracked adversary test files
verdict: NEEDS-CHANGE
cases: executed 1517→1528, red 4
origin: introduced 4 / pre-existing 0 / undecided 1
wrote-outside-worktree: 4 paths (all in the assigned scratch directory)
needs-coordinator: decide whether F3 (extraction refuses a `replaces` fact under a superseded key) holds the unit. I did not take a worktree lease; the coordinator holds the tree.

## 1. What I touched (`git status --short`; the files are untracked, so `git diff --stat` is empty)
```
?? crates/ekr-kernel/tests/adversary_w20261019a_supersede_node.rs        828 lines
?? crates/ekr-views/tests/adversary_w20261019a_supersession_reads.rs     103 lines
?? crates/ekr/tests/adversary_w20261019a_supersede_extraction.rs        234 lines
```
All three are test files. I changed no implementation file. `cargo fmt` was run for ekr-kernel, ekr-views and ekr.

## 2. Cases added (each run alone first, before any suite run)
| case | asserts | now |
|---|---|---|
| `adv_a_node_ref_value_naming_a_node_superseded_in_its_transaction_is_refused` (kernel) | a NodeRef value naming the node the same transaction supersedes (UpdateProperty, CreateNode) is `superseded-node-named-in-transaction` | RED |
| `adv_a_later_node_ref_value_naming_a_superseded_node_is_node_superseded` (kernel, control for the case above) | the same value in a later transaction is `node-superseded` | green |
| `adv_an_operation_on_an_edge_retired_in_its_transaction_is_refused_once` (kernel) | DeleteEdge or AddAssertion on an edge being retired gets exactly one refusal, the structural one | RED |
| `adv_two_nodes_superseded_together_with_an_edge_between_them` (kernel) | a 3-step chain, alias index, the edge kept in one record, reopen and full replay give identical roots | green |
| `adv_merge_invoke_and_an_object_naming_a_superseded_node_are_node_superseded` (kernel) | MergeEntity, Invoke and an assertion object naming the superseded node are refused | green |
| `adv_a_forged_supersession_record_in_a_checkpoint_is_ignored` (kernel) | 6 forgeries of the checkpoint record are all ignored, with a replay from the seed; a store with no supersession writes no `superseded` key | green |
| `adv_a_superseded_nodes_id_and_a_retired_edges_id_are_not_taken_again` (kernel) | design § 110.3: reusing either id is `identity-already-exists` | green |
| `adv_a_seed_holding_a_supersession_record_is_refused` (kernel) | design § 110.4: such a seed is `seed-superseded` | green |
| `adv_every_other_head_read_answers_after_a_supersession` (views) | OCEL, process map, quality, code names and changes (head and revision 1) all answer; OCEL and code names do not name beta | green |
| `adv_a_sampled_fact_about_a_superseded_node_names_the_node_it_is_superseded_by` (views) | ADR 0017 decision 3: a read that reaches the assertion from elsewhere names the pointer | RED |
| `adv_an_extraction_replacing_a_value_under_a_superseded_key_replaces_the_kept_nodes_value` (ekr) | after the supersession, `ekr resolve` of the old key answers Globex (this passes); then a `replaces: true` fact under the old key is not rejected | RED |

Red output when each case was first run alone (verbatim):
```
sqlite: an UpdateProperty answered [] and a CreateNode []; the head holds the supersession with relates = Some([NodeRef(CanonicalRef { id: NodeId(302240678275694148453125) })])
---
sqlite: AddAssertion about the edge
  left: [(Structural, "superseded-node-named-in-transaction"), (Reference, "unresolved-edge")]
 right: [(Structural, "superseded-node-named-in-transaction")]
---
file: the sampled fact names beta without the node it is superseded by: {"assertion":{...,"id":"00000000-0000-4000-8000-000000f20011",...},...,"subject":"00000000-0000-4000-8000-000000000111","subject_kind":"Node","subject_type":""}
---
file: the replacement under the old key is refused: {...,"rejected":[{"issues":[],"item":"facts[0]","refusal":"replacement-without-active-assertion: facts[0]: the store holds no active Organization.legal_name of [\"globex-corp\"]","transaction_id":null}],...}
```
One correction to my process: the first alone-run of the NodeRef case built its CreateNode transaction on the same store as the UpdateProperty one, so its result was confounded. I gave it a store of its own and reran it alone; the output above is that rerun.

## 3. Suite run, after the cases existed
Script: `<home>/.cache/ekr-w19a-u1-adversary/suite.sh`. It runs the brief's two test-gate commands, with `--test adversary_w20261019a_supersede_extraction` added to the second.
- `cargo test --locked -p ekr-graph -p ekr-kernel -p ekr-store -p ekr-integrate -p ekr-views -p ekr-sdk --no-fail-fast`: 1424 passed, 3 failed, 11 ignored, EXIT 101. Failed targets: `-p ekr-kernel --test adversary_w20261019a_supersede_node` and `-p ekr-views --test adversary_w20261019a_supersession_reads`.
- `cargo test --locked -p ekr --test conformance --test conformance_integrate --test docs_cli --test agent_cli --test story_contract --test public_surface --test schema_cli --test adversary_w20261019a_supersede_extraction --no-fail-fast`: 100 passed, 1 failed, EXIT 101. Failed target: `-p ekr --test adversary_w20261019a_supersede_extraction`.
- The only failures are my 4 red cases, each with the same message as above.
- How `before` was counted: no implementor `cases:` line was available to me, and I made no separate deselected run. 1517 is this run's 1528 executed cases minus the 11 in my three files.

## 4. Findings (cover tree 7f0deb561 plus my test files)
| # | file:line | severity | verdict | origin | finding | what reaches it |
|---|---|---|---|---|---|---|
| F1 | crates/ekr-kernel/src/validate/supersession.rs:43 | warning | CONFIRMED | introduced | `named()` ignores values. So a NodeRef in a CreateNode, UpdateProperty, CreateEdge or Invoke argument naming the node being superseded commits in the same transaction, while the same value in a later transaction is `node-superseded`. Also, NodeRefs written before the supersession stay in canonical state (see J1). | any proposer, via `ekr propose`, with an ontology that has a NodeRef property. Fix: collect NodeRefs from values in `named()`. |
| F2 | crates/ekr-kernel/src/validate/candidate.rs:185 | warning | CONFIRMED | introduced | The candidate drops edges being retired, so `Known::edge` (reference.rs:444) adds a false `unresolved-edge` beside the structural refusal. The comment at candidate.rs:168 says such an operation is "refused once". | an erroneous proposer. Inferred, not run: a code comment (structural.rs:1044) says replay compares retained rejections' messages with the current rules, so I expect a fix after release to change those replays. Fix: have `Known::edge` skip edges that `candidate.retires()` reports. |
| F3 | crates/ekr-sdk/src/extraction.rs:819 | warning | NEEDS-CHANGE | introduced | The comment at :818 says `holds` covers "The nodes `ekr resolve` answers a typed reference with", but it reads only `graph.nodes`. A `replaces: true` fact under a superseded key is rejected as `replacement-without-active-assertion`, even though resolve lands on the kept node, which holds the value. | `ekr apply-extraction` (red on both verb runs, file and sqlite), the "next ingestion run" the ADR is for. Fix: follow `graph.superseded` in `holds`, as `ekr_integrate::resolve` does. |
| F4 | crates/ekr-views/src/sample.rs:305 | warning | CONFIRMED | introduced | ADR 0017 decision 3 says a read that reaches such an assertion from elsewhere "names the retired id with its pointer". The fact sample names the superseded subject with no `by`, and with `subject_type: ""`. | `ekr sample` (crates/ekr/src/cli/sample.rs:52). The story narrowed this to "answers without failing", which it does, so it is the story against the ADR. |
| J1 | crates/ekr-kernel/src/apply.rs:333 | note | INFEASIBLE | undecided | NodeRef property values that named the superseded node before the supersession keep naming it. The ADR is silent on this, and fixing it is MergeEntity work, not this unit's. | any store with a NodeRef property. I wrote no case of its own; F1's red output shows the same state. |

## 5. Attacked, could not break
- Encoding of the knowledge root: the tagged option plus map framing leave no prefix ambiguity between "attachments only" and "attachments plus records" (crates/ekr-core/src/canonical.rs:380).
- A forged checkpoint record (`by`, reason, revision, edges, node name, record removed) is ignored on both providers. A store that never supersedes writes no `superseded` key.
- Same-transaction cases: R by K together with K by R or K by L; two nodes superseded together with an edge between them; `by` created in the same transaction; DeleteEdge of an edge being retired (refused once).
- Chains of 3 repoint every record, and the alias index never answers a superseded id.
- Later MergeEntity, Invoke and assertion object naming a superseded node are refused; reuse of the node's or a retired edge's id is refused; a seed holding a record is `seed-superseded`.
- OCEL, process map, quality, code names and changes all answer after a supersession.
- `ekr operations` lists 18 kinds, matching docs/cli.md. The CHANGELOG and the refusal codes in design § 110 match the code.

## 6. Paths written outside the worktree
- <home>/.cache/ekr-w19a-u1-adversary/one.sh
- <home>/.cache/ekr-w19a-u1-adversary/suite.sh
- <home>/.cache/ekr-w19a-u1-adversary/suite-crates.log
- <home>/.cache/ekr-w19a-u1-adversary/suite-ekr.log
- The tests' `tempfile` directories under `$TMPDIR` are removed when they drop.

## 7. Findings block
```findings
- file: crates/ekr-kernel/src/validate/supersession.rs
  line: 43
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a NodeRef value naming the node its own transaction supersedes commits, while the same value one transaction later is refused as node-superseded
- file: crates/ekr-kernel/src/validate/candidate.rs
  line: 185
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: an AddAssertion about an edge its own transaction retires is refused twice, the second time as a false unresolved-edge, contrary to the refused-once comment
- file: crates/ekr-sdk/src/extraction.rs
  line: 819
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the extraction planner's holds check ignores supersession records, so a replaces fact under a superseded key is rejected although ekr resolve lands on the kept node holding the value
- file: crates/ekr-views/src/sample.rs
  line: 305
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the fact sample names a superseded subject without its by and with an empty subject_type, against ADR 0017 decision 3
- file: crates/ekr-kernel/src/apply.rs
  line: 333
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: NodeRef property values written before a supersession keep naming the superseded node; the ADR is silent and the rewrite belongs to MergeEntity
```
